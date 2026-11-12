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
    let correcto =
        json!({"tipo":"FLOW_CAPACIDADES","version":"0.2.0","protocolos":{"descarga_xls":1,"pdf_sin_registros":1,"bitacoras_lectura":1,"historia_lectura":1,"calendario_lectura":1},"escritura_rus":false});
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
