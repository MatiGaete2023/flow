//! Preferencias locales del Descargador (`descargador/preferencias.py`): sin filtros privados ni autenticación.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Pausa entre consultas a SITFA (`PAUSA_MINIMA_S`, `PAUSA_DEFECTO_S`, `PAUSA_MAXIMA_S`).
pub const PAUSA_MINIMA_S: f64 = 0.5;
pub const PAUSA_DEFECTO_S: f64 = 1.0;
pub const PAUSA_MAXIMA_S: f64 = 10.0;

/// `clamp_pause`: nunca menos de 0,5 s ni más de 10 s.
pub fn limitar_pausa(valor: f64) -> f64 {
    if valor.is_nan() {
        return PAUSA_DEFECTO_S;
    }
    valor.clamp(PAUSA_MINIMA_S, PAUSA_MAXIMA_S)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferencias {
    pub destino: String,
    pub pdf_sin_registros: bool,
    pub pausa_consultas: f64,
}

impl Default for Preferencias {
    fn default() -> Self {
        Self { destino: String::new(), pdf_sin_registros: true, pausa_consultas: PAUSA_DEFECTO_S }
    }
}

impl Preferencias {
    /// `%LOCALAPPDATA%\FlowRust\descargador_preferencias.json`.
    pub fn ruta() -> PathBuf {
        super::extension::carpeta_base().join("descargador_preferencias.json")
    }

    /// Lee las preferencias; un archivo ilegible o con tipos inválidos vuelve a los valores por omisión.
    pub fn cargar(ruta: &Path) -> Self {
        let mut p: Self = std::fs::read(ruta)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        p.pausa_consultas = limitar_pausa(p.pausa_consultas);
        p
    }

    /// Escritura atómica: archivo temporal y reemplazo.
    pub fn guardar(&self, ruta: &Path) -> Result<(), String> {
        if let Some(carpeta) = ruta.parent() {
            std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
        }
        let datos = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let temporal = ruta.with_extension("tmp");
        std::fs::write(&temporal, datos).map_err(|e| e.to_string())?;
        std::fs::rename(&temporal, ruta).map_err(|e| e.to_string())
    }
}
