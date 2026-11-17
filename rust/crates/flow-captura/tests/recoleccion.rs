use base64::{engine::general_purpose::STANDARD, Engine};
use flow_captura::{
    lotes::{self, Conexion, Lote, Resumen},
    recoleccion::{self, Control, Opciones, PaginaActual, Recoleccion},
    sitfa::Seleccion,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Mutex},
    time::Duration,
};

const FECHA: &str = "2026-10-20T12:00:00Z";
struct Ficticia {
    fallar_historia: bool,
    comandos: Mutex<Vec<String>>,
}
impl Conexion for Ficticia {
    fn llamar(&self, comando: &str, payload: Value, _plazo: Duration) -> Result<Value, String> {
        if comando == "capabilities" {
            return Ok(
                json!({"tipo":"FLOW_CAPACIDADES","version":"0.2.0","protocolos":{"descarga_xls":1,"pdf_sin_registros":1,"bitacoras_lectura":1,"historia_lectura":1,"calendario_lectura":1},"escritura_rus":false}),
            );
        }
        self.comandos.lock().unwrap().push(comando.into());
        if comando == "history_open" && self.fallar_historia {
            return Err("Fallo ficticio de historia.".into());
        }
        let casos: Vec<Value> = serde_json::from_str(include_str!("datos/lecturas_ficticias.json")).unwrap();
        let tipo = match comando {
            "diary_open" => "bitacora",
            "history_open" => "historia",
            "calendar_open" => "calendario",
            _ => panic!("Comando inesperado"),
        };
        let html = casos.iter().find(|c| c["tipo"] == tipo).unwrap()["html"].as_str().unwrap().to_string();
        let html = if tipo == "historia" {
            html.replace("C-99999-2099", "X-1-2099")
        } else {
            html.replace("name=\"COD_Tribunal\" value=\"11\"", "name=\"COD_Tribunal\" value=\"901\"")
                .replace("name=\"COD_Tribunal\" value=\"49\"", "name=\"COD_Tribunal\" value=\"901\"")
                .replace("name=\"CRR_IdCausa\" value=\"22\"", "name=\"CRR_IdCausa\" value=\"100\"")
                .replace("name=\"ID_Ingreso\" value=\"33\"", "name=\"ID_Ingreso\" value=\"201\"")
        };
        let mut r = json!({"status":200,"type":"text/html","body":STANDARD.encode(html.as_bytes()),"digest":lotes::huella(html.as_bytes()),"startedAt":FECHA});
        if comando == "calendar_open" {
            r["args"] = payload["args"].clone();
        } else {
            r["params"] = payload["params"].clone();
        }
        Ok(r)
    }
}

static SECUENCIA: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct Temporal(PathBuf);
impl Temporal {
    fn nueva() -> Self {
        let p = std::env::temp_dir().join(format!(
            "flow-recoleccion-ficticia-{}-{}-{}",
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

fn resumen() -> Resumen {
    let s = Seleccion {
        tribunal: "901".into(),
        screen: "seguimiento".into(),
        tab: "Cumplimiento".into(),
        modality: "2".into(),
        report: None,
        state: None,
        month: None,
        year: None,
    };
    Resumen {
        version: 1,
        lote: Lote {
            consultas: vec![s],
            fechas: None,
            conservar_filtros: false,
            evidencia: false,
            pausa_ms: 250,
            lecturas: Opciones { bitacoras: true, historia: true, calendario: true, meses: 4 },
        },
        estado: "EN_CURSO".into(),
        terminadas: vec![],
        vacias: vec![],
        archivos: vec![],
        avisos: vec![],
        paginas_lectura: vec![],
        lecturas: vec![],
        inicio: String::new(),
        tribunales: Default::default(),
    }
}
fn pagina() -> String {
    let c: Vec<Value> = serde_json::from_str(include_str!("datos/vinculos_ficticios.json")).unwrap();
    c[0]["html"].as_str().unwrap().replace("tablaEspera", "tablaCumplimiento")
}
fn leer_pagina(r: &mut Recoleccion, f: &Ficticia, resumen: &mut Resumen) {
    r.pagina(
        f,
        &json!({"tribunales":{"901":"Juzgado Canario de Prueba"}}),
        resumen,
        PaginaActual { consulta: 0, numero: 1, html: pagina().as_bytes() },
        Control { cancelar: &AtomicBool::new(false), progreso: &|_| {} },
    )
    .unwrap();
}

#[test]
fn captura_explicita_renueva_bitacora_legada_propia_y_hermana_sin_tocar_original() {
    for hermana in [false, true] {
        let t = Temporal::nueva();
        let c = t.0.join("lote");
        std::fs::create_dir(&c).unwrap();
        let mut resumen = resumen();
        let f = Ficticia { fallar_historia: false, comandos: Mutex::new(vec![]) };
        let fecha = "2026-10-20".parse().unwrap();
        let mut r = Recoleccion::con_fecha(&c, &resumen, fecha).unwrap();
        leer_pagina(&mut r, &f, &mut resumen);
        let indice = resumen.lecturas.iter().position(|l| l.tipo == "bitacora").unwrap();
        let mut legado = recoleccion::leer(&c, &resumen.lecturas[indice]).unwrap();
        legado["datos"].as_object_mut().unwrap().remove("revision_bitacoras");
        let bytes = serde_json::to_vec(&legado).unwrap();
        let original = c.join("lectura-legada-ficticia.json");
        std::fs::write(&original, &bytes).unwrap();
        resumen.lecturas[indice].archivo = "lectura-legada-ficticia.json".into();
        resumen.lecturas[indice].sha256 = lotes::huella(&bytes);
        resumen.guardar(&c).unwrap();
        let destino = if hermana {
            let p = t.0.join("otro"); std::fs::create_dir(&p).unwrap();
            resumen.lecturas.clear(); p
        } else { c.clone() };
        f.comandos.lock().unwrap().clear();
        let mut r = Recoleccion::con_fecha(&destino, &resumen, fecha).unwrap();
        leer_pagina(&mut r, &f, &mut resumen);
        assert_eq!(*f.comandos.lock().unwrap(), vec!["diary_open".to_string()], "hermana={hermana}");
        assert_eq!(std::fs::read(&original).unwrap(), bytes);
        assert_eq!(resumen.lecturas.len(), 3);
        let referencia = resumen.lecturas.iter().find(|l| l.tipo == "bitacora").unwrap();
        let nueva = recoleccion::leer(&destino, referencia).unwrap();
        assert_eq!(nueva["datos"]["revision_bitacoras"]["version"], 1);
        assert!(nueva["datos"]["revision_bitacoras"]["entries"].is_array());
        let mut r = Recoleccion::con_fecha(&destino, &resumen, fecha).unwrap();
        leer_pagina(&mut r, &f, &mut resumen);
        assert_eq!(f.comandos.lock().unwrap().len(), 1, "V1 se reutiliza sin lectura adicional");
    }
}

#[test]
fn guarda_tres_lecturas_y_retomar_del_mismo_dia_no_abre_otras_ventanas() {
    let t = Temporal::nueva();
    let c = t.0.join("lote");
    std::fs::create_dir(&c).unwrap();
    let mut resumen = resumen();
    let f = Ficticia { fallar_historia: false, comandos: Mutex::new(vec![]) };
    let fecha = "2026-10-20".parse().unwrap();
    let mut r = Recoleccion::con_fecha(&c, &resumen, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut resumen);
    r.terminar(&mut resumen).unwrap();
    assert_eq!(resumen.lecturas.len(), 3);
    assert_eq!(f.comandos.lock().unwrap().len(), 3);
    for referencia in &resumen.lecturas {
        let datos = recoleccion::leer(&c, referencia).unwrap();
        assert!(!datos.to_string().contains("<html"));
        assert!(!datos.to_string().contains("TOKENDOCCANARIO"));
    }
    let mut r = Recoleccion::con_fecha(&c, &resumen, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut resumen);
    assert_eq!(f.comandos.lock().unwrap().len(), 3);
    let otro = t.0.join("otro");
    std::fs::create_dir(&otro).unwrap();
    let mut copia = resumen.clone();
    copia.lecturas.clear();
    let mut r = Recoleccion::con_fecha(&otro, &copia, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut copia);
    assert!(copia.lecturas.iter().all(|l| l.reutilizada));
    assert_eq!(f.comandos.lock().unwrap().len(), 3);
}

/// Como `reuse_base` del Descargador: un paquete nuevo reutiliza las lecturas de hoy de los paquetes anteriores de la
/// misma carpeta de destino (sus lotes quedan en «Paquete CSMP …/respaldo/»).
#[test]
fn paquete_nuevo_reutiliza_lecturas_de_hoy_de_otro_paquete() {
    let t = Temporal::nueva();
    let primero = t.0.join("Paquete CSMP 2026-10-20 10-00-00-000001").join("respaldo").join("lote");
    let segundo = t.0.join("Paquete CSMP 2026-10-20 11-00-00-000001").join("respaldo").join("lote");
    std::fs::create_dir_all(&primero).unwrap();
    std::fs::create_dir_all(&segundo).unwrap();
    let f = Ficticia { fallar_historia: false, comandos: Mutex::new(vec![]) };
    let fecha = "2026-10-20".parse().unwrap();
    let mut resumen = resumen();
    let mut r = Recoleccion::con_fecha(&primero, &resumen, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut resumen);
    assert_eq!(f.comandos.lock().unwrap().len(), 3);
    let mut copia = resumen.clone();
    copia.lecturas.clear();
    let mut r = Recoleccion::con_fecha(&segundo, &copia, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut copia);
    assert_eq!(copia.lecturas.len(), 3);
    assert!(copia.lecturas.iter().all(|l| l.reutilizada));
    assert_eq!(f.comandos.lock().unwrap().len(), 3, "no se abren ventanas nuevas");
}

#[test]
fn fallo_parcial_conserva_lo_leido_y_reintenta_solo_lo_pendiente() {
    let t = Temporal::nueva();
    let c = t.0.join("lote");
    std::fs::create_dir(&c).unwrap();
    let mut resumen = resumen();
    let fecha = "2026-10-20".parse().unwrap();
    let f = Ficticia { fallar_historia: true, comandos: Mutex::new(vec![]) };
    let mut r = Recoleccion::con_fecha(&c, &resumen, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut resumen);
    resumen.terminadas.push(0);
    // A1: como el Descargador, la lectura fallida queda anotada aparte; la consulta con sus XLS sigue terminada
    // y el retomar la vuelve a recorrer solo para reintentar lo pendiente.
    r.terminar(&mut resumen).unwrap();
    assert_eq!(resumen.terminadas, vec![0]);
    assert_eq!(recoleccion::consultas_pendientes(&resumen).into_iter().collect::<Vec<_>>(), vec![0]);
    assert_eq!(recoleccion::lecturas_pendientes(&resumen), 1);
    assert_eq!(resumen.lecturas.len(), 2);
    let f = Ficticia { fallar_historia: false, comandos: Mutex::new(vec![]) };
    let mut r = Recoleccion::con_fecha(&c, &resumen, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut resumen);
    r.terminar(&mut resumen).unwrap();
    assert_eq!(*f.comandos.lock().unwrap(), vec!["history_open"]);
    assert_eq!(resumen.lecturas.len(), 3);
    assert!(!resumen.avisos.iter().any(|s| s.contains("pendiente")), "{:?}", resumen.avisos);
    assert!(recoleccion::consultas_pendientes(&resumen).is_empty());
}

/// A1: un listado cuyos vínculos no se pueden leer no detiene el lote; queda como aviso y se reintenta al retomar.
#[test]
fn vinculos_ilegibles_quedan_como_aviso_y_no_detienen() {
    let t = Temporal::nueva();
    let c = t.0.join("lote");
    std::fs::create_dir(&c).unwrap();
    let mut resumen = resumen();
    let f = Ficticia { fallar_historia: false, comandos: Mutex::new(vec![]) };
    let mut r = Recoleccion::con_fecha(&c, &resumen, "2026-10-20".parse().unwrap()).unwrap();
    // Dos tablas con el mismo id: `extraer` falla; antes eso abortaba todo el lote.
    let doble = pagina().replace("</form>", "<table id=\"tablaCumplimiento\"></table></form>");
    r.pagina(
        &f,
        &json!({"tribunales":{"901":"Juzgado Canario de Prueba"}}),
        &mut resumen,
        PaginaActual { consulta: 0, numero: 1, html: doble.as_bytes() },
        Control { cancelar: &AtomicBool::new(false), progreso: &|_| {} },
    )
    .unwrap();
    assert!(f.comandos.lock().unwrap().is_empty());
    assert!(resumen.avisos.iter().any(|s| s.starts_with("Consulta 1, página 1: vínculos del listado pendientes.")), "{:?}", resumen.avisos);
    assert_eq!(recoleccion::consultas_pendientes(&resumen).into_iter().collect::<Vec<_>>(), vec![0]);
    leer_pagina(&mut r, &f, &mut resumen);
    assert!(recoleccion::consultas_pendientes(&resumen).is_empty(), "{:?}", resumen.avisos);
}

/// A1: como `MAX_FALLAS_SEGUIDAS` del Descargador, cinco bitácoras fallidas seguidas detienen el lote; menos no.
#[test]
fn cinco_bitacoras_fallidas_seguidas_detienen_el_lote() {
    struct Caida(Mutex<usize>);
    impl Conexion for Caida {
        fn llamar(&self, comando: &str, _payload: Value, _plazo: Duration) -> Result<Value, String> {
            assert_eq!(comando, "diary_open");
            *self.0.lock().unwrap() += 1;
            Err("Fallo ficticio de sesión.".into())
        }
    }
    let t = Temporal::nueva();
    let c = t.0.join("lote");
    std::fs::create_dir(&c).unwrap();
    let mut resumen = resumen();
    resumen.lote.lecturas = Opciones { bitacoras: true, historia: false, calendario: false, meses: 4 };
    let f = Caida(Mutex::new(0));
    let mut r = Recoleccion::con_fecha(&c, &resumen, "2026-10-20".parse().unwrap()).unwrap();
    let base = pagina();
    let mut resultados = vec![];
    for numero in 1..=5 {
        // Cada página es un ingreso distinto (otro ID_Ingreso, otra persona) para que ninguno se descarte como ambiguo.
        let html = base
            .replace("'201'", &format!("'2{numero:02}'"))
            .replace("Persona Ficticia", &format!("Persona Ficticia {numero}"))
            .replace("'100'", &format!("'1{numero:02}'"))
            .replace("X-1-2099", &format!("X-{numero}-2099"));
        resultados.push(r.pagina(
            &f,
            &json!({"tribunales":{"901":"Juzgado Canario de Prueba"}}),
            &mut resumen,
            PaginaActual { consulta: 0, numero, html: html.as_bytes() },
            Control { cancelar: &AtomicBool::new(false), progreso: &|_| {} },
        ));
    }
    assert_eq!(*f.0.lock().unwrap(), 5);
    assert!(resultados[..4].iter().all(Result::is_ok), "{resultados:?}");
    let error = resultados[4].clone().unwrap_err();
    assert!(error.contains("5 bitácoras seguidas fallaron"), "{error}");
    assert_eq!(recoleccion::lecturas_pendientes(&resumen), 5);
}

#[test]
fn lectura_alterada_no_se_reutiliza_como_verificada() {
    let t = Temporal::nueva();
    let c = t.0.join("lote");
    std::fs::create_dir(&c).unwrap();
    let mut resumen = resumen();
    let fecha = "2026-10-20".parse().unwrap();
    let f = Ficticia { fallar_historia: false, comandos: Mutex::new(vec![]) };
    let mut r = Recoleccion::con_fecha(&c, &resumen, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut resumen);
    std::fs::write(c.join(&resumen.lecturas[0].archivo), b"alterado ficticio").unwrap();
    assert!(Recoleccion::con_fecha(&c, &resumen, fecha).is_err());
}

#[test]
fn ampliar_periodo_relee_bitacora_e_historia_pero_reutiliza_calendario() {
    let t = Temporal::nueva();
    let c = t.0.join("lote");
    std::fs::create_dir(&c).unwrap();
    let mut resumen = resumen();
    resumen.lote.lecturas.meses = 1;
    let fecha = "2026-10-20".parse().unwrap();
    let f = Ficticia { fallar_historia: false, comandos: Mutex::new(vec![]) };
    let mut r = Recoleccion::con_fecha(&c, &resumen, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut resumen);
    let otro = t.0.join("otro");
    std::fs::create_dir(&otro).unwrap();
    let mut copia = resumen.clone();
    copia.lecturas.clear();
    copia.lote.lecturas.meses = 4;
    let mut r = Recoleccion::con_fecha(&otro, &copia, fecha).unwrap();
    leer_pagina(&mut r, &f, &mut copia);
    assert_eq!(*f.comandos.lock().unwrap(), vec!["diary_open", "history_open", "calendar_open", "diary_open", "history_open"]);
    assert_eq!(copia.lecturas.iter().filter(|l| l.reutilizada).count(), 1);
}
#[test]
fn avisos_de_filas_homonimas_en_paginas_distintas_no_se_borran() {
    let t = Temporal::nueva();
    let c = t.0.join("lote");
    std::fs::create_dir(&c).unwrap();
    let mut resumen = resumen();
    resumen.lote.lecturas.bitacoras = false;
    resumen.lote.lecturas.calendario = false;
    let fallida = Ficticia { fallar_historia: true, comandos: Mutex::new(vec![]) };
    let mut r = Recoleccion::con_fecha(&c, &resumen, "2026-10-20".parse().unwrap()).unwrap();
    leer_pagina(&mut r, &fallida, &mut resumen);
    let aviso = resumen
        .avisos
        .iter()
        .find(|s| s.contains("historia pendiente."))
        .expect("la página 1 debe producir un fallo real de historia")
        .clone();
    // La misma posición de la página 2 no tiene enlace de historia; su aviso no resuelve el fallo de la página 1.
    let segunda = pagina().replace("ShowHistoria", "FuncionNoLeible");
    let sin_fallo = Ficticia { fallar_historia: false, comandos: Mutex::new(vec![]) };
    r.pagina(
        &sin_fallo,
        &json!({"tribunales":{"901":"Juzgado Canario de Prueba"}}),
        &mut resumen,
        PaginaActual { consulta: 0, numero: 2, html: segunda.as_bytes() },
        Control { cancelar: &AtomicBool::new(false), progreso: &|_| {} },
    )
    .unwrap();
    assert!(resumen.avisos.contains(&aviso), "el aviso de la primera página debe conservarse íntegro");
    assert!(resumen.avisos.iter().any(|s| s.contains("historia sin vínculo comprobado;")));
}

/// Meta 4 reconoce las lecturas pendientes por estos prefijos: el texto que se emite debe ser el que se interpreta.
#[test]
fn prefijos_de_aviso_son_los_que_reconoce_retomar() {
    use flow_captura::recoleccion::{prefijo_lectura, prefijo_listado};
    assert_eq!(prefijo_lectura(0, 2, 3, "bitacora"), "Consulta 1, página 2, fila 3: bitacora ");
    assert_eq!(prefijo_listado(4, 1), "Consulta 5, página 1: vínculos del listado ");
}


