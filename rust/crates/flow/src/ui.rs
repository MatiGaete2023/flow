//! Ventana de Flow: Trabajo → Correos → Resoluciones → Resultados.
//! Todo queda en el equipo. No escribe en RUS ni SATURNO y no envía correos.

use crate::cli;
use eframe::egui::{self, Key, RichText};
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
    Meta4,
    Revision,
}

/// Qué herramienta abre el ejecutable. Se decide por su nombre (`Flow_Descargador.exe`, …); `flow.exe` abre todas.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Herramienta {
    Todas,
    Asistente,
    Descargador,
    Anonimizador,
    Meta4,
}

impl Herramienta {
    pub fn desde_ejecutable(nombre: &str) -> Self {
        let n = nombre.to_lowercase();
        if n.contains("descargador") || n.contains("captura") {
            Herramienta::Descargador
        } else if n.contains("anonim") {
            Herramienta::Anonimizador
        } else if n.contains("meta4") || n.contains("meta_4") || n.contains("meta-4") {
            Herramienta::Meta4
        } else if n.contains("asistente") {
            Herramienta::Asistente
        } else {
            Herramienta::Todas
        }
    }

    pub fn titulo(self) -> &'static str {
        match self {
            Herramienta::Todas => "Flow",
            Herramienta::Asistente => "Flow · Asistente",
            Herramienta::Descargador => "Flow · Descargador",
            Herramienta::Anonimizador => "Flow · Anonimizador",
            Herramienta::Meta4 => "Flow · Meta 4",
        }
    }

    fn pestanas(self) -> &'static [(Pestana, &'static str)] {
        const TODAS: &[(Pestana, &str)] = &[
            (Pestana::Captura, "Captura"),
            (Pestana::Trabajo, "1 · Trabajo"),
            (Pestana::Correos, "2 · Correos"),
            (Pestana::Resoluciones, "3 · Resoluciones"),
            (Pestana::Resultados, "4 · Resultados"),
            (Pestana::Contactos, "Contactos"),
            (Pestana::Anonimo, "Anonimizador"),
            (Pestana::Meta4, "Meta 4"),
            (Pestana::Revision, "Revisión de bitácoras"),
        ];
        match self {
            Herramienta::Todas => TODAS,
            Herramienta::Asistente => &[
                (Pestana::Trabajo,"1 · Trabajo"),(Pestana::Correos,"2 · Correos"),
                (Pestana::Resoluciones,"3 · Resoluciones"),(Pestana::Resultados,"4 · Resultados"),
                (Pestana::Contactos,"Contactos"),(Pestana::Revision,"Revisión de bitácoras"),
            ],
            Herramienta::Descargador => &TODAS[0..1],
            Herramienta::Anonimizador => &TODAS[6..7],
            Herramienta::Meta4 => &TODAS[7..8],
        }
    }

    /// Las herramientas sueltas no analizan RUS: no muestran apertura de Excel, corte ni etapa.
    fn analiza_rus(self) -> bool {
        matches!(self, Herramienta::Todas | Herramienta::Asistente)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filtro {
    Todos,
    PorRevisar,
    ConAccion,
    Editados,
}

/// Versión del estado guardado con la sesión (`session.py`: `version=2`).
const VERSION_SESION: u32 = 2;

/// Lo que la sesión guarda además del trabajo (`session.py::capture`): borradores y proyectos preparados con sus
/// ediciones, la selección con que se prepararon los proyectos y las preferencias de la ventana. El estado de cada
/// borrador en Outlook no se guarda aquí: se lee del diario de recibos.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct EstadoSesion {
    version: u32,
    #[serde(default)]
    borradores: Vec<Borrador>,
    #[serde(default)]
    proyectos: Vec<word::Proyecto>,
    #[serde(default)]
    seleccion_preparada: Vec<(String, String)>,
    #[serde(default)]
    modalidades: Vec<String>,
    #[serde(default)]
    otra_modalidad: String,
    #[serde(default)]
    periodo: String,
    /// Filas con proyecto Word generado (recibos `word` de Python): alimentan el reporte de proyectos.
    #[serde(default)]
    word_generados: std::collections::BTreeSet<String>,
    #[serde(default)]
    carpeta: Option<PathBuf>,
    /// Filas marcadas en Trabajo y «Solo filas seleccionadas» (`preferences['records']`, `manual_mail`).
    #[serde(default)]
    marcadas: std::collections::BTreeSet<String>,
    #[serde(default)]
    manual: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Ajustes {
    salida: Option<PathBuf>,
}

struct App {
    herramienta: Herramienta,
    datos: PathBuf,
    ajustes: Ajustes,
    contactos: Contactos,
    contactos_error: Option<String>,
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
    correo_activo: Option<usize>,
    estados_borradores: std::collections::BTreeMap<usize, crate::borradores::Estado>,
    proyectos: Vec<word::Proyecto>,
    proyecto_activo: Option<usize>,
    /// Selección (fila, tipo) con que se prepararon los proyectos; si cambia, se preparan de nuevo antes de generar.
    seleccion_preparada: Vec<(String, String)>,
    word_generados: std::collections::BTreeSet<String>,
    /// PERIODO de las plantillas que lo usan (reporte de proyectos).
    periodo: String,
    /// Tipo elegido para «Preparar este tipo».
    tipo_elegido: Tipo,
    /// Filas marcadas en Trabajo para «Solo filas seleccionadas».
    marcadas: std::collections::BTreeSet<String>,
    manual: bool,
    resultados: Vec<String>,
    biblioteca: crate::resultados_ui::Biblioteca,
    carpeta: Option<PathBuf>,
    mensaje: Option<(bool, String)>,
    outlook: bool,
    /// Programa en edición: (nombre, correos, alias separados por «;»).
    nuevo: (String, String, String),
    /// Nombre del programa que se está editando (para renombrar); `None`: contacto nuevo.
    edicion: Option<String>,
    buscar_contacto: String,
    ficha_contacto:bool,
    /// Contactos leídos del Asistente Python (solo lectura): no se guardan en Flow hasta que el usuario los confirme.
    contactos_csmp: Option<PathBuf>,
    /// Importación revisada y pendiente de confirmar: (origen, resultado).
    importacion: Option<(String, correos::Importacion)>,
    ayuda: bool,
    sin_guardar: bool,
    confirmar_recalculo: bool,
    panel_trabajo:u8,
    distribucion:u64,
    captura: crate::captura_ui::Captura,
    anonimo: crate::anonimo_ui::Anonimo,
    meta4: crate::meta4_ui::Meta4,
    revision: crate::revision_bitacoras_ui::RevisionUi,
    evidencia: Option<crate::evidencia_ui::Captura>,
}

fn ruta_ajustes(datos: &Path) -> PathBuf {
    datos.join("ajustes.json")
}

fn ruta_contactos(datos: &Path) -> PathBuf {
    datos.join("contactos.json")
}

/// `config.contacts_missing`: sin programas ni destinatarios de tribunal (un archivo vacío no cuenta como propio).
fn sin_contactos(c: &Contactos) -> bool {
    c.programas.is_empty() && c.tribunales.values().all(|v| v.trim().is_empty())
}

/// `app_base.CONTACTS_MISSING`, con los botones de Flow.
const CONTACTOS_FALTAN: &str = "Faltan los contactos. En la pestaña Contactos pulsa «Importar contactos de CSMP…» y elige \
tu contactos.json del Asistente anterior, o «Importar catastro Excel…». Mientras tanto, los correos se preparan con el \
destinatario pendiente.";

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
    ["rit", "nombre", "programa", "rut", "tribunal"].iter().any(|c| flow_dominio::texto::normalizar(&fila.registro.texto(c)).contains(&b))
        || flow_dominio::texto::normalizar(&fila.observacion).contains(&b)
}

impl App {
    fn atajo_guardar(&mut self, guardar: bool) {
        if guardar && self.pestana == Pestana::Trabajo && self.trabajo.is_some() {
            self.guardar_sesion();
            self.guardar_excel();
        }
    }
    fn nueva(archivo: Option<PathBuf>, herramienta: Herramienta) -> Self {
        let mut app = Self::con_origen(almacen::carpeta_datos(), archivo, &correos::rutas_csmp());
        app.usar_herramienta(herramienta);
        if herramienta == Herramienta::Descargador && app.mensaje.as_ref().is_some_and(|(_, m)| m == CONTACTOS_FALTAN) {
            app.mensaje = None;
        }
        app
    }

    fn usar_herramienta(&mut self, herramienta: Herramienta) {
        self.herramienta = herramienta;
        self.captura.independiente = herramienta == Herramienta::Descargador;
        if !herramienta.pestanas().iter().any(|(p, _)| *p == self.pestana) {
            self.pestana = herramienta.pestanas()[0].0;
        }
    }

    /// Sin leer los contactos de CSMP del equipo (pruebas).
    #[cfg(test)]
    fn con_datos(datos: PathBuf, archivo: Option<PathBuf>) -> Self {
        Self::con_origen(datos, archivo, &[])
    }

    /// `csmp`: archivos de contactos del Asistente Python (`correos::rutas_csmp`), que se leen (sin modificarlos) en
    /// ese orden si Flow aún no tiene contactos.
    fn con_origen(datos: PathBuf, archivo: Option<PathBuf>, csmp: &[PathBuf]) -> Self {
        let _ = std::fs::create_dir_all(&datos);
        almacen::aplicar_retencion(&datos, std::time::SystemTime::now());
        let ajustes = std::fs::read_to_string(ruta_ajustes(&datos)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let (mut contactos, contactos_error) = match Contactos::cargar(&ruta_contactos(&datos)) {
            Ok(c) => (c, None),
            Err(e) => (Contactos::default(), Some(e)),
        };
        let mut aviso_contactos = None;
        let mut contactos_csmp = None;
        if contactos_error.is_none() && sin_contactos(&contactos) {
            let leidos: Vec<(PathBuf, Result<Option<Contactos>, String>)> =
                csmp.iter().map(|r| (r.clone(), Contactos::leer_csmp(r))).collect();
            let hallado = leidos.iter().find_map(|(r, l)| l.as_ref().ok().cloned().flatten().map(|c| (r.clone(), c)));
            let error = leidos.iter().find_map(|(_, l)| l.as_ref().err().cloned());
            match (hallado, error) {
                (Some((ruta, mut c)), _) => {
                    // Lo propio de Flow (CC adicional, firma, alias) prevalece sobre lo de CSMP.
                    if !contactos.cc_adicional.trim().is_empty() {
                        c.cc_adicional = contactos.cc_adicional.clone();
                    }
                    if !contactos.firma.trim().is_empty() {
                        c.firma = contactos.firma.clone();
                    }
                    c.aliases.extend(contactos.aliases.clone());
                    aviso_contactos = Some((
                        false,
                        format!(
                            "Contactos tomados de CSMP ({} programas). Revísalos en la pestaña Contactos y pulsa «Guardar en \
                             Flow» para conservarlos; el archivo de CSMP no se modifica.",
                            c.programas.len()
                        ),
                    ));
                    contactos = c;
                    contactos_csmp = Some(ruta);
                }
                (None, Some(e)) => aviso_contactos = Some((true, format!("{e} {CONTACTOS_FALTAN}"))),
                (None, None) => aviso_contactos = Some((false, CONTACTOS_FALTAN.to_string())),
            }
        }
        let mut app = App {
            herramienta: Herramienta::Todas,
            contactos,
            contactos_error: contactos_error.clone(),
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
            correo_activo: None,
            estados_borradores: Default::default(),
            proyectos: Vec::new(),
            proyecto_activo: None,
            seleccion_preparada: Vec::new(),
            word_generados: Default::default(),
            periodo: flow_tabular::libro::hoy().format("%m/%Y").to_string(),
            tipo_elegido: Tipo::Proyectos,
            marcadas: Default::default(),
            manual: false,
            resultados: Vec::new(),
            biblioteca: Default::default(),
            carpeta: None,
            mensaje: contactos_error.map(|e| (true, e)).or(aviso_contactos),
            outlook: flow_outlook::disponible(),
            nuevo: Default::default(),
            edicion: None,
            buscar_contacto: String::new(),
            ficha_contacto:false,
            contactos_csmp,
            importacion: None,
            ayuda: false,
            sin_guardar: false,
            confirmar_recalculo: false,
            panel_trabajo:0,
            distribucion:0,
            captura: Default::default(),
            anonimo: Default::default(),
            meta4: Default::default(),
            revision: Default::default(),
            evidencia: None,
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

    /// Recupera las sesiones de los trabajos recién abiertos. Devuelve cuántas y los avisos para mostrar una vez.
    fn recuperar_trabajos(&mut self, trabajos: &mut [Trabajo]) -> Result<(usize, Vec<String>), String> {
        let mut recuperados = 0;
        let mut avisos = Vec::new();
        for trabajo in trabajos {
            let (recuperada, aviso) = almacen::recuperar(&self.datos, trabajo)?;
            recuperados += usize::from(recuperada);
            avisos.extend(aviso);
        }
        Ok((recuperados, avisos))
    }
    fn cargar(&mut self, ruta: PathBuf) {
        self.cargar_modo(ruta, false);
    }
    fn cargar_modo(&mut self, ruta: PathBuf, reanalizar: bool) {
        let Some(corte) = self.fecha_corte() else {
            return self.mal("La fecha de corte no se entiende. Usa dd/mm/aaaa.");
        };
        match cli::cargar(&ruta, self.etapa, None, corte, &self.motor) {
            Ok(mut trabajos) => {
                if !self.guardar_sesion() {
                    return;
                }
                let (recuperados, avisos) = if reanalizar {
                    (0, Vec::new())
                } else {
                    match self.recuperar_trabajos(&mut trabajos) {
                        Ok(r) => r,
                        Err(e) => return self.mal(e),
                    }
                };
                let trabajo = trabajos.remove(0);
                let mut texto =
                    format!("{}: {} registros, {} con observación.", trabajo.nombre(), trabajo.filas.len(), trabajo.con_observacion());
                if !trabajos.is_empty() {
                    let nombres: Vec<&str> = trabajos.iter().map(|t| t.nombre()).collect();
                    texto.push_str(&format!(" El archivo trae además: {}.", nombres.join(", ")));
                }
                if recuperados > 0 {
                    texto.push_str(" Se recuperaron las ediciones y marcas de la sesión guardada.");
                }
                // Reglas distintas o sesión ilegible: se dice una vez, sin guardarlo en la sesión.
                for aviso in avisos {
                    texto.push(' ');
                    texto.push_str(&aviso);
                }
                if let Some(aviso) = self.activar(trabajo) {
                    texto.push(' ');
                    texto.push_str(&aviso);
                }
                self.otros = trabajos;
                self.archivo = Some(ruta);
                self.pestana = Pestana::Trabajo;
                self.guardar_sesion();
                self.bien(texto);
            }
            Err(e) => self.mal(e),
        }
    }

    /// Guarda en el trabajo activo lo preparado en la ventana (`session.py::capture`).
    fn capturar_estado(&mut self) {
        let Some(t) = &mut self.trabajo else { return };
        let estado = EstadoSesion {
            version: VERSION_SESION,
            borradores: self.borradores.clone(),
            proyectos: self.proyectos.clone(),
            seleccion_preparada: self.seleccion_preparada.clone(),
            modalidades: self.modalidades.iter().filter(|(_, si)| *si).map(|(m, _)| m.clone()).collect(),
            otra_modalidad: self.otra_modalidad.clone(),
            periodo: self.periodo.clone(),
            word_generados: self.word_generados.clone(),
            carpeta: self.carpeta.clone(),
            marcadas: self.marcadas.clone(),
            manual: self.manual,
        };
        t.sesion = serde_json::to_value(&estado).ok();
    }

    /// Deja un trabajo como el activo: descarta los productos del anterior y recupera los que guardó su sesión
    /// (`session.py::restore`). Devuelve un texto para mostrar una vez, si corresponde.
    fn activar(&mut self, trabajo: Trabajo) -> Option<String> {
        self.seleccion = (!trabajo.filas.is_empty()).then_some(0);
        self.borradores.clear();
        self.estados_borradores.clear();
        self.proyectos.clear();
        self.seleccion_preparada.clear();
        self.word_generados.clear();
        self.marcadas.clear();
        self.manual = false;
        self.biblioteca.recordar(self.carpeta.as_deref());
        self.carpeta = None;
        let aviso = match trabajo.sesion.clone().map(serde_json::from_value::<EstadoSesion>) {
            None => None,
            Some(Err(_)) => Some("No se pudieron leer los productos guardados en la sesión; vuelve a prepararlos.".to_string()),
            Some(Ok(e)) if e.version != VERSION_SESION => {
                Some("La versión de los productos guardados no es compatible; vuelve a prepararlos.".to_string())
            }
            Some(Ok(e)) => {
                for (m, marcada) in &mut self.modalidades {
                    *marcada = e.modalidades.contains(m);
                }
                self.otra_modalidad = e.otra_modalidad;
                if !e.periodo.trim().is_empty() {
                    self.periodo = e.periodo;
                }
                self.borradores = e.borradores;
                self.proyectos = e.proyectos;
                self.seleccion_preparada = e.seleccion_preparada;
                self.word_generados = e.word_generados;
                self.carpeta = e.carpeta.filter(|c| c.is_dir());
                self.marcadas = e.marcadas.into_iter().filter(|id| trabajo.filas.iter().any(|f| &f.id == id)).collect();
                self.manual = e.manual;
                // El estado en Outlook sale del diario de recibos, no de la sesión.
                for (i, b) in self.borradores.iter().enumerate() {
                    if let Ok(Some(estado)) = crate::borradores::estado(&self.datos, b) {
                        self.estados_borradores.insert(i, estado);
                    }
                }
                (!self.borradores.is_empty() || !self.proyectos.is_empty()).then(|| {
                    format!("Se recuperaron {} borrador(es) y {} proyecto(s) preparados.", self.borradores.len(), self.proyectos.len())
                })
            }
        };
        self.trabajo = Some(trabajo);
        // Productos de otras reglas o de registros que cambiaron: se conservan, pero hay que prepararlos de nuevo.
        let obsoletos = self.borradores_obsoletos().len() + self.proyectos_obsoletos().len();
        if obsoletos > 0 {
            let nota = format!("{obsoletos} producto(s) preparado(s) quedan obsoletos: prepáralos de nuevo antes de guardarlos.");
            return Some(aviso.map_or(nota.clone(), |a| format!("{a} {nota}")));
        }
        aviso
    }

    /// Cambia a otra etapa del mismo archivo conservando las ediciones de ambas.
    fn cambiar_etapa(&mut self, indice: usize) {
        self.guardar_sesion();
        let nuevo = self.otros.remove(indice);
        if let Some(actual) = self.trabajo.take() {
            self.otros.push(actual);
        }
        let mut texto = format!("{}: {} registros.", nuevo.nombre(), nuevo.filas.len());
        if let Some(aviso) = self.activar(nuevo) {
            texto.push(' ');
            texto.push_str(&aviso);
        }
        self.bien(texto);
    }

    fn elegir_archivo(&mut self) {
        let dialogo = rfd::FileDialog::new()
            .add_filter("Excel de RUS o paquete del Descargador", &["xls", "xlsx", "xlsm", "zip"])
            .set_title("Elegir Excel de RUS o paquete ZIP");
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
        if !self.guardar_sesion() {
            return;
        }
        let avisos = match self.recuperar_trabajos(&mut trabajos) {
            Ok((_, avisos)) => avisos,
            Err(e) => return self.mal(e),
        };
        let actual = trabajos.remove(0);
        let recuperado = self.activar(actual);
        self.otros = trabajos;
        self.archivo = None;
        self.pestana = Pestana::Trabajo;
        self.guardar_sesion();
        let mut texto = "Captura cargada. Revisa las observaciones antes de preparar los productos.".to_string();
        for aviso in avisos.into_iter().chain(recuperado) {
            texto.push(' ');
            texto.push_str(&aviso);
        }
        self.bien(texto);
    }

    fn guardar_sesion(&mut self) -> bool {
        self.capturar_estado();
        if let Some(t) = &self.trabajo {
            if let Err(e) = almacen::guardar(&self.datos, t) {
                self.mal(e);
                return false;
            }
        }
        self.sin_guardar = false;
        true
    }

    fn reanudar(&mut self) {
        match almacen::listar(&self.datos).first().map(|(r, _)| almacen::cargar(r)) {
            Some(Ok(t)) => {
                self.corte = t.corte.format("%d/%m/%Y").to_string();
                let mut texto = format!("Trabajo reanudado: {} del {} ({}).", t.nombre(), self.corte, t.archivo);
                if t.version_reglas != flow_trabajo::version_reglas(&self.motor) {
                    texto.push_str(
                        " Sus propuestas se calcularon con otras reglas: abre de nuevo el archivo para recalcularlas \
                         (se conservan tus ediciones y marcas).",
                    );
                }
                if let Some(aviso) = self.activar(t) {
                    texto.push(' ');
                    texto.push_str(&aviso);
                }
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

    /// Claves de las modalidades marcadas para filtrar filas (`modalities.py::selected_row`). Sin marcas, o con
    /// «Otra…» (no se sabe qué filas comprende), no se filtra.
    fn claves_modalidades(&self) -> Option<std::collections::BTreeSet<&'static str>> {
        if !self.otra_modalidad.trim().is_empty() {
            return None;
        }
        correos::claves_modalidad(self.modalidades.iter().filter(|(_, si)| *si).map(|(m, _)| m.as_str()))
    }

    /// Reglas vigentes en el programa (huella de vigencia de los productos).
    fn reglas(&self) -> String {
        flow_trabajo::version_reglas(&self.motor)
    }

    /// `_mail_selected_ids`: con «Solo filas seleccionadas», las marcadas en Trabajo (al menos una).
    fn seleccion_correos(&self) -> Result<Option<std::collections::BTreeSet<String>>, String> {
        if !self.manual {
            return Ok(None);
        }
        let ids: std::collections::BTreeSet<String> =
            self.trabajo.iter().flat_map(|t| &t.filas).filter(|f| self.marcadas.contains(&f.id)).map(|f| f.id.clone()).collect();
        if ids.is_empty() {
            return Err("Selecciona los registros de esta gestión en la pestaña Trabajo.".into());
        }
        Ok(Some(ids))
    }

    /// Deja los borradores recién preparados conservando las ediciones de los anteriores (`_display_prepared_drafts`).
    fn mostrar_borradores(&mut self, mut lista: Vec<Borrador>) {
        correos::conservar_ediciones(&self.borradores, &mut lista);
        self.borradores = lista;
        self.correo_activo = None;
        self.estados_borradores.clear();
        for (i, b) in self.borradores.iter().enumerate() {
            if let Ok(Some(estado)) = crate::borradores::estado(&self.datos, b) {
                self.estados_borradores.insert(i, estado);
            }
        }
    }

    /// «Preparar todos» de Python (`prepare_required_drafts`): exige modalidades; el reporte de proyectos y la gestión
    /// particular se preparan a pedido.
    fn preparar_borradores(&mut self) {
        if self.trabajo.as_ref().is_some_and(|t| t.fuente.is_some()) {
            return self.mal("Esta consulta auxiliar no genera borradores ni proyectos de resolución.");
        }
        let seleccion = match self.seleccion_correos() {
            Ok(s) => s,
            Err(e) => return self.mal(e),
        };
        let opciones = correos::Opciones {
            modalidades: self.texto_modalidades(),
            hoy: Some(flow_tabular::libro::hoy()),
            claves_modalidad: self.claves_modalidades(),
            periodo: self.periodo.clone(),
            con_word: self.word_generados.clone(),
            seleccion,
            manual: false,
            reglas: self.reglas(),
        };
        if opciones.modalidades.is_empty() {
            return self.mal("Selecciona al menos una modalidad para preparar el conjunto de correos del trabajo.");
        }
        let Some(carpeta) = self.carpeta_productos() else { return };
        let Some(trabajo) = &self.trabajo else { return };
        let (lista, fallidos) = match correos::preparar_todos(trabajo, &opciones, &self.contactos, &carpeta.join("correos")) {
            Ok(r) => r,
            Err(e) => return self.mal(e),
        };
        let mut texto = format!("{} correos preparados para el alcance elegido. Todavía no se guardaron en Outlook.", lista.len());
        if !fallidos.is_empty() {
            texto.push_str(&format!(" No se prepararon: {}", fallidos.join(" ")));
        }
        self.mostrar_borradores(lista);
        if fallidos.is_empty() {
            self.bien(texto);
        } else {
            self.mal(texto);
        }
    }

    /// «Preparar tipo» de Python: prepara solo el tipo elegido (reemplaza la lista preparada). Las modalidades se
    /// exigen y filtran solo si la plantilla las usa.
    fn preparar_tipo(&mut self, tipo: Tipo) {
        if self.trabajo.as_ref().is_some_and(|t| t.fuente.is_some()) {
            return self.mal("Esta consulta auxiliar no genera borradores ni proyectos de resolución.");
        }
        let Some(etapa) = self.trabajo.as_ref().map(|t| t.etapa) else { return };
        let usa = tipo.usa_modalidades(etapa);
        let modalidades = if usa { self.texto_modalidades() } else { String::new() };
        if usa && modalidades.is_empty() {
            return self.mal("Selecciona al menos una modalidad para este tipo de correo.");
        }
        let seleccion = match self.seleccion_correos() {
            Ok(s) => s,
            Err(e) => return self.mal(e),
        };
        let opciones = correos::Opciones {
            modalidades,
            hoy: Some(flow_tabular::libro::hoy()),
            claves_modalidad: if usa { self.claves_modalidades() } else { None },
            periodo: self.periodo.trim().to_string(),
            con_word: self.word_generados.clone(),
            manual: seleccion.is_some(),
            seleccion,
            reglas: self.reglas(),
        };
        let Some(carpeta) = self.carpeta_productos() else { return };
        let Some(trabajo) = &self.trabajo else { return };
        match correos::preparar(trabajo, tipo, &opciones, &self.contactos, &carpeta.join("correos")) {
            Ok(lista) if lista.is_empty() => self.mal(if tipo == Tipo::Proyectos {
                "No hay registros con proyecto Word generado en este trabajo: genera primero el Word en Resoluciones."
            } else {
                "No hay registros del trabajo para este tipo de correo."
            }),
            Ok(lista) => {
                let texto = format!(
                    "{} borradores de «{}» preparados para revisión; todavía no se guardaron en Outlook.",
                    lista.len(),
                    tipo.titulo(etapa)
                );
                self.mostrar_borradores(lista);
                self.bien(texto);
            }
            Err(e) => self.mal(format!("{}: {e}", tipo.titulo(etapa))),
        }
    }

    /// Guarda los borradores listos. En Outlook quedan en Borradores; si no, como archivos .eml sin enviar.
    fn guardar_borradores(&mut self, en_outlook: bool) {
        self.guardar_borradores_con(en_outlook, &flow_outlook::guardar_borrador);
    }
    fn guardar_borradores_con(&mut self, en_outlook: bool, guardar: &dyn Fn(&Borrador) -> Result<flow_outlook::Resultado, String>) {
        let Some(carpeta) = self.carpeta_productos() else {
            return;
        };
        let (mut hechos, mut ya_guardados, mut aprendidos) = (0, 0, 0);
        let mut problemas = Vec::new();
        // `require_fresh`: antes de cualquier guardado, con la configuración de ahora.
        let obsoletos = self.borradores_obsoletos();
        for (i, b) in self.borradores.iter().enumerate().filter(|(_, b)| !b.omitido) {
            let quien = if b.programa.is_empty() { b.tribunal.clone() } else { b.programa.clone() };
            if obsoletos.contains(&i) {
                problemas.push(format!("{quien}: {}", flow_productos::vigencia::OBSOLETO));
                continue;
            }
            if !b.faltantes().is_empty() {
                problemas.push(format!("{quien}: falta {}", b.faltantes().join(", ")));
                continue;
            }
            let linea = if en_outlook {
                match crate::borradores::ejecutar(&self.datos, b, guardar) {
                    crate::borradores::Resultado::Guardado => {
                        self.estados_borradores.insert(i, crate::borradores::Estado::Guardado);
                        format!("Borrador en Outlook · {} → {quien}", b.asunto)
                    }
                    crate::borradores::Resultado::YaGuardado => {
                        self.estados_borradores.insert(i, crate::borradores::Estado::Guardado);
                        ya_guardados += 1;
                        continue;
                    }
                    crate::borradores::Resultado::Incierto(m) => {
                        self.estados_borradores.insert(i, crate::borradores::Estado::Incierto);
                        problemas.push(format!("{quien}: guardado incierto. {m}"));
                        self.resultados.push(format!("REVISAR en Outlook · {quien}: {m}"));
                        continue;
                    }
                    crate::borradores::Resultado::Bloqueado(e) => {
                        self.estados_borradores.remove(&i);
                        problemas.push(format!("{quien}: no se intentó guardar. {e}"));
                        continue;
                    }
                    crate::borradores::Resultado::Fallido(e) => {
                        self.estados_borradores.insert(i, crate::borradores::Estado::NoGuardado);
                        problemas.push(format!("{quien}: {e}"));
                        continue;
                    }
                }
            } else {
                match correos::guardar_eml(b, &carpeta.join("correos")) {
                    Ok(p) => format!("Borrador .eml · {}", p.display()),
                    Err(e) => {
                        problemas.push(format!("{quien}: {e}"));
                        continue;
                    }
                }
            };
            hechos += 1;
            self.resultados.push(linea);
            if self.contactos_error.is_none() && !b.programa.is_empty() && self.contactos.de_programa(&b.programa).is_empty() {
                self.contactos.programas.insert(b.programa.clone(), correos::direcciones(&b.para).join("; "));
                aprendidos += 1;
            }
        }
        // Con los contactos de CSMP sin confirmar, lo aprendido queda en memoria hasta «Guardar en Flow».
        if aprendidos > 0 && self.contactos_csmp.is_none() {
            if let Err(e) = self.contactos.guardar(&ruta_contactos(&self.datos)) {
                problemas.push(e);
            }
        }
        let destino = if en_outlook { "en Borradores de Outlook" } else { "como archivos .eml" };
        let resumen = format!("{hechos} guardados {destino}; {ya_guardados} ya estaban guardados y se omitieron.");
        if problemas.is_empty() {
            self.bien(format!("{resumen} Ninguno fue enviado."));
        } else {
            self.mal(format!("{resumen} Pendientes: {}.", problemas.join("; ")));
        }
    }

    /// Borradores obsoletos (`product_state.stale`): cambiaron sus registros, los contactos o las reglas.
    fn borradores_obsoletos(&self) -> std::collections::BTreeSet<usize> {
        let Some(t) = &self.trabajo else { return Default::default() };
        if self.borradores.is_empty() {
            return Default::default();
        }
        let huellas = flow_productos::vigencia::Huellas::nuevas(t, &self.reglas());
        self.borradores.iter().enumerate().filter(|(_, b)| huellas.borrador_obsoleto(b, &self.contactos)).map(|(i, _)| i).collect()
    }

    /// Proyectos obsoletos: cambiaron sus registros, su plantilla o las reglas.
    fn proyectos_obsoletos(&self) -> std::collections::BTreeSet<usize> {
        let Some(t) = &self.trabajo else { return Default::default() };
        if self.proyectos.is_empty() {
            return Default::default();
        }
        let propias = self.plantillas_usuario();
        let huellas = flow_productos::vigencia::Huellas::nuevas(t, &self.reglas());
        self.proyectos
            .iter()
            .enumerate()
            .filter(|(_, p)| huellas.proyecto_obsoleto(p, word::plantilla(p.tribunal, &p.tipo, Some(&propias)).as_deref()))
            .map(|(i, _)| i)
            .collect()
    }

    fn plantillas_usuario(&self) -> PathBuf {
        self.datos.join("plantillas")
    }

    /// `_prepare_words`: agrupa por tribunal, RIT y tipo y conserva los valores editados de la preparación anterior.
    fn preparar_proyectos(&mut self) {
        let Some(trabajo) = &self.trabajo else { return };
        let propias = self.plantillas_usuario();
        let seleccion = word::selecciones(trabajo);
        let mut nuevos = word::preparar(trabajo, &seleccion, flow_tabular::libro::hoy(), Some(&propias), &self.reglas());
        word::conservar_ediciones(&self.proyectos, &mut nuevos, Some(&propias));
        self.proyectos = nuevos;
        self.proyecto_activo = None;
        self.seleccion_preparada = seleccion;
        let por_completar = self.proyectos.iter().filter(|p| p.listo() && !p.faltantes.is_empty()).count();
        let sin_plantilla = self.proyectos.iter().filter(|p| p.sin_plantilla).count();
        self.bien(format!(
            "{} proyectos agrupados por tribunal, RIT y tipo; {sin_plantilla} sin plantilla. {por_completar} llevan datos \
             [COMPLETAR …] para completar en el Word.",
            self.proyectos.len()
        ));
    }

    fn generar_word(&mut self) {
        // Como `_generate_words`: si hay proyectos obsoletos o cambió lo que corresponde preparar, se preparan de
        // nuevo (conservando las ediciones) antes de generar.
        let vigente = self.trabajo.as_ref().map(word::selecciones).unwrap_or_default();
        if self.proyectos.is_empty() || vigente != self.seleccion_preparada || !self.proyectos_obsoletos().is_empty() {
            self.preparar_proyectos();
        }
        let Some(carpeta) = self.carpeta_productos() else { return };
        let propias = self.plantillas_usuario();
        match word::generar(&self.proyectos, &carpeta, Some(&propias)) {
            Ok(rutas) => {
                for r in &rutas {
                    self.resultados.push(format!("Proyectos Word · {}", r.display()));
                }
                let filas: Vec<String> = self.proyectos.iter().filter(|p| p.listo()).flat_map(|p| p.filas.clone()).collect();
                self.word_generados.extend(filas);
                self.guardar_sesion();
                let mut texto = format!("Word conjunto generado: {}.", rutas[0].display());
                if rutas.len() > 1 {
                    texto.push_str(&format!(
                        " {} plantilla(s) con imágenes, vínculos o numeración quedaron en documento aparte.",
                        rutas.len() - 1
                    ));
                }
                texto.push_str(" Revísalo y completa lo marcado [COMPLETAR …] antes de usarlo.");
                self.bien(texto);
            }
            Err(e) => self.mal(e),
        }
    }

    fn barra(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading(self.herramienta.titulo());
            if !self.herramienta.analiza_rus() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Ayuda (F1)").clicked() {
                        self.ayuda = true;
                    }
                });
                return;
            }
            ui.separator();
            ui.menu_button("Archivo", |ui| {
                if ui.button("Abrir Excel o paquete…  (Ctrl+O)").clicked() {
                    ui.close_menu(); self.elegir_archivo();
                }
                let recuperable = almacen::listar(&self.datos).first().map(|(r,_)|r.clone());
                let etiqueta = recuperable.as_ref().and_then(|r|almacen::cargar(r).ok())
                    .map(|t|format!("Reanudar {} · corte {}",t.archivo,t.corte))
                    .unwrap_or_else(||"Reanudar último trabajo".into());
                if ui.add_enabled(recuperable.is_some(),egui::Button::new(etiqueta))
                    .on_hover_text("Recupera la sesión local más reciente. Sin sesión guardada, esta opción no está disponible.").clicked() {
                    ui.close_menu(); self.reanudar();
                }
            });
            if self.pestana == Pestana::Trabajo {
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
                ui.menu_button("Más", |ui| {
                    if ui.add_enabled(self.archivo.is_some(), egui::Button::new("Recalcular propuestas…"))
                        .on_hover_text("Permite descartar las ediciones después de una confirmación.").clicked() {
                        self.confirmar_recalculo = true; ui.close_menu();
                    }
                });
            } else if let Some(t) = &self.trabajo {
                if self.pestana != Pestana::Revision {
                ui.small(format!("{} · {} · corte {}",t.archivo,t.nombre(),t.corte));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Ayuda (F1)").clicked() {
                    self.ayuda = true;
                }
            });
        });
        let pestanas = self.herramienta.pestanas();
        if pestanas.len()>1 {
            if ui.available_width()<1000.0 {
                let actual=pestanas.iter().find(|(p,_)|*p==self.pestana).map(|(_,n)|*n).unwrap_or("Herramientas");
                ui.horizontal_wrapped(|ui|{
                    ui.label("Herramienta:");
                    egui::ComboBox::from_id_salt("navegacion_compacta").width(260.0).selected_text(actual).show_ui(ui,|ui|{
                        for (p,nombre) in pestanas {ui.selectable_value(&mut self.pestana,*p,*nombre);}
                    });
                });
            }else{
                ui.horizontal_wrapped(|ui|{for (p,nombre) in pestanas{ui.selectable_value(&mut self.pestana,*p,*nombre);}});
            }
        }
    }

    fn pie(&mut self, ui: &mut egui::Ui) {
        if ui.available_width()<1000.0 {
            ui.horizontal(|ui| {
                ui.menu_button("Estado",|ui| {
                    if let Some(e)=&self.contactos_error {ui.colored_label(crate::tema::ERROR,e);}
                    if let Some((error,texto))=&self.mensaje {
                        if *error {ui.colored_label(crate::tema::ERROR,texto);} else {ui.label(texto);}
                    }
                    ui.label("No escribe en RUS ni SATURNO. No envía correos.");
                    ui.label(format!("CC fijo: {CC_FIJO}"));
                });
                let texto=self.mensaje.as_ref().map(|(_,s)|s.as_str()).unwrap_or("Listo");
                let color=if self.contactos_error.is_some()||self.mensaje.as_ref().is_some_and(|(e,_)|*e){crate::tema::ERROR}else{ui.visuals().text_color()};
                ui.add(egui::Label::new(RichText::new(texto).color(color)).truncate()).on_hover_text(texto);
            });
            return;
        }
        if self.contactos_error.is_some() {
            ui.colored_label(
                crate::tema::ERROR,
                "Contactos no disponibles: revise la pestaña Contactos. El archivo anterior se conserva.",
            );
        }
        ui.horizontal_wrapped(|ui| {
            match &self.mensaje {
                Some((true, texto)) => ui.colored_label(crate::tema::ERROR, format!("⚠ {texto}")),
                Some((false, texto)) => ui.label(texto),
                None => ui.label("Abre un Excel de RUS para comenzar."),
            };
        });
        ui.horizontal_wrapped(|ui| {
            ui.weak("No escribe en RUS ni SATURNO · No envía correos · CC fijo:");
            ui.label(CC_FIJO);
        });
    }

    fn vista_trabajo(&mut self, ui: &mut egui::Ui) {
        let compacto=ui.available_width()<1000.0 || ui.available_height()<480.0;
        let mut guardar_excel=false;
        ui.horizontal_wrapped(|ui|{
            if compacto {
                for (p,n) in [(0,"Lista"),(1,"Evidencia"),(2,"Observación")] {ui.selectable_value(&mut self.panel_trabajo,p,n);}
            }
            if ui.button("Restaurar distribución").clicked(){self.distribucion=self.distribucion.wrapping_add(1);self.panel_trabajo=0;}
            if ui.add_enabled(self.trabajo.is_some(),egui::Button::new("Guardar Excel (Ctrl+S)")).clicked(){guardar_excel=true;}
        });
        let panel=self.panel_trabajo;

        let Some(trabajo) = &mut self.trabajo else {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.heading("Abre el Excel de RUS o el paquete ZIP del Descargador");
                ui.label("Flow lee Espera, Cumplimiento o Informes, propone la observación de cada registro\ny prepara el Excel, los borradores de correo y los proyectos de resolución.");
            });
            return;
        };
        let mut cambiar = None;
        crate::paneles_ui::configuracion(ui,compacto,"Filtros y etapas de Trabajo",|ui| {
            egui::ComboBox::from_id_salt("tablas_trabajo")
                .selected_text(format!("{} ({})",trabajo.nombre(),trabajo.filas.len())).show_ui(ui,|ui|{
                    ui.label(format!("Actual: {} ({})",trabajo.nombre(),trabajo.filas.len()));
                    for (i,otro) in self.otros.iter().enumerate() {
                        if ui.selectable_label(false,format!("{} ({})",otro.nombre(),otro.filas.len())).clicked() {
                            cambiar=Some(i);ui.close_menu();
                        }
                    }
                });
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
            if ui.button("Limpiar filtros").clicked(){self.filtro=Filtro::Todos;self.buscar.clear();}
        });
        if !trabajo.avisos.is_empty(){
            crate::paneles_ui::configuracion(ui,compacto,&format!("Avisos de fuente ({})",trabajo.avisos.len()),|ui|{
                for aviso in &trabajo.avisos{ui.colored_label(crate::tema::AVISO,format!("⚠ {aviso}"));}
            });
        }
        if compacto && (self.filtro!=Filtro::Todos || !self.buscar.is_empty()){ui.small("Filtros de Trabajo activos; abre Filtros y etapas para verlos o limpiarlos.");}
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
        let mut editor=|ui:&mut egui::Ui| {
            let Some(fila) = seleccion.and_then(|i| trabajo.filas.get_mut(i)) else {
                ui.label("Elige un registro para revisar su observación.");
                return;
            };
            egui::ScrollArea::vertical().id_salt("editor_observacion").show(ui, |ui| {
                ui.label(RichText::new("Observación").strong());
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut fila.revisada, "Revisada").changed() {
                        cambio = true;
                    }
                    if fila.editada() && ui.button("Restaurar la propuesta").clicked() {
                        fila.observacion = fila.propuesta.observacion.clone();
                        cambio = true;
                    }
                    if ui.button("Copiar observación").clicked() {
                        ui.ctx().copy_text(fila.observacion.clone());
                    }
                });
                if ui.add(egui::TextEdit::multiline(&mut fila.observacion).desired_width(f32::INFINITY).desired_rows(if compacto{3}else{7})).changed() {
                    cambio = true;
                }
                if fila.editada() {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Propuesta original").strong());
                    ui.label(if fila.propuesta.observacion.is_empty() { "(sin observación)" } else { &fila.propuesta.observacion });
                }
            });
        };
        if compacto {if panel==2{editor(ui);}}else{
            egui::TopBottomPanel::bottom(egui::Id::new(("editor_trabajo",self.distribucion))).resizable(true).default_height(210.0).min_height(110.0).show_inside(ui,&mut editor);
        }
        let mut detalle=|ui:&mut egui::Ui| {
            let Some(fila) = seleccion.and_then(|i| trabajo.filas.get_mut(i)) else {
                ui.label("Elige un registro de la lista.");
                return;
            };
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(format!("{} · {}", fila.registro.texto("rit"), excel::valor(fila, "nombre")));
                ui.label(format!(
                    "{} · {} · fila {} del Excel",
                    fila.registro.texto("programa"),
                    fila.registro.texto("tribunal"),
                    fila.numero
                ));
                ui.add_space(6.0);
                if !fila.propuesta.marcas.is_empty() || !fila.propuesta.avisos.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Revisar").strong());
                    for m in &fila.propuesta.marcas {
                        let color = match m.nivel {
                            1 => crate::tema::ERROR,
                            2 => crate::tema::AVISO,
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
        };
        if compacto {if panel==1{detalle(ui);}}else{
            egui::SidePanel::right(egui::Id::new(("detalle",self.distribucion))).resizable(true).default_width(410.0).min_width(220.0).show_inside(ui,&mut detalle);
        }
        let marcadas = &mut self.marcadas;
        let mut cambio_marcas = false;
        let marcadas_cambio = &mut cambio_marcas;
        if !compacto || panel==0 {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let alto = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
            TableBuilder::new(ui)
                .striped(true)
                .sense(egui::Sense::click())
                .column(Column::exact(24.0))
                .column(Column::exact(44.0))
                .column(Column::exact(96.0))
                .column(Column::initial(140.0).clip(true).resizable(true))
                .column(Column::initial(120.0).clip(true).resizable(true))
                .column(Column::exact(54.0))
                .column(Column::remainder().clip(true))
                .header(alto, |mut h| {
                    for titulo in ["☐", "Fila", "RIT", "Nombre", "Programa", "Estado", "Observación"] {
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
                        row.col(|ui| {
                            let mut marcada = marcadas.contains(&f.id);
                            if ui.checkbox(&mut marcada, "").on_hover_text("Fila elegida para «Solo filas seleccionadas»").changed() {
                                if marcada {
                                    marcadas.insert(f.id.clone());
                                } else {
                                    marcadas.remove(&f.id);
                                }
                                *marcadas_cambio = true;
                            }
                        });
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
        }
        self.seleccion = seleccion;
        if cambio || cambio_marcas {
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
        let compacto=ui.available_width()<1000.0||ui.available_height()<480.0;
        let mut preparar = false;
        let mut preparar_tipo = None;
        let mut guardar = None;
        crate::paneles_ui::configuracion(ui,compacto,"Preparación y alcance de Correos",|ui|{
        ui.label(RichText::new("Modalidades comprendidas en la revisión (para los correos al tribunal)").strong());
        ui.horizontal_wrapped(|ui| {
            for (nombre, marcada) in &mut self.modalidades {
                ui.checkbox(marcada, nombre.as_str());
            }
            ui.add(egui::TextEdit::singleline(&mut self.otra_modalidad).hint_text("Otra modalidad (texto libre)").desired_width(210.0)).on_hover_text("Cambia el alcance de modalidades al preparar; no filtra la tabla.");
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Preparar un tipo:");
            egui::ComboBox::from_id_salt("tipo_correo").selected_text(self.tipo_elegido.titulo(etapa)).show_ui(ui, |ui| {
                for tipo in Tipo::TODOS {
                    ui.selectable_value(&mut self.tipo_elegido, tipo, tipo.titulo(etapa));
                }
            });
            if ui.button("Preparar este tipo").on_hover_text("Reemplaza la lista de borradores preparados").clicked() {
                preparar_tipo = Some(self.tipo_elegido);
            }
            if self.tipo_elegido==Tipo::Proyectos {
                ui.collapsing("Opciones del reporte",|ui|{
                    ui.label("Período");
                    ui.add(egui::TextEdit::singleline(&mut self.periodo).desired_width(90.0).hint_text("mm/aaaa"))
                        .on_hover_text("Se usa en el reporte de proyectos de resolución.");
                });
            }
        });
        let marcadas = self.marcadas.len();
        self.sin_guardar |= ui
            .checkbox(&mut self.manual, format!("Solo filas seleccionadas en Trabajo ({marcadas})"))
            .on_hover_text("Marca las filas en la primera columna de Trabajo")
            .changed();
        });
        if compacto {
            ui.small(format!("Alcance: {} · {}",if self.otra_modalidad.trim().is_empty(){format!("{} modalidades",self.modalidades.iter().filter(|(_,v)|*v).count())}else{"modalidad en texto libre".into()},
                if self.manual{format!("{} filas seleccionadas",self.marcadas.len())}else{"trabajo completo".into()}));
        }
        let obsoletos = self.borradores_obsoletos();
        ui.horizontal_wrapped(|ui| {
            if ui.button("Preparar borradores").on_hover_text("Todos los que surgen del trabajo, como «Preparar todos»").clicked() {
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
        let incluidos=self.borradores.iter().filter(|b|!b.omitido).count();
        ui.label(format!("{} incluidos · {} excluidos",incluidos,self.borradores.len()-incluidos));
        let titulos=self.borradores.iter().enumerate().map(|(i,b)|format!("{}{} · {} · {}",
            if obsoletos.contains(&i){"⚠ Obsoleto · "}else{""},if b.omitido{"Excluido"}else{"Incluido"},
            b.tipo.titulo(etapa),if b.programa.is_empty(){b.tribunal.as_str()}else{b.programa.as_str()})).collect::<Vec<_>>();
        let activo=crate::paneles_ui::selector(ui,"lista_correos",&titulos,&mut self.correo_activo);
        let mut aviso_recibo: Option<Result<String, String>> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, b) in self.borradores.iter_mut().enumerate() {
                if activo!=Some(i){continue}
                let quien = if b.programa.is_empty() { b.tribunal.clone() } else { format!("{} · {}", b.programa, b.tribunal) };
                let faltan = b.faltantes();
                let titulo = format!(
                    "{}{}{} — {} ({}){}",
                    if obsoletos.contains(&i) { "(obsoleto: prepara de nuevo) " } else { "" },
                    if b.omitido { "(omitido) " } else { "" },
                    b.tipo.titulo(etapa),
                    quien,
                    if b.filas.len() == 1 { "1 registro".to_string() } else { format!("{} registros", b.filas.len()) },
                    if faltan.is_empty() { String::new() } else { format!(" — falta {}", faltan.join(", ")) }
                );
                egui::CollapsingHeader::new(titulo).id_salt(("borrador", i)).default_open(true).show(ui, |ui| {
                    if let Some(estado) = self.estados_borradores.get(&i).copied() {
                        match estado {
                            crate::borradores::Estado::Guardado => {
                                ui.label("Guardado en Outlook; un nuevo clic no lo duplica.");
                            }
                            crate::borradores::Estado::NoGuardado => {
                                ui.label("No guardado; puede reintentar o crear una copia .eml.");
                            }
                            _ => {
                                ui.colored_label(
                                    crate::tema::ERROR,
                                    "Guardado incierto: compruebe personalmente la carpeta Borradores de Outlook.",
                                );
                                for (existe, etiqueta) in
                                    [(true, "Comprobé que está en Outlook"), (false, "Comprobé que no está; permitir reintento")]
                                {
                                    if ui.small_button(etiqueta).clicked() {
                                        match crate::borradores::resolver(&self.datos, b, existe) {
                                            Ok(()) => {
                                                self.estados_borradores.insert(
                                                    i,
                                                    if existe {
                                                        crate::borradores::Estado::Guardado
                                                    } else {
                                                        crate::borradores::Estado::NoGuardado
                                                    },
                                                );
                                            }
                                            Err(e) => aviso_recibo = Some(Err(e)),
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let mut editado = false;
                    // Las ediciones y omisiones quedan en la sesión (`session.py::capture`).
                    self.sin_guardar |= ui.checkbox(&mut b.omitido, "Excluir este borrador").changed();
                    egui::Grid::new(("campos", i)).num_columns(2).show(ui, |ui| {
                        ui.label("Para");
                        editado |= ui
                            .add(egui::TextEdit::singleline(&mut b.para).desired_width(520.0).hint_text("correo del destinatario"))
                            .changed();
                        ui.end_row();
                        ui.label("CC");
                        ui.label(&b.cc).on_hover_text("El CC fijo va en todos los correos y no se puede quitar.");
                        ui.end_row();
                        ui.label("Asunto");
                        editado |= ui.add(egui::TextEdit::singleline(&mut b.asunto).desired_width(520.0)).changed();
                        ui.end_row();
                    });
                    editado |= ui.add(egui::TextEdit::multiline(&mut b.cuerpo).desired_width(f32::INFINITY).desired_rows(6)).changed();
                    if editado {
                        self.estados_borradores.remove(&i);
                        self.sin_guardar = true;
                    }
                    if ui.small_button("Consultar recibo local de Outlook").clicked() {
                        match crate::borradores::estado(&self.datos, b) {
                            Ok(Some(e)) => {
                                self.estados_borradores.insert(i, e);
                            }
                            Ok(None) => {
                                self.estados_borradores.remove(&i);
                                aviso_recibo = Some(Ok("Sin guardado previo de este borrador.".into()));
                            }
                            Err(e) => aviso_recibo = Some(Err(e)),
                        }
                    }
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
        if let Some(aviso) = aviso_recibo {
            match aviso {
                Ok(s) => self.bien(s),
                Err(e) => self.mal(e),
            }
        }
        if preparar {
            self.preparar_borradores();
            self.sin_guardar = true;
        }
        if let Some(tipo) = preparar_tipo {
            self.preparar_tipo(tipo);
            self.sin_guardar = true;
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
            ui.menu_button("Plantillas", |ui| {
                if ui.button("Carpeta de plantillas propias").on_hover_text("Ej.: TOME_PC_IE.docx con variables {{RIT}}, {{NOMBRE}}…").clicked() {
                    let carpeta = self.plantillas_usuario();
                    let _ = std::fs::create_dir_all(&carpeta);
                    abrir_en_sistema(&carpeta);
                }
            });
        });
        ui.small(
            "Un proyecto por tribunal, RIT y tipo, con todas las personas de la causa; un solo Word con todos, cada \
             proyecto en página nueva. Lo que falte queda como [COMPLETAR …] para completarlo en el Word.",
        );
        ui.separator();
        let propias = self.plantillas_usuario();
        let obsoletos = self.proyectos_obsoletos();
        let titulos=self.proyectos.iter().enumerate().map(|(i,p)|format!("{}{} · {} · {}",
            if obsoletos.contains(&i){"⚠ Obsoleto · "}else{""},p.tipo,p.rit,p.tribunal.titulo())).collect::<Vec<_>>();
        let activo=crate::paneles_ui::selector(ui,"lista_resoluciones",&titulos,&mut self.proyecto_activo);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, p) in self.proyectos.iter_mut().enumerate() {
                if activo!=Some(i){continue}
                let estado = if obsoletos.contains(&i) {
                    "obsoleto: cambiaron sus registros o plantilla; se prepara de nuevo al generar".to_string()
                } else if p.omitido {
                    "omitido".to_string()
                } else if p.sin_plantilla {
                    format!("sin plantilla para {}", p.tribunal.titulo())
                } else if !p.vacias().is_empty() {
                    format!("vacío: {} (escribe un valor)", p.vacias().join(", "))
                } else if p.faltantes.is_empty() {
                    "listo".to_string()
                } else {
                    format!("listo · por completar en el Word: {}", p.faltantes.join(", "))
                };
                let registros = if p.filas.len() == 1 { "1 registro".to_string() } else { format!("{} registros", p.filas.len()) };
                egui::CollapsingHeader::new(format!("{} · {} · {} · {registros} — {}", p.tipo, p.rit, p.tribunal.titulo(), estado))
                    .id_salt(("proyecto", i)).default_open(true)
                    .show(ui, |ui| {
                        let mut cambio = ui.checkbox(&mut p.omitido, "No generar este proyecto").changed();
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
                            self.sin_guardar = true;
                        }
                    });
            }
        });
        if preparar {
            self.preparar_proyectos();
            self.sin_guardar = true;
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
            if ui.button("Destino de próximos productos…").clicked() {
                if let Some(nueva) = rfd::FileDialog::new().pick_folder() {
                    self.ajustes.salida = Some(nueva);
                    self.biblioteca.recordar(self.carpeta.as_deref());
        self.carpeta = None;
                    if let Ok(texto) = serde_json::to_string_pretty(&self.ajustes) {
                        let _ = std::fs::write(ruta_ajustes(&self.datos), texto);
                    }
                }
            }
        });
        if let Some(destino)=&self.ajustes.salida {ui.small(format!("Destino de próximos productos: {}",destino.display()));}
        ui.separator();
        ui.collapsing("Registro de operaciones y recibos",|ui|{
            egui::ScrollArea::vertical().id_salt("registro_productos").max_height(150.0).show(ui,|ui|{
                for linea in &self.resultados {ui.label(linea);}
            });
        });
        self.biblioteca.mostrar(ui,self.carpeta.as_deref());
    }

    fn vista_contactos(&mut self, ui: &mut egui::Ui) {
        if let Some(error) = &self.contactos_error {
            ui.colored_label(crate::tema::ERROR, error);
            ui.label("El archivo de contactos permanece intacto. Corríjalo o recupere una copia y pulse Releer.");
            if ui.button("Releer contactos").clicked() {
                match Contactos::cargar(&ruta_contactos(&self.datos)) {
                    Ok(c) => {
                        self.contactos = c;
                        self.contactos_error = None;
                        self.bien("Contactos recuperados.");
                    }
                    Err(e) => self.contactos_error = Some(e),
                }
            }
            return;
        }
        let mut guardar = false;
        ui.label("Estas direcciones se guardan solo en este equipo.");
        if let Some(ruta) = self.contactos_csmp.clone() {
            ui.add_space(4.0);
            ui.colored_label(
                crate::tema::AVISO,
                format!("Contactos tomados de CSMP ({}), solo lectura: no se guardan en Flow hasta que los confirmes.", ruta.display()),
            );
            if ui.button("Guardar en Flow").on_hover_text("Copia estos contactos a la carpeta de Flow; CSMP no se modifica.").clicked() {
                self.contactos_csmp = None;
                guardar = true;
                self.bien("Contactos de CSMP guardados en Flow.");
            }
        }
        let editable = self.contactos_csmp.is_none();
        ui.add_space(4.0);
        ui.add_enabled_ui(editable, |ui| {
            ui.menu_button("Importar", |ui| {
                if ui
                    .button("CSMP (JSON)…")
                    .on_hover_text("contactos.json del Asistente anterior (lista Nombre/Mail)")
                    .clicked()
                {
                    self.importar_contactos_json();
                }
                if ui.button("Importar catastro Excel…").on_hover_text("Columnas Nombre/Programa y Mail/Correo").clicked() {
                    self.importar_catastro();
                }
            });
        });
        if let Some((origen, imp)) = self.importacion.clone() {
            ui.group(|ui| {
                ui.label(RichText::new(format!("Importación pendiente de confirmar: {origen}")).strong());
                ui.label(format!("{} contacto(s) nuevos o completados.", imp.nuevos.len()));
                if !imp.tribunales.is_empty() {
                    ui.label(format!("Destinatarios de tribunal que cambian: {}.", imp.tribunales.join(", ")));
                }
                if !imp.conflictos.is_empty() {
                    ui.colored_label(
                        crate::tema::ERROR,
                        format!("No se sustituyen (ya tienen otra dirección): {}.", imp.conflictos.join("; ")),
                    );
                }
                ui.horizontal(|ui| {
                    if ui.button("Confirmar importación").clicked() {
                        self.confirmar_importacion();
                    }
                    if ui.button("Cancelar").clicked() {
                        self.importacion = None;
                    }
                });
            });
        }
        ui.add_space(4.0);
        ui.add_enabled_ui(editable, |ui| {
            ui.collapsing("Tribunales, CC y firma",|ui| {
            ui.label(RichText::new("Tribunales").strong());
            egui::Grid::new("tribunales").num_columns(2).show(ui, |ui| {
                for clave in correos::TRIBUNALES {
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
            });
            let mut quitar = None;
            let mut editar = None;
            let busqueda=flow_dominio::texto::normalizar(&self.buscar_contacto);
            let compacto=ui.available_width()<900.0;
            if compacto {ui.horizontal_wrapped(|ui|{
                ui.selectable_value(&mut self.ficha_contacto,false,"Lista de programas");
                ui.selectable_value(&mut self.ficha_contacto,true,"Ficha del programa");
            });}
            ui.columns(if compacto{1}else{2},|columnas|{
                if !compacto || !self.ficha_contacto {
                let ui=&mut columnas[0];
                ui.heading("Programas");
                ui.add(egui::TextEdit::singleline(&mut self.buscar_contacto).hint_text("Buscar programa o correo…").desired_width(f32::INFINITY));
                ui.small(format!("{} contactos",self.contactos.programas.len()));
            egui::ScrollArea::vertical().id_salt("lista_contactos").max_height(350.0).show(ui, |ui| {
                egui::Grid::new("programas").num_columns(4).striped(true).show(ui, |ui| {
                    for (nombre, correo) in &self.contactos.programas {
                        if !flow_dominio::texto::normalizar(&format!("{nombre} {correo}")).contains(&busqueda) {continue;}
                        ui.label(nombre.as_str());
                        ui.label(if correo.is_empty() { "(sin correo)" } else { correo.as_str() });
                        if ui.small_button("Editar").clicked() {
                            editar = Some(nombre.clone());
                        }
                        if ui.small_button("Quitar").clicked() {
                            quitar = Some(nombre.clone());
                        }
                        ui.end_row();
                    }
                });
            });

                }
                if !compacto || self.ficha_contacto {
                let ui=&mut columnas[if compacto{0}else{1}];
                ui.heading("Ficha del programa");
                ui.label(self.edicion.as_deref().unwrap_or("Contacto nuevo"));
                ui.label("Programa");
                ui.add(egui::TextEdit::singleline(&mut self.nuevo.0).desired_width(f32::INFINITY));
                ui.label("Correos separados por punto y coma");
                ui.add(egui::TextEdit::singleline(&mut self.nuevo.1).desired_width(f32::INFINITY));
                ui.label("Alias separados por punto y coma");
                ui.add(egui::TextEdit::singleline(&mut self.nuevo.2).desired_width(f32::INFINITY));
                ui.horizontal_wrapped(|ui| {
                if ui.button("Guardar contacto").clicked() {
                    let alias: Vec<String> = self.nuevo.2.split(';').map(str::to_string).collect();
                    match self.contactos.guardar_programa(self.edicion.as_deref(), &self.nuevo.0, &self.nuevo.1, &alias) {
                        Ok(()) => {
                            self.edicion = Some(self.nuevo.0.trim().to_string());
                            guardar = true;
                            self.bien("Contacto y alias guardados.");
                        }
                        Err(e) => self.mal(e),
                    }
                }
                if ui.button("Nuevo").clicked() {
                    self.nuevo = Default::default();
                    self.edicion = None;
                }
            });

                }
            });
            if let Some(nombre) = editar {
                let correo = self.contactos.programas.get(&nombre).cloned().unwrap_or_default();
                self.nuevo = (nombre.clone(), correo, self.contactos.alias_de(&nombre).join("; "));
                self.edicion = Some(nombre);
                self.ficha_contacto=true;
            }
            if let Some(nombre) = quitar {
                self.contactos.quitar_programa(&nombre);
                if self.edicion.as_deref() == Some(nombre.as_str()) {
                    self.edicion = None;
                }
                guardar = true;
                self.bien("Contacto eliminado.");
            }
        });
        if guardar && self.contactos_csmp.is_none() {
            if let Err(e) = self.contactos.guardar(&ruta_contactos(&self.datos)) {
                self.mal(e);
            }
        }
    }

    /// Guarda en Flow la importación revisada (`app_base._import_contacts`, tras confirmar).
    fn confirmar_importacion(&mut self) {
        let Some((_, imp)) = self.importacion.take() else { return };
        self.contactos = imp.contactos;
        match self.contactos.guardar(&ruta_contactos(&self.datos)) {
            Ok(()) => self.bien(format!(
                "✓ Catastro incorporado · {} contactos disponibles · copia local en contactos.json.",
                self.contactos.programas.len()
            )),
            Err(e) => self.mal(e),
        }
    }

    /// `app_base._import_contacts` con un JSON de CSMP: se revisa y se confirma antes de guardar.
    fn importar_contactos_json(&mut self) {
        let Some(ruta) =
            rfd::FileDialog::new().add_filter("Contactos de CSMP", &["json"]).set_title("Importar contactos de CSMP").pick_file()
        else {
            return;
        };
        self.revisar_contactos_json(&ruta);
    }

    fn revisar_contactos_json(&mut self, ruta: &Path) {
        let lista = std::fs::metadata(ruta)
            .ok()
            .filter(|m| m.len() <= 2 * 1024 * 1024)
            .ok_or("El archivo de contactos no se pudo leer o supera 2 MiB.".to_string())
            .and_then(|_| {
                std::fs::read_to_string(ruta).map_err(|_| "El archivo de contactos no se pudo leer como texto UTF-8.".to_string())
            })
            .and_then(|t| correos::leer_lista_csmp(&t));
        match lista {
            Ok(lista) => self.importacion = Some((nombre_archivo(ruta), self.contactos.importar_csmp(&lista))),
            Err(e) => self.mal(e),
        }
    }

    /// `outputs.import_contacts`: catastro Excel (primera hoja, primera fila de títulos).
    fn importar_catastro(&mut self) {
        let Some(ruta) = rfd::FileDialog::new()
            .add_filter("Catastro Excel", &["xlsx", "xlsm", "xls"])
            .set_title("Importar catastro de contactos")
            .pick_file()
        else {
            return;
        };
        self.revisar_catastro(&ruta);
    }

    fn revisar_catastro(&mut self, ruta: &Path) {
        let resultado = flow_tabular::leer(ruta).map_err(|e| format!("No se pudo leer el catastro: {e}")).and_then(|libro| {
            let filas: Vec<Vec<String>> =
                libro.hojas.first().map(|h| h.filas.iter().map(|f| f.iter().map(|c| c.texto()).collect()).collect()).unwrap_or_default();
            let (cabeceras, resto) = filas.split_first().ok_or("El catastro no tiene filas.".to_string())?;
            self.contactos.importar_catastro(cabeceras, resto)
        });
        match resultado {
            Ok(imp) => self.importacion = Some((nombre_archivo(ruta), imp)),
            Err(e) => self.mal(e),
        }
    }
}

fn nombre_archivo(ruta: &Path) -> String {
    ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

const TEXTO_AYUDA_DESCARGADOR: &str = "Descarga consultas de SITFA desde tu propia sesión de Chrome, con la extensión \
Flow. Abre la conexión local, copia el código y pégalo en la extensión desde la pestaña SITFA. Elige pantalla, \
opciones y carpeta. Cada página se verifica antes de guardarse; el resumen del lote queda en la carpeta.\n\n\
Flow no escribe en SITFA, RUS ni SATURNO. Para analizar los Excel descargados, ábrelos en Flow Asistente.";

const TEXTO_AYUDA_ANONIMIZADOR: &str = "Elige los archivos o la carpeta, revisa la vista previa y genera las copias \
anonimizadas en otra carpeta. Los originales no se modifican. Revisa las copias antes de compartirlas.";

const TEXTO_AYUDA_META4: &str = "Agrega las carpetas de lote que dejó Flow Descargador (las que contienen resumen.json), \
indica la fecha de corte y genera la planilla Meta 4. Los lotes no se modifican. Revisa la planilla antes de usarla.";

const TEXTO_AYUDA: &str = "1 · Trabajo: abre el Excel de RUS (Ctrl+O). Revisa cada observación; puedes editarla y marcarla como \
revisada. Ctrl+S guarda el Excel de observaciones.\n\n2 · Correos: marca las modalidades, prepara los borradores, \
revisa destinatario y texto, y guárdalos. Quedan en Borradores de Outlook o como archivos .eml. Flow nunca envía. \
El reporte de proyectos y la gestión particular se preparan con «Preparar este tipo».\n\n\
Para una gestión de algunas filas, márcalas en Trabajo y activa «Solo filas seleccionadas». Un borrador o proyecto \
cuyos registros cambiaron queda obsoleto: prepáralo de nuevo.\n\n\
Revisión de bitácoras: abre el paquete, confirma Desde/Hasta/Corte y analiza. No verificable significa que falta \
evidencia suficiente de autoría o cobertura. El motor usa el contexto completo y propone solo para Sin observación. \
Revisa evidencia, propuesta y edición; marca Revisada y elige Seleccionados o Todos al exportar Excel + JSON. \
En tamaño compacto, Configurar reúne criterios y acciones.\n\n\
3 · Resoluciones: prepara los proyectos (uno por tribunal, RIT y tipo), revisa sus valores y genera el Word (uno \
solo con todos); lo que falte queda marcado [COMPLETAR …] para completarlo en el documento.\n\n4 · Resultados: lista de lo \
creado y acceso a la carpeta.\n\nFlow no escribe en RUS ni en SATURNO. La observación propuesta es una ayuda: \
quien la registra y responde por ella es el usuario.";

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(evidencia)=&mut self.evidencia {evidencia.actualizar(ctx);}
        let (abrir, guardar, ayuda) = ctx.input(|i| {
            (i.modifiers.command && i.key_pressed(Key::O), i.modifiers.command && i.key_pressed(Key::S), i.key_pressed(Key::F1))
        });
        if abrir && self.herramienta.analiza_rus() {
            self.elegir_archivo();
        }
        self.atajo_guardar(guardar);
        if ayuda {
            self.ayuda = true;
        }
        egui::TopBottomPanel::top("barra").show(ctx, |ui| self.barra(ui));
        egui::TopBottomPanel::bottom("pie").show(ctx, |ui| self.pie(ui));
        egui::CentralPanel::default().show(ctx, |ui| match self.pestana {
            Pestana::Captura => {
                if let Some((carpeta, resumen)) = egui::ScrollArea::vertical().id_salt("pagina_captura").show(ui,|ui|self.captura.mostrar(ui,ctx)).inner {
                    if self.herramienta == Herramienta::Descargador {
                        // El Descargador solo descarga y verifica; el análisis se hace en el Asistente.
                        abrir_en_sistema(&carpeta);
                        self.bien(format!(
                            "Lote verificado en {} ({} archivo(s)). Para analizarlo, abre esos Excel en Flow Asistente.",
                            carpeta.display(),
                            resumen.archivos.len()
                        ));
                        self.captura.listo_de_nuevo(carpeta, resumen);
                    } else {
                        self.importar_captura(carpeta, resumen)
                    }
                }
            }
            Pestana::Trabajo => self.vista_trabajo(ui),
            Pestana::Correos => self.vista_correos(ui),
            Pestana::Resoluciones => self.vista_resoluciones(ui),
            Pestana::Resultados => self.vista_resultados(ui),
            Pestana::Contactos => {egui::ScrollArea::vertical().id_salt("pagina_contactos").show(ui,|ui|self.vista_contactos(ui));}
            Pestana::Anonimo => {egui::ScrollArea::vertical().id_salt("pagina_anonimo").show(ui,|ui|self.anonimo.mostrar(ui,ctx));}
            Pestana::Meta4 => {egui::ScrollArea::vertical().id_salt("pagina_meta4").show(ui,|ui|self.meta4.mostrar(ui));}
            Pestana::Revision => self.revision.mostrar(ui, ctx, &self.motor),
        });
        if self.confirmar_recalculo {
            egui::Window::new("Recalcular propuestas").collapsible(false).resizable(false).show(ctx, |ui| {
                ui.label("Se descartarán las ediciones y marcas de revisión recuperadas para este archivo y corte.");
                ui.label("Los borradores y proyectos deberán prepararse nuevamente. Esta acción no se puede deshacer desde la sesión.");
                ui.horizontal(|ui| {
                    if ui.button("Cancelar").clicked() { self.confirmar_recalculo = false; }
                    if ui.button("Descartar ediciones y recalcular").clicked() {
                        self.confirmar_recalculo = false;
                        if let Some(ruta) = self.archivo.clone() { self.cargar_modo(ruta, true); }
                    }
                });
            });
        }
        if self.ayuda {
            let mut abierta = true;
            egui::Window::new("Cómo se usa Flow").open(&mut abierta).collapsible(false).default_width(520.0).show(ctx, |ui| {
                if self.herramienta.analiza_rus() {
                    ui.label("Revisión de bitácoras: abre un paquete, confirma las fechas y analiza. Solo la ausencia acreditada habilita propuestas del motor. Revisa su evidencia y exporta todos los clasificados o una selección explícita a Excel y JSON.");
                    ui.separator();
                }
                ui.label(match self.herramienta {
                    Herramienta::Descargador => TEXTO_AYUDA_DESCARGADOR,
                    Herramienta::Anonimizador => TEXTO_AYUDA_ANONIMIZADOR,
                    Herramienta::Meta4 => TEXTO_AYUDA_META4,
                    Herramienta::Todas | Herramienta::Asistente => TEXTO_AYUDA,
                });
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
        if self.contactos_error.is_none() && self.contactos_csmp.is_none() {
            if let Err(e) = self.contactos.guardar(&ruta_contactos(&self.datos)) {
                eprintln!("{e}");
            }
        }
    }
}

pub fn abrir_demostracion(config:crate::evidencia_ui::Config)->Result<(),String>{
    let evidencia=crate::evidencia_ui::Captura::nueva(config)?;
    let opciones=eframe::NativeOptions {
        viewport:egui::ViewportBuilder::default().with_inner_size([evidencia.config.ancho,evidencia.config.alto])
            .with_min_inner_size([320.0,240.0]).with_clamp_size_to_monitor_size(false).with_title("Flow · DEMOSTRACIÓN SINTÉTICA"),
        ..Default::default()
    };
    eframe::run_native("Flow · DEMOSTRACIÓN SINTÉTICA",opciones,Box::new(move|cc|{
        crate::tema::aplicar(&cc.egui_ctx);cc.egui_ctx.set_zoom_factor(evidencia.config.escala);
        let raiz=evidencia.temporal.path();
        let mut app=App::con_origen(raiz.join("datos"),None,&[]);
        app.outlook=false;
        let salida=raiz.join("productos");std::fs::create_dir(&salida)?;
        app.ajustes.salida=Some(salida);
        let origen=raiz.join("Origen_FICTICIO.xlsx");
        std::fs::write(&origen,include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/evidencia_p0/golden/Origen_ESPERA.xlsx")))?;
        app.corte="08/10/2026".into();app.cargar(origen);app.seleccion=Some(0);
        app.contactos.programas.insert("Programa Ficticio".into(),"programa@example.invalid".into());
        for t in correos::TRIBUNALES {app.contactos.tribunales.insert(t.into(),"tribunal@example.invalid".into());}
        for (_,activa) in &mut app.modalidades {*activa=true;}
        app.preparar_borradores();app.preparar_proyectos();
        if evidencia.config.pantalla.starts_with("revision"){
            let panel=match evidencia.config.pantalla.as_str(){"revision-evidencia"=>1,"revision-propuesta"=>2,_=>0};
            app.revision.demostracion(crate::demo_paquete::crear()?,&app.motor,panel)?;
            if evidencia.config.pantalla=="revision-inicial"{app.revision.demostracion_inicial();}
        }
        if evidencia.config.pantalla=="sin-sesion" {app=App::con_origen(raiz.join("sin-sesion"),None,&[]);app.outlook=false;}
        if evidencia.config.pantalla=="correo-incierto" {app.estados_borradores.insert(0,crate::borradores::Estado::Incierto);app.correo_activo=Some(0);}
        if evidencia.config.pantalla=="proyecto-obsoleto" {if let Some(t)=&mut app.trabajo {for f in &mut t.filas {f.observacion="Edición ficticia posterior a preparar proyectos.".into();}}}
        app.mensaje=Some((false,"DEMOSTRACIÓN · Datos ficticios · Almacenamiento temporal aislado".into()));
        if evidencia.config.pantalla=="error" {app.mal("Fallo ficticio de lectura. El archivo anterior se conserva. Revisa el origen y vuelve a intentarlo; no se publicaron productos.");}
        app.pestana=match evidencia.config.pantalla.as_str(){
            "correos"|"correo-incierto"=>Pestana::Correos,"resoluciones"|"proyecto-obsoleto"=>Pestana::Resoluciones,"resultados"=>Pestana::Resultados,
            "contactos"=>Pestana::Contactos,"captura"=>Pestana::Captura,"anonimizador"=>Pestana::Anonimo,
            "meta4"=>Pestana::Meta4,"revision"|"revision-evidencia"|"revision-propuesta"|"revision-inicial"=>Pestana::Revision,_=>Pestana::Trabajo,
        };
        app.ayuda=evidencia.config.pantalla=="ayuda";
        app.panel_trabajo=match evidencia.config.pantalla.as_str(){"trabajo-evidencia"=>1,"trabajo-editor"=>2,_=>0};
        app.confirmar_recalculo=evidencia.config.pantalla=="recalculo";
        app.evidencia=Some(evidencia);
        Ok(Box::new(app))
    })).map_err(|e|e.to_string())
}

/// Abre la ventana. `archivo` permite arrastrar un Excel sobre el ejecutable.
pub fn abrir(archivo: Option<PathBuf>, herramienta: Herramienta) -> i32 {
    let opciones = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 720.0])
            .with_min_inner_size([320.0, 240.0])
            .with_title(herramienta.titulo()),
        ..Default::default()
    };
    match eframe::run_native(
        herramienta.titulo(),
        opciones,
        Box::new(|cc| {
            crate::tema::aplicar(&cc.egui_ctx);
            Ok(Box::new(App::nueva(archivo, herramienta)))
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
mod pruebas_herramientas {
    use super::*;

    #[test]
    fn nombre_del_ejecutable_elige_la_herramienta_y_sus_pestanas() {
        for (nombre, h, primera) in [
            ("flow", Herramienta::Todas, Pestana::Captura),
            ("Flow_Asistente", Herramienta::Asistente, Pestana::Trabajo),
            ("Flow_Descargador", Herramienta::Descargador, Pestana::Captura),
            ("Flow_Anonimizador", Herramienta::Anonimizador, Pestana::Anonimo),
            ("Flow_Meta4", Herramienta::Meta4, Pestana::Meta4),
        ] {
            assert_eq!(Herramienta::desde_ejecutable(nombre), h);
            assert!(h.pestanas()[0].0 == primera, "{nombre}");
        }
        assert_eq!(Herramienta::Todas.pestanas().len(), 9);
        assert_eq!(Herramienta::Asistente.pestanas().len(), 6);
        assert!(Herramienta::Asistente.pestanas().iter().any(|(p,_)| *p == Pestana::Revision));
        assert!(!Herramienta::Descargador.analiza_rus() && Herramienta::Asistente.analiza_rus());
    }
}

#[cfg(test)]
mod pruebas_captura {
    use super::*;

    static SECUENCIA: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn preparar() -> (PathBuf, flow_captura::lotes::Resumen) {
        let base = std::env::temp_dir().join(format!(
            "flow-ui-ficticia-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
            SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
            inicio: String::new(),
            tribunales: Default::default(),
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
    fn guardar_desde_revision_no_exporta_el_trabajo_anterior() {
        let (base, resumen) = preparar();
        let mut app = App::con_datos(base.join("sesiones-ficticias"), None);
        app.corte = "08/10/2026".into();
        app.importar_captura(base.clone(), resumen);
        assert!(app.trabajo.is_some());
        let salida = base.join("salida-atajo");
        app.ajustes.salida = Some(salida.clone());
        app.pestana = Pestana::Revision;
        app.revision.demostracion(crate::demo_paquete::crear().unwrap(), &Config::default(), 2).unwrap();
        app.atajo_guardar(true);
        assert!(!salida.exists());
        assert!(app.carpeta.is_none());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn captura_paquete_y_json_no_reintroducen_texto_fuera_del_limite_historico() {
        use flow_captura::{lotes::Conexion,recoleccion::{Recoleccion,PaginaActual,Control}};
        use std::sync::atomic::AtomicBool;
        struct Ficticia;
        impl Conexion for Ficticia {
            fn llamar(&self,comando:&str,payload:serde_json::Value,_plazo:std::time::Duration)->Result<serde_json::Value,String>{
                assert_eq!(comando,"diary_open");
                let mut r:serde_json::Value=serde_json::from_str(include_str!("../pruebas/respuesta_bitacora_antigua.json")).unwrap();
                r["params"]=payload["params"].clone();Ok(r)
            }
        }
        let (base,mut resumen)=preparar();
        resumen.lote.lecturas.bitacoras=true;resumen.lote.lecturas.meses=4;
        let paginas:Vec<serde_json::Value>=serde_json::from_str(include_str!("../../flow-captura/tests/datos/vinculos_ficticios.json")).unwrap();
        let mut app=App::con_datos(base.join("datos"),None);app.corte="20/10/2026".into();app.cargar(base.join("consulta-001-p001-de-001.xls"));
        let f=&app.trabajo.as_ref().unwrap().filas[0];
        let mut html=paginas[0]["html"].as_str().unwrap().to_string();
        for (original,campo) in [("X-1-2099","rit"),("11111111-0","rut"),("Persona Ficticia","nombre"),("PRM Centro Ficticio","programa")] {
            let texto=f.registro.texto(campo);assert!(!texto.is_empty(),"{campo}");
            html=html.replace(original,&texto.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;"));
        }
        let mut captura=Recoleccion::con_fecha(&base,&resumen,"2026-10-20".parse().unwrap()).unwrap();
        captura.pagina(&Ficticia,&serde_json::json!({"tribunales":{"901":"Juzgado Canario de Prueba"}}),&mut resumen,
            PaginaActual{consulta:0,numero:1,html:html.as_bytes()},
            Control{cancelar:&AtomicBool::new(false),progreso:&|_|{}}).unwrap();
        assert_eq!(resumen.lecturas.len(),1);
        let lectura=flow_captura::recoleccion::leer(&base,&resumen.lecturas[0]).unwrap();
        assert!(!lectura.to_string().contains("ANTIGUA_NO_PERSISTIR"));
        assert_eq!(lectura["datos"]["revision_bitacoras"]["anteriores_omitidas"],1);
        // Compatibilidad con V1 de la punta anterior: el archivo original se
        // conserva, pero una nueva exportación no reintroduce su texto antiguo.
        let mut anterior=lectura.clone();
        anterior["datos"]["revision_bitacoras"]["entries"]=serde_json::json!([{ "entry_id":"44","fecha":"2026-01-12T10:13:00","origen":"centro","texto":"ANTIGUA_NO_PERSISTIR" }]);
        anterior["datos"]["revision_bitacoras"].as_object_mut().unwrap().remove("anteriores_omitidas");
        let original=serde_json::to_vec(&anterior).unwrap();let ruta_original=base.join("lectura-v1-anterior.json");
        std::fs::write(&ruta_original,&original).unwrap();
        resumen.lecturas[0].archivo="lectura-v1-anterior.json".into();resumen.lecturas[0].sha256=flow_captura::lotes::huella(&original);
        resumen.guardar(&base).unwrap();
        struct SinNuevaLectura;
        impl Conexion for SinNuevaLectura {
            fn llamar(&self,_:&str,_:serde_json::Value,_:std::time::Duration)->Result<serde_json::Value,String>{panic!("La prueba exige reutilizar V1 sin leer otra vez")}
        }
        let mut reuso=Recoleccion::con_fecha(&base,&resumen,"2026-10-20".parse().unwrap()).unwrap();
        reuso.pagina(&SinNuevaLectura,&serde_json::json!({}),&mut resumen,PaginaActual{consulta:0,numero:1,html:html.as_bytes()},
            Control{cancelar:&AtomicBool::new(false),progreso:&|_|{}}).unwrap();
        assert_eq!(std::fs::read(&ruta_original).unwrap(),original);
        // Reuso desde una carpeta hermana: la copia nueva también se acota.
        let hermanas=tempfile::tempdir().unwrap();let origen=hermanas.path().join("origen");let destino=hermanas.path().join("destino");
        std::fs::create_dir(&origen).unwrap();std::fs::create_dir(&destino).unwrap();
        std::fs::write(origen.join("lectura-v1-anterior.json"),&original).unwrap();
        for a in &resumen.archivos {std::fs::copy(base.join(&a.nombre),origen.join(&a.nombre)).unwrap();}
        resumen.guardar(&origen).unwrap();
        let mut copia=resumen.clone();copia.lecturas.clear();
        let mut reuso=Recoleccion::con_fecha(&destino,&copia,"2026-10-20".parse().unwrap()).unwrap();
        reuso.pagina(&SinNuevaLectura,&serde_json::json!({}),&mut copia,PaginaActual{consulta:0,numero:1,html:html.as_bytes()},
            Control{cancelar:&AtomicBool::new(false),progreso:&|_|{}}).unwrap();
        assert_eq!(copia.lecturas.len(),1);assert!(copia.lecturas[0].reutilizada);
        let nueva=flow_captura::recoleccion::leer(&destino,&copia.lecturas[0]).unwrap();
        assert!(!nueva.to_string().contains("ANTIGUA_NO_PERSISTIR"));
        assert_eq!(std::fs::read(origen.join("lectura-v1-anterior.json")).unwrap(),original);
        let zip=base.join("paquete.zip");
        flow_trabajo::exportar::exportar(&[flow_trabajo::exportar::Lote{carpeta:&base,resumen:&resumen}],&zip,&Default::default()).unwrap();
        let datos=std::fs::read(&zip).unwrap();
        let mut archivo=zip::ZipArchive::new(std::io::Cursor::new(&datos)).unwrap();
        for i in 0..archivo.len(){let mut f=archivo.by_index(i).unwrap();let mut texto=String::new();std::io::Read::read_to_string(&mut f,&mut texto).unwrap();assert!(!texto.contains("ANTIGUA_NO_PERSISTIR"));}
        let desde="2026-06-20".parse().unwrap();let hasta="2026-10-20".parse().unwrap();
        let informe=flow_trabajo::paquete::revisar_bitacoras(&datos,desde,hasta,hasta).unwrap();
        assert!(informe.filas.iter().any(|f|!f.lecturas.is_empty()),"La prueba debe asociar la lectura al registro real del fixture");
        assert!(informe.filas.iter().all(|f|f.revision.clasificacion==flow_trabajo::revision_bitacoras::Clasificacion::NoVerificable));
        let salida=base.join("informes");std::fs::create_dir(&salida).unwrap();
        let producto=flow_productos::revision::exportar(&informe,None,&salida,&AtomicBool::new(false)).unwrap();
        assert!(!std::fs::read_to_string(producto.json).unwrap().contains("ANTIGUA_NO_PERSISTIR"));
        assert_eq!(std::fs::read(&ruta_original).unwrap(),original);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn correos_compactos_muestran_incertidumbre_antes_del_editor() {
        let (base, _) = preparar();
        let mut app = App::con_datos(base.join("datos"), None);
        app.corte="08/10/2026".into();app.cargar(base.join("consulta-001-p001-de-001.xls"));
        let salida=base.join("salida");std::fs::create_dir(&salida).unwrap();app.ajustes.salida=Some(salida);
        app.otra_modalidad="modalidad ficticia".into();app.preparar_borradores();
        assert!(!app.borradores.is_empty());
        app.estados_borradores.insert(0,crate::borradores::Estado::Incierto);app.correo_activo=Some(0);
        let ctx=egui::Context::default();crate::tema::aplicar(&ctx);
        let mut pintar=||ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(683.0,384.0))),..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|app.vista_correos(ui));
        });
        let _=pintar();let salida=pintar();
        for texto in ["Guardado incierto:","Comprobé que está en Outlook","Comprobé que no está; permitir reintento"] {
            let posicion=salida.shapes.iter().find_map(|s|match &s.shape{
                egui::epaint::Shape::Text(t) if t.galley.job.text.starts_with(texto)=>Some(t.pos),_=>None
            }).unwrap_or_else(||panic!("Control de recuperación no visible: {texto}"));
            assert!(posicion.y<384.0,"Control fuera del área visible: {texto}");
        }
        assert_eq!(app.estados_borradores.get(&0),Some(&crate::borradores::Estado::Incierto));
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn el_paquete_creado_desde_el_lote_se_abre_con_abrir_excel_o_paquete() {
        let (base, resumen) = preparar();
        let zip = base.join("paquete").join(flow_trabajo::exportar::NOMBRE);
        flow_trabajo::exportar::exportar(&[flow_trabajo::exportar::Lote { carpeta: &base, resumen: &resumen }], &zip, &Default::default())
            .unwrap();
        let mut app = App::con_datos(base.join("sesiones-ficticias"), None);
        app.corte = "08/10/2026".into();
        app.cargar(zip.clone());
        let trabajo = app.trabajo.as_ref().unwrap();
        assert_eq!(trabajo.etapa, Etapa::Espera);
        assert_eq!(trabajo.filas.len(), 3);
        assert_eq!(app.archivo.as_deref(), Some(zip.as_path()));
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
    #[test]
    fn abrir_el_mismo_excel_recupera_ediciones_y_marcas() {
        let (base, _) = preparar();
        let datos = base.join("datos");
        let archivo = base.join("consulta-001-p001-de-001.xls");
        let mut app = App::con_datos(datos.clone(), None);
        app.corte = "08/10/2026".into();
        app.cargar(archivo.clone());
        let t = app.trabajo.as_mut().unwrap();
        t.filas[0].observacion = "Edición ficticia conservada".into();
        t.filas[0].revisada = true;
        assert!(app.guardar_sesion());
        let mut nueva = App::con_datos(datos, None);
        nueva.corte = "08/10/2026".into();
        nueva.cargar(archivo);
        let t = nueva.trabajo.as_ref().unwrap();
        assert_eq!(t.filas[0].observacion, "Edición ficticia conservada");
        assert!(t.filas[0].revisada);
        std::fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn contactos_de_csmp_se_leen_sin_guardarse_hasta_confirmar() {
        let base = std::env::temp_dir().join(format!("flow-ui-csmp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (datos, csmp) = (base.join("datos"), base.join("CSMP_Personal"));
        std::fs::create_dir_all(&datos).unwrap();
        std::fs::create_dir_all(&csmp).unwrap();
        let ruta_csmp = csmp.join("contactos.json");
        let texto = r#"[{"Nombre":"PRM Ficticio","Mail":"prm@example.invalid"},{"Nombre":"Tribunal: LAJA","Mail":"laja@example.invalid"}]"#;
        std::fs::write(&ruta_csmp, texto).unwrap();
        // Un contactos.json de Flow vacío (de una versión anterior) no cuenta como contactos propios.
        std::fs::write(
            datos.join("contactos.json"),
            br#"{"programas":{},"tribunales":{"LAJA":""},"cc_adicional":"","firma":"Firma ficticia"}"#,
        )
        .unwrap();
        let mut app = App::con_origen(datos.clone(), None, &[base.join("Prototipo").join("configuracion.json"), ruta_csmp.clone()]);
        assert_eq!(app.contactos_csmp.as_deref(), Some(ruta_csmp.as_path()));
        assert_eq!(app.contactos.de_programa("prm ficticio"), "prm@example.invalid");
        assert_eq!(app.contactos.tribunales["LAJA"], "laja@example.invalid");
        assert_eq!(app.contactos.firma, "Firma ficticia");
        assert!(app.mensaje.as_ref().unwrap().1.starts_with("Contactos tomados de CSMP"));
        eframe::App::on_exit(&mut app, None);
        assert!(Contactos::cargar(&datos.join("contactos.json")).unwrap().programas.is_empty(), "no se guarda sin confirmar");
        assert_eq!(std::fs::read_to_string(&ruta_csmp).unwrap(), texto, "CSMP no se modifica");
        // Confirmados, quedan en Flow y ya no se vuelven a tomar de CSMP.
        app.contactos_csmp = None;
        eframe::App::on_exit(&mut app, None);
        std::fs::write(&ruta_csmp, r#"[{"Nombre":"Otro Ficticio","Mail":"otro@example.invalid"}]"#).unwrap();
        let app = App::con_origen(datos.clone(), None, &[ruta_csmp.clone()]);
        assert!(app.contactos_csmp.is_none());
        assert_eq!(app.contactos.programas.keys().collect::<Vec<_>>(), ["PRM Ficticio"]);
        // Sin contactos en ninguna parte: se avisa y los correos se preparan con el destinatario pendiente.
        let vacia = base.join("vacia");
        let app = App::con_origen(vacia, None, &[base.join("no-existe.json")]);
        assert_eq!(app.mensaje.as_ref().unwrap().1, CONTACTOS_FALTAN);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn sin_contactos_los_correos_se_preparan_y_quedan_con_destinatario_pendiente() {
        let (base, _) = preparar();
        let mut app = App::con_datos(base.join("datos"), None);
        assert!(sin_contactos(&app.contactos));
        app.corte = "08/10/2026".into();
        app.cargar(base.join("consulta-001-p001-de-001.xls"));
        let salida = base.join("salida");
        std::fs::create_dir(&salida).unwrap();
        app.ajustes.salida = Some(salida);
        app.otra_modalidad = "modalidad ficticia".into();
        app.preparar_borradores();
        assert!(app.borradores.iter().any(|b| b.tipo.a_programa() && b.para.is_empty()), "{:?}", app.mensaje);
        let guardar = |_: &Borrador| Ok(flow_outlook::Resultado::Guardado);
        app.guardar_borradores_con(true, &guardar);
        // Como `check_products` de Python: los que no tienen destinatario quedan pendientes («Completar destinatario»).
        assert!(app.mensaje.as_ref().is_some_and(|(error, s)| *error && s.contains("falta destinatario")), "{:?}", app.mensaje);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn importar_contactos_json_y_catastro_con_confirmacion() {
        let base = std::env::temp_dir().join(format!("flow-ui-importar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let datos = base.join("datos");
        std::fs::create_dir_all(&datos).unwrap();
        let mut app = App::con_datos(datos.clone(), None);
        app.contactos.programas.insert("PRM Ficticio".into(), "prm@example.invalid".into());
        let json = base.join("contactos.json");
        std::fs::write(
            &json,
            r#"[{"Nombre":"PRM Ficticio","Mail":"cambio@example.invalid"},{"Nombre":"FAE Ficticio","Mail":"fae@example.invalid"}]"#,
        )
        .unwrap();
        app.revisar_contactos_json(&json);
        let (_, imp) = app.importacion.clone().unwrap();
        assert_eq!(imp.conflictos, ["PRM Ficticio"]);
        assert!(!ruta_contactos(&datos).exists(), "nada se guarda antes de confirmar");
        app.confirmar_importacion();
        let guardados = Contactos::cargar(&ruta_contactos(&datos)).unwrap();
        assert_eq!(guardados.programas["PRM Ficticio"], "prm@example.invalid");
        assert_eq!(guardados.programas["FAE Ficticio"], "fae@example.invalid");
        // Catastro (XLS en HTML, como los de SITFA) con columnas Nombre y Mail.
        let catastro = base.join("catastro.xls");
        std::fs::write(
            &catastro,
            "<html><table><tr><th>Nombre</th><th>Mail</th></tr><tr><td>DAM Ficticio</td><td>dam@example.invalid</td></tr></table></html>",
        )
        .unwrap();
        app.revisar_catastro(&catastro);
        assert_eq!(app.importacion.as_ref().unwrap().1.nuevos, ["DAM Ficticio"]);
        app.importacion = None;
        assert!(!Contactos::cargar(&ruta_contactos(&datos)).unwrap().programas.contains_key("DAM Ficticio"), "cancelar no guarda");
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn contacto_y_sesion_danados_se_conservan_al_abrir_y_cerrar() {
        let (base, _) = preparar();
        let datos = base.join("datos");
        std::fs::create_dir(&datos).unwrap();
        let contactos = datos.join("contactos.json");
        std::fs::write(&contactos, b"incompleto ficticio").unwrap();
        let mut app = App::con_datos(datos.clone(), None);
        assert!(app.contactos_error.is_some());
        eframe::App::on_exit(&mut app, None);
        assert_eq!(std::fs::read(&contactos).unwrap(), b"incompleto ficticio");
        let archivo = base.join("consulta-001-p001-de-001.xls");
        app.corte = "08/10/2026".into();
        app.cargar(archivo.clone());
        let ruta = almacen::listar(&datos)[0].0.clone();
        std::fs::write(&ruta, b"sesion incompleta ficticia").unwrap();
        let mut nueva = App::con_datos(datos.clone(), None);
        nueva.corte = "08/10/2026".into();
        nueva.cargar(archivo);
        // Una sesión ilegible (dañada o de un formato anterior) no impide abrir el archivo: se analiza de nuevo y la
        // sesión anterior se conserva intacta con otro nombre.
        assert!(nueva.trabajo.is_some());
        assert!(nueva.mensaje.as_ref().is_some_and(|(error, s)| !error && s.contains("se conserva sin cambios")));
        assert_eq!(std::fs::read(ruta.with_extension("json.ilegible")).unwrap(), b"sesion incompleta ficticia");
        assert_eq!(almacen::listar(&datos).len(), 1, "la sesión nueva se guarda aparte");
        // Volver a abrir recupera la sesión nueva y no repite el aviso.
        let mut otra = App::con_datos(datos.clone(), None);
        otra.corte = "08/10/2026".into();
        otra.cargar(base.join("consulta-001-p001-de-001.xls"));
        assert!(otra.mensaje.as_ref().is_some_and(|(error, s)| !error && !s.contains("se conserva sin cambios")));
        std::fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn guardado_incierto_no_cuenta_como_exito_y_un_segundo_clic_no_duplica() {
        let (base, resumen) = preparar();
        let mut app = App::con_datos(base.join("datos"), None);
        app.corte = "08/10/2026".into();
        app.importar_captura(base.clone(), resumen);
        let salida = base.join("salida");
        std::fs::create_dir(&salida).unwrap();
        app.ajustes.salida = Some(salida);
        app.borradores = (0..4)
            .map(|i| Borrador {
                tipo: Tipo::Informativo,
                tribunal: "Laja".into(),
                programa: String::new(),
                para: "ficticio@example.invalid".into(),
                cc: CC_FIJO.into(),
                asunto: format!("Asunto ficticio {i}"),
                cuerpo: "Texto ficticio".into(),
                adjuntos: vec![],
                filas: vec![format!("fila-{i}")],
                omitido: false,
                originales: None,
                huella: String::new(),
            })
            .collect();
        let n = std::cell::Cell::new(0);
        let guardar = |b: &Borrador| {
            n.set(n.get() + 1);
            if b.asunto.ends_with('2') {
                Ok(flow_outlook::Resultado::Incierto("Fallo ficticio de confirmación".into()))
            } else {
                Ok(flow_outlook::Resultado::Guardado)
            }
        };
        app.guardar_borradores_con(true, &guardar);
        assert_eq!(n.get(), 4);
        assert!(app.mensaje.as_ref().is_some_and(|(error, s)| *error && s.starts_with("3 guardados")));
        app.guardar_borradores_con(true, &guardar);
        assert_eq!(n.get(), 4);
        assert!(app.mensaje.as_ref().is_some_and(|(error, s)| *error && s.contains("3 ya estaban guardados")));
        assert_eq!(app.estados_borradores[&2], crate::borradores::Estado::Incierto);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn un_tipo_sin_programa_no_anula_los_demas_borradores() {
        let (base, _) = preparar();
        let datos = base.join("datos");
        let archivo = base.join("consulta-001-p001-de-001.xls");
        let mut app = App::con_datos(datos, None);
        app.corte = "08/10/2026".into();
        app.cargar(archivo);
        let salida = base.join("salida");
        std::fs::create_dir(&salida).unwrap();
        app.ajustes.salida = Some(salida);
        // «Otra…» no filtra filas (con un filtro, la fila sin programa quedaría fuera) y habilita el informativo.
        app.otra_modalidad = "modalidad ficticia".into();
        let t = app.trabajo.as_mut().unwrap();
        // Una fila con acción «programa_espera» y PROGRAMA vacío: ese correo pide completar el programa.
        let i = t
            .filas
            .iter()
            .position(|f| f.tiene_accion("programa_espera") && flow_dominio::Tribunal::desde(&f.registro.texto("tribunal")).is_some())
            .expect("caso ficticio con lista de espera");
        t.filas[i].registro.poner("programa", flow_dominio::Celda::Vacia);
        app.preparar_borradores();
        assert!(!app.borradores.is_empty(), "los demás tipos se preparan");
        assert!(app.borradores.iter().all(|b| b.tipo != Tipo::ProgramaEspera || !b.programa.is_empty()));
        assert!(
            app.mensaje.as_ref().is_some_and(|(error, s)| *error && s.contains("No se prepararon") && s.contains("PROGRAMA")),
            "{:?}",
            app.mensaje
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn la_sesion_guarda_borradores_proyectos_y_preferencias() {
        let (base, _) = preparar();
        let datos = base.join("datos");
        let archivo = base.join("consulta-001-p001-de-001.xls");
        let mut app = App::con_datos(datos.clone(), None);
        app.corte = "08/10/2026".into();
        app.cargar(archivo.clone());
        let salida = base.join("salida");
        std::fs::create_dir(&salida).unwrap();
        app.ajustes.salida = Some(salida);
        app.modalidades[1].1 = true;
        app.otra_modalidad = "modalidad ficticia".into();
        app.periodo = "02/2099".into();
        app.preparar_borradores();
        assert!(!app.borradores.is_empty(), "{:?}", app.mensaje);
        app.borradores[0].asunto = "Asunto editado ficticio".into();
        app.borradores[0].omitido = true;
        app.preparar_proyectos();
        assert!(!app.proyectos.is_empty(), "{:?}", app.mensaje);
        app.proyectos[0].valores.insert("PROGRAMA".into(), "Programa editado ficticio".into());
        app.proyectos[0].omitido = true;
        app.guardar_sesion();
        let (borradores, proyectos, seleccion) = (app.borradores.clone(), app.proyectos.clone(), app.seleccion_preparada.clone());
        assert!(!seleccion.is_empty());

        let mut nueva = App::con_datos(datos.clone(), None);
        nueva.corte = "08/10/2026".into();
        nueva.cargar(archivo.clone());
        assert_eq!(nueva.borradores, borradores, "borradores con sus ediciones y omisiones");
        assert_eq!(nueva.proyectos, proyectos, "proyectos con sus valores editados");
        assert_eq!(nueva.seleccion_preparada, seleccion);
        assert!(nueva.modalidades[1].1 && !nueva.modalidades[0].1);
        assert_eq!((nueva.otra_modalidad.as_str(), nueva.periodo.as_str()), ("modalidad ficticia", "02/2099"));
        assert_eq!(nueva.carpeta, app.carpeta, "los productos siguen en la misma carpeta");
        assert!(nueva.mensaje.as_ref().is_some_and(|(error, s)| !error && s.contains("Se recuperaron")), "{:?}", nueva.mensaje);
        // Volver a preparar conserva los valores editados (`merge_project_edits`).
        nueva.preparar_proyectos();
        assert_eq!(nueva.proyectos[0].valores["PROGRAMA"], "Programa editado ficticio");
        assert!(nueva.proyectos[0].omitido);

        // Una sesión anterior a este campo se sigue leyendo: sin productos guardados y sin error.
        let ruta = almacen::listar(&datos)[0].0.clone();
        let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(&ruta).unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("sesion");
        std::fs::write(&ruta, serde_json::to_vec(&json).unwrap()).unwrap();
        let mut antigua = App::con_datos(datos, None);
        antigua.corte = "08/10/2026".into();
        antigua.cargar(archivo);
        assert!(antigua.trabajo.is_some() && antigua.borradores.is_empty() && antigua.proyectos.is_empty());
        assert!(antigua.mensaje.as_ref().is_some_and(|(error, _)| !error), "{:?}", antigua.mensaje);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn obsoletos_ediciones_conservadas_y_solo_filas_seleccionadas() {
        let (base, _) = preparar();
        let datos = base.join("datos");
        let archivo = base.join("consulta-001-p001-de-001.xls");
        let mut app = App::con_datos(datos.clone(), None);
        app.corte = "08/10/2026".into();
        app.cargar(archivo.clone());
        let salida = base.join("salida");
        std::fs::create_dir(&salida).unwrap();
        app.ajustes.salida = Some(salida);
        // «Preparar todos» exige modalidades, como Python.
        app.preparar_borradores();
        assert!(app.mensaje.as_ref().is_some_and(|(e, s)| *e && s.contains("al menos una modalidad")), "{:?}", app.mensaje);
        app.otra_modalidad = "modalidad ficticia".into();
        app.preparar_borradores();
        assert!(!app.borradores.is_empty(), "{:?}", app.mensaje);
        assert!(app.borradores_obsoletos().is_empty());
        app.borradores[0].asunto = "Asunto editado ficticio".into();
        app.borradores[0].para = "ficticio@example.test".into();
        // Editar después de preparar deja obsoleto el borrador: no se guarda.
        let id = app.borradores[0].filas[0].clone();
        app.trabajo.as_mut().unwrap().filas.iter_mut().find(|f| f.id == id).unwrap().observacion = "Edición ficticia posterior.".into();
        assert!(app.borradores_obsoletos().contains(&0));
        let llamados = std::cell::RefCell::new(Vec::new());
        let guardar = |b: &Borrador| {
            llamados.borrow_mut().push(b.asunto.clone());
            Ok(flow_outlook::Resultado::Guardado)
        };
        app.guardar_borradores_con(true, &guardar);
        assert!(!llamados.borrow().contains(&"Asunto editado ficticio".to_string()));
        assert!(app.mensaje.as_ref().is_some_and(|(e, s)| *e && s.contains("necesita actualizarse")), "{:?}", app.mensaje);
        // Volver a preparar conserva las ediciones (`merge_draft_edits`) y deja el borrador vigente.
        app.preparar_borradores();
        let b = app.borradores.iter().find(|b| b.filas.contains(&id)).unwrap();
        assert_eq!((b.asunto.as_str(), b.para.as_str()), ("Asunto editado ficticio", "ficticio@example.test"));
        assert!(app.borradores_obsoletos().is_empty());
        // «Solo filas seleccionadas»: exige marcar filas y se limita a ellas.
        app.manual = true;
        app.preparar_borradores();
        assert!(app.mensaje.as_ref().is_some_and(|(e, s)| *e && s.contains("Selecciona los registros")));
        app.marcadas.insert(id.clone());
        app.preparar_borradores();
        assert!(!app.borradores.is_empty() && app.borradores.iter().all(|b| b.filas == vec![id.clone()]));
        app.preparar_proyectos();
        app.guardar_sesion();
        // Al reanudar con otras reglas en el programa, los productos se conservan pero quedan obsoletos.
        let mut nueva = App::con_datos(datos, None);
        nueva.motor.umbrales.insert("FICTICIO.UMBRAL".into(), 1);
        nueva.reanudar();
        assert!(nueva.manual && nueva.marcadas.contains(&id));
        assert_eq!(nueva.borradores.len(), app.borradores.len());
        assert_eq!(nueva.borradores_obsoletos().len(), nueva.borradores.len());
        assert_eq!(nueva.proyectos_obsoletos().len(), nueva.proyectos.len());
        assert!(nueva.mensaje.as_ref().is_some_and(|(_, s)| s.contains("quedan obsoletos")), "{:?}", nueva.mensaje);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn las_modalidades_marcadas_filtran_filas_y_otra_no_filtra() {
        let mut app = App::con_datos(std::env::temp_dir().join("flow-ui-modalidades-ficticia"), None);
        assert_eq!(app.claves_modalidades(), None);
        app.modalidades[1].1 = true;
        assert_eq!(app.claves_modalidades(), Some(["AMB"].into()));
        app.otra_modalidad = "Otra ficticia".into();
        assert_eq!(app.claves_modalidades(), None);
    }
}













