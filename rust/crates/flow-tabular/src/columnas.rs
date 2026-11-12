//! Nombres de columna de RUS y su campo lógico (`src/nurus/rus/columns.py`).
//! Una columna ambigua es un error: nunca se adivina.

use flow_dominio::texto::normalizar_columna;
use flow_dominio::Etapa;
use std::collections::BTreeMap;

type Alias = (&'static str, &'static [&'static str]);

const VENCIMIENTO: &[&str] = &[
    "FECHA VENCIMIENTO",
    "FEC.VENCIMIENTO",
    "FEC. VENCIMIENTO",
    "F. VENCIMIENTO",
    "F.VENCIMIENTO",
    "FVENCIMIENTO",
    "FECHA DE VENCIMIENTO",
];
const INGRESO: &[&str] = &["FEC.INGRESO EFECTIVO", "FEC. INGRESO EFECTIVO", "FEC INGRESO EFECTIVO"];

const COMUNES: &[Alias] = &[
    ("programa", &["DERIVACION", "DERIVACIÓN", "PROGRAMA", "NOMBRE CENTRO"]),
    ("tribunal", &["TRIBUNAL"]),
    ("nombre", &["NOMBRE", "NOMBRE COMPLETO", "NOMBRE MENOR"]),
    ("nacimiento", &["FEC. NACIMIENTO", "FEC.NACIMIENTO", "FECHA NACIMIENTO", "FEC NACIMIENTO"]),
    (
        "curador",
        &[
            "CURADOR",
            "CURADOR AD LITEM",
            "CURADOR AD-LITEM",
            "CURADOR AD LITEM.",
            "CUR. AD LITEM",
            "CURADOR/A AD LITEM",
            "CURADOR/A AD-LITEM",
        ],
    ),
    ("oido", &["FEC. OIDO", "FEC.OIDO", "FEC OIDO", "FECHAOIDO", "FEC. OÍDO", "FEC.OÍDO"]),
    (
        "resolucion",
        &["FEC. RESOLUCIÓN", "FEC.RESOLUCIÓN", "FEC. RESOLUCION", "FEC.RESOLUCION", "FECHA RESOLUCION", "FEC RESOLUCION"],
    ),
    (
        "prox_aud",
        &["PROXS. AUDS.", "PROXS AUDS", "PROX AUD", "PROXIMA AUDIENCIA", "PRÓXIMA AUDIENCIA", "PROX. AUD.", "PROX. AUDS."],
    ),
    ("rit", &["RIT"]),
    ("rut", &["RUT", "RUT MENOR", "RUT NNA", "RUT LITIGANTE"]),
];

const ESPERA: &[Alias] =
    &[("espera", &["T ESPERA", "T_ESPERA", "DIAS_ESPERA", "TESPERA", "DÍAS DE ESPERA", "DIAS DE ESPERA"])];

const CUMPLIMIENTO: &[Alias] = &[
    ("dias_cumpl", &["DIAS DE CUMPLIMIENTO", "DÍAS DE CUMPLIMIENTO", "DIAS CUMPLIMIENTO"]),
    ("dias_egresar", &["DIAS PARA EGRESAR", "DÍAS PARA EGRESAR", "DIAS EGRESAR"]),
    ("ingreso", INGRESO),
    ("egreso_proy", &["FEC.EGRESO PROYECTADO", "FEC. EGRESO PROYECTADO", "FEC EGRESO PROYECTADO"]),
    (
        "ficha_fae",
        &["FEC.ACT.F.FAE", "FEC. ACT. F. FAE", "FEC ACT F FAE", "FEC.ACT.F.FAE/FAS", "FEC. ACT. F. FAE/FAS"],
    ),
    ("ficha_ind", &["FEC.ACT.F.INDIVIDUAL", "FEC. ACT. F. INDIVIDUAL", "FEC ACT F INDIVIDUAL"]),
];

// FECHA INGRESO y FEC. INGRESO EFECTIVO son datos distintos en RUS: no deben colisionar.
const INFORMES: &[Alias] = &[("vencimiento", VENCIMIENTO), ("ingreso", INGRESO)];

/// Hoja de cruce (hoja 2) de Cumplimiento: próximo informe por identidad completa.
pub const CRUCE: &[Alias] = &[
    ("rit", &["RIT"]),
    ("rut", &["RUT MENOR", "RUT"]),
    ("nombre", &["NOMBRE MENOR", "NOMBRE"]),
    ("tribunal", &["TRIBUNAL"]),
    ("programa", &["NOMBRE CENTRO", "DERIVACION", "DERIVACIÓN"]),
    ("vencimiento", VENCIMIENTO),
];

pub fn obligatorias(etapa: Etapa) -> &'static [&'static str] {
    match etapa {
        Etapa::Espera => &["programa", "tribunal", "espera"],
        Etapa::Cumplimiento => &["programa", "tribunal", "dias_cumpl", "dias_egresar"],
        Etapa::Informes => &["programa", "tribunal", "vencimiento"],
    }
}

fn mapear_con(cabeceras: &[String], grupos: &[&[Alias]]) -> Result<BTreeMap<String, usize>, String> {
    let mut mapeo = BTreeMap::new();
    for (campo, nombres) in grupos.iter().flat_map(|g| g.iter()) {
        let alias: Vec<String> = nombres.iter().map(|n| normalizar_columna(n)).collect();
        let mut candidatas: Vec<usize> = Vec::new();
        for (i, cabecera) in cabeceras.iter().enumerate() {
            let limpia = cabecera.trim();
            if alias.contains(&normalizar_columna(limpia)) && !candidatas.iter().any(|j| cabeceras[*j].trim() == limpia) {
                candidatas.push(i);
            }
        }
        if candidatas.len() > 1 {
            let nombres: Vec<&str> = candidatas.iter().map(|i| cabeceras[*i].trim()).collect();
            return Err(format!("Columna ambigua para {campo}: {}.", nombres.join(", ")));
        }
        if let Some(i) = candidatas.first() {
            mapeo.insert(campo.to_string(), *i);
        }
    }
    Ok(mapeo)
}

/// Campo lógico → índice de columna, para una etapa.
pub fn mapear(cabeceras: &[String], etapa: Etapa) -> Result<BTreeMap<String, usize>, String> {
    let propias = match etapa {
        Etapa::Espera => ESPERA,
        Etapa::Cumplimiento => CUMPLIMIENTO,
        Etapa::Informes => INFORMES,
    };
    mapear_con(cabeceras, &[COMUNES, propias])
}

pub fn mapear_cruce(cabeceras: &[String]) -> Result<BTreeMap<String, usize>, String> {
    mapear_con(cabeceras, &[CRUCE])
}

/// El mapeo tiene las columnas obligatorias de la etapa y algún identificador de persona o causa.
pub fn es_tabla_de(mapeo: &BTreeMap<String, usize>, etapa: Etapa) -> bool {
    obligatorias(etapa).iter().all(|c| mapeo.contains_key(*c))
        && ["rit", "rut", "nombre"].iter().any(|c| mapeo.contains_key(*c))
}

/// Cabeceras duplicadas o equivalentes: la fila no sirve como encabezado.
pub fn duplicadas(cabeceras: &[String]) -> Option<String> {
    let mut vistas: Vec<String> = Vec::new();
    for cabecera in cabeceras {
        let n = normalizar_columna(cabecera.trim());
        if n.is_empty() {
            continue;
        }
        if vistas.contains(&n) {
            return Some(cabecera.trim().to_string());
        }
        vistas.push(n);
    }
    None
}

/// Nombre de columna canónico de un campo (el primero de sus alias) para una etapa, en orden estable.
pub fn canonicas(etapa: Etapa) -> Vec<(&'static str, &'static str)> {
    let propias = match etapa {
        Etapa::Espera => ESPERA,
        Etapa::Cumplimiento => CUMPLIMIENTO,
        Etapa::Informes => INFORMES,
    };
    COMUNES.iter().chain(propias.iter()).map(|(campo, alias)| (*campo, alias[0])).collect()
}
