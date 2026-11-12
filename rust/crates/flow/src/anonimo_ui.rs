//! Anonimización local fuera del hilo de la ventana; la semilla nunca se guarda en ajustes.
use eframe::egui;
use flow_anonimo::{
    lote::{self, Avance, Resultado},
    Modo, Opciones,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver},
        Arc,
    },
};
enum Evento {
    Avance(Avance),
    Fin(Result<Resultado, String>),
}
pub struct Anonimo {
    entradas: Vec<PathBuf>,
    destino: Option<PathBuf>,
    conservador: bool,
    dias: i64,
    nacimiento: bool,
    rechazar: bool,
    semilla: String,
    receptor: Option<Receiver<Evento>>,
    cancelar: Arc<AtomicBool>,
    avance: Option<Avance>,
    mensaje: String,
    resultado: Option<PathBuf>,
}
impl Default for Anonimo {
    fn default() -> Self {
        Self {
            entradas: vec![],
            destino: None,
            conservador: false,
            dias: 0,
            nacimiento: false,
            rechazar: false,
            semilla: String::new(),
            receptor: None,
            cancelar: Arc::new(AtomicBool::new(false)),
            avance: None,
            mensaje: String::new(),
            resultado: None,
        }
    }
}
impl Drop for Anonimo {
    fn drop(&mut self) {
        self.cancelar.store(true, Ordering::Relaxed);
    }
}
impl Anonimo {
    fn iniciar(&mut self, ctx: &egui::Context) {
        let Some(destino) = self.destino.clone() else {
            return;
        };
        let entradas = self.entradas.clone();
        let opciones = Opciones {
            modo: if self.conservador { Modo::Conservador } else { Modo::Estricto },
            desplazar_dias: self.dias,
            generalizar_nacimiento: self.nacimiento,
            semilla: std::mem::take(&mut self.semilla),
            rechazar_no_admitidos: self.rechazar,
        };
        let (tx, rx) = channel();
        self.receptor = Some(rx);
        self.cancelar = Arc::new(AtomicBool::new(false));
        let cancelar = self.cancelar.clone();
        let ctx = ctx.clone();
        self.resultado = None;
        self.mensaje.clear();
        std::thread::spawn(move || {
            let r = lote::ejecutar(&entradas, &destino, opciones, &cancelar, |a| {
                let _ = tx.send(Evento::Avance(a));
                ctx.request_repaint();
            });
            let _ = tx.send(Evento::Fin(r));
            ctx.request_repaint();
        });
    }
    pub fn mostrar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mut fin = None;
        if let Some(rx) = &self.receptor {
            loop {
                match rx.try_recv() {
                    Ok(Evento::Avance(a)) => self.avance = Some(a),
                    Ok(Evento::Fin(r)) => {
                        fin = Some(r);
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(_) => {
                        fin = Some(Err("El proceso se interrumpió; revise la carpeta de destino.".into()));
                        break;
                    }
                }
            }
        }
        if let Some(r) = fin {
            self.receptor = None;
            match r {
                Ok(r) => {
                    self.mensaje = format!(
                        "{} copias verificadas; {} entradas excluidas. Revise INFORME.json y las copias antes de compartir.",
                        r.informe.archivos.len(),
                        r.informe.excluidos
                    );
                    self.resultado = Some(r.carpeta)
                }
                Err(e) => self.mensaje = e,
            }
        }
        ui.heading("Anonimizador");
        ui.label("Crea copias locales con identidades ficticias. Revise siempre las copias antes de compartirlas.");
        ui.add_enabled_ui(self.receptor.is_none(), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Agregar archivos…").clicked() {
                    if let Some(ps) = rfd::FileDialog::new().pick_files() {
                        self.entradas.extend(ps);
                    }
                }
                if ui.button("Agregar carpeta…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        self.entradas.push(p);
                    }
                }
                if ui.button("Vaciar").clicked() {
                    self.entradas.clear();
                }
            });
            egui::ScrollArea::vertical().id_salt("entradas_anon").max_height(130.).show(ui, |ui| {
                for p in &self.entradas {
                    ui.label(p.display().to_string());
                }
            });
            ui.checkbox(&mut self.conservador, "Modo Conservador: conserva más texto; requiere una revisión más cuidadosa");
            ui.horizontal(|ui| {
                ui.label("Desplazar fechas (días):");
                ui.add(egui::DragValue::new(&mut self.dias).range(-36500..=36500));
            });
            ui.checkbox(&mut self.nacimiento, "Generalizar fecha de nacimiento al 1 de enero");
            ui.checkbox(&mut self.rechazar, "Rechazar la selección completa si incluye archivos no admitidos");
            ui.horizontal(|ui| {
                ui.label("Semilla opcional:");
                ui.add(egui::TextEdit::singleline(&mut self.semilla).password(true))
                    .on_hover_text("Permite repetir los mismos seudónimos. Solo permanece en memoria durante esta ejecución.");
            });
            ui.horizontal(|ui| {
                if ui.button("Destino…").clicked() {
                    self.destino = rfd::FileDialog::new().pick_folder();
                }
                ui.label(self.destino.as_ref().map(|p| p.display().to_string()).unwrap_or("Elija una carpeta existente".into()));
            });
            if ui.add_enabled(!self.entradas.is_empty() && self.destino.is_some(), egui::Button::new("Crear copias anónimas")).clicked() {
                self.iniciar(ctx);
            }
        });
        if self.receptor.is_some() {
            if let Some(a) = &self.avance {
                ui.label(format!("{}: {} / {}", a.fase, a.completados, a.total));
                ui.add(egui::ProgressBar::new(a.completados as f32 / a.total.max(1) as f32));
            }
            if ui.button("Cancelar").clicked() {
                self.cancelar.store(true, Ordering::Relaxed);
            }
        }
        if !self.mensaje.is_empty() {
            ui.label(&self.mensaje);
        }
        if let Some(p) = &self.resultado {
            if ui.button("Abrir resultados").clicked() {
                crate::ui::abrir_en_sistema(p);
            }
        }
        ui.separator();
        ui.weak("Formatos: Excel, CSV/TSV, JSON/JSONL, texto, HTML, ICS, DOCX sin objetos y ZIP. Los PDF, imágenes y objetos incrustados se excluyen; nunca se copian sin tratar.");
    }
}
