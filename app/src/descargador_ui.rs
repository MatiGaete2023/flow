use crate::theme::{self, Pal};
use crate::util::parse_date_text;
use egui::{ComboBox, RichText, Ui};
use flow_rust::descargador::{
    Catalogo, Edicion, Pantalla, Perfil, PlanLote, Puente, Seleccion, SeleccionLote, MODALIDADES,
    PESTANAS, TODAS_LAS_MEDIDAS,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::Duration;

#[derive(Default)]
pub struct Descargador {
    edicion: Edicion,
    catalogo: Option<Catalogo>,
    seleccion: Seleccion,
    desde: String,
    hasta: String,
    perfil: Option<Perfil>,
    error: Option<String>,
    lote: SeleccionLote,
    lote_desde: String,
    lote_hasta: String,
    plan: Option<Result<PlanLote, String>>,
    conexion: Conexion,
    catalogo_remoto: bool,
}

#[derive(Default)]
struct Conexion {
    puente: Option<Arc<Puente>>,
    pantalla: Pantalla,
    carga: Option<mpsc::Receiver<Result<Catalogo, String>>>,
    error: Option<String>,
}

impl Conexion {
    fn cerrar(&mut self) {
        self.carga = None;
        self.error = None;
        if let Some(puente) = self.puente.take() {
            puente.cerrar();
        }
    }

    fn catalogo(&mut self) -> Option<Catalogo> {
        match self.carga.as_ref()?.try_recv() {
            Ok(Ok(catalogo)) => {
                self.carga = None;
                Some(catalogo)
            }
            Ok(Err(e)) => {
                self.carga = None;
                self.error = Some(e);
                None
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.carga = None;
                self.error = Some("Se interrumpió la carga del catálogo.".into());
                None
            }
            Err(mpsc::TryRecvError::Empty) => None,
        }
    }

    fn mostrar(&mut self, ui: &mut Ui, p: &Pal, edicion: Edicion) {
        egui::CollapsingHeader::new("Conexión con Chrome").show(ui, |ui| {
            let Some(puente) = self.puente.clone() else {
                if ui.button("Abrir conexión local").clicked() {
                    match Puente::abrir() {
                        Ok(puente) => {
                            self.puente = Some(Arc::new(puente));
                            self.error = None;
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(p.amber));
                }
                return;
            };
            let conectado = puente.conectado();
            ui.ctx().request_repaint_after(Duration::from_millis(500));
            ui.horizontal_wrapped(|ui| {
                if ui.button("Copiar código").clicked() {
                    ui.ctx().copy_text(puente.codigo_conexion());
                }
                if ui.button("Cerrar conexión").clicked() {
                    self.cerrar();
                }
                if conectado {
                    ui.label(RichText::new("Chrome conectado").color(p.green));
                } else {
                    ui.label("Pega el código en la extensión Integral o Lite de Flow Python.");
                }
            });
            if self.puente.is_none() {
                return;
            }
            ui.add_enabled_ui(conectado && self.carga.is_none(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    let pantallas: &[Pantalla] = match edicion {
                        Edicion::Integral => &Pantalla::INTEGRAL,
                        Edicion::Lite => &Pantalla::LITE,
                    };
                    ComboBox::from_id_salt("pantalla de Chrome")
                        .selected_text(self.pantalla.etiqueta())
                        .show_ui(ui, |ui| {
                            for p in pantallas {
                                ui.selectable_value(&mut self.pantalla, *p, p.etiqueta());
                            }
                        });
                    if ui.button("Cargar opciones de Chrome").clicked() {
                        let (envio, recepcion) = mpsc::channel();
                        let pantalla = self.pantalla;
                        let ctx = ui.ctx().clone();
                        std::thread::spawn(move || {
                            let _ = envio.send(puente.catalogo(edicion, pantalla));
                            ctx.request_repaint();
                        });
                        self.error = None;
                        self.carga = Some(recepcion);
                    }
                });
            });
            if self.carga.is_some() {
                ui.label("Cargando catálogo…");
            }
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(p.amber));
            }
        });
    }
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
                if self.conexion.carga.is_some() {
                    self.conexion.cerrar();
                }
                self.usar_catalogo(Some(catalogo), false);
            }
            Err(e) => self.error = Some(format!("No se pudo abrir el catálogo: {e}")),
        }
    }

    fn usar_catalogo(&mut self, catalogo: Option<Catalogo>, remoto: bool) {
        self.catalogo = catalogo;
        self.catalogo_remoto = remoto;
        self.seleccion = Seleccion::default();
        self.desde.clear();
        self.hasta.clear();
        self.perfil = None;
        self.error = None;
        self.lote = SeleccionLote::default();
        self.lote_desde.clear();
        self.lote_hasta.clear();
        self.plan = None;
    }

    pub fn mostrar(&mut self, ui: &mut Ui, p: &Pal) {
        if let Some(catalogo) = self.conexion.catalogo() {
            self.usar_catalogo(Some(catalogo), true);
        }
        theme::titulo(
            ui,
            p,
            "Descargador",
            "Perfiles de consulta desde Chrome o un catálogo JSON",
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
                self.conexion.cerrar();
                self.conexion.pantalla = Pantalla::default();
                if self.catalogo_remoto {
                    self.usar_catalogo(None, false);
                }
                self.perfil = None;
                self.error = None;
                self.plan = None;
            }
            self.conexion.mostrar(ui, p, self.edicion);
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
                                    "Cumplimiento" => opciones(
                                        ui,
                                        "Medidas",
                                        medidas(catalogo),
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
                                fecha(ui, "Desde", &mut self.desde);
                                fecha(ui, "Hasta", &mut self.hasta);
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
        if let Some(catalogo) = &self.catalogo {
            if catalogo.pantalla != Pantalla::Carga || self.edicion == Edicion::Integral {
                ui.add_space(8.0);
                theme::card(p).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::CollapsingHeader::new("Planificar lote")
                        .default_open(self.plan.is_some())
                        .show(ui, |ui| {
                            mostrar_lote(
                                ui,
                                p,
                                catalogo,
                                self.edicion,
                                &mut self.lote,
                                (&mut self.lote_desde, &mut self.lote_hasta),
                                &mut self.plan,
                            );
                        });
                });
            }
        }
    }

    pub fn ejemplo(&mut self) {
        self.catalogo = serde_json::from_value(serde_json::json!({
            "pantalla": "seguimiento",
            "tribunales": {"ficticio": "Tribunal ficticio norte", "ficticio-sur": "Tribunal ficticio sur"},
            "pestanas": ["Cumplimiento", "Informes"], "modalidades": {"1": "Residencia", "2": "Ambulatorio"},
            "informes": {"i": "Por vencer"}, "medidas": {"m": "Vigentes"},
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
        self.lote = SeleccionLote {
            tribunales: vec!["ficticio".into(), "ficticio-sur".into()],
            pestanas: vec!["Cumplimiento".into(), "Informes".into()],
            modalidades: vec!["2".into()],
            informes: vec!["i".into()],
            medidas: vec![TODAS_LAS_MEDIDAS.into()],
            ..Default::default()
        };
        self.plan = self
            .catalogo
            .as_ref()
            .map(|c| c.planificar(self.edicion, &self.lote));
    }
}

fn mostrar_lote(
    ui: &mut Ui,
    p: &Pal,
    catalogo: &Catalogo,
    edicion: Edicion,
    lote: &mut SeleccionLote,
    (desde, hasta): (&mut String, &mut String),
    plan: &mut Option<Result<PlanLote, String>>,
) {
    let mut cambio = marcas(
        ui,
        "Tribunales",
        pares(&catalogo.tribunales),
        &mut lote.tribunales,
    );
    match catalogo.pantalla {
        Pantalla::Seguimiento => {
            cambio |= marcas(
                ui,
                "Pestañas",
                PESTANAS
                    .into_iter()
                    .filter(|p| catalogo.pestanas.iter().any(|s| s == p))
                    .map(|p| (p, p)),
                &mut lote.pestanas,
            );
            cambio |= marcas(
                ui,
                "Modalidades",
                pares(&catalogo.modalidades).filter(|(k, _)| MODALIDADES.contains(k)),
                &mut lote.modalidades,
            );
            if lote.pestanas.iter().any(|s| s == "Informes") {
                cambio |= marcas(
                    ui,
                    "Tipos de informe",
                    pares(&catalogo.informes),
                    &mut lote.informes,
                );
            }
            if lote.pestanas.iter().any(|s| s == "Cumplimiento") {
                cambio |= marcas(ui, "Medidas", medidas(catalogo), &mut lote.medidas);
            }
        }
        Pantalla::Litigantes => {
            cambio |= marcas(ui, "Estados", pares(&catalogo.estados), &mut lote.estados);
        }
        Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas => {
            cambio |= marcas(ui, "Meses", pares(&catalogo.meses), &mut lote.meses);
            let anterior = lote.anio.clone();
            ui.horizontal_wrapped(|ui| mapa(ui, "Año", &catalogo.anios, &mut lote.anio));
            cambio |= anterior != lote.anio;
        }
        Pantalla::Carga => {}
    }
    if !matches!(
        catalogo.pantalla,
        Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas
    ) {
        cambio |= ui
            .horizontal_wrapped(|ui| fecha(ui, "Desde", desde) | fecha(ui, "Hasta", hasta))
            .inner;
    }
    if cambio {
        *plan = None;
    }
    ui.add_space(6.0);
    if theme::primary(ui, p, "Preparar lote").clicked() {
        *plan = Some(if desde.trim().is_empty() && hasta.trim().is_empty() {
            lote.fechas = None;
            catalogo.planificar(edicion, lote)
        } else if let Some(fechas) = parse_date_text(desde).zip(parse_date_text(hasta)) {
            lote.fechas = Some(fechas);
            catalogo.planificar(edicion, lote)
        } else {
            Err("Completa las dos fechas del período.".into())
        });
    }
    match plan.as_ref() {
        Some(Ok(plan)) => {
            ui.label(
                RichText::new(format!("Consultas preparadas ({})", plan.consultas.len()))
                    .color(p.green),
            );
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for consulta in &plan.consultas {
                        ui.label(&consulta.etiqueta);
                    }
                });
        }
        Some(Err(e)) => {
            ui.label(RichText::new(e).color(p.amber));
        }
        None => {}
    }
}

fn fecha(ui: &mut Ui, titulo: &str, texto: &mut String) -> bool {
    ui.label(titulo);
    ui.add(
        egui::TextEdit::singleline(texto)
            .desired_width(94.0)
            .hint_text("dd/mm/aaaa"),
    )
    .changed()
}

fn marcas<'a>(
    ui: &mut Ui,
    titulo: &str,
    lista: impl Iterator<Item = (&'a str, &'a str)>,
    elegidas: &mut Vec<String>,
) -> bool {
    ui.horizontal_wrapped(|ui| {
        ui.label(titulo);
        let mut cambio = false;
        for (clave, etiqueta) in lista {
            let elegida = elegidas.iter().position(|s| s == clave);
            let mut marcada = elegida.is_some();
            if ui.checkbox(&mut marcada, etiqueta).changed() {
                if let Some(pos) = elegida {
                    elegidas.remove(pos);
                } else {
                    elegidas.push(clave.to_owned());
                }
                cambio = true;
            }
        }
        cambio
    })
    .inner
}

fn pares(lista: &BTreeMap<String, String>) -> impl Iterator<Item = (&str, &str)> {
    lista.iter().map(|(k, v)| (k.as_str(), v.as_str()))
}

fn medidas(catalogo: &Catalogo) -> impl Iterator<Item = (&str, &str)> {
    std::iter::once((TODAS_LAS_MEDIDAS, "Todas las medidas"))
        .chain(pares(&catalogo.medidas).filter(|(k, _)| *k != TODAS_LAS_MEDIDAS))
}

fn mapa(ui: &mut Ui, titulo: &str, lista: &BTreeMap<String, String>, valor: &mut String) {
    opciones(ui, titulo, pares(lista), valor);
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
