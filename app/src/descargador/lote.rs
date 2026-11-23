//! Ejecución secuencial de un lote Lite (`lotes.BatchRunner.run` y `motor.run_sequence` de Flow Python).
//! Una carpeta por lote, nombres descriptivos, manifiesto por consulta y `resumen.json` actualizado tras cada paso.
//! Solo lectura: el puente admite órdenes de consulta y descarga; nada se escribe en RUS ni SITFA.

use super::evidencia::png_a_pdf;
use super::sitfa::{
    siguiente_consulta, validar_descarga, validar_pantalla_evidencia, validar_respuesta, Consulta,
    Pares, Respuesta,
};
use super::xls::{huella, Verificador};
use super::{Alcance, Catalogo, Edicion, Pantalla, Perfil, PlanLote, Puente, Seleccion};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::NaiveDate;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::Duration;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub const PLAZO: Duration = Duration::from_secs(75);
const PLAZO_CIERRE: Duration = Duration::from_secs(8);
const ESPERA_SITFA_MS: u64 = 60_000;
const MAX_NOMBRE: usize = 150;
const SUFIJO_MAXIMO: &str = " (99) - p99 de 99.xls";
const CANCELADO: &str = "Lote cancelado. Se conservan los archivos verificados.";
const CANCELADO_PAGINAS: &str = "Descarga cancelada. Las paginas ya verificadas se conservan.";

/// Orden local hacia la extensión. `Puente` la implementa; las pruebas usan un SITFA ficticio.
pub trait Conexion {
    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String>;
}

impl Conexion for Puente {
    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
        Puente::llamar(self, comando, payload, plazo)
    }
}

#[derive(Clone, Debug)]
pub struct Opciones {
    /// Conserva los filtros adicionales del formulario SITFA (`keep_filters`).
    pub conservar_filtros: bool,
    /// Pantallazo PDF de las consultas sin registros.
    pub evidencia: bool,
    pub pausa: Duration,
}

impl Default for Opciones {
    fn default() -> Self {
        Self {
            conservar_filtros: false,
            evidencia: true,
            pausa: Duration::ZERO,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Evento {
    Carpeta(PathBuf),
    Plan(usize),
    Consulta { indice: usize, total: usize, etiqueta: String },
    Estado(String),
    Total(usize),
    Pagina { consulta: String, pagina: usize, total: usize },
    Vacia { consulta: String },
    Evidencia { archivo: String },
    ConsultaLista { consulta: String, estado: String },
    Aviso(String),
    Fin(Fin),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fin {
    pub carpeta: PathBuf,
    pub consultas: usize,
    pub archivos: usize,
    pub pdfs: usize,
    pub vacias: usize,
    pub registros: usize,
}

const MODALIDADES_LITE: [(&str, &str); 4] =
    [("1", "Residencia"), ("2", "Ambulatorio"), ("3", "FAE"), ("4", "DCE")];

fn titulo(pantalla: Pantalla) -> &'static str {
    match pantalla {
        Pantalla::Seguimiento => "Seguimiento",
        Pantalla::Litigantes => "Busqueda Lit",
        Pantalla::CalendarioInformes => "Informes por vencer",
        Pantalla::CalendarioMedidas => "Medidas por vencer",
        Pantalla::Carga => "Carga",
    }
}

fn largo(s: &str) -> usize {
    s.chars().count()
}

fn cortar(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// `lotes.safe_name`: sin tildes, caracteres prohibidos en Windows ni puntos; espacios simples; 125 caracteres.
pub fn nombre_seguro(valor: &str) -> String {
    let limpio: String = valor
        .nfkd()
        .filter(|c| !is_combining_mark(*c))
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '.') || (c as u32) < 0x20 {
                ' '
            } else {
                c
            }
        })
        .collect();
    let unido = limpio.split_whitespace().collect::<Vec<_>>().join(" ");
    cortar(unido.trim_end_matches([' ', '.']), 125)
}

/// `lotes.tribunal_name`: abreviaturas de los juzgados.
pub fn nombre_tribunal(valor: &str) -> String {
    static REGLAS: LazyLock<[(Regex, &str); 3]> = LazyLock::new(|| {
        [
            (Regex::new(r"(?i)^Juzgado de Familia\s+").unwrap(), "Jdo Familia "),
            (Regex::new(r"(?i)^Jgdo\s+L\s+y\s+G\s+de\s+").unwrap(), "Jdo Letras y Garantia "),
            (Regex::new(r"(?i)^Jgdo\s+(?:Letras|L)\s+de\s+").unwrap(), "Jdo Letras "),
        ]
    });
    let mut v = nombre_seguro(valor);
    for (re, reemplazo) in REGLAS.iter() {
        v = re.replacen(&v, 1, *reemplazo).into_owned();
    }
    v
}

fn fecha_sitfa(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

fn etiqueta_fechas(fechas: (NaiveDate, NaiveDate)) -> String {
    format!("{} al {}", fechas.0.format("%d-%m-%Y"), fechas.1.format("%d-%m-%Y"))
}

fn calendario(p: Pantalla) -> bool {
    matches!(p, Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas)
}

fn etiqueta<'a>(grupo: &'a std::collections::BTreeMap<String, String>, clave: &str) -> &'a str {
    grupo.get(clave).map(String::as_str).unwrap_or("")
}

/// `lotes.report_label`.
fn etiqueta_reporte(perfil: &Perfil, catalogo: &Catalogo) -> String {
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, reporte, .. } if pestana == "Informes" => {
            etiqueta(&catalogo.informes, reporte.as_deref().unwrap_or("")).into()
        }
        Alcance::Seguimiento { pestana, reporte, .. } if pestana == "Cumplimiento" => match reporte {
            Some(r) => etiqueta(&catalogo.medidas, r).into(),
            None => "Todas las medidas".into(),
        },
        _ => String::new(),
    }
}

/// `lotes.query_parts`: etiquetas en el orden en que aparecen en el nombre del archivo.
pub fn partes_consulta(perfil: &Perfil, catalogo: &Catalogo, fechas: Option<(NaiveDate, NaiveDate)>) -> Vec<String> {
    let mut partes = vec![titulo(perfil.pantalla).to_string()];
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, modalidad, .. } => {
            partes.push(pestana.clone());
            partes.push(
                MODALIDADES_LITE
                    .iter()
                    .find(|(c, _)| c == modalidad)
                    .map(|(_, n)| n.to_string())
                    .unwrap_or_else(|| etiqueta(&catalogo.modalidades, modalidad).into()),
            );
            let reporte = etiqueta_reporte(perfil, catalogo);
            if !reporte.is_empty() {
                partes.push(reporte);
            }
        }
        Alcance::Litigantes { estado } => partes.push(etiqueta(&catalogo.estados, estado).into()),
        Alcance::Calendario { mes, anio } => partes.push(format!(
            "{} {}",
            etiqueta(&catalogo.meses, mes),
            etiqueta(&catalogo.anios, anio)
        )),
        Alcance::Carga { .. } => {}
    }
    if let Some(f) = fechas.filter(|_| !calendario(perfil.pantalla)) {
        partes.push(etiqueta_fechas(f));
    }
    partes
        .iter()
        .map(|p| nombre_seguro(p))
        .filter(|p| !p.is_empty())
        .collect()
}

/// `lotes.name_limit`: carpeta, nombre y sufijo de página caben en los 260 caracteres de Windows.
pub fn limite_nombre(carpeta: &Path) -> usize {
    let absoluta = std::path::absolute(carpeta).unwrap_or_else(|_| carpeta.to_path_buf());
    let disponible = 259usize
        .saturating_sub(largo(&absoluta.to_string_lossy()))
        .saturating_sub(1 + largo(SUFIJO_MAXIMO));
    disponible.clamp(60, MAX_NOMBRE)
}

/// `lotes.file_base`. Ej.: «Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia X - 01-10-2026 al 31-10-2026».
pub fn base_archivo(
    perfil: &Perfil,
    catalogo: &Catalogo,
    fechas: Option<(NaiveDate, NaiveDate)>,
    limite: usize,
) -> String {
    let partes = partes_consulta(perfil, catalogo, fechas);
    let corte = (if perfil.pantalla == Pantalla::Seguimiento { 4 } else { 2 }).min(partes.len());
    let cabeza = partes[..corte].join(" - ");
    let cola = &partes[corte..];
    let tribunal = nombre_tribunal(etiqueta(&catalogo.tribunales, &perfil.tribunal));
    let espacio = limite as isize
        - largo(&cabeza) as isize
        - cola.iter().map(|x| largo(x) as isize + 3).sum::<isize>()
        - 3;
    let tribunal = cortar(&tribunal, espacio.max(12) as usize);
    let tribunal = tribunal.trim_end_matches([' ', '.']);
    let mut todo = vec![cabeza.as_str(), tribunal];
    todo.extend(cola.iter().map(String::as_str));
    cortar(&todo.join(" - "), limite)
        .trim_end_matches([' ', '.', '-'])
        .to_string()
}

/// `descargador/lotes.file_base` (Integral): «ESP Ambulatorio Jdo Familia X», «Informes Jdo Familia X»…
pub fn base_integral(perfil: &Perfil, catalogo: &Catalogo) -> String {
    let tribunal = nombre_tribunal(etiqueta(&catalogo.tribunales, &perfil.tribunal));
    let base = match (&perfil.alcance, perfil.pantalla) {
        (_, Pantalla::CalendarioInformes) => format!("Informes {tribunal}"),
        (_, Pantalla::CalendarioMedidas) => format!("Medidas por vencer {tribunal}"),
        (Alcance::Litigantes { estado }, _) => format!("Litigantes {} {tribunal}", etiqueta(&catalogo.estados, estado)),
        (Alcance::Carga { desde, hasta }, _) => format!("Carga {tribunal} {} {}", fecha_sitfa(*desde), fecha_sitfa(*hasta)),
        (Alcance::Seguimiento { pestana, modalidad, .. }, _) => match pestana.as_str() {
            "Informes" => format!("Informes {tribunal}"),
            "Egresados" => format!("Egresados {tribunal}"),
            p => format!(
                "{} {} {tribunal}",
                if p == "Espera" { "ESP" } else { "CUMP" },
                MODALIDADES_LITE.iter().find(|(c, _)| c == modalidad).map_or("", |(_, n)| n)
            ),
        },
        (Alcance::Calendario { .. }, _) => tribunal,
    };
    nombre_seguro(&base)
}

/// `perfiles.selection_profile`: clave y etiqueta del manifiesto.
fn caso_y_rotulo(perfil: &Perfil, catalogo: &Catalogo) -> (String, String) {
    let t = &perfil.tribunal;
    let nombre = etiqueta(&catalogo.tribunales, t);
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, modalidad, reporte } => {
            let mut clave = format!("T{t}-{pestana}-M{modalidad}");
            let mut rotulo = format!("{nombre} / {pestana} / {}", etiqueta(&catalogo.modalidades, modalidad));
            if let Some(r) = reporte.as_ref().filter(|_| matches!(pestana.as_str(), "Informes" | "Cumplimiento")) {
                let grupo = if pestana == "Informes" { &catalogo.informes } else { &catalogo.medidas };
                rotulo.push_str(&format!(" / {}", etiqueta(grupo, r)));
                clave.push_str(&format!("-I{r}"));
            }
            (clave, rotulo)
        }
        Alcance::Litigantes { estado } => (
            format!("T{t}-Litigantes-E{estado}"),
            format!("{nombre} / Litigantes / {}", etiqueta(&catalogo.estados, estado)),
        ),
        Alcance::Calendario { mes, anio } => {
            let tipo = if perfil.pantalla == Pantalla::CalendarioInformes { "Informes" } else { "Medidas" };
            (format!("T{t}-{tipo}-{anio}-{mes}"), format!("{nombre} / {tipo} por vencer / {mes}-{anio}"))
        }
        Alcance::Carga { desde, hasta } => (
            format!("T{t}-Carga-{}-{}", desde.format("%Y-%m-%d"), hasta.format("%Y-%m-%d")),
            format!("{nombre} / Carga / {}–{}", fecha_sitfa(*desde), fecha_sitfa(*hasta)),
        ),
    }
}

/// `asdict(Selection)` de Integral: agrega `start` y `end`, vacíos fuera de Carga.
fn seleccion_edicion(perfil: &Perfil, edicion: Edicion) -> Value {
    let mut v = seleccion(perfil);
    if edicion == Edicion::Integral && perfil.pantalla != Pantalla::Carga {
        v["start"] = Value::Null;
        v["end"] = Value::Null;
    }
    v
}

/// `dataclasses.asdict(Selection)` con los valores por omisión de Flow Python.
fn seleccion(perfil: &Perfil) -> Value {
    let pantalla = serde_json::to_value(perfil.pantalla).unwrap_or_default();
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, modalidad, reporte } => json!({
            "tribunal": perfil.tribunal, "tab": pestana, "modality": modalidad, "report": reporte,
            "screen": pantalla, "state": "1", "month": null, "year": null,
        }),
        Alcance::Litigantes { estado } => json!({
            "tribunal": perfil.tribunal, "tab": "", "modality": "", "report": null,
            "screen": pantalla, "state": estado, "month": null, "year": null,
        }),
        Alcance::Calendario { mes, anio } => json!({
            "tribunal": perfil.tribunal, "tab": "", "modality": "", "report": null,
            "screen": pantalla, "state": "1", "month": mes, "year": anio,
        }),
        Alcance::Carga { desde, hasta } => json!({
            "tribunal": perfil.tribunal, "tab": "", "modality": "", "report": null,
            "screen": pantalla, "state": "1", "month": null, "year": null,
            "start": fecha_sitfa(*desde), "end": fecha_sitfa(*hasta),
        }),
    }
}

/// `lotes.evidence_context`: identifica la consulta en el pantallazo.
fn contexto(perfil: &Perfil, catalogo: &Catalogo, fechas: Option<(NaiveDate, NaiveDate)>) -> Vec<[String; 2]> {
    let par = |a: &str, b: String| [a.to_string(), b];
    let mut c = vec![
        par("Tribunal", format!("{} (código {})", etiqueta(&catalogo.tribunales, &perfil.tribunal), perfil.tribunal)),
        par("Consulta", titulo(perfil.pantalla).into()),
    ];
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, modalidad, .. } => {
            c.push(par("Pestaña", pestana.clone()));
            c.push(par("Modalidad", etiqueta(&catalogo.modalidades, modalidad).into()));
            let reporte = etiqueta_reporte(perfil, catalogo);
            if !reporte.is_empty() {
                c.push(par(if pestana == "Informes" { "Informe" } else { "Medidas" }, reporte));
            }
        }
        Alcance::Litigantes { estado } => c.push(par("Estado", etiqueta(&catalogo.estados, estado).into())),
        Alcance::Calendario { mes, anio } => c.push(par(
            "Mes",
            format!("{} {}", etiqueta(&catalogo.meses, mes), etiqueta(&catalogo.anios, anio)),
        )),
        Alcance::Carga { .. } => {}
    }
    if !calendario(perfil.pantalla) {
        c.push(par(
            "Fechas",
            fechas
                .map(|(a, b)| format!("{} – {}", fecha_sitfa(a), fecha_sitfa(b)))
                .unwrap_or_else(|| "Sin filtro de fechas".into()),
        ));
    }
    for p in &mut c {
        p[1] = cortar(&p[1], 500);
    }
    c
}

/// Contexto del pantallazo en `descargador/lotes.py`: Tribunal, Modalidad y Pestaña, Consulta (etiqueta) y Fechas.
fn contexto_integral(perfil: &Perfil, catalogo: &Catalogo, fechas: Option<(NaiveDate, NaiveDate)>) -> Vec<[String; 2]> {
    let par = |a: &str, b: String| [a.to_string(), b];
    let mut c = vec![par(
        "Tribunal",
        format!("{} (código {})", etiqueta(&catalogo.tribunales, &perfil.tribunal), perfil.tribunal),
    )];
    if let Alcance::Seguimiento { pestana, modalidad, .. } = &perfil.alcance {
        c.push(par("Modalidad", etiqueta(&catalogo.modalidades, modalidad).into()));
        c.push(par("Pestaña", pestana.clone()));
    }
    c.push(par("Consulta", caso_y_rotulo(perfil, catalogo).1));
    c.push(par(
        "Fechas",
        fechas
            .map(|(a, b)| format!("{} – {}", fecha_sitfa(a), fecha_sitfa(b)))
            .unwrap_or_else(|| "Sin filtro de fechas".into()),
    ));
    c
}

fn ahora() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, false)
}

/// `motor.write_error`: motivo concreto de una escritura fallida.
fn error_escritura(e: &std::io::Error, ruta: &Path) -> String {
    let completa = std::path::absolute(ruta).unwrap_or_else(|_| ruta.to_path_buf());
    let codigo = e.raw_os_error();
    let carpeta = completa.parent().map(|p| p.display().to_string()).unwrap_or_default();
    let motivo = if matches!(codigo, Some(3 | 206)) || largo(&completa.to_string_lossy()) >= 260 {
        format!(
            "la ruta es demasiado larga para Windows ({} caracteres). Elige una carpeta de destino más corta",
            largo(&completa.to_string_lossy())
        )
    } else if e.kind() == ErrorKind::PermissionDenied || matches!(codigo, Some(5 | 13 | 32 | 33)) {
        "Windows no permitió escribir en esa carpeta (permiso, archivo abierto o protección «Acceso controlado a carpetas» del antivirus). Permite Flow en esa protección o elige otra carpeta".into()
    } else if e.kind() == ErrorKind::StorageFull || matches!(codigo, Some(28 | 112)) {
        "no queda espacio en el disco".into()
    } else {
        format!("error del sistema {:?} {}", e.kind(), codigo.unwrap_or_default())
    };
    format!("Falló la escritura local del archivo: {motivo}. Carpeta: {carpeta}")
}

/// `motor.save_exclusive`: nunca sobrescribe; solo queda el archivo escrito completo.
pub fn guardar_exclusivo(ruta: &Path, datos: &[u8]) -> Result<(), String> {
    let mut archivo = match OpenOptions::new().write(true).create_new(true).open(ruta) {
        Ok(a) => a,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            return Err("Ya existe el archivo de destino; usar una ejecucion nueva.".into())
        }
        Err(e) => return Err(error_escritura(&e, ruta)),
    };
    if let Err(e) = archivo.write_all(datos).and_then(|_| archivo.sync_all()) {
        drop(archivo);
        let _ = fs::remove_file(ruta);
        return Err(error_escritura(&e, ruta));
    }
    Ok(())
}

/// Escritura atómica: temporal y reemplazo. `resumen.json` usa `resumen.tmp`; los manifiestos, `<nombre>.tmp`.
fn escribir_json(carpeta: &Path, nombre: &str, valor: &Value) -> Result<(), String> {
    let temporal = match nombre.strip_suffix(".json").filter(|_| nombre == "resumen.json") {
        Some(raiz) => carpeta.join(format!("{raiz}.tmp")),
        None => carpeta.join(format!("{nombre}.tmp")),
    };
    let datos = serde_json::to_string_pretty(valor).map_err(|_| "No se pudo preparar el resumen del lote.")?;
    fs::write(&temporal, datos)
        .and_then(|_| fs::rename(&temporal, carpeta.join(nombre)))
        .map_err(|e| error_escritura(&e, &carpeta.join(nombre)))
}

fn cuerpo(respuesta: &Value, excel: bool) -> Result<Vec<u8>, String> {
    let error = || "Respuesta vacía, vencida o fuera del formato permitido.".to_string();
    let texto = respuesta["body"].as_str().ok_or_else(error)?;
    if texto.len() > 4 * (80 * 1024 * 1024 + 2) / 3 {
        return Err(error());
    }
    if !excel && !respuesta["type"].is_string() {
        return Err(error());
    }
    let datos = STANDARD.decode(texto).map_err(|_| error())?;
    if respuesta["status"].as_u64() != Some(200) || datos.is_empty() || datos.len() > 80 * 1024 * 1024 {
        return Err(error());
    }
    if !excel
        && !respuesta["type"]
            .as_str()
            .is_some_and(|t| t.to_ascii_lowercase().contains("html"))
    {
        return Err(error());
    }
    Ok(datos)
}

fn cancelado(cancelar: &AtomicBool, mensaje: &str) -> Result<(), String> {
    if cancelar.load(Ordering::SeqCst) {
        Err(mensaje.into())
    } else {
        Ok(())
    }
}

/// Espera cancelable, como `PauseGate.wait`.
fn pausa(duracion: Duration, cancelar: &AtomicBool, mensaje: &str) -> Result<(), String> {
    let mut resta = duracion;
    while !resta.is_zero() {
        cancelado(cancelar, mensaje)?;
        let paso = resta.min(Duration::from_millis(50));
        std::thread::sleep(paso);
        resta -= paso;
    }
    cancelado(cancelar, mensaje)
}

fn carpeta_nueva(destino: &Path) -> Result<PathBuf, String> {
    let ahora = chrono::Local::now();
    let mut carpeta = destino.join(ahora.format("%Y-%m-%d %H-%M-%S").to_string());
    if carpeta.exists() {
        carpeta = destino.join(ahora.format("%Y-%m-%d %H-%M-%S-%6f").to_string());
    }
    fs::create_dir_all(destino).map_err(|e| error_escritura(&e, destino))?;
    fs::create_dir(&carpeta).map_err(|e| {
        if e.kind() == ErrorKind::AlreadyExists {
            "Ya existe la carpeta del lote; vuelve a iniciar.".to_string()
        } else {
            error_escritura(&e, &carpeta)
        }
    })?;
    Ok(carpeta)
}

struct Lote<'a> {
    conexion: &'a dyn Conexion,
    edicion: Edicion,
    catalogo: &'a Catalogo,
    opciones: &'a Opciones,
    cancelar: &'a AtomicBool,
    carpeta: PathBuf,
}

impl Lote<'_> {
    fn pedir(&self, comando: &str, payload: Value) -> Result<Value, String> {
        self.conexion.llamar(comando, payload, PLAZO)
    }

    fn manifiesto(&self, nombre: &str, caso: &(String, String), archivos: &[Value], total: usize, estado: &str) -> Result<(), String> {
        escribir_json(
            &self.carpeta,
            nombre,
            &json!({
                "fecha_hora": ahora(), "caso": caso.0, "consulta": caso.1, "estado": estado,
                "paginas_esperadas": total, "paginas_verificadas": archivos.len(), "archivos": archivos,
            }),
        )
    }

    /// `motor.run_sequence`: POST(n) → XLS(n) → validar → fsync → releer → POST(n+1). Sin reintentos silenciosos.
    #[allow(clippy::too_many_arguments)]
    fn paginas(
        &self,
        pares: &Pares,
        primera: super::sitfa::Pagina,
        consulta: &Consulta,
        caso: &(String, String),
        manifiesto: &str,
        nombre: &dyn Fn(usize, usize) -> String,
        emitir: &mut dyn FnMut(Evento),
        etiqueta_consulta: &str,
    ) -> Result<Vec<Value>, String> {
        let total = primera.total;
        let mut resultados: Vec<Value> = Vec::new();
        self.manifiesto(manifiesto, caso, &resultados, total, "EN_CURSO")?;
        let ejecucion = (|| -> Result<(), String> {
            let mut verificador = Verificador::default();
            let mut actuales = pares.clone();
            let mut pagina = primera.clone();
            for numero in 1..=total {
                cancelado(self.cancelar, CANCELADO_PAGINAS)?;
                if numero > 1 {
                    pausa(self.opciones.pausa, self.cancelar, CANCELADO_PAGINAS)?;
                    actuales = siguiente_consulta(&actuales, consulta, &pagina, numero)?;
                    let html = cuerpo(
                        &self.pedir("search", json!({"pairs": actuales, "timeout": ESPERA_SITFA_MS}))?,
                        false,
                    )?;
                    pagina = match validar_respuesta(&html, &actuales, consulta, Some((numero, &primera)))? {
                        Respuesta::ConRegistros(p) => p,
                        Respuesta::SinRegistros => {
                            return Err("No se pudieron identificar todos los registros de la pagina.".into())
                        }
                    };
                }
                validar_descarga(&pagina.excel_url, &primera.plantilla)?;
                let datos = cuerpo(
                    &self.pedir("download", json!({"url": pagina.excel_url, "timeout": ESPERA_SITFA_MS}))?,
                    true,
                )?;
                let (libro, digest) = verificador.verificar(&datos, &pagina, consulta)?;
                let archivo = nombre(numero, total);
                if Path::new(&archivo).file_name().and_then(|n| n.to_str()) != Some(archivo.as_str())
                    || !archivo.ends_with(".xls")
                {
                    return Err("Nombre de archivo fuera de alcance.".into());
                }
                let ruta = self.carpeta.join(&archivo);
                guardar_exclusivo(&ruta, &datos)?;
                let guardado = fs::read(&ruta).map_err(|e| error_escritura(&e, &ruta))?;
                if guardado != datos
                    || Verificador::default().verificar(&guardado, &pagina, consulta)?.0.registros != pagina.registros
                {
                    return Err("El archivo guardado no coincide con la descarga validada.".into());
                }
                verificador.registrar(&libro, digest.clone())?;
                resultados.push(json!({
                    "pagina": numero, "total": total, "archivo": archivo, "bytes": guardado.len(),
                    "sha256": digest, "formato": libro.formato,
                    "registros": if consulta.pantalla == Pantalla::Carga {
                        pagina.carga.values().sum::<usize>()
                    } else {
                        libro.registros.values().sum::<usize>()
                    },
                    "coincide_con_html": true, "estado": "OK",
                }));
                self.manifiesto(manifiesto, caso, &resultados, total, "EN_CURSO")?;
                emitir(Evento::Pagina {
                    consulta: etiqueta_consulta.into(),
                    pagina: numero,
                    total,
                });
            }
            if resultados.len() != total
                || resultados
                    .iter()
                    .any(|r| !self.carpeta.join(r["archivo"].as_str().unwrap_or("")).is_file())
            {
                return Err("La cantidad de XLS no coincide con el total de paginas.".into());
            }
            Ok(())
        })();
        match ejecucion {
            Ok(()) => {
                self.manifiesto(manifiesto, caso, &resultados, total, "VALIDADA")?;
                Ok(resultados)
            }
            Err(e) => {
                let _ = self.manifiesto(manifiesto, caso, &resultados, total, "INCOMPLETA");
                Err(e)
            }
        }
    }

    /// `BatchRunner.capture_empty`: pantallazo de SITFA con la selección visible, guardado como PDF.
    fn evidencia(
        &self,
        perfil: &Perfil,
        consulta: &Consulta,
        pares: &Pares,
        fechas: Option<(NaiveDate, NaiveDate)>,
        nombre: &str,
        emitir: &mut dyn FnMut(Evento),
    ) -> Result<Value, String> {
        emitir(Evento::Estado("Consulta sin registros: capturando pantallazo…".into()));
        let integral = self.edicion == Edicion::Integral;
        let contexto = if integral {
            contexto_integral(perfil, self.catalogo, fechas)
        } else {
            contexto(perfil, self.catalogo, fechas)
        };
        let rotulo = caso_y_rotulo(perfil, self.catalogo).1;
        let resultado = (|| -> Result<Value, String> {
            let vista = self.pedir("evidence_open", json!({"pairs": pares, "label": rotulo, "context": contexto}))?;
            let html = cuerpo(&vista, false)?;
            let digest = huella(&html);
            if vista["digest"].as_str() != Some(digest.as_str()) {
                return Err("La pantalla de evidencia no coincide con la respuesta recibida.".into());
            }
            let textos = fechas.map(|(a, b)| (fecha_sitfa(a), fecha_sitfa(b)));
            validar_pantalla_evidencia(
                &html,
                pares,
                consulta,
                &contexto,
                textos.as_ref().map(|(a, b)| (a.as_str(), b.as_str())),
            )
            .map_err(|e| match (integral, e.ends_with("Recarga la extensión Lite.")) {
                (true, true) => "La pantalla no identifica el contexto de la consulta solicitada. Recarga la extensión de esta entrega.".into(),
                _ => e,
            })?;
            cancelado(self.cancelar, CANCELADO)?;
            let captura = self.pedir("evidence_capture", json!({"digest": digest}))?;
            let pdf = png_a_pdf(&captura, &digest, &rotulo)?;
            let suma = huella(&pdf);
            let ruta = self.carpeta.join(nombre);
            guardar_exclusivo(&ruta, &pdf)?;
            if fs::read(&ruta).map(|d| huella(&d)).ok().as_deref() != Some(suma.as_str()) {
                let _ = fs::remove_file(&ruta);
                return Err("No se pudo verificar la escritura del PDF.".into());
            }
            Ok(json!({
                "archivo": nombre, "registros": 0, "sha256": suma,
                "tipo": if integral { "PDF_CAPTURA_SITFA" } else { "PDF_PANTALLAZO_SIN_REGISTROS" },
                "contexto_consulta": contexto,
            }))
        })();
        let _ = self.conexion.llamar("evidence_close", json!({}), PLAZO_CIERRE);
        resultado
    }
}

fn contar_extension(carpeta: &Path, extension: &str) -> usize {
    fs::read_dir(carpeta)
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some(extension))
                .count()
        })
        .unwrap_or(0)
}

/// Consultas de un lote incompleto que faltan por descargar (`preferencias.pending_plan`).
#[derive(Debug, PartialEq, Eq)]
pub struct Pendientes {
    pub pantalla: Pantalla,
    pub selecciones: Vec<Seleccion>,
    pub fechas: Option<(NaiveDate, NaiveDate)>,
}

fn sin_fecha_descarga(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(o) = v.as_object_mut() {
        o.remove("fecha_descarga");
    }
    v
}

/// Lee `resumen.json` de un lote y devuelve las consultas del plan que no figuran como completadas. Los filtros
/// privados preparados en SITFA no se guardan: un lote que los usó no se recupera. Las pendientes se descargan desde
/// la página 1 en un lote nuevo.
pub fn pendientes(carpeta: &Path) -> Result<Pendientes, String> {
    let texto = fs::read_to_string(carpeta.join("resumen.json"))
        .map_err(|_| "La carpeta elegida no tiene el resumen.json de un lote.".to_string())?;
    let resumen: Value = serde_json::from_str(&texto).map_err(|_| "El resumen.json del lote no se pudo leer.".to_string())?;
    let plan = resumen["plan"].as_array().filter(|p| !p.is_empty()).ok_or("El lote antiguo no tiene plan estructurado.")?;
    if resumen["otros_filtros"].as_str().is_some_and(|f| f.starts_with("formulario")) {
        return Err("Prepara otra vez los filtros privados; no se guardan para recuperación.".into());
    }
    let hechas: Vec<Value> = resumen["consultas"]
        .as_array()
        .map(|c| c.iter().map(|q| sin_fecha_descarga(&q["seleccion"])).collect())
        .unwrap_or_default();
    let faltan: Vec<&Value> = plan.iter().filter(|s| !hechas.contains(&sin_fecha_descarga(s))).collect();
    let primera = faltan.first().ok_or("No hay consultas pendientes.")?;
    let pantalla: Pantalla = serde_json::from_value(primera["screen"].clone()).map_err(|_| "El plan del lote no indica una pantalla conocida.")?;
    let fecha = |v: &Value| v.as_str().and_then(|t| NaiveDate::parse_from_str(t, "%d/%m/%Y").ok());
    let filtro = &resumen["filtro_fecha"];
    let fechas = fecha(&filtro["start"]).or_else(|| fecha(&filtro["desde"]))
        .zip(fecha(&filtro["end"]).or_else(|| fecha(&filtro["hasta"])));
    let texto = |v: &Value| v.as_str().unwrap_or("").to_owned();
    let mut selecciones = Vec::new();
    for s in faltan {
        if serde_json::from_value::<Pantalla>(s["screen"].clone()).ok() != Some(pantalla) {
            return Err("El plan pendiente mezcla pantallas distintas.".into());
        }
        selecciones.push(Seleccion {
            tribunal: texto(&s["tribunal"]),
            pestana: texto(&s["tab"]),
            modalidad: texto(&s["modality"]),
            reporte: texto(&s["report"]),
            estado: texto(&s["state"]),
            mes: texto(&s["month"]),
            anio: texto(&s["year"]),
            desde: fechas.map(|f| f.0),
            hasta: fechas.map(|f| f.1),
        });
    }
    Ok(Pendientes { pantalla, selecciones, fechas })
}

/// Ejecuta el plan en una carpeta nueva dentro de `destino`. Devuelve las consultas completadas.
#[allow(clippy::too_many_arguments)]
pub fn ejecutar(
    conexion: &dyn Conexion,
    catalogo: &Catalogo,
    edicion: Edicion,
    plan: &PlanLote,
    destino: &Path,
    opciones: &Opciones,
    cancelar: &AtomicBool,
    emitir: &mut dyn FnMut(Evento),
) -> Result<Vec<Value>, String> {
    let integral = edicion == Edicion::Integral;
    if !integral && plan.consultas.iter().any(|p| p.pantalla == Pantalla::Carga) {
        return Err("Carga está disponible en Integral.".into());
    }
    if plan.consultas.is_empty() {
        return Err("El lote no tiene consultas.".into());
    }
    let consultas: Vec<Consulta> = plan.consultas.iter().map(Consulta::desde).collect::<Result<_, _>>()?;
    let carpeta = carpeta_nueva(destino)?;
    // Lite anuncia la carpeta antes del primer resumen; Integral, después (E3B-14).
    if !integral {
        emitir(Evento::Carpeta(carpeta.clone()));
        emitir(Evento::Plan(plan.consultas.len()));
    }
    let lote = Lote { conexion, edicion, catalogo, opciones, cancelar, carpeta: carpeta.clone() };
    let limite = limite_nombre(&carpeta);
    let bases: Vec<String> = plan
        .consultas
        .iter()
        .map(|p| if integral { base_integral(p, catalogo) } else { base_archivo(p, catalogo, plan.fechas, limite) })
        .collect();
    let mut repetidas: HashMap<&str, usize> = HashMap::new();
    for b in &bases {
        *repetidas.entry(b.as_str()).or_default() += 1;
    }
    let usadas: std::cell::RefCell<HashMap<String, usize>> = Default::default();
    let unico = |base: &str, extension: &str| -> String {
        let mut nombre = base.to_string();
        if repetidas.get(base).copied().unwrap_or(0) > 1 {
            let mut usadas = usadas.borrow_mut();
            let n = usadas.entry(base.to_string()).or_default();
            *n += 1;
            nombre.push_str(&format!(" ({n})"));
        }
        nombre + extension
    };
    // Integral: ordinal « N» por base en todo el lote, contado por cada XLS y PDF.
    let ordinal = |base: &str, varias: bool| -> String {
        let mut usadas = usadas.borrow_mut();
        let n = usadas.entry(base.to_string()).or_default();
        *n += 1;
        if varias || repetidas.get(base).copied().unwrap_or(0) > 1 {
            format!("{base} {n}")
        } else {
            base.to_string()
        }
    };
    let planificadas: Vec<Value> = plan
        .consultas
        .iter()
        .map(|p| {
            let mut s = seleccion_edicion(p, edicion);
            s["tribunal_nombre"] = json!(etiqueta(&catalogo.tribunales, &p.tribunal));
            if integral {
                let modalidad = match &p.alcance {
                    Alcance::Seguimiento { modalidad, .. } => modalidad.as_str(),
                    _ => "",
                };
                s["modalidad_nombre"] = json!(MODALIDADES_LITE.iter().find(|(c, _)| *c == modalidad).map_or("", |(_, n)| n));
                s["fecha_descarga"] = json!(ahora());
            } else {
                s["etiquetas"] = json!(partes_consulta(p, catalogo, plan.fechas));
            }
            s
        })
        .collect();
    let mut completadas: Vec<Value> = Vec::new();
    let filtro_fecha = plan.fechas.map(|(a, b)| json!({"start": fecha_sitfa(a), "end": fecha_sitfa(b)}));
    let resumen = |estado: &str, completadas: &[Value]| -> Result<(), String> {
        if integral {
            return escribir_json(
                &carpeta,
                "resumen.json",
                &json!({
                    "version_contrato": 1, "fecha_hora": ahora(), "estado": estado, "plan": planificadas,
                    "consultas_esperadas": plan.consultas.len(), "consultas_completadas": completadas.len(),
                    "consultas": completadas,
                    "filtro_fecha": plan.fechas.map(|(a, b)| json!({"desde": fecha_sitfa(a), "hasta": fecha_sitfa(b)})),
                    "otros_filtros": if opciones.conservar_filtros { "formulario" } else { "todos los registros" },
                }),
            );
        }
        escribir_json(
            &carpeta,
            "resumen.json",
            &json!({
                "version": "lite-1", "fecha_hora": ahora(), "estado": estado,
                "consultas_esperadas": plan.consultas.len(), "consultas_completadas": completadas.len(),
                "filtro_fecha": filtro_fecha,
                "otros_filtros": if opciones.conservar_filtros { "formulario SITFA" } else { "todos los registros" },
                "consultas": completadas, "plan": planificadas,
            }),
        )
    };
    resumen("EN_CURSO", &completadas)?;
    if integral {
        emitir(Evento::Carpeta(carpeta.clone()));
        emitir(Evento::Plan(plan.consultas.len()));
    }
    let mut bloqueado = false;
    let ejecucion = (|| -> Result<(), String> {
        lote.pedir("lock", json!({}))?;
        bloqueado = true;
        for (i, (perfil, consulta)) in plan.consultas.iter().zip(&consultas).enumerate() {
            let indice = i + 1;
            cancelado(cancelar, CANCELADO)?;
            if indice > 1 && !opciones.pausa.is_zero() {
                pausa(opciones.pausa, cancelar, CANCELADO)?;
            }
            let base = &bases[i];
            let caso = caso_y_rotulo(perfil, catalogo);
            // En Integral la consulta se identifica por la etiqueta del perfil; en Lite, por el nombre del archivo.
            let rotulo = if integral { caso.1.clone() } else { base.clone() };
            emitir(Evento::Consulta { indice, total: plan.consultas.len(), etiqueta: rotulo.clone() });
            let fechas = if integral { plan.fechas } else { plan.fechas.filter(|_| !calendario(perfil.pantalla)) };
            let preparado = lote.pedir(
                "prepare",
                json!({
                    "selection": seleccion_edicion(perfil, edicion),
                    "dates": fechas.map(|(a, b)| json!({"start": fecha_sitfa(a), "end": fecha_sitfa(b)})),
                    "keep_filters": opciones.conservar_filtros && (integral || !calendario(perfil.pantalla)),
                }),
            )?;
            let pares: Pares = preparado["pairs"]
                .as_array()
                .filter(|a| a.len() <= 100)
                .and_then(|a| {
                    a.iter()
                        .map(|par| match par.as_array().map(Vec::as_slice) {
                            Some([Value::String(k), Value::String(v)]) => Some((k.clone(), v.clone())),
                            _ => None,
                        })
                        .collect()
                })
                .ok_or("El formulario no entregó parámetros válidos.")?;
            consulta.validar_consulta(&pares)?;
            let campo = |k: &str| pares.iter().find(|(c, _)| c == k).map(|(_, v)| v.as_str());
            // `descargador/lotes.py`: Carga no usa FLG_Consulta ni FEC_Inicio/FEC_Fin; su período va en el objetivo.
            if !calendario(perfil.pantalla) && perfil.pantalla != Pantalla::Carga {
                if campo("FLG_Consulta") != Some(if fechas.is_some() { "1" } else { "0" }) {
                    return Err("No se pudo activar correctamente el filtro de fecha.".into());
                }
                if let Some((a, b)) = fechas {
                    if campo("FEC_Inicio") != Some(fecha_sitfa(a).as_str())
                        || campo("FEC_Fin") != Some(fecha_sitfa(b).as_str())
                        || campo("CHK_Consulta") != Some("on")
                    {
                        return Err("Las fechas enviadas no coinciden con la selección.".into());
                    }
                }
            }
            let html = cuerpo(&lote.pedir("search", json!({"pairs": pares, "timeout": ESPERA_SITFA_MS}))?, false)?;
            let manifiesto = format!("verificacion {indice:03}.json");
            let mut evidencia = None;
            let resultado = match validar_respuesta(&html, &pares, consulta, None)? {
                Respuesta::SinRegistros => {
                    cancelado(cancelar, CANCELADO)?;
                    if opciones.evidencia {
                        let nombre = if integral {
                            ordinal(base, false) + ".pdf"
                        } else {
                            unico(base, " - SIN REGISTROS.pdf")
                        };
                        evidencia = Some(lote.evidencia(perfil, consulta, &pares, fechas, &nombre, emitir)?);
                    }
                    lote.manifiesto(&manifiesto, &caso, &[], 0, "SIN_RESULTADOS")?;
                    match &evidencia {
                        Some(e) => emitir(Evento::Evidencia { archivo: e["archivo"].as_str().unwrap_or("").into() }),
                        None => emitir(Evento::Vacia { consulta: rotulo.clone() }),
                    }
                    Vec::new()
                }
                Respuesta::ConRegistros(primera) => {
                    emitir(Evento::Total(primera.total));
                    let raiz = std::cell::OnceCell::new();
                    let nombre = |numero: usize, total: usize| {
                        if integral {
                            return ordinal(base, total > 1) + ".xls";
                        }
                        let raiz = raiz.get_or_init(|| unico(base, ""));
                        format!("{raiz}{}.xls", if total > 1 { format!(" - p{numero} de {total}") } else { String::new() })
                    };
                    lote.paginas(&pares, primera, consulta, &caso, &manifiesto, &nombre, emitir, &rotulo)?
                }
            };
            let archivos: Vec<Value> = if resultado.is_empty() {
                evidencia.iter().map(|e| e["archivo"].clone()).collect()
            } else {
                resultado.iter().map(|r| r["archivo"].clone()).collect()
            };
            let mut hecha = json!({
                "consulta": rotulo,
                "estado": if resultado.is_empty() { "SIN_RESULTADOS" } else { "VALIDADA" },
                "paginas": resultado.len(),
                "registros": resultado.iter().map(|r| r["registros"].as_u64().unwrap_or(0)).sum::<u64>(),
                "archivos": archivos, "verificacion": manifiesto, "seleccion": planificadas[i],
            });
            if integral {
                if let Some(o) = hecha.as_object_mut() {
                    o.remove("archivos");
                }
                if let Some(e) = &evidencia {
                    hecha["evidencia"] = e.clone();
                }
            }
            let ruta = carpeta.join(&manifiesto);
            let mut info: Value = fs::read(&ruta)
                .ok()
                .and_then(|d| serde_json::from_slice(&d).ok())
                .ok_or("No se pudo releer el manifiesto de la consulta.")?;
            info["seleccion"] = planificadas[i].clone();
            info["filtro_fecha"] = json!(fechas.map(|(a, b)| json!({"start": fecha_sitfa(a), "end": fecha_sitfa(b)})));
            if let Some(e) = &evidencia {
                info["evidencia"] = e.clone();
            } else if resultado.is_empty() {
                info["evidencia_omitida"] = json!(true);
                hecha["evidencia_omitida"] = json!(true);
            }
            escribir_json(&carpeta, &manifiesto, &info)?;
            completadas.push(hecha);
            resumen("EN_CURSO", &completadas)?;
            emitir(Evento::ConsultaLista {
                consulta: rotulo.clone(),
                estado: completadas.last().map(|c| c["estado"].as_str().unwrap_or("").to_string()).unwrap_or_default(),
            });
        }
        Ok(())
    })();
    let ejecucion = ejecucion.and_then(|()| {
        resumen("VALIDADA", &completadas)?;
        emitir(Evento::Fin(Fin {
            carpeta: carpeta.clone(),
            consultas: completadas.len(),
            archivos: contar_extension(&carpeta, "xls"),
            pdfs: contar_extension(&carpeta, "pdf"),
            vacias: completadas.iter().filter(|c| c["estado"] == "SIN_RESULTADOS").count(),
            registros: completadas.iter().map(|c| c["registros"].as_u64().unwrap_or(0) as usize).sum(),
        }));
        Ok(())
    });
    if ejecucion.is_err() {
        let _ = resumen("INCOMPLETA", &completadas);
    }
    if integral {
        // Como el `finally` de Integral: el alcance queda también en la verificación de la consulta interrumpida.
        for (i, s) in planificadas.iter().enumerate() {
            let nombre = format!("verificacion {:03}.json", i + 1);
            let Ok(datos) = fs::read(carpeta.join(&nombre)) else { continue };
            match serde_json::from_slice::<Value>(&datos) {
                Ok(mut info) if info.get("seleccion").is_none() => {
                    info["seleccion"] = s.clone();
                    if escribir_json(&carpeta, &nombre, &info).is_err() {
                        emitir(Evento::Aviso("E_RESUMEN: no se pudo completar el alcance de una verificación.".into()));
                    }
                }
                Ok(_) => {}
                Err(_) => emitir(Evento::Aviso("E_RESUMEN: no se pudo completar el alcance de una verificación.".into())),
            }
        }
    }
    if bloqueado {
        match conexion.llamar("unlock", json!({}), PLAZO_CIERRE) {
            Ok(v) if v["restored"] == json!(false) => emitir(Evento::Aviso(if integral {
                "No se restauraron todos los filtros de SITFA. Revisa el formulario antes de volver a consultar; los archivos verificados se conservan.".into()
            } else {
                "No se restauraron todos los filtros de SITFA. Revisa el formulario.".into()
            })),
            Ok(_) => {}
            Err(_) => emitir(Evento::Aviso(if integral {
                "No se pudo confirmar la restauración de los filtros de SITFA. Revisa la pestaña antes de volver a consultar; los archivos verificados se conservan.".into()
            } else {
                "No se pudo confirmar la restauración de los filtros de SITFA.".into()
            })),
        }
    }
    ejecucion.map(|()| completadas)
}
