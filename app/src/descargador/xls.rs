//! Lectura y cotejo de los XLS descargados (`motor.read_excel`, `verified_download` y `verify_month` de Flow Python).

use super::sitfa::registros::{columnas, contar, normalizar, registro};
use super::sitfa::{decodificar, Consulta, Pagina, Registros, LIMITE_BYTES};
use calamine::{Data, Reader};
use chrono::{Datelike, NaiveDate};
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::sync::LazyLock;

const XML_2003: &[u8] = b"urn:schemas-microsoft-com:office:spreadsheet";

type Grilla = Vec<Vec<String>>;
type FilasCompletas = BTreeMap<(Vec<String>, Vec<String>), usize>;

static TABLAS: LazyLock<Selector> = LazyLock::new(|| Selector::parse("table").unwrap());
static ACTIVOS: LazyLock<Selector> = LazyLock::new(|| Selector::parse("form, script").unwrap());
static ACCESO: LazyLock<Selector> = LazyLock::new(|| Selector::parse("input").unwrap());

pub struct Libro {
    pub formato: &'static str,
    pub registros: Registros,
    grilla: Grilla,
}

pub fn huella(datos: &[u8]) -> String {
    format!("{:x}", Sha256::digest(datos))
}

fn fecha_serial(serial: f64) -> Option<NaiveDate> {
    (1.0..2_958_466.0).contains(&serial)
        .then(|| NaiveDate::from_ymd_opt(1899, 12, 30))
        .flatten()
        .map(|base| base + chrono::Duration::days(serial.floor() as i64))
}

fn texto_celda(dato: &Data) -> String {
    match dato {
        Data::Empty | Data::Error(_) => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", *f as i64),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "True" } else { "False" }.into(),
        Data::DateTime(d) => fecha_serial(d.as_f64())
            .map(|f| f.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
    }
}

fn grillas_calamine(datos: &[u8]) -> Result<Vec<Grilla>, String> {
    let mut libro = calamine::open_workbook_auto_from_rs(Cursor::new(datos.to_vec()))
        .map_err(|_| "No se pudo abrir/parsear el archivo Excel; requiere revision local.")?;
    let mut celdas = 0usize;
    let mut grillas = Vec::new();
    for (_, rango) in libro.worksheets() {
        let (filas, cols) = rango.get_size();
        celdas += filas * cols;
        if filas > 200_000 || cols > 200 || celdas > 2_000_000 {
            return Err("Las dimensiones del Excel exceden los límites de lectura.".into());
        }
        grillas.push(rango.rows().map(|f| f.iter().map(texto_celda).collect()).collect());
    }
    Ok(grillas)
}

fn grillas_html(datos: &[u8]) -> Result<Vec<Grilla>, String> {
    let raiz = Html::parse_document(&decodificar(datos));
    if raiz.select(&ACCESO).any(|e| {
        e.value()
            .attr("type")
            .is_some_and(|t| t.eq_ignore_ascii_case("password"))
    }) {
        return Err("Sesion expirada: la descarga contiene un formulario de acceso.".into());
    }
    if raiz.select(&ACTIVOS).next().is_some() {
        return Err("La descarga es una pagina web activa, no un Excel verificable.".into());
    }
    raiz.select(&TABLAS)
        .map(super::sitfa::registros::grilla)
        .collect()
}

/// Formato y listado único de causas y nombres. Una descarga sin listado no es una consulta vacía.
pub fn leer(datos: &[u8]) -> Result<Libro, String> {
    if datos.is_empty() {
        return Err("El XLS esta vacio.".into());
    }
    if datos.len() > LIMITE_BYTES {
        return Err("El Excel supera 80 MB.".into());
    }
    let (formato, grillas) = if datos.starts_with(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]) {
        ("XLS-BIFF", grillas_calamine(datos)?)
    } else if datos.starts_with(b"PK\x03\x04") {
        ("OOXML-con-extension-xls", grillas_calamine(datos)?)
    } else if datos[..datos.len().min(4096)]
        .windows(XML_2003.len())
        .any(|w| w == XML_2003)
    {
        return Err("El Excel XML 2003 todavía no se verifica en Flow Rust; descarga detenida.".into());
    } else {
        ("HTML-tabular-con-extension-xls", grillas_html(datos)?)
    };
    let mut candidatas = Vec::new();
    for grilla in grillas {
        let registros = contar(&grilla)?;
        if !registros.is_empty() {
            candidatas.push((registros, grilla));
        }
    }
    if candidatas.len() != 1 {
        return Err("Se recibio el Excel, pero no se reconoce un unico listado de causas y nombres. No es una consulta sin resultados.".into());
    }
    let (registros, grilla) = candidatas.pop().unwrap();
    Ok(Libro {
        formato,
        registros,
        grilla,
    })
}

/// Filas completas: un RIT y nombre pueden tener varias medidas.
fn filas_completas(libro: &Libro) -> Result<FilasCompletas, String> {
    let mut filas = FilasCompletas::new();
    let (mut cols, mut cabecera) = (None, Vec::new());
    for fila in &libro.grilla {
        if let Some(c) = columnas(fila)? {
            cols = Some(c);
            cabecera = fila.iter().map(|s| normalizar(s)).collect();
        } else if cols.as_ref().and_then(|c| registro(fila, c)).is_some() {
            *filas
                .entry((cabecera.clone(), fila.iter().map(|s| normalizar(s)).collect()))
                .or_insert(0) += 1;
        }
    }
    Ok(filas)
}

fn fecha_texto(texto: &str) -> Option<NaiveDate> {
    let t = texto.trim();
    ["%d/%m/%Y", "%Y-%m-%d", "%d-%m-%Y"]
        .iter()
        .find_map(|f| NaiveDate::parse_from_str(t, f).ok())
}

/// Una ruta temporal con las mismas personas no acredita el mes consultado.
fn verificar_mes(libro: &Libro, consulta: &Consulta) -> Result<(), String> {
    let mut alias = vec![
        "FECHA VENCIMIENTO",
        "FEC.VENCIMIENTO",
        "FEC. VENCIMIENTO",
        "F. VENCIMIENTO",
        "F.VENCIMIENTO",
        "F.VENC",
        "F. VENC",
        "VENCIMIENTO",
    ];
    if consulta.pantalla == super::Pantalla::CalendarioMedidas {
        alias.extend([
            "FEC.EGRESO PROYECTADO",
            "FEC. EGRESO PROYECTADO",
            "FEC EGRESO PROYECTADO",
            "FECHA EGRESO PROYECTADO",
        ]);
    }
    let mes = &consulta.objetivo["COD_Mes_sel"];
    let anio = &consulta.objetivo["COD_Anio_Sel"];
    let (mut cols, mut posicion, mut comprobadas) = (None, 0, 0usize);
    for (numero, fila) in libro.grilla.iter().enumerate() {
        if let Some(c) = columnas(fila)? {
            let fechas: Vec<_> = fila
                .iter()
                .enumerate()
                .filter(|(_, s)| alias.contains(&normalizar(s).as_str()))
                .collect();
            if fechas.len() != 1 {
                return Err("El calendario XLS no permite comprobar sus fechas.".into());
            }
            cols = Some(c);
            posicion = fechas[0].0;
        } else if cols.as_ref().and_then(|c| registro(fila, c)).is_some() {
            let dia = fila.get(posicion).and_then(|s| fecha_texto(s)).ok_or_else(|| {
                format!("La fecha del calendario XLS en la fila {} está vacía o no se reconoce; no se pudo comprobar el período.", numero + 1)
            })?;
            if dia.month().to_string() != *mes || dia.year().to_string() != *anio {
                return Err(format!(
                    "La fecha {} del calendario XLS (fila {}) no corresponde al mes {mes}/{anio} consultado; puede ser una descarga anterior.",
                    dia.format("%d/%m/%Y"),
                    numero + 1
                ));
            }
            comprobadas += 1;
        }
    }
    if comprobadas != libro.registros.values().sum::<usize>() {
        return Err("No se pudieron comprobar todas las fechas del calendario XLS.".into());
    }
    Ok(())
}

/// Estado de cotejo de una consulta: huellas y filas de las páginas ya guardadas, solo en memoria.
#[derive(Default)]
pub struct Verificador {
    huellas: BTreeSet<String>,
    filas: Vec<FilasCompletas>,
}

impl Verificador {
    pub fn verificar(&self, datos: &[u8], pagina: &Pagina, consulta: &Consulta) -> Result<(Libro, String), String> {
        let libro = leer(datos)?;
        if libro.registros != pagina.registros {
            return Err("Los registros XLS no coinciden con la pagina HTML (puede ser un XLS anterior o de toda la consulta).".into());
        }
        if consulta.calendario() {
            verificar_mes(&libro, consulta)?;
        }
        let digest = huella(datos);
        if self.huellas.contains(&digest) || self.filas.contains(&filas_completas(&libro)?) {
            return Err("Dos paginas contienen el mismo archivo o los mismos registros; descarga detenida.".into());
        }
        Ok((libro, digest))
    }

    pub fn registrar(&mut self, libro: &Libro, digest: String) -> Result<(), String> {
        self.huellas.insert(digest);
        self.filas.push(filas_completas(libro)?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_tabular_con_listado_unico_y_rechazo_de_paginas_activas() {
        let datos = b"<html><table><tr><td>RIT</td><td>Nombre</td></tr><tr><td>X-1-2026</td><td>Persona Ficticia</td></tr></table></html>";
        let libro = leer(datos).unwrap();
        assert_eq!(libro.formato, "HTML-tabular-con-extension-xls");
        assert_eq!(libro.registros.values().sum::<usize>(), 1);
        assert!(leer(b"<html><form></form><table></table></html>").is_err());
        assert!(leer(b"<html><table><tr><td>Sin datos</td></tr></table></html>").is_err());
        assert_eq!(fecha_texto("5/10/2026"), NaiveDate::from_ymd_opt(2026, 10, 5));
        assert_eq!(fecha_serial(46300.0), NaiveDate::from_ymd_opt(2026, 10, 5));
    }
}
