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
const GUIONES: [char; 7] = ['-', '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}', '\u{2212}'];

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
    /// «EXPLICITO» = asociado a una proyección de este ingreso; «AJENO» = asociado expresamente a OTRO ingreso (nunca es
    /// candidato aquí); vacío = sin vínculo.
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
    /// Dos lecturas del mismo instante se contradicen (E3B-02, E3C-01): no se atribuye nada y se pide revisión.
    #[serde(default)]
    pub discordante: bool,
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
    /// Folios de Historia asociados expresamente a OTRO informe de la causa: la vía genérica no los atribuye a este.
    #[serde(default)]
    pub folios_ajenos: Vec<String>,
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

static PALABRA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{L}+|\p{Nd}+|[^\w\s]").unwrap());
/// Palabras que nunca forman parte del nombre del sujeto (`evidencia.py::_NO_NOMBRE`).
const NO_NOMBRE: [&str; 46] = [
    "que", "se", "hace", "presente", "informa", "informo", "indica", "indico", "senala", "senalo", "comunica", "refiere",
    "menciona", "consta", "da", "cuenta", "el", "la", "los", "las", "un", "una", "nino", "nina", "nna", "adolescente",
    "joven", "menor", "y", "e", "o", "de", "del", "al", "a", "con", "por", "para", "en", "segun", "ya", "actualmente",
    "hoy", "tambien", "asimismo", "programa",
];

const PARTICULAS: [&str; 6] = ["de", "del", "la", "las", "los", "san"];

/// Nombres que preceden inmediatamente al verbo, hasta 6 palabras; partículas entre nombres incluidas y una coma pegada al
/// verbo ignorada. El prefijo llega en NFC (E3C-05; `evidencia.py::_sujeto`).
fn sujeto(prefijo: &str) -> Vec<String> {
    let mut tokens: Vec<&str> = PALABRA.find_iter(prefijo).map(|m| m.as_str()).collect();
    if tokens.last() == Some(&",") {
        tokens.pop();
    }
    let alfa = |t: &str| t.chars().next().is_some_and(char::is_alphabetic);
    let mut salida: Vec<String> = Vec::new();
    for i in (0..tokens.len()).rev() {
        let token = tokens[i];
        let llave = clave_texto(token);
        if !alfa(token) || salida.len() >= 6 {
            break;
        }
        if NO_NOMBRE.contains(&llave.as_str()) {
            let anterior = if i > 0 { tokens[i - 1] } else { "" };
            let clave_anterior = clave_texto(anterior);
            let entre_nombres =
                alfa(anterior) && (!NO_NOMBRE.contains(&clave_anterior.as_str()) || PARTICULAS.contains(&clave_anterior.as_str()));
            if !salida.is_empty() && PARTICULAS.contains(&llave.as_str()) && entre_nombres {
                salida.push(llave);
                continue;
            }
            break;
        }
        salida.push(llave);
    }
    salida.reverse();
    salida
}

/// Observación registrada que ya hizo constar que ese niño cuenta con ingreso efectivo a ese programa: nombre completo
/// del programa (no por subcadena) y SUJETO = el niño: los nombres justo antes del verbo empiezan por su primer nombre y, si
/// se conoce su nombre completo, coinciden con sus primeras palabras (E3-12, E3B-07). Sin negación.
pub fn constancia_en_bitacora<'a>(
    observaciones: &'a [Observacion],
    primer_nombre: &str,
    programa: &str,
    nombre_completo: &str,
) -> Option<&'a Observacion> {
    let objetivo = clave_programa(programa);
    let nombre = clave_texto(primer_nombre);
    let completo: Vec<String> = clave_texto(nombre_completo).split_whitespace().map(String::from).collect();
    if objetivo.is_empty() || nombre.is_empty() {
        return None;
    }
    use unicode_normalization::UnicodeNormalization;
    for obs in observaciones {
        // NFC antes de buscar el verbo: «ingresó» con acento combinante es el mismo verbo (E3C-05).
        let texto: String = obs.texto.nfc().collect();
        for oracion in oraciones_de(&texto) {
            for hallazgo in CONSTANCIA.captures_iter(&oracion) {
                let completo_match = hallazgo.get(0).unwrap();
                if clave_programa(&hallazgo[1]) != objetivo {
                    continue;
                }
                let previo = &oracion[..completo_match.start()];
                if NEGACION.is_match(&clave_texto(previo)) {
                    continue;
                }
                let s = sujeto(previo);
                if s.first() != Some(&nombre) {
                    continue;
                }
                // Sin nombre completo no se distingue «Ana Luisa» de otra Ana: no se da por constancia.
                if s.len() > 1 && (s.len() > completo.len() || s[..] != completo[..s.len()]) {
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

#[cfg(test)]
mod pruebas_e3b {
    use super::*;

    /// Espejo de `tests/test_integracion_20261118.py::test_e3b_07_sujeto_con_nombre_compuesto_y_dos_ninos_con_igual_primer_nombre`.
    #[test]
    fn e3b_07_sujeto_con_nombre_compuesto() {
        let previa = |texto: &str, completo: &str| {
            let obs = [Observacion { fecha: "2026-10-01".parse().unwrap(), texto: texto.into() }];
            constancia_en_bitacora(&obs, "Ana", "PRM Luna", completo).is_some()
        };
        assert!(previa("Ana María cuenta con ingreso efectivo al programa PRM Luna.", "ANA MARÍA SOTO"));
        assert!(previa("Se hace presente que ANA MARI\u{301}A cuenta con ingreso efectivo al programa PRM Luna.", "Ana María Soto"));
        assert!(!previa("Ana María cuenta con ingreso efectivo al programa PRM Luna.", "Ana Luisa Pérez"));
        assert!(previa("Ana cuenta con ingreso efectivo al programa PRM Luna.", "Ana Luisa Pérez"));
        assert!(!previa("Ana, informa que Beto cuenta con ingreso efectivo al programa PRM Luna.", "Ana Luisa Pérez"));
        assert!(!previa("La niña Ana no cuenta con ingreso efectivo al programa PRM Luna.", ""));
    }
}

#[cfg(test)]
mod pruebas_e3c {
    use super::*;

    /// Espejo de `test_e3c_05_sujeto_con_verbo_nfd_particulas_coma_y_nombre_incompleto`.
    #[test]
    fn e3c_05_sujeto() {
        let previa = |texto: &str, completo: &str| {
            let obs = [Observacion { fecha: "2026-10-01".parse().unwrap(), texto: texto.into() }];
            constancia_en_bitacora(&obs, "Ana", "PRM Luna", completo).is_some()
        };
        let nfd = "Ana María ingreso\u{301} efectivamente al programa PRM Luna.";
        assert!(previa(nfd, "Ana María Soto") && previa(&nfd.replace("o\u{301}", "ó"), "Ana María Soto"));
        assert!(previa("Ana María del Río cuenta con ingreso efectivo al programa PRM Luna.", "Ana María del Río Soto"));
        assert!(previa("Ana de la Fuente cuenta con ingreso efectivo al programa PRM Luna.", "Ana de la Fuente Soto"));
        assert!(!previa("Ana María del Río cuenta con ingreso efectivo al programa PRM Luna.", "Ana Luisa Pérez"));
        assert!(previa("Ana, cuenta con ingreso efectivo al programa PRM Luna.", "Ana Soto"));
        assert!(previa("Ana María, cuenta con ingreso efectivo al programa PRM Luna.", "Ana María Soto"));
        assert!(!previa("Ana, informa que Beto cuenta con ingreso efectivo al programa PRM Luna.", "Ana Soto"));
        assert!(!previa("Ana Luisa cuenta con ingreso efectivo al programa PRM Luna.", ""));
        assert!(previa("Ana cuenta con ingreso efectivo al programa PRM Luna.", ""));
        assert!(!previa("Se informa que la Ana Luisa cuenta con ingreso efectivo al programa PRM Luna.", ""));
    }
}
