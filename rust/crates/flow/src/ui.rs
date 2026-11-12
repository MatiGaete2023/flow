//! Ventana de Flow: Trabajo → Correos → Resoluciones → Resultados.
//! Todo queda en el equipo. No escribe en RUS ni SATURNO y no envía correos.

use crate::cli;
use eframe::egui::{self, Color32, Key, RichText};
use egui_extras::{Column, TableBuilder};
use flow_dominio::celda::fecha_de_texto;
use flow_dominio::{Etapa, Fecha, CC_FIJO};
use flow_motor::Config;
use flow_productos::correos::{self, Borrador, Contactos, Tipo};
use flow_productos::{excel, word};
use flow_trabajo::{almacen, FilaTrabajo, Trabajo};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pestana {
    Captura,
    Trabajo,
    Correos,
    Resoluciones,
    Resultados,
    Contactos,
    Anonimo,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filtro {
    Todos,
    PorRevisar,
    ConAccion,
    Editados,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Ajustes {
    salida: Option<PathBuf>,
}

struct App {
    datos: PathBuf,
    ajustes: Ajustes,
    contactos: Contactos,
    motor: Config,
    trabajo: Option<Trabajo>,
    /// Otras etapas del mismo archivo (un paquete o un consolidado traen varias).
    otros: Vec<Trabajo>,
    archivo: Option<PathBuf>,
    corte: String,
    etapa: Option<Etapa>,
    pestana: Pestana,
    seleccion: Option<usize>,
    filtro: Filtro,
    buscar: String,
    modalidades: Vec<(String, bool)>,
    otra_modalidad: String,
    borradores: Vec<Borrador>,
    proyectos: Vec<word::Proyecto>,
    resultados: Vec<String>,
    carpeta: Option<PathBuf>,
    mensaje: Option<(bool, String)>,
    outlook: bool,
    nuevo: (String, String),
    ayuda: bool,
    sin_guardar: bool,
    captura: crate::captura_ui::Captura,
    anonimo: crate::anonimo_ui::Anonimo,
}

fn ruta_ajustes(datos: &Path) -> PathBuf {
    datos.join("ajustes.json")
}

fn ruta_contactos(datos: &Path) -> PathBuf {
    datos.join("contactos.json")
}

pub(crate) fn abrir_en_sistema(ruta: &Path) {
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(ruta).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(ruta).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(ruta).spawn();
}

fn coincide(fila: &FilaTrabajo, buscar: &str) -> bool {
    if buscar.trim().is_empty() {
        return true;
    }
    let b = flow_dominio::texto::normalizar(buscar);
    ["rit", "nombre", "programa", "rut", "tribunal"]
        .iter()
        .any(|c| flow_dominio::texto::normalizar(&fila.registro.texto(c)).contains(&b))
        || flow_dominio::texto::normalizar(&fila.observacion).contains(&b)
}

impl App {
    fn nueva(archivo: Option<PathBuf>) -> Self {
        Self::con_datos(almacen::carpeta_datos(), archivo)
    }

    fn con_datos(datos: PathBuf, archivo: Option<PathBuf>) -> Self {
        let _ = std::fs::create_dir_all(&datos);
        almacen::aplicar_retencion(&datos, std::time::SystemTime::now());
        let ajustes = std::fs::read_to_string(ruta_ajustes(&datos))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let mut app = App {
            contactos: Contactos::cargar(&ruta_contactos(&datos)),
            datos,
            ajustes,
            motor: Config::default(),
            trabajo: None,
            otros: Vec::new(),
            archivo: None,
            corte: flow_tabular::libro::hoy().format("%d/%m/%Y").to_string(),
            etapa: None,
            pestana: Pestana::Trabajo,
            seleccion: None,
            filtro: Filtro::Todos,
            buscar: String::new(),
            modalidades: correos::modalidades_base().into_iter().map(|m| (m, false)).collect(),
            otra_modalidad: String::new(),
            borradores: Vec::new(),
            proyectos: Vec::new(),
            resultados: Vec::new(),
            carpeta: None,
            mensaje: None,
            outlook: flow_outlook::disponible(),
            nuevo: Default::default(),
            ayuda: false,
            sin_guardar: false,
            captura: Default::default(),
            anonimo: Default::default(),
        };
        if let Some(ruta) = archivo {
            app.cargar(ruta);
        }
        app
    }

    fn bien(&mut self, texto: impl Into<String>) {
        self.mensaje = Some((false, texto.into()));
    }

    fn mal(&mut self, texto: impl Into<String>) {
        self.mensaje = Some((true, texto.into()));
    }

    fn fecha_corte(&self) -> Option<Fecha> {
        fecha_de_texto(&self.corte)
    }

    fn cargar(&mut self, ruta: PathBuf) {
        let Some(corte) = self.fecha_corte() else {
            return self.mal("La fecha de corte no se entiende. Usa dd/mm/aaaa.");
        };
        match cli::cargar(&ruta, self.etapa, None, corte, &self.motor) {
            Ok(mut trabajos) => {
                let trabajo = trabajos.remove(0);
                let mut texto =
                    format!("{}: {} registros, {} con observación.", trabajo.nombre(), trabajo.filas.len(), trabajo.con_observacion());
                if !trabajos.is_empty() {
                    let nombres: Vec<&str> = trabajos.iter().map(|t| t.nombre()).collect();
                    texto.push_str(&format!(" El archivo trae además: {}.", nombres.join(", ")));
                }
                self.activar(trabajo);
                self.otros = trabajos;
                self.archivo = Some(ruta);
                self.pestana = Pestana::Trabajo;
                self.guardar_sesion();
                self.bien(texto);
            }
            Err(e) => self.mal(e),
        }
    }

    /// Deja un trabajo como el activo y descarta los productos preparados del anterior.
    fn activar(&mut self, trabajo: Trabajo) {
        self.seleccion = (!trabajo.filas.is_empty()).then_some(0);
        self.trabajo = Some(trabajo);
        self.borradores.clear();
        self.proyectos.clear();
        self.carpeta = None;
    }

    /// Cambia a otra etapa del mismo archivo conservando las ediciones de ambas.
    fn cambiar_etapa(&mut self, indice: usize) {
        self.guardar_sesion();
        let nuevo = self.otros.remove(indice);
        if let Some(actual) = self.trabajo.take() {
            self.otros.push(actual);
        }
        let texto = format!("{}: {} registros.", nuevo.nombre(), nuevo.filas.len());
        self.activar(nuevo);
        self.bien(texto);
    }

    fn elegir_archivo(&mut self) {
        let dialogo = rfd::FileDialog::new().add_filter("Excel de RUS o paquete del Descargador", &["xls", "xlsx", "xlsm", "zip"]).set_title("Elegir Excel de RUS o paquete ZIP");
        if let Some(ruta) = dialogo.pick_file() {
            self.cargar(ruta);
        }
    }

    fn importar_captura(&mut self, carpeta: PathBuf, resumen: flow_captura::lotes::Resumen) {
        let Some(corte) = self.fecha_corte() else {
            return self.mal("Revisa la fecha de corte antes de llevar la captura a Trabajo.");
        };
        let mut trabajos = match flow_trabajo::captura::importar(&carpeta, &resumen, corte, &self.motor) {
            Ok(t) => t,
            Err(e) => return self.mal(e),
        };
        for archivo in &resumen.archivos {
            self.resultados.push(format!("Descarga verificada: {}", carpeta.join(&archivo.nombre).display()));
        }
        if trabajos.is_empty() {
            self.pestana = Pestana::Resultados;
            return self.bien("Las consultas están verificadas en la carpeta del lote. No traen tablas de intervención para analizar.");
        }
        self.guardar_sesion();
        let actual = trabajos.remove(0);
        self.activar(actual);
        self.otros = trabajos;
        self.archivo = None;
        self.pestana = Pestana::Trabajo;
        self.guardar_sesion();
        self.bien("Captura cargada. Revisa las observaciones antes de preparar los productos.");
    }

    fn guardar_sesion(&mut self) {
        if let Some(t) = &self.trabajo {
            if let Err(e) = almacen::guardar(&self.datos, t) {
                self.mal(e);
            } else {
                self.sin_guardar = false;
            }
        }
    }

    fn reanudar(&mut self) {
        match almacen::listar(&self.datos).first().map(|(r, _)| almacen::cargar(r)) {
            Some(Ok(t)) => {
                self.corte = t.corte.format("%d/%m/%Y").to_string();
                let texto = format!("Trabajo reanudado: {} del {} ({}).", t.nombre(), self.corte, t.archivo);
                self.activar(t);
                self.otros.clear();
                self.archivo = None;
                self.bien(texto);
            }
            Some(Err(e)) => self.mal(e),
            None => self.mal("No hay un trabajo anterior guardado."),
        }
    }

    /// Carpeta de productos de este trabajo; se crea una vez y se reutiliza.
    fn carpeta_productos(&mut self) -> Option<PathBuf> {
        if let Some(c) = &self.carpeta {
            return Some(c.clone());
        }
        let trabajo = self.trabajo.as_ref()?;
        if self.ajustes.salida.is_none() {
            let elegida = rfd::FileDialog::new().set_title("Carpeta donde Flow guardará los productos").pick_folder()?;
            self.ajustes.salida = Some(elegida);
            if let Ok(texto) = serde_json::to_string_pretty(&self.ajustes) {
                let _ = std::fs::write(ruta_ajustes(&self.datos), texto);
            }
        }
        let base = self.ajustes.salida.clone()?;
        let nombre = format!("Flow {} {}", trabajo.nombre(), trabajo.corte.format("%Y-%m-%d"));
        match excel::carpeta_nueva(&base, &nombre) {
            Ok(carpeta) => {
                self.carpeta = Some(carpeta.clone());
                Some(carpeta)
            }
            Err(e) => {
                self.mal(e);
                None
            }
        }
    }

    fn guardar_excel(&mut self) {
        let Some(carpeta) = self.carpeta_productos() else { return };
        let Some(trabajo) = &self.trabajo else { return };
        let destino = excel::ruta_nueva(&carpeta, &format!("Observaciones {}", trabajo.nombre()), "xlsx");
        match excel::observaciones(trabajo, &destino) {
            Ok(()) => {
                self.resultados.push(format!("Excel de observaciones · {}", destino.display()));
                self.bien(format!("Excel guardado: {}", destino.display()));
            }
            Err(e) => self.mal(e),
        }
    }

    fn texto_modalidades(&self) -> String {
        let mut lista: Vec<String> = self.modalidades.iter().filter(|(_, si)| *si).map(|(m, _)| m.clone()).collect();
        if !self.otra_modalidad.trim().is_empty() {
            lista.push(self.otra_modalidad.trim().to_string());
        }
        match lista.len() {
            0 => String::new(),
            1 => lista.remove(0),
            n => format!("{} y {}", lista[..n - 1].join(", "), lista[n - 1]),
        }
    }

    fn preparar_borradores(&mut self) {
        let Some(carpeta) = self.carpeta_productos() else { return };
        let Some(trabajo) = &self.trabajo else { return };
        let opciones = correos::Opciones { modalidades: self.texto_modalidades(), hoy: Some(flow_tabular::libro::hoy()) };
        let mut lista = Vec::new();
        let mut saltados = Vec::new();
        for tipo in Tipo::TODOS {
            if tipo.usa_modalidades(trabajo.etapa) && opciones.modalidades.is_empty() {
                saltados.push(tipo.titulo(trabajo.etapa));
                continue;
            }
            match correos::preparar(trabajo, tipo, &opciones, &self.contactos, &carpeta.join("correos")) {
                Ok(mut nuevos) => lista.append(&mut nuevos),
                Err(e) => return self.mal(e),
            }
        }
        let texto = if saltados.is_empty() {
            format!("{} borradores preparados. Revisa cada uno antes de guardarlo.", lista.len())
        } else {
            format!(
                "{} borradores preparados. Sin modalidades marcadas no se preparan: {}.",
                lista.len(),
                saltados.join("; ")
            )
        };
        self.borradores = lista;
        self.bien(texto);
    }

    /// Guarda los borradores listos. En Outlook quedan en Borradores; si no, como archivos .eml sin enviar.
    fn guardar_borradores(&mut self, en_outlook: bool) {
        let Some(carpeta) = self.carpeta_productos() else { return };
        let mut hechos = 0;
        let mut problemas = Vec::new();
        let mut aprendidos = 0;
        for b in self.borradores.iter().filter(|b| !b.omitido) {
            let quien = if b.programa.is_empty() { b.tribunal.clone() } else { b.programa.clone() };
            if !b.faltantes().is_empty() {
                problemas.push(format!("{quien}: falta {}", b.faltantes().join(", ")));
                continue;
            }
            let resultado = if en_outlook {
                flow_outlook::guardar_borrador(b).map(|r| match r {
                    flow_outlook::Resultado::Guardado => format!("Borrador en Outlook · {} → {quien}", b.asunto),
                    flow_outlook::Resultado::Incierto(m) => format!("REVISAR en Outlook · {quien}: {m}"),
                })
            } else {
                correos::guardar_eml(b, &carpeta.join("correos")).map(|r| format!("Borrador .eml · {}", r.display()))
            };
            match resultado {
                Ok(linea) => {
                    hechos += 1;
                    self.resultados.push(linea);
                    // Dirección escrita a mano para un programa sin contacto: se recuerda para el próximo lote.
                    if !b.programa.is_empty() && self.contactos.de_programa(&b.programa).is_empty() {
                        self.contactos.programas.insert(b.programa.clone(), correos::direcciones(&b.para).join("; "));
                        aprendidos += 1;
                    }
                }
                Err(e) => problemas.push(format!("{quien}: {e}")),
            }
        }
        if aprendidos > 0 {
            let _ = self.contactos.guardar(&ruta_contactos(&self.datos));
        }
        let destino = if en_outlook { "en Borradores de Outlook" } else { "como archivos .eml" };
        if problemas.is_empty() {
            self.bien(format!("{hechos} borradores guardados {destino}. Ninguno fue enviado."));
        } else {
            self.mal(format!("{hechos} guardados {destino}. Pendientes: {}.", problemas.join("; ")));
        }
    }

    fn plantillas_usuario(&self) -> PathBuf {
        self.datos.join("plantillas")
    }

    fn preparar_proyectos(&mut self) {
        let Some(trabajo) = &self.trabajo else { return };
        let propias = self.plantillas_usuario();
        self.proyectos = word::proyectos(trabajo, flow_tabular::libro::hoy(), Some(&propias));
        let listos = self.proyectos.iter().filter(|p| p.listo()).count();
        self.bien(format!("Proyectos: {}. Listos para generar: {listos}.", self.proyectos.len()));
    }

    fn generar_word(&mut self) {
        let Some(carpeta) = self.carpeta_productos() else { return };
        let propias = self.plantillas_usuario();
        match word::generar(&self.proyectos, &carpeta.join("proyectos"), Some(&propias)) {
            Ok(rutas) => {
                for r in &rutas {
                    self.resultados.push(format!("Proyectos Word · {}", r.display()));
                }
                self.bien(format!("{} documento(s) Word creados. Revísalos antes de usarlos.", rutas.len()));
            }
            Err(e) => self.mal(e),
        }
    }

    fn barra(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Flow");
            ui.separator();
            if ui.button("Abrir Excel o paquete…  (Ctrl+O)").clicked() {
                self.elegir_archivo();
            }
            if ui.button("Reanudar último").clicked() {
                self.reanudar();
            }
            ui.separator();
            ui.label("Corte:");
            ui.add(egui::TextEdit::singleline(&mut self.corte).desired_width(86.0)).on_hover_text("dd/mm/aaaa");
            ui.label("Etapa:");
            let actual = self.etapa.map_or("Automática", Etapa::nombre);
            egui::ComboBox::from_id_salt("etapa").selected_text(actual).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.etapa, None, "Automática");
                for e in [Etapa::Espera, Etapa::Cumplimiento, Etapa::Informes] {
                    ui.selectable_value(&mut self.etapa, Some(e), e.nombre());
                }
            });
            if self.archivo.is_some() && ui.button("Volver a analizar").on_hover_text("Descarta las ediciones").clicked() {
                let ruta = self.archivo.clone().unwrap();
                self.cargar(ruta);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Ayuda (F1)").clicked() {
                    self.ayuda = true;
                }
            });
        });
        ui.horizontal(|ui| {
            for (p, nombre) in [
                (Pestana::Captura, "Captura"),
                (Pestana::Trabajo, "1 · Trabajo"),
                (Pestana::Correos, "2 · Correos"),
                (Pestana::Resoluciones, "3 · Resoluciones"),
                (Pestana::Resultados, "4 · Resultados"),
                (Pestana::Contactos, "Contactos"),
                (Pestana::Anonimo, "Anonimizador"),
            ] {
                ui.selectable_value(&mut self.pestana, p, nombre);
            }
        });
    }

    fn pie(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            match &self.mensaje {
                Some((true, texto)) => ui.colored_label(Color32::from_rgb(190, 40, 40), texto),
                Some((false, texto)) => ui.label(texto),
                None => ui.label("Abre un Excel de RUS para comenzar."),
            };
        });
        ui.horizontal(|ui| {
            ui.weak("No escribe en RUS ni SATURNO · No envía correos · CC fijo:");
            ui.label(CC_FIJO);
        });
    }

    fn vista_trabajo(&mut self, ui: &mut egui::Ui) {
        let Some(trabajo) = &mut self.trabajo else {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.heading("Abre el Excel de RUS o el paquete ZIP del Descargador");
                ui.label("Flow lee Espera, Cumplimiento o Informes, propone la observación de cada registro\ny prepara el Excel, los borradores de correo y los proyectos de resolución.");
            });
            return;
        };
        let mut guardar_excel = false;
        let mut cambiar = None;
        ui.horizontal_wrapped(|ui| {
            for (i, otro) in self.otros.iter().enumerate() {
                if ui.button(format!("Ver {} ({})", otro.nombre(), otro.filas.len())).clicked() {
                    cambiar = Some(i);
                }
            }
            ui.label(format!(
                "{} · «{}» · {} registros · {} por revisar",
                trabajo.nombre(),
                trabajo.archivo,
                trabajo.filas.len(),
                trabajo.por_revisar()
            ));
            ui.separator();
            for (f, nombre) in [
                (Filtro::Todos, "Todos"),
                (Filtro::PorRevisar, "Por revisar"),
                (Filtro::ConAccion, "Con correo o proyecto"),
                (Filtro::Editados, "Editados"),
            ] {
                ui.selectable_value(&mut self.filtro, f, nombre);
            }
            ui.add(egui::TextEdit::singleline(&mut self.buscar).hint_text("Buscar RIT, nombre, programa…").desired_width(190.0));
            if ui.button("Guardar Excel de observaciones  (Ctrl+S)").clicked() {
                guardar_excel = true;
            }
        });
        for aviso in &trabajo.avisos {
            ui.colored_label(Color32::from_rgb(170, 110, 0), aviso);
        }
        let filtro = self.filtro;
        let buscar = self.buscar.clone();
        let visibles: Vec<usize> = trabajo
            .filas
            .iter()
            .enumerate()
            .filter(|(_, f)| match filtro {
                Filtro::Todos => true,
                Filtro::PorRevisar => !f.revisada && (!f.propuesta.marcas.is_empty() || !f.propuesta.avisos.is_empty()),
                Filtro::ConAccion => !f.propuesta.acciones.is_empty(),
                Filtro::Editados => f.editada(),
            })
            .filter(|(_, f)| coincide(f, &buscar))
            .map(|(i, _)| i)
            .collect();
        let mut seleccion = self.seleccion;
        let mut cambio = false;
        egui::SidePanel::right("detalle").resizable(true).default_width(410.0).show_inside(ui, |ui| {
            let Some(fila) = seleccion.and_then(|i| trabajo.filas.get_mut(i)) else {
                ui.label("Elige un registro de la lista.");
                return;
            };
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(format!("{} · {}", fila.registro.texto("rit"), excel::valor(fila, "nombre")));
                ui.label(format!("{} · {} · fila {} del Excel", fila.registro.texto("programa"), fila.registro.texto("tribunal"), fila.numero));
                ui.add_space(6.0);
                ui.label(RichText::new("Observación").strong());
                if ui.add(egui::TextEdit::multiline(&mut fila.observacion).desired_width(f32::INFINITY).desired_rows(7)).changed() {
                    cambio = true;
                }
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut fila.revisada, "Revisada").changed() {
                        cambio = true;
                    }
                    if fila.editada() && ui.button("Restaurar la propuesta").clicked() {
                        fila.observacion = fila.propuesta.observacion.clone();
                        cambio = true;
                    }
                    if ui.button("Copiar").clicked() {
                        ui.ctx().copy_text(fila.observacion.clone());
                    }
                });
                if fila.editada() {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Propuesta original").strong());
                    ui.label(if fila.propuesta.observacion.is_empty() { "(sin observación)" } else { &fila.propuesta.observacion });
                }
                if !fila.propuesta.marcas.is_empty() || !fila.propuesta.avisos.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Revisar").strong());
                    for m in &fila.propuesta.marcas {
                        let color = match m.nivel {
                            1 => Color32::from_rgb(190, 40, 40),
                            2 => Color32::from_rgb(170, 110, 0),
                            _ => ui.visuals().text_color(),
                        };
                        ui.colored_label(color, format!("● {}", m.texto));
                    }
                    for a in &fila.propuesta.avisos {
                        ui.label(format!("· {a}"));
                    }
                }
                crate::cronologia_ui::mostrar(ui, fila, &self.motor, trabajo.corte);
                if !fila.calendario.is_empty() {
                    ui.collapsing("Calendario del ingreso", |ui| {
                        for p in &fila.calendario {
                            ui.label(format!("{} · {}", p.fecha.format("%d/%m/%Y"), p.estado));
                        }
                    });
                }
                if !fila.propuesta.acciones.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Acciones que origina").strong());
                    ui.label(fila.propuesta.acciones.join(", "));
                }
                ui.add_space(6.0);
                ui.collapsing("Reglas aplicadas y datos de RUS", |ui| {
                    ui.small(fila.propuesta.reglas.join(", "));
                    for (campo, celda) in &fila.registro.campos {
                        ui.small(format!("{campo}: {}", celda.texto()));
                    }
                });
            });
        });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let alto = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
            TableBuilder::new(ui)
                .striped(true)
                .sense(egui::Sense::click())
                .column(Column::exact(44.0))
                .column(Column::exact(96.0))
                .column(Column::initial(140.0).clip(true).resizable(true))
                .column(Column::initial(120.0).clip(true).resizable(true))
                .column(Column::exact(54.0))
                .column(Column::remainder().clip(true))
                .header(alto, |mut h| {
                    for titulo in ["Fila", "RIT", "Nombre", "Programa", "Estado", "Observación"] {
                        h.col(|ui| {
                            ui.strong(titulo);
                        });
                    }
                })
                .body(|body| {
                    body.rows(alto, visibles.len(), |mut row| {
                        let i = visibles[row.index()];
                        let f = &trabajo.filas[i];
                        row.set_selected(seleccion == Some(i));
                        let mut estado = String::new();
                        if f.revisada {
                            estado.push('✔');
                        }
                        if f.editada() {
                            estado.push('✏');
                        }
                        if f.propuesta.marcas.iter().any(|m| m.nivel == 1) {
                            estado.push('‼');
                        } else if !f.propuesta.marcas.is_empty() || !f.propuesta.avisos.is_empty() {
                            estado.push('!');
                        }
                        let celdas = [
                            f.numero.to_string(),
                            f.registro.texto("rit"),
                            excel::valor(f, "nombre"),
                            f.registro.texto("programa"),
                            estado,
                            f.observacion.clone(),
                        ];
                        for texto in celdas {
                            row.col(|ui| {
                                ui.add(egui::Label::new(texto).truncate().selectable(false));
                            });
                        }
                        if row.response().clicked() {
                            seleccion = Some(i);
                        }
                    });
                });
        });
        self.seleccion = seleccion;
        if cambio {
            self.sin_guardar = true;
        }
        if guardar_excel {
            self.guardar_sesion();
            self.guardar_excel();
        }
        if let Some(i) = cambiar {
            self.cambiar_etapa(i);
        }
    }

    fn vista_correos(&mut self, ui: &mut egui::Ui) {
        let Some(etapa) = self.trabajo.as_ref().map(|t| t.etapa) else {
            ui.label("Primero abre un Excel en Trabajo.");
            return;
        };
        ui.label(RichText::new("Modalidades comprendidas en la revisión (para los correos al tribunal)").strong());
        ui.horizontal_wrapped(|ui| {
            for (nombre, marcada) in &mut self.modalidades {
                ui.checkbox(marcada, nombre.as_str());
            }
            ui.add(egui::TextEdit::singleline(&mut self.otra_modalidad).hint_text("Otra…").desired_width(160.0));
        });
        let mut preparar = false;
        let mut guardar = None;
        ui.horizontal(|ui| {
            if ui.button("Preparar borradores").clicked() {
                preparar = true;
            }
            if !self.borradores.is_empty() {
                if self.outlook && ui.button("Guardar en Borradores de Outlook").clicked() {
                    guardar = Some(true);
                }
                if ui.button("Guardar como archivos .eml").on_hover_text("Se abren con Outlook como correo sin enviar").clicked() {
                    guardar = Some(false);
                }
            }
            if !self.outlook {
                ui.small("Outlook de escritorio no detectado: se usan archivos .eml.");
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, b) in self.borradores.iter_mut().enumerate() {
                let quien = if b.programa.is_empty() { b.tribunal.clone() } else { format!("{} · {}", b.programa, b.tribunal) };
                let faltan = b.faltantes();
                let titulo = format!(
                    "{}{} — {} ({}){}",
                    if b.omitido { "(omitido) " } else { "" },
                    b.tipo.titulo(etapa),
                    quien,
                    if b.filas.len() == 1 { "1 registro".to_string() } else { format!("{} registros", b.filas.len()) },
                    if faltan.is_empty() { String::new() } else { format!(" — falta {}", faltan.join(", ")) }
                );
                egui::CollapsingHeader::new(titulo).id_salt(("borrador", i)).show(ui, |ui| {
                    ui.checkbox(&mut b.omitido, "No guardar este borrador");
                    egui::Grid::new(("campos", i)).num_columns(2).show(ui, |ui| {
                        ui.label("Para");
                        ui.add(egui::TextEdit::singleline(&mut b.para).desired_width(520.0).hint_text("correo del destinatario"));
                        ui.end_row();
                        ui.label("CC");
                        ui.label(&b.cc).on_hover_text("El CC fijo va en todos los correos y no se puede quitar.");
                        ui.end_row();
                        ui.label("Asunto");
                        ui.add(egui::TextEdit::singleline(&mut b.asunto).desired_width(520.0));
                        ui.end_row();
                    });
                    ui.add(egui::TextEdit::multiline(&mut b.cuerpo).desired_width(f32::INFINITY).desired_rows(6));
                    for adjunto in &b.adjuntos {
                        ui.horizontal(|ui| {
                            ui.label(format!("Adjunto: {}", adjunto.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()));
                            if ui.small_button("Abrir").clicked() {
                                abrir_en_sistema(adjunto);
                            }
                        });
                    }
                });
            }
        });
        if preparar {
            self.preparar_borradores();
        }
        if let Some(en_outlook) = guardar {
            self.guardar_borradores(en_outlook);
        }
    }

    fn vista_resoluciones(&mut self, ui: &mut egui::Ui) {
        if self.trabajo.is_none() {
            ui.label("Primero abre un Excel en Trabajo.");
            return;
        }
        let mut preparar = false;
        let mut generar = false;
        ui.horizontal(|ui| {
            if ui.button("Preparar proyectos").clicked() {
                preparar = true;
            }
            if self.proyectos.iter().any(|p| p.listo()) && ui.button("Generar Word").clicked() {
                generar = true;
            }
            if ui.button("Carpeta de plantillas propias").on_hover_text("Ej.: TOME_PC_IE.docx con variables {{RIT}}, {{NOMBRE}}…").clicked() {
                let carpeta = self.plantillas_usuario();
                let _ = std::fs::create_dir_all(&carpeta);
                abrir_en_sistema(&carpeta);
            }
        });
        ui.small("Un documento por tribunal y tipo; cada proyecto empieza en página nueva.");
        ui.separator();
        let propias = self.plantillas_usuario();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, p) in self.proyectos.iter_mut().enumerate() {
                let estado = if p.omitido {
                    "omitido".to_string()
                } else if p.sin_plantilla {
                    format!("sin plantilla para {}", p.tribunal.titulo())
                } else if p.faltantes.is_empty() {
                    "listo".to_string()
                } else {
                    format!("falta {}", p.faltantes.join(", "))
                };
                egui::CollapsingHeader::new(format!("{} · {} · {} — {}", p.tipo, p.rit, p.tribunal.titulo(), estado))
                    .id_salt(("proyecto", i))
                    .default_open(!p.faltantes.is_empty())
                    .show(ui, |ui| {
                        ui.checkbox(&mut p.omitido, "No generar este proyecto");
                        let mut cambio = false;
                        egui::Grid::new(("valores", i)).num_columns(2).show(ui, |ui| {
                            for (clave, valor) in p.valores.iter_mut() {
                                ui.label(clave.as_str());
                                if ui.add(egui::TextEdit::singleline(valor).desired_width(480.0)).changed() {
                                    cambio = true;
                                }
                                ui.end_row();
                            }
                        });
                        if cambio {
                            p.actualizar(Some(&propias));
                        }
                    });
            }
        });
        if preparar {
            self.preparar_proyectos();
        }
        if generar {
            self.generar_word();
        }
    }

    fn vista_resultados(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(c) = &self.carpeta {
                if ui.button("Abrir la carpeta de productos").clicked() {
                    abrir_en_sistema(c);
                }
                ui.label(c.display().to_string());
            }
            if ui.button("Cambiar carpeta de salida…").clicked() {
                if let Some(nueva) = rfd::FileDialog::new().pick_folder() {
                    self.ajustes.salida = Some(nueva);
                    self.carpeta = None;
                    if let Ok(texto) = serde_json::to_string_pretty(&self.ajustes) {
                        let _ = std::fs::write(ruta_ajustes(&self.datos), texto);
                    }
                }
            }
        });
        ui.separator();
        if self.resultados.is_empty() {
            ui.label("Aún no se ha creado ningún producto.");
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for linea in &self.resultados {
                ui.label(linea);
            }
        });
    }

    fn vista_contactos(&mut self, ui: &mut egui::Ui) {
        let mut guardar = false;
        ui.label("Estas direcciones se guardan solo en este equipo.");
        ui.add_space(4.0);
        ui.label(RichText::new("Tribunales").strong());
        egui::Grid::new("tribunales").num_columns(2).show(ui, |ui| {
            for clave in ["LAJA", "MULCHEN", "TOME"] {
                ui.label(clave);
                let valor = self.contactos.tribunales.entry(clave.to_string()).or_default();
                guardar |= ui.add(egui::TextEdit::singleline(valor).desired_width(420.0)).lost_focus();
                ui.end_row();
            }
            ui.label("CC adicional");
            guardar |= ui.add(egui::TextEdit::singleline(&mut self.contactos.cc_adicional).desired_width(420.0)).lost_focus();
            ui.end_row();
            ui.label("CC fijo");
            ui.label(CC_FIJO);
            ui.end_row();
        });
        ui.label(RichText::new("Firma al pie de los correos").strong());
        guardar |= ui.add(egui::TextEdit::multiline(&mut self.contactos.firma).desired_rows(3).desired_width(520.0)).lost_focus();
        ui.add_space(6.0);
        ui.label(RichText::new("Programas").strong());
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.nuevo.0).hint_text("Nombre del programa").desired_width(240.0));
            ui.add(egui::TextEdit::singleline(&mut self.nuevo.1).hint_text("correo").desired_width(240.0));
            if ui.button("Agregar").clicked() && !self.nuevo.0.trim().is_empty() && self.nuevo.1.contains('@') {
                self.contactos.programas.insert(self.nuevo.0.trim().to_string(), self.nuevo.1.trim().to_string());
                self.nuevo = Default::default();
                guardar = true;
            }
        });
        let mut quitar = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("programas").num_columns(3).striped(true).show(ui, |ui| {
                for (nombre, correo) in self.contactos.programas.iter_mut() {
                    ui.label(nombre.as_str());
                    guardar |= ui.add(egui::TextEdit::singleline(correo).desired_width(320.0)).lost_focus();
                    if ui.small_button("Quitar").clicked() {
                        quitar = Some(nombre.clone());
                    }
                    ui.end_row();
                }
            });
        });
        if let Some(nombre) = quitar {
            self.contactos.programas.remove(&nombre);
            guardar = true;
        }
        if guardar {
            if let Err(e) = self.contactos.guardar(&ruta_contactos(&self.datos)) {
                self.mal(e);
            }
        }
    }
}

const TEXTO_AYUDA: &str = "1 · Trabajo: abre el Excel de RUS (Ctrl+O). Revisa cada observación; puedes editarla y marcarla como \
revisada. Ctrl+S guarda el Excel de observaciones.\n\n2 · Correos: marca las modalidades, prepara los borradores, \
revisa destinatario y texto, y guárdalos. Quedan en Borradores de Outlook o como archivos .eml. Flow nunca envía.\n\n\
3 · Resoluciones: prepara los proyectos, completa lo que falte y genera el Word.\n\n4 · Resultados: lista de lo \
creado y acceso a la carpeta.\n\nFlow no escribe en RUS ni en SATURNO. La observación propuesta es una ayuda: \
quien la registra y responde por ella es el usuario.";

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (abrir, guardar, ayuda) = ctx.input(|i| {
            (i.modifiers.command && i.key_pressed(Key::O), i.modifiers.command && i.key_pressed(Key::S), i.key_pressed(Key::F1))
        });
        if abrir {
            self.elegir_archivo();
        }
        if guardar && self.trabajo.is_some() {
            self.guardar_sesion();
            self.guardar_excel();
        }
        if ayuda {
            self.ayuda = true;
        }
        egui::TopBottomPanel::top("barra").show(ctx, |ui| self.barra(ui));
        egui::TopBottomPanel::bottom("pie").show(ctx, |ui| self.pie(ui));
        egui::CentralPanel::default().show(ctx, |ui| match self.pestana {
            Pestana::Captura => {
                if let Some((carpeta, resumen)) = self.captura.mostrar(ui, ctx) {
                    self.importar_captura(carpeta, resumen)
                }
            }
            Pestana::Trabajo => self.vista_trabajo(ui),
            Pestana::Correos => self.vista_correos(ui),
            Pestana::Resoluciones => self.vista_resoluciones(ui),
            Pestana::Resultados => self.vista_resultados(ui),
            Pestana::Contactos => self.vista_contactos(ui),
            Pestana::Anonimo => self.anonimo.mostrar(ui, ctx),
        });
        if self.ayuda {
            let mut abierta = true;
            egui::Window::new("Cómo se usa Flow").open(&mut abierta).collapsible(false).default_width(520.0).show(ctx, |ui| {
                ui.label(TEXTO_AYUDA);
            });
            self.ayuda = abierta;
        }
        // Las ediciones se guardan solas en la sesión local al dejar de escribir.
        if self.sin_guardar && !ctx.input(|i| i.pointer.any_down() || !i.events.is_empty()) {
            self.guardar_sesion();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.captura.cerrar();
        self.guardar_sesion();
        let _ = self.contactos.guardar(&ruta_contactos(&self.datos));
    }
}

/// Abre la ventana. `archivo` permite arrastrar un Excel sobre el ejecutable.
pub fn abrir(archivo: Option<PathBuf>) -> i32 {
    let opciones = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 720.0]).with_min_inner_size([900.0, 560.0]).with_title("Flow"),
        ..Default::default()
    };
    match eframe::run_native("Flow", opciones, Box::new(|cc| {
            cc.egui_ctx.set_theme(egui::ThemePreference::System);
            cc.egui_ctx.set_zoom_factor(1.15);
            Ok(Box::new(App::nueva(archivo)))
        }),
    ) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("No se pudo abrir la ventana: {e}");
            1
        }
    }
}

#[cfg(test)]
mod pruebas_captura {
    use super::*;

    static SECUENCIA:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);

    fn preparar() -> (PathBuf, flow_captura::lotes::Resumen) {
        let base = std::env::temp_dir().join(format!(
            "flow-ui-ficticia-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
            SECUENCIA.fetch_add(1,std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&base).unwrap();
        let datos = std::fs::read(format!("{}/../../../docs/evidencia_p0/golden/Origen_ESPERA.xlsx", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let sha256 = flow_captura::lotes::huella(&datos);
        // El nombre del archivo no anuncia la etapa: esta viene de la consulta verificada.
        std::fs::write(base.join("consulta-001-p001-de-001.xls"), datos).unwrap();
        let seleccion = flow_captura::sitfa::Seleccion {
            tribunal: "901".into(),
            screen: "seguimiento".into(),
            tab: "Espera".into(),
            modality: "2".into(),
            report: None,
            state: None,
            month: None,
            year: None,
        };
        let lote = flow_captura::lotes::Lote {
            consultas: vec![seleccion],
            fechas: None,
            conservar_filtros: false,
            evidencia: false,
            pausa_ms: 250,
            lecturas: Default::default(),
        };
        let resumen = flow_captura::lotes::Resumen {
            version: 1,
            lote,
            estado: "VALIDADA".into(),
            terminadas: vec![0],
            vacias: vec![],
            archivos: vec![flow_captura::lotes::Archivo {
                nombre: "consulta-001-p001-de-001.xls".into(),
                sha256,
                consulta: 0,
                pagina: 1,
                total: 1,
                registros: 3,
            }],
            avisos: vec![],
            paginas_lectura: vec![],
            lecturas: vec![],
        };
        (base, resumen)
    }

    #[test]
    fn descarga_verificada_pasa_a_trabajo_con_etapa_y_sesion() {
        let (base, resumen) = preparar();
        let mut app = App::con_datos(base.join("sesiones-ficticias"), None);
        app.corte = "08/10/2026".into();
        app.importar_captura(base.clone(), resumen);
        let trabajo = app.trabajo.as_ref().unwrap();
        assert_eq!(trabajo.etapa, Etapa::Espera);
        assert_eq!(trabajo.filas.len(), 3);
        assert_eq!(almacen::listar(&app.datos).len(), 1);
        assert!(app.mensaje.as_ref().is_some_and(|(error, _)| !error));
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn no_importa_archivo_que_cambio_despues_de_la_descarga() {
        let (base, resumen) = preparar();
        std::fs::write(base.join(&resumen.archivos[0].nombre), b"contenido ficticio modificado").unwrap();
        let mut app = App::con_datos(base.join("sesiones-ficticias"), None);
        app.importar_captura(base.clone(), resumen);
        assert!(app.trabajo.is_none());
        assert!(app.mensaje.as_ref().is_some_and(|(error, _)| *error));
        assert!(almacen::listar(&app.datos).is_empty());
        std::fs::remove_dir_all(base).unwrap();
    }
}
