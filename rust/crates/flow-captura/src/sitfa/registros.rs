//! Cotejo por RIT y nombre completo (con multiplicidad), sin confundir «Nombre Centro» con «Nombre».

use super::{selector, Error, Registros};
use regex::Regex;
use scraper::ElementRef;
use std::sync::LazyLock;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

static RIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[A-Z]+\s*-\s*\d+\s*-\s*\d{4}\b").unwrap());

pub(super) fn normalizar(s: &str) -> String {
    s.nfkd().filter(|c| !is_combining_mark(*c)).collect::<String>().to_uppercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) struct Columnas {
    causa: Vec<usize>,
    nombre: usize,
}

pub(super) fn columnas(fila: &[String]) -> Result<Option<Columnas>, Error> {
    let valores: Vec<_> = fila.iter().map(|v| normalizar(v)).collect();
    let causa = if valores.iter().any(|s| s == "RIT") {
        vec!["RIT"]
    } else if ["TIPO CAUSA", "ROL CAUSA", "ERA CAUSA"].iter().all(|k| valores.iter().any(|s| s == k)) {
        vec!["TIPO CAUSA", "ROL CAUSA", "ERA CAUSA"]
    } else {
        return Ok(None);
    };
    let nombres: Vec<_> = valores.iter().enumerate().filter(|(_, s)| matches!(s.as_str(), "NOMBRE" | "NOMBRE MENOR")).collect();
    if nombres.is_empty() {
        return Ok(None);
    }
    if nombres.len() != 1 || causa.iter().any(|k| valores.iter().filter(|s| s == k).count() != 1) {
        return Err("El listado contiene columnas de causa o nombre ambiguas.".into());
    }
    Ok(Some(Columnas { causa: causa.iter().map(|k| valores.iter().position(|s| s == k).unwrap()).collect(), nombre: nombres[0].0 }))
}

pub(super) fn clave(rit: &str, nombre: &str) -> Result<(String, String), Error> {
    let rit = normalizar(rit).replace(['–', '—'], "-");
    let nombre = normalizar(nombre);
    let rit = RIT.find(&rit).ok_or("No se reconoce la estructura RIT/Nombre del listado.")?;
    if nombre.is_empty() {
        return Err("Falta el nombre en un registro del listado.".into());
    }
    Ok((rit.as_str().split_whitespace().collect(), nombre))
}

pub(super) fn registro(fila: &[String], c: &Columnas) -> Option<(String, String)> {
    let partes = c.causa.iter().map(|i| fila.get(*i).map(|s| normalizar(s))).collect::<Option<Vec<_>>>()?;
    if partes.len() == 3
        && !(partes[0].len() == 1
            && partes[0].bytes().all(|b| b.is_ascii_uppercase())
            && !partes[1].is_empty()
            && partes[1].bytes().all(|b| b.is_ascii_digit())
            && partes[2].len() == 4
            && partes[2].bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    clave(&partes.join("-"), fila.get(c.nombre)?).ok()
}

/// Solo filas directas de la tabla; una tabla anidada nunca se cuenta como otra fila.
pub(super) fn grilla(tabla: ElementRef<'_>) -> Result<Vec<Vec<String>>, Error> {
    let mut filas = Vec::new();
    let mut total = 0;
    for fila in tabla.select(&selector("tr")) {
        if fila.ancestors().filter_map(ElementRef::wrap).find(|e| e.value().name() == "table").map(|e| e.id()) != Some(tabla.id()) {
            continue;
        }
        let mut valores = Vec::new();
        for celda in fila.children().filter_map(ElementRef::wrap).filter(|e| matches!(e.value().name(), "th" | "td")) {
            if celda.select(&selector("table")).next().is_some() {
                return Err("El listado contiene una tabla anidada no auditada.".into());
            }
            valores.push(celda.text().collect());
            if valores.len() > 200 {
                return Err("La tabla supera el límite de columnas.".into());
            }
        }
        total += valores.len();
        if filas.len() >= 200_000 || total > 2_000_000 {
            return Err("La tabla supera los límites de filas y celdas.".into());
        }
        if !valores.is_empty() {
            filas.push(valores);
        }
    }
    Ok(filas)
}

pub(super) fn contar(filas: &[Vec<String>], estricto: bool) -> Result<Registros, Error> {
    let mut cabecera = None;
    let mut registros = Registros::new();
    for fila in filas {
        if let Some(c) = columnas(fila)? {
            cabecera = Some(c);
        } else if let Some(clave) = cabecera.as_ref().and_then(|c| registro(fila, c)) {
            *registros.entry(clave).or_default() += 1;
        } else if estricto
            && !(fila.len() == 1
                && matches!(
                    normalizar(&fila[0]).trim_end_matches('.'),
                    "SIN REGISTROS" | "NO HAY REGISTROS" | "SIN RESULTADOS" | "NO SE ENCONTRARON REGISTROS" | "NO EXISTEN REGISTROS"
                ))
        {
            return Err("No se pudieron identificar todos los registros de la consulta.".into());
        }
    }
    Ok(registros)
}
