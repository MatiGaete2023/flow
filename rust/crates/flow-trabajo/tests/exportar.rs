//! «Crear paquete de este lote»: el ZIP que se arma desde un lote ficticio del Descargador Rust se importa igual que
//! el lote (mismas observaciones, reglas, acciones y cronología). `rust/oraculo/validar_paquete_lote.py` lo pasa por
//! el validador y el importador de Python.

mod comun;

use comun::{datos, lote, Temporal};
use flow_captura::lotes::huella;
use flow_motor::Config;
use flow_trabajo::{captura, exportar, paquete};
use serde_json::{json, Value};

#[test]
fn paquete_del_lote_es_reproducible_y_se_importa_igual_que_el_lote() {
    let t = Temporal::nueva();
    let r = lote(&t.0);
    let lotes = [exportar::Lote { carpeta: &t.0, resumen: &r }];
    let producto = exportar::construir(&lotes, &exportar::Opciones::default()).unwrap();
    assert!(producto.avisos.is_empty(), "{:?}", producto.avisos);
    assert!(producto.bitacoras.contains("Bitácoras conservadas por tribunal/causa/ingreso: 2; ajenas excluidas: 0."));
    // Bitácoras siempre PARCIAL (la tabla de SITFA no acredita historial completo), como el Descargador Python.
    assert_eq!(producto.incompletas, ["bitacoras: PARCIAL", "consultas: PARCIAL"]);
    let referencia = datos().join("paquete_desde_lote.zip");
    if std::env::var_os("FLOW_REGENERAR").is_some() {
        std::fs::write(&referencia, &producto.datos).unwrap();
    }
    assert_eq!(std::fs::read(&referencia).unwrap(), producto.datos, "el paquete del lote ficticio cambió; regenera con FLOW_REGENERAR=1");

    let corte = "2026-10-20".parse().unwrap();
    let config = Config::default();
    let desde_lote = captura::importar(&t.0, &r, corte, &config).unwrap();
    let desde_zip = paquete::importar(exportar::NOMBRE, &producto.datos, corte, &config).unwrap().trabajos;
    assert_eq!(desde_lote.len(), 1);
    assert_eq!(desde_zip.len(), 1);
    let (a, b) = (&desde_lote[0].filas, &desde_zip[0].filas);
    assert_eq!(a.len(), 3);
    assert_eq!(b.len(), 3);
    let resumen_cronologia = |c: &Option<Value>| {
        c.as_ref().map(|c| {
            let textos = |k: &str| c[k].as_array().unwrap().iter().map(|e| (e["event_at"].clone(), e["text"].clone())).collect::<Vec<_>>();
            (c["status"].clone(), textos("last_center"), textos("center_period"), textos("pending_court"))
        })
    };
    for (x, y) in a.iter().zip(b) {
        assert_eq!(x.registro.texto("rit"), y.registro.texto("rit"));
        assert_eq!(x.propuesta.observacion, y.propuesta.observacion, "{}", x.registro.texto("rit"));
        assert_eq!(x.propuesta.reglas, y.propuesta.reglas, "{}", x.registro.texto("rit"));
        assert_eq!(x.propuesta.acciones, y.propuesta.acciones, "{}", x.registro.texto("rit"));
        assert_eq!(x.propuesta.marcas, y.propuesta.marcas, "{}", x.registro.texto("rit"));
        assert_eq!(x.proximo_informe, y.proximo_informe, "{}", x.registro.texto("rit"));
        assert_eq!(x.calendario, y.calendario, "{}", x.registro.texto("rit"));
        assert_eq!(resumen_cronologia(&x.cronologia), resumen_cronologia(&y.cronologia), "{}", x.registro.texto("rit"));
    }
    // La persona leída trae su bitácora; la de bitácora pendiente queda como no leída; la tercera no tiene vínculo.
    assert_eq!(b[0].cronologia.as_ref().unwrap()["status"], "ACREDITADA");
    assert_eq!(b[1].cronologia.as_ref().unwrap()["status"], "NO_LEIDA");
    assert!(b[2].cronologia.is_none());
    assert_eq!(b[2].registro.texto("nombre"), "Tercera Persona", "el NOMBRE del paquete va sin paréntesis");
}

#[test]
fn paquete_no_se_crea_de_un_lote_incompleto_ni_reemplaza_otro() {
    let t = Temporal::nueva();
    let mut r = lote(&t.0);
    let destino = t.0.join("salida").join(exportar::NOMBRE);
    let p = exportar::exportar(&[exportar::Lote { carpeta: &t.0, resumen: &r }], &destino, &Default::default()).unwrap();
    assert_eq!(std::fs::read(&destino).unwrap(), p.datos);
    assert!(exportar::exportar(&[exportar::Lote { carpeta: &t.0, resumen: &r }], &destino, &Default::default())
        .unwrap_err()
        .contains("ya existe"));
    r.estado = "INCOMPLETA".into();
    assert!(exportar::construir(&[exportar::Lote { carpeta: &t.0, resumen: &r }], &Default::default()).unwrap_err().contains("incompleto"));
}

#[test]
fn historia_ilegible_no_impide_el_paquete() {
    let t = Temporal::nueva();
    let mut r = lote(&t.0);
    // Una historia guardada con un vínculo que no se reconoce: el paquete sale igual, sin historia, y lo avisa.
    let h = r.lecturas.iter_mut().find(|l| l.tipo == "historia").unwrap();
    let ruta = t.0.join(&h.archivo);
    let mut valor: Value = serde_json::from_slice(&std::fs::read(&ruta).unwrap()).unwrap();
    valor["vinculo"] = json!("ilegible");
    let bytes = serde_json::to_vec(&valor).unwrap();
    std::fs::write(&ruta, &bytes).unwrap();
    h.sha256 = huella(&bytes);
    let p = exportar::construir(&[exportar::Lote { carpeta: &t.0, resumen: &r }], &Default::default()).unwrap();
    assert_eq!(p.avisos, vec!["La historia no pudo incluirse en el paquete; las bitácoras se conservan.".to_string()]);
    let trabajos = paquete::importar("x.zip", &p.datos, "2026-10-20".parse().unwrap(), &Config::default()).unwrap().trabajos;
    assert_eq!(trabajos[0].filas[0].cronologia.as_ref().unwrap()["status"], "ACREDITADA");
}

/// Lo que el Asistente Python propone al importar el mismo ZIP (`rust/oraculo/validar_paquete_lote.py`).
#[test]
fn paquete_del_lote_igual_que_el_asistente_python() {
    let esperado: Value = serde_json::from_slice(&std::fs::read(datos().join("paquete_desde_lote.json")).unwrap()).unwrap();
    let zip = std::fs::read(datos().join("paquete_desde_lote.zip")).unwrap();
    let corte = esperado["corte"].as_str().unwrap().parse().unwrap();
    let p = paquete::importar(exportar::NOMBRE, &zip, corte, &Config::default()).unwrap();
    let filas: Vec<_> = p.trabajos.iter().flat_map(|t| t.filas.iter()).collect();
    let esperadas = esperado["filas"].as_array().unwrap();
    assert_eq!(filas.len(), esperadas.len());
    assert!(esperadas.iter().any(|e| e["reglas"].as_array().unwrap().iter().any(|r| r == "HISTORIA.BASE_TRAMITE")));
    for e in esperadas {
        let rit = e["rit"].as_str().unwrap();
        let f = filas.iter().find(|f| f.registro.texto("rit") == rit).unwrap_or_else(|| panic!("falta {rit}"));
        assert_eq!(f.propuesta.observacion, e["observacion"].as_str().unwrap(), "{rit}");
        assert_eq!(json!(f.propuesta.reglas), e["reglas"], "{rit}");
        assert_eq!(json!(f.propuesta.acciones), e["acciones"], "{rit}");
        let marcas: Vec<Value> =
            f.propuesta.marcas.iter().map(|m| json!({"nivel": m.nivel, "codigo": m.codigo, "texto": m.texto})).collect();
        assert_eq!(json!(marcas), e["marcas"], "{rit}");
        let estado = f.cronologia.as_ref().map(|c| c["status"].clone()).unwrap_or(json!(""));
        assert_eq!(estado, e["cronologia"], "{rit}");
    }
}

/// Plan 18-nov-2026: la bandeja de Escritos que trae la respuesta de Historia viaja al paquete como tabla `escritos`
/// (versión 1.2.0, aditiva). `paquete_rust_escritos.zip` lo valida el lector Python (`tests/test_integracion_captura_20261118.py`).
#[test]
fn escritos_de_la_historia_viajan_al_paquete_1_2_0() {
    use std::io::Read;
    let t = Temporal::nueva();
    let mut r = lote(&t.0);
    let h = r.lecturas.iter_mut().find(|l| l.tipo == "historia").unwrap();
    let ruta = t.0.join(&h.archivo);
    let mut valor: Value = serde_json::from_slice(&std::fs::read(&ruta).unwrap()).unwrap();
    valor["datos"]["escritos"] = json!({"vacio_explicito":false,"advertencias":[],"filas":[
        {"fila":1,"doc":"DOC-1","tipo":"Inf. Cump Programa Ambulatorio","solicitante":"Programa Ficticio","estado":"Pendiente",
         "literal":{"Doc.":"DOC-1","Tipo Escrito":"Inf. Cump Programa Ambulatorio","Solicitante":"Programa Ficticio","Estado":"Pendiente"}}]});
    let bytes = serde_json::to_vec(&valor).unwrap();
    std::fs::write(&ruta, &bytes).unwrap();
    h.sha256 = huella(&bytes);
    let p = exportar::construir(&[exportar::Lote { carpeta: &t.0, resumen: &r }], &Default::default()).unwrap();
    let referencia = datos().join("paquete_rust_escritos.zip");
    if std::env::var_os("FLOW_REGENERAR").is_some() {
        std::fs::write(&referencia, &p.datos).unwrap();
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&p.datos)).unwrap();
    let mut manifiesto = String::new();
    zip.by_name("manifest.json").unwrap().read_to_string(&mut manifiesto).unwrap();
    let m: Value = serde_json::from_str(&manifiesto).unwrap();
    assert_eq!(m["schema_version"], "1.2.0");
    assert_eq!(m["table_states"]["escritos"], "COMPLETA");
    assert_eq!(m["table_states"]["historico_proteccion"], "NO_SOLICITADA");
    let mut tabla = String::new();
    zip.by_name("data/escritos.csv").unwrap().read_to_string(&mut tabla).unwrap();
    let filas = paquete::leer_csv(&tabla);
    assert_eq!(filas.len(), 1);
    assert_eq!((filas[0]["doc"].as_str(), filas[0]["escrito_status"].as_str(), filas[0]["scope"].as_str()), ("DOC-1", "Pendiente", "CAUSA"));
    // El mismo importador Rust lo lee sin cambiar las filas del trabajo.
    assert!(paquete::importar("x.zip", &p.datos, "2026-10-20".parse().unwrap(), &Config::default()).is_ok());
}

/// Plan 18-nov-2026: captura manual del Ingreso Histórico Protección → tabla `historico_proteccion` (1.2.0). Mismo
/// `historico.json` que escribe el Descargador Python. `paquete_rust_historico.zip` lo valida el lector Python.
#[test]
fn historico_manual_viaja_al_paquete_con_pertenencia_y_estados() {
    use flow_trabajo::historico_manual::{capturar, Persona};
    use std::io::Read;
    let t = Temporal::nueva();
    let r = lote(&t.0);
    let html = "<html><body><h4>Historial 11.111.111-1</h4><table><thead><tr><th>Causal egreso</th><th>Programa</th><th>Fecha egreso</th></tr></thead><tbody>\
        <tr><td>SIN INFORMACION</td><td>PRM Ficticio</td><td>--</td></tr><tr><td>SIN INFORMACION</td><td>FAE Otro Ficticio</td><td>--</td></tr>\
        <tr><td>EGRESO</td><td>PDE Antiguo</td><td>01/05/2026</td></tr></tbody></table></body></html>";
    let carpeta = t.0.join("historico-manual");
    let persona = Persona { tribunal: "113", tribunal_nombre: "Jgdo. L. y G. de Laja", person_id: "persona-ficticia-1", rut: "11.111.111-1", nombre: "Persona", url: "" };
    let e = capturar(&carpeta, html.as_bytes(), &persona, "2026-10-20T15:00:00+00:00").unwrap();
    assert_eq!((e["estado"].as_str(), e["ownership"].as_str()), (Some("LEIDA"), Some("VERIFICADA")));
    // Un nombre suelto que aparece en cualquier texto no prueba pertenencia.
    let otra = Persona { person_id: "persona-ficticia-2", rut: "99.999.999-9", nombre: "Otro", ..persona };
    assert_eq!(capturar(&carpeta, html.as_bytes(), &otra, "2026-10-20T15:00:00+00:00").unwrap()["ownership"], "INCIERTA");
    let mala = Persona { person_id: "persona-ficticia-3", ..otra };
    assert_eq!(capturar(&carpeta, b"<html><body><p>nada</p></body></html>", &mala, "2026-10-20T15:00:00+00:00").unwrap()["estado"], "FALLIDA");
    assert!(capturar(&carpeta, b"", &mala, "x").is_err());

    // Exportar solo la persona verificada: se parte de una carpeta con esa única captura.
    let solo = t.0.join("solo-verificada");
    capturar(&solo, html.as_bytes(), &persona, "2026-10-20T15:00:00+00:00").unwrap();
    let opciones = exportar::Opciones { historico: Some(solo), ..Default::default() };
    let p = exportar::construir(&[exportar::Lote { carpeta: &t.0, resumen: &r }], &opciones).unwrap();
    let referencia = datos().join("paquete_rust_historico.zip");
    if std::env::var_os("FLOW_REGENERAR").is_some() {
        std::fs::write(&referencia, &p.datos).unwrap();
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&p.datos)).unwrap();
    let mut manifiesto = String::new();
    zip.by_name("manifest.json").unwrap().read_to_string(&mut manifiesto).unwrap();
    let m: Value = serde_json::from_str(&manifiesto).unwrap();
    assert_eq!((m["schema_version"].as_str(), m["table_states"]["historico_proteccion"].as_str()), (Some("1.2.0"), Some("COMPLETA")));
    let mut tabla = String::new();
    zip.by_name("data/historico_proteccion.csv").unwrap().read_to_string(&mut tabla).unwrap();
    let filas = paquete::leer_csv(&tabla);
    let datos_filas: Vec<_> = filas.iter().map(|f| (f["program_name"].as_str(), f["egreso_date"].as_str(), f["ownership"].as_str())).collect();
    assert_eq!(datos_filas, [("PRM Ficticio", "--", "VERIFICADA"), ("FAE Otro Ficticio", "--", "VERIFICADA"), ("PDE Antiguo", "01/05/2026", "VERIFICADA")]);

    // Una captura con pertenencia incierta o ilegible queda visible en el paquete, nunca vacía.
    let opciones = exportar::Opciones { historico: Some(carpeta), ..Default::default() };
    let p = exportar::construir(&[exportar::Lote { carpeta: &t.0, resumen: &r }], &opciones).unwrap();
    assert!(p.incompletas.iter().any(|i| i.starts_with("historico_proteccion: ")), "{:?}", p.incompletas);
}
