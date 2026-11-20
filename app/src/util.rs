//! Utilidades de texto y fechas compartidas por el motor, la lectura y los productos.

use chrono::{Datelike, Duration, NaiveDate};
use regex::Regex;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

/// Valor de una celda ya leído del libro.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Cell {
    Empty,
    Text(String),
    Number(f64),
    Date(NaiveDate),
    Bool(bool),
}

impl Default for Cell {
    fn default() -> Self {
        Cell::Empty
    }
}

impl Cell {
    /// Texto visible de la celda (fechas como dd/mm/aaaa, enteros sin decimales).
    pub fn display(&self) -> String {
        match self {
            Cell::Empty => String::new(),
            Cell::Text(s) => s.clone(),
            Cell::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Cell::Date(d) => d.format("%d/%m/%Y").to_string(),
            Cell::Bool(b) => if *b { "VERDADERO".into() } else { "FALSO".into() },
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Cell::Empty => true,
            Cell::Text(s) => is_empty_text(s),
            _ => false,
        }
    }
}

const EMPTY_VALUES: [&str; 6] = ["", "---", "-", "nan", "nat", "none"];

pub fn is_empty_text(s: &str) -> bool {
    let t = s.trim().to_lowercase();
    EMPTY_VALUES.contains(&t.as_str())
}

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).expect("regex válida"))
}

/// Sin tildes, minúscula, guiones/paréntesis/puntos como espacio, espacios colapsados.
pub fn normalizar(txt: &str) -> String {
    static G: OnceLock<Regex> = OnceLock::new();
    static S: OnceLock<Regex> = OnceLock::new();
    let ascii: String = txt.nfkd().filter(|c| c.is_ascii()).collect();
    let low = ascii.to_lowercase();
    let low = low.trim();
    let sin = re(&G, r"[-().]+").replace_all(low, " ");
    re(&S, r"\s+").replace_all(&sin, " ").trim().to_string()
}

/// Clave de comparación: normalizado y con espacios únicos.
pub fn clave(txt: &str) -> String {
    normalizar(txt).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Sin tildes y en mayúscula (para siglas).
pub fn ascii_upper(txt: &str) -> String {
    txt.nfkd().filter(|c| c.is_ascii()).collect::<String>().to_uppercase()
}

/// Primera letra en mayúscula y el resto en minúscula.
pub fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

/// Quita paréntesis y lo escrito en ellos, colapsa espacios y deja cada palabra en Title Case.
pub fn limpiar_nombre(txt: &str) -> String {
    sin_parentesis(txt).split_whitespace().map(capitalize).collect::<Vec<_>>().join(" ")
}

/// Nombre sin paréntesis ni su contenido (para nóminas, correos y proyectos), sin cambiar mayúsculas.
pub fn sin_parentesis(txt: &str) -> String {
    static P: OnceLock<Regex> = OnceLock::new();
    let a = re(&P, r"\([^()]*\)").replace_all(txt, " ");
    a.replace(['(', ')'], " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn primer_nombre(nombre: &str) -> String {
    limpiar_nombre(nombre).split_whitespace().next().unwrap_or("").to_string()
}

const PREPOSICIONES: [&str; 11] = ["de", "del", "los", "las", "y", "en", "el", "la", "por", "con", "a"];

/// Nombre de programa legible: siglas en mayúscula, preposiciones en minúscula, el resto capitalizado.
pub fn titulo_programa(txt: &str) -> String {
    static SEP: OnceLock<Regex> = OnceLock::new();
    let colapsado = txt.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = Vec::new();
    for (i, t) in re(&SEP, r"[\s\-]+").split(colapsado.trim()).enumerate() {
        if t.is_empty() {
            continue;
        }
        let solo: String = t.chars().filter(|c| c.is_alphabetic()).collect();
        let low = normalizar(t);
        let prep = PREPOSICIONES.contains(&low.as_str());
        if i > 0 && prep {
            out.push(t.to_lowercase());
        } else if !solo.is_empty() && solo.chars().count() <= 4 && !prep {
            out.push(t.to_uppercase());
        } else {
            out.push(capitalize(t));
        }
    }
    out.join(" ")
}

/// Prefijo de la observación: «PrimerNombre XXX: ».
pub fn prefijo_observacion(nombre: &str, programa: &str) -> String {
    let primer = primer_nombre(nombre);
    let tres: String = ascii_upper(programa).chars().take(3).collect();
    match (primer.is_empty(), tres.is_empty()) {
        (false, false) => format!("{primer} {tres}: "),
        (false, true) => format!("{primer}: "),
        (true, false) => format!("{tres}: "),
        _ => String::new(),
    }
}

const MESES: [&str; 13] = [
    "", "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre",
    "noviembre", "diciembre",
];

/// «13 de mayo de 2026».
pub fn fecha_es(d: NaiveDate) -> String {
    format!("{} de {} de {}", d.day(), MESES[d.month() as usize], d.year())
}

/// `outputs.number_in_words` de Flow Python: cardinal para días y años (0 a 9999).
pub fn numero_en_palabras(n: u32) -> String {
    const UNIDADES: [&str; 10] = ["cero", "uno", "dos", "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve"];
    const ESPECIALES: [&str; 20] = [
        "diez", "once", "doce", "trece", "catorce", "quince", "dieciséis", "diecisiete", "dieciocho", "diecinueve",
        "veinte", "veintiuno", "veintidós", "veintitrés", "veinticuatro", "veinticinco", "veintiséis", "veintisiete",
        "veintiocho", "veintinueve",
    ];
    const DECENAS: [&str; 10] = ["", "", "", "treinta", "cuarenta", "cincuenta", "sesenta", "setenta", "ochenta", "noventa"];
    const CENTENAS: [&str; 10] = [
        "", "cien", "doscientos", "trescientos", "cuatrocientos", "quinientos", "seiscientos", "setecientos", "ochocientos",
        "novecientos",
    ];
    match n {
        0..=9 => UNIDADES[n as usize].into(),
        10..=29 => ESPECIALES[(n - 10) as usize].into(),
        30..=99 => {
            let u = n % 10;
            let cola = if u > 0 { format!(" y {}", UNIDADES[u as usize]) } else { String::new() };
            format!("{}{cola}", DECENAS[(n / 10) as usize])
        }
        100..=999 => {
            let resto = n % 100;
            let cabeza = if n / 100 == 1 && resto > 0 { "ciento" } else { CENTENAS[(n / 100) as usize] };
            if resto > 0 {
                format!("{cabeza} {}", numero_en_palabras(resto))
            } else {
                cabeza.into()
            }
        }
        _ => {
            let n = n.min(9999);
            let (miles, resto) = (n / 1000, n % 1000);
            let cabeza = if miles == 1 { "mil".to_string() } else { format!("{} mil", numero_en_palabras(miles)) };
            if resto > 0 {
                format!("{cabeza} {}", numero_en_palabras(resto))
            } else {
                cabeza
            }
        }
    }
}

/// `outputs.date_in_words`: «catorce de septiembre de dos mil veintiséis».
pub fn fecha_en_palabras(d: NaiveDate) -> String {
    format!(
        "{} de {} de {}",
        numero_en_palabras(d.day()),
        MESES[d.month() as usize],
        numero_en_palabras(d.year().max(0) as u32)
    )
}

/// «el A», «el A y el B», «el A, el B y el C».
pub fn lista_fechas_es(fechas: &[NaiveDate]) -> String {
    let partes: Vec<String> = fechas.iter().map(|f| format!("el {}", fecha_es(*f))).collect();
    lista_y(&partes)
}

/// «A», «A y B», «A, B y C».
pub fn lista_y(partes: &[String]) -> String {
    match partes.len() {
        0 => String::new(),
        1 => partes[0].clone(),
        n => format!("{} y {}", partes[..n - 1].join(", "), partes[n - 1]),
    }
}

pub fn excel_serial(n: f64) -> Option<NaiveDate> {
    if !(0.0 < n && n < 100000.0) {
        return None;
    }
    NaiveDate::from_ymd_opt(1899, 12, 30).map(|b| b + Duration::days(n.floor() as i64))
}

/// Lectura determinista de una fecha: fecha nativa, número de serie Excel, ISO o dd/mm/aaaa.
pub fn get_date(c: &Cell) -> Option<NaiveDate> {
    match c {
        Cell::Date(d) => Some(*d),
        Cell::Number(n) if *n > 30000.0 && *n < 60000.0 => excel_serial(*n),
        Cell::Text(s) => parse_date_text(s),
        _ => None,
    }
}

pub fn parse_date_text(s: &str) -> Option<NaiveDate> {
    let t = s.trim().trim_end_matches('\u{a0}').trim();
    if is_empty_text(t) {
        return None;
    }
    // Fecha con hora («2026-11-19 00:00:00»): se toma solo la fecha. Otra cosa tras un espacio no es una fecha única.
    static HORA: OnceLock<Regex> = OnceLock::new();
    let base = match t.split_once(['T', ' ']) {
        Some((d, resto)) if re(&HORA, r"^\d{1,2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:?\d{2})?$").is_match(resto.trim()) => d,
        Some(_) => return None,
        None => t,
    };
    for fmt in ["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d.%m.%Y", "%Y/%m/%d"] {
        if let Ok(d) = NaiveDate::parse_from_str(base, fmt) {
            return Some(d);
        }
    }
    for fmt in ["%d/%m/%y", "%d-%m-%y"] {
        if let Ok(d) = NaiveDate::parse_from_str(base, fmt) {
            return Some(d);
        }
    }
    if let Ok(n) = base.replace(',', ".").parse::<f64>() {
        if n > 30000.0 && n < 60000.0 {
            return excel_serial(n);
        }
    }
    None
}

/// Celda con una o varias fechas separadas por coma, punto y coma o salto de línea.
/// Devuelve (fechas legibles, partes ilegibles).
pub fn fechas_varias(c: &Cell) -> (Vec<NaiveDate>, usize) {
    if let Some(d) = get_date(c) {
        return (vec![d], 0);
    }
    let txt = match c {
        Cell::Text(s) => s.clone(),
        Cell::Empty => return (vec![], 0),
        other => other.display(),
    };
    if is_empty_text(&txt) {
        return (vec![], 0);
    }
    let mut fechas = Vec::new();
    let mut ilegibles = 0;
    for parte in txt.split([',', ';', '\n']) {
        let p = parte.trim();
        if is_empty_text(p) {
            continue;
        }
        match parse_date_text(p) {
            Some(d) => fechas.push(d),
            None => ilegibles += 1,
        }
    }
    (fechas, ilegibles)
}

/// Entero exacto o nada (los decimales no se aceptan).
pub fn get_int(c: &Cell) -> Option<i64> {
    match c {
        Cell::Number(n) if n.is_finite() && n.fract() == 0.0 => Some(*n as i64),
        Cell::Text(s) => {
            let t = s.trim().trim_end_matches('\u{a0}').trim();
            if is_empty_text(t) {
                return None;
            }
            let n: f64 = t.replace(',', ".").parse().ok()?;
            (n.is_finite() && n.fract() == 0.0).then_some(n as i64)
        }
        _ => None,
    }
}

pub fn fecha_mayoria(nac: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(nac.year() + 18, nac.month(), nac.day())
        .or_else(|| NaiveDate::from_ymd_opt(nac.year() + 18, nac.month(), 28))
        .unwrap_or(nac)
}

pub fn edad(nac: NaiveDate, hoy: NaiveDate) -> i32 {
    let mut e = hoy.year() - nac.year();
    if (hoy.month(), hoy.day()) < (nac.month(), nac.day()) {
        e -= 1;
    }
    e
}

pub fn es_dce(programa: &str) -> bool {
    let t = normalizar(programa).to_uppercase();
    t.contains("DCE") || t.contains("DIAGNOSTICO")
}

/// Derivaciones que este Centro no sigue (OPD, DAM, salud, colegio…).
pub fn es_derivacion_sin_seguimiento(programa: &str) -> bool {
    static P: OnceLock<Regex> = OnceLock::new();
    let r = re(
        &P,
        r"^(opd|dam|salud privada|hospital|unidad de salud mental|cesfam|red salud|consulta externa|colegio|chile crece contigo)\b",
    );
    r.is_match(&normalizar(programa))
}

/// Curador efectivamente asignado: un RUT o una institución.
pub fn tiene_curador_real(txt: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"\d{6,8}-[\dkK]").is_match(txt) || txt.contains("Institución:")
}

/// Instituciones distintas en una celda «(RUT)NOMBRE - Institución: X#(RUT)…».
pub fn instituciones(celda: &str) -> usize {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = re(&R, r"(?i)Instituci[oó]n:\s*(.+)$");
    let mut set = std::collections::BTreeSet::new();
    for entrada in celda.split('#') {
        if let Some(m) = r.captures(entrada.trim()) {
            set.insert(clave(&m[1]));
        }
    }
    set.len()
}

pub fn contiene_token(texto: &str, tokens: &[&str]) -> bool {
    let norm = normalizar(texto);
    let palabras: Vec<&str> = norm.split(|c: char| !c.is_ascii_alphanumeric()).filter(|s| !s.is_empty()).collect();
    tokens.iter().any(|t| palabras.contains(&normalizar(t).as_str()))
}

/// Tribunal reconocido (Laja, Mulchén o Tomé).
pub fn tribunal(txt: &str) -> Option<&'static str> {
    let t = normalizar(txt).to_uppercase();
    if t.contains("MULCHEN") {
        Some("MULCHEN")
    } else if t.contains("LAJA") {
        Some("LAJA")
    } else if t.contains("TOME") {
        Some("TOME")
    } else {
        None
    }
}

/// Clave de identidad para cruces: sin paréntesis, minúscula, espacios únicos.
pub fn historical_match(txt: &str) -> String {
    static P: OnceLock<Regex> = OnceLock::new();
    let s = re(&P, r"\(.*?\)|[()]").replace_all(txt, "");
    s.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn rut_clave(rut: &str) -> String {
    rut.to_uppercase().chars().filter(|c| c.is_ascii_digit() || *c == 'K').collect()
}

/// Nombre de archivo seguro.
pub fn safe_filename(txt: &str) -> String {
    let s: String = txt
        .chars()
        .map(|c| if c.is_alphanumeric() || " -_.".contains(c) { c } else { '_' })
        .collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s = s.trim_matches('.').to_string();
    if s.is_empty() { "archivo".into() } else { s.chars().take(80).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textos() {
        assert_eq!(limpiar_nombre("SOFÍA (hija) PÉREZ"), "Sofía Pérez");
        assert_eq!(prefijo_observacion("SOFÍA (hija) PÉREZ", "PRM Ñuble"), "Sofía PRM: ");
        assert_eq!(titulo_programa("PRM ÑUBLE de concepcion"), "PRM Ñuble de Concepcion");
        assert_eq!(fecha_es(NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()), "13 de mayo de 2026");
        assert!(es_derivacion_sin_seguimiento("OPD Laja"));
        let f = |a, m, d| NaiveDate::from_ymd_opt(a, m, d).unwrap();
        assert_eq!(fecha_en_palabras(f(2026, 9, 14)), "catorce de septiembre de dos mil veintiséis");
        assert_eq!(fecha_en_palabras(f(2031, 1, 1)), "uno de enero de dos mil treinta y uno");
        assert_eq!(numero_en_palabras(121), "ciento veintiuno");
        assert_eq!(numero_en_palabras(1900), "mil novecientos");
        assert!(!es_derivacion_sin_seguimiento("PRM Laja"));
        assert_eq!(get_int(&Cell::Text("45".into())), Some(45));
        assert_eq!(get_int(&Cell::Text("4.5".into())), None);
        assert_eq!(fecha_mayoria(NaiveDate::from_ymd_opt(2008, 2, 29).unwrap()), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    }
}
