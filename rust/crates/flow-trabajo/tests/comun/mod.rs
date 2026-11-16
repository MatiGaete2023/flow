//! Lote ficticio del Descargador Rust compartido por las pruebas del paquete y del flujo.
#![allow(dead_code)]

use flow_captura::{lecturas, lotes::huella, lotes::Resumen, recoleccion, vinculos};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

static SECUENCIA: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct Temporal(pub PathBuf);
impl Temporal {
    pub fn nueva() -> Self {
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

pub const RUT: &str = "11.111.111-1";
pub const RUT2: &str = "22.222.222-2";

fn caso(i: usize) -> Value {
    let casos: Vec<Value> = serde_json::from_str(include_str!("../../../flow-captura/tests/datos/lecturas_ficticias.json")).unwrap();
    casos[i].clone()
}

pub fn guardar(carpeta: &Path, r: &mut Resumen, tipo: &str, v: &vinculos::Vinculo, datos: Value, capturada: &str) {
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
pub fn lote(carpeta: &Path) -> Resumen {
    lote_con(carpeta, true)
}

/// Como [`lote`]; sin `pendiente`, la bitácora de la segunda persona ya se leyó (sin aviso pendiente).
pub fn lote_con(carpeta: &Path, pendiente: bool) -> Resumen {
    let cumplimiento = json!({"tribunal":"901","screen":"seguimiento","tab":"Cumplimiento","modality":"2","report":null,"state":null,"month":null,"year":null});
    let espera =
        json!({"tribunal":"901","screen":"seguimiento","tab":"Espera","modality":"2","report":null,"state":null,"month":null,"year":null});
    let mut r: Resumen = serde_json::from_value(json!({"version":1,"lote":{"consultas":[cumplimiento, espera],"fechas":null,
        "conservar_filtros":false,"evidencia":false,"pausa_ms":250,"lecturas":{"bitacoras":true,"historia":true,"calendario":true,"meses":4}},
        "estado":"VALIDADA","terminadas":[0,1],"vacias":[1],"archivos":[],"avisos":[],"lecturas":[],"paginas_lectura":[],
        "inicio":"2026-10-20T11:00:00-03:00","tribunales":{"901":"Juzgado de Letras Ficticio"}}))
    .unwrap();
    let casos: Vec<Value> = serde_json::from_str(include_str!("../../../flow-captura/tests/datos/vinculos_ficticios.json")).unwrap();
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
    if !pendiente {
        let html2 = bit["html"]
            .as_str()
            .unwrap()
            .replace("value=\"11\"", "value=\"901\"")
            .replace("value=\"22\"", "value=\"101\"")
            .replace("value=\"33\"", "value=\"202\"")
            .replace("P-12-2099", "X-2-2099");
        let mut b2 = lecturas::leer_bitacora(html2.as_bytes(), &v2.params_bitacora("2").unwrap(), "", capturada).unwrap();
        b2["desde"] = json!("2026-06-20");
        b2["hasta"] = json!("2026-10-20");
        guardar(carpeta, &mut r, "bitacora", &v2, b2, capturada);
    }
    if pendiente {
        r.avisos.push(format!("{}pendiente. La bitácora no respondió a tiempo.", recoleccion::prefijo_lectura(0, 1, v2.fila, "bitacora")));
    }
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

pub fn datos() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/datos")
}
