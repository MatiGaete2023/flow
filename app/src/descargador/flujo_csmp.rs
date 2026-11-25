//! Descarga conjunta CSMP: selección y recuperación por fase.

use chrono::{Datelike, NaiveDate};

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
