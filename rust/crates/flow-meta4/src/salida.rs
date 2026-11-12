//! Libros de salida de Meta 4. Se escriben en una carpeta temporal hermana y solo se publican completos:
//! un error no deja una planilla parcial.

use crate::tribunales::{norma, TRIBUNALES};
use crate::{periodo, Meta4, Registro4, SIN_TRIBUNAL};
use chrono::{Datelike, NaiveDate, Timelike};
use flow_dominio::Celda;
use rust_xlsxwriter::{Color, ExcelDateTime, Format, FormatAlign, FormatBorder, Note, Workbook, Worksheet, XlsxError};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ROJO: u32 = 0xFFC7CE;
const AMARILLO: u32 = 0xFFF2CC;
const AZUL: u32 = 0xDDEBF7;

pub const ESPERA: [&str; 24] = [
    "RIT", "TRIBUNAL", "RUT", "NOMBRE", "NACIONALIDAD / PAIS", "TIPO", "SEXO", "DERIVACIÓN", "CURADOR", "T ESPERA",
    "FEC. RESOLUCIÓN", "REVISIÓN", "DURACIÓN", "80 BIS", "FEC. NACIMIENTO", "EDAD", "FEC. ACT. F. INDIVIDUAL",
    "FEC. ACT. F. AMBULATORIA", "FEC. ACT. F. FAE/FAS", "FEC. OÍDO", "PROXS. AUDS.", "FECHA OBS.", "ARCHIVO ORIGEN",
    "RUTA ORIGEN",
];
pub const CUMPLIMIENTO: [&str; 24] = [
    "RIT", "TRIBUNAL", "RUT", "NOMBRE", "NACIONALIDAD / PAÍS", "TIPO", "SEXO", "DERIVACIÓN", "CURADOR",
    "FEC. RESOLUCIÓN", "FEC. INGRESO EFECTIVO", "FEC. EGRESO PROYECTADO", "DÍAS DE CUMPLIMIENTO", "DÍAS PARA EGRESAR",
    "80 BIS", "FEC. NACIMIENTO", "EDAD", "FEC. ACT. F. INDIVIDUAL", "FEC. ACT. F. AMBULATORIA", "FEC. ACT. F. FAE/FAS",
    "FEC. OÍDO", "FECHA OBS.", "ARCHIVO ORIGEN", "RUTA ORIGEN",
];
pub const EXTRA: [&str; 3] = ["ETAPA ÚLTIMA OBS.", "CONTROL BITÁCORA", "ALERTA"];
const RESUMEN: [&str; 10] = [
    "TRIBUNAL", "FAE ESPERA", "FAE CUMPLIMIENTO", "RESIDENCIA ESPERA", "RESIDENCIA CUMPLIMIENTO", "TOTAL REGISTROS",
    "ALERTAS", "REVISAR BITÁCORA", "SIN FECHA OBS.", "CONSULTAS SIN BITÁCORAS",
];
/// Hojas de la planilla: (nombre, lista, categoría, título del informe).
pub const HOJAS: [(&str, &str, &str, &str); 4] = [
    ("Residencia Espera", "Espera", "Residencia", "Informe de Ingresos en Lista de Espera"),
    ("Residencia Cumplimiento", "Cumplimiento", "Residencia", "Informe de Ingresos en Lista de Cumplimiento"),
    ("FAE Espera", "Espera", "FAE", "Informe de Ingresos en Lista de Espera"),
    ("FAE Cumplimiento", "Cumplimiento", "FAE", "Informe de Ingresos en Lista de Cumplimiento"),
];

fn en(e: XlsxError) -> String {
    e.to_string()
}

fn es_fecha(titulo: &str) -> bool {
    let n = norma(titulo);
    n.starts_with("FEC ") || n == "FECHA OBS"
}

fn fecha_excel(f: NaiveDate) -> Option<ExcelDateTime> {
    ExcelDateTime::from_ymd(f.year() as u16, f.month() as u8, f.day() as u8).ok()
}

/// Valor de un registro para un título de las hojas de Registros.
fn valor(r: &Registro4, titulo: &str) -> Celda {
    match titulo {
        "FECHA OBS." => r.fecha_obs.map(Celda::Fecha).unwrap_or_default(),
        "ARCHIVO ORIGEN" => Celda::Texto(r.archivo.clone()),
        "RUTA ORIGEN" => Celda::Texto(r.lote.clone()),
        "ETAPA ÚLTIMA OBS." => Celda::Texto(r.etapa_ultima.clone()),
        "CONTROL BITÁCORA" => Celda::Texto(r.control.clone()),
        "ALERTA" => Celda::Texto(r.alerta.clone()),
        "UBICACIÓN" => Celda::Texto(r.lista.to_string()),
        "DERIVACIÓN" => {
            let directo = r.valor(titulo);
            if directo.es_vacia() {
                ["PROGRAMA", "NOMBRE CENTRO"].iter().map(|t| r.valor(t)).find(|c| !c.es_vacia()).unwrap_or_default()
            } else {
                directo
            }
        }
        t if es_fecha(t) => {
            let c = r.valor(t);
            c.fecha().map(Celda::Fecha).unwrap_or(c)
        }
        t => r.valor(t),
    }
}

struct Estilos {
    cabecera: Format,
    normal: Format,
    fecha: Format,
    negrita: Format,
}

fn estilos(relleno: Option<u32>) -> Estilos {
    let mut base = Format::new().set_border(FormatBorder::Thin);
    if let Some(color) = relleno {
        base = base.set_background_color(Color::RGB(color));
    }
    Estilos {
        cabecera: Format::new()
            .set_bold()
            .set_background_color(Color::RGB(AZUL))
            .set_border(FormatBorder::Thin)
            .set_align(FormatAlign::Center)
            .set_text_wrap(),
        fecha: base.clone().set_num_format("dd-mm-yyyy"),
        normal: base,
        negrita: Format::new().set_bold(),
    }
}

fn escribir(hoja: &mut Worksheet, fila: u32, col: u16, celda: &Celda, e: &Estilos) -> Result<(), XlsxError> {
    match celda {
        Celda::Vacia => hoja.write_blank(fila, col, &e.normal)?,
        // Siempre como texto: un valor que empieza con «=» no se convierte en fórmula.
        Celda::Texto(t) => hoja.write_string_with_format(fila, col, t, &e.normal)?,
        Celda::Numero(n) => hoja.write_number_with_format(fila, col, *n, &e.normal)?,
        Celda::Fecha(f) => match fecha_excel(*f) {
            Some(fecha) => hoja.write_datetime_with_format(fila, col, &fecha, &e.fecha)?,
            None => hoja.write_string_with_format(fila, col, celda.texto(), &e.normal)?,
        },
    };
    Ok(())
}

/// Tabla simple con encabezado, filtro y marcas de color por fila (rojo alerta, amarillo revisar).
fn tabla(hoja: &mut Worksheet, titulos: &[&str], filas: &[(Vec<Celda>, Option<u32>)]) -> Result<(), XlsxError> {
    let base = estilos(None);
    for (c, titulo) in titulos.iter().enumerate() {
        hoja.write_string_with_format(0, c as u16, *titulo, &base.cabecera)?;
        hoja.set_column_width(c as u16, (titulo.chars().count() as f64 + 4.0).clamp(12.0, 40.0))?;
    }
    let (rojo, amarillo) = (estilos(Some(ROJO)), estilos(Some(AMARILLO)));
    for (f, (celdas, color)) in filas.iter().enumerate() {
        let e = match color {
            Some(ROJO) => &rojo,
            Some(_) => &amarillo,
            None => &base,
        };
        for (c, celda) in celdas.iter().enumerate() {
            escribir(hoja, f as u32 + 1, c as u16, celda, e)?;
        }
    }
    hoja.set_freeze_panes(1, 0)?;
    if !filas.is_empty() {
        hoja.autofilter(0, 0, filas.len() as u32, titulos.len() as u16 - 1)?;
    }
    Ok(())
}

fn color(r: &Registro4) -> Option<u32> {
    if !r.alerta.is_empty() {
        Some(ROJO)
    } else if r.requiere_revision() {
        Some(AMARILLO)
    } else {
        None
    }
}

fn fila_registro(r: &Registro4, titulos: &[&str]) -> (Vec<Celda>, Option<u32>) {
    (titulos.iter().map(|t| valor(r, t)).collect(), color(r))
}

/// Conteos por tribunal y fila TOTAL.
pub fn resumen(m: &Meta4) -> Vec<BTreeMap<&'static str, Value>> {
    let mut conteos: BTreeMap<String, BTreeMap<&'static str, i64>> = BTreeMap::new();
    for r in &m.registros {
        let c = conteos.entry(r.tribunal_o_sin().to_string()).or_default();
        let lista = match (r.categoria, r.lista) {
            (Some("FAE"), "Espera") => Some("FAE ESPERA"),
            (Some("FAE"), _) => Some("FAE CUMPLIMIENTO"),
            (Some(_), "Espera") => Some("RESIDENCIA ESPERA"),
            (Some(_), _) => Some("RESIDENCIA CUMPLIMIENTO"),
            (None, _) => None,
        };
        if let Some(l) = lista {
            *c.entry(l).or_default() += 1;
        }
        *c.entry("TOTAL REGISTROS").or_default() += 1;
        *c.entry("ALERTAS").or_default() += i64::from(!r.alerta.is_empty());
        *c.entry("REVISAR BITÁCORA").or_default() += i64::from(!r.revisada());
        *c.entry("SIN FECHA OBS.").or_default() += i64::from(r.fecha_obs.is_none());
    }
    for (_, tribunal) in &m.sin_bitacoras {
        *conteos.entry(tribunal.clone()).or_default().entry("CONSULTAS SIN BITÁCORAS").or_default() += 1;
    }
    let mut orden: Vec<String> = TRIBUNALES
        .iter()
        .filter(|t| match &m.tribunales {
            Some(elegidos) => elegidos.contains(t),
            None => conteos.contains_key(**t),
        })
        .map(|t| t.to_string())
        .collect();
    orden.extend(conteos.keys().filter(|t| !TRIBUNALES.contains(&t.as_str())).cloned());
    let mut total: BTreeMap<&'static str, i64> = BTreeMap::new();
    let mut salida = Vec::new();
    for t in orden {
        let c = conteos.get(&t).cloned().unwrap_or_default();
        let mut fila: BTreeMap<&'static str, Value> = BTreeMap::new();
        fila.insert("TRIBUNAL", json!(t));
        for titulo in &RESUMEN[1..] {
            let n = c.get(titulo).copied().unwrap_or(0);
            *total.entry(titulo).or_default() += n;
            fila.insert(titulo, json!(n));
        }
        salida.push(fila);
    }
    let mut fila: BTreeMap<&'static str, Value> = BTreeMap::new();
    fila.insert("TRIBUNAL", json!("TOTAL"));
    for titulo in &RESUMEN[1..] {
        fila.insert(titulo, json!(total.get(titulo).copied().unwrap_or(0)));
    }
    salida.push(fila);
    salida
}

/// Líneas de la hoja «Control bitácoras».
fn lineas_control(m: &Meta4) -> Vec<Vec<String>> {
    let (inicio, fin) = periodo(m.corte);
    let mut estados: Vec<&str> = m.lecturas.iter().map(|l| l.control.as_str()).collect();
    estados.sort_unstable();
    estados.dedup();
    let dia = |f: NaiveDate| f.format("%d-%m-%Y").to_string();
    vec![
        vec!["Período de bitácoras".into(), dia(inicio), dia(fin)],
        vec!["Lectura de bitácoras".into(), if estados.is_empty() { "Sin lecturas".into() } else { estados.join(", ") }],
        vec!["Consultas sin bitácoras".into(), m.sin_bitacoras.len().to_string()],
        vec!["Tribunales incluidos".into(), m.tribunales.as_ref().map_or("Todos".into(), |t| t.join(", "))],
        vec!["Filtros del lote de registros".into(), if m.filtros.is_empty() { "Sin filtros".into() } else { m.filtros.join(" | ") }],
        vec!["Lotes de registros".into(), m.lotes.len().to_string(), m.lotes.join(" | ")],
        vec!["Rojo: última observación en Espera y registro en Cumplimiento. Amarillo: revisar control de bitácora.".into()],
        vec!["Sin observación en el mes: FECHA OBS. vacía. Informes no participa.".into()],
    ]
}

/// Planilla META 4: cuatro hojas con un bloque por tribunal y la hoja de control.
pub fn planilla(m: &Meta4, destino: &Path) -> Result<BTreeMap<String, usize>, String> {
    let mut libro = Workbook::new();
    let bloques: Vec<&str> = TRIBUNALES.iter().copied().filter(|t| m.tribunales.as_ref().is_none_or(|e| e.contains(t))).collect();
    let emision = format!("Fecha Emisión   : {}", m.emision.format("%d-%m-%Y"));
    let (base, rojo, amarillo) = (estilos(None), estilos(Some(ROJO)), estilos(Some(AMARILLO)));
    let mut totales = BTreeMap::new();
    for (nombre, lista, categoria, informe) in HOJAS {
        let titulos: [&str; 8] = if lista == "Espera" {
            ["RIT", "TRIBUNAL", "TIPO", "SEXO", "DERIVACIÓN", "T ESPERA", "FEC. RESOLUCIÓN", "FECHA OBS."]
        } else {
            ["RIT", "TRIBUNAL", "TIPO", "SEXO", "DERIVACIÓN", "FEC. RESOLUCIÓN", "FEC. INGRESO EFECTIVO", "FECHA OBS."]
        };
        let hoja = libro.add_worksheet();
        hoja.set_name(nombre).map_err(en)?;
        hoja.set_screen_gridlines(false);
        hoja.set_landscape();
        hoja.set_print_fit_to_pages(1, 0);
        for (c, ancho) in [13, 34, 15, 8, 44, 18, 22, 16].into_iter().enumerate() {
            hoja.set_column_width(c as u16, ancho).map_err(en)?;
        }
        let mut fila = 0u32;
        let mut total = 0usize;
        for bloque in &bloques {
            let registros: Vec<&Registro4> = m
                .registros
                .iter()
                .filter(|r| r.lista == lista && r.categoria == Some(categoria) && r.tribunal == Some(*bloque))
                .collect();
            hoja.write_string_with_format(fila, 0, *bloque, &base.negrita).map_err(en)?;
            hoja.write_string(fila + 1, 0, informe).map_err(en)?;
            hoja.write_string(fila + 2, 0, &emision).map_err(en)?;
            fila += 4;
            if registros.is_empty() {
                hoja.write_string_with_format(fila, 0, "SIN REGISTROS", &base.negrita).map_err(en)?;
                fila += 1;
            } else {
                for (c, titulo) in titulos.iter().enumerate() {
                    hoja.write_string_with_format(fila, c as u16, *titulo, &base.cabecera).map_err(en)?;
                }
                fila += 1;
                for r in &registros {
                    let e = match color(r) {
                        Some(ROJO) => &rojo,
                        Some(_) => &amarillo,
                        None => &base,
                    };
                    for (c, titulo) in titulos.iter().enumerate() {
                        let celda = match *titulo {
                            "T ESPERA" => {
                                let v = valor(r, titulo);
                                v.entero_laxo().map(|n| Celda::Numero(n as f64)).unwrap_or(v)
                            }
                            t => valor(r, t),
                        };
                        escribir(hoja, fila, c as u16, &celda, e).map_err(en)?;
                    }
                    if r.requiere_revision() {
                        let motivo = if r.alerta.is_empty() { &r.control } else { &r.alerta };
                        hoja.insert_note(fila, 7, &Note::new(motivo).set_author("META 4")).map_err(en)?;
                    }
                    fila += 1;
                }
                total += registros.len();
            }
            fila += 1;
        }
        fila += 1;
        hoja.write_string_with_format(fila, 0, "TOTAL:", &base.negrita).map_err(en)?;
        hoja.write_number_with_format(fila, 1, total as f64, &base.negrita).map_err(en)?;
        totales.insert(nombre.to_string(), total);
    }
    let control = libro.add_worksheet();
    control.set_name("Control bitácoras").map_err(en)?;
    control.set_column_width(0, 34).map_err(en)?;
    control.set_column_width(1, 30).map_err(en)?;
    control.set_column_width(2, 60).map_err(en)?;
    for (f, linea) in lineas_control(m).iter().enumerate() {
        for (c, texto) in linea.iter().enumerate() {
            control.write_string(f as u32, c as u16, texto).map_err(en)?;
        }
    }
    libro.save(destino).map_err(|e| format!("No se pudo guardar {}: {e}", destino.display()))?;
    Ok(totales)
}

fn con_extra(base: &[&'static str]) -> Vec<&'static str> {
    base.iter().copied().chain(EXTRA).collect()
}

/// Registros.xlsx: la hoja de trabajo, con todas las columnas de origen y los controles.
pub fn registros(m: &Meta4, destino: &Path) -> Result<(), String> {
    let mut libro = Workbook::new();
    for (nombre, lista, titulos) in [("ESPERA", "Espera", con_extra(&ESPERA)), ("CUMPLIMIENTO", "Cumplimiento", con_extra(&CUMPLIMIENTO))] {
        let filas: Vec<_> = m.registros.iter().filter(|r| r.lista == lista).map(|r| fila_registro(r, &titulos)).collect();
        let hoja = libro.add_worksheet();
        hoja.set_name(nombre).map_err(en)?;
        tabla(hoja, &titulos, &filas).map_err(en)?;
    }
    let avisos = libro.add_worksheet();
    avisos.set_name("ERRORES_Y_ADVERTENCIAS").map_err(en)?;
    avisos.set_column_width(0, 140).map_err(en)?;
    avisos.write_string_with_format(0, 0, "ADVERTENCIA", &estilos(None).cabecera).map_err(en)?;
    for (i, aviso) in m.avisos.iter().enumerate() {
        avisos.write_string(i as u32 + 1, 0, aviso).map_err(en)?;
    }
    libro.save(destino).map_err(|e| format!("No se pudo guardar {}: {e}", destino.display()))
}

/// Registros consolidados: resumen por tribunal, Espera, Cumplimiento, duplicados y lo que requiere revisión.
pub fn consolidado(m: &Meta4, destino: &Path) -> Result<(), String> {
    let mut libro = Workbook::new();
    let hoja = libro.add_worksheet();
    hoja.set_name("Resumen por tribunal").map_err(en)?;
    let filas: Vec<(Vec<Celda>, Option<u32>)> = resumen(m)
        .iter()
        .map(|f| {
            let celdas = RESUMEN
                .iter()
                .map(|t| match &f[t] {
                    Value::String(s) => Celda::Texto(s.clone()),
                    v => Celda::Numero(v.as_f64().unwrap_or(0.0)),
                })
                .collect();
            (celdas, None)
        })
        .collect();
    tabla(hoja, &RESUMEN, &filas).map_err(en)?;
    hoja.set_column_width(0, 30).map_err(en)?;
    let (espera, cumplimiento) = (con_extra(&ESPERA), con_extra(&CUMPLIMIENTO));
    for (nombre, titulos) in [("Espera", &espera), ("Cumplimiento", &cumplimiento)] {
        let filas: Vec<_> = m.registros.iter().filter(|r| r.lista == nombre).map(|r| fila_registro(r, titulos)).collect();
        let hoja = libro.add_worksheet();
        hoja.set_name(nombre).map_err(en)?;
        tabla(hoja, titulos, &filas).map_err(en)?;
    }
    // Títulos unificados de ambas listas, sin repetir los que solo difieren en tildes o puntuación.
    let mut unificados: Vec<&'static str> = vec!["UBICACIÓN"];
    for t in ESPERA.iter().chain(CUMPLIMIENTO.iter()).chain(EXTRA.iter()) {
        if !unificados.iter().any(|u| norma(u) == norma(t)) {
            unificados.push(t);
        }
    }
    let clave = |r: &Registro4| (flow_dominio::texto::rut_clave(&r.texto("RUT")), norma(&r.texto("NOMBRE")));
    let mut grupos: BTreeMap<(String, String), usize> = BTreeMap::new();
    for r in &m.registros {
        let k = clave(r);
        if !k.0.is_empty() && !k.1.is_empty() {
            *grupos.entry(k).or_default() += 1;
        }
    }
    let duplicados: Vec<_> =
        m.registros.iter().filter(|r| grupos.get(&clave(r)).is_some_and(|n| *n > 1)).map(|r| fila_registro(r, &unificados)).collect();
    let hoja = libro.add_worksheet();
    hoja.set_name("Duplicados").map_err(en)?;
    tabla(hoja, &unificados, &duplicados).map_err(en)?;
    let revisar: Vec<_> = m.registros.iter().filter(|r| r.requiere_revision()).map(|r| fila_registro(r, &unificados)).collect();
    let hoja = libro.add_worksheet();
    hoja.set_name("Alertas y revisión").map_err(en)?;
    tabla(hoja, &unificados, &revisar).map_err(en)?;
    libro.save(destino).map_err(|e| format!("No se pudo guardar {}: {e}", destino.display()))
}

/// Observaciones del centro dentro del período y control de cada lectura.
pub fn bitacoras_del_mes(m: &Meta4, destino: &Path) -> Result<(), String> {
    let mut libro = Workbook::new();
    let hoja = libro.add_worksheet();
    hoja.set_name("Observaciones del mes").map_err(en)?;
    let titulos = ["TRIBUNAL", "RIT", "RUT", "NOMBRE", "PROGRAMA", "CAUSA ID", "INGRESO ID", "FECHA CENTRO", "ETAPA", "TIPO OBSERVACIÓN", "OBSERVACIÓN CENTRO"];
    let base = estilos(None);
    let momento = Format::new().set_border(FormatBorder::Thin).set_num_format("dd-mm-yyyy hh:mm:ss");
    for (c, t) in titulos.iter().enumerate() {
        hoja.write_string_with_format(0, c as u16, *t, &base.cabecera).map_err(en)?;
        hoja.set_column_width(c as u16, if c == 10 { 90.0 } else { 20.0 }).map_err(en)?;
    }
    for (i, o) in m.observaciones.iter().enumerate() {
        let f = i as u32 + 1;
        for (c, texto) in [&o.tribunal, &o.rit, &o.rut, &o.nombre, &o.programa, &o.causa, &o.ingreso].into_iter().enumerate() {
            hoja.write_string_with_format(f, c as u16, texto, &base.normal).map_err(en)?;
        }
        let fecha = ExcelDateTime::from_ymd(o.fecha.year() as u16, o.fecha.month() as u8, o.fecha.day() as u8)
            .and_then(|d| d.and_hms(o.fecha.hour() as u16, o.fecha.minute() as u8, o.fecha.second() as f64))
            .map_err(en)?;
        hoja.write_datetime_with_format(f, 7, &fecha, &momento).map_err(en)?;
        for (c, texto) in [(8, &o.etapa), (9, &o.tipo), (10, &o.texto)] {
            hoja.write_string_with_format(f, c, texto, &base.normal).map_err(en)?;
        }
    }
    hoja.set_freeze_panes(1, 0).map_err(en)?;
    let control = libro.add_worksheet();
    control.set_name("Control de lecturas").map_err(en)?;
    let titulos = ["TRIBUNAL", "RIT", "NOMBRE", "PROGRAMA", "CAUSA ID", "INGRESO ID", "CONTROL"];
    let filas: Vec<(Vec<Celda>, Option<u32>)> = m
        .lecturas
        .iter()
        .map(|l| {
            let textos = [&l.tribunal, &l.rit, &l.nombre, &l.programa, &l.causa, &l.ingreso, &l.control];
            (textos.into_iter().map(|t| Celda::Texto(t.clone())).collect(), None)
        })
        .collect();
    tabla(control, &titulos, &filas).map_err(en)?;
    libro.save(destino).map_err(|e| format!("No se pudo guardar {}: {e}", destino.display()))
}

/// Reporte en JSON: período, tribunales, resumen, alertas, controles y avisos. Sin datos de personas.
pub fn reporte(m: &Meta4, totales: &BTreeMap<String, usize>) -> Value {
    let (inicio, fin) = periodo(m.corte);
    let mut controles: BTreeMap<&str, usize> = BTreeMap::new();
    for r in &m.registros {
        *controles.entry(r.control.as_str()).or_default() += 1;
    }
    json!({
        "herramienta": "Flow · Meta 4",
        "periodo": {"desde": inicio.to_string(), "hasta": fin.to_string()},
        "fecha_emision": m.emision.to_string(),
        "tribunales_incluidos": m.tribunales.as_ref().map(|t| json!(t)).unwrap_or(json!("Todos")),
        "resumen_por_tribunal": resumen(m),
        "totales_por_hoja": totales,
        "registros": m.registros.len(),
        "alertas": m.registros.iter().filter(|r| !r.alerta.is_empty()).count(),
        "requieren_revision": m.registros.iter().filter(|r| r.requiere_revision()).count(),
        "sin_fecha_obs": m.registros.iter().filter(|r| r.fecha_obs.is_none()).count(),
        "fuera_de_meta4": m.registros.iter().filter(|r| r.categoria.is_none() || r.tribunal.is_none()).count(),
        "conteo_por_control": controles,
        "consultas_sin_bitacoras": m.sin_bitacoras.iter().map(|(c, _)| c).collect::<Vec<_>>(),
        "filtros_registros": m.filtros,
        "lotes": m.lotes,
        "avisos": m.avisos,
        "estado": "Propuesta: requiere revisión del usuario",
    })
}

/// Escribe los cinco productos en una carpeta nueva. Si algo falla, no queda nada publicado.
pub fn publicar(m: &Meta4, destino: &Path) -> Result<Vec<PathBuf>, String> {
    publicar_interno(m, destino, None)
}

/// Regeneración desde Registros.xlsx revisado. La bitácora no cambia ni se sustituye por una vacía.
pub fn publicar_regenerado(
    m: &Meta4, destino: &Path, bitacoras_originales: &Path, huella_registros: &str
) -> Result<Vec<PathBuf>, String> {
    if !bitacoras_originales.is_file() {
        return Err("Falta Bitacoras_del_mes.xlsx original: no se regenera perdiendo bitácoras.".into());
    }
    publicar_interno(m, destino, Some((bitacoras_originales, huella_registros)))
}

fn publicar_interno(m: &Meta4, destino: &Path, regenerar: Option<(&Path, &str)>) -> Result<Vec<PathBuf>, String> {
    crate::validar_destino(destino)?;
    let padre = destino.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    std::fs::create_dir_all(padre).map_err(|e| format!("No se pudo crear {}: {e}", padre.display()))?;
    let nombre = destino.file_name().ok_or("Carpeta de salida inválida.")?.to_string_lossy();
    let temporal = padre.join(format!(".en-curso-{nombre}"));
    let _ = std::fs::remove_dir_all(&temporal);
    std::fs::create_dir(&temporal).map_err(|e| format!("No se pudo preparar la salida: {e}"))?;
    let parcial = if m.tribunales.is_some() { " - parcial" } else { "" };
    let nombres = [
        format!("META 4 - actualizado {}{parcial}.xlsx", m.corte.format("%Y-%m")),
        "Registros.xlsx".to_string(),
        "Registros consolidados.xlsx".to_string(),
        "Bitacoras_del_mes.xlsx".to_string(),
        "Reporte META4.json".to_string(),
    ];
    let escribir = || -> Result<(), String> {
        let totales = planilla(m, &temporal.join(&nombres[0]))?;
        // Verificación final: lo escrito en la planilla suma lo mismo que los registros clasificados.
        let esperados = m.registros.iter().filter(|r| r.categoria.is_some() && r.tribunal.is_some()).count();
        if totales.values().sum::<usize>() != esperados {
            return Err("La planilla no contiene todos los registros clasificados; no se publicó nada.".into());
        }
        registros(m, &temporal.join(&nombres[1]))?;
        consolidado(m, &temporal.join(&nombres[2]))?;
        if let Some((original, _)) = regenerar {
            // No se reinterpreta el texto íntegro de bitácoras: se copia el libro comprobado.
            std::fs::copy(original, temporal.join(&nombres[3]))
                .map_err(|e| format!("No se pudo conservar Bitacoras_del_mes.xlsx: {e}"))?;
        } else {
            bitacoras_del_mes(m, &temporal.join(&nombres[3]))?;
        }
        let mut informe = reporte(m, &totales);
        if let Some((_, huella)) = regenerar {
            informe["regenerado_desde"] = json!("Registros.xlsx revisado");
            informe["sha256_registros_revisados"] = json!(huella);
        }
        let texto = serde_json::to_string_pretty(&informe).map_err(|e| e.to_string())?;
        std::fs::write(temporal.join(&nombres[4]), texto).map_err(|e| format!("No se pudo guardar el reporte: {e}"))
    };
    if let Err(e) = escribir() {
        let _ = std::fs::remove_dir_all(&temporal);
        return Err(e);
    }
    std::fs::rename(&temporal, destino).map_err(|e| format!("No se pudo publicar la salida: {e}"))?;
    Ok(nombres.iter().map(|n| destino.join(n)).collect())
}

#[allow(dead_code)]
fn _sin_tribunal() -> &'static str {
    SIN_TRIBUNAL
}
