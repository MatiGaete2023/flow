//! Captura en un hilo de trabajo: la ventana sigue respondiendo mientras Chrome consulta SITFA.

use eframe::egui;
use flow_captura::{
    lotes::{self, Fechas, Lote, Resumen},
    sitfa::Seleccion,
    Puente,
};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver},
        Arc,
    },
    time::Duration,
};

enum Evento {
    Opciones(Result<Value, String>),
    Avance(String),
    Fin(PathBuf, Box<Result<Resumen, String>>),
}

pub struct Captura {
    puente: Option<Arc<Puente>>,
    pantalla: String,
    catalogo: Option<Value>,
    tribunales: Vec<(String, String, bool)>,
    modalidades: Vec<(String, String, bool)>,
    pestañas: Vec<(String, bool)>,
    informes: Vec<(String, String, bool)>,
    medidas: Vec<(String, String, bool)>,
    estados: Vec<(String, String, bool)>,
    meses: Vec<(String, String, bool)>,
    año: String,
    fechas: bool,
    desde: String,
    hasta: String,
    filtros: bool,
    evidencia: bool,
    lecturas: flow_captura::recoleccion::Opciones,
    receptor: Option<Receiver<Evento>>,
    cancelar: Arc<AtomicBool>,
    mensaje: String,
    carpeta: Option<PathBuf>,
    listo: Option<(PathBuf, Resumen)>,
}

impl Default for Captura {
    fn default() -> Self {
        Self {
            puente: None,
            pantalla: "seguimiento".into(),
            catalogo: None,
            tribunales: vec![],
            modalidades: vec![],
            pestañas: vec![],
            informes: vec![],
            medidas: vec![],
            estados: vec![],
            meses: vec![],
            año: chrono::Local::now().format("%Y").to_string(),
            fechas: false,
            desde: chrono::Local::now().format("01/%m/%Y").to_string(),
            hasta: chrono::Local::now().format("%d/%m/%Y").to_string(),
            filtros: false,
            evidencia: true,
            lecturas: Default::default(),
            receptor: None,
            cancelar: Arc::new(AtomicBool::new(false)),
            mensaje: "Abre SITFA en Chrome y conecta la extensión de Flow.".into(),
            carpeta: None,
            listo: None,
        }
    }
}

fn opciones(catalogo: &Value, clave: &str) -> Vec<(String, String, bool)> {
    catalogo[clave]
        .as_object()
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or(k).to_string(), false)).collect())
        .unwrap_or_default()
}

fn marcas(ui: &mut egui::Ui, titulo: &str, opciones: &mut [(String, String, bool)]) {
    ui.label(egui::RichText::new(titulo).strong());
    ui.horizontal_wrapped(|ui| {
        for (_, nombre, marca) in opciones {
            ui.checkbox(marca, nombre.as_str());
        }
    });
}

impl Captura {
    pub fn ocupada(&self) -> bool {
        self.receptor.is_some()
    }

    fn recibir(&mut self) {
        let mut eventos = vec![];
        if let Some(r) = &self.receptor {
            eventos.extend(r.try_iter());
        }
        for e in eventos {
            match e {
                Evento::Avance(m) => self.mensaje = m,
                Evento::Opciones(r) => {
                    self.receptor = None;
                    match r {
                        Ok(c) => {
                            self.tribunales = opciones(&c, "tribunales");
                            self.modalidades = opciones(&c, "modalidades");
                            self.informes = opciones(&c, "informes");
                            self.medidas = vec![("0".into(), "Todas las medidas".into(), true)];
                            self.medidas.extend(opciones(&c, "medidas"));
                            self.estados = opciones(&c, "estados");
                            self.meses = opciones(&c, "meses");
                            self.pestañas = c["pestanas"]
                                .as_array()
                                .map(|ps| ps.iter().filter_map(|p| p.as_str().map(|s| (s.to_string(), false))).collect())
                                .unwrap_or_default();
                            self.catalogo = Some(c);
                            self.mensaje = "Opciones cargadas. Marca las consultas y pulsa Descargar.".into();
                        }
                        Err(e) => {
                            self.catalogo = None;
                            self.mensaje = e;
                        }
                    }
                }
                Evento::Fin(carpeta, r) => {
                    self.receptor = None;
                    self.carpeta = Some(carpeta.clone());
                    match *r {
                        Ok(resumen) => {
                            self.mensaje = format!(
                                "Lote terminado: {} consultas, {} sin registros. Puedes llevar los Excel a Trabajo.",
                                resumen.terminadas.len(),
                                resumen.vacias.len()
                            );
                            self.listo = Some((carpeta, resumen));
                        }
                        Err(e) => self.mensaje = e,
                    }
                }
            }
        }
    }

    fn cargar_opciones(&mut self, ctx: &egui::Context) {
        let Some(puente) = self.puente.clone() else {
            return;
        };
        let pantalla = self.pantalla.clone();
        let ctx = ctx.clone();
        let (tx, rx) = channel();
        self.receptor = Some(rx);
        self.mensaje = "Comprobando extensión y cargando opciones…".into();
        std::thread::spawn(move || {
            let r = lotes::catalogo(puente.as_ref(), &pantalla);
            let _ = tx.send(Evento::Opciones(r));
            ctx.request_repaint();
        });
    }

    fn plan(&self) -> Result<Lote, String> {
        let Some(catalogo) = &self.catalogo else {
            return Err("Carga las opciones primero.".into());
        };
        let marcadas = |xs: &Vec<(String, String, bool)>| xs.iter().filter(|(_, _, m)| *m).map(|(k, _, _)| k.clone()).collect::<Vec<_>>();
        let mut consultas = vec![];
        for tribunal in marcadas(&self.tribunales) {
            let base = Seleccion {
                tribunal,
                screen: self.pantalla.clone(),
                tab: String::new(),
                modality: String::new(),
                report: None,
                state: None,
                month: None,
                year: None,
            };
            if self.pantalla == "seguimiento" {
                for (tab, _) in self.pestañas.iter().filter(|(_, m)| *m) {
                    for modalidad in marcadas(&self.modalidades) {
                        let filtros = match tab.as_str() {
                            "Informes" => marcadas(&self.informes).into_iter().map(Some).collect::<Vec<_>>(),
                            "Cumplimiento" => marcadas(&self.medidas).into_iter().map(|s| if s == "0" { None } else { Some(s) }).collect(),
                            _ => vec![None],
                        };
                        if filtros.is_empty() {
                            return Err(format!("Marca un filtro para {tab}."));
                        }
                        for report in filtros {
                            let mut s = base.clone();
                            s.tab = tab.clone();
                            s.modality = modalidad.clone();
                            s.report = report;
                            consultas.push(s);
                        }
                    }
                }
            } else if self.pantalla == "litigantes" {
                for estado in marcadas(&self.estados) {
                    let mut s = base.clone();
                    s.state = Some(estado);
                    consultas.push(s);
                }
            } else {
                for mes in marcadas(&self.meses) {
                    let mut s = base.clone();
                    s.month = Some(mes);
                    s.year = Some(self.año.clone());
                    consultas.push(s);
                }
            }
        }
        let lote = Lote {
            consultas,
            fechas: self.fechas.then(|| Fechas { start: self.desde.clone(), end: self.hasta.clone() }),
            conservar_filtros: self.filtros,
            evidencia: self.evidencia,
            pausa_ms: 1000,
            lecturas: self.lecturas.clone(),
        };
        lote.validar(catalogo)?;
        Ok(lote)
    }

    fn iniciar(&mut self, lote: Lote, carpeta: PathBuf, retomar: bool, ctx: &egui::Context) {
        let (Some(puente), Some(catalogo)) = (self.puente.clone(), self.catalogo.clone()) else {
            return;
        };
        let (tx, rx) = channel();
        self.receptor = Some(rx);
        self.cancelar = Arc::new(AtomicBool::new(false));
        let cancelar = self.cancelar.clone();
        let ctx = ctx.clone();
        self.listo = None;
        self.carpeta = Some(carpeta.clone());
        std::thread::spawn(move || {
            let progreso = |m| {
                let _ = tx.send(Evento::Avance(m));
                ctx.request_repaint();
            };
            let r = lotes::ejecutar(puente.as_ref(), &catalogo, &lote, &carpeta, retomar, &cancelar, &progreso);
            let _ = tx.send(Evento::Fin(carpeta, Box::new(r)));
            ctx.request_repaint();
        });
    }

    pub fn mostrar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> Option<(PathBuf, Resumen)> {
        self.recibir();
        ui.heading("Captura desde SITFA");
        ui.label("Usa tu sesión abierta de Chrome. Solo consultas; no escribe en RUS ni SATURNO.");
        let ocupada = self.ocupada();
        ui.horizontal(|ui| {
            if self.puente.is_none() && ui.button("Abrir conexión local").clicked() {
                match Puente::abrir() {
                    Ok(p) => self.puente = Some(Arc::new(p)),
                    Err(e) => self.mensaje = e,
                }
            }
            if let Some(p) = &self.puente {
                ui.label(if p.conectado() { "Chrome conectado" } else { "Esperando conexión de Chrome" });
                if ui.button("Copiar código de conexión").clicked() {
                    ctx.copy_text(p.codigo_conexion());
                }
            }
            if self.puente.is_some() && ui.add_enabled(!ocupada, egui::Button::new("Cerrar conexión")).clicked() {
                if let Some(p) = self.puente.take() {
                    p.cerrar();
                }
                self.catalogo = None;
            }
        });
        if self.puente.is_some() {
            ui.label("En Chrome: pulsa el icono de Flow en SITFA, pega el código y deja abierta la pestaña de conexión.");
        }
        ui.separator();
        ui.add_enabled_ui(!ocupada, |ui| {
            let anterior = self.pantalla.clone();
            egui::ComboBox::from_id_salt("captura_pantalla")
                .selected_text(match self.pantalla.as_str() {
                    "litigantes" => "Búsqueda de litigantes",
                    "calendario_informes" => "Informes por vencer",
                    "calendario_medidas" => "Medidas por vencer",
                    _ => "Seguimiento",
                })
                .show_ui(ui, |ui| {
                    for (k, n) in [
                        ("seguimiento", "Seguimiento"),
                        ("litigantes", "Búsqueda de litigantes"),
                        ("calendario_informes", "Informes por vencer"),
                        ("calendario_medidas", "Medidas por vencer"),
                    ] {
                        ui.selectable_value(&mut self.pantalla, k.into(), n);
                    }
                });
            if anterior != self.pantalla {
                self.catalogo = None;
                self.fechas = false;
            }
            if ui.add_enabled(self.puente.as_ref().is_some_and(|p| p.conectado()), egui::Button::new("Cargar opciones")).clicked() {
                self.cargar_opciones(ctx);
            }
        });
        egui::ScrollArea::vertical().id_salt("opciones_captura").max_height(330.0).show(ui, |ui| {
            ui.add_enabled_ui(!ocupada && self.catalogo.is_some(), |ui| {
                marcas(ui, "Tribunales", &mut self.tribunales);
                if self.pantalla == "seguimiento" {
                    ui.label(egui::RichText::new("Lecturas por ingreso y causa").strong());
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.lecturas.bitacoras,"Leer bitácoras");
                        ui.checkbox(&mut self.lecturas.historia,"Leer historia de la causa");
                        ui.checkbox(&mut self.lecturas.calendario,"Leer calendario del ingreso");
                    });
                    ui.horizontal(|ui| {ui.label("Período:");ui.add(egui::DragValue::new(&mut self.lecturas.meses).range(1..=4));ui.label("meses naturales");});
                    ui.label("Reutiliza lecturas verificadas de hoy. Máximo de 600 bitácoras/calendarios y 600 historias por corrida; Retomar conserva el avance.");
                    marcas(ui, "Modalidades", &mut self.modalidades);
                    ui.horizontal_wrapped(|ui| {
                        for (n, m) in &mut self.pestañas {
                            ui.checkbox(m, n.as_str());
                        }
                    });
                    if self.pestañas.iter().any(|(n, m)| n == "Cumplimiento" && *m) {
                        marcas(ui, "Medidas de Cumplimiento", &mut self.medidas);
                    }
                    if self.pestañas.iter().any(|(n, m)| n == "Informes" && *m) {
                        marcas(ui, "Informes", &mut self.informes);
                    }
                } else if self.pantalla == "litigantes" {
                    marcas(ui, "Estados", &mut self.estados);
                } else {
                    marcas(ui, "Meses", &mut self.meses);
                    ui.horizontal(|ui| {
                        ui.label("Año:");
                        ui.text_edit_singleline(&mut self.año);
                    });
                }
                if !self.pantalla.starts_with("calendario_") {
                    ui.checkbox(&mut self.fechas, "Filtrar por fechas");
                    if self.fechas {
                        ui.horizontal(|ui| {
                            ui.label("Desde:");
                            ui.text_edit_singleline(&mut self.desde);
                            ui.label("Hasta:");
                            ui.text_edit_singleline(&mut self.hasta);
                        });
                    }
                    ui.checkbox(&mut self.filtros, "Conservar otros filtros del formulario SITFA");
                }
                ui.checkbox(&mut self.evidencia, "Guardar PDF cuando una consulta no tiene registros");
            });
        });
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(!self.ocupada() && self.catalogo.is_some(), egui::Button::new("Descargar")).clicked() {
                match self.plan() {
                    Ok(lote) => {
                        if let Some(destino) = rfd::FileDialog::new().set_title("Carpeta para los lotes de Captura").pick_folder() {
                            match lotes::carpeta_nueva(&destino) {
                                Ok(c) => self.iniciar(lote, c, false, ctx),
                                Err(e) => self.mensaje = e,
                            }
                        }
                    }
                    Err(e) => self.mensaje = e,
                }
            }
            if ui.add_enabled(!self.ocupada() && self.catalogo.is_some(), egui::Button::new("Retomar lote…")).clicked() {
                if let Some(c) = rfd::FileDialog::new().set_title("Carpeta del lote incompleto").pick_folder() {
                    match Resumen::leer(&c) {
                        Ok(r) => self.iniciar(r.lote, c, true, ctx),
                        Err(e) => self.mensaje = e,
                    }
                }
            }
            if ui.add_enabled(self.ocupada(), egui::Button::new("Detener")).clicked() {
                self.cancelar.store(true, Ordering::SeqCst);
                self.mensaje = "Deteniendo al finalizar la consulta en curso…".into();
            }
            if self.ocupada() {
                ui.spinner();
            }
        });
        ui.label(&self.mensaje);
        let llevar = self.listo.is_some() && ui.button("Llevar Excel verificados a Trabajo").clicked();
        if self.puente.is_some() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        if llevar {
            self.listo.take()
        } else {
            None
        }
    }

    pub fn cerrar(&self) {
        self.cancelar.store(true, Ordering::SeqCst);
        if let Some(p) = &self.puente {
            let _ = p.llamar("unlock", serde_json::json!({}), Duration::from_secs(2));
            p.cerrar();
        }
    }
}
