use flow_anonimo::{documentos, formatos, Modo, Opciones};
use serde_json::Value;
use sha2::{Digest, Sha256};
fn opciones() -> Opciones {
    Opciones { modo: Modo::Conservador, semilla: "FICTICIA".into(), ..Default::default() }
}
#[test]
fn zip_reconstruye_nombres_huellas_y_excluye_binarios() {
    let bytes = documentos::zip(&[
        ("Privado Apellido/datos.json".into(), br#"{"Nombre":"Identcanario Apelcanario","RIT":"P-1-2099"}"#.to_vec()),
        ("adjunto.pdf".into(), b"%PDF-1.7 no admitido".to_vec()),
    ])
    .unwrap();
    let s = formatos::anonimizar("entrada.zip", &bytes, opciones()).unwrap();
    let entries = formatos::validar_zip(s.datos()).unwrap();
    assert_eq!(entries.len(), 3);
    assert!(!s.avisos().is_empty());
    let m: Value = serde_json::from_slice(&entries.iter().find(|(n, _)| n == "MANIFIESTO.json").unwrap().1).unwrap();
    for f in m["archivos"].as_array().unwrap() {
        let raw = &entries.iter().find(|(n, _)| n == f["path"].as_str().unwrap()).unwrap().1;
        assert_eq!(f["sha256"], format!("{:x}", Sha256::digest(raw)));
    }
    for (n, raw) in entries {
        assert!(!n.contains("Privado"));
        assert!(!String::from_utf8_lossy(&raw).contains("Identcanario"));
    }
    assert!(formatos::anonimizar("entrada.zip", &bytes, Opciones { rechazar_no_admitidos: true, ..opciones() }).is_err());
}
#[test]
fn capturas_conservan_contrato_huellas_y_referencias() {
    for bytes in [
        include_bytes!("../../flow-trabajo/tests/datos/paquete_cronologia.zip").as_slice(),
        include_bytes!("../../flow-trabajo/tests/datos/paquete_controles.zip").as_slice(),
        include_bytes!("../../flow-trabajo/tests/datos/paquete_historia.zip").as_slice(),
    ] {
        let salida = formatos::anonimizar("captura.zip", bytes, opciones()).unwrap();
        let entries = formatos::validar_zip(salida.datos()).unwrap();
        let m: Value = serde_json::from_slice(&entries.iter().find(|(n, _)| n == "manifest.json").unwrap().1).unwrap();
        assert_eq!(m["schema_id"], "CSMP_CAPTURE");
        for f in m["files"].as_array().unwrap() {
            let raw = &entries.iter().find(|(n, _)| n == f["path"].as_str().unwrap()).unwrap().1;
            assert_eq!(f["bytes"], raw.len());
            assert_eq!(f["sha256"], format!("{:x}", Sha256::digest(raw)));
        }
        assert!(matches!(formatos::cargar("copia.zip", salida.datos()).unwrap(), formatos::Entrada::Zip(_)));
    }
}
#[test]
fn captura_alterada_falla_cerrado() {
    let mut entries = formatos::validar_zip(include_bytes!("../../flow-trabajo/tests/datos/paquete_cronologia.zip")).unwrap();
    let i = entries.iter().position(|(n, _)| n == "data/registros.csv").unwrap();
    entries[i].1.push(b' ');
    assert!(formatos::anonimizar("mala.zip", &documentos::zip(&entries).unwrap(), opciones()).is_err());
}
#[test]
fn documentos_apuntan_al_evento_ficticio_y_se_marcan_sin_adjunto() {
    let mut entries = formatos::validar_zip(include_bytes!("../../flow-trabajo/tests/datos/paquete_cronologia.zip")).unwrap();
    let i = entries.iter().position(|(n, _)| n == "manifest.json").unwrap();
    let mut m: Value = serde_json::from_slice(&entries[i].1).unwrap();
    let bit = &entries.iter().find(|(n, _)| n == "data/bitacoras.csv").unwrap().1;
    let mut r = csv::Reader::from_reader(bit.as_slice());
    let headers = r.headers().unwrap().clone();
    let event = headers
        .iter()
        .zip(r.records().next().unwrap().unwrap().iter())
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let esquema: Value = serde_json::from_str(include_str!("../../../datos/captura_anonima.json")).unwrap();
    let fields = esquema["tablas"]["documentos"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>();
    let mut doc = event.clone();
    doc.insert("parent_event_id".into(), event["event_id"].clone());
    doc.insert("document_id".into(), "DOC_FICTICIO".into());
    doc.insert("download_state".into(), "NO_DISPONIBLE".into());
    doc.insert("content_read".into(), "false".into());
    let mut w = csv::Writer::from_writer(vec![]);
    w.write_record(&fields).unwrap();
    w.write_record(fields.iter().map(|k| doc.get(*k).map(String::as_str).unwrap_or(""))).unwrap();
    let raw = w.into_inner().unwrap();
    m["files"].as_array_mut().unwrap().push(serde_json::json!({"path":"data/documentos.csv","type":"normalized_table","table":"documentos","rows":1,"bytes":raw.len(),"sha256":format!("{:x}",Sha256::digest(&raw))}));
    m["table_states"]["documentos"] = "COMPLETA".into();
    entries[i].1 = serde_json::to_vec(&m).unwrap();
    entries.push(("data/documentos.csv".into(), raw));
    let out = formatos::anonimizar("captura.zip", &documentos::zip(&entries).unwrap(), opciones()).unwrap();
    let files = formatos::validar_zip(out.datos()).unwrap();
    let valor = |file: &str, key: &str| {
        let b = &files.iter().find(|(n, _)| n == file).unwrap().1;
        let mut reader = csv::Reader::from_reader(b.as_slice());
        let index = reader.headers().unwrap().iter().position(|k| k == key).unwrap();
        reader.records().next().unwrap().unwrap()[index].to_string()
    };
    assert_eq!(valor("data/bitacoras.csv", "event_id"), valor("data/documentos.csv", "parent_event_id"));
    assert_ne!(valor("data/bitacoras.csv", "event_id"), event["event_id"]);
    assert_eq!(valor("data/documentos.csv", "local_file"), "");
    assert_eq!(valor("data/documentos.csv", "download_state"), "NO_DISPONIBLE");
}
