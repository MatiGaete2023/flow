//! Descarga conjunta CSMP: selección y recuperación por fase.

use chrono::{Datelike, NaiveDate};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use super::{lote, paquete};

pub const FLOW_DONE: &str = "PREPARADO_SIN_CONSULTA_FIRMAS";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FasePlan {
    pub clave: String,
    pub modo: String,
    pub mes: Option<(u32, i32)>,
}

/// Equivalente de `months`: mes de corte y el siguiente.
pub fn meses(hoy: NaiveDate) -> [(u32, i32); 2] {
    let siguiente = if hoy.month() == 12 { (1, hoy.year() + 1) } else { (hoy.month() + 1, hoy.year()) };
    [(hoy.month(), hoy.year()), siguiente]
}

/// Valida el alcance y conserva el orden de las fases de Python.
pub fn plan(tribunales: &[String], modalidades: &[String], modo: &str, hoy: NaiveDate, dias: i32) -> Result<Vec<FasePlan>, String> {
    if !matches!(modo, "Espera" | "Cumplimiento") {
        return Err("El flujo conjunto admite Espera o Cumplimiento.".into());
    }
    if !(60..=730).contains(&dias) {
        return Err("Metadato legado de firmas fuera de rango (60–730).".into());
    }
    if tribunales.is_empty() || modalidades.is_empty() {
        return Err("Selecciona tribunales y modalidades.".into());
    }
    let mut fases = vec![FasePlan { clave: "principal".into(), modo: modo.into(), mes: None }];
    fases.extend(meses(hoy).into_iter().map(|(mes, anio)| FasePlan {
        clave: format!("informes-{anio}-{mes}"), modo: "calendario_informes".into(), mes: Some((mes, anio)),
    }));
    Ok(fases)
}

fn clave_seleccion(seleccion: &Value) -> String {
    let campos = ["tribunal", "tab", "modality", "report", "screen", "state", "month", "year", "start", "end"];
    serde_json::to_string(&campos.map(|campo| seleccion.get(campo).cloned().unwrap_or(Value::Null))).unwrap_or_default()
}

/// Reúne consultas completas sin modificar sus lotes de origen.
pub fn reunir_fase(lotes: &[paquete::Lote], esperadas: &[Value], destino: &Path) -> Result<PathBuf, String> {
    let esperado: BTreeSet<_> = esperadas.iter().map(clave_seleccion).collect();
    let nombre = format!("Reunido {}", chrono::Local::now().format("%Y%m%d_%H%M%S_%6f"));
    let carpeta = destino.join(nombre);
    std::fs::create_dir_all(&carpeta).map_err(|e| e.to_string())?;
    let mut vistas = BTreeSet::new();
    let mut consultas = Vec::new();
    let mut plan = Vec::new();
    for lote_origen in lotes {
        for (consulta, manifiesto) in &lote_origen.consultas {
            if !matches!(consulta["estado"].as_str(), Some("VALIDADA" | "SIN_RESULTADOS")) { continue; }
            let seleccion = consulta["seleccion"].clone();
            let clave = clave_seleccion(&seleccion);
            if !esperado.contains(&clave) || !vistas.insert(clave) {
                return Err("Consulta duplicada o fuera del alcance al recuperar la fase.".into());
            }
            let indice = consultas.len() + 1;
            let mut control = manifiesto.clone();
            if let Some(archivos) = control["archivos"].as_array_mut() {
                for item in archivos {
                    if item["estado"] != "OK" { return Err("Una consulta completa contiene un archivo pendiente.".into()); }
                    copiar_original(&lote_origen.carpeta, &carpeta, indice, item)?;
                }
            }
            if control.get("evidencia").is_some_and(|v| !v.is_null()) {
                copiar_original(&lote_origen.carpeta, &carpeta, indice, &mut control["evidencia"])?;
            }
            let archivo = format!("verificacion {indice:03}.json");
            escribir_json(&carpeta.join(&archivo), &control)?;
            let mut consulta = consulta.clone();
            consulta["verificacion"] = json!(archivo);
            consultas.push(consulta);
            plan.push(seleccion);
        }
    }
    if vistas != esperado { return Err("La recuperación no contiene todas las consultas de la fase.".into()); }
    escribir_json(&carpeta.join("resumen.json"), &json!({
        "version_contrato": 1, "estado": "VALIDADA", "consultas": consultas, "plan": plan,
        "consultas_esperadas": esperado.len(), "consultas_completadas": esperado.len(),
        "otros_filtros": "todos los registros", "filtro_fecha": null,
        "fecha_hora": chrono::Local::now().naive_local().to_string(),
    }))?;
    paquete::cargar_lote(&carpeta)?;
    Ok(carpeta)
}

fn copiar_original(origen: &Path, destino: &Path, indice: usize, item: &mut Value) -> Result<(), String> {
    let bytes = paquete::comprobar(origen, item)?;
    let nombre = item["archivo"].as_str().ok_or("El manifiesto contiene una ruta fuera del lote.")?;
    let nuevo = format!("{indice:03}-{nombre}");
    lote::guardar_exclusivo(&destino.join(&nuevo), &bytes)?;
    item["archivo"] = json!(nuevo);
    Ok(())
}

fn escribir_json(ruta: &Path, valor: &Value) -> Result<(), String> {
    let mut datos = serde_json::to_vec_pretty(valor).map_err(|e| e.to_string())?;
    datos.push(b'\n');
    lote::guardar_exclusivo(ruta, &datos)
}
