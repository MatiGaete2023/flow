//! Texto en español: normalización, nombres, programas y fechas escritas.

use chrono::{Datelike, NaiveDate};
use regex::Regex;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

const PREPOSICIONES: [&str; 11] = ["de", "del", "los", "las", "y", "en", "el", "la", "por", "con", "a"];
pub const VALORES_VACIOS: [&str; 6] = ["", "---", "-", "nan", "nat", "none"];
const MESES: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre",
    "diciembre",
];

static RUT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d{6,8}-[\dkK]").unwrap());
static PAREN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(.*?\)").unwrap());
static GUIONES_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[-().]+").unwrap());
static ESPACIOS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static SEP_PROG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\s\-]+").unwrap());
static PALABRAS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z0-9]+").unwrap());

pub fn es_vacio(texto: &str) -> bool {
    let t = texto.trim().to_lowercase();
    VALORES_VACIOS.contains(&t.as_str())
}

/// NFKD y descarte de todo lo que no sea ASCII (equivale a `.encode("ascii", "ignore")`).
pub fn ascii(texto: &str) -> String {
    texto.nfkd().filter(char::is_ascii).collect()
}

/// `motor/utilidades.py::normalizar`: sin tildes, minúsculas, sin guiones ni paréntesis, espacios simples.
pub fn normalizar(texto: &str) -> String {
    let t = ascii(texto).to_lowercase();
    let t = GUIONES_RE.replace_all(t.trim(), " ");
    ESPACIOS_RE.replace_all(&t, " ").trim().to_string()
}

/// `rus/columns.py::normalize`: igual que `normalizar` pero puede dejar un espacio final.
pub fn normalizar_columna(texto: &str) -> String {
    let t = ascii(texto).to_lowercase();
    let t = GUIONES_RE.replace_all(t.trim(), " ");
    ESPACIOS_RE.replace_all(&t, " ").to_string()
}

/// `motor/historia.py::clave`: forma comparable de un nombre de trámite o de un texto.
pub fn clave(texto: &str) -> String {
    normalizar_columna(texto).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `rus/util.py::historical_match`: sin paréntesis, minúsculas, espacios simples (conserva tildes).
pub fn identidad(texto: &str) -> String {
    let sin = PAREN_RE.replace_all(texto, "");
    let sin = sin.replace(['(', ')'], "");
    sin.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn capitalizar(palabra: &str) -> String {
    let mut letras = palabra.chars();
    match letras.next() {
        Some(primera) => primera.to_uppercase().collect::<String>() + &letras.as_str().to_lowercase(),
        None => String::new(),
    }
}

/// Quita paréntesis y su contenido, colapsa espacios y deja cada palabra con mayúscula inicial.
pub fn limpiar_nombre(texto: &str) -> String {
    let s = PAREN_RE.replace_all(texto, "");
    let s = s.replace(['(', ')'], "");
    s.split_whitespace().map(capitalizar).collect::<Vec<_>>().join(" ")
}

pub fn primer_nombre(nombre: &str) -> String {
    limpiar_nombre(nombre).split_whitespace().next().unwrap_or("").to_string()
}

fn solo_letras(token: &str) -> String {
    token.chars().filter(|c| c.is_ascii_alphabetic() || "áéíóúÁÉÍÓÚüÜñÑ".contains(*c)).collect()
}

/// Siglas en mayúsculas, preposiciones en minúsculas, el resto con mayúscula inicial.
pub fn titulo_programa(texto: &str) -> String {
    if texto.is_empty() {
        return String::new();
    }
    let limpio = ESPACIOS_RE.replace_all(texto, " ");
    let mut salida = Vec::new();
    for (i, token) in SEP_PROG_RE.split(limpio.trim()).enumerate() {
        if token.is_empty() {
            continue;
        }
        let solo = solo_letras(token);
        let bajo = normalizar(token);
        let preposicion = PREPOSICIONES.contains(&bajo.as_str());
        if i > 0 && preposicion {
            salida.push(token.to_lowercase());
        } else if !solo.is_empty() && solo.chars().count() <= 4 && !preposicion {
            salida.push(token.to_uppercase());
        } else {
            salida.push(capitalizar(token));
        }
    }
    salida.join(" ")
}

/// «PrimerNombre TRE: » que encabeza cada observación.
pub fn prefijo_observacion(nombre: &str, programa: &str) -> String {
    let primer = primer_nombre(nombre);
    let tres: String = ascii(programa).to_uppercase().chars().take(3).collect();
    match (primer.is_empty(), tres.is_empty()) {
        (false, false) => format!("{primer} {tres}: "),
        (false, true) => format!("{primer}: "),
        (true, false) => format!("{tres}: "),
        (true, true) => String::new(),
    }
}

pub fn contiene_token(texto: &str, tokens: &[&str]) -> bool {
    let norm = normalizar(texto);
    let palabras: Vec<&str> = PALABRAS_RE.find_iter(&norm).map(|m| m.as_str()).collect();
    tokens.iter().any(|t| palabras.contains(&normalizar(t).as_str()))
}

pub fn es_dce(texto: &str) -> bool {
    let t = normalizar(texto).to_uppercase();
    t.contains("DCE") || t.contains("DIAGNOSTICO")
}

/// Derivaciones que el Centro no sigue (OPD, DAM, salud, colegio…): el programa empieza por una de ellas.
pub fn es_derivacion_sin_seguimiento(texto: &str) -> bool {
    const PATRONES: [&str; 10] = [
        "opd",
        "dam",
        "salud privada",
        "hospital",
        "unidad de salud mental",
        "cesfam",
        "red salud",
        "consulta externa",
        "colegio",
        "chile crece contigo",
    ];
    let norm = normalizar(texto);
    PATRONES.iter().any(|p| {
        norm.strip_prefix(p).is_some_and(|resto| match resto.chars().next() {
            None => true,
            Some(c) => !(c.is_ascii_alphanumeric() || c == '_'),
        })
    })
}

/// Hay curador asignado: un RUT en la celda o un curador institucional.
pub fn tiene_curador_real(celda: &str) -> bool {
    RUT_RE.is_match(celda) || celda.contains("Institución:")
}

/// «13 de mayo de 2026».
pub fn fecha_es(fecha: NaiveDate) -> String {
    format!("{} de {} de {}", fecha.day(), MESES[fecha.month0() as usize], fecha.year())
}

/// «el A y el B» o «el A, el B y el C».
pub fn lista_fechas_es(fechas: &[NaiveDate]) -> String {
    let partes: Vec<String> = fechas.iter().map(|f| format!("el {}", fecha_es(*f))).collect();
    match partes.len() {
        0 => String::new(),
        1 => partes[0].clone(),
        n => format!("{} y {}", partes[..n - 1].join(", "), partes[n - 1]),
    }
}

/// Fecha en que se cumplen 18 años (29 de febrero pasa al 28).
pub fn fecha_mayoria(nacimiento: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(nacimiento.year() + 18, nacimiento.month(), nacimiento.day())
        .or_else(|| NaiveDate::from_ymd_opt(nacimiento.year() + 18, nacimiento.month(), 28))
        .expect("fecha de mayoría")
}

pub fn edad_exacta(nacimiento: NaiveDate, hoy: NaiveDate) -> i32 {
    hoy.year() - nacimiento.year() - i32::from((hoy.month(), hoy.day()) < (nacimiento.month(), nacimiento.day()))
}

pub fn dias_para_mayoria(nacimiento: NaiveDate, hoy: NaiveDate) -> i64 {
    (fecha_mayoria(nacimiento) - hoy).num_days()
}

/// RUT comparable: solo dígitos y K.
pub fn rut_clave(rut: &str) -> String {
    rut.to_uppercase().chars().filter(|c| c.is_ascii_digit() || *c == 'K').collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn nombres_y_programas() {
        assert_eq!(limpiar_nombre("JUAN (hijo)  PÉREZ"), "Juan Pérez");
        assert_eq!(titulo_programa("PRM  CIUDAD DEL NIÑO - LAJA"), "PRM Ciudad del NIÑO LAJA");
        assert_eq!(prefijo_observacion("ÁNGELA SOTO", "Ñuble FAE"), "Ángela NUB: ");
        assert!(es_derivacion_sin_seguimiento("OPD Laja"));
        assert!(!es_derivacion_sin_seguimiento("OPDX"));
        assert!(es_dce("Diagnóstico clínico"));
        assert!(contiene_token("FAE-PRO Biobío", &["fae", "fas"]));
        assert!(!contiene_token("FAENA", &["fae"]));
    }

    #[test]
    fn fechas() {
        let f = NaiveDate::from_ymd_opt(2008, 2, 29).unwrap();
        assert_eq!(fecha_mayoria(f), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        assert_eq!(fecha_es(NaiveDate::from_ymd_opt(2026, 5, 13).unwrap()), "13 de mayo de 2026");
    }
}
