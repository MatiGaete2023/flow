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
        assert!(fallos.is_empty());
    }
}
