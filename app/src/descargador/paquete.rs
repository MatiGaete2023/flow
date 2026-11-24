//! E-Q5 · Paquete `CSMP_CAPTURE` desde las carpetas del Descargador, con la salida de Flow Python:
//! `nurus.capture.contract.write_capture`, `resultados.load_lot/tables` y `export_csmp_capture.export_capture`.
//! Lo abre el Asistente (Trabajo) igual que un paquete de Python.
//!
//! E-Q5a: lotes de listados de Seguimiento (consultas, registros e incidencias). E-Q5b: Carga y calendario de informes
//! con su asociación. E-Q5c: bitácoras, calendario por ingreso, historia, Escritos e histórico (`package_current_lot`).

use super::xls;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const VERSION: &str = "1.0.0";
pub const VERSION_EXTENDIDA: &str = "1.2.0";
pub const VERSION_CONSOLIDADA: &str = "1.1.0";
pub const RUTA_CONSOLIDADO: &str = "consolidado/consolidado.xlsx";
pub const MAPPING_VERSION: &str = "baseline-20261008-v1";
const MAX_FILAS: usize = 200_000;

/// `contract.COMMON`.
pub const COMUNES: [&str; 19] = [
    "capture_id", "query_id", "source_system", "court_code", "court_name", "rit_original", "cause_id", "ingreso_id",
    "person_id", "program_id", "modality_raw", "modality_code", "tab", "captured_at", "original_file", "original_sheet",
    "original_row", "source_sha256", "coverage",
];

/// `contract.TABLES` (en el orden de Python) y `EXTENDED_TABLES`.
pub fn columnas(tabla: &str) -> Option<Vec<&'static str>> {
    let con_comunes = |extra: &[&'static str]| COMUNES.iter().copied().chain(extra.iter().copied()).collect::<Vec<_>>();
    Some(match tabla {
        "consultas" => vec![
            "query_id", "source_system", "screen", "tab", "court_code", "court_name", "modality_code", "coverage",
            "captured_at", "filters_json", "expected_pages", "saved_pages", "period_from", "period_to", "time_zone",
            "history_complete", "institutional_verified", "error",
        ],
        "registros" => con_comunes(&["record_id", "record_type", "identity_status", "raw_json", "normalized_json"]),
        "bitacoras" => con_comunes(&[
            "event_id", "event_at", "time_precision", "time_zone", "origin", "origin_verified", "author", "event_type", "text",
            "response", "response_at", "identity_status",
        ]),
        "calendarios" => con_comunes(&["calendar_id", "event_at", "event_type", "status", "association", "raw_json", "normalized_json"]),
        "historia_tramites" => con_comunes(&[
            "event_id", "event_at", "time_precision", "time_zone", "scope", "event_type", "description", "reference", "status",
            "document_id", "identity_status",
        ]),
        "documentos" => con_comunes(&[
            "document_id", "parent_event_id", "scope", "reference", "local_file", "sha256", "download_state", "content_read",
        ]),
        "incidencias" => vec!["capture_id", "query_id", "record_id", "incident_id", "phase", "code", "severity", "message", "recoverable"],
        "escritos" => con_comunes(&[
            "escrito_id", "scope", "doc", "escrito_type", "requester", "escrito_status", "reference", "linked_ingreso_id",
            "linked_projection", "identity_status", "raw_json",
        ]),
        "historico_proteccion" => con_comunes(&["historico_id", "program_name", "egreso_date", "egreso_cause", "ownership", "raw_json"]),
        _ => return None,
    })
}

pub const TABLAS: [&str; 9] = [
    "consultas", "registros", "bitacoras", "calendarios", "historia_tramites", "documentos", "incidencias", "escritos",
    "historico_proteccion",
];
const EXTENDIDAS: [&str; 2] = ["escritos", "historico_proteccion"];
/// `contract.EXTENSIONS`: columnas aditivas al final, solo si alguna fila las trae.
fn extensiones(tabla: &str) -> &'static [&'static str] {
    if tabla == "historia_tramites" { &["folio", "stage", "annex_visible", "raw_json"] } else { &[] }
}

fn sha256(datos: &[u8]) -> String {
    format!("{:x}", Sha256::digest(datos))
}

/// `json.dumps(valor, ensure_ascii=False)` de Python: separadores «, » y «: », claves en orden de inserción (o
/// ordenadas con `ordenar`), sin escapar lo no ASCII.
pub fn json_python(v: &Value, ordenar: bool) -> String {
    match v {
        Value::Array(a) => format!("[{}]", a.iter().map(|x| json_python(x, ordenar)).collect::<Vec<_>>().join(", ")),
        Value::Object(m) => {
            let mut pares: Vec<(&String, &Value)> = m.iter().collect();
            if ordenar {
                pares.sort_by(|a, b| a.0.cmp(b.0));
            }
            let partes: Vec<String> = pares
                .iter()
                .map(|(k, x)| format!("{}: {}", serde_json::to_string(k).unwrap(), json_python(x, ordenar)))
                .collect();
            format!("{{{}}}", partes.join(", "))
        }
        otro => serde_json::to_string(otro).unwrap_or_default(),
    }
}

/// `json.dumps(m, ensure_ascii=False, sort_keys=True, indent=2)`.
fn json_manifiesto(v: &Value) -> String {
    fn ordenado(v: &Value) -> Value {
        match v {
            Value::Object(m) => {
                let mut claves: Vec<&String> = m.keys().collect();
                claves.sort();
                Value::Object(claves.into_iter().map(|k| (k.clone(), ordenado(&m[k]))).collect())
            }
            Value::Array(a) => Value::Array(a.iter().map(ordenado).collect()),
            otro => otro.clone(),
        }
    }
    serde_json::to_string_pretty(&ordenado(v)).unwrap_or_default()
}

/// `csv.DictWriter(lineterminator='\n')` con `QUOTE_MINIMAL`: comillas si el campo trae coma, comillas o fin de línea.
fn campo_csv(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

/// `csv_bytes`: encabezado y filas con las columnas del contrato (más las extensiones que alguna fila traiga).
pub fn csv_bytes(tabla: &str, filas: &[Map<String, Value>]) -> Result<Vec<u8>, String> {
    let mut cols: Vec<&str> = columnas(tabla).ok_or("Tabla desconocida.")?;
    for c in extensiones(tabla) {
        if filas.iter().any(|f| f.contains_key(*c)) {
            cols.push(c);
        }
    }
    let mut salida = cols.join(",") + "\n";
    for f in filas {
        if f.keys().any(|k| !cols.contains(&k.as_str())) {
            return Err(format!("Columnas no definidas en {tabla}"));
        }
        let mut valores = Vec::with_capacity(cols.len());
        for c in &cols {
            match f.get(*c) {
                None => valores.push(String::new()),
                Some(Value::String(s)) => valores.push(campo_csv(s)),
                Some(_) => return Err("Valores CSV deben ser texto; IDs nunca números.".into()),
            }
        }
        salida += &valores.join(",");
        salida.push('\n');
    }
    Ok(salida.into_bytes())
}

/// `write_capture`: publicación nueva (nunca sobrescribe), manifiesto con la huella de cada CSV y el ZIP validado con el
/// lector del Asistente antes de dejarlo en su destino.
pub fn escribir_captura(destino: &Path, metadatos: Value, tablas: &[(String, Vec<Map<String, Value>>)]) -> Result<PathBuf, String> {
    escribir_captura_con(destino, metadatos, tablas, None)
}

/// `write_capture(..., consolidated=bytes)`: con el libro «Para CSMP» en `consolidado/consolidado.xlsx` (1.1.0).
pub fn escribir_captura_con(
    destino: &Path,
    mut metadatos: Value,
    tablas: &[(String, Vec<Map<String, Value>>)],
    consolidado: Option<&[u8]>,
) -> Result<PathBuf, String> {
    if destino.exists() {
        return Err("El paquete de destino ya existe.".into());
    }
    let extendida = tablas.iter().any(|(t, _)| EXTENDIDAS.contains(&t.as_str()));
    metadatos["schema_id"] = json!("CSMP_CAPTURE");
    metadatos["schema_version"] = json!(if extendida {
        VERSION_EXTENDIDA
    } else if consolidado.is_some() {
        VERSION_CONSOLIDADA
    } else {
        VERSION
    });
    metadatos["files"] = json!([]);
    if let Some(estados) = metadatos.get_mut("table_states").and_then(Value::as_object_mut) {
        for t in EXTENDIDAS {
            if extendida {
                estados.entry(t).or_insert(json!("NO_SOLICITADA"));
            } else {
                estados.remove(t);
            }
        }
    }
    if metadatos.get("mapping_version").is_none() {
        metadatos["mapping_version"] = json!(MAPPING_VERSION);
    }
    if metadatos.get("required_features").is_none() {
        metadatos["required_features"] = json!([]);
    }
    let mut contenido: Vec<(String, Vec<u8>)> = Vec::new();
    for (tabla, filas) in tablas {
        let bruto = csv_bytes(tabla, filas)?;
        let nombre = format!("data/{tabla}.csv");
        metadatos["files"].as_array_mut().unwrap().push(json!({"path": nombre, "table": tabla, "type": "normalized_table",
            "rows": filas.len(), "bytes": bruto.len(), "sha256": sha256(&bruto)}));
        contenido.push((nombre, bruto));
    }
    if let Some(libro) = consolidado {
        metadatos["files"].as_array_mut().unwrap().push(json!({"path": RUTA_CONSOLIDADO, "type": "consolidated_workbook",
            "bytes": libro.len(), "sha256": sha256(libro)}));
        contenido.push((RUTA_CONSOLIDADO.into(), libro.to_vec()));
    }
    contenido.push(("manifest.json".into(), json_manifiesto(&metadatos).into_bytes()));
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let fecha = zip::DateTime::from_date_and_time(2026, 10, 8, 0, 0, 0).map_err(|_| "Fecha ZIP inválida.".to_string())?;
    let opciones = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(fecha)
        .unix_permissions(0o600);
    for (nombre, bruto) in &contenido {
        zip.start_file(nombre, opciones).map_err(|e| e.to_string())?;
        zip.write_all(bruto).map_err(|e| e.to_string())?;
    }
    let bytes = zip.finish().map_err(|e| e.to_string())?.into_inner();
    crate::captura::leer_captura(&bytes).map_err(|e| format!("El paquete generado no pasa la validación del contrato: {e}"))?;
    if let Some(padre) = destino.parent() {
        std::fs::create_dir_all(padre).map_err(|e| format!("No se pudo crear la carpeta del paquete: {e}"))?;
    }
    super::lote::guardar_exclusivo(destino, &bytes)?;
    Ok(destino.to_path_buf())
}

// ---------------------------------------------------------------- lote (`resultados.load_lot`)

pub(super) fn leer_json(ruta: &Path) -> Result<Value, String> {
    let datos = std::fs::read(ruta).map_err(|_| format!("Falta {}.", ruta.file_name().and_then(|n| n.to_str()).unwrap_or("")))?;
    serde_json::from_slice(&datos).map_err(|_| "Un JSON del lote no se pudo leer.".to_string())
}

/// `inside`: un nombre del manifiesto solo puede ser un archivo de la carpeta del lote.
pub(super) fn dentro(carpeta: &Path, nombre: &str) -> Result<PathBuf, String> {
    if nombre.is_empty() || Path::new(nombre).file_name().and_then(|n| n.to_str()) != Some(nombre) || nombre.contains('\\') {
        return Err("El manifiesto contiene una ruta fuera del lote.".into());
    }
    Ok(carpeta.join(nombre))
}

/// `check_file`: el archivo verificado sigue íntegro (tamaño y huella).
pub(super) fn comprobar(carpeta: &Path, item: &Value) -> Result<Vec<u8>, String> {
    let ruta = dentro(carpeta, item["archivo"].as_str().unwrap_or(""))?;
    let datos = std::fs::read(&ruta).map_err(|_| "Falta un archivo o supera el límite de lectura.".to_string())?;
    if datos.len() > 80 * 1024 * 1024 {
        return Err("Falta un archivo o supera el límite de lectura.".into());
    }
    if item["bytes"].as_u64().is_some_and(|b| b as usize != datos.len()) {
        return Err("El tamaño de un archivo verificado cambió.".into());
    }
    if item["sha256"].as_str().is_none_or(|h| h.is_empty() || h != sha256(&datos)) {
        return Err("Un archivo cambió después de la descarga. No se exportó el lote.".into());
    }
    Ok(datos)
}

pub struct Lote {
    pub carpeta: PathBuf,
    pub resumen: Value,
    pub consultas: Vec<(Value, Value)>,
    /// `Lot.partial`: el lote no terminó y se abrió a sabiendas (`load_lot(..., partial=True)`).
    pub parcial: bool,
}

/// `load_lot(folder, partial=False)`: un lote VALIDADA completo, con sus manifiestos y archivos íntegros.
pub fn cargar_lote(carpeta: &Path) -> Result<Lote, String> {
    cargar_lote_con(carpeta, false)
}

/// `load_lot(folder, partial)`: con `parcial`, un lote incompleto se abre con sus consultas INCOMPLETA.
pub fn cargar_lote_con(carpeta: &Path, parcial: bool) -> Result<Lote, String> {
    let carpeta = carpeta.canonicalize().map_err(|e| format!("No se pudo abrir el lote: {e}"))?;
    let resumen = leer_json(&carpeta.join("resumen.json"))?;
    let incompleto = resumen["estado"] != "VALIDADA";
    if incompleto && !parcial {
        return Err("El lote es incompleto. Selecciona explícitamente una exportación parcial.".into());
    }
    let mut consultas = Vec::new();
    let mut vistas = BTreeSet::new();
    for q in resumen["consultas"].as_array().into_iter().flatten() {
        let ruta = dentro(&carpeta, q["verificacion"].as_str().unwrap_or(""))?;
        if !vistas.insert(ruta.clone()) {
            return Err("Verificación repetida en el resumen.".into());
        }
        let manifiesto = leer_json(&ruta)?;
        if !matches!(manifiesto["estado"].as_str(), Some("VALIDADA" | "SIN_RESULTADOS")) {
            return Err("El estado de una consulta no coincide con el resumen.".into());
        }
        if q["estado"] != manifiesto["estado"] {
            return Err("Estados de consulta incoherentes.".into());
        }
        consultas.push((q.clone(), manifiesto));
    }
    if incompleto {
        // La consulta interrumpida puede no figurar aún en las completadas.
        let mut nombres: Vec<PathBuf> = std::fs::read_dir(&carpeta).map_err(|e| e.to_string())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("verificacion ") && n.ends_with(".json")))
            .collect();
        nombres.sort();
        for ruta in nombres {
            if vistas.contains(&ruta) {
                continue;
            }
            let manifiesto = leer_json(&ruta)?;
            if manifiesto["estado"] == "INCOMPLETA" {
                let q = json!({
                    "consulta": manifiesto.get("consulta").cloned().unwrap_or(json!("Consulta incompleta")),
                    "estado": "INCOMPLETA",
                    "seleccion": manifiesto.get("seleccion").cloned().unwrap_or(json!({})),
                    "verificacion": ruta.file_name().unwrap().to_string_lossy(),
                });
                consultas.push((q, manifiesto));
            }
        }
    } else if Some(consultas.len() as u64) != resumen["consultas_esperadas"].as_u64() {
        return Err("El lote no contiene todas sus consultas.".into());
    }
    let mut nombres = BTreeSet::new();
    for (_, m) in &consultas {
        for item in m["archivos"].as_array().into_iter().flatten().filter(|i| i["estado"] == "OK") {
            comprobar(&carpeta, item)?;
            if !nombres.insert(item["archivo"].as_str().unwrap_or("").to_string()) {
                return Err("El mismo archivo aparece en varias consultas.".into());
            }
        }
        if m["estado"] == "VALIDADA" {
            if let Some(esperadas) = m["paginas_esperadas"].as_u64() {
                if m["archivos"].as_array().map_or(0, Vec::len) as u64 != esperadas {
                    return Err("Faltan páginas en una consulta validada.".into());
                }
            }
        }
        if m.get("evidencia").is_some_and(|e| !e.is_null()) {
            comprobar(&carpeta, &m["evidencia"])?;
        }
    }
    Ok(Lote { carpeta, resumen, consultas, parcial: incompleto })
}

// ---------------------------------------------------------------- tablas (`resultados.tables`)

type Filas = Vec<(usize, Vec<String>)>;

pub(super) fn normalizado(s: &str) -> String {
    super::sitfa::registros::normalizar(s)
}

/// `resultados.ALIASES` / `mapping`.
pub(super) fn mapeo(cabecera: &[String]) -> Result<BTreeMap<&'static str, usize>, String> {
    const ALIAS: [(&str, &[&str]); 10] = [
        ("rit", &["RIT"]),
        ("nombre", &["NOMBRE", "NOMBRE MENOR", "NOMBRE COMPLETO"]),
        ("rut", &["RUT", "RUT MENOR", "RUT NNA"]),
        ("tribunal", &["TRIBUNAL"]),
        ("programa", &["DERIVACION", "PROGRAMA", "NOMBRE CENTRO"]),
        ("espera", &["T ESPERA", "T_ESPERA", "DIAS_ESPERA", "TESPERA", "DIAS DE ESPERA"]),
        ("dias_cumpl", &["DIAS DE CUMPLIMIENTO", "DIAS CUMPLIMIENTO"]),
        ("dias_egresar", &["DIAS PARA EGRESAR", "DIAS EGRESAR"]),
        ("vencimiento", &["FECHA VENCIMIENTO", "FEC.VENCIMIENTO", "FEC. VENCIMIENTO", "F. VENCIMIENTO", "F.VENCIMIENTO", "F.VENC",
            "F. VENC", "VENCIMIENTO"]),
        ("egreso_proy", &["FEC.EGRESO PROYECTADO", "FEC. EGRESO PROYECTADO", "FEC EGRESO PROYECTADO"]),
    ];
    let mut salida = BTreeMap::new();
    for (clave, nombres) in ALIAS {
        let nombres: BTreeSet<String> = nombres.iter().map(|n| normalizado(n)).collect();
        let hallados: Vec<usize> = cabecera.iter().enumerate().filter(|(_, h)| nombres.contains(&normalizado(h))).map(|(i, _)| i).collect();
        if hallados.len() > 1 {
            return Err(format!("Columnas ambiguas para {clave}."));
        }
        if let Some(i) = hallados.first() {
            salida.insert(clave, *i);
        }
    }
    Ok(salida)
}

/// `headers_and_rows`: cabecera única y filas de registros del Excel reconocido, con su número de fila.
fn cabecera_y_filas(libro: &xls::Libro) -> Result<(Vec<String>, Filas), String> {
    use super::sitfa::registros::{columnas as columnas_registro, registro};
    let mut cabecera: Option<Vec<String>> = None;
    let mut filas = Vec::new();
    for (i, fila) in libro.grilla().iter().enumerate() {
        let numero = i + 1;
        if columnas_registro(fila)?.is_some() {
            let mut actual: Vec<String> = fila.iter().map(|v| v.trim().to_string()).collect();
            while actual.last().is_some_and(String::is_empty) {
                actual.pop();
            }
            let unicos: BTreeSet<String> = actual.iter().map(|h| normalizado(h)).collect();
            if actual.iter().any(String::is_empty) || unicos.len() != actual.len() {
                return Err("Cabeceras vacías o ambiguas: requiere revisión local.".into());
            }
            if cabecera.as_ref().is_some_and(|c| c != &actual) {
                return Err("La cabecera cambia dentro del Excel.".into());
            }
            cabecera = Some(actual);
            continue;
        }
        let Some(c) = cabecera.as_ref() else { continue };
        let cols = columnas_registro(c)?.ok_or("Cabecera no reconocida.")?;
        if registro(fila, &cols).is_some() {
            if fila.len() > c.len() && fila[c.len()..].iter().any(|v| !v.is_empty()) {
                return Err("Hay datos fuera de la cabecera reconocida.".into());
            }
            let mut valores = fila.clone();
            valores.resize(c.len(), String::new());
            filas.push((numero, valores));
        } else if fila.iter().any(|v| !v.is_empty()) {
            let unida = normalizado(&fila.join(" "));
            if ["TOTAL", "SIN REGISTROS", "NO HAY REGISTROS"].iter().any(|p| unida.starts_with(p)) {
                continue;
            }
            return Err("Hay filas no reconocidas; no se consolidan silenciosamente.".into());
        }
    }
    let total: usize = libro.registros.values().sum();
    match cabecera {
        Some(c) if filas.len() == total => Ok((c, filas)),
        _ => Err("No coinciden las filas del Excel reconocido.".into()),
    }
}

/// `sin_parentesis`.
pub(super) fn sin_parentesis(texto: &str) -> String {
    static GRUPO: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"\([^()]*\)").unwrap());
    let limpio = GRUPO.replace_all(texto, " ").replace(['(', ')'], " ");
    limpio.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `table_mode` (más EGRESADOS/OTRO como en `export_capture`).
fn modo_tabla(seleccion: &Value) -> &'static str {
    let tab = seleccion["tab"].as_str().unwrap_or("");
    match seleccion["screen"].as_str().unwrap_or("") {
        "seguimiento" => match tab {
            "Espera" => return "ESPERA",
            "Cumplimiento" => return "CUMPLIMIENTO",
            "Informes" => return "INFORMES",
            _ => {}
        },
        "calendario_informes" => return "INFORMES",
        _ => {}
    }
    if matches!(tab, "Egresados" | "Egreso") { "EGRESADOS" } else { "OTRO" }
}

/// `lotes.tribunal_name` y la clave de tribunal de `tables` (Laja, Mulchén y Tomé por su terminación).
fn clave_tribunal(valor: &str) -> String {
    let n = normalizado(&super::lote::nombre_tribunal(valor));
    for etiqueta in ["LAJA", "MULCHEN", "TOME"] {
        if n == etiqueta || n.ends_with(&format!(" {etiqueta}")) {
            return etiqueta.to_string();
        }
    }
    n
}

/// `vinculos.identity`.
fn identidad(rit: &str, rut: &str, nombre: &str, programa: &str) -> [String; 4] {
    [normalizado(rit), normalizado(rut).replace(['.', ' '], ""), normalizado(nombre), normalizado(programa)]
}

/// `vinculos.match_rows`: la identidad remota de cada fila desde los vínculos comprobados de su página.
fn vincular(cabecera: &mut Vec<String>, filas: &mut Filas, vinculos: &[Value], tribunal: &str) -> Result<(), String> {
    let cols = mapeo(cabecera)?;
    let claves = ["rit", "rut", "nombre", "programa"];
    let mut indice: HashMap<[String; 4], Vec<&Value>> = HashMap::new();
    for v in vinculos.iter().filter(|v| v["tribunal_codigo"] == tribunal) {
        let t = |k: &str| v[k].as_str().unwrap_or("").to_string();
        indice.entry(identidad(&t("rit"), &t("rut"), &t("nombre"), &t("programa"))).or_default().push(v);
    }
    for (_, fila) in filas.iter_mut() {
        let candidatos: Vec<&Value> = if claves.iter().all(|k| cols.contains_key(k)) {
            let c = |k: &str| fila[cols[k]].clone();
            indice.get(&identidad(&c("rit"), &c("rut"), &c("nombre"), &c("programa"))).cloned().unwrap_or_default()
        } else {
            Vec::new()
        };
        let unicos: BTreeSet<Vec<String>> = candidatos
            .iter()
            .map(|v| ["causa_id", "ingreso_id", "etapa", "persona_id", "centro_id", "antiguo"].iter().map(|k| v[*k].as_str().unwrap_or("").to_string()).collect())
            .collect();
        if unicos.len() == 1 {
            fila.extend(unicos.into_iter().next().unwrap());
            fila.push("Vinculado desde respuesta actual".into());
        } else {
            fila.extend(std::iter::repeat_n(String::new(), 6));
            fila.push(if unicos.is_empty() { "Sin vínculo comprobado" } else { "Identidad ambigua" }.into());
        }
    }
    cabecera.extend(
        ["SITFA_CAUSA_RUS", "SITFA_INGRESO_RUS", "SITFA_ETAPA_RUS", "SITFA_PERSONA_RUS", "SITFA_CENTRO_RUS", "SITFA_ANTIGUO_RUS", "SITFA_VINCULO_ESTADO"]
            .map(String::from),
    );
    Ok(())
}

/// Una fuente del lote ya tabulada: consulta del resumen, selección, archivo, cabecera y filas.
pub(super) struct Fuente {
    pub(super) verificacion: String,
    pub(super) seleccion: Value,
    pub(super) item: Value,
    pub(super) cabecera: Vec<String>,
    pub(super) filas: Filas,
    cabecera_original: Vec<String>,
    filas_originales: BTreeMap<usize, Vec<String>>,
    /// Fila cruda de la grilla (`ExcelInfo.grid`) por número de fila, con sus tipos.
    tipadas: BTreeMap<usize, Vec<xls::Celda>>,
    /// Columnas de la fila que vienen de la grilla (Carga conserva la fila completa).
    ancho_original: usize,
}

impl Fuente {
    /// La fila de `tables(lot)` con los valores crudos de Python: lo que la grilla trae como número, fecha o vacío
    /// conserva su tipo; el texto y las columnas añadidas quedan como texto procesado.
    pub(super) fn fila_tipada(&self, numero: usize, valores: &[String]) -> Vec<xls::Celda> {
        let cruda = self.tipadas.get(&numero).map(Vec::as_slice).unwrap_or(&[]);
        let ancho = self.ancho_original.min(cruda.len());
        valores
            .iter()
            .enumerate()
            .map(|(i, v)| match cruda.get(i) {
                Some(c) if i < ancho && !c.es_texto() => c.clone(),
                _ => xls::Celda::Texto(v.clone()),
            })
            .collect()
    }
}

/// `carga.HEADERS`.
const ENCABEZADOS_CARGA: [&str; 23] = [
    "RIT", "TRIBUNAL", "NOMBRE FUNCIONARIO", "NOMBRE JUEZ", "TRÁMITE", "INVALIDADO", "DOC. RES", "DOC. ESC", "DÍAS", "CARGO",
    "PRIMER PREGRABADO FECHA", "PRIMER PREGRABADO HORA", "ÚLTIMO PREGRABADO FECHA", "ÚLTIMO PREGRABADO HORA", "DESPACHO FECHA",
    "DESPACHO HORA", "DEVOLUCIÓN FECHA", "DEVOLUCIÓN HORA", "NUEVO DESPACHO FECHA", "NUEVO DESPACHO HORA",
    "CANTIDAD ESCRITOS RESOLUCIÓN", "FECHA FIRMA", "HORA FIRMA",
];
/// `carga.RIT_RE.fullmatch`.
static RIT_CARGA: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"^[A-Z]+-\d+-\d{4}$").unwrap());

/// `resultados.tables(lot)` para Seguimiento, calendarios y Carga.
pub(super) fn tablas_lote(lote: &Lote) -> Result<Vec<Fuente>, String> {
    let mut salida = Vec::new();
    let mut cuenta = 0usize;
    for (q, m) in &lote.consultas {
        for item in m["archivos"].as_array().into_iter().flatten().filter(|i| i["estado"] == "OK") {
            let datos = comprobar(&lote.carpeta, item)?;
            let seleccion = if q.get("seleccion").is_some_and(|s| !s.is_null()) { q["seleccion"].clone() } else { m["seleccion"].clone() };
            let carga = seleccion["screen"] == "carga";
            let (cabecera_original, filas_originales) = if carga {
                // `carga.HEADERS` y las filas de la grilla cuyo primer valor canónico es un RIT (fila completa).
                let (canonica, literal) = xls::grilla_carga(&datos)?;
                let filas: Filas = canonica.iter().zip(literal).enumerate()
                    .filter(|(_, (c, _))| !c.is_empty() && RIT_CARGA.is_match(&normalizado(&c[0])))
                    .map(|(i, (_, l))| (i + 1, l)).collect();
                (ENCABEZADOS_CARGA.iter().map(|h| h.to_string()).collect::<Vec<_>>(), filas)
            } else {
                cabecera_y_filas(&xls::leer(&datos)?)?
            };
            let grilla = xls::grilla_tipada(&datos, carga)?;
            let tipadas: BTreeMap<usize, Vec<xls::Celda>> =
                filas_originales.iter().filter_map(|(n, _)| grilla.get(n - 1).map(|f| (*n, f.clone()))).collect();
            let ancho_original = if carga { usize::MAX } else { cabecera_original.len() };
            let (mut cabecera, mut filas) = (cabecera_original.clone(), filas_originales.clone());
            if Some(filas.len() as u64) != item["registros"].as_u64() {
                return Err("El recuento no coincide con la verificación.".into());
            }
            if cabecera.iter().any(|h| h.starts_with("SITFA_")) {
                return Err("La descarga contiene columnas reservadas de procedencia.".into());
            }
            cuenta += filas.len();
            if cuenta > MAX_FILAS {
                return Err("El lote supera 200.000 filas para exportación local.".into());
            }
            let tribunal_nombre = seleccion["tribunal_nombre"].as_str().unwrap_or("").to_string();
            let cols = mapeo(&cabecera)?;
            if !cols.contains_key("tribunal") && !tribunal_nombre.is_empty() {
                cabecera.push("TRIBUNAL".into());
                filas.iter_mut().for_each(|(_, f)| f.push(tribunal_nombre.clone()));
            }
            let cols = mapeo(&cabecera)?;
            if let (Some(&c), false) = (cols.get("tribunal"), tribunal_nombre.is_empty()) {
                if filas.iter().any(|(_, f)| clave_tribunal(&f[c]) != clave_tribunal(&tribunal_nombre)) {
                    return Err("Un tribunal del Excel no coincide con la consulta; revisa sus etiquetas.".into());
                }
            }
            let modalidad = match seleccion["modality"].as_str() {
                Some("1") => Some("RES"),
                Some("2") => Some("AMB"),
                Some("3") => Some("FAE"),
                Some("4") => Some("DCE"),
                _ => None,
            };
            if let (true, Some(m)) = (seleccion["screen"] == "seguimiento", modalidad) {
                let existe = cabecera.iter().any(|h| matches!(normalizado(h).as_str(), "MODALIDAD" | "TIPO PROGRAMA"));
                cabecera.push(if existe { "SITFA_MODALIDAD" } else { "MODALIDAD" }.into());
                filas.iter_mut().for_each(|(_, f)| f.push(m.into()));
            }
            if let Some(t) = seleccion["tribunal"].as_str().filter(|t| !t.is_empty()) {
                cabecera.push("SITFA_TRIBUNAL_CODIGO".into());
                filas.iter_mut().for_each(|(_, f)| f.push(t.into()));
            }
            if seleccion["screen"] == "seguimiento" {
                let vinculos = item["vinculos_ingreso"].as_array().cloned().unwrap_or_default();
                vincular(&mut cabecera, &mut filas, &vinculos, seleccion["tribunal"].as_str().unwrap_or(""))?;
            }
            if let Some(&c) = mapeo(&cabecera)?.get("nombre") {
                filas.iter_mut().for_each(|(_, f)| f[c] = sin_parentesis(&f[c]));
            }
            // En Carga, `export_capture` toma como «originales» la cabecera y las filas ya procesadas.
            let (cabecera_original, filas_originales) =
                if carga { (cabecera.clone(), filas.clone()) } else { (cabecera_original, filas_originales) };
            salida.push(Fuente {
                verificacion: q["verificacion"].as_str().unwrap_or("").to_string(),
                seleccion,
                item: item.clone(),
                cabecera,
                filas,
                cabecera_original,
                filas_originales: filas_originales.into_iter().collect(),
                tipadas,
                ancho_original,
            });
        }
    }
    Ok(salida)
}

// ---------------------------------------------------------------- `export_capture`

/// `coverage.aggregate`.
fn agregar<'a>(estados: impl IntoIterator<Item = &'a str>) -> String {
    let v: Vec<&str> = estados.into_iter().collect();
    if v.is_empty() {
        "NO_SOLICITADA".into()
    } else if v.iter().all(|e| *e == "VACIA_COMPROBADA") {
        "VACIA_COMPROBADA".into()
    } else if v.iter().all(|e| matches!(*e, "COMPLETA" | "VACIA_COMPROBADA")) {
        "COMPLETA".into()
    } else if v.iter().all(|e| *e == v[0]) {
        v[0].into()
    } else {
        "PARCIAL".into()
    }
}

fn texto(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        otro => otro.to_string(),
    }
}

/// `str(valor)` de un valor del control (texto tal cual; `null` → vacío).
pub(super) fn texto_de(v: &Value) -> String {
    texto(v)
}

pub(super) fn sha256_hex(datos: &[u8]) -> String {
    sha256(datos)
}

fn fila(pares: &[(&str, String)]) -> Map<String, Value> {
    pares.iter().map(|(k, v)| (k.to_string(), json!(v))).collect()
}

type Fila = Map<String, Value>;
type Clave = [String; 3];
type PorIngreso = Vec<(Clave, Value)>;

/// `{k: '' for k in COMMON}` con los valores dados.
fn comun(pares: &[(&str, String)]) -> Fila {
    let mut c: Fila = COMUNES.iter().map(|k| (k.to_string(), json!(""))).collect();
    for (k, v) in pares {
        c.insert(k.to_string(), json!(v));
    }
    c
}

fn con(mut base: Fila, pares: &[(&str, String)]) -> Fila {
    for (k, v) in pares {
        base.insert(k.to_string(), json!(v));
    }
    base
}

fn primeros(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Opciones de `export_capture` además de los lotes de listados (E-Q5c): `diaries`, `match_diaries`, `historia`,
/// `historico`, `meses` y `avisos`.
#[derive(Clone, Debug)]
pub struct Opciones {
    pub bitacoras: Vec<PathBuf>,
    pub emparejar: bool,
    pub historia: Option<PathBuf>,
    pub historico: Option<PathBuf>,
    pub meses: u32,
    pub avisos: Vec<(String, String)>,
    /// `consolidated`: el libro «Para CSMP» dentro del paquete; si el lote no es apto, el paquete sale sin él y la
    /// nota lo explica (`export_capture_with_note`).
    pub consolidado: bool,
}

impl Default for Opciones {
    fn default() -> Self {
        Self { bitacoras: Vec::new(), emparejar: false, historia: None, historico: None, meses: 4, avisos: Vec::new(), consolidado: false }
    }
}

/// Resultado de `exportar_con`: ruta del paquete, mensaje de reutilización de bitácoras y si el histórico quedó
/// incompleto (lo que `package_current_lot` informa a la interfaz).
#[derive(Clone, Debug)]
pub struct Exportado {
    pub ruta: PathBuf,
    /// La nota de `export_capture_with_note` («Incluye el XLSX consolidado.» o por qué no).
    pub nota: String,
    pub reutilizacion: Option<String>,
    pub historico_incompleto: bool,
}

/// Tablas del paquete en el orden en que Python las crea (`data` es un dict: el ZIP sigue ese orden).
struct Datos {
    cid: String,
    tablas: Vec<(String, Vec<Fila>)>,
    n: usize,
}

impl Datos {
    fn tabla(&mut self, nombre: &str) -> &mut Vec<Fila> {
        let i = match self.tablas.iter().position(|(t, _)| t == nombre) {
            Some(i) => i,
            None => {
                self.tablas.push((nombre.to_string(), Vec::new()));
                self.tablas.len() - 1
            }
        };
        &mut self.tablas[i].1
    }

    fn tiene(&self, nombre: &str) -> bool {
        self.tablas.iter().any(|(t, _)| t == nombre)
    }

    fn filas(&self, nombre: &str) -> &[Fila] {
        self.tablas.iter().find(|(t, _)| t == nombre).map(|(_, f)| f.as_slice()).unwrap_or(&[])
    }

    fn incidencia(&mut self, qid: &str, codigo: &str, mensaje: &str, rid: &str, severidad: &str) {
        self.n += 1;
        let f = fila(&[("capture_id", self.cid.clone()), ("query_id", qid.into()), ("record_id", rid.into()),
            ("incident_id", format!("i{}", self.n)), ("phase", "EXPORTAR".into()), ("code", codigo.into()),
            ("severity", severidad.into()), ("message", mensaje.into()), ("recoverable", "true".into())]);
        self.tabla("incidencias").push(f);
    }

    fn aviso(&mut self, qid: &str, codigo: &str, mensaje: &str) {
        self.incidencia(qid, codigo, mensaje, "", "ADVERTENCIA");
    }

    #[allow(clippy::too_many_arguments)]
    fn consulta(&mut self, qid: &str, seleccion: &Value, estado: &str, cuando: &str, esperadas: String, guardadas: usize,
                pantalla: Option<&str>, error: &str) -> Fila {
        let mut r: Fila = columnas("consultas").unwrap().into_iter().map(|c| (c.to_string(), json!(""))).collect();
        for (k, v) in [
            ("query_id", qid.to_string()),
            ("source_system", "RUS".into()),
            ("screen", pantalla.map(str::to_string).unwrap_or_else(|| seleccion["screen"].as_str().unwrap_or("seguimiento").to_string())),
            ("tab", seleccion["tab"].as_str().unwrap_or("").into()),
            ("court_code", texto(&seleccion["tribunal"])),
            ("court_name", seleccion["tribunal_nombre"].as_str().unwrap_or("").into()),
            ("modality_code", texto(&seleccion["modality"])),
            ("coverage", estado.into()),
            ("captured_at", cuando.into()),
            ("filters_json", json_python(seleccion, false)),
            ("expected_pages", esperadas),
            ("saved_pages", guardadas.to_string()),
            ("time_zone", "DESCONOCIDA".into()),
            ("history_complete", "false".into()),
            ("institutional_verified", "false".into()),
            ("error", error.into()),
        ] {
            r.insert(k.into(), json!(v));
        }
        self.tabla("consultas").push(r.clone());
        r
    }
}

// ---------------------------------------------------------------- bitácoras (`nurus.capture.diaries`)

const IDENTIDAD_BITACORA: [&str; 3] = ["tribunal_codigo", "causa_id", "ingreso_id"];
const LOTE_MAX: u64 = 32 * 1024 * 1024;
const COPIA_MAX: u64 = 8 * 1024 * 1024;
const COPIA_NO_VERIFICABLE: &str = "No se pudo abrir una copia verificable.";

fn clave_de(v: &Value) -> Clave {
    IDENTIDAD_BITACORA.map(|k| texto(&v[k]))
}

fn poner(v: &mut PorIngreso, k: Clave, valor: Value) {
    match v.iter_mut().find(|(c, _)| *c == k) {
        Some(p) => p.1 = valor,
        None => v.push((k, valor)),
    }
}

/// `diary_paths`: rutas resueltas sin repetir, en orden.
fn rutas_bitacoras(rutas: &[PathBuf]) -> Vec<PathBuf> {
    let mut salida: Vec<PathBuf> = Vec::new();
    for r in rutas {
        let r = r.canonicalize().unwrap_or_else(|_| std::path::absolute(r).unwrap_or_else(|_| r.clone()));
        if !salida.contains(&r) {
            salida.push(r);
        }
    }
    salida
}

/// Copia HTML junto al control: `Err(Some(msg))` es el `ValueError` de Python; `Err(None)`, el error genérico.
fn copia_local(carpeta: &Path, nombre: &str, fuera: &str) -> Result<Vec<u8>, Option<String>> {
    let mut partes = Path::new(nombre).components();
    let simple = matches!((partes.next(), partes.next()), (Some(std::path::Component::Normal(_)), None));
    let ruta = carpeta.join(nombre);
    let enlazada_fuera = ruta.canonicalize().ok().is_some_and(|c| c.parent() != Some(carpeta));
    if !simple || enlazada_fuera || ruta.extension() != Some(std::ffi::OsStr::new("html")) {
        return Err(Some(fuera.into()));
    }
    let meta = std::fs::metadata(&ruta).map_err(|_| None)?;
    if meta.len() > COPIA_MAX {
        return Err(Some(fuera.into()));
    }
    std::fs::read(&ruta).map_err(|_| None)
}

fn cadena<'a>(v: &'a Value, k: &str) -> Result<&'a str, Option<String>> {
    v.get(k).and_then(Value::as_str).ok_or(None)
}

/// La bitácora LEIDA de un registro, releída de su HTML con hash e identidad comprobadas.
fn leer_registro(carpeta: &Path, r: &Value, id: &Clave) -> Result<Value, Option<String>> {
    let nombre = cadena(r, "archivo")?;
    let datos = copia_local(carpeta, nombre, "Archivo de bitácora fuera de alcance.")?;
    if json!(sha256(&datos)) != *r.get("sha256").ok_or(None)? {
        return Err(Some("La copia HTML cambió después de su verificación.".into()));
    }
    let mut params = BTreeMap::new();
    for (k, v) in r.get("params").and_then(Value::as_object).into_iter().flatten() {
        params.insert(k.clone(), v.as_str().ok_or(None)?.to_string());
    }
    let mut q = super::lecturas::leer_bitacora(&datos, &params, nombre, cadena(r, "capturada")?).map_err(Some)?;
    if clave_de(&q) != *id {
        return Err(Some("La bitácora pertenece a otro ingreso.".into()));
    }
    let notas = super::bitacoras::advertencias(&q, cadena(r, "rit")?, cadena(r, "nombre")?);
    if let Some(w) = q["warnings"].as_array_mut() {
        w.extend(notas.into_iter().map(Value::String));
    }
    Ok(q)
}

/// `_calendar` (WD-1): calendario por ingreso releído desde su HTML con hash; una copia alterada queda FALLIDA.
fn calendario_ingreso(carpeta: &Path, r: &Value, id: &Clave) -> Value {
    let lectura = &r["calendario_lectura"];
    let error = texto(&lectura["error"]);
    let mut res = json!({
        "tribunal_codigo": id[0], "causa_id": id[1], "ingreso_id": id[2], "coverage": "FALLIDA",
        "error": if error.is_empty() { "No se completó la lectura del calendario.".to_string() } else { error },
        "proyecciones": [], "archivo": texto(&lectura["archivo"]), "captured_at": texto(&lectura["capturada"]), "html_sha256": "",
    });
    if lectura["estado"] != "LEIDA" {
        return res;
    }
    let leido = (|| -> Result<Value, Option<String>> {
        let nombre = cadena(lectura, "archivo")?;
        let datos = copia_local(carpeta, nombre, "Archivo de calendario fuera de alcance.")?;
        if json!(sha256(&datos)) != *lectura.get("sha256").ok_or(None)? {
            return Err(Some("La copia HTML del calendario cambió después de su verificación.".into()));
        }
        let args: Vec<String> = lectura.get("args").and_then(Value::as_array).ok_or(None)?.iter().map(texto).collect();
        super::lecturas::leer_calendario(&datos, &args, nombre, cadena(lectura, "capturada")?).map_err(Some)
    })();
    match leido {
        Err(e) => res["error"] = json!(e.unwrap_or_else(|| COPIA_NO_VERIFICABLE.into())),
        Ok(c) => {
            let vacio = c["proyecciones"].as_array().is_none_or(Vec::is_empty);
            res["coverage"] = json!(if vacio { "VACIA_COMPROBADA" } else { "COMPLETA" });
            res["error"] = json!("");
            res["proyecciones"] = c["proyecciones"].clone();
            res["plazo_remision_meses"] = c.get("plazo_remision_meses").cloned().unwrap_or(Value::Null);
            res["plazo_intervencion_meses"] = c.get("plazo_intervencion_meses").cloned().unwrap_or(Value::Null);
            res["html_sha256"] = c["html_sha256"].clone();
        }
    }
    res
}

fn momento(q: &Value) -> Option<chrono::DateTime<chrono::Utc>> {
    let s = q["captured_at"].as_str()?.replace('Z', "+00:00");
    chrono::DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc))
}

/// `_merge_snapshot`: dos copias del mismo ingreso; la idéntica se acumula, la distinta exige fecha inequívoca.
fn fusionar(viejo: Option<Value>, nuevo: Value) -> Result<Value, String> {
    let Some(viejo) = viejo else { return Ok(nuevo) };
    if viejo["coverage"] == "FALLIDA" {
        return Ok(nuevo);
    }
    if nuevo["coverage"] == "FALLIDA" {
        return Ok(viejo);
    }
    let contenido = |q: &Value| -> Value {
        let mut m = Map::new();
        for k in ["tribunal_codigo", "causa_id", "ingreso_id", "rit", "persona", "centro", "stage_code", "row_metadata"] {
            m.insert(k.into(), q.get(k).cloned().unwrap_or(Value::Null));
        }
        let entradas: Vec<Value> = q["entries"].as_array().into_iter().flatten().map(|e| {
            let mut e = e.clone();
            if let Some(o) = e.as_object_mut() {
                o.shift_remove("fuente");
            }
            e
        }).collect();
        m.insert("entries".into(), Value::Array(entradas));
        Value::Object(m)
    };
    let identicas = viejo["html_sha256"] == nuevo["html_sha256"] || contenido(&viejo) == contenido(&nuevo);
    let (a, b) = (momento(&viejo), momento(&nuevo));
    if identicas {
        let mut elegido = if matches!((a, b), (Some(a), Some(b)) if b > a) { nuevo.clone() } else { viejo.clone() };
        elegido["captures"] = json!(viejo["captures"].as_i64().unwrap_or(0) + nuevo["captures"].as_i64().unwrap_or(0));
        let mut avisos: Vec<Value> = Vec::new();
        for w in viejo["warnings"].as_array().into_iter().flatten().chain(nuevo["warnings"].as_array().into_iter().flatten()) {
            if !avisos.contains(w) {
                avisos.push(w.clone());
            }
        }
        elegido["warnings"] = Value::Array(avisos);
        return Ok(elegido);
    }
    match (a, b) {
        (Some(a), Some(b)) if a != b => Ok(if b > a { nuevo } else { viejo }),
        _ => Err("Hay copias distintas del mismo ingreso sin fecha inequívoca. Selecciona un solo lote de bitácoras para comparar.".into()),
    }
}

/// Hoja adicional de `import_lote` (`extra_sheets`): título, encabezados y filas con los valores del control.
pub(super) type HojaExtra = (String, Vec<&'static str>, Vec<Vec<Value>>);

/// `import_lote` completo: bitácoras y calendarios por ingreso, la fuente y las hojas de control del Excel.
pub(super) struct LoteImportado {
    consultas: PorIngreso,
    calendarios: PorIngreso,
    pub(super) fuente: Value,
    pub(super) extras: Vec<HojaExtra>,
}

fn valor_o_vacio(v: &Value, k: &str) -> Value {
    v.get(k).cloned().unwrap_or(json!(""))
}

/// `import_lote`: bitácoras y calendarios por ingreso de un control `bitacoras.json`, releídos de sus HTML.
pub(super) fn importar_lote_completo(manifiesto: &Path) -> Result<LoteImportado, String> {
    let ausente = || format!("No se encuentra el control de bitácoras: {}. Recupera bitacoras.json con sus HTML originales o \
                              excluye esa referencia del paquete.", manifiesto.display());
    let meta = std::fs::metadata(manifiesto).map_err(|_| ausente())?;
    if meta.len() > LOTE_MAX {
        return Err("Lote de bitácoras demasiado grande.".into());
    }
    let bruto = std::fs::read(manifiesto).map_err(|_| ausente())?;
    let datos: Value = std::str::from_utf8(&bruto).ok().and_then(|s| serde_json::from_str(s).ok())
        .filter(Value::is_object).ok_or("El control de bitácoras no es un JSON válido.")?;
    if datos["tipo"] != "BITACORAS_LECTURA" || datos["version"] != 1 {
        return Err("El archivo no es un lote de lectura de bitácoras.".into());
    }
    let (Some(registros), Some(_)) = (datos["registros"].as_array(), datos["consultas"].as_array()) else {
        return Err("Falta el control de lecturas del lote.".into());
    };
    let carpeta = manifiesto.parent().ok_or("Control de bitácoras sin carpeta.")?;
    let (mut consultas, mut calendarios) = (Vec::new(), Vec::new());
    let mut libro: Vec<Vec<Value>> = Vec::new();
    for r in registros {
        if !r.is_object() || r.get("params").is_some_and(|p| !p.is_object()) {
            return Err("El control de lecturas tiene un registro mal formado.".into());
        }
        let id = clave_de(r);
        let completa = id.iter().all(|s| !s.is_empty());
        if completa && r["calendario_lectura"].is_object() {
            poner(&mut calendarios, id.clone(), calendario_ingreso(carpeta, r, &id));
        }
        let mut error = texto(&r["error"]);
        let mut estado = r.get("estado").cloned().unwrap_or(json!("FALLIDA"));
        let mut consulta = None;
        if estado == "LEIDA" {
            match leer_registro(carpeta, r, &id) {
                Ok(q) => {
                    let visibles = super::bitacoras::advertencias(&q, &texto(&r["rit"]), &texto(&r["nombre"]));
                    if !visibles.is_empty() {
                        error = visibles.join(" ");
                    }
                    consulta = Some(q)
                }
                Err(e) => {
                    estado = json!("FALLIDA");
                    error = e.unwrap_or_else(|| COPIA_NO_VERIFICABLE.into());
                }
            }
        }
        let mut fila: Vec<Value> = ["tribunal_nombre", "modalidad_nombre", "pestana", "pagina", "fila", "rit", "nombre", "programa", "causa_id", "ingreso_id"]
            .iter().map(|k| valor_o_vacio(r, k)).collect();
        fila.extend([estado.clone(), json!(error), valor_o_vacio(r, "archivo"), valor_o_vacio(r, "sha256"), valor_o_vacio(r, "capturada"),
            r["diagnostico"].get("codigo_extension").cloned().unwrap_or(json!(""))]);
        libro.push(fila);
        if completa {
            let q = consulta.unwrap_or_else(|| json!({
                "tribunal_codigo": id[0], "causa_id": id[1], "ingreso_id": id[2], "rit": texto(&r["rit"]),
                "persona": texto(&r["nombre"]), "centro": texto(&r["programa"]), "coverage": "FALLIDA",
                "error": if error.is_empty() { "No se completó la lectura.".to_string() } else { error.clone() },
                "entries": [], "stage_code": "", "captured_at": "", "captures": 0, "html_sha256": "", "warnings": [],
                "row_metadata": [],
            }));
            let viejo = consultas.iter().find(|(c, _)| *c == id).map(|(_, v): &(Clave, Value)| v.clone());
            poner(&mut consultas, id, fusionar(viejo, q)?);
        }
    }
    let alcances = datos["consultas"].as_array().cloned().unwrap_or_default();
    let mut filas_consultas: Vec<Vec<Value>> = alcances.iter()
        .map(|s| ["tribunal_nombre", "modalidad_nombre", "tab", "estado", "paginas", "registros", "error"].iter().map(|k| valor_o_vacio(s, k)).collect())
        .collect();
    let esperado = datos.get("seleccion").cloned().unwrap_or(json!({}));
    let lista = |k: &str| esperado[k].as_array().cloned().unwrap_or_default();
    let mut planificadas = BTreeSet::new();
    for t in lista("tribunals") {
        for m in lista("modalities") {
            for p in lista("tabs") {
                planificadas.insert((texto(&t), texto(&m), texto(&p)));
            }
        }
    }
    let intentadas: BTreeSet<(String, String, String)> = alcances.iter()
        .map(|s| (texto(&s["tribunal"]), texto(&s["modality"]), texto(&s["tab"]))).collect();
    for (t, m, p) in planificadas.difference(&intentadas) {
        filas_consultas.push(vec![json!(t), json!(m), json!(p), json!("NO_CONSULTADA"), json!(0), json!(0),
            json!("La ejecución terminó antes de esta consulta.")]);
    }
    let unir = |k: &str, defecto: Value| json_python(&datos.get(k).cloned().unwrap_or(defecto), false);
    let mut extras: Vec<HojaExtra> = vec![
        ("Lecturas".into(), vec!["Tribunal", "Modalidad", "Pestaña", "Página", "Fila", "RIT", "Persona", "Centro", "Causa RUS",
            "Ingreso RUS", "Estado lectura", "Detalle", "Archivo", "SHA-256", "Fecha captura", "Código extensión"], libro.clone()),
        ("Consultas".into(), vec!["Tribunal", "Modalidad", "Pestaña", "Estado", "Páginas recorridas", "Registros enumerados", "Detalle"],
            filas_consultas),
        ("Lote".into(), vec!["Dato", "Valor"], vec![
            vec![json!("Estado del recorrido"), datos.get("estado").cloned().unwrap_or(json!("INCOMPLETA"))],
            vec![json!("Fecha"), valor_o_vacio(&datos, "fecha")],
            vec![json!("Error del recorrido"), valor_o_vacio(&datos, "error")],
            vec![json!("Descargador"), json!(unir("origen", json!({})))],
            vec![json!("Extensión"), json!(unir("extension", json!({})))],
            vec![json!("Selección"), json!(json_python(&esperado, false))],
            vec![json!("Historial"), json!("Copia de la tabla capturada; no acredita historial remoto completo.")],
        ]),
    ];
    if let Some(anteriores) = datos["anteriores_no_reenumeradas"].as_array().filter(|a| !a.is_empty()) {
        let filas = anteriores.iter().map(|r| {
            let mut f: Vec<Value> = ["tribunal_nombre", "modalidad_nombre", "pestana", "rit", "nombre", "causa_id", "ingreso_id", "estado", "archivo", "sha256"]
                .iter().map(|k| valor_o_vacio(r, k)).collect();
            f.push(json!("No se reenumeró en esta ejecución; no se acredita desaparición del registro."));
            f
        }).collect();
        extras.push(("Lecturas anteriores".into(), vec!["Tribunal", "Modalidad", "Pestaña", "RIT", "Persona", "Causa RUS", "Ingreso RUS",
            "Estado previo", "Archivo previo", "SHA-256 previo", "Alcance"], filas));
    }
    let fuente = json!({
        "file": manifiesto.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        "sha256": sha256(&bruto), "requests": registros.len(),
        "openings": libro.iter().filter(|f| f[10] == "LEIDA").count(), "detail": valor_o_vacio(&datos, "estado"),
    });
    Ok(LoteImportado { consultas, calendarios, fuente, extras })
}

pub(super) struct Importacion {
    pub(super) consultas: Vec<Value>,
    calendarios: Vec<Value>,
    emparejadas: usize,
    excluidas: usize,
    pub(super) fuentes: Vec<Value>,
    pub(super) extras: Vec<HojaExtra>,
}

/// `import_lotes`: reutiliza originales comprobados; con `identidades`, solo los ingresos verificados del paquete
/// (nunca por RIT o nombre).
pub(super) fn importar_lotes(rutas: &[PathBuf], identidades: Option<&BTreeSet<Clave>>) -> Result<Importacion, String> {
    let rutas = rutas_bitacoras(rutas);
    if rutas.is_empty() {
        return Err("Añade un control bitacoras.json con sus HTML originales.".into());
    }
    let (mut consultas, mut calendarios): (PorIngreso, PorIngreso) = (Vec::new(), Vec::new());
    let mut excluidas = 0;
    let (mut fuentes, mut extras): (Vec<Value>, Vec<HojaExtra>) = (Vec::new(), Vec::new());
    for ruta in &rutas {
        let LoteImportado { consultas: lote_consultas, calendarios: lote_calendarios, fuente, extras: hojas } = importar_lote_completo(ruta)?;
        fuentes.push(fuente);
        for (titulo, cabecera, filas) in hojas {
            match extras.iter_mut().find(|(t, _, _)| *t == titulo) {
                Some(h) => h.2.extend(filas),
                None => extras.push((titulo, cabecera, filas)),
            }
        }
        for (k, cal) in lote_calendarios {
            if identidades.is_some_and(|i| !i.contains(&k)) {
                continue;
            }
            let ok = |c: &Value| c["coverage"] != "FALLIDA";
            let reemplaza = match calendarios.iter().find(|(c, _)| *c == k) {
                None => true,
                Some((_, viejo)) => (ok(&cal) && !ok(viejo))
                    || (ok(&cal) == ok(viejo) && texto(&cal["captured_at"]) >= texto(&viejo["captured_at"])),
            };
            if reemplaza {
                poner(&mut calendarios, k, cal);
            }
        }
        for (k, q) in lote_consultas {
            if identidades.is_some_and(|i| !i.contains(&k)) {
                excluidas += 1;
                continue;
            }
            let viejo = consultas.iter().find(|(c, _)| *c == k).map(|(_, v)| v.clone());
            poner(&mut consultas, k, fusionar(viejo, q)?);
        }
    }
    Ok(Importacion {
        emparejadas: consultas.len(),
        consultas: consultas.into_iter().map(|(_, v)| v).collect(),
        calendarios: calendarios.into_iter().map(|(_, v)| v).collect(),
        excluidas,
        fuentes,
        extras,
    })
}

/// `temporal.earliest`: primer día de la ventana de 1 a 4 meses naturales.
fn primera_fecha(corte: chrono::NaiveDate, meses: u32) -> Result<chrono::NaiveDate, String> {
    use chrono::Datelike;
    if !(1..=4).contains(&meses) {
        return Err("El período a recolectar va de 1 a 4 meses.".into());
    }
    let mes = corte.year() * 12 + corte.month0() as i32 - meses as i32;
    let (anio, mes0) = (mes.div_euclid(12), mes.rem_euclid(12) as u32 + 1);
    let ultimo = (28..=31).rev().find(|d| chrono::NaiveDate::from_ymd_opt(anio, mes0, *d).is_some()).unwrap_or(28);
    chrono::NaiveDate::from_ymd_opt(anio, mes0, corte.day().min(ultimo)).ok_or_else(|| "Fecha de corte inválida.".into())
}

// ---------------------------------------------------------------- historia (`nurus.capture.historias`)

/// `nurus.personal.motor.vocabulario.CONTEXTO`: nomenclaturas cuyo último trámite anterior a la ventana se conserva.
const CONTEXTO: [&str; 9] = [
    "Orden de búsqueda", "Termino de búsqueda", "Deja sin efecto búsqueda", "Conciliacion", "Dictación de sentencia",
    "Egreso de servicio protección especializada y otras redes", "Designa abogado", "Deja sin efecto designación C. Ad-litem",
    "Prorroga",
];
const HISTORIA_MAX: u64 = 8 * 1024 * 1024;

pub(super) struct Causa {
    pub(super) clave: String,
    pub(super) causa: Value,
    pub(super) lectura: Option<Value>,
    pub(super) ventana: Vec<Value>,
    anteriores: Vec<Value>,
    escritos: Option<Value>,
    escritos_error: String,
}

/// `historias.cargar`: (estado, causas). Cada causa leída se relee del HTML respaldado y se verifica su SHA-256.
pub(super) fn cargar_historias(manifiesto: &Path) -> Result<(Value, Vec<Causa>), String> {
    let estado: Value = std::fs::read(manifiesto).ok().and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or("No se pudo leer historias.json.")?;
    if estado["version"] != 1 || estado["tipo"] != "HISTORIA_LECTURA" {
        return Err("El archivo no es una lectura de historias.".into());
    }
    let carpeta = manifiesto.parent().ok_or("historias.json sin carpeta.")?;
    let meses = estado["meses"].as_u64().unwrap_or(4) as u32;
    let mut causas = Vec::new();
    for (clave, causa) in estado["causas"].as_object().ok_or("historias.json sin causas.")? {
        let mut item = Causa { clave: clave.clone(), causa: causa.clone(), lectura: None, ventana: vec![], anteriores: vec![],
            escritos: None, escritos_error: String::new() };
        if matches!(causa["estado"].as_str(), Some("LEIDA" | "VACIA_COMPROBADA")) {
            let ruta = carpeta.join(causa["archivo"].as_str().ok_or("Falta el respaldo de una historia leída.")?);
            let meta = std::fs::metadata(&ruta).ok().filter(|m| m.is_file() && m.len() <= HISTORIA_MAX);
            let datos = meta.and_then(|_| std::fs::read(&ruta).ok()).ok_or("Falta el respaldo de una historia leída.")?;
            if json!(sha256(&datos)) != causa["sha256"] {
                return Err("El respaldo de una historia no coincide con su SHA-256.".into());
            }
            let lectura = super::lecturas::leer_historia(&datos, "", "")?;
            let leida = chrono::NaiveDate::parse_from_str(&primeros(causa["leida"].as_str().unwrap_or(""), 10), "%Y-%m-%d")
                .map_err(|_| "Fecha de lectura de historia inválida.".to_string())?;
            let filas = lectura["filas"].as_array().cloned().unwrap_or_default();
            let [dentro, anteriores, posteriores, sin_fecha] = super::lecturas::separar_ventana(&filas, leida, meses)?;
            if !posteriores.is_empty() {
                let mut avisos: Vec<Value> = item.causa["advertencias"].as_array().cloned().unwrap_or_default();
                if !avisos.contains(&json!("HISTORIA_FECHA_POSTERIOR")) {
                    avisos.push(json!("HISTORIA_FECHA_POSTERIOR"));
                }
                item.causa["advertencias"] = Value::Array(avisos);
            }
            match lectura.get("escritos") {
                Some(e) if e.get("error").is_some() => {
                    let codigo = texto(&e["error"]);
                    item.escritos_error = if codigo == "ESCRITOS_AUSENTE" { String::new() } else { codigo };
                }
                Some(e) => item.escritos = Some(e.clone()),
                None => {}
            }
            let mut ventana: Vec<Value> = dentro.into_iter().chain(posteriores).chain(sin_fecha).collect();
            ventana.sort_by_key(|f| f["fila"].as_u64().unwrap_or(0));
            item.ventana = ventana;
            item.anteriores = anteriores;
            item.lectura = Some(lectura);
        }
        causas.push(item);
    }
    Ok((estado, causas))
}

/// `contexto_anterior`: último trámite anterior a la ventana de cada nomenclatura de contexto, en orden de fila.
fn contexto_anterior(anteriores: &[Value]) -> Vec<Value> {
    let nombres: BTreeSet<String> = CONTEXTO.iter().map(|n| super::lecturas::normalizar(n)).collect();
    let mut ultimos: BTreeMap<String, ((String, i64), &Value)> = BTreeMap::new();
    for f in anteriores {
        let nombre = super::lecturas::normalizar(f["descripcion"].as_str().unwrap_or(""));
        let fecha = texto(&f["fecha_iso"]);
        if !nombres.contains(&nombre) || fecha.is_empty() {
            continue;
        }
        let folio = texto(&f["folio"]);
        let orden = (fecha, if !folio.is_empty() && folio.chars().all(|c| c.is_ascii_digit()) { folio.parse().unwrap_or(-1) } else { -1 });
        if ultimos.get(&nombre).is_none_or(|(o, _)| orden > *o) {
            ultimos.insert(nombre, (orden, f));
        }
    }
    let mut salida: Vec<Value> = ultimos.into_values().map(|(_, f)| f.clone()).collect();
    salida.sort_by_key(|f| f["fila"].as_u64().unwrap_or(0));
    salida
}

fn nombre_lote(manifiesto: &Path) -> String {
    let r = manifiesto.canonicalize().unwrap_or_else(|_| manifiesto.to_path_buf());
    r.parent().and_then(Path::parent).and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// `_historia`: una consulta por causa (alcance CAUSA, sin remote_identity) y una fila por trámite de la ventana.
fn exportar_historia(estado: &Value, causas: &[Causa], lote: &str, d: &mut Datos, capturada: &str) {
    d.tabla("historia_tramites");
    let mut vinculos: BTreeMap<String, &Value> = BTreeMap::new();
    for v in estado["vinculos"].as_array().into_iter().flatten() {
        vinculos.entry(texto(&v["clave"])).or_insert(v);
    }
    let mut fallos = estado["errores_listado"].as_array().cloned().unwrap_or_default();
    if estado["estado"] != "COMPLETA" && fallos.is_empty() {
        fallos = vec![json!({"error": "HISTORIA_ENUMERACION_INCOMPLETA"})];
    }
    let listado = json!({"tab": "HISTORIA", "capture_tables": ["historia_tramites"]});
    for n in 1..=fallos.len() {
        let qid = format!("hist-listado-{n}");
        d.consulta(&qid, &listado, "FALLIDA", capturada, "0".into(), 0, Some("historia_tramites"), "HISTORIA_ENUMERACION_FALLIDA");
        d.aviso(&qid, "HISTORIA_ENUMERACION_FALLIDA", "No se pudo enumerar toda la historia solicitada.");
    }
    if causas.is_empty() && fallos.is_empty() {
        d.consulta("hist-listado-vacio", &listado, "VACIA_COMPROBADA", capturada, "0".into(), 0, Some("historia_tramites"), "");
    }
    let vacio = json!({});
    for (n, item) in causas.iter().enumerate() {
        let n = n + 1;
        let causa = &item.causa;
        let qid = format!("hist-{n}");
        let cobertura = match causa["estado"].as_str().unwrap_or("") {
            "LEIDA" => "COMPLETA",
            "VACIA_COMPROBADA" => "VACIA_COMPROBADA",
            "IDENTIDAD_INCIERTA" => "IDENTIDAD_INCIERTA",
            "PENDIENTE" | "CANCELADA" => "CANCELADA",
            _ => "FALLIDA",
        };
        let primera = vinculos.get(&item.clave).copied().unwrap_or(&vacio);
        let seleccion = json!({
            "tribunal": causa["tribunal"], "tribunal_nombre": texto(&causa["tribunal_nombre"]), "modality": texto(&primera["modalidad"]),
            "tab": "HISTORIA", "capture_tables": ["historia_tramites"],
            "historia_causa": {"source_system": "RUS", "court_code": causa["tribunal"], "cause_id": causa["causa_id"]},
            "ventana": {
                "criterio": estado.get("criterio_temporal").cloned().unwrap_or(json!("ventana_4_meses")),
                "desde": texto(&causa["desde"]), "hasta": texto(&causa["hasta"]),
            },
        });
        let leida = texto(&causa["leida"]);
        let cuando = if leida.is_empty() { capturada.to_string() } else { leida };
        let ok = matches!(cobertura, "COMPLETA" | "VACIA_COMPROBADA");
        let error = if cobertura == "FALLIDA" { texto(&causa["error"]) } else { String::new() };
        let fila_q = d.consulta(&qid, &seleccion, cobertura, &cuando, "1".into(), usize::from(ok), Some("historia_tramites"), &error);
        let o = |defecto: &str| { let e = texto(&causa["error"]); if e.is_empty() { defecto.to_string() } else { e } };
        match cobertura {
            "FALLIDA" => d.aviso(&qid, "HISTORIA_FALLIDA", &o("No se pudo leer la historia.")),
            "IDENTIDAD_INCIERTA" => d.aviso(&qid, "HISTORIA_IDENTIDAD_INCIERTA", &o("Identidad de la causa no verificable.")),
            "CANCELADA" => d.aviso(&qid, "HISTORIA_NO_LEIDA", "La historia no se leyó (lote cancelado o con tope)."),
            _ => {}
        }
        for codigo in causa["advertencias"].as_array().into_iter().flatten() {
            d.aviso(&qid, &texto(codigo), "Advertencia del lector de historia.");
        }
        let q = |k: &str| texto(&fila_q[k]);
        for f in item.ventana.iter().cloned().chain(contexto_anterior(&item.anteriores)) {
            let s = |k: &str| texto(&f[k]);
            let base = comun(&[
                ("capture_id", d.cid.clone()), ("query_id", qid.clone()), ("source_system", "RUS".into()), ("court_code", q("court_code")),
                ("court_name", q("court_name")), ("rit_original", texto(&causa["rit"])), ("cause_id", texto(&causa["causa_id"])),
                ("modality_code", q("modality_code")), ("tab", texto(&primera["pestana"])), ("captured_at", cuando.clone()),
                ("original_file", format!("respaldo/{lote}/historia/{}", texto(&causa["archivo"]))), ("original_row", s("fila")),
                ("source_sha256", texto(&causa["sha256"])), ("coverage", "COMPLETA".into()),
            ]);
            let crudo = json!({"Folio": f["folio"], "Etapa": f["etapa"], "Trámite": f["tramite"], "Desc. Trámite": f["descripcion"],
                "Referencia": f["referencia"], "Fecha Tram": f["fecha_texto"]});
            let r = con(base, &[
                ("event_id", format!("{qid}-{}", s("fila"))), ("event_at", s("fecha_iso")),
                ("time_precision", if s("fecha_iso").is_empty() { "DESCONOCIDA" } else { "DIA" }.into()),
                ("time_zone", "DESCONOCIDA".into()), ("scope", "CAUSA".into()), ("event_type", "tramite".into()),
                ("description", s("descripcion")), ("reference", s("referencia")), ("identity_status", "CAUSA".into()),
                ("folio", s("folio")), ("stage", s("etapa")),
                ("annex_visible", if f["anexo"].as_bool().unwrap_or(false) { "true" } else { "false" }.into()),
                ("raw_json", json_python(&crudo, false)),
            ]);
            d.tabla("historia_tramites").push(r);
        }
    }
}

/// `_escritos`: bandeja de Escritos de cada causa leída (alcance CAUSA). Solo si la respuesta respaldada trae la
/// pestaña; presente pero ilegible deja la consulta FALLIDA, nunca vacía.
fn exportar_escritos(causas: &[Causa], lote: &str, d: &mut Datos, capturada: &str) {
    let (mut filas, mut consultas) = (Vec::new(), 0);
    for (n, item) in causas.iter().enumerate() {
        let n = n + 1;
        let causa = &item.causa;
        if item.escritos.is_none() && item.escritos_error.is_empty() {
            continue;
        }
        consultas += 1;
        let qid = format!("escritos-{n}");
        let cobertura = match &item.escritos {
            None => "FALLIDA",
            Some(l) if l["advertencias"].as_array().is_some_and(|a| !a.is_empty()) => "PARCIAL",
            Some(l) if l["vacio_explicito"].as_bool().unwrap_or(false) => "VACIA_COMPROBADA",
            Some(_) => "COMPLETA",
        };
        let seleccion = json!({
            "tribunal": causa["tribunal"], "tribunal_nombre": texto(&causa["tribunal_nombre"]), "tab": "ESCRITOS",
            "escritos_causa": {"source_system": "RUS", "court_code": causa["tribunal"], "cause_id": causa["causa_id"]},
        });
        let leida = texto(&causa["leida"]);
        let cuando = if leida.is_empty() { capturada.to_string() } else { leida };
        let fila_q = d.consulta(&qid, &seleccion, cobertura, &cuando, "1".into(), usize::from(item.escritos.is_some()), Some("escritos"),
            &item.escritos_error);
        let Some(lectura) = &item.escritos else {
            d.aviso(&qid, &item.escritos_error, "La pestaña Escritos de la causa no se pudo leer.");
            continue;
        };
        for a in lectura["advertencias"].as_array().into_iter().flatten() {
            d.aviso(&qid, &texto(&a["codigo"]), "Advertencia del lector de escritos.");
        }
        let q = |k: &str| texto(&fila_q[k]);
        for f in lectura["filas"].as_array().into_iter().flatten() {
            let s = |k: &str| texto(&f[k]);
            let base = comun(&[
                ("capture_id", d.cid.clone()), ("query_id", qid.clone()), ("source_system", "RUS".into()), ("court_code", q("court_code")),
                ("court_name", q("court_name")), ("rit_original", texto(&causa["rit"])), ("cause_id", texto(&causa["causa_id"])),
                ("modality_code", q("modality_code")), ("tab", "ESCRITOS".into()), ("captured_at", cuando.clone()),
                ("original_file", format!("respaldo/{lote}/historia/{}", texto(&causa["archivo"]))), ("original_row", s("fila")),
                ("source_sha256", texto(&causa["sha256"])), ("coverage", cobertura.into()),
            ]);
            filas.push(con(base, &[
                ("escrito_id", format!("{qid}-{}", s("fila"))), ("scope", "CAUSA".into()), ("doc", s("doc")), ("escrito_type", s("tipo")),
                ("requester", s("solicitante")), ("escrito_status", s("estado")), ("reference", String::new()),
                ("linked_ingreso_id", String::new()), ("linked_projection", String::new()), ("identity_status", "CAUSA".into()),
                ("raw_json", json_python(&f["literal"], false)),
            ]));
        }
    }
    if consultas > 0 {
        *d.tabla("escritos") = filas;
    }
}

// ---------------------------------------------------------------- histórico de protección

/// `_historico_pendientes`: intentos que quedaron solo en el control de bitácoras de la carpeta del histórico.
fn historico_pendientes(bitacoras: &[PathBuf], historico: &Path, entradas: Option<&[Value]>) -> bool {
    if bitacoras.is_empty() {
        return false;
    }
    let registrados: BTreeSet<(String, String)> =
        entradas.into_iter().flatten().map(|e| (texto(&e["tribunal"]), texto(&e["person_id"]))).collect();
    let carpeta = historico.canonicalize().unwrap_or_else(|_| historico.to_path_buf());
    for ruta in rutas_bitacoras(bitacoras) {
        if ruta.parent() != Some(carpeta.as_path()) {
            continue;
        }
        let Some(control) = std::fs::read(&ruta).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { continue };
        let registros = control["registros"].as_array().into_iter().flatten()
            .chain(control["anteriores_no_reenumeradas"].as_array().into_iter().flatten());
        for r in registros {
            let intento = &r["historico_lectura"];
            let hay = intento.as_object().is_some_and(|o| !o.is_empty());
            if hay && !registrados.contains(&(texto(&r["tribunal_codigo"]), texto(&intento["person_id"]))) {
                return true;
            }
        }
    }
    false
}

/// `_historico`: estados del histórico, con una incidencia cuando no hay respaldo verificable.
fn exportar_historico(entradas: Option<&[Value]>, d: &mut Datos, capturada: &str, pendientes: bool) {
    let vacias = entradas.is_none_or(<[Value]>::is_empty);
    if vacias || pendientes {
        let codigo = if entradas.is_none() { "HISTORICO_RESPALDO_INVALIDO" } else { "HISTORICO_PENDIENTE" };
        d.consulta("historico-respaldo", &json!({"tab": "HISTORICO"}), "FALLIDA", capturada, "0".into(), 0, Some("historico_proteccion"), codigo);
        d.aviso("historico-respaldo", codigo, "Hay lecturas del histórico sin respaldo verificable.");
        d.tabla("historico_proteccion").clear();
    }
    let mut filas = Vec::new();
    for (n, e) in entradas.into_iter().flatten().enumerate() {
        let qid = format!("historico-{}", n + 1);
        let mut cobertura = match e["estado"].as_str() {
            Some("LEIDA") => "COMPLETA",
            Some("VACIA_COMPROBADA") => "VACIA_COMPROBADA",
            _ => "FALLIDA",
        };
        let con_avisos = e["advertencias"].as_array().is_some_and(|a| !a.is_empty());
        if cobertura != "FALLIDA" && (con_avisos || e["ownership"] != "VERIFICADA") {
            cobertura = "PARCIAL";
        }
        let seleccion = json!({
            "tribunal": e["tribunal"], "tribunal_nombre": texto(&e["tribunal_nombre"]), "tab": "HISTORICO",
            "historico_persona": {"source_system": "RUS", "court_code": e["tribunal"], "person_id": e["person_id"]},
        });
        let leida = texto(&e["leida"]);
        let fila_q = d.consulta(&qid, &seleccion, cobertura, &leida, "1".into(), usize::from(cobertura != "FALLIDA"),
            Some("historico_proteccion"), &texto(&e["error"]));
        if cobertura == "FALLIDA" {
            let codigo = texto(&e["error"]);
            d.aviso(&qid, if codigo.is_empty() { "HISTORICO_NO_VERIFICADA" } else { &codigo }, "El histórico de protección no se pudo leer.");
            continue;
        }
        if e["ownership"] != "VERIFICADA" {
            d.aviso(&qid, "HISTORICO_PERTENENCIA_INCIERTA", "No se comprobó que la vista sea de la persona seleccionada.");
        }
        let q = |k: &str| texto(&fila_q[k]);
        for f in e["filas"].as_array().into_iter().flatten() {
            let s = |k: &str| texto(&f[k]);
            let base = comun(&[
                ("capture_id", d.cid.clone()), ("query_id", qid.clone()), ("source_system", "RUS".into()), ("court_code", q("court_code")),
                ("court_name", q("court_name")), ("person_id", texto(&e["person_id"])), ("modality_code", q("modality_code")),
                ("tab", "HISTORICO".into()), ("captured_at", leida.clone()),
                ("original_file", format!("respaldo/historico/{}", texto(&e["archivo"]))), ("original_row", s("fila")),
                ("source_sha256", texto(&e["sha256"])), ("coverage", cobertura.into()),
            ]);
            let crudo = json!({"Programa": f["programa"], "Fecha egreso": f["fecha_egreso"], "Causal egreso": f["causal_egreso"]});
            filas.push(con(base, &[
                ("historico_id", format!("{qid}-{}", s("fila"))), ("program_name", s("programa")), ("egreso_date", s("fecha_egreso")),
                ("egreso_cause", s("causal_egreso")), ("ownership", texto(&e["ownership"])), ("raw_json", json_python(&crudo, false)),
            ]));
        }
    }
    if !vacias {
        *d.tabla("historico_proteccion") = filas;
    }
}

/// `_persona_por_rut`: clave `rut-…` del histórico automático para una fila sin `SITFA_PERSONA_RUS`, solo si existe.
fn persona_por_rut(tribunal: &str, normalizado: &Fila, personas: &BTreeSet<(String, String)>) -> String {
    if personas.is_empty() {
        return String::new();
    }
    let valor = |k: &str| normalizado.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let rut = super::lecturas::rut_canonico(&Some(valor("RUT")).filter(|s| !s.is_empty()).unwrap_or_else(|| valor("RUT (->RCEI)")));
    let clave = if rut.is_empty() { String::new() } else { super::bitacoras::persona_por_rut(&rut) };
    if personas.contains(&(tribunal.to_string(), clave.clone())) { clave } else { String::new() }
}

/// `nurus.rus.columns.COMMON` y `MODE_COLUMNS` (Python, autoridad): alias por clave lógica.
const RUS_COMUNES: [(&str, &[&str]); 10] = [
    ("programa", &["DERIVACION", "DERIVACIÓN", "PROGRAMA", "NOMBRE CENTRO"]),
    ("tribunal", &["TRIBUNAL"]),
    ("nombre", &["NOMBRE", "NOMBRE COMPLETO", "NOMBRE MENOR"]),
    ("nacimiento", &["FEC. NACIMIENTO", "FEC.NACIMIENTO", "FECHA NACIMIENTO", "FEC NACIMIENTO"]),
    ("curador", &["CURADOR", "CURADOR AD LITEM", "CURADOR AD-LITEM", "CURADOR AD LITEM.", "CUR. AD LITEM", "CURADOR/A AD LITEM",
                  "CURADOR/A AD-LITEM"]),
    ("oido", &["FEC. OIDO", "FEC.OIDO", "FEC OIDO", "FECHAOIDO", "FEC. OÍDO", "FEC.OÍDO"]),
    ("resolucion", &["FEC. RESOLUCIÓN", "FEC.RESOLUCIÓN", "FEC. RESOLUCION", "FEC.RESOLUCION", "FECHA RESOLUCION", "FEC RESOLUCION"]),
    ("prox_aud", &["PROXS. AUDS.", "PROXS AUDS", "PROX AUD", "PROXIMA AUDIENCIA", "PRÓXIMA AUDIENCIA", "PROX. AUD.", "PROX. AUDS."]),
    ("rit", &["RIT"]),
    ("rut", &["RUT", "RUT MENOR", "RUT NNA", "RUT LITIGANTE"]),
];
const RUS_INGRESO: &[&str] = &["FEC.INGRESO EFECTIVO", "FEC. INGRESO EFECTIVO", "FEC INGRESO EFECTIVO"];

fn rus_modo(modo: &str) -> Option<Vec<(&'static str, &'static [&'static str])>> {
    Some(match modo {
        "ESPERA" => vec![("espera", &["T ESPERA", "T_ESPERA", "DIAS_ESPERA", "TESPERA", "DÍAS DE ESPERA", "DIAS DE ESPERA"][..])],
        "CUMPLIMIENTO" => vec![
            ("dias_cumpl", &["DIAS DE CUMPLIMIENTO", "DÍAS DE CUMPLIMIENTO", "DIAS CUMPLIMIENTO"][..]),
            ("dias_egresar", &["DIAS PARA EGRESAR", "DÍAS PARA EGRESAR", "DIAS EGRESAR"][..]),
            ("ingreso", RUS_INGRESO),
            ("egreso_proy", &["FEC.EGRESO PROYECTADO", "FEC. EGRESO PROYECTADO", "FEC EGRESO PROYECTADO"][..]),
            ("ficha_fae", &["FEC.ACT.F.FAE", "FEC. ACT. F. FAE", "FEC ACT F FAE", "FEC.ACT.F.FAE/FAS", "FEC. ACT. F. FAE/FAS"][..]),
            ("ficha_ind", &["FEC.ACT.F.INDIVIDUAL", "FEC. ACT. F. INDIVIDUAL", "FEC ACT F INDIVIDUAL"][..]),
        ],
        "INFORMES" => vec![
            ("vencimiento", &["FECHA VENCIMIENTO", "FEC.VENCIMIENTO", "FEC. VENCIMIENTO", "F. VENCIMIENTO", "F.VENCIMIENTO",
                              "FVENCIMIENTO", "FECHA DE VENCIMIENTO"][..]),
            ("ingreso", RUS_INGRESO),
        ],
        _ => return None,
    })
}

/// `columns.map_columns(headers, mode)`: cabecera elegida por clave; `None` si el modo no existe o si dos cabeceras
/// distintas compiten por la misma clave (`ColumnMappingError`).
fn columnas_rus(cabeceras: &[String], modo: &str) -> Option<BTreeMap<&'static str, String>> {
    let mut salida = BTreeMap::new();
    for (clave, alias) in RUS_COMUNES.into_iter().chain(rus_modo(modo)?) {
        let alias: Vec<String> = alias.iter().map(|a| super::lecturas::normalizar(a)).collect();
        let mut candidatos: Vec<&str> = Vec::new();
        for h in cabeceras.iter().map(|h| h.trim()) {
            if alias.contains(&super::lecturas::normalizar(h)) && !candidatos.contains(&h) {
                candidatos.push(h);
            }
        }
        match candidatos.as_slice() {
            [] => {}
            [h] => {
                salida.insert(clave, h.to_string());
            }
            _ => return None,
        }
    }
    Some(salida)
}

/// `resultados.parse_day`: día de una celda de texto en los formatos de SITFA.
fn dia(valor: &str) -> Option<chrono::NaiveDate> {
    let v = valor.trim();
    ["%d/%m/%Y", "%Y-%m-%d", "%d-%m-%Y"].iter().find_map(|f| chrono::NaiveDate::parse_from_str(v, f).ok())
}

/// `calendar_date`: vencimiento ISO de una fila de calendario de informes; vacío si no se reconoce.
fn fecha_calendario(normalizado: &Fila) -> Result<String, String> {
    let cabeceras: Vec<String> = normalizado.keys().cloned().collect();
    let columnas = columnas_rus(&cabeceras, "INFORMES").ok_or("El calendario de informes tiene columnas ambiguas.")?;
    Ok(columnas.get("vencimiento").and_then(|c| normalizado.get(c)).and_then(Value::as_str).and_then(dia)
        .map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default())
}

/// `asociar_calendarios`: un calendario de informes sin ids remotos se asocia a un ingreso verificado solo si
/// tribunal, RIT, nombre y programa coinciden con exactamente un ingreso de las demás tablas (nunca por RIT solo ni
/// con empate). Devuelve los `record_id` resueltos.
fn asociar_calendarios(d: &mut Datos) -> Vec<String> {
    const IDS: [&str; 4] = ["cause_id", "ingreso_id", "person_id", "program_id"];
    let clave = |tribunal: &str, normalizado_json: &Value, modo: &str| -> Option<[String; 4]> {
        let fila: Fila = serde_json::from_str(normalizado_json.as_str()?).ok()?;
        let cabeceras: Vec<String> = fila.keys().cloned().collect();
        let columnas = columnas_rus(&cabeceras, modo)?;
        let partes = ["rit", "nombre", "programa"].map(|k| {
            columnas.get(k).and_then(|c| fila.get(c)).map(|v| normalizado(&texto(v))).unwrap_or_default()
        });
        partes.iter().all(|p| !p.is_empty()).then(|| [tribunal.to_string(), partes[0].clone(), partes[1].clone(), partes[2].clone()])
    };
    let mut ingresos: BTreeMap<[String; 4], BTreeSet<[String; 4]>> = BTreeMap::new();
    for r in d.filas("registros") {
        let modo = texto(&r["record_type"]);
        if r["identity_status"] != "VERIFICADA" || matches!(modo.as_str(), "INFORMES" | "OTRO" | "EGRESADOS") {
            continue;
        }
        if let Some(k) = clave(&texto(&r["court_code"]), &r["normalized_json"], &modo) {
            ingresos.entry(k).or_default().insert(IDS.map(|c| texto(&r[c])));
        }
    }
    let fisica = |r: &Fila| ["query_id", "source_sha256", "original_row"].map(|k| texto(&r[k]));
    let mut cambios: Vec<(usize, Option<usize>, [String; 4])> = Vec::new();
    for (ci, c) in d.filas("calendarios").iter().enumerate() {
        if c["association"] == "VERIFICADA" {
            continue;
        }
        let Some(k) = clave(&texto(&c["court_code"]), &c["normalized_json"], "INFORMES") else { continue };
        let Some(candidatos) = ingresos.get(&k).filter(|s| s.len() == 1) else { continue };
        let registro = d.filas("registros").iter().position(|r| r["record_type"] == "INFORMES" && fisica(r) == fisica(c));
        cambios.push((ci, registro, candidatos.iter().next().unwrap().clone()));
    }
    let mut resueltos = Vec::new();
    for (ci, ri, ids) in cambios {
        let c = &mut d.tabla("calendarios")[ci];
        for (k, v) in IDS.iter().zip(&ids) {
            c.insert(k.to_string(), json!(v));
        }
        c.insert("association".into(), json!("VERIFICADA"));
        if let Some(ri) = ri {
            let r = &mut d.tabla("registros")[ri];
            for (k, v) in IDS.iter().zip(&ids) {
                r.insert(k.to_string(), json!(v));
            }
            r.insert("identity_status".into(), json!("VERIFICADA"));
            resueltos.push(texto(&r["record_id"]));
        }
    }
    resueltos
}

/// `export_capture(lots, destination)` para lotes de listados (E-Q5a). Devuelve la ruta del paquete.
pub fn exportar(carpetas: &[PathBuf], destino: &Path) -> Result<PathBuf, String> {
    exportar_con(carpetas, destino, &Opciones::default()).map(|e| e.ruta)
}

/// `package_current_lot`: la ruta de Resultados › Paquete. Bitácoras emparejadas por identidad verificada
/// (`match_diaries`), historia e histórico del lote de bitácoras y destino nuevo. Devuelve `{path, reuse[, historico_incompleto]}`.
#[allow(clippy::too_many_arguments)]
pub fn empaquetar_lote(
    carpeta: &Path,
    bitacoras: &Path,
    destino: &Path,
    historia: Option<&Path>,
    historico: Option<&Path>,
    meses: u32,
    avisos: &[(String, String)],
    consolidado: bool,
) -> Result<Value, String> {
    if destino.exists() {
        return Err("El destino ya existe; elige un nombre nuevo.".into());
    }
    let opciones = Opciones {
        bitacoras: vec![bitacoras.to_path_buf()],
        emparejar: true,
        historia: historia.map(Path::to_path_buf),
        historico: historico.map(Path::to_path_buf),
        meses,
        avisos: avisos.to_vec(),
        consolidado,
    };
    let e = exportar_con(&[carpeta.to_path_buf()], destino, &opciones)?;
    let resumen = e.reutilizacion.unwrap_or_else(|| "Sin bitácoras añadidas.".into());
    let mut r = json!({"path": e.ruta.to_string_lossy(), "reuse": resumen + &e.nota});
    if historico.is_some() {
        r["historico_incompleto"] = json!(e.historico_incompleto);
    }
    Ok(r)
}

/// `export_capture`: lotes de listados (E-Q5a) más bitácoras, calendario por ingreso, historia, Escritos e
/// histórico (E-Q5c), calendario de informes con su asociación y Carga (E-Q5b).
pub fn exportar_con(carpetas: &[PathBuf], destino: &Path, op: &Opciones) -> Result<Exportado, String> {
    let lotes: Vec<Lote> = carpetas.iter().map(|c| cargar_lote(c)).collect::<Result<_, _>>()?;
    if lotes.is_empty() {
        return Err("Selecciona un lote verificado.".into());
    }
    let reloj = chrono::Local::now().fixed_offset();
    let ahora = reloj.to_rfc3339_opts(chrono::SecondsFormat::Micros, false);
    let mut omision = None;
    let (consolidado, nota) = if op.consolidado {
        match super::resultados::libro_consolidado(&lotes, &reloj) {
            Ok(b) => (Some(b), " Incluye el XLSX consolidado.".to_string()),
            Err(e) => {
                omision = Some(format!("Sin XLSX consolidado: {e}"));
                (None, format!(" Sin XLSX consolidado: {e}"))
            }
        }
    } else {
        (None, String::new())
    };
    let capturada = lotes[0].resumen["fecha_hora"].as_str().map(str::to_string).filter(|s| !s.is_empty()).unwrap_or_else(|| ahora.clone());
    let mut huella = json_python(&Value::Array(lotes.iter().map(|l| l.resumen.clone()).collect()), true);
    if !op.bitacoras.is_empty() {
        let mut hashes = Vec::new();
        for r in rutas_bitacoras(&op.bitacoras) {
            let b = std::fs::read(&r).map_err(|_| format!("No se encuentra el control de bitácoras: {}. Recupera bitacoras.json con sus \
                HTML originales o excluye esa referencia del paquete.", r.display()))?;
            hashes.push(sha256(&b));
        }
        hashes.sort();
        huella += &json_python(&json!(hashes), false);
        huella += if op.emparejar { "MATCH" } else { "ALL" };
    }
    if let Some(h) = &op.historia {
        huella += &format!("HISTORIA{}", sha256(&std::fs::read(h).map_err(|_| "No se encuentra historias.json.".to_string())?));
    }
    let mut entradas_historico: Option<Vec<Value>> = None;
    if let Some(h) = &op.historico {
        let manifiesto = h.join(super::historico::MANIFIESTO);
        let estado = if !manifiesto.exists() {
            entradas_historico = Some(Vec::new());
            "AUSENTE".to_string()
        } else {
            match std::fs::read(&manifiesto).map_err(|e| e.to_string()).and_then(|b| super::historico::cargar(h, &ahora).map(|e| (sha256(&b), e))) {
                Ok((s, e)) => {
                    entradas_historico = Some(e);
                    s
                }
                Err(_) => "INVALIDO".to_string(),
            }
        };
        huella += &format!("HISTORICO{estado}");
    }
    let cid = format!("captura-{}", &sha256(huella.as_bytes())[..24]);
    let personas: BTreeSet<(String, String)> = entradas_historico.iter().flatten()
        .filter(|e| e["person_id"].as_str().is_some_and(|p| p.starts_with("rut-")))
        .map(|e| (texto(&e["tribunal"]), texto(&e["person_id"]))).collect();
    let mut d = Datos { cid: cid.clone(), tablas: ["consultas", "registros", "incidencias"].map(|t| (t.to_string(), Vec::new())).to_vec(), n: 0 };
    for (li, lote) in lotes.iter().enumerate() {
        let cuando = lote.resumen["fecha_hora"].as_str().filter(|s| !s.is_empty()).unwrap_or(&capturada).to_string();
        let mut indice: HashMap<String, Fila> = HashMap::new();
        for (qi, (item, m)) in lote.consultas.iter().enumerate() {
            let qid = format!("q{li}-{qi}");
            let seleccion = if item.get("seleccion").is_some_and(|s| !s.is_null()) { item["seleccion"].clone() } else { m["seleccion"].clone() };
            let estado = match m["estado"].as_str() {
                Some("VALIDADA") => "COMPLETA",
                Some("SIN_RESULTADOS") => "VACIA_COMPROBADA",
                Some("INCOMPLETA") => "PARCIAL",
                _ => "FALLIDA",
            };
            let archivos = m["archivos"].as_array().map(|a| a.iter().filter(|f| f["estado"] == "OK").count()).unwrap_or(0);
            let esperadas = m.get("paginas_esperadas").map(texto).unwrap_or_else(|| archivos.to_string());
            let q = d.consulta(&qid, &seleccion, estado, &cuando, esperadas, archivos, None, "");
            indice.insert(item["verificacion"].as_str().unwrap_or("").to_string(), q);
            if estado == "PARCIAL" {
                d.aviso(&qid, "CAPTURA_PARCIAL", "La consulta no tiene cobertura completa.");
            }
            if estado == "VACIA_COMPROBADA" && m["evidencia_omitida"].as_bool().unwrap_or(false) {
                d.incidencia(&qid, "VACIA_SIN_PDF", "Consulta sin registros; no se pidió pantallazo PDF.", "", "INFORMACION");
            }
        }
        let intentadas: Vec<Value> = lote.consultas.iter().map(|(i, _)| i.get("seleccion").cloned().unwrap_or(json!({}))).collect();
        for (pi, seleccion) in lote.resumen["plan"].as_array().into_iter().flatten().enumerate() {
            if !intentadas.contains(seleccion) {
                let qid = format!("q{li}-pendiente-{pi}");
                d.consulta(&qid, seleccion, "NO_CONSULTADA", &capturada, "0".into(), 0, None, "");
                d.aviso(&qid, "NO_CONSULTADA", "Consulta planificada no completada.");
            }
        }
        for f in tablas_lote(lote)? {
            let q = indice[&f.verificacion].clone();
            let nombre = f.item["archivo"].as_str().unwrap_or("").to_string();
            let huella_fuente = f.item["sha256"].as_str().unwrap_or("").to_string();
            let modo = modo_tabla(&f.seleccion);
            for (numero, valores) in &f.filas {
                let normalizado: Fila = f.cabecera.iter().cloned().zip(valores.iter().map(|v| json!(v))).collect();
                let bruto: Fila = f.cabecera_original.iter().cloned()
                    .zip(f.filas_originales[numero].iter().map(|v| json!(v))).collect();
                let n = |k: &str| normalizado.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                let qv = |k: &str| q[k].as_str().unwrap_or("").to_string();
                let persona = Some(n("SITFA_PERSONA_RUS")).filter(|p| !p.is_empty())
                    .unwrap_or_else(|| persona_por_rut(&qv("court_code"), &normalizado, &personas));
                let comun = comun(&[
                    ("capture_id", cid.clone()), ("query_id", qv("query_id")), ("source_system", "RUS".into()),
                    ("court_code", qv("court_code")), ("court_name", qv("court_name")), ("rit_original", n("RIT")),
                    ("cause_id", n("SITFA_CAUSA_RUS")), ("ingreso_id", n("SITFA_INGRESO_RUS")), ("person_id", persona),
                    ("program_id", n("SITFA_CENTRO_RUS")), ("modality_code", qv("modality_code")), ("modality_raw", n("MODALIDAD")),
                    ("tab", qv("tab")), ("captured_at", qv("captured_at")), ("original_file", nombre.clone()),
                    ("original_sheet", String::new()), ("original_row", numero.to_string()), ("source_sha256", huella_fuente.clone()),
                    ("coverage", qv("coverage")),
                ]);
                let rid = sha256(format!("{}|{huella_fuente}|{numero}", qv("query_id")).as_bytes())[..24].to_string();
                let verificada = ["court_code", "cause_id", "ingreso_id"].iter().all(|k| comun[*k].as_str().is_some_and(|s| !s.is_empty()));
                let r = con(comun.clone(), &[
                    ("record_id", rid.clone()), ("record_type", modo.into()),
                    ("identity_status", if verificada { "VERIFICADA" } else { "INCIERTA" }.into()),
                    ("raw_json", json_python(&Value::Object(bruto.clone()), false)),
                    ("normalized_json", json_python(&Value::Object(normalizado.clone()), false)),
                ]);
                d.tabla("registros").push(r);
                if !verificada {
                    d.incidencia(&qv("query_id"), "IDENTIDAD_INCIERTA",
                        "No hay ingreso remoto verificable; no se relacionan eventos por RIT/nombre.", &rid, "ADVERTENCIA");
                }
                if f.seleccion["screen"] == "calendario_informes" {
                    let crudo = json_python(&Value::Object(bruto.clone()), false);
                    let r = con(comun, &[
                        ("calendar_id", format!("cal-{rid}")), ("event_at", fecha_calendario(&normalizado)?),
                        ("event_type", "INFORME".into()), ("status", "SEGUN_FUENTE".into()),
                        ("association", if verificada { "VERIFICADA" } else { "NO_ASOCIADO" }.into()),
                        ("raw_json", crudo), ("normalized_json", json_python(&Value::Object(normalizado.clone()), false)),
                    ]);
                    d.tabla("calendarios").push(r);
                }
            }
        }
    }
    for (codigo, mensaje) in &op.avisos {
        let qid = d.filas("consultas").first().map(|q| texto(&q["query_id"])).unwrap_or_default();
        d.aviso(&qid, codigo, mensaje);
    }
    // El motivo viaja con el paquete: quien lo abra después sabe por qué no trae el libro consolidado.
    if let Some(motivo) = &omision {
        let qid = d.filas("consultas").first().map(|q| texto(&q["query_id"])).unwrap_or_default();
        d.aviso(&qid, "CONSOLIDADO_OMITIDO", motivo);
    }
    let resueltos = asociar_calendarios(&mut d);
    if !resueltos.is_empty() {
        d.tabla("incidencias").retain(|i| !(i["code"] == "IDENTIDAD_INCIERTA" && resueltos.contains(&texto(&i["record_id"]))));
    }
    let mut reutilizacion = None;
    if !op.bitacoras.is_empty() {
        d.tabla("bitacoras");
        let identidades: Option<BTreeSet<Clave>> = op.emparejar.then(|| d.filas("registros").iter()
            .filter(|r| r["identity_status"] == "VERIFICADA")
            .map(|r| ["court_code", "cause_id", "ingreso_id"].map(|k| texto(&r[k]))).collect());
        let lote = importar_lotes(&op.bitacoras, identidades.as_ref())?;
        let qid0 = d.filas("consultas").first().map(|q| texto(&q["query_id"])).unwrap_or_default();
        if op.emparejar {
            let mensaje = format!("Bitácoras conservadas por tribunal/causa/ingreso: {}; ajenas excluidas: {}. Se conservan fecha y \
                cobertura originales; no se acredita actualización remota.", lote.emparejadas, lote.excluidas);
            d.aviso(&qid0, "BITACORAS_REUTILIZADAS", &mensaje);
            reutilizacion = Some(mensaje);
        }
        if op.emparejar && lote.consultas.is_empty() {
            d.consulta("diary-sin-coincidencia", &json!({"capture_tables": ["bitacoras"]}), "IDENTIDAD_INCIERTA", &capturada, "0".into(), 0,
                Some("bitacoras"), "Las bitácoras disponibles no tienen correspondencia remota verificada con los registros seleccionados.");
        }
        let corte = chrono::NaiveDate::parse_from_str(&primeros(&capturada, 10), "%Y-%m-%d").unwrap_or_else(|_| chrono::Local::now().date_naive());
        let desde = primera_fecha(corte, op.meses)?.format("%Y-%m-%d").to_string();
        for (qi, s) in lote.consultas.iter().enumerate() {
            let qid = format!("diary-{qi}");
            let mut seleccion = json!({
                "tribunal": s["tribunal_codigo"], "tab": "BITACORAS", "capture_tables": ["bitacoras"],
                "remote_identity": {"source_system": "RUS", "court_code": s["tribunal_codigo"], "cause_id": s["causa_id"], "ingreso_id": s["ingreso_id"]},
            });
            let capturada_s = texto(&s["captured_at"]);
            let cuando = if capturada_s.is_empty() { capturada.clone() } else { capturada_s.clone() };
            let alcance = texto(&s["alcance"]);
            let avisos = s["warnings"].as_array().cloned().unwrap_or_default();
            if s["coverage"] != "FALLIDA" && !alcance.is_empty() {
                seleccion["revision_bitacoras"] = json!({
                    "version": 2, "desde": if op.meses < 4 { desde.as_str() } else { "1900-01-01" }, "hasta": primeros(&cuando, 10),
                    "metodo": texto(&s["alcance_metodo"]), "coverage": alcance,
                    "completa": matches!(alcance.as_str(), "COMPLETA" | "VACIA_COMPROBADA") && avisos.is_empty(),
                    "captured_at": capturada_s, "warnings": avisos.len(),
                });
            }
            let cobertura = texto(&s["coverage"]);
            d.consulta(&qid, &seleccion, &cobertura, &cuando, "1".into(), 1, Some("bitacoras"), "");
            if cobertura == "FALLIDA" {
                d.aviso(&qid, "BITACORA_FALLIDA", &texto(&s["error"]));
            }
            for w in &avisos {
                d.aviso(&qid, "BITACORA_ADVERTENCIA", &texto(w));
            }
            for e in s["entries"].as_array().into_iter().flatten() {
                let v = |k: &str| texto(&e[k]);
                let fecha = v("fecha");
                if op.meses < 4 && primeros(&fecha, 10) < desde {
                    continue;
                }
                let base = comun(&[
                    ("capture_id", cid.clone()), ("query_id", qid.clone()), ("source_system", "RUS".into()),
                    ("court_code", texto(&s["tribunal_codigo"])), ("rit_original", v("rit")), ("cause_id", v("causa_id")),
                    ("ingreso_id", v("ingreso_id")), ("captured_at", cuando.clone()), ("original_file", v("fuente")),
                    ("original_sheet", "TablaInforme".into()), ("original_row", v("fila_fuente")),
                    ("source_sha256", texto(&s["html_sha256"])), ("coverage", cobertura.clone()),
                ]);
                let solo_dia = fecha.ends_with("T00:00:00");
                let r = con(base, &[
                    ("event_id", format!("{qid}-{}", v("entry_id"))),
                    ("event_at", if solo_dia { primeros(&fecha, 10) } else { fecha.clone() }),
                    ("time_precision", if solo_dia { "DIA" } else { "SEGUNDO" }.into()), ("time_zone", "DESCONOCIDA".into()),
                    ("origin", "CENTRO".into()), ("origin_verified", "true".into()), ("author", v("autor")), ("event_type", v("tipo")),
                    ("text", v("texto")), ("response", v("respuesta")), ("response_at", v("fecha_respuesta")),
                    ("identity_status", "VERIFICADA".into()),
                ]);
                d.tabla("bitacoras").push(r);
            }
        }
        for (ci, c) in lote.calendarios.iter().enumerate() {
            let qid = format!("calendar-{ci}");
            let seleccion = json!({
                "tribunal": c["tribunal_codigo"], "tab": "CALENDARIO", "capture_tables": ["calendarios"],
                "remote_identity": {"source_system": "RUS", "court_code": c["tribunal_codigo"], "cause_id": c["causa_id"], "ingreso_id": c["ingreso_id"]},
            });
            let capturada_c = texto(&c["captured_at"]);
            let cuando = if capturada_c.is_empty() { capturada.clone() } else { capturada_c };
            let cobertura = texto(&c["coverage"]);
            let fallida = cobertura == "FALLIDA";
            d.consulta(&qid, &seleccion, &cobertura, &cuando, "1".into(), usize::from(!fallida), Some("calendario_ingreso"), &texto(&c["error"]));
            if fallida {
                d.aviso(&qid, "CALENDARIO_FALLIDO", &texto(&c["error"]));
                continue;
            }
            d.tabla("calendarios");
            for p in c["proyecciones"].as_array().into_iter().flatten() {
                let base = comun(&[
                    ("capture_id", cid.clone()), ("query_id", qid.clone()), ("source_system", "RUS".into()),
                    ("court_code", texto(&c["tribunal_codigo"])), ("cause_id", texto(&c["causa_id"])), ("ingreso_id", texto(&c["ingreso_id"])),
                    ("captured_at", cuando.clone()), ("original_file", texto(&c["archivo"])), ("original_sheet", "Calendario".into()),
                    ("original_row", texto(&p["fila"])), ("source_sha256", texto(&c["html_sha256"])), ("coverage", "COMPLETA".into()),
                ]);
                let mut valores = p.clone();
                if let Some(o) = valores.as_object_mut() {
                    for k in ["plazo_remision_meses", "plazo_intervencion_meses"] {
                        o.insert(k.into(), c.get(k).cloned().unwrap_or(Value::Null));
                    }
                }
                let valores = json_python(&valores, false);
                let r = con(base, &[
                    ("calendar_id", format!("{qid}-{}", texto(&p["fila"]))), ("event_at", texto(&p["fecha_proyeccion"])),
                    ("event_type", "INFORME_PROYECTADO".into()), ("status", texto(&p["estado"])), ("association", "VERIFICADA".into()),
                    ("raw_json", valores.clone()), ("normalized_json", valores),
                ]);
                d.tabla("calendarios").push(r);
            }
        }
    }
    if let Some(h) = &op.historia {
        let (estado, causas) = cargar_historias(h)?;
        let lote = nombre_lote(h);
        exportar_historia(&estado, &causas, &lote, &mut d, &capturada);
        let (_, causas) = cargar_historias(h)?;
        exportar_escritos(&causas, &lote, &mut d, &capturada);
    }
    if let Some(h) = &op.historico {
        let pendientes = historico_pendientes(&op.bitacoras, h, entradas_historico.as_deref());
        exportar_historico(entradas_historico.as_deref(), &mut d, &capturada, pendientes);
    }
    let cobertura_de = |d: &Datos, pantallas: &[&str]| agregar(d.filas("consultas").iter()
        .filter(|q| pantallas.contains(&q["screen"].as_str().unwrap_or("")))
        .map(|q| q["coverage"].as_str().unwrap_or("")));
    let mut estados: Map<String, Value> = TABLAS.iter().map(|t| (t.to_string(), json!("NO_SOLICITADA"))).collect();
    let excluidas = ["bitacoras", "calendario_ingreso", "historia_tramites", "escritos", "historico_proteccion"];
    estados.insert("consultas".into(), json!(agregar(d.filas("consultas").iter().map(|q| q["coverage"].as_str().unwrap_or("")))));
    estados.insert("registros".into(), json!(agregar(d.filas("consultas").iter()
        .filter(|q| !excluidas.contains(&q["screen"].as_str().unwrap_or("")))
        .map(|q| q["coverage"].as_str().unwrap_or("")))));
    estados.insert("incidencias".into(), json!(if d.filas("incidencias").is_empty() { "VACIA_COMPROBADA" } else { "COMPLETA" }));
    for (nombre, pantallas) in [("calendarios", &["calendario_informes", "calendario_ingreso"][..]), ("bitacoras", &["bitacoras"][..])] {
        let hay = d.filas("consultas").iter().any(|q| pantallas.contains(&q["screen"].as_str().unwrap_or("")));
        if d.tiene(nombre) || hay {
            let estado = cobertura_de(&d, pantallas);
            let estado = if d.filas(nombre).is_empty() && estado == "COMPLETA" { "VACIA_COMPROBADA".to_string() } else { estado };
            estados.insert(nombre.into(), json!(estado));
            d.tabla(nombre);
        }
    }
    if op.historia.is_some() {
        let estado = cobertura_de(&d, &["historia_tramites"]);
        let estado = if d.filas("historia_tramites").is_empty() && estado == "COMPLETA" { "VACIA_COMPROBADA".to_string() } else { estado };
        estados.insert("historia_tramites".into(), json!(estado));
    }
    if d.tiene("historico_proteccion") {
        estados.insert("historico_proteccion".into(), json!(cobertura_de(&d, &["historico_proteccion"])));
    }
    if d.tiene("escritos") {
        estados.insert("escritos".into(), json!(cobertura_de(&d, &["escritos"])));
    }
    let historico_incompleto = d.filas("consultas").iter()
        .any(|q| q["screen"] == "historico_proteccion" && !matches!(q["coverage"].as_str(), Some("COMPLETA" | "VACIA_COMPROBADA")));
    let metadatos = json!({
        "capture_id": cid, "captured_at": capturada,
        "downloader": {"version": super::extension::version(super::Edicion::Integral), "commit": "desarrollo"},
        "contains_sensitive_data": true, "originals_authorized": false, "queries": d.filas("consultas"), "table_states": estados,
    });
    let ruta = escribir_captura_con(destino, metadatos, &d.tablas, consolidado.as_deref())?;
    Ok(Exportado { ruta, nota, reutilizacion, historico_incompleto })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::io::Read;

    fn reconstruir(destino: &Path, archivos: &Value) {
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    fn contenido(zip: &Path) -> BTreeMap<String, Value> {
        let mut z = zip::ZipArchive::new(std::fs::File::open(zip).unwrap()).unwrap();
        let mut salida = BTreeMap::new();
        for i in 0..z.len() {
            let mut f = z.by_index(i).unwrap();
            let nombre = f.name().to_string();
            if nombre != "manifest.json" && !nombre.starts_with("data/") {
                continue;
            }
            let mut texto = String::new();
            f.read_to_string(&mut texto).unwrap();
            if nombre == "manifest.json" {
                salida.insert(nombre, serde_json::from_str(&texto).unwrap());
            } else if nombre.starts_with("data/") {
                salida.insert(nombre, json!(texto));
            }
        }
        salida
    }

    /// E-Q5d: con `consolidado`, el paquete de lotes aptos para CSMP lleva `consolidado/consolidado.xlsx` (1.1.0); las
    /// tablas son las mismas que sin él (oráculo de Python) y el libro es el «Para CSMP» de `exportar_libro`.
    #[test]
    fn paquete_con_consolidado() {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/paquete.json")).unwrap();
        let e = datos["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "calendario_informes").unwrap();
        let base = std::env::temp_dir().join(format!("flow-eq5d-consolidado-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let mut carpetas = Vec::new();
        for l in std::iter::once(&e["lote"]).chain(e["otros_lotes"].as_array().unwrap()) {
            let c = base.join("lotes").join(l["carpeta"].as_str().unwrap());
            reconstruir(&c, &l["archivos"]);
            carpetas.push(c);
        }
        let destino = base.join("paquete.zip");
        let r = exportar_con(&carpetas, &destino, &Opciones { consolidado: true, ..Default::default() }).unwrap();
        assert_eq!(r.nota, " Incluye el XLSX consolidado.");
        if let Some(salida) = std::env::var_os("FLOW_PAQUETE_SALIDA") {
            std::fs::create_dir_all(&salida).unwrap();
            std::fs::copy(&destino, PathBuf::from(salida).join("consolidado.zip")).unwrap();
        }
        let obtenido = contenido(&destino);
        let manifiesto = &obtenido["manifest.json"];
        assert_eq!(manifiesto["schema_version"], VERSION_CONSOLIDADA);
        let libro = manifiesto["files"].as_array().unwrap().iter().find(|f| f["type"] == "consolidated_workbook").unwrap();
        assert_eq!(libro["path"], RUTA_CONSOLIDADO);
        for (k, v) in e["paquete"].as_object().unwrap() {
            if k.starts_with("data/") {
                assert_eq!(&obtenido[k], v, "{k} cambia con el consolidado");
            }
        }
        let mut z = zip::ZipArchive::new(std::fs::File::open(&destino).unwrap()).unwrap();
        let mut bytes = Vec::new();
        z.by_name(RUTA_CONSOLIDADO).unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(libro["sha256"], json!(sha256(&bytes)));
        let hojas = crate::descargador::excel::volcado::volcar(&bytes, "");
        let titulos: Vec<&str> = hojas["hojas"].as_array().unwrap().iter().map(|h| h["titulo"].as_str().unwrap()).collect();
        assert_eq!(titulos, ["Resumen", "SITFA_ARCHIVOS", "CUMPLIMIENTO", "ESPERA", "INFORMES", "Vencimientos"]);
        // Lote no apto: el paquete sale sin libro y la nota da el motivo.
        let listados = datos["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "listados").unwrap();
        let no_apto = base.join("no-apto").join(listados["lote"]["carpeta"].as_str().unwrap());
        reconstruir(&no_apto, &listados["lote"]["archivos"]);
        let otro = base.join("sin-libro.zip");
        let r = exportar_con(&[no_apto], &otro, &Opciones { consolidado: true, ..Default::default() }).unwrap();
        assert_eq!(contenido(&otro)["manifest.json"]["schema_version"], VERSION);
        assert!(r.nota.starts_with(" Sin XLSX consolidado: "), "{}", r.nota);
        // El motivo queda dentro del paquete como incidencia; sin el consolidado pedido no hay incidencia.
        let captura = crate::captura::leer_captura(&std::fs::read(&otro).unwrap()).unwrap();
        let omitidas: Vec<String> = captura.filas("incidencias").filter(|i| i.get("code") == "CONSOLIDADO_OMITIDO").map(|i| i.get("message").to_string()).collect();
        assert_eq!(omitidas, [r.nota.trim_start().to_string()]);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// E-Q5: el paquete de Rust desde las carpetas que armó Flow Python trae el mismo `manifest.json` y los mismos
    /// CSV, byte a byte, que `export_capture` / `package_current_lot` de Python (`app/qa/generar_paquete.py`), incluido el
    /// escenario con bitácoras, calendario por ingreso, historia, Escritos e histórico.
    #[test]
    fn paquete_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/paquete.json")).unwrap();
        let escenarios = datos["escenarios"].as_array().unwrap();
        let (mut iguales, mut fallos) = (0usize, Vec::new());
        for e in escenarios {
            let nombre = e["escenario"].as_str().unwrap();
            let base = std::env::temp_dir().join(format!("flow-eq5-paquete-{}-{nombre}", std::process::id()));
            let _ = std::fs::remove_dir_all(&base);
            let carpeta = base.join("lotes").join(e["lote"]["carpeta"].as_str().unwrap());
            reconstruir(&carpeta, &e["lote"]["archivos"]);
            let destino = base.join("paquete.zip");
            let mut diferencias = Vec::new();
            // Escenario de contexto: lote de bitácoras (calendario, historia e histórico) y `package_current_lot`.
            let salida = if e.get("bitacoras").is_some() {
                let bitacoras = base.join("bitacoras").join(e["bitacoras"]["carpeta"].as_str().unwrap());
                reconstruir(&bitacoras, &e["bitacoras"]["archivos"]);
                empaquetar_lote(&carpeta, &bitacoras.join("bitacoras.json"), &destino, Some(&bitacoras.join("historia").join("historias.json")),
                    Some(&bitacoras), 4, &[], false)
                    .map(|mut r| {
                        r.as_object_mut().unwrap().remove("path");
                        if r != e["resultado"] {
                            diferencias.push(format!("resultado:\n    Rust   {r}\n    Python {}", e["resultado"]));
                        }
                    })
            } else {
                let mut carpetas = vec![carpeta.clone()];
                for otro in e["otros_lotes"].as_array().into_iter().flatten() {
                    let c = base.join("lotes").join(otro["carpeta"].as_str().unwrap());
                    reconstruir(&c, &otro["archivos"]);
                    carpetas.push(c);
                }
                exportar(&carpetas, &destino).map(|_| ())
            };
            match salida {
                Err(err) => diferencias.push(format!("exportar: {err}")),
                Ok(_) => {
                    // Con FLOW_PAQUETE_SALIDA se conserva el ZIP de Rust para abrirlo con Flow Python (app/qa).
                    if let Some(salida) = std::env::var_os("FLOW_PAQUETE_SALIDA") {
                        let salida = PathBuf::from(salida);
                        std::fs::create_dir_all(&salida).unwrap();
                        std::fs::copy(&destino, salida.join(format!("{nombre}.zip"))).unwrap();
                    }
                    let obtenido = contenido(&destino);
                    let esperado: BTreeMap<String, Value> =
                        e["paquete"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    for clave in obtenido.keys().chain(esperado.keys()).collect::<BTreeSet<_>>() {
                        if obtenido.get(clave) != esperado.get(clave) {
                            diferencias.push(format!("{clave}:\n    Rust   {:?}\n    Python {:?}", obtenido.get(clave), esperado.get(clave)));
                        }
                    }
                }
            }
            let _ = std::fs::remove_dir_all(&base);
            if diferencias.is_empty() {
                iguales += 1;
            } else {
                fallos.push(format!("{nombre}:\n  {}", diferencias.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("PAQUETE: {iguales}/{}, fallos {}", escenarios.len(), fallos.len());
        assert!(fallos.is_empty(), "{} escenarios difieren de Flow Python", fallos.len());
    }
}
