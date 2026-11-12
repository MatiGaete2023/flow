use flow_motor::{Config, Resultado};
use flow_trabajo::cronologia;
use serde_json::{json, Value};

#[test]
fn cronologia_avisos_y_reiteracion_son_los_de_python() {
    let oraculo: Value = serde_json::from_str(include_str!("datos/cronologia.json")).unwrap();
    for (i, c) in oraculo["casos"].as_array().unwrap().iter().enumerate() {
        let corte = c["corte"].as_str().unwrap().parse().unwrap();
        let display =
            cronologia::mostrar(c["eventos"].as_array().unwrap(), corte, c["solicitada"].as_bool().unwrap(), c["leida"].as_bool().unwrap());
        assert_eq!(display, c["display"], "cronología {i}");
        let config = Config { desactivadas: serde_json::from_value(c["desactivadas"].clone()).unwrap(), ..Default::default() };
        let mut resultado = Resultado {
            observacion: c["propuesta"].as_str().unwrap().into(),
            reglas: serde_json::from_value(c["reglas"].clone()).unwrap(),
            acciones: vec!["programa_espera".into()],
            avisos: vec![],
            marcas: vec![],
            excluido: false,
        };
        let avisos = cronologia::avisos(&display, &resultado.observacion, &resultado.reglas, &config, corte);
        assert_eq!(json!(avisos), c["avisos"], "avisos {i}");
        cronologia::reiterar(&mut resultado, &display, &config);
        assert_eq!(
            json!({"observacion":resultado.observacion,"reglas":resultado.reglas,"acciones":resultado.acciones,"avisos":resultado.avisos}),
            c["reiteracion"],
            "reiteración {i}"
        );
    }
}

#[test]
fn similitud_conserva_desempates_y_no_descarta_texto_repetido() {
    let oraculo: Value = serde_json::from_str(include_str!("datos/cronologia.json")).unwrap();
    for c in oraculo["similitudes"].as_array().unwrap() {
        let ratio = cronologia::similitud(c["a"].as_str().unwrap(), c["b"].as_str().unwrap());
        assert!((ratio - c["ratio"].as_f64().unwrap()).abs() < 1e-14, "{c}");
    }
}

#[test]
fn paquete_aplica_reiteracion_y_conserva_cronologia_y_acciones_de_python() {
    let esperado: Value = serde_json::from_str(include_str!("datos/paquete_cronologia.json")).unwrap();
    let config = Config::default();
    let corte = esperado["corte"].as_str().unwrap().parse().unwrap();
    let p =
        flow_trabajo::paquete::importar("paquete_cronologia.zip", include_bytes!("datos/paquete_cronologia.zip"), corte, &config).unwrap();
    let fila = &p.trabajos[0].filas[0];
    let e = &esperado["filas"][0];
    assert_eq!(fila.observacion, e["observacion"]);
    assert_eq!(json!(fila.propuesta.reglas), e["reglas"]);
    assert_eq!(json!(fila.propuesta.acciones), e["acciones"]);
    assert_eq!(json!(fila.propuesta.avisos), e["avisos"]);
    assert_eq!(fila.cronologia.as_ref().unwrap(), &e["cronologia"]);
    assert_eq!(
        json!(cronologia::avisos(fila.cronologia.as_ref().unwrap(), &fila.observacion, &fila.propuesta.reglas, &config, corte)),
        e["alertas"]
    );
    assert!(fila.propuesta.reglas.contains(&"COMUN.REITERA_OBSERVACION".into()));
    let json = serde_json::to_vec(&p.trabajos[0]).unwrap();
    let guardado: flow_trabajo::Trabajo = serde_json::from_slice(&json).unwrap();
    assert_eq!(guardado.filas[0].cronologia, fila.cronologia);
    assert_eq!(guardado.filas[0].observacion, fila.observacion);
}
