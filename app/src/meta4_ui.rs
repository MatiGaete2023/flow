//! E-R2 · Vista «Meta 4»: genera la planilla Meta 4 y sus consolidados desde los lotes del Descargador, o la regenera
//! desde un `Registros.xlsx` revisado. Solo lee carpetas locales; cada ejecución crea una carpeta nueva y no modifica
//! los lotes ni los libros de origen. Equivale a `META4_Bitacoras/Meta4.py`.

use crate::theme::{self, Pal};
use chrono::NaiveDate;
use egui::{RichText, Ui};
use flow_rust::descargador::meta4;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub struct Salida {
    pub carpeta: PathBuf,
    pub archivos: Vec<PathBuf>,
    pub avisos: Vec<String>,
    pub registros: usize,
    pub regenerado: bool,
}

enum Mensaje {
    Tribunales(Result<Vec<&'static str>, String>),
    Resultado(Result<Salida, String>),
}

pub struct Meta4Vista {
    pub lotes: Vec<PathBuf>,
    pub seleccion_lote: Option<usize>,
    pub bitacoras: String,
    desde: String,
    pub corte: String,
    corte_editado: bool,
    pub mes: String,
    pub tribunales: Vec<(&'static str, bool)>,
    pub plantilla: Option<PathBuf>,
    pub destino: String,
    pub estado: Option<Result<String, String>>,
    pub avisos: Vec<String>,
    pub ultima: Option<PathBuf>,
    ocupado: bool,
    recepcion: Option<Receiver<Mensaje>>,
}

fn base_documentos() -> PathBuf {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default().join("Documents")
}

impl Default for Meta4Vista {
    fn default() -> Self {
        let hoy = chrono::Local::now().date_naive();
        Self {
            lotes: Vec::new(),
            seleccion_lote: None,
            bitacoras: String::new(),
            desde: String::new(),
            corte: hoy.to_string(),
            corte_editado: false,
            mes: hoy.format("%Y-%m").to_string(),
            tribunales: Vec::new(),
            plantilla: None,
            destino: base_documentos().join("Meta4").display().to_string(),
            estado: None,
            avisos: Vec::new(),
            ultima: None,
            ocupado: false,
            recepcion: None,
        }
    }
}

fn dia(texto: &str, campo: &str) -> Result<NaiveDate, String> {
    let texto = texto.trim();
    NaiveDate::parse_from_str(texto, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(texto, "%d/%m/%Y"))
        .map_err(|_| format!("Escribe una fecha válida en {campo} con formato DD/MM/AAAA o AAAA-MM-DD."))
}

fn periodo_elegido(desde: &str, hasta: &str) -> Result<(NaiveDate, NaiveDate), String> {
    let hasta = dia(hasta, "Hasta")?;
    let desde = if desde.trim().is_empty() {
        flow_meta4::periodo(hasta).0
    } else {
        dia(desde, "Desde")?
    };
    if desde > hasta {
        return Err("Desde debe ser anterior o igual a Hasta.".into());
    }
    Ok((desde, hasta))
}

/// `--month AAAA-MM`: el primer día del mes de emisión de la planilla.
fn emision(mes: &str, corte: NaiveDate) -> Result<NaiveDate, String> {
    let mes = mes.trim();
    if mes.is_empty() {
        return Ok(corte);
    }
    let (a, m) = mes.split_once('-').ok_or("El mes de la planilla debe ser AAAA-MM.")?;
    let (a, m): (i32, u32) = (a.parse().map_err(|_| "El mes de la planilla debe ser AAAA-MM.")?, m.parse().map_err(|_| "El mes de la planilla debe ser AAAA-MM.")?);
    NaiveDate::from_ymd_opt(a, m, 1).ok_or_else(|| "El mes de la planilla debe ser AAAA-MM.".to_string())
}

/// Carpeta nueva y corta (Windows no crea rutas de más de 259 caracteres).
fn carpeta_nueva(base: &Path, regenerado: bool) -> PathBuf {
    let marca = chrono::Local::now().format("%Y-%m-%d %H-%M-%S");
    base.join(format!("Resultado META4 {marca}{}", if regenerado { " regenerado" } else { "" }))
}

fn largo_seguro(destino: &Path) -> Result<(), String> {
    let n = destino.display().to_string().chars().count();
    if n + 70 > 259 {
        return Err(format!("La ruta de salida es demasiado larga ({n} caracteres); elige una carpeta más corta, por ejemplo D:\\Meta4."));
    }
    Ok(())
}

fn generar(
    lotes: Vec<PathBuf>,
    bitacoras: Vec<PathBuf>,
    periodo: (NaiveDate, NaiveDate),
    mes: NaiveDate,
    elegidos: Vec<String>,
    plantilla: Option<PathBuf>,
    base: PathBuf,
) -> Result<Salida, String> {
    let destino = carpeta_nueva(&base, false);
    largo_seguro(&destino)?;
    if periodo.1 > chrono::Local::now().date_naive() {
        return Err("La fecha Hasta no puede estar en el futuro.".into());
    }
    flow_meta4::validar_destino(&destino)?;
    let m = meta4::calcular_en_periodo(&lotes, &bitacoras, periodo, mes, &elegidos)?;
    let archivos = flow_meta4::salida::publicar_con_plantilla(&m, &destino, plantilla.as_deref())?;
    Ok(Salida { carpeta: destino, archivos, avisos: m.avisos.clone(), registros: m.registros.len(), regenerado: false })
}

fn regenerar(registros: PathBuf, plantilla: Option<PathBuf>, base: PathBuf) -> Result<Salida, String> {
    let destino = carpeta_nueva(&base, true);
    largo_seguro(&destino)?;
    let archivos = flow_meta4::regenerar::ejecutar_con_plantilla(&registros, &destino, plantilla.as_deref())?;
    Ok(Salida { carpeta: destino, archivos, avisos: Vec::new(), registros: 0, regenerado: true })
}

impl Meta4Vista {
    fn en_hilo(&mut self, ctx: &egui::Context, trabajo: impl FnOnce() -> Mensaje + Send + 'static) {
        let (envio, recepcion) = mpsc::channel();
        self.recepcion = Some(recepcion);
        self.ocupado = true;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = envio.send(trabajo());
            ctx.request_repaint();
        });
    }

    /// Tribunales de los lotes agregados y, si las bitácoras están dentro, el corte propuesto.
    fn refrescar(&mut self, ctx: &egui::Context) {
        if self.lotes.is_empty() {
            self.tribunales.clear();
            return;
        }
        let lotes = self.lotes.clone();
        self.en_hilo(ctx, move || Mensaje::Tribunales(meta4::tribunales_disponibles(&lotes)));
    }

    fn rutas_bitacoras(&self) -> Vec<PathBuf> {
        if self.bitacoras.trim().is_empty() { Vec::new() } else { vec![PathBuf::from(self.bitacoras.trim())] }
    }

    fn proponer_corte(&mut self) {
        if self.corte_editado {
            return;
        }
        let mut raices = self.rutas_bitacoras();
        if raices.is_empty() {
            raices = self.lotes.clone();
        }
        if let Some(d) = meta4::fecha_lectura(&raices) {
            if d <= chrono::Local::now().date_naive() {
                self.corte = d.to_string();
            }
        }
    }

    fn lanzar_generar(&mut self, ctx: &egui::Context) {
        let periodo = match periodo_elegido(&self.desde, &self.corte) {
            Ok(p) => p,
            Err(e) => return self.estado = Some(Err(e)),
        };
        let mes = match emision(&self.mes, periodo.1) {
            Ok(m) => m,
            Err(e) => return self.estado = Some(Err(e)),
        };
        if self.lotes.is_empty() {
            return self.estado = Some(Err("Agrega al menos un lote de registros.".into()));
        }
        let (lotes, bitacoras) = (self.lotes.clone(), self.rutas_bitacoras());
        let elegidos: Vec<String> = if self.tribunales.iter().all(|(_, s)| *s) {
            Vec::new()
        } else {
            self.tribunales.iter().filter(|(_, s)| *s).map(|(t, _)| t.to_string()).collect()
        };
        if elegidos.is_empty() && !self.tribunales.is_empty() && !self.tribunales.iter().all(|(_, s)| *s) {
            return self.estado = Some(Err("Marca al menos un tribunal.".into()));
        }
        let (plantilla, base) = (self.plantilla.clone(), PathBuf::from(self.destino.trim()));
        self.estado = Some(Ok("Leyendo lotes y bitácoras localmente…".into()));
        self.avisos.clear();
        self.en_hilo(ctx, move || Mensaje::Resultado(generar(lotes, bitacoras, periodo, mes, elegidos, plantilla, base)));
    }

    fn lanzar_regenerar(&mut self, ctx: &egui::Context) {
        let Some(registros) = rfd::FileDialog::new().set_title("Registros.xlsx revisado").add_filter("Excel", &["xlsx"]).pick_file() else { return };
        let (plantilla, base) = (self.plantilla.clone(), PathBuf::from(self.destino.trim()));
        self.estado = Some(Ok("Regenerando desde el libro revisado…".into()));
        self.avisos.clear();
        self.en_hilo(ctx, move || Mensaje::Resultado(regenerar(registros, plantilla, base)));
    }

    pub fn recibir(&mut self) {
        let Some(r) = &self.recepcion else { return };
        let mensaje = match r.try_recv() {
            Ok(m) => m,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.ocupado = false;
                self.recepcion = None;
                self.estado = Some(Err("E_META4: no se pudo completar; los lotes y libros de origen no se modificaron.".into()));
                return;
            }
        };
        self.ocupado = false;
        self.recepcion = None;
        match mensaje {
            Mensaje::Tribunales(Ok(t)) => {
                let previos = std::mem::take(&mut self.tribunales);
                self.tribunales = t.into_iter().map(|t| (t, previos.iter().find(|(p, _)| *p == t).is_none_or(|(_, s)| *s))).collect();
            }
            Mensaje::Tribunales(Err(e)) | Mensaje::Resultado(Err(e)) => self.estado = Some(Err(e)),
            Mensaje::Resultado(Ok(s)) => {
                self.estado = Some(Ok(format!(
                    "{} generado en {} ({} archivos{}).",
                    if s.regenerado { "Meta 4 regenerado" } else { "Meta 4" },
                    s.carpeta.display(),
                    s.archivos.len(),
                    if s.regenerado { String::new() } else { format!(", {} registros", s.registros) }
                )));
                self.avisos = s.avisos;
                self.ultima = Some(s.carpeta);
            }
        }
    }

    pub fn mostrar(&mut self, ui: &mut Ui, p: &Pal) {
        self.recibir();
        let ctx = ui.ctx().clone();
        ui.add_space(8.0);
        ui.label(RichText::new("Meta 4 · registros y bitácoras").size(20.0).strong());
        ui.label(
            RichText::new("Lee los lotes del Descargador y arma la planilla del mes. No se conecta a SITFA ni a RUS; cada ejecución crea una carpeta nueva.")
                .color(p.muted),
        );
        ui.add_space(10.0);
        theme::card(p).show(ui, |ui| {
            theme::seccion(ui, p, "Lotes de registros");
            ui.label(RichText::new("Espera y Cumplimiento de FAE y Residencia, descargados con el Descargador (carpeta con resumen.json).").color(p.muted));
            egui::ScrollArea::vertical().id_salt("lotes de meta 4").max_height(120.0).show(ui, |ui| {
                for (i, l) in self.lotes.iter().enumerate() {
                    let marcado = self.seleccion_lote == Some(i);
                    if ui.selectable_label(marcado, l.display().to_string()).clicked() {
                        self.seleccion_lote = if marcado { None } else { Some(i) };
                    }
                }
                if self.lotes.is_empty() {
                    ui.label(RichText::new("Ningún lote agregado.").color(p.muted));
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(!self.ocupado, egui::Button::new("Agregar…")).clicked() {
                    if let Some(c) = rfd::FileDialog::new().set_title("Carpeta de un lote (o de varios)").pick_folder() {
                        if !self.lotes.contains(&c) {
                            self.lotes.push(c);
                        }
                        self.proponer_corte();
                        self.refrescar(&ctx);
                    }
                }
                if ui.add_enabled(!self.ocupado && self.seleccion_lote.is_some(), egui::Button::new("Quitar")).clicked() {
                    if let Some(i) = self.seleccion_lote.take() {
                        self.lotes.remove(i);
                        self.refrescar(&ctx);
                    }
                }
                if ui.add_enabled(!self.ocupado && !self.lotes.is_empty(), egui::Button::new("Vaciar")).clicked() {
                    self.lotes.clear();
                    self.seleccion_lote = None;
                    self.refrescar(&ctx);
                }
            });
        });
        ui.add_space(8.0);
        theme::card(p).show(ui, |ui| {
            theme::seccion(ui, p, "Bitácoras, período y tribunales");
            ui.horizontal_wrapped(|ui| {
                ui.label("Carpeta de bitácoras");
                ui.add_enabled(!self.ocupado, egui::TextEdit::singleline(&mut self.bitacoras).hint_text("vacío: dentro de los lotes").desired_width(340.0));
                if ui.add_enabled(!self.ocupado, egui::Button::new("Elegir…")).clicked() {
                    if let Some(c) = rfd::FileDialog::new().set_title("Carpeta con bitacoras.json y sus HTML").pick_folder() {
                        self.bitacoras = c.display().to_string();
                        self.proponer_corte();
                    }
                }
            });
            let desde_sugerido = dia(&self.corte, "Hasta").ok().map(|hasta| flow_meta4::periodo(hasta).0.to_string());
            ui.label("Fechas de búsqueda de observaciones (DD/MM/AAAA o AAAA-MM-DD)");
            ui.horizontal_wrapped(|ui| {
                ui.label("Desde");
                ui.add_enabled(!self.ocupado, egui::TextEdit::singleline(&mut self.desde)
                    .hint_text(desde_sugerido.as_deref().unwrap_or("Mes móvil")).desired_width(108.0));
                ui.label("Hasta");
                let hasta = ui.add_enabled(!self.ocupado,
                    egui::TextEdit::singleline(&mut self.corte).desired_width(108.0));
                self.corte_editado |= hasta.changed();
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Mes de la planilla (AAAA-MM)");
                ui.add_enabled(!self.ocupado, egui::TextEdit::singleline(&mut self.mes).desired_width(80.0));
            });
            match periodo_elegido(&self.desde, &self.corte) {
                Ok((a, b)) => { ui.label(RichText::new(format!(
                    "Período de búsqueda: {} al {}, ambos incluidos.",
                    a.format("%d/%m/%Y"), b.format("%d/%m/%Y")
                )).color(p.muted)); }
                Err(e) => { ui.label(RichText::new(e).color(p.amber)); }
            }
            if !self.tribunales.is_empty() {
                ui.label("Tribunales (todos marcados = todos)");
                ui.horizontal_wrapped(|ui| {
                    for (t, s) in &mut self.tribunales {
                        ui.add_enabled(!self.ocupado, egui::Checkbox::new(s, *t));
                    }
                });
            }
        });
        ui.add_space(8.0);
        theme::card(p).show(ui, |ui| {
            theme::seccion(ui, p, "Salida");
            ui.horizontal_wrapped(|ui| {
                ui.label("Carpeta de resultados");
                ui.add_enabled(!self.ocupado, egui::TextEdit::singleline(&mut self.destino).desired_width(340.0));
                if ui.add_enabled(!self.ocupado, egui::Button::new("Elegir…")).clicked() {
                    if let Some(c) = rfd::FileDialog::new().set_title("Carpeta donde crear los resultados").pick_folder() {
                        self.destino = c.display().to_string();
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(match &self.plantilla {
                    Some(p) => format!("Plantilla: {}", p.display()),
                    None => "Diseño estándar".into(),
                });
                if ui.add_enabled(!self.ocupado, egui::Button::new("Elegir plantilla Excel…")).clicked() {
                    if let Some(f) = rfd::FileDialog::new().set_title("Plantilla con las cuatro hojas de Meta 4").add_filter("Excel", &["xlsx"]).pick_file() {
                        self.plantilla = Some(f);
                    }
                }
                if self.plantilla.is_some() && ui.add_enabled(!self.ocupado, egui::Button::new("Usar diseño estándar")).clicked() {
                    self.plantilla = None;
                }
            });
            ui.horizontal_wrapped(|ui| {
                let listo = !self.ocupado && !self.lotes.is_empty();
                if ui
                    .add_enabled(listo, egui::Button::new(RichText::new("Generar Meta 4").color(egui::Color32::WHITE).strong()).fill(p.accent))
                    .clicked()
                {
                    self.lanzar_generar(&ctx);
                }
                if ui.add_enabled(!self.ocupado, egui::Button::new("Regenerar desde Registros revisado…")).clicked() {
                    self.lanzar_regenerar(&ctx);
                }
                if ui.add_enabled(self.ultima.is_some(), egui::Button::new("Abrir resultados")).clicked() {
                    if let Some(u) = &self.ultima {
                        crate::ui::abrir_en_sistema(u);
                    }
                }
            });
        });
        ui.add_space(8.0);
        if self.ocupado {
            ui.horizontal(|ui| {
                ui.spinner();
                if let Some(Ok(t)) = &self.estado {
                    ui.label(t);
                }
            });
        } else {
            match &self.estado {
                Some(Ok(t)) => {
                    ui.label(RichText::new(t).color(p.green));
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(e).color(p.amber));
                }
                None => {}
            }
        }
        if !self.avisos.is_empty() {
            egui::ScrollArea::vertical().id_salt("avisos de meta 4").max_height(160.0).show(ui, |ui| {
                for a in &self.avisos {
                    ui.label(RichText::new(format!("• {a}")).color(p.amber));
                }
            });
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn fechas_y_carpetas_de_la_vista() {
        let corte = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
        assert_eq!(dia("2026-10-16", "Hasta").unwrap(), corte);
        assert_eq!(dia("16/10/2026", "Hasta").unwrap(), corte);
        assert!(periodo_elegido("17/10/2026", "16/10/2026").is_err());
        assert!(periodo_elegido("31/09/2026", "16/10/2026").is_err());
        assert_eq!(periodo_elegido("", "16/10/2026").unwrap(), flow_meta4::periodo(corte));
        assert_eq!(emision("", corte).unwrap(), corte);
        assert_eq!(emision("2026-09", corte).unwrap(), NaiveDate::from_ymd_opt(2026, 9, 1).unwrap());
        assert!(emision("septiembre", corte).is_err());
        let largo = PathBuf::from("C:\\").join("x".repeat(250));
        assert!(largo_seguro(&largo).is_err());
        assert!(largo_seguro(Path::new("C:\\Meta4")).is_ok());
        assert!(carpeta_nueva(Path::new("base"), true).display().to_string().ends_with("regenerado"));
    }

    #[test]
    fn la_vista_genera_y_regenera_desde_un_lote_del_descargador() {
        use base64::Engine;
        let datos: serde_json::Value = serde_json::from_str(include_str!("../tests/datos/paquete.json")).unwrap();
        let e = datos["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "contexto").unwrap();
        let base = std::env::temp_dir().join(format!("flow-meta4-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (lote, bitacoras) = (base.join("lote"), base.join("bitacoras"));
        for (destino, archivos) in [(&lote, &e["lote"]["archivos"]), (&bitacoras, &e["bitacoras"]["archivos"])] {
            for (ruta, b64) in archivos.as_object().unwrap() {
                let r = destino.join(ruta);
                std::fs::create_dir_all(r.parent().unwrap()).unwrap();
                std::fs::write(r, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
            }
        }
        for r in [lote.join("resumen.json"), lote.join("verificacion 001.json"), bitacoras.join("bitacoras.json")] {
            let t = std::fs::read_to_string(&r).unwrap();
            let t = t.replace("\"modality\": \"2\"", "\"modality\": \"1\"").replace("\"modalidad\": \"2\"", "\"modalidad\": \"1\"");
            let t = t.replace("\"modalities\": [\"2\"]", "\"modalities\": [\"1\"]").replace(": \"Ambulatorio\"", ": \"Residencia\"").replace("/ Ambulatorio\"", "/ Residencia\"");
            std::fs::write(r, t).unwrap();
        }
        let corte = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let (desde, _) = periodo_elegido("12/08/2026", "08/10/2026").unwrap();
        let dentro = meta4::calcular_en_periodo(
            std::slice::from_ref(&lote), std::slice::from_ref(&bitacoras), (desde, corte), corte, &[]
        ).unwrap();
        assert_eq!(dentro.observaciones.len(), 2);
        assert!(dentro.observaciones.iter().all(|o| o.fecha.date() == desde));
        let fuera = meta4::calcular_en_periodo(
            std::slice::from_ref(&lote), std::slice::from_ref(&bitacoras), (desde.succ_opt().unwrap(), corte), corte, &[]
        ).unwrap();
        assert!(fuera.observaciones.is_empty());
        let salida = generar(vec![lote], vec![bitacoras], (desde, corte), corte, Vec::new(), None, base.join("resultados")).unwrap();
        assert_eq!(salida.registros, 2);
        assert!(salida.archivos.len() >= 4, "{:?}", salida.archivos);
        let registros = salida.carpeta.join("Registros.xlsx");
        assert!(registros.is_file());
        let nueva = regenerar(registros, None, base.join("resultados")).unwrap();
        assert!(nueva.regenerado && nueva.carpeta != salida.carpeta && nueva.carpeta.display().to_string().ends_with("regenerado"));
        for carpeta in [&salida.carpeta, &nueva.carpeta] {
            let json = std::fs::read(carpeta.join("Reporte META4.json")).unwrap();
            let reporte: serde_json::Value = serde_json::from_slice(&json).unwrap();
            assert_eq!(reporte["periodo"], serde_json::json!({"desde": "2026-08-12", "hasta": "2026-10-08"}));
        }
        // Una carpeta de salida se crea una vez: repetir en el mismo segundo no pisa la anterior.
        let _ = std::fs::remove_dir_all(&base);
    }
}
