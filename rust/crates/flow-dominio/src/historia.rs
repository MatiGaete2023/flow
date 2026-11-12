//! Historia de la causa y bitácora: solo datos y búsquedas, sin reglas (`motor/historia.py`).

use crate::texto::clave;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tramite {
    pub desc: String,
    #[serde(default)]
    pub clave: String,
    pub fecha: NaiveDate,
    #[serde(default)]
    pub folio_txt: String,
    /// Folio numérico, o -1 si no es un número.
    #[serde(default = "sin_folio")]
    pub folio: i64,
}

fn sin_folio() -> i64 {
    -1
}

impl Tramite {
    pub fn nuevo(desc: &str, fecha: NaiveDate, folio: &str) -> Self {
        let folio_txt = folio.trim().to_string();
        let numero = if !folio_txt.is_empty() && folio_txt.bytes().all(|b| b.is_ascii_digit()) {
            folio_txt.parse().unwrap_or(-1)
        } else {
            -1
        };
        Tramite { desc: desc.trim().to_string(), clave: clave(desc), fecha, folio_txt, folio: numero }
    }
    pub fn id(&self) -> (String, NaiveDate, String) {
        (self.clave.clone(), self.fecha, self.folio_txt.clone())
    }
    pub fn folio_visible(&self) -> &str {
        if self.folio_txt.is_empty() {
            "s/f"
        } else {
            &self.folio_txt
        }
    }
    /// `self` es posterior a `otro`: por folio si ambos son numéricos; si no, por fecha.
    pub fn posterior_a(&self, otro: &Tramite) -> bool {
        if self.folio >= 0 && otro.folio >= 0 {
            self.folio > otro.folio
        } else {
            self.fecha > otro.fecha
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observacion {
    pub fecha: NaiveDate,
    pub texto: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Historia {
    /// COMPLETA, PARCIAL, NO_CONSULTADA, FALLIDA…
    pub cobertura: String,
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
    pub tramites: Vec<Tramite>,
}

impl Historia {
    pub fn nueva(cobertura: &str, desde: Option<NaiveDate>, hasta: Option<NaiveDate>, mut tramites: Vec<Tramite>) -> Self {
        tramites.retain(|t| !t.desc.is_empty());
        tramites.sort_by_key(|t| (t.fecha, t.folio));
        Historia { cobertura: cobertura.to_string(), desde, hasta, tramites }
    }

    pub fn con_datos(&self) -> bool {
        matches!(self.cobertura.as_str(), "COMPLETA" | "PARCIAL")
    }

    pub fn buscar(&self, nombres: &[&str]) -> Vec<&Tramite> {
        let claves: Vec<String> = nombres.iter().map(|n| clave(n)).collect();
        self.tramites.iter().filter(|t| claves.contains(&t.clave)).collect()
    }

    /// Vista al corte: nunca acredita ausencia antes de su ventana ni usa trámites posteriores al corte.
    pub fn al_corte(&self, corte: NaiveDate) -> Historia {
        if self.desde.is_some_and(|d| corte < d) {
            return Historia { cobertura: "NO_CONSULTADA".into(), desde: None, hasta: None, tramites: Vec::new() };
        }
        Historia {
            cobertura: self.cobertura.clone(),
            desde: self.desde,
            hasta: self.hasta.map(|h| h.min(corte)),
            tramites: self.tramites.iter().filter(|t| t.fecha <= corte).cloned().collect(),
        }
    }
}

/// Último trámite por (fecha, folio); ante empate, el primero de la lista.
pub fn ultimo<'a>(tramites: &[&'a Tramite]) -> Option<&'a Tramite> {
    let mut mejor: Option<&Tramite> = None;
    for t in tramites {
        if mejor.is_none_or(|m| (t.fecha, t.folio) > (m.fecha, m.folio)) {
            mejor = Some(t);
        }
    }
    mejor
}
