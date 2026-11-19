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

/// (día de la lectura tal como se registró, instante comparable en UTC) de `captured_at` (E3B-01; `escritos.py::_instante`).
fn instante(texto: &str) -> (String, String) {
    let texto = texto.trim();
    let dia: String = texto.chars().take(10).collect();
    let formato = "%Y-%m-%dT%H:%M:%S%.6f";
    let utc = chrono::DateTime::parse_from_rfc3339(texto)
        .map(|d| d.with_timezone(&chrono::Utc).naive_utc())
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(texto, "%Y-%m-%dT%H:%M:%S%.f"))
        .or_else(|_| NaiveDate::parse_from_str(texto, "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).unwrap()));
    match utc {
        Ok(v) => (dia, v.format(formato).to_string()),
        Err(_) => (dia, texto.to_string()),
    }
}

/// Filas normalizadas y ordenadas por consulta: para saber si dos lecturas del mismo instante dicen lo mismo (E3B-02). Es
/// un MULTICONJUNTO (E3C-02): dos escritos iguales no valen lo mismo que uno; el orden de las filas no importa.
fn firmas(tabla: &[FilaCsv], campos: &[&str]) -> HashMap<String, Vec<Vec<String>>> {
    let mut salida: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for f in tabla {
        salida.entry(dato(f, "query_id").to_string()).or_default().push(campos.iter().map(|k| clave_texto(dato(f, k))).collect());
    }
    for filas in salida.values_mut() {
        filas.sort();
    }
    salida
}

/// {clave: consulta} con la ÚLTIMA lectura fiable al corte por clave (E3-07, E3B-01, E3B-02; `escritos.py::_mejores_consultas`).
/// Se descartan las lecturas cuyo día es posterior al corte. Entre completas manda la más reciente en UTC; si varias comparten
/// ese instante y discrepan (cobertura o filas), la de menor `query_id` queda PARCIAL y marcada `_discordante`. Sin completas,
/// la de mejor cobertura, luego la más reciente y, ante empate, la de menor `query_id`.
fn mejores_consultas(
    manifiesto: &Value,
    pantalla: &str,
    filtro: &str,
    ultimo: &str,
    corte: Option<NaiveDate>,
    tabla: &[FilaCsv],
    campos: &[&str],
) -> HashMap<[String; 3], Value> {
    let mut grupos: HashMap<[String; 3], Vec<&Value>> = HashMap::new();
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
        grupos.entry(clave).or_default().push(consulta);
    }
    let firmas = firmas(tabla, campos);
    let vacio = Vec::new();
    let qid = |q: &Value| q["query_id"].as_str().unwrap_or("").to_string();
    let cuando = |q: &Value| instante(q["captured_at"].as_str().unwrap_or("")).1;
    let mut mejor = HashMap::new();
    for (clave, consultas) in grupos {
        let completas: Vec<&Value> =
            consultas.iter().copied().filter(|q| matches!(q["coverage"].as_str(), Some("COMPLETA" | "VACIA_COMPROBADA"))).collect();
        let elegida = if let Some(ultimo) = completas.iter().map(|q| cuando(q)).max() {
            let mut mismas: Vec<&Value> = completas.iter().copied().filter(|q| cuando(q) == ultimo).collect();
            mismas.sort_by_key(|q| qid(q));
            let mut e = mismas[0].clone();
            let discordantes = mismas.iter().any(|q| q["coverage"] != e["coverage"])
                || mismas.iter().any(|q| firmas.get(&qid(q)).unwrap_or(&vacio) != firmas.get(&qid(mismas[0])).unwrap_or(&vacio));
            if discordantes {
                e["coverage"] = Value::from("PARCIAL");
                e["_discordante"] = Value::from(true);
                e["_empatadas"] = Value::from(mismas.iter().map(|q| qid(q)).collect::<Vec<_>>());
            }
            e
        } else {
            let rango_max = consultas.iter().map(|q| rango(q["coverage"].as_str().unwrap_or(""))).max().unwrap_or(0);
            let candidatas: Vec<&Value> = consultas.iter().copied().filter(|q| rango(q["coverage"].as_str().unwrap_or("")) == rango_max).collect();
            let ultimo = candidatas.iter().map(|q| cuando(q)).max().unwrap_or_default();
            (*candidatas.iter().filter(|q| cuando(q) == ultimo).min_by_key(|q| qid(q)).unwrap()).clone()
        };
        mejor.insert(clave, elegida);
    }
    mejor
}

const FIRMA_ESCRITOS: [&str; 6] = ["escrito_type", "escrito_status", "doc", "requester", "linked_ingreso_id", "linked_projection"];
const FIRMA_HISTORICO: [&str; 5] = ["program_name", "egreso_date", "egreso_cause", "ownership", "person_id"];

pub(crate) fn filas_de_escritos(manifiesto: &Value, tabla: &[FilaCsv], corte: Option<NaiveDate>) -> HashMap<[String; 3], FuenteEscritos> {
    mejores_consultas(manifiesto, "escritos", "escritos_causa", "cause_id", corte, tabla, &FIRMA_ESCRITOS)
        .into_iter()
        .map(|(clave, mut consulta)| {
            let id = consulta["query_id"].as_str().unwrap_or("").to_string();
            let filas = tabla.iter().filter(|f| dato(f, "query_id") == id).cloned().collect();
            if consulta["_discordante"] == true {
                // Las filas de TODAS las lecturas empatadas: sus referencias quedan bloqueadas (E3C-01).
                let empatadas: Vec<String> =
                    consulta["_empatadas"].as_array().into_iter().flatten().filter_map(|q| q.as_str().map(String::from)).collect();
                let todas: Vec<&FilaCsv> = empatadas.iter().flat_map(|q| tabla.iter().filter(move |f| dato(f, "query_id") == q)).collect();
                consulta["_filas_empate"] = serde_json::to_value(todas).unwrap_or_default();
            }
            (clave, (consulta, filas))
        })
        .collect()
}

/// `Escritos` del motor para un registro de ingreso verificado, con la asociación explícita de ESE ingreso.
pub(crate) fn escritos_para(registro: &FilaCsv, fuentes: &HashMap<[String; 3], FuenteEscritos>) -> Option<Escritos> {
    let clave = ["source_system", "court_code", "cause_id"].map(|k| dato(registro, k).to_string());
    let (consulta, filas) = fuentes.get(&clave)?;
    // Lecturas simultáneas que se contradicen: ninguna asociación explícita vale (E3C-01); el motor pide revisión.
    let discordante = consulta["_discordante"] == true;
    let escritos = filas
        .iter()
        .enumerate()
        .map(|(n, f)| {
            let propio =
                dato(f, "linked_ingreso_id") == dato(registro, "ingreso_id") && !dato(f, "linked_projection").is_empty() && !discordante;
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
    Some(Escritos { cobertura: consulta["coverage"].as_str().unwrap_or("").to_string(), leido: dia(consulta["captured_at"].as_str().unwrap_or("")), filas: escritos, discordante })
}

pub(crate) fn historicos_por_persona(manifiesto: &Value, tabla: &[FilaCsv], corte: Option<NaiveDate>) -> HashMap<[String; 3], Historico> {
    mejores_consultas(manifiesto, "historico_proteccion", "historico_persona", "person_id", corte, tabla, &FIRMA_HISTORICO)
        .into_iter()
        .map(|(clave, consulta)| {
            let id = consulta["query_id"].as_str().unwrap_or("");
            let filas: Vec<&FilaCsv> = tabla.iter().filter(|f| dato(f, "query_id") == id).collect();
            let verificada = !filas.is_empty() && filas.iter().all(|f| dato(f, "ownership") == "VERIFICADA");
            let incierta = consulta["coverage"].as_str() == Some("IDENTIDAD_INCIERTA")
                || (!filas.is_empty() && !verificada)
                || consulta["_discordante"] == true;
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
    let (consulta, filas) = fuentes.get(&clave)?;
    if consulta["coverage"].as_str() == Some("IDENTIDAD_INCIERTA") {
        return None; // una lectura de identidad incierta no asocia nada (E3B-03)
    }
    let propio = (dato(registro, "ingreso_id").to_string(), vencimiento.to_string());
    let discordante = consulta["_discordante"] == true;
    let empate: Vec<FilaCsv> = serde_json::from_value(consulta["_filas_empate"].clone()).unwrap_or_default();
    // referencia documental -> dueños (ingreso, proyección); varios dueños = referencia ambigua (E3B-04)
    let mut docs: HashMap<String, std::collections::BTreeSet<(String, String)>> = HashMap::new();
    for f in if discordante { &empate } else { filas } {
        let referencia = clave_texto(dato(f, "doc"));
        if !dato(f, "linked_ingreso_id").is_empty() && !dato(f, "linked_projection").is_empty() && !referencia.is_empty() {
            let dueno = (dato(f, "linked_ingreso_id").to_string(), dato(f, "linked_projection").chars().take(10).collect());
            docs.entry(referencia).or_default().insert(dueno);
        }
    }
    let escritos: Vec<String> = ESCRITOS_DE_INFORME.iter().map(|n| clave_texto(n)).collect();
    let (mut folio_escrito, mut folio_resolucion, mut fecha_resolucion) = (String::new(), String::new(), None);
    let mut ajenos: Vec<String> = Vec::new();
    for fila in historia {
        let referencia = clave_texto(dato(fila, "reference"));
        let folio = dato(fila, "folio").trim().to_string();
        let Some(duenos) = docs.get(&referencia).filter(|_| !referencia.is_empty()) else { continue };
        if duenos.len() != 1 || !duenos.contains(&propio) || discordante {
            if !folio.is_empty() && !ajenos.contains(&folio) {
                ajenos.push(folio); // de otro informe, o ambigua (varios dueños)
            }
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

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;

    fn consulta(id: &str, cuando: &str, cobertura: &str) -> Value {
        let filtros = json!({"escritos_causa":{"source_system":"SITFA","court_code":"901","cause_id":"100"}}).to_string();
        json!({"query_id":id,"screen":"escritos","filters_json":filtros,"captured_at":cuando,"coverage":cobertura})
    }

    fn fila(campos: &[(&str, &str)]) -> FilaCsv {
        campos.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn registro() -> FilaCsv {
        fila(&[("source_system", "SITFA"), ("court_code", "901"), ("cause_id", "100"), ("ingreso_id", "201")])
    }

    #[test]
    fn e3b_01_instante_en_utc_y_dia_local() {
        assert_eq!(instante("2026-11-19T21:30:00-03:00"), ("2026-11-19".into(), "2026-11-20T00:30:00.000000".into()));
        assert_eq!(instante("2026-11-20T00:10:00Z").1, "2026-11-20T00:10:00.000000");
        assert_eq!(instante("2026-11-19T10:00:00").1, "2026-11-19T10:00:00.000000");
        assert_eq!(instante("2026-11-19").1, "2026-11-19T00:00:00.000000");
        // 21:30 en Santiago es posterior a 00:10 UTC del día siguiente: manda la lectura de Santiago.
        let m = json!({"queries":[consulta("q2", "2026-11-20T00:10:00Z", "COMPLETA"), consulta("q1", "2026-11-19T21:30:00-03:00", "COMPLETA")]});
        let mejor = mejores_consultas(&m, "escritos", "escritos_causa", "cause_id", None, &[], &FIRMA_ESCRITOS);
        assert_eq!(mejor.values().next().unwrap()["query_id"], "q1");
        // El corte usa el día escrito en la lectura.
        let corte = Some("2026-11-19".parse().unwrap());
        let mejor = mejores_consultas(&m, "escritos", "escritos_causa", "cause_id", corte, &[], &FIRMA_ESCRITOS);
        assert_eq!(mejor.values().next().unwrap()["query_id"], "q1");
    }

    #[test]
    fn e3b_02_empate_discordante_queda_parcial_en_cualquier_orden() {
        let tabla = vec![
            fila(&[("query_id", "q1"), ("escrito_type", "Inf. Cump FAE/FAS"), ("escrito_status", "Pendiente")]),
            fila(&[("query_id", "q2"), ("escrito_type", "Inf. Cump FAE/FAS"), ("escrito_status", "Resuelto")]),
        ];
        for orden in [["q1", "q2"], ["q2", "q1"]] {
            let m = json!({"queries":orden.map(|q| consulta(q, "2026-11-19T10:00:00-03:00", "COMPLETA"))});
            let e = mejores_consultas(&m, "escritos", "escritos_causa", "cause_id", None, &tabla, &FIRMA_ESCRITOS);
            let e = e.values().next().unwrap();
            assert_eq!((e["query_id"].as_str(), e["coverage"].as_str(), e["_discordante"].as_bool()), (Some("q1"), Some("PARCIAL"), Some(true)));
        }
        // Mismo contenido: no hay discordancia.
        let iguales = vec![fila(&[("query_id", "q1"), ("doc", "A")]), fila(&[("query_id", "q2"), ("doc", "a")])];
        let m = json!({"queries":[consulta("q2", "2026-11-19T10:00:00-03:00", "COMPLETA"), consulta("q1", "2026-11-19T10:00:00-03:00", "COMPLETA")]});
        let e = mejores_consultas(&m, "escritos", "escritos_causa", "cause_id", None, &iguales, &FIRMA_ESCRITOS);
        assert_eq!(e.values().next().unwrap()["coverage"], "COMPLETA");
    }

    fn historia(referencia: &str, folio: &str, descripcion: &str) -> FilaCsv {
        fila(&[("reference", referencia), ("folio", folio), ("description", descripcion), ("event_at", "2026-10-01")])
    }

    #[test]
    fn e3b_03_identidad_incierta_no_asocia() {
        let filas = vec![fila(&[("doc", "D-1"), ("linked_ingreso_id", "201"), ("linked_projection", "2026-10-15")])];
        let mut fuentes = HashMap::new();
        let clave = ["SITFA", "901", "100"].map(String::from);
        fuentes.insert(clave.clone(), (consulta("q1", "2026-11-19", "IDENTIDAD_INCIERTA"), filas.clone()));
        let h = historia("D-1", "31", "Inf. Cump FAE/FAS");
        let vence = Some("2026-10-15".parse().unwrap());
        assert!(evidencia_informe(&registro(), vence, &fuentes, &[&h]).is_none());
        fuentes.insert(clave, (consulta("q1", "2026-11-19", "COMPLETA"), filas));
        assert_eq!(evidencia_informe(&registro(), vence, &fuentes, &[&h]).unwrap().escrito_historia_folio, "31");
    }

    #[test]
    fn e3b_04_referencia_con_varios_duenos_no_se_atribuye() {
        let filas = vec![
            fila(&[("doc", "D-1"), ("linked_ingreso_id", "201"), ("linked_projection", "2026-10-15")]),
            fila(&[("doc", "D-1"), ("linked_ingreso_id", "202"), ("linked_projection", "2026-10-15")]),
        ];
        let mut fuentes = HashMap::new();
        fuentes.insert(["SITFA", "901", "100"].map(String::from), (consulta("q1", "2026-11-19", "COMPLETA"), filas));
        let h = historia("D-1", "31", "Inf. Cump FAE/FAS");
        let e = evidencia_informe(&registro(), Some("2026-10-15".parse().unwrap()), &fuentes, &[&h]).unwrap();
        assert_eq!((e.escrito_historia_folio.as_str(), e.folios_ajenos.clone()), ("", vec!["31".to_string()]));
    }

    fn fila_es(q: &str, extra: &[(&str, &str)]) -> FilaCsv {
        let mut f = fila(&[("query_id", q), ("escrito_type", "Inf. Cump FAE/FAS"), ("escrito_status", "Pendiente"), ("requester", "P")]);
        f.extend(extra.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        f
    }

    /// Espejo de `test_e3c_02_uno_frente_a_dos_escritos_iguales_es_discordancia`.
    #[test]
    fn e3c_02_multiconjunto() {
        let m = json!({"queries":[consulta("q1", "2026-11-19T10:00:00-03:00", "COMPLETA"), consulta("q2", "2026-11-19T10:00:00-03:00", "COMPLETA")]});
        let elegir = |tabla: Vec<FilaCsv>| {
            mejores_consultas(&m, "escritos", "escritos_causa", "cause_id", None, &tabla, &FIRMA_ESCRITOS).into_values().next().unwrap()["coverage"]
                .as_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(elegir(vec![fila_es("q1", &[]), fila_es("q2", &[]), fila_es("q2", &[])]), "PARCIAL");
        let otra = [("requester", "Q")];
        assert_eq!(elegir(vec![fila_es("q1", &[]), fila_es("q1", &otra), fila_es("q2", &otra), fila_es("q2", &[])]), "COMPLETA");
    }

    /// Espejo de `test_e3c_01_folio_de_lecturas_discordantes_queda_bloqueado`.
    #[test]
    fn e3c_01_discordancia_no_atribuye_ni_vincula() {
        let m = json!({"queries":[consulta("q1", "2026-11-19T10:00:00-03:00", "COMPLETA"), consulta("q2", "2026-11-19T10:00:00-03:00", "COMPLETA")]});
        let propio = [("doc", "D"), ("linked_ingreso_id", "201"), ("linked_projection", "2026-10-15")];
        let mut resuelto = propio.to_vec();
        resuelto.push(("escrito_status", "Resuelto"));
        let tabla = vec![fila_es("q1", &propio), fila_es("q2", &resuelto)];
        let fuentes = filas_de_escritos(&m, &tabla, None);
        let h = historia("D", "31", "Inf. Cump FAE/FAS");
        let e = evidencia_informe(&registro(), Some("2026-10-15".parse().unwrap()), &fuentes, &[&h]).unwrap();
        assert_eq!((e.escrito_historia_folio.as_str(), e.folios_ajenos.clone()), ("", vec!["31".to_string()]));
        let esc = escritos_para(&registro(), &fuentes).unwrap();
        assert!(esc.discordante);
        assert_eq!(esc.filas.iter().map(|f| f.vinculo.as_str()).collect::<Vec<_>>(), [""]);
    }
}
