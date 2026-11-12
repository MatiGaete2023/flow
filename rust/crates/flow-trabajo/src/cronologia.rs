//! Vista de bitácoras y avisos CT: traducción de temporal.py y bitacora_avisos.py, sin decidir actuaciones.
use chrono::{DateTime, Months};
use flow_dominio::{
    texto::{ascii, fecha_es, normalizar_columna},
    Fecha,
};
use flow_motor::{Config, Resultado};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

fn texto<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn dia(v: &Value) -> Option<Fecha> {
    texto(v, "event_at").get(..10)?.parse().ok()
}
fn centro(v: &Value) -> bool {
    v["origin"] == "CENTRO" && v["origin_verified"] == "true" && matches!(texto(v, "coverage"), "COMPLETA" | "PARCIAL")
}
fn momento(v: &Value) -> Option<DateTime<chrono::FixedOffset>> {
    if v["time_precision"] != "SEGUNDO" || matches!(texto(v, "time_zone"), "" | "DESCONOCIDA") {
        return None;
    }
    DateTime::parse_from_rfc3339(texto(v, "event_at")).ok()
}
fn normal(s: &str) -> String {
    ascii(s).to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// La misma normalización de export_csmp_capture.py; la hora de SITFA no aporta una zona cierta.
pub fn desde_lectura(lectura: &Value) -> Vec<Value> {
    lectura["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| {
            let cuando = texto(e, "fecha");
            let solo_dia = cuando.ends_with("T00:00:00");
            json!({"event_id":texto(e,"entry_id"),"event_at":if solo_dia{cuando.get(..10).unwrap_or("")}else{cuando},
            "time_precision":if solo_dia{"DIA"}else{"SEGUNDO"},"time_zone":"DESCONOCIDA",
            "origin":if e["origen"]=="centro"{"CENTRO"}else{"DESCONOCIDO"},"origin_verified":if e["origen"]=="centro"{"true"}else{"false"},
            "coverage":texto(lectura,"coverage"),"author":texto(e,"autor"),"event_type":texto(e,"tipo"),"text":texto(e,"texto"),
            "response":texto(e,"respuesta"),"response_at":texto(e,"fecha_respuesta"),"identity_status":"VERIFICADA"})
        })
        .collect()
}

pub fn mostrar(eventos: &[Value], corte: Fecha, solicitada: bool, leida: bool) -> Value {
    let desde = corte.checked_sub_months(Months::new(4)).unwrap();
    let mut recientes = eventos.iter().filter(|e| dia(e).is_some_and(|d| desde <= d && d <= corte)).cloned().collect::<Vec<_>>();
    recientes.sort_by(|a, b| (texto(a, "event_at"), texto(a, "event_id")).cmp(&(texto(b, "event_at"), texto(b, "event_id"))));
    let centros = recientes.iter().filter(|e| centro(e)).cloned().collect::<Vec<_>>();
    let mut ultimas = vec![];
    if let Some(ultimo_dia) = centros.iter().filter_map(dia).max() {
        ultimas = centros.iter().filter(|e| dia(e) == Some(ultimo_dia)).cloned().collect();
        if ultimas.iter().all(|e| momento(e).is_some()) {
            let max = ultimas.iter().filter_map(momento).max();
            ultimas.retain(|e| momento(e) == max);
        }
    }
    let ids = ultimas.iter().map(|e| texto(e, "event_id")).collect::<HashSet<_>>();
    let (mut posteriores, mut inciertas) = (vec![], vec![]);
    if let Some(fecha) = ultimas.first().and_then(dia) {
        for e in &recientes {
            if ids.contains(texto(e, "event_id")) {
                continue;
            }
            if dia(e) > Some(fecha) {
                posteriores.push(e.clone());
            } else if dia(e) == Some(fecha) {
                if momento(e).is_some() && ultimas.iter().all(|e| momento(e).is_some()) {
                    if momento(e) > ultimas.iter().filter_map(momento).max() {
                        posteriores.push(e.clone());
                    }
                } else {
                    inciertas.push(e.clone());
                }
            }
        }
    }
    let pendientes = centros
        .iter()
        .filter(|e| {
            normal(texto(e, "event_type")) == "al tribunal"
                && texto(e, "response").trim().is_empty()
                && dia(e).is_some_and(|d| (corte - d).num_days() >= 30)
        })
        .cloned()
        .collect::<Vec<_>>();
    let estado = if !ultimas.is_empty() {
        "ACREDITADA"
    } else if !recientes.is_empty() {
        "ORIGEN_DESCONOCIDO"
    } else if !solicitada && eventos.is_empty() {
        "NO_SOLICITADA"
    } else if !leida && eventos.is_empty() {
        "NO_LEIDA"
    } else {
        "SIN_ENTRADAS_CAPTURADAS"
    };
    json!({"version":"solo-mostrar-4","period_from":desde.to_string(),"period_to":corte.to_string(),"status":estado,
        "last_center":ultimas,"later":posteriores,"uncertain":inciertas,"unattributed":if estado=="ACREDITADA"{vec![]}else{recientes},"center_period":centros,"pending_court":pendientes,
        "outside_horizon":eventos.iter().filter(|e|dia(e).is_some_and(|d|d<desde)).count(),"after_cutoff":eventos.iter().filter(|e|dia(e).is_some_and(|d|d>corte)).count(),"undated":eventos.iter().filter(|e|dia(e).is_none()).count()})
}

fn cuerpo(s: &str) -> String {
    let t = normalizar_columna(s);
    match t.split_once(": ") {
        Some((head, rest)) if head.chars().count() <= 40 => rest.to_string(),
        _ => t,
    }
}

/// difflib.SequenceMatcher(..., autojunk=False), sin elementos junk: conserva los desempates de Python.
pub fn similitud(a: &str, b: &str) -> f64 {
    let a = a.chars().collect::<Vec<_>>();
    let b = b.chars().collect::<Vec<_>>();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let mut b2j: HashMap<char, Vec<usize>> = HashMap::new();
    for (j, c) in b.iter().enumerate() {
        b2j.entry(*c).or_default().push(j);
    }
    let mut pila = vec![(0, a.len(), 0, b.len())];
    let mut coincidencias = 0;
    while let Some((alo, ahi, blo, bhi)) = pila.pop() {
        let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0);
        let mut previo: HashMap<usize, usize> = HashMap::new();
        for (i, c) in a.iter().enumerate().take(ahi).skip(alo) {
            let mut actual = HashMap::new();
            for &j in b2j.get(c).into_iter().flatten().filter(|&&j| blo <= j && j < bhi) {
                let k = if j == 0 { 1 } else { previo.get(&(j - 1)).copied().unwrap_or(0) + 1 };
                actual.insert(j, k);
                if k > bestsize {
                    besti = i + 1 - k;
                    bestj = j + 1 - k;
                    bestsize = k;
                }
            }
            previo = actual;
        }
        if bestsize > 0 {
            coincidencias += bestsize;
            if alo < besti && blo < bestj {
                pila.push((alo, besti, blo, bestj));
            }
            if besti + bestsize < ahi && bestj + bestsize < bhi {
                pila.push((besti + bestsize, ahi, bestj + bestsize, bhi));
            }
        }
    }
    2.0 * coincidencias as f64 / (a.len() + b.len()) as f64
}

fn lista<'a>(display: &'a Value, k: &str) -> impl Iterator<Item = &'a Value> {
    display[k].as_array().into_iter().flatten()
}
fn repite<'a>(display: &'a Value, propuesta: &str) -> Option<&'a Value> {
    if display["status"] != "ACREDITADA" {
        return None;
    }
    let cuerpo = cuerpo(propuesta);
    if cuerpo.is_empty() {
        return None;
    }
    lista(display, "last_center").find(|e| similitud(&cuerpo, &self::cuerpo(texto(e, "text"))) >= 0.92)
}
fn fecha_corta(e: &Value) -> String {
    dia(e).map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_default()
}

pub fn avisos(display: &Value, propuesta: &str, reglas: &[String], config: &Config, corte: Fecha) -> Vec<(String, String)> {
    if display["status"] != "ACREDITADA" {
        return vec![];
    }
    let mut result = vec![];
    if let Some(e) = repite(display, propuesta) {
        result.push((
            "CT-07".into(),
            format!("La observación propuesta repite la última del centro ({}); revisar antes de registrarla.", fecha_corta(e)),
        ));
    }
    if reglas.iter().any(|r| r == "ESPERA.E05_SOLO_CORREO") {
        let correo = cuerpo(config.textos.get("ESPERA.E05_SOLO_CORREO").map(String::as_str).unwrap_or(""));
        let correo = correo.trim_end_matches(['.', ' ']);
        if let Some(e) = lista(display, "center_period")
            .filter(|e| {
                !correo.is_empty() && cuerpo(texto(e, "text")).contains(correo) && dia(e).is_some_and(|d| (corte - d).num_days() >= 30)
            })
            .max_by_key(|e| dia(e))
        {
            result.push((
                "CT-11".into(),
                format!(
                    "El correo de E-05 ya se envió el {} (hace {} días) y sigue sin ingreso efectivo; evaluar proyecto de resolución.",
                    fecha_corta(e),
                    (corte - dia(e).unwrap()).num_days()
                ),
            ));
        }
    }
    for e in lista(display, "pending_court") {
        if let Some(d) = dia(e) {
            result.push((
                "CT-05".into(),
                format!("Observación al tribunal del {} sin respuesta registrada (hace {} días).", fecha_corta(e), (corte - d).num_days()),
            ));
        }
    }
    result
}

pub fn reiterar(propuesta: &mut Resultado, display: &Value, config: &Config) {
    let regla = "COMUN.REITERA_OBSERVACION";
    if config.desactivadas.contains(regla) {
        return;
    }
    let Some(e) = repite(display, &propuesta.observacion) else {
        return;
    };
    let Some(plantilla) = config.textos.get(regla) else {
        return;
    };
    let anterior = &propuesta.observacion;
    let prefijo = match anterior.split_once(": ") {
        Some((head, _)) if head.chars().count() <= 40 => format!("{head}: "),
        _ => String::new(),
    };
    let texto = plantilla.replace("{FECHA_OBSERVACION_ANTERIOR}", &fecha_es(dia(e).unwrap()));
    propuesta.avisos.push(format!(
        "CT-07: la observación propuesta repetía la del {}; se consigna la reiteración. Texto propuesto originalmente: {anterior}",
        fecha_corta(e)
    ));
    propuesta.observacion = prefijo + &texto;
    propuesta.reglas.push(regla.into());
}
