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

#[derive(Debug)]
pub struct Libro {
    pub formato: &'static str,
    pub registros: Registros,
    grilla: Grilla,
}

impl Libro {
    /// La grilla completa del listado reconocido (`ExcelInfo.grid`).
    pub fn grilla(&self) -> &[Vec<String>] {
        &self.grilla
    }
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

/// `carga.canonical`: las fechas de Excel se comparan como `dd/mm/aaaa`.
fn texto_celda_con(dato: &Data, formato_fecha: &str) -> String {
    match dato {
        Data::Empty | Data::Error(_) => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", *f as i64),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "True" } else { "False" }.into(),
        Data::DateTime(d) => fecha_serial(d.as_f64())
            .map(|f| f.format(formato_fecha).to_string())
            .unwrap_or_default(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
    }
}

fn grillas_calamine_con(datos: &[u8], formato_fecha: &str) -> Result<Vec<Grilla>, String> {
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
        grillas.push(rango.rows().map(|f| f.iter().map(|d| texto_celda_con(d, formato_fecha)).collect()).collect());
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

/// `read_excel` para SpreadsheetML 2003: una grilla por `ss:Table`, con `ss:Index` de filas y celdas, el texto de
/// `ss:Data` (incluido el de su marcado interno) y las fechas `DateTime` como `AAAA-MM-DD`. Sin DTD ni entidades
/// externas; los mismos límites de filas y columnas que Flow Python.
fn grillas_xml_2003(datos: &[u8]) -> Result<Vec<Grilla>, String> {
    use quick_xml::events::Event;
    let texto = match std::str::from_utf8(datos) {
        Ok(t) => std::borrow::Cow::Borrowed(t),
        Err(_) => std::borrow::Cow::Owned(decodificar(datos)),
    };
    let mut lector = quick_xml::Reader::from_str(&texto);
    let invalido = |_| "No se pudo abrir/parsear el archivo Excel; requiere revision local.".to_string();
    let indice = |e: &quick_xml::events::BytesStart| -> Result<Option<usize>, String> {
        for a in e.attributes() {
            let a = a.map_err(|_| "No se pudo abrir/parsear el archivo Excel; requiere revision local.".to_string())?;
            if a.key.as_ref() == "ss:Index" {
                return a.value.trim().parse().map(Some).map_err(|_| "Índice de fila XML inválido.".to_string());
            }
        }
        Ok(None)
    };
    let es_fecha = |e: &quick_xml::events::BytesStart| {
        e.attributes().flatten().any(|a| a.key.as_ref() == "ss:Type" && a.value.as_ref() == "DateTime")
    };
    let (mut grillas, mut grilla, mut fila, mut celda): (Vec<Grilla>, Option<Grilla>, Option<Vec<String>>, Option<String>) =
        (Vec::new(), None, None, None);
    let mut dentro_dato = 0usize;
    let mut fecha = false;
    loop {
        let evento = lector.read_event().map_err(invalido)?;
        match &evento {
            Event::Start(e) | Event::Empty(e) => {
                let vacio = matches!(evento, Event::Empty(_));
                match e.local_name().as_ref() {
                    "Table" if !vacio => grilla = Some(Vec::new()),
                    "Table" => grillas.push(Vec::new()),
                    "Row" if grilla.is_some() => {
                        let g = grilla.as_mut().unwrap();
                        if let Some(objetivo) = indice(e)? {
                            if !(g.len() < objetivo && objetivo <= 200_000) {
                                return Err("Índice de fila XML inválido.".into());
                            }
                            g.resize(objetivo - 1, Vec::new());
                        }
                        if vacio {
                            g.push(Vec::new());
                        } else {
                            fila = Some(Vec::new());
                        }
                    }
                    "Cell" if fila.is_some() => {
                        let f = fila.as_mut().unwrap();
                        if let Some(i) = indice(e)? {
                            if !(1..=200).contains(&i) {
                                return Err("El índice XML excede los límites de columnas.".into());
                            }
                            if f.len() < i - 1 {
                                f.resize(i - 1, String::new());
                            }
                        }
                        if vacio {
                            f.push(String::new());
                        } else {
                            celda = Some(String::new());
                            fecha = false;
                        }
                    }
                    "Data" if celda.is_some() && !vacio => {
                        dentro_dato += 1;
                        if dentro_dato == 1 {
                            fecha = es_fecha(e);
                        }
                    }
                    _ if dentro_dato > 0 && !vacio => dentro_dato += 1,
                    _ => {}
                }
            }
            Event::Text(t) if dentro_dato > 0 => celda.as_mut().unwrap().push_str(&t.xml10_content()),
            Event::CData(t) if dentro_dato > 0 => celda.as_mut().unwrap().push_str(&t.xml10_content()),
            Event::GeneralRef(r) if dentro_dato > 0 => {
                let c = match r.resolve_char_ref().map_err(|_| "No se pudo abrir/parsear el archivo Excel; requiere revision local.".to_string())? {
                    Some(c) => c,
                    None => match &**r {
                        "amp" => '&',
                        "lt" => '<',
                        "gt" => '>',
                        "quot" => '"',
                        "apos" => '\'',
                        _ => return Err("No se pudo abrir/parsear el archivo Excel; requiere revision local.".into()),
                    },
                };
                celda.as_mut().unwrap().push(c);
            }
            Event::End(e) => match e.local_name().as_ref() {
                "Data" if dentro_dato == 1 => dentro_dato = 0,
                _ if dentro_dato > 1 => dentro_dato -= 1,
                "Cell" if celda.is_some() => {
                    let mut valor = celda.take().unwrap();
                    if fecha {
                        let iso = valor.trim_end_matches('Z');
                        if let Some(d) = iso.get(..10).and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()) {
                            if iso.len() == 10 || iso.as_bytes().get(10) == Some(&b'T') {
                                valor = d.format("%Y-%m-%d").to_string();
                            }
                        }
                    }
                    let f = fila.as_mut().unwrap();
                    f.push(valor);
                    if f.len() > 200 {
                        return Err("La fila XML excede los límites de columnas.".into());
                    }
                }
                "Row" if fila.is_some() => {
                    let g = grilla.as_mut().unwrap();
                    g.push(fila.take().unwrap());
                    if g.len() > 200_000 {
                        return Err("La tabla XML excede los límites de filas.".into());
                    }
                }
                "Table" if grilla.is_some() => grillas.push(grilla.take().unwrap()),
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(grillas)
}

/// Formato y grillas del XLS, con el mismo despacho que `read_excel`.
fn grillas(datos: &[u8], formato_fecha: &str) -> Result<(&'static str, Vec<Grilla>), String> {
    if datos.is_empty() {
        return Err("El XLS esta vacio.".into());
    }
    if datos.len() > LIMITE_BYTES {
        return Err("El Excel supera 80 MB.".into());
    }
    Ok(if datos.starts_with(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]) {
        ("XLS-BIFF", grillas_calamine_con(datos, formato_fecha)?)
    } else if datos.starts_with(b"PK\x03\x04") {
        ("OOXML-con-extension-xls", grillas_calamine_con(datos, formato_fecha)?)
    } else if datos[..datos.len().min(4096)].windows(XML_2003.len()).any(|w| w == XML_2003) {
        ("XML-Spreadsheet-2003", grillas_xml_2003(datos)?)
    } else {
        ("HTML-tabular-con-extension-xls", grillas_html(datos)?)
    })
}

/// `read_excel(data, record_reader=carga.records_in_grid)`: formato y filas del único listado de Carga.
pub fn leer_carga(datos: &[u8]) -> Result<(&'static str, super::sitfa::carga::FilasCarga), String> {
    let (formato, grillas) = grillas(datos, "%d/%m/%Y")?;
    let mut candidatas = Vec::new();
    for grilla in &grillas {
        let filas = super::sitfa::carga::registros_en_grilla(grilla)?;
        if !filas.is_empty() {
            candidatas.push(filas);
        }
    }
    if candidatas.len() != 1 {
        return Err("Se recibio el Excel, pero no se reconoce un unico listado de causas y nombres. No es una consulta sin resultados.".into());
    }
    Ok((formato, candidatas.pop().unwrap()))
}

/// `read_excel(data, carga.records_in_grid).grid` para el paquete CSMP: la grilla del único listado de Carga dos veces,
/// con las fechas de Excel como `dd/mm/aaaa` (lo que compara `carga.canonical`) y como `datetime.isoformat()` (lo que
/// escribe `literal`).
pub fn grilla_carga(datos: &[u8]) -> Result<(Grilla, Grilla), String> {
    let (_, canonicas) = grillas(datos, "%d/%m/%Y")?;
    let mut elegidas = Vec::new();
    for (i, grilla) in canonicas.iter().enumerate() {
        if !super::sitfa::carga::registros_en_grilla(grilla)?.is_empty() {
            elegidas.push(i);
        }
    }
    if elegidas.len() != 1 {
        return Err("Se recibio el Excel, pero no se reconoce un unico listado de causas y nombres. No es una consulta sin resultados.".into());
    }
    let (_, mut literales) = grillas(datos, "%Y-%m-%dT00:00:00")?;
    let i = elegidas[0];
    Ok((canonicas.into_iter().nth(i).unwrap(), literales.swap_remove(i)))
}

/// Formato y listado único de causas y nombres. Una descarga sin listado no es una consulta vacía.
pub fn leer(datos: &[u8]) -> Result<Libro, String> {
    let (formato, grillas) = grillas(datos, "%Y-%m-%d")?;
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
        if consulta.pantalla == super::Pantalla::Carga {
            // `verified_download` con `carga.records_in_grid`: las filas completas del XLS deben ser las de la tabla web.
            let (formato, filas) = leer_carga(datos)?;
            if filas != pagina.carga {
                return Err("Los registros XLS no coinciden con la pagina HTML (puede ser un XLS anterior o de toda la consulta).".into());
            }
            let digest = huella(datos);
            if self.huellas.contains(&digest) {
                return Err("Dos paginas contienen el mismo archivo o los mismos registros; descarga detenida.".into());
            }
            return Ok((Libro { formato, registros: Registros::new(), grilla: Grilla::new() }, digest));
        }
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

    #[test]
    fn a7_excel_xml_2003() {
        // 1. Libro SpreadsheetML ficticio con ss:Index, html:Font y DateTime
        let xml_valido = r##"<?xml version="1.0"?>
<?mso-application progid="Excel.Sheet"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:html="http://www.w3.org/TR/REC-html40">
 <Worksheet ss:Name="Hoja1">
  <Table>
   <Row ss:Index="1">
    <Cell ss:Index="1"><Data ss:Type="String">RIT</Data></Cell>
    <Cell ss:Index="2"><Data ss:Type="String">Nombre</Data></Cell>
    <Cell ss:Index="4"><Data ss:Type="String">Fecha</Data></Cell>
   </Row>
   <Row ss:Index="3">
    <Cell ss:Index="1"><Data ss:Type="String">X-100-2099</Data></Cell>
    <Cell ss:Index="2"><Data ss:Type="String"><Font html:Color="#000000">Persona Ficticia</Font></Data></Cell>
    <Cell ss:Index="4"><Data ss:Type="DateTime">2026-10-15T00:00:00.000</Data></Cell>
   </Row>
  </Table>
 </Worksheet>
</Workbook>"##;

        let libro = leer(xml_valido.as_bytes()).unwrap();
        assert_eq!(libro.formato, "XML-Spreadsheet-2003");
        assert_eq!(libro.registros.len(), 1);
        assert_eq!(libro.registros[&("X-100-2099".into(), "PERSONA FICTICIA".into())], 1);
        assert_eq!(libro.grilla.len(), 3);
        assert_eq!(libro.grilla[2][0], "X-100-2099");
        assert_eq!(libro.grilla[2][1], "Persona Ficticia");
        assert_eq!(libro.grilla[2][3], "2026-10-15");

        // 2. Error de índice de fila no creciente
        let xml_no_creciente = r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet" xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet">
 <Worksheet ss:Name="Hoja1">
  <Table>
   <Row ss:Index="5"><Cell><Data ss:Type="String">A</Data></Cell></Row>
   <Row ss:Index="3"><Cell><Data ss:Type="String">B</Data></Cell></Row>
  </Table>
 </Worksheet>
</Workbook>"#;
        assert_eq!(
            leer(xml_no_creciente.as_bytes()).unwrap_err(),
            "Índice de fila XML inválido."
        );

        // 3. Error de columna 201
        let xml_col_201 = r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet" xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet">
 <Worksheet ss:Name="Hoja1">
  <Table>
   <Row><Cell ss:Index="201"><Data ss:Type="String">X</Data></Cell></Row>
  </Table>
 </Worksheet>
</Workbook>"#;
        assert_eq!(
            leer(xml_col_201.as_bytes()).unwrap_err(),
            "El índice XML excede los límites de columnas."
        );

        // 4. Error de más de 200 celdas por fila
        let mut celdas = String::new();
        for _ in 0..201 {
            celdas.push_str("<Cell><Data ss:Type=\"String\">C</Data></Cell>");
        }
        let xml_201_celdas = format!(
            r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet" xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet">
 <Worksheet ss:Name="Hoja1">
  <Table>
   <Row>{}</Row>
  </Table>
 </Worksheet>
</Workbook>"#,
            celdas
        );
        assert_eq!(
            leer(xml_201_celdas.as_bytes()).unwrap_err(),
            "La fila XML excede los límites de columnas."
        );

        // 5. <!DOCTYPE> con entidad externa no debe resolverse
        let xml_xxe = r#"<?xml version="1.0"?>
<!DOCTYPE test [
<!ENTITY xxe SYSTEM "http://127.0.0.1:9999/xxe">
]>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet" xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet">
 <Worksheet ss:Name="Hoja1">
  <Table>
   <Row><Cell><Data ss:Type="String">&xxe;</Data></Cell></Row>
  </Table>
 </Worksheet>
</Workbook>"#;
        assert!(leer(xml_xxe.as_bytes()).is_err());
    }
    /// E-A78: el mismo resultado que Flow Python (`motor.read_excel`, `carga.records_in_grid`) sobre los bytes y
    /// grillas sintéticas de `app/tests/datos/a78.json`, regenerable con `python app/qa/generar_a78.py`.
    #[test]
    fn a78_igual_a_python_sobre_bytes_fijos() {
        use base64::Engine;
        use serde_json::{json, Value};
        let fixture: Value = serde_json::from_str(include_str!("../../tests/datos/a78.json")).unwrap();
        let b64 = |c: &Value| base64::engine::general_purpose::STANDARD.decode(c["bytes_b64"].as_str().unwrap()).unwrap();
        let mut fallos = Vec::new();
        let mut aceptadas: Vec<String> = Vec::new();
        let mut cuenta = [0usize; 3];

        for c in fixture["a7"].as_array().unwrap() {
            let obtenido = match leer(&b64(c)) {
                Ok(libro) => {
                    let mut registros: Vec<Value> = libro
                        .registros
                        .iter()
                        .map(|((rit, nombre), n)| json!([rit, nombre, n]))
                        .collect();
                    registros.sort_by_key(|v| v.to_string());
                    json!({"formato": libro.formato, "registros": registros, "grilla": libro.grilla})
                }
                Err(e) => json!({"error": e}),
            };
            let mut esperado = c["python"].clone();
            if let Some(r) = esperado.get_mut("registros").and_then(Value::as_array_mut) {
                r.sort_by_key(|v| v.to_string());
            }
            if obtenido == esperado {
                cuenta[0] += 1;
            } else if c["caso"] == "entidad_externa"
                && obtenido == json!({"error": "No se pudo abrir/parsear el archivo Excel; requiere revision local."})
            {
                // D-3, opción a (usuario, 23-nov-2026): se conserva el mensaje de Rust. Ambos rechazan sin resolver la
                // entidad; Python lo informa como listado irreconocible y Rust como XML ilegible. Cualquier otro cambio
                // en este caso es FALLA.
                aceptadas.push(format!("D-3 A7 entidad_externa: Python {:?} / Rust {:?}", esperado["error"], obtenido["error"]));
            } else {
                fallos.push(format!("A7 {}: esp {esperado} / obt {obtenido}", c["caso"]));
            }
        }

        let filas_json = |filas: &super::super::sitfa::carga::FilasCarga| -> Vec<Value> {
            let mut v: Vec<Value> = filas
                .iter()
                .map(|(fila, n)| {
                    let mut f: Vec<Value> = fila.iter().map(|s| json!(s)).collect();
                    f.push(json!(n));
                    Value::Array(f)
                })
                .collect();
            v.sort_by_key(|x| x.to_string());
            v
        };
        let ordenar = |mut v: Value| {
            if let Some(f) = v.get_mut("filas").and_then(Value::as_array_mut) {
                f.sort_by_key(|x| x.to_string());
            }
            v
        };
        for c in fixture["a8"].as_array().unwrap() {
            let grilla: Vec<Vec<String>> = serde_json::from_value(c["grilla"].clone()).unwrap();
            let obtenido = match super::super::sitfa::carga::registros_en_grilla(&grilla) {
                Ok(filas) => json!({"filas": filas_json(&filas)}),
                Err(e) => json!({"error": e}),
            };
            let esperado = ordenar(c["python"].clone());
            if obtenido == esperado {
                cuenta[1] += 1;
            } else {
                fallos.push(format!("A8 {}: esp {esperado} / obt {obtenido}", c["caso"]));
            }
        }
        for c in fixture["a8_libro"].as_array().unwrap() {
            let obtenido = match leer_carga(&b64(c)) {
                Ok((formato, filas)) => json!({"formato": formato, "filas": filas_json(&filas)}),
                Err(e) => json!({"error": e}),
            };
            let esperado = ordenar(c["python"].clone());
            if obtenido == esperado {
                cuenta[2] += 1;
            } else {
                fallos.push(format!("A8 libro {}: esp {esperado} / obt {obtenido}", c["caso"]));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        for a in &aceptadas {
            println!("DIFERENCIA_ACEPTADA {a}");
        }
        let total = [fixture["a7"].as_array().unwrap().len(), fixture["a8"].as_array().unwrap().len(), fixture["a8_libro"].as_array().unwrap().len()];
        println!(
            "A78: A7 {}/{}, A8 {}/{}, A8_LIBRO {}/{}, aceptadas {}, fallos {}",
            cuenta[0], total[0], cuenta[1], total[1], cuenta[2], total[2], aceptadas.len(), fallos.len()
        );
        assert_eq!(aceptadas.len(), 1, "solo la diferencia D-3 está aceptada");
        assert_eq!(total, [8, 7, 1], "el fixture cambió de tamaño: regenerar y revisar");
        assert!(fallos.is_empty(), "{} casos difieren de Flow Python", fallos.len());
    }
}
