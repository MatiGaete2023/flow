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

/// Filas directas de la tabla (las de una tabla anidada nunca cuentan como otra fila), igual que
/// `motor.py::table_grid` del Descargador. Junto a cada fila indica si es una fila de datos del cuerpo: una fila con
/// celdas `td` que está directo en la tabla o en `tbody` (`./tr[td] | ./tbody/tr[td]`). Las de `thead` y `tfoot`
/// (títulos, paginación) se leen para encontrar la cabecera, pero no se exigen como registros.
pub(super) fn grilla_con_cuerpo(tabla: ElementRef<'_>) -> Result<Vec<(Vec<String>, bool)>, Error> {
    let mut filas = Vec::new();
    let mut total = 0;
    for fila in tabla.select(&selector("tr")) {
        if fila.ancestors().filter_map(ElementRef::wrap).find(|e| e.value().name() == "table").map(|e| e.id()) != Some(tabla.id()) {
            continue;
        }
        let seccion = fila.parent().and_then(ElementRef::wrap).map(|e| e.value().name().to_string()).unwrap_or_default();
        let mut valores = Vec::new();
        let mut con_td = false;
        for celda in fila.children().filter_map(ElementRef::wrap).filter(|e| matches!(e.value().name(), "th" | "td")) {
            con_td |= celda.value().name() == "td";
            // Una tabla dentro de una celda aporta su texto, como `text_content()` de lxml en el Descargador.
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
            filas.push((valores, con_td && matches!(seccion.as_str(), "table" | "tbody")));
        }
    }
    Ok(filas)
}

pub(super) fn grilla(tabla: ElementRef<'_>) -> Result<Vec<Vec<String>>, Error> {
    Ok(grilla_con_cuerpo(tabla)?.into_iter().map(|(f, _)| f).collect())
}

/// Filas de datos del cuerpo (para cotejar con otra fuente, como el calendario).
pub(super) fn filas_de_datos(tabla: ElementRef<'_>) -> Result<usize, Error> {
    Ok(grilla_con_cuerpo(tabla)?.iter().filter(|(_, cuerpo)| *cuerpo).count())
}

fn mensaje_vacio(fila: &[String]) -> bool {
    fila.len() == 1
        && matches!(
            normalizar(&fila[0]).trim_end_matches('.'),
            "SIN REGISTROS" | "NO HAY REGISTROS" | "SIN RESULTADOS" | "NO SE ENCONTRARON REGISTROS" | "NO EXISTEN REGISTROS"
        )
}

/// Listado de la consulta (`motor.py::result_records`): cada fila de datos del cuerpo debe ser un registro,
/// salvo cabeceras y el mensaje exacto de lista vacía. Nunca se descarta en silencio una fila desconocida.
pub(super) fn contar_listado(filas: &[(Vec<String>, bool)]) -> Result<Registros, Error> {
    let solo: Vec<Vec<String>> = filas.iter().map(|(f, _)| f.clone()).collect();
    let registros = contar(&solo)?;
    let mut datos = 0usize;
    for (fila, cuerpo) in filas {
        if *cuerpo && columnas(fila)?.is_none() && !mensaje_vacio(fila) {
            datos += 1;
        }
    }
    if registros.values().sum::<usize>() != datos {
        return Err("No se pudieron identificar todos los registros de la consulta.".into());
    }
    Ok(registros)
}

/// Registros reconocidos bajo la última cabecera; las demás filas se ignoran (el control estricto es `contar_listado`).
pub(super) fn contar(filas: &[Vec<String>]) -> Result<Registros, Error> {
    let mut cabecera = None;
    let mut registros = Registros::new();
    for fila in filas {
        if let Some(c) = columnas(fila)? {
            cabecera = Some(c);
        } else if let Some(clave) = cabecera.as_ref().and_then(|c| registro(fila, c)) {
            *registros.entry(clave).or_default() += 1;
        }
    }
    Ok(registros)
}
