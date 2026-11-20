//! Clasificación por período a partir de evidencia V2 del Descargador Python.

use super::{json, Captura, Fila};
use chrono::NaiveDate;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};

const METODO: &str = "SITFA_TABLA_INFORME_INGRESO_V1";
const CAMPOS_ID: [&str; 4] = ["source_system", "court_code", "cause_id", "ingreso_id"];
type Identidad = [String; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Clase {
    ConObservacion,
    SinObservacion,
    NoVerificable,
}

impl Clase {
    pub fn etiqueta(self) -> &'static str {
        match self {
            Self::ConObservacion => "Con observación",
            Self::SinObservacion => "Sin observación",
            Self::NoVerificable => "No verificable",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Resultado {
    pub clase: Clase,
    pub motivo: String,
}

#[derive(Default)]
struct Evidencia {
    leida: bool,
    recortada: bool,
    alcance_declarado: bool,
    intervalo: Option<(NaiveDate, NaiveDate)>,
    centro: BTreeSet<NaiveDate>,
    inciertas: BTreeSet<NaiveDate>,
}

pub struct Revision {
    por_ingreso: HashMap<Identidad, Evidencia>,
}

fn dia(s: &str) -> Option<NaiveDate> {
    s.get(..10)?.parse().ok()
}

fn identidad(fila: Fila<'_>) -> Identidad {
    CAMPOS_ID.map(|k| fila.get(k).to_owned())
}

fn texto<'a>(obj: &'a Value, clave: &str) -> &'a str {
    obj[clave].as_str().unwrap_or("")
}

impl Revision {
    pub fn nueva(captura: &Captura) -> Self {
        let mut por_ingreso: HashMap<Identidad, Evidencia> = HashMap::new();
        let mut consultas = HashMap::new();
        let advertidas: HashSet<_> = captura
            .filas("incidencias")
            .filter(|i| i.get("code") == "BITACORA_ADVERTENCIA")
            .map(|i| i.get("query_id"))
            .collect();
        for q in captura
            .filas("consultas")
            .filter(|q| q.get("screen") == "bitacoras")
        {
            let Ok(filtros) = json::objeto(q.get("filters_json")) else {
                continue;
            };
            let Some(scope) = filtros.get("remote_identity").and_then(Value::as_object) else {
                continue;
            };
            let id = CAMPOS_ID.map(|k| {
                scope
                    .get(k)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            });
            if id.iter().any(String::is_empty) {
                continue;
            }
            let alcance = filtros.get("revision_bitacoras").unwrap_or(&Value::Null);
            let version = alcance["version"].as_u64();
            let advertencia = advertidas.contains(q.get("query_id"))
                || alcance
                    .get("warnings")
                    .is_some_and(|v| v.as_u64() != Some(0));
            let lectura = q.get("coverage");
            let leida = matches!(lectura, "COMPLETA" | "PARCIAL" | "VACIA_COMPROBADA");
            let cobertura = texto(alcance, "coverage");
            let alcance_declarado = matches!(cobertura, "COMPLETA" | "VACIA_COMPROBADA");
            let rango = intervalo_v2(alcance, q, advertencia);
            consultas.insert(
                q.get("query_id"),
                (id.clone(), version == Some(2) && leida && !advertencia),
            );
            por_ingreso
                .entry(id)
                .and_modify(|e| {
                    e.leida |= leida;
                    e.recortada |= cobertura == "PARCIAL";
                    e.intervalo = None;
                })
                .or_insert(Evidencia {
                    leida,
                    recortada: cobertura == "PARCIAL",
                    alcance_declarado,
                    intervalo: rango,
                    ..Default::default()
                });
        }
        for evento in captura.filas("bitacoras") {
            let Some(dia) = dia(evento.get("event_at")) else {
                continue;
            };
            if evento.get("text").trim().is_empty() {
                continue;
            }
            let id = identidad(evento);
            let verificada = consultas
                .get(evento.get("query_id"))
                .is_some_and(|(consultada, fiable)| consultada == &id && *fiable);
            let Some(e) = por_ingreso.get_mut(&id) else {
                continue;
            };
            if !verificada
                || evento.get("identity_status") != "VERIFICADA"
                || evento.get("origin_verified") != "true"
                || evento.get("origin") == "DESCONOCIDO"
            {
                e.inciertas.insert(dia);
            } else if evento.get("origin") == "CENTRO" {
                e.centro.insert(dia);
            }
        }
        Self { por_ingreso }
    }

    pub fn clasificar(
        &self,
        registro: Fila<'_>,
        desde: NaiveDate,
        hasta: NaiveDate,
    ) -> Result<Resultado, String> {
        if desde > hasta {
            return Err("La fecha desde es posterior a hasta.".into());
        }
        let no = |motivo: &str| {
            Ok(Resultado {
                clase: Clase::NoVerificable,
                motivo: motivo.into(),
            })
        };
        let disponible = (registro.get("identity_status") == "VERIFICADA")
            .then(|| self.por_ingreso.get(&identidad(registro)))
            .flatten();
        let Some(e) = disponible else {
            return no("Sin bitácora asociada a una identidad verificada en el paquete.");
        };
        let mut dias = e.centro.range(desde..=hasta);
        if let Some(ultimo) = dias.next_back() {
            let n = dias.count() + 1;
            let cantidad = if n == 1 {
                "1 día".into()
            } else {
                format!("{n} días")
            };
            return Ok(Resultado {
                clase: Clase::ConObservacion,
                motivo: format!(
                    "Observaciones del centro en {cantidad} del período. Último día {ultimo}."
                ),
            });
        }
        if e.inciertas.range(desde..=hasta).next().is_some() {
            return no("Hay entradas en el período sin procedencia o autoría comprobadas.");
        }
        if !e.leida {
            return no("La bitácora no se pudo leer.");
        }
        let Some((inicio, fin)) = e.intervalo else {
            if e.recortada {
                return no("La ventana mostró indicios de recorte.");
            }
            if e.alcance_declarado {
                return no(
                    "La lectura tiene advertencias o carece de evidencia válida de cobertura.",
                );
            }
            return no("Lectura sin evidencia de alcance V2.");
        };
        if hasta > fin {
            return no("La bitácora se leyó antes del fin del período.");
        }
        if desde < inicio {
            return no("El período comienza antes del intervalo conservado en el paquete.");
        }
        Ok(Resultado {
            clase: Clase::SinObservacion,
            motivo: "Bitácora leída completa, sin observaciones del centro en el período.".into(),
        })
    }
}

fn intervalo_v2(v: &Value, q: Fila<'_>, advertencia: bool) -> Option<(NaiveDate, NaiveDate)> {
    let lectura = q.get("coverage");
    let cobertura = texto(v, "coverage");
    if advertencia
        || v["version"].as_u64() != Some(2)
        || texto(v, "metodo") != METODO
        || v["completa"].as_bool() != Some(true)
        || v["warnings"].as_u64() != Some(0)
        || !matches!(lectura, "COMPLETA" | "PARCIAL" | "VACIA_COMPROBADA")
        || !matches!(cobertura, "COMPLETA" | "VACIA_COMPROBADA")
        || (lectura == "VACIA_COMPROBADA" && cobertura != "VACIA_COMPROBADA")
    {
        return None;
    }
    let desde: NaiveDate = texto(v, "desde").parse().ok()?;
    let hasta: NaiveDate = texto(v, "hasta").parse().ok()?;
    (desde <= hasta
        && texto(v, "desde") == desde.to_string()
        && texto(v, "hasta") == hasta.to_string()
        && dia(texto(v, "captured_at")) == Some(hasta)
        && dia(q.get("captured_at")) == Some(hasta))
    .then_some((desde, hasta))
}
