//! Pestaña Meta 4: lee lotes de captura locales y genera la planilla y sus consolidados. No se conecta a nada.
use eframe::egui::{self, Color32, RichText};
use flow_dominio::celda::fecha_de_texto;
use flow_meta4::{calcular, fecha_lectura, regenerar, salida, tribunales::TRIBUNALES, tribunales_en_lotes};
use std::path::PathBuf;

pub struct Meta4 {
    lotes: Vec<PathBuf>,
    corte: String,
    elegidos: Vec<bool>,
    /// Tribunales presentes en los lotes agregados: solo esos se ofrecen (como `tribunales_disponibles` de Python).
    disponibles: Vec<&'static str>,
    destino: Option<PathBuf>,
    mensaje: Option<(bool, String)>,
    avisos: Vec<String>,
    resultado: Option<PathBuf>,
    plantilla: Option<PathBuf>,
}

impl Default for Meta4 {
    fn default() -> Self {
        Self {
            lotes: Vec::new(),
            corte: flow_tabular::libro::hoy().format("%d/%m/%Y").to_string(),
            elegidos: vec![false; TRIBUNALES.len()],
            disponibles: Vec::new(),
            destino: None,
            mensaje: None,
            avisos: Vec::new(),
            resultado: None,
            plantilla: None,
        }
    }
}

impl Meta4 {
    /// Al cambiar los lotes: propone el corte según el día de lectura de sus bitácoras (`suggest_cutoff` de Python)
    /// y deja elegir solo los tribunales presentes.
    fn lotes_cambiados(&mut self) {
        self.mensaje = None;
        let hoy = flow_tabular::libro::hoy();
        if let Some(dia) = fecha_lectura(&self.lotes).filter(|d| *d < hoy) {
            self.corte = dia.format("%d/%m/%Y").to_string();
            self.mensaje = Some((false, format!("Corte ajustado al día de lectura de las bitácoras: {}.", dia.format("%d-%m-%Y"))));
        }
        self.disponibles = if self.lotes.is_empty() {
            Vec::new()
        } else {
            match tribunales_en_lotes(&self.lotes) {
                Ok(t) => t,
                Err(e) => {
                    self.mensaje = Some((true, e));
                    Vec::new()
                }
            }
        };
        for (t, marcado) in TRIBUNALES.iter().zip(self.elegidos.iter_mut()) {
            *marcado &= self.disponibles.contains(t);
        }
    }

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
            salida::publicar_con_plantilla(&m, &destino, self.plantilla.as_deref()).map(|_| (m, destino))
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

    /// Reconstruye desde Registros.xlsx revisado, sin volver a capturar ni modificar el origen.
    fn regenerar(&mut self) {
        let Some(archivo) = rfd::FileDialog::new()
            .set_title("Elegir Registros.xlsx revisado")
            .add_filter("Libro Excel", &["xlsx"])
            .pick_file() else { return };
        if self.destino.is_none() {
            self.destino = rfd::FileDialog::new()
                .set_title("Carpeta donde dejar la regeneración")
                .pick_folder();
        }
        let Some(base) = self.destino.clone() else { return };
        self.avisos.clear();
        self.resultado = None;
        let cargado = regenerar::cargar(&archivo).map(|c| (c.meta.corte, c.bitacoras.is_some()));
        let resultado = cargado.and_then(|(corte, con_bitacoras)| {
            let mut destino = base.join(format!("META4 regenerado {}", corte.format("%Y-%m-%d")));
            let mut n = 2;
            while destino.exists() {
                destino = base.join(format!("META4 regenerado {} ({n})", corte.format("%Y-%m-%d")));
                n += 1;
            }
            regenerar::ejecutar_con_plantilla(&archivo, &destino, self.plantilla.as_deref()).map(|_| (destino, con_bitacoras))
        });
        match resultado {
            Ok((destino, con_bitacoras)) => {
                let bitacoras = if con_bitacoras {
                    "Se conservaron las bitácoras."
                } else {
                    "No había Bitacoras_del_mes.xlsx junto a Registros.xlsx: el resultado no la incluye."
                };
                self.mensaje = Some((false, format!("Meta 4 regenerado desde Registros.xlsx revisado. {bitacoras}")));
                self.resultado = Some(destino);
            }
            Err(e) => self.mensaje = Some((true, e)),
        }
    }

    pub fn mostrar(&mut self, ui: &mut egui::Ui) {
        ui.label("Genera la planilla Meta 4 con lo que ya capturó Flow: Espera y Cumplimiento de FAE y Residencia, con sus bitácoras.");
        ui.add_space(4.0);
        ui.label(RichText::new("Lotes de registros").strong());
        ui.weak("Una carpeta con varios lotes (subcarpetas con resumen.json) los agrega todos.");
        ui.horizontal(|ui| {
            if ui.button("Agregar lote…").on_hover_text("La carpeta del lote, la que contiene resumen.json").clicked() {
                let mut agregados = false;
                for carpeta in rfd::FileDialog::new().set_title("Carpeta del lote capturado").pick_folders().unwrap_or_default() {
                    if !self.lotes.contains(&carpeta) {
                        self.lotes.push(carpeta);
                        agregados = true;
                    }
                }
                if agregados {
                    self.lotes_cambiados();
                }
            }
            if !self.lotes.is_empty() && ui.button("Vaciar").clicked() {
                self.lotes.clear();
                self.lotes_cambiados();
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
            self.lotes_cambiados();
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Fecha de corte:");
            ui.add(egui::TextEdit::singleline(&mut self.corte).desired_width(86.0)).on_hover_text("dd/mm/aaaa. Se usa el mes que termina ese día.");
        });
        ui.label(RichText::new("Tribunales (sin marcar ninguno, van todos)").strong());
        ui.horizontal_wrapped(|ui| {
            if self.disponibles.is_empty() {
                ui.weak("Agrega lotes para ver sus tribunales.");
            }
            for (nombre, marcado) in TRIBUNALES.iter().zip(self.elegidos.iter_mut()) {
                if self.disponibles.contains(nombre) {
                    ui.checkbox(marcado, *nombre);
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Elegir plantilla Excel…").clicked() {
                if let Some(plantilla) = rfd::FileDialog::new().add_filter("Plantilla Excel", &["xlsx"]).pick_file() {
                    self.plantilla = Some(plantilla);
                }
            }
            if self.plantilla.is_some() && ui.button("Sin plantilla").clicked() {
                self.plantilla = None;
            }
            if let Some(plantilla) = &self.plantilla {
                ui.label(format!("Plantilla: {}", plantilla.file_name().unwrap_or_default().to_string_lossy()));
            } else {
                ui.weak("Formato estándar");
            }
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.lotes.is_empty(), egui::Button::new("Generar Meta 4")).clicked() {
                self.generar();
            }
            if ui.button("Regenerar desde Registros revisado…")
                .on_hover_text("Usa Registros.xlsx y Reporte META4.json del resultado anterior; no cambia los originales.")
                .clicked() {
                self.regenerar();
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
