//! «Crear paquete de este lote»: el ZIP que se arma desde un lote ficticio del Descargador Rust se importa igual que
//! el lote (mismas observaciones, reglas, acciones y cronología). `rust/oraculo/validar_paquete_lote.py` lo pasa por
//! el validador y el importador de Python.

use flow_captura::{lecturas, lotes::huella, lotes::Resumen, recoleccion, vinculos};
use flow_motor::Config;
use flow_trabajo::{captura, exportar, paquete};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

static SECUENCIA: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct Temporal(PathBuf);
impl Temporal {
    fn nueva() -> Self {
        let p = std::env::temp_dir().join(format!(
            "flow-exportar-ficticio-{}-{}",
            std::process::id(),
            SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const RUT: &str = "11.111.111-1";
const RUT2: &str = "22.222.222-2";

fn caso(i: usize) -> Value {
    let casos: Vec<Value> = serde_json::from_str(include_str!("../../flow-captura/tests/datos/lecturas_ficticias.json")).unwrap();
    casos[i].clone()
}

fn guardar(carpeta: &Path, r: &mut Resumen, tipo: &str, v: &vinculos::Vinculo, datos: Value, capturada: &str) {
    let clave = recoleccion::clave(tipo, v);
    let bytes = serde_json::to_vec(&json!({"version":1,"tipo":tipo,"clave":clave,"vinculo":v,"datos":datos})).unwrap();
    let sha256 = huella(&bytes);
    let archivo = format!("lectura-{}-{}.json", &huella(clave.as_bytes())[..20], &sha256[..20]);
    std::fs::write(carpeta.join(&archivo), &bytes).unwrap();
    r.lecturas.push(recoleccion::Referencia {
        tipo: tipo.into(),
        clave,
        archivo,
        sha256,
        capturada: capturada.into(),
        meses: 4,
        tribunal: v.tribunal.clone(),
        causa: v.causa.clone(),
        ingreso: v.ingreso.clone(),
        reutilizada: false,
    });
}

/// Lote ficticio de Cumplimiento: dos personas vinculadas (una con bitácora, historia y calendario leídos; otra con la
/// bitácora pendiente), una tercera sin vínculo y una consulta de Espera sin registros.
fn lote(carpeta: &Path) -> Resumen {
    let cumplimiento = json!({"tribunal":"901","screen":"seguimiento","tab":"Cumplimiento","modality":"2","report":null,"state":null,"month":null,"year":null});
    let espera =
        json!({"tribunal":"901","screen":"seguimiento","tab":"Espera","modality":"2","report":null,"state":null,"month":null,"year":null});
    let mut r: Resumen = serde_json::from_value(json!({"version":1,"lote":{"consultas":[cumplimiento, espera],"fechas":null,
        "conservar_filtros":false,"evidencia":false,"pausa_ms":250,"lecturas":{"bitacoras":true,"historia":true,"calendario":true,"meses":4}},
        "estado":"VALIDADA","terminadas":[0,1],"vacias":[1],"archivos":[],"avisos":[],"lecturas":[],"paginas_lectura":[],
        "inicio":"2026-10-20T11:00:00-03:00","tribunales":{"901":"Juzgado de Letras Ficticio"}}))
    .unwrap();
    let casos: Vec<Value> = serde_json::from_str(include_str!("../../flow-captura/tests/datos/vinculos_ficticios.json")).unwrap();
    let html = casos[0]["html"].as_str().unwrap().replace("tablaEspera", "tablaCumplimiento").replace("11111111-0", RUT);
    let v = vinculos::extraer(html.as_bytes(), &r.lote.consultas[0]).unwrap().remove(0);
    let mut v2 = v.clone();
    v2.fila += 1;
    v2.rit = "X-2-2099".into();
    v2.rit_enlace = "X-2-2099".into();
    v2.rut = RUT2.into();
    v2.nombre = "Otra Persona Ficticia".into();
    v2.causa = "101".into();
    v2.ingreso = "202".into();
    v2.calendario = None;
    r.paginas_lectura.push(recoleccion::Pagina { consulta: 0, numero: 1, vinculos: vec![v.clone(), v2.clone()] });
    let capturada = "2026-10-20T15:00:00Z";
    // Bitácora: el lector real sobre la respuesta ficticia, con la identidad del vínculo.
    let bit = caso(0);
    let html_bit = bit["html"]
        .as_str()
        .unwrap()
        .replace("value=\"11\"", "value=\"901\"")
        .replace("value=\"22\"", "value=\"100\"")
        .replace("value=\"33\"", "value=\"201\"")
        .replace("P-12-2099", "X-1-2099");
    let mut bitacora = lecturas::leer_bitacora(html_bit.as_bytes(), &v.params_bitacora("2").unwrap(), "", capturada).unwrap();
    bitacora["desde"] = json!("2026-06-20");
    bitacora["hasta"] = json!("2026-10-20");
    guardar(carpeta, &mut r, "bitacora", &v, bitacora, capturada);
    // Historia: la ventana de cuatro meses y una orden de búsqueda anterior como contexto.
    let html_hist = caso(2)["html"].as_str().unwrap().replace("/2099", "/2026");
    let mut historia = lecturas::leer_historia(html_hist.as_bytes(), "", capturada).unwrap();
    let mut filas = historia["filas"].as_array().unwrap().clone();
    filas.push(json!({"fila":5,"folio":"9000","etapa":"Cumplimiento","tramite":"Resolución","descripcion":"Orden de búsqueda",
        "referencia":"Sin referencia.","fecha_texto":"05/03/2026","fecha_iso":"2026-03-05","anexo":false}));
    filas.push(json!({"fila":6,"folio":"9121","etapa":"Cumplimiento","tramite":"Actuación","descripcion":"Derecho a ser oído",
        "referencia":"Sin referencia.","fecha_texto":"10/10/2026","fecha_iso":"2026-10-10","anexo":false}));
    let [dentro, anteriores, posteriores, sin_fecha] = lecturas::separar_ventana(&filas, "2026-10-20".parse().unwrap(), 4).unwrap();
    historia["filas"] = json!(dentro.into_iter().chain(posteriores).chain(sin_fecha).collect::<Vec<_>>());
    historia["desde"] = json!("2026-06-20");
    historia["hasta"] = json!("2026-10-20");
    historia["anteriores"] = json!(anteriores.len());
    historia["contexto_anterior"] = json!(recoleccion::contexto_anterior(&anteriores));
    guardar(carpeta, &mut r, "historia", &v, historia, capturada);
    // Calendario del ingreso.
    let html_cal = caso(7)["html"].as_str().unwrap().replace("value=\"49\"", "value=\"901\"");
    let calendario = lecturas::leer_calendario(html_cal.as_bytes(), v.calendario.as_ref().unwrap(), "", capturada).unwrap();
    guardar(carpeta, &mut r, "calendario", &v, calendario, capturada);
    r.avisos.push(format!("{}pendiente. La bitácora no respondió a tiempo.", recoleccion::prefijo_lectura(0, 1, v2.fila, "bitacora")));
    let xls = format!(
        "<html><table><tr><th>RIT</th><th>RUT</th><th>Nombre</th><th>Derivación</th><th>Tribunal</th><th>DIAS DE CUMPLIMIENTO</th>\
         <th>DIAS PARA EGRESAR</th></tr>\
         <tr><td>X-1-2099</td><td>{RUT}</td><td>Persona Ficticia</td><td>PRM Centro Ficticio</td><td>Laja</td><td>100</td><td>100</td></tr>\
         <tr><td>X-2-2099</td><td>{RUT2}</td><td>Otra Persona Ficticia</td><td>PRM Centro Ficticio</td><td>Laja</td><td>200</td><td>10</td></tr>\
         <tr><td>X-3-2099</td><td>33.333.333-3</td><td>Tercera Persona (Ficticia)</td><td>PRM Centro Ficticio</td><td>Laja</td><td>400</td><td>15</td></tr>\
         </table></html>"
    );
    let nombre = "consulta-001-p001-de-001.xls";
    std::fs::write(carpeta.join(nombre), &xls).unwrap();
    r.archivos.push(flow_captura::lotes::Archivo {
        nombre: nombre.into(),
        sha256: huella(xls.as_bytes()),
        consulta: 0,
        pagina: 1,
        total: 1,
        registros: 3,
    });
    std::fs::write(carpeta.join("resumen.json"), serde_json::to_vec_pretty(&r).unwrap()).unwrap();
    r
}

fn datos() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/datos")
}

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
