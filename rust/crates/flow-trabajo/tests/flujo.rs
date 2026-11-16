//! «Consultar y crear paquete» con un SITFA ficticio que escribe lotes ficticios: carpeta «Paquete CSMP …» con
//! `respaldo/`, calendario de informes por mes, estados PARCIAL/COMPLETO/INCOMPLETO y reanudación.

mod comun;

use comun::{datos, lote_con, Temporal};
use flow_captura::lotes::{huella, Archivo, Lote, Resumen};
use flow_motor::Config;
use flow_trabajo::{exportar, flujo, paquete};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

#[derive(Default)]
struct Falso {
    llamadas: Mutex<Vec<String>>,
    /// Mes del calendario que falla la próxima vez.
    falla_mes: Mutex<Option<String>>,
    /// La próxima consulta de Seguimiento se corta.
    cortar: AtomicBool,
    cancelar: AtomicBool,
    /// El Excel del calendario cambia después de verificarse: el paquete no puede incluirlo.
    alterar: AtomicBool,
}

fn calendario(carpeta: &Path, lote: &Lote, alterar: bool) -> Resumen {
    let mes = lote.consultas[0].month.clone().unwrap();
    let mut r: Resumen = serde_json::from_value(json!({"version":1,"lote":lote,"estado":"VALIDADA","terminadas":[0],
        "vacias":[],"archivos":[],"avisos":[],"inicio":"2026-10-20T12:00:00-03:00",
        "tribunales":{"901":"Juzgado de Letras Ficticio"}}))
    .unwrap();
    if mes == "11" {
        let xls = format!(
            "<html><table><tr><th>RIT</th><th>RUT</th><th>Nombre</th><th>Derivacion</th><th>Tribunal</th><th>FECHA VENCIMIENTO</th></tr>\
             <tr><td>X-1-2099</td><td>{}</td><td>Persona Ficticia</td><td>PRM Centro Ficticio</td><td>Laja</td><td>15/11/2026</td></tr>\
             </table></html>",
            comun::RUT
        );
        let nombre = "consulta-001-p001-de-001.xls";
        std::fs::write(carpeta.join(nombre), if alterar { "alterado ficticio" } else { xls.as_str() }).unwrap();
        r.archivos.push(Archivo { nombre: nombre.into(), sha256: huella(xls.as_bytes()), consulta: 0, pagina: 1, total: 1, registros: 1 });
    } else {
        r.vacias.push(0);
    }
    std::fs::write(carpeta.join("resumen.json"), serde_json::to_vec_pretty(&r).unwrap()).unwrap();
    r
}

impl flujo::Sitfa for Falso {
    fn lote(&self, lote: &Lote, carpeta: &Path, retomar: bool) -> Result<Resumen, String> {
        let s = &lote.consultas[0];
        let nombre =
            if s.screen == "calendario_informes" { format!("calendario {}", s.month.as_deref().unwrap()) } else { s.screen.clone() };
        self.llamadas.lock().unwrap().push(format!("{nombre}{}", if retomar { " (retomar)" } else { "" }));
        if s.screen == "calendario_informes" {
            if self.falla_mes.lock().unwrap().as_deref() == s.month.as_deref() {
                *self.falla_mes.lock().unwrap() = None;
                return Err("Fallo ficticio del calendario.".into());
            }
            return Ok(calendario(carpeta, lote, self.alterar.load(Ordering::SeqCst)));
        }
        assert!(lote.lecturas.bitacoras, "las bitácoras siempre se leen");
        let mut r = lote_con(carpeta, !retomar);
        r.lote = lote.clone();
        if self.cortar.swap(false, Ordering::SeqCst) {
            r.estado = "INCOMPLETA".into();
            r.avisos.push("Lote detenido: sesión ficticia vencida.".into());
            std::fs::write(carpeta.join("resumen.json"), serde_json::to_vec_pretty(&r).unwrap()).unwrap();
            return Err("Sesión ficticia vencida.".into());
        }
        std::fs::write(carpeta.join("resumen.json"), serde_json::to_vec_pretty(&r).unwrap()).unwrap();
        Ok(r)
    }
    fn catalogo_informes(&self) -> Result<Value, String> {
        self.llamadas.lock().unwrap().push("catálogo informes".into());
        Ok(json!({}))
    }
    fn volver(&self) {
        self.llamadas.lock().unwrap().push("volver a Seguimiento".into());
    }
    fn cancelado(&self) -> bool {
        self.cancelar.load(Ordering::SeqCst)
    }
    fn avance(&self, _: String) {}
}

fn plan() -> Lote {
    let t = Temporal::nueva();
    let r = comun::lote(&t.0);
    r.lote
}

fn prefs() -> flujo::Preferencias {
    flujo::Preferencias { leer_historia: true, leer_calendarios: true, ..Default::default() }
}

const HOY: &str = "2026-10-20";

#[test]
fn paquete_parcial_se_retoma_hasta_completo() {
    let destino = Temporal::nueva();
    let sitfa = Falso::default();
    *sitfa.falla_mes.lock().unwrap() = Some("11".into());
    let r = flujo::ejecutar(&sitfa, &plan(), &destino.0, &prefs(), HOY.parse().unwrap()).unwrap();
    assert!(r.parcial);
    assert!(r.carpeta.file_name().unwrap().to_string_lossy().starts_with("Paquete CSMP "));
    assert_eq!(r.paquete, r.carpeta.join("Captura_CSMP.zip"));
    assert!(r.avisos.iter().any(|a| a.starts_with("No se incluyó completo el calendario de informes por vencer: Fallo ficticio")));
    assert_eq!(
        *sitfa.llamadas.lock().unwrap(),
        ["seguimiento", "catálogo informes", "calendario 10", "calendario 11", "volver a Seguimiento"]
    );
    let control: Value = serde_json::from_slice(&std::fs::read(r.carpeta.join("respaldo/proceso.json")).unwrap()).unwrap();
    assert_eq!(control["estado"], "PARCIAL");
    assert!(control["carpetas"]["consultas"].as_str().unwrap().starts_with("respaldo/Flow Captura "));
    assert!(flujo::pendientes(&destino.0, 7).is_empty(), "un paquete PARCIAL terminó: se revisa en el Asistente");
    // El paquete parcial ya se abre en el Asistente.
    let zip = std::fs::read(&r.paquete).unwrap();
    let corte = HOY.parse().unwrap();
    assert_eq!(paquete::importar("x.zip", &zip, corte, &Config::default()).unwrap().trabajos[0].filas.len(), 3);

    sitfa.llamadas.lock().unwrap().clear();
    let r2 = flujo::retomar(&sitfa, &r.carpeta, &prefs(), HOY.parse().unwrap()).unwrap();
    assert!(!r2.parcial, "{:?}", r2.avisos);
    assert_eq!(
        *sitfa.llamadas.lock().unwrap(),
        ["seguimiento (retomar)", "catálogo informes", "calendario 11", "calendario 12", "volver a Seguimiento"],
        "el mes 10 validado se reutiliza; el 11 cortado se consulta de nuevo"
    );
    let control: Value = serde_json::from_slice(&std::fs::read(r.carpeta.join("respaldo/proceso.json")).unwrap()).unwrap();
    assert_eq!(control["estado"], "COMPLETO");
    assert_eq!(control["reintentos"], 1);
    let anteriores = std::fs::read_dir(r.carpeta.join("respaldo"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("Captura_CSMP anterior "))
        .count();
    assert_eq!(anteriores, 1, "el ZIP parcial queda en respaldo");
    // El calendario de informes por vencer se asocia al ingreso verificado: aporta la fecha del próximo informe.
    let zip = std::fs::read(&r2.paquete).unwrap();
    let trabajos = paquete::importar("x.zip", &zip, corte, &Config::default()).unwrap().trabajos;
    assert_eq!(trabajos.len(), 1, "el calendario solo aporta fechas al cruce");
    assert_eq!(trabajos[0].filas[0].proximo_informe, Some("2026-11-15".parse().unwrap()));
    assert_eq!(trabajos[0].filas[1].cronologia.as_ref().unwrap()["status"], "ACREDITADA");
    let referencia = datos().join("paquete_flujo.zip");
    if std::env::var_os("FLOW_REGENERAR").is_some() {
        std::fs::write(&referencia, &zip).unwrap();
    }
    assert_eq!(std::fs::read(&referencia).unwrap(), zip, "el paquete del flujo ficticio cambió; regenera con FLOW_REGENERAR=1");
    assert!(flujo::retomar(&sitfa, &r.carpeta, &prefs(), HOY.parse().unwrap()).unwrap_err().contains("ya está completo"));
}

#[test]
fn calendario_que_no_entra_al_paquete_no_lo_impide() {
    let destino = Temporal::nueva();
    let sitfa = Falso::default();
    sitfa.alterar.store(true, Ordering::SeqCst);
    let r = flujo::ejecutar(&sitfa, &plan(), &destino.0, &prefs(), HOY.parse().unwrap()).unwrap();
    assert!(r.parcial);
    assert!(
        r.avisos.iter().any(|a| a.starts_with("El calendario de informes por vencer no pudo incluirse en el paquete:")),
        "{:?}",
        r.avisos
    );
    let zip = std::fs::read(&r.paquete).unwrap();
    let trabajos = paquete::importar("x.zip", &zip, HOY.parse().unwrap(), &Config::default()).unwrap().trabajos;
    assert_eq!(trabajos[0].filas[0].proximo_informe, None);
    let control: Value = serde_json::from_slice(&std::fs::read(r.carpeta.join("respaldo/proceso.json")).unwrap()).unwrap();
    assert_eq!(control["estado"], "PARCIAL");
}

#[test]
fn corte_en_las_consultas_deja_incompleto_y_retomar_continua() {
    let destino = Temporal::nueva();
    let sitfa = Falso::default();
    sitfa.cortar.store(true, Ordering::SeqCst);
    let sin_informes = flujo::Preferencias { informes_por_vencer: false, ..prefs() };
    assert!(flujo::ejecutar(&sitfa, &plan(), &destino.0, &sin_informes, HOY.parse().unwrap()).is_err());
    let pendientes = flujo::pendientes(&destino.0, 7);
    assert_eq!(pendientes.len(), 1);
    assert_eq!(pendientes[0].1, "consultas");
    let carpeta = &pendientes[0].0;
    assert!(!carpeta.join("Captura_CSMP.zip").exists());
    let control: Value = serde_json::from_slice(&std::fs::read(carpeta.join("respaldo/proceso.json")).unwrap()).unwrap();
    assert_eq!(control["estado"], "INCOMPLETO");
    let r = flujo::retomar(&sitfa, carpeta, &sin_informes, HOY.parse().unwrap()).unwrap();
    assert!(!r.parcial);
    assert!(r.paquete.exists());
    assert_eq!(*sitfa.llamadas.lock().unwrap(), ["seguimiento", "seguimiento (retomar)"]);
}

#[test]
fn detener_deja_incompleto_y_solo_seguimiento_crea_paquete() {
    let destino = Temporal::nueva();
    let sitfa = Falso::default();
    sitfa.cancelar.store(true, Ordering::SeqCst);
    assert!(flujo::ejecutar(&sitfa, &plan(), &destino.0, &prefs(), HOY.parse().unwrap()).unwrap_err().contains("Proceso detenido"));
    assert_eq!(flujo::pendientes(&destino.0, 7).len(), 1);
    let mut otra = plan();
    otra.consultas[0].screen = "litigantes".into();
    assert!(flujo::ejecutar(&Falso::default(), &otra, &destino.0, &prefs(), HOY.parse().unwrap()).unwrap_err().contains("Seguimiento"));
}

#[test]
fn paquete_del_flujo_igual_que_el_asistente_python() {
    let esperado: Value = serde_json::from_slice(&std::fs::read(datos().join("paquete_flujo.json")).unwrap()).unwrap();
    let zip = std::fs::read(datos().join("paquete_flujo.zip")).unwrap();
    let corte = esperado["corte"].as_str().unwrap().parse().unwrap();
    let p = paquete::importar(exportar::NOMBRE, &zip, corte, &Config::default()).unwrap();
    let filas: Vec<_> = p.trabajos.iter().flat_map(|t| t.filas.iter()).collect();
    let esperadas = esperado["filas"].as_array().unwrap();
    assert_eq!(filas.len(), esperadas.len());
    for e in esperadas {
        let rit = e["rit"].as_str().unwrap();
        let f = filas.iter().find(|f| f.registro.texto("rit") == rit).unwrap_or_else(|| panic!("falta {rit}"));
        assert_eq!(f.propuesta.observacion, e["observacion"].as_str().unwrap(), "{rit}");
        assert_eq!(json!(f.propuesta.reglas), e["reglas"], "{rit}");
        assert_eq!(json!(f.propuesta.acciones), e["acciones"], "{rit}");
        let estado = f.cronologia.as_ref().map(|c| c["status"].clone()).unwrap_or(json!(""));
        assert_eq!(estado, e["cronologia"], "{rit}");
    }
}

/// SITFA ficticio del calendario de informes por vencer (`calendario_informes/` de `sitfa_ficticio.json`): su Excel
/// trae solo RIT, Nombre y Fecha Vencimiento.
struct CalendarioFicticio {
    caso: Value,
}

impl flow_captura::lotes::Conexion for CalendarioFicticio {
    fn llamar(&self, comando: &str, _payload: Value, _plazo: std::time::Duration) -> Result<Value, String> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let codificado = |s: &str| json!({"status":200,"type":"text/html; charset=utf-8","body":STANDARD.encode(s)});
        match comando {
            "capabilities" => Ok(json!({"tipo":"FLOW_CAPACIDADES","version":"0.2.0","protocolos":{"descarga_xls":1,"pdf_sin_registros":1,
                "bitacoras_lectura":1,"historia_lectura":1,"calendario_lectura":1},"escritura_rus":false})),
            "catalog" => Ok(self.caso["catalogo"].clone()),
            "lock" | "unlock" => Ok(json!({"ok":true,"restored":true})),
            "prepare" => Ok(json!({"pairs":self.caso["pares"]})),
            "search" => Ok(codificado(self.caso["html"].as_str().unwrap())),
            "download" => Ok(codificado(self.caso["excel"].as_str().unwrap())),
            _ => Err(format!("Comando inesperado: {comando}")),
        }
    }
}

/// El lote del calendario que arma el flujo pasa la validación real del lote y su Excel reducido entra al paquete.
#[test]
fn calendario_real_con_columnas_reducidas_entra_al_paquete() {
    let casos: Vec<Value> = serde_json::from_str(include_str!("../../flow-captura/tests/datos/sitfa_ficticio.json")).unwrap();
    let conexion = CalendarioFicticio { caso: casos.into_iter().find(|c| c["nombre"] == "calendario_informes/").unwrap() };
    let cancelar = AtomicBool::new(false);
    let sitfa =
        flujo::EnChrome { conexion: &conexion, catalogo: &json!({}), cancelar: &cancelar, progreso: &|_| {}, informes: Default::default() };
    use flujo::Sitfa;
    sitfa.catalogo_informes().unwrap();
    let t = Temporal::nueva();
    let seguimiento = t.0.join("seguimiento");
    let informes = t.0.join("informes");
    std::fs::create_dir(&seguimiento).unwrap();
    std::fs::create_dir(&informes).unwrap();
    let prefs = flujo::Preferencias { pausa_ms: 250, pdf_sin_registros: false, ..prefs() };
    let plan = flujo::plan_informes(&["901".into()], "10", "2026", &prefs);
    let cal = sitfa.lote(&plan, &informes, false).unwrap();
    assert_eq!(cal.estado, "VALIDADA");
    assert_eq!(cal.tribunales["901"], "Juzgado de Familia Ficticio Norte");
    let r = lote_con(&seguimiento, false);
    let lotes = [exportar::Lote { carpeta: &seguimiento, resumen: &r }, exportar::Lote { carpeta: &informes, resumen: &cal }];
    let p = exportar::construir(&lotes, &Default::default()).unwrap();
    let corte = HOY.parse().unwrap();
    let trabajos = paquete::importar("x.zip", &p.datos, corte, &Config::default()).unwrap().trabajos;
    assert_eq!(trabajos.len(), 1, "el calendario solo aporta fechas al cruce");
    if std::env::var_os("FLOW_REGENERAR").is_some() {
        std::fs::write(datos().join("paquete_calendario.zip"), &p.datos).unwrap();
    }
}
