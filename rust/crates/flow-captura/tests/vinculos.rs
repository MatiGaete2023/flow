use flow_captura::{
    sitfa::Seleccion,
    vinculos::{self, extraer, llamadas},
};
use serde_json::{json, Value};

fn seleccion() -> Seleccion {
    Seleccion {
        tribunal: "901".into(),
        screen: "seguimiento".into(),
        tab: "Espera".into(),
        modality: "2".into(),
        report: None,
        state: None,
        month: None,
        year: None,
    }
}
fn casos() -> Vec<Value> {
    serde_json::from_str(include_str!("datos/vinculos_ficticios.json")).unwrap()
}

#[test]
fn enlaces_de_listados_ficticios_coinciden_con_python() {
    for caso in casos() {
        let filas = extraer(caso["html"].as_str().unwrap().as_bytes(), &seleccion()).unwrap();
        let bitacoras = filas.iter().filter(|v| v.bitacora_verificada).collect::<Vec<_>>();
        assert_eq!(
            filas.iter().filter(|v| v.historia_verificada).count(),
            caso["historias"].as_array().unwrap().iter().filter(|h| h["identidad"] == "VERIFICADA").count(),
            "{}",
            caso["nombre"]
        );
        assert_eq!(bitacoras.len(), caso["bitacoras"].as_array().unwrap().len(), "{}", caso["nombre"]);
        for (v, esperado) in bitacoras.iter().zip(caso["bitacoras"].as_array().unwrap()) {
            assert_eq!(v.ingreso, esperado["ingreso_id"].as_str().unwrap());
            assert_eq!(v.causa, esperado["causa_id"].as_str().unwrap());
            assert_eq!(v.rut, esperado["rut"].as_str().unwrap());
            assert_eq!(v.calendario.as_ref().map(|a| json!(a)), esperado.get("calendario").cloned());
        }
        for (v, esperado) in filas.iter().filter(|v| !v.causa.is_empty()).zip(caso["historias"].as_array().unwrap()) {
            assert_eq!(v.historia_verificada, esperado["identidad"] == "VERIFICADA", "{}", caso["nombre"]);
        }
    }
}

#[test]
fn parser_de_llamadas_nunca_ejecuta_expresiones_ni_confunde_texto_con_codigo() {
    let accion = "ShowHistoria(901,'X-1-2099',100)";
    for falso in [
        format!("\"{accion}\""),
        format!("`{accion}`"),
        format!("/* {accion} */"),
        format!("// {accion}"),
        format!("if(x) /{accion}/.test(x)"),
        format!("obj.{accion}"),
    ] {
        assert!(llamadas(&falso, "ShowHistoria").is_empty(), "{falso}");
    }
    assert_eq!(llamadas(&format!("/* texto */ {accion}"), "ShowHistoria"), vec![vec!["901", "X-1-2099", "100"]]);
    assert!(llamadas("ShowHistoria(document.value,123)", "ShowHistoria").is_empty());
    assert!(llamadas("ShowHistoria(obtener(),123)", "ShowHistoria").is_empty());
    assert_eq!(llamadas("ShowObservaciones(\"texto (con, comas)\",123)", "ShowObservaciones"), vec![vec!["texto (con, comas)", "123"]]);
}

#[test]
fn conserva_filas_sin_enlace_y_no_confunde_rut_con_codigo_embebido() {
    let caso = casos().remove(0);
    let html = caso["html"].as_str().unwrap().replace("ShowHistoria", "OtraAccion").replace("ShowObservaciones", "OtraAccion");
    let filas = extraer(html.as_bytes(), &seleccion()).unwrap();
    assert_eq!(filas.len(), 1);
    assert!(!filas[0].bitacora_verificada);
    assert!(!filas[0].historia_verificada);
    assert!(filas[0].params_bitacora("2").is_err());
    assert_eq!(filas[0].rit, "X-1-2099");
    let html = caso["html"].as_str().unwrap().replace("11111111-0</td>", "11111111-0<script>ajeno()</script></td>");
    let filas = extraer(html.as_bytes(), &seleccion()).unwrap();
    assert_eq!(filas[0].rut, "11111111-0");
}

#[test]
fn causas_ambiguas_entre_paginas_y_hermanos_no_se_atribuyen_a_un_ingreso() {
    let caso = casos().remove(0);
    let html = caso["html"].as_str().unwrap();
    let mut filas = extraer(html.as_bytes(), &seleccion()).unwrap();
    filas.extend(extraer(html.replace("'100'", "'101'").as_bytes(), &seleccion()).unwrap());
    vinculos::descartar_ambiguos(&mut filas);
    assert!(filas.iter().all(|v| !v.historia_verificada));
    let mut filas = extraer(html.as_bytes(), &seleccion()).unwrap();
    let mut hermano = filas[0].clone();
    hermano.nombre = "Otra Persona Ficticia".into();
    hermano.rut = "22222222-0".into();
    filas.push(hermano);
    vinculos::descartar_ambiguos(&mut filas);
    assert!(filas.iter().all(|v| !v.bitacora_verificada && v.calendario.is_none()));
    assert!(filas.iter().all(|v| v.historia_verificada));
}

#[test]
fn identidad_de_excel_usa_los_cuatro_campos_completos() {
    assert_eq!(
        vinculos::identidad("X-1-2099", "11.111.111-0", "Niña Ficticia", "PRM Ficticio"),
        vinculos::identidad("x-1-2099", "11111111-0", "NINA FICTICIA", "prm ficticio")
    );
    assert_ne!(
        vinculos::identidad("X-1-2099", "11111111-0", "Persona Uno", "PRM"),
        vinculos::identidad("X-1-2099", "11111111-0", "Persona Dos", "PRM")
    );
}

#[test]
fn lectura_exige_capacidades_identidad_huella_y_fecha_antes_de_aceptar_datos(){
    use flow_captura::lotes::Conexion;
    use base64::{engine::general_purpose::STANDARD,Engine};
    struct Ficticia {mutacion:&'static str,datos:String,lecturas:std::sync::atomic::AtomicUsize}
    impl Conexion for Ficticia {
        fn llamar(&self,comando:&str,payload:Value,_plazo:std::time::Duration)->Result<Value,String>{
            if comando=="capabilities"{return Ok(json!({"tipo":"FLOW_CAPACIDADES","version":"0.2.0","protocolos":{"descarga_xls":1,"pdf_sin_registros":1,"bitacoras_lectura":1,"historia_lectura":1,"calendario_lectura":1},"escritura_rus":self.mutacion=="escritura"}));}
            assert_eq!(comando,"diary_open");self.lecturas.fetch_add(1,std::sync::atomic::Ordering::SeqCst);
            let datos=if self.mutacion=="html_ajeno"{self.datos.replace("name=\"ID_Ingreso\" value=\"201\"","name=\"ID_Ingreso\" value=\"999\"")}else{self.datos.clone()};
            let mut r=json!({"status":200,"type":"text/html","body":STANDARD.encode(datos.as_bytes()),"params":payload["params"],"digest":flow_captura::lotes::huella(datos.as_bytes()),"startedAt":"2026-11-12T12:00:00Z"});
            match self.mutacion {"params"=>r["params"]["ID_Ingreso"]=json!("999"),"digest"=>r["digest"]=json!("alterada"),"startedAt"=>r["startedAt"]=json!("ilegible"),_=>()};Ok(r)
        }
    }
    let c=casos().remove(0);let v=extraer(c["html"].as_str().unwrap().as_bytes(),&seleccion()).unwrap().remove(0);
    let lecturas:Vec<Value>=serde_json::from_str(include_str!("datos/lecturas_ficticias.json")).unwrap();
    let datos=lecturas[0]["html"].as_str().unwrap().replace("name=\"COD_Tribunal\" value=\"11\"","name=\"COD_Tribunal\" value=\"901\"")
        .replace("name=\"CRR_IdCausa\" value=\"22\"","name=\"CRR_IdCausa\" value=\"100\"").replace("name=\"ID_Ingreso\" value=\"33\"","name=\"ID_Ingreso\" value=\"201\"");
    let positiva=Ficticia{mutacion:"ninguna",datos:datos.clone(),lecturas:Default::default()};
    assert_eq!(vinculos::abrir_bitacora(&positiva,&v,"2").unwrap()["coverage"],"PARCIAL");
    for mutacion in ["params","digest","startedAt","html_ajeno","escritura"]{
        let f=Ficticia{mutacion,datos:datos.clone(),lecturas:Default::default()};assert!(vinculos::abrir_bitacora(&f,&v,"2").is_err(),"{mutacion}");
        assert_eq!(f.lecturas.load(std::sync::atomic::Ordering::SeqCst),usize::from(mutacion!="escritura"));
    }
}
