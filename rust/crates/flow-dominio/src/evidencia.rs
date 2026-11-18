//! Evidencia de la integración 18-nov-2026: Escritos de la causa, Ingreso Histórico Protección y asociación de cada
//! informe con su proyección. Solo datos y búsquedas puras (`motor/evidencia.py`).
//!
//! Estados de fuente (del contrato de captura): COMPLETA, VACIA_COMPROBADA, PARCIAL, FALLIDA, NO_SOLICITADA,
//! NO_CONSULTADA e IDENTIDAD_INCIERTA. Solo hay filas utilizables con COMPLETA o PARCIAL.

use crate::historia::Observacion;
use crate::texto::clave;
use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

pub const ESTADO_PENDIENTE: &str = "pendiente";
const CAUSAL_VIGENTE: &str = "sin informacion";
const GUIONES: [char; 6] = ['-', '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}'];

pub fn con_filas(cobertura: &str) -> bool {
    matches!(cobertura, "COMPLETA" | "PARCIAL")
}

/// Comparación estable: sin acentos, minúsculas, espacios colapsados (también NBSP).
pub fn clave_texto(texto: &str) -> String {
    clave(&texto.replace('\u{a0}', " "))
}

/// Identidad del programa por su denominación completa: dos PRM distintos son programas distintos.
pub fn clave_programa(nombre: &str) -> String {
    clave_texto(nombre)
}

/// Fila de la tabla Escritos de la causa (valores de la misma fila, sin importar el orden de columnas).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Escrito {
    pub tipo: String,
    pub estado: String,
    #[serde(default)]
    pub doc: String,
    #[serde(default)]
    pub solicitante: String,
    #[serde(default)]
    pub fila: i64,
    /// «EXPLICITO» si el escrito se asoció a una proyección concreta; vacío si no.
    #[serde(default)]
    pub vinculo: String,
    #[serde(default)]
    pub fecha_proyectada: Option<NaiveDate>,
    #[serde(default)]
    pub periodo: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Escritos {
    pub cobertura: String,
    #[serde(default)]
    pub leido: Option<NaiveDate>,
    #[serde(default)]
    pub filas: Vec<Escrito>,
}

impl Escritos {
    pub fn con_filas(&self) -> bool {
        con_filas(&self.cobertura)
    }
}

/// Fila del Ingreso Histórico Protección de la persona.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProgramaHistorico {
    pub programa: String,
    #[serde(default)]
    pub fecha_egreso: String,
    #[serde(default)]
    pub causal_egreso: String,
    #[serde(default)]
    pub fila: i64,
}

fn es_sin_fecha(texto: &str) -> bool {
    let limpio = texto.replace('\u{a0}', " ");
    let limpio = limpio.trim();
    limpio.chars().count() == 2 && limpio.chars().all(|c| GUIONES.contains(&c))
}

impl ProgramaHistorico {
    /// Vigente solo si la fecha es «--» y la causal «SIN INFORMACION» en la misma fila. Lo ilegible o vacío no acredita.
    pub fn vigente(&self) -> bool {
        es_sin_fecha(&self.fecha_egreso) && clave_texto(&self.causal_egreso) == CAUSAL_VIGENTE
    }
}

fn verificada() -> String {
    "VERIFICADA".into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Historico {
    pub cobertura: String,
    #[serde(default)]
    pub leido: Option<NaiveDate>,
    /// VERIFICADA solo si se probó que la tabla visible es la de la persona seleccionada.
    #[serde(default = "verificada")]
    pub identidad: String,
    #[serde(default)]
    pub filas: Vec<ProgramaHistorico>,
}

impl Historico {
    pub fn utilizable(&self) -> bool {
        con_filas(&self.cobertura) && self.identidad == "VERIFICADA"
    }
}

/// Asociación explícita de un informe esperado (ingreso + fecha proyectada) con sus evidencias, al mismo corte.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EvidenciaInforme {
    #[serde(default)]
    pub escrito_historia_folio: String,
    #[serde(default)]
    pub resolucion_folio: String,
    #[serde(default)]
    pub resolucion_fecha: Option<NaiveDate>,
    /// PENDIENTE, ENVIADO o vacío.
    #[serde(default)]
    pub calendario: String,
    #[serde(default)]
    pub fecha_proyectada: Option<NaiveDate>,
}

/// Nombres (tal como se muestran) de los otros programas vigentes del mismo niño: sin repetir y sin el actual.
/// Un programa con una fila antigua egresada y otra vigente cuenta como vigente; las vigentes repetidas se colapsan.
pub fn programas_vigentes_otros(historico: Option<&Historico>, programa_actual: &str) -> Vec<String> {
    let Some(h) = historico.filter(|h| h.utilizable()) else { return Vec::new() };
    let actual = clave_programa(programa_actual);
    let mut vistos: Vec<(String, String)> = Vec::new();
    for fila in &h.filas {
        let llave = clave_programa(&fila.programa);
        if llave.is_empty() || llave == actual || !fila.vigente() || vistos.iter().any(|(k, _)| *k == llave) {
            continue;
        }
        vistos.push((llave, fila.programa.split_whitespace().collect::<Vec<_>>().join(" ")));
    }
    vistos.into_iter().map(|(_, nombre)| nombre).collect()
}

static NEGACION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:no|nunca|ni|sin)\b[^.]{0,40}$").unwrap());
static CONSTANCIA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:cuenta\s+con\s+)?ingres(?:o\s+efectivo|[oó]\s+efectivamente)\s+(?:al|a)\s+(?:programa\s+)?(.+?)\s*(?:[.;:\n]|$)").unwrap()
});

fn oraciones_de(texto: &str) -> Vec<String> {
    let plano = texto.replace('\u{a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    let mut salida = Vec::new();
    let mut actual = String::new();
    let mut previo = ' ';
    for pieza in plano.split(' ') {
        if previo == '.' && !actual.is_empty() {
            salida.push(std::mem::take(&mut actual));
        }
        if !actual.is_empty() {
            actual.push(' ');
        }
        actual.push_str(pieza);
        previo = pieza.chars().next_back().unwrap_or(' ');
    }
    if !actual.is_empty() {
        salida.push(actual);
    }
    salida
}

/// Observación registrada que ya hizo constar que ese niño cuenta con ingreso efectivo a ese programa: nombre completo
/// del programa (no por subcadena), primer nombre del niño en la misma oración y sin negación.
pub fn constancia_en_bitacora<'a>(observaciones: &'a [Observacion], primer_nombre: &str, programa: &str) -> Option<&'a Observacion> {
    let objetivo = clave_programa(programa);
    let nombre = clave_texto(primer_nombre);
    if objetivo.is_empty() {
        return None;
    }
    for obs in observaciones {
        for oracion in oraciones_de(&obs.texto) {
            for hallazgo in CONSTANCIA.captures_iter(&oracion) {
                let completo = hallazgo.get(0).unwrap();
                if clave_programa(&hallazgo[1]) != objetivo {
                    continue;
                }
                if NEGACION.is_match(&clave_texto(&oracion[..completo.start()])) {
                    continue;
                }
                if !nombre.is_empty() && !clave_texto(&oracion).split(' ').any(|t| t == nombre) {
                    continue;
                }
                return Some(obs);
            }
        }
    }
    None
}

/// La bitácora nombra el programa (con límites de palabra) sin ser una constancia reconocible.
pub fn mencion_previa<'a>(observaciones: &'a [Observacion], programa: &str) -> Option<&'a Observacion> {
    let objetivo = clave_programa(programa);
    if objetivo.is_empty() {
        return None;
    }
    observaciones.iter().find(|obs| {
        let texto = clave_texto(&obs.texto);
        let mut desde = 0;
        while let Some(p) = texto[desde..].find(&objetivo) {
            let ini = desde + p;
            let fin = ini + objetivo.len();
            let antes = texto[..ini].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
            let despues = texto[fin..].chars().next().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
            if antes && despues {
                return true;
            }
            desde = ini + 1;
            while !texto.is_char_boundary(desde) {
                desde += 1;
            }
        }
        false
    })
}
