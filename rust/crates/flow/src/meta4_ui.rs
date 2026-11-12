//! Pestaña Meta 4: lee lotes de captura locales y genera la planilla y sus consolidados. No se conecta a nada.
use eframe::egui::{self, Color32, RichText};
use flow_dominio::celda::fecha_de_texto;
use flow_meta4::{calcular, salida, tribunales::TRIBUNALES};
use std::path::PathBuf;

pub struct Meta4 {
    lotes: Vec<PathBuf>,
    corte: String,
    elegidos: Vec<bool>,
    destino: Option<PathBuf>,
    mensaje: Option<(bool, String)>,
    avisos: Vec<String>,
    resultado: Option<PathBuf>,
}

impl Default for Meta4 {
    fn default() -> Self {
        Self {
            lotes: Vec::new(),
            corte: flow_tabular::libro::hoy().format("%d/%m/%Y").to_string(),
            elegidos: vec![false; TRIBUNALES.len()],
            destino: None,
            mensaje: None,
            avisos: Vec::new(),
            resultado: None,
        }
    }
}

impl Meta4 {
    fn generar(&mut self) {
        self.resultado = None;
        self.avisos.clear();
        let Some(corte) = fecha_de_texto(&self.corte) else {
            self.mensaje = Some((true, "La fecha de corte no se entiende. Usa dd/mm/aaaa.".into()));
            return;
        };
        if corte > flow_tabular::libro::hoy() {
            self.mensaje = Some((true, "La fecha de corte no puede ser futura.".into()));
            return;
        }
        if self.destino.is_none() {
            self.destino = rfd::FileDialog::new().set_title("Carpeta donde dejar los resultados de Meta 4").pick_folder();
        }
        let Some(base) = self.destino.clone() else { return };
        let elegidos: Vec<String> =
            TRIBUNALES.iter().zip(&self.elegidos).filter(|(_, si)| **si).map(|(t, _)| t.to_string()).collect();
        let resultado = calcular(&self.lotes, corte, flow_tabular::libro::hoy(), &elegidos).and_then(|m| {
            let mut destino = base.join(format!("Resultado META4 {}", corte.format("%Y-%m-%d")));
            let mut n = 2;
            while destino.exists() {
                destino = base.join(format!("Resultado META4 {} ({n})", corte.format("%Y-%m-%d")));
                n += 1;
            }
            salida::publicar(&m, &destino).map(|_| (m, destino))
        });
        match resultado {
            Ok((m, destino)) => {
                let alertas = m.registros.iter().filter(|r| !r.alerta.is_empty()).count();
                let revisar = m.registros.iter().filter(|r| r.requiere_revision()).count();
                self.mensaje = Some((
                    false,
                    format!("Meta 4 generado: {} registros, {alertas} alertas, {revisar} por revisar.", m.registros.len()),
                ));
                self.avisos = m.avisos;
                self.resultado = Some(destino);
            }
            Err(e) => self.mensaje = Some((true, e)),
        }
    }

    pub fn mostrar(&mut self, ui: &mut egui::Ui) {
        ui.label("Genera la planilla Meta 4 con lo que ya capturó Flow: Espera y Cumplimiento de FAE y Residencia, con sus bitácoras.");
        ui.add_space(4.0);
        ui.label(RichText::new("Lotes de registros").strong());
        ui.horizontal(|ui| {
            if ui.button("Agregar lote…").on_hover_text("La carpeta del lote, la que contiene resumen.json").clicked() {
                for carpeta in rfd::FileDialog::new().set_title("Carpeta del lote capturado").pick_folders().unwrap_or_default() {
                    if !self.lotes.contains(&carpeta) {
                        self.lotes.push(carpeta);
                    }
                }
            }
            if !self.lotes.is_empty() && ui.button("Vaciar").clicked() {
                self.lotes.clear();
            }
        });
        let mut quitar = None;
        for (i, lote) in self.lotes.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.small_button("Quitar").clicked() {
                    quitar = Some(i);
                }
                ui.label(lote.display().to_string());
            });
        }
        if let Some(i) = quitar {
            self.lotes.remove(i);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Fecha de corte:");
            ui.add(egui::TextEdit::singleline(&mut self.corte).desired_width(86.0)).on_hover_text("dd/mm/aaaa. Se usa el mes que termina ese día.");
        });
        ui.label(RichText::new("Tribunales (sin marcar ninguno, van todos)").strong());
        ui.horizontal_wrapped(|ui| {
            for (nombre, marcado) in TRIBUNALES.iter().zip(self.elegidos.iter_mut()) {
                ui.checkbox(marcado, *nombre);
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.lotes.is_empty(), egui::Button::new("Generar Meta 4")).clicked() {
                self.generar();
            }
            if let Some(carpeta) = &self.resultado {
                if ui.button("Abrir resultados").clicked() {
                    crate::ui::abrir_en_sistema(carpeta);
                }
            }
            if ui.button("Cambiar carpeta de salida…").clicked() {
                self.destino = rfd::FileDialog::new().pick_folder().or(self.destino.take());
            }
        });
        match &self.mensaje {
            Some((true, texto)) => {
                ui.colored_label(Color32::from_rgb(190, 40, 40), texto);
            }
            Some((false, texto)) => {
                ui.label(texto);
            }
            None => {}
        }
        for aviso in &self.avisos {
            ui.colored_label(Color32::from_rgb(170, 110, 0), format!("• {aviso}"));
        }
        ui.add_space(6.0);
        ui.weak("Rojo: última observación en Espera y registro en Cumplimiento. Amarillo: revisar el control de bitácora. La fecha es la de la observación del centro; Informes no participa.");
    }
}
