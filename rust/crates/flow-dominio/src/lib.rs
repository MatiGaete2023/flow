//! Tipos y utilidades de texto y fecha compartidos por todo Flow.
//! Sin E/S. La lógica replica `src/nurus/personal/motor/utilidades.py` y `src/nurus/rus/columns.py`.

pub mod celda;
pub mod historia;
pub mod texto;

pub use celda::{Celda, Registro};
pub use chrono::NaiveDate as Fecha;
pub use historia::{Historia, Observacion, Tramite};

use serde::{Deserialize, Serialize};

/// CC obligatorio de todo correo. No es configurable.
pub const CC_FIJO: &str = "ucc_concepcion@pjud.cl";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Etapa {
    Espera,
    Cumplimiento,
    Informes,
}

impl Etapa {
    pub fn nombre(self) -> &'static str {
        match self {
            Etapa::Espera => "ESPERA",
            Etapa::Cumplimiento => "CUMPLIMIENTO",
            Etapa::Informes => "INFORMES",
        }
    }
    pub fn desde(texto: &str) -> Option<Self> {
        match texto::normalizar(texto).as_str() {
            "espera" => Some(Etapa::Espera),
            "cumplimiento" => Some(Etapa::Cumplimiento),
            "informes" | "informe" => Some(Etapa::Informes),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Tribunal {
    Laja,
    Mulchen,
    Tome,
}

impl Tribunal {
    pub fn nombre(self) -> &'static str {
        match self {
            Tribunal::Laja => "LAJA",
            Tribunal::Mulchen => "MULCHEN",
            Tribunal::Tome => "TOME",
        }
    }
    pub fn titulo(self) -> &'static str {
        match self {
            Tribunal::Laja => "Laja",
            Tribunal::Mulchen => "Mulchén",
            Tribunal::Tome => "Tomé",
        }
    }
    /// Reconoce el tribunal dentro del texto de la celda (igual que `rus/util.py::tribunal`).
    pub fn desde(texto: &str) -> Option<Self> {
        let t = texto::normalizar_columna(texto).to_uppercase();
        if t.contains("MULCHEN") {
            Some(Tribunal::Mulchen)
        } else if t.contains("LAJA") {
            Some(Tribunal::Laja)
        } else if t.contains("TOME") {
            Some(Tribunal::Tome)
        } else {
            None
        }
    }
}
