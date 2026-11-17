//! Paquete enteramente ficticio para pruebas reproducibles de la interfaz y del usuario.
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
use std::{collections::BTreeSet,io::{Cursor,Write}};
fn csv(filas:&[Value])->Vec<u8>{
    let claves=filas.iter().flat_map(|v|v.as_object().into_iter().flat_map(|m|m.keys().cloned())).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
    let mut s=claves.join(",");s.push('\n');
    for fila in filas {
        s.push_str(&claves.iter().map(|k|format!("\"{}\"",fila[k].as_str().map(str::to_owned).unwrap_or_else(||fila[k].to_string()).replace('"',"\"\""))).collect::<Vec<_>>().join(","));
        s.push('\n');
    }
    s.into_bytes()
}
pub fn crear()->Result<Vec<u8>,String>{
    let mut registros=vec![];let mut consultas=vec![];let mut bitacoras=vec![];
    for i in 0..4 {
        let ingreso=format!("{}",201+i);let causa=format!("{}",100+i);let qid=format!("b{i}");
        let completa=i!=2;let cobertura=if completa{"COMPLETA"}else{"PARCIAL"};
        let entries=if i==1{vec![json!({"entry_id":"e1","fecha":"2026-11-10","origen":"centro","autor":"Funcionario Ficticio","tipo":"Al centro","etapa":"Espera","texto":"Nota ficticia de seguimiento."})]}else{vec![]};
        let valores=json!({"RIT":format!("X-{}-2099",i+1),"RUT":"11.111.111-1","NOMBRE":format!("Persona Ficticia {}",i+1),
            "TRIBUNAL":"Laja","PROGRAMA":"AFT FICTICIO","MODALIDAD":"AMB","T ESPERA":180});
        registros.push(json!({"record_id":format!("r{i}"),"record_type":if i==3{"EGRESADOS"}else{"ESPERA"},"query_id":"q0",
            "identity_status":"VERIFICADA","source_system":"RUS","court_code":"901","cause_id":causa,"ingreso_id":ingreso,
            "captured_at":"2026-11-17T12:00:00-03:00","coverage":"COMPLETA","original_file":"Origen_FICTICIO.xls","original_sheet":"Espera",
            "original_row":format!("{}",i+2),"modality_raw":"AMB","normalized_json":valores.to_string()}));
        consultas.push(json!({"query_id":qid,"screen":"bitacoras","coverage":cobertura,"captured_at":"2026-11-17T12:00:00-03:00",
            "filters_json":json!({"remote_identity":{"source_system":"RUS","court_code":"901","cause_id":causa,"ingreso_id":ingreso},
                "revision_bitacoras":{"version":1,"desde":"2026-11-01","hasta":"2026-11-17","captured_at":"2026-11-17T12:00:00-03:00",
                    "coverage":cobertura,"completa":completa,"warnings":[],"entries":entries}}).to_string()}));
        if i==1 {bitacoras.push(json!({"query_id":qid,"event_id":"b1-e1","source_system":"RUS","court_code":"901","cause_id":causa,"ingreso_id":ingreso,
            "identity_status":"VERIFICADA","event_at":"2026-11-10","origin":"CENTRO","origin_verified":"true","coverage":"COMPLETA","text":"Nota ficticia de seguimiento."}));}
    }
    let mut contenidos=vec![];let mut archivos=vec![];
    for (nombre,filas) in [("registros",registros),("consultas",consultas),("bitacoras",bitacoras)]{
        let bytes=csv(&filas);let ruta=format!("data/{nombre}.csv");
        let sha=Sha256::digest(&bytes).iter().map(|b|format!("{b:02x}")).collect::<String>();
        archivos.push(json!({"path":ruta,"type":"normalized_table","table":nombre,"rows":filas.len(),"bytes":bytes.len(),"sha256":sha}));
        contenidos.push((ruta,bytes));
    }
    let manifiesto=json!({"schema_id":"CSMP_CAPTURE","schema_version":"1.0.0","required_features":[],"files":archivos,
        "table_states":{"registros":"COMPLETA","bitacoras":"PARCIAL"},"queries":[]});
    let mut zip=zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opciones=zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json",opciones).map_err(|e|e.to_string())?;
    zip.write_all(manifiesto.to_string().as_bytes()).map_err(|e|e.to_string())?;
    for (p,b) in contenidos {zip.start_file(p,opciones).map_err(|e|e.to_string())?;zip.write_all(&b).map_err(|e|e.to_string())?;}
    zip.finish().map(|z|z.into_inner()).map_err(|e|e.to_string())
}
#[cfg(test)]
mod tests{
    #[test]
    fn demo_recorre_cuatro_estados_y_motor_real(){
        let datos=super::crear().unwrap();let desde="2026-11-01".parse().unwrap();let hasta="2026-11-17".parse().unwrap();
        let mut informe=flow_trabajo::paquete::revisar_bitacoras(&datos,desde,hasta,hasta).unwrap();
        assert_eq!(informe.conteos(),[1,1,1,1]);
        flow_trabajo::revision_informe::aplicar_motor(&mut informe,"Paquete_FICTICIO.zip",&datos,&flow_motor::Config::default()).unwrap();
        assert!(informe.filas[0].propuesta.is_some());assert!(informe.filas[1..].iter().all(|f|f.propuesta.is_none()));
    }
}

