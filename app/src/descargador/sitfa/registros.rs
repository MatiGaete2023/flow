//! Cotejo por RIT y nombre completo con multiplicidad (`motor.record_columns`, `record_from_row` y `records_in_grid`).

use super::Registros;
use regex::Regex;
use scraper::{ElementRef, Selector};
use std::sync::LazyLock;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

static RIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Z]+\s*-\s*\d+\s*-\s*\d{4}\b").unwrap());
static FILAS: LazyLock<Selector> = LazyLock::new(|| Selector::parse("tr").unwrap());

pub fn normalizar(s: &str) -> String {
    s.replace('\u{a0}', " ")
        .nfkd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_uppercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct Columnas {
    causa: Vec<usize>,
    nombre: usize,
}

/// Encabezados observados en XLS reales; «Nombre Centro» no se confunde con el nombre del NNA.
pub fn columnas(fila: &[String]) -> Result<Option<Columnas>, String> {
    let valores: Vec<_> = fila.iter().map(|v| normalizar(v)).collect();
    let causa = if valores.iter().any(|s| s == "RIT") {
        vec!["RIT"]
    } else if ["TIPO CAUSA", "ROL CAUSA", "ERA CAUSA"]
        .iter()
        .all(|k| valores.iter().any(|s| s == k))
    {
        vec!["TIPO CAUSA", "ROL CAUSA", "ERA CAUSA"]
    } else {
        return Ok(None);
    };
    let nombres: Vec<_> = valores
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.as_str(), "NOMBRE" | "NOMBRE MENOR"))
        .collect();
    if nombres.is_empty() {
        return Ok(None);
    }
    if nombres.len() != 1
        || causa
            .iter()
            .any(|k| valores.iter().filter(|s| s == k).count() != 1)
    {
        return Err("El listado contiene columnas de causa o nombre ambiguas.".into());
    }
    Ok(Some(Columnas {
        causa: causa
            .iter()
            .map(|k| valores.iter().position(|s| s == k).unwrap())
            .collect(),
        nombre: nombres[0].0,
    }))
}

pub fn clave(rit: &str, nombre: &str) -> Result<(String, String), String> {
    let rit = normalizar(rit).replace(['–', '—'], "-");
    let nombre = normalizar(nombre);
    let rit = RIT
        .find(&rit)
        .ok_or("No se reconoce la estructura RIT/Nombre del listado; descarga detenida.")?;
    if nombre.is_empty() {
        return Err("No se reconoce la estructura RIT/Nombre del listado; descarga detenida.".into());
    }
    Ok((rit.as_str().split_whitespace().collect(), nombre))
}

pub fn registro(fila: &[String], c: &Columnas) -> Option<(String, String)> {
    if fila.len() <= c.causa.iter().copied().chain([c.nombre]).max()? {
        return None;
    }
    let partes: Vec<String> = c.causa.iter().map(|i| normalizar(&fila[*i])).collect();
    let rit = if partes.len() == 3 {
        let letra = partes[0].len() == 1 && partes[0].bytes().all(|b| b.is_ascii_uppercase());
        let rol = !partes[1].is_empty() && partes[1].bytes().all(|b| b.is_ascii_digit());
        let era = partes[2].len() == 4 && partes[2].bytes().all(|b| b.is_ascii_digit());
        if !(letra && rol && era) {
            return None;
        }
        partes.join("-")
    } else {
        partes[0].clone()
    };
    if !RIT.is_match(&rit) {
        return None;
    }
    clave(&rit, &fila[c.nombre]).ok()
}

/// Registros reconocidos bajo la última cabecera; las demás filas se ignoran.
pub fn contar(filas: &[Vec<String>]) -> Result<Registros, String> {
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

/// Filas directas de la tabla (`./tr | ./thead/tr | ./tbody/tr | ./tfoot/tr`). Indica también si la fila es de datos
/// del cuerpo (`./tr[td] | ./tbody/tr[td]`). Las filas de una tabla anidada nunca cuentan como filas propias.
pub fn grilla_con_cuerpo(tabla: ElementRef<'_>) -> Result<Vec<(Vec<String>, bool)>, String> {
    let mut filas = Vec::new();
    let mut total = 0;
    for fila in tabla.select(&FILAS) {
        let padre = fila.parent().and_then(ElementRef::wrap);
        let seccion = padre.map(|e| e.value().name().to_string()).unwrap_or_default();
        let propia = match seccion.as_str() {
            "table" => padre.map(|p| p.id()) == Some(tabla.id()),
            "thead" | "tbody" | "tfoot" => {
                padre
                    .and_then(|p| p.parent())
                    .and_then(ElementRef::wrap)
                    .map(|t| t.id())
                    == Some(tabla.id())
            }
            _ => false,
        };
        if !propia {
            continue;
        }
        let mut valores = Vec::new();
        let mut con_td = false;
        for celda in fila
            .children()
            .filter_map(ElementRef::wrap)
            .filter(|e| matches!(e.value().name(), "th" | "td"))
        {
            con_td |= celda.value().name() == "td";
            valores.push(celda.text().collect::<String>());
            if valores.len() > 200 {
                return Err("La tabla supera los límites de filas/celdas.".into());
            }
        }
        total += valores.len();
        if filas.len() >= 200_000 || total > 2_000_000 {
            return Err("La tabla supera los límites de filas/celdas.".into());
        }
        filas.push((valores, con_td && matches!(seccion.as_str(), "table" | "tbody")));
    }
    Ok(filas)
}

pub fn grilla(tabla: ElementRef<'_>) -> Result<Vec<Vec<String>>, String> {
    Ok(grilla_con_cuerpo(tabla)?.into_iter().map(|(f, _)| f).collect())
}

pub fn filas_de_datos(tabla: ElementRef<'_>) -> Result<usize, String> {
    Ok(grilla_con_cuerpo(tabla)?.iter().filter(|(_, cuerpo)| *cuerpo).count())
}

fn mensaje_vacio(fila: &[String]) -> bool {
    fila.len() == 1
        && matches!(
            normalizar(&fila[0]).trim_end_matches('.'),
            "SIN REGISTROS"
                | "NO HAY REGISTROS"
                | "SIN RESULTADOS"
                | "NO SE ENCONTRARON REGISTROS"
                | "NO EXISTEN REGISTROS"
        )
}

/// Listado de la consulta (`motor.result_records`): cada fila de datos del cuerpo es un registro, salvo cabeceras y el
/// mensaje exacto de lista vacía. Nunca se descarta en silencio una fila desconocida.
pub fn contar_listado(filas: &[(Vec<String>, bool)]) -> Result<Registros, String> {
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
