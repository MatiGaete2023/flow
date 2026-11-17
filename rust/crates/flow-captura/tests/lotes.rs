//! Fallos entre páginas, reanudación, cancelación y PDF usando únicamente respuestas ficticias.

use base64::{engine::general_purpose::STANDARD, Engine};
use flow_captura::lotes::{self, Conexion, Lote, Resumen};
use flow_captura::sitfa::Seleccion;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};

struct Ficticio<'a> {
    caso: Value,
    estado: Mutex<(usize, usize, Vec<String>)>,
    cancelar: Option<&'a AtomicBool>,
    falla: bool,
}

fn caso(nombre: &str) -> Value {
    serde_json::from_str::<Vec<Value>>(include_str!("datos/sitfa_ficticio.json"))
        .unwrap()
        .into_iter()
        .find(|c| c["nombre"] == nombre)
        .unwrap()
}

fn codificado(s: &str) -> Value {
    json!({"status":200,"type":"text/html; charset=utf-8","body":STANDARD.encode(s)})
}

impl Conexion for Ficticio<'_> {
    fn llamar(&self, comando: &str, payload: Value, _plazo: Duration) -> Result<Value, String> {
        let mut estado = self.estado.lock().unwrap();
        estado.2.push(comando.into());
        match comando {
            "capabilities" => Ok(
                json!({"tipo":"FLOW_CAPACIDADES","version":"0.2.0","protocolos":{"descarga_xls":1,"pdf_sin_registros":1,"bitacoras_lectura":1,"historia_lectura":1,"calendario_lectura":1},"escritura_rus":false}),
            ),
            "catalog" => Ok(self.caso["catalogo"].clone()),
            "lock" | "unlock" | "evidence_close" => Ok(json!({"ok":true,"restored":true})),
            "prepare" => Ok(json!({"pairs":self.caso["pares"]})),
            "search" => {
                estado.0 = payload["pairs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|p| p[0].as_str().unwrap().starts_with("NUM_Pagina"))
                    .and_then(|p| p[1].as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
                if self.falla && estado.0 == 2 {
                    return Err("Fallo ficticio en Chrome.".into());
                }
                Ok(codificado(self.caso[if estado.0 == 2 { "html2" } else { "html" }].as_str().unwrap()))
            }
            "download" => {
                estado.1 += 1;
                if let Some(c) = self.cancelar {
                    if estado.1 == 1 {
                        c.store(true, Ordering::SeqCst);
                    }
                }
                Ok(codificado(self.caso[if estado.0 == 2 { "excel2" } else { "excel" }].as_str().unwrap()))
            }
            "evidence_open" => {
                let contexto = payload["context"].to_string();
                let texto = format!(
                    "Consulta solicitada · {}",
                    payload["context"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|p| format!("{}: {}", p[0].as_str().unwrap(), p[1].as_str().unwrap()))
                        .collect::<Vec<_>>()
                        .join(" · ")
                );
                let escapar = |s: &str| s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;");
                let html = self.caso["html"]
                    .as_str()
                    .unwrap()
                    .replace(
                        "<body>",
                        &format!("<body><div id='sitfaContextoConsulta' data-context=\"{}\">{}</div>", escapar(&contexto), texto),
                    )
                    .replace("name=\"COD_Lengueta\" value=\"\"", "name=\"COD_Lengueta\" value=\"tdEspera\"");
                let mut r = codificado(&html);
                r["digest"] = json!(lotes::huella(html.as_bytes()));
                Ok(r)
            }
            "evidence_capture" => {
                let imagen = image::RgbImage::from_pixel(800, 400, image::Rgb([255, 255, 255]));
                let mut png = std::io::Cursor::new(Vec::new());
                imagen.write_to(&mut png, image::ImageFormat::Png).unwrap();
                Ok(json!({"png":STANDARD.encode(png.into_inner()),"digest":payload["digest"]}))
            }
            _ => panic!("Comando inesperado: {comando}"),
        }
    }
}

fn plan(c: &Value, evidencia: bool) -> Lote {
    Lote {
        consultas: vec![serde_json::from_value::<Seleccion>(c["seleccion"].clone()).unwrap()],
        fechas: None,
        conservar_filtros: false,
        evidencia,
        pausa_ms: 250,
        lecturas: Default::default(),
    }
}

struct Carpeta(PathBuf);
impl Carpeta {
    fn nueva() -> Self {
        static NUMERO: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let raiz =
            std::env::temp_dir().join(format!("flow-lote-ficticio-{}-{}", std::process::id(), NUMERO.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir(&raiz).unwrap();
        Self(raiz)
    }
}
impl Drop for Carpeta {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn descarga_dos_paginas_con_huellas_y_no_sobrescribe() {
    let f = Ficticio { caso: caso("seguimiento/Espera"), estado: Mutex::new((1, 0, vec![])), cancelar: None, falla: false };
    let c = Carpeta::nueva();
    let lote = plan(&f.caso, false);
    let cancelar = AtomicBool::new(false);
    let r = lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, false, &cancelar, &|_| {}).unwrap();
    assert_eq!(r.estado, "VALIDADA");
    assert_eq!(r.archivos.len(), 2);
    assert_eq!(r.archivos.iter().map(|a| a.registros).sum::<usize>(), 6);
    assert!(r.archivos.iter().all(|a| a.sha256.len() == 64));
    assert!(lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, false, &cancelar, &|_| {}).is_err());
    let r = lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, true, &cancelar, &|_| {}).unwrap();
    assert_eq!(r.archivos.len(), 2);
    assert_eq!(f.estado.lock().unwrap().1, 2);
    assert_eq!(f.estado.lock().unwrap().2.last().unwrap(), "unlock");
}

#[test]
fn cancelar_conserva_checkpoint_y_retomar_no_duplica_paginas() {
    let cancelar = AtomicBool::new(false);
    let c = Carpeta::nueva();
    let f = Ficticio { caso: caso("seguimiento/Espera"), estado: Mutex::new((1, 0, vec![])), cancelar: Some(&cancelar), falla: false };
    let lote = plan(&f.caso, false);
    assert!(lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, false, &cancelar, &|_| {}).unwrap_err().contains("cancelado"));
    let r = Resumen::leer(&c.0).unwrap();
    assert_eq!(r.estado, "INCOMPLETA");
    assert_eq!(r.archivos.len(), 1);
    assert!(r.terminadas.is_empty());
    assert_eq!(f.estado.lock().unwrap().2.last().unwrap(), "unlock");
    cancelar.store(false, Ordering::SeqCst);
    let r = lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, true, &cancelar, &|_| {}).unwrap();
    assert_eq!(r.estado, "VALIDADA");
    assert_eq!(r.archivos.len(), 2);
    assert_eq!(f.estado.lock().unwrap().1, 2);
}

#[test]
fn fallo_de_chrome_desbloquea_formulario_y_permite_retomar() {
    let f = Ficticio { caso: caso("seguimiento/Espera"), estado: Mutex::new((1, 0, vec![])), cancelar: None, falla: true };
    let c = Carpeta::nueva();
    let lote = plan(&f.caso, false);
    assert!(lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, false, &AtomicBool::new(false), &|_| {}).is_err());
    assert_eq!(f.estado.lock().unwrap().2.last().unwrap(), "unlock");
    assert_eq!(Resumen::leer(&c.0).unwrap().archivos.len(), 1);
}

#[test]
fn retomar_rechaza_archivos_editados_y_rutas_ajenas() {
    let f = Ficticio { caso: caso("seguimiento/Espera"), estado: Mutex::new((1, 0, vec![])), cancelar: None, falla: false };
    let c = Carpeta::nueva();
    let lote = plan(&f.caso, false);
    let cancelar = AtomicBool::new(false);
    let mut r = lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, false, &cancelar, &|_| {}).unwrap();
    std::fs::write(c.0.join(&r.archivos[0].nombre), b"archivo modificado ficticio").unwrap();
    assert!(lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, true, &cancelar, &|_| {}).unwrap_err().contains("Cambió"));
    r.archivos[0].nombre = "../ajeno.xls".into();
    r.guardar(&c.0).unwrap();
    assert!(Resumen::leer(&c.0).is_err());
    for nombre in ["../x.xls", "C:\\x.xls", "/x.xls", "x/y.xls", ".", ".."] {
        assert!(lotes::ruta_archivo(&c.0, nombre).is_err());
    }
}

#[test]
fn consulta_vacia_solo_genera_pdf_si_se_pide_y_queda_acreditada() {
    for evidencia in [false, true] {
        let f = Ficticio { caso: caso("seguimiento/Espera/vacia"), estado: Mutex::new((1, 0, vec![])), cancelar: None, falla: false };
        let c = Carpeta::nueva();
        let lote = plan(&f.caso, evidencia);
        let r = lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, false, &AtomicBool::new(false), &|_| {}).unwrap();
        assert_eq!(r.vacias, vec![0]);
        assert_eq!(r.archivos.len(), usize::from(evidencia));
        if evidencia {
            let pdf = std::fs::read(c.0.join(&r.archivos[0].nombre)).unwrap();
            assert!(pdf.starts_with(b"%PDF-1.4"));
            assert!(pdf.ends_with(b"%%EOF\n"));
        }
        assert_eq!(f.estado.lock().unwrap().2.contains(&"evidence_capture".into()), evidencia);
    }
}

#[test]
fn capacidades_y_respuestas_fuera_de_contrato_fallan_cerrado() {
    let correcto = json!({"tipo":"FLOW_CAPACIDADES","version":"0.2.0","protocolos":{"descarga_xls":1,"pdf_sin_registros":1,"bitacoras_lectura":1,"historia_lectura":1,"calendario_lectura":1},"escritura_rus":false});
    lotes::comprobar_capacidades(&correcto).unwrap();
    for (campo, valor) in [
        ("escritura_rus", json!(true)),
        ("escritura_rus", Value::Null),
        ("version", json!("otra")),
        ("tipo", json!("SITFA_LITE_CAPACIDADES")),
        ("protocolos", json!({"descarga_xls":1})),
    ] {
        let mut v = correcto.clone();
        v[campo] = valor;
        assert!(lotes::comprobar_capacidades(&v).is_err());
    }
    for v in [
        json!({}),
        json!({"status":200,"body":"###","type":"text/html"}),
        json!({"status":302,"body":STANDARD.encode("hola"),"type":"text/html"}),
        json!({"status":200,"body":STANDARD.encode("hola"),"type":"application/json"}),
    ] {
        assert!(lotes::cuerpo(&v, false).is_err());
    }
    assert!(flow_captura::evidencia::png_a_pdf(&json!({"png":STANDARD.encode("no-es-png"),"digest":"otra"}), "esperada").is_err());
}
#[test]
fn retomar_evidencia_sin_checkpoint_conserva_pdf_y_completa_consulta() {
    let f = Ficticio { caso: caso("seguimiento/Espera/vacia"), estado: Mutex::new((1, 0, vec![])), cancelar: None, falla: false };
    let c = Carpeta::nueva();
    let lote = plan(&f.caso, true);
    let cancelar = AtomicBool::new(false);
    let mut r = lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, false, &cancelar, &|_| {}).unwrap();
    let original = c.0.join(&r.archivos[0].nombre);
    let pdf = std::fs::read(&original).unwrap();
    r.archivos.clear();
    r.vacias.clear();
    r.terminadas.clear();
    r.estado = "INCOMPLETA".into();
    r.guardar(&c.0).unwrap();
    let r = lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, true, &cancelar, &|_| {}).unwrap();
    assert_eq!(r.estado, "VALIDADA");
    assert_eq!(r.archivos.len(), 1);
    assert_eq!(r.vacias, vec![0]);
    assert_ne!(c.0.join(&r.archivos[0].nombre), original);
    assert_eq!(std::fs::read(original).unwrap(), pdf);
    assert!(r.avisos.iter().any(|s| s.contains("sin checkpoint")));
    assert_eq!(Resumen::leer(&c.0).unwrap().archivos.len(), 1);
}

/// Extensión que responde solo lo indicado y registra el orden y plazo de cada orden.
struct Orden {
    catalogo: Result<Value, String>,
    llamadas: Mutex<Vec<(String, Duration)>>,
}

impl Conexion for Orden {
    fn llamar(&self, comando: &str, _payload: Value, plazo: Duration) -> Result<Value, String> {
        self.llamadas.lock().unwrap().push((comando.into(), plazo));
        match comando {
            "catalog" => self.catalogo.clone(),
            "capabilities" => Ok(
                json!({"tipo":"FLOW_CAPACIDADES","version":"0.2.0","protocolos":{"descarga_xls":1,"pdf_sin_registros":1,"bitacoras_lectura":1,"historia_lectura":1,"calendario_lectura":1},"escritura_rus":false}),
            ),
            _ => Err("orden no esperada".into()),
        }
    }
}

#[test]
fn opciones_se_piden_antes_que_la_version_y_con_plazos_acotados() {
    // Prueba del usuario (13-nov-2026): «se queda pegado en la carga de opciones».
    let bien = Orden { catalogo: Ok(caso("seguimiento/Espera")["catalogo"].clone()), llamadas: Mutex::new(vec![]) };
    lotes::catalogo(&bien, "seguimiento").unwrap();
    let llamadas = bien.llamadas.lock().unwrap().clone();
    assert_eq!(llamadas.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(), ["catalog", "capabilities"]);
    assert!(llamadas.iter().all(|(_, p)| *p <= Duration::from_secs(75)), "{llamadas:?}");
    // SITFA sin pantalla de consulta: el mensaje de la extensión llega tal cual y no se espera nada más.
    let sin_pantalla = Orden {
        catalogo: Err("El enlace con Chrome funciona, pero no se encuentra la pantalla de consulta.".into()),
        llamadas: Mutex::new(vec![]),
    };
    let error = lotes::catalogo(&sin_pantalla, "seguimiento").unwrap_err();
    assert!(error.contains("pantalla de consulta"), "{error}");
    assert_eq!(sin_pantalla.llamadas.lock().unwrap().len(), 1);
}

/// Listado ficticio de Espera con identidad completa y un enlace de bitácora por fila; delega el resto en `Ficticio`.
struct ConLecturas<'a> {
    base: Ficticio<'a>,
    /// Cuántas de las próximas aperturas de bitácora fallan.
    fallas: Mutex<usize>,
    aperturas: Mutex<usize>,
}

fn con_enlaces(html: &str) -> String {
    let mut html = html.replace(
        "<tr><td>RIT</td><td>Nombre</td></tr>",
        "<tr><td>RIT</td><td>RUT</td><td>Nombre</td><td>Derivación</td><td>Acciones</td></tr>",
    );
    for p in 1..=2 {
        for f in 0..3 {
            html = html
                .replace(&format!("<td>X-{p}{f}-2099</td><td>Persona"), &format!("<td>X-{p}{f}-2099</td><td>1{p}{f}-0</td><td>Persona"));
            html = html.replace(
                &format!("{p}-{f}</td></tr>"),
                &format!("{p}-{f}</td><td>PRM Ficticio</td><td><a onclick=\"ShowObservaciones('5{p}{f}','901','7{p}{f}','12','3','0')\">Obs</a></td></tr>"),
            );
        }
    }
    html
}

impl Conexion for ConLecturas<'_> {
    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
        match comando {
            "search" => {
                let r = self.base.llamar(comando, payload, plazo)?;
                let html = String::from_utf8(STANDARD.decode(r["body"].as_str().unwrap()).unwrap()).unwrap();
                Ok(codificado(&con_enlaces(&html)))
            }
            "diary_open" => {
                *self.aperturas.lock().unwrap() += 1;
                let mut fallas = self.fallas.lock().unwrap();
                if *fallas > 0 {
                    *fallas -= 1;
                    return Err("Fallo ficticio de bitácora.".into());
                }
                let p = &payload["params"];
                let casos: Vec<Value> = serde_json::from_str(include_str!("datos/lecturas_ficticias.json")).unwrap();
                let html = casos[0]["html"]
                    .as_str()
                    .unwrap()
                    .replace("name=\"COD_Tribunal\" value=\"11\"", "name=\"COD_Tribunal\" value=\"901\"")
                    .replace(
                        "name=\"CRR_IdCausa\" value=\"22\"",
                        &format!("name=\"CRR_IdCausa\" value=\"{}\"", p["CRR_IdCausa"].as_str().unwrap()),
                    )
                    .replace(
                        "name=\"ID_Ingreso\" value=\"33\"",
                        &format!("name=\"ID_Ingreso\" value=\"{}\"", p["ID_Ingreso"].as_str().unwrap()),
                    );
                let mut r = json!({"status":200,"type":"text/html","body":STANDARD.encode(html.as_bytes()),"digest":lotes::huella(html.as_bytes()),"startedAt":chrono::Utc::now().to_rfc3339()});
                r["params"] = p.clone();
                Ok(r)
            }
            _ => self.base.llamar(comando, payload, plazo),
        }
    }
}

fn con_lecturas(fallas: usize) -> ConLecturas<'static> {
    ConLecturas {
        base: Ficticio { caso: caso("seguimiento/Espera"), estado: Mutex::new((1, 0, vec![])), cancelar: None, falla: false },
        fallas: Mutex::new(fallas),
        aperturas: Mutex::new(0),
    }
}

/// A1: lecturas fallidas no quitan la consulta de `terminadas`, el lote termina con sus XLS y el retomar reintenta
/// solo las pendientes.
#[test]
fn lecturas_fallidas_no_impiden_terminar_y_se_reintentan_al_retomar() {
    let f = con_lecturas(4);
    let c = Carpeta::nueva();
    let mut lote = plan(&f.base.caso, false);
    lote.lecturas.bitacoras = true;
    let r = lotes::ejecutar(&f, &f.base.caso["catalogo"], &lote, &c.0, false, &AtomicBool::new(false), &|_| {}).unwrap();
    assert_eq!(r.estado, "VALIDADA");
    assert_eq!(r.terminadas, vec![0]);
    assert_eq!(r.archivos.len(), 2);
    assert_eq!(r.lecturas.len(), 2);
    assert_eq!(flow_captura::recoleccion::lecturas_pendientes(&r), 4);
    assert!(r.avisos.iter().any(|a| a.contains("Quedaron 4 lecturas")), "{:?}", r.avisos);
    assert_eq!(*f.aperturas.lock().unwrap(), 6);
    // B4: la versión de la extensión se pide una vez por lote, no por lectura.
    assert_eq!(f.base.estado.lock().unwrap().2.iter().filter(|c| *c == "capabilities").count(), 1);
    let r = lotes::ejecutar(&f, &f.base.caso["catalogo"], &lote, &c.0, true, &AtomicBool::new(false), &|_| {}).unwrap();
    assert_eq!(r.terminadas, vec![0]);
    assert_eq!(r.lecturas.len(), 6);
    assert_eq!(flow_captura::recoleccion::lecturas_pendientes(&r), 0, "{:?}", r.avisos);
    assert!(!r.avisos.iter().any(|a| a.contains("pendiente")), "{:?}", r.avisos);
    assert_eq!(*f.aperturas.lock().unwrap(), 10, "solo se reabren las cuatro pendientes");
    assert_eq!(f.base.estado.lock().unwrap().1, 2, "los XLS verificados no se vuelven a descargar");
}

/// A1: cinco bitácoras fallidas seguidas detienen el lote (como `MAX_FALLAS_SEGUIDAS`), con los XLS conservados.
#[test]
fn cinco_bitacoras_seguidas_fallidas_detienen_el_lote() {
    let f = con_lecturas(usize::MAX);
    let c = Carpeta::nueva();
    let mut lote = plan(&f.base.caso, false);
    lote.lecturas.bitacoras = true;
    let e = lotes::ejecutar(&f, &f.base.caso["catalogo"], &lote, &c.0, false, &AtomicBool::new(false), &|_| {}).unwrap_err();
    assert!(e.contains("5 bitácoras seguidas fallaron"), "{e}");
    assert_eq!(*f.aperturas.lock().unwrap(), 5);
    let r = Resumen::leer(&c.0).unwrap();
    assert_eq!(r.estado, "INCOMPLETA");
    assert_eq!(r.archivos.len(), 2);
}

/// M3: en Seguimiento los vínculos del listado se guardan aunque no se pidan lecturas, sin abrir ventanas.
#[test]
fn vinculos_del_listado_se_guardan_sin_lecturas() {
    let f = con_lecturas(0);
    let c = Carpeta::nueva();
    let lote = plan(&f.base.caso, false);
    assert!(!lote.lecturas.activa());
    let r = lotes::ejecutar(&f, &f.base.caso["catalogo"], &lote, &c.0, false, &AtomicBool::new(false), &|_| {}).unwrap();
    assert_eq!(r.paginas_lectura.len(), 2);
    assert!(r.paginas_lectura.iter().all(|p| p.vinculos.len() == 3 && p.vinculos.iter().all(|v| v.bitacora_verificada)));
    assert_eq!(*f.aperturas.lock().unwrap(), 0);
    assert!(r.lecturas.is_empty());
}

/// M5: «restored:false» y la falta de confirmación son avisos distintos, con un código técnico sin datos personales.
#[test]
fn restauracion_de_filtros_separa_avisos_con_codigo() {
    let no_restaurado = lotes::aviso_restauracion(&Ok(json!({"ok":true,"restored":false}))).unwrap();
    assert!(no_restaurado.starts_with("No se restauraron todos los filtros de SITFA."), "{no_restaurado}");
    assert!(no_restaurado.contains("restored:false"));
    assert_eq!(lotes::aviso_restauracion(&Ok(json!({"ok":true,"restored":true}))), None);
    let tiempo = lotes::aviso_restauracion(&Err(("TIEMPO".into(), "Chrome no respondió a tiempo.".into()))).unwrap();
    assert!(tiempo.starts_with("No se pudo confirmar la restauración de los filtros de SITFA."), "{tiempo}");
    assert!(tiempo.contains("(Código técnico: tiempo.)"));
    let codigo = lotes::aviso_restauracion(&Err(("SITE_ACCESS".into(), "texto".into()))).unwrap();
    assert!(codigo.contains("(Código técnico: SITE_ACCESS.)") && !codigo.contains("texto"), "{codigo}");
}

#[test]
fn archivos_se_rotulan_como_el_descargador() {
    // Dos páginas: base con ordinal, como `lotes.filename` (total > 1).
    let f = Ficticio { caso: caso("seguimiento/Espera"), estado: Mutex::new((1, 0, vec![])), cancelar: None, falla: false };
    let c = Carpeta::nueva();
    let lote = plan(&f.caso, false);
    let r = lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, false, &AtomicBool::new(false), &|_| {}).unwrap();
    let nombres: Vec<_> = r.archivos.iter().map(|a| a.nombre.as_str()).collect();
    assert_eq!(nombres, ["ESP Ambulatorio Jdo Familia Ficticio Norte 1.xls", "ESP Ambulatorio Jdo Familia Ficticio Norte 2.xls"]);
    assert!(nombres.iter().all(|n| c.0.join(n).is_file()));
    // Sin registros: un PDF con la base sola (la base no se repite en el plan).
    let f = Ficticio { caso: caso("seguimiento/Espera/vacia"), estado: Mutex::new((1, 0, vec![])), cancelar: None, falla: false };
    let c = Carpeta::nueva();
    let r = lotes::ejecutar(&f, &f.caso["catalogo"], &plan(&f.caso, true), &c.0, false, &AtomicBool::new(false), &|_| {}).unwrap();
    assert_eq!(r.archivos[0].nombre, "ESP Ambulatorio Jdo Familia Ficticio Norte.pdf");
}

#[test]
fn bases_de_nombre_como_file_base() {
    let catalogo = json!({
        "tribunales": {"901": "Juzgado de Familia Ficticio: Norte.", "902": "Jgdo L y G de Pueblo Ficticio", "903": "JGDO LETRAS DE Villa Ficticia", "904": "Jgdo L de Aldea Ficticia"},
        "estados": {"1": "Pendientes", "2": "Cumplidas"}
    });
    let s = |tribunal: &str, screen: &str, tab: &str, modality: &str| Seleccion {
        tribunal: tribunal.into(),
        screen: screen.into(),
        tab: tab.into(),
        modality: modality.into(),
        report: None,
        state: Some("2".into()),
        month: None,
        year: None,
    };
    let b = |x: Seleccion| lotes::base_archivo(&x, &catalogo);
    assert_eq!(b(s("901", "seguimiento", "Espera", "3")), "ESP FAE Jdo Familia Ficticio Norte");
    assert_eq!(b(s("902", "seguimiento", "Cumplimiento", "1")), "CUMP Residencia Jdo Letras y Garantia Pueblo Ficticio");
    assert_eq!(b(s("903", "seguimiento", "Informes", "4")), "Informes Jdo Letras Villa Ficticia");
    assert_eq!(b(s("904", "seguimiento", "Egresados", "2")), "Egresados Jdo Letras Aldea Ficticia");
    assert_eq!(b(s("901", "litigantes", "", "")), "Litigantes Cumplidas Jdo Familia Ficticio Norte");
    assert_eq!(b(s("901", "calendario_informes", "", "")), "Informes Jdo Familia Ficticio Norte");
    assert_eq!(b(s("901", "calendario_medidas", "", "")), "Medidas por vencer Jdo Familia Ficticio Norte");
    assert_eq!(lotes::nombre_seguro("Á/b\\c*d?e\"f<g>h|i\u{1}.  j. "), "A b c d e f g h i j");
    assert_eq!(lotes::nombre_seguro(&"x".repeat(300)).chars().count(), 125);
}

#[test]
fn retomar_lote_con_nombres_anteriores_conserva_los_del_avance() {
    let cancelar = AtomicBool::new(false);
    let c = Carpeta::nueva();
    let f = Ficticio { caso: caso("seguimiento/Espera"), estado: Mutex::new((1, 0, vec![])), cancelar: Some(&cancelar), falla: false };
    let lote = plan(&f.caso, false);
    assert!(lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, false, &cancelar, &|_| {}).is_err());
    // El avance de un lote anterior tenía los nombres «consulta-NNN-pNNN-de-NNN.xls».
    let mut r = Resumen::leer(&c.0).unwrap();
    let viejo = "consulta-001-p001-de-002.xls";
    std::fs::rename(c.0.join(&r.archivos[0].nombre), c.0.join(viejo)).unwrap();
    r.archivos[0].nombre = viejo.into();
    r.guardar(&c.0).unwrap();
    cancelar.store(false, Ordering::SeqCst);
    let r = lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, true, &cancelar, &|_| {}).unwrap();
    assert_eq!(r.estado, "VALIDADA");
    let nombres: Vec<_> = r.archivos.iter().map(|a| a.nombre.as_str()).collect();
    assert_eq!(nombres, [viejo, "ESP Ambulatorio Jdo Familia Ficticio Norte 2.xls"]);
    // Y se vuelve a retomar sin descargar nada.
    let antes = f.estado.lock().unwrap().1;
    lotes::ejecutar(&f, &f.caso["catalogo"], &lote, &c.0, true, &cancelar, &|_| {}).unwrap();
    assert_eq!(f.estado.lock().unwrap().1, antes);
}

#[test]
fn carpeta_del_lote_con_fecha_como_el_descargador() {
    let c = Carpeta::nueva();
    let a = lotes::carpeta_nueva(&c.0).unwrap();
    let b = lotes::carpeta_nueva(&c.0).unwrap();
    let n = a.file_name().unwrap().to_string_lossy().into_owned();
    assert!(chrono::NaiveDateTime::parse_from_str(&n, "%Y-%m-%d %H-%M-%S").is_ok(), "{n}");
    assert_ne!(a, b);
}
