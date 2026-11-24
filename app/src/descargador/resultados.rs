//! E-S1 · Productos locales de un lote verificado, con la salida de `descargador/resultados.py`: libro consolidado y
//! «Para CSMP» (`export_book`), CSV por esquema (`export_csv`), calendario ICS (`export_ics`), ZIP de entrega
//! (`export_zip`) y comparación de lotes (`compare_lots`). Sin Chrome ni Office.

use super::excel::{self, Filtro, Hoja};
use super::paquete::{self, Fuente, Lote};
use super::xls::Celda;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const MAX_FILAS: usize = 200_000;

/// `ORIGIN`.
pub const ORIGEN: [&str; 7] =
    ["SITFA_LOTE", "SITFA_CONSULTA", "SITFA_ARCHIVO", "SITFA_PAGINA", "SITFA_FILA", "SITFA_SHA256", "SITFA_FECHA_DESCARGA"];

type Ahora = chrono::DateTime<chrono::FixedOffset>;

fn sha256(datos: &[u8]) -> String {
    format!("{:x}", Sha256::digest(datos))
}

fn t(s: impl Into<String>) -> Celda {
    Celda::Texto(s.into())
}

fn nombre_carpeta(lote: &Lote) -> String {
    lote.carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// `table_mode`: modo CSMP de la selección o vacío.
pub fn modo_tabla(seleccion: &Value) -> &'static str {
    match (seleccion["screen"].as_str().unwrap_or(""), seleccion["tab"].as_str().unwrap_or("")) {
        ("seguimiento", "Espera") => "ESPERA",
        ("seguimiento", "Cumplimiento") => "CUMPLIMIENTO",
        ("seguimiento", "Informes") => "INFORMES",
        ("calendario_informes", _) => "INFORMES",
        _ => "",
    }
}

fn requeridas(modo: &str) -> &'static [&'static str] {
    match modo {
        "ESPERA" => &["rit", "nombre", "tribunal", "programa", "espera"],
        "CUMPLIMIENTO" => &["rit", "nombre", "tribunal", "programa", "dias_cumpl", "dias_egresar"],
        "INFORMES" => &["rit", "nombre", "tribunal", "programa", "vencimiento"],
        _ => &[],
    }
}

/// Una fuente tabulada con sus filas tipadas (`tables(lot)`).
pub struct Tabla {
    pub consulta: Value,
    pub seleccion: Value,
    pub item: Value,
    pub cabecera: Vec<String>,
    pub filas: Vec<(usize, Vec<Celda>)>,
}

/// `tables(lot)` con los valores crudos de la grilla (número, fecha, vacío o texto).
pub fn tablas(lote: &Lote) -> Result<Vec<Tabla>, String> {
    let consultas: BTreeMap<String, Value> = lote
        .consultas
        .iter()
        .map(|(q, _)| (q["verificacion"].as_str().unwrap_or("").to_string(), q.clone()))
        .collect();
    let fuentes: Vec<Fuente> = paquete::tablas_lote(lote)?;
    Ok(fuentes
        .into_iter()
        .map(|f| Tabla {
            consulta: consultas.get(&f.verificacion).cloned().unwrap_or(json!({})),
            filas: f.filas.iter().map(|(n, v)| (*n, f.fila_tipada(*n, v))).collect(),
            seleccion: f.seleccion,
            item: f.item,
            cabecera: f.cabecera,
        })
        .collect())
}

/// `compatibility(headers, selection, rows)`.
pub fn compatibilidad(cabecera: &[String], seleccion: &Value, filas: &[(usize, Vec<Celda>)]) -> Result<Value, String> {
    let modo = modo_tabla(seleccion);
    let cols = paquete::mapeo(cabecera)?;
    if modo.is_empty() {
        return Ok(json!({"modo": "", "estado": "Solo análisis", "faltan": ["Pantalla sin modo CSMP"]}));
    }
    let mut faltan: Vec<String> = requeridas(modo).iter().filter(|k| !cols.contains_key(*k)).map(|k| k.to_string()).collect();
    if modo == "INFORMES" && seleccion["screen"] == "seguimiento" && seleccion["report"] == "3" {
        faltan.push("Contrato para Informes recibidos (no corresponde a pendientes)".into());
    }
    let vacio = |c: &Celda| matches!(c, Celda::Vacia) || matches!(c, Celda::Texto(s) if s.is_empty());
    let incompletas = filas
        .iter()
        .filter(|(_, f)| requeridas(modo).iter().filter_map(|k| cols.get(k)).any(|&i| f.get(i).is_none_or(vacio)))
        .count();
    let estado = if !faltan.is_empty() {
        "Sin datos suficientes"
    } else if incompletas > 0 {
        "Compatible con limitaciones"
    } else {
        "Compatible"
    };
    Ok(json!({"modo": modo, "estado": estado, "faltan": faltan, "filas_incompletas": incompletas}))
}

/// `resultados.parse_day` sobre un valor crudo.
pub fn dia(c: &Celda) -> Option<chrono::NaiveDate> {
    match c {
        Celda::Fecha(d) => Some(d.date()),
        otro => {
            let v = otro.texto_python();
            let v = v.trim();
            ["%d/%m/%Y", "%Y-%m-%d", "%d-%m-%Y"].iter().find_map(|f| chrono::NaiveDate::parse_from_str(v, f).ok())
        }
    }
}

/// `publish`: nombre `<etiqueta segura> AAAAMMDD_HHMMSS_ffffff<sufijo>` en la carpeta, creado sin sobrescribir.
pub fn publicar(carpeta: &Path, etiqueta: &str, sufijo: &str, ahora: &Ahora, bytes: &[u8]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(carpeta).map_err(|e| format!("No se pudo crear la carpeta del producto: {e}"))?;
    let nombre = format!("{} {}{sufijo}", super::lote::nombre_seguro(etiqueta), ahora.format("%Y%m%d_%H%M%S_%6f"));
    let destino = carpeta.join(nombre);
    super::lote::guardar_exclusivo(&destino, bytes)?;
    Ok(destino)
}

/// `json.dumps(..., ensure_ascii=False, indent=2)` (Python no ordena las claves).
fn json_sangrado(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_default()
}

/// `export_book(lots, destination, csmp)`: libro con Resumen, SITFA_ARCHIVOS, una hoja por familia y esquema,
/// Vencimientos y su metadato JSON al lado. Devuelve la ruta del XLSX.
pub fn exportar_libro(lotes: &[Lote], destino: Option<&Path>, csmp: bool, ahora: &Ahora) -> Result<PathBuf, String> {
    if lotes.is_empty() {
        return Err("Selecciona al menos un lote.".into());
    }
    let tablas_por_lote: Vec<Vec<Tabla>> = lotes.iter().map(tablas).collect::<Result<_, _>>()?;
    let consulta_de = |t: &Tabla| t.consulta["consulta"].as_str().unwrap_or("").to_string();
    if csmp {
        if lotes.iter().any(|l| l.parcial) {
            return Err("Para CSMP se requieren lotes completos.".into());
        }
        let mut esquemas: BTreeMap<&str, BTreeSet<Vec<String>>> = BTreeMap::new();
        let mut total = 0;
        for ts in &tablas_por_lote {
            for t in ts {
                let r = compatibilidad(&t.cabecera, &t.seleccion, &t.filas)?;
                let faltan: Vec<String> = r["faltan"].as_array().unwrap().iter().map(paquete::texto_de).collect();
                if r["modo"] == "" || !faltan.is_empty() {
                    return Err(format!("No apto para CSMP: {} · {}", consulta_de(t), faltan.join(", ")));
                }
                esquemas.entry(modo_tabla(&t.seleccion)).or_default().insert(t.cabecera.clone());
                total += t.filas.len();
            }
        }
        if esquemas.values().any(|v| v.len() > 1) {
            return Err("CSMP requiere una estructura compatible por modo. Separa los esquemas diferentes.".into());
        }
        if total == 0 {
            return Err("El lote no contiene registros para CSMP; consulta sus PDF y resumen.".into());
        }
        if lotes.len() > 1 {
            if lotes.iter().any(|l| l.resumen["otros_filtros"] == "formulario") {
                return Err("No se cruzan filtros privados no comparables.".into());
            }
            let fechas: BTreeSet<String> = lotes.iter().map(|l| paquete::json_python(&l.resumen["filtro_fecha"], true)).collect();
            let tribunales: Vec<BTreeSet<String>> = lotes
                .iter()
                .map(|l| l.resumen["plan"].as_array().into_iter().flatten().map(|s| paquete::texto_de(&s["tribunal"])).collect())
                .collect();
            if fechas.len() > 1 || tribunales[0].is_empty() || tribunales.iter().any(|c| *c != tribunales[0]) {
                return Err("Los lotes de cruce no comparten tribunales y filtros de fecha.".into());
            }
        }
    }
    let encabezado = |xs: &[&str]| xs.iter().map(|x| t(*x)).collect::<Vec<_>>();
    let cab_resumen = ["Resultado", "Lote", "Consulta", "Hoja", "Filas", "Compatibilidad CSMP", "Columnas faltantes"];
    let mut resumen = vec![encabezado(&cab_resumen)];
    let cab_origen = ["Carpeta origen", "Archivo origen", "SHA256"];
    let mut origen = vec![encabezado(&cab_origen)];
    // grupo (familia, cabecera) → (índice de hoja, filas)
    let mut grupos: Vec<((String, Vec<String>), Hoja)> = Vec::new();
    let mut nombres: Vec<(String, usize)> = Vec::new();
    let mut usados: BTreeSet<String> = ["Resumen", "SITFA_ARCHIVOS"].iter().map(|s| s.to_string()).collect();
    let (mut reportes, mut fuentes, mut filas_fuente) = (Vec::new(), Vec::new(), 0usize);
    for (lote, ts) in lotes.iter().zip(&tablas_por_lote) {
        let carpeta = nombre_carpeta(lote);
        for (q, m) in &lote.consultas {
            if m["estado"] == "SIN_RESULTADOS" {
                let pdf = m.get("evidencia").is_some_and(|e| !e.is_null() && *e != json!({}) && *e != json!(""));
                resumen.push(vec![t("SIN_RESULTADOS"), t(&carpeta), t(paquete::texto_de(&q["consulta"])),
                    t(if pdf { "PDF" } else { "Sin PDF" }), Celda::Entero(0), t("Sin registros"), t("")]);
            } else if m["estado"] == "INCOMPLETA" && m["archivos"].as_array().is_none_or(Vec::is_empty) {
                resumen.push(vec![t("INCOMPLETA"), t(&carpeta), t(paquete::texto_de(&q["consulta"])), t(""), Celda::Entero(0),
                    t("Consulta pendiente"), t("")]);
            }
        }
        for tabla in ts {
            let r = compatibilidad(&tabla.cabecera, &tabla.seleccion, &tabla.filas)?;
            let faltan: Vec<String> = r["faltan"].as_array().unwrap().iter().map(paquete::texto_de).collect();
            if csmp && (r["modo"] == "" || !faltan.is_empty()) {
                return Err(format!("No apto para CSMP: {} · {}", consulta_de(tabla), faltan.join(", ")));
            }
            let familia = match r["modo"].as_str().unwrap_or("") {
                "" => tabla.seleccion["screen"].as_str().filter(|s| !s.is_empty()).unwrap_or("Datos").to_string(),
                m => m.to_string(),
            };
            reportes.push(r.clone());
            let clave = (familia.clone(), tabla.cabecera.clone());
            let indice = match grupos.iter().position(|(k, _)| *k == clave) {
                Some(i) => i,
                None => {
                    let n = match nombres.iter_mut().find(|(f, _)| *f == familia) {
                        Some((_, n)) => {
                            *n += 1;
                            *n
                        }
                        None => {
                            nombres.push((familia.clone(), 1));
                            1
                        }
                    };
                    let base = if n == 1 { familia.clone() } else { format!("{familia}_{n}") };
                    let mut nombre: String = base.chars().take(31).collect();
                    if matches!(nombre.as_str(), "Resumen" | "SITFA_ARCHIVOS" | "Vencimientos") {
                        nombre = format!("Datos_{nombre}");
                    }
                    while usados.contains(&nombre) {
                        nombre = format!("{}_{}", nombre.chars().take(27).collect::<String>(), usados.len()).chars().take(31).collect();
                    }
                    usados.insert(nombre.clone());
                    let cabeceras: Vec<String> = tabla.cabecera.iter().cloned().chain(ORIGEN.iter().map(|s| s.to_string())).collect();
                    grupos.push((clave, Hoja {
                        nombre,
                        filas: vec![cabeceras.iter().map(t).collect()],
                        presentacion: Some(cabeceras.clone()),
                        congelar: true,
                        filtro: Filtro::Ninguno,
                        solo_formato: false,
                    }));
                    grupos.len() - 1
                }
            };
            let fecha = tabla.seleccion.get("fecha_descarga").cloned().unwrap_or_else(|| lote.resumen.get("fecha_hora").cloned().unwrap_or(json!("")));
            let hoja = &mut grupos[indice].1;
            for (n, fila) in &tabla.filas {
                let mut valores = fila.clone();
                valores.extend([t(&carpeta), t(consulta_de(tabla)), excel::celda_json(&tabla.item["archivo"]),
                    excel::celda_json(&tabla.item["pagina"]), Celda::Entero(*n as i64), excel::celda_json(&tabla.item["sha256"]),
                    excel::celda_json(&fecha)]);
                hoja.filas.push(valores);
            }
            filas_fuente += tabla.filas.len();
            if filas_fuente > MAX_FILAS {
                return Err("La exportación supera 200.000 filas.".into());
            }
            resumen.push(vec![t(if lote.parcial { "INCOMPLETO" } else { "VERIFICADO" }), t(&carpeta), t(consulta_de(tabla)),
                t(&hoja.nombre), Celda::Entero(tabla.filas.len() as i64), excel::celda_json(&r["estado"]), t(faltan.join(", "))]);
            origen.push(vec![t(lote.carpeta.display().to_string()), excel::celda_json(&tabla.item["archivo"]),
                excel::celda_json(&tabla.item["sha256"])]);
            fuentes.push(json!({"lote": lote.carpeta.display().to_string(), "archivo": tabla.item["archivo"],
                "sha256": tabla.item["sha256"], "filas": tabla.filas.len()}));
        }
    }
    if csmp {
        if let Some((f, _)) = nombres.iter().find(|(_, n)| *n > 1) {
            return Err(format!("Hay varios esquemas para {f}. Exporta cada esquema por separado antes de CSMP."));
        }
    }
    let cab_venc = ["Vencimiento", "Tribunal", "Lote", "Archivo", "Fila", "Estado de fecha"];
    let mut vencimientos = vec![encabezado(&cab_venc)];
    let mut filas_venc: Vec<(chrono::NaiveDate, String, String, String, String, usize)> = Vec::new();
    for (lote, ts) in lotes.iter().zip(&tablas_por_lote) {
        for tabla in ts {
            let cols = paquete::mapeo(&tabla.cabecera)?;
            let Some(&c) = cols.get("vencimiento").or_else(|| cols.get("egreso_proy")) else { continue };
            for (n, fila) in &tabla.filas {
                let crudo = fila.get(c).cloned().unwrap_or(Celda::Vacia);
                filas_venc.push((dia(&crudo).unwrap_or(chrono::NaiveDate::MAX), crudo.texto_python(),
                    paquete::texto_de(&tabla.seleccion["tribunal_nombre"]), nombre_carpeta(lote), paquete::texto_de(&tabla.item["archivo"]), *n));
            }
        }
    }
    filas_venc.sort();
    for (d, crudo, tribunal, lote, archivo, n) in filas_venc {
        let valida = d != chrono::NaiveDate::MAX;
        vencimientos.push(vec![t(if valida { d.format("%Y-%m-%d").to_string() } else { crudo }), t(tribunal), t(lote), t(archivo),
            Celda::Entero(n as i64), t(if valida { "Fecha válida" } else { "No interpretable" })]);
    }
    let presentada = |nombre: &str, cab: &[&str], filas: Vec<Vec<Celda>>| Hoja {
        nombre: nombre.into(),
        filas,
        presentacion: Some(cab.iter().map(|s| s.to_string()).collect()),
        congelar: true,
        filtro: Filtro::Ninguno,
        solo_formato: false,
    };
    let mut hojas = vec![presentada("Resumen", &cab_resumen, resumen), presentada("SITFA_ARCHIVOS", &cab_origen, origen)];
    for ((_, cabecera), mut hoja) in grupos {
        hoja.filtro = Filtro::Rango(hoja.filas.len() as u32, (cabecera.len() + ORIGEN.len()) as u16);
        hojas.push(hoja);
    }
    hojas.push(presentada("Vencimientos", &cab_venc, vencimientos));
    let bytes = excel::libro(&hojas)?;
    let parcial = lotes.iter().any(|l| l.parcial);
    let etiqueta = format!("{}{}", if csmp { "Para CSMP" } else { "Consolidado" }, if parcial { " INCOMPLETO" } else { "" });
    let carpeta = destino.map(Path::to_path_buf).unwrap_or_else(|| lotes[0].carpeta.clone());
    let ruta = publicar(&carpeta, &etiqueta, ".xlsx", ahora, &bytes)?;
    let metadato = json!({
        "version_contrato": 1,
        "archivo": ruta.file_name().unwrap().to_string_lossy(),
        "sha256": sha256(&bytes),
        "filas": filas_fuente,
        "estado": if parcial { "INCOMPLETO" } else { "VALIDADA" },
        "fuentes": fuentes,
        "compatibilidad": reportes,
        "hojas": nombres.iter().map(|(f, _)| f.clone()).collect::<Vec<_>>(),
        "fecha_exportacion": ahora.to_rfc3339_opts(chrono::SecondsFormat::Micros, false),
    });
    let json_ruta = ruta.with_extension("json");
    std::fs::write(&json_ruta, json_sangrado(&metadato)).map_err(|e| format!("No se pudo guardar el metadato del libro: {e}"))?;
    Ok(ruta)
}

/// `consolidated_workbook`: el libro «Para CSMP» de los lotes como bytes, en una carpeta temporal propia.
pub fn libro_consolidado(lotes: &[Lote], ahora: &Ahora) -> Result<Vec<u8>, String> {
    let carpeta = std::env::temp_dir().join(format!("consolidado-{}-{}", std::process::id(), ahora.timestamp_micros()));
    let resultado = exportar_libro(lotes, Some(&carpeta), true, ahora).and_then(|r| std::fs::read(r).map_err(|e| e.to_string()));
    let _ = std::fs::remove_dir_all(&carpeta);
    resultado
}

// ---------------------------------------------------------------- `export_csv`, `export_ics`, `export_zip`, `compare_lots`

/// `csv.writer(delimiter=';')` con `QUOTE_MINIMAL` y fin de línea `\r\n`.
fn campo_csv(s: &str) -> String {
    if s.contains([';', '"', '\r', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

/// Valor de la fila para el CSV: el texto que puede ejecutar una fórmula en Excel se neutraliza con «'».
fn valor_csv(c: &Celda) -> String {
    match c {
        Celda::Texto(s) if s.trim_start().starts_with(['=', '+', '-', '@']) => format!("'{s}"),
        otro => otro.texto_python(),
    }
}

fn familia(t: &Tabla) -> String {
    match modo_tabla(&t.seleccion) {
        "" => t.seleccion["screen"].as_str().filter(|s| !s.is_empty()).unwrap_or("Datos").to_string(),
        m => m.to_string(),
    }
}

/// `export_csv(lot)`: un CSV (UTF-8 con BOM, `;`) por familia y esquema de columnas, con la procedencia de cada fila.
pub fn exportar_csv(lote: &Lote, ahora: &Ahora) -> Result<Vec<PathBuf>, String> {
    let datos = tablas(lote)?;
    let mut esquemas: Vec<(String, Vec<String>)> = Vec::new();
    for t in &datos {
        let clave = (familia(t), t.cabecera.clone());
        if !esquemas.contains(&clave) {
            esquemas.push(clave);
        }
    }
    let carpeta = nombre_carpeta(lote);
    let mut salida = Vec::new();
    for (numero, (fam, cabecera)) in esquemas.iter().enumerate() {
        let mut texto = String::from("\u{feff}");
        let linea = |valores: Vec<String>| valores.iter().map(|v| campo_csv(v)).collect::<Vec<_>>().join(";") + "\r\n";
        texto += &linea(cabecera.iter().cloned().chain(ORIGEN.iter().map(|s| s.to_string())).collect());
        for t in datos.iter().filter(|t| familia(t) == *fam && t.cabecera == *cabecera) {
            for (n, fila) in &t.filas {
                let mut valores: Vec<Celda> = fila.clone();
                valores.extend([
                    Celda::Texto(carpeta.clone()),
                    excel::celda_json(&t.consulta["consulta"]),
                    excel::celda_json(&t.item["archivo"]),
                    excel::celda_json(&t.item["pagina"]),
                    Celda::Entero(*n as i64),
                    excel::celda_json(&t.item["sha256"]),
                    excel::celda_json(&t.seleccion.get("fecha_descarga").cloned().unwrap_or(json!(""))),
                ]);
                texto += &linea(valores.iter().map(valor_csv).collect());
            }
        }
        salida.push(publicar(&lote.carpeta, &format!("Tabla {fam} {}", numero + 1), ".csv", ahora, texto.as_bytes())?);
    }
    Ok(salida)
}

/// `escape` del ICS (RFC 5545).
fn escapar_ics(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace(';', "\\;").replace(',', "\\,").replace('\r', "")
}

/// `str(v)` de Python para la clave de respaldo del ICS (`None` → «None»).
fn repr_ics(c: &Celda) -> String {
    match c {
        Celda::Vacia => "None".into(),
        otro => otro.texto_python(),
    }
}

/// `export_ics(lot)`: un evento por vencimiento o egreso proyectado legible, con UID estable por identidad.
pub fn exportar_ics(lote: &Lote, ahora: &Ahora) -> Result<PathBuf, String> {
    let datos = tablas(lote)?;
    let identidad = |t: &Tabla, fila: &[Celda], campo: &str| -> Result<Option<String>, String> {
        let cols = paquete::mapeo(&t.cabecera)?;
        let mut partes = Vec::new();
        for k in ["rit", "rut", "programa"] {
            let Some(&i) = cols.get(k) else { return Ok(None) };
            let v = fila.get(i).map(Celda::texto_python).unwrap_or_default();
            if v.trim().is_empty() {
                return Ok(None);
            }
            partes.push(json!(v.trim()));
        }
        let mut lista = vec![t.seleccion.get("tribunal").cloned().unwrap_or(Value::Null),
            t.seleccion.get("screen").cloned().unwrap_or(Value::Null), json!(campo)];
        lista.extend(partes);
        Ok(Some(paquete::json_python(&Value::Array(lista), false)))
    };
    let mut identidades: BTreeMap<String, usize> = BTreeMap::new();
    for t in &datos {
        let cols = paquete::mapeo(&t.cabecera)?;
        for campo in ["vencimiento", "egreso_proy"] {
            let Some(&c) = cols.get(campo) else { continue };
            for (_, fila) in &t.filas {
                if let Some(k) = identidad(t, fila, campo)? {
                    if fila.get(c).and_then(dia).is_some() {
                        *identidades.entry(k).or_default() += 1;
                    }
                }
            }
        }
    }
    let sello = ahora.with_timezone(&chrono::Utc).format("%Y%m%dT%H%M%SZ").to_string();
    let (mut eventos, mut ocurrencias): (Vec<String>, BTreeMap<String, usize>) = (Vec::new(), BTreeMap::new());
    for t in &datos {
        let cols = paquete::mapeo(&t.cabecera)?;
        for campo in ["vencimiento", "egreso_proy"] {
            let Some(&c) = cols.get(campo) else { continue };
            for (n, fila) in &t.filas {
                let Some(d) = fila.get(c).and_then(dia) else { continue };
                let estable = identidad(t, fila, campo)?;
                let clave = match estable {
                    Some(k) if identidades.get(&k) == Some(&1) => k,
                    _ => paquete::json_python(&json!([t.seleccion.get("tribunal").cloned().unwrap_or(Value::Null), campo,
                        fila.iter().map(repr_ics).collect::<Vec<_>>()]), false),
                };
                let cuenta = ocurrencias.entry(clave.clone()).or_default();
                *cuenta += 1;
                let uid = format!("{}@sitfa-local", sha256(format!("{clave}:{cuenta}").as_bytes()));
                let rit = cols.get("rit").and_then(|&i| fila.get(i)).map(|v| repr_ics(v).trim().to_string()).unwrap_or_default();
                let (mut etiqueta, mut sentido) = if campo == "vencimiento" {
                    ("Vencimiento de informe", "Verificar vencimiento en el original; no acredita entrega del informe.")
                } else {
                    ("Egreso proyectado", "Fecha proyectada de la fuente; no acredita un egreso efectivo.")
                };
                if campo == "vencimiento" && t.seleccion["screen"] == "calendario_medidas" {
                    etiqueta = "Vencimiento de medida";
                    sentido = "Fecha de vencimiento de la medida en la fuente; no acredita un egreso efectivo.";
                }
                let tribunal = paquete::texto_de(&t.seleccion.get("tribunal_nombre").cloned().unwrap_or(json!("")));
                eventos.extend([
                    "BEGIN:VEVENT".to_string(),
                    format!("UID:{uid}"),
                    format!("DTSTAMP:{sello}"),
                    format!("DTSTART;VALUE=DATE:{}", d.format("%Y%m%d")),
                    format!("SUMMARY:{}", escapar_ics(&format!("{etiqueta} SITFA · {rit} · {tribunal}"))),
                    format!("DESCRIPTION:{}", escapar_ics(&format!(
                        "Fuente: {}, fila {n}. Columna: {}. {sentido} Cobertura del lote: {}.",
                        paquete::texto_de(&t.item["archivo"]), t.cabecera[c], if lote.parcial { "INCOMPLETO" } else { "VALIDADO" }))),
                    "END:VEVENT".to_string(),
                ]);
            }
        }
    }
    if eventos.is_empty() {
        return Err("No hay fechas verificables para exportar al calendario.".into());
    }
    let mut lineas = vec!["BEGIN:VCALENDAR".to_string(), "VERSION:2.0".into(), "PRODID:-//SITFA local//Vencimientos//ES".into(),
        "CALSCALE:GREGORIAN".into()];
    lineas.extend(eventos);
    lineas.push("END:VCALENDAR".into());
    // Plegado a ≤ 75 octetos sin cortar secuencias UTF-8.
    let mut plegadas = Vec::new();
    for linea in &lineas {
        let mut actual = String::new();
        for c in linea.chars() {
            if actual.len() + c.len_utf8() > 73 {
                plegadas.push(std::mem::replace(&mut actual, format!(" {c}")));
            } else {
                actual.push(c);
            }
        }
        plegadas.push(actual);
    }
    let contenido = plegadas.join("\r\n") + "\r\n";
    let vevent = contenido.matches("\r\nBEGIN:VEVENT\r\n").count();
    if vevent != lineas.iter().filter(|l| *l == "BEGIN:VEVENT").count() {
        return Err("El calendario no superó la comprobación de eventos.".into());
    }
    publicar(&lote.carpeta, "Vencimientos", ".ics", ahora, contenido.as_bytes())
}

/// `export_zip(lot)`: entrega con `INDICE.txt`, `MANIFIESTO.json` y cada archivo del lote, comprobados dentro del ZIP.
pub fn exportar_zip(lote: &Lote, ahora: &Ahora) -> Result<PathBuf, String> {
    use std::io::{Read, Write};
    let mut rutas: BTreeSet<PathBuf> = BTreeSet::from([lote.carpeta.join("resumen.json")]);
    for (q, m) in &lote.consultas {
        if let Some(v) = q["verificacion"].as_str().filter(|v| !v.is_empty()) {
            rutas.insert(paquete::dentro(&lote.carpeta, v)?);
        }
        for item in m["archivos"].as_array().into_iter().flatten().filter(|i| i["estado"] == "OK") {
            paquete::comprobar(&lote.carpeta, item)?;
            rutas.insert(paquete::dentro(&lote.carpeta, item["archivo"].as_str().unwrap_or(""))?);
        }
        if m.get("evidencia").is_some_and(|e| e.is_object()) {
            paquete::comprobar(&lote.carpeta, &m["evidencia"])?;
            rutas.insert(paquete::dentro(&lote.carpeta, m["evidencia"]["archivo"].as_str().unwrap_or(""))?);
        }
    }
    let nombre = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut archivos = Vec::new();
    let mut entradas = Vec::new();
    for r in &rutas {
        let b = std::fs::read(r).map_err(|e| format!("No se pudo leer un archivo del lote: {e}"))?;
        entradas.push(json!({"archivo": nombre(r), "bytes": b.len(), "sha256": sha256(&b)}));
        archivos.push((nombre(r), b));
    }
    let estado = if lote.parcial { "INCOMPLETO" } else { "VERIFICADO" };
    let indice = format!("{estado}\nArchivos del lote actual. Consultar MANIFIESTO.json para comprobar integridad.\n{}",
        rutas.iter().map(|r| nombre(r)).collect::<Vec<_>>().join("\n"));
    let manifiesto = json_sangrado(&json!({"version": 1, "estado": estado, "archivos": entradas}));
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opciones = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let e = |x: zip::result::ZipError| x.to_string();
    for (n, b) in [("INDICE.txt".to_string(), indice.into_bytes()), ("MANIFIESTO.json".to_string(), manifiesto.into_bytes())]
        .into_iter()
        .chain(archivos.iter().cloned())
    {
        zip.start_file(n, opciones).map_err(e)?;
        zip.write_all(&b).map_err(|x| x.to_string())?;
    }
    let bytes = zip.finish().map_err(e)?.into_inner();
    let mut lectura = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone())).map_err(e)?;
    for (n, b) in &archivos {
        let mut copia = Vec::new();
        lectura.by_name(n).map_err(e)?.read_to_end(&mut copia).map_err(|x| x.to_string())?;
        if sha256(&copia) != sha256(b) {
            return Err("Un archivo cambió durante la entrega.".into());
        }
    }
    publicar(&lote.carpeta, &format!("Entrega{}", if lote.parcial { " INCOMPLETO" } else { "" }), ".zip", ahora, &bytes)
}

/// `comparable_scope`.
fn alcance_comparable(lote: &Lote) -> Result<String, String> {
    if lote.parcial || lote.resumen["otros_filtros"] == "formulario" {
        return Err("No se comparan lotes parciales ni filtros privados no verificables.".into());
    }
    let plan = lote.resumen["plan"].as_array().filter(|p| !p.is_empty())
        .ok_or("Este lote antiguo no tiene alcance estructurado para comparar.")?;
    let mut partes: Vec<String> = plan.iter().map(|s| paquete::json_python(&limpio(s), true)).collect();
    partes.sort();
    Ok(paquete::json_python(&json!([partes, lote.resumen.get("filtro_fecha").cloned().unwrap_or(Value::Null)]), true))
}

fn limpio(s: &Value) -> Value {
    let mut s = s.clone();
    if let Some(o) = s.as_object_mut() {
        for k in ["fecha_descarga", "tribunal_nombre", "modalidad_nombre"] {
            o.shift_remove(k);
        }
    }
    s
}

type Inventario = Vec<((String, Vec<String>, Vec<String>), usize)>;

fn inventario(lote: &Lote) -> Result<Inventario, String> {
    let mut salida: Inventario = Vec::new();
    for t in tablas(lote)? {
        let alcance = paquete::json_python(&limpio(&t.seleccion), true);
        for (_, fila) in &t.filas {
            let clave = (alcance.clone(), t.cabecera.clone(), fila.iter().map(Celda::texto_python).collect::<Vec<_>>());
            match salida.iter_mut().find(|(k, _)| *k == clave) {
                Some((_, n)) => *n += 1,
                None => salida.push((clave, 1)),
            }
        }
    }
    Ok(salida)
}

/// `Counter.__sub__`: claves de `a` (en su orden) con cuenta positiva tras restar `b`.
fn restar(a: &Inventario, b: &Inventario) -> Inventario {
    a.iter()
        .filter_map(|(k, n)| {
            let m = b.iter().find(|(x, _)| x == k).map_or(0, |(_, m)| *m);
            (*n > m).then(|| (k.clone(), n - m))
        })
        .collect()
}

/// `compare_lots(old, new)`: filas nuevas y ausentes (fila completa y multiplicidad) entre dos lotes del mismo alcance.
pub fn comparar_lotes(anterior: &Lote, nuevo: &Lote, ahora: &Ahora) -> Result<PathBuf, String> {
    if alcance_comparable(anterior)? != alcance_comparable(nuevo)? {
        return Err("Los lotes tienen alcances diferentes.".into());
    }
    let (a, b) = (inventario(anterior)?, inventario(nuevo)?);
    let esquemas = |inv: &Inventario| inv.iter().map(|((s, h, _), _)| (s.clone(), h.clone())).collect::<BTreeSet<_>>();
    if !a.is_empty() && !b.is_empty() && esquemas(&a) != esquemas(&b) {
        return Err("Las estructuras de columnas no coinciden.".into());
    }
    let (nuevas, ausentes) = (restar(&b, &a), restar(&a, &b));
    let hoja = |nombre: &str, delta: &Inventario| {
        let mut filas = vec![["Consulta", "Columnas", "Valores de fila", "Cantidad"].map(t).to_vec()];
        for ((alcance, cabecera, fila), n) in delta {
            filas.push(vec![t(alcance), t(paquete::json_python(&json!(cabecera), false)), t(paquete::json_python(&json!(fila), false)),
                Celda::Entero(*n as i64)]);
        }
        Hoja { nombre: nombre.into(), filas, presentacion: None, congelar: false, filtro: Filtro::Ninguno, solo_formato: true }
    };
    let suma = |d: &Inventario| d.iter().map(|(_, n)| *n as i64).sum::<i64>();
    let resumen = Hoja {
        nombre: "Resumen".into(),
        filas: vec![
            vec![t("Lote anterior"), t(anterior.carpeta.display().to_string())],
            vec![t("Lote nuevo"), t(nuevo.carpeta.display().to_string())],
            vec![t("Nuevas"), Celda::Entero(suma(&nuevas))],
            vec![t("Ausentes"), Celda::Entero(suma(&ausentes))],
            vec![t("Interpretación"), t("Ausente no implica egreso ni eliminación; se comparan filas completas y multiplicidad.")],
        ],
        presentacion: None,
        congelar: false,
        filtro: Filtro::Ninguno,
        solo_formato: true,
    };
    let bytes = excel::libro(&[hoja("Nuevas", &nuevas), hoja("Ausentes", &ausentes), resumen])?;
    publicar(&nuevo.carpeta, "Comparacion", ".xlsx", ahora, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descargador::excel::volcado;
    use base64::Engine;

    fn reconstruir(destino: &Path, archivos: &Value) {
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    fn ahora() -> Ahora {
        chrono::DateTime::parse_from_rfc3339("2026-10-08T13:00:00.123456-03:00").unwrap()
    }

    /// E-S1: `export_csv`, `export_ics`, `export_zip` y `compare_lots` de Rust contra Flow Python
    /// (`app/qa/generar_resultados.py`): nombre y bytes del CSV e ICS, contenido del ZIP y hojas de la comparación.
    #[test]
    fn productos_iguales_a_python() {
        let paquete: Value = serde_json::from_str(include_str!("../../tests/datos/paquete.json")).unwrap();
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/resultados.json")).unwrap();
        let escenarios: BTreeMap<String, Value> = paquete["escenarios"].as_array().unwrap().iter()
            .map(|e| (e["escenario"].as_str().unwrap().to_string(), e.clone())).collect();
        let raiz = std::env::temp_dir().join(format!("flow-es1-productos-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        std::fs::create_dir_all(&raiz).unwrap();
        let raiz = raiz.canonicalize().unwrap();
        let lote = |escenario: &str, i: usize| -> PathBuf {
            let e = &escenarios[escenario];
            let l: Vec<&Value> = std::iter::once(&e["lote"]).chain(e["otros_lotes"].as_array().into_iter().flatten()).collect();
            let c = raiz.join(escenario).join("lotes").join(l[i]["carpeta"].as_str().unwrap());
            if !c.exists() {
                reconstruir(&c, &l[i]["archivos"]);
            }
            c
        };
        let modificado = raiz.join("modificado").join("lotes").join(datos["modificado"]["carpeta"].as_str().unwrap());
        reconstruir(&modificado, &datos["modificado"]["archivos"]);
        let base_txt = raiz.display().to_string();
        let (mut iguales, mut fallos, mut total) = (0, Vec::new(), 0);
        let mut anotar = |nombre: String, d: Vec<String>| {
            total += 1;
            if d.is_empty() { iguales += 1 } else { fallos.push(format!("{nombre}:\n  {}", d.join("\n  "))) }
        };
        // CSV: agrupados por lote, en el orden de Python.
        let mut por_lote: Vec<((String, usize), Vec<&Value>)> = Vec::new();
        for c in datos["csv"].as_array().unwrap() {
            let k = (c["escenario"].as_str().unwrap().to_string(), c["lote"].as_u64().unwrap() as usize);
            match por_lote.iter_mut().find(|(x, _)| *x == k) {
                Some((_, v)) => v.push(c),
                None => por_lote.push((k, vec![c])),
            }
        }
        for ((escenario, i), esperados) in por_lote {
            let carpeta = if escenario == "modificado" { modificado.clone() } else { lote(&escenario, i) };
            let mut d = Vec::new();
            match exportar_csv(&paquete::cargar_lote(&carpeta).unwrap(), &ahora()) {
                Err(e) => d.push(format!("exportar: {e}")),
                Ok(rutas) => {
                    if rutas.len() != esperados.len() {
                        d.push(format!("{} CSV en Rust, {} en Python", rutas.len(), esperados.len()));
                    }
                    for (r, e) in rutas.iter().zip(&esperados) {
                        let bytes = std::fs::read(r).unwrap();
                        let python = base64::engine::general_purpose::STANDARD.decode(e["contenido"].as_str().unwrap()).unwrap();
                        if r.file_name().unwrap().to_string_lossy() != e["archivo"].as_str().unwrap() || bytes != python {
                            d.push(format!("{}:\n    Rust   {:?}\n    Python {:?}", e["archivo"],
                                String::from_utf8_lossy(&bytes).chars().take(600).collect::<String>(),
                                String::from_utf8_lossy(&python).chars().take(600).collect::<String>()));
                        }
                        if escenario == "modificado" {
                            std::fs::remove_file(r).unwrap();
                        }
                    }
                }
            }
            anotar(format!("csv {escenario} {i}"), d);
        }
        for e in datos["ics"].as_array().unwrap() {
            let (escenario, i) = (e["escenario"].as_str().unwrap(), e["lote"].as_u64().unwrap() as usize);
            let r = exportar_ics(&paquete::cargar_lote(&lote(escenario, i)).unwrap(), &ahora());
            let d = match (r, e.get("error")) {
                (Err(x), Some(p)) if x == p.as_str().unwrap() => vec![],
                (Ok(r), None) => {
                    let texto = std::fs::read_to_string(&r).unwrap();
                    if r.file_name().unwrap().to_string_lossy() == e["archivo"].as_str().unwrap() && texto == e["contenido"].as_str().unwrap() {
                        vec![]
                    } else {
                        vec![format!("Rust:\n{texto}\nPython:\n{}", e["contenido"].as_str().unwrap())]
                    }
                }
                (r, p) => vec![format!("Rust {:?} · Python {:?}", r.map(|p| p.display().to_string()), p)],
            };
            anotar(format!("ics {escenario} {i}"), d);
        }
        for e in datos["zip"].as_array().unwrap() {
            let (escenario, i) = (e["escenario"].as_str().unwrap(), e["lote"].as_u64().unwrap() as usize);
            let r = exportar_zip(&paquete::cargar_lote(&lote(escenario, i)).unwrap(), &ahora()).unwrap();
            let mut z = zip::ZipArchive::new(std::fs::File::open(&r).unwrap()).unwrap();
            let mut orden = Vec::new();
            let mut entradas = serde_json::Map::new();
            for k in 0..z.len() {
                let mut f = z.by_index(k).unwrap();
                let mut b = Vec::new();
                std::io::Read::read_to_end(&mut f, &mut b).unwrap();
                let n = f.name().to_string();
                let v = if n == "INDICE.txt" || n == "MANIFIESTO.json" { json!(String::from_utf8(b).unwrap()) } else { json!(sha256(&b)) };
                orden.push(json!(n.clone()));
                entradas.insert(n, v);
            }
            let mut d = Vec::new();
            if r.file_name().unwrap().to_string_lossy() != e["archivo"].as_str().unwrap() {
                d.push(format!("archivo {}", r.display()));
            }
            if json!(orden) != e["orden"] {
                d.push(format!("orden Rust {orden:?} · Python {}", e["orden"]));
            }
            if Value::Object(entradas.clone()) != e["entradas"] {
                d.push(format!("entradas:\n    Rust   {}\n    Python {}", Value::Object(entradas), e["entradas"]));
            }
            anotar(format!("zip {escenario} {i}"), d);
        }
        let anterior = paquete::cargar_lote(&lote("listados", 0)).unwrap();
        for e in datos["comparacion"].as_array().unwrap() {
            let nombre = e["escenario"].as_str().unwrap();
            let nuevo = match nombre {
                "modificado" => modificado.clone(),
                "mismo" => lote("listados", 0),
                _ => lote("carga", 0),
            };
            let r = comparar_lotes(&anterior, &paquete::cargar_lote(&nuevo).unwrap(), &ahora());
            let d = match (r, e.get("error")) {
                (Err(x), Some(p)) if x == p.as_str().unwrap() => vec![],
                (Ok(r), None) => {
                    let mut d = volcado::diferencias(&volcado::volcar(&std::fs::read(&r).unwrap(), &base_txt), &e["libro"]);
                    if r.file_name().unwrap().to_string_lossy() != e["archivo"].as_str().unwrap() {
                        d.push(format!("archivo {}", r.display()));
                    }
                    std::fs::remove_file(r).unwrap();
                    d
                }
                (r, p) => vec![format!("Rust {:?} · Python {:?}", r.map(|p| p.display().to_string()), p)],
            };
            anotar(format!("comparacion {nombre}"), d);
        }
        let _ = std::fs::remove_dir_all(&raiz);
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("PRODUCTOS: {iguales}/{total}, fallos {}", fallos.len());
        assert_eq!(total, 13, "el oráculo debe traer 13 productos");
        assert!(fallos.is_empty());
    }

    /// Prueba permanente del oráculo de productos: un CSV esperado con un byte cambiado ya no coincide con el de Rust.
    #[test]
    fn oraculo_de_productos_detecta_alteraciones() {
        let paquete: Value = serde_json::from_str(include_str!("../../tests/datos/paquete.json")).unwrap();
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/resultados.json")).unwrap();
        let e = paquete["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "carga").unwrap();
        let caso = datos["csv"].as_array().unwrap().iter().find(|c| c["escenario"] == "carga").unwrap();
        let raiz = std::env::temp_dir().join(format!("flow-es1-mutacion-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let carpeta = raiz.join(e["otros_lotes"][0]["carpeta"].as_str().unwrap());
        reconstruir(&carpeta, &e["otros_lotes"][0]["archivos"]);
        let rutas = exportar_csv(&paquete::cargar_lote(&carpeta).unwrap(), &ahora()).unwrap();
        let rust = std::fs::read(&rutas[0]).unwrap();
        let mut python = base64::engine::general_purpose::STANDARD.decode(caso["contenido"].as_str().unwrap()).unwrap();
        assert_eq!(rust, python);
        let ultimo = python.len() - 3;
        python[ultimo] ^= 1;
        assert_ne!(rust, python, "un byte alterado del CSV esperado no se detecta");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    /// E-S1: `export_book` de Rust (consolidado y Para CSMP) contra Flow Python (`app/qa/generar_excel.py`): nombre,
    /// hojas, celdas con su tipo, panel congelado, filtro, anchos, metadato y los rechazos de CSMP.
    #[test]
    fn libro_igual_a_python() {
        let paquete: Value = serde_json::from_str(include_str!("../../tests/datos/paquete.json")).unwrap();
        let excel: Value = serde_json::from_str(include_str!("../../tests/datos/excel.json")).unwrap();
        let escenarios: BTreeMap<String, Value> = paquete["escenarios"].as_array().unwrap().iter()
            .map(|e| (e["escenario"].as_str().unwrap().to_string(), e.clone())).collect();
        let raiz = std::env::temp_dir().join(format!("flow-es1-libro-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let (mut iguales, mut fallos) = (0, Vec::new());
        let casos = excel["libros"].as_array().unwrap();
        for caso in casos {
            let nombre = caso["escenario"].as_str().unwrap();
            let csmp = caso["csmp"].as_bool().unwrap();
            let base = raiz.join(format!("{nombre}-{csmp}"));
            let fuentes: Vec<Value> = match caso.get("lotes") {
                Some(propios) => propios.as_array().unwrap().clone(),
                None => {
                    let e = &escenarios[nombre];
                    std::iter::once(&e["lote"]).chain(e["otros_lotes"].as_array().into_iter().flatten()).cloned().collect()
                }
            };
            let parcial = caso["parcial"].as_bool().unwrap_or(false);
            let mut lotes = Vec::new();
            for l in &fuentes {
                let c = base.join("lotes").join(l["carpeta"].as_str().unwrap());
                reconstruir(&c, &l["archivos"]);
                lotes.push(paquete::cargar_lote_con(&c, parcial).unwrap());
            }
            let destino = base.join(if csmp { "libro-csmp" } else { "libro" });
            let base_txt = base.canonicalize().unwrap().display().to_string();
            let mut d = Vec::new();
            match (exportar_libro(&lotes, Some(&destino), csmp, &ahora()), caso.get("error")) {
                (Err(err), Some(esperado)) => {
                    if err != esperado.as_str().unwrap() {
                        d.push(format!("error: Rust {err} · Python {esperado}"));
                    }
                }
                (Err(err), None) => d.push(format!("exportar: {err}")),
                (Ok(r), Some(esperado)) => d.push(format!("Rust exportó {} y Python rechazó: {esperado}", r.display())),
                (Ok(r), None) => {
                    if r.file_name().unwrap().to_string_lossy() != caso["archivo"].as_str().unwrap() {
                        d.push(format!("archivo: {}", r.display()));
                    }
                    let bytes = std::fs::read(&r).unwrap();
                    d.extend(volcado::diferencias(&volcado::volcar(&bytes, &base_txt), &caso["libro"]));
                    let mut meta: Value = serde_json::from_slice(&std::fs::read(r.with_extension("json")).unwrap()).unwrap();
                    assert_eq!(meta["sha256"], json!(sha256(&bytes)), "huella del XLSX en el metadato");
                    meta["sha256"] = json!("<SHA256>");
                    for f in meta["fuentes"].as_array_mut().unwrap() {
                        f["lote"] = json!(volcado::sin_base(f["lote"].as_str().unwrap(), &base_txt));
                    }
                    if meta != caso["metadato"] {
                        d.push(format!("metadato:\n    Rust   {meta}\n    Python {}", caso["metadato"]));
                    }
                }
            }
            if d.is_empty() { iguales += 1 } else { fallos.push(format!("{nombre} csmp={csmp}:\n  {}", d.join("\n  "))) }
        }
        let _ = std::fs::remove_dir_all(&raiz);
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("LIBROS: {iguales}/{}, fallos {}", casos.len(), fallos.len());
        assert_eq!(casos.len(), 14, "el oráculo debe traer 14 libros");
        assert!(fallos.is_empty());
    }
}
