//! E-S1 · Sección «Resultados del lote» del Descargador (`ventana_resultados.py::ResultsWindow`): abre un lote ya
//! descargado, lista sus consultas con su compatibilidad CSMP y ofrece los productos de `descargador::resultados`
//! (consolidado, Para CSMP, cruce de informes, comparación, CSV, ICS y ZIP de entrega). Solo lee el lote; cada
//! producto se escribe en una carpeta o con un nombre nuevo, nunca sobre un original.

use crate::theme::{self, Pal};
use egui::{RichText, Ui};
use flow_rust::descargador::paquete::{self, Lote};
use flow_rust::descargador::resultados;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub const SIN_LOTE: &str = "Selecciona la carpeta de un lote descargado.";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Producto {
    Consolidado,
    Csmp,
    Csv,
    Ics,
    Zip,
}

pub struct Detalle {
    pub consulta: String,
    pub estado: String,
    pub registros: i64,
    pub aptitud: String,
}

pub struct Carga {
    pub nombre: String,
    pub parcial: bool,
    pub filas: Vec<Detalle>,
}

enum Mensaje {
    Cargado(Result<Carga, String>),
    Producto(Result<String, String>),
}

#[derive(Default)]
pub struct Resultados {
    pub carpeta: String,
    pub parcial: bool,
    pub cargado: Option<Carga>,
    pub estado: Option<Result<String, String>>,
    ocupado: bool,
    recepcion: Option<Receiver<Mensaje>>,
    ofrecida: Option<PathBuf>,
}

fn ahora() -> chrono::DateTime<chrono::FixedOffset> {
    chrono::Local::now().fixed_offset()
}

fn lista(rutas: &[PathBuf]) -> String {
    rutas.iter().map(|r| r.display().to_string()).collect::<Vec<_>>().join("; ")
}

/// Lo que `load` hace en el hilo: el lote verificado y, por consulta, su aptitud para CSMP.
fn leer(carpeta: &Path, parcial: bool) -> Result<Carga, String> {
    let lote = paquete::cargar_lote_con(carpeta, parcial)?;
    let mut aptitudes: Vec<(String, Vec<String>)> = Vec::new();
    for t in resultados::tablas(&lote)? {
        let r = resultados::compatibilidad(&t.cabecera, &t.seleccion, &t.filas)?;
        let faltan: Vec<&str> = r["faltan"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let texto = format!(
            "{}{}",
            r["estado"].as_str().unwrap_or(""),
            if faltan.is_empty() { String::new() } else { format!(" · faltan {}", faltan.join(", ")) }
        );
        let consulta = t.consulta["consulta"].as_str().unwrap_or("").to_string();
        match aptitudes.iter_mut().find(|(c, _)| *c == consulta) {
            Some((_, v)) => v.push(texto),
            None => aptitudes.push((consulta, vec![texto])),
        }
    }
    let filas = lote
        .consultas
        .iter()
        .map(|(q, manifiesto)| {
            let consulta = q["consulta"].as_str().unwrap_or("").to_string();
            let mut vistas = BTreeSet::new();
            let aptitud = aptitudes
                .iter()
                .find(|(c, _)| *c == consulta)
                .map(|(_, v)| v.iter().filter(|x| vistas.insert(x.as_str())).cloned().collect::<Vec<_>>().join("; "))
                .unwrap_or_else(|| "Sin registros".into());
            Detalle {
                registros: manifiesto["archivos"].as_array().into_iter().flatten().filter_map(|a| a["registros"].as_i64()).sum(),
                estado: manifiesto["estado"].as_str().unwrap_or("").to_string(),
                consulta,
                aptitud,
            }
        })
        .collect();
    Ok(Carga {
        nombre: lote.carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        parcial: lote.parcial,
        filas,
    })
}

fn producto(lote: &Lote, cual: Producto) -> Result<String, String> {
    let ahora = ahora();
    Ok(match cual {
        Producto::Consolidado => resultados::exportar_libro(std::slice::from_ref(lote), None, false, &ahora)?.display().to_string(),
        Producto::Csmp => resultados::exportar_libro(std::slice::from_ref(lote), None, true, &ahora)?.display().to_string(),
        Producto::Csv => lista(&resultados::exportar_csv(lote, &ahora)?),
        Producto::Ics => resultados::exportar_ics(lote, &ahora)?.display().to_string(),
        Producto::Zip => resultados::exportar_zip(lote, &ahora)?.display().to_string(),
    })
}

/// «Añadir informes al cruce»: un lote de Cumplimiento y otro de Informes, sin otras pantallas.
pub fn cruzar(actual: Lote, otro: &Path) -> Result<String, String> {
    let otro = paquete::cargar_lote(otro)?;
    let mut modos = BTreeSet::new();
    for l in [&actual, &otro] {
        for t in resultados::tablas(l)? {
            modos.insert(resultados::compatibilidad(&t.cabecera, &t.seleccion, &t.filas)?["modo"].as_str().unwrap_or("").to_string());
        }
    }
    if modos != BTreeSet::from(["CUMPLIMIENTO".to_string(), "INFORMES".to_string()]) {
        return Err("El cruce requiere un lote de Cumplimiento y otro de Informes, sin otras pantallas.".into());
    }
    Ok(resultados::exportar_libro(&[actual, otro], None, true, &ahora())?.display().to_string())
}

/// «Comparar con otro lote»: el elegido es el anterior y el abierto es el nuevo.
pub fn comparar(actual: &Lote, anterior: &Path) -> Result<String, String> {
    Ok(resultados::comparar_lotes(&paquete::cargar_lote(anterior)?, actual, &ahora())?.display().to_string())
}

impl Resultados {
    fn ruta(&self) -> PathBuf {
        PathBuf::from(self.carpeta.trim())
    }

    /// El lote que acaba de terminar la descarga se abre una sola vez.
    pub fn ofrecer(&mut self, ctx: &egui::Context, carpeta: Option<PathBuf>) {
        if let Some(c) = carpeta {
            if self.ofrecida.as_ref() != Some(&c) && !self.ocupado {
                self.carpeta = c.display().to_string();
                self.ofrecida = Some(c);
                self.cargar(ctx);
            }
        }
    }

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

    pub fn cargar(&mut self, ctx: &egui::Context) {
        self.cargado = None;
        if self.carpeta.trim().is_empty() {
            self.estado = Some(Err(SIN_LOTE.into()));
            return;
        }
        self.estado = Some(Ok("Comprobando archivos y procesando resultados localmente…".into()));
        let (carpeta, parcial) = (self.ruta(), self.parcial);
        self.en_hilo(ctx, move || Mensaje::Cargado(leer(&carpeta, parcial)));
    }

    fn producir(&mut self, ctx: &egui::Context, cual: Producto) {
        let (carpeta, parcial) = (self.ruta(), self.parcial);
        self.estado = Some(Ok("Comprobando archivos y procesando resultados localmente…".into()));
        self.en_hilo(ctx, move || Mensaje::Producto(paquete::cargar_lote_con(&carpeta, parcial).and_then(|l| producto(&l, cual))));
    }

    fn con_otro(&mut self, ctx: &egui::Context, titulo: &str, accion: fn(Lote, &Path) -> Result<String, String>) {
        let Some(otro) = rfd::FileDialog::new().set_title(titulo).pick_folder() else { return };
        let (carpeta, parcial) = (self.ruta(), self.parcial);
        self.estado = Some(Ok("Comprobando archivos y procesando resultados localmente…".into()));
        self.en_hilo(ctx, move || Mensaje::Producto(paquete::cargar_lote_con(&carpeta, parcial).and_then(|l| accion(l, &otro))));
    }

    pub fn recibir(&mut self) {
        let Some(r) = &self.recepcion else { return };
        let mensaje = match r.try_recv() {
            Ok(m) => m,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.ocupado = false;
                self.recepcion = None;
                self.estado = Some(Err("E_RESULTADOS: no se pudo generar el producto; los originales se conservan.".into()));
                return;
            }
        };
        self.ocupado = false;
        self.recepcion = None;
        match mensaje {
            Mensaje::Cargado(Ok(c)) => {
                self.estado = Some(Ok(format!(
                    "{}{} · {} consultas. Cada producto se escribe en esta carpeta sin tocar los originales.",
                    if c.parcial { "INCOMPLETO · " } else { "" },
                    c.nombre,
                    c.filas.len()
                )));
                self.cargado = Some(c);
            }
            Mensaje::Cargado(Err(e)) | Mensaje::Producto(Err(e)) => self.estado = Some(Err(e)),
            Mensaje::Producto(Ok(rutas)) => self.estado = Some(Ok(format!("Creado: {rutas}"))),
        }
    }

    pub fn mostrar(&mut self, ui: &mut Ui, p: &Pal) {
        self.recibir();
        ui.add_space(8.0);
        theme::seccion(ui, p, "Resultados del lote");
        ui.horizontal_wrapped(|ui| {
            ui.label("Carpeta del lote");
            ui.add_enabled(!self.ocupado, egui::TextEdit::singleline(&mut self.carpeta).desired_width(360.0));
            if ui.add_enabled(!self.ocupado, egui::Button::new("Elegir…")).clicked() {
                if let Some(c) = rfd::FileDialog::new().set_title("Carpeta de un lote descargado").pick_folder() {
                    self.carpeta = c.display().to_string();
                    self.cargar(ui.ctx());
                }
            }
            if ui.add_enabled(!self.ocupado, egui::Button::new("Abrir lote")).clicked() {
                self.cargar(ui.ctx());
            }
            let antes = self.parcial;
            ui.add_enabled(!self.ocupado, egui::Checkbox::new(&mut self.parcial, "Permitir resultados INCOMPLETOS"));
            if self.parcial != antes && !self.carpeta.trim().is_empty() {
                self.cargar(ui.ctx());
            }
        });
        if let Some(c) = &self.cargado {
            egui::ScrollArea::vertical().id_salt("consultas del lote").max_height(200.0).show(ui, |ui| {
                egui::Grid::new("detalle del lote").num_columns(4).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                    for t in ["Consulta", "Resultado", "Registros", "Compatibilidad CSMP"] {
                        ui.label(RichText::new(t).strong());
                    }
                    ui.end_row();
                    for f in &c.filas {
                        ui.label(&f.consulta);
                        ui.label(&f.estado);
                        ui.label(f.registros.to_string());
                        ui.label(&f.aptitud);
                        ui.end_row();
                    }
                });
            });
        }
        let listo = self.cargado.is_some() && !self.ocupado;
        let ctx = ui.ctx().clone();
        ui.add_enabled_ui(listo, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Consolidar XLSX").clicked() {
                    self.producir(&ctx, Producto::Consolidado);
                }
                if ui.button("Libro CSMP (Excel)").clicked() {
                    self.producir(&ctx, Producto::Csmp);
                }
                if ui.button("Añadir informes al cruce…").clicked() {
                    self.con_otro(&ctx, "Lote de informes compatible para el cruce", cruzar);
                }
                if ui.button("Comparar con otro lote…").clicked() {
                    self.con_otro(&ctx, "Lote anterior del mismo alcance", |l, o| comparar(&l, o));
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("CSV").clicked() {
                    self.producir(&ctx, Producto::Csv);
                }
                if ui.button("Calendario ICS").clicked() {
                    self.producir(&ctx, Producto::Ics);
                }
                if ui.button("ZIP de originales del lote").clicked() {
                    self.producir(&ctx, Producto::Zip);
                }
                if ui.button("Abrir carpeta").clicked() {
                    crate::ui::abrir_en_sistema(&self.ruta());
                }
            });
        });
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
                None => {
                    ui.label(RichText::new(SIN_LOTE).color(p.muted));
                }
            }
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use base64::Engine;
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;

    fn reconstruir(destino: &Path, archivos: &Value) {
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    struct Fuente {
        paquete: Value,
        resultados: Value,
        base: PathBuf,
    }

    impl Fuente {
        fn nueva(nombre: &str, perfil: &Path) -> Self {
            let base = perfil.join(nombre);
            let _ = std::fs::remove_dir_all(&base);
            Fuente {
                paquete: serde_json::from_str(include_str!("../tests/datos/paquete.json")).unwrap(),
                resultados: serde_json::from_str(include_str!("../tests/datos/resultados.json")).unwrap(),
                base,
            }
        }

        fn lote(&self, escenario: &str, i: usize) -> PathBuf {
            let e = self.paquete["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == escenario).unwrap();
            let l: Vec<&Value> = std::iter::once(&e["lote"]).chain(e["otros_lotes"].as_array().into_iter().flatten()).collect();
            let c = self.base.join(escenario).join(i.to_string()).join(l[i]["carpeta"].as_str().unwrap());
            if !c.exists() {
                reconstruir(&c, &l[i]["archivos"]);
            }
            c
        }
    }

    fn esperar(h: &mut Harness<'_, Resultados>) {
        for _ in 0..500 {
            h.step();
            if !h.state().ocupado {
                h.run();
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("la operación de Resultados no terminó");
    }

    fn vista() -> Harness<'static, Resultados> {
        let pal = theme::pal(false);
        Harness::builder().with_size(egui::Vec2::new(1200.0, 600.0)).build_ui_state(move |ui, r: &mut Resultados| r.mostrar(ui, &pal), Resultados::default())
    }

    fn creado(h: &Harness<'_, Resultados>) -> Vec<PathBuf> {
        match &h.state().estado {
            Some(Ok(t)) => t.strip_prefix("Creado: ").unwrap_or_else(|| panic!("estado {t}")).split("; ").map(PathBuf::from).collect(),
            otro => panic!("estado {otro:?}"),
        }
    }

    /// E-S1: la sección «Resultados del lote» abre un lote, lista sus consultas con su compatibilidad CSMP y los
    /// botones crean los productos de `descargador::resultados`: el CSV de la vista es el de Flow Python.
    #[test]
    fn ui_resultados_abre_un_lote_y_crea_sus_productos() {
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let f = Fuente::nueva("es1-ui", &perfil);
        let mut h = vista();
        h.run();
        assert!(h.query_by_label(SIN_LOTE).is_some(), "sin lote se pide la carpeta");
        assert!(h.get_by_label("Consolidar XLSX").is_disabled(), "sin lote no hay productos");
        // Cuatro consultas del lote con dos pestañas: nombre, resultado, registros y aptitud CSMP, como `ResultsWindow.load`.
        h.state_mut().carpeta = f.lote("listados_dos_pestanas", 0).display().to_string();
        let ctx = h.ctx.clone();
        h.state_mut().cargar(&ctx);
        esperar(&mut h);
        let c = h.state().cargado.as_ref().expect("lote cargado");
        assert_eq!(c.filas.len(), 4);
        assert!(c.filas.iter().all(|f| f.estado == "VALIDADA" && f.registros == 2));
        let espera = c.filas.iter().filter(|f| f.aptitud == "Sin datos suficientes · faltan espera").count();
        let cumpl = c.filas.iter().filter(|f| f.aptitud == "Sin datos suficientes · faltan dias_cumpl, dias_egresar").count();
        assert_eq!((espera, cumpl), (2, 2));
        assert_eq!(h.query_all_by_label("Sin datos suficientes · faltan espera").count(), 2, "las filas se ven en la vista");
        // Un lote sin las columnas de CSMP no genera el Libro CSMP: el rechazo de Python llega a la vista.
        h.get_by_label("Libro CSMP (Excel)").click();
        esperar(&mut h);
        match &h.state().estado {
            Some(Err(e)) => assert!(e.starts_with("No apto para CSMP: "), "{e}"),
            otro => panic!("estado {otro:?}"),
        }
        // CSV de la carga XLSX: el mismo contenido que `export_csv` de Python.
        h.state_mut().carpeta = f.lote("carga", 1).display().to_string();
        h.state_mut().cargar(&ctx);
        esperar(&mut h);
        assert_eq!(h.state().cargado.as_ref().unwrap().filas.len(), 1);
        h.get_by_label("CSV").click();
        esperar(&mut h);
        let rutas = creado(&h);
        let esperados: Vec<&Value> = f.resultados["csv"].as_array().unwrap().iter().filter(|c| c["escenario"] == "carga" && c["lote"] == 1).collect();
        assert_eq!((rutas.len(), esperados.len()), (1, 1));
        let python = base64::engine::general_purpose::STANDARD.decode(esperados[0]["contenido"].as_str().unwrap()).unwrap();
        assert_eq!(std::fs::read(&rutas[0]).unwrap(), python, "el CSV de la vista difiere del de Python");
        // Sin fechas verificables no hay calendario: el rechazo de Python llega a la vista.
        h.get_by_label("Calendario ICS").click();
        esperar(&mut h);
        assert!(matches!(&h.state().estado, Some(Err(e)) if e == "No hay fechas verificables para exportar al calendario."), "{:?}", h.state().estado);
        // Consolidado y ZIP: archivos nuevos dentro del lote, con los originales intactos.
        for (boton, extension) in [("Consolidar XLSX", "xlsx"), ("ZIP de originales del lote", "zip")] {
            h.get_by_label(boton).click();
            esperar(&mut h);
            let r = creado(&h);
            assert_eq!(r.len(), 1, "{boton}");
            assert_eq!(r[0].extension().and_then(|e| e.to_str()), Some(extension), "{boton}");
            assert!(r[0].is_file() && r[0].starts_with(f.lote("carga", 1).canonicalize().unwrap()), "{boton}: {}", r[0].display());
        }
        // Un lote con vencimientos sí da su calendario.
        h.state_mut().carpeta = f.lote("calendario_informes", 0).display().to_string();
        h.state_mut().cargar(&ctx);
        esperar(&mut h);
        h.get_by_label("Calendario ICS").click();
        esperar(&mut h);
        let r = creado(&h);
        assert!(r.len() == 1 && r[0].extension().is_some_and(|e| e == "ics") && r[0].is_file(), "{r:?}");
        // Un lote alterado después de la descarga no se exporta.
        let lote = f.lote("carga", 0);
        h.state_mut().carpeta = lote.display().to_string();
        h.state_mut().cargar(&ctx);
        esperar(&mut h);
        let archivo = std::fs::read_dir(&lote).unwrap().filter_map(|e| e.ok().map(|e| e.path())).find(|p| p.extension().is_some_and(|e| e == "xls" || e == "xlsx" || e == "html")).unwrap();
        std::fs::write(&archivo, b"alterado").unwrap();
        h.get_by_label("CSV").click();
        esperar(&mut h);
        assert!(matches!(&h.state().estado, Some(Err(e)) if !e.is_empty()), "un archivo alterado debe rechazarse: {:?}", h.state().estado);
        let _ = std::fs::remove_dir_all(&f.base);
    }

    /// E-S1: cruce y comparación (los selectores de carpeta se prueban en su función) y el aviso de una carpeta sin lote.
    #[test]
    fn resultados_cruza_compara_y_rechaza_como_python() {
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let f = Fuente::nueva("es1-cruce", &perfil);
        let actual = |e: &str, i: usize| paquete::cargar_lote(&f.lote(e, i)).unwrap();
        // Cumplimiento + Informes: un «Para CSMP» con los dos; Cumplimiento + Espera o Cumplimiento solo, no.
        let libro = cruzar(actual("calendario_informes", 0), &f.lote("calendario_informes", 2)).unwrap();
        assert!(Path::new(&libro).is_file() && libro.contains("Para CSMP"), "{libro}");
        for otro in [1, 0] {
            let e = cruzar(actual("calendario_informes", 0), &f.lote("calendario_informes", otro)).unwrap_err();
            assert_eq!(e, "El cruce requiere un lote de Cumplimiento y otro de Informes, sin otras pantallas.");
        }
        // Comparación: el lote elegido es el anterior; con otro alcance se rechaza.
        let libro = comparar(&actual("listados", 0), &f.lote("listados", 0)).unwrap();
        assert!(Path::new(&libro).is_file() && libro.contains("Comparacion"), "{libro}");
        assert_eq!(comparar(&actual("listados", 0), &f.lote("carga", 0)).unwrap_err(), "Los lotes tienen alcances diferentes.");
        // El lote que termina de descargarse se abre una sola vez; después manda la carpeta que escribe la persona.
        let mut h = vista();
        let ctx = h.ctx.clone();
        let terminado = f.lote("listados_dos_pestanas", 0);
        h.state_mut().ofrecer(&ctx, Some(terminado.clone()));
        esperar(&mut h);
        assert_eq!(h.state().cargado.as_ref().map(|c| c.filas.len()), Some(4));
        h.state_mut().carpeta = "otra".into();
        h.state_mut().ofrecer(&ctx, Some(terminado));
        assert!(!h.state().ocupado && h.state().carpeta == "otra", "el mismo lote no se vuelve a ofrecer");
        // Una carpeta que no es un lote deja el aviso y nada cargado.
        h.state_mut().carpeta = perfil.join("no-existe").display().to_string();
        h.state_mut().cargar(&ctx);
        esperar(&mut h);
        assert!(h.state().cargado.is_none());
        assert!(matches!(&h.state().estado, Some(Err(e)) if e.starts_with("No se pudo abrir el lote")), "{:?}", h.state().estado);
        let _ = std::fs::remove_dir_all(&f.base);
    }
}
