//! «Crear paquete de este lote»: arma `Captura_CSMP.zip` (`CSMP_CAPTURE_V1`) desde lotes de Flow Descargador, como
//! `descargador/export_csmp_capture.py::export_capture` con las bitácoras del mismo lote (`match_diaries=True`).
//!
//! Solo lee archivos ya verificados del lote (XLS y lecturas `lectura-*.json`, cada uno contra su huella); no consulta
//! SITFA ni escribe en RUS. El ZIP no lleva originales ni el XLSX consolidado.

use crate::captura::Vinculos;
use flow_captura::{
    lotes::{huella, ruta_archivo, Resumen},
    recoleccion::{self, Referencia},
    sitfa::Seleccion,
    vinculos::{self, Vinculo},
};
use flow_dominio::{Celda, Etapa};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::Write,
    path::{Path, PathBuf},
    sync::LazyLock,
};

/// Nombre del paquete que entrega «Consultar y crear paquete».
pub const NOMBRE: &str = "Captura_CSMP.zip";
const VERSION: &str = "1.0.0";
const MAPPING_VERSION: &str = "baseline-20261008-v1";

const COMMON: [&str; 19] = [
    "capture_id",
    "query_id",
    "source_system",
    "court_code",
    "court_name",
    "rit_original",
    "cause_id",
    "ingreso_id",
    "person_id",
    "program_id",
    "modality_raw",
    "modality_code",
    "tab",
    "captured_at",
    "original_file",
    "original_sheet",
    "original_row",
    "source_sha256",
    "coverage",
];
const CONSULTAS: [&str; 18] = [
    "query_id",
    "source_system",
    "screen",
    "tab",
    "court_code",
    "court_name",
    "modality_code",
    "coverage",
    "captured_at",
    "filters_json",
    "expected_pages",
    "saved_pages",
    "period_from",
    "period_to",
    "time_zone",
    "history_complete",
    "institutional_verified",
    "error",
];
const INCIDENCIAS: [&str; 9] =
    ["capture_id", "query_id", "record_id", "incident_id", "phase", "code", "severity", "message", "recoverable"];
const TABLAS: [&str; 7] = ["consultas", "registros", "bitacoras", "calendarios", "historia_tramites", "documentos", "incidencias"];
const MODALIDADES: [(&str, &str); 4] = [("1", "RES"), ("2", "AMB"), ("3", "FAE"), ("4", "DCE")];

fn columnas(tabla: &str) -> Vec<&'static str> {
    let propias: &[&'static str] = match tabla {
        "consultas" => return CONSULTAS.to_vec(),
        "incidencias" => return INCIDENCIAS.to_vec(),
        "registros" => &["record_id", "record_type", "identity_status", "raw_json", "normalized_json"],
        "bitacoras" => &[
            "event_id",
            "event_at",
            "time_precision",
            "time_zone",
            "origin",
            "origin_verified",
            "author",
            "event_type",
            "text",
            "response",
            "response_at",
            "identity_status",
        ],
        "calendarios" => &["calendar_id", "event_at", "event_type", "status", "association", "raw_json", "normalized_json"],
        "historia_tramites" => &[
            "event_id",
            "event_at",
            "time_precision",
            "time_zone",
            "scope",
            "event_type",
            "description",
            "reference",
            "status",
            "document_id",
            "identity_status",
        ],
        _ => &["document_id", "parent_event_id", "scope", "reference", "local_file", "sha256", "download_state", "content_read"],
    };
    COMMON.iter().chain(propias).copied().collect()
}

/// Columnas aditivas al final (`EXTENSIONS` del contrato): solo se escriben si alguna fila las trae.
const EXTENSIONES_HISTORIA: [&str; 4] = ["folio", "stage", "annex_visible", "raw_json"];

type Fila = HashMap<&'static str, String>;

fn fila(pares: &[(&'static str, String)]) -> Fila {
    pares.iter().cloned().collect()
}

/// Un lote de Flow Descargador y su carpeta.
pub struct Lote<'a> {
    pub carpeta: &'a Path,
    pub resumen: &'a Resumen,
}

#[derive(Debug, Clone)]
pub struct Opciones {
    /// Avisos del flujo (código, mensaje) que viajan en el paquete, como `avisos` de `export_capture`.
    pub avisos: Vec<(String, String)>,
    /// Incluir la historia de la causa leída en el lote (`historia_en_paquete`).
    pub historia: bool,
}

impl Default for Opciones {
    fn default() -> Self {
        Self { avisos: Vec::new(), historia: true }
    }
}

#[derive(Debug)]
pub struct Producto {
    pub datos: Vec<u8>,
    pub capture_id: String,
    /// Avisos para el usuario (p. ej. la historia no pudo incluirse).
    pub avisos: Vec<String>,
    /// Mensaje de `BITACORAS_REUTILIZADAS` o «Sin bitácoras añadidas.», como `package_current_lot`.
    pub bitacoras: String,
    /// Tablas que no quedaron completas (estado distinto de COMPLETA, VACIA_COMPROBADA o NO_SOLICITADA).
    pub incompletas: Vec<String>,
}

/// `nurus.capture.coverage.aggregate`.
pub fn agregar<'a>(estados: impl IntoIterator<Item = &'a str>) -> &'a str {
    let valores: Vec<&str> = estados.into_iter().collect();
    if valores.is_empty() {
        return "NO_SOLICITADA";
    }
    if valores.iter().all(|v| *v == "VACIA_COMPROBADA") {
        return "VACIA_COMPROBADA";
    }
    if valores.iter().all(|v| matches!(*v, "COMPLETA" | "VACIA_COMPROBADA")) {
        return "COMPLETA";
    }
    if valores.iter().all(|v| *v == valores[0]) {
        return valores[0];
    }
    "PARCIAL"
}

/// Instante RFC 3339 con segundos enteros (el contrato admite hasta 6 decimales).
fn instante(texto: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(texto).ok().map(|f| f.format("%Y-%m-%dT%H:%M:%S%:z").to_string())
}

/// `literal` de `nurus.capture.historias`: fecha ISO, número como texto.
fn literal(celda: &Celda) -> String {
    match celda {
        Celda::Fecha(f) => f.format("%Y-%m-%d").to_string(),
        otra => otra.texto(),
    }
}

/// `resultados.sin_parentesis`: el NOMBRE del paquete no lleva lo escrito entre paréntesis.
fn sin_parentesis(texto: &str) -> String {
    static GRUPO: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\([^()]*\)").unwrap());
    let limpio = GRUPO.replace_all(texto, " ").replace(['(', ')'], " ");
    limpio.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn texto<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}

/// Cuándo empezó el lote: `inicio` del resumen; en lotes anteriores, el nombre de su carpeta o su primera lectura.
fn inicio(lote: &Lote) -> String {
    if let Some(i) = instante(&lote.resumen.inicio) {
        return i;
    }
    let nombre = lote.carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if let Some(resto) = nombre.strip_prefix("Flow Captura ") {
        if let Ok(f) = chrono::NaiveDateTime::parse_from_str(resto.get(..19).unwrap_or(""), "%Y-%m-%d %H-%M-%S") {
            if let Some(local) = f.and_local_timezone(chrono::Local).earliest() {
                return local.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            }
        }
    }
    if let Some(primera) = lote.resumen.lecturas.iter().filter_map(|l| instante(&l.capturada)).min() {
        return primera;
    }
    let modificado = std::fs::metadata(lote.carpeta.join("resumen.json")).and_then(|m| m.modified()).ok();
    let local: chrono::DateTime<chrono::Local> = modificado.map(Into::into).unwrap_or_else(chrono::Local::now);
    local.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

static PENDIENTE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\AConsulta (\d+), página (\d+), fila (\d+): (bitacora|historia|calendario) pendiente\. ?(.*)\z").unwrap()
});

/// Lecturas por ingreso que fallaron: (tipo, vínculo, motivo), desde los avisos del avance y sus vínculos.
fn pendientes(resumen: &Resumen) -> Vec<(String, Vinculo, String)> {
    let mut salida = Vec::new();
    for aviso in &resumen.avisos {
        let Some(c) = PENDIENTE.captures(aviso) else { continue };
        let (Ok(consulta), Ok(pagina), Ok(fila)) = (c[1].parse::<usize>(), c[2].parse::<usize>(), c[3].parse::<usize>()) else {
            continue;
        };
        let vinculo = resumen
            .paginas_lectura
            .iter()
            .filter(|p| p.consulta + 1 == consulta && p.numero == pagina)
            .flat_map(|p| p.vinculos.iter())
            .find(|v| v.fila == fila);
        if let Some(v) = vinculo {
            salida.push((c[4].to_string(), v.clone(), c[5].to_string()));
        }
    }
    salida
}

struct Datos {
    cid: String,
    captured: String,
    consultas: Vec<Fila>,
    registros: Vec<Fila>,
    /// Clave de `asociar_calendarios` de cada registro (tribunal, RIT, nombre, programa normalizados).
    claves: Vec<Option<[String; 4]>>,
    /// Calendario de informes por vencer y el índice de su registro.
    calendarios: Option<Vec<(Fila, Option<usize>)>>,
    bitacoras: Option<Vec<Fila>>,
    historia: Option<Vec<Fila>>,
    incidencias: Vec<Fila>,
    siguiente: usize,
}

struct Consulta<'a> {
    qid: String,
    filtros: Value,
    screen: &'a str,
    tab: &'a str,
    court_code: &'a str,
    court_name: &'a str,
    modality_code: &'a str,
    coverage: &'a str,
    when: &'a str,
    expected: usize,
    saved: usize,
    error: &'a str,
}

impl Datos {
    fn incidencia(&mut self, qid: &str, codigo: &str, mensaje: &str, rid: &str, severidad: &str) {
        let id = format!("i{}", self.siguiente);
        self.siguiente += 1;
        self.incidencias.push(fila(&[
            ("capture_id", self.cid.clone()),
            ("query_id", qid.into()),
            ("record_id", rid.into()),
            ("incident_id", id),
            ("phase", "EXPORTAR".into()),
            ("code", codigo.into()),
            ("severity", severidad.into()),
            ("message", mensaje.into()),
            ("recoverable", "true".into()),
        ]));
    }

    fn aviso(&mut self, qid: &str, codigo: &str, mensaje: &str) {
        self.incidencia(qid, codigo, mensaje, "", "ADVERTENCIA");
    }

    fn consulta(&mut self, c: Consulta) -> usize {
        let mut f: Fila = CONSULTAS.iter().map(|k| (*k, String::new())).collect();
        for (k, v) in [
            ("query_id", c.qid.as_str()),
            ("source_system", "RUS"),
            ("screen", c.screen),
            ("tab", c.tab),
            ("court_code", c.court_code),
            ("court_name", c.court_name),
            ("modality_code", c.modality_code),
            ("coverage", c.coverage),
            ("captured_at", c.when),
            ("time_zone", "DESCONOCIDA"),
            ("history_complete", "false"),
            ("institutional_verified", "false"),
            ("error", c.error),
        ] {
            f.insert(k, v.to_string());
        }
        f.insert("filters_json", c.filtros.to_string());
        f.insert("expected_pages", c.expected.to_string());
        f.insert("saved_pages", c.saved.to_string());
        self.consultas.push(f);
        self.consultas.len() - 1
    }

    fn comun(&self, q: &Fila) -> Fila {
        let mut f: Fila = COMMON.iter().map(|k| (*k, String::new())).collect();
        f.insert("capture_id", self.cid.clone());
        for k in ["query_id", "source_system", "court_code", "court_name", "modality_code", "tab", "captured_at", "coverage"] {
            f.insert(k, q[k].clone());
        }
        f
    }
}

fn identidad_remota(tribunal: &str, causa: &str, ingreso: &str) -> Value {
    json!({"source_system":"RUS","court_code":tribunal,"cause_id":causa,"ingreso_id":ingreso})
}

/// Tabla de la descarga según la consulta, con el tipo de registro del contrato (`table_mode`). Como `tables()` del
/// Descargador, una página sin las columnas que pide el motor (p. ej. un calendario sin Derivación ni Tribunal) se lee
/// igual como tabla de personas: el tipo de registro lo da la consulta, no las columnas.
fn tabla_de(libro: &flow_tabular::Libro, s: &Seleccion) -> Result<(flow_tabular::Tabla, String), String> {
    let generica = || flow_tabular::leer_consulta(libro, "EGRESADOS");
    let (estricta, tipo) = match s.screen.as_str() {
        "seguimiento" => match Etapa::desde(&s.tab) {
            Some(etapa) => (flow_tabular::leer_tabla(libro, Some(etapa), None), etapa.nombre().to_string()),
            None => {
                let tipo = if matches!(s.tab.as_str(), "Egresados" | "Egreso") { "EGRESADOS" } else { "OTRO" };
                (generica(), tipo.to_string())
            }
        },
        "calendario_informes" => (flow_tabular::leer_tabla(libro, Some(Etapa::Informes), None), "INFORMES".into()),
        "litigantes" => (flow_tabular::leer_consulta(libro, "BUSQUEDA_LITIGANTES"), "OTRO".into()),
        _ => return Err("Las consultas de medidas por vencer no se llevan al paquete del Asistente.".into()),
    };
    match estricta {
        Ok(t) => Ok((t, tipo)),
        Err(e) => generica().map(|t| (t, tipo)).map_err(|_| e),
    }
}

/// Vencimiento de una fila de calendario de informes (`calendar_date`), aunque la tabla se haya leído como genérica.
fn vencimiento(tabla: &flow_tabular::Tabla, f: &flow_tabular::Fila) -> Option<chrono::NaiveDate> {
    f.registro.fecha("vencimiento").or_else(|| {
        let i = *flow_tabular::columnas::mapear(&tabla.cabeceras, Etapa::Informes).ok()?.get("vencimiento")?;
        f.celdas.get(i)?.fecha()
    })
}

/// Registros (y calendario de informes) de un lote.
fn registros(d: &mut Datos, li: usize, lote: &Lote) -> Result<(), String> {
    let r = lote.resumen;
    let cuando = inicio(lote);
    let vinculos = Vinculos::nuevo(r);
    let mut indices: HashMap<usize, usize> = HashMap::new();
    for (qi, s) in r.lote.consultas.iter().enumerate() {
        let xls: Vec<_> = r.archivos.iter().filter(|a| a.consulta == qi && a.nombre.ends_with(".xls")).collect();
        let esperadas = xls.first().map(|a| a.total).unwrap_or(0);
        let estado = if r.vacias.contains(&qi) {
            "VACIA_COMPROBADA"
        } else if r.terminadas.contains(&qi) && xls.len() == esperadas {
            "COMPLETA"
        } else if !xls.is_empty() {
            "PARCIAL"
        } else {
            "NO_CONSULTADA"
        };
        let nombre = r.tribunales.get(&s.tribunal).cloned().unwrap_or_default();
        let mut filtros = serde_json::to_value(s).map_err(|_| "No se pudo describir una consulta del lote.")?;
        if !nombre.is_empty() {
            filtros["tribunal_nombre"] = json!(nombre);
        }
        let pendiente = estado == "NO_CONSULTADA";
        let qid = if pendiente { format!("q{li}-pendiente-{qi}") } else { format!("q{li}-{qi}") };
        let (esperadas, guardadas) = if pendiente || estado == "VACIA_COMPROBADA" { (0, 0) } else { (esperadas, xls.len()) };
        let when = if pendiente { d.captured.clone() } else { cuando.clone() };
        let i = d.consulta(Consulta {
            qid: qid.clone(),
            filtros,
            screen: &s.screen,
            tab: &s.tab,
            court_code: &s.tribunal,
            court_name: &nombre,
            modality_code: &s.modality,
            coverage: estado,
            when: &when,
            expected: esperadas,
            saved: guardadas,
            error: "",
        });
        indices.insert(qi, i);
        match estado {
            "PARCIAL" => d.aviso(&qid, "CAPTURA_PARCIAL", "La consulta no tiene cobertura completa."),
            "NO_CONSULTADA" => d.aviso(&qid, "NO_CONSULTADA", "Consulta planificada no completada."),
            "VACIA_COMPROBADA" if !r.archivos.iter().any(|a| a.consulta == qi && a.nombre.ends_with(".pdf")) => {
                // La respuesta de SITFA comprobó cero registros; no se guardó pantallazo PDF.
                d.incidencia(&qid, "VACIA_SIN_PDF", "Consulta sin registros; no se pidió pantallazo PDF.", "", "INFORMACION")
            }
            _ => {}
        }
    }
    for archivo in r.archivos.iter().filter(|a| a.nombre.ends_with(".xls")) {
        let s = r.lote.consultas.get(archivo.consulta).ok_or("La descarga no identifica su consulta.")?;
        let q = d.consultas[indices[&archivo.consulta]].clone();
        let ruta = ruta_archivo(lote.carpeta, &archivo.nombre)?;
        if std::fs::metadata(&ruta).map_err(|_| "Falta una descarga del lote.")?.len() > 80 * 1024 * 1024 {
            return Err("Una descarga excede el tamaño permitido.".into());
        }
        let datos = std::fs::read(&ruta).map_err(|_| "No se pudo leer una descarga del lote.")?;
        if huella(&datos) != archivo.sha256 {
            return Err("Cambió una descarga desde su verificación; no se creó el paquete.".into());
        }
        let libro = flow_tabular::leer_bytes(&archivo.nombre, datos)
            .map_err(|e| format!("Consulta {}, página {}: {e}", archivo.consulta + 1, archivo.pagina))?;
        let (tabla, tipo) =
            tabla_de(&libro, s).map_err(|e| format!("Consulta {}, página {}: {e}", archivo.consulta + 1, archivo.pagina))?;
        let seguimiento = s.screen == "seguimiento";
        let modalidad = MODALIDADES.iter().find(|(k, _)| *k == s.modality).map(|(_, v)| *v);
        let tribunal_nombre = r.tribunales.get(&s.tribunal).cloned().unwrap_or_default();
        let columna_modalidad = tabla.cabeceras.iter().any(|c| matches!(vinculos::normalizar(c).as_str(), "MODALIDAD" | "TIPO PROGRAMA"));
        let columna_nombre = tabla.mapeo.get("nombre").copied();
        for f in &tabla.filas {
            let mut raw = Map::new();
            let mut normalizado = Map::new();
            for (i, (cabecera, celda)) in tabla.cabeceras.iter().zip(&f.celdas).enumerate() {
                if cabecera.trim().is_empty() {
                    continue;
                }
                let valor = literal(celda);
                let limpio = if Some(i) == columna_nombre { sin_parentesis(&valor) } else { valor.clone() };
                raw.insert(cabecera.clone(), json!(valor));
                normalizado.insert(cabecera.clone(), json!(limpio));
            }
            if !tabla.mapeo.contains_key("tribunal") && !tribunal_nombre.is_empty() {
                normalizado.insert("TRIBUNAL".into(), json!(tribunal_nombre));
            }
            if let (true, Some(m)) = (seguimiento, modalidad) {
                normalizado.insert(if columna_modalidad { "SITFA_MODALIDAD" } else { "MODALIDAD" }.into(), json!(m));
            }
            if !s.tribunal.is_empty() {
                normalizado.insert("SITFA_TRIBUNAL_CODIGO".into(), json!(s.tribunal));
            }
            let identidad = vinculos::identidad(
                &f.registro.texto("rit"),
                &f.registro.texto("rut"),
                &f.registro.texto("nombre"),
                &f.registro.texto("programa"),
            );
            let remoto = if seguimiento { vinculos.ingreso(archivo.consulta, &s.tribunal, &identidad) } else { None };
            if seguimiento {
                let argumento = |i: usize| remoto.and_then(|v| v.calendario.as_ref()).and_then(|a| a.get(i).cloned()).unwrap_or_default();
                let estado = if remoto.is_some() {
                    "Vinculado desde respuesta actual"
                } else if vinculos.alguno(archivo.consulta, &s.tribunal, &identidad) {
                    "Identidad ambigua"
                } else {
                    "Sin vínculo comprobado"
                };
                for (k, v) in [
                    ("SITFA_CAUSA_RUS", remoto.map(|v| v.causa.clone()).unwrap_or_default()),
                    ("SITFA_INGRESO_RUS", remoto.map(|v| v.ingreso.clone()).unwrap_or_default()),
                    ("SITFA_ETAPA_RUS", remoto.map(|v| v.etapa.clone()).unwrap_or_default()),
                    ("SITFA_PERSONA_RUS", argumento(1)),
                    ("SITFA_CENTRO_RUS", argumento(3)),
                    ("SITFA_ANTIGUO_RUS", remoto.map(|v| v.antiguo.clone()).unwrap_or_default()),
                    ("SITFA_VINCULO_ESTADO", estado.to_string()),
                ] {
                    normalizado.insert(k.into(), json!(v));
                }
            }
            let campo = |k: &str| normalizado.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            let mut comun = d.comun(&q);
            for (k, v) in [
                ("rit_original", campo("RIT")),
                ("cause_id", campo("SITFA_CAUSA_RUS")),
                ("ingreso_id", campo("SITFA_INGRESO_RUS")),
                ("person_id", campo("SITFA_PERSONA_RUS")),
                ("program_id", campo("SITFA_CENTRO_RUS")),
                ("modality_raw", campo("MODALIDAD")),
                ("original_file", archivo.nombre.clone()),
                ("original_row", f.numero.to_string()),
                ("source_sha256", archivo.sha256.clone()),
            ] {
                comun.insert(k, v);
            }
            let rid = huella(format!("{}|{}|{}", q["query_id"], archivo.sha256, f.numero).as_bytes())[..24].to_string();
            let verificada = !comun["court_code"].is_empty() && !comun["cause_id"].is_empty() && !comun["ingreso_id"].is_empty();
            let raw = Value::Object(raw).to_string();
            let normalizado_json = Value::Object(normalizado.clone()).to_string();
            let mut registro = comun.clone();
            registro.insert("record_id", rid.clone());
            registro.insert("record_type", tipo.clone());
            registro.insert("identity_status", if verificada { "VERIFICADA" } else { "INCIERTA" }.into());
            registro.insert("raw_json", raw.clone());
            registro.insert("normalized_json", normalizado_json.clone());
            d.registros.push(registro);
            let partes = [f.registro.texto("rit"), sin_parentesis(&f.registro.texto("nombre")), f.registro.texto("programa")]
                .map(|p| vinculos::normalizar(&p));
            d.claves.push(
                partes
                    .iter()
                    .all(|p| !p.is_empty())
                    .then(|| [comun["court_code"].clone(), partes[0].clone(), partes[1].clone(), partes[2].clone()]),
            );
            if !verificada {
                d.incidencia(
                    &q["query_id"],
                    "IDENTIDAD_INCIERTA",
                    "No hay ingreso remoto verificable; no se relacionan eventos por RIT/nombre.",
                    &rid,
                    "ADVERTENCIA",
                );
            }
            if s.screen == "calendario_informes" {
                let mut calendario = comun;
                calendario.insert("calendar_id", format!("cal-{rid}"));
                calendario.insert("event_at", vencimiento(&tabla, f).map(|v| v.to_string()).unwrap_or_default());
                calendario.insert("event_type", "INFORME".into());
                calendario.insert("status", "SEGUN_FUENTE".into());
                calendario.insert("association", if verificada { "VERIFICADA" } else { "NO_ASOCIADO" }.into());
                calendario.insert("raw_json", raw);
                calendario.insert("normalized_json", normalizado_json);
                let indice = d.registros.len() - 1;
                d.calendarios.get_or_insert_with(Vec::new).push((calendario, Some(indice)));
            }
        }
    }
    Ok(())
}

/// `asociar_calendarios`: un calendario de informes se asocia solo si tribunal, RIT, nombre y programa coinciden con
/// exactamente un ingreso verificado de Espera o Cumplimiento (nunca por RIT solo ni con empate).
fn asociar_calendarios(d: &mut Datos) -> BTreeSet<String> {
    let mut ingresos: HashMap<[String; 4], BTreeSet<[String; 4]>> = HashMap::new();
    for (r, clave) in d.registros.iter().zip(&d.claves) {
        if r["identity_status"] != "VERIFICADA" || matches!(r["record_type"].as_str(), "INFORMES" | "OTRO" | "EGRESADOS") {
            continue;
        }
        if let Some(k) = clave {
            ingresos.entry(k.clone()).or_default().insert(["cause_id", "ingreso_id", "person_id", "program_id"].map(|c| r[c].clone()));
        }
    }
    let mut resueltos = BTreeSet::new();
    let Some(calendarios) = d.calendarios.as_mut() else { return resueltos };
    for (calendario, registro) in calendarios.iter_mut() {
        if calendario["association"] == "VERIFICADA" {
            continue;
        }
        let Some(indice) = *registro else { continue };
        let Some(candidatos) = d.claves[indice].as_ref().and_then(|k| ingresos.get(k)) else { continue };
        if candidatos.len() != 1 {
            continue;
        }
        let ids = candidatos.iter().next().unwrap().clone();
        for (c, v) in ["cause_id", "ingreso_id", "person_id", "program_id"].iter().zip(&ids) {
            calendario.insert(c, v.clone());
            d.registros[indice].insert(c, v.clone());
        }
        calendario.insert("association", "VERIFICADA".into());
        d.registros[indice].insert("identity_status", "VERIFICADA".into());
        resueltos.insert(d.registros[indice]["record_id"].clone());
    }
    resueltos
}

struct Lectura {
    referencia: Referencia,
    datos: Value,
}

fn leer_lecturas(lotes: &[Lote], tipo: &str) -> Result<Vec<Lectura>, String> {
    let mut salida = Vec::new();
    for lote in lotes {
        for r in lote.resumen.lecturas.iter().filter(|l| l.tipo == tipo) {
            let valor = recoleccion::leer(lote.carpeta, r)?;
            salida.push(Lectura { referencia: r.clone(), datos: valor["datos"].clone() });
        }
    }
    Ok(salida)
}

/// Bitácoras y calendarios por ingreso de las identidades verificadas del paquete (`match_diaries=True`).
fn bitacoras(d: &mut Datos, lotes: &[Lote]) -> Result<(), String> {
    let identidades: BTreeSet<[String; 3]> = d
        .registros
        .iter()
        .filter(|r| r["identity_status"] == "VERIFICADA")
        .map(|r| ["court_code", "cause_id", "ingreso_id"].map(|k| r[k].clone()))
        .collect();
    // Por ingreso (tribunal, causa, ingreso), como `import_lotes`: varias etapas o páginas del mismo ingreso se unen.
    let mut instantaneas: BTreeMap<[String; 3], (Option<Lectura>, String)> = BTreeMap::new();
    for l in leer_lecturas(lotes, "bitacora")? {
        let clave = [l.referencia.tribunal.clone(), l.referencia.causa.clone(), l.referencia.ingreso.clone()];
        match instantaneas.get_mut(&clave) {
            Some((Some(previa), _)) => {
                let vistas: BTreeSet<String> =
                    previa.datos["entries"].as_array().into_iter().flatten().map(|e| texto(e, "entry_id").to_string()).collect();
                let nuevas: Vec<Value> = l.datos["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|e| !vistas.contains(texto(e, "entry_id")))
                    .cloned()
                    .collect();
                if let Some(lista) = previa.datos["entries"].as_array_mut() {
                    lista.extend(nuevas);
                }
            }
            _ => {
                instantaneas.insert(clave, (Some(l), String::new()));
            }
        }
    }
    let mut calendarios: BTreeMap<[String; 3], (Option<Lectura>, String)> = BTreeMap::new();
    for l in leer_lecturas(lotes, "calendario")? {
        let clave = [l.referencia.tribunal.clone(), l.referencia.causa.clone(), l.referencia.ingreso.clone()];
        // Se conserva la lectura más reciente.
        if calendarios.get(&clave).and_then(|(p, _)| p.as_ref()).is_none_or(|p| p.referencia.capturada <= l.referencia.capturada) {
            calendarios.insert(clave, (Some(l), String::new()));
        }
    }
    for lote in lotes {
        for (tipo, v, motivo) in pendientes(lote.resumen) {
            let clave = [v.tribunal.clone(), v.causa.clone(), v.ingreso.clone()];
            if clave.iter().any(String::is_empty) {
                continue;
            }
            let destino = match tipo.as_str() {
                "bitacora" => &mut instantaneas,
                "calendario" => &mut calendarios,
                _ => continue,
            };
            destino.entry(clave).or_insert((None, if motivo.is_empty() { "No se completó la lectura.".into() } else { motivo }));
        }
    }
    let total = instantaneas.len();
    instantaneas.retain(|k, _| identidades.contains(k));
    calendarios.retain(|k, _| identidades.contains(k));
    let primera = d.consultas.first().map(|q| q["query_id"].clone()).unwrap_or_default();
    d.aviso(
        &primera,
        "BITACORAS_REUTILIZADAS",
        &format!(
            "Bitácoras conservadas por tribunal/causa/ingreso: {}; ajenas excluidas: {}. Se conservan fecha y cobertura originales; no se acredita actualización remota.",
            instantaneas.len(),
            total - instantaneas.len()
        ),
    );
    if instantaneas.is_empty() {
        let captured = d.captured.clone();
        d.consulta(Consulta {
            qid: "diary-sin-coincidencia".into(),
            filtros: json!({"capture_tables":["bitacoras"]}),
            screen: "bitacoras",
            tab: "",
            court_code: "",
            court_name: "",
            modality_code: "",
            coverage: "IDENTIDAD_INCIERTA",
            when: &captured,
            expected: 0,
            saved: 0,
            error: "Las bitácoras disponibles no tienen correspondencia remota verificada con los registros seleccionados.",
        });
    }
    let mut filas = Vec::new();
    for (qi, ([tribunal, causa, ingreso], (lectura, motivo))) in instantaneas.into_iter().enumerate() {
        let qid = format!("diary-{qi}");
        let filtros = json!({"tribunal":tribunal,"tab":"BITACORAS","capture_tables":["bitacoras"],
            "remote_identity":identidad_remota(&tribunal, &causa, &ingreso)});
        let cobertura = lectura.as_ref().map(|l| texto(&l.datos, "coverage")).filter(|c| !c.is_empty()).unwrap_or("FALLIDA").to_string();
        let cuando = lectura
            .as_ref()
            .and_then(|l| instante(texto(&l.datos, "captured_at")).or_else(|| instante(&l.referencia.capturada)))
            .unwrap_or_else(|| d.captured.clone());
        let i = d.consulta(Consulta {
            qid: qid.clone(),
            filtros,
            screen: "bitacoras",
            tab: "BITACORAS",
            court_code: &tribunal,
            court_name: "",
            modality_code: "",
            coverage: &cobertura,
            when: &cuando,
            expected: 1,
            saved: 1,
            error: "",
        });
        let Some(lectura) = lectura else {
            d.aviso(&qid, "BITACORA_FALLIDA", &motivo);
            continue;
        };
        for aviso in lectura.datos["warnings"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            d.aviso(&qid, "BITACORA_ADVERTENCIA", aviso);
        }
        let q = d.consultas[i].clone();
        let sha = Some(texto(&lectura.datos, "html_sha256")).filter(|s| s.len() == 64).unwrap_or(&lectura.referencia.sha256).to_string();
        for e in lectura.datos["entries"].as_array().into_iter().flatten() {
            let mut f = d.comun(&q);
            let fuente = Some(texto(e, "fuente")).filter(|s| !s.is_empty()).unwrap_or(&lectura.referencia.archivo).to_string();
            let cuando = texto(e, "fecha");
            let solo_dia = cuando.ends_with("T00:00:00");
            for (k, v) in [
                ("rit_original", texto(e, "rit").to_string()),
                ("cause_id", causa.clone()),
                ("ingreso_id", ingreso.clone()),
                ("original_file", fuente),
                ("original_sheet", "TablaInforme".into()),
                ("original_row", e["fila_fuente"].as_u64().unwrap_or(1).max(1).to_string()),
                ("source_sha256", sha.clone()),
                ("event_id", format!("{qid}-{}", texto(e, "entry_id"))),
                ("event_at", if solo_dia { cuando.get(..10).unwrap_or("").to_string() } else { cuando.to_string() }),
                ("time_precision", if solo_dia { "DIA" } else { "SEGUNDO" }.into()),
                ("time_zone", "DESCONOCIDA".into()),
                ("origin", "CENTRO".into()),
                ("origin_verified", "true".into()),
                ("author", texto(e, "autor").into()),
                ("event_type", texto(e, "tipo").into()),
                ("text", texto(e, "texto").into()),
                ("response", texto(e, "respuesta").into()),
                ("response_at", texto(e, "fecha_respuesta").into()),
                ("identity_status", "VERIFICADA".into()),
            ] {
                f.insert(k, v);
            }
            filas.push(f);
        }
    }
    d.bitacoras = Some(filas);
    for (ci, ([tribunal, causa, ingreso], (lectura, motivo))) in calendarios.into_iter().enumerate() {
        let qid = format!("calendar-{ci}");
        let filtros = json!({"tribunal":tribunal,"tab":"CALENDARIO","capture_tables":["calendarios"],
            "remote_identity":identidad_remota(&tribunal, &causa, &ingreso)});
        let cuando = lectura
            .as_ref()
            .and_then(|l| instante(texto(&l.datos, "captured_at")).or_else(|| instante(&l.referencia.capturada)))
            .unwrap_or_else(|| d.captured.clone());
        let i = d.consulta(Consulta {
            qid: qid.clone(),
            filtros,
            screen: "calendario_ingreso",
            tab: "CALENDARIO",
            court_code: &tribunal,
            court_name: "",
            modality_code: "",
            coverage: if lectura.is_some() { "COMPLETA" } else { "FALLIDA" },
            when: &cuando,
            expected: 1,
            saved: usize::from(lectura.is_some()),
            error: if lectura.is_some() { "" } else { &motivo },
        });
        let Some(lectura) = lectura else {
            d.aviso(&qid, "CALENDARIO_FALLIDO", &motivo);
            continue;
        };
        let q = d.consultas[i].clone();
        let sha = Some(texto(&lectura.datos, "html_sha256")).filter(|s| s.len() == 64).unwrap_or(&lectura.referencia.sha256).to_string();
        let lista = d.calendarios.get_or_insert_with(Vec::new);
        for p in lectura.datos["proyecciones"].as_array().into_iter().flatten() {
            let mut valores = p.as_object().cloned().unwrap_or_default();
            for k in ["plazo_remision_meses", "plazo_intervencion_meses"] {
                valores.insert(k.into(), lectura.datos[k].clone());
            }
            let valores = Value::Object(valores).to_string();
            let numero = p["fila"].as_u64().unwrap_or(1).max(1);
            let mut f = HashMap::new();
            for k in COMMON {
                f.insert(k, q.get(k).cloned().unwrap_or_default());
            }
            for (k, v) in [
                ("capture_id", d.cid.clone()),
                ("rit_original", String::new()),
                ("cause_id", causa.clone()),
                ("ingreso_id", ingreso.clone()),
                ("original_file", lectura.referencia.archivo.clone()),
                ("original_sheet", "Calendario".into()),
                ("original_row", numero.to_string()),
                ("source_sha256", sha.clone()),
                ("coverage", "COMPLETA".into()),
                ("calendar_id", format!("{qid}-{numero}")),
                ("event_at", texto(p, "fecha_proyeccion").into()),
                ("event_type", "INFORME_PROYECTADO".into()),
                ("status", texto(p, "estado").into()),
                ("association", "VERIFICADA".into()),
                ("raw_json", valores.clone()),
                ("normalized_json", valores),
            ] {
                f.insert(k, v);
            }
            lista.push((f, None));
        }
    }
    Ok(())
}

/// Historia de la causa: una consulta por causa (alcance CAUSA, sin `remote_identity`) y una fila por trámite de la
/// ventana, más el último trámite anterior de cada nomenclatura de contexto (`contexto_anterior`).
fn historia(d: &mut Datos, lotes: &[Lote]) -> Result<(), String> {
    let mut causas: BTreeMap<[String; 2], (Option<Lectura>, Vinculo, String)> = BTreeMap::new();
    let mut origen: HashMap<[String; 2], (String, String)> = HashMap::new();
    for lote in lotes {
        for p in &lote.resumen.paginas_lectura {
            let s = &lote.resumen.lote.consultas[p.consulta];
            for v in p.vinculos.iter().filter(|v| v.historia_verificada) {
                origen.entry([v.tribunal.clone(), v.causa.clone()]).or_insert_with(|| (s.modality.clone(), s.tab.clone()));
            }
        }
    }
    let mut nombres: HashMap<String, String> = HashMap::new();
    for lote in lotes {
        nombres.extend(lote.resumen.tribunales.iter().map(|(k, v)| (k.clone(), v.clone())));
        for r in lote.resumen.lecturas.iter().filter(|l| l.tipo == "historia") {
            let valor = recoleccion::leer(lote.carpeta, r)?;
            let v: Vinculo =
                serde_json::from_value(valor["vinculo"].clone()).map_err(|_| "El vínculo de una historia guardada no se reconoce.")?;
            causas.insert(
                [r.tribunal.clone(), r.causa.clone()],
                (Some(Lectura { referencia: r.clone(), datos: valor["datos"].clone() }), v, String::new()),
            );
        }
        for (tipo, v, motivo) in pendientes(lote.resumen) {
            if tipo == "historia" && !v.causa.is_empty() {
                let motivo = if motivo.is_empty() { "No se pudo leer la historia.".to_string() } else { motivo };
                causas.entry([v.tribunal.clone(), v.causa.clone()]).or_insert((None, v, motivo));
            }
        }
    }
    let mut filas = Vec::new();
    if causas.is_empty() {
        let captured = d.captured.clone();
        d.consulta(Consulta {
            qid: "hist-listado-vacio".into(),
            filtros: json!({"tab":"HISTORIA","capture_tables":["historia_tramites"]}),
            screen: "historia_tramites",
            tab: "",
            court_code: "",
            court_name: "",
            modality_code: "",
            coverage: "VACIA_COMPROBADA",
            when: &captured,
            expected: 0,
            saved: 0,
            error: "",
        });
    }
    for (n, ([tribunal, causa], (lectura, v, motivo))) in causas.into_iter().enumerate() {
        let qid = format!("hist-{}", n + 1);
        let (modalidad, pestana) = origen.get(&[tribunal.clone(), causa.clone()]).cloned().unwrap_or_default();
        let nombre = nombres.get(&tribunal).cloned().unwrap_or_default();
        let datos = lectura.as_ref().map(|l| &l.datos);
        let filas_ventana: Vec<Value> = datos.and_then(|x| x["filas"].as_array()).cloned().unwrap_or_default();
        let contexto: Vec<Value> = datos.and_then(|x| x["contexto_anterior"].as_array()).cloned().unwrap_or_default();
        let cobertura = match datos {
            None => "FALLIDA",
            Some(x) if x["vacio_explicito"] == true && filas_ventana.is_empty() && contexto.is_empty() => "VACIA_COMPROBADA",
            Some(_) => "COMPLETA",
        };
        let meses = lectura.as_ref().map(|l| l.referencia.meses).unwrap_or(4);
        let filtros = json!({"tribunal":tribunal,"tribunal_nombre":nombre,"modality":modalidad,"tab":"HISTORIA",
            "capture_tables":["historia_tramites"],
            "historia_causa":{"source_system":"RUS","court_code":tribunal,"cause_id":causa},
            "ventana":{"criterio":format!("ventana_{meses}_meses"),"desde":datos.map(|x| texto(x, "desde")).unwrap_or(""),
                       "hasta":datos.map(|x| texto(x, "hasta")).unwrap_or("")}});
        let cuando = lectura.as_ref().and_then(|l| instante(&l.referencia.capturada)).unwrap_or_else(|| d.captured.clone());
        let ok = cobertura != "FALLIDA";
        let i = d.consulta(Consulta {
            qid: qid.clone(),
            filtros,
            screen: "historia_tramites",
            tab: "HISTORIA",
            court_code: &tribunal,
            court_name: &nombre,
            modality_code: &modalidad,
            coverage: cobertura,
            when: &cuando,
            expected: 1,
            saved: usize::from(ok),
            error: if ok { "" } else { &motivo },
        });
        let Some(lectura) = lectura else {
            d.aviso(&qid, "HISTORIA_FALLIDA", &motivo);
            continue;
        };
        for a in lectura.datos["advertencias"].as_array().into_iter().flatten() {
            d.aviso(&qid, texto(a, "codigo"), "Advertencia del lector de historia.");
        }
        let q = d.consultas[i].clone();
        let sha = Some(texto(&lectura.datos, "sha256")).filter(|s| s.len() == 64).unwrap_or(&lectura.referencia.sha256).to_string();
        let mut ventana = filas_ventana;
        ventana.sort_by_key(|f| f["fila"].as_u64().unwrap_or(u64::MAX));
        for t in ventana.iter().chain(&contexto) {
            let numero = t["fila"].as_u64().unwrap_or(1).max(1);
            let fecha = texto(t, "fecha_iso");
            let mut f = d.comun(&q);
            let crudo = json!({"Folio":texto(t,"folio"),"Etapa":texto(t,"etapa"),"Trámite":texto(t,"tramite"),
                "Desc. Trámite":texto(t,"descripcion"),"Referencia":texto(t,"referencia"),"Fecha Tram":texto(t,"fecha_texto")});
            for (k, val) in [
                ("rit_original", v.rit.clone()),
                ("cause_id", causa.clone()),
                ("tab", pestana.clone()),
                ("original_file", lectura.referencia.archivo.clone()),
                ("original_row", numero.to_string()),
                ("source_sha256", sha.clone()),
                ("coverage", "COMPLETA".into()),
                ("event_id", format!("{qid}-{numero}")),
                ("event_at", fecha.into()),
                ("time_precision", if fecha.is_empty() { "DESCONOCIDA" } else { "DIA" }.into()),
                ("time_zone", "DESCONOCIDA".into()),
                ("scope", "CAUSA".into()),
                ("event_type", "tramite".into()),
                ("description", texto(t, "descripcion").into()),
                ("reference", texto(t, "referencia").into()),
                ("identity_status", "CAUSA".into()),
                ("folio", texto(t, "folio").into()),
                ("stage", texto(t, "etapa").into()),
                ("annex_visible", if t["anexo"] == true { "true" } else { "false" }.into()),
                ("raw_json", crudo.to_string()),
            ] {
                f.insert(k, val);
            }
            filas.push(f);
        }
    }
    d.historia = Some(filas);
    Ok(())
}

fn csv(tabla: &str, filas: &[&Fila]) -> Vec<u8> {
    let mut cols = columnas(tabla);
    if tabla == "historia_tramites" {
        cols.extend(EXTENSIONES_HISTORIA.iter().filter(|c| filas.iter().any(|f| f.contains_key(*c))));
    }
    let campo = |s: &str| {
        if s.contains([',', '"', '\n', '\r']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    let mut salida = cols.join(",");
    salida.push('\n');
    for f in filas {
        salida.push_str(&cols.iter().map(|c| campo(f.get(c).map(String::as_str).unwrap_or(""))).collect::<Vec<_>>().join(","));
        salida.push('\n');
    }
    salida.into_bytes()
}

fn construir_con(lotes: &[Lote], opciones: &Opciones, con_historia: bool) -> Result<Producto, String> {
    let primero = lotes.first().ok_or("Selecciona un lote verificado.")?;
    for l in lotes {
        if l.resumen.estado != "VALIDADA" {
            return Err("El lote está incompleto: usa «Retomar lote…» antes de crear el paquete.".into());
        }
    }
    let captured = inicio(primero);
    let lecturas = |f: fn(&flow_captura::recoleccion::Opciones) -> bool| lotes.iter().any(|l| f(&l.resumen.lote.lecturas));
    let con_bitacoras = lecturas(|o| o.bitacoras || o.calendario);
    let con_historia = con_historia && lecturas(|o| o.historia);
    let mut huella_lotes = String::new();
    for l in lotes {
        huella_lotes.push_str(&serde_json::to_string(l.resumen).map_err(|_| "No se pudo describir el lote.")?);
    }
    huella_lotes.push_str(if con_bitacoras { "MATCH" } else { "SIN" });
    if con_historia {
        huella_lotes.push_str("HISTORIA");
    }
    let cid = format!("captura-{}", &huella(huella_lotes.as_bytes())[..24]);
    let mut d = Datos {
        cid: cid.clone(),
        captured: captured.clone(),
        consultas: vec![],
        registros: vec![],
        claves: vec![],
        calendarios: None,
        bitacoras: None,
        historia: None,
        incidencias: vec![],
        siguiente: 1,
    };
    for (li, lote) in lotes.iter().enumerate() {
        registros(&mut d, li, lote)?;
    }
    let primera = d.consultas.first().map(|q| q["query_id"].clone()).unwrap_or_default();
    for (codigo, mensaje) in &opciones.avisos {
        d.aviso(&primera, codigo, mensaje);
    }
    let resueltos = asociar_calendarios(&mut d);
    d.incidencias.retain(|i| !(i["code"] == "IDENTIDAD_INCIERTA" && resueltos.contains(&i["record_id"])));
    if con_bitacoras {
        bitacoras(&mut d, lotes)?;
    }
    if con_historia {
        historia(&mut d, lotes)?;
    }
    let pantalla = |q: &Fila| q["screen"].clone();
    let mut estados: BTreeMap<&str, String> = TABLAS.iter().map(|t| (*t, "NO_SOLICITADA".to_string())).collect();
    estados.insert("consultas", agregar(d.consultas.iter().map(|q| q["coverage"].as_str())).into());
    estados.insert(
        "registros",
        agregar(
            d.consultas
                .iter()
                .filter(|q| !matches!(q["screen"].as_str(), "bitacoras" | "calendario_ingreso" | "historia_tramites"))
                .map(|q| q["coverage"].as_str()),
        )
        .into(),
    );
    estados.insert("incidencias", if d.incidencias.is_empty() { "VACIA_COMPROBADA" } else { "COMPLETA" }.into());
    for (tabla, pantallas, presente, vacia) in [
        (
            "calendarios",
            &["calendario_informes", "calendario_ingreso"][..],
            d.calendarios.is_some(),
            d.calendarios.as_ref().is_none_or(Vec::is_empty),
        ),
        ("bitacoras", &["bitacoras"][..], d.bitacoras.is_some(), d.bitacoras.as_ref().is_none_or(Vec::is_empty)),
    ] {
        let relevantes: Vec<String> =
            d.consultas.iter().filter(|q| pantallas.contains(&pantalla(q).as_str())).map(|q| q["coverage"].clone()).collect();
        if presente || !relevantes.is_empty() {
            let estado = agregar(relevantes.iter().map(String::as_str));
            estados.insert(tabla, if vacia && estado == "COMPLETA" { "VACIA_COMPROBADA" } else { estado }.into());
            if tabla == "calendarios" {
                d.calendarios.get_or_insert_with(Vec::new);
            } else {
                d.bitacoras.get_or_insert_with(Vec::new);
            }
        }
    }
    if let Some(h) = &d.historia {
        let estado = agregar(d.consultas.iter().filter(|q| q["screen"] == "historia_tramites").map(|q| q["coverage"].as_str()));
        estados.insert("historia_tramites", if h.is_empty() && estado == "COMPLETA" { "VACIA_COMPROBADA" } else { estado }.into());
    }
    let calendarios: Option<Vec<&Fila>> = d.calendarios.as_ref().map(|c| c.iter().map(|(f, _)| f).collect());
    let mut tablas: Vec<(&str, Vec<&Fila>)> =
        vec![("consultas", d.consultas.iter().collect()), ("registros", d.registros.iter().collect())];
    if let Some(b) = &d.bitacoras {
        tablas.push(("bitacoras", b.iter().collect()));
    }
    if let Some(c) = calendarios {
        tablas.push(("calendarios", c));
    }
    if let Some(h) = &d.historia {
        tablas.push(("historia_tramites", h.iter().collect()));
    }
    tablas.push(("incidencias", d.incidencias.iter().collect()));
    let mut archivos = Vec::new();
    let mut contenido = Vec::new();
    for (tabla, filas) in &tablas {
        let bytes = csv(tabla, filas);
        let ruta = format!("data/{tabla}.csv");
        archivos.push(
            json!({"path":ruta,"table":tabla,"type":"normalized_table","rows":filas.len(),"bytes":bytes.len(),"sha256":huella(&bytes)}),
        );
        contenido.push((ruta, bytes));
    }
    let consultas: Vec<Value> =
        d.consultas.iter().map(|q| Value::Object(CONSULTAS.iter().map(|k| (k.to_string(), json!(q[k]))).collect())).collect();
    let manifiesto = json!({
        "capture_id": cid,
        "captured_at": captured,
        "contains_sensitive_data": true,
        "downloader": {"version": format!("flow-rust {}", env!("CARGO_PKG_VERSION")), "commit": option_env!("FLOW_COMMIT").unwrap_or("flow-rust")},
        "files": archivos,
        "mapping_version": MAPPING_VERSION,
        "originals_authorized": false,
        "queries": consultas,
        "required_features": [],
        "schema_id": "CSMP_CAPTURE",
        "schema_version": VERSION,
        "table_states": estados,
    });
    contenido.push(("manifest.json".into(), serde_json::to_vec_pretty(&manifiesto).map_err(|_| "No se pudo preparar el manifiesto.")?));
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let fecha = zip::DateTime::from_date_and_time(2026, 10, 8, 0, 0, 0).map_err(|_| "Fecha técnica del ZIP inválida.")?;
    let opciones_zip = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(fecha)
        .unix_permissions(0o600);
    for (ruta, bytes) in &contenido {
        zip.start_file(ruta.as_str(), opciones_zip).map_err(|e| format!("No se pudo armar el paquete: {e}"))?;
        zip.write_all(bytes).map_err(|e| format!("No se pudo armar el paquete: {e}"))?;
    }
    let datos = zip.finish().map_err(|e| format!("No se pudo armar el paquete: {e}"))?.into_inner();
    crate::paquete::comprobar(&datos)?;
    let bitacoras = d
        .incidencias
        .iter()
        .find(|i| i["code"] == "BITACORAS_REUTILIZADAS")
        .map(|i| i["message"].clone())
        .unwrap_or_else(|| "Sin bitácoras añadidas.".into());
    let incompletas = estados
        .iter()
        .filter(|(_, v)| !matches!(v.as_str(), "COMPLETA" | "VACIA_COMPROBADA" | "NO_SOLICITADA"))
        .map(|(k, v)| format!("{k}: {v}"))
        .collect();
    Ok(Producto { datos, capture_id: cid, avisos: vec![], bitacoras, incompletas })
}

/// Arma el paquete en memoria. Un fallo de la historia nunca impide el paquete: se crea sin ella y se avisa.
pub fn construir(lotes: &[Lote], opciones: &Opciones) -> Result<Producto, String> {
    match construir_con(lotes, opciones, opciones.historia) {
        Err(e) if opciones.historia && lotes.iter().any(|l| l.resumen.lote.lecturas.historia) => {
            let mut p = construir_con(lotes, opciones, false).map_err(|_| e)?;
            p.avisos.push("La historia no pudo incluirse en el paquete; las bitácoras se conservan.".into());
            Ok(p)
        }
        otro => otro,
    }
}

/// Escribe el paquete en un archivo nuevo; nunca reemplaza otro paquete.
pub fn exportar(lotes: &[Lote], destino: &Path, opciones: &Opciones) -> Result<Producto, String> {
    if destino.exists() {
        return Err("El paquete de destino ya existe; elige un nombre nuevo.".into());
    }
    let producto = construir(lotes, opciones)?;
    let carpeta = destino.parent().filter(|p| !p.as_os_str().is_empty()).map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&carpeta).map_err(|_| "No se pudo crear la carpeta del paquete.")?;
    let temporal = carpeta.join(format!(".capture-{}.zip", &producto.capture_id));
    let _ = std::fs::remove_file(&temporal);
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporal)
            .map_err(|_| "No se pudo crear el paquete. Revisa permisos y espacio.")?;
        if f.write_all(&producto.datos).and_then(|_| f.sync_all()).is_err() {
            drop(f);
            let _ = std::fs::remove_file(&temporal);
            return Err("Falló la escritura del paquete; no se conserva un archivo incompleto.".into());
        }
    }
    let resultado = std::fs::hard_link(&temporal, destino).or_else(|_| {
        if destino.exists() {
            Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists))
        } else {
            std::fs::copy(&temporal, destino).map(|_| ())
        }
    });
    let _ = std::fs::remove_file(&temporal);
    resultado.map_err(|_| "No se pudo publicar el paquete (¿ya existe otro con ese nombre?).")?;
    Ok(producto)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn agregado_como_coverage_py() {
        assert_eq!(agregar([]), "NO_SOLICITADA");
        assert_eq!(agregar(["VACIA_COMPROBADA", "VACIA_COMPROBADA"]), "VACIA_COMPROBADA");
        assert_eq!(agregar(["COMPLETA", "VACIA_COMPROBADA"]), "COMPLETA");
        assert_eq!(agregar(["FALLIDA", "FALLIDA"]), "FALLIDA");
        assert_eq!(agregar(["PARCIAL", "FALLIDA"]), "PARCIAL");
    }

    #[test]
    fn nombres_sin_parentesis_y_csv_con_comillas() {
        assert_eq!(sin_parentesis("Persona (alias) Ficticia ("), "Persona Ficticia");
        let f = fila(&[("query_id", "a,\"b\"\nc".into())]);
        let texto = String::from_utf8(csv("consultas", &[&f])).unwrap();
        assert!(texto.starts_with("query_id,source_system,"));
        assert!(texto.contains("\"a,\"\"b\"\"\nc\""));
        assert_eq!(crate::paquete::leer_csv(&texto)[0]["query_id"], "a,\"b\"\nc");
    }

    #[test]
    fn instante_con_segundos_enteros() {
        assert_eq!(instante("2026-10-20T12:00:00.123456789Z").as_deref(), Some("2026-10-20T12:00:00+00:00"));
        assert_eq!(instante("ilegible"), None);
    }
}
