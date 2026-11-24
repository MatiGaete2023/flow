//! E-Q5 · Paquete `CSMP_CAPTURE` desde las carpetas del Descargador, con la salida de Flow Python:
//! `nurus.capture.contract.write_capture`, `resultados.load_lot/tables` y `export_csmp_capture.export_capture`.
//! Lo abre el Asistente (Trabajo) igual que un paquete de Python.
//!
//! E-Q5a: lotes de listados de Seguimiento (consultas, registros e incidencias). Pendiente en las partes siguientes:
//! Carga, calendario de informes y su asociación, bitácoras, calendario por ingreso, historia, Escritos e histórico.

use super::xls;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const VERSION: &str = "1.0.0";
pub const VERSION_EXTENDIDA: &str = "1.2.0";
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
pub fn escribir_captura(destino: &Path, mut metadatos: Value, tablas: &[(String, Vec<Map<String, Value>>)]) -> Result<PathBuf, String> {
    if destino.exists() {
        return Err("El paquete de destino ya existe.".into());
    }
    let extendida = tablas.iter().any(|(t, _)| EXTENDIDAS.contains(&t.as_str()));
    metadatos["schema_id"] = json!("CSMP_CAPTURE");
    metadatos["schema_version"] = json!(if extendida { VERSION_EXTENDIDA } else { VERSION });
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

fn leer_json(ruta: &Path) -> Result<Value, String> {
    let datos = std::fs::read(ruta).map_err(|_| format!("Falta {}.", ruta.file_name().and_then(|n| n.to_str()).unwrap_or("")))?;
    serde_json::from_slice(&datos).map_err(|_| "Un JSON del lote no se pudo leer.".to_string())
}

/// `inside`: un nombre del manifiesto solo puede ser un archivo de la carpeta del lote.
fn dentro(carpeta: &Path, nombre: &str) -> Result<PathBuf, String> {
    if nombre.is_empty() || Path::new(nombre).file_name().and_then(|n| n.to_str()) != Some(nombre) || nombre.contains('\\') {
        return Err("El manifiesto contiene una ruta fuera del lote.".into());
    }
    Ok(carpeta.join(nombre))
}

/// `check_file`: el archivo verificado sigue íntegro (tamaño y huella).
fn comprobar(carpeta: &Path, item: &Value) -> Result<Vec<u8>, String> {
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
}

/// `load_lot(folder, partial=False)`: un lote VALIDADA completo, con sus manifiestos y archivos íntegros.
pub fn cargar_lote(carpeta: &Path) -> Result<Lote, String> {
    let carpeta = carpeta.canonicalize().map_err(|e| format!("No se pudo abrir el lote: {e}"))?;
    let resumen = leer_json(&carpeta.join("resumen.json"))?;
    if resumen["estado"] != "VALIDADA" {
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
    if Some(consultas.len() as u64) != resumen["consultas_esperadas"].as_u64() {
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
    Ok(Lote { carpeta, resumen, consultas })
}

// ---------------------------------------------------------------- tablas (`resultados.tables`)

type Filas = Vec<(usize, Vec<String>)>;

fn normalizado(s: &str) -> String {
    super::sitfa::registros::normalizar(s)
}

/// `resultados.ALIASES` / `mapping`.
fn mapeo(cabecera: &[String]) -> Result<BTreeMap<&'static str, usize>, String> {
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
fn sin_parentesis(texto: &str) -> String {
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
struct Fuente {
    verificacion: String,
    seleccion: Value,
    item: Value,
    cabecera: Vec<String>,
    filas: Filas,
    cabecera_original: Vec<String>,
    filas_originales: BTreeMap<usize, Vec<String>>,
}

/// `resultados.tables(lot)` para Seguimiento y calendarios (Carga es de E-Q5b).
fn tablas_lote(lote: &Lote) -> Result<Vec<Fuente>, String> {
    let mut salida = Vec::new();
    let mut cuenta = 0usize;
    for (q, m) in &lote.consultas {
        for item in m["archivos"].as_array().into_iter().flatten().filter(|i| i["estado"] == "OK") {
            let datos = comprobar(&lote.carpeta, item)?;
            let seleccion = if q.get("seleccion").is_some_and(|s| !s.is_null()) { q["seleccion"].clone() } else { m["seleccion"].clone() };
            if seleccion["screen"] == "carga" {
                return Err("E-Q5: el paquete desde lotes de Carga todavía no está disponible en Flow Rust.".into());
            }
            let libro = xls::leer(&datos)?;
            let (cabecera_original, filas_originales) = cabecera_y_filas(&libro)?;
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
            salida.push(Fuente {
                verificacion: q["verificacion"].as_str().unwrap_or("").to_string(),
                seleccion,
                item: item.clone(),
                cabecera,
                filas,
                cabecera_original,
                filas_originales: filas_originales.into_iter().collect(),
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

fn fila(pares: &[(&str, String)]) -> Map<String, Value> {
    pares.iter().map(|(k, v)| (k.to_string(), json!(v))).collect()
}

/// `export_capture(lots, destination)` para lotes de listados (E-Q5a). Devuelve la ruta del paquete.
pub fn exportar(carpetas: &[PathBuf], destino: &Path) -> Result<PathBuf, String> {
    let lotes: Vec<Lote> = carpetas.iter().map(|c| cargar_lote(c)).collect::<Result<_, _>>()?;
    if lotes.is_empty() {
        return Err("Selecciona un lote verificado.".into());
    }
    let capturada = lotes[0].resumen["fecha_hora"].as_str().map(str::to_string).filter(|s| !s.is_empty())
        .unwrap_or_else(|| chrono::Local::now().to_rfc3339());
    let huella = json_python(&Value::Array(lotes.iter().map(|l| l.resumen.clone()).collect()), true);
    let cid = format!("captura-{}", &sha256(huella.as_bytes())[..24]);
    let mut consultas: Vec<Map<String, Value>> = Vec::new();
    let mut registros: Vec<Map<String, Value>> = Vec::new();
    let mut incidencias: Vec<Map<String, Value>> = Vec::new();
    let mut n_incidencia = 0usize;
    let mut incidencia = |incidencias: &mut Vec<Map<String, Value>>, qid: &str, codigo: &str, mensaje: &str, rid: &str, severidad: &str| {
        n_incidencia += 1;
        incidencias.push(fila(&[("capture_id", cid.clone()), ("query_id", qid.into()), ("record_id", rid.into()),
            ("incident_id", format!("i{n_incidencia}")), ("phase", "EXPORTAR".into()), ("code", codigo.into()),
            ("severity", severidad.into()), ("message", mensaje.into()), ("recoverable", "true".into())]));
    };
    let consulta = |qid: &str, seleccion: &Value, estado: &str, cuando: &str, esperadas: String, guardadas: usize, pantalla: Option<&str>, error: &str| {
        let mut r: Map<String, Value> = columnas("consultas").unwrap().into_iter().map(|c| (c.to_string(), json!(""))).collect();
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
        r
    };
    for (li, lote) in lotes.iter().enumerate() {
        let cuando = lote.resumen["fecha_hora"].as_str().filter(|s| !s.is_empty()).unwrap_or(&capturada).to_string();
        let mut indice: HashMap<String, usize> = HashMap::new();
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
            consultas.push(consulta(&qid, &seleccion, estado, &cuando, esperadas, archivos, None, ""));
            indice.insert(item["verificacion"].as_str().unwrap_or("").to_string(), consultas.len() - 1);
            if estado == "PARCIAL" {
                incidencia(&mut incidencias, &qid, "CAPTURA_PARCIAL", "La consulta no tiene cobertura completa.", "", "ADVERTENCIA");
            }
            if estado == "VACIA_COMPROBADA" && m["evidencia_omitida"].as_bool().unwrap_or(false) {
                incidencia(&mut incidencias, &qid, "VACIA_SIN_PDF", "Consulta sin registros; no se pidió pantallazo PDF.", "", "INFORMACION");
            }
        }
        let intentadas: Vec<Value> = lote.consultas.iter().map(|(i, _)| i.get("seleccion").cloned().unwrap_or(json!({}))).collect();
        for (pi, seleccion) in lote.resumen["plan"].as_array().into_iter().flatten().enumerate() {
            if !intentadas.contains(seleccion) {
                let qid = format!("q{li}-pendiente-{pi}");
                consultas.push(consulta(&qid, seleccion, "NO_CONSULTADA", &capturada, "0".into(), 0, None, ""));
                incidencia(&mut incidencias, &qid, "NO_CONSULTADA", "Consulta planificada no completada.", "", "ADVERTENCIA");
            }
        }
        for f in tablas_lote(lote)? {
            let q = consultas[indice[&f.verificacion]].clone();
            let nombre = f.item["archivo"].as_str().unwrap_or("").to_string();
            let huella_fuente = f.item["sha256"].as_str().unwrap_or("").to_string();
            let modo = modo_tabla(&f.seleccion);
            for (numero, valores) in &f.filas {
                let normalizado: Map<String, Value> = f.cabecera.iter().cloned().zip(valores.iter().map(|v| json!(v))).collect();
                let bruto: Map<String, Value> = f.cabecera_original.iter().cloned()
                    .zip(f.filas_originales[numero].iter().map(|v| json!(v))).collect();
                let n = |k: &str| normalizado.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                let qv = |k: &str| q[k].as_str().unwrap_or("").to_string();
                let mut comun: Map<String, Value> = COMUNES.iter().map(|c| (c.to_string(), json!(""))).collect();
                for (k, v) in [
                    ("capture_id", cid.clone()), ("query_id", qv("query_id")), ("source_system", "RUS".into()),
                    ("court_code", qv("court_code")), ("court_name", qv("court_name")), ("rit_original", n("RIT")),
                    ("cause_id", n("SITFA_CAUSA_RUS")), ("ingreso_id", n("SITFA_INGRESO_RUS")), ("person_id", n("SITFA_PERSONA_RUS")),
                    ("program_id", n("SITFA_CENTRO_RUS")), ("modality_code", qv("modality_code")), ("modality_raw", n("MODALIDAD")),
                    ("tab", qv("tab")), ("captured_at", qv("captured_at")), ("original_file", nombre.clone()),
                    ("original_sheet", String::new()), ("original_row", numero.to_string()), ("source_sha256", huella_fuente.clone()),
                    ("coverage", qv("coverage")),
                ] {
                    comun.insert(k.into(), json!(v));
                }
                let rid = sha256(format!("{}|{huella_fuente}|{numero}", qv("query_id")).as_bytes())[..24].to_string();
                let verificada = ["court_code", "cause_id", "ingreso_id"].iter().all(|k| comun[*k].as_str().is_some_and(|s| !s.is_empty()));
                let mut r = comun.clone();
                for (k, v) in [
                    ("record_id", rid.clone()), ("record_type", modo.into()),
                    ("identity_status", if verificada { "VERIFICADA" } else { "INCIERTA" }.into()),
                    ("raw_json", json_python(&Value::Object(bruto.clone()), false)),
                    ("normalized_json", json_python(&Value::Object(normalizado.clone()), false)),
                ] {
                    r.insert(k.into(), json!(v));
                }
                registros.push(r);
                if !verificada {
                    incidencia(&mut incidencias, &qv("query_id"), "IDENTIDAD_INCIERTA",
                        "No hay ingreso remoto verificable; no se relacionan eventos por RIT/nombre.", &rid, "ADVERTENCIA");
                }
                if f.seleccion["screen"] == "calendario_informes" {
                    return Err("E-Q5: el paquete con calendario de informes todavía no está disponible en Flow Rust.".into());
                }
            }
        }
    }
    let mut estados: Map<String, Value> = TABLAS.iter().map(|t| (t.to_string(), json!("NO_SOLICITADA"))).collect();
    let excluidas = ["bitacoras", "calendario_ingreso", "historia_tramites", "escritos", "historico_proteccion"];
    estados.insert("consultas".into(), json!(agregar(consultas.iter().map(|q| q["coverage"].as_str().unwrap_or("")))));
    estados.insert("registros".into(), json!(agregar(consultas.iter()
        .filter(|q| !excluidas.contains(&q["screen"].as_str().unwrap_or("")))
        .map(|q| q["coverage"].as_str().unwrap_or("")))));
    estados.insert("incidencias".into(), json!(if incidencias.is_empty() { "VACIA_COMPROBADA" } else { "COMPLETA" }));
    let metadatos = json!({
        "capture_id": cid, "captured_at": capturada,
        "downloader": {"version": super::extension::version(super::Edicion::Integral), "commit": "desarrollo"},
        "contains_sensitive_data": true, "originals_authorized": false, "queries": consultas, "table_states": estados,
    });
    let tablas = vec![("consultas".to_string(), consultas), ("registros".to_string(), registros), ("incidencias".to_string(), incidencias)];
    escribir_captura(destino, metadatos, &tablas)
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
            let mut texto = String::new();
            f.read_to_string(&mut texto).unwrap();
            let nombre = f.name().to_string();
            if nombre == "manifest.json" {
                salida.insert(nombre, serde_json::from_str(&texto).unwrap());
            } else if nombre.starts_with("data/") {
                salida.insert(nombre, json!(texto));
            }
        }
        salida
    }

    /// E-Q5: el paquete de Rust desde las carpetas que armó Flow Python trae el mismo `manifest.json` y los mismos
    /// CSV, byte a byte, que `export_capture` de Python (`app/qa/generar_paquete.py`).
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
            match exportar(&[carpeta.clone()], &destino) {
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
