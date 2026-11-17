use flow_motor::Config;
use flow_trabajo::{paquete, revision_bitacoras::Clasificacion as C, revision_informe};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io::{Cursor, Write}};

fn fecha(s: &str) -> chrono::NaiveDate { s.parse().unwrap() }
fn hex(b: &[u8]) -> String { Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect() }
fn csv(rows: &[Value]) -> Vec<u8> {
    let keys = rows.iter().flat_map(|v| v.as_object().unwrap().keys().cloned()).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
    let escape = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let mut s = keys.join(","); s.push('\n');
    for r in rows {
        s.push_str(&keys.iter().map(|k| escape(r[k].as_str().unwrap_or(""))).collect::<Vec<_>>().join(","));
        s.push('\n');
    }
    s.into_bytes()
}
fn build(rows: Vec<Value>, qs: Vec<Value>) -> Vec<u8> {
    let mut events = Vec::new();
    for q in &qs {
        let f: Value = serde_json::from_str(q["filters_json"].as_str().unwrap()).unwrap();
        let m = &f["remote_identity"];
        for e in f["revision_bitacoras"]["entries"].as_array().into_iter().flatten() {
            let day = e["fecha"].as_str().and_then(|s|s.get(..10)).and_then(|s|s.parse::<chrono::NaiveDate>().ok());
            let from = f["revision_bitacoras"]["desde"].as_str().and_then(|s|s.parse::<chrono::NaiveDate>().ok());
            if day.zip(from).is_some_and(|(a,b)|a>=b) {
                events.push(json!({"query_id":q["query_id"],"source_system":m["source_system"],"court_code":m["court_code"],
                    "cause_id":m["cause_id"],"ingreso_id":m["ingreso_id"],"identity_status":"VERIFICADA",
                    "event_id":format!("{}-{}",q["query_id"].as_str().unwrap(),e["entry_id"].as_str().unwrap_or("")),
                    "event_at":e["fecha"],"text":e["texto"],"origin":"CENTRO","origin_verified":"true","coverage":q["coverage"]}));
            }
        }
    }
    build_tables(vec![("registros",rows),("consultas",qs),("bitacoras",events)])
}
fn build_tables(tables: Vec<(&str,Vec<Value>)>) -> Vec<u8> {
    let mut files = Vec::new();
    let mut contents = Vec::new();
    for (name, rows) in tables {
        let bytes = csv(&rows); let path = format!("data/{name}.csv");
        files.push(json!({"path":path,"type":"normalized_table","table":name,"rows":rows.len(),"bytes":bytes.len(),"sha256":hex(&bytes)}));
        contents.push((path, bytes));
    }
    let manifest = json!({"schema_id":"CSMP_CAPTURE","schema_version":"1.0.0","required_features":[],
        "table_states":{"registros":"COMPLETA","bitacoras":"COMPLETA"},"files":files,"queries":[]});
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(manifest.to_string().as_bytes()).unwrap();
    for (p,b) in contents { zip.start_file(p, options).unwrap(); zip.write_all(&b).unwrap(); }
    zip.finish().unwrap().into_inner()
}
fn row(id: &str, ingreso: &str) -> Value {
    json!({"record_id":id,"record_type":"ESPERA","query_id":"q0","identity_status":"VERIFICADA",
        "source_system":"RUS","court_code":"901","cause_id":"100","ingreso_id":ingreso,
        "captured_at":"2026-11-17T12:00:00-03:00","coverage":"COMPLETA","original_file":"ficticio.xls",
        "original_sheet":"Espera","original_row":"2","modality_raw":"FAE",
        "normalized_json":json!({"RIT":"X-1-2099","RUT":"11.111.111-1","NOMBRE":"Persona Ficticia",
            "TRIBUNAL":"Laja","PROGRAMA":"FAE Ficticio","MODALIDAD":"FAE","T ESPERA":180}).to_string()})
}
fn query(id: &str, ingreso: &str, entries: Vec<Value>, complete: bool) -> Value {
    json!({"query_id":id,"screen":"bitacoras","coverage":if complete {"COMPLETA"}else{"PARCIAL"},
        "captured_at":"2026-11-17T12:00:00-03:00",
        "filters_json":json!({"remote_identity":{"source_system":"RUS","court_code":"901","cause_id":"100","ingreso_id":ingreso},
            "revision_bitacoras":{"version":1,"desde":"2026-10-17","hasta":"2026-11-17","captured_at":"2026-11-17T12:00:00-03:00",
                "coverage":if complete {"COMPLETA"}else{"PARCIAL"},"completa":complete,"warnings":[],"entries":entries}}).to_string()})
}
fn event(id: &str, day: &str, actor: &str) -> Value {
    json!({"entry_id":id,"fecha":day,"origen":actor,"texto":"Nota ficticia","tipo":"Al centro","etapa":"Espera"})
}
fn inspect(data: &[u8]) -> revision_informe::Informe {
    paquete::revisar_bitacoras(data, fecha("2026-11-01"), fecha("2026-11-17"), fecha("2026-11-17")).unwrap()
}

#[test]
fn cuatro_estados_con_conteos_y_procedencia_conciliados() {
    let mut no_admitida = row("r4","204"); no_admitida["record_type"] = json!("EGRESADOS");
    let data = build(vec![row("r1","201"),row("r2","202"),row("r3","203"),no_admitida],
        vec![query("b1","201",vec![],true),query("b2","202",vec![event("e1","2026-11-01","centro")],false)]);
    let report = inspect(&data);
    assert_eq!(report.conteos(), [1,1,1,1]);
    assert_eq!(report.conteos().iter().sum::<usize>(),report.filas.len());
    assert_eq!(report.filas_fuente,4);
    assert_eq!(report.filas[1].revision.clasificacion,C::ConObservacion);
    assert!(report.filas[1].revision.cobertura_parcial);
    assert_eq!(report.filas[0].procedencia[0].registro_id,"r1");
    assert_eq!(report.huella_paquete,hex(&data));
}

#[test]
fn paquete_legado_no_acredita_ausencia_por_tabla_global() {
    let mut q = query("b1","201",vec![],true);
    let mut f: Value = serde_json::from_str(q["filters_json"].as_str().unwrap()).unwrap();
    f.as_object_mut().unwrap().remove("revision_bitacoras");
    q["filters_json"] = json!(f.to_string());
    assert_eq!(inspect(&build(vec![row("r1","201")],vec![q])).conteos(),[0,0,1,0]);
}

#[test]
fn identidad_compuesta_no_mezcla_ingresos_tribunales_ni_causas() {
    let mut other = row("r2","201"); other["court_code"] = json!("902");
    let mut cause = row("r3","201"); cause["cause_id"] = json!("101");
    let report = inspect(&build(vec![row("r1","201"),other,cause,row("r4","202")],
        vec![query("b1","201",vec![],true)]));
    assert_eq!(report.conteos(),[1,0,3,0]);
    assert_eq!(report.filas.len(),4);
}

#[test]
fn autores_fechas_limites_y_cobertura_antigua_no_ocultan_eventos() {
    for (entries, complete, expected) in [
        (vec![event("e1","2026-10-31","centro")],true,C::SinObservacion),
        (vec![event("e1","2026-11-17","tribunal")],true,C::SinObservacion),
        (vec![event("e1","2026-11-17","desconocido")],true,C::NoVerificable),
        (vec![event("e1","ilegible","centro")],true,C::NoVerificable),
        (vec![],false,C::NoVerificable),
        (vec![event("e1","2026-11-17","centro")],false,C::ConObservacion),
    ] {
        assert_eq!(inspect(&build(vec![row("r1","201")],vec![query("b1","201",entries,complete)])).filas[0].revision.clasificacion,expected);
    }
    let mut q = query("b1","201",vec![],true);
    q["captured_at"] = json!("2026-11-16T12:00:00-03:00");
    assert_eq!(inspect(&build(vec![row("r1","201")],vec![q])).conteos(),[0,0,1,0]);
}

#[test]
fn duplicados_identicos_se_deduplican_y_contradictorios_no_se_resuelven() {
    let r = row("r1","201");
    let data = build(vec![r.clone(),r.clone()],vec![query("b1","201",vec![],true)]);
    let report = inspect(&data);
    assert_eq!(report.conteos(),[1,0,0,0]); assert_eq!(report.duplicados_registros,1);
    assert_eq!(report.filas[0].procedencia.len(),2);
    let mut conflict = r.clone(); conflict["normalized_json"] = json!(json!({"NOMBRE":"Otra Persona Ficticia"}).to_string());
    assert_eq!(inspect(&build(vec![r,conflict],vec![query("b1","201",vec![],true)])).conteos(),[0,0,1,0]);
    let q1 = query("b1","201",vec![],true);
    let q2 = query("b2","201",vec![event("e1","2026-11-17","centro")],true);
    assert_eq!(inspect(&build(vec![row("r1","201")],vec![q1,q2])).conteos(),[0,0,1,0]);
}

#[test]
fn aplicar_motor_conserva_resultados_del_conjunto_completo_e_invalidacion() {
    let data = build(vec![row("r1","201"),row("r2","202")],
        vec![query("b1","201",vec![],true),query("b2","202",vec![event("e1","2026-11-17","centro")],true)]);
    let mut report = inspect(&data);
    let config = Config::default();
    let normal = paquete::importar("ficticio.zip",&data,report.corte,&config).unwrap();
    revision_informe::aplicar_motor(&mut report,"ficticio.zip",&data,&config).unwrap();
    let expected = &normal.trabajos[0].filas[0].propuesta;
    let aislado = build(vec![row("r1","201")],vec![query("b1","201",vec![],true)]);
    let aislado = paquete::importar("ficticio.zip",&aislado,report.corte,&config).unwrap();
    assert_ne!(serde_json::to_value(expected).unwrap(),serde_json::to_value(&aislado.trabajos[0].filas[0].propuesta).unwrap(),
        "El fixture debe demostrar una dependencia real del registro no elegido.");
    assert_eq!(serde_json::to_value(report.filas[0].propuesta.as_ref().unwrap()).unwrap(),serde_json::to_value(expected).unwrap());
    assert!(report.filas[1].propuesta.is_none());
    report.filas[0].edicion = Some("Edición ficticia".into());
    assert!(revision_informe::aplicar_motor(&mut report,"ficticio.zip",&data,&config).is_err());
    assert_eq!(report.filas[0].edicion.as_deref(),Some("Edición ficticia"));
    report.cambiar_periodo(fecha("2026-11-02"),fecha("2026-11-17"),fecha("2026-11-17")).unwrap();
    assert!(!report.motor_aplicado); assert!(report.filas[0].edicion.is_none());
    let mut changed = data.clone(); changed[0] ^= 1;
    assert!(revision_informe::aplicar_motor(&mut report,"ficticio.zip",&changed,&config).is_err());
}

#[test]
fn zip_alterado_version_no_admitida_y_rutas_inseguras_se_rechazan() {
    let mut data = build(vec![row("r1","201")],vec![]); data[70] ^= 1;
    assert!(paquete::revisar_bitacoras(&data,fecha("2026-11-01"),fecha("2026-11-17"),fecha("2026-11-17")).is_err());
    for name in ["../fuera.csv","C:/fuera.csv","a\\b.csv","/fuera.csv"] {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(name,zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"x").unwrap();
        let data = zip.finish().unwrap().into_inner();
        assert!(paquete::revisar_bitacoras(&data,fecha("2026-11-01"),fecha("2026-11-17"),fecha("2026-11-17")).is_err());
    }
}

#[test]
fn metadatos_y_tabla_normalizada_discrepantes_son_no_verificables() {
    let q = query("b1","201",vec![],true);
    let e = json!({"query_id":"b1","source_system":"RUS","court_code":"901","cause_id":"100","ingreso_id":"201",
        "identity_status":"VERIFICADA","event_at":"2026-11-17","text":"Nota ficticia","origin":"CENTRO","origin_verified":"true","coverage":"COMPLETA"});
    let data = build_tables(vec![("registros",vec![row("r1","201")]),("consultas",vec![q]),("bitacoras",vec![e])]);
    assert_eq!(inspect(&data).conteos(),[0,0,1,0]);
    let q = query("b1","201",vec![event("e1","2026-11-17","centro")],true);
    let data = build_tables(vec![("registros",vec![row("r1","201")]),("consultas",vec![q]),("bitacoras",vec![])]);
    assert_eq!(inspect(&data).conteos(),[0,0,1,0]);
}

#[test]
fn ids_distintos_y_cambio_a_periodo_anterior_conservan_evidencia() {
    let events = vec![event("e1","2026-11-17T10:00:00","centro"),event("e2","2026-11-17T11:00:00","centro")];
    let data = build(vec![row("r1","201")],vec![query("b1","201",events,true)]);
    assert_eq!(inspect(&data).filas[0].revision.eventos_elegibles,2);
    let mut q = query("b1","201",vec![],true);
    q["captured_at"] = json!("2026-11-16T12:00:00-03:00");
    let mut f: Value = serde_json::from_str(q["filters_json"].as_str().unwrap()).unwrap();
    f["revision_bitacoras"]["captured_at"] = q["captured_at"].clone();
    f["revision_bitacoras"]["hasta"] = json!("2026-11-16");
    q["filters_json"] = json!(f.to_string());
    let mut report = inspect(&build(vec![row("r1","201")],vec![q]));
    assert_eq!(report.conteos(),[0,0,1,0]);
    report.cambiar_periodo(fecha("2026-11-01"),fecha("2026-11-16"),fecha("2026-11-17")).unwrap();
    assert_eq!(report.conteos(),[1,0,0,0]);
}

#[test]
fn cotejo_conserva_id_consulta_y_precision_y_solo_deduplica_copias_identicas() {
    let normalizada = |id: &str, momento: &str| json!({
        "query_id":"b1","event_id":format!("b1-{id}"),
        "source_system":"RUS","court_code":"901","cause_id":"100","ingreso_id":"201",
        "identity_status":"VERIFICADA","event_at":momento,"time_precision":"SEGUNDO",
        "text":"Nota ficticia","origin":"CENTRO","origin_verified":"true","coverage":"COMPLETA"
    });
    let e1 = event("e1","2026-11-17T10:00:00","centro");
    let e2 = event("e2","2026-11-17T11:00:00","centro");
    let q = query("b1","201",vec![e1.clone(),e2.clone()],true);
    let n1 = normalizada("e1","2026-11-17T10:00:00");
    let n2 = normalizada("e2","2026-11-17T11:00:00");
    let pack = |q: Value, events: Vec<Value>| build_tables(vec![
        ("registros",vec![row("r1","201")]),("consultas",vec![q]),("bitacoras",events)]);
    let report = inspect(&pack(q.clone(),vec![n1.clone(),n2.clone()]));
    assert_eq!(report.conteos(),[0,1,0,0]);
    assert_eq!(report.filas[0].revision.eventos_elegibles,2);
    // Antes el conjunto (día,texto) no detectaba la pérdida de esta segunda entrada.
    assert_eq!(inspect(&pack(q.clone(),vec![n1.clone()])).conteos(),[0,0,1,0]);
    // Tampoco se acepta una segunda entrada con el mismo ID y otra hora.
    assert_eq!(inspect(&pack(q,vec![n1.clone(),normalizada("e2","2026-11-17T12:00:00")])).conteos(),[0,0,1,0]);
    let q = query("b1","201",vec![e1.clone(),e1],true);
    let report = inspect(&pack(q,vec![n1.clone(),n1]));
    assert_eq!(report.conteos(),[0,1,0,0]);
    assert_eq!(report.filas[0].revision.eventos_elegibles,1);
    assert_eq!(report.filas[0].revision.duplicados_identicos,1);
    // Compatibilidad: el ID normalizado puede conservarse directamente, sin prefijo.
    let mut directo = n2; directo["event_id"] = json!("e2");
    let q = query("b1","201",vec![e2],true);
    assert_eq!(inspect(&pack(q,vec![directo])).conteos(),[0,1,0,0]);
    let e = event("midnight","2026-11-17T00:00:00","centro");
    let mut n = normalizada("midnight","2026-11-17");
    n["time_precision"] = json!("DIA");
    let q = query("b1","201",vec![e],true);
    assert_eq!(inspect(&pack(q.clone(),vec![n.clone()])).conteos(),[0,1,0,0]);
    let mut distinta = n.clone(); distinta["author"] = json!("Autor distinto");
    assert_eq!(inspect(&pack(q,vec![n,distinta])).conteos(),[0,0,1,0]);
}

#[test]
fn cotejo_iso8601_con_zona_conserva_fecha_civil_y_no_inventa_zona() {
    let n = |momento: &str| json!({
        "query_id":"b1","event_id":"b1-e1",
        "source_system":"RUS","court_code":"901","cause_id":"100","ingreso_id":"201",
        "identity_status":"VERIFICADA","event_at":momento,"time_precision":"SEGUNDO",
        "text":"Nota ficticia","origin":"CENTRO","origin_verified":"true","coverage":"COMPLETA"
    });
    for (raw, csv, esperado) in [
        ("2026-11-17T10:00:00Z","2026-11-17T10:00:00+00:00",C::ConObservacion),
        ("2026-11-17T10:00:00+00:00","2026-11-17T10:00:00Z",C::ConObservacion),
        ("2026-11-17T10:00:00-03:00","2026-11-17T10:00:00-03:00",C::ConObservacion),
        ("2026-11-17T10:00:00-03:00","2026-11-17T10:00:00+00:00",C::NoVerificable),
        ("2026-11-17T10:00:00-03:00","2026-11-17T13:00:00Z",C::NoVerificable),
        ("2026-11-17T10:00:00","2026-11-17T10:00:00Z",C::NoVerificable),
    ] {
        let q = query("b1","201",vec![event("e1",raw,"centro")],true);
        let bytes = build_tables(vec![("registros",vec![row("r1","201")]),("consultas",vec![q]),("bitacoras",vec![n(csv)])]);
        assert_eq!(inspect(&bytes).filas[0].revision.clasificacion,esperado,"{raw} frente a {csv}");
    }
}

#[test]
fn futuras_fuera_del_rango_no_ocultan_positivo_y_solo_futuras_no_acreditan_ausencia() {
    let normalizada = |id: &str, momento: &str| json!({
        "query_id":"b1","event_id":format!("b1-{id}"),
        "source_system":"RUS","court_code":"901","cause_id":"100","ingreso_id":"201",
        "identity_status":"VERIFICADA","event_at":momento,"time_precision":"SEGUNDO",
        "text":"Nota ficticia","origin":"CENTRO","origin_verified":"true","coverage":"COMPLETA"
    });
    let valida = event("dentro","2026-11-10T10:00:00","centro");
    let futura = event("futura","2026-11-18T10:00:00","centro");
    let n_valida = normalizada("dentro","2026-11-10T10:00:00");
    let n_futura = normalizada("futura","2026-11-18T10:00:00");
    let pack = |q: Value, events: Vec<Value>| build_tables(vec![
        ("registros",vec![row("r1","201")]),("consultas",vec![q]),("bitacoras",events)]);
    for conservar_futura in [false,true] {
        for incluir_valida in [false,true] {
            let mut entries = Vec::new();
            let mut normalizados = Vec::new();
            if incluir_valida { entries.push(valida.clone()); normalizados.push(n_valida.clone()); }
            if conservar_futura { entries.push(futura.clone()); }
            normalizados.push(n_futura.clone());
            let mut q = query("b1","201",entries,true);
            let mut filtros: Value = serde_json::from_str(q["filters_json"].as_str().unwrap()).unwrap();
            filtros["revision_bitacoras"]["completa"] = json!(false);
            filtros["revision_bitacoras"]["posteriores_omitidas"] = json!(1);
            filtros["revision_bitacoras"]["warnings"] = json!(["Una entrada posterior a la captura fue omitida."]);
            q["filters_json"] = json!(filtros.to_string());
            let report = inspect(&pack(q.clone(),normalizados));
            assert_eq!(report.filas[0].revision.clasificacion,if incluir_valida{C::ConObservacion}else{C::NoVerificable});
            assert!(report.filas[0].revision.cobertura_parcial);
            assert!(!report.filas[0].evidencia.contradictoria,"El recorte esperado no es contradicción");
            assert_eq!(report.filas[0].revision.eventos_elegibles,usize::from(incluir_valida));
            if incluir_valida {
                assert_eq!(inspect(&pack(q.clone(),vec![n_futura.clone()])).conteos(),[0,0,1,0],
                    "La nota dentro del rango no puede faltar del CSV.");
                let mut ajena = n_futura.clone(); ajena["ingreso_id"] = json!("999");
                assert_eq!(inspect(&pack(q,vec![n_valida.clone(),ajena])).conteos(),[0,0,1,0],
                    "Incluso la nota futura debe conservar la identidad verificada.");
            }
        }
    }
    // Aunque el productor omita el contador/aviso por error, la fila futura del CSV
    // impide una conclusión negativa, sin impedir el positivo válido.
    let q = query("b1","201",vec![valida],true);
    let report = inspect(&pack(q,vec![n_valida,n_futura]));
    assert_eq!(report.conteos(),[0,1,0,0]);
    assert!(report.filas[0].revision.cobertura_parcial);
}

#[test]
fn miles_de_ingresos_se_indizan_una_vez() {
    let mut rows = Vec::new(); let mut queries = Vec::new();
    for i in 0..2000 {
        let id = format!("r{i}"); let ingreso = format!("{}",1000+i);
        rows.push(row(&id,&ingreso)); queries.push(query(&format!("b{i}"),&ingreso,vec![],true));
    }
    let bytes = build(rows,queries);
    let inicio = std::time::Instant::now();
    let report = inspect(&bytes);
    assert_eq!(report.conteos(),[2000,0,0,0]);
    // Límite holgado para CI y Windows en depuración; evita regresión cuadrática de parseo.
    assert!(inicio.elapsed() < std::time::Duration::from_secs(20));
}


#[test]
fn paquete_legado_con_bitacoras_explica_falta_de_lectura_y_no_contradiccion() {
    let mut q = query("b1","201",vec![],true);
    let mut f: Value = serde_json::from_str(q["filters_json"].as_str().unwrap()).unwrap();
    f.as_object_mut().unwrap().remove("revision_bitacoras");
    q["filters_json"] = json!(f.to_string());
    let e = json!({"query_id":"b1","event_id":"b1-1","source_system":"RUS","court_code":"901","cause_id":"100","ingreso_id":"201",
        "identity_status":"VERIFICADA","event_at":"2026-11-10","time_precision":"DIA","text":"Nota ficticia",
        "origin":"CENTRO","origin_verified":"true","coverage":"COMPLETA"});
    let report = inspect(&build_tables(vec![("registros",vec![row("r1","201")]),("consultas",vec![q]),("bitacoras",vec![e])]));
    assert_eq!(report.conteos(),[0,0,1,0]);
    assert!(!report.filas[0].evidencia.contradictoria, "Un paquete legado no es evidencia contradictoria");
    assert_eq!(report.filas[0].revision.motivo,"No hay lectura acreditada para este ingreso.");
}
