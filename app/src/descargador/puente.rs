//! Transporte local de Flow para las extensiones Integral y Lite de Flow Python.
//! Escucha solo en 127.0.0.1. El código de conexión y las tareas viven en memoria; no se registra nada.

use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tiny_http::{Header, Method, Request, Response, Server};

/// Órdenes de lectura que admite la extensión Lite (`puente.Bridge.call` de Flow Python). Ninguna escribe en SITFA.
const COMANDOS: [&str; 10] = [
    "catalog",
    "capabilities",
    "lock",
    "unlock",
    "prepare",
    "search",
    "download",
    "evidence_open",
    "evidence_capture",
    "evidence_close",
];
/// Lecturas de contexto (E-Q3): solo con la extensión Integral verificada en este puente (`verificar(Integral)`).
const LECTURAS_INTEGRAL: [&str; 3] = ["diary_open", "history_open", "calendar_open"];
const MOTIVOS_EVIDENCIA: [&str; 9] = [
    "estructura", "sesion", "cambio", "pantalla", "tiempo", "pagina", "acceso", "permiso", "pestaña",
];
const CUERPO_MAXIMO: usize = 4 * 1024 * 1024;
const ESPERA_TAREA: Duration = Duration::from_secs(4);
const VIGENCIA_CONEXION: Duration = Duration::from_secs(20);

struct Tarea {
    id: String,
    comando: String,
    payload: Value,
}

struct Estado {
    codigo: String,
    origen: Mutex<Option<String>>,
    tareas: Mutex<VecDeque<Tarea>>,
    hay_tarea: Condvar,
    pendientes: Mutex<HashMap<String, Sender<Value>>>,
    visto: Mutex<Option<Instant>>,
    cerrado: AtomicBool,
    /// La última verificación de la extensión fue Integral: habilita `LECTURAS_INTEGRAL`.
    integral: AtomicBool,
}

pub struct Puente {
    puerto: u16,
    estado: Arc<Estado>,
    servidor: Arc<Server>,
}

fn aleatorio<const N: usize>() -> Result<String, String> {
    let mut datos = [0u8; N];
    getrandom::fill(&mut datos).map_err(|_| "No se pudo generar el código local.")?;
    Ok(datos.iter().map(|b| format!("{b:02x}")).collect())
}

fn es_id_extension(texto: &str) -> bool {
    texto.len() == 32 && texto.bytes().all(|b| (b'a'..=b'p').contains(&b))
}

fn iguales_en_tiempo_constante(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

fn cabecera(peticion: &Request, nombre: &str) -> Option<String> {
    peticion
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(nombre))
        .map(|h| h.value.as_str().to_string())
}

/// Origen CORS válido: `chrome-extension://<id>`.
fn origen_cors(peticion: &Request) -> Option<String> {
    cabecera(peticion, "Origin").filter(|o| {
        o.strip_prefix("chrome-extension://")
            .is_some_and(es_id_extension)
    })
}

/// Identidad de la extensión. Chrome puede omitir `Origin`; la identidad declarada nunca reemplaza al código secreto.
fn origen_extension(peticion: &Request) -> Option<String> {
    let declarada = cabecera(peticion, "X-Sitfa-Extension").unwrap_or_default();
    if !declarada.is_empty() && !es_id_extension(&declarada) {
        return None;
    }
    match cabecera(peticion, "Origin") {
        Some(origen) => {
            let valido = origen_cors(peticion)?;
            if !declarada.is_empty() && origen != format!("chrome-extension://{declarada}") {
                return None;
            }
            Some(valido)
        }
        None => (!declarada.is_empty()).then(|| format!("chrome-extension://{declarada}")),
    }
}

fn responder(peticion: Request, estado_http: u16, datos: Value) {
    let origen = origen_cors(&peticion);
    let mut respuesta = Response::from_string(datos.to_string()).with_status_code(estado_http);
    let mut poner = |nombre: &str, valor: &str| {
        if let Ok(h) = Header::from_bytes(nombre.as_bytes(), valor.as_bytes()) {
            respuesta.add_header(h);
        }
    };
    if let Some(o) = &origen {
        poner("Access-Control-Allow-Origin", o);
    }
    poner("Vary", "Origin");
    poner("Cache-Control", "no-store");
    poner("Content-Type", "application/json; charset=utf-8");
    let _ = peticion.respond(respuesta);
}

fn autorizada(estado: &Estado, peticion: &Request) -> Result<String, &'static str> {
    let origen = origen_extension(peticion).ok_or("EXTENSION_ORIGIN")?;
    let codigo = cabecera(peticion, "X-Sitfa-Code").unwrap_or_default();
    let formato = codigo.len() == 32
        && codigo
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !formato || !iguales_en_tiempo_constante(&codigo, &estado.codigo) {
        return Err("CONNECTION_CODE");
    }
    if estado
        .origen
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|fijado| *fijado != origen)
    {
        return Err("EXTENSION_CHANGED");
    }
    *estado.visto.lock().unwrap() = Some(Instant::now());
    Ok(origen)
}

fn atender(estado: &Estado, mut peticion: Request) {
    if *peticion.method() == Method::Options {
        let Some(origen) = origen_cors(&peticion) else {
            return responder(peticion, 403, json!({}));
        };
        let mut respuesta = Response::empty(204);
        for (nombre, valor) in [
            ("Access-Control-Allow-Origin", origen.as_str()),
            ("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
            (
                "Access-Control-Allow-Headers",
                "Content-Type, X-Sitfa-Code, X-Sitfa-Extension",
            ),
            ("Access-Control-Allow-Private-Network", "true"),
            ("Cache-Control", "no-store"),
        ] {
            if let Ok(h) = Header::from_bytes(nombre.as_bytes(), valor.as_bytes()) {
                respuesta.add_header(h);
            }
        }
        let _ = peticion.respond(respuesta);
        return;
    }
    let origen = match autorizada(estado, &peticion) {
        Ok(o) => o,
        Err(motivo) => return responder(peticion, 403, json!({"error": motivo})),
    };
    let ruta = peticion.url().to_string();
    if *peticion.method() == Method::Get {
        if ruta != "/next" || estado.origen.lock().unwrap().is_none() {
            return responder(peticion, 404, json!({}));
        }
        let limite = Instant::now() + ESPERA_TAREA;
        let mut cola = estado.tareas.lock().unwrap();
        loop {
            while let Some(tarea) = cola.pop_front() {
                // Una tarea cuyo plazo ya venció no se entrega.
                if estado.pendientes.lock().unwrap().contains_key(&tarea.id) {
                    drop(cola);
                    return responder(
                        peticion,
                        200,
                        json!({"id": tarea.id, "command": tarea.comando, "payload": tarea.payload}),
                    );
                }
            }
            let resta = limite.saturating_duration_since(Instant::now());
            if resta.is_zero() || estado.cerrado.load(Ordering::SeqCst) {
                drop(cola);
                return responder(peticion, 200, json!({"idle": true}));
            }
            cola = estado.hay_tarea.wait_timeout(cola, resta).unwrap().0;
        }
    }
    if *peticion.method() != Method::Post {
        return responder(peticion, 404, json!({}));
    }
    let largo = peticion.body_length().unwrap_or(0);
    if largo == 0 || largo > CUERPO_MAXIMO {
        return responder(peticion, 400, json!({"error": "Mensaje invalido."}));
    }
    let mut cuerpo = Vec::with_capacity(largo.min(1 << 20));
    if peticion
        .as_reader()
        .take(CUERPO_MAXIMO as u64 + 1)
        .read_to_end(&mut cuerpo)
        .is_err()
    {
        return responder(peticion, 400, json!({"error": "Mensaje invalido."}));
    }
    let Ok(Value::Object(datos)) = serde_json::from_slice::<Value>(&cuerpo) else {
        return responder(peticion, 400, json!({"error": "Mensaje invalido."}));
    };
    match ruta.as_str() {
        "/connect" => {
            let mut fijado = estado.origen.lock().unwrap();
            if fijado.as_ref().is_some_and(|fijado| *fijado != origen) {
                return responder(peticion, 403, json!({"error": "EXTENSION_CHANGED"}));
            }
            *fijado = Some(origen);
            *estado.visto.lock().unwrap() = Some(Instant::now());
            drop(fijado);
            responder(peticion, 200, json!({"ok": true}))
        }
        "/complete" => {
            if estado.origen.lock().unwrap().is_none() {
                return responder(peticion, 409, json!({"error": "NOT_CONNECTED"}));
            }
            let (Some(id), Some(_)) = (
                datos.get("id").and_then(Value::as_str),
                datos.get("ok").and_then(Value::as_bool),
            ) else {
                return responder(peticion, 400, json!({"error": "Mensaje invalido."}));
            };
            let canal = estado.pendientes.lock().unwrap().remove(id);
            if canal.is_some_and(|canal| canal.send(Value::Object(datos)).is_ok()) {
                responder(peticion, 200, json!({"ok": true}));
            } else {
                responder(peticion, 409, json!({"error": "TASK_EXPIRED"}));
            }
        }
        _ => responder(peticion, 404, json!({})),
    }
}

/// Texto fijo por código de la extensión; el motivo técnico de la evidencia solo se acepta de una lista cerrada.
fn mensaje_error(comando: &str, datos: &Value) -> String {
    let codigo = datos["error"].as_str().unwrap_or("");
    let mut texto = match codigo {
        "FOLLOWUP_REQUIRED" => "Entra a Seguimiento en la pestaña SITFA y vuelve a cargar las opciones.",
        "SITE_ACCESS" => "Comprueba que Chrome siga abierto en SITFA y que la extensión pueda acceder a esa pestaña.",
        "MULTIPLE_FORMS" => "Se encontró más de un formulario de Seguimiento. Abre la conexión desde una pestaña con un solo formulario.",
        "SCREEN_OPEN" => "No se pudo abrir la pantalla elegida. Revisa la sesión SITFA y vuelve a cargar las opciones.",
        "EVIDENCE_CAPTURE" => "No se pudo capturar la consulta vacía. Mantén SITFA visible, pulsa el icono de la extensión desde SITFA y vuelve a conectar. Los archivos guardados se conservan.",
        "EVIDENCE_OPEN" => "No se pudo cargar la vista SITFA para generar el PDF de la consulta vacía. Revisa la sesión y actualiza la extensión. Los archivos guardados se conservan.",
        _ if comando == "catalog" => "La extensión no pudo cargar el catálogo. Revisa la sesión de SITFA y la pestaña de conexión.",
        _ => "La extensión no pudo completar la consulta. Revisa su pestaña de conexión y la sesión SITFA.",
    }
    .to_string();
    if let Some(motivo) = datos["motivo"]
        .as_str()
        .filter(|m| matches!(codigo, "EVIDENCE_CAPTURE" | "EVIDENCE_OPEN") && MOTIVOS_EVIDENCIA.contains(m))
    {
        texto.push_str(&format!(" Motivo técnico: {motivo}."));
    }
    texto
}

impl Puente {
    /// Abre el puente en un puerto libre de 127.0.0.1.
    pub fn abrir() -> Result<Puente, String> {
        let servidor = Arc::new(
            Server::http("127.0.0.1:0")
                .map_err(|e| format!("No se pudo abrir el puente local: {e}"))?,
        );
        let puerto = servidor
            .server_addr()
            .to_ip()
            .map(|a| a.port())
            .ok_or("Puente sin puerto.")?;
        let estado = Arc::new(Estado {
            codigo: aleatorio::<16>()?,
            origen: Mutex::new(None),
            tareas: Mutex::new(VecDeque::new()),
            hay_tarea: Condvar::new(),
            pendientes: Mutex::new(HashMap::new()),
            visto: Mutex::new(None),
            cerrado: AtomicBool::new(false),
            integral: AtomicBool::new(false),
        });
        for _ in 0..4 {
            let (servidor, estado) = (servidor.clone(), estado.clone());
            std::thread::spawn(move || {
                while !estado.cerrado.load(Ordering::SeqCst) {
                    match servidor.recv_timeout(Duration::from_millis(300)) {
                        Ok(Some(peticion)) => atender(&estado, peticion),
                        Ok(None) => {}
                        Err(_) => break,
                    }
                }
            });
        }
        Ok(Puente {
            puerto,
            estado,
            servidor,
        })
    }

    /// La extensión verificada es Integral (habilita las lecturas de contexto) o no (Lite, o verificación fallida).
    pub(super) fn habilitar_lecturas(&self, integral: bool) {
        self.estado.integral.store(integral, Ordering::SeqCst);
    }

    #[cfg(test)]
    fn puerto(&self) -> u16 {
        self.puerto
    }

    /// Lo que el usuario pega en la extensión: `<puerto>-<32 hex>`. Cambia cada vez que se abre el puente.
    pub fn codigo_conexion(&self) -> String {
        format!("{}-{}", self.puerto, self.estado.codigo)
    }

    /// La extensión se conectó y sigue consultando.
    pub fn conectado(&self) -> bool {
        !self.estado.cerrado.load(Ordering::SeqCst)
            && self.estado.origen.lock().unwrap().is_some()
            && self
                .estado
                .visto
                .lock()
                .unwrap()
                .is_some_and(|t| t.elapsed() < VIGENCIA_CONEXION)
    }

    pub(super) fn llamar(
        &self,
        comando: &str,
        payload: Value,
        plazo: Duration,
    ) -> Result<Value, String> {
        let lectura = LECTURAS_INTEGRAL.contains(&comando);
        if !COMANDOS.contains(&comando) && !(lectura && self.estado.integral.load(Ordering::SeqCst)) {
            return Err("Acción no permitida.".into());
        }
        if !self.conectado() {
            return Err("Conecta la extensión desde la pestaña SITFA y mantén abierta su ventana de conexión.".into());
        }
        let id = aleatorio::<12>()?;
        let (envio, recepcion) = channel();
        {
            let mut pendientes = self.estado.pendientes.lock().unwrap();
            if self.estado.cerrado.load(Ordering::SeqCst) {
                return Err("Se cerró la conexión local.".into());
            }
            pendientes.insert(id.clone(), envio);
        }
        self.estado.tareas.lock().unwrap().push_back(Tarea {
            id: id.clone(),
            comando: comando.into(),
            payload,
        });
        self.estado.hay_tarea.notify_all();
        let respuesta = recepcion.recv_timeout(plazo);
        self.estado.pendientes.lock().unwrap().remove(&id);
        let datos = respuesta.map_err(|_| {
            "Chrome no respondió a tiempo o se cerró la conexión local. Los archivos ya verificados se conservan."
        })?;
        if datos["ok"].as_bool() == Some(true) {
            return Ok(datos["result"].clone());
        }
        Err(mensaje_error(comando, &datos))
    }

    pub fn cerrar(&self) {
        self.estado.cerrado.store(true, Ordering::SeqCst);
        self.estado.pendientes.lock().unwrap().clear();
        *self.estado.origen.lock().unwrap() = None;
        self.estado.hay_tarea.notify_all();
        self.servidor.unblock();
    }
}

impl Drop for Puente {
    fn drop(&mut self) {
        self.cerrar();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descargador::{Edicion, Pantalla};
    use std::io::Write;
    use std::net::{Ipv4Addr, TcpStream};

    fn pedir(
        puente: &Puente,
        method: &str,
        ruta: &str,
        origen: &str,
        cuerpo: &Value,
    ) -> (u16, Value) {
        let mut socket = TcpStream::connect((Ipv4Addr::LOCALHOST, puente.puerto())).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(8)))
            .unwrap();
        let datos = cuerpo.to_string();
        write!(
            socket,
            "{method} {ruta} HTTP/1.1\r\nHost: localhost\r\nOrigin: {origen}\r\nX-Sitfa-Code: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{datos}",
            puente.estado.codigo,
            datos.len(),
        ).unwrap();
        let mut respuesta = String::new();
        socket.read_to_string(&mut respuesta).unwrap();
        let (cabecera, cuerpo) = respuesta.split_once("\r\n\r\n").unwrap();
        let status = cabecera.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, serde_json::from_str(cuerpo).unwrap())
    }

    #[test]
    fn catalogo_lite_por_el_puente_local_con_extension_ficticia() {
        let puente = Puente::abrir().unwrap();
        let extension = format!("chrome-extension://{}", "a".repeat(32));
        let otra = format!("chrome-extension://{}", "b".repeat(32));
        let vacio = json!({});
        assert_eq!(
            pedir(
                &puente,
                "POST",
                "/connect",
                "https://sitio-ficticio.invalid",
                &vacio
            )
            .0,
            403
        );
        assert_eq!(
            pedir(&puente, "POST", "/connect", &extension, &vacio).0,
            200
        );
        assert_eq!(pedir(&puente, "POST", "/connect", &otra, &vacio).0, 403);
        // E-Q3: sin la verificación Integral (Lite o recién conectado) las lecturas de contexto se rechazan.
        for excluida in ["diary_open", "history_open", "calendar_open", "navigate", "carga"] {
            assert_eq!(
                puente.llamar(excluida, vacio.clone(), Duration::ZERO).unwrap_err(),
                "Acción no permitida."
            );
        }
        puente.habilitar_lecturas(true);
        for lectura in ["diary_open", "history_open", "calendar_open"] {
            assert_ne!(
                puente.llamar(lectura, vacio.clone(), Duration::ZERO).unwrap_err(),
                "Acción no permitida.",
                "{lectura} debe pasar el filtro con Integral verificada"
            );
        }
        for excluida in ["navigate", "carga"] {
            assert_eq!(puente.llamar(excluida, vacio.clone(), Duration::ZERO).unwrap_err(), "Acción no permitida.");
        }
        puente.habilitar_lecturas(false);
        assert_eq!(puente.llamar("diary_open", vacio.clone(), Duration::ZERO).unwrap_err(), "Acción no permitida.");

        std::thread::scope(|scope| {
            let trabajo = scope.spawn(|| puente.catalogo(Edicion::Lite, Pantalla::Litigantes));
            for (orden, result) in [
                (
                    "catalog",
                    json!({
                        "pantalla": "litigantes", "tribunales": {"901": "Tribunal ficticio"},
                        "estados": {"1": "Pendientes"}
                    }),
                ),
                (
                    "capabilities",
                    json!({
                        "tipo": "SITFA_LITE_CAPACIDADES", "version": "1.1.0", "escritura_rus": false,
                        "protocolos": {"descarga_xls": 1, "pdf_sin_registros": 1}
                    }),
                ),
            ] {
                let (status, tarea) = pedir(&puente, "GET", "/next", &extension, &vacio);
                assert_eq!(status, 200);
                assert_eq!(tarea["command"], orden);
                if orden == "catalog" {
                    assert_eq!(tarea["payload"]["screen"], "litigantes");
                }
                let mensaje = json!({"id": tarea["id"], "ok": true, "result": result});
                assert_eq!(
                    pedir(&puente, "POST", "/complete", &extension, &mensaje).0,
                    200
                );
            }
            let catalogo = trabajo.join().unwrap().unwrap();
            assert_eq!(catalogo.pantalla, Pantalla::Litigantes);
            assert_eq!(catalogo.tribunales["901"], "Tribunal ficticio");
        });
    }
}
