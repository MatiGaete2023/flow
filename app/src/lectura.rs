//! Lectura de libros de RUS/SITFA: .xlsx/.xlsm, .xls BIFF y los .xls de SITFA que son HTML o XML 2003.
//! Detecta la fila de encabezados, la etapa (Espera, Cumplimiento, Informes) y la hoja de cruce.

use crate::motor::{Cols, Mode};
use crate::util::{normalizar, parse_date_text, Cell};
use calamine::{Data, Reader};
use chrono::NaiveDate;
use regex::Regex;
use std::io::Cursor;
use std::sync::OnceLock;

#[derive(Clone, Debug, Default)]
pub struct Sheet {
    pub name: String,
    pub rows: Vec<Vec<Cell>>,
}

pub fn read_sheets(bytes: &[u8]) -> Result<Vec<Sheet>, String> {
    let head = &bytes[..bytes.len().min(4096)];
    let low = String::from_utf8_lossy(head).to_lowercase();
    if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]) {
        return read_calamine(bytes);
    }
    if low.contains("urn:schemas-microsoft-com:office:spreadsheet") {
        return read_xml2003(&decode_text(bytes));
    }
    if low.contains("<table") || low.contains("<html") {
        return read_html(&decode_text(bytes));
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        if s.contains(';') || s.contains(',') || s.contains('\t') {
            return Ok(vec![read_csv(s)]);
        }
    }
    Err("No se reconoce el formato del archivo. Usa un Excel de RUS (.xls o .xlsx).".into())
}

fn read_calamine(bytes: &[u8]) -> Result<Vec<Sheet>, String> {
    let mut wb = calamine::open_workbook_auto_from_rs(Cursor::new(bytes.to_vec())).map_err(|e| format!("No se pudo abrir el libro: {e}"))?;
    let mut out = Vec::new();
    for name in wb.sheet_names().to_owned() {
        let Ok(range) = wb.worksheet_range(&name) else { continue };
        let (r0, c0) = range.start().unwrap_or((0, 0));
        let mut rows: Vec<Vec<Cell>> = vec![Vec::new(); r0 as usize];
        for row in range.rows() {
            let mut cells: Vec<Cell> = vec![Cell::Empty; c0 as usize];
            cells.extend(row.iter().map(convert));
            rows.push(cells);
        }
        out.push(Sheet { name, rows });
    }
    Ok(out)
}

fn convert(d: &Data) -> Cell {
    match d {
        Data::Empty => Cell::Empty,
        Data::String(s) => Cell::Text(s.trim_end_matches('\u{a0}').to_string()),
        Data::Float(f) => Cell::Number(*f),
        Data::Int(i) => Cell::Number(*i as f64),
        Data::Bool(b) => Cell::Bool(*b),
        Data::DateTime(dt) => crate::util::excel_serial(dt.as_f64()).map(Cell::Date).unwrap_or_else(|| Cell::Number(dt.as_f64())),
        Data::DateTimeIso(s) => parse_date_text(s).map(Cell::Date).unwrap_or_else(|| Cell::Text(s.clone())),
        Data::DurationIso(s) => Cell::Text(s.clone()),
        Data::Error(_) => Cell::Empty,
    }
}

/// HTML/XML de SITFA: respeta la declaración charset; sin ella, UTF-8 estricto y si no Windows-1252.
pub fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf][..]).unwrap_or(bytes);
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(2048)]).to_lowercase();
    let declared_latin = ["iso-8859-1", "latin-1", "latin1", "windows-1252", "cp1252"].iter().any(|c| head.contains(c));
    if !declared_latin {
        if let Ok(s) = std::str::from_utf8(bytes) {
            return s.to_string();
        }
    }
    let (s, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    s.into_owned()
}

fn re(c: &'static OnceLock<Regex>, p: &str) -> &'static Regex {
    c.get_or_init(|| Regex::new(p).unwrap())
}

fn unescape(s: &str) -> String {
    static NUM: OnceLock<Regex> = OnceLock::new();
    let s = s
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");
    let s = re(&NUM, r"&#(x?[0-9a-fA-F]+);").replace_all(&s, |c: &regex::Captures| {
        let v = &c[1];
        let n = if let Some(h) = v.strip_prefix('x') { u32::from_str_radix(h, 16).ok() } else { v.parse().ok() };
        n.and_then(char::from_u32).map(|c| c.to_string()).unwrap_or_default()
    });
    s.replace("&amp;", "&").replace('\u{a0}', " ")
}

fn strip_tags(s: &str) -> String {
    static T: OnceLock<Regex> = OnceLock::new();
    static BR: OnceLock<Regex> = OnceLock::new();
    let s = re(&BR, r"(?i)<br\s*/?>").replace_all(s, "\n");
    let s = re(&T, r"(?s)<[^>]*>").replace_all(&s, "");
    unescape(&s).lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n").trim().to_string()
}

fn text_cell(s: String) -> Cell {
    if s.trim().is_empty() { Cell::Empty } else { Cell::Text(s) }
}

pub fn read_html(html: &str) -> Result<Vec<Sheet>, String> {
    static TABLE: OnceLock<Regex> = OnceLock::new();
    static TR: OnceLock<Regex> = OnceLock::new();
    static TD: OnceLock<Regex> = OnceLock::new();
    let mut out = Vec::new();
    for (n, t) in re(&TABLE, r"(?is)<table[^>]*>(.*?)</table>").captures_iter(html).enumerate() {
        let mut rows = Vec::new();
        for tr in re(&TR, r"(?is)<tr[^>]*>(.*?)</tr>").captures_iter(&t[1]) {
            let mut cells = Vec::new();
            for td in re(&TD, r#"(?is)<t([dh])([^>]*)>(.*?)</t[dh]>"#).captures_iter(&tr[1]) {
                cells.push(text_cell(strip_tags(&td[3])));
                if let Some(span) = Regex::new(r#"(?i)colspan\s*=\s*"?(\d+)"#).unwrap().captures(&td[2]) {
                    let k: usize = span[1].parse().unwrap_or(1);
                    for _ in 1..k.min(50) {
                        cells.push(Cell::Empty);
                    }
                }
            }
            rows.push(cells);
        }
        if !rows.is_empty() {
            out.push(Sheet { name: format!("Tabla {}", n + 1), rows });
        }
    }
    if out.is_empty() {
        return Err("El archivo HTML no contiene tablas.".into());
    }
    Ok(out)
}

pub fn read_xml2003(xml: &str) -> Result<Vec<Sheet>, String> {
    static WS: OnceLock<Regex> = OnceLock::new();
    static ROW: OnceLock<Regex> = OnceLock::new();
    static CELL: OnceLock<Regex> = OnceLock::new();
    static DATA: OnceLock<Regex> = OnceLock::new();
    static IDX: OnceLock<Regex> = OnceLock::new();
    let mut out = Vec::new();
    for w in re(&WS, r#"(?is)<(?:ss:)?Worksheet[^>]*?(?:ss:)?Name="([^"]*)"[^>]*>(.*?)</(?:ss:)?Worksheet>"#).captures_iter(xml) {
        let mut rows = Vec::new();
        for r in re(&ROW, r"(?is)<(?:ss:)?Row[^>]*?(?:/>|>(.*?)</(?:ss:)?Row>)").captures_iter(&w[2]) {
            let mut cells: Vec<Cell> = Vec::new();
            let body = r.get(1).map(|m| m.as_str()).unwrap_or("");
            for c in re(&CELL, r"(?is)<(?:ss:)?Cell([^>]*?)(?:/>|>(.*?)</(?:ss:)?Cell>)").captures_iter(body) {
                if let Some(i) = re(&IDX, r#"(?:ss:)?Index="(\d+)""#).captures(&c[1]) {
                    let target: usize = i[1].parse::<usize>().unwrap_or(1).saturating_sub(1);
                    while cells.len() < target {
                        cells.push(Cell::Empty);
                    }
                }
                let inner = c.get(2).map(|m| m.as_str()).unwrap_or("");
                let cell = match re(&DATA, r#"(?is)<(?:ss:)?Data[^>]*?(?:ss:)?Type="([^"]*)"[^>]*>(.*?)</(?:ss:)?Data>"#).captures(inner) {
                    Some(d) => {
                        let v = strip_tags(&d[2]);
                        match &d[1] {
                            "Number" => v.parse::<f64>().map(Cell::Number).unwrap_or(text_cell(v)),
                            "DateTime" => parse_date_text(&v).map(Cell::Date).unwrap_or(text_cell(v)),
                            _ => text_cell(v),
                        }
                    }
                    None => Cell::Empty,
                };
                cells.push(cell);
            }
            rows.push(cells);
        }
        out.push(Sheet { name: unescape(&w[1]), rows });
    }
    if out.is_empty() {
        return Err("El XML no contiene hojas de cálculo.".into());
    }
    Ok(out)
}

pub fn read_csv(s: &str) -> Sheet {
    let first = s.lines().next().unwrap_or("");
    let sep = [';', '\t', ','].into_iter().max_by_key(|c| first.matches(*c).count()).unwrap_or(';');
    let rows = s
        .lines()
        .map(|l| l.split(sep).map(|v| text_cell(v.trim().trim_matches('"').to_string())).collect())
        .collect();
    Sheet { name: "CSV".into(), rows }
}

// ── columnas ─────────────────────────────────────────────────────────────────

const COMMON: &[(&str, &[&str])] = &[
    ("programa", &["DERIVACION", "DERIVACIÓN", "PROGRAMA", "NOMBRE CENTRO"]),
    ("tribunal", &["TRIBUNAL"]),
    ("nombre", &["NOMBRE", "NOMBRE COMPLETO", "NOMBRE MENOR"]),
    ("nacimiento", &["FEC. NACIMIENTO", "FEC.NACIMIENTO", "FECHA NACIMIENTO", "FEC NACIMIENTO"]),
    ("curador", &["CURADOR", "CURADOR AD LITEM", "CURADOR AD-LITEM", "CURADOR AD LITEM.", "CUR. AD LITEM", "CURADOR/A AD LITEM", "CURADOR/A AD-LITEM"]),
    ("oido", &["FEC. OIDO", "FEC.OIDO", "FEC OIDO", "FECHAOIDO", "FEC. OÍDO", "FEC.OÍDO"]),
    ("resolucion", &["FEC. RESOLUCIÓN", "FEC.RESOLUCIÓN", "FEC. RESOLUCION", "FEC.RESOLUCION", "FECHA RESOLUCION", "FEC RESOLUCION"]),
    ("prox_aud", &["PROXS. AUDS.", "PROXS AUDS", "PROX AUD", "PROXIMA AUDIENCIA", "PRÓXIMA AUDIENCIA", "PROX. AUD.", "PROX. AUDS."]),
    ("rit", &["RIT"]),
    ("rut", &["RUT", "RUT MENOR", "RUT NNA", "RUT LITIGANTE"]),
    ("duracion", &["DURACION", "DURACIÓN"]),
    ("res", &["RES"]),
];

const VENCIMIENTO: &[&str] = &["FECHA VENCIMIENTO", "FEC.VENCIMIENTO", "FEC. VENCIMIENTO", "F. VENCIMIENTO", "F.VENCIMIENTO", "FVENCIMIENTO", "FECHA DE VENCIMIENTO"];
const INGRESO: &[&str] = &["FEC.INGRESO EFECTIVO", "FEC. INGRESO EFECTIVO", "FEC INGRESO EFECTIVO"];

fn mode_columns(m: Mode) -> Vec<(&'static str, &'static [&'static str])> {
    match m {
        Mode::Espera => vec![("espera", &["T ESPERA", "T_ESPERA", "DIAS_ESPERA", "TESPERA", "DÍAS DE ESPERA", "DIAS DE ESPERA"][..])],
        Mode::Cumplimiento => vec![
            ("dias_cumpl", &["DIAS DE CUMPLIMIENTO", "DÍAS DE CUMPLIMIENTO", "DIAS CUMPLIMIENTO"][..]),
            ("dias_egresar", &["DIAS PARA EGRESAR", "DÍAS PARA EGRESAR", "DIAS EGRESAR"][..]),
            ("ingreso", INGRESO),
            ("egreso_proy", &["FEC.EGRESO PROYECTADO", "FEC. EGRESO PROYECTADO", "FEC EGRESO PROYECTADO"][..]),
            ("ficha_fae", &["FEC.ACT.F.FAE", "FEC. ACT. F. FAE", "FEC ACT F FAE", "FEC.ACT.F.FAE/FAS", "FEC. ACT. F. FAE/FAS"][..]),
            ("ficha_ind", &["FEC.ACT.F.INDIVIDUAL", "FEC. ACT. F. INDIVIDUAL", "FEC ACT F INDIVIDUAL"][..]),
        ],
        Mode::Informes => vec![("vencimiento", VENCIMIENTO), ("ingreso", INGRESO)],
    }
}

fn required(m: Mode) -> &'static [&'static str] {
    match m {
        Mode::Espera => &["programa", "tribunal", "espera"],
        Mode::Cumplimiento => &["programa", "tribunal", "dias_cumpl", "dias_egresar"],
        Mode::Informes => &["programa", "tribunal", "vencimiento"],
    }
}

fn map_with(headers: &[String], aliases: &[(&'static str, &[&str])]) -> Cols {
    let norm: Vec<String> = headers.iter().map(|h| normalizar(h)).collect();
    let mut cols = Cols::new();
    for (key, names) in aliases {
        let set: Vec<String> = names.iter().map(|n| normalizar(n)).collect();
        if let Some(i) = norm.iter().position(|h| !h.is_empty() && set.contains(h)) {
            cols.insert(key, i);
        }
    }
    cols
}

pub fn map_columns(headers: &[String], m: Mode) -> Cols {
    let mut all: Vec<(&'static str, &[&str])> = COMMON.to_vec();
    all.extend(mode_columns(m));
    map_with(headers, &all)
}

pub fn map_cross(headers: &[String]) -> Cols {
    map_with(
        headers,
        &[
            ("rit", &["RIT"][..]),
            ("rut", &["RUT MENOR", "RUT"][..]),
            ("nombre", &["NOMBRE MENOR", "NOMBRE"][..]),
            ("tribunal", &["TRIBUNAL"][..]),
            ("programa", &["NOMBRE CENTRO", "DERIVACION", "DERIVACIÓN"][..]),
            ("vencimiento", VENCIMIENTO),
        ],
    )
}

pub fn headers_of(row: &[Cell]) -> Vec<String> {
    row.iter().map(|c| c.display().trim().to_string()).collect()
}

/// Tabla detectada dentro de un libro.
#[derive(Clone, Debug)]
pub struct Tabla {
    pub sheet: usize,
    pub header_row: usize,
    pub modes: Vec<Mode>,
}

fn sheet_mode_hint(name: &str) -> Option<Mode> {
    let n = normalizar(name);
    if n.contains("espera") {
        Some(Mode::Espera)
    } else if n.contains("cumpl") {
        Some(Mode::Cumplimiento)
    } else if n.contains("inform") {
        Some(Mode::Informes)
    } else {
        None
    }
}

/// Encuentra las tablas de RUS del libro (las primeras 30 filas de cada hoja).
pub fn detectar(sheets: &[Sheet]) -> Vec<Tabla> {
    let mut out = Vec::new();
    for (si, s) in sheets.iter().enumerate() {
        for (ri, row) in s.rows.iter().take(30).enumerate() {
            let h = headers_of(row);
            let modes: Vec<Mode> = Mode::ALL
                .into_iter()
                .filter(|m| {
                    let c = map_columns(&h, *m);
                    required(*m).iter().all(|k| c.contains_key(k))
                })
                .collect();
            if !modes.is_empty() {
                out.push(Tabla { sheet: si, header_row: ri, modes });
                break;
            }
        }
    }
    out
}

/// Elige la etapa: la pedida, la del nombre de la hoja o la única posible.
pub fn elegir(sheets: &[Sheet], tablas: &[Tabla], pedida: Option<Mode>) -> Option<(usize, Mode)> {
    if let Some(m) = pedida {
        if let Some((i, _)) = tablas.iter().enumerate().find(|(_, t)| t.modes.contains(&m) && sheet_mode_hint(&sheets[t.sheet].name) == Some(m)) {
            return Some((i, m));
        }
        return tablas.iter().position(|t| t.modes.contains(&m)).map(|i| (i, m));
    }
    for (i, t) in tablas.iter().enumerate() {
        if let Some(h) = sheet_mode_hint(&sheets[t.sheet].name) {
            if t.modes.contains(&h) {
                return Some((i, h));
            }
        }
    }
    let t = tablas.first()?;
    // Varias etapas posibles: la más específica según sus columnas propias.
    let m = if t.modes.len() == 1 {
        t.modes[0]
    } else if t.modes.contains(&Mode::Espera) && !t.modes.contains(&Mode::Cumplimiento) {
        Mode::Espera
    } else {
        t.modes[0]
    };
    Some((0, m))
}

/// Hoja de cruce de Cumplimiento (próximo informe): otra hoja con RIT, RUT, NOMBRE, TRIBUNAL, programa y vencimiento.
pub fn buscar_cruce(sheets: &[Sheet], principal: usize) -> Option<(usize, usize, Cols)> {
    for (si, s) in sheets.iter().enumerate() {
        if si == principal {
            continue;
        }
        for (ri, row) in s.rows.iter().take(30).enumerate() {
            let c = map_cross(&headers_of(row));
            if c.len() == 6 {
                return Some((si, ri, c));
            }
        }
    }
    None
}

pub fn date_of(c: &Cell) -> Option<NaiveDate> {
    crate::util::get_date(c)
}
