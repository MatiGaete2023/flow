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
    Paquete(Result<String, String>),
    /// «Consultar y crear paquete»: carpeta de destino (o del paquete retomado) y resultado.
    Flujo(PathBuf, Box<Result<flow_trabajo::flujo::Resultado, String>>),
}

pub struct Captura {
    /// Flow Descargador: solo descarga y verifica; no ofrece llevar los Excel a Trabajo.
    pub independiente: bool,
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
    /// Calendario de informes por vencer en el paquete (`informes_por_vencer`, solo con Cumplimiento).
    informes_por_vencer: bool,
    lecturas: flow_captura::recoleccion::Opciones,
    receptor: Option<Receiver<Evento>>,
    /// Inicio de la carga de opciones en curso, para mostrar el tiempo y permitir cancelarla.
    cargando: Option<std::time::Instant>,
    cancelar: Arc<AtomicBool>,
    mensaje: String,
    carpeta: Option<PathBuf>,
    listo: Option<(PathBuf, Resumen)>,
}

impl Captura {
    /// Mantiene disponible el lote terminado (el Descargador puede volver a abrir su carpeta).
    pub fn listo_de_nuevo(&mut self, carpeta: std::path::PathBuf, resumen: flow_captura::lotes::Resumen) {
        self.listo = Some((carpeta, resumen));
    }
}

impl Default for Captura {
    fn default() -> Self {
        Self {
            independiente: false,
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
            informes_por_vencer: true,
            lecturas: Default::default(),
            receptor: None,
            cargando: None,
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

/// «Todas las medidas» (clave `0`) una sola vez aunque el catálogo de SITFA también la traiga, y sin claves repetidas:
/// dos casillas con la misma clave generaban consultas repetidas.
fn medidas(catalogo: &Value) -> Vec<(String, String, bool)> {
    let mut salida = vec![("0".to_string(), "Todas las medidas".to_string(), true)];
    for (k, n, m) in opciones(catalogo, "medidas") {
        if !salida.iter().any(|(c, _, _)| *c == k) {
            salida.push((k, n, m));
        }
    }
    salida
}

/// Mensaje de fin de lote; menciona las lecturas por ingreso que quedaron pendientes (sus XLS ya están verificados).
fn mensaje_fin(resumen: &Resumen, independiente: bool) -> String {
    let pendientes = flow_captura::recoleccion::lecturas_pendientes(resumen);
    format!(
        "Lote terminado: {} consultas, {} sin registros. {}{}",
        resumen.terminadas.len(),
        resumen.vacias.len(),
        if independiente {
            "Los Excel verificados quedaron en la carpeta del lote; ábrelos en Flow Asistente."
        } else {
            "Puedes llevar los Excel a Trabajo."
        },
        if pendientes > 0 {
            format!(" Quedaron {pendientes} lecturas por ingreso pendientes; usa «Retomar lote…» para reintentarlas.")
        } else {
            String::new()
        }
    )
}

/// Tras un lote detenido, el Descargador puede abrir la carpeta si hay archivos verificados; Trabajo solo recibe el lote
/// si todas sus consultas tienen sus XLS verificados (o quedaron sin registros).
fn disponible_tras_error(resumen: &Resumen, independiente: bool) -> bool {
    !resumen.archivos.is_empty()
        && (independiente || (0..resumen.lote.consultas.len()).all(|i| resumen.terminadas.contains(&i) || resumen.vacias.contains(&i)))
}

/// Resultado de un paquete creado: ruta y bitácoras incluidas, como `reuse` de `package_current_lot`.
fn mensaje_paquete(destino: &std::path::Path, p: &flow_trabajo::exportar::Producto) -> String {
    let mut m = format!("Paquete creado: {}. {} Ábrelo en Flow Asistente con «Abrir Excel o paquete…».", destino.display(), p.bitacoras);
    for a in &p.avisos {
        m.push(' ');
        m.push_str(a);
    }
    m
}

/// Fin de «Consultar y crear paquete», como `package_done` del Descargador.
fn mensaje_flujo(r: &flow_trabajo::flujo::Resultado) -> String {
    let mut m = format!("Paquete {}: {}. {}", if r.parcial { "PARCIAL" } else { "COMPLETO" }, r.paquete.display(), r.bitacoras);
    for a in &r.avisos {
        m.push(' ');
        m.push_str(a);
    }
    if r.parcial {
        m.push_str(" Puedes abrirlo así en el Asistente o usar «Retomar paquete…» para reintentar lo pendiente.");
    } else {
        m.push_str(" Ábrelo en Flow Asistente con «Abrir Excel o paquete…».");
    }
    m
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
                    self.cargando = None;
                    match r {
                        Ok(c) => {
                            self.tribunales = opciones(&c, "tribunales");
                            self.modalidades = opciones(&c, "modalidades");
                            self.informes = opciones(&c, "informes");
                            self.medidas = medidas(&c);
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
                Evento::Flujo(carpeta, r) => {
                    self.receptor = None;
                    self.mensaje = match *r {
                        Ok(r) => mensaje_flujo(&r),
                        Err(e) => {
                            let raiz = if carpeta.join("respaldo").join(flow_trabajo::flujo::PROCESO).is_file() {
                                Some(carpeta)
                            } else {
                                flow_trabajo::flujo::pendientes(&carpeta, 7).into_iter().next().map(|(c, _)| c)
                            };
                            match raiz {
                                Some(c) => format!(
                                    "{e} El avance quedó en {}: usa «Retomar paquete…» para continuar sin repetir lo ya hecho.",
                                    c.display()
                                ),
                                None => e,
                            }
                        }
                    };
                }
                Evento::Paquete(r) => {
                    self.receptor = None;
                    self.mensaje = match r {
                        Ok(m) => m,
                        Err(e) => format!("No se creó el paquete: {e}"),
                    };
                }
                Evento::Fin(carpeta, r) => {
                    self.receptor = None;
                    self.carpeta = Some(carpeta.clone());
                    match *r {
                        Ok(resumen) => {
                            self.mensaje = mensaje_fin(&resumen, self.independiente);
                            self.listo = Some((carpeta, resumen));
                        }
                        Err(e) => {
                            self.mensaje = e;
                            if let Ok(resumen) = Resumen::leer(&carpeta) {
                                if disponible_tras_error(&resumen, self.independiente) {
                                    self.listo = Some((carpeta, resumen));
                                }
                            }
                        }
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
        self.cargando = Some(std::time::Instant::now());
        self.mensaje = "Pidiendo las opciones a Chrome…".into();
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
                            if !consultas.contains(&s) {
                                consultas.push(s);
                            }
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

    /// Lote para «Crear paquete de este lote…»: el recién terminado, el último abierto o uno que elija el usuario.
    fn lote_para_paquete(&self) -> Option<Result<(PathBuf, Resumen), String>> {
        if let Some(listo) = &self.listo {
            return Some(Ok(listo.clone()));
        }
        if let Some(r) =
            self.carpeta.as_ref().and_then(|c| Resumen::leer(c).ok().filter(|r| r.estado == "VALIDADA").map(|r| (c.clone(), r)))
        {
            return Some(Ok(r));
        }
        let carpeta = rfd::FileDialog::new().set_title("Carpeta del lote verificado").pick_folder()?;
        Some(Resumen::leer(&carpeta).map(|r| (carpeta, r)))
    }

    /// Como `create_package` del Descargador: el paquete del Asistente con las hojas y lecturas de este lote.
    fn crear_paquete(&mut self, ctx: &egui::Context) {
        let (carpeta, resumen) = match self.lote_para_paquete() {
            None => return,
            Some(Err(e)) => return self.mensaje = e,
            Some(Ok(l)) => l,
        };
        let Some(destino) = rfd::FileDialog::new()
            .set_title("Guardar paquete para el Asistente")
            .add_filter("Paquete CSMP", &["zip"])
            .set_directory(carpeta.parent().unwrap_or(&carpeta))
            .set_file_name(chrono::Local::now().format("Captura_CSMP_%Y-%m-%d_%H%M.zip").to_string())
            .save_file()
        else {
            return;
        };
        if destino.exists() {
            self.mensaje = "Ese archivo ya existe. Elige un nombre nuevo para no reemplazar otro paquete.".into();
            return;
        }
        let (tx, rx) = channel();
        self.receptor = Some(rx);
        self.mensaje = "Creando el paquete del lote…".into();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let lotes = [flow_trabajo::exportar::Lote { carpeta: &carpeta, resumen: &resumen }];
            let r = flow_trabajo::exportar::exportar(&lotes, &destino, &Default::default()).map(|p| mensaje_paquete(&destino, &p));
            let _ = tx.send(Evento::Paquete(r));
            ctx.request_repaint();
        });
    }

    fn preferencias(&self) -> flow_trabajo::flujo::Preferencias {
        flow_trabajo::flujo::Preferencias {
            leer_historia: self.lecturas.historia,
            leer_calendarios: self.lecturas.calendario,
            informes_por_vencer: self.informes_por_vencer,
            meses_atras: self.lecturas.meses,
            pdf_sin_registros: self.evidencia,
            ..Default::default()
        }
    }

    /// «Consultar y crear paquete» (con `lote`) o «Retomar paquete…» (sin él, `carpeta` es el paquete cortado).
    fn iniciar_flujo(&mut self, lote: Option<Lote>, carpeta: PathBuf, ctx: &egui::Context) {
        let (Some(puente), Some(catalogo)) = (self.puente.clone(), self.catalogo.clone()) else {
            return;
        };
        let (tx, rx) = channel();
        self.receptor = Some(rx);
        self.cancelar = Arc::new(AtomicBool::new(false));
        let cancelar = self.cancelar.clone();
        let prefs = self.preferencias();
        let ctx = ctx.clone();
        self.listo = None;
        self.mensaje = "Preparando el paquete: consultas, lecturas por ingreso, informes por vencer y ZIP…".into();
        std::thread::spawn(move || {
            let progreso = |m| {
                let _ = tx.send(Evento::Avance(m));
                ctx.request_repaint();
            };
            let sitfa = flow_trabajo::flujo::EnChrome {
                conexion: puente.as_ref(),
                catalogo: &catalogo,
                cancelar: &cancelar,
                progreso: &progreso,
                informes: Default::default(),
            };
            let hoy = chrono::Local::now().date_naive();
            let r = match &lote {
                Some(l) => flow_trabajo::flujo::ejecutar(&sitfa, l, &carpeta, &prefs, hoy),
                None => flow_trabajo::flujo::retomar(&sitfa, &carpeta, &prefs, hoy),
            };
            let _ = tx.send(Evento::Flujo(carpeta, Box::new(r)));
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
            if let Some(inicio) = self.cargando {
                ui.spinner();
                ui.label(format!(
                    "Cargando opciones… {} s. Mira la pestaña de conexión en Chrome: ahí dice qué está haciendo.",
                    inicio.elapsed().as_secs()
                ));
                if ui.button("Cancelar").clicked() {
                    // La respuesta tardía de Chrome se descarta; la conexión sigue abierta.
                    self.receptor = None;
                    self.cargando = None;
                    self.mensaje =
                        "Carga de opciones cancelada. Revisa que la pestaña SITFA muestre la pantalla de consulta y vuelve a intentar."
                            .into();
                }
                ctx.request_repaint_after(Duration::from_millis(500));
            } else if ui.add_enabled(self.puente.as_ref().is_some_and(|p| p.conectado()), egui::Button::new("Cargar opciones")).clicked() {
                self.cargar_opciones(ctx);
            }
        });
        egui::ScrollArea::vertical().id_salt("opciones_captura").max_height(330.0).show(ui, |ui| {
            ui.add_enabled_ui(!ocupada && self.catalogo.is_some(), |ui| {
                marcas(ui, "Tribunales", &mut self.tribunales);
                if self.pantalla == "seguimiento" {
                    ui.label(egui::RichText::new("Lecturas por ingreso y causa").strong());
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.lecturas.bitacoras,"Leer bitácoras al descargar");
                        ui.checkbox(&mut self.lecturas.historia,"Leer historia de la causa");
                        ui.checkbox(&mut self.lecturas.calendario,"Leer calendario del ingreso");
                    });
                    ui.horizontal(|ui| {ui.label("Período:");ui.add(egui::DragValue::new(&mut self.lecturas.meses).range(1..=4));ui.label("meses naturales");});
                    ui.label("Reutiliza lecturas verificadas de hoy. Máximo de 2000 bitácoras/calendarios y 2000 historias por corrida; Retomar conserva el avance.");
                    ui.checkbox(
                        &mut self.informes_por_vencer,
                        "Paquete: incluir el calendario de informes por vencer (60 días, con Cumplimiento)",
                    );
                    ui.label("Bitácoras incluidas en «Consultar y crear paquete». La casilla anterior solo configura «Descargar».");
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
            let seguimiento = self.pantalla == "seguimiento";
            if seguimiento
                && ui
                    .add_enabled(
                        !self.ocupada() && self.catalogo.is_some(),
                        egui::Button::new(egui::RichText::new("Consultar y crear paquete").strong()),
                    )
                    .on_hover_text("Consulta Seguimiento, lee las bitácoras y crea Captura_CSMP.zip para el Asistente.")
                    .clicked()
            {
                match self.plan() {
                    Ok(lote) => {
                        if let Some(destino) = rfd::FileDialog::new().set_title("Carpeta para los paquetes CSMP").pick_folder() {
                            self.iniciar_flujo(Some(lote), destino, ctx);
                        }
                    }
                    Err(e) => self.mensaje = e,
                }
            }
            if seguimiento && ui.add_enabled(!self.ocupada() && self.catalogo.is_some(), egui::Button::new("Retomar paquete…")).clicked()
            {
                if let Some(c) = rfd::FileDialog::new().set_title("Carpeta «Paquete CSMP …» a retomar").pick_folder() {
                    if c.join("respaldo").join(flow_trabajo::flujo::PROCESO).is_file() {
                        self.iniciar_flujo(None, c, ctx);
                    } else {
                        self.mensaje = "Elige una carpeta «Paquete CSMP …» (contiene respaldo/proceso.json).".into();
                    }
                }
            }
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
            if ui
                .add_enabled(!self.ocupada(), egui::Button::new(if self.listo.is_some() { "Crear paquete del lote actual…" } else { "Crear paquete de un lote…" }))
                .on_hover_text("Arma Captura_CSMP.zip con los Excel y las lecturas verificadas del lote, para abrirlo en el Asistente.")
                .clicked()
            {
                self.crear_paquete(ctx);
            }
            if ui.add_enabled(self.ocupada(), egui::Button::new("Detener descarga")).clicked() {
                self.cancelar.store(true, Ordering::SeqCst);
                self.mensaje = "Deteniendo al finalizar la consulta en curso…".into();
            }
            if self.ocupada() {
                ui.spinner();
            }
        });
        ui.label(&self.mensaje);
        let llevar = self.listo.is_some()
            && if self.independiente {
                ui.button("Abrir carpeta del lote").clicked()
            } else {
                ui.button("Llevar Excel verificados a Trabajo").clicked()
            };
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

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;

    #[test]
    fn todas_las_medidas_aparece_una_sola_vez() {
        let c = json!({"medidas": {"0": "Todas", "5": "Medida ficticia", "6": "Otra ficticia"}});
        let m = medidas(&c);
        assert_eq!(m.iter().map(|(k, _, _)| k.as_str()).collect::<Vec<_>>(), ["0", "5", "6"]);
        assert_eq!(m[0].1, "Todas las medidas");
    }

    fn resumen(terminadas: Vec<usize>, avisos: Vec<String>) -> Resumen {
        serde_json::from_value(json!({
            "version": 1,
            "lote": {"consultas": [{"tribunal":"901","screen":"seguimiento","tab":"Espera","modality":"2","report":null,"state":null,"month":null,"year":null},
                                   {"tribunal":"901","screen":"seguimiento","tab":"Cumplimiento","modality":"2","report":null,"state":null,"month":null,"year":null}],
                     "fechas": null, "conservar_filtros": false, "evidencia": false, "pausa_ms": 250},
            "estado": "INCOMPLETA", "terminadas": terminadas, "vacias": [],
            "archivos": [{"nombre": "a.xls", "sha256": "0".repeat(64), "consulta": 0, "pagina": 1, "total": 1, "registros": 1}],
            "avisos": avisos
        }))
        .unwrap()
    }

    #[test]
    fn fin_con_lecturas_pendientes_lo_dice_y_lote_detenido_sigue_disponible_si_corresponde() {
        let r = resumen(vec![0, 1], vec!["Consulta 1, página 1, fila 2: bitacora pendiente. Fallo ficticio.".into()]);
        assert!(mensaje_fin(&r, false).contains("Quedaron 1 lecturas por ingreso pendientes"));
        assert!(!mensaje_fin(&resumen(vec![0, 1], vec![]), true).contains("pendientes"));
        assert!(disponible_tras_error(&r, false));
        let parcial = resumen(vec![0], vec![]);
        assert!(disponible_tras_error(&parcial, true), "el Descargador puede abrir la carpeta");
        assert!(!disponible_tras_error(&parcial, false), "Trabajo no recibe un lote con consultas sin XLS");
    }
}
