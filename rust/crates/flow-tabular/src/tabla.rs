//! De un libro a la tabla de una etapa: fila de encabezado, mapeo de columnas, registros y hoja de cruce.

use crate::columnas::{self, CRUCE};
use crate::libro::{Hoja, Libro};
use flow_dominio::texto::{identidad, normalizar_columna};
use flow_dominio::{Celda, Etapa, Fecha, Registro};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const FILAS_A_REVISAR: usize = 30;
const ETAPAS: [Etapa; 3] = [Etapa::Espera, Etapa::Cumplimiento, Etapa::Informes];

#[derive(Debug, Clone, PartialEq)]
pub struct Fila {
    /// Identificador estable: depende del libro, la hoja y el número de fila.
    pub id: String,
    /// Número de fila en el libro (la primera es 1).
    pub numero: usize,
    pub celdas: Vec<Celda>,
    pub registro: Registro,
}

#[derive(Debug, Clone)]
pub struct Tabla {
    pub etapa: Etapa,
    pub hoja: String,
    pub fila_cabecera: usize,
    pub cabeceras: Vec<String>,
    pub mapeo: BTreeMap<String, usize>,
    pub filas: Vec<Fila>,
    /// Hoja de cruce de Cumplimiento: (rit, rut, nombre, tribunal, programa) → vencimiento.
    pub cruce: Vec<([String; 5], Fecha)>,
    pub avisos: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidata {
    pub hoja: String,
    pub fila_cabecera: usize,
    pub etapas: Vec<Etapa>,
}

fn cabeceras_de(fila: &[Celda]) -> Vec<String> {
    fila.iter().map(|c| c.texto().trim().to_string()).collect()
}

fn hoja_ignorada(nombre: &str) -> bool {
    matches!(normalizar_columna(nombre).trim(), "ob" | "medidas vencidas")
}

/// Hojas y filas que parecen el encabezado de una tabla de Espera, Cumplimiento o Informes.
pub fn candidatas(libro: &Libro) -> Vec<Candidata> {
    let mut salida = Vec::new();
    for hoja in libro.hojas.iter().filter(|h| !hoja_ignorada(&h.nombre)) {
        for (i, fila) in hoja.filas.iter().take(FILAS_A_REVISAR).enumerate() {
            let cabeceras = cabeceras_de(fila);
            if columnas::duplicadas(&cabeceras).is_some() {
                continue;
            }
            let etapas: Vec<Etapa> = ETAPAS
                .into_iter()
                .filter(|e| columnas::mapear(&cabeceras, *e).is_ok_and(|m| columnas::es_tabla_de(&m, *e)))
                .collect();
            if !etapas.is_empty() {
                salida.push(Candidata { hoja: hoja.nombre.clone(), fila_cabecera: i + 1, etapas });
            }
        }
    }
    salida
}

/// Etapa que corresponde a una candidata cuando no se pidió una: por el nombre de la hoja, por ser la única
/// posible o por el nombre del archivo.
fn etapa_natural(c: &Candidata, archivo: &str) -> Option<Etapa> {
    if let Some(e) = Etapa::desde(&c.hoja).filter(|e| c.etapas.contains(e)) {
        return Some(e);
    }
    if c.etapas.len() == 1 {
        return Some(c.etapas[0]);
    }
    let nombre = normalizar_columna(archivo);
    let en_nombre: Vec<Etapa> =
        c.etapas.iter().copied().filter(|e| nombre.contains(&e.nombre().to_lowercase())).collect();
    (en_nombre.len() == 1).then(|| en_nombre[0])
}

fn id_fila(libro: &Libro, hoja: &str, numero: usize) -> String {
    let crudo = format!("{}|{}|{}", libro.sha256, hoja, numero);
    Sha256::digest(crudo.as_bytes()).iter().take(12).map(|b| format!("{b:02x}")).collect()
}

fn filas_de(libro: &Libro, hoja: &Hoja, fila_cabecera: usize, mapeo: &BTreeMap<String, usize>) -> Vec<Fila> {
    let mut filas = Vec::new();
    for (i, celdas) in hoja.filas.iter().enumerate().skip(fila_cabecera) {
        // Totales, pies y notas no son registros: un registro tiene dato en al menos dos columnas reconocidas.
        let con_dato = mapeo
            .values()
            .filter(|col| celdas.get(**col).is_some_and(|c| !flow_dominio::texto::es_vacio(&c.texto())))
            .count();
        if con_dato < 2 {
            continue;
        }
        let mut registro = Registro::default();
        for (campo, col) in mapeo {
            registro.poner(campo, celdas.get(*col).cloned().unwrap_or_default());
        }
        filas.push(Fila { id: id_fila(libro, &hoja.nombre, i + 1), numero: i + 1, celdas: celdas.clone(), registro });
    }
    filas
}

/// Hoja de cruce: otra hoja con las seis columnas de identidad y vencimiento. Si hay más de una, no se usa ninguna.
fn cruce(libro: &Libro, principal: &str, avisos: &mut Vec<String>) -> Vec<([String; 5], Fecha)> {
    let mut encontradas: Vec<(&Hoja, usize, BTreeMap<String, usize>)> = Vec::new();
    for hoja in libro.hojas.iter().filter(|h| h.nombre != principal && !hoja_ignorada(&h.nombre)) {
        for (i, fila) in hoja.filas.iter().take(FILAS_A_REVISAR).enumerate() {
            let cabeceras = cabeceras_de(fila);
            if columnas::duplicadas(&cabeceras).is_some() {
                continue;
            }
            if let Ok(mapeo) = columnas::mapear_cruce(&cabeceras) {
                if mapeo.len() == CRUCE.len() {
                    encontradas.push((hoja, i + 1, mapeo));
                }
            }
        }
    }
    if encontradas.len() != 1 {
        avisos.push(if encontradas.is_empty() {
            "Sin hoja de cruce: no se evalúa el próximo informe (C-10). El resto del análisis continúa.".to_string()
        } else {
            "Hay más de una hoja de cruce posible: no se usa ninguna. Deja una sola o corrígela.".to_string()
        });
        return Vec::new();
    }
    let (hoja, fila_cabecera, mapeo) = &encontradas[0];
    let mut salida = Vec::new();
    for celdas in hoja.filas.iter().skip(*fila_cabecera) {
        let texto = |campo: &str| celdas.get(mapeo[campo]).map(Celda::texto).unwrap_or_default();
        if let Some(vence) = celdas.get(mapeo["vencimiento"]).and_then(Celda::fecha) {
            let clave = ["rit", "rut", "nombre", "tribunal", "programa"].map(|c| identidad(&texto(c)));
            salida.push((clave, vence));
        }
    }
    salida
}

fn construir(libro: &Libro, c: &Candidata, etapa: Etapa) -> Result<Tabla, String> {
    let hoja = libro.hojas.iter().find(|h| h.nombre == c.hoja).ok_or("Hoja no encontrada.")?;
    let cabeceras = cabeceras_de(&hoja.filas[c.fila_cabecera - 1]);
    let mapeo = columnas::mapear(&cabeceras, etapa)?;
    let mut avisos = Vec::new();
    if c.fila_cabecera != 1 {
        avisos.push(format!("Se usó la fila {} como encabezado de «{}».", c.fila_cabecera, c.hoja));
    }
    let cruce = if etapa == Etapa::Cumplimiento { cruce(libro, &c.hoja, &mut avisos) } else { Vec::new() };
    Ok(Tabla {
        etapa,
        hoja: c.hoja.clone(),
        fila_cabecera: c.fila_cabecera,
        filas: filas_de(libro, hoja, c.fila_cabecera, &mapeo),
        cabeceras,
        mapeo,
        cruce,
        avisos,
    })
}

/// Lee la tabla de una etapa. Sin `etapa`, se deduce; si no es inequívoca, se pide elegir.
pub fn leer_tabla(libro: &Libro, etapa: Option<Etapa>, hoja: Option<&str>) -> Result<Tabla, String> {
    let mut todas = candidatas(libro);
    if let Some(pedida) = hoja {
        todas.retain(|c| c.hoja.trim().eq_ignore_ascii_case(pedida.trim()));
        if todas.is_empty() {
            return Err(format!("La hoja «{pedida}» no existe o no contiene una tabla de RUS reconocible."));
        }
    }
    if todas.is_empty() {
        return Err("No se identifica una tabla de Espera, Cumplimiento o Informes en el archivo. \
                    Revisa sus encabezados."
            .into());
    }
    let mut opciones: Vec<(&Candidata, Etapa)> = match etapa {
        Some(e) => todas.iter().filter(|c| c.etapas.contains(&e)).map(|c| (c, e)).collect(),
        None => todas.iter().filter_map(|c| etapa_natural(c, &libro.nombre).map(|e| (c, e))).collect(),
    };
    if let Some(e) = etapa {
        if opciones.is_empty() {
            let hay: Vec<String> =
                todas.iter().flat_map(|c| c.etapas.iter().map(move |x| format!("{} ({})", x.nombre(), c.hoja))).collect();
            return Err(format!("No hay una tabla compatible con {}. Se detectaron: {}.", e.nombre(), hay.join(", ")));
        }
        // Varias hojas compatibles: gana la que se llama como la etapa.
        if opciones.len() > 1 && opciones.iter().any(|(c, _)| Etapa::desde(&c.hoja) == Some(e)) {
            opciones.retain(|(c, _)| Etapa::desde(&c.hoja) == Some(e));
        }
    }
    match opciones.len() {
        1 => construir(libro, opciones[0].0, opciones[0].1),
        0 => Err("La tabla sirve para más de una etapa y el archivo no indica cuál. Elige la etapa.".into()),
        _ => {
            let lista: Vec<String> =
                opciones.iter().map(|(c, e)| format!("{} en «{}» (fila {})", e.nombre(), c.hoja, c.fila_cabecera)).collect();
            Err(format!("El archivo tiene más de una tabla posible: {}. Elige etapa u hoja.", lista.join("; ")))
        }
    }
}

/// Todas las tablas inequívocas del libro (un consolidado trae una hoja por etapa).
pub fn leer_todas(libro: &Libro) -> Vec<Tabla> {
    let mut salida: Vec<Tabla> = Vec::new();
    for c in candidatas(libro) {
        if let Some(e) = etapa_natural(&c, &libro.nombre) {
            if !salida.iter().any(|t| t.etapa == e) {
                if let Ok(t) = construir(libro, &c, e) {
                    salida.push(t);
                }
            }
        }
    }
    salida
}

/// Fuentes auxiliares conocidas. Conservan sus columnas, sin fingir que satisfacen las de una etapa del motor.
pub fn leer_consulta(libro: &Libro, tipo: &str) -> Result<Tabla, String> {
    if !matches!(tipo, "EGRESADOS" | "BUSQUEDA_LITIGANTES") {
        return Err("Tipo de consulta auxiliar no permitido.".into());
    }
    let mut candidatas = vec![];
    for hoja in &libro.hojas {
        for (n, fila) in hoja.filas.iter().take(FILAS_A_REVISAR).enumerate() {
            let cabeceras = cabeceras_de(fila);
            if columnas::duplicadas(&cabeceras).is_some() {
                continue;
            }
            let Ok(mapeo) = columnas::mapear(&cabeceras, Etapa::Espera) else {
                continue;
            };
            if mapeo.contains_key("nombre") && (mapeo.contains_key("rit") || mapeo.contains_key("rut")) {
                candidatas.push((hoja, n + 1, cabeceras, mapeo));
            }
        }
    }
    if candidatas.len() != 1 {
        return Err("La consulta auxiliar no contiene una tabla única de personas reconocible.".into());
    }
    let (hoja, fila_cabecera, cabeceras, mapeo) = candidatas.remove(0);
    let filas = filas_de(libro, hoja, fila_cabecera, &mapeo);
    Ok(Tabla { etapa: Etapa::Espera, hoja: tipo.into(), fila_cabecera, cabeceras, mapeo, filas, cruce: vec![], avisos: vec![] })
}

impl Tabla {
    /// Próximo informe según el cruce: última fila con vencimiento posterior al corte para la identidad completa.
    pub fn proximo_informe(&self, registro: &Registro, corte: Fecha) -> Option<Fecha> {
        let clave = ["rit", "rut", "nombre", "tribunal", "programa"].map(|c| identidad(&registro.texto(c)));
        self.cruce.iter().rev().find(|(k, vence)| *k == clave && *vence > corte).map(|(_, v)| *v)
    }
}
