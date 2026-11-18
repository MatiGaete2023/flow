//! Escritos de la causa e Ingreso Histórico Protección de la persona: del paquete de captura a la evidencia del motor
//! (`nurus/capture/escritos.py`, plan 18-nov-2026). Solo transforma lo ya capturado; cada fuente conserva su estado.

use chrono::NaiveDate;
use flow_dominio::evidencia::clave_texto;
use flow_dominio::{Escrito, Escritos, EvidenciaInforme, Historico};
use flow_dominio::evidencia::ProgramaHistorico;
use serde_json::Value;
use std::collections::HashMap;

type FilaCsv = HashMap<String, String>;
pub(crate) type FuenteEscritos = (Value, Vec<FilaCsv>);

const ESCRITOS_DE_INFORME: [&str; 3] = ["Inf. Cump Programa Ambulatorio", "Inf. Cump FAE/FAS", "Inf. Cump Centro Residencial"];
const INGRESO_INFORME: &str = "Ingreso informe de cumplimiento X";

fn dato<'a>(fila: &'a FilaCsv, clave: &str) -> &'a str {
    fila.get(clave).map(String::as_str).unwrap_or("")
}

fn dia(texto: &str) -> Option<NaiveDate> {
    texto.get(..10).and_then(|d| d.parse().ok())
}

fn rango(cobertura: &str) -> u8 {
    match cobertura {
        "COMPLETA" => 3,
        "PARCIAL" => 2,
        "VACIA_COMPROBADA" => 1,
        _ => 0,
    }
}

fn cobertura(consulta: &Value) -> String {
    match consulta["coverage"].as_str().unwrap_or("") {
        "IDENTIDAD_INCIERTA" => "PARCIAL".into(),
        otra => otra.to_string(),
    }
}

/// {clave: consulta} con la mejor cobertura por clave; `ultimo` es `cause_id` (escritos) o `person_id` (histórico).
fn mejores_consultas(manifiesto: &Value, pantalla: &str, filtro: &str, ultimo: &str) -> HashMap<[String; 3], Value> {
    let mut mejor: HashMap<[String; 3], Value> = HashMap::new();
    for consulta in manifiesto["queries"].as_array().into_iter().flatten() {
        if consulta["screen"].as_str() != Some(pantalla) {
            continue;
        }
        let Ok(filtros) = serde_json::from_str::<Value>(consulta["filters_json"].as_str().unwrap_or("")) else { continue };
        let alcance = &filtros[filtro];
        if !alcance.is_object() {
            continue;
        }
        let clave = ["source_system", "court_code", ultimo].map(|k| alcance[k].as_str().unwrap_or("").to_string());
        let nueva = rango(consulta["coverage"].as_str().unwrap_or(""));
        if mejor.get(&clave).is_none_or(|actual| nueva > rango(actual["coverage"].as_str().unwrap_or(""))) {
            mejor.insert(clave, consulta.clone());
        }
    }
    mejor
}

pub(crate) fn filas_de_escritos(manifiesto: &Value, tabla: &[FilaCsv]) -> HashMap<[String; 3], FuenteEscritos> {
    mejores_consultas(manifiesto, "escritos", "escritos_causa", "cause_id")
        .into_iter()
        .map(|(clave, consulta)| {
            let id = consulta["query_id"].as_str().unwrap_or("").to_string();
            let filas = tabla.iter().filter(|f| dato(f, "query_id") == id).cloned().collect();
            (clave, (consulta, filas))
        })
        .collect()
}

/// `Escritos` del motor para un registro de ingreso verificado, con la asociación explícita de ESE ingreso.
pub(crate) fn escritos_para(registro: &FilaCsv, fuentes: &HashMap<[String; 3], FuenteEscritos>) -> Option<Escritos> {
    let clave = ["source_system", "court_code", "cause_id"].map(|k| dato(registro, k).to_string());
    let (consulta, filas) = fuentes.get(&clave)?;
    let escritos = filas
        .iter()
        .enumerate()
        .map(|(n, f)| {
            let propio = dato(f, "linked_ingreso_id") == dato(registro, "ingreso_id") && !dato(f, "linked_projection").is_empty();
            Escrito {
                tipo: dato(f, "escrito_type").into(),
                estado: dato(f, "escrito_status").into(),
                doc: dato(f, "doc").into(),
                solicitante: dato(f, "requester").into(),
                fila: dato(f, "original_row").parse().unwrap_or(n as i64 + 1),
                vinculo: if propio { "EXPLICITO".into() } else { String::new() },
                fecha_proyectada: if propio { dia(dato(f, "linked_projection")) } else { None },
                periodo: String::new(),
            }
        })
        .collect();
    Some(Escritos { cobertura: cobertura(consulta), leido: dia(consulta["captured_at"].as_str().unwrap_or("")), filas: escritos })
}

pub(crate) fn historicos_por_persona(manifiesto: &Value, tabla: &[FilaCsv]) -> HashMap<[String; 3], Historico> {
    mejores_consultas(manifiesto, "historico_proteccion", "historico_persona", "person_id")
        .into_iter()
        .map(|(clave, consulta)| {
            let id = consulta["query_id"].as_str().unwrap_or("");
            let filas: Vec<&FilaCsv> = tabla.iter().filter(|f| dato(f, "query_id") == id).collect();
            let verificada = !filas.is_empty() && filas.iter().all(|f| dato(f, "ownership") == "VERIFICADA");
            let incierta = consulta["coverage"].as_str() == Some("IDENTIDAD_INCIERTA") || (!filas.is_empty() && !verificada);
            let programas = filas
                .iter()
                .enumerate()
                .map(|(n, f)| ProgramaHistorico {
                    programa: dato(f, "program_name").into(),
                    fecha_egreso: dato(f, "egreso_date").into(),
                    causal_egreso: dato(f, "egreso_cause").into(),
                    fila: dato(f, "original_row").parse().unwrap_or(n as i64 + 1),
                })
                .collect();
            (
                clave,
                Historico {
                    cobertura: cobertura(&consulta),
                    leido: dia(consulta["captured_at"].as_str().unwrap_or("")),
                    identidad: if incierta { "INCIERTA".into() } else { "VERIFICADA".into() },
                    filas: programas,
                },
            )
        })
        .collect()
}

/// Asociación explícita de un informe (ingreso + fecha proyectada) con sus evidencias; `None` si no hay vínculo. Las
/// filas de Historia con la misma referencia documental (`Doc.`) que un escrito asociado se enlazan al mismo informe.
pub(crate) fn evidencia_informe(
    registro: &FilaCsv,
    vencimiento: Option<NaiveDate>,
    fuentes: &HashMap<[String; 3], FuenteEscritos>,
    historia: &[&FilaCsv],
) -> Option<EvidenciaInforme> {
    let vencimiento = vencimiento?;
    let clave = ["source_system", "court_code", "cause_id"].map(|k| dato(registro, k).to_string());
    let (_, filas) = fuentes.get(&clave)?;
    let iso = vencimiento.to_string();
    let docs: Vec<String> = filas
        .iter()
        .filter(|f| dato(f, "linked_ingreso_id") == dato(registro, "ingreso_id") && dato(f, "linked_projection").get(..10) == Some(iso.as_str()))
        .map(|f| clave_texto(dato(f, "doc")))
        .filter(|d| !d.is_empty())
        .collect();
    let (mut folio_escrito, mut folio_resolucion, mut fecha_resolucion) = (String::new(), String::new(), None);
    for fila in historia {
        if docs.is_empty() || !docs.contains(&clave_texto(dato(fila, "reference"))) {
            continue;
        }
        let descripcion = clave_texto(dato(fila, "description"));
        if ESCRITOS_DE_INFORME.iter().any(|n| clave_texto(n) == descripcion) {
            if !dato(fila, "folio").trim().is_empty() {
                folio_escrito = dato(fila, "folio").trim().to_string();
            }
        } else if descripcion == clave_texto(INGRESO_INFORME) {
            if !dato(fila, "folio").trim().is_empty() {
                folio_resolucion = dato(fila, "folio").trim().to_string();
            }
            fecha_resolucion = dia(dato(fila, "event_at"));
        }
    }
    if folio_escrito.is_empty() && folio_resolucion.is_empty() {
        return None;
    }
    Some(EvidenciaInforme {
        escrito_historia_folio: folio_escrito,
        resolucion_folio: folio_resolucion,
        resolucion_fecha: fecha_resolucion,
        calendario: String::new(),
        fecha_proyectada: Some(vencimiento),
    })
}

/// La bitácora del ingreso no se leyó completa: no acredita ausencia de una constancia previa. Mismo cálculo que
/// `coverage.build_index`: estados de las filas verificadas y de las consultas con alcance de ese ingreso.
pub(crate) fn bitacora_parcial(manifiesto: &Value, bitacoras: &[FilaCsv], ident: &[String; 4]) -> bool {
    let mut estados: Vec<String> = Vec::new();
    for fila in bitacoras {
        if dato(fila, "identity_status") == "VERIFICADA"
            && ["source_system", "court_code", "cause_id", "ingreso_id"].map(|k| dato(fila, k)) == ident.each_ref().map(String::as_str)
        {
            estados.push(dato(fila, "coverage").into());
            if let Some(q) = manifiesto["queries"].as_array().into_iter().flatten().find(|q| q["query_id"].as_str() == Some(dato(fila, "query_id"))) {
                estados.push(q["coverage"].as_str().unwrap_or("").into());
            }
        }
    }
    for q in manifiesto["queries"].as_array().into_iter().flatten() {
        let Ok(filtros) = serde_json::from_str::<Value>(q["filters_json"].as_str().unwrap_or("")) else { continue };
        let alcance = &filtros["remote_identity"];
        let declaradas = filtros["capture_tables"].as_array();
        let de_bitacoras = match declaradas {
            Some(lista) => lista.iter().any(|t| t.as_str() == Some("bitacoras")),
            None => q["screen"].as_str() == Some("bitacoras"),
        };
        if de_bitacoras && alcance.is_object() {
            let clave = ["source_system", "court_code", "cause_id", "ingreso_id"].map(|k| alcance[k].as_str().unwrap_or("").to_string());
            if &clave == ident {
                estados.push(q["coverage"].as_str().unwrap_or("").into());
            }
        }
    }
    if estados.is_empty() {
        return false;
    }
    let adecuado = |s: &String| s == "COMPLETA" || s == "VACIA_COMPROBADA";
    let agregado = if estados.iter().all(|s| s == "VACIA_COMPROBADA") {
        "VACIA_COMPROBADA".to_string()
    } else if estados.iter().all(adecuado) {
        "COMPLETA".to_string()
    } else if estados.iter().all(|s| *s == estados[0]) {
        estados[0].clone()
    } else {
        "PARCIAL".to_string()
    };
    !adecuado(&agregado)
}
