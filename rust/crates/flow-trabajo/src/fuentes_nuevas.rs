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

fn instante(texto: &str) -> (String, String) {
    (texto.chars().take(10).collect(), texto.replace('Z', "+00:00"))
}

/// {clave: consulta} con la ÚLTIMA lectura fiable al corte por clave (E3-07; `escritos.py::_mejores_consultas`). `ultimo` es
/// `cause_id` (escritos) o `person_id` (histórico). Se descartan las lecturas posteriores al corte; entre completas (con filas o
/// vacías comprobadas) manda la más reciente y, si dos del mismo instante discrepan en cobertura, queda PARCIAL.
fn mejores_consultas(manifiesto: &Value, pantalla: &str, filtro: &str, ultimo: &str, corte: Option<NaiveDate>) -> HashMap<[String; 3], Value> {
    let mut grupos: HashMap<[String; 3], Vec<&Value>> = HashMap::new();
    let mut orden: Vec<[String; 3]> = Vec::new();
    for consulta in manifiesto["queries"].as_array().into_iter().flatten() {
        if consulta["screen"].as_str() != Some(pantalla) {
            continue;
        }
        let Ok(filtros) = serde_json::from_str::<Value>(consulta["filters_json"].as_str().unwrap_or("")) else { continue };
        let alcance = &filtros[filtro];
        if !alcance.is_object() {
            continue;
        }
        if let Some(c) = corte {
            if instante(consulta["captured_at"].as_str().unwrap_or("")).0 > c.to_string() {
                continue;
            }
        }
        let clave = ["source_system", "court_code", ultimo].map(|k| alcance[k].as_str().unwrap_or("").to_string());
        if !grupos.contains_key(&clave) {
            orden.push(clave.clone());
        }
        grupos.entry(clave).or_default().push(consulta);
    }
    let mut mejor = HashMap::new();
    for clave in orden {
        let consultas = &grupos[&clave];
        let cuando = |q: &Value| instante(q["captured_at"].as_str().unwrap_or(""));
        let completas: Vec<&Value> =
            consultas.iter().copied().filter(|q| matches!(q["coverage"].as_str(), Some("COMPLETA" | "VACIA_COMPROBADA"))).collect();
        let elegida = if let Some(ultimo) = completas.iter().map(|q| cuando(q)).max() {
            let mismas: Vec<&Value> = completas.iter().copied().filter(|q| cuando(q) == ultimo).collect();
            let mut e = (*mismas.last().unwrap()).clone();
            if mismas.iter().any(|q| q["coverage"] != e["coverage"]) {
                e["coverage"] = Value::from("PARCIAL"); // lecturas discordantes del mismo instante
            }
            e
        } else {
            (*consultas.iter().max_by_key(|q| (rango(q["coverage"].as_str().unwrap_or("")), cuando(q))).unwrap()).clone()
        };
        mejor.insert(clave, elegida);
    }
    mejor
}

pub(crate) fn filas_de_escritos(manifiesto: &Value, tabla: &[FilaCsv], corte: Option<NaiveDate>) -> HashMap<[String; 3], FuenteEscritos> {
    mejores_consultas(manifiesto, "escritos", "escritos_causa", "cause_id", corte)
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
            let ajeno = !dato(f, "linked_ingreso_id").is_empty() && dato(f, "linked_ingreso_id") != dato(registro, "ingreso_id");
            Escrito {
                tipo: dato(f, "escrito_type").into(),
                estado: dato(f, "escrito_status").into(),
                doc: dato(f, "doc").into(),
                solicitante: dato(f, "requester").into(),
                fila: dato(f, "original_row").parse().unwrap_or(n as i64 + 1),
                vinculo: if propio {
                    "EXPLICITO".into()
                } else if ajeno {
                    "AJENO".into()
                } else {
                    String::new()
                },
                fecha_proyectada: if propio { dia(dato(f, "linked_projection")) } else { None },
                periodo: String::new(),
            }
        })
        .collect();
    // La identidad incierta de Escritos no se rebaja a «parcial»: no acredita nada (E3-05).
    Some(Escritos { cobertura: consulta["coverage"].as_str().unwrap_or("").to_string(), leido: dia(consulta["captured_at"].as_str().unwrap_or("")), filas: escritos })
}

pub(crate) fn historicos_por_persona(manifiesto: &Value, tabla: &[FilaCsv], corte: Option<NaiveDate>) -> HashMap<[String; 3], Historico> {
    mejores_consultas(manifiesto, "historico_proteccion", "historico_persona", "person_id", corte)
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

/// Asociación explícita de un informe (ingreso + fecha proyectada) con sus evidencias; `None` si no hay vínculo. Las filas
/// de Historia con la misma referencia documental (`Doc.`) que un escrito asociado se enlazan a ese informe; las enlazadas a
/// OTRO informe de la causa se devuelven como `folios_ajenos` (E3-10).
pub(crate) fn evidencia_informe(
    registro: &FilaCsv,
    vencimiento: Option<NaiveDate>,
    fuentes: &HashMap<[String; 3], FuenteEscritos>,
    historia: &[&FilaCsv],
) -> Option<EvidenciaInforme> {
    let vencimiento = vencimiento?;
    let clave = ["source_system", "court_code", "cause_id"].map(|k| dato(registro, k).to_string());
    let (_, filas) = fuentes.get(&clave)?;
    let propio = (dato(registro, "ingreso_id").to_string(), vencimiento.to_string());
    let mut docs: Vec<((String, String), String)> = Vec::new();
    for f in filas {
        let referencia = clave_texto(dato(f, "doc"));
        if !dato(f, "linked_ingreso_id").is_empty() && !dato(f, "linked_projection").is_empty() && !referencia.is_empty() {
            let dueno = (dato(f, "linked_ingreso_id").to_string(), dato(f, "linked_projection").chars().take(10).collect());
            docs.push((dueno, referencia));
        }
    }
    let escritos: Vec<String> = ESCRITOS_DE_INFORME.iter().map(|n| clave_texto(n)).collect();
    let (mut folio_escrito, mut folio_resolucion, mut fecha_resolucion) = (String::new(), String::new(), None);
    let mut ajenos: Vec<String> = Vec::new();
    for fila in historia {
        let referencia = clave_texto(dato(fila, "reference"));
        let folio = dato(fila, "folio").trim().to_string();
        if referencia.is_empty() {
            continue;
        }
        let duenos: Vec<&(String, String)> = docs.iter().filter(|(_, d)| *d == referencia).map(|(k, _)| k).collect();
        if !folio.is_empty() && duenos.iter().any(|k| **k != propio) && !ajenos.contains(&folio) {
            ajenos.push(folio.clone());
        }
        if !duenos.contains(&&propio) {
            continue;
        }
        let descripcion = clave_texto(dato(fila, "description"));
        if escritos.contains(&descripcion) {
            if !folio.is_empty() {
                folio_escrito = folio;
            }
        } else if descripcion == clave_texto(INGRESO_INFORME) {
            if !folio.is_empty() {
                folio_resolucion = folio;
            }
            fecha_resolucion = dia(dato(fila, "event_at"));
        }
    }
    ajenos.retain(|f| *f != folio_escrito && *f != folio_resolucion);
    ajenos.sort();
    if folio_escrito.is_empty() && folio_resolucion.is_empty() && ajenos.is_empty() {
        return None;
    }
    Some(EvidenciaInforme {
        escrito_historia_folio: folio_escrito,
        resolucion_folio: folio_resolucion,
        resolucion_fecha: fecha_resolucion,
        calendario: String::new(),
        fecha_proyectada: Some(vencimiento),
        folios_ajenos: ajenos,
    })
}

/// Filas de las tablas nuevas fuera del alcance que declara su consulta (pantalla, sistema, tribunal y causa/persona):
/// el paquete se rechaza, como en `contract.py` (E3-02).
pub(crate) fn validar_alcance(manifiesto: &Value, tablas: &HashMap<String, Vec<FilaCsv>>) -> Result<(), String> {
    let consultas: HashMap<&str, &Value> =
        manifiesto["queries"].as_array().into_iter().flatten().filter_map(|q| Some((q["query_id"].as_str()?, q))).collect();
    for (tabla, pantalla, filtro, campo) in
        [("escritos", "escritos", "escritos_causa", "cause_id"), ("historico_proteccion", "historico_proteccion", "historico_persona", "person_id")]
    {
        for fila in tablas.get(tabla).into_iter().flatten() {
            let q = consultas.get(dato(fila, "query_id")).ok_or(format!("Fila de {tabla} sin consulta."))?;
            let filtros: Value = serde_json::from_str(q["filters_json"].as_str().unwrap_or("")).unwrap_or(Value::Null);
            let alcance = &filtros[filtro];
            if q["screen"].as_str() != Some(pantalla) || !alcance.is_object() {
                return Err(format!("Fila de {tabla} sin consulta de su pantalla y alcance."));
            }
            if ["source_system", "court_code", campo].iter().any(|k| alcance[*k].as_str() != Some(dato(fila, k))) {
                return Err(format!("Fila de {tabla} fuera del alcance declarado por su consulta."));
            }
        }
    }
    Ok(())
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
