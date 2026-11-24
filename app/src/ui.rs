//! Ventana principal: navegación lateral y las pantallas del Asistente y del Anonimizador.

use crate::anon;
use crate::config::{self, Config, CC_OBLIGATORIO, UMBRALES};
use crate::motor::Mode;
use crate::productos::{self, Borrador, Proyecto, MODALIDADES, TIPOS};
use crate::theme::{self, Pal};
use crate::trabajo::{Estado, Trabajo};
use crate::util::*;
use chrono::NaiveDate;
use egui::{Color32, Key, KeyboardShortcut, Modifiers, RichText};
use egui_extras::{Column, TableBuilder};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Vista {
    Trabajo,
    Correos,
    Resoluciones,
    Resultados,
    Contactos,
    Anonimizador,
    Descargador,
    Ajustes,
}

const VISTAS: [(Vista, &str, &str); 8] = [
    (Vista::Trabajo, "🗂", "Trabajo"),
    (Vista::Correos, "✉", "Correos"),
    (Vista::Resoluciones, "⚖", "Resoluciones"),
    (Vista::Resultados, "📊", "Resultados"),
    (Vista::Contactos, "👥", "Contactos"),
    (Vista::Anonimizador, "🛡", "Anonimizador"),
    (Vista::Descargador, "↓", "Descargador"),
    (Vista::Ajustes, "⚙", "Ajustes"),
];

pub struct Producto {
    pub ruta: PathBuf,
    pub tipo: String,
    pub hora: String,
}

pub struct Screenshot {
    pub path: PathBuf,
    pub frames: u32,
    pub pedido: bool,
}

/// Conflictos entre la captura abierta y la nueva: el trabajo nuevo espera las respuestas de la persona.
struct DialogoConciliacion {
    nuevo: Trabajo,
    conflictos: Vec<crate::conciliar::Conflicto>,
    conservar: Vec<bool>,
}

/// Conflictos de «Actualizar desde Excel» a la espera de la decisión de la persona.
struct DialogoSincronia {
    conflictos: Vec<crate::sincronizar::Conflicto>,
    usar_excel: Vec<bool>,
    headers: Vec<String>,
    filas: Vec<Vec<Cell>>,
}

pub struct App {
    cfg: Config,
    vista: Vista,
    work: Option<Trabajo>,
    corte_txt: String,
    pedida: Option<Mode>,
    sel: Option<usize>,
    filtro: String,
    filtro_estado: Option<Estado>,
    filtro_trib: String,
    solo_marcadas: bool,
    ultima_marca: Option<usize>,
    cache_key: String,
    cache: Vec<usize>,
    revision: u64,
    trib_cache: (u64, Vec<String>),
    // correos
    modalidades: [bool; 4],
    tribunales_sel: BTreeSet<String>,
    solo_sel_correo: bool,
    borradores: Vec<Borrador>,
    bsel: Option<usize>,
    // resoluciones
    proyectos: Vec<Proyecto>,
    psel: Option<usize>,
    solo_sel_res: bool,
    // resultados
    productos: Vec<Producto>,
    // contactos
    nuevo_prog: String,
    nuevo_mail: String,
    filtro_contactos: String,
    filtro_umbrales: String,
    // anonimizador
    anon_files: Vec<PathBuf>,
    anon_out: String,
    anon_opts: anon::Opciones,
    anon_inf: Option<anon::Informe>,
    descargador: crate::descargador_ui::Descargador,
    // ajustes
    texto_sel: String,
    // estado
    avisos: Vec<(String, bool, Instant)>,
    pub screenshot: Option<Screenshot>,
    sucio: Option<Instant>,
    confirmar_reinicio: bool,
    revision_bitacoras: Option<crate::revision_ui::Dialogo>,
    sincronia: Option<DialogoSincronia>,
    conciliacion: Option<DialogoConciliacion>,
    ultimo_tema: Option<bool>,
}

impl App {
    pub fn new(screenshot: Option<PathBuf>) -> Self {
        let (cfg, err) = config::load();
        let hoy = chrono::Local::now().date_naive();
        let mut app = App {
            cfg,
            vista: Vista::Trabajo,
            work: None,
            corte_txt: hoy.format("%d/%m/%Y").to_string(),
            pedida: None,
            sel: None,
            filtro: String::new(),
            filtro_estado: None,
            filtro_trib: String::new(),
            solo_marcadas: false,
            ultima_marca: None,
            cache_key: String::new(),
            cache: vec![],
            revision: 0,
            trib_cache: (u64::MAX, vec![]),
            modalidades: [true; 4],
            tribunales_sel: BTreeSet::new(),
            solo_sel_correo: false,
            borradores: vec![],
            bsel: None,
            proyectos: vec![],
            psel: None,
            solo_sel_res: false,
            productos: vec![],
            nuevo_prog: String::new(),
            nuevo_mail: String::new(),
            filtro_contactos: String::new(),
            filtro_umbrales: String::new(),
            anon_files: vec![],
            anon_out: String::new(),
            anon_opts: anon::Opciones::default(),
            anon_inf: None,
            descargador: Default::default(),
            texto_sel: "ESPERA.E05_PROYECTO_Y_CORREO".into(),
            avisos: vec![],
            screenshot: screenshot.map(|path| Screenshot { path, frames: 0, pedido: false }),
            sucio: None,
            confirmar_reinicio: false,
            revision_bitacoras: None,
            sincronia: None,
            conciliacion: None,
            ultimo_tema: None,
        };
        if let Some(e) = err {
            app.aviso(e, true);
        }
        if app.screenshot.is_some() {
            // La captura de control muestra un trabajo de ejemplo ficticio, con su configuración base.
            app.cfg = Config::default();
            app.cargar_ejemplo(Mode::Espera);
            app.avisos.clear();
            app.sel = Some(1);
        }
        app
    }

    fn p(&self) -> Pal {
        theme::pal(self.cfg.tema_oscuro)
    }

    fn aviso(&mut self, t: impl Into<String>, error: bool) {
        self.avisos.push((t.into(), error, Instant::now()));
    }

    fn corte(&self) -> NaiveDate {
        parse_date_text(&self.corte_txt).unwrap_or_else(|| chrono::Local::now().date_naive())
    }

    fn guardar_cfg(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        if let Err(e) = config::save(&self.cfg) {
            self.aviso(format!("No se pudo guardar la configuración: {e}"), true);
        }
    }

    fn marcar_sucio(&mut self) {
        self.revision += 1;
        self.sucio = Some(Instant::now());
    }

    fn carpeta_productos(&mut self) -> Option<PathBuf> {
        if !self.cfg.carpeta_productos.is_empty() {
            let p = PathBuf::from(&self.cfg.carpeta_productos);
            if std::fs::create_dir_all(&p).is_ok() {
                return Some(p);
            }
        }
        let p = rfd::FileDialog::new().set_title("Carpeta donde dejar los productos").pick_folder()?;
        self.cfg.carpeta_productos = p.display().to_string();
        self.guardar_cfg();
        Some(p)
    }

    fn registrar(&mut self, ruta: PathBuf, tipo: &str) {
        self.productos.push(Producto { ruta, tipo: tipo.into(), hora: chrono::Local::now().format("%H:%M:%S").to_string() });
    }

    // ── abrir ────────────────────────────────────────────────────────────────

    fn abrir_dialogo(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .set_title("Abrir Excel de RUS o paquete CSMP")
            .add_filter("Excel de RUS", &["xls", "xlsx", "xlsm", "htm", "html", "xml", "csv"])
            .add_filter("Paquete CSMP", &["zip"])
            .pick_file()
        {
            self.abrir(p);
        }
    }

    fn abrir(&mut self, p: PathBuf) {
        if !self.guardar_pendiente() {
            return;
        }
        // Un paquete nuevo con otro paquete abierto continúa su trabajo en la misma etapa (conciliación entre capturas).
        let previo_paquete = self.work.as_ref().is_some_and(|w| w.paquete.is_some()) && crate::captura_trabajo::es_zip(&p);
        let pedida = if previo_paquete { self.work.as_ref().map(|w| w.mode()) } else { self.pedida };
        match Trabajo::abrir_archivo(p.clone(), pedida, self.corte(), &self.cfg) {
            Ok(w) if previo_paquete && w.paquete.is_some() => self.conciliar_con_anterior(w, Default::default()),
            Ok(w) => {
                self.work = Some(w);
                self.activar_trabajo();
            }
            Err(e) if previo_paquete && pedida.is_some() => match Trabajo::abrir_archivo(p, self.pedida, self.corte(), &self.cfg) {
                Ok(w) => self.conciliar_con_anterior(w, Default::default()),
                Err(_) => self.aviso(e, true),
            },
            Err(e) => self.aviso(e, true),
        }
    }

    /// Pasa a `nuevo` las decisiones del trabajo abierto; con conflictos, pide las respuestas antes de cambiar nada.
    fn conciliar_con_anterior(&mut self, mut nuevo: Trabajo, respuestas: crate::conciliar::Respuestas) {
        let Some(previo) = self.work.take() else {
            self.work = Some(nuevo);
            self.activar_trabajo();
            return;
        };
        match crate::conciliar::conciliar(&mut nuevo, &previo, &respuestas, &self.cfg) {
            Ok(r) => {
                let mut texto = String::new();
                if let Some(c) = &nuevo.cambios {
                    let m = c.resumen();
                    texto = format!(
                        " Frente a la captura anterior: {} nuevas, {} con fuente cambiada, {} con reglas cambiadas, {} egresadas.",
                        m["Nuevas"], m["Fuente"], m["Reglas"], m["Egresados"]
                    );
                }
                let n = nuevo.registros.iter().filter(|r| r.editada.is_some() || r.revisada || !r.res.is_empty()).count();
                self.work = Some(nuevo);
                self.activar_trabajo();
                self.conciliacion = None;
                if r.continuada {
                    self.aviso(format!("Paquete conciliado: {} filas continúan y {n} conservan decisiones.{texto}", r.emparejadas.len()), false);
                }
            }
            Err(crate::conciliar::Error::Conflictos(conflictos)) => {
                let conservar = conflictos.iter().map(|c| c.puede_conservar).collect();
                self.work = Some(previo);
                self.conciliacion = Some(DialogoConciliacion { nuevo, conflictos, conservar });
            }
            Err(crate::conciliar::Error::Respuesta(m)) => {
                self.work = Some(previo);
                self.aviso(m, true);
            }
        }
    }

    fn dialogo_conciliacion(&mut self, ctx: &egui::Context, p: &Pal) {
        let Some(d) = self.conciliacion.as_mut() else { return };
        let mut abierto = true;
        let (mut aplicar, mut cancelar) = (false, false);
        egui::Window::new("Conciliar capturas").open(&mut abierto).default_width(680.0).show(ctx, |ui| {
            ui.label(format!("{} filas con decisiones tuyas cambiaron entre las dos capturas. Elige qué hacer con cada una; el trabajo anterior sigue intacto hasta aplicar.", d.conflictos.len()));
            egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                for (c, conservar) in d.conflictos.iter().zip(&mut d.conservar) {
                    ui.separator();
                    ui.label(RichText::new(&c.motivo).strong());
                    if !c.campos.is_empty() {
                        ui.label(format!("Campos distintos: {}", c.campos.join(", ")));
                        for k in &c.campos {
                            ui.label(RichText::new(format!("{k}: {} → {}", c.antes.get(k).map(texto_json).unwrap_or_default(), c.despues.get(k).map(texto_json).unwrap_or_default())).color(p.muted).size(12.0));
                        }
                    } else {
                        ui.label(RichText::new("No hay una correspondencia única: no se trasladarán decisiones automáticamente.").color(p.muted));
                    }
                    ui.add_enabled_ui(c.puede_conservar, |ui| ui.radio_value(conservar, true, "Conservar mis decisiones"));
                    ui.radio_value(conservar, false, "Usar la nueva evaluación");
                }
            });
            ui.label(RichText::new("Los productos preparados con la captura anterior deben actualizarse.").color(p.muted).size(12.0));
            ui.horizontal(|ui| {
                aplicar = theme::primary(ui, p, "Aplicar elecciones").clicked();
                cancelar = ui.button("Cancelar importación").clicked();
            });
        });
        if cancelar || !abierto {
            self.conciliacion = None;
        } else if aplicar {
            let d = self.conciliacion.take().unwrap();
            let respuestas = d
                .conflictos
                .iter()
                .zip(&d.conservar)
                .map(|(c, conservar)| (c.id.clone(), if *conservar { crate::conciliar::Respuesta::Conservar } else { crate::conciliar::Respuesta::Reiniciar }))
                .collect();
            self.conciliar_con_anterior(d.nuevo, respuestas);
        }
    }

    fn activar_trabajo(&mut self) {
        let w = self.work.as_ref().unwrap();
        let n = w.registros.len();
        let ms = w.analisis_ms;
        let modo = w.mode().label();
        if w.registros.iter().all(|r| !r.seleccionada) {
            self.solo_marcadas = false;
        }
        for aviso in w.avisos.clone() {
            self.aviso(aviso, false);
        }
        self.sel = (n > 0).then_some(0);
        self.borradores.clear();
        self.bsel = None;
        self.proyectos.clear();
        self.psel = None;
        self.vista = Vista::Trabajo;
        self.revision_bitacoras = None;
        self.revision += 1;
        self.aviso(format!("{n} registros de {modo} analizados en {ms} ms."), false);
    }

    fn guardar_pendiente(&mut self) -> bool {
        if self.sucio.is_some() && self.screenshot.is_none() {
            if let Some(w) = &self.work {
                if let Err(e) = w.guardar_sesion() {
                    self.aviso(format!("No se pudo guardar la sesión: {e}"), true);
                    return false;
                }
            }
            self.sucio = None;
        }
        true
    }

    fn elegir_etapa(&mut self, elegida: Option<Mode>) {
        if let Some(w) = self.work.as_ref().filter(|w| w.paquete.is_some()) {
            let modo = elegida.unwrap_or(w.modos_posibles[0]);
            if w.mode() != modo && w.modos_posibles.contains(&modo) {
                if self.guardar_pendiente() {
                    let corte = self.corte();
                    self.work.as_mut().unwrap().cambiar_etapa_paquete(modo, corte, &self.cfg);
                    self.pedida = elegida;
                    self.activar_trabajo();
                }
                return;
            }
        }
        self.pedida = elegida;
        if elegida.is_some() {
            self.reanalizar();
        }
    }

    /// Solo para revisar pantallas con --screenshot: abre otra vista ya preparada con el ejemplo.
    pub fn vista_inicial(&mut self, nombre: &str) {
        match nombre {
            "correos" => {
                self.preparar_correos();
                self.vista = Vista::Correos;
            }
            "resoluciones" => {
                if let Some(w) = &self.work {
                    self.proyectos = productos::preparar_proyectos(w, &self.cfg, false);
                    self.psel = if self.proyectos.is_empty() { None } else { Some(0) };
                }
                self.vista = Vista::Resoluciones;
            }
            "cumplimiento" => self.cargar_ejemplo(Mode::Cumplimiento),
            "informes" => self.cargar_ejemplo(Mode::Informes),
            "trabajo-contexto" if self.screenshot.is_some() => {
                let ruta = std::env::var_os("FLOW_E3_PAQUETE").expect("paquete ficticio de contexto");
                self.corte_txt = "19/10/2026".into();
                self.abrir(PathBuf::from(ruta));
                assert!(self.work.as_ref().is_some_and(|w| w.paquete.is_some()), "No abrió el ZIP ficticio de contexto.");
            }
            "contactos" => self.vista = Vista::Contactos,
            "anonimizador" => self.vista = Vista::Anonimizador,
            "descargador" => {
                self.descargador.ejemplo();
                self.vista = Vista::Descargador;
            }
            "ajustes" => self.vista = Vista::Ajustes,
            "oscuro" => self.cfg.tema_oscuro = true,
            _ => {}
        }
        self.avisos.clear();
    }

    /// Archivo pasado al abrir el programa (por ejemplo, «Abrir con…» desde el Explorador).
    pub fn abrir_externo(&mut self, p: PathBuf) {
        if self.screenshot.is_none() {
            self.abrir(p);
        }
    }

    fn cargar_ejemplo(&mut self, m: Mode) {
        match crate::demo::libro(m, self.corte()) {
            Ok(p) => {
                let prev = self.pedida;
                self.pedida = Some(m);
                self.abrir(p);
                self.pedida = prev;
            }
            Err(e) => self.aviso(e, true),
        }
    }

    fn reanalizar(&mut self) {
        let corte = self.corte();
        self.revision_bitacoras = None;
        if let Some(w) = &mut self.work {
            w.corte = Some(corte);
            if let Some(m) = self.pedida {
                if w.paquete.is_none() && w.modos_posibles.contains(&m) {
                    w.mode = Some(m);
                }
            }
            w.analizar(&self.cfg);
            self.revision += 1;
            let ms = w.analisis_ms;
            let n = w.registros.len();
            self.aviso(format!("Reglas aplicadas de nuevo a {n} registros ({ms} ms). Tus ediciones se conservan."), false);
        }
    }

    fn guardar_excel(&mut self) {
        if self.work.is_none() {
            self.aviso("Primero abre un Excel de RUS o un paquete CSMP.", true);
            return;
        }
        let Some(dir) = self.carpeta_productos() else { return };
        let w = self.work.as_ref().unwrap();
        // Como Flow Python: copia del Excel original (o del consolidado del paquete) con las columnas de revisión.
        // Sin libro que preservar, una tabla nueva de fidelidad reducida, con el aviso visible.
        let hoja = crate::copia_excel::base(w).map(|b| b.hoja).unwrap_or_else(|_| w.mode().label().to_uppercase());
        let r = match crate::copia_excel::base(w) {
            Ok(_) => crate::copia_excel::exportar(w, &dir).map(|p| (p, None)),
            Err(aviso) => crate::producto_excel::exportar(w, &dir, Some(&aviso)).map(|p| (p, Some(aviso))),
        };
        match r {
            Ok((p, aviso)) => {
                let texto = format!("Excel revisable guardado: {}", p.display());
                self.aviso(aviso.map_or(texto.clone(), |a| format!("{texto} · {a}")), false);
                if let Some(w) = self.work.as_mut() {
                    crate::sincronizar::recordar_exportacion(w, &hoja);
                    w.copia = Some(p.clone());
                }
                self.registrar(p, "Excel de observaciones");
            }
            Err(e) => self.aviso(e, true),
        }
    }

    /// «Actualizar desde Excel»: trae a Trabajo la revisión hecha en la última copia guardada.
    fn actualizar_desde_excel(&mut self, previo: Option<(Vec<String>, Vec<Vec<Cell>>)>, resoluciones: crate::sincronizar::Resoluciones) {
        let Some(w) = self.work.as_mut() else { return };
        let (headers, filas) = match previo {
            Some(datos) => datos,
            None => {
                let ruta = match w.copia.clone().filter(|r| r.is_file()) {
                    Some(r) => r,
                    None => match rfd::FileDialog::new().set_title("Copia de Excel de este trabajo").add_filter("Excel", &["xlsx", "xlsm"]).pick_file() {
                        Some(r) => r,
                        None => return,
                    },
                };
                let leida = std::fs::read(&ruta).map_err(|e| format!("No se pudo leer la copia: {e}")).and_then(|b| crate::sincronizar::leer_copia(w, &b));
                match leida {
                    Ok(datos) => {
                        w.copia = Some(ruta);
                        datos
                    }
                    Err(e) => {
                        self.aviso(e, true);
                        return;
                    }
                }
            }
        };
        match crate::sincronizar::actualizar(w, &headers, &filas, &resoluciones) {
            Ok(resumen) => {
                let mut texto = format!("Copia de Excel incorporada: {} filas cambiaron.", resumen.filas_cambiadas);
                for a in &resumen.avisos {
                    texto.push_str(&format!(" {a}"));
                }
                self.sincronia = None;
                self.revision += 1;
                self.sucio = Some(Instant::now());
                self.aviso(texto, false);
            }
            Err(crate::sincronizar::Error::Conflictos(conflictos)) => {
                let usar_excel = vec![false; conflictos.len()];
                self.sincronia = Some(DialogoSincronia { conflictos, usar_excel, headers, filas });
            }
            Err(e) => self.aviso(e.mensaje(), true),
        }
    }

    fn dialogo_sincronia(&mut self, ctx: &egui::Context, p: &Pal) {
        let Some(d) = self.sincronia.as_mut() else { return };
        let mut abierto = true;
        let (mut aplicar, mut cancelar) = (false, false);
        egui::Window::new("Cambios en Trabajo y en Excel").open(&mut abierto).default_width(640.0).show(ctx, |ui| {
            ui.label(format!("{} campos se editaron aquí y en Excel. Elige cuál conservar; no se aplicó nada todavía.", d.conflictos.len()));
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                for (c, usar) in d.conflictos.iter().zip(&mut d.usar_excel) {
                    ui.separator();
                    ui.label(RichText::new(format!("Fila {} · {}", c.fila + 1, c.campo)).strong());
                    ui.horizontal_wrapped(|ui| {
                        ui.radio_value(usar, false, format!("Aquí: {}", c.local));
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.radio_value(usar, true, format!("Excel: {}", c.entrante));
                    });
                    ui.label(RichText::new(format!("Antes: {}", c.base)).color(p.muted).size(12.0));
                }
            });
            ui.horizontal(|ui| {
                aplicar = theme::primary(ui, p, "Aplicar elecciones").clicked();
                cancelar = ui.button("Cancelar").clicked();
            });
        });
        if cancelar || !abierto {
            self.sincronia = None;
        } else if aplicar {
            let d = self.sincronia.take().unwrap();
            let resoluciones = d
                .conflictos
                .iter()
                .zip(&d.usar_excel)
                .map(|(c, usar)| ((c.fila, c.campo.clone()), if *usar { c.entrante.clone() } else { c.local.clone() }))
                .collect();
            self.actualizar_desde_excel(Some((d.headers, d.filas)), resoluciones);
        }
    }

    // ── frame ────────────────────────────────────────────────────────────────

    pub fn frame(&mut self, ctx: &egui::Context) {
        if self.ultimo_tema != Some(self.cfg.tema_oscuro) {
            theme::aplicar(ctx, self.cfg.tema_oscuro);
            self.ultimo_tema = Some(self.cfg.tema_oscuro);
        }
        // Atajos
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::O))) {
            if self.vista == Vista::Descargador {
                self.descargador.abrir_dialogo();
            } else {
                self.abrir_dialogo();
            }
        }
        if self.vista != Vista::Descargador && ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::S))) {
            self.guardar_excel();
        }
        // Archivos arrastrados a la ventana
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            if self.vista == Vista::Anonimizador {
                self.anon_files.extend(dropped);
            } else if self.vista == Vista::Descargador {
                self.descargador.cargar(&dropped[0]);
            } else {
                self.abrir(dropped[0].clone());
            }
        }
        // Guardado automático de decisiones
        if let Some(t) = self.sucio {
            if t.elapsed().as_millis() > 800 && self.screenshot.is_none() {
                if let Some(w) = &self.work {
                    if let Err(e) = w.guardar_sesion() {
                        self.aviso(format!("No se pudo guardar la sesión: {e}"), true);
                    }
                }
                self.sucio = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(300));
            }
        }
        let p = self.p();
        self.nav(ctx, &p);
        if self.vista != Vista::Descargador {
            self.barra(ctx, &p);
        }
        self.avisos_panel(ctx, &p);
        self.dialogo_sincronia(ctx, &p);
        self.dialogo_conciliacion(ctx, &p);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(egui::Margin::symmetric(18, 12)))
            .show(ctx, |ui| match self.vista {
                Vista::Trabajo => self.v_trabajo(ui, &p),
                Vista::Correos => self.v_correos(ui, &p),
                Vista::Resoluciones => self.v_resoluciones(ui, &p),
                Vista::Resultados => self.v_resultados(ui, &p),
                Vista::Contactos => self.v_contactos(ui, &p),
                Vista::Anonimizador => self.v_anon(ui, &p),
                Vista::Descargador => self.descargador.mostrar(ui, &p),
                Vista::Ajustes => self.v_ajustes(ui, &p),
            });
        if let Some(paquete) = self.descargador.paquete_para_trabajo() {
            self.abrir(paquete);
        }
        if self.confirmar_reinicio {
            self.dialogo_reinicio(ctx, &p);
        }
    }

    fn nav(&mut self, ctx: &egui::Context, p: &Pal) {
        egui::SidePanel::left("nav")
            .exact_width(196.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.nav).inner_margin(egui::Margin::symmetric(12, 16)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 8.0, p.accent);
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "F", egui::FontId::proportional(18.0), Color32::WHITE);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Flow").size(18.0).strong().color(Color32::WHITE));
                        ui.label(RichText::new("Asistente CSMP").size(11.5).color(p.nav_text.gamma_multiply(0.7)));
                    });
                });
                ui.add_space(18.0);
                for (v, icon, label) in VISTAS {
                    if v == Vista::Anonimizador {
                        ui.add_space(10.0);
                        ui.label(RichText::new("HERRAMIENTAS").size(10.5).color(p.nav_text.gamma_multiply(0.5)));
                    }
                    let activo = self.vista == v;
                    let fill = if activo { p.accent } else { Color32::TRANSPARENT };
                    let txt_color = if activo { Color32::WHITE } else { p.nav_text };
                    let badge = match v {
                        Vista::Correos if !self.borradores.is_empty() => format!("{}", self.borradores.len()),
                        Vista::Resoluciones if !self.proyectos.is_empty() => format!("{}", self.proyectos.len()),
                        Vista::Resultados if !self.productos.is_empty() => format!("{}", self.productos.len()),
                        _ => String::new(),
                    };
                    let r = egui::Frame::new().fill(fill).corner_radius(8).inner_margin(egui::Margin::symmetric(10, 7)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(icon).size(15.0).color(txt_color));
                            ui.label(RichText::new(label).size(14.0).color(txt_color));
                            if !badge.is_empty() {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(RichText::new(badge).size(11.5).color(if activo { Color32::WHITE } else { p.accent }).strong());
                                });
                            }
                        });
                    });
                    let resp = ui.interact(r.response.rect, egui::Id::new(("nav", label)), egui::Sense::click());
                    if resp.hovered() && !activo {
                        ui.painter().rect_stroke(r.response.rect, 8.0, egui::Stroke::new(1.0_f32, p.nav_text.gamma_multiply(0.25)), egui::StrokeKind::Inside);
                    }
                    if resp.clicked() {
                        self.vista = v;
                    }
                    ui.add_space(2.0);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.label(RichText::new("No envía correos ni escribe en RUS.\nSolo borradores, con CC fijo.").size(11.0).color(p.nav_text.gamma_multiply(0.55)));
                    ui.add_space(6.0);
                    let t = if self.cfg.tema_oscuro { "☀  Tema claro" } else { "🌙  Tema oscuro" };
                    if ui.add(egui::Button::new(RichText::new(t).color(p.nav_text)).fill(Color32::TRANSPARENT)).clicked() {
                        self.cfg.tema_oscuro = !self.cfg.tema_oscuro;
                        self.guardar_cfg();
                    }
                });
            });
    }

    fn barra(&mut self, ctx: &egui::Context, p: &Pal) {
        egui::TopBottomPanel::top("barra")
            .frame(egui::Frame::new().fill(p.panel).inner_margin(egui::Margin::symmetric(18, 10)).stroke(egui::Stroke::new(1.0_f32, p.border)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if theme::primary(ui, p, "📂  Abrir Excel o ZIP…").on_hover_text("Ctrl+O · Excel de RUS o paquete CSMP · también puedes arrastrar el archivo").clicked() {
                        self.abrir_dialogo();
                    }
                    if ui.button("💾  Guardar observaciones").on_hover_text("Ctrl+S · copia del Excel original con la observación de cada fila (requiere Excel)").clicked() {
                        self.guardar_excel();
                    }
                    ui.separator();
                    ui.label(RichText::new("Corte").color(p.muted));
                    let r = ui.add(egui::TextEdit::singleline(&mut self.corte_txt).desired_width(88.0).hint_text("dd/mm/aaaa"));
                    if r.lost_focus() && parse_date_text(&self.corte_txt).is_some() {
                        self.reanalizar();
                    }
                    ui.label(RichText::new("Etapa").color(p.muted));
                    let actual = self.work.as_ref().map(|w| w.mode()).or(self.pedida);
                    let mut elegida = actual;
                    egui::ComboBox::from_id_salt("etapa")
                        .selected_text(actual.map(|m| m.label()).unwrap_or("Automática"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut elegida, None, "Automática");
                            for m in Mode::ALL {
                                let ok = self.work.as_ref().map_or(true, |w| w.modos_posibles.contains(&m));
                                ui.add_enabled_ui(ok, |ui| {
                                    ui.selectable_value(&mut elegida, Some(m), m.label());
                                });
                            }
                        });
                    if elegida != actual {
                        self.elegir_etapa(elegida);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(w) = &self.work {
                            ui.label(RichText::new(format!("{} · {} filas", w.nombre, w.registros.len())).color(p.muted).size(12.5));
                            ui.label(RichText::new("📄").color(p.muted));
                        } else {
                            ui.label(RichText::new("Sin archivo abierto").color(p.muted).size(12.5));
                        }
                    });
                });
            });
    }

    fn avisos_panel(&mut self, ctx: &egui::Context, p: &Pal) {
        self.avisos.retain(|(_, e, t)| t.elapsed().as_secs() < if *e { 14 } else { 7 });
        if self.avisos.is_empty() {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        let mut cerrar = None;
        egui::Area::new(egui::Id::new("avisos"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .show(ctx, |ui| {
                for (i, (t, e, _)) in self.avisos.iter().enumerate().rev().take(4) {
                    let c = if *e { p.red } else { p.accent };
                    egui::Frame::new()
                        .fill(p.panel)
                        .stroke(egui::Stroke::new(1.0_f32, c))
                        .corner_radius(8)
                        .inner_margin(10)
                        .shadow(egui::Shadow { offset: [0, 3], blur: 12, spread: 0, color: Color32::from_black_alpha(40) })
                        .show(ui, |ui| {
                            ui.set_max_width(420.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(if *e { "⚠" } else { "ℹ" }).color(c));
                                ui.label(RichText::new(t).color(p.text).size(13.0));
                                if ui.small_button("✕").clicked() {
                                    cerrar = Some(i);
                                }
                            });
                        });
                    ui.add_space(6.0);
                }
            });
        if let Some(i) = cerrar {
            self.avisos.remove(i);
        }
    }

    // ── Trabajo ──────────────────────────────────────────────────────────────

    fn color_estado(p: &Pal, e: Estado) -> Color32 {
        match e {
            Estado::Propuesta => p.accent,
            Estado::Atencion => p.amber,
            Estado::Revisada => p.green,
            Estado::Excluido => p.gray,
            Estado::SinObservacion => p.muted,
        }
    }

    fn filtrados(&mut self) -> Vec<usize> {
        let Some(w) = &self.work else { return vec![] };
        let key = format!("{}|{:?}|{}|{}|{}", self.filtro, self.filtro_estado, self.filtro_trib, self.solo_marcadas, self.revision);
        if key == self.cache_key {
            return self.cache.clone();
        }
        let q = clave(&self.filtro);
        let out: Vec<usize> = w
            .registros
            .iter()
            .enumerate()
            .filter(|(_, r)| self.filtro_estado.map_or(true, |e| r.estado() == e))
            .filter(|(_, r)| self.filtro_trib.is_empty() || w.tribunal_clave(r) == self.filtro_trib)
            .filter(|(_, r)| !self.solo_marcadas || r.seleccionada)
            .filter(|(_, r)| {
                q.is_empty()
                    || ["rit", "nombre", "programa", "rut"].iter().any(|k| clave(&w.value(r, k)).contains(&q))
                    || clave(r.observacion()).contains(&q)
            })
            .map(|(i, _)| i)
            .collect();
        self.cache_key = key;
        self.cache = out.clone();
        out
    }

    fn vacio(&mut self, ui: &mut egui::Ui, p: &Pal) {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            theme::card(p).show(ui, |ui| {
                ui.set_max_width(620.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("🗂").size(42.0).color(p.accent));
                    ui.label(RichText::new("Abre un Excel de RUS o un paquete CSMP").size(22.0).strong());
                    ui.label(RichText::new("Puedes abrir los .xls de SITFA (también HTML o XML), .xlsx, .xlsm y paquetes ZIP del Descargador. Flow propone observaciones con las reglas de Laja, Mulchén y Tomé.").color(p.muted));
                    ui.add_space(12.0);
                    if theme::primary(ui, p, "📂  Abrir Excel o ZIP…").clicked() {
                        self.abrir_dialogo();
                    }
                    ui.add_space(14.0);
                    ui.label(RichText::new("¿Solo quieres probar? Usa un libro de ejemplo con datos ficticios:").color(p.muted).size(12.5));
                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() / 2.0 - 170.0).max(0.0));
                        for m in Mode::ALL {
                            if ui.button(format!("Ejemplo {}", m.label())).clicked() {
                                self.cargar_ejemplo(m);
                            }
                        }
                    });
                });
            });
        });
    }

    fn seleccionar_desde_bitacoras(&mut self, filas: &[usize]) -> bool {
        let Some(w) = &mut self.work else { return false; };
        let anteriores: Vec<_> = w.registros.iter().map(|r| r.seleccionada).collect();
        let seleccionadas = w.reemplazar_seleccion(filas);
        let Some(&primera) = seleccionadas.first() else { return false; };
        if self.screenshot.is_none() {
            if let Err(error) = w.guardar_sesion() {
                for (r, marcada) in w.registros.iter_mut().zip(anteriores) {
                    r.seleccionada = marcada;
                }
                self.aviso(format!("No se pudo guardar la selección: {error}"), true);
                return false;
            }
            self.sucio = None;
        }
        self.sel = Some(primera);
        self.filtro.clear();
        self.filtro_estado = None;
        self.filtro_trib.clear();
        self.solo_marcadas = true;
        self.revision += 1;
        self.aviso(format!("{} registros seleccionados en Trabajo.", seleccionadas.len()), false);
        true
    }

    fn v_trabajo(&mut self, ui: &mut egui::Ui, p: &Pal) {
        if self.work.is_none() {
            self.vacio(ui, p);
            return;
        }
        let dark = self.cfg.tema_oscuro;
        let (conteos, total, ms, modo, cruce) = {
            let w = self.work.as_ref().unwrap();
            (w.conteos(), w.registros.len(), w.analisis_ms, w.mode(), w.cruce.clone())
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Trabajo · {}", modo.label())).size(22.0).strong());
            ui.label(RichText::new(format!("análisis en {ms} ms{}", cruce.map(|c| format!(" · cruce con «{c}»")).unwrap_or_default())).color(p.muted).size(12.5));
        });
        let estado = self.work.as_ref().unwrap().paquete.as_ref()
            .map(|p| p.fuente.estado("registros").to_owned());
        if let Some(estado) = estado {
            let abrir = ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!(
                    "Paquete CSMP · registros {estado} · análisis tabular"
                )).color(p.muted).size(12.5));
                ui.button("Revisión de bitácoras…").clicked()
            }).inner;
            if abrir {
                let w = self.work.as_ref().unwrap();
                self.revision_bitacoras = Some(crate::revision_ui::Dialogo::nuevo(w, self.corte()));
            }
        }
        if self.work.as_ref().is_some_and(|w| w.copia.is_some()) && ui.button("Actualizar desde Excel").on_hover_text("Trae a Trabajo la revisión hecha en la última copia de Excel guardada: observación, FECHA_OBS, TT, CC y RES").clicked() {
            self.actualizar_desde_excel(None, Default::default());
        }
        if let Some(c) = self.work.as_ref().and_then(|w| w.cambios.as_ref()) {
            let m = c.resumen();
            let egresados: Vec<String> = c.egresados.iter().map(|e| format!("{} · {} · {}", e.rit, e.nombre, e.programa)).collect();
            egui::CollapsingHeader::new(format!(
                "Frente a la captura anterior · {} nuevas · {} con fuente cambiada · {} con reglas cambiadas · {} egresadas",
                m["Nuevas"], m["Fuente"], m["Reglas"], m["Egresados"]
            ))
            .id_salt("cambios de captura")
            .show(ui, |ui| {
                for e in &egresados {
                    ui.label(RichText::new(e).color(p.muted).size(12.5));
                }
            });
        }
        let avisos = self.work.as_ref().and_then(|w| w.paquete.as_ref()).map(|p| p.avisos_agrupados()).unwrap_or_default();
        if !avisos.is_empty() {
            let total: usize = avisos.iter().map(|a| a.incidencias).sum();
            egui::CollapsingHeader::new(format!("Avisos del paquete · {} tipos · {total} incidencias", avisos.len()))
                .id_salt("avisos del paquete")
                .show(ui, |ui| {
                    for a in &avisos {
                        let donde = if a.consultas == 1 { "1 consulta".to_string() } else { format!("{} consultas", a.consultas) };
                        ui.label(RichText::new(format!("{} · {} · {} en {donde}", a.codigo, a.severidad, a.incidencias)).strong());
                        ui.label(RichText::new(&a.mensaje).color(p.muted).size(12.5));
                    }
                });
        }
        if let (Some(dialogo), Some(w)) = (&mut self.revision_bitacoras, &self.work) {
            let (abierto, accion, productos) = dialogo.mostrar(ui.ctx(), p, w);
            let accion_aplicada = match accion {
                Some(crate::revision_ui::Accion::AbrirFila(i)) => {
                    self.sel = Some(i);
                    true
                }
                Some(crate::revision_ui::Accion::SeleccionarEnTrabajo(filas)) => {
                    self.seleccionar_desde_bitacoras(&filas)
                }
                None => false,
            };
            if !abierto || accion_aplicada {
                self.revision_bitacoras = None;
            }
            if let Some([excel, json]) = productos {
                self.registrar(excel, "Revisión de bitácoras · Excel");
                self.registrar(json, "Revisión de bitácoras · JSON");
            }
        }
        ui.add_space(4.0);
        let mut nuevo_filtro: Option<Option<Estado>> = None;
        ui.horizontal(|ui| {
            let filtro_actual = self.filtro_estado;
            let mut tarjeta = |ui: &mut egui::Ui, label: &str, n: usize, color: Color32, estado: Option<Estado>| {
                let activo = filtro_actual == estado;
                let mut fr = theme::card(p).inner_margin(egui::Margin::symmetric(14, 10));
                if activo && estado.is_some() {
                    fr = fr.stroke(egui::Stroke::new(1.5_f32, color));
                }
                let r = fr.show(ui, |ui| {
                    ui.set_min_width(112.0);
                    ui.label(RichText::new(n.to_string()).size(24.0).strong().color(color));
                    ui.label(RichText::new(label).size(12.5).color(p.muted));
                });
                if ui.interact(r.response.rect, egui::Id::new(("card", label)), egui::Sense::click()).clicked() {
                    nuevo_filtro = Some(if activo { None } else { estado });
                }
            };
            tarjeta(ui, "Registros", total, p.text, None);
            tarjeta(ui, "Propuestas", *conteos.get("Propuesta").unwrap_or(&0), p.accent, Some(Estado::Propuesta));
            tarjeta(ui, "Para revisar", *conteos.get("Revisar").unwrap_or(&0), p.amber, Some(Estado::Atencion));
            tarjeta(ui, "Revisadas", *conteos.get("Revisada").unwrap_or(&0), p.green, Some(Estado::Revisada));
            tarjeta(ui, "Excluidos", *conteos.get("Excluido").unwrap_or(&0), p.gray, Some(Estado::Excluido));
            tarjeta(ui, "Sin observación", *conteos.get("Sin observación").unwrap_or(&0), p.muted, Some(Estado::SinObservacion));
        });
        if let Some(f) = nuevo_filtro {
            self.filtro_estado = f;
        }
        ui.add_space(6.0);
        if self.trib_cache.0 != self.revision {
            let w = self.work.as_ref().unwrap();
            let mut t: Vec<String> = w.registros.iter().map(|r| w.tribunal_clave(r)).collect();
            t.sort();
            t.dedup();
            self.trib_cache = (self.revision, t);
        }
        let tribunales = self.trib_cache.1.clone();
        let mut marcar_visibles = false;
        let mut marcar_seleccion: Option<bool> = None;
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.filtro).hint_text("🔍  Buscar RIT, nombre, RUT, programa u observación").desired_width(380.0));
            egui::ComboBox::from_id_salt("ftrib")
                .selected_text(if self.filtro_trib.is_empty() { "Todos los tribunales".to_string() } else { self.filtro_trib.clone() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filtro_trib, String::new(), "Todos los tribunales");
                    for t in &tribunales {
                        ui.selectable_value(&mut self.filtro_trib, t.clone(), t);
                    }
                });
            ui.checkbox(&mut self.solo_marcadas, "Solo filas marcadas");
            if ui.button("☑ Marcar visibles").on_hover_text("Marca todas las filas que muestra el filtro actual").clicked() {
                marcar_seleccion = Some(true);
            }
            if ui.button("☐ Desmarcar todas").clicked() {
                marcar_seleccion = Some(false);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("↻  Volver a analizar…").on_hover_text("Descarta ediciones y marcas y aplica las reglas desde cero").clicked() {
                    self.confirmar_reinicio = true;
                }
                if ui.button("✔  Marcar visibles como revisadas").clicked() {
                    marcar_visibles = true;
                }
            });
        });
        if let Some(marcar) = marcar_seleccion {
            let visibles = if marcar { self.filtrados() } else { Vec::new() };
            if let Some(w) = &mut self.work {
                if marcar {
                    for i in visibles {
                        w.registros[i].seleccionada = true;
                    }
                } else {
                    for r in &mut w.registros {
                        r.seleccionada = false;
                    }
                }
            }
            self.ultima_marca = None;
            self.marcar_sucio();
        }
        if marcar_visibles {
            let ids = self.filtrados();
            if let Some(w) = &mut self.work {
                for i in ids {
                    if !w.registros[i].excluido {
                        w.registros[i].revisada = true;
                    }
                }
            }
            self.marcar_sucio();
        }
        ui.add_space(4.0);
        let ids = self.filtrados();
        // Navegación con teclado
        if !ui.ctx().wants_keyboard_input() && !ids.is_empty() {
            let pos = self.sel.and_then(|s| ids.iter().position(|i| *i == s));
            if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                self.sel = Some(ids[pos.map_or(0, |p| (p + 1).min(ids.len() - 1))]);
            }
            if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                self.sel = Some(ids[pos.map_or(0, |p| p.saturating_sub(1))]);
            }
            if ui.input(|i| i.key_pressed(Key::Space)) {
                if let (Some(s), Some(w)) = (self.sel, &mut self.work) {
                    w.registros[s].revisada = !w.registros[s].revisada;
                }
                self.marcar_sucio();
            }
        }
        let ancho_det = (ui.available_width() * 0.34).clamp(330.0, 520.0);
        egui::SidePanel::right("panel_detalle")
            .resizable(true)
            .default_width(ancho_det)
            .min_width(320.0)
            .max_width(640.0)
            .show_separator_line(false)
            .frame(egui::Frame::new().inner_margin(egui::Margin { left: 12, right: 0, top: 0, bottom: 0 }))
            .show_inside(ui, |ui| {
                self.detalle(ui, p, dark);
            });
        egui::CentralPanel::default().frame(egui::Frame::new()).show_inside(ui, |ui| {
            let alto = ui.available_height();
            theme::card(p).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                ui.set_min_height(alto - 14.0);
                self.tabla(ui, p, &ids, dark);
            });
        });
    }

    fn tabla(&mut self, ui: &mut egui::Ui, p: &Pal, ids: &[usize], dark: bool) {
        let modo = self.work.as_ref().map(|w| w.mode()).unwrap_or(Mode::Espera);
        let (metric_key, metric_label) = match modo {
            Mode::Espera => ("espera", "T espera"),
            Mode::Cumplimiento => ("egreso_proy", "Egreso proy."),
            Mode::Informes => ("vencimiento", "Vencimiento"),
        };
        let mut nueva_sel = None;
        let mut cambio = false;
        let mayus = ui.input(|i| i.modifiers.shift);
        let mut ultima = self.ultima_marca;
        let mut rango: Option<(usize, usize, bool)> = None;
        let sel = self.sel;
        let w = self.work.as_mut().unwrap();
        // Anchos proporcionales al espacio: la tabla nunca se sale de su tarjeta.
        let libre = (ui.available_width() - 26.0 - 104.0 - 74.0 - 80.0 - 70.0).max(300.0);
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .auto_shrink(false)
            .sense(egui::Sense::click())
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(26.0))
            .column(Column::exact(104.0))
            .column(Column::initial((libre * 0.17).max(70.0)).at_least(60.0).clip(true))
            .column(Column::initial((libre * 0.25).max(90.0)).at_least(70.0).clip(true))
            .column(Column::initial((libre * 0.20).max(80.0)).at_least(60.0).clip(true))
            .column(Column::initial(74.0).at_least(50.0).clip(true))
            .column(Column::initial(80.0).at_least(50.0).clip(true))
            .column(Column::remainder().at_least(120.0).clip(true))
            .header(26.0, |mut h| {
                for t in ["", "Estado", "RIT", "Nombre", "Programa", "Tribunal", metric_label, "Observación propuesta"] {
                    h.col(|ui| {
                        ui.label(RichText::new(t).size(12.5).strong().color(p.muted));
                    });
                }
            })
            .body(|body| {
                body.rows(30.0, ids.len(), |mut row| {
                    let i = ids[row.index()];
                    let pos = row.index();
                    row.set_selected(sel == Some(i));
                    let estado = w.registros[i].estado();
                    let tribunal = w.tribunal_clave(&w.registros[i]);
                    let rit = w.value(&w.registros[i], "rit");
                    let nombre = limpiar_nombre(&w.value(&w.registros[i], "nombre"));
                    let prog = w.value(&w.registros[i], "programa");
                    let metric = w.value(&w.registros[i], metric_key);
                    let obs: String = w.registros[i].observacion().chars().take(220).collect();
                    let editada = w.registros[i].editada.is_some();
                    let n_acc = w.registros[i].acciones.len();
                    row.col(|ui| {
                        let mut m = w.registros[i].seleccionada;
                        if ui.checkbox(&mut m, "").on_hover_text("Mayús+clic marca o desmarca el tramo desde la última casilla").changed() {
                            w.registros[i].seleccionada = m;
                            if let Some(previa) = ultima.filter(|_| mayus) {
                                rango = Some((previa.min(pos), previa.max(pos), m));
                            }
                            ultima = Some(pos);
                            cambio = true;
                        }
                    });
                    row.col(|ui| theme::chip(ui, estado.label(), App::color_estado(p, estado), dark));
                    row.col(|ui| {
                        ui.label(RichText::new(rit).monospace().size(12.5));
                    });
                    row.col(|ui| {
                        ui.label(nombre);
                    });
                    row.col(|ui| {
                        ui.label(RichText::new(prog).color(p.muted));
                    });
                    row.col(|ui| {
                        ui.label(RichText::new(tribunal).size(12.5));
                    });
                    row.col(|ui| {
                        ui.label(metric);
                    });
                    row.col(|ui| {
                        if editada {
                            ui.label(RichText::new("✎").color(p.amber)).on_hover_text("Editada por ti");
                        }
                        if n_acc > 0 {
                            ui.label(RichText::new(format!("✉{n_acc}")).color(p.accent).size(12.0)).on_hover_text("Genera correos o proyectos");
                        }
                        ui.label(RichText::new(obs).size(12.5));
                    });
                    if row.response().clicked() {
                        nueva_sel = Some(i);
                    }
                });
            });
        if let Some(s) = nueva_sel {
            self.sel = Some(s);
        }
        self.ultima_marca = ultima;
        if let (Some((desde, hasta, marcar)), Some(w)) = (rango, self.work.as_mut()) {
            for &i in &ids[desde..=hasta.min(ids.len().saturating_sub(1))] {
                w.registros[i].seleccionada = marcar;
            }
        }
        if cambio {
            self.marcar_sucio();
        }
    }

    fn detalle(&mut self, ui: &mut egui::Ui, p: &Pal, dark: bool) {
        let Some(i) = self.sel else {
            theme::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Elige una fila para ver y editar su observación.").color(p.muted));
            });
            return;
        };
        let mostrar_cronologia = self.screenshot.is_some();
        let mut cambio = false;
        let mut consignacion: Option<(String, bool)> = None;
        egui::ScrollArea::vertical().id_salt("detalle").show(ui, |ui| {
            let w = self.work.as_mut().unwrap();
            if i >= w.registros.len() {
                return;
            }
            let rit = w.value(&w.registros[i], "rit");
            let nombre = limpiar_nombre(&w.value(&w.registros[i], "nombre"));
            let prog = w.value(&w.registros[i], "programa");
            let trib = w.value(&w.registros[i], "tribunal");
            let rut = w.value(&w.registros[i], "rut");
            let headers = w.headers.clone();
            let consignados = w.consignados(&w.registros[i]);
            theme::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let e = w.registros[i].estado();
                    theme::chip(ui, e.label(), App::color_estado(p, e), dark);
                    ui.label(RichText::new(format!("Fila {}", w.registros[i].fila_excel)).color(p.muted).size(12.0));
                });
                ui.label(RichText::new(if nombre.is_empty() { "(sin nombre)".to_string() } else { nombre }).size(18.0).strong());
                ui.label(RichText::new(format!("{rit}  ·  {rut}")).monospace().color(p.muted));
                if let Some(c) = w.cambios.as_ref().zip(w.id_registro(&w.registros[i])).and_then(|(c, id)| c.filas.get(&id)) {
                    if c.tipo != crate::conciliar::Tipo::SinCambio {
                        let campos = if c.campos.is_empty() { String::new() } else { format!(" · {}", c.campos.join(", ")) };
                        ui.label(RichText::new(format!("Frente a la captura anterior: {}{campos}", c.tipo.etiqueta())).color(p.amber).size(12.5));
                    }
                }
                ui.label(RichText::new(format!("{prog}  ·  {trib}")).color(p.muted));
                ui.add_space(8.0);
                theme::seccion(ui, p, "Observación");
                let r = &mut w.registros[i];
                let mut texto = r.observacion().to_string();
                let resp = ui.add(egui::TextEdit::multiline(&mut texto).desired_rows(7).desired_width(f32::INFINITY).font(egui::TextStyle::Body));
                if resp.changed() {
                    r.editada = if texto == r.propuesta { None } else { Some(texto) };
                    cambio = true;
                }
                let atajo = ui.input_mut(|inp| inp.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Enter)));
                ui.horizontal(|ui| {
                    let label = if r.revisada { "✔ Revisada" } else { "Marcar revisada" };
                    let b = if r.revisada {
                        ui.add(egui::Button::new(RichText::new(label).color(Color32::WHITE).strong()).fill(p.green))
                    } else {
                        theme::primary(ui, p, label)
                    };
                    if b.on_hover_text("Ctrl+Enter (o Espacio en la tabla)").clicked() || atajo {
                        r.revisada = !r.revisada;
                        cambio = true;
                    }
                    if r.editada.is_some() && ui.button("↺ Restaurar propuesta").clicked() {
                        r.editada = None;
                        cambio = true;
                    }
                    if ui.button("📋 Copiar").on_hover_text("Copia la observación para pegarla en la bitácora de RUS").clicked() {
                        ui.ctx().copy_text(r.observacion().to_string());
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("RES").color(p.muted));
                    egui::ComboBox::from_id_salt("res")
                        .selected_text(if r.res.is_empty() { "Automático".to_string() } else { r.res.clone() })
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(r.res.is_empty(), "Automático").clicked() {
                                r.res.clear();
                                cambio = true;
                            }
                            for (k, l) in TIPOS {
                                if ui.selectable_label(r.res == k, l).clicked() {
                                    r.res = k.into();
                                    cambio = true;
                                }
                            }
                        });
                    let mut m = r.seleccionada;
                    if ui.checkbox(&mut m, "Marcada").changed() {
                        r.seleccionada = m;
                        cambio = true;
                    }
                });
                if !r.acciones.is_empty() {
                    ui.add_space(4.0);
                    theme::seccion(ui, p, "Genera");
                    ui.horizontal_wrapped(|ui| {
                        for a in &r.acciones {
                            let t = match *a {
                                "programa_espera" => "✉ Lista de espera",
                                "programa_vencido" => "✉ Informe pendiente",
                                "programa_por_vencer" => "✉ Informe por vencer",
                                "medidas" => "✉ Nómina de medidas",
                                "PC_IE" => "⚖ PC_IE",
                                "PC_INFO" => "⚖ PC_INFO",
                                _ => "⚖ NOMENCL",
                            };
                            theme::chip(ui, t, p.accent, dark);
                        }
                    });
                }
                if !r.avisos.is_empty() || !r.marcas.is_empty() {
                    ui.add_space(4.0);
                    theme::seccion(ui, p, "Avisos");
                    for a in &r.avisos {
                        ui.label(RichText::new(format!("⚠ {a}")).color(p.amber).size(12.5));
                    }
                    for m in &r.marcas {
                        ui.label(RichText::new(format!("● Nivel {}: {}", m.nivel, m.texto)).color(if m.nivel >= 3 { p.red } else { p.amber }).size(12.5));
                    }
                }
                if !consignados.is_empty() || !r.otros_programas.is_empty() {
                    ui.add_space(4.0);
                    theme::seccion(ui, p, "Otro programa vigente");
                    for prog in &r.otros_programas {
                        if ui.button(format!("Marcar «{prog}» como consignada"))
                            .on_hover_text("Ya consta en la bitácora de RUS: la propuesta deja de repetir esta constancia")
                            .clicked()
                        {
                            consignacion = Some((prog.clone(), true));
                        }
                    }
                    for prog in &consignados {
                        if ui.button(format!("Deshacer consignación de «{prog}»")).clicked() {
                            consignacion = Some((prog.clone(), false));
                        }
                    }
                }
                if !r.eventos.is_empty() {
                    ui.add_space(4.0);
                    theme::seccion(ui, p, "Reglas aplicadas");
                    ui.label(RichText::new(r.eventos.join(" · ")).size(11.5).color(p.muted).monospace());
                }
            });
            let contexto = w.paquete.as_ref().and_then(|pq| {
                let id = headers.iter().position(|h| h == "ID_REGISTRO_CAPTURA").and_then(|c| w.registros[i].values.get(c))?.display();
                pq.contextos.get(&id).map(|contexto| (id, contexto.clone()))
            });
            if let Some((id, ctx)) = contexto {
                ui.add_space(8.0);
                theme::card(p).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    theme::seccion(ui, p, "Fuentes del paquete");
                    if let Some(e) = &ctx.escritos {
                        let estado = match e.cobertura.as_str() {
                            _ if e.discordante => "lecturas simultáneas discordantes".to_string(),
                            "VACIA_COMPROBADA" => "sin escritos".to_string(),
                            "FALLIDA" | "IDENTIDAD_INCIERTA" | "NO_CONSULTADA" => "no verificada".to_string(),
                            _ if e.pendientes.is_empty() => format!("sin pendientes ({} escritos)", e.filas),
                            _ => format!("{} pendiente(s) de {} escritos", e.pendientes.len(), e.filas),
                        };
                        let parcial = if e.cobertura == "PARCIAL" { " · lectura parcial" } else { "" };
                        let color = if e.pendientes.is_empty() && !e.discordante { p.muted } else { p.amber };
                        ui.label(RichText::new(format!("Escritos de la causa: {estado}{parcial} · leído {}", e.leido)).color(color));
                        for pend in &e.pendientes {
                            ui.label(RichText::new(format!("   • {pend}")).size(12.0));
                        }
                    }
                    if let Some(h) = &ctx.historico {
                        let identidad = if h.verificada { "" } else { " · identidad no verificada" };
                        let texto = if h.discordante {
                            format!("Ingreso histórico (Mejor Niñez) · lecturas simultáneas discordantes · {}", h.leido)
                        } else if !matches!(h.cobertura.as_str(), "COMPLETA" | "PARCIAL" | "VACIA_COMPROBADA" | "IDENTIDAD_INCIERTA") {
                            format!("Ingreso histórico (Mejor Niñez) · lectura no verificada · {}", h.leido)
                        } else if h.vigentes.is_empty() && h.cobertura != "PARCIAL" && h.verificada {
                            format!("Ingreso histórico (Mejor Niñez): sin programas vigentes ({} filas){identidad} · leído {}", h.filas, h.leido)
                        } else if h.vigentes.is_empty() {
                            format!("Ingreso histórico (Mejor Niñez) · sin programas vigentes en la lectura disponible{identidad} · leído {}", h.leido)
                        } else {
                            format!("Ingreso histórico (Mejor Niñez): vigentes {}{identidad} · leído {}", h.vigentes.join("; "), h.leido)
                        };
                        ui.label(RichText::new(texto).color(if h.verificada { p.text } else { p.amber }));
                    }
                    for (nombre, lectura) in [
                        ("Historia de la causa", ctx.historia.as_ref()),
                        ("Bitácora del ingreso", ctx.bitacora.as_ref()),
                        ("Calendario del ingreso", ctx.calendario.as_ref().map(|c| &c.lectura)),
                    ] {
                        if let Some(lectura) = lectura {
                            let (estado, color) = match lectura.cobertura.as_str() {
                                "COMPLETA" => (format!("{} registros", lectura.filas), p.text),
                                "VACIA_COMPROBADA" => ("sin registros".into(), p.muted),
                                "PARCIAL" => (format!("{} registros en lectura parcial", lectura.filas), p.amber),
                                _ => ("lectura no verificada".into(), p.amber),
                            };
                            ui.label(RichText::new(format!("{nombre} · {estado} · {}", lectura.leido)).color(color));
                        }
                    }
                    if let Some(primera) = ctx.calendario.as_ref().and_then(|c| c.pendientes.first()) {
                        ui.label(RichText::new(format!("Primera proyección pendiente · {}", primera.format("%d/%m/%Y"))).size(12.0));
                    }
                    if let Some(cronologia) = &ctx.cronologia {
                        egui::CollapsingHeader::new("Cronología de bitácora")
                            .id_salt(&id)
                            .default_open(mostrar_cronologia)
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!(
                                    "{} · {} a {}",
                                    cronologia.estado.etiqueta(),
                                    cronologia.desde.format("%d/%m/%Y"),
                                    cronologia.hasta.format("%d/%m/%Y"),
                                )).color(p.muted));
                                for (nombre, entradas) in [
                                    ("Última del centro", &cronologia.ultimas),
                                    ("Posteriores", &cronologia.posteriores),
                                    ("Orden incierto", &cronologia.orden_incierto),
                                    ("Origen del centro sin acreditar", &cronologia.sin_autoria),
                                ] {
                                    if entradas.is_empty() {
                                        continue;
                                    }
                                    egui::CollapsingHeader::new(format!("{nombre} ({})", entradas.len()))
                                        .id_salt(nombre)
                                        .default_open(nombre == "Última del centro")
                                        .show(ui, |ui| {
                                            for evento in entradas {
                                                ui.label(format!("{} · {}", evento.fecha.format("%d/%m/%Y"), evento.texto));
                                            }
                                        });
                                }
                                if let Some(fecha) = cronologia.sin_respuesta.last() {
                                    ui.label(RichText::new(format!(
                                        "{} observaciones al tribunal sin respuesta registrada · última {}",
                                        cronologia.sin_respuesta.len(), fecha.format("%d/%m/%Y")
                                    )).color(p.amber));
                                }
                            });
                    }
                });
            }
            ui.add_space(8.0);
            theme::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                theme::seccion(ui, p, "Datos de RUS");
                egui::Grid::new("datos").num_columns(2).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    for (h, v) in headers.iter().zip(&w.registros[i].values) {
                        let v = v.display();
                        if h.is_empty() && v.is_empty() {
                            continue;
                        }
                        ui.label(RichText::new(h).size(12.0).color(p.muted));
                        ui.label(RichText::new(v).size(12.5));
                        ui.end_row();
                    }
                });
            });
        });
        if let Some((programa, marcar)) = consignacion {
            let mensaje = self.work.as_mut().and_then(|w| w.consignar(i, &programa, marcar, &self.cfg));
            if let Some(m) = mensaje {
                self.aviso(m, false);
                cambio = true;
            }
        }
        if cambio {
            self.marcar_sucio();
        }
    }

    fn dialogo_reinicio(&mut self, ctx: &egui::Context, p: &Pal) {
        let mut hecho = false;
        let mut cancelar = false;
        egui::Window::new("Volver a analizar")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("Se descartarán tus ediciones, marcas de revisada y filas marcadas de este archivo, y se aplicarán las reglas desde cero.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Descartar y analizar").color(Color32::WHITE)).fill(p.red)).clicked() {
                        hecho = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        cancelar = true;
                    }
                });
            });
        if hecho {
            if let Some(w) = &mut self.work {
                w.reiniciar(&self.cfg);
            }
            self.confirmar_reinicio = false;
            self.marcar_sucio();
            self.aviso("Análisis rehecho desde cero.", false);
        }
        if cancelar {
            self.confirmar_reinicio = false;
        }
    }

    // ── Correos ──────────────────────────────────────────────────────────────

    fn v_correos(&mut self, ui: &mut egui::Ui, p: &Pal) {
        theme::titulo(ui, p, "Correos", "Borradores listos para revisar: en Borradores de Outlook o como .eml sin enviar. Nunca se envían. Todos llevan CC a ucc_concepcion@pjud.cl.");
        let mut accion: Option<&'static str> = None;
        let mut guardar_destino = false;
        theme::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                theme::seccion(ui, p, "Modalidades revisadas");
                for (j, (k, l)) in MODALIDADES.iter().enumerate() {
                    ui.checkbox(&mut self.modalidades[j], *k).on_hover_text(*l);
                }
                ui.separator();
                theme::seccion(ui, p, "Tribunales");
                for t in ["LAJA", "MULCHEN", "TOME"] {
                    let mut on = self.tribunales_sel.is_empty() || self.tribunales_sel.contains(t);
                    if ui.checkbox(&mut on, t).changed() {
                        if self.tribunales_sel.is_empty() {
                            self.tribunales_sel = ["LAJA", "MULCHEN", "TOME"].iter().map(|s| s.to_string()).collect();
                        }
                        if on {
                            self.tribunales_sel.insert(t.into());
                        } else {
                            self.tribunales_sel.remove(t);
                        }
                    }
                }
                ui.separator();
                ui.checkbox(&mut self.solo_sel_correo, "Solo filas marcadas");
                ui.separator();
                theme::seccion(ui, p, "Guardar en");
                let antes = self.cfg.destino_outlook;
                ui.radio_value(&mut self.cfg.destino_outlook, false, "Archivos .eml");
                ui.radio_value(&mut self.cfg.destino_outlook, true, "Borradores de Outlook")
                    .on_hover_text("Outlook clásico de Windows. Solo guarda: nunca envía.");
                if antes != self.cfg.destino_outlook {
                    guardar_destino = true;
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let hay = self.work.is_some();
                if ui.add_enabled(hay, egui::Button::new(RichText::new("✉  Preparar borradores").color(Color32::WHITE).strong()).fill(p.accent)).clicked() {
                    accion = Some("preparar");
                }
                if ui.add_enabled(hay, egui::Button::new("Gestión particular")).on_hover_text("Correo libre al tribunal con las filas marcadas en Trabajo").clicked() {
                    accion = Some("especial");
                }
                if ui.add_enabled(!self.proyectos.is_empty(), egui::Button::new("Reporte de proyectos")).on_hover_text("Necesita proyectos preparados en Resoluciones").clicked() {
                    accion = Some("proyectos");
                }
                if !self.borradores.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::primary(ui, p, "💾  Guardar borradores marcados").clicked() {
                            accion = Some("guardar");
                        }
                    });
                }
            });
            if !self.modalidades.iter().any(|x| *x) {
                ui.label(RichText::new("Marca al menos una modalidad para preparar borradores.").color(p.amber));
            }
        });
        if guardar_destino {
            self.guardar_cfg();
        }
        match accion {
            Some("preparar") => self.preparar_correos(),
            Some("guardar") => self.guardar_borradores(None),
            Some(k) => self.preparar_tipo(k),
            None => {}
        }
        ui.add_space(8.0);
        if self.borradores.is_empty() {
            ui.label(
                RichText::new(if self.work.is_none() {
                    "Abre un Excel de RUS en Trabajo y vuelve aquí."
                } else {
                    "Pulsa «Preparar borradores»: se agrupan por programa y tribunal, con su nómina adjunta."
                })
                .color(p.muted),
            );
            return;
        }
        let alto = ui.available_height();
        let mut guardar_uno = None;
        let mut guardar_nomina = None;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(egui::vec2(330.0, alto), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().id_salt("blist").show(ui, |ui| {
                    for (j, b) in self.borradores.iter_mut().enumerate() {
                        let activo = self.bsel == Some(j);
                        let mut fr = theme::card(p).inner_margin(egui::Margin::same(10));
                        if activo {
                            fr = fr.stroke(egui::Stroke::new(1.5_f32, p.accent));
                        }
                        // El clic de la tarjeta se registra debajo del contenido: el checkbox recibe su propio clic.
                        let r = ui.scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| fr.show(ui, |ui| {
                            ui.set_width(300.0);
                            ui.set_max_width(300.0);
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut b.incluir, "");
                                ui.add(egui::Label::new(RichText::new(&b.titulo).strong().size(13.0)).truncate()).on_hover_text(&b.titulo);
                            });
                            let falta = b.para.trim().is_empty();
                            let para = if falta { "⚠ Falta destinatario".to_string() } else { b.para.clone() };
                            ui.add(egui::Label::new(RichText::new(para).size(12.0).color(if falta { p.amber } else { p.muted })).truncate());
                            ui.horizontal(|ui| {
                                if let Some(n) = &b.nomina {
                                    ui.label(RichText::new(format!("📎 {} filas", n.incluir.iter().filter(|x| **x).count())).size(12.0).color(p.muted));
                                }
                                if b.guardado.is_some() {
                                    ui.label(RichText::new("✔ guardado").size(12.0).color(p.green));
                                }
                            });
                        }));
                        if r.response.clicked() {
                            self.bsel = Some(j);
                        }
                        ui.add_space(4.0);
                    }
                });
            });
            ui.add_space(8.0);
            let Some(j) = self.bsel.filter(|j| *j < self.borradores.len()) else { return };
            ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), alto), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().id_salt("bedit").show(ui, |ui| {
                    let b = &mut self.borradores[j];
                    theme::card(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::Grid::new("cab").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                            ui.label(RichText::new("Para").color(p.muted));
                            ui.add(egui::TextEdit::singleline(&mut b.para).desired_width(f32::INFINITY).hint_text("correo@example.invalid; otro@example.invalid"));
                            ui.end_row();
                            ui.label(RichText::new("CC").color(p.muted));
                            ui.add(egui::TextEdit::singleline(&mut b.cc).desired_width(f32::INFINITY));
                            ui.end_row();
                            ui.label(RichText::new("Asunto").color(p.muted));
                            ui.add(egui::TextEdit::singleline(&mut b.asunto).desired_width(f32::INFINITY));
                            ui.end_row();
                        });
                        if !b.cc.to_lowercase().contains(CC_OBLIGATORIO) {
                            ui.label(RichText::new(format!("El CC {CC_OBLIGATORIO} se agrega siempre al guardar.")).color(p.amber).size(12.0));
                        }
                        ui.add(egui::TextEdit::multiline(&mut b.cuerpo).desired_rows(9).desired_width(f32::INFINITY));
                        ui.horizontal(|ui| {
                            if theme::primary(ui, p, "💾  Guardar este borrador").clicked() {
                                guardar_uno = Some(j);
                            }
                            if b.nomina.is_some() && ui.button("📎 Guardar solo la nómina").clicked() {
                                guardar_nomina = Some(j);
                            }
                        });
                    });
                    if let Some(n) = &mut b.nomina {
                        ui.add_space(8.0);
                        theme::card(p).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            theme::seccion(ui, p, "Nómina adjunta (edita, desmarca o agrega filas antes de guardar)");
                            let mut quitar = None;
                            egui::ScrollArea::horizontal().id_salt("nomina").show(ui, |ui| {
                                egui::Grid::new("nom").striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
                                    ui.label("");
                                    for h in &n.headers {
                                        ui.label(RichText::new(h).strong().size(12.0).color(p.muted));
                                    }
                                    ui.label("");
                                    ui.end_row();
                                    for (k, row) in n.rows.iter_mut().enumerate() {
                                        ui.checkbox(&mut n.incluir[k], "");
                                        for v in row.iter_mut() {
                                            let ancho = (v.chars().count().clamp(6, 40) as f32) * 7.5;
                                            ui.add(egui::TextEdit::singleline(v).desired_width(ancho).font(egui::TextStyle::Small));
                                        }
                                        if ui.small_button("✕").on_hover_text("Quitar fila de la nómina").clicked() {
                                            quitar = Some(k);
                                        }
                                        ui.end_row();
                                    }
                                });
                            });
                            if let Some(k) = quitar {
                                n.rows.remove(k);
                                n.incluir.remove(k);
                            }
                            if ui.small_button("＋ Agregar fila").clicked() {
                                n.rows.push(vec![String::new(); n.headers.len()]);
                                n.incluir.push(true);
                            }
                        });
                    }
                });
            });
        });
        if let Some(j) = guardar_uno {
            self.guardar_borradores(Some(j));
        }
        if let Some(j) = guardar_nomina {
            if let Some(dir) = self.carpeta_productos() {
                match productos::guardar_nomina(&self.borradores[j], &dir.join("Nominas")) {
                    Ok(Some(pth)) => {
                        self.aviso(format!("Nómina guardada: {}", pth.display()), false);
                        self.registrar(pth, "Nómina");
                    }
                    Ok(None) => self.aviso("La nómina no tiene filas marcadas.", true),
                    Err(e) => self.aviso(e, true),
                }
            }
        }
    }

    fn opciones_modalidades(&self) -> Vec<&'static str> {
        MODALIDADES.iter().zip(self.modalidades).filter(|(_, on)| *on).map(|((k, _), _)| *k).collect()
    }

    fn preparar_correos(&mut self) {
        let Some(w) = &self.work else { return };
        let mods = self.opciones_modalidades();
        if mods.is_empty() {
            self.aviso("Marca al menos una modalidad.", true);
            return;
        }
        let tribs: Vec<String> = self.tribunales_sel.iter().cloned().collect();
        let o = productos::OpcionesCorreo { modalidades: &mods, tribunales: &tribs, solo_seleccionadas: self.solo_sel_correo };
        let nuevos = productos::preparar_borradores(w, &self.cfg, &o);
        // Volver a preparar conserva lo editado en cada borrador (mismo tipo y título).
        let mut out = Vec::new();
        for mut b in nuevos {
            if let Some(v) = self.borradores.iter().find(|v| v.kind == b.kind && v.titulo == b.titulo && v.guardado.is_none()) {
                b.para = v.para.clone();
                b.cc = v.cc.clone();
                b.asunto = v.asunto.clone();
                b.cuerpo = v.cuerpo.clone();
            }
            out.push(b);
        }
        let n = out.len();
        let sin = out.iter().filter(|b| b.para.trim().is_empty()).count();
        self.borradores = out;
        self.bsel = if n > 0 { Some(0) } else { None };
        if n == 0 {
            self.aviso("No hay correos para las modalidades y tribunales elegidos.", false);
        } else {
            let extra = if sin > 0 { format!("; {sin} sin destinatario (complétalos o agrega el contacto)") } else { String::new() };
            self.aviso(format!("{n} borradores preparados{extra}."), false);
        }
    }

    fn preparar_tipo(&mut self, kind: &str) {
        let filas: Vec<usize> = self
            .work
            .as_ref()
            .map(|w| w.registros.iter().enumerate().filter(|(_, r)| r.seleccionada).map(|(i, _)| i).collect())
            .unwrap_or_default();
        if kind == "proyectos" {
            let Some(dir) = self.carpeta_productos() else { return };
            match productos::reporte_proyectos(&self.proyectos, &dir, &self.cfg.periodo) {
                Ok(pth) => {
                    if let Some(mut b) = productos::borrador_plantilla(self.work.as_ref(), &self.cfg, "proyectos", &[]) {
                        let headers = vec!["TRIBUNAL", "RIT", "TIPO", "NOMBRE", "PROGRAMA"].into_iter().map(String::from).collect();
                        let rows: Vec<Vec<String>> = self
                            .proyectos
                            .iter()
                            .filter(|p| p.incluir)
                            .map(|p| vec![p.tribunal.clone(), p.rit.clone(), p.tipo.clone(), p.vars["NOMBRE"].clone(), p.vars["PROGRAMA"].clone()])
                            .collect();
                        let n = rows.len();
                        b.nomina = Some(productos::Nomina { headers, rows, incluir: vec![true; n] });
                        self.borradores.push(b);
                        self.bsel = Some(self.borradores.len() - 1);
                    }
                    self.registrar(pth, "Reporte de proyectos");
                }
                Err(e) => self.aviso(e, true),
            }
            return;
        }
        if filas.is_empty() {
            self.aviso("Marca en Trabajo las filas de la gestión particular (primera columna).", true);
            return;
        }
        if let Some(b) = productos::borrador_plantilla(self.work.as_ref(), &self.cfg, kind, &filas) {
            self.borradores.push(b);
            self.bsel = Some(self.borradores.len() - 1);
        }
    }

    fn guardar_borradores(&mut self, solo: Option<usize>) {
        let Some(dir) = self.carpeta_productos() else { return };
        let dir = dir.join(format!("Borradores_{}", chrono::Local::now().format("%Y-%m-%d")));
        let mut ok = 0;
        let mut errores = Vec::new();
        let idx: Vec<usize> = match solo {
            Some(j) => vec![j],
            None => (0..self.borradores.len()).filter(|j| self.borradores[*j].incluir && self.borradores[*j].guardado.is_none()).collect(),
        };
        let outlook = self.cfg.destino_outlook;
        for j in idx {
            if outlook {
                match productos::guardar_outlook(&self.borradores[j]) {
                    Ok(()) => {
                        self.borradores[j].guardado = Some(PathBuf::from("Outlook: Borradores"));
                        ok += 1;
                        continue;
                    }
                    // Si Outlook falla, queda una copia .eml de respaldo y se sigue con los demás.
                    Err(e) => errores.push(e),
                }
            }
            match productos::guardar_eml(&self.borradores[j], &dir) {
                Ok(pth) => {
                    if !outlook {
                        self.borradores[j].guardado = Some(pth.clone());
                        ok += 1;
                    }
                    self.registrar(pth, if outlook { "Copia .eml de respaldo" } else { "Borrador de correo" });
                }
                Err(e) => errores.push(e),
            }
        }
        let sin_destinatario = (0..self.borradores.len())
            .filter(|j| self.borradores[*j].guardado.is_some() && self.borradores[*j].para.trim().is_empty())
            .count();
        if sin_destinatario > 0 {
            self.aviso(format!("{sin_destinatario} borrador(es) quedaron sin destinatario: complétalo en Outlook antes de enviar."), false);
        }
        if ok > 0 && outlook {
            self.aviso(format!("{ok} borradores guardados en Borradores de Outlook. Revísalos y envíalos tú."), false);
        } else if ok > 0 {
            self.aviso(format!("{ok} borradores guardados en {}. Ábrelos con Outlook, revísalos y envíalos tú.", dir.display()), false);
        } else if errores.is_empty() {
            self.aviso("No hay borradores pendientes marcados (los ya guardados se omiten).", false);
        }
        for e in errores {
            self.aviso(e, true);
        }
    }

    // ── Resoluciones ─────────────────────────────────────────────────────────

    fn v_resoluciones(&mut self, ui: &mut egui::Ui, p: &Pal) {
        theme::titulo(ui, p, "Resoluciones", "Un proyecto por tribunal, RIT y tipo con todas las personas de la causa. Lo que falte queda como [COMPLETAR …] en el documento.");
        let mut preparar = false;
        let mut generar = false;
        theme::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let hay = self.work.is_some();
                if ui.add_enabled(hay, egui::Button::new(RichText::new("⚖  Preparar proyectos").color(Color32::WHITE).strong()).fill(p.accent)).clicked() {
                    preparar = true;
                }
                ui.checkbox(&mut self.solo_sel_res, "Solo filas marcadas");
                if !self.proyectos.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::primary(ui, p, "📝  Generar Word").clicked() {
                            generar = true;
                        }
                        ui.label(RichText::new(format!("{} de {} marcados", self.proyectos.iter().filter(|x| x.incluir).count(), self.proyectos.len())).color(p.muted));
                    });
                }
            });
            ui.label(
                RichText::new("Plantillas propias: deja TRIBUNAL_TIPO.docx (por ejemplo TOME_PC_IE.docx) en la carpeta de plantillas (Ajustes), con {{RIT}}, {{NOMBRE}}, {{RUT}}, {{PROGRAMA}}, {{FECHA}}, {{FECHA_RESOLUCION}}, {{DURACION}} y {{NOMENCLATURA}}.")
                    .size(12.0)
                    .color(p.muted),
            );
        });
        if preparar {
            if let Some(w) = &self.work {
                self.proyectos = productos::preparar_proyectos(w, &self.cfg, self.solo_sel_res);
                self.psel = if self.proyectos.is_empty() { None } else { Some(0) };
                let n = self.proyectos.len();
                self.aviso(if n == 0 { "No hay registros que pidan proyecto (PC_IE, PC_INFO o NOMENCL).".to_string() } else { format!("{n} proyectos preparados.") }, false);
            }
        }
        if generar {
            if let Some(dir) = self.carpeta_productos() {
                match productos::generar_word(&self.proyectos, &dir, self.corte()) {
                    Ok(pth) => {
                        self.aviso(format!("Proyectos generados con la matriz de cada tribunal (un Word por proyecto) en {}", pth.display()), false);
                        self.registrar(pth, "Proyectos de resolución (Word)");
                    }
                    Err(e) => self.aviso(e, true),
                }
            }
        }
        ui.add_space(8.0);
        if self.proyectos.is_empty() {
            return;
        }
        let alto = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(egui::vec2(330.0, alto), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().id_salt("plist").show(ui, |ui| {
                    for (j, pr) in self.proyectos.iter_mut().enumerate() {
                        let activo = self.psel == Some(j);
                        let mut fr = theme::card(p).inner_margin(egui::Margin::same(10));
                        if activo {
                            fr = fr.stroke(egui::Stroke::new(1.5_f32, p.accent));
                        }
                        // El clic de la tarjeta se registra debajo del contenido: el checkbox recibe su propio clic.
                        let r = ui.scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| fr.show(ui, |ui| {
                            ui.set_width(300.0);
                            ui.set_max_width(300.0);
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut pr.incluir, "");
                                ui.label(RichText::new(format!("{} · {}", pr.tipo, pr.rit)).strong().monospace());
                            });
                            ui.add(egui::Label::new(RichText::new(&pr.vars["NOMBRE"]).size(12.5)).truncate());
                            ui.label(RichText::new(&pr.tribunal).size(12.0).color(p.muted));
                            let faltan: Vec<&str> = pr
                                .vars
                                .iter()
                                .filter(|(k, v)| v.trim().is_empty() && plantilla_usa(&pr.tribunal_clave, &pr.tipo, k))
                                .map(|(k, _)| k.as_str())
                                .collect();
                            if !faltan.is_empty() {
                                ui.label(RichText::new(format!("Falta: {}", faltan.join(", "))).size(11.5).color(p.amber));
                            }
                        }));
                        if r.response.clicked() {
                            self.psel = Some(j);
                        }
                        ui.add_space(4.0);
                    }
                });
            });
            ui.add_space(8.0);
            let Some(j) = self.psel.filter(|j| *j < self.proyectos.len()) else { return };
            ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), alto), egui::Layout::top_down(egui::Align::Min), |ui| {
                theme::card(p).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let pr = &mut self.proyectos[j];
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Tipo").color(p.muted));
                        egui::ComboBox::from_id_salt("ptipo").selected_text(pr.tipo.clone()).show_ui(ui, |ui| {
                            for (k, l) in TIPOS {
                                ui.selectable_value(&mut pr.tipo, k.to_string(), l);
                            }
                        });
                        ui.label(RichText::new(&pr.tribunal).color(p.muted));
                    });
                    egui::Grid::new("pvars").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        let (tc, tp) = (pr.tribunal_clave.clone(), pr.tipo.clone());
                        for (k, v) in pr.vars.iter_mut() {
                            if !plantilla_usa(&tc, &tp, k) {
                                continue;
                            }
                            ui.label(RichText::new(k).color(p.muted).size(12.5));
                            ui.add(egui::TextEdit::singleline(v).desired_width(f32::INFINITY).hint_text(format!("[COMPLETAR {k}]")));
                            ui.end_row();
                        }
                    });
                    ui.add_space(6.0);
                    theme::seccion(ui, p, "Vista previa");
                    let alto_prev = (ui.available_height() - 10.0).max(120.0);
                    egui::ScrollArea::vertical().id_salt("prev").max_height(alto_prev).show(ui, |ui| {
                        egui::Frame::new().fill(Color32::from_rgb(252, 252, 250)).stroke(egui::Stroke::new(1.0_f32, p.border)).corner_radius(6).inner_margin(18).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            for t in productos::texto_proyecto(pr) {
                                ui.label(RichText::new(t).color(Color32::from_rgb(30, 30, 30)).size(13.5));
                                ui.add_space(4.0);
                            }
                        });
                    });
                });
            });
        });
    }

    // ── Resultados ───────────────────────────────────────────────────────────

    fn v_resultados(&mut self, ui: &mut egui::Ui, p: &Pal) {
        theme::titulo(ui, p, "Resultados", "Lo creado en esta sesión. Nada se envía ni se registra en RUS: tú revisas y actúas.");
        let mut cambiar = false;
        theme::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Carpeta de productos:").color(p.muted));
                ui.label(if self.cfg.carpeta_productos.is_empty() { "(se pide la primera vez que guardas)".to_string() } else { self.cfg.carpeta_productos.clone() });
                if !self.cfg.carpeta_productos.is_empty() && ui.button("📁 Abrir carpeta").clicked() {
                    abrir_en_sistema(&PathBuf::from(&self.cfg.carpeta_productos));
                }
                if ui.button("Cambiar…").clicked() {
                    cambiar = true;
                }
            });
        });
        if cambiar {
            if let Some(d) = rfd::FileDialog::new().pick_folder() {
                self.cfg.carpeta_productos = d.display().to_string();
                self.guardar_cfg();
            }
        }
        ui.add_space(8.0);
        if let Some(w) = &self.work {
            let revisadas = w.registros.iter().filter(|r| r.revisada).count();
            let pend = w.registros.iter().filter(|r| !r.revisada && !r.excluido).count();
            let guardados = self.borradores.iter().filter(|b| b.guardado.is_some()).count();
            theme::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                theme::seccion(ui, p, "Lo que falta para terminar");
                ui.label(format!("• {revisadas} filas revisadas; {pend} pendientes de revisar."));
                ui.label(format!("• {} borradores preparados; {guardados} guardados.", self.borradores.len()));
                ui.label(format!("• {} proyectos de resolución preparados.", self.proyectos.len()));
            });
            ui.add_space(8.0);
        }
        if self.productos.is_empty() {
            ui.label(RichText::new("Aún no se ha creado nada en esta sesión.").color(p.muted));
            return;
        }
        theme::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("prods").striped(true).num_columns(4).spacing([14.0, 6.0]).show(ui, |ui| {
                for t in ["Hora", "Tipo", "Archivo", ""] {
                    ui.label(RichText::new(t).strong().color(p.muted));
                }
                ui.end_row();
                for pr in self.productos.iter().rev() {
                    ui.label(&pr.hora);
                    ui.label(&pr.tipo);
                    ui.label(pr.ruta.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
                    ui.horizontal(|ui| {
                        if ui.small_button("Abrir").clicked() {
                            abrir_en_sistema(&pr.ruta);
                        }
                        if ui.small_button("Carpeta").clicked() {
                            if let Some(d) = pr.ruta.parent() {
                                abrir_en_sistema(d);
                            }
                        }
                    });
                    ui.end_row();
                }
            });
        });
    }

    // ── Contactos ────────────────────────────────────────────────────────────

    fn v_contactos(&mut self, ui: &mut egui::Ui, p: &Pal) {
        theme::titulo(ui, p, "Contactos", "Correos de programas y tribunales, CC adicional y firma. Se guardan solo en este equipo.");
        let mut guardar = false;
        let mut importar = false;
        let mut exportar = false;
        let faltan: Vec<String> = match &self.work {
            Some(w) => {
                let mut f: Vec<String> = w.registros.iter().map(|r| w.value(r, "programa")).filter(|pr| !pr.is_empty() && self.cfg.contacto_programa(pr).is_empty()).collect();
                f.sort();
                f.dedup();
                f
            }
            None => vec![],
        };
        egui::ScrollArea::vertical().id_salt("cont").show(ui, |ui| {
            theme::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                theme::seccion(ui, p, "Tribunales (destinatarios de los informativos y nóminas)");
                egui::Grid::new("tribs").num_columns(3).spacing([10.0, 6.0]).show(ui, |ui| {
                    for (k, t) in self.cfg.tribunales.iter_mut() {
                        ui.label(RichText::new(k).strong());
                        guardar |= ui.add(egui::TextEdit::singleline(&mut t.nombre).desired_width(240.0)).lost_focus();
                        guardar |= ui.add(egui::TextEdit::singleline(&mut t.para).desired_width(360.0).hint_text("correo@example.invalid; otro@example.invalid")).lost_focus();
                        ui.end_row();
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CC fijo").color(p.muted));
                    ui.label(RichText::new(CC_OBLIGATORIO).strong());
                    ui.label(RichText::new("CC adicional").color(p.muted));
                    guardar |= ui.add(egui::TextEdit::singleline(&mut self.cfg.cc_adicional).desired_width(300.0)).lost_focus();
                });
                ui.label(RichText::new("Firma (se agrega al final de cada borrador)").color(p.muted));
                guardar |= ui.add(egui::TextEdit::multiline(&mut self.cfg.firma).desired_rows(3).desired_width(f32::INFINITY)).lost_focus();
            });
            ui.add_space(8.0);
            theme::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    theme::seccion(ui, p, &format!("Programas ({})", self.cfg.contactos.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Importar CSV/Excel…").on_hover_text("Columnas Nombre y Mail (como el original)").clicked() {
                            importar = true;
                        }
                        if ui.button("Exportar…").clicked() {
                            exportar = true;
                        }
                        ui.add(egui::TextEdit::singleline(&mut self.filtro_contactos).hint_text("🔍 Buscar").desired_width(180.0));
                    });
                });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.nuevo_prog).hint_text("Nombre del programa (como en DERIVACION)").desired_width(300.0));
                    ui.add(egui::TextEdit::singleline(&mut self.nuevo_mail).hint_text("correo@example.invalid").desired_width(260.0));
                    if ui.button("➕ Agregar").clicked() && !self.nuevo_prog.trim().is_empty() {
                        self.cfg.contactos.insert(self.nuevo_prog.trim().to_string(), self.nuevo_mail.trim().to_string());
                        self.nuevo_prog.clear();
                        self.nuevo_mail.clear();
                        guardar = true;
                    }
                });
                if !faltan.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Sin contacto en el trabajo abierto:").color(p.amber).size(12.5));
                        for f in faltan.iter().take(12) {
                            if ui.small_button(f).on_hover_text("Usar este nombre").clicked() {
                                self.nuevo_prog = f.clone();
                            }
                        }
                    });
                }
                ui.add_space(4.0);
                let q = clave(&self.filtro_contactos);
                let mut borrar = None;
                egui::Grid::new("progs").striped(true).num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                    for (k, v) in self.cfg.contactos.iter_mut() {
                        if !q.is_empty() && !clave(k).contains(&q) && !clave(v).contains(&q) {
                            continue;
                        }
                        ui.label(k.as_str());
                        guardar |= ui.add(egui::TextEdit::singleline(v).desired_width(340.0)).lost_focus();
                        if ui.small_button("🗑").clicked() {
                            borrar = Some(k.clone());
                        }
                        ui.end_row();
                    }
                });
                if let Some(k) = borrar {
                    self.cfg.contactos.remove(&k);
                    guardar = true;
                }
            });
        });
        if importar {
            self.importar_contactos();
        }
        if exportar {
            self.exportar_contactos();
        }
        if guardar {
            self.guardar_cfg();
        }
    }

    fn importar_contactos(&mut self) {
        let Some(pth) = rfd::FileDialog::new().add_filter("Contactos", &["csv", "txt", "xlsx", "xls", "json"]).pick_file() else { return };
        let Ok(bytes) = std::fs::read(&pth) else { return };
        let mut n = 0;
        if pth.extension().is_some_and(|e| e == "json") {
            if let Ok(v) = serde_json::from_slice::<Vec<std::collections::BTreeMap<String, String>>>(&bytes) {
                for m in v {
                    if let (Some(a), Some(b)) = (m.get("Nombre"), m.get("Mail")) {
                        self.cfg.contactos.insert(a.clone(), b.clone());
                        n += 1;
                    }
                }
            }
        } else if let Ok(sheets) = crate::lectura::read_sheets(&bytes) {
            for s in sheets {
                let Some(h) = s.rows.first() else { continue };
                let hs = crate::lectura::headers_of(h);
                let ni = hs.iter().position(|x| ["nombre", "programa", "derivacion"].contains(&normalizar(x).as_str()));
                let mi = hs.iter().position(|x| ["mail", "correo", "email", "e mail"].contains(&normalizar(x).as_str()));
                if let (Some(ni), Some(mi)) = (ni, mi) {
                    for r in s.rows.iter().skip(1) {
                        let a = r.get(ni).map(|c| c.display()).unwrap_or_default();
                        let b = r.get(mi).map(|c| c.display()).unwrap_or_default();
                        if a.trim().is_empty() {
                            continue;
                        }
                        if let Some(t) = a.trim().strip_prefix("Tribunal: ") {
                            if let Some(tt) = self.cfg.tribunales.get_mut(&t.trim().to_uppercase()) {
                                tt.para = b.trim().to_string();
                            }
                        } else {
                            self.cfg.contactos.insert(a.trim().to_string(), b.trim().to_string());
                        }
                        n += 1;
                    }
                }
            }
        }
        self.guardar_cfg();
        self.aviso(format!("{n} contactos importados."), n == 0);
    }

    fn exportar_contactos(&mut self) {
        let Some(pth) = rfd::FileDialog::new().set_file_name("contactos.csv").save_file() else { return };
        let mut s = String::from("Nombre;Mail\n");
        for (k, t) in &self.cfg.tribunales {
            s += &format!("Tribunal: {k};{}\n", t.para);
        }
        for (k, v) in &self.cfg.contactos {
            s += &format!("{k};{v}\n");
        }
        match std::fs::write(&pth, s) {
            Ok(_) => self.aviso(format!("Contactos exportados: {}", pth.display()), false),
            Err(e) => self.aviso(e.to_string(), true),
        }
    }

    // ── Anonimizador ─────────────────────────────────────────────────────────

    fn v_anon(&mut self, ui: &mut egui::Ui, p: &Pal) {
        theme::titulo(ui, p, "Anonimizador", "Copias con identidades ficticias de los archivos del Descargador o del Asistente. El original no se toca; nada sale del equipo.");
        let mut ejecutar = false;
        let mut agregar = false;
        let mut elegir_salida = false;
        egui::ScrollArea::vertical().id_salt("anon").show(ui, |ui| {
            theme::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if theme::primary(ui, p, "➕  Agregar archivos…").clicked() {
                        agregar = true;
                    }
                    if !self.anon_files.is_empty() && ui.button("Quitar todos").clicked() {
                        self.anon_files.clear();
                    }
                    ui.label(RichText::new("o arrastra archivos a la ventana · Excel, CSV, HTML/XML de SITFA, TXT, JSON").color(p.muted).size(12.5));
                });
                let mut quitar = None;
                for (i, f) in self.anon_files.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("📄").color(p.muted));
                        ui.label(f.display().to_string());
                        if ui.small_button("✕").clicked() {
                            quitar = Some(i);
                        }
                    });
                }
                if let Some(i) = quitar {
                    self.anon_files.remove(i);
                }
            });
            ui.add_space(8.0);
            theme::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                theme::seccion(ui, p, "Opciones");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.anon_opts.modo, anon::Modo::Estricto, "Estricto")
                        .on_hover_text("Todo texto libre y toda columna desconocida se reemplazan por [texto omitido]");
                    ui.radio_value(&mut self.anon_opts.modo, anon::Modo::Conservador, "Conservador")
                        .on_hover_text("Conserva observaciones sustituyendo identidades conocidas; revisa cada texto");
                });
                egui::Grid::new("aop").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Semilla (opcional)");
                    ui.add(egui::TextEdit::singleline(&mut self.anon_opts.semilla).hint_text("misma frase = mismas personas ficticias entre ejecuciones").desired_width(360.0));
                    ui.end_row();
                    ui.label("Desplazar fechas (días)");
                    ui.add(egui::DragValue::new(&mut self.anon_opts.desplazar_dias).range(-3650..=3650));
                    ui.end_row();
                    ui.label("Nacimientos al 1 de enero");
                    ui.checkbox(&mut self.anon_opts.generalizar_nacimiento, "");
                    ui.end_row();
                    ui.label("Carpeta de salida");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.anon_out).hint_text("nueva carpeta junto al primer archivo").desired_width(300.0));
                        if ui.button("Elegir…").clicked() {
                            elegir_salida = true;
                        }
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                if ui
                    .add_enabled(!self.anon_files.is_empty(), egui::Button::new(RichText::new("🛡  Crear copias anonimizadas").color(Color32::WHITE).strong()).fill(p.accent))
                    .clicked()
                {
                    ejecutar = true;
                }
            });
            if let Some(inf) = &self.anon_inf {
                ui.add_space(8.0);
                theme::card(p).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    theme::seccion(ui, p, "Resultado");
                    for c in &inf.creados {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✔").color(p.green));
                            ui.label(c.display().to_string());
                            if ui.small_button("Abrir carpeta").clicked() {
                                if let Some(d) = c.parent() {
                                    abrir_en_sistema(d);
                                }
                            }
                        });
                    }
                    for e in &inf.errores {
                        ui.label(RichText::new(format!("✕ {e}")).color(p.red));
                    }
                    for a in &inf.advertencias {
                        ui.label(RichText::new(format!("⚠ {a}")).color(p.amber));
                    }
                    if !inf.reemplazos.is_empty() {
                        ui.label(RichText::new(inf.reemplazos.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join(" · ")).color(p.muted).size(12.5));
                    }
                });
            }
        });
        if agregar {
            if let Some(f) = rfd::FileDialog::new()
                .add_filter("Admitidos", &["xlsx", "xlsm", "xls", "csv", "tsv", "html", "htm", "xml", "txt", "md", "log", "json", "jsonl", "ics"])
                .pick_files()
            {
                self.anon_files.extend(f);
            }
        }
        if elegir_salida {
            if let Some(d) = rfd::FileDialog::new().pick_folder() {
                self.anon_out = d.display().to_string();
            }
        }
        if ejecutar {
            let out = if self.anon_out.trim().is_empty() {
                let base = self.anon_files[0].parent().map(|d| d.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
                base.join(format!("anonimizado_{}", chrono::Local::now().format("%Y%m%d_%H%M%S")))
            } else {
                PathBuf::from(self.anon_out.trim())
            };
            let t0 = Instant::now();
            let inf = anon::ejecutar(&self.anon_files, &out, &self.anon_opts);
            let n = inf.creados.len();
            for c in inf.creados.clone() {
                self.registrar(c, "Copia anonimizada");
            }
            let err = !inf.errores.is_empty();
            self.aviso(format!("{n} copias creadas en {} ms.", t0.elapsed().as_millis()), err);
            self.anon_inf = Some(inf);
        }
    }

    // ── Ajustes ──────────────────────────────────────────────────────────────

    fn v_ajustes(&mut self, ui: &mut egui::Ui, p: &Pal) {
        theme::titulo(ui, p, "Ajustes", "Umbrales y textos de las reglas. Los cambios se aplican al volver a analizar; los valores base son los del original.");
        let mut guardar = false;
        let mut reanalizar = false;
        egui::ScrollArea::vertical().id_salt("aj").show(ui, |ui| {
            ui.columns(2, |cols| {
                let (izq, der) = cols.split_at_mut(1);
                let ui = &mut izq[0];
                theme::card(p).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    theme::seccion(ui, p, "Redacción");
                    if ui
                        .checkbox(&mut self.cfg.redaccion_registro, "Redacción del registro mensual")
                        .on_hover_text("Decisiones del 20-nov-2026: sin el próximo informe cuando la medida está por vencer y nombres de programa como en el registro. Desmarcado: textos de Flow Python.")
                        .changed()
                    {
                        guardar = true;
                        reanalizar = true;
                    }
                });
                ui.add_space(8.0);
                theme::card(p).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    theme::seccion(ui, p, "Umbrales (días)");
                    ui.add(egui::TextEdit::singleline(&mut self.filtro_umbrales).hint_text("Buscar parámetro…").desired_width(f32::INFINITY));
                    // `_filter_config_parameters`: coincidencia normalizada con la etiqueta o la clave.
                    let consulta = normalizar(&self.filtro_umbrales);
                    egui::Grid::new("umb").num_columns(3).spacing([10.0, 5.0]).striped(true).show(ui, |ui| {
                        for (k, base, label) in UMBRALES {
                            if !consulta.is_empty() && !normalizar(&format!("{label} {k}")).contains(&consulta) {
                                continue;
                            }
                            let v = self.cfg.umbrales.entry(k.to_string()).or_insert(base);
                            ui.label(label);
                            if ui.add(egui::DragValue::new(v).range(0..=999)).changed() {
                                guardar = true;
                            }
                            if *v != base {
                                if ui.small_button(format!("↺ {base}")).on_hover_text("Volver al valor base").clicked() {
                                    *v = base;
                                    guardar = true;
                                }
                            } else {
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
                    if self.cfg.umbral("proyecto_tome") < self.cfg.umbral("espera_tome") {
                        ui.label(RichText::new("El proyecto de Tomé no puede ir antes que su correo.").color(p.amber));
                    }
                    ui.add_space(6.0);
                    if ui.button("Aplicar a la nómina abierta").clicked() {
                        reanalizar = true;
                    }
                });
                let ui = &mut der[0];
                theme::card(p).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    theme::seccion(ui, p, "Textos de las observaciones");
                    egui::ComboBox::from_id_salt("txt").width(380.0).selected_text(self.texto_sel.clone()).show_ui(ui, |ui| {
                        for (k, _) in config::TEXTOS {
                            let marca = if self.cfg.textos.contains_key(k) { " ✎" } else { "" };
                            ui.selectable_value(&mut self.texto_sel, k.to_string(), format!("{k}{marca}"));
                        }
                    });
                    let k = self.texto_sel.clone();
                    let mut t = self.cfg.texto(&k);
                    if ui.add(egui::TextEdit::multiline(&mut t).desired_rows(5).desired_width(f32::INFINITY)).changed() {
                        if t == config::texto_base(&k) {
                            self.cfg.textos.remove(&k);
                        } else {
                            self.cfg.textos.insert(k.clone(), t);
                        }
                        guardar = true;
                    }
                    ui.horizontal(|ui| {
                        let mut activa = !self.cfg.desactivadas.contains(&k);
                        if ui.checkbox(&mut activa, "Regla activa").changed() {
                            if activa {
                                self.cfg.desactivadas.remove(&k);
                            } else {
                                self.cfg.desactivadas.insert(k.clone());
                            }
                            guardar = true;
                        }
                        if self.cfg.textos.contains_key(&k) && ui.button("↺ Texto base").clicked() {
                            self.cfg.textos.remove(&k);
                            guardar = true;
                        }
                    });
                    ui.label(RichText::new("Las variables entre llaves ({PROGRAMA}, {FECHA_VENCIMIENTO}…) se completan solas.").size(12.0).color(p.muted));
                    ui.add_space(10.0);
                    theme::seccion(ui, p, "Correos");
                    ui.horizontal(|ui| {
                        ui.label("Período (reporte de proyectos)");
                        guardar |= ui.add(egui::TextEdit::singleline(&mut self.cfg.periodo).desired_width(220.0)).lost_focus();
                    });
                    ui.add_space(10.0);
                    theme::seccion(ui, p, "Carpetas");
                    ui.horizontal(|ui| {
                        if ui.button("📁 Plantillas Word propias").clicked() {
                            let d = config::data_dir().join("plantillas");
                            let _ = std::fs::create_dir_all(&d);
                            abrir_en_sistema(&d);
                        }
                        if ui.button("📁 Datos de Flow").on_hover_text("Configuración y sesiones (60 días, hasta 10 anteriores)").clicked() {
                            let d = config::data_dir();
                            let _ = std::fs::create_dir_all(&d);
                            abrir_en_sistema(&d);
                        }
                    });
                });
            });
        });
        if guardar {
            self.guardar_cfg();
        }
        if reanalizar {
            self.reanalizar();
        }
    }
}

fn plantilla_usa(trib: &str, tipo: &str, var: &str) -> bool {
    match var {
        "NOMENCLATURA" => tipo == "NOMENCL",
        "DURACION" | "FECHA_RESOLUCION" => tipo == "PC_IE" && trib != "LAJA",
        "FECHA" => trib != "LAJA",
        "NOMBRE" | "RUT" | "PROGRAMA" => tipo != "NOMENCL",
        _ => true,
    }
}

fn texto_json(v: &serde_json::Value) -> String {
    v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())
}

pub fn abrir_en_sistema(p: &std::path::Path) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(p).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(p).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(p).spawn();
}

#[cfg(test)]
mod pruebas_paquete {
    use super::*;

    #[test]
    fn trabajo_abre_paquete_cambia_etapa_y_conserva_el_activo_ante_error() {
        let raiz = crate::prueba_entorno::requerida("FLOW_REFERENCIA");
        let raiz = PathBuf::from(raiz);
        let mut app = App::new(None);
        app.cfg = Config::default();
        app.corte_txt = "20/11/2026".into();
        app.abrir(raiz.join("rust/crates/flow-trabajo/tests/datos/paquete_integracion.zip"));
        let trabajo = app.work.as_mut().expect("paquete de integración");
        assert_eq!(trabajo.mode(), Mode::Espera);
        assert_eq!(trabajo.registros.len(), 2);
        assert!(trabajo.headers.iter().any(|h| h == "ID_REGISTRO_CAPTURA"));
        trabajo.registros[0].editada = Some("Edición ficticia".into());
        app.abrir(raiz.join("paquete_ficticio_inexistente.zip"));
        assert_eq!(app.work.as_ref().unwrap().registros[0].observacion(), "Edición ficticia");
        app.elegir_etapa(Some(Mode::Informes));
        let trabajo = app.work.as_ref().unwrap();
        assert_eq!(trabajo.mode(), Mode::Informes);
        assert_eq!(trabajo.registros.len(), 2);
        assert_ne!(trabajo.registros[0].observacion(), "Edición ficticia");
        app.elegir_etapa(None);
        assert_eq!(app.work.as_ref().unwrap().mode(), Mode::Espera);
    }
}

/// E-S1/Propuestas del 24-nov: un paquete del Descargador sin consolidado conserva todas las columnas de sus
/// registros en Trabajo y se exporta con la estructura de `FAE_RES`.
#[cfg(test)]
mod pruebas_producto {
    use super::*;
    use base64::Engine;
    use flow_rust::descargador::paquete::{self, Opciones};
    use serde_json::Value;

    fn reconstruir(destino: &std::path::Path, archivos: &Value) {
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    #[test]
    fn paquete_sin_consolidado_exporta_la_estructura_de_flow_python() {
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let oraculo: Value = serde_json::from_str(include_str!("../tests/datos/paquete.json")).unwrap();
        let e = oraculo["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "listados_dos_pestanas").unwrap();
        let base = perfil.join("producto-excel");
        let _ = std::fs::remove_dir_all(&base);
        let lote = base.join("lote").join(e["lote"]["carpeta"].as_str().unwrap());
        reconstruir(&lote, &e["lote"]["archivos"]);
        let zip = base.join("Captura.zip");
        let resultado = paquete::exportar_con(&[lote], &zip, &Opciones { consolidado: false, ..Default::default() }).unwrap();
        assert!(resultado.nota.is_empty() || !resultado.nota.contains("Incluye"), "{}", resultado.nota);
        let mut app = App::new(None);
        app.cfg = Config::default();
        app.corte_txt = "20/11/2026".into();
        app.abrir(zip.clone());
        let w = app.work.as_ref().expect("paquete abierto");
        assert!(w.paquete.is_some());
        // Punto 1: nada del registro normalizado se pierde en Trabajo.
        let captura = flow_rust::captura::leer_captura(&std::fs::read(&zip).unwrap()).unwrap();
        let normalizado: serde_json::Map<String, Value> = captura
            .filas("registros")
            .find(|r| r.get("record_type") == w.mode().label().to_uppercase())
            .map(|r| serde_json::from_str(r.get("normalized_json")).unwrap())
            .unwrap();
        for clave in ["SITFA_TRIBUNAL_CODIGO", "SITFA_CAUSA_RUS", "SITFA_INGRESO_RUS", "SITFA_ETAPA_RUS"] {
            assert!(normalizado.contains_key(clave), "el paquete sintético no trae {clave}");
            assert!(w.headers.iter().any(|h| h == clave), "Trabajo perdió {clave}: {:?}", w.headers);
        }
        assert!(w.headers.len() >= normalizado.len(), "Trabajo conserva {} de {} columnas", w.headers.len(), normalizado.len());
        // Puntos 2 y 3: libro nuevo con el orden del usuario y la clave de ingreso.
        let carpeta = base.join("productos");
        let ruta = crate::producto_excel::exportar(w, &carpeta, Some("motivo de prueba")).unwrap();
        let hojas = crate::lectura::read_sheets(&std::fs::read(&ruta).unwrap()).unwrap();
        let nombres: Vec<&str> = hojas.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(nombres, ["NURUS_LISTAS", "NURUS_TRAZABILIDAD", "Resumen", "SITFA_ARCHIVOS", w.mode().label().to_uppercase().as_str(), "Vencimientos"]);
        let datos = hojas.iter().find(|h| h.name == w.mode().label().to_uppercase()).expect("hoja de datos");
        let cab: Vec<String> = datos.rows[0].iter().map(|c| c.display()).collect();
        let desde = cab.iter().position(|h| h == "NURUS_ID_REGISTRO").expect("NURUS_ID_REGISTRO");
        let orden: Vec<&str> = cab[desde..desde + 10].iter().map(String::as_str).collect();
        assert_eq!(orden, crate::producto_excel::COLUMNAS_NURUS);
        assert_eq!(datos.rows.len(), w.registros.len() + 1);
        let col = |n: &str| cab.iter().position(|h| h == n).unwrap_or_else(|| panic!("falta {n}"));
        let (rit, clave, id) = (col("RIT"), col("NURUS_CLAVE_INGRESO"), col("NURUS_ID_REGISTRO"));
        for fila in &datos.rows[1..] {
            assert!(!fila[rit].display().is_empty(), "RIT vacío");
            assert!(!fila[id].display().is_empty());
            let k = fila[clave].display();
            assert_eq!(k.split('-').count(), 3, "clave de ingreso {k}");
        }
        let trazabilidad = hojas.iter().find(|h| h.name == "NURUS_TRAZABILIDAD").unwrap();
        assert!(trazabilidad.rows.iter().any(|f| f.first().map(|c| c.display()) == Some("FIDELIDAD".into())
            && f.get(1).map(|c| c.display()) == Some("motivo de prueba".into())));
        // Los avisos repetidos del paquete se muestran agrupados, con su cantidad.
        let avisos = w.paquete.as_ref().unwrap().avisos_agrupados();
        let incidencias = captura.filas("incidencias").count();
        assert_eq!(avisos.iter().map(|a| a.incidencias).sum::<usize>(), incidencias);
        assert!(avisos.len() <= incidencias);
        assert!(avisos.windows(2).all(|p| p[0].incidencias >= p[1].incidencias));
        // Con calendario de informes sin ingreso verificable, las incidencias iguales de varios registros salen juntas.
        let cal = oraculo["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "calendario_informes").unwrap();
        let lotes: Vec<std::path::PathBuf> = std::iter::once(&cal["lote"]).chain(cal["otros_lotes"].as_array().unwrap())
            .map(|l| {
                let c = base.join("cal").join(l["carpeta"].as_str().unwrap());
                reconstruir(&c, &l["archivos"]);
                c
            })
            .collect();
        let zip_cal = base.join("Calendario.zip");
        paquete::exportar_con(&lotes, &zip_cal, &Opciones::default()).unwrap();
        let captura_cal = flow_rust::captura::leer_captura(&std::fs::read(&zip_cal).unwrap()).unwrap();
        let n_incidencias = captura_cal.filas("incidencias").count();
        let mut app_cal = App::new(None);
        app_cal.cfg = Config::default();
        app_cal.corte_txt = "20/11/2026".into();
        app_cal.abrir(zip_cal);
        let agrupados = app_cal.work.as_ref().unwrap().paquete.as_ref().unwrap().avisos_agrupados();
        assert_eq!(agrupados.iter().map(|a| a.incidencias).sum::<usize>(), n_incidencias);
        assert!(n_incidencias >= 2 && agrupados.len() < n_incidencias, "{n_incidencias} incidencias en {} grupos", agrupados.len());
        assert!(agrupados[0].incidencias >= 2);
        // Un paquete al que le falte una columna normalizada debe hacer fallar esta prueba: sin la clave, no hay producto.
        let mut sin = App::new(None);
        sin.cfg = Config::default();
        sin.corte_txt = "20/11/2026".into();
        sin.abrir(zip);
        let mut w2 = sin.work.take().unwrap();
        w2.headers.retain(|h| h != "SITFA_INGRESO_RUS");
        assert!(crate::producto_excel::encabezados(&w2).iter().all(|h| h != "NURUS_CLAVE_INGRESO"));
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// E-A1: el flujo guardar la copia, revisarla en Excel y traerla de vuelta con «Actualizar desde Excel».
#[cfg(test)]
mod pruebas_sincronia {
    use super::*;
    use base64::Engine;
    use flow_rust::descargador::paquete::{self, Opciones};
    use serde_json::Value;

    #[test]
    fn guardar_la_copia_y_actualizar_desde_excel_trae_la_revision() {
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let oraculo: Value = serde_json::from_str(include_str!("../tests/datos/paquete.json")).unwrap();
        let e = oraculo["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "listados_dos_pestanas").unwrap();
        let base = perfil.join("sincronia");
        let _ = std::fs::remove_dir_all(&base);
        let lote = base.join("lote").join(e["lote"]["carpeta"].as_str().unwrap());
        for (ruta, b64) in e["lote"]["archivos"].as_object().unwrap() {
            let ruta = lote.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
        let zip = base.join("Captura.zip");
        paquete::exportar_con(&[lote], &zip, &Opciones::default()).unwrap();
        let mut app = App::new(None);
        app.cfg = Config::default();
        app.cfg.carpeta_productos = base.join("productos").display().to_string();
        app.corte_txt = "20/11/2026".into();
        app.abrir(zip);
        app.guardar_excel();
        let copia = app.work.as_ref().unwrap().copia.clone().expect("copia guardada");
        assert!(copia.is_file());
        assert!(app.work.as_ref().unwrap().registros.iter().all(|r| !r.id_excel.is_empty() && r.sync_base.is_some()));
        // La persona completa FECHA_OBS, TT y CC en Excel (aquí: se guarda otra copia con esos campos y se la señala).
        let w = app.work.as_mut().unwrap();
        w.registros[0].campos.insert("FECHA_OBS".into(), "2026-11-24".into());
        w.registros[0].campos.insert("TT".into(), "1".into());
        w.registros[0].campos.insert("CC".into(), "0".into());
        let revisada = crate::producto_excel::exportar(w, &base.join("revisada"), None).unwrap();
        w.registros[0].campos.clear();
        w.copia = Some(revisada);
        app.actualizar_desde_excel(None, Default::default());
        let w = app.work.as_ref().unwrap();
        assert!(app.sincronia.is_none());
        assert_eq!(w.registros[0].campos.get("FECHA_OBS").map(String::as_str), Some("2026-11-24"));
        assert_eq!((w.registros[0].campos.get("TT").map(String::as_str), w.registros[0].campos.get("CC").map(String::as_str)), (Some("1"), Some("0")));
        assert!(w.registros[1].campos.is_empty());
        // Una copia que no es de este trabajo se rechaza sin aplicar nada.
        let mut otro = App::new(None);
        otro.cfg = Config::default();
        otro.corte_txt = "20/11/2026".into();
        otro.abrir(base.join("Captura.zip"));
        let w2 = otro.work.as_mut().unwrap();
        w2.registros.truncate(1);
        let truncada = crate::producto_excel::exportar(w2, &base.join("truncada"), None).unwrap();
        let w = app.work.as_mut().unwrap();
        w.copia = Some(truncada);
        app.actualizar_desde_excel(None, Default::default());
        assert!(app.avisos.iter().any(|(t, error, _)| *error && t.contains("Faltan registros en la copia")), "{:?}", app.avisos);
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// E-A2: abrir un paquete nuevo con otro abierto conserva las decisiones y pide respuesta ante un conflicto.
#[cfg(test)]
mod pruebas_conciliacion {
    use super::*;
    use base64::Engine;
    use flow_rust::descargador::paquete::{self, Opciones};
    use serde_json::Value;

    fn lote_a_zip(base: &std::path::Path, nombre: &str, lote: &Value) -> PathBuf {
        let carpeta = base.join(nombre).join(lote["carpeta"].as_str().unwrap());
        for (ruta, b64) in lote["archivos"].as_object().unwrap() {
            let ruta = carpeta.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
        let zip = base.join(nombre).join("captura.zip");
        paquete::exportar_con(&[carpeta], &zip, &Opciones::default()).unwrap();
        zip
    }

    #[test]
    fn un_paquete_nuevo_conserva_decisiones_y_pide_respuesta_ante_un_conflicto() {
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let paquetes: Value = serde_json::from_str(include_str!("../tests/datos/paquete.json")).unwrap();
        let conciliacion: Value = serde_json::from_str(include_str!("../tests/datos/conciliacion.json")).unwrap();
        let listados = paquetes["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "listados").unwrap()["lote"].clone();
        let base = perfil.join("conciliacion-ui");
        let _ = std::fs::remove_dir_all(&base);
        let anterior = lote_a_zip(&base, "anterior", &listados);
        let nuevo = lote_a_zip(&base, "nuevo", &conciliacion["lote_programa_cambiado"]);
        let mut app = App::new(None);
        app.cfg = Config::default();
        app.corte_txt = "08/10/2026".into();
        app.abrir(anterior);
        let w = app.work.as_mut().unwrap();
        assert_eq!(w.registros.len(), 2);
        w.registros[0].editada = Some("Edición de la primera".into());
        w.registros[1].editada = Some("Edición de la segunda".into());
        w.registros[1].revisada = true;
        // La segunda persona cambió de programa: conflicto; el trabajo abierto sigue intacto y nada se aplicó.
        app.abrir(nuevo);
        let d = app.conciliacion.as_ref().expect("diálogo de conciliación");
        assert_eq!(d.conflictos.len(), 1);
        assert_eq!((d.conflictos[0].motivo.as_str(), d.conflictos[0].puede_conservar), ("Datos fuente modificados.", true));
        assert_eq!(d.conflictos[0].campos, ["Derivación"]);
        let w = app.work.as_ref().unwrap();
        assert_eq!(w.registros[1].editada.as_deref(), Some("Edición de la segunda"));
        assert!(w.cambios.is_none());
        // Conservar mis decisiones: las dos ediciones pasan al trabajo nuevo, con la fuente nueva y su comparación.
        let d = app.conciliacion.take().unwrap();
        let respuestas = d.conflictos.iter().map(|c| (c.id.clone(), crate::conciliar::Respuesta::Conservar)).collect();
        app.conciliar_con_anterior(d.nuevo, respuestas);
        assert!(app.conciliacion.is_none());
        let w = app.work.as_ref().unwrap();
        assert_eq!(w.registros[0].editada.as_deref(), Some("Edición de la primera"));
        assert_eq!(w.registros[1].editada.as_deref(), Some("Edición de la segunda"));
        assert!(w.registros[1].revisada && !w.registros[0].revisada);
        assert!(w.value(&w.registros[1], "programa").contains("Cambiado"), "la fuente nueva debe verse");
        let resumen = w.cambios.as_ref().unwrap().resumen();
        assert_eq!((resumen["Fuente"], resumen["Sin cambio"], resumen["Egresados"]), (1, 1, 0));
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// E-D · Paquete D2 automatizado con `egui_kittest`: la interfaz real (`App::frame`) en un perfil aislado.
#[cfg(test)]
mod pruebas_interfaz {
    use super::*;
    use egui_kittest::kittest::{Key, Queryable};
    use egui_kittest::Harness;

    fn interfaz(app: App) -> Harness<'static, App> {
        Harness::builder()
            .with_size(egui::Vec2::new(1440.0, 900.0))
            .build_state(|ctx, app: &mut App| app.frame(ctx), app)
    }

    fn umbrales_visibles(harness: &Harness<'_, App>) -> usize {
        UMBRALES.iter().map(|(_, _, label)| harness.query_all_by_label(label).count()).sum()
    }

    /// El buscador de umbrales: egui 0.31 no expone `hint_text` a AccessKit, así que se elige el campo de texto
    /// inmediatamente bajo el título «UMBRALES (DÍAS)» de su tarjeta (`theme::seccion` lo muestra en mayúsculas).
    fn buscador_umbrales<'t>(harness: &'t Harness<'_, App>) -> egui_kittest::kittest::Node<'t> {
        let titulo = harness.get_by_label("UMBRALES (DÍAS)").bounding_box().expect("título con posición").y1;
        harness
            .get_all_by_role(egui::accesskit::Role::TextInput)
            .filter_map(|n| n.bounding_box().map(|b| (b.y0 - titulo, n)))
            .filter(|(dy, _)| *dy >= 0.0)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, n)| n)
            .expect("buscador de umbrales")
    }

    fn esperados(texto: &str) -> usize {
        let consulta = normalizar(texto);
        UMBRALES
            .iter()
            .filter(|(clave, _, label)| normalizar(&format!("{label} {clave}")).contains(&consulta))
            .count()
    }

    #[test]
    fn ui_ajustes_muestra_16_umbrales_y_filtra() {
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        assert!(config::data_dir().starts_with(&perfil), "el perfil de la prueba no está aislado");
        let mut app = App::new(None);
        app.vista = Vista::Ajustes;
        let mut harness = interfaz(app);
        harness.run();
        assert_eq!(UMBRALES.len(), 16);
        assert_eq!(umbrales_visibles(&harness), 16, "Ajustes debe listar 16 umbrales");
        let mut filtrados = Vec::new();
        // Cifras literales (config::UMBRALES): «prórroga» → prorroga_reciente; «búsqueda» y «ventana_busqueda» →
        // ventana_busqueda. `esperados` replica el filtro solo como control cruzado.
        for (texto, literal) in [("prórroga", 1usize), ("búsqueda", 1), ("ventana_busqueda", 1)] {
            buscador_umbrales(&harness).type_text(texto);
            harness.run();
            let visibles = umbrales_visibles(&harness);
            assert!(visibles > 0 && visibles < 16, "«{texto}» debe filtrar la lista: {visibles}");
            assert_eq!(visibles, literal, "filtro «{texto}»");
            assert_eq!(visibles, esperados(texto), "filtro «{texto}» (control cruzado)");
            filtrados.push(format!("{texto}={visibles}"));
            for _ in 0..texto.chars().count() {
                buscador_umbrales(&harness).key_press(Key::Backspace);
            }
            harness.run();
            assert_eq!(umbrales_visibles(&harness), 16, "vaciar el buscador tras «{texto}» devuelve 16");
        }
        println!("D2_AJUSTES: umbrales 16, filtros {}, vacio 16", filtrados.join(" "));
    }

    #[test]
    #[ignore = "requiere FLOW_E3_PAQUETE"]
    fn ui_trabajo_consigna_otro_programa_y_conserva() {
        let ruta = PathBuf::from(crate::prueba_entorno::requerida("FLOW_E3_PAQUETE"));
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        assert!(config::data_dir().starts_with(&perfil), "el perfil de la prueba no está aislado");
        let mut app = App::new(None);
        app.corte_txt = "19/10/2026".into();
        app.abrir(ruta.clone());
        let fila = app
            .work
            .as_ref()
            .expect("abre el ZIP ficticio")
            .registros
            .iter()
            .position(|r| !r.otros_programas.is_empty())
            .expect("fila con otro programa vigente");
        let programa = app.work.as_ref().unwrap().registros[fila].otros_programas[0].clone();
        app.sel = Some(fila);
        let mut harness = interfaz(app);
        harness.run();
        assert!(harness.query_by_label("OTRO PROGRAMA VIGENTE").is_some(), "falta la sección");
        assert!(harness.state().work.as_ref().unwrap().registros[fila].propuesta.contains(&programa));
        harness.get_by_label(&format!("Marcar «{programa}» como consignada")).click();
        harness.run();
        assert!(!harness.state().work.as_ref().unwrap().registros[fila].propuesta.contains(&programa), "consignar retira la frase");
        assert!(harness.query_by_label(&format!("Deshacer consignación de «{programa}»")).is_some(), "falta «Deshacer»");
        // Guardado automático de la interfaz (800 ms después del cambio).
        std::thread::sleep(std::time::Duration::from_millis(900));
        harness.run();
        assert!(harness.state().sucio.is_none(), "la interfaz no guardó la sesión");
        drop(harness);

        let mut reabierta = App::new(None);
        reabierta.corte_txt = "19/10/2026".into();
        reabierta.abrir(ruta);
        reabierta.sel = Some(fila);
        let mut harness = interfaz(reabierta);
        harness.run();
        assert!(!harness.state().work.as_ref().unwrap().registros[fila].propuesta.contains(&programa), "la consignación no se conservó");
        assert!(harness.query_by_label(&format!("Deshacer consignación de «{programa}»")).is_some(), "tras reabrir falta «Deshacer»");
        println!("D2_TRABAJO: fila {fila}, programa «{programa}», consignada, deshacer visible, conservada al reabrir");
    }
}
