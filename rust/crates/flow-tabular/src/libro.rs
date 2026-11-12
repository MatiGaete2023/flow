//! Lectura de un libro por su contenido, no por su extensión: SITFA guarda tablas HTML y XML con extensión `.xls`.

use crate::html;
use calamine::{Data, Reader};
use chrono::NaiveDate;
use flow_dominio::Celda;
use quick_xml::events::Event;
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::Path;

pub const LIMITE_BYTES: usize = 80 * 1024 * 1024;
const LIMITE_CELDAS: usize = 2_000_000;
const LIMITE_COLUMNAS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formato {
    XlsBiff,
    Xlsx,
    Xml2003,
    Html,
}

impl Formato {
    pub fn nombre(self) -> &'static str {
        match self {
            Formato::XlsBiff => "Excel 97-2003 (BIFF)",
            Formato::Xlsx => "Excel (OOXML)",
            Formato::Xml2003 => "XML Spreadsheet 2003",
            Formato::Html => "tabla HTML",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hoja {
    pub nombre: String,
    pub filas: Vec<Vec<Celda>>,
}

#[derive(Debug, Clone)]
pub struct Libro {
    pub nombre: String,
    pub sha256: String,
    pub formato: Formato,
    pub hojas: Vec<Hoja>,
}

pub fn detectar(datos: &[u8]) -> Option<Formato> {
    let inicio = &datos[..datos.len().min(4096)];
    let bajo = String::from_utf8_lossy(inicio).to_lowercase();
    if datos.starts_with(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]) {
        Some(Formato::XlsBiff)
    } else if datos.starts_with(b"PK\x03\x04") {
        Some(Formato::Xlsx)
    } else if bajo.contains("urn:schemas-microsoft-com:office:spreadsheet") {
        Some(Formato::Xml2003)
    } else if bajo.contains("<html") || bajo.contains("<table") {
        Some(Formato::Html)
    } else {
        None
    }
}

fn celda_de(dato: &Data) -> Celda {
    match dato {
        Data::Empty | Data::Error(_) => Celda::Vacia,
        Data::String(s) => Celda::Texto(s.clone()),
        Data::Float(f) => Celda::Numero(*f),
        Data::Int(i) => Celda::Numero(*i as f64),
        Data::Bool(b) => Celda::Texto(if *b { "VERDADERO" } else { "FALSO" }.into()),
        Data::DateTime(d) => match d.as_datetime() {
            Some(dt) if !d.is_duration() => Celda::Fecha(dt.date()),
            _ => Celda::Numero(d.as_f64()),
        },
        Data::DateTimeIso(s) | Data::DurationIso(s) => {
            flow_dominio::celda::fecha_de_texto(s).map(Celda::Fecha).unwrap_or_else(|| Celda::Texto(s.clone()))
        }
    }
}

fn hojas_calamine<R: Reader<Cursor<Vec<u8>>>>(mut libro: R) -> Result<Vec<Hoja>, String>
where
    R::Error: std::fmt::Display,
{
    let mut hojas = Vec::new();
    let mut celdas = 0usize;
    for nombre in libro.sheet_names() {
        let rango = libro.worksheet_range(&nombre).map_err(|e| format!("No se pudo leer la hoja «{nombre}»: {e}"))?;
        let mut filas = Vec::new();
        if let Some((fila0, col0)) = rango.start() {
            // El rango empieza en la primera celda usada: se rellena para conservar el número real de fila y columna.
            filas.resize(fila0 as usize, Vec::new());
            for fila in rango.rows() {
                let mut valores = vec![Celda::Vacia; col0 as usize];
                valores.extend(fila.iter().map(celda_de));
                celdas += valores.len();
                if valores.len() > LIMITE_COLUMNAS * 4 || celdas > LIMITE_CELDAS {
                    return Err("El libro excede los límites de lectura.".into());
                }
                filas.push(valores);
            }
        }
        hojas.push(Hoja { nombre, filas });
    }
    Ok(hojas)
}

fn atributo(e: &quick_xml::events::BytesStart, nombre: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        let clave = a.key.as_ref();
        let local = clave.rsplit(|b: &u8| *b == b':').next().unwrap_or(clave);
        (local == nombre.as_bytes()).then(|| String::from_utf8_lossy(&a.value).into_owned())
    })
}

fn hojas_xml2003(datos: &[u8]) -> Result<Vec<Hoja>, String> {
    let texto = html::decodificar(datos);
    if texto.contains("<!DOCTYPE") || texto.contains("<!ENTITY") {
        return Err("El XML declara DOCTYPE o entidades: no se lee.".into());
    }
    let mut lector = quick_xml::Reader::from_str(&texto);
    let mut hojas: Vec<Hoja> = Vec::new();
    let mut fila: Option<Vec<Celda>> = None;
    let mut tipo = String::new();
    let mut valor: Option<String> = None;
    let mut celdas = 0usize;
    let local = |n: &[u8]| String::from_utf8_lossy(n.rsplit(|b| *b == b':').next().unwrap_or(n)).into_owned();
    loop {
        match lector.read_event().map_err(|e| format!("XML ilegible: {e}"))? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => match local(e.name().as_ref()).as_str() {
                "Worksheet" => {
                    let nombre = atributo(&e, "Name").unwrap_or_else(|| format!("Hoja{}", hojas.len() + 1));
                    hojas.push(Hoja { nombre, filas: Vec::new() });
                }
                "Row" => {
                    if hojas.is_empty() {
                        hojas.push(Hoja { nombre: "Hoja1".into(), filas: Vec::new() });
                    }
                    let hoja = hojas.last_mut().unwrap();
                    if let Some(i) = atributo(&e, "Index").and_then(|i| i.parse::<usize>().ok()) {
                        if i <= hoja.filas.len() || i > 200_000 {
                            return Err("Índice de fila XML inválido.".into());
                        }
                        hoja.filas.resize(i - 1, Vec::new());
                    }
                    fila = Some(Vec::new());
                }
                "Cell" => {
                    if let Some(f) = fila.as_mut() {
                        if let Some(i) = atributo(&e, "Index").and_then(|i| i.parse::<usize>().ok()) {
                            if i == 0 || i > LIMITE_COLUMNAS {
                                return Err("El índice XML excede los límites de columnas.".into());
                            }
                            if i - 1 > f.len() {
                                f.resize(i - 1, Celda::Vacia);
                            }
                        }
                        f.push(Celda::Vacia);
                    }
                }
                "Data" => {
                    tipo = atributo(&e, "Type").unwrap_or_default();
                    valor = Some(String::new());
                }
                _ => {}
            },
            Event::Text(t) => {
                if let Some(v) = valor.as_mut() {
                    v.push_str(&html::sin_entidades(&String::from_utf8_lossy(&t)));
                }
            }
            Event::CData(t) => {
                if let Some(v) = valor.as_mut() {
                    v.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Event::End(e) => match local(e.name().as_ref()).as_str() {
                "Data" => {
                    if let (Some(v), Some(f)) = (valor.take(), fila.as_mut()) {
                        let celda = match tipo.as_str() {
                            "Number" => v.trim().parse().map(Celda::Numero).unwrap_or(Celda::Texto(v)),
                            "DateTime" => flow_dominio::celda::fecha_de_texto(v.trim_end_matches('Z'))
                                .map(Celda::Fecha)
                                .unwrap_or(Celda::Texto(v)),
                            _ if v.is_empty() => Celda::Vacia,
                            _ => Celda::Texto(v),
                        };
                        if let Some(ultima) = f.last_mut() {
                            *ultima = celda;
                        }
                    }
                }
                "Row" => {
                    if let (Some(f), Some(hoja)) = (fila.take(), hojas.last_mut()) {
                        celdas += f.len();
                        if f.len() > LIMITE_COLUMNAS || celdas > LIMITE_CELDAS {
                            return Err("La tabla XML excede los límites.".into());
                        }
                        hoja.filas.push(f);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(hojas)
}

/// Lee un libro desde sus bytes. `nombre` es solo informativo.
pub fn leer_bytes(nombre: &str, datos: Vec<u8>) -> Result<Libro, String> {
    if datos.is_empty() {
        return Err("El archivo está vacío.".into());
    }
    if datos.len() > LIMITE_BYTES {
        return Err("El archivo supera 80 MB.".into());
    }
    let sha256 = Sha256::digest(&datos).iter().map(|b| format!("{b:02x}")).collect();
    let formato = detectar(&datos).ok_or("El archivo no es un Excel ni una tabla reconocible.")?;
    let hojas = match formato {
        Formato::XlsBiff => {
            let libro = calamine::Xls::new(Cursor::new(datos)).map_err(|e| format!("No se pudo abrir el libro: {e}"))?;
            hojas_calamine(libro)?
        }
        Formato::Xlsx => {
            let libro: calamine::Xlsx<_> =
                calamine::Xlsx::new(Cursor::new(datos)).map_err(|e| format!("No se pudo abrir el libro: {e}"))?;
            hojas_calamine(libro)?
        }
        Formato::Xml2003 => hojas_xml2003(&datos)?,
        Formato::Html => html::tablas(&datos)?
            .into_iter()
            .enumerate()
            .map(|(i, filas)| Hoja { nombre: format!("Tabla{}", i + 1), filas })
            .collect(),
    };
    if hojas.is_empty() {
        return Err("El libro no contiene hojas.".into());
    }
    Ok(Libro { nombre: nombre.to_string(), sha256, formato, hojas })
}

pub fn leer(ruta: &Path) -> Result<Libro, String> {
    let datos = std::fs::read(ruta).map_err(|e| format!("No se pudo leer {}: {e}", ruta.display()))?;
    let nombre = ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    leer_bytes(&nombre, datos)
}

/// Fecha de corte sugerida por el nombre del archivo no se usa: el corte lo decide el usuario.
pub fn hoy() -> NaiveDate {
    chrono::Local::now().date_naive()
}
