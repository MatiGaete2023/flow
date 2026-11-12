use flow_dominio::Celda;
use flow_motor::Config;
use flow_trabajo::controles::{self, Proyeccion};
use serde_json::{json, Value};

fn fecha(v: &Value) -> Option<chrono::NaiveDate> {
    v.as_str()?.parse().ok()
}
#[test]
fn ordenes_y_cotejo_son_los_de_python() {
    let datos: Value = serde_json::from_str(include_str!("datos/controles.json")).unwrap();
    for (i, c) in datos["ordenes"].as_array().unwrap().iter().enumerate() {
        let valores =
            c["valores"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), Celda::Texto(v.as_str().unwrap().into()))).collect();
        assert_eq!(json!(controles::orden(&valores, fecha(&c["corte"]).unwrap())), c["avisos"], "orden {i}");
    }
    for (i, c) in datos["calendarios"].as_array().unwrap().iter().enumerate() {
        let items: Vec<Proyeccion> = serde_json::from_value(c["items"].clone()).unwrap();
        let corte = fecha(&c["corte"]).unwrap();
        assert_eq!(json!(controles::cotejar(&items, corte, fecha(&c["vencimiento"]), fecha(&c["hoja2"]))), c["avisos"], "cotejo {i}");
        assert_eq!(json!(controles::primera(&items)), c["primera"]);
        assert_eq!(json!(controles::proxima(&items, corte)), c["proxima"]);
    }
}

#[test]
fn paquete_con_egresados_y_ordenes_conserva_filas_y_avisos_de_python() {
    let esperado: Value = serde_json::from_str(include_str!("datos/paquete_controles.json")).unwrap();
    let p = flow_trabajo::paquete::importar(
        "paquete_controles.zip",
        include_bytes!("datos/paquete_controles.zip"),
        fecha(&esperado["corte"]).unwrap(),
        &Config::default(),
    )
    .unwrap();
    let filas = p.trabajos.iter().flat_map(|t| &t.filas).collect::<Vec<_>>();
    assert_eq!(filas.len(), 5);
    for e in esperado["filas"].as_array().unwrap() {
        let f = filas.iter().find(|f| f.id == e["id"]).unwrap();
        assert_eq!(f.observacion, e["observacion"], "{}", f.id);
        assert_eq!(json!(f.propuesta.reglas), e["reglas"], "{}", f.id);
        assert_eq!(json!(f.propuesta.acciones), e["acciones"], "{}", f.id);
        assert_eq!(json!(f.propuesta.avisos), e["avisos"], "{}", f.id);
        if f.origen.as_ref().unwrap().tipo == "CUMPLIMIENTO" {
            assert_eq!(json!(f.proximo_informe), e["proximo"]);
        }
    }
    for tipo in ["EGRESADOS", "BUSQUEDA_LITIGANTES"] {
        let t = p.trabajos.iter().find(|t| t.nombre() == tipo).unwrap();
        assert!(t.filas.iter().all(|f| f.observacion.is_empty() && f.propuesta.acciones.is_empty()));
    }
    let orden = p.trabajos.iter().find(|t| t.nombre() == "BUSQUEDA_LITIGANTES").unwrap();
    assert!(orden.cabeceras.iter().any(|c| c == "ESTADO O B"));
}

#[test]
fn consulta_auxiliar_local_conserva_columnas_y_no_pasa_por_el_motor() {
    let datos=b"<html><table><tr><th>RIT</th><th>Nombre</th><th>ESTADO O B</th><th>ESTADO ENVIO</th></tr><tr><td>X-1-2099</td><td>Persona Ficticia</td><td>Activa</td><td>Pendiente</td></tr></table></html>";
    let libro = flow_tabular::leer_bytes("consulta.xls", datos.to_vec()).unwrap();
    assert_eq!(flow_trabajo::consulta::detectar(&libro), Some("BUSQUEDA_LITIGANTES"));
    let corte = "2026-10-20".parse().unwrap();
    let config = Config::default();
    let t = flow_trabajo::consulta::analizar(&libro, "BUSQUEDA_LITIGANTES", corte, &config).unwrap();
    assert_eq!(t.nombre(), "BUSQUEDA_LITIGANTES");
    assert!(t.filas[0].observacion.is_empty());
    assert!(t.filas[0].propuesta.avisos.iter().any(|a| a.starts_with("OB-03:")));
    assert!(t.cabeceras.iter().any(|c| c == "ESTADO O B"));
    let egresados = flow_trabajo::consulta::analizar(&libro, "EGRESADOS", corte, &config).unwrap();
    assert_eq!(egresados.nombre(), "EGRESADOS");
    assert!(egresados.filas[0].propuesta.acciones.is_empty() && egresados.filas[0].propuesta.reglas.is_empty());
}
