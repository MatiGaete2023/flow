//! Órdenes de búsqueda (`ordenes_busqueda.py` de Flow Python): avisos OB-01 a OB-05 solo para mostrar, sobre las filas de
//! «Búsqueda Lit» (pantalla Litigantes). No cruzan con otras fuentes ni cambian observaciones ni acciones.

use crate::trabajo::Trabajo;
use crate::util::{clave, fecha_es, fecha_mayoria, fechas_varias, get_date, is_empty_text, Cell};
use chrono::NaiveDate;
use serde_json::Value;
use std::collections::BTreeMap;

const DIAS_SIN_RESULTADO: i64 = 60;
const COLUMNAS: [(&str, &str); 8] = [
    ("estado", "ESTADO O B"),
    ("resultado", "RESULTADO"),
    ("tramite", "FECHA TRAMITE"),
    ("nacimiento", "FEC NACIMIENTO"),
    ("envio", "ESTADO ENVIO"),
    ("tipo", "TIPO LITIGANTE"),
    ("curador", "CURADOR"),
    ("audiencias", "PROX AUDS"),
];

fn clave_columna(t: &str) -> String {
    clave(t).to_uppercase()
}

/// Avisos de una fila de Búsqueda Lit; solo órdenes en estado «Activa». Una columna ausente omite su aviso.
pub fn avisos_orden(valores: &BTreeMap<String, String>, como_de: NaiveDate) -> Vec<String> {
    let disponibles: BTreeMap<String, &String> = valores.keys().map(|k| (clave_columna(k), k)).collect();
    let dato = |nombre: &str| -> Option<&str> {
        let titulo = COLUMNAS.iter().find(|(n, _)| *n == nombre)?.1;
        disponibles.get(&clave_columna(titulo)).and_then(|k| valores.get(*k)).map(String::as_str)
    };
    let dia = |t: &str| get_date(&Cell::Text(t.to_string()));
    if dato("estado").is_none_or(|e| clave_columna(e) != "ACTIVA") {
        return Vec::new();
    }
    let mut avisos = Vec::new();
    let tramite = dato("tramite").and_then(dia);
    if let (Some(r), Some(t)) = (dato("resultado"), tramite) {
        if is_empty_text(r) && (como_de - t).num_days() > DIAS_SIN_RESULTADO {
            avisos.push(format!("OB-01: orden de búsqueda activa sin resultado desde el {} ({} días).", fecha_es(t), (como_de - t).num_days()));
        }
    }
    if let Some(nac) = dato("nacimiento").and_then(dia) {
        let mayoria = fecha_mayoria(nac);
        if mayoria <= como_de {
            avisos.push(format!("OB-02: orden de búsqueda activa y la persona cumplió 18 años el {}.", fecha_es(mayoria)));
        }
    }
    if let Some(envio) = dato("envio") {
        if clave_columna(envio) != "REALIZADA" {
            let estado = if envio.trim().is_empty() { "sin estado" } else { envio.trim() };
            avisos.push(format!("OB-03: el envío de la orden no figura como realizado ({estado})."));
        }
    }
    if let (Some(tipo), Some(curador)) = (dato("tipo"), dato("curador")) {
        if matches!(clave_columna(tipo).as_str(), "NINO" | "NINA" | "ADOLESCENTE") && is_empty_text(curador) {
            avisos.push("OB-04: orden de búsqueda activa de un niño, niña o adolescente sin curador ad litem registrado.".into());
        }
    }
    if let Some(auds) = dato("audiencias") {
        let mut futuras: Vec<NaiveDate> = fechas_varias(&Cell::Text(auds.to_string())).0.into_iter().filter(|f| *f >= como_de).collect();
        futuras.sort();
        if let Some(f) = futuras.first() {
            avisos.push(format!("OB-05: audiencia fijada para el {} con orden de búsqueda activa.", fecha_es(*f)));
        }
    }
    avisos
}

/// Una orden de búsqueda con avisos del paquete abierto.
#[derive(Clone)]
pub struct OrdenConAvisos {
    pub rit: String,
    pub nombre: String,
    pub avisos: Vec<String>,
}

/// Filas de las consultas de Litigantes del paquete que tienen algún aviso OB.
pub fn ordenes_del_paquete(w: &Trabajo, como_de: NaiveDate) -> Vec<OrdenConAvisos> {
    let Some(p) = &w.paquete else { return Vec::new() };
    let pantallas: BTreeMap<&str, &str> = p.fuente.filas("consultas").map(|q| (q.get("query_id"), q.get("screen"))).collect();
    p.fuente
        .filas("registros")
        .filter(|r| pantallas.get(r.get("query_id")) == Some(&"litigantes"))
        .filter_map(|r| {
            let datos: serde_json::Map<String, Value> = serde_json::from_str(r.get("normalized_json")).ok()?;
            let valores: BTreeMap<String, String> = datos.iter().map(|(k, v)| (k.clone(), v.as_str().map_or_else(|| v.to_string(), str::to_string))).collect();
            let avisos = avisos_orden(&valores, como_de);
            let dato = |opciones: &[&str]| opciones.iter().find_map(|o| valores.iter().find(|(k, _)| clave_columna(k) == *o).map(|(_, v)| v.clone())).unwrap_or_default();
            (!avisos.is_empty()).then(|| OrdenConAvisos { rit: dato(&["RIT"]), nombre: dato(&["NOMBRE", "NOMBRE MENOR", "NOMBRE LITIGANTE"]), avisos })
        })
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn comparar(corte: NaiveDate, caso: &Value) -> Vec<String> {
        let valores: BTreeMap<String, String> = caso["valores"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string())).collect();
        let obtenidos = avisos_orden(&valores, corte);
        let previstos: Vec<String> = caso["avisos"].as_array().unwrap().iter().map(|a| a.as_str().unwrap().to_string()).collect();
        if obtenidos == previstos { vec![] } else { vec![format!("Rust {obtenidos:?} · Python {previstos:?}")] }
    }

    /// E-A5 (F02): los avisos de órdenes de búsqueda de Rust contra `ordenes_busqueda.avisos_orden` de Flow Python.
    #[test]
    fn ordenes_iguales_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/ordenes.json")).unwrap();
        let corte = NaiveDate::parse_from_str(datos["corte"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        let casos = datos["casos"].as_array().unwrap();
        let fallos: Vec<String> = casos.iter().filter_map(|c| {
            let d = comparar(corte, c);
            (!d.is_empty()).then(|| format!("{}: {}", c["nombre"], d.join(" ")))
        }).collect();
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("ORDENES: {}/{}, fallos {}", casos.len() - fallos.len(), casos.len(), fallos.len());
        assert_eq!(casos.len(), 21, "el oráculo debe traer 21 casos");
        assert!(casos.iter().any(|c| !c["avisos"].as_array().unwrap().is_empty()), "el oráculo debe tener avisos");
        assert!(fallos.is_empty());
    }

    #[test]
    fn oraculo_de_ordenes_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/ordenes.json")).unwrap();
        let corte = NaiveDate::parse_from_str(datos["corte"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        let mut caso = datos["casos"].as_array().unwrap().iter().find(|c| c["nombre"] == "mayor_de_edad").unwrap().clone();
        assert!(comparar(corte, &caso).is_empty());
        caso["avisos"][0] = Value::String("OB-02: otro texto".into());
        assert!(!comparar(corte, &caso).is_empty(), "un aviso esperado alterado no se detecta");
    }
}
