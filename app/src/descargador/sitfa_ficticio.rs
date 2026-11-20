//! SITFA ficticio (`descargador_lite/tests/sitfa_ficticio.py`): personas, RIT y tribunales inventados, sin red.
//! Las pruebas reproducen los casos de `descargador_lite/tests/test_lite.py` sobre el ejecutor de Flow Rust.

use super::lote::{ejecutar, Conexion, Evento, Opciones};
use super::{Catalogo, Edicion, Pantalla, SeleccionLote, TODAS_LAS_MEDIDAS};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const PESTANAS: [(&str, &str, &str, &str, &str); 4] = super::sitfa::PESTANAS_SITFA;

fn catalogo_json(pantalla: &str) -> Value {
    let tribunales = json!({"901": "Juzgado de Familia Ficticio Norte", "902": "Juzgado de Familia Ficticio Sur"});
    match pantalla {
        "seguimiento" => json!({
            "pantalla": "seguimiento", "tribunales": tribunales,
            "modalidades": {"1": "Residencia", "2": "Ambulatorio", "3": "FAE", "4": "DCE"},
            "informes": {"1": "Por vencer (a 25 días)", "2": "Vencidos", "3": "Recibidos"},
            "medidas": {"4": "Por vencer (a 45 días)", "5": "Vencidos"},
            "pestanas": ["Espera", "Cumplimiento", "Informes", "Egresados"],
        }),
        "litigantes" => json!({
            "pantalla": "litigantes", "tribunales": tribunales,
            "estados": {"1": "Pendientes", "2": "Cumplidas"},
        }),
        _ => {
            let meses: BTreeMap<String, &str> = [
                "Enero", "Febrero", "Marzo", "Abril", "Mayo", "Junio", "Julio", "Agosto", "Septiembre",
                "Octubre", "Noviembre", "Diciembre",
            ]
            .iter()
            .enumerate()
            .map(|(i, n)| ((i + 1).to_string(), *n))
            .collect();
            json!({
                "pantalla": pantalla, "tribunales": tribunales, "meses": meses,
                "anios": {"2026": "2026", "2027": "2027"},
            })
        }
    }
}

pub fn catalogo(pantalla: &str) -> Catalogo {
    serde_json::from_value(catalogo_json(pantalla)).unwrap()
}

fn escapar(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#x27;")
}

/// Responde como la extensión Lite. `paginas` = {clave de consulta: páginas (0 = sin registros)}.
pub struct SitfaFicticio {
    pantalla: &'static str,
    paginas: HashMap<String, usize>,
    pub llamadas: RefCell<Vec<String>>,
    actual: RefCell<BTreeMap<String, String>>,
}

impl SitfaFicticio {
    pub fn nuevo(pantalla: &'static str, paginas: &[(&str, usize)]) -> Self {
        Self {
            pantalla,
            paginas: paginas.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            llamadas: RefCell::new(Vec::new()),
            actual: RefCell::new(BTreeMap::new()),
        }
    }

    fn clave(&self, p: &BTreeMap<String, String>) -> String {
        let g = |k: &str| p.get(k).cloned().unwrap_or_default();
        match self.pantalla {
            "seguimiento" => {
                let tab = PESTANAS.iter().find(|t| t.1 == g("COD_Lengueta")).unwrap().0;
                let informe = p.get("TIP_Informe").cloned().unwrap_or_else(|| "None".into());
                format!("{}|{tab}|{}|{informe}", g("COD_Tribunal_sel"), g("TIP_Consulta"))
            }
            "litigantes" => format!("{}|{}", g("COD_Tribunal_sel"), g("COD_EstBusqueda")),
            _ => format!("{}|{}", g("COD_Tribunal_sel"), g("COD_Mes_sel")),
        }
    }

    fn total(&self, p: &BTreeMap<String, String>) -> usize {
        self.paginas.get(&self.clave(p)).copied().unwrap_or(1)
    }

    fn calendario(&self) -> bool {
        self.pantalla.starts_with("calendario_")
    }

    fn filas(&self, p: &BTreeMap<String, String>, pagina: usize) -> Vec<(String, String)> {
        let clave = self.clave(p);
        let (letra, n) = if self.calendario() { ("C", 2) } else { ("X", 3) };
        (0..n)
            .map(|i| (format!("{letra}-{pagina}{i}-2026"), format!("Persona ficticia {clave} {pagina}-{i}")))
            .collect()
    }

    fn pagina(&self, p: &BTreeMap<String, String>) -> usize {
        let campo = match self.pantalla {
            "seguimiento" => PESTANAS.iter().find(|t| Some(&t.1.to_string()) == p.get("COD_Lengueta")).unwrap().2,
            "litigantes" => "NUM_PaginaInforme",
            _ => return 1,
        };
        p.get(campo).and_then(|v| v.parse().ok()).unwrap_or(1)
    }

    fn preparar(&self, s: &Value, fechas: &Value) -> Vec<(String, String)> {
        let t = |k: &str| s[k].as_str().unwrap_or("").to_string();
        let mut pares = vec![("COD_Tribunal_sel".to_string(), t("tribunal"))];
        let mut poner = |k: &str, v: String| pares.push((k.to_string(), v));
        match self.pantalla {
            "seguimiento" => {
                let tab = PESTANAS.iter().find(|x| x.0 == t("tab")).unwrap();
                poner("COD_Lengueta", tab.1.into());
                poner("TIP_Consulta", t("modality"));
                if matches!(tab.0, "Informes" | "Cumplimiento") {
                    poner("TIP_Informe", s["report"].as_str().unwrap_or(TODAS_LAS_MEDIDAS).into());
                }
                poner(tab.2, String::new());
                poner(tab.3, String::new());
            }
            "litigantes" => {
                poner("TIP_Consulta", "2".into());
                poner("COD_EstBusqueda", t("state"));
                poner("NUM_PaginaInforme", String::new());
                poner("NUM_Total", String::new());
            }
            _ => {
                poner("TIP_Consulta", "1".into());
                poner("COD_Mes_sel", t("month"));
                poner("COD_Anio_Sel", t("year"));
            }
        }
        if !self.calendario() {
            poner("FLG_Consulta", if fechas.is_null() { "0" } else { "1" }.into());
            if !fechas.is_null() {
                poner("CHK_Consulta", "on".into());
                poner("FEC_Inicio", fechas["start"].as_str().unwrap().into());
                poner("FEC_Fin", fechas["end"].as_str().unwrap().into());
            }
        }
        let accion = match self.pantalla {
            "seguimiento" => "Buscar medida",
            "litigantes" => "Consulta Informe",
            "calendario_informes" => "Buscar Inf.",
            _ => "Buscar",
        };
        poner("irAccion", accion.into());
        pares
    }

    fn html(&self, p: &BTreeMap<String, String>) -> String {
        let total = self.total(p);
        let pagina = self.pagina(p);
        let filas = if total > 0 { self.filas(p, pagina) } else { vec![] };
        let mut campos: String = p
            .iter()
            .filter(|(k, _)| {
                ["COD_Tribunal_sel", "TIP_Consulta", "TIP_Informe", "COD_EstBusqueda", "COD_Mes_sel", "COD_Anio_Sel"]
                    .contains(&k.as_str())
            })
            .map(|(k, v)| format!(r#"<select name="{k}"><option value="{0}" selected>{0}</option></select>"#, escapar(v)))
            .collect();
        let fecha = |k: &str| escapar(p.get(k).map(String::as_str).unwrap_or(""));
        campos += &format!(r#"<input name="FEC_Inicio" value="{}"><input name="FEC_Fin" value="{}">"#, fecha("FEC_Inicio"), fecha("FEC_Fin"));
        let plantilla = match self.pantalla {
            "litigantes" => "BusquedaLit",
            "calendario_informes" => "InfoPorVencer",
            "calendario_medidas" => "MedPorVencer",
            _ => "listdemo",
        };
        let tablas = match self.pantalla {
            "seguimiento" => {
                let tab = PESTANAS.iter().find(|t| Some(&t.1.to_string()) == p.get("COD_Lengueta")).unwrap();
                campos += r#"<input name="COD_Lengueta" value="">"#;
                campos += &format!(
                    r#"<input name="{}" value="{}"><input name="{}" value="{}">"#,
                    tab.2,
                    if total > 0 { pagina } else { 1 },
                    tab.3,
                    total.max(1)
                );
                let mut cuerpo: String = filas.iter().map(|(r, n)| format!("<tr><td>{r}</td><td>{n}</td></tr>")).collect();
                if cuerpo.is_empty() {
                    cuerpo = "<tr><td>Sin registros</td></tr>".into();
                }
                PESTANAS
                    .iter()
                    .map(|v| {
                        format!(
                            r#"<table id="{}"><tr><td>RIT</td><td>Nombre</td></tr>{}</table>"#,
                            v.4,
                            if v.1 == tab.1 { cuerpo.as_str() } else { "" }
                        )
                    })
                    .collect::<String>()
            }
            "litigantes" => {
                campos += &format!(r#"<input name="NUM_PaginaInforme" value="{pagina}"><input name="NUM_Total" value="{}">"#, total.max(1));
                let cuerpo: String = filas.iter().map(|(r, n)| format!("<tr><td>{r}</td><td>{n}</td></tr>")).collect();
                format!("<table><tr><td>RIT</td><td>Nombre</td></tr>{cuerpo}</table>")
            }
            _ => {
                let cuerpo: String = filas.iter().map(|(r, _)| format!("<tr><td>{r}</td></tr>")).collect();
                format!(r#"<table id="TablaCalendarioSemana">{cuerpo}</table><table id="TablaCalendarioDia">{cuerpo}</table>"#)
            }
        };
        let mut scripts =
            format!(r#"function ShowExcel(){{window.open("https://familia.pjud.cl/sitfa/reportes/u1_{plantilla}.xls");}}"#);
        if self.calendario() {
            for (r, n) in &filas {
                scripts += &format!(r#"Ritxdias[dia] = "{r}"; Nombresxdias[dia] = "{n}";"#);
            }
        }
        format!(
            r#"<html><body><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">{campos}{tablas}</form><script>{scripts}</script></body></html>"#
        )
    }

    fn excel(&self, p: &BTreeMap<String, String>) -> Vec<u8> {
        let filas = self.filas(p, self.pagina(p));
        if self.calendario() {
            let mes: u32 = p["COD_Mes_sel"].parse().unwrap();
            let cuerpo: String = filas
                .iter()
                .map(|(r, n)| format!("<tr><td>{r}</td><td>{n}</td><td>15/{mes:02}/2026</td></tr>"))
                .collect();
            return format!("<html><table><tr><td>RIT</td><td>Nombre</td><td>Fecha Vencimiento</td></tr>{cuerpo}</table></html>").into_bytes();
        }
        let cuerpo: String = filas.iter().map(|(r, n)| format!("<tr><td>{r}</td><td>{n}</td></tr>")).collect();
        format!("<html><table><tr><td>RIT</td><td>Nombre</td></tr>{cuerpo}</table></html>").into_bytes()
    }
}

fn png() -> String {
    let mut datos = Vec::new();
    image::RgbImage::from_pixel(800, 400, image::Rgb([255, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut datos), image::ImageFormat::Png)
        .unwrap();
    STANDARD.encode(datos)
}

impl Conexion for SitfaFicticio {
    fn llamar(&self, comando: &str, payload: Value, _plazo: Duration) -> Result<Value, String> {
        self.llamadas.borrow_mut().push(comando.into());
        let pares = |v: &Value| -> BTreeMap<String, String> {
            v.as_array()
                .unwrap()
                .iter()
                .map(|p| (p[0].as_str().unwrap().into(), p[1].as_str().unwrap().into()))
                .collect()
        };
        Ok(match comando {
            "lock" | "unlock" | "evidence_close" => json!({"ok": true, "restored": true}),
            "prepare" => json!({"pairs": self.preparar(&payload["selection"], &payload["dates"])}),
            "search" => {
                let p = pares(&payload["pairs"]);
                let cuerpo = self.html(&p);
                *self.actual.borrow_mut() = p;
                json!({"status": 200, "type": "text/html; charset=utf-8", "body": STANDARD.encode(cuerpo)})
            }
            "download" => {
                json!({"status": 200, "type": "application/vnd.ms-excel", "body": STANDARD.encode(self.excel(&self.actual.borrow()))})
            }
            "evidence_open" => {
                let p = pares(&payload["pairs"]);
                let lengueta = p.get("COD_Lengueta").cloned().unwrap_or_default();
                let html = self.html(&p).replace(
                    r#"name="COD_Lengueta" value="""#,
                    &format!(r#"name="COD_Lengueta" value="{lengueta}""#),
                );
                let contexto: Vec<[String; 2]> = serde_json::from_value(payload["context"].clone()).unwrap();
                let texto = format!(
                    "Consulta solicitada · {}",
                    contexto.iter().map(|[a, b]| format!("{a}: {b}")).collect::<Vec<_>>().join(" · ")
                );
                let banner = format!(
                    r#"<div id="sitfaContextoConsulta" data-context="{}">{}</div>"#,
                    escapar(&serde_json::to_string(&contexto).unwrap()),
                    escapar(&texto)
                );
                let cuerpo = html.replace("<body>", &format!("<body>{banner}"));
                let digest = format!("{:x}", Sha256::digest(cuerpo.as_bytes()));
                json!({"status": 200, "type": "text/html; charset=utf-8", "body": STANDARD.encode(&cuerpo), "digest": digest})
            }
            "evidence_capture" => json!({"png": png(), "digest": payload["digest"]}),
            otro => return Err(format!("comando no permitido en Lite: {otro}")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Carpeta(PathBuf);

    impl Carpeta {
        fn nueva() -> Self {
            let mut ruta = std::env::temp_dir();
            let mut sal = [0u8; 8];
            getrandom::fill(&mut sal).unwrap();
            ruta.push(format!("flow-lote-{}", sal.iter().map(|b| format!("{b:02x}")).collect::<String>()));
            std::fs::create_dir_all(&ruta).unwrap();
            Self(ruta)
        }
    }

    impl Drop for Carpeta {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn nombres(lote: &Path, extension: &str) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(lote)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(extension))
            .collect();
        v.sort();
        v
    }

    fn correr(
        sitfa: &SitfaFicticio,
        pantalla: &str,
        lote: &SeleccionLote,
        opciones: &Opciones,
        destino: &Path,
    ) -> (Vec<Value>, PathBuf, Vec<Evento>) {
        let catalogo = catalogo(pantalla);
        let plan = catalogo.planificar(Edicion::Lite, lote).unwrap();
        let mut eventos = Vec::new();
        let resultado = ejecutar(
            sitfa,
            &catalogo,
            Edicion::Lite,
            &plan,
            destino,
            opciones,
            &AtomicBool::new(false),
            &mut |e| eventos.push(e),
        )
        .unwrap();
        let carpeta = std::fs::read_dir(destino).unwrap().next().unwrap().unwrap().path();
        (resultado, carpeta, eventos)
    }

    fn fechas(a: &str, b: &str) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
        let f = |s: &str| chrono::NaiveDate::parse_from_str(s, "%d/%m/%Y").unwrap();
        Some((f(a), f(b)))
    }

    fn json(lote: &Path, nombre: &str) -> Value {
        serde_json::from_slice(&std::fs::read(lote.join(nombre)).unwrap()).unwrap()
    }

    #[test]
    fn seguimiento_pestanas_modalidades_informes_medidas_y_fechas() {
        let destino = Carpeta::nueva();
        let sitfa = SitfaFicticio::nuevo("seguimiento", &[("901|Cumplimiento|2|5", 3)]);
        let lote = SeleccionLote {
            tribunales: vec!["901".into()],
            modalidades: vec!["2".into()],
            pestanas: vec!["Cumplimiento".into(), "Informes".into()],
            informes: vec!["1".into(), "2".into()],
            medidas: vec!["0".into(), "5".into()],
            fechas: fechas("01/10/2026", "30/10/2026"),
            ..Default::default()
        };
        let (resultado, carpeta, _) = correr(&sitfa, "seguimiento", &lote, &Opciones::default(), &destino.0);
        assert_eq!(resultado.len(), 4);
        let mut esperado = vec![
            "Seguimiento - Cumplimiento - Ambulatorio - Todas las medidas - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026.xls",
            "Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026 - p1 de 3.xls",
            "Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026 - p2 de 3.xls",
            "Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026 - p3 de 3.xls",
            "Seguimiento - Informes - Ambulatorio - Por vencer (a 25 dias) - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026.xls",
            "Seguimiento - Informes - Ambulatorio - Vencidos - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026.xls",
        ];
        esperado.sort();
        assert_eq!(nombres(&carpeta, ".xls"), esperado);
        let resumen = json(&carpeta, "resumen.json");
        assert_eq!(resumen["estado"], "VALIDADA");
        assert_eq!(resumen["consultas_completadas"], 4);
        assert_eq!(resumen["version"], "lite-1");
        assert_eq!(resumen["filtro_fecha"], json!({"start": "01/10/2026", "end": "30/10/2026"}));
        let manifiesto = json(&carpeta, "verificacion 002.json");
        assert_eq!(manifiesto["estado"], "VALIDADA");
        assert_eq!(manifiesto["paginas_verificadas"], 3);
        assert_eq!(manifiesto["caso"], "T901-Cumplimiento-M2-I5");
        assert_eq!(manifiesto["archivos"][0]["formato"], "HTML-tabular-con-extension-xls");
        assert_eq!(manifiesto["seleccion"]["report"], "5");
        let llamadas = sitfa.llamadas.borrow();
        assert_eq!(llamadas.first().map(String::as_str), Some("lock"));
        assert_eq!(llamadas.last().map(String::as_str), Some("unlock"));
        assert!(!llamadas.iter().any(|c| c.contains("diary") || c.contains("history")));
    }

    #[test]
    fn sin_registros_guarda_pantallazo_pdf() {
        let destino = Carpeta::nueva();
        let sitfa = SitfaFicticio::nuevo("seguimiento", &[("902|Espera|1|None", 0)]);
        let lote = SeleccionLote {
            tribunales: vec!["901".into(), "902".into()],
            modalidades: vec!["1".into()],
            pestanas: vec!["Espera".into()],
            ..Default::default()
        };
        let (resultado, carpeta, _) = correr(&sitfa, "seguimiento", &lote, &Opciones::default(), &destino.0);
        let estados: Vec<_> = resultado.iter().map(|r| r["estado"].as_str().unwrap()).collect();
        assert_eq!(estados, ["VALIDADA", "SIN_RESULTADOS"]);
        let pdfs = nombres(&carpeta, ".pdf");
        assert_eq!(pdfs, ["Seguimiento - Espera - Residencia - Jdo Familia Ficticio Sur - SIN REGISTROS.pdf"]);
        assert!(std::fs::read(carpeta.join(&pdfs[0])).unwrap().starts_with(b"%PDF-"));
        assert_eq!(nombres(&carpeta, ".xls"), ["Seguimiento - Espera - Residencia - Jdo Familia Ficticio Norte.xls"]);
        let manifiesto = json(&carpeta, "verificacion 002.json");
        assert_eq!(manifiesto["estado"], "SIN_RESULTADOS");
        assert_eq!(manifiesto["evidencia"]["archivo"], pdfs[0].as_str());
        assert!(sitfa.llamadas.borrow().iter().any(|c| c == "evidence_capture"));
    }

    #[test]
    fn sin_registros_sin_pdf_si_se_desmarca() {
        let destino = Carpeta::nueva();
        let sitfa = SitfaFicticio::nuevo("seguimiento", &[("902|Espera|1|None", 0)]);
        let lote = SeleccionLote {
            tribunales: vec!["901".into(), "902".into()],
            modalidades: vec!["1".into()],
            pestanas: vec!["Espera".into()],
            ..Default::default()
        };
        let opciones = Opciones { evidencia: false, ..Default::default() };
        let (resultado, carpeta, eventos) = correr(&sitfa, "seguimiento", &lote, &opciones, &destino.0);
        assert_eq!(resultado[1]["estado"], "SIN_RESULTADOS");
        assert_eq!(resultado[1]["evidencia_omitida"], true);
        assert!(nombres(&carpeta, ".pdf").is_empty());
        assert!(!sitfa.llamadas.borrow().iter().any(|c| c == "evidence_open"));
        let manifiesto = json(&carpeta, "verificacion 002.json");
        assert_eq!(manifiesto["evidencia_omitida"], true);
        assert!(manifiesto.get("evidencia").is_none());
        let consulta = resultado[1]["consulta"].as_str().unwrap().to_string();
        assert!(eventos.contains(&Evento::Vacia { consulta }));
        match eventos.last() {
            Some(Evento::Fin(f)) => assert_eq!((f.vacias, f.pdfs, f.archivos, f.registros), (1, 0, 1, 3)),
            otro => panic!("fin esperado: {otro:?}"),
        }
    }

    #[test]
    fn busqueda_litigantes_estados_y_fechas() {
        let destino = Carpeta::nueva();
        let sitfa = SitfaFicticio::nuevo("litigantes", &[("901|2", 2)]);
        let lote = SeleccionLote {
            tribunales: vec!["901".into()],
            estados: vec!["1".into(), "2".into()],
            fechas: fechas("01/09/2026", "30/09/2026"),
            ..Default::default()
        };
        let (_, carpeta, _) = correr(&sitfa, "litigantes", &lote, &Opciones::default(), &destino.0);
        assert_eq!(
            nombres(&carpeta, ".xls"),
            [
                "Busqueda Lit - Cumplidas - Jdo Familia Ficticio Norte - 01-09-2026 al 30-09-2026 - p1 de 2.xls",
                "Busqueda Lit - Cumplidas - Jdo Familia Ficticio Norte - 01-09-2026 al 30-09-2026 - p2 de 2.xls",
                "Busqueda Lit - Pendientes - Jdo Familia Ficticio Norte - 01-09-2026 al 30-09-2026.xls",
            ]
        );
    }

    #[test]
    fn informes_y_medidas_por_vencer_mes_ano() {
        for (pantalla, titulo) in [("calendario_informes", "Informes"), ("calendario_medidas", "Medidas")] {
            let destino = Carpeta::nueva();
            let sitfa = SitfaFicticio::nuevo(if pantalla == "calendario_informes" { "calendario_informes" } else { "calendario_medidas" }, &[("902|11", 0)]);
            let lote = SeleccionLote {
                tribunales: vec!["901".into(), "902".into()],
                meses: vec!["10".into(), "11".into()],
                anio: "2026".into(),
                ..Default::default()
            };
            let (resultado, carpeta, _) = correr(&sitfa, pantalla, &lote, &Opciones::default(), &destino.0);
            assert_eq!(resultado.len(), 4);
            assert_eq!(
                nombres(&carpeta, ".xls"),
                [
                    format!("{titulo} por vencer - Noviembre 2026 - Jdo Familia Ficticio Norte.xls"),
                    format!("{titulo} por vencer - Octubre 2026 - Jdo Familia Ficticio Norte.xls"),
                    format!("{titulo} por vencer - Octubre 2026 - Jdo Familia Ficticio Sur.xls"),
                ]
            );
            assert_eq!(
                nombres(&carpeta, ".pdf"),
                [format!("{titulo} por vencer - Noviembre 2026 - Jdo Familia Ficticio Sur - SIN REGISTROS.pdf")]
            );
        }
    }

    #[test]
    fn nombres_validos_en_windows_y_acotados() {
        let mut modelo = catalogo("seguimiento");
        modelo
            .tribunales
            .insert("901".into(), format!("Juzgado de Familia {}", "Muy Largo / Con: Signos? ".repeat(10)));
        let perfil = modelo
            .preparar(
                Edicion::Lite,
                &super::super::Seleccion {
                    tribunal: "901".into(),
                    pestana: "Informes".into(),
                    modalidad: "2".into(),
                    reporte: "1".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        let base = super::super::lote::base_archivo(&perfil, &modelo, fechas("01/10/2026", "30/10/2026"), 150);
        assert!(base.chars().count() <= 150);
        assert!(!base.contains(['<', '>', ':', '"', '/', '\\', '|', '?', '*']));
        assert!(base.starts_with("Seguimiento - Informes - Ambulatorio - Por vencer"));
        assert!(base.ends_with("01-10-2026 al 30-10-2026"));
        let profunda = std::env::temp_dir().join("x".repeat(180));
        assert_eq!(super::super::lote::limite_nombre(&profunda), 60);
        assert!(super::super::lote::limite_nombre(Path::new("C:/SITFA")) <= 150);
    }

    #[test]
    fn cancelar_conserva_lo_verificado_y_desbloquea() {
        let destino = Carpeta::nueva();
        let sitfa = SitfaFicticio::nuevo("seguimiento", &[]);
        let modelo = catalogo("seguimiento");
        let plan = modelo
            .planificar(
                Edicion::Lite,
                &SeleccionLote {
                    tribunales: vec!["901".into(), "902".into()],
                    modalidades: vec!["1".into()],
                    pestanas: vec!["Espera".into()],
                    ..Default::default()
                },
            )
            .unwrap();
        let cancelar = AtomicBool::new(false);
        let error = ejecutar(&sitfa, &modelo, Edicion::Lite, &plan, &destino.0, &Opciones::default(), &cancelar, &mut |e| {
            if matches!(e, Evento::ConsultaLista { .. }) {
                cancelar.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        })
        .unwrap_err();
        assert!(error.contains("cancelado"));
        let carpeta = std::fs::read_dir(&destino.0).unwrap().next().unwrap().unwrap().path();
        let resumen = json(&carpeta, "resumen.json");
        assert_eq!(resumen["estado"], "INCOMPLETA");
        assert_eq!(resumen["consultas_completadas"], 1);
        assert_eq!(nombres(&carpeta, ".xls").len(), 1);
        assert_eq!(sitfa.llamadas.borrow().last().map(String::as_str), Some("unlock"));
        assert!(ejecutar(&sitfa, &modelo, Edicion::Integral, &plan, &destino.0, &Opciones::default(), &cancelar, &mut |_| {})
            .is_err());
    }

    #[test]
    fn rechaza_un_xls_que_no_coincide_con_la_pagina() {
        struct Alterado(SitfaFicticio);
        impl Conexion for Alterado {
            fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
                let mut v = self.0.llamar(comando, payload, plazo)?;
                if comando == "download" {
                    let otro = b"<html><table><tr><td>RIT</td><td>Nombre</td></tr><tr><td>Z-1-2026</td><td>Otra Persona Ficticia</td></tr></table></html>";
                    v["body"] = json!(STANDARD.encode(otro));
                }
                Ok(v)
            }
        }
        let destino = Carpeta::nueva();
        let sitfa = Alterado(SitfaFicticio::nuevo("seguimiento", &[]));
        let modelo = catalogo("seguimiento");
        let plan = modelo
            .planificar(
                Edicion::Lite,
                &SeleccionLote {
                    tribunales: vec!["901".into()],
                    modalidades: vec!["1".into()],
                    pestanas: vec!["Espera".into()],
                    ..Default::default()
                },
            )
            .unwrap();
        let error = ejecutar(&sitfa, &modelo, Edicion::Lite, &plan, &destino.0, &Opciones::default(), &AtomicBool::new(false), &mut |_| {})
            .unwrap_err();
        assert!(error.contains("no coinciden con la pagina HTML"));
        let carpeta = std::fs::read_dir(&destino.0).unwrap().next().unwrap().unwrap().path();
        assert!(nombres(&carpeta, ".xls").is_empty());
        assert_eq!(json(&carpeta, "verificacion 001.json")["estado"], "INCOMPLETA");
        assert_eq!(json(&carpeta, "resumen.json")["estado"], "INCOMPLETA");
        let _ = Pantalla::Seguimiento;
    }
}
