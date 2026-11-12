//! Prueba diferencial: el motor Rust debe responder exactamente lo mismo que el motor Python
//! sobre los casos ficticios de `rust/casos/` (generados por `rust/oraculo/generar_casos.py`).

use chrono::NaiveDate;
use flow_dominio::{Celda, Etapa, Historia, Observacion, Registro, Tramite};
use flow_motor::{analizar, Config, Duplicidad, Entrada, Marca, Resultado};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct TramiteCaso {
    desc: String,
    fecha: NaiveDate,
    folio_txt: String,
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
            let tramites = h.tramites.iter().map(|t| Tramite::nuevo(&t.desc, t.fecha, &t.folio_txt)).collect();
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
        });
        let e = &caso.esperado;
        let esperado = Resultado {
            observacion: e.observacion.clone(),
            reglas: e.reglas.clone(),
            acciones: e.acciones.clone(),
            avisos: e.avisos.clone(),
            marcas: e.marcas.clone(),
            excluido: e.excluido,
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
