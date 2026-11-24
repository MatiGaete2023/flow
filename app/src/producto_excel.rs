//! Producto Excel del Asistente con la estructura de Flow Python (`FAE_RES`): hojas de servicio, hoja de datos con
//! todas las columnas del registro y las columnas de revisión en el orden del usuario (decisión del 20-nov-2026).
//! Se usa cuando no hay un libro que preservar (por ejemplo, un paquete del Descargador sin consolidado). Escribe un
//! archivo nuevo con rust_xlsxwriter; no requiere Excel de escritorio y nunca toca el original.

use crate::copia_excel::{decisiones, identificadores, literal, revisada, TIPOS_RES};
use crate::productos::nueva_ruta;
use crate::trabajo::{Registro, Trabajo};
use crate::util::{normalizar, Cell};
use rust_xlsxwriter::{Color, DataValidation, ExcelDateTime, Format, FormatBorder, Workbook, Worksheet};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Orden del registro mensual (decisión del 20-nov-2026), igual en propuesta y revisión.
pub const COLUMNAS_NURUS: [&str; 10] = [
    "NURUS_ID_REGISTRO",
    "NURUS_REGLAS",
    "NURUS_PROPUESTA",
    "FECHA_OBS",
    "OBSERVACION",
    "TT",
    "CC",
    "RES",
    "NURUS_ESTADO_REVISION",
    "NURUS_DECISIONES",
];
/// Clave de ingreso que se conserva entre capturas (el `record_id` cambia con cada descarga).
pub const CLAVE_INGRESO: &str = "NURUS_CLAVE_INGRESO";
const CAMPOS_CLAVE: [&str; 3] = ["SITFA_TRIBUNAL_CODIGO", "SITFA_CAUSA_RUS", "SITFA_INGRESO_RUS"];
const FECHA_HORA: &str = "%Y-%m-%d %H-%M-%S";

fn err(e: rust_xlsxwriter::XlsxError) -> String {
    e.to_string()
}

fn pos(headers: &[String], titulo: &str) -> Option<usize> {
    let k = normalizar(titulo);
    headers.iter().position(|h| normalizar(h) == k)
}

/// Clave «tribunal-causa-ingreso» de una fila, si sus tres identificadores están.
pub fn clave_ingreso(headers: &[String], valores: &[Cell]) -> Option<String> {
    let partes: Vec<String> = CAMPOS_CLAVE
        .iter()
        .map(|c| pos(headers, c).and_then(|i| valores.get(i)).map(|v| v.display().trim().to_string()).unwrap_or_default())
        .collect();
    partes.iter().all(|p| !p.is_empty()).then(|| partes.join("-"))
}

fn estado(r: &Registro, etapa_revisada: bool) -> &'static str {
    if r.excluido {
        "EXCLUIDO"
    } else if !etapa_revisada && !r.avisos.is_empty() {
        "REQUIERE_REVISION"
    } else if !etapa_revisada {
        "PROPUESTA"
    } else if revisada(r) {
        "REVISADO"
    } else {
        "PENDIENTE"
    }
}

struct Formatos {
    cabecera: Format,
    fecha: Format,
    exclusion: Format,
}

impl Formatos {
    fn nuevos() -> Self {
        Formatos {
            cabecera: Format::new().set_bold().set_background_color(Color::RGB(0xDDE7F5)).set_border_bottom(FormatBorder::Thin),
            fecha: Format::new().set_num_format("dd/mm/yyyy"),
            exclusion: Format::new().set_background_color(Color::RGB(0xFFF2CC)),
        }
    }
}

fn escribir_texto(ws: &mut Worksheet, fila: u32, col: u16, v: &str) -> Result<(), String> {
    let v: String = v.chars().take(32767).collect();
    ws.write_string(fila, col, &v).map(|_| ()).map_err(err)
}

fn tabla(ws: &mut Worksheet, f: &Formatos, filas: &[Vec<String>], congelar: bool) -> Result<(), String> {
    for (i, fila) in filas.iter().enumerate() {
        for (c, v) in fila.iter().enumerate() {
            if i == 0 {
                ws.write_string_with_format(0, c as u16, v, &f.cabecera).map_err(err)?;
            } else {
                escribir_texto(ws, i as u32, c as u16, v)?;
            }
        }
    }
    for c in 0..filas.first().map_or(0, Vec::len) {
        let ancho = filas.iter().map(|r| r.get(c).map_or(0, |s| s.chars().count())).max().unwrap_or(0);
        ws.set_column_width(c as u16, (ancho as f64 + 2.0).clamp(10.0, 60.0)).map_err(err)?;
    }
    if congelar && !filas.is_empty() {
        ws.set_freeze_panes(1, 0).map_err(err)?;
    }
    Ok(())
}

/// Lo que el producto sabe de las fuentes de sus filas: una fila de `Resumen` por consulta y una de
/// `SITFA_ARCHIVOS` por archivo de origen.
struct Fuentes {
    resumen: Vec<Vec<String>>,
    archivos: Vec<Vec<String>>,
}

fn nombre_lote(capturada: &str) -> String {
    let t: String = capturada.chars().take(19).collect();
    chrono::NaiveDateTime::parse_from_str(&t, "%Y-%m-%dT%H:%M:%S")
        .map(|d| d.format(FECHA_HORA).to_string())
        .unwrap_or_else(|_| capturada.to_string())
}

fn fuentes(w: &Trabajo, hoja: &str) -> Fuentes {
    let cabecera_resumen = ["Resultado", "Lote", "Consulta", "Hoja", "Filas", "Compatibilidad CSMP", "Columnas faltantes"];
    let cabecera_archivos = ["Carpeta origen", "Archivo origen", "SHA256"];
    let mut resumen = vec![cabecera_resumen.iter().map(|s| s.to_string()).collect::<Vec<_>>()];
    let mut archivos = vec![cabecera_archivos.iter().map(|s| s.to_string()).collect::<Vec<_>>()];
    if let Some(p) = &w.paquete {
        let columna = w.headers.iter().position(|h| h == "ID_REGISTRO_CAPTURA");
        let propios: HashSet<String> = columna
            .map(|c| w.registros.iter().filter_map(|r| r.values.get(c)).map(Cell::display).collect())
            .unwrap_or_default();
        let consultas: BTreeMap<&str, _> = p.fuente.filas("consultas").map(|q| (q.get("query_id"), q)).collect();
        let mut por_consulta: BTreeMap<&str, (usize, String)> = BTreeMap::new();
        let mut vistos: Vec<(String, String)> = Vec::new();
        for r in p.fuente.filas("registros").filter(|r| propios.contains(r.get("record_id"))) {
            let e = por_consulta.entry(r.get("query_id")).or_insert((0, r.get("modality_raw").to_string()));
            e.0 += 1;
            let origen = (r.get("original_file").to_string(), r.get("source_sha256").to_string());
            if !vistos.contains(&origen) {
                vistos.push(origen);
            }
        }
        for (qid, (filas, modalidad)) in &por_consulta {
            let q = consultas.get(qid);
            let get = |k: &str| q.map(|q| q.get(k).to_string()).unwrap_or_default();
            let consulta = [get("court_name"), get("tab"), modalidad.clone()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" / ");
            resumen.push(vec!["VERIFICADO".into(), nombre_lote(&get("captured_at")), consulta, hoja.into(), filas.to_string(), "Desde paquete CSMP_CAPTURE".into(), String::new()]);
        }
        for (archivo, huella) in vistos {
            archivos.push(vec![format!("Paquete {}", p.fuente.manifiesto.capture_id), archivo, huella]);
        }
    } else {
        resumen.push(vec!["VERIFICADO".into(), String::new(), w.nombre.clone(), hoja.into(), w.registros.len().to_string(), "Desde el archivo abierto".into(), String::new()]);
        archivos.push(vec![w.path.parent().map(|p| p.display().to_string()).unwrap_or_default(), w.nombre.clone(), w.hash.clone()]);
    }
    Fuentes { resumen, archivos }
}

fn vencimientos(w: &Trabajo) -> Vec<Vec<String>> {
    let mut filas = vec![["Vencimiento", "Tribunal", "Lote", "Archivo", "Fila", "Estado de fecha"].iter().map(|s| s.to_string()).collect::<Vec<_>>()];
    let mut datos: Vec<(chrono::NaiveDate, String, String, String)> = Vec::new();
    let archivo = pos(&w.headers, "SITFA_ARCHIVO");
    for r in &w.registros {
        let clave = ["vencimiento", "egreso_proy"].into_iter().find(|k| w.cols.contains_key(*k));
        let Some(clave) = clave else { break };
        let crudo = w.value(r, clave);
        let (d, texto) = match w.date(r, clave) {
            Some(d) => (d, d.format("%Y-%m-%d").to_string()),
            None if crudo.is_empty() => continue,
            None => (chrono::NaiveDate::MAX, crudo),
        };
        let valida = d != chrono::NaiveDate::MAX;
        datos.push((
            d,
            texto + if valida { "|Fecha válida" } else { "|No interpretable" },
            w.value(r, "tribunal"),
            archivo.and_then(|c| r.values.get(c)).map(Cell::display).unwrap_or_default() + "|" + &r.fila_excel.to_string(),
        ));
    }
    datos.sort();
    for (_, texto, tribunal, origen) in datos {
        let (venc, estado) = texto.split_once('|').unwrap_or((&texto, ""));
        let (archivo, fila) = origen.split_once('|').unwrap_or((&origen, ""));
        filas.push(vec![venc.into(), tribunal, String::new(), archivo.into(), fila.into(), estado.into()]);
    }
    filas
}

fn trazabilidad(w: &Trabajo, hoja: &str, etapa_revisada: bool, motivo: Option<&str>) -> Vec<Vec<String>> {
    let huella = if w.hash.is_empty() { hex(&Sha256::digest(&w.bytes)) } else { w.hash.clone() };
    let mut filas: Vec<Vec<String>> = vec![
        vec!["CAMPO".into(), "VALOR".into()],
        vec!["ESTADO_DOCUMENTO".into(), if etapa_revisada { "CONSTANCIA_REVISADA" } else { "PROPUESTA_NO_REVISADA" }.into()],
        vec!["EVALUACION_SHA256".into(), String::new()],
        vec!["SNAPSHOT".into(), String::new()],
        vec!["FUENTE_SHA256".into(), huella.clone()],
        vec!["MODO".into(), w.mode().label().to_uppercase()],
        vec!["HOJA_PROCESADA".into(), hoja.into()],
        vec!["EXCEPCIONES".into(), "[]".into()],
        vec!["GENERADO_POR".into(), "Flow Rust (libro nuevo, sin copia preservada)".into()],
    ];
    if let Some(m) = motivo {
        filas.push(vec!["FIDELIDAD".into(), m.to_string()]);
    }
    filas.push(Vec::new());
    filas.push(["HOJA", "FILA", "ESTADO", "MOTIVO", "OBSERVACION", "REGLAS", "HASH_ORIGEN"].iter().map(|s| s.to_string()).collect());
    for r in &w.registros {
        let decision = if r.excluido { "excluded" } else if revisada(r) { "approved" } else { "pending" };
        filas.push(vec![
            hoja.into(),
            r.fila_excel.to_string(),
            decision.into(),
            r.avisos.join("; "),
            literal(r.observacion()),
            serde_json::to_string(&r.eventos).unwrap_or_else(|_| "[]".into()),
            huella.clone(),
        ]);
    }
    filas
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn es_nurus(titulo: &str) -> bool {
    let k = normalizar(titulo);
    COLUMNAS_NURUS.iter().chain(&[CLAVE_INGRESO]).any(|t| normalizar(t) == k)
}

/// Encabezados finales de la hoja de datos: las columnas del registro (sin las de revisión) y, al final, las de
/// revisión en el orden del usuario aunque el origen ya trajera alguna en otro lugar.
pub fn encabezados(w: &Trabajo) -> Vec<String> {
    let mut h: Vec<String> = w.headers.iter().filter(|t| !es_nurus(t)).cloned().collect();
    h.extend(COLUMNAS_NURUS.iter().map(|t| t.to_string()));
    if w.registros.first().is_some_and(|r| clave_ingreso(&w.headers, &r.values).is_some()) {
        h.push(CLAVE_INGRESO.to_string());
    }
    h
}

/// Filas de la hoja de datos, alineadas con `encabezados`.
pub fn filas(w: &Trabajo) -> Vec<Vec<Cell>> {
    let h = encabezados(w);
    let etapa_revisada = w.registros.iter().any(revisada);
    let hoja = w.mode().label().to_uppercase();
    let ids = identificadores(w, &hoja);
    w.registros
        .iter()
        .zip(&ids)
        .map(|(r, id)| {
            let mut fila: Vec<Cell> = vec![Cell::Empty; h.len()];
            for (t, v) in w.headers.iter().zip(&r.values) {
                if let Some(i) = pos(&h, t) {
                    fila[i] = v.clone();
                }
            }
            let aprobada = revisada(r) && !r.excluido;
            let mut poner = |titulo: &str, v: String| {
                if let Some(i) = pos(&h, titulo) {
                    fila[i] = Cell::Text(v);
                }
            };
            poner("NURUS_ID_REGISTRO", id.clone());
            poner("NURUS_REGLAS", serde_json::to_string(&r.eventos).unwrap_or_else(|_| "[]".into()));
            poner("NURUS_PROPUESTA", literal(&r.propuesta));
            poner("NURUS_ESTADO_REVISION", estado(r, etapa_revisada).into());
            poner("NURUS_DECISIONES", decisiones(r));
            if !etapa_revisada || aprobada {
                poner("OBSERVACION", literal(r.observacion()));
            }
            if !r.res.is_empty() && TIPOS_RES.contains(&r.res.as_str()) {
                poner("RES", r.res.clone());
            }
            if let Some(k) = clave_ingreso(&w.headers, &r.values) {
                poner(CLAVE_INGRESO, k);
            }
            fila
        })
        .collect()
}

/// Hoja de datos del producto: encabezado, valores con su tipo, filtro, panel congelado, columnas ocultas, lista de
/// RES y las filas excluidas sombreadas. La usan el producto general y la revisión de bitácoras.
pub fn hoja_datos(nombre: &str, h: &[String], datos: &[Vec<Cell>], excluidas: &[bool]) -> Result<Worksheet, String> {
    let f = Formatos::nuevos();
    let mut ws = Worksheet::new();
    ws.set_name(nombre).map_err(err)?;
    for (c, t) in h.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, t, &f.cabecera).map_err(err)?;
        let ancho = datos.iter().map(|r| r.get(c).map_or(0, |v| v.display().chars().count())).max().unwrap_or(0).max(t.chars().count());
        ws.set_column_width(c as u16, (ancho as f64 + 2.0).clamp(10.0, 70.0)).map_err(err)?;
    }
    for (i, (fila, excluido)) in datos.iter().zip(excluidas).enumerate() {
        let n = i as u32 + 1;
        for (c, v) in fila.iter().enumerate() {
            let c = c as u16;
            if *excluido {
                ws.write_string_with_format(n, c, &v.display(), &f.exclusion).map_err(err)?;
                continue;
            }
            match v {
                Cell::Empty => {}
                Cell::Text(s) => escribir_texto(&mut ws, n, c, s)?,
                Cell::Number(x) => {
                    ws.write_number(n, c, *x).map_err(err)?;
                }
                Cell::Bool(b) => {
                    ws.write_boolean(n, c, *b).map_err(err)?;
                }
                Cell::Date(d) => {
                    let fecha = ExcelDateTime::from_ymd(chrono::Datelike::year(d) as u16, chrono::Datelike::month(d) as u8, chrono::Datelike::day(d) as u8).map_err(err)?;
                    ws.write_datetime_with_format(n, c, &fecha, &f.fecha).map_err(err)?;
                }
            }
        }
    }
    if !h.is_empty() {
        ws.autofilter(0, 0, datos.len() as u32, h.len() as u16 - 1).map_err(err)?;
        ws.set_freeze_panes(1, 0).map_err(err)?;
    }
    for oculta in ["NURUS_REGLAS", "NURUS_DECISIONES"] {
        if let Some(i) = pos(&h, oculta) {
            ws.set_column_hidden(i as u16).map_err(err)?;
        }
    }
    if let (Some(i), true) = (pos(&h, "RES"), !datos.is_empty()) {
        let lista = DataValidation::new()
            .allow_list_strings(&TIPOS_RES)
            .map_err(err)?
            .ignore_blank(true)
            .set_error_title("Tipo RES no válido")
            .map_err(err)?
            .set_error_message("Usa PC_IE, PC_INFO o NOMENCL, o deja la celda vacía.")
            .map_err(err)?
            .set_input_title("Tipo de resolución")
            .map_err(err)?
            .set_input_message("Selecciona PC_IE, PC_INFO o NOMENCL. Vacío: sin proyecto.")
            .map_err(err)?;
        ws.add_data_validation(1, i as u16, datos.len() as u32, i as u16, &lista).map_err(err)?;
        ws.set_column_width(i as u16, 14).map_err(err)?;
    }

    Ok(ws)
}

/// Escribe el producto y devuelve su ruta (nueva: nunca reemplaza un archivo).
pub fn exportar(w: &Trabajo, dir: &Path, motivo: Option<&str>) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("No se pudo crear la carpeta de productos: {e}"))?;
    let fecha = w.corte.map(|c| c.format("%Y-%m-%d").to_string()).unwrap_or_default();
    let ruta = nueva_ruta(dir, &format!("Observaciones_{}_{}", w.mode().label().to_uppercase(), fecha), "xlsx");
    let hoja = w.mode().label().to_uppercase();
    let h = encabezados(w);
    let datos = filas(w);
    let etapa_revisada = w.registros.iter().any(revisada);
    let f = Formatos::nuevos();
    let excluidas: Vec<bool> = w.registros.iter().map(|r| r.excluido).collect();
    let mut libro = Workbook::new();

    // NURUS_LISTAS y NURUS_TRAZABILIDAD: ocultas, como en la copia preservada.
    let mut listas = Worksheet::new();
    listas.set_name("NURUS_LISTAS").map_err(err)?;
    escribir_texto(&mut listas, 0, 0, "TIPOS_RES")?;
    for (i, t) in TIPOS_RES.iter().enumerate() {
        escribir_texto(&mut listas, i as u32 + 1, 0, t)?;
    }
    listas.set_hidden(true);
    libro.push_worksheet(listas);

    let mut traza = Worksheet::new();
    traza.set_name("NURUS_TRAZABILIDAD").map_err(err)?;
    for (i, fila) in trazabilidad(w, &hoja, etapa_revisada, motivo).iter().enumerate() {
        for (c, v) in fila.iter().enumerate() {
            escribir_texto(&mut traza, i as u32, c as u16, v)?;
        }
    }
    traza.set_hidden(true);
    libro.push_worksheet(traza);

    let fu = fuentes(w, &hoja);
    for (nombre, filas) in [("Resumen", fu.resumen), ("SITFA_ARCHIVOS", fu.archivos)] {
        let mut ws = Worksheet::new();
        ws.set_name(nombre).map_err(err)?;
        tabla(&mut ws, &f, &filas, true)?;
        libro.push_worksheet(ws);
    }

    libro.push_worksheet(hoja_datos(&hoja, &h, &datos, &excluidas)?);

    let mut venc = Worksheet::new();
    venc.set_name("Vencimientos").map_err(err)?;
    tabla(&mut venc, &f, &vencimientos(w), true)?;
    libro.push_worksheet(venc);

    // La hoja de datos es la activa, como en la copia preservada.
    libro.worksheet_from_name(&hoja).map_err(err)?.set_active(true);
    libro.save(&ruta).map_err(|e| format!("No se pudo guardar {}: {e}", ruta.display()))?;
    Ok(ruta)
}

/// Excel de la revisión de bitácoras sin libro base: las filas elegidas del producto general, con la misma hoja de datos,
/// y al final las columnas de la revisión.
pub fn exportar_subconjunto(w: &Trabajo, indices: &[usize], extras: &[&str], valores: &[Vec<String>], ruta: &Path) -> Result<(), String> {
    let mut h = encabezados(w);
    h.extend(extras.iter().map(|t| t.to_string()));
    let todas = filas(w);
    let mut datos = Vec::with_capacity(indices.len());
    let mut excluidas = Vec::with_capacity(indices.len());
    for (&i, extra) in indices.iter().zip(valores) {
        let mut fila = todas.get(i).cloned().ok_or("La selección contiene una fila que ya no está en Trabajo.")?;
        fila.extend(extra.iter().map(|v| Cell::Text(v.clone())));
        datos.push(fila);
        excluidas.push(w.registros[i].excluido);
    }
    let mut libro = Workbook::new();
    libro.push_worksheet(hoja_datos(&w.mode().label().to_uppercase(), &h, &datos, &excluidas)?);
    libro.save(ruta).map_err(|e| format!("No se pudo guardar {}: {e}", ruta.display()))
}
