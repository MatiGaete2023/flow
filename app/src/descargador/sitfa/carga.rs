//! Contrato del reporte de Carga (`descargador/carga.py`): lectura completa, sin inferir cobertura de firmas.

use super::registros::normalizar;
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// Encabezado completo y ordenado del XLS de Carga (`carga.HEADERS`).
pub const ENCABEZADOS: [&str; 23] = [
    "RIT",
    "TRIBUNAL",
    "NOMBRE FUNCIONARIO",
    "NOMBRE JUEZ",
    "TRÁMITE",
    "INVALIDADO",
    "DOC. RES",
    "DOC. ESC",
    "DÍAS",
    "CARGO",
    "PRIMER PREGRABADO FECHA",
    "PRIMER PREGRABADO HORA",
    "ÚLTIMO PREGRABADO FECHA",
    "ÚLTIMO PREGRABADO HORA",
    "DESPACHO FECHA",
    "DESPACHO HORA",
    "DEVOLUCIÓN FECHA",
    "DEVOLUCIÓN HORA",
    "NUEVO DESPACHO FECHA",
    "NUEVO DESPACHO HORA",
    "CANTIDAD ESCRITOS RESOLUCIÓN",
    "FECHA FIRMA",
    "HORA FIRMA",
];

/// Filas completas de Carga (23 valores canónicos) y sus repeticiones (`Counter`).
pub type FilasCarga = BTreeMap<Vec<String>, usize>;

static RIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z]+-\d+-\d{4}$").unwrap());

fn es_registro(fila: &[String]) -> bool {
    fila.first().is_some_and(|v| RIT.is_match(&normalizar(v)))
}

/// `repr` de una tupla de Python, para los mensajes de esquema.
fn tupla(valores: &[String]) -> String {
    let partes: Vec<String> = valores.iter().map(|v| format!("'{}'", v.replace('\\', "\\\\").replace('\'', "\\'"))).collect();
    match partes.len() {
        1 => format!("({},)", partes[0]),
        _ => format!("({})", partes.join(", ")),
    }
}

fn encabezados() -> Vec<String> {
    ENCABEZADOS.iter().map(|s| s.to_string()).collect()
}

/// `_record_rows_unchecked`: filas posicionales que empiezan con un RIT; deben tener 23 columnas.
fn filas_sin_validar(grilla: &[Vec<String>]) -> Result<Vec<Vec<String>>, String> {
    let mut filas = Vec::new();
    for fila in grilla.iter().filter(|f| es_registro(f)) {
        if fila.len() != ENCABEZADOS.len() {
            return Err("Cambió el esquema de carga: se esperaban 23 columnas.".into());
        }
        filas.push(fila.iter().map(|v| normalizar(v)).collect());
    }
    Ok(filas)
}

/// `_validate_excel_schema`: un único encabezado completo y ordenado, sin registros antes de él.
fn validar_esquema(grilla: &[Vec<String>]) -> Result<(), String> {
    let esperado: Vec<String> = ENCABEZADOS.iter().map(|s| normalizar(s)).collect();
    let mut cabeceras = Vec::new();
    for (i, fila) in grilla.iter().enumerate() {
        let normal: Vec<String> = fila.iter().map(|v| normalizar(v)).collect();
        // Reconoce un encabezado con una columna renombrada sin confundirlo con filas de título.
        let parece = normal.first() == esperado.first()
            || (normal.len() == esperado.len() && normal.iter().filter(|v| esperado.contains(v)).collect::<std::collections::BTreeSet<_>>().len() >= 12);
        if parece {
            cabeceras.push((i, fila, normal));
        }
    }
    let (indice, fila, normal) = match cabeceras.as_slice() {
        [] => {
            let recibido = grilla.iter().find(|f| es_registro(f)).cloned().unwrap_or_default();
            return Err(format!(
                "No se encontró el encabezado de carga esperado: {}; recibido: {}.",
                tupla(&encabezados()),
                if recibido.is_empty() { "()".into() } else { tupla(&recibido) }
            ));
        }
        [una] => una,
        _ => return Err("El informe contiene más de un encabezado de carga.".into()),
    };
    if *normal != esperado {
        return Err(format!(
            "Encabezado de carga inesperado; esperado: {}; recibido: {}.",
            tupla(&encabezados()),
            tupla(fila)
        ));
    }
    if grilla.iter().take(*indice).any(|f| es_registro(f)) {
        return Err("Hay registros antes del encabezado de carga esperado.".into());
    }
    Ok(())
}

fn contar(filas: Vec<Vec<String>>) -> FilasCarga {
    let mut salida = FilasCarga::new();
    for f in filas {
        *salida.entry(f).or_insert(0) += 1;
    }
    salida
}

/// `records_in_grid`: filas del XLS de Carga, tras validar el esquema.
pub fn registros_en_grilla(grilla: &[Vec<String>]) -> Result<FilasCarga, String> {
    validar_esquema(grilla)?;
    Ok(contar(filas_sin_validar(grilla)?))
}

/// `result_records`: filas de la tabla web de resultados; más de un listado con registros es un error.
pub fn registros_resultado(tablas: &[Vec<Vec<String>>]) -> Result<FilasCarga, String> {
    let mut candidatas = Vec::new();
    for tabla in tablas {
        let filas = filas_sin_validar(tabla)?;
        if !filas.is_empty() {
            candidatas.push(filas);
        }
    }
    match candidatas.len() {
        0 => Ok(FilasCarga::new()),
        1 => Ok(contar(candidatas.pop().unwrap())),
        _ => Err("Más de un listado de carga; revisar la estructura.".into()),
    }
}
