//! De punta a punta con los libros ficticios de `docs/evidencia_p0/golden`: leer el Excel y analizarlo debe dar
//! las observaciones, reglas y acciones que registró el Asistente Python en `semantic.json`.

use flow_dominio::Etapa;
use flow_motor::Config;
use flow_trabajo::{analizar_tabla, Contexto};
use serde_json::Value;

fn golden() -> String {
    format!("{}/../../../docs/evidencia_p0/golden", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn libros_golden_dan_lo_mismo_que_python() {
    let semantico: Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{}/semantic.json", golden())).unwrap()).unwrap();
    let corte = semantico["as_of"].as_str().unwrap().parse().unwrap();
    let config = Config::default();
    for etapa in [Etapa::Espera, Etapa::Cumplimiento, Etapa::Informes] {
        let ruta = format!("{}/Origen_{}.xlsx", golden(), etapa.nombre());
        let libro = flow_tabular::leer(std::path::Path::new(&ruta)).unwrap();
        assert_eq!(libro.sha256, semantico["modes"][etapa.nombre()]["source_sha256"].as_str().unwrap());
        let tabla = flow_tabular::leer_tabla(&libro, None, None).unwrap();
        assert_eq!(tabla.etapa, etapa, "la etapa se deduce del nombre de la hoja");
        let trabajo = analizar_tabla(&libro, &tabla, corte, &config, &Contexto::default());
        let esperadas = semantico["modes"][etapa.nombre()]["rows"].as_array().unwrap();
        assert_eq!(trabajo.filas.len(), esperadas.len(), "{etapa:?}");
        for (fila, esperada) in trabajo.filas.iter().zip(esperadas) {
            let lista = |campo: &str| -> Vec<String> {
                esperada[campo].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect()
            };
            assert_eq!(fila.propuesta.observacion, esperada["observation"].as_str().unwrap(), "{etapa:?} fila {}", fila.numero);
            assert_eq!(fila.propuesta.reglas, lista("rules"), "{etapa:?} fila {}", fila.numero);
            assert_eq!(fila.propuesta.acciones, lista("actions"), "{etapa:?} fila {}", fila.numero);
            assert_eq!(fila.propuesta.avisos, lista("warnings"), "{etapa:?} fila {}", fila.numero);
            assert_eq!(fila.propuesta.excluido, esperada["excluded"].as_bool().unwrap());
        }
    }
}
