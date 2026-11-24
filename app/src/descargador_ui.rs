use crate::descargador_resultados_ui::Resultados;
use crate::theme::{self, Pal};
use crate::util::parse_date_text;
use egui::{ComboBox, RichText, Ui};
use flow_rust::descargador::bitacoras::{self, LoteBitacoras};
use flow_rust::descargador::lote::{self, Evento, Fin, Opciones};
use flow_rust::descargador::paquete;
use serde_json::Value;
use flow_rust::descargador::preferencias::{self, Preferencias, PAUSA_MAXIMA_S, PAUSA_MINIMA_S};
use flow_rust::descargador::{
    Catalogo, Edicion, Pantalla, Perfil, PlanLote, Puente, Seleccion, SeleccionLote, MODALIDADES,
    PESTANAS, TODAS_LAS_MEDIDAS,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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
    ejecucion: Ejecucion,
    instalacion: Option<Result<String, String>>,
    favorito_nombre: String,
    favorito_elegido: String,
    favoritos: Option<Vec<String>>,
    favorito_mensaje: Option<Result<String, String>>,
    resultados: Resultados,
}

enum Mensaje {
    Evento(Evento),
    Termino(Result<usize, String>),
}

#[derive(PartialEq)]
enum EstadoFila {
    EnCurso,
    Lista,
    Vacia,
}

struct Fila {
    etiqueta: String,
    detalle: String,
    estado: EstadoFila,
}

/// Ejecución de un lote Lite en segundo plano (`SitfaLite.worker` y `process_event` de Flow Python).
struct Ejecucion {
    destino: String,
    evidencia: bool,
    conservar_filtros: bool,
    pausa: f32,
    recepcion: Option<mpsc::Receiver<Mensaje>>,
    cancelar: Option<Arc<AtomicBool>>,
    filas: Vec<Fila>,
    estado: String,
    avisos: Vec<String>,
    error: Option<String>,
    fin: Option<Fin>,
    carpeta: Option<PathBuf>,
    contexto: Contexto,
}

impl Default for Ejecucion {
    fn default() -> Self {
        let base = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_default();
        // Preferencias persistentes (`preferencias.py`): destino, PDF sin registros y pausa.
        let guardadas = Preferencias::cargar(&Preferencias::ruta());
        let destino = if guardadas.destino.trim().is_empty() {
            base.join("Documents").join("SITFA Lite").display().to_string()
        } else {
            guardadas.destino
        };
        Self {
            destino,
            evidencia: guardadas.pdf_sin_registros,
            conservar_filtros: false,
            pausa: guardadas.pausa_consultas as f32,
            recepcion: None,
            cancelar: None,
            filas: Vec::new(),
            estado: String::new(),
            avisos: Vec::new(),
            error: None,
            fin: None,
            carpeta: None,
            contexto: Contexto::desde(&guardadas_contexto()),
        }
    }
}

fn texto_valor(v: &Value) -> String {
    v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())
}

fn guardadas_contexto() -> Preferencias {
    Preferencias::cargar(&Preferencias::ruta())
}

enum Trabajo {
    Bitacoras(Result<Value, String>),
    Paquete(Result<Value, String>),
    /// E-Q7 · `query`, `diary_progress` o `warning` del lote de bitácoras.
    Avance(String, Value),
}

/// E-Q6 · Lecturas de contexto del lote (Integral, Seguimiento) y paquete del Asistente: `start_diary` y
/// `create_package` de Flow Python. Lee con la sesión del usuario a través de la extensión; nada se escribe en SITFA.
struct Contexto {
    reutilizar: bool,
    calendarios: bool,
    historia: bool,
    historia_en_paquete: bool,
    historico: bool,
    meses: u32,
    consolidado: bool,
    recepcion: Option<mpsc::Receiver<Trabajo>>,
    cancelar: Option<Arc<AtomicBool>>,
    estado: String,
    bitacoras: Option<Value>,
    paquete: Option<Value>,
    /// El paquete recién creado que la persona pidió abrir en Trabajo (lo recoge `App`).
    abrir_en_trabajo: Option<PathBuf>,
    error: Option<String>,
    /// Filas revisadas (`diary_progress`: página, registros, «RIT · estado») y avisos del lote en curso.
    filas: Vec<(String, String)>,
    avisos: Vec<String>,
}

impl Contexto {
    fn desde(p: &Preferencias) -> Self {
        Self {
            reutilizar: p.reutilizar_bitacoras,
            calendarios: p.leer_calendarios,
            historia: p.leer_historia,
            historia_en_paquete: p.historia_en_paquete,
            historico: p.leer_historico,
            meses: p.meses_atras,
            consolidado: p.paquete_consolidado,
            recepcion: None,
            cancelar: None,
            estado: String::new(),
            bitacoras: None,
            paquete: None,
            abrir_en_trabajo: None,
            error: None,
            filas: Vec::new(),
            avisos: Vec::new(),
        }
    }

    fn activa(&self) -> bool {
        self.recepcion.is_some()
    }

    fn guardar_preferencias(&self) -> Option<String> {
        let ruta = Preferencias::ruta();
        let mut p = Preferencias::cargar(&ruta);
        p.reutilizar_bitacoras = self.reutilizar;
        p.leer_calendarios = self.calendarios;
        p.leer_historia = self.historia;
        p.historia_en_paquete = self.historia_en_paquete;
        p.leer_historico = self.historico;
        p.meses_atras = self.meses;
        p.paquete_consolidado = self.consolidado;
        p.guardar(&ruta).err()
    }

    fn recibir(&mut self) {
        let Some(recepcion) = &self.recepcion else { return };
        let mensajes: Vec<Trabajo> = recepcion.try_iter().collect();
        let desconectado = matches!(recepcion.try_recv(), Err(mpsc::TryRecvError::Disconnected));
        for m in mensajes {
            if let Trabajo::Avance(nombre, v) = m {
                self.aplicar_avance(&nombre, &v);
                continue;
            }
            self.recepcion = None;
            self.cancelar = None;
            self.estado.clear();
            match m {
                Trabajo::Bitacoras(Ok(v)) => self.bitacoras = Some(v),
                Trabajo::Paquete(Ok(v)) => self.paquete = Some(v),
                Trabajo::Bitacoras(Err(e)) | Trabajo::Paquete(Err(e)) => self.error = Some(e),
                Trabajo::Avance(..) => {}
            }
        }
        if desconectado && self.recepcion.is_some() {
            self.recepcion = None;
            self.cancelar = None;
            self.error = Some("Se interrumpió la operación.".into());
        }
    }

    /// `process_event` de Python para el lote de bitácoras: una fila por registro revisado y el estado de la consulta.
    fn aplicar_avance(&mut self, nombre: &str, v: &Value) {
        let texto = |k: &str| match &v[k] {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            otro => otro.to_string(),
        };
        match nombre {
            "query" => self.estado = format!("Consulta {} de {} · {}", texto("index"), texto("total"), texto("label")),
            "diary_progress" => {
                self.filas.push((texto("pagina"), format!("{} · {}", texto("rit"), texto("estado"))));
                self.estado = format!("Bitácora {} · {}", texto("rit"), texto("estado"));
            }
            "warning" => self.avisos.push(texto_valor(v)),
            _ => {}
        }
    }

    /// `start_diary`: bitácoras del alcance del lote (más calendario, historia e histórico si se eligieron) en una
    /// subcarpeta nueva del destino.
    /// Con `retomar`, el alcance es el guardado en esa carpeta (`resume_diary`): no se reutilizan lecturas de otros
    /// lotes y la historia ya leída se toma de su `historias.json`.
    #[allow(clippy::too_many_arguments)]
    fn leer(
        &mut self,
        ctx: &egui::Context,
        puente: Arc<Puente>,
        catalogo: Catalogo,
        alcance: LoteBitacoras,
        destino: PathBuf,
        pausa: f64,
        retomar: Option<PathBuf>,
    ) {
        self.error = self.guardar_preferencias().map(|e| format!("No se guardaron las preferencias del Descargador: {e}"));
        self.bitacoras = None;
        self.paquete = None;
        self.filas.clear();
        self.avisos.clear();
        let historia_anterior = retomar.as_ref().map(|r| r.join("historia").join("historias.json")).filter(|h| self.historia && h.is_file());
        let opciones = bitacoras::Opciones {
            pausa: Duration::from_secs_f64(pausa),
            reutilizar_base: (self.reutilizar && retomar.is_none()).then(|| destino.clone()),
            retomar,
            calendarios: self.calendarios,
            historico: self.historico,
            historia: self.historia.then_some(self.meses),
            historia_anterior,
            ..Default::default()
        };
        let cancelar = Arc::new(AtomicBool::new(false));
        let (envio, recepcion) = mpsc::channel();
        self.recepcion = Some(recepcion);
        self.cancelar = Some(cancelar.clone());
        self.estado = "Leyendo y copiando bitácoras. Las fallas quedan en el control del lote.".into();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let avisar = |nombre: &str, v: Value| {
                let _ = envio.send(Trabajo::Avance(nombre.to_string(), v));
                ctx.request_repaint();
            };
            let resultado = puente.verificar(Edicion::Integral).and_then(|()| {
                bitacoras::ejecutar_con_avance(puente.as_ref(), &catalogo, &alcance, &destino, &opciones, &cancelar, &avisar)
            });
            let _ = envio.send(Trabajo::Bitacoras(resultado));
            ctx.request_repaint();
        });
    }

    /// `create_package`: paquete del Asistente con el lote de listados y, si se leyeron, sus bitácoras (y el
    /// histórico e historia del mismo lote de bitácoras). El destino debe ser nuevo.
    fn crear_paquete(&mut self, ctx: &egui::Context, listados: PathBuf, destino: PathBuf) {
        self.error = None;
        self.paquete = None;
        if destino.exists() {
            self.error = Some("Ese archivo ya existe. Elige un nombre nuevo para no reemplazar otro paquete.".into());
            return;
        }
        let manifiesto = self.bitacoras.as_ref().and_then(|b| b["manifest"].as_str()).map(PathBuf::from);
        let historia = manifiesto.as_ref().and_then(|m| m.parent()).map(|c| c.join("historia").join("historias.json"))
            .filter(|h| self.historia_en_paquete && h.is_file());
        let historico = manifiesto.as_ref().and_then(|m| m.parent()).map(Path::to_path_buf)
            .filter(|c| c.join("historico.json").is_file());
        let meses = self.meses;
        let consolidado = self.consolidado;
        let (envio, recepcion) = mpsc::channel();
        self.recepcion = Some(recepcion);
        self.estado = "Creando el paquete…".into();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let resultado = match &manifiesto {
                Some(m) => paquete::empaquetar_lote(&listados, m, &destino, historia.as_deref(), historico.as_deref(), meses, &[], consolidado),
                None => {
                    let opciones = paquete::Opciones { consolidado, meses, ..Default::default() };
                    paquete::exportar_con(&[listados], &destino, &opciones).map(|r| {
                        serde_json::json!({"path": r.ruta.display().to_string(), "reuse": format!("Sin bitácoras añadidas.{}", r.nota)})
                    })
                }
            };
            let _ = envio.send(Trabajo::Paquete(resultado));
            ctx.request_repaint();
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn mostrar(
        &mut self,
        ui: &mut Ui,
        p: &Pal,
        catalogo: &Catalogo,
        lote: &SeleccionLote,
        puente: Option<Arc<Puente>>,
        listados: Option<PathBuf>,
        destino: &str,
        pausa: f64,
        ocupado: bool,
    ) {
        self.recibir();
        ui.add_space(8.0);
        theme::seccion(ui, p, "Bitácoras y paquete");
        let activa = self.activa();
        ui.add_enabled_ui(!activa && !ocupado, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.reutilizar, "Reutilizar bitácoras de hoy")
                    .on_hover_text("Solo lecturas del mismo día y de menos de 12 h, con su huella comprobada");
                ui.checkbox(&mut self.calendarios, "Calendario de informes")
                    .on_hover_text("Calendario de cada ingreso de Cumplimiento e Informes; cuenta en el máximo de lecturas del lote");
                ui.checkbox(&mut self.historia, "Historia de la causa (prueba)");
                if self.historia {
                    ui.checkbox(&mut self.historia_en_paquete, "Historia en el paquete");
                }
                ui.checkbox(&mut self.historico, "Ingreso Histórico (Mejor Niñez)");
                ui.checkbox(&mut self.consolidado, "XLSX consolidado")
                    .on_hover_text("El paquete lleva solo el XLSX «Para CSMP» (nunca HTML ni PDF); si el lote no es apto, se informa");
                ui.label(RichText::new("Meses de bitácora").color(p.muted));
                ComboBox::from_id_salt("meses de bitácora").selected_text(self.meses.to_string()).width(40.0).show_ui(ui, |ui| {
                    for m in 1..=4 {
                        ui.selectable_value(&mut self.meses, m, m.to_string());
                    }
                });
            });
        });
        let conectado = puente.as_ref().is_some_and(|b| b.conectado());
        let alcance = LoteBitacoras {
            tribunales: lote.tribunales.clone(),
            modalidades: lote.modalidades.clone(),
            pestanas: lote.pestanas.clone(),
        };
        let impedimento_lectura = if !conectado {
            Some("Conecta la extensión Integral desde la pestaña SITFA.")
        } else if alcance.tribunales.is_empty() || alcance.modalidades.is_empty() || alcance.pestanas.is_empty() {
            Some("Selecciona tribunales, modalidades y pestañas.")
        } else if destino.trim().is_empty() {
            Some("Elige la carpeta de destino.")
        } else {
            None
        };
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if activa {
                if let Some(c) = &self.cancelar {
                    let pedido = c.load(Ordering::SeqCst);
                    if ui.add_enabled(!pedido, egui::Button::new("Cancelar lectura")).clicked() {
                        c.store(true, Ordering::SeqCst);
                    }
                }
                ui.spinner();
                ui.label(&self.estado);
                return;
            }
            let libre = !ocupado;
            if ui.add_enabled(libre && impedimento_lectura.is_none(), egui::Button::new("Leer bitácoras del lote")).clicked() {
                if let Some(puente) = puente.clone() {
                    self.leer(ui.ctx(), puente, catalogo.clone(), alcance.clone(), PathBuf::from(destino.trim()), pausa, None);
                }
            }
            if ui.add_enabled(libre && conectado, egui::Button::new("Reintentar lecturas fallidas…")).clicked() {
                if let (Some(puente), Some(carpeta)) = (
                    puente.clone(),
                    rfd::FileDialog::new().set_title("Carpeta del lote de bitácoras a reintentar").pick_folder(),
                ) {
                    match LoteBitacoras::guardado(&carpeta) {
                        Ok(guardado) => self.leer(ui.ctx(), puente, catalogo.clone(), guardado, PathBuf::from(destino.trim()), pausa, Some(carpeta)),
                        Err(e) => self.error = Some(e),
                    }
                }
            }
            let boton = ui.add_enabled(libre && listados.is_some(), egui::Button::new("Crear paquete de este lote…"));
            if boton.clicked() {
                if let Some(carpeta) = listados.clone() {
                    let nombre = format!("Captura_CSMP_{}.zip", chrono::Local::now().format("%Y-%m-%d_%H%M"));
                    if let Some(destino) = rfd::FileDialog::new()
                        .set_title("Guardar paquete para el Asistente")
                        .add_filter("Paquete CSMP", &["zip"])
                        .set_file_name(&nombre)
                        .save_file()
                    {
                        self.crear_paquete(ui.ctx(), carpeta, destino);
                    }
                }
            }
            if let Some(texto) = impedimento_lectura {
                ui.label(RichText::new(texto).color(p.muted));
            }
            if listados.is_none() {
                ui.label(RichText::new("Descarga el lote para crear su paquete.").color(p.muted));
            }
        });
        if !self.filas.is_empty() {
            ui.label(RichText::new(format!("{} filas revisadas en el lote", self.filas.len())).color(p.muted));
            egui::ScrollArea::vertical().id_salt("avance de bitácoras").max_height(160.0).stick_to_bottom(true).show(ui, |ui| {
                for (pagina, detalle) in &self.filas {
                    ui.label(RichText::new(format!("Página {pagina} · {detalle}")).color(p.text));
                }
            });
        }
        for aviso in &self.avisos {
            ui.label(RichText::new(aviso).color(p.amber));
        }
        if let Some(b) = &self.bitacoras {
            let n = |k: &str| b[k].as_u64().unwrap_or(0);
            let mut texto = format!("Bitácoras: {} leídas · {} fallidas", n("leidas"), n("fallidas"));
            if self.calendarios {
                texto.push_str(&format!(" · calendarios {} leídos, {} fallidos", n("calendarios_leidos"), n("calendarios_fallidos")));
            }
            let color = if n("fallidas") == 0 && n("consultas_fallidas") == 0 { p.green } else { p.amber };
            ui.label(RichText::new(texto).color(color));
            if let Some(d) = b["detalle"].as_str().filter(|d| !d.is_empty()) {
                ui.label(RichText::new(d).color(p.amber));
            }
        }
        if let Some(r) = &self.paquete {
            ui.label(RichText::new(format!("Paquete creado: {}", r["path"].as_str().unwrap_or(""))).color(p.green));
            if let Some(reuse) = r["reuse"].as_str() {
                ui.label(RichText::new(reuse).color(p.muted));
            }
            if r["historico_incompleto"].as_bool() == Some(true) {
                ui.label(RichText::new("El Ingreso Histórico quedó incompleto: revisa sus incidencias en el paquete.").color(p.amber));
            }
            ui.horizontal_wrapped(|ui| {
                if let Some(ruta) = r["path"].as_str().map(PathBuf::from) {
                    if theme::primary(ui, p, "Abrir en Trabajo").on_hover_text("Abre este paquete en Trabajo para analizarlo y exportar su Excel").clicked() {
                        self.abrir_en_trabajo = Some(ruta.clone());
                    }
                    if let Some(carpeta) = ruta.parent() {
                        if ui.button("Abrir carpeta del paquete").clicked() {
                            crate::ui::abrir_en_sistema(carpeta);
                        }
                    }
                }
            });
        }
        if let Some(e) = &self.error {
            ui.label(RichText::new(e).color(p.amber));
        }
    }
}

impl Ejecucion {
    fn activa(&self) -> bool {
        self.recepcion.is_some()
    }

    /// La carpeta del lote cuya descarga terminó, lista para abrirla en Resultados.
    fn terminada(&self) -> Option<PathBuf> {
        self.carpeta.clone().filter(|_| !self.activa() && self.fin.is_some())
    }

    fn recibir(&mut self) {
        let Some(recepcion) = &self.recepcion else {
            return;
        };
        let mensajes: Vec<Mensaje> = recepcion.try_iter().collect();
        let desconectado = matches!(recepcion.try_recv(), Err(mpsc::TryRecvError::Disconnected));
        for mensaje in mensajes {
            match mensaje {
                Mensaje::Evento(evento) => self.aplicar(evento),
                Mensaje::Termino(resultado) => {
                    self.recepcion = None;
                    self.cancelar = None;
                    self.estado.clear();
                    if let Err(e) = resultado {
                        self.error = Some(e);
                        if let Some(fila) = self.filas.last_mut().filter(|f| f.estado == EstadoFila::EnCurso) {
                            fila.detalle = "detenida".into();
                        }
                    }
                }
            }
        }
        if desconectado && self.recepcion.is_some() {
            self.recepcion = None;
            self.cancelar = None;
            self.error = Some("Se interrumpió la ejecución del lote.".into());
        }
    }

    fn aplicar(&mut self, evento: Evento) {
        match evento {
            Evento::Carpeta(carpeta) => self.carpeta = Some(carpeta),
            Evento::Plan(total) => self.estado = format!("{total} consultas en el lote"),
            Evento::Consulta { indice, total, etiqueta } => {
                self.estado = format!("Consultando {indice} de {total}");
                self.filas.push(Fila { etiqueta, detalle: "consultando".into(), estado: EstadoFila::EnCurso });
            }
            Evento::Estado(texto) => self.estado = texto,
            Evento::Total(paginas) => {
                if let Some(f) = self.filas.last_mut() {
                    f.detalle = format!("{paginas} página{}", if paginas == 1 { "" } else { "s" });
                }
            }
            Evento::Pagina { pagina, total, .. } => {
                if let Some(f) = self.filas.last_mut() {
                    f.detalle = format!("página {pagina} de {total} verificada");
                }
            }
            Evento::Vacia { .. } => {
                if let Some(f) = self.filas.last_mut() {
                    f.detalle = "sin registros · sin PDF".into();
                }
            }
            Evento::Evidencia { .. } => {
                if let Some(f) = self.filas.last_mut() {
                    f.detalle = "sin registros · PDF".into();
                }
            }
            Evento::ConsultaLista { estado, .. } => {
                if let Some(f) = self.filas.last_mut() {
                    if estado == "SIN_RESULTADOS" {
                        f.estado = EstadoFila::Vacia;
                    } else {
                        f.estado = EstadoFila::Lista;
                        f.detalle = f.detalle.replace(" verificada", "");
                    }
                }
            }
            Evento::Aviso(texto) => self.avisos.push(texto),
            Evento::Fin(fin) => self.fin = Some(fin),
        }
    }

    fn iniciar(
        &mut self,
        ctx: &egui::Context,
        puente: Arc<Puente>,
        edicion: Edicion,
        catalogo: Catalogo,
        plan: PlanLote,
    ) {
        let destino = PathBuf::from(self.destino.trim());
        if self.destino.trim().is_empty() {
            self.error = Some("Elige la carpeta de destino.".into());
            return;
        }
        let conservar = lote::conservar_filtros(self.conservar_filtros, catalogo.pantalla);
        let pausa = preferencias::limitar_pausa(f64::from(self.pausa));
        self.pausa = pausa as f32;
        let opciones = Opciones {
            conservar_filtros: conservar,
            evidencia: self.evidencia,
            pausa: Duration::from_secs_f64(pausa),
        };
        let mut guardar = Preferencias::cargar(&Preferencias::ruta());
        guardar.destino = self.destino.trim().to_owned();
        guardar.pdf_sin_registros = self.evidencia;
        guardar.pausa_consultas = pausa;
        let error_preferencias = guardar.guardar(&Preferencias::ruta()).err();
        let cancelar = Arc::new(AtomicBool::new(false));
        let (envio, recepcion) = mpsc::channel();
        self.filas.clear();
        self.avisos.clear();
        if let Some(e) = error_preferencias {
            self.avisos.push(format!("No se guardaron las preferencias del Descargador: {e}"));
        }
        self.error = None;
        self.fin = None;
        self.carpeta = None;
        self.estado = "Comprobando la extensión…".into();
        self.recepcion = Some(recepcion);
        self.cancelar = Some(cancelar.clone());
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut emitir = |e: Evento| {
                let _ = envio.send(Mensaje::Evento(e));
                ctx.request_repaint();
            };
            let resultado = puente.verificar(edicion).and_then(|()| {
                lote::ejecutar(
                    puente.as_ref(),
                    &catalogo,
                    edicion,
                    &plan,
                    &destino,
                    &opciones,
                    &cancelar,
                    &mut emitir,
                )
            });
            let _ = envio.send(Mensaje::Termino(resultado.map(|c| c.len())));
            ctx.request_repaint();
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn mostrar(
        &mut self,
        ui: &mut Ui,
        p: &Pal,
        edicion: Edicion,
        catalogo: &Catalogo,
        plan: Option<&PlanLote>,
        puente: Option<Arc<Puente>>,
        remoto: bool,
        lote: &SeleccionLote,
    ) {
        ui.add_space(8.0);
        theme::seccion(ui, p, "Ejecutar lote");
        let activa = self.activa();
        ui.add_enabled_ui(!activa, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Carpeta de destino");
                ui.add(egui::TextEdit::singleline(&mut self.destino).desired_width(320.0));
                if ui.button("Elegir…").clicked() {
                    if let Some(carpeta) = rfd::FileDialog::new()
                        .set_title("Carpeta de destino del lote")
                        .pick_folder()
                    {
                        self.destino = carpeta.display().to_string();
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.evidencia, "PDF de las consultas sin registros");
                if matches!(catalogo.pantalla, Pantalla::Seguimiento | Pantalla::Litigantes) {
                    ui.checkbox(
                        &mut self.conservar_filtros,
                        "Conservar los otros filtros preparados en SITFA",
                    );
                }
                ui.label("Pausa entre consultas (s)");
                ui.add(
                    egui::DragValue::new(&mut self.pausa)
                        .range(PAUSA_MINIMA_S as f32..=PAUSA_MAXIMA_S as f32)
                        .speed(0.1),
                )
                .on_hover_text("Nunca menos de 0,5 s entre consultas a SITFA");
            });
        });
        let conectado = puente.as_ref().is_some_and(|b| b.conectado());
        let impedimento = if catalogo.pantalla == Pantalla::Carga && edicion != Edicion::Integral {
            Some("Carga está disponible en Integral.")
        } else if !remoto {
            Some("Carga las opciones desde Chrome para ejecutar el lote.")
        } else if !conectado {
            Some(if edicion == Edicion::Lite {
                "Conecta la extensión Lite desde la pestaña SITFA."
            } else {
                "Conecta la extensión Integral desde la pestaña SITFA."
            })
        } else if plan.is_none() {
            Some("Prepara el lote para revisar sus consultas antes de descargar.")
        } else {
            None
        };
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if activa {
                let pedido = self.cancelar.as_ref().is_some_and(|c| c.load(Ordering::SeqCst));
                if ui.add_enabled(!pedido, egui::Button::new("Cancelar")).clicked() {
                    if let Some(c) = &self.cancelar {
                        c.store(true, Ordering::SeqCst);
                    }
                }
                ui.spinner();
                ui.label(&self.estado);
            } else {
                let listo = impedimento.is_none();
                let boton = ui.add_enabled_ui(listo, |ui| theme::primary(ui, p, "Descargar lote")).inner;
                if boton.clicked() {
                    if let (Some(puente), Some(plan)) = (puente.clone(), plan) {
                        self.iniciar(ui.ctx(), puente, edicion, catalogo.clone(), plan.clone());
                    }
                }
                if let Some(texto) = impedimento {
                    ui.label(RichText::new(texto).color(p.muted));
                }
            }
        });
        if !self.filas.is_empty() {
            egui::ScrollArea::vertical()
                .id_salt("avance del lote")
                .max_height(180.0)
                .show(ui, |ui| {
                    for fila in &self.filas {
                        let color = match fila.estado {
                            EstadoFila::EnCurso => p.text,
                            EstadoFila::Lista => p.green,
                            EstadoFila::Vacia => p.muted,
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(&fila.etiqueta).color(color));
                            ui.label(RichText::new(format!("· {}", fila.detalle)).color(p.muted));
                        });
                    }
                });
        }
        for aviso in &self.avisos {
            ui.label(RichText::new(aviso).color(p.amber));
        }
        if let Some(e) = &self.error {
            ui.label(RichText::new(e).color(p.amber));
        }
        if let Some(fin) = &self.fin {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Descarga terminada · {} consultas · {} registros · {} XLS · {} PDF · {} sin registros",
                        fin.consultas, fin.registros, fin.archivos, fin.pdfs, fin.vacias
                    ))
                    .color(p.green),
                );
            });
        }
        if let Some(carpeta) = self.carpeta.clone().filter(|_| !activa) {
            if ui.button("Abrir carpeta del lote").clicked() {
                crate::ui::abrir_en_sistema(&carpeta);
            }
        }
        if edicion == Edicion::Integral && catalogo.pantalla == Pantalla::Seguimiento && remoto {
            // El paquete se arma desde un lote terminado: sin `fin`, una carpeta a medias no se ofrece.
            let listados = self.carpeta.clone().filter(|_| !activa && self.fin.is_some());
            let pausa = preferencias::limitar_pausa(f64::from(self.pausa));
            self.contexto.mostrar(ui, p, catalogo, lote, puente, listados, &self.destino, pausa, activa);
        }
    }
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

    /// `Descargador.setup` de Flow Python: carpeta propia, ruta al portapapeles, Chrome en chrome://extensions.
    fn instalar_extension(&self, ctx: &egui::Context) -> Result<String, String> {
        use flow_rust::descargador::extension;
        let carpeta = extension::instalar(self.edicion, &extension::carpeta_base())?;
        ctx.copy_text(carpeta.display().to_string());
        let chrome = if extension::abrir_extensiones_chrome() {
            "Se abrió chrome://extensions."
        } else {
            "Abre Chrome y escribe chrome://extensions en la barra de dirección."
        };
        Ok(format!(
            "Extensión {} {} lista en {} (ruta copiada al portapapeles). {chrome} Activa «Modo de desarrollador», pulsa «Cargar extensión sin empaquetar» y elige esa carpeta. Después entra a Seguimiento en SITFA, pulsa el icono de la extensión y pega el código de «Conexión con Chrome».",
            if self.edicion == Edicion::Lite { "Lite" } else { "Integral" },
            extension::version(self.edicion),
            carpeta.display()
        ))
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

    /// El paquete que la persona pidió abrir en Trabajo desde «Crear paquete de este lote…».
    pub fn paquete_para_trabajo(&mut self) -> Option<PathBuf> {
        self.ejecucion.contexto.abrir_en_trabajo.take()
    }

    pub fn mostrar(&mut self, ui: &mut Ui, p: &Pal) {
        self.ejecucion.recibir();
        if let Some(catalogo) = self.conexion.catalogo() {
            self.usar_catalogo(Some(catalogo), true);
        }
        let ejecutando = self.ejecucion.activa();
        theme::titulo(
            ui,
            p,
            "Descargador",
            "Perfiles de consulta desde Chrome o un catálogo JSON",
        );
        theme::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_enabled_ui(!ejecutando, |ui| self.mostrar_perfiles(ui, p));
        });
        if let Some(catalogo) = &self.catalogo {
            if catalogo.pantalla != Pantalla::Carga || self.edicion == Edicion::Integral {
                ui.add_space(8.0);
                theme::card(p).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::CollapsingHeader::new("Planificar lote")
                        .default_open(self.plan.is_some())
                        .show(ui, |ui| {
                            ui.add_enabled_ui(!ejecutando, |ui| {
                                mostrar_lote(
                                    ui,
                                    p,
                                    catalogo,
                                    self.edicion,
                                    &mut self.lote,
                                    (&mut self.lote_desde, &mut self.lote_hasta),
                                    &mut self.plan,
                                );
                                let hoy = chrono::Local::now().date_naive();
                                let favoritos = self.favoritos.get_or_insert_with(|| {
                                    Preferencias::cargar(&Preferencias::ruta()).favoritos.into_keys().collect()
                                });
                                ui.horizontal_wrapped(|ui| {
                                    ui.add(egui::TextEdit::singleline(&mut self.favorito_nombre).hint_text("Nombre del favorito").desired_width(180.0));
                                    if ui.button("Guardar favorito").on_hover_text("Guarda el alcance; las fechas se recalculan al aplicarlo").clicked() {
                                        let ruta = Preferencias::ruta();
                                        let mut prefs = Preferencias::cargar(&ruta);
                                        let resultado = catalogo
                                            .planificar(self.edicion, &self.lote)
                                            .and_then(|_| prefs.guardar_favorito(&self.favorito_nombre, catalogo.pantalla, &self.lote, self.ejecucion.conservar_filtros, hoy))
                                            .and_then(|()| prefs.guardar(&ruta));
                                        self.favorito_mensaje = Some(resultado.map(|()| "Perfil guardado; recalcula el período al aplicarlo.".into()));
                                        *favoritos = prefs.favoritos.into_keys().collect();
                                    }
                                    if !favoritos.is_empty() {
                                        ComboBox::from_id_salt("favoritos")
                                            .selected_text(if self.favorito_elegido.is_empty() { "Elegir favorito" } else { self.favorito_elegido.as_str() })
                                            .show_ui(ui, |ui| {
                                                for f in favoritos.iter() {
                                                    ui.selectable_value(&mut self.favorito_elegido, f.clone(), f);
                                                }
                                            });
                                        if ui.button("Aplicar").clicked() {
                                            match Preferencias::cargar(&Preferencias::ruta()).aplicar_favorito(&self.favorito_elegido, catalogo, hoy) {
                                                Ok((lote, retirados)) => {
                                                    if let Some((a, b)) = lote.fechas {
                                                        self.lote_desde = a.format("%d/%m/%Y").to_string();
                                                        self.lote_hasta = b.format("%d/%m/%Y").to_string();
                                                    } else {
                                                        self.lote_desde.clear();
                                                        self.lote_hasta.clear();
                                                    }
                                                    self.lote = lote;
                                                    self.plan = None;
                                                    let mut texto = String::from("Favorito aplicado; revisa el alcance y las fechas.");
                                                    if !retirados.is_empty() {
                                                        texto.push_str(&format!(" Opciones retiradas: {}", retirados.join(", ")));
                                                    }
                                                    self.favorito_mensaje = Some(Ok(texto));
                                                }
                                                Err(e) => self.favorito_mensaje = Some(Err(e)),
                                            }
                                        }
                                    }
                                });
                                match &self.favorito_mensaje {
                                    Some(Ok(t)) => { ui.label(RichText::new(t).color(p.green)); }
                                    Some(Err(e)) => { ui.label(RichText::new(e).color(p.amber)); }
                                    None => {}
                                }
                                if ui
                                    .button("Recuperar pendientes de un lote…")
                                    .on_hover_text("Elige la carpeta de un lote incompleto: sus consultas faltantes se descargan desde la página 1 en un lote nuevo")
                                    .clicked()
                                {
                                    if let Some(carpeta) = rfd::FileDialog::new().set_title("Carpeta del lote incompleto").pick_folder() {
                                        let edicion = self.edicion;
                                        self.plan = Some(
                                            lote::pendientes(&carpeta)
                                                .and_then(|p| catalogo.planificar_pendientes(edicion, &p)),
                                        );
                                    }
                                }
                            });
                            let plan = self.plan.as_ref().and_then(|r| r.as_ref().ok());
                            self.ejecucion.mostrar(
                                ui,
                                p,
                                self.edicion,
                                catalogo,
                                plan,
                                self.conexion.puente.clone(),
                                self.catalogo_remoto,
                                &self.lote,
                            );
                        });
                });
            }
        }
        let terminada = self.ejecucion.terminada();
        self.resultados.ofrecer(ui.ctx(), terminada);
        ui.add_space(8.0);
        theme::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.resultados.mostrar(ui, p);
        });
    }

    fn mostrar_perfiles(&mut self, ui: &mut Ui, p: &Pal) {
        {
            let edicion_anterior = self.edicion;
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.edicion, Edicion::Integral, "Integral");
                ui.selectable_value(&mut self.edicion, Edicion::Lite, "Lite");
                ui.separator();
                if ui.button("Abrir catálogo JSON…").clicked() {
                    self.abrir_dialogo();
                }
                if ui
                    .button("Instalar extensión…")
                    .on_hover_text("Copia la extensión de Chrome de esta edición en tu equipo y abre chrome://extensions")
                    .clicked()
                {
                    self.instalacion = Some(self.instalar_extension(ui.ctx()));
                }
            });
            match &self.instalacion {
                Some(Ok(texto)) => {
                    ui.label(RichText::new(texto).color(p.green));
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(e).color(p.amber));
                }
                None => {}
            }
            if edicion_anterior != self.edicion {
                self.conexion.cerrar();
                self.instalacion = None;
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

/// E-D · D2 del Descargador: la pausa se limita a 0,5–10 s y destino, PDF sin registros y pausa se recuperan del perfil
/// al reconstruir la vista (simulación de reinicio), en un perfil aislado.
#[cfg(test)]
mod pruebas_interfaz {
    use super::*;

    #[test]
    fn ui_descargador_limita_pausa_y_conserva_preferencias() {
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let ruta = Preferencias::ruta();
        assert!(ruta.starts_with(&perfil), "las preferencias de la prueba no están aisladas: {}", ruta.display());
        assert_eq!((PAUSA_MINIMA_S, PAUSA_MAXIMA_S), (0.5, 10.0));
        let destino = perfil.join("Destino ficticio").display().to_string();
        let mut resultados = Vec::new();
        for (pedida, esperada) in [(0.0, 0.5), (0.2, 0.5), (3.0, 3.0), (25.0, 10.0)] {
            Preferencias {
                destino: destino.clone(),
                pdf_sin_registros: true,
                pausa_consultas: pedida,
                ..Default::default()
            }
            .guardar(&ruta)
            .unwrap();
            let vista = Descargador::default();
            assert_eq!(vista.ejecucion.destino, destino);
            assert!(vista.ejecucion.evidencia, "PDF sin registros no se conservó");
            assert_eq!(f64::from(vista.ejecucion.pausa), esperada, "pausa pedida {pedida}");
            resultados.push(format!("{pedida}->{}", vista.ejecucion.pausa));
        }
        println!("D2_DESCARGADOR: pausas {}, destino y PDF conservados", resultados.join(" "));
    }

    struct Vista {
        contexto: Contexto,
        catalogo: Catalogo,
        lote: SeleccionLote,
        listados: Option<PathBuf>,
        destino: String,
    }

    fn reconstruir(destino: &Path, archivos: &Value) {
        use base64::Engine;
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    /// E-Q6: la sección «Bitácoras y paquete» del Descargador (Integral, Seguimiento). Opciones con los valores por
    /// omisión de Python y persistentes; sin extensión no se puede leer; «Crear paquete» arma el paquete del lote con
    /// sus bitácoras, historia e histórico (carpetas del oráculo `paquete.json`, escenario «contexto») y lo informa.
    #[test]
    fn ui_descargador_bitacoras_y_paquete() {
        use egui_kittest::kittest::Queryable;
        use egui_kittest::Harness;
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/paquete.json")).unwrap();
        let e = datos["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "contexto").unwrap();
        let base = perfil.join("eq6");
        let _ = std::fs::remove_dir_all(&base);
        let listados = base.join("lotes").join(e["lote"]["carpeta"].as_str().unwrap());
        let bitacoras = base.join("bitacoras").join(e["bitacoras"]["carpeta"].as_str().unwrap());
        reconstruir(&listados, &e["lote"]["archivos"]);
        reconstruir(&bitacoras, &e["bitacoras"]["archivos"]);
        let catalogo: Catalogo = serde_json::from_value(serde_json::json!({
            "pantalla": "seguimiento", "tribunales": {"49": "Juzgado de Familia Tomé"}, "pestanas": ["Cumplimiento"],
            "modalidades": {"2": "Ambulatorio"}, "informes": {"1": "Por vencer"}, "medidas": {"4": "Medidas por vencer"},
        }))
        .unwrap();
        let vista = Vista {
            contexto: Contexto::desde(&Preferencias::cargar(&Preferencias::ruta())),
            catalogo,
            lote: SeleccionLote {
                tribunales: vec!["49".into()],
                pestanas: vec!["Cumplimiento".into()],
                modalidades: vec!["2".into()],
                ..Default::default()
            },
            listados: None,
            destino: base.display().to_string(),
        };
        let pal = theme::pal(false);
        let mut h = Harness::builder().with_size(egui::Vec2::new(1200.0, 700.0)).build_ui_state(
            move |ui, v: &mut Vista| {
                let Vista { contexto, catalogo, lote, listados, destino } = v;
                contexto.mostrar(ui, &pal, catalogo, lote, None, listados.clone(), destino, 1.0, false);
            },
            vista,
        );
        h.run();
        // Valores por omisión de `preferencias.py`: reutilizar sí; calendario, historia e histórico no; 4 meses.
        let c = &h.state().contexto;
        assert_eq!((c.reutilizar, c.calendarios, c.historia, c.historico, c.historia_en_paquete, c.meses, c.consolidado),
                   (true, false, false, false, true, 4, true));
        assert!(h.query_by_label("Historia en el paquete").is_none(), "«Historia en el paquete» solo con la historia elegida");
        h.get_by_label("Historia de la causa (prueba)").click();
        h.run();
        assert!(h.query_by_label("Historia en el paquete").is_some());
        assert!(h.query_by_label("Conecta la extensión Integral desde la pestaña SITFA.").is_some());
        assert!(h.query_by_label("Descarga el lote para crear su paquete.").is_some());
        let leer = h.get_by_label("Leer bitácoras del lote");
        assert!(leer.is_disabled(), "sin extensión no se leen bitácoras");
        assert!(h.get_by_label("Reintentar lecturas fallidas…").is_disabled(), "sin extensión no se retoma");
        // `saved_batch`: el alcance para retomar sale del manifiesto del lote de bitácoras.
        let guardado = LoteBitacoras::guardado(&bitacoras).unwrap();
        assert_eq!((guardado.tribunales, guardado.modalidades, guardado.pestanas),
                   (vec!["49".to_string()], vec!["2".to_string()], vec!["Cumplimiento".to_string()]));
        assert!(LoteBitacoras::guardado(&listados).is_err(), "una carpeta de listados no es un lote de bitácoras");
        // E-Q7: los eventos del lote se ven como filas revisadas, estado y avisos.
        {
            let c = &mut h.state_mut().contexto;
            c.aplicar_avance("query", &serde_json::json!({"index": 1, "total": 2, "label": "Tribunal ficticio · Cumplimiento"}));
            c.aplicar_avance("diary_progress", &serde_json::json!({"pagina": 1, "registros": 1, "archivo": "b.html", "rit": "X-1-2099", "estado": "LEIDA"}));
            c.aplicar_avance("diary_progress", &serde_json::json!({"pagina": 2, "registros": 1, "archivo": "FALLIDA", "rit": "X-2-2099", "estado": "FALLIDA"}));
            c.aplicar_avance("warning", &serde_json::json!("Aviso ficticio del lote"));
        }
        h.run();
        for etiqueta in ["2 filas revisadas en el lote", "Página 1 · X-1-2099 · LEIDA", "Página 2 · X-2-2099 · FALLIDA", "Aviso ficticio del lote"] {
            assert!(h.query_by_label(etiqueta).is_some(), "falta «{etiqueta}»");
        }
        assert_eq!(h.state().contexto.estado, "Bitácora X-2-2099 · FALLIDA");
        // Preferencias: se guardan con las claves de Python y se recuperan al reconstruir la vista.
        h.state_mut().contexto.historico = true;
        h.state_mut().contexto.meses = 2;
        assert!(h.state().contexto.guardar_preferencias().is_none());
        let guardadas = Preferencias::cargar(&Preferencias::ruta());
        assert!(guardadas.leer_historia && guardadas.leer_historico && guardadas.meses_atras == 2);
        let crudo: Value = serde_json::from_slice(&std::fs::read(Preferencias::ruta()).unwrap()).unwrap();
        for clave in ["reutilizar_bitacoras", "leer_calendarios", "leer_historia", "leer_historico", "historia_en_paquete", "meses_atras"] {
            assert!(crudo.get(clave).is_some(), "falta la preferencia {clave}");
        }
        // Paquete del lote con las bitácoras recién leídas (el mismo `package_current_lot` que Python, 4 meses).
        h.state_mut().contexto.meses = 4;
        h.state_mut().contexto.consolidado = false; // el oráculo de Python se generó sin consolidado
        h.state_mut().listados = Some(listados.clone());
        h.state_mut().contexto.bitacoras = Some(serde_json::json!({
            "manifest": bitacoras.join("bitacoras.json").display().to_string(), "leidas": 2, "fallidas": 0, "consultas_fallidas": 0,
        }));
        h.run();
        assert!(!h.get_by_label("Crear paquete de este lote…").is_disabled());
        let destino = base.join("Captura_CSMP_prueba.zip");
        let ctx = h.ctx.clone();
        h.state_mut().contexto.crear_paquete(&ctx, listados.clone(), destino.clone());
        let inicio = std::time::Instant::now();
        while h.state().contexto.paquete.is_none() && h.state().contexto.error.is_none() {
            assert!(inicio.elapsed() < Duration::from_secs(30), "el paquete no terminó");
            std::thread::sleep(Duration::from_millis(20));
            h.step(); // el spinner repinta sin parar mientras trabaja el hilo
        }
        h.run();
        assert_eq!(h.state().contexto.error, None);
        let mut esperado = e["resultado"].clone();
        esperado["path"] = serde_json::json!(destino.to_string_lossy());
        assert_eq!(h.state().contexto.paquete.as_ref(), Some(&esperado), "resultado distinto de package_current_lot");
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&destino).unwrap()).unwrap();
        let manifiesto: Value = serde_json::from_reader(zip.by_name("manifest.json").unwrap()).unwrap();
        assert_eq!(manifiesto, e["paquete"]["manifest.json"], "el paquete de la interfaz difiere del de Flow Python");
        assert!(h.query_by_label(&format!("Paquete creado: {}", destino.display())).is_some());
        assert!(h.query_by_label(e["resultado"]["reuse"].as_str().unwrap()).is_some());
        assert!(h.query_by_label("El Ingreso Histórico quedó incompleto: revisa sus incidencias en el paquete.").is_some());
        assert!(h.query_by_label("Abrir carpeta del paquete").is_some());
        // «Abrir en Trabajo» deja el paquete listo para que `App` lo abra, una sola vez.
        assert!(h.state().contexto.abrir_en_trabajo.is_none());
        h.get_by_label("Abrir en Trabajo").click();
        h.run();
        assert_eq!(h.state().contexto.abrir_en_trabajo.as_deref(), Some(destino.as_path()));
        assert_eq!(h.state_mut().contexto.abrir_en_trabajo.take().as_deref(), Some(destino.as_path()));
        assert!(h.state().contexto.abrir_en_trabajo.is_none());
        // Con «XLSX consolidado», el lote de contexto no es apto para CSMP: el paquete sale sin libro y lo dice.
        h.state_mut().contexto.consolidado = true;
        let con_libro = base.join("Captura_CSMP_consolidado.zip");
        h.state_mut().contexto.crear_paquete(&ctx, listados.clone(), con_libro.clone());
        let inicio = std::time::Instant::now();
        while h.state().contexto.activa() {
            assert!(inicio.elapsed() < Duration::from_secs(30), "el paquete no terminó");
            std::thread::sleep(Duration::from_millis(20));
            h.step();
        }
        h.run();
        let reuse = h.state().contexto.paquete.as_ref().unwrap()["reuse"].as_str().unwrap().to_string();
        assert!(reuse.contains(" Sin XLSX consolidado: No apto para CSMP"), "{reuse}");
        assert!(h.query_by_label(&reuse).is_some());
        // Un destino existente nunca se reemplaza.
        h.state_mut().contexto.crear_paquete(&ctx, listados, destino.clone());
        h.run();
        assert_eq!(
            h.state().contexto.error.as_deref(),
            Some("Ese archivo ya existe. Elige un nombre nuevo para no reemplazar otro paquete.")
        );
        let _ = std::fs::remove_dir_all(&base);
        println!("E-Q6_DESCARGADOR: opciones, preferencias, impedimentos y paquete del lote verificados");
    }
}
