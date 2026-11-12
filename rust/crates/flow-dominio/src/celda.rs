//! Valor de una celda leída de RUS y registro (fila) con sus campos lógicos.

use crate::texto;
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "t", content = "v", rename_all = "lowercase")]
pub enum Celda {
    #[default]
    Vacia,
    Texto(String),
    Numero(f64),
    Fecha(NaiveDate),
}

fn serial_excel(valor: f64) -> Option<NaiveDate> {
    if valor > 0.0 && valor < 100_000.0 {
        NaiveDate::from_ymd_opt(1899, 12, 30).map(|base| base + Duration::days(valor.floor() as i64))
    } else {
        None
    }
}

/// Fecha desde texto: `aaaa-mm-dd` (con hora opcional), `dd/mm/aaaa` o `dd-mm-aaaa`. Otro formato es ilegible.
pub fn fecha_de_texto(texto: &str) -> Option<NaiveDate> {
    let s = texto.trim();
    if texto::es_vacio(s) {
        return None;
    }
    let dia = s.split([' ', 'T']).next().unwrap_or(s);
    let partes: Vec<&str> = dia.split(['-', '/']).collect();
    if partes.len() != 3 || partes.iter().any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let n: Vec<u32> = partes.iter().map(|p| p.parse().unwrap_or(0)).collect();
    if partes[0].len() == 4 && dia.contains('-') {
        NaiveDate::from_ymd_opt(n[0] as i32, n[1], n[2])
    } else if partes[2].len() == 4 && dia.len() == s.len() {
        NaiveDate::from_ymd_opt(n[2] as i32, n[1], n[0])
    } else {
        None
    }
}

impl Celda {
    pub fn es_vacia(&self) -> bool {
        match self {
            Celda::Vacia => true,
            Celda::Texto(t) => t.trim().is_empty(),
            _ => false,
        }
    }

    /// Texto tal como se mostraría (sin recortar espacios internos).
    pub fn texto(&self) -> String {
        match self {
            Celda::Vacia => String::new(),
            Celda::Texto(t) => t.clone(),
            Celda::Numero(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", *n as i64),
            Celda::Numero(n) => n.to_string(),
            Celda::Fecha(f) => f.format("%d/%m/%Y").to_string(),
        }
    }

    pub fn fecha(&self) -> Option<NaiveDate> {
        match self {
            Celda::Vacia => None,
            Celda::Fecha(f) => Some(*f),
            Celda::Numero(n) => serial_excel(*n),
            Celda::Texto(t) => fecha_de_texto(t),
        }
    }

    /// Entero exacto; un decimal, un texto no numérico o un vacío no son enteros.
    pub fn entero(&self) -> Option<i64> {
        let numero = match self {
            Celda::Numero(n) => *n,
            Celda::Texto(t) => {
                let s = t.trim().to_lowercase();
                if texto::VALORES_VACIOS.contains(&s.as_str()) {
                    return None;
                }
                s.parse::<f64>().ok()?
            }
            _ => return None,
        };
        (numero.is_finite() && numero.fract() == 0.0).then_some(numero as i64)
    }

    /// Como `entero`, aceptando coma decimal («12,0»).
    pub fn entero_laxo(&self) -> Option<i64> {
        match self {
            Celda::Texto(t) => Celda::Texto(t.replace(',', ".")).entero(),
            otra => otra.entero(),
        }
    }

    /// Una o varias fechas separadas por coma, punto y coma o salto de línea. Devuelve (legibles, ilegibles).
    pub fn fechas_varias(&self) -> (Vec<NaiveDate>, usize) {
        if let Some(unica) = self.fecha() {
            return (vec![unica], 0);
        }
        let Celda::Texto(t) = self else { return (Vec::new(), 0) };
        if texto::es_vacio(t) {
            return (Vec::new(), 0);
        }
        let mut fechas = Vec::new();
        let mut ilegibles = 0;
        for parte in t.trim().split([',', ';', '\n']) {
            let parte = parte.trim();
            if texto::es_vacio(parte) {
                continue;
            }
            match fecha_de_texto(parte) {
                Some(f) => fechas.push(f),
                None => ilegibles += 1,
            }
        }
        (fechas, ilegibles)
    }
}

/// Fila de RUS por campo lógico. Un campo ausente significa que el libro no trae esa columna.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Registro {
    pub campos: BTreeMap<String, Celda>,
}

impl Registro {
    pub fn tiene(&self, campo: &str) -> bool {
        self.campos.contains_key(campo)
    }
    pub fn celda(&self, campo: &str) -> Option<&Celda> {
        self.campos.get(campo)
    }
    pub fn texto(&self, campo: &str) -> String {
        self.campos.get(campo).map(Celda::texto).unwrap_or_default()
    }
    pub fn fecha(&self, campo: &str) -> Option<NaiveDate> {
        self.campos.get(campo).and_then(Celda::fecha)
    }
    pub fn entero(&self, campo: &str) -> Option<i64> {
        self.campos.get(campo).and_then(Celda::entero)
    }
    pub fn poner(&mut self, campo: &str, celda: Celda) {
        self.campos.insert(campo.to_string(), celda);
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn f(a: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, d).unwrap()
    }

    #[test]
    fn fechas_de_texto() {
        assert_eq!(fecha_de_texto("2026-01-05"), Some(f(2026, 1, 5)));
        assert_eq!(fecha_de_texto("2026-01-05 00:00:00"), Some(f(2026, 1, 5)));
        assert_eq!(fecha_de_texto("5/1/2026"), Some(f(2026, 1, 5)));
        assert_eq!(fecha_de_texto("05-01-2026"), Some(f(2026, 1, 5)));
        assert_eq!(fecha_de_texto("31/02/2026"), None);
        assert_eq!(fecha_de_texto("---"), None);
        assert_eq!(fecha_de_texto("pronto"), None);
        assert_eq!(Celda::Numero(46027.0).fecha(), Some(f(2026, 1, 5)));
    }

    #[test]
    fn enteros_y_varias() {
        assert_eq!(Celda::Texto(" 12.0 ".into()).entero(), Some(12));
        assert_eq!(Celda::Texto("12,0".into()).entero(), None);
        assert_eq!(Celda::Texto("12,0".into()).entero_laxo(), Some(12));
        assert_eq!(Celda::Numero(1.5).entero(), None);
        let (ok, mal) = Celda::Texto("05/01/2026, x; 06/01/2026".into()).fechas_varias();
        assert_eq!((ok.len(), mal), (2, 1));
    }
}
