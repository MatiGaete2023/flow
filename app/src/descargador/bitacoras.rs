//! E-Q4a · Lote de bitácoras: el comportamiento de `descargador/lectura_bitacoras.py::DiaryRunner` (Flow Python) para
//! la ruta principal. Enumera cada consulta de Seguimiento, lee la bitácora de cada fila con vínculo comprobado por el
//! puente (solo con la extensión Integral) y escribe `bitacoras.json` y las instantáneas `bitacora-*.html` con el mismo
//! formato que Python. Solo lectura de SITFA: no hay operaciones de escritura en RUS.
//!
//! Pendiente en E-Q4b/c/d: retomar y reutilizar lecturas, calendario, historia e histórico, el diagnóstico anónimo y el
//! Excel consolidado.

use super::lote::{self, Conexion};
use super::sitfa::{self, Consulta, Pares, Respuesta};
use super::{lecturas, vinculos, Alcance, Catalogo, Edicion, Pantalla, Perfil};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const MANIFIESTO: &str = "bitacoras.json";
/// `PAUSA_SITFA_S`, `CHECKPOINT_CADA`, `MAX_LECTURAS_POR_LOTE` y `MAX_FALLAS_SEGUIDAS` de `lectura_bitacoras.py`.
pub const PAUSA_SITFA: Duration = Duration::from_secs(1);
const CHECKPOINT_CADA: usize = 25;
pub const MAX_LECTURAS_POR_LOTE: usize = 5000;
pub const MAX_FALLAS_SEGUIDAS: usize = 5;
const ESPERA_MS: u64 = 60_000;
const MODALIDADES: [(&str, &str); 4] = [("1", "Residencia"), ("2", "Ambulatorio"), ("3", "FAE"), ("4", "DCE")];

/// `lotes.Batch` de Seguimiento para bitácoras (informe por omisión «1», sin fechas ni filtros conservados).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoteBitacoras {
    pub tribunales: Vec<String>,
    pub modalidades: Vec<String>,
    pub pestanas: Vec<String>,
}

impl LoteBitacoras {
    /// `asdict(batch)`.
    fn seleccion(&self) -> Value {
        json!({"tribunals": self.tribunales, "modalities": self.modalidades, "tabs": self.pestanas, "report": "1",
            "dates": null, "keep_filters": false, "screen": "seguimiento", "state": "1", "month": null, "year": null})
    }

    /// `Batch.plan`: tribunal × pestaña × modalidad, con informe solo en Informes.
    fn plan(&self, catalogo: &Catalogo) -> Result<Vec<Perfil>, String> {
        if self.tribunales.is_empty() || self.modalidades.is_empty() || self.pestanas.is_empty() {
            return Err("Seleccione tribunales, modalidades y pestañas.".into());
        }
        for valores in [&self.tribunales, &self.modalidades, &self.pestanas] {
            if valores.iter().collect::<BTreeSet<_>>().len() != valores.len() {
                return Err("El lote tiene opciones repetidas.".into());
            }
        }
        if self.modalidades.iter().any(|m| !MODALIDADES.iter().any(|(c, _)| c == m)) {
            return Err("Modalidad no auditada.".into());
        }
        let mut plan = Vec::new();
        for t in &self.tribunales {
            if !catalogo.tribunales.contains_key(t) {
                return Err("Tribunal fuera del catálogo.".into());
            }
            for tab in &self.pestanas {
                for m in &self.modalidades {
                    plan.push(Perfil {
                        pantalla: Pantalla::Seguimiento,
                        tribunal: t.clone(),
                        alcance: Alcance::Seguimiento {
                            pestana: tab.clone(),
                            modalidad: m.clone(),
                            reporte: (tab == "Informes").then(|| "1".to_string()),
                        },
                        etiqueta: String::new(),
                    });
                }
            }
        }
        Ok(plan)
    }
}

enum Detencion {
    /// `LoteDetenido` / `LimiteLecturas`: detiene todo el lote.
    Lote(String),
    /// Error de una consulta: queda FALLIDA y el lote sigue.
    Consulta(String),
}

impl From<String> for Detencion {
    fn from(e: String) -> Self {
        Detencion::Consulta(e)
    }
}

struct Sesion<'a> {
    conexion: &'a dyn Conexion,
    carpeta: PathBuf,
    estado: Value,
    pendiente: usize,
    aperturas: usize,
    fallas_seguidas: usize,
    cancelar: &'a AtomicBool,
    pausa: Duration,
}

impl Sesion<'_> {
    /// `_Lectura.persistir`: escritura atómica (`bitacoras.tmp` y reemplazo); con `perezosa`, cada `CHECKPOINT_CADA`.
    fn persistir(&mut self, perezosa: bool) -> Result<(), String> {
        if perezosa {
            self.pendiente += 1;
            if self.pendiente < CHECKPOINT_CADA {
                return Ok(());
            }
            self.pendiente = 0;
        }
        let datos = serde_json::to_vec_pretty(&self.estado).map_err(|e| e.to_string())?;
        let temporal = self.carpeta.join("bitacoras.tmp");
        std::fs::write(&temporal, datos)
            .and_then(|_| std::fs::rename(&temporal, self.carpeta.join(MANIFIESTO)))
            .map_err(|e| format!("No se pudo guardar el manifiesto de bitácoras: {e}"))
    }

    fn detener_si_cancelada(&self) -> Result<(), Detencion> {
        if self.cancelar.load(Ordering::SeqCst) {
            return Err(Detencion::Lote("Lectura cancelada. Se conserva lo ya comprobado.".into()));
        }
        Ok(())
    }

    fn pedir(&self, comando: &str, payload: Value) -> Result<Value, String> {
        self.conexion.llamar(comando, payload, lote::PLAZO)
    }
}

/// `json.dumps([...], ensure_ascii=False)` de Python: separadores «, » y cadenas sin escapar lo no ASCII.
fn json_python(v: &Value) -> String {
    match v {
        Value::Array(a) => format!("[{}]", a.iter().map(json_python).collect::<Vec<_>>().join(", ")),
        otro => serde_json::to_string(otro).unwrap_or_default(),
    }
}

fn sha256(datos: &[u8]) -> String {
    format!("{:x}", Sha256::digest(datos))
}

/// `bitacora_html.visible_identity_warnings`.
fn advertencias(leida: &Value, rit: &str, nombre: &str) -> Vec<String> {
    let mut a = Vec::new();
    if lecturas::normalizar(leida["rit"].as_str().unwrap_or("")) != lecturas::normalizar(rit) {
        a.push("Advertencia: RIT visible de bitácora distinto al listado; revisar el ingreso.".to_string());
    }
    if lecturas::normalizar(leida["persona"].as_str().unwrap_or("")) != lecturas::normalizar(nombre) {
        a.push("Advertencia: nombre visible de bitácora distinto al listado; revisar el ingreso.".to_string());
    }
    a
}

/// `faltantes_por_tribunal`: bitácoras que no quedaron LEIDA, por tribunal y estado.
pub fn faltantes_por_tribunal(registros: &[Value]) -> Value {
    let mut salida: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for r in registros.iter().filter(|r| r["estado"] != "LEIDA") {
        let codigo = r["tribunal_codigo"].as_str().unwrap_or("").to_string();
        let item = salida.entry(codigo).or_insert_with(|| {
            Map::from_iter([("nombre".to_string(), json!(r["tribunal_nombre"].as_str().unwrap_or("")))])
        });
        let estado = r["estado"].as_str().unwrap_or("FALLIDA").to_string();
        let n = item.get(&estado).and_then(Value::as_u64).unwrap_or(0) + 1;
        item.insert(estado, json!(n));
    }
    json!(salida)
}

fn ahora_local() -> chrono::DateTime<chrono::Local> {
    chrono::Local::now()
}

/// `DiaryRunner.run` (ruta principal). Devuelve el resultado de `_resultado` sin el Excel ni el diagnóstico (E-Q4d).
pub fn ejecutar(
    conexion: &dyn Conexion,
    catalogo: &Catalogo,
    lote_bitacoras: &LoteBitacoras,
    destino: &Path,
    pausa: Duration,
    cancelar: &AtomicBool,
) -> Result<Value, String> {
    let plan = lote_bitacoras.plan(catalogo)?;
    let inicio = ahora_local();
    let carpeta = destino.join(format!("Bitacoras {}", inicio.format("%Y-%m-%d %H-%M-%S-%6f")));
    std::fs::create_dir_all(destino).map_err(|e| format!("No se pudo crear la carpeta de destino: {e}"))?;
    std::fs::create_dir(&carpeta).map_err(|e| format!("No se pudo crear la carpeta del lote de bitácoras: {e}"))?;
    let mut s = Sesion {
        conexion,
        carpeta: carpeta.clone(),
        estado: json!({
            "version": 1, "tipo": "BITACORAS_LECTURA", "estado": "EN_CURSO", "seleccion": lote_bitacoras.seleccion(),
            "fecha": inicio.format("%Y-%m-%dT%H:%M:%S%.6f%:z").to_string(), "consultas": [], "registros": [],
            "origen": {"version": super::extension::version(Edicion::Integral), "commit": "desarrollo"},
        }),
        pendiente: 0,
        aperturas: 0,
        fallas_seguidas: 0,
        cancelar,
        pausa,
    };
    s.persistir(false)?;
    let mut bloqueado = false;
    let corrida = (|| -> Result<(), Detencion> {
        let version = s.pedir("capabilities", json!({}))?;
        super::comprobar_extension(Edicion::Integral, &version)?;
        conexion.habilitar_lecturas(true);
        s.estado["extension"] = json!({"version": version["version"], "protocolos": version["protocolos"]});
        s.persistir(false)?;
        s.pedir("lock", json!({}))?;
        bloqueado = true;
        for perfil in &plan {
            if !enumerar(&mut s, catalogo, perfil)? {
                break;
            }
        }
        Ok(())
    })();
    let completa = |e: &Value| {
        e["consultas"].as_array().is_some_and(|c| {
            c.len() == plan.len() && c.iter().all(|q| matches!(q["estado"].as_str(), Some("ENUMERADA" | "SIN_REGISTROS")))
        }) && e["registros"].as_array().is_some_and(|r| r.iter().all(|r| r["estado"] == "LEIDA"))
    };
    let cancelada = cancelar.load(Ordering::SeqCst);
    match corrida {
        Ok(()) => {
            s.estado["estado"] =
                json!(if cancelada { "CANCELADA" } else if completa(&s.estado) { "COMPLETA" } else { "INCOMPLETA" });
        }
        Err(Detencion::Lote(e)) | Err(Detencion::Consulta(e)) => {
            s.estado["estado"] = json!(if cancelada { "CANCELADA" } else { "INCOMPLETA" });
            s.estado["error"] = json!(e);
        }
    }
    // `_cerrar`
    if s.estado["estado"] == "EN_CURSO" {
        s.estado["estado"] = json!("INCOMPLETA");
    }
    if s.estado.get("anteriores_no_reenumeradas").is_none() {
        s.estado["anteriores_no_reenumeradas"] = json!([]);
    }
    let guardado = s.persistir(false);
    if bloqueado {
        let _ = conexion.llamar("unlock", json!({}), Duration::from_secs(8));
    }
    conexion.habilitar_lecturas(false);
    guardado?;
    let registros = s.estado["registros"].as_array().cloned().unwrap_or_default();
    let consultas = s.estado["consultas"].as_array().cloned().unwrap_or_default();
    let manifiesto = carpeta.join(MANIFIESTO);
    let detalle = consultas
        .iter()
        .find(|q| q["estado"] == "FALLIDA")
        .map(|q| q["error"].as_str().unwrap_or("").to_string())
        .unwrap_or_else(|| s.estado["error"].as_str().unwrap_or("").to_string());
    Ok(json!({
        "folder": carpeta.display().to_string(),
        "manifest": manifiesto.display().to_string(),
        "sha256": sha256(&std::fs::read(&manifiesto).map_err(|e| e.to_string())?),
        "estado": s.estado["estado"],
        "leidas": registros.iter().filter(|r| r["estado"] == "LEIDA").count(),
        "fallidas": registros.iter().filter(|r| r["estado"] != "LEIDA").count(),
        "consultas_fallidas": consultas.iter().filter(|q| q["estado"] == "FALLIDA").count(),
        "calendarios_leidos": 0, "calendarios_fallidos": 0,
        "detalle": detalle,
        "faltantes_por_tribunal": faltantes_por_tribunal(&registros),
        "historicos": {},
    }))
}

/// `enumerar_listado`: formulario, páginas del listado y lectura de cada fila. `Ok(false)` detiene el lote.
fn enumerar(s: &mut Sesion<'_>, catalogo: &Catalogo, perfil: &Perfil) -> Result<bool, Detencion> {
    s.detener_si_cancelada()?;
    let Alcance::Seguimiento { pestana, modalidad, reporte } = &perfil.alcance else {
        return Err(Detencion::Lote("Las bitácoras se consultan desde Seguimiento.".into()));
    };
    let mut consulta_json = lote::seleccion_edicion(perfil, Edicion::Integral);
    consulta_json["tribunal_nombre"] = json!(catalogo.tribunales.get(&perfil.tribunal).cloned().unwrap_or_default());
    consulta_json["modalidad_nombre"] =
        json!(MODALIDADES.iter().find(|(c, _)| c == modalidad).map_or("", |(_, n)| n));
    consulta_json["estado"] = json!("EN_CURSO");
    consulta_json["paginas"] = json!(0);
    consulta_json["registros"] = json!(0);
    s.estado["consultas"].as_array_mut().unwrap().push(consulta_json);
    let q = s.estado["consultas"].as_array().unwrap().len() - 1;
    s.persistir(false)?;
    let _ = reporte;
    let resultado = (|| -> Result<(), Detencion> {
        let consulta = Consulta::desde(perfil)?;
        let preparado = s.pedir(
            "prepare",
            json!({"selection": lote::seleccion_edicion(perfil, Edicion::Integral), "dates": null, "keep_filters": false}),
        )?;
        let pares: Pares = preparado["pairs"]
            .as_array()
            .filter(|a| a.len() <= 100)
            .and_then(|a| {
                a.iter()
                    .map(|par| match par.as_array().map(Vec::as_slice) {
                        Some([Value::String(k), Value::String(v)]) => Some((k.clone(), v.clone())),
                        _ => None,
                    })
                    .collect()
            })
            .ok_or("El formulario no entregó parámetros válidos.".to_string())?;
        consulta.validar_consulta(&pares)?;
        if pares.iter().find(|(k, _)| k == "FLG_Consulta").map(|(_, v)| v.as_str()) != Some("0") {
            return Err(Detencion::Consulta("El filtro de fechas no coincide.".into()));
        }
        let html = lote::cuerpo(&s.pedir("search", json!({"pairs": pares, "timeout": ESPERA_MS}))?, false)?;
        let primera = match sitfa::validar_respuesta(&html, &pares, &consulta, None)? {
            Respuesta::SinRegistros => {
                s.estado["consultas"][q]["estado"] = json!("SIN_REGISTROS");
                s.persistir(false)?;
                return Ok(());
            }
            Respuesta::ConRegistros(p) => p,
        };
        if primera.actual != 1 {
            return Err(Detencion::Consulta("La consulta no comenzó en la primera página.".into()));
        }
        let mut vistos = BTreeSet::new();
        let (mut actuales, mut pagina, mut cuerpo_html) = (pares.clone(), primera.clone(), html);
        for numero in 1..=primera.total {
            s.detener_si_cancelada()?;
            if numero > 1 {
                actuales = sitfa::siguiente_consulta(&actuales, &consulta, &pagina, numero)?;
                cuerpo_html = lote::cuerpo(&s.pedir("search", json!({"pairs": actuales, "timeout": ESPERA_MS}))?, false)?;
                pagina = match sitfa::validar_respuesta(&cuerpo_html, &actuales, &consulta, Some((numero, &primera)))? {
                    Respuesta::ConRegistros(p) => p,
                    Respuesta::SinRegistros => {
                        return Err(Detencion::Consulta(
                            "No se pudieron identificar todos los registros de la pagina.".into(),
                        ))
                    }
                };
            }
            let seleccion = vinculos::Seleccion {
                tribunal: perfil.tribunal.clone(),
                screen: "seguimiento".into(),
                tab: pestana.clone(),
            };
            let filas = vinculos::extraer_filas(&cuerpo_html, &seleccion)?;
            // `listing_rows`: el control de filas debe coincidir con el listado validado.
            let mut control = sitfa::Registros::new();
            for v in &filas {
                let clave = super::sitfa::registros::clave(&v.rit, &v.nombre)?;
                *control.entry(clave).or_insert(0) += 1;
            }
            if control != pagina.registros {
                return Err(Detencion::Consulta("El control de filas no coincide con el listado validado.".into()));
            }
            for v in &filas {
                leer_fila(s, perfil, pestana, modalidad, q, numero, v, &mut vistos)?;
            }
            s.estado["consultas"][q]["paginas"] = json!(numero);
            s.persistir(false)?;
        }
        s.estado["consultas"][q]["estado"] = json!("ENUMERADA");
        s.persistir(false)?;
        Ok(())
    })();
    match resultado {
        Ok(()) => Ok(true),
        Err(Detencion::Lote(e)) => {
            s.estado["consultas"][q]["estado"] = json!("INCOMPLETA");
            Err(Detencion::Lote(e))
        }
        Err(Detencion::Consulta(e)) => {
            s.estado["consultas"][q]["estado"] = json!("FALLIDA");
            s.estado["consultas"][q]["error"] = json!(e);
            s.persistir(false)?;
            Ok(!s.cancelar.load(Ordering::SeqCst))
        }
    }
}

/// `leer_bitacora`: identidad, lectura de la bitácora de la fila y pausa.
#[allow(clippy::too_many_arguments)]
fn leer_fila(
    s: &mut Sesion<'_>,
    perfil: &Perfil,
    pestana: &str,
    modalidad: &str,
    q: usize,
    numero: usize,
    v: &vinculos::Vinculo,
    vistos: &mut BTreeSet<(String, String, String)>,
) -> Result<(), Detencion> {
    s.detener_si_cancelada()?;
    let consulta = &s.estado["consultas"][q];
    let (tribunal_nombre, modalidad_nombre) = (consulta["tribunal_nombre"].clone(), consulta["modalidad_nombre"].clone());
    let mut r = if v.bitacora_verificada {
        v.bitacora_python()
    } else {
        json!({"rit": v.rit, "nombre": v.nombre, "programa": v.programa, "rut": v.rut,
            "vinculo_diagnostico": v.diagnostico_bitacora})
    };
    r["fila"] = json!(v.fila);
    r["tribunal_codigo"] = json!(perfil.tribunal);
    r["tribunal_nombre"] = tribunal_nombre;
    r["modalidad"] = json!(modalidad);
    r["modalidad_nombre"] = modalidad_nombre;
    r["pestana"] = json!(pestana);
    r["pagina"] = json!(numero);
    r["estado"] = json!("PENDIENTE");
    r["error"] = json!("");
    let campo = |r: &Value, k: &str| r[k].as_str().unwrap_or("").to_string();
    let identidad = (perfil.tribunal.clone(), campo(&r, "causa_id"), campo(&r, "ingreso_id"));
    let completa = !identidad.0.is_empty() && !identidad.1.is_empty() && !identidad.2.is_empty();
    if completa && !vistos.insert(identidad.clone()) {
        return Err(Detencion::Consulta("Un ingreso está repetido en el listado; se detuvo esta consulta.".into()));
    }
    let base = json!([pestana, modalidad, [identidad.0, identidad.1, identidad.2], r["rit"], r["nombre"], r["programa"],
        r["rut"], if completa { json!("") } else { r["fila"].clone() }]);
    r["clave"] = json!(sha256(json_python(&base).as_bytes()));
    s.estado["registros"].as_array_mut().unwrap().push(r);
    let i = s.estado["registros"].as_array().unwrap().len() - 1;
    let registros = s.estado["consultas"][q]["registros"].as_u64().unwrap_or(0) + 1;
    s.estado["consultas"][q]["registros"] = json!(registros);
    if !completa {
        s.estado["registros"][i]["estado"] = json!("SIN_VINCULO");
        s.estado["registros"][i]["error"] = json!(format!(
            "La fila no tiene un enlace de bitácora inequívoco. No se construyó una consulta por RIT. Diagnóstico: {}",
            s.estado["registros"][i]["vinculo_diagnostico"].as_str().unwrap_or("NO_VERIFICADO")
        ));
    } else {
        let params = json!({"tipo_popUp": campo(&s.estado["registros"][i], "tipo_popup"), "CRR_IdCausa": identidad.1,
            "COD_Tribunal": perfil.tribunal, "TIP_Consulta": modalidad, "ID_Ingreso": identidad.2,
            "COD_Etapa": campo(&s.estado["registros"][i], "etapa"), "FLG_MejorNinez": campo(&s.estado["registros"][i], "antiguo")});
        s.estado["registros"][i]["params"] = params.clone();
        match abrir(s, i, &params) {
            Ok(()) => {}
            Err(Detencion::Lote(e)) => {
                if s.estado["registros"][i]["estado"] != "LEIDA" {
                    s.estado["registros"][i]["estado"] = json!("PENDIENTE");
                    s.estado["registros"][i]["error"] = json!("Pendiente: se alcanzó el máximo de lecturas del lote.");
                }
                return Err(Detencion::Lote(e));
            }
            Err(Detencion::Consulta(e)) => {
                s.estado["registros"][i]["estado"] = json!("FALLIDA");
                s.estado["registros"][i]["error"] = json!(e);
            }
        }
    }
    let estado = s.estado["registros"][i]["estado"].as_str().unwrap_or("").to_string();
    if estado == "FALLIDA" {
        s.fallas_seguidas += 1;
    } else if estado == "LEIDA" {
        s.fallas_seguidas = 0;
    }
    s.persistir(true)?;
    if s.fallas_seguidas >= MAX_FALLAS_SEGUIDAS {
        return Err(Detencion::Lote(format!(
            "{MAX_FALLAS_SEGUIDAS} bitácoras seguidas fallaron ({}). Se detuvo el lote para no esperar en cada causa. \
             Revisa la sesión SITFA y usa «Retomar paquete…»: no se vuelve a leer lo ya leído.",
            s.estado["registros"][i]["error"].as_str().unwrap_or("")
        )));
    }
    let espera = if estado != "SIN_VINCULO" { s.pausa } else { Duration::from_millis(50) };
    let fin = std::time::Instant::now() + espera;
    while std::time::Instant::now() < fin {
        if s.cancelar.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep((fin - std::time::Instant::now()).min(Duration::from_millis(20)));
    }
    s.detener_si_cancelada()
}

/// `_abrir_bitacora`: `diary_open`, cotejo de huella, parámetros y fecha, lectura e instantánea local exclusiva.
fn abrir(s: &mut Sesion<'_>, i: usize, params: &Value) -> Result<(), Detencion> {
    if s.aperturas >= MAX_LECTURAS_POR_LOTE {
        return Err(Detencion::Lote(format!(
            "Se alcanzó el máximo de {MAX_LECTURAS_POR_LOTE} lecturas por lote. Usa «Más › Retomar paquete…» (o \
             «Reintentar lecturas fallidas…») para continuar."
        )));
    }
    s.aperturas += 1;
    let respuesta = s.pedir("diary_open", json!({"params": params, "timeout": ESPERA_MS}))?;
    let datos = lote::cuerpo(&respuesta, false)?;
    let huella = sha256(&datos);
    if datos.len() > 8 * 1024 * 1024 || respuesta["digest"].as_str() != Some(huella.as_str()) || &respuesta["params"] != params {
        return Err(Detencion::Consulta("La respuesta de bitácora no coincide con la solicitud.".into()));
    }
    let capturada = respuesta["startedAt"].as_str().unwrap_or("").to_string();
    chrono::DateTime::parse_from_rfc3339(&capturada.replace('Z', "+00:00"))
        .map_err(|_| Detencion::Consulta("La fecha de la lectura de bitácora no es válida.".into()))?;
    let parametros: BTreeMap<String, String> = serde_json::from_value(params.clone()).map_err(|e| e.to_string())?;
    let leida = lecturas::leer_bitacora(&datos, &parametros, "", &capturada)?;
    let r = &s.estado["registros"][i];
    let avisos = advertencias(&leida, r["rit"].as_str().unwrap_or(""), r["nombre"].as_str().unwrap_or(""));
    let clave = r["clave"].as_str().unwrap_or("").to_string();
    let nombre = format!("bitacora-{}-{}.html", &clave[..24], &huella[..16]);
    let ruta = s.carpeta.join(&nombre);
    if ruta.exists() {
        if std::fs::read(&ruta).map_err(|e| e.to_string())? != datos {
            return Err(Detencion::Consulta("El archivo local de bitácora cambió.".into()));
        }
    } else {
        lote::guardar_exclusivo(&ruta, &datos)?;
    }
    let r = &mut s.estado["registros"][i];
    r["error"] = json!(avisos.join(" "));
    r["advertencias"] = json!(avisos);
    r["estado"] = json!("LEIDA");
    r["archivo"] = json!(nombre);
    r["sha256"] = json!(huella);
    r["capturada"] = json!(capturada);
    r["entradas"] = json!(leida["entries"].as_array().map_or(0, Vec::len));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Reproduce las respuestas que Flow Python recibió del puente ficticio, en el mismo orden.
    struct Guion {
        pasos: Vec<Value>,
        pedidos: RefCell<Vec<Value>>,
    }

    impl Conexion for Guion {
        fn llamar(&self, comando: &str, payload: Value, _plazo: Duration) -> Result<Value, String> {
            let n = self.pedidos.borrow().len();
            self.pedidos.borrow_mut().push(json!({"comando": comando, "payload": payload}));
            let paso = self.pasos.get(n).ok_or_else(|| format!("orden {n} ({comando}) fuera del guion de Python"))?;
            if paso["comando"] != comando {
                return Err(format!("orden {n}: Rust pide {comando}, Python pidió {}", paso["comando"]));
            }
            match paso.get("respuesta") {
                Some(r) => Ok(r.clone()),
                None => Err(paso["error"].as_str().unwrap_or("error del puente").to_string()),
            }
        }
    }

    /// `null` (Python) y `{}` (Rust) son la misma orden sin datos.
    fn sin_vacio(v: &Value) -> Value {
        if v.is_null() || v.as_object().is_some_and(Map::is_empty) {
            Value::Null
        } else {
            v.clone()
        }
    }

    fn instantaneas(carpeta: &Path) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        for e in std::fs::read_dir(carpeta).unwrap().flatten() {
            let nombre = e.file_name().to_string_lossy().to_string();
            if nombre.starts_with("bitacora-") && nombre.ends_with(".html") {
                m.insert(nombre, sha256(&std::fs::read(e.path()).unwrap()));
            }
        }
        m
    }

    /// E-Q4a: mismas órdenes al puente, mismo `bitacoras.json`, mismas instantáneas y mismo resultado que
    /// `DiaryRunner` de Flow Python en los escenarios de `app/qa/generar_bitacoras.py` (`fecha` declarada).
    #[test]
    fn lote_de_bitacoras_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/bitacoras.json")).unwrap();
        let catalogo: Catalogo = serde_json::from_value(datos["catalogo"].clone()).unwrap();
        let (mut iguales, mut fallos) = (0usize, Vec::new());
        let escenarios = datos["escenarios"].as_array().unwrap();
        for e in escenarios {
            let nombre = e["escenario"].as_str().unwrap();
            let lista = |k: &str| -> Vec<String> { serde_json::from_value(e["lote"][k].clone()).unwrap() };
            let lote = LoteBitacoras { tribunales: lista("tribunales"), modalidades: lista("modalidades"), pestanas: lista("pestanas") };
            let guion = Guion { pasos: e["guion"].as_array().unwrap().clone(), pedidos: RefCell::default() };
            let destino = std::env::temp_dir().join(format!("flow-eq4-{}-{nombre}", std::process::id()));
            let _ = std::fs::remove_dir_all(&destino);
            let resultado = ejecutar(&guion, &catalogo, &lote, &destino, Duration::ZERO, &AtomicBool::new(false)).unwrap();
            let carpeta = PathBuf::from(resultado["folder"].as_str().unwrap());
            let mut manifiesto: Value = serde_json::from_slice(&std::fs::read(carpeta.join(MANIFIESTO)).unwrap()).unwrap();
            manifiesto["fecha"] = json!("<FECHA>");
            let mut diferencias = Vec::new();
            let pedidos = guion.pedidos.borrow();
            let esperados = e["guion"].as_array().unwrap();
            if pedidos.len() != esperados.len() {
                diferencias.push(format!("órdenes: Rust {} / Python {}", pedidos.len(), esperados.len()));
            }
            for (k, (p, x)) in pedidos.iter().zip(esperados).enumerate() {
                if p["comando"] != x["comando"] || sin_vacio(&p["payload"]) != sin_vacio(&x["payload"]) {
                    diferencias.push(format!("orden {k}: Rust {p} / Python {}", json!({"comando": x["comando"], "payload": x["payload"]})));
                }
            }
            for clave in manifiesto.as_object().unwrap().keys().chain(e["manifiesto"].as_object().unwrap().keys()) {
                if manifiesto.get(clave) != e["manifiesto"].get(clave) {
                    diferencias.push(format!("manifiesto.{clave}:\n    Rust   {:?}\n    Python {:?}", manifiesto.get(clave), e["manifiesto"].get(clave)));
                }
            }
            let esperadas: BTreeMap<String, String> = serde_json::from_value(e["instantaneas"].clone()).unwrap();
            if instantaneas(&carpeta) != esperadas {
                diferencias.push(format!("instantáneas: Rust {:?} / Python {esperadas:?}", instantaneas(&carpeta)));
            }
            for (clave, valor) in e["resultado"].as_object().unwrap() {
                if &resultado[clave] != valor {
                    diferencias.push(format!("resultado.{clave}: Rust {} / Python {valor}", resultado[clave]));
                }
            }
            let _ = std::fs::remove_dir_all(&destino);
            if diferencias.is_empty() {
                iguales += 1;
            } else {
                diferencias.dedup();
                fallos.push(format!("{nombre}:\n  {}", diferencias.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("BITACORAS: {iguales}/{}, fallos {}", escenarios.len(), fallos.len());
        assert_eq!(escenarios.len(), 4, "el fixture cambió de tamaño: regenerar y revisar");
        assert!(fallos.is_empty(), "{} escenarios difieren de Flow Python", fallos.len());
    }
}
