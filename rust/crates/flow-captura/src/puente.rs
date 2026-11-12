//! Puente local con la extensión de Chrome (`descargador_lite/app/puente.py`).
//! Escucha solo en 127.0.0.1. El código de conexión y las tareas viven en memoria; no se registra nada.

use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tiny_http::{Header, Method, Request, Response, Server};

/// Lista cerrada de órdenes que el programa puede pedir a la extensión. Todas son de lectura.
pub const COMANDOS: [&str; 10] = [
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
const CUERPO_MAXIMO: usize = 112 * 1024 * 1024;
const ESPERA_TAREA: Duration = Duration::from_secs(4);
const VIGENCIA_CONEXION: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorPuente {
    /// Código estable (`SITFA_TASK`, `FOLLOWUP_REQUIRED`, `SIN_CONEXION`, `TIEMPO`…).
    pub codigo: String,
    pub mensaje: String,
}

impl std::fmt::Display for ErrorPuente {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.mensaje)
    }
}

fn error(codigo: &str, mensaje: &str) -> ErrorPuente {
    ErrorPuente { codigo: codigo.into(), mensaje: mensaje.into() }
}

fn mensaje_de(codigo: &str) -> &'static str {
    match codigo {
        "FOLLOWUP_REQUIRED" => {
            "El enlace con Chrome funciona, pero no se encuentra la pantalla de consulta. Entra a Seguimiento en la \
             pestaña original de SITFA y vuelve a cargar las opciones."
        }
        "SITE_ACCESS" => {
            "Chrome no permite acceder a la pestaña SITFA. Revisa que siga abierta y que la extensión tenga acceso a \
             familia.pjud.cl."
        }
        "MULTIPLE_FORMS" => "Hay más de un formulario de Seguimiento en esa pestaña. Abre la conexión desde una pestaña con un solo formulario.",
        "SCREEN_OPEN" => "No se pudo abrir la pantalla elegida. Revisa la sesión de SITFA y vuelve a cargar las opciones.",
        "EVIDENCE_CAPTURE" => "No se pudo capturar la consulta vacía. Mantén SITFA visible y vuelve a conectar.",
        "EVIDENCE_OPEN" => "No se pudo cargar la vista de SITFA para el PDF de la consulta vacía. Revisa la sesión.",
        _ => "La extensión no pudo completar la consulta. Revisa su pestaña de conexión y la sesión de SITFA.",
    }
}

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
}

pub struct Puente {
    puerto: u16,
    estado: Arc<Estado>,
    servidor: Arc<Server>,
}

fn aleatorio(bytes: usize) -> String {
    let mut datos = vec![0u8; bytes];
    getrandom::getrandom(&mut datos).expect("generador aleatorio del sistema");
    datos.iter().map(|b| format!("{b:02x}")).collect()
}

fn es_id_extension(texto: &str) -> bool {
    texto.len() == 32 && texto.bytes().all(|b| (b'a'..=b'p').contains(&b))
}

fn iguales_en_tiempo_constante(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

fn cabecera(peticion: &Request, nombre: &str) -> Option<String> {
    peticion.headers().iter().find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(nombre)).map(|h| h.value.as_str().to_string())
}

/// Origen CORS válido: `chrome-extension://<id>`.
fn origen_cors(peticion: &Request) -> Option<String> {
    cabecera(peticion, "Origin").filter(|o| o.strip_prefix("chrome-extension://").is_some_and(es_id_extension))
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
    let formato = codigo.len() == 32 && codigo.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !formato || !iguales_en_tiempo_constante(&codigo, &estado.codigo) {
        return Err("CONNECTION_CODE");
    }
    if estado.origen.lock().unwrap().as_ref().is_some_and(|fijado| *fijado != origen) {
        return Err("EXTENSION_CHANGED");
    }
    *estado.visto.lock().unwrap() = Some(Instant::now());
    Ok(origen)
}

fn atender(estado: &Estado, mut peticion: Request) {
    if *peticion.method() == Method::Options {
        let Some(origen) = origen_cors(&peticion) else { return responder(peticion, 403, json!({})) };
        let mut respuesta = Response::empty(204);
        for (nombre, valor) in [
            ("Access-Control-Allow-Origin", origen.as_str()),
            ("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
            ("Access-Control-Allow-Headers", "Content-Type, X-Sitfa-Code, X-Sitfa-Extension"),
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
                    return responder(peticion, 200, json!({"id": tarea.id, "command": tarea.comando, "payload": tarea.payload}));
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
    if peticion.as_reader().take(CUERPO_MAXIMO as u64 + 1).read_to_end(&mut cuerpo).is_err() {
        return responder(peticion, 400, json!({"error": "Mensaje invalido."}));
    }
    let Ok(Value::Object(datos)) = serde_json::from_slice::<Value>(&cuerpo) else {
        return responder(peticion, 400, json!({"error": "Mensaje invalido."}));
    };
    match ruta.as_str() {
        "/connect" => {
            *estado.origen.lock().unwrap() = Some(origen);
            responder(peticion, 200, json!({"ok": true}))
        }
        "/complete" => {
            if estado.origen.lock().unwrap().is_none() {
                return responder(peticion, 409, json!({"error": "NOT_CONNECTED"}));
            }
            let (Some(id), Some(_)) = (datos.get("id").and_then(Value::as_str), datos.get("ok").and_then(Value::as_bool)) else {
                return responder(peticion, 400, json!({"error": "Mensaje invalido."}));
            };
            match estado.pendientes.lock().unwrap().remove(id) {
                Some(canal) if canal.send(Value::Object(datos.clone())).is_ok() => responder(peticion, 200, json!({"ok": true})),
                _ => responder(peticion, 409, json!({"error": "TASK_EXPIRED"})),
            }
        }
        _ => responder(peticion, 404, json!({})),
    }
}

impl Puente {
    /// Abre el puente en un puerto libre de 127.0.0.1.
    pub fn abrir() -> Result<Puente, String> {
        let servidor = Arc::new(Server::http("127.0.0.1:0").map_err(|e| format!("No se pudo abrir el puente local: {e}"))?);
        let puerto = servidor.server_addr().to_ip().map(|a| a.port()).ok_or("Puente sin puerto.")?;
        let estado = Arc::new(Estado {
            codigo: aleatorio(16),
            origen: Mutex::new(None),
            tareas: Mutex::new(VecDeque::new()),
            hay_tarea: Condvar::new(),
            pendientes: Mutex::new(HashMap::new()),
            visto: Mutex::new(None),
            cerrado: AtomicBool::new(false),
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
        Ok(Puente { puerto, estado, servidor })
    }

    pub fn puerto(&self) -> u16 {
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
            && self.estado.visto.lock().unwrap().is_some_and(|t| t.elapsed() < VIGENCIA_CONEXION)
    }

    /// Pide una tarea a la extensión y espera su resultado.
    pub fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, ErrorPuente> {
        if !COMANDOS.contains(&comando) {
            return Err(error("NO_PERMITIDA", "Acción no permitida."));
        }
        if !self.conectado() {
            return Err(error(
                "SIN_CONEXION",
                "Conecta la extensión desde la pestaña SITFA de tu Chrome y mantén abierta su pestaña de conexión.",
            ));
        }
        let id = aleatorio(12);
        let (envio, recepcion) = channel();
        self.estado.pendientes.lock().unwrap().insert(id.clone(), envio);
        self.estado.tareas.lock().unwrap().push_back(Tarea { id: id.clone(), comando: comando.into(), payload });
        self.estado.hay_tarea.notify_all();
        let respuesta = recepcion.recv_timeout(plazo);
        self.estado.pendientes.lock().unwrap().remove(&id);
        let datos = respuesta.map_err(|_| error("TIEMPO", "Chrome no respondió a tiempo. Lo ya verificado se conserva."))?;
        if datos["ok"].as_bool() == Some(true) {
            return Ok(datos["result"].clone());
        }
        let codigo = datos["error"].as_str().unwrap_or("SITFA_TASK");
        let conocido = matches!(codigo, "FOLLOWUP_REQUIRED" | "SITE_ACCESS" | "MULTIPLE_FORMS" | "SCREEN_OPEN" | "EVIDENCE_CAPTURE" | "EVIDENCE_OPEN");
        Err(error(if conocido { codigo } else { "SITFA_TASK" }, mensaje_de(codigo)))
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
