//! Prueba diferencial: el motor Rust debe responder exactamente lo mismo que el motor Python
//! sobre los casos ficticios de `rust/casos/` (generados por `rust/oraculo/generar_casos.py`).

use chrono::NaiveDate;
use flow_dominio::{Celda, Escritos, Etapa, EvidenciaInforme, Historia, Historico, Observacion, Registro, Tramite};
use flow_motor::{analizar, Config, Duplicidad, Entrada, Marca, OtroPrograma, Resultado};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct TramiteCaso {
    desc: String,
    fecha: NaiveDate,
    folio_txt: String,
    /// Columna «Trámite» de SITFA (Resolución, Escrito, Actuación).
    #[serde(default)]
    tipo: String,
}

#[derive(Deserialize)]
struct HistoriaCaso {
    cobertura: String,
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    tramites: Vec<TramiteCaso>,
}

#[derive(Deserialize)]
struct Esperado {
    observacion: String,
    reglas: Vec<String>,
    acciones: Vec<String>,
    avisos: Vec<String>,
    marcas: Vec<Marca>,
    excluido: bool,
    #[serde(default)]
    otros_programas: Vec<OtroPrograma>,
}

#[derive(Deserialize)]
struct Caso {
    id: String,
    etapa: Etapa,
    corte: NaiveDate,
    desactivadas: Vec<String>,
    campos: BTreeMap<String, Celda>,
    historia: Option<HistoriaCaso>,
    bitacora: Vec<Observacion>,
    duplicidad: Option<Duplicidad>,
    acumulado: Option<(i64, NaiveDate)>,
    fecha_hoja2: Option<NaiveDate>,
    /// Integración 18-nov-2026 (los casos anteriores no la traen).
    #[serde(default)]
    escritos: Option<Escritos>,
    #[serde(default)]
    historico: Option<Historico>,
    #[serde(default)]
    evidencia: Option<EvidenciaInforme>,
    #[serde(default)]
    consignados: Vec<String>,
    #[serde(default)]
    bitacora_parcial: bool,
    esperado: Esperado,
}

fn comparar(archivo: &str) {
    let ruta = format!("{}/../../casos/{archivo}", env!("CARGO_MANIFEST_DIR"));
    let casos: Vec<Caso> = serde_json::from_str(&std::fs::read_to_string(&ruta).expect(&ruta)).expect("casos");
    assert!(casos.len() >= 100, "pocos casos en {archivo}");
    let mut fallas = Vec::new();
    for caso in &casos {
        let mut config = Config::default();
        config.desactivadas = caso.desactivadas.iter().cloned().collect();
        let registro = Registro { campos: caso.campos.clone() };
        let historia = caso.historia.as_ref().map(|h| {
            let tramites = h.tramites.iter().map(|t| Tramite::nuevo(&t.desc, t.fecha, &t.folio_txt).con_tipo(&t.tipo)).collect();
            Historia::nueva(&h.cobertura, h.desde, h.hasta, tramites)
        });
        let obtenido = analizar(&Entrada {
            etapa: caso.etapa,
            registro: &registro,
            corte: caso.corte,
            config: &config,
            historia: historia.as_ref(),
            bitacora: &caso.bitacora,
            duplicidad: caso.duplicidad,
            acumulado: caso.acumulado,
            fecha_hoja2: caso.fecha_hoja2,
            escritos: caso.escritos.as_ref(),
            historico: caso.historico.as_ref(),
            evidencia: caso.evidencia.as_ref(),
            consignados: &caso.consignados,
            bitacora_parcial: caso.bitacora_parcial,
        });
        let e = &caso.esperado;
        let esperado = Resultado {
            observacion: e.observacion.clone(),
            reglas: e.reglas.clone(),
            acciones: e.acciones.clone(),
            avisos: e.avisos.clone(),
            marcas: e.marcas.clone(),
            excluido: e.excluido,
            otros_programas: e.otros_programas.clone(),
        };
        if obtenido != esperado {
            fallas.push(format!("{}\n  esperado: {esperado:?}\n  obtenido: {obtenido:?}", caso.id));
        }
    }
    assert!(
        fallas.is_empty(),
        "{} de {} casos difieren en {archivo}. Primeros:\n{}",
        fallas.len(),
        casos.len(),
        fallas.iter().take(4).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn espera_igual_que_python() {
    comparar("espera.json");
}

#[test]
fn cumplimiento_igual_que_python() {
    comparar("cumplimiento.json");
}

#[test]
fn informes_igual_que_python() {
    comparar("informes.json");
}

#[test]
fn integracion_igual_que_python() {
    comparar("integracion.json");
}
