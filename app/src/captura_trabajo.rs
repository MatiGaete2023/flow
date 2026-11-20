//! Adapta las filas normalizadas del paquete a las hojas de Trabajo.

use crate::lectura::{self, Sheet};
use crate::motor::{Cols, Mode};
use crate::util::Cell;
use flow_rust::captura::{self, Captura, Fila};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

const LIMITE_ANALISIS: usize = 20_000;
const PROCEDENCIA: [(&str, &str); 5] = [
    ("SITFA_ARCHIVO", "original_file"),
    ("SITFA_HOJA", "original_sheet"),
    ("SITFA_FILA", "original_row"),
    ("SITFA_SHA256", "source_sha256"),
    ("ID_REGISTRO_CAPTURA", "record_id"),
];

pub struct PaqueteTrabajo {
    pub fuente: Captura,
    pub modos: Vec<Mode>,
    pub columnas_por_fila: Vec<Vec<Cols>>,
    pub omitidos: usize,
    /// Escritos y Mejor Niñez por `record_id` del paquete.
    pub contextos: HashMap<String, ContextoFila>,
}

pub fn es_zip(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

pub fn abrir(bytes: &[u8]) -> Result<(PaqueteTrabajo, Vec<Sheet>), String> {
    let fuente = captura::leer_captura(bytes)?;
    let calendarios: HashSet<_> = fuente
        .filas("consultas")
        .filter(|q| q.get("screen") == "calendario_informes")
        .map(|q| q.get("query_id"))
        .collect();
    let hay_seguimiento = fuente
        .filas("registros")
        .any(|r| !calendarios.contains(r.get("query_id")));
    let filas = fuente
        .filas("registros")
        .filter(|r| !hay_seguimiento || !calendarios.contains(r.get("query_id")));
    let mut grupos = HashMap::<Mode, Vec<Fila<'_>>>::new();
    let mut omitidos = 0;
    for (i, fila) in filas.enumerate() {
        if i >= LIMITE_ANALISIS {
            return Err(
                "La captura supera 20.000 registros de trabajo. Divide la exportación en lotes."
                    .into(),
            );
        }
        let modo = match fila.get("record_type") {
            "ESPERA" => Mode::Espera,
            "CUMPLIMIENTO" => Mode::Cumplimiento,
            "INFORMES" => Mode::Informes,
            _ => {
                omitidos += 1;
                continue;
            }
        };
        grupos.entry(modo).or_default().push(fila);
    }
    let mut modos = Vec::new();
    let mut hojas = Vec::new();
    let mut columnas_por_fila = Vec::new();
    for modo in Mode::ALL {
        if let Some(filas) = grupos.remove(&modo) {
            let (hoja, columnas) = hoja(modo, &filas)?;
            modos.push(modo);
            hojas.push(hoja);
            columnas_por_fila.push(columnas);
        }
    }
    if hojas.is_empty() {
        return Err(
            "El paquete no incluye registros de Espera, Cumplimiento o Informes para Trabajo."
                .into(),
        );
    }
    Ok((
        PaqueteTrabajo {
            contextos: contextos(&fuente),
            fuente,
            modos,
            columnas_por_fila,
            omitidos,
        },
        hojas,
    ))
}

fn hoja(modo: Mode, filas: &[Fila<'_>]) -> Result<(Sheet, Vec<Cols>), String> {
    let columnas = lectura::columnas_canonicas(modo);
    let mut celdas = Vec::with_capacity(filas.len() + 1);
    celdas.push(
        columnas
            .iter()
            .map(|&(_, titulo)| Cell::Text(titulo.into()))
            .chain(
                PROCEDENCIA
                    .iter()
                    .map(|&(titulo, _)| Cell::Text(titulo.into())),
            )
            .collect(),
    );
    let mut columnas_por_fila = Vec::with_capacity(filas.len());
    for &fila in filas {
        let mapa: serde_json::Map<String, Value> =
            serde_json::from_str(fila.get("normalized_json"))
                .map_err(|_| "No se pudo leer un registro normalizado.")?;
        let encabezados: Vec<_> = mapa.keys().cloned().collect();
        let origen = lectura::map_columns(&encabezados, modo);
        let mut disponibles = Cols::new();
        let mut valores = Vec::with_capacity(columnas.len() + PROCEDENCIA.len());
        for (i, &(clave, _)) in columnas.iter().enumerate() {
            let valor = origen.get(clave).map(|&idx| &mapa[&encabezados[idx]]);
            if valor.is_some() {
                disponibles.insert(clave, i);
            }
            valores.push(celda(valor));
        }
        valores.extend(
            PROCEDENCIA
                .iter()
                .map(|&(_, campo)| Cell::Text(fila.get(campo).into())),
        );
        columnas_por_fila.push(disponibles);
        celdas.push(valores);
    }
    Ok((
        Sheet {
            name: modo.label().into(),
            rows: celdas,
        },
        columnas_por_fila,
    ))
}

fn celda(valor: Option<&Value>) -> Cell {
    match valor {
        None | Some(Value::Null) => Cell::Empty,
        Some(Value::String(s)) => Cell::Text(s.clone()),
        Some(Value::Number(n)) => n.as_f64().map_or(Cell::Empty, Cell::Number),
        Some(Value::Bool(b)) => Cell::Bool(*b),
        Some(v) => Cell::Text(v.to_string()),
    }
}

/// Escritos de la causa leídos en el paquete (`revision_bitacoras.resumen_escritos`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResumenEscritos {
    pub cobertura: String,
    pub leido: String,
    pub filas: usize,
    /// «Tipo · documento» de cada escrito pendiente, sin los vinculados expresamente a otro ingreso.
    pub pendientes: Vec<String>,
}

/// Ingreso Histórico Protección (Mejor Niñez) de la persona (`escritos.historicos_por_persona`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResumenHistorico {
    pub cobertura: String,
    pub leido: String,
    pub verificada: bool,
    /// Programas vigentes: fecha de egreso «--» y causal «SIN INFORMACION» en la misma fila.
    pub vigentes: Vec<String>,
    pub filas: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextoFila {
    pub escritos: Option<ResumenEscritos>,
    pub historico: Option<ResumenHistorico>,
}

fn clave_texto(t: &str) -> String {
    crate::util::normalizar(&t.replace('\u{a0}', " "))
}

fn rango(cobertura: &str) -> u8 {
    match cobertura {
        "COMPLETA" => 3,
        "PARCIAL" | "IDENTIDAD_INCIERTA" => 2,
        "VACIA_COMPROBADA" => 1,
        _ => 0,
    }
}

/// `_mejores_consultas`, simplificada: por clave, la lectura completa (o vacía comprobada) más reciente; sin ellas, la de
/// mejor cobertura y más reciente; empate, menor `query_id`. Pendiente: lecturas simultáneas discordantes (E3B-02).
fn mejores_consultas(fuente: &Captura, pantalla: &str, filtro: &str, campo: &str) -> HashMap<(String, String, String), (String, String, String)> {
    let mut grupos: HashMap<(String, String, String), Vec<(String, String, String)>> = HashMap::new();
    for q in fuente.filas("consultas").filter(|q| q.get("screen") == pantalla) {
        let Ok(filtros) = serde_json::from_str::<Value>(q.get("filters_json")) else { continue };
        let alcance = &filtros[filtro];
        if !alcance.is_object() {
            continue;
        }
        let parte = |k: &str| alcance[k].as_str().unwrap_or("").to_string();
        grupos
            .entry((parte("source_system"), parte("court_code"), parte(campo)))
            .or_default()
            .push((q.get("query_id").into(), q.get("coverage").into(), q.get("captured_at").into()));
    }
    grupos
        .into_iter()
        .filter_map(|(clave, consultas)| {
            let completas: Vec<_> = consultas.iter().filter(|q| matches!(q.1.as_str(), "COMPLETA" | "VACIA_COMPROBADA")).collect();
            let candidatas: Vec<_> = if completas.is_empty() {
                let mejor = consultas.iter().map(|q| rango(&q.1)).max()?;
                consultas.iter().filter(|q| rango(&q.1) == mejor && mejor > 0).collect()
            } else {
                completas
            };
            let ultimo = candidatas.iter().map(|q| q.2.as_str()).max()?;
            let elegida = candidatas.iter().filter(|q| q.2 == ultimo).min_by(|a, b| a.0.cmp(&b.0))?;
            Some((clave, (*elegida).clone()))
        })
        .collect()
}

fn dia(instante: &str) -> String {
    instante.get(..10).unwrap_or(instante).to_string()
}

/// Escritos y Mejor Niñez de cada registro del paquete, por causa y persona del registro.
fn contextos(fuente: &Captura) -> HashMap<String, ContextoFila> {
    let mut escritos_por_consulta: HashMap<String, Vec<Fila<'_>>> = HashMap::new();
    for f in fuente.filas("escritos") {
        escritos_por_consulta.entry(f.get("query_id").into()).or_default().push(f);
    }
    let mut historico_por_consulta: HashMap<String, Vec<Fila<'_>>> = HashMap::new();
    for f in fuente.filas("historico_proteccion") {
        historico_por_consulta.entry(f.get("query_id").into()).or_default().push(f);
    }
    let escritos = mejores_consultas(fuente, "escritos", "escritos_causa", "cause_id");
    let historicos = mejores_consultas(fuente, "historico_proteccion", "historico_persona", "person_id");
    let mut salida = HashMap::new();
    for r in fuente.filas("registros") {
        let sistema = r.get("source_system").to_string();
        let tribunal = r.get("court_code").to_string();
        let mut ctx = ContextoFila::default();
        if let Some((id, cobertura, leido)) = escritos.get(&(sistema.clone(), tribunal.clone(), r.get("cause_id").to_string())) {
            let filas = escritos_por_consulta.get(id).map(Vec::as_slice).unwrap_or(&[]);
            let pendientes = filas
                .iter()
                .filter(|e| clave_texto(e.get("escrito_status")) == "pendiente")
                .filter(|e| e.get("linked_ingreso_id").is_empty() || e.get("linked_ingreso_id") == r.get("ingreso_id"))
                .map(|e| format!("{} · {}", e.get("escrito_type"), e.get("doc")))
                .collect();
            ctx.escritos = Some(ResumenEscritos { cobertura: cobertura.clone(), leido: dia(leido), filas: filas.len(), pendientes });
        }
        if let Some((id, cobertura, leido)) = historicos.get(&(sistema, tribunal, r.get("person_id").to_string())) {
            let filas = historico_por_consulta.get(id).map(Vec::as_slice).unwrap_or(&[]);
            let verificada = cobertura != "IDENTIDAD_INCIERTA" && filas.iter().all(|f| f.get("ownership") == "VERIFICADA");
            let mut vigentes: Vec<String> = Vec::new();
            for f in filas {
                let sin_fecha = f.get("egreso_date").trim().trim_matches(|c: char| "-\u{2010}\u{2011}\u{2012}\u{2013}\u{2014}\u{2212}".contains(c)).is_empty();
                let nombre = f.get("program_name").split_whitespace().collect::<Vec<_>>().join(" ");
                if sin_fecha && clave_texto(f.get("egreso_cause")) == "sin informacion" && !vigentes.contains(&nombre) {
                    vigentes.push(nombre);
                }
            }
            ctx.historico = Some(ResumenHistorico { cobertura: cobertura.clone(), leido: dia(leido), verificada, vigentes, filas: filas.len() });
        }
        if ctx != ContextoFila::default() {
            salida.insert(r.get("record_id").to_string(), ctx);
        }
    }
    salida
}
