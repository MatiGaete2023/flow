//! Cotejo de la evidencia nativa V1 con la tabla normalizada del mismo ingreso.

use super::{texto, Evidencia, Fila, METODO};
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
enum Momento {
    Dia(NaiveDate),
    Local(NaiveDateTime),
    ConZona(NaiveDateTime, i32),
}

impl Momento {
    fn leer(s: &str, precision: &str) -> Option<Self> {
        if s.len() == 10 {
            return (precision != "SEGUNDO")
                .then(|| s.parse().ok())
                .flatten()
                .map(Self::Dia);
        }
        if let Ok(t) = DateTime::parse_from_rfc3339(s) {
            return (precision != "DIA")
                .then(|| Self::ConZona(t.naive_local(), t.offset().local_minus_utc()));
        }
        let t = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").ok()?;
        if s.ends_with("T00:00:00") && precision != "SEGUNDO" {
            Some(Self::Dia(t.date()))
        } else {
            (precision != "DIA").then_some(Self::Local(t))
        }
    }
}

#[derive(PartialEq)]
struct Entrada {
    dia: NaiveDate,
    momento: Momento,
    texto: String,
    origen: String,
}

pub(super) struct Lectura<'a> {
    ventana: Option<(NaiveDate, NaiveDate)>,
    huella: Option<String>,
    firma: String,
    esperadas: HashMap<String, Entrada>,
    confirmadas: HashMap<String, Fila<'a>>,
    advertida: bool,
    resultado: Evidencia,
    prefijo: String,
}

impl<'a> Lectura<'a> {
    pub(super) fn nueva(q: Fila<'a>, v: &Value, advertencia: bool) -> Self {
        let estado = q.get("coverage");
        let cobertura = texto(v, "coverage");
        let leida = matches!(estado, "COMPLETA" | "PARCIAL" | "VACIA_COMPROBADA");
        let advertida = advertencia || !v["warnings"].as_array().is_some_and(Vec::is_empty);
        let rango = texto(v, "desde")
            .parse::<NaiveDate>()
            .ok()
            .zip(texto(v, "hasta").parse::<NaiveDate>().ok());
        let capturada = fecha_rfc3339(q.get("captured_at"))
            .filter(|c| fecha_rfc3339(texto(v, "captured_at")) == Some(*c));
        let ventana = rango
            .zip(capturada)
            .filter(|((a, b), c)| a <= b && a <= c && leida)
            .map(|((a, b), c)| (a, b.min(c)));
        let alcance = &v["alcance_revision_fuente"];
        let huella = texto(alcance, "html_sha256");
        let respaldo = texto(alcance, "metodo") == METODO
            && alcance["coverage"] == v["coverage"]
            && huella.len() == 64
            && huella.bytes().all(|b| b.is_ascii_hexdigit());
        let cobertura_completa = matches!(cobertura, "COMPLETA" | "VACIA_COMPROBADA");
        let puede_acreditar = !advertida
            && cobertura_completa
            && (matches!(estado, "COMPLETA" | "VACIA_COMPROBADA")
                || (estado == "PARCIAL" && respaldo))
            && (estado != "VACIA_COMPROBADA" || cobertura == "VACIA_COMPROBADA")
            && v["completa"].as_bool() == Some(true)
            && ["posteriores_omitidas", "ilegibles_omitidas"]
                .iter()
                .all(|k| v.get(*k).is_none_or(|n| n.as_u64() == Some(0)))
            && rango
                .zip(capturada)
                .is_some_and(|((a, b), c)| a <= b && b <= c);
        let mut lectura = Self {
            ventana,
            huella: respaldo.then(|| huella.to_ascii_lowercase()),
            firma: v.to_string(),
            esperadas: HashMap::new(),
            confirmadas: HashMap::new(),
            advertida,
            resultado: Evidencia {
                leida,
                recortada: cobertura == "PARCIAL",
                alcance_declarado: cobertura_completa,
                intervalo: if puede_acreditar { ventana } else { None },
                ..Default::default()
            },
            prefijo: format!("{}-", q.get("query_id")),
        };
        let Some(entradas) = v["entries"].as_array() else {
            lectura.resultado.intervalo = None;
            return lectura;
        };
        if cobertura == "VACIA_COMPROBADA" && !entradas.is_empty() {
            lectura.invalidar();
        }
        for e in entradas {
            lectura.esperar(e);
        }
        lectura
    }

    pub(super) fn invalidar(&mut self) {
        self.resultado.contradictoria = true;
    }

    fn dentro(&mut self, dia: NaiveDate) -> bool {
        let Some((desde, hasta)) = self.ventana else {
            return false;
        };
        if dia > hasta {
            self.resultado.intervalo = None;
        }
        (desde..=hasta).contains(&dia)
    }

    fn esperar(&mut self, e: &Value) {
        let fecha = texto(e, "fecha");
        let Some(dia) = super::dia(fecha) else {
            self.resultado.intervalo = None;
            return;
        };
        if !self.dentro(dia) {
            return;
        }
        let Some(texto_entrada) = e["texto"].as_str() else {
            self.resultado.intervalo = None;
            return;
        };
        if texto_entrada.trim().is_empty() {
            return;
        }
        let id = texto(e, "entry_id");
        let Some(momento) = Momento::leer(fecha, "").filter(|_| !id.is_empty()) else {
            self.invalidar();
            return;
        };
        let entrada = Entrada {
            dia,
            momento,
            texto: texto_entrada.into(),
            origen: texto(e, "origen").into(),
        };
        if let Some(precedente) = self.esperadas.get(id) {
            if precedente != &entrada {
                self.invalidar();
            }
        } else {
            self.esperadas.insert(id.into(), entrada);
        }
    }

    pub(super) fn confirmar(&mut self, fila: Fila<'a>) {
        if fila.get("identity_status") != "VERIFICADA"
            || self
                .huella
                .as_ref()
                .is_some_and(|h| !fila.get("source_sha256").eq_ignore_ascii_case(h))
        {
            self.invalidar();
            return;
        }
        let texto = fila.get("text");
        if texto.trim().is_empty() {
            return;
        }
        let fecha = fila.get("event_at");
        let Some(dia) = super::dia(fecha) else {
            self.invalidar();
            return;
        };
        if !self.dentro(dia) {
            return;
        }
        let id = fila.get("event_id");
        let exacta = self.esperadas.get_key_value(id);
        let prefijada = id
            .strip_prefix(&self.prefijo)
            .and_then(|s| self.esperadas.get_key_value(s));
        let candidata = match (exacta, prefijada) {
            (Some(_), Some(_)) => None,
            (Some(e), None) | (None, Some(e)) => Some(e),
            _ => None,
        };
        let Some((id, esperada)) = candidata else {
            self.invalidar();
            return;
        };
        if esperada.texto != texto
            || Momento::leer(fecha, fila.get("time_precision")).as_ref() != Some(&esperada.momento)
        {
            self.invalidar();
            return;
        }
        if let Some(primer) = self.confirmadas.insert(id.clone(), fila) {
            if fila
                .pares()
                .any(|(k, v)| k != "event_id" && primer.campo(k) != Some(v))
            {
                self.invalidar();
            }
        }
    }

    pub(super) fn finalizar(mut self) -> (String, Evidencia, Vec<String>) {
        if self.esperadas.len() != self.confirmadas.len() {
            self.invalidar();
        }
        let mut centros = Vec::new();
        for (id, e) in self.esperadas {
            match e.origen.as_str() {
                "centro" if !self.advertida => {
                    self.resultado.centro.insert(e.dia);
                    if let Some(fila) = self.confirmadas.get(&id) {
                        centros.push(fila.get("event_id").to_owned());
                    }
                }
                "tribunal" | "programa" if !self.advertida => {}
                _ => {
                    self.resultado.inciertas.insert(e.dia);
                }
            }
        }
        (self.firma, self.resultado, centros)
    }
}

fn fecha_rfc3339(s: &str) -> Option<NaiveDate> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coteja_el_momento_civil_y_su_precision() {
        let leer = Momento::leer;
        assert_eq!(leer("2026-08-12T00:00:00", ""), leer("2026-08-12", "DIA"));
        assert_ne!(
            leer("2026-08-12T00:00:00", "SEGUNDO"),
            leer("2026-08-12", "DIA")
        );
        assert_eq!(
            leer("2026-08-12T10:00:00Z", ""),
            leer("2026-08-12T10:00:00+00:00", "SEGUNDO")
        );
        assert_ne!(
            leer("2026-08-12T10:00:00-03:00", ""),
            leer("2026-08-12T13:00:00Z", "SEGUNDO")
        );
    }
}
