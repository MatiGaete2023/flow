use crate::theme::{self, Pal};
use crate::util::parse_date_text;
use egui::{ComboBox, RichText, Ui};
use flow_rust::descargador::{Catalogo, Edicion, Pantalla, Perfil, Seleccion};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default)]
pub struct Descargador {
    edicion: Edicion,
    catalogo: Option<Catalogo>,
    seleccion: Seleccion,
    desde: String,
    hasta: String,
    perfil: Option<Perfil>,
    error: Option<String>,
}

impl Descargador {
    pub fn abrir_dialogo(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Abrir catálogo de consultas")
            .add_filter("Catálogo JSON", &["json"])
            .pick_file()
        {
            self.cargar(&path);
        }
    }

    pub fn cargar(&mut self, ruta: &Path) {
        let catalogo = std::fs::File::open(ruta)
            .map_err(|e| e.to_string())
            .and_then(|archivo| {
                serde_json::from_reader(std::io::BufReader::new(archivo)).map_err(|e| e.to_string())
            });
        match catalogo {
            Ok(catalogo) => {
                self.catalogo = Some(catalogo);
                self.seleccion = Seleccion::default();
                self.desde.clear();
                self.hasta.clear();
                self.perfil = None;
                self.error = None;
            }
            Err(e) => self.error = Some(format!("No se pudo abrir el catálogo: {e}")),
        }
    }

    pub fn mostrar(&mut self, ui: &mut Ui, p: &Pal) {
        theme::titulo(
            ui,
            p,
            "Descargador",
            "Perfiles de consulta desde un catálogo local",
        );
        theme::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let edicion_anterior = self.edicion;
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.edicion, Edicion::Integral, "Integral");
                ui.selectable_value(&mut self.edicion, Edicion::Lite, "Lite");
                ui.separator();
                if ui.button("Abrir catálogo JSON…").clicked() {
                    self.abrir_dialogo();
                }
            });
            if edicion_anterior != self.edicion {
                self.perfil = None;
                self.error = None;
            }
            if let Some(catalogo) = &self.catalogo {
                ui.add_space(10.0);
                theme::seccion(ui, p, catalogo.pantalla.etiqueta());
                let anterior = self.seleccion.clone();
                let habilitado =
                    self.edicion == Edicion::Integral || catalogo.pantalla != Pantalla::Carga;
                ui.add_enabled_ui(habilitado, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        mapa(
                            ui,
                            "Tribunal",
                            &catalogo.tribunales,
                            &mut self.seleccion.tribunal,
                        );
                        match catalogo.pantalla {
                            Pantalla::Seguimiento => {
                                opciones(
                                    ui,
                                    "Pestaña",
                                    catalogo.pestanas.iter().map(|s| (s.as_str(), s.as_str())),
                                    &mut self.seleccion.pestana,
                                );
                                if anterior.pestana != self.seleccion.pestana {
                                    self.seleccion.reporte.clear();
                                }
                                mapa(
                                    ui,
                                    "Modalidad",
                                    &catalogo.modalidades,
                                    &mut self.seleccion.modalidad,
                                );
                                match self.seleccion.pestana.as_str() {
                                    "Informes" => mapa(
                                        ui,
                                        "Tipo de informe",
                                        &catalogo.informes,
                                        &mut self.seleccion.reporte,
                                    ),
                                    "Cumplimiento" => mapa(
                                        ui,
                                        "Medidas",
                                        &catalogo.medidas,
                                        &mut self.seleccion.reporte,
                                    ),
                                    _ => {}
                                }
                            }
                            Pantalla::Litigantes => {
                                mapa(ui, "Estado", &catalogo.estados, &mut self.seleccion.estado)
                            }
                            Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas => {
                                mapa(ui, "Mes", &catalogo.meses, &mut self.seleccion.mes);
                                mapa(ui, "Año", &catalogo.anios, &mut self.seleccion.anio);
                            }
                            Pantalla::Carga => {
                                ui.label("Desde");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.desde)
                                        .desired_width(94.0)
                                        .hint_text("dd/mm/aaaa"),
                                );
                                ui.label("Hasta");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.hasta)
                                        .desired_width(94.0)
                                        .hint_text("dd/mm/aaaa"),
                                );
                                self.seleccion.desde = parse_date_text(&self.desde);
                                self.seleccion.hasta = parse_date_text(&self.hasta);
                            }
                        }
                    });
                    if anterior != self.seleccion {
                        self.perfil = None;
                        self.error = None;
                    }
                    ui.add_space(6.0);
                    if theme::primary(ui, p, "Validar perfil").clicked() {
                        match catalogo.preparar(self.edicion, &self.seleccion) {
                            Ok(perfil) => {
                                self.perfil = Some(perfil);
                                self.error = None;
                            }
                            Err(e) => {
                                self.perfil = None;
                                self.error = Some(e);
                            }
                        }
                    }
                });
                if !habilitado {
                    ui.label(RichText::new("Carga está disponible en Integral.").color(p.muted));
                }
            }
            if let Some(perfil) = &self.perfil {
                ui.label(RichText::new(&perfil.etiqueta).color(p.green));
            }
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(p.amber));
            }
        });
    }

    pub fn ejemplo(&mut self) {
        self.catalogo = serde_json::from_value(serde_json::json!({
            "pantalla": "seguimiento", "tribunales": {"ficticio": "Tribunal ficticio"},
            "pestanas": ["Informes"], "modalidades": {"2": "Ambulatorio"},
            "informes": {"i": "Por vencer"},
        }))
        .ok();
        self.seleccion = Seleccion {
            tribunal: "ficticio".into(),
            pestana: "Informes".into(),
            modalidad: "2".into(),
            reporte: "i".into(),
            ..Default::default()
        };
        self.perfil = self
            .catalogo
            .as_ref()
            .and_then(|c| c.preparar(self.edicion, &self.seleccion).ok());
    }
}

fn mapa(ui: &mut Ui, titulo: &str, lista: &BTreeMap<String, String>, valor: &mut String) {
    opciones(
        ui,
        titulo,
        lista.iter().map(|(k, v)| (k.as_str(), v.as_str())),
        valor,
    );
}

fn opciones<'a>(
    ui: &mut Ui,
    titulo: &str,
    lista: impl Iterator<Item = (&'a str, &'a str)>,
    valor: &mut String,
) {
    let lista: Vec<_> = lista.collect();
    let texto = lista
        .iter()
        .find(|(k, _)| *k == valor.as_str())
        .map_or("Seleccionar…", |(_, v)| *v);
    ui.label(titulo);
    ComboBox::from_id_salt(titulo)
        .selected_text(texto)
        .show_ui(ui, |ui| {
            for (k, v) in lista {
                ui.selectable_value(valor, k.to_owned(), v);
            }
        });
}
