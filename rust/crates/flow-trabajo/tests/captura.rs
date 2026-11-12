use flow_captura::{
    lotes::{huella, Resumen},
    recoleccion, vinculos,
};
use flow_motor::Config;
use flow_trabajo::captura;
use serde_json::{json, Value};

static SECUENCIA: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct Temporal(std::path::PathBuf);
impl Temporal {
    fn nueva() -> Self {
        let p = std::env::temp_dir().join(format!(
            "flow-trabajo-captura-ficticia-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
            SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn preparar() -> (Temporal, Resumen) {
    let t = Temporal::nueva();
    let seleccion = json!({"tribunal":"901","screen":"seguimiento","tab":"Cumplimiento","modality":"2","report":null,"state":null,"month":null,"year":null});
    let mut r:Resumen=serde_json::from_value(json!({"version":1,"lote":{"consultas":[seleccion],"fechas":null,"conservar_filtros":false,"evidencia":false,"pausa_ms":250,"lecturas":{"bitacoras":true,"historia":true,"calendario":true,"meses":4}},"estado":"VALIDADA","terminadas":[0],"vacias":[],"archivos":[],"avisos":[],"lecturas":[],"paginas_lectura":[]})).unwrap();
    let casos: Vec<Value> = serde_json::from_str(include_str!("../../flow-captura/tests/datos/vinculos_ficticios.json")).unwrap();
    let html = casos[0]["html"].as_str().unwrap().replace("tablaEspera", "tablaCumplimiento");
    let v = vinculos::extraer(html.as_bytes(), &r.lote.consultas[0]).unwrap().remove(0);
    r.paginas_lectura.push(recoleccion::Pagina { consulta: 0, numero: 1, vinculos: vec![v.clone()] });
    let datos = [
        (
            "historia",
            json!({"desde":"2026-06-20","hasta":"2026-10-20","advertencias":[],"filas":[{"descripcion":"Derecho a ser oído","fecha_iso":"2026-10-10","folio":"1"}]}),
        ),
        ("bitacora", json!({"entries":[{"fecha":"2026-10-11","texto":"Se informa derecho a ser oído aplicado el 10/10/2026."}]})),
        (
            "calendario",
            json!({"proyecciones":[{"estado":"PENDIENTE","fecha_proyeccion":"2026-11-01"},{"estado":"RECIBIDO","fecha_proyeccion":"2026-10-22"}]}),
        ),
    ];
    for (tipo, datos) in datos {
        let clave = recoleccion::clave(tipo, &v);
        let bytes = serde_json::to_vec(&json!({"version":1,"tipo":tipo,"clave":clave,"vinculo":v,"datos":datos})).unwrap();
        let archivo = format!("{tipo}.json");
        std::fs::write(t.0.join(&archivo), &bytes).unwrap();
        r.lecturas.push(recoleccion::Referencia {
            tipo: tipo.into(),
            clave,
            archivo,
            sha256: huella(&bytes),
            capturada: "2026-10-20T12:00:00Z".into(),
            meses: 4,
            tribunal: "901".into(),
            causa: "100".into(),
            ingreso: "201".into(),
            reutilizada: false,
        });
    }
    for pagina in 1..=2 {
        let nombre = if pagina == 1 { "Persona Ficticia" } else { "Otra Persona Ficticia" };
        let html=format!("<html><table><tr><th>RIT</th><th>RUT</th><th>Nombre</th><th>Derivación</th><th>Tribunal</th><th>DIAS DE CUMPLIMIENTO</th><th>DIAS PARA EGRESAR</th></tr><tr><td>X-1-2099</td><td>11111111-0</td><td>{nombre}</td><td>PRM Centro Ficticio</td><td>Laja</td><td>100</td><td>100</td></tr></table></html>");
        let nombre = format!("pagina-{pagina}.xls");
        std::fs::write(t.0.join(&nombre), &html).unwrap();
        r.archivos.push(flow_captura::lotes::Archivo {
            nombre,
            sha256: huella(html.as_bytes()),
            consulta: 0,
            pagina,
            total: 2,
            registros: 1,
        });
    }
    (t, r)
}

#[test]
fn contextos_se_aplican_por_identidad_completa_y_paginas_comparten_prepase() {
    let (t, mut r) = preparar();
    let corte = "2026-10-20".parse().unwrap();
    // Otra consulta puede incluir el mismo ingreso en otra etapa; no sustituye el vínculo de esta consulta.
    let mut otra = r.lote.consultas[0].clone();
    otra.tab = "Espera".into();
    r.lote.consultas.push(otra);
    let mut pagina = r.paginas_lectura[0].clone();
    pagina.consulta = 1;
    pagina.vinculos[0].etapa = "2".into();
    r.paginas_lectura.push(pagina);
    // Sin la bitácora, la historia propone dejar constancia del derecho a ser oído.
    let bit = r.lecturas.remove(1);
    let sin = captura::importar(&t.0, &r, corte, &Config::default()).unwrap();
    assert!(sin[0].filas[0].propuesta.reglas.contains(&"COMUN.OIDO".into()));
    assert!(!sin[0].filas[1].propuesta.reglas.contains(&"COMUN.OIDO".into()));
    r.lecturas.push(bit);
    let trabajos = captura::importar(&t.0, &r, corte, &Config::default()).unwrap();
    assert_eq!(trabajos.len(), 1);
    let filas = &trabajos[0].filas;
    assert_eq!(filas.len(), 2);
    assert!(!filas[0].propuesta.reglas.contains(&"COMUN.OIDO".into()));
    assert_eq!(filas[0].proximo_informe, Some("2026-11-01".parse().unwrap()));
    assert_eq!(filas[1].proximo_informe, None);
    assert!(filas.iter().all(|f| f.propuesta.marcas.iter().any(|m| m.codigo == "R6_DUPLICIDAD")));
}

#[test]
fn contexto_o_descarga_alterados_impiden_importacion() {
    let (t, r) = preparar();
    let corte = "2026-10-20".parse().unwrap();
    let referencia = &r.lecturas[0];
    let ruta = t.0.join(&referencia.archivo);
    let original = std::fs::read(&ruta).unwrap();
    std::fs::write(&ruta, b"{}").unwrap();
    assert!(captura::importar(&t.0, &r, corte, &Config::default()).is_err());
    std::fs::write(&ruta, original).unwrap();
    std::fs::write(t.0.join(&r.archivos[0].nombre), b"alterado ficticio").unwrap();
    assert!(captura::importar(&t.0, &r, corte, &Config::default()).is_err());
}

#[test]
fn lectura_nativa_aplica_reiteracion_sin_cambiar_acciones() {
    let (t, mut r) = preparar();
    r.lecturas.retain(|l| l.tipo != "historia");
    let corte = "2026-10-20".parse().unwrap();
    let config = Config::default();
    let inicial = captura::importar(&t.0, &r, corte, &config).unwrap();
    let previa = &inicial[0].filas[0].propuesta;
    let referencia = r.lecturas.iter_mut().find(|r| r.tipo == "bitacora").unwrap();
    let mut lectura = recoleccion::leer(&t.0, referencia).unwrap();
    lectura["datos"] = json!({"coverage":"PARCIAL","entries":[{"entry_id":"44","fecha":"2026-10-19T12:00:00","origen":"centro","autor":"Centro ficticio","tipo":"Al tribunal","texto":previa.observacion,"respuesta":"","fecha_respuesta":""}]});
    let bytes = serde_json::to_vec(&lectura).unwrap();
    std::fs::write(t.0.join(&referencia.archivo), &bytes).unwrap();
    referencia.sha256 = huella(&bytes);
    let nuevo = captura::importar(&t.0, &r, corte, &config).unwrap();
    let fila = &nuevo[0].filas[0];
    assert_eq!(fila.cronologia.as_ref().unwrap()["status"], "ACREDITADA");
    assert!(fila.propuesta.reglas.contains(&"COMUN.REITERA_OBSERVACION".into()));
    assert_eq!(fila.propuesta.acciones, previa.acciones);
    assert!(fila.propuesta.avisos.iter().any(|a| a.contains(&previa.observacion)));
    assert!(nuevo[0].filas[1].cronologia.is_none());
}

#[test]
fn calendario91_conserva_prioridad_y_coteja_la_proyeccion_del_ingreso() {
    let (t, mut r) = preparar();
    let mut s = r.lote.consultas[0].clone();
    s.screen = "calendario_informes".into();
    s.tab = String::new();
    r.lote.consultas.push(s);
    let html=b"<html><table><tr><th>RIT</th><th>RUT</th><th>Nombre</th><th>Derivacion</th><th>Tribunal</th><th>FECHA VENCIMIENTO</th></tr><tr><td>X-1-2099</td><td>11111111-0</td><td>Persona Ficticia</td><td>PRM Centro Ficticio</td><td>Laja</td><td>2026-10-30</td></tr></table></html>";
    std::fs::write(t.0.join("calendario.xls"), html).unwrap();
    r.archivos.push(flow_captura::lotes::Archivo {
        nombre: "calendario.xls".into(),
        sha256: huella(html),
        consulta: 1,
        pagina: 1,
        total: 1,
        registros: 1,
    });
    let trabajos = captura::importar(&t.0, &r, "2026-10-20".parse().unwrap(), &Config::default()).unwrap();
    assert_eq!(trabajos.len(), 1);
    let fila = &trabajos[0].filas[0];
    assert_eq!(fila.proximo_informe, Some("2026-10-30".parse().unwrap()));
    assert_eq!(fila.calendario.len(), 2);
    assert_eq!(fila.propuesta.avisos.iter().filter(|a| a.starts_with("CAL-03:")).count(), 1);
    assert_eq!(trabajos[0].filas[1].proximo_informe, None);
}

#[test]
fn egresados_no_generan_observaciones_y_participan_en_n04_y_q03() {
    let (t, mut r) = preparar();
    for (i, tab) in [(1, "Egresados"), (2, "Informes")] {
        let mut s = r.lote.consultas[0].clone();
        s.tab = tab.into();
        r.lote.consultas.push(s);
        let mut p = r.paginas_lectura[0].clone();
        p.consulta = i;
        r.paginas_lectura.push(p);
        let html=b"<html><table><tr><th>RIT</th><th>RUT</th><th>Nombre</th><th>Derivacion</th><th>Tribunal</th><th>FECHA VENCIMIENTO</th></tr><tr><td>X-1-2099</td><td>11111111-0</td><td>Persona Ficticia</td><td>PRM Centro Ficticio</td><td>Laja</td><td>2026-10-30</td></tr></table></html>";
        let nombre = format!("consulta-auxiliar-{i}.xls");
        std::fs::write(t.0.join(&nombre), html).unwrap();
        r.archivos.push(flow_captura::lotes::Archivo { nombre, sha256: huella(html), consulta: i, pagina: 1, total: 1, registros: 1 });
    }
    let trabajos = captura::importar(&t.0, &r, "2026-10-20".parse().unwrap(), &Config::default()).unwrap();
    assert_eq!(trabajos.len(), 3);
    let egresados = trabajos.iter().find(|t| t.nombre() == "EGRESADOS").unwrap();
    let fila = &egresados.filas[0];
    assert!(fila.observacion.is_empty() && fila.propuesta.acciones.is_empty() && fila.propuesta.reglas.is_empty());
    assert!(fila.propuesta.avisos.iter().any(|a| a.starts_with("N-04:")));
    assert!(fila.propuesta.avisos.iter().any(|a| a.starts_with("Q-03:")));
}
