//! Lotes secuenciales, checkpoints locales y reanudación con cotejo. El código del puente nunca se persiste.

use crate::{
    evidencia::png_a_pdf,
    sitfa::{siguiente_consulta, validar_pantalla_evidencia, validar_respuesta, Pares, Perfil, Respuesta, Seleccion, VerificadorDescargas},
    Puente,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub const CAPACIDADES: &str = "FLOW_CAPACIDADES";
pub const VERSION_EXTENSION: &str = "0.2.0";
const MAX_RESUMEN: usize = 64 * 1024 * 1024;

/// Plazo por orden a la extensión, como `puente.call(timeout=75)` del Descargador.
pub const PLAZO: Duration = Duration::from_secs(75);

pub trait Conexion: Send + Sync {
    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String>;
    /// Como `llamar`, con el código técnico estable del error (`TIEMPO`, `SITFA_TASK`…), nunca texto de SITFA.
    fn llamar_con_codigo(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, (String, String)> {
        self.llamar(comando, payload, plazo).map_err(|m| ("SITFA_TASK".into(), m))
    }
}

impl Conexion for Puente {
    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
        Puente::llamar(self, comando, payload, plazo).map_err(|e| e.mensaje)
    }
    fn llamar_con_codigo(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, (String, String)> {
        Puente::llamar(self, comando, payload, plazo).map_err(|e| (e.codigo, e.mensaje))
    }
}

fn pedir(c: &dyn Conexion, comando: &str, payload: Value) -> Result<Value, String> {
    c.llamar(comando, payload, PLAZO)
}

/// Aviso al desbloquear, separado como en `descargador/lotes.py`, con un código técnico sin datos de personas.
pub fn aviso_restauracion(respuesta: &Result<Value, (String, String)>) -> Option<String> {
    let codigo = match respuesta {
        Ok(v) if v["restored"].as_bool() == Some(false) => {
            return Some(
                "No se restauraron todos los filtros de SITFA. Revisa el formulario antes de volver a consultar; los archivos \
                 verificados se conservan. (Código técnico: restored:false.)"
                    .into(),
            )
        }
        Ok(_) => return None,
        Err((codigo, _)) if codigo == "TIEMPO" => "tiempo".to_string(),
        Err((codigo, _)) => codigo.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(40).collect(),
    };
    Some(format!(
        "No se pudo confirmar la restauración de los filtros de SITFA. Revisa la pestaña antes de volver a consultar; los \
         archivos verificados se conservan. (Código técnico: {codigo}.)"
    ))
}

pub fn comprobar_capacidades(v: &Value) -> Result<(), String> {
    if *v
        != json!({"tipo":CAPACIDADES,"version":VERSION_EXTENSION,"protocolos":{"descarga_xls":1,"pdf_sin_registros":1,"bitacoras_lectura":1,"historia_lectura":1,"calendario_lectura":1},"escritura_rus":false})
    {
        return Err(
            "La extensión no corresponde a esta versión de Flow o no garantiza consultas de solo lectura. Carga la carpeta extension de esta entrega y vuelve a conectar.".into()
        );
    }
    Ok(())
}

pub fn catalogo(conexion: &dyn Conexion, pantalla: &str) -> Result<Value, String> {
    if !matches!(pantalla, "seguimiento" | "litigantes" | "calendario_informes" | "calendario_medidas") {
        return Err("Pantalla no permitida.".into());
    }
    // Como el Descargador: primero las opciones, que abren la pantalla elegida si la pestaña está en otra y dan un
    // mensaje claro si SITFA no muestra una pantalla de consulta; después, la comprobación de la extensión.
    // Mismo plazo que el Descargador (75 s): con SITFA lento, abrir la pantalla elegida puede tardar.
    let catalogo = conexion.llamar("catalog", json!({"screen":pantalla}), PLAZO)?;
    let capacidades = conexion.llamar("capabilities", json!({}), Duration::from_secs(15)).map_err(|_| {
        "La extensión respondió las opciones, pero no confirmó su versión. Carga la carpeta extension de esta entrega de Flow en chrome://extensions y vuelve a conectar.".to_string()
    })?;
    comprobar_capacidades(&capacidades)?;
    if catalogo["pantalla"].as_str() != Some(pantalla) || !catalogo["tribunales"].is_object() {
        return Err("La extensión no devolvió las opciones de la pantalla elegida.".into());
    }
    Ok(catalogo)
}

pub fn cuerpo(respuesta: &Value, excel: bool) -> Result<Vec<u8>, String> {
    let error = || "Respuesta vacía, vencida o fuera del formato permitido.".to_string();
    let texto = respuesta["body"].as_str().ok_or_else(error)?;
    if respuesta["status"].as_u64() != Some(200)
        || texto.len() > 4 * (80 * 1024 * 1024_usize).div_ceil(3)
        || !excel && !respuesta["type"].as_str().is_some_and(|t| t.to_ascii_lowercase().contains("html"))
    {
        return Err(error());
    }
    let datos = STANDARD.decode(texto).map_err(|_| error())?;
    if datos.is_empty() || datos.len() > 80 * 1024 * 1024 {
        return Err(error());
    }
    Ok(datos)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fechas {
    pub start: String,
    pub end: String,
}

impl Fechas {
    fn validar(&self) -> Result<(), String> {
        let dia = |s: &str| chrono::NaiveDate::parse_from_str(s, "%d/%m/%Y").ok().filter(|f| f.format("%d/%m/%Y").to_string() == s);
        match (dia(&self.start), dia(&self.end)) {
            (Some(a), Some(b))
                if a <= b
                    && a >= chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap()
                    && b <= chrono::NaiveDate::from_ymd_opt(2100, 12, 31).unwrap() =>
            {
                Ok(())
            }
            _ => Err("Usa fechas válidas DD/MM/AAAA, con Desde anterior o igual a Hasta.".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lote {
    pub consultas: Vec<Seleccion>,
    pub fechas: Option<Fechas>,
    pub conservar_filtros: bool,
    pub evidencia: bool,
    pub pausa_ms: u64,
    #[serde(default)]
    pub lecturas: crate::recoleccion::Opciones,
}

impl Lote {
    pub fn validar(&self, catalogo: &Value) -> Result<(), String> {
        if self.consultas.is_empty() || self.consultas.len() > 600 {
            return Err("Selecciona de 1 a 600 consultas por lote.".into());
        }
        if self.pausa_ms < 250 || self.pausa_ms > 60_000 {
            return Err("La pausa debe ser de 250 a 60000 ms.".into());
        }
        self.lecturas.validar()?;
        if let Some(f) = &self.fechas {
            f.validar()?;
        }
        for (i, s) in self.consultas.iter().enumerate() {
            Perfil::desde_catalogo(s, catalogo)?;
            if self.consultas[..i].contains(s) {
                return Err("El lote tiene consultas repetidas.".into());
            }
            if s.screen.starts_with("calendario_") && self.fechas.is_some() {
                return Err("El calendario usa mes y año, sin rango de fechas.".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archivo {
    pub nombre: String,
    pub sha256: String,
    pub consulta: usize,
    pub pagina: usize,
    pub total: usize,
    pub registros: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resumen {
    pub version: u32,
    pub lote: Lote,
    pub estado: String,
    pub terminadas: Vec<usize>,
    pub vacias: Vec<usize>,
    pub archivos: Vec<Archivo>,
    pub avisos: Vec<String>,
    #[serde(default)]
    pub paginas_lectura: Vec<crate::recoleccion::Pagina>,
    #[serde(default)]
    pub lecturas: Vec<crate::recoleccion::Referencia>,
    /// Inicio del lote (RFC 3339, segundos enteros): `fecha_hora` del resumen del Descargador. Vacío en lotes previos.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub inicio: String,
    /// Nombre visible de cada tribunal del lote según el catálogo de SITFA (`tribunal_nombre` de la selección).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub tribunales: std::collections::BTreeMap<String, String>,
}

/// Instante local con segundos enteros y zona, como lo acepta `CSMP_CAPTURE_V1`.
pub fn ahora() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

impl Resumen {
    pub fn leer(carpeta: &Path) -> Result<Self, String> {
        let mut datos = vec![];
        fs::File::open(carpeta.join("resumen.json"))
            .map_err(|_| "No se pudo leer el avance del lote.")?
            .take(MAX_RESUMEN as u64 + 1)
            .read_to_end(&mut datos)
            .map_err(|_| "No se pudo leer el avance del lote.")?;
        if datos.len() > MAX_RESUMEN {
            return Err("El avance del lote es demasiado grande.".into());
        }
        let r: Self = serde_json::from_slice(&datos).map_err(|_| "El avance del lote no se reconoce.")?;
        if r.version != 1
            || r.lote.consultas.is_empty()
            || r.archivos.iter().any(|a| {
                ruta_archivo(carpeta, &a.nombre).is_err()
                    || a.consulta >= r.lote.consultas.len()
                    || a.pagina == 0
                    || a.total > 500
                    || a.pagina > a.total
                    || a.sha256.len() != 64
                    || !a.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            })
            || r.terminadas.iter().chain(&r.vacias).any(|i| *i >= r.lote.consultas.len())
            || r.lecturas.iter().any(|l| {
                ruta_archivo(carpeta, &l.archivo).is_err() || l.sha256.len() != 64 || !l.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            })
            || r.paginas_lectura.iter().any(|p| p.consulta >= r.lote.consultas.len() || p.numero == 0 || p.numero > 500)
        {
            return Err("El avance del lote contiene referencias inválidas.".into());
        }
        Ok(r)
    }
    pub fn guardar(&self, carpeta: &Path) -> Result<(), String> {
        let temporal = carpeta.join("resumen.tmp");
        let datos = serde_json::to_vec_pretty(self).map_err(|_| "No se pudo preparar el avance del lote.")?;
        if datos.len() > MAX_RESUMEN {
            return Err("El avance excede el tamaño permitido; se conserva el checkpoint anterior.".into());
        }
        let mut archivo = fs::File::create(&temporal).map_err(|_| "No se pudo guardar el avance del lote.")?;
        archivo.write_all(&datos).and_then(|_| archivo.sync_all()).map_err(|_| "No se pudo guardar el avance del lote.")?;
        fs::rename(&temporal, carpeta.join("resumen.json")).map_err(|_| "No se pudo completar el guardado del avance del lote.".into())
    }
}

pub fn ruta_archivo(carpeta: &Path, nombre: &str) -> Result<PathBuf, String> {
    if nombre.is_empty()
        || nombre.contains(['/', '\\', ':'])
        || nombre == "."
        || nombre == ".."
        || Path::new(nombre).file_name().and_then(|s| s.to_str()) != Some(nombre)
    {
        return Err("El avance contiene un nombre de archivo no permitido.".into());
    }
    Ok(carpeta.join(nombre))
}

/// Modalidades auditadas del Descargador (`lotes.MODALITIES`).
const MODALIDADES: [(&str, &str); 4] = [("1", "Residencia"), ("2", "Ambulatorio"), ("3", "FAE"), ("4", "DCE")];

/// `lotes.safe_name`: sin tildes, sin caracteres prohibidos en Windows ni puntos, espacios simples y 125 caracteres.
pub fn nombre_seguro(valor: &str) -> String {
    use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};
    let limpio: String = valor
        .nfkd()
        .filter(|c| !is_combining_mark(*c))
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '.') || (c as u32) < 0x20 { ' ' } else { c })
        .collect();
    let unido = limpio.split_whitespace().collect::<Vec<_>>().join(" ");
    unido.trim_end_matches([' ', '.']).chars().take(125).collect()
}

/// `lotes.tribunal_name`: abreviaturas de los juzgados.
pub fn nombre_tribunal(valor: &str) -> String {
    static REGLAS: std::sync::LazyLock<[(regex::Regex, &str); 3]> = std::sync::LazyLock::new(|| {
        [
            (regex::Regex::new(r"(?i)^Juzgado de Familia\s+").unwrap(), "Jdo Familia "),
            (regex::Regex::new(r"(?i)^Jgdo\s+L\s+y\s+G\s+de\s+").unwrap(), "Jdo Letras y Garantia "),
            (regex::Regex::new(r"(?i)^Jgdo\s+(?:Letras|L)\s+de\s+").unwrap(), "Jdo Letras "),
        ]
    });
    let mut v = nombre_seguro(valor);
    for (re, reemplazo) in REGLAS.iter() {
        v = re.replacen(&v, 1, *reemplazo).into_owned();
    }
    v
}

/// `lotes.file_base`: «ESP|CUMP <Modalidad> <Tribunal>», «Informes <Tribunal>», «Egresados <Tribunal>»,
/// «Litigantes <estado> <tribunal>» o «Medidas por vencer <Tribunal>». Nunca un nombre reservado de Windows.
pub fn base_archivo(s: &Seleccion, catalogo: &Value) -> String {
    let etiqueta = |grupo: &str, valor: &str| catalogo[grupo][valor].as_str().unwrap_or(valor).to_string();
    let tribunal = nombre_tribunal(&etiqueta("tribunales", &s.tribunal));
    let base = match (s.screen.as_str(), s.tab.as_str()) {
        ("calendario_informes", _) | (_, "Informes") => nombre_seguro(&format!("Informes {tribunal}")),
        ("calendario_medidas", _) => nombre_seguro(&format!("Medidas por vencer {tribunal}")),
        ("litigantes", _) => nombre_seguro(&format!("Litigantes {} {tribunal}", etiqueta("estados", s.state.as_deref().unwrap_or("1")))),
        (_, "Egresados") => nombre_seguro(&format!("Egresados {tribunal}")),
        (_, pestana) => {
            let modalidad = MODALIDADES.iter().find(|(c, _)| *c == s.modality).map_or(s.modality.as_str(), |(_, n)| n);
            nombre_seguro(&format!("{} {modalidad} {tribunal}", if pestana == "Espera" { "ESP" } else { "CUMP" }))
        }
    };
    let raiz = base.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    let reservado = matches!(raiz.as_str(), "" | "CON" | "PRN" | "AUX" | "NUL")
        || (raiz.len() == 4 && (raiz.starts_with("COM") || raiz.starts_with("LPT")) && raiz.as_bytes()[3].is_ascii_digit());
    if reservado {
        format!("Consulta {base}").trim().to_string()
    } else {
        base
    }
}

/// Clave de una base para contar repeticiones: Windows no distingue mayúsculas.
fn clave_base(base: &str) -> String {
    base.to_lowercase()
}

/// Nombre del siguiente archivo de la consulta `indice`, como `BatchRunner.run`: la base con un ordinal ` N` (contado
/// en todo el lote por base) si la consulta tiene varias páginas o la base se repite en el plan. El ordinal sale de
/// los archivos ya registrados en el avance, que se escriben en el mismo orden; sus nombres nunca se deducen.
fn nombre_siguiente(resumen: &Resumen, catalogo: &Value, indice: usize, paginas: usize, extension: &str) -> String {
    let clave = |i: usize| clave_base(&base_archivo(&resumen.lote.consultas[i], catalogo));
    let base = base_archivo(&resumen.lote.consultas[indice], catalogo);
    let propia = clave_base(&base);
    let repetida = (0..resumen.lote.consultas.len()).filter(|i| clave(*i) == propia).count() > 1;
    let ordinal = resumen.archivos.iter().filter(|a| a.consulta < resumen.lote.consultas.len() && clave(a.consulta) == propia).count() + 1;
    let usado = |n: &str| resumen.archivos.iter().any(|a| a.nombre.to_lowercase() == n.to_lowercase());
    let mut nombre = if paginas > 1 || repetida { format!("{base} {ordinal}.{extension}") } else { format!("{base}.{extension}") };
    // Una base que termina en número («X 1») puede dar el nombre de otra con ordinal («X» página 1): se sigue contando.
    let mut siguiente = ordinal;
    while usado(&nombre) {
        siguiente += 1;
        nombre = format!("{base} {siguiente}.{extension}");
    }
    nombre
}

pub fn huella(datos: &[u8]) -> String {
    format!("{:x}", Sha256::digest(datos))
}

pub(crate) fn exclusivo(ruta: &Path, datos: &[u8]) -> Result<(), String> {
    let mut archivo = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ruta)
        .map_err(|_| "No se pudo crear el archivo local. Revisa permisos, espacio y que no exista otro con el mismo nombre.")?;
    if archivo.write_all(datos).and_then(|_| archivo.sync_all()).is_err() {
        drop(archivo);
        let _ = fs::remove_file(ruta);
        return Err("Falló la escritura local; no se conserva un archivo incompleto.".into());
    }
    Ok(())
}

/// Carpeta del lote con la fecha y hora, como `BatchRunner.run` (`%Y-%m-%d %H-%M-%S`; si coincide el segundo, con
/// microsegundos). Los lotes anteriores («Flow Captura …») se siguen leyendo igual.
pub fn carpeta_nueva(destino: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(destino).map_err(|_| "No se pudo crear la carpeta de destino.")?;
    let ahora = chrono::Local::now();
    let nombre = ahora.format("%Y-%m-%d %H-%M-%S").to_string();
    let preciso = ahora.format("%Y-%m-%d %H-%M-%S-%6f").to_string();
    for n in 0..1000 {
        let ruta = destino.join(match n {
            0 => nombre.clone(),
            1 => preciso.clone(),
            _ => format!("{preciso} ({n})"),
        });
        match fs::create_dir(&ruta) {
            Ok(()) => return Ok(ruta),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err("No se pudo crear la carpeta del lote.".into()),
        }
    }
    Err("No se encontró un nombre libre para el lote.".into())
}

fn cancelado(cancelar: &AtomicBool) -> Result<(), String> {
    if cancelar.load(Ordering::SeqCst) {
        Err("Lote cancelado. Se conservan los archivos verificados; puedes retomar.".into())
    } else {
        Ok(())
    }
}

pub(crate) fn pausa(ms: u64, cancelar: &AtomicBool) -> Result<(), String> {
    for _ in 0..ms.div_ceil(50) {
        cancelado(cancelar)?;
        std::thread::sleep(Duration::from_millis(50));
    }
    cancelado(cancelar)
}

fn contexto(s: &Seleccion, catalogo: &Value, fechas: Option<&Fechas>) -> Vec<[String; 2]> {
    let etiqueta = |grupo: &str, valor: &str| catalogo[grupo][valor].as_str().unwrap_or(valor).to_string();
    let mut c = vec![
        ["Tribunal".into(), format!("{} (código {})", etiqueta("tribunales", &s.tribunal), s.tribunal)],
        ["Consulta".into(), s.screen.clone()],
    ];
    if s.screen == "seguimiento" {
        c.push(["Pestaña".into(), s.tab.clone()]);
        c.push(["Modalidad".into(), etiqueta("modalidades", &s.modality)]);
        if let Some(r) = &s.report {
            c.push(["Filtro".into(), etiqueta(if s.tab == "Informes" { "informes" } else { "medidas" }, r)]);
        }
    } else if s.screen == "litigantes" {
        c.push(["Estado".into(), etiqueta("estados", s.state.as_deref().unwrap_or("1"))]);
    } else {
        c.push(["Mes".into(), format!("{} {}", etiqueta("meses", s.month.as_deref().unwrap_or("")), s.year.as_deref().unwrap_or(""))]);
    }
    if !s.screen.starts_with("calendario_") {
        c.push(["Fechas".into(), fechas.map(|f| format!("{} – {}", f.start, f.end)).unwrap_or("Sin filtro de fechas".into())]);
    }
    for par in &mut c {
        par[1] = par[1].chars().take(500).collect();
    }
    c
}

fn evidencia(conexion: &dyn Conexion, pares: &Pares, perfil: &Perfil, contexto: &[[String; 2]]) -> Result<Vec<u8>, String> {
    let resultado: Result<Vec<u8>, String> = (|| {
        let vista = pedir(conexion, "evidence_open", json!({"pairs":pares,"label":"Consulta sin registros","context":contexto}))?;
        let html = cuerpo(&vista, false)?;
        let digest = huella(&html);
        if vista["digest"].as_str() != Some(&digest) {
            return Err("La pantalla de evidencia no coincide con la respuesta recibida.".into());
        }
        validar_pantalla_evidencia(&html, pares, perfil, contexto)?;
        png_a_pdf(&pedir(conexion, "evidence_capture", json!({"digest":digest}))?, &digest)
    })();
    let _ = pedir(conexion, "evidence_close", json!({}));
    resultado
}

/// Nuevo lote o carpeta con checkpoint a retomar. No descarga nada que no haya pasado la negociación de lectura.
pub fn ejecutar(
    conexion: &dyn Conexion,
    catalogo: &Value,
    lote: &Lote,
    carpeta: &Path,
    retomar: bool,
    cancelar: &AtomicBool,
    progreso: &dyn Fn(String),
) -> Result<Resumen, String> {
    lote.validar(catalogo)?;
    comprobar_capacidades(&pedir(conexion, "capabilities", json!({}))?)?;
    let mut resumen = if retomar {
        let mut r = Resumen::leer(carpeta)?;
        if r.lote != *lote {
            return Err("El plan de consultas cambió; inicia un lote nuevo.".into());
        }
        // E3B-12: una lectura desaparecida o alterada ya no bloquea el retomar; su ingreso se vuelve a leer.
        for aviso in crate::recoleccion::reparar_lecturas(carpeta, &mut r) {
            progreso(aviso);
        }
        r
    } else {
        if carpeta.join("resumen.json").exists() {
            return Err("La carpeta ya tiene un lote; usa Retomar.".into());
        }
        Resumen {
            version: 1,
            lote: lote.clone(),
            estado: "EN_CURSO".into(),
            terminadas: vec![],
            vacias: vec![],
            archivos: vec![],
            avisos: vec![],
            paginas_lectura: vec![],
            lecturas: vec![],
            inicio: ahora(),
            tribunales: Default::default(),
        }
    };
    for s in &lote.consultas {
        if let Some(nombre) = catalogo["tribunales"][&s.tribunal].as_str() {
            resumen.tribunales.entry(s.tribunal.clone()).or_insert_with(|| nombre.to_string());
        }
    }
    // Ningún archivo desaparecido o editado se da por completado al reanudar.
    for a in &resumen.archivos {
        let datos =
            fs::read(ruta_archivo(carpeta, &a.nombre)?).map_err(|_| "Falta un archivo verificado del lote; inicia una ejecución nueva.")?;
        if huella(&datos) != a.sha256 {
            return Err("Cambió un archivo verificado del lote; inicia una ejecución nueva.".into());
        }
    }
    resumen.estado = "EN_CURSO".into();
    let mut recoleccion = crate::recoleccion::Recoleccion::nueva(carpeta, &resumen)?;
    resumen.guardar(carpeta)?;
    if let Err(e) = pedir(conexion, "lock", json!({})) {
        resumen.estado = "INCOMPLETA".into();
        resumen.guardar(carpeta)?;
        return Err(e);
    }
    let resultado: Result<(), String> = (|| {
        for (indice, s) in lote.consultas.iter().enumerate() {
            cancelado(cancelar)?;
            // Una consulta terminada con lecturas pendientes se recorre otra vez: sus XLS se releen del disco y solo
            // se reintentan las lecturas fallidas.
            if resumen.terminadas.contains(&indice) && !crate::recoleccion::consultas_pendientes(&resumen).contains(&indice) {
                continue;
            }
            progreso(format!("Consulta {} de {} · {}", indice + 1, lote.consultas.len(), s.tab));
            let perfil = Perfil::desde_catalogo(s, catalogo)?;
            let preparado = pedir(conexion, "prepare", json!({"selection":s,"dates":lote.fechas,"keep_filters":lote.conservar_filtros}))?;
            let pares: Pares =
                serde_json::from_value(preparado["pairs"].clone()).map_err(|_| "El formulario no entregó parámetros válidos.")?;
            perfil.validar_consulta(&pares)?;
            let fields: std::collections::BTreeMap<_, _> = pares.iter().cloned().collect();
            if !s.screen.starts_with("calendario_")
                && (fields.get("FLG_Consulta").map(String::as_str) != Some(if lote.fechas.is_some() { "1" } else { "0" })
                    || lote.fechas.as_ref().is_some_and(|f| {
                        fields.get("FEC_Inicio") != Some(&f.start)
                            || fields.get("FEC_Fin") != Some(&f.end)
                            || fields.get("CHK_Consulta").map(String::as_str) != Some("on")
                    }))
            {
                return Err("Las fechas enviadas no coinciden con la selección.".into());
            }
            pausa(lote.pausa_ms, cancelar)?;
            let html = cuerpo(&pedir(conexion, "search", json!({"pairs":pares,"timeout":60000}))?, false)?;
            let respuesta = validar_respuesta(&html, &pares, &perfil, None)?;
            match respuesta {
                Respuesta::SinRegistros => {
                    if resumen.archivos.iter().any(|a| a.consulta == indice && a.registros != 0) {
                        return Err("La consulta cambió desde el avance guardado; inicia un lote nuevo.".into());
                    }
                    if lote.evidencia && !resumen.archivos.iter().any(|a| a.consulta == indice && a.nombre.ends_with(".pdf")) {
                        cancelado(cancelar)?;
                        let pdf = evidencia(conexion, &pares, &perfil, &contexto(s, catalogo, lote.fechas.as_ref()))?;
                        let primero = nombre_siguiente(&resumen, catalogo, indice, 1, "pdf");
                        let base = primero.strip_suffix(".pdf").unwrap_or(&primero).to_string();
                        let mut nombre = primero.clone();
                        let mut recuperadas = 0;
                        // La evidencia huérfana no tiene un checkpoint que acredite su consulta. Se conserva
                        // y se crea otra copia verificada con nombre libre, sin quedar bloqueados al retomar.
                        while ruta_archivo(carpeta, &nombre)?.try_exists().map_err(|_| "No se pudo comprobar la evidencia local.")? {
                            recuperadas += 1;
                            if recuperadas > 100 {
                                return Err("Demasiadas evidencias sin checkpoint; elija otro lote.".into());
                            }
                            nombre = format!("{base} recuperada {recuperadas}.pdf");
                        }
                        exclusivo(&ruta_archivo(carpeta, &nombre)?, &pdf)?;
                        if recuperadas > 0 {
                            resumen.avisos.push(format!(
                                "Consulta {}: se creó una evidencia nueva; la copia anterior sin checkpoint se conservó.",
                                indice + 1
                            ));
                        }
                        resumen.archivos.push(Archivo {
                            nombre,
                            sha256: huella(&pdf),
                            consulta: indice,
                            pagina: 1,
                            total: 1,
                            registros: 0,
                        });
                        resumen.guardar(carpeta)?;
                    }
                    if !resumen.vacias.contains(&indice) {
                        resumen.vacias.push(indice);
                    }
                }
                Respuesta::ConRegistros(primera) => {
                    let mut verificador = VerificadorDescargas::default();
                    for numero in 1..=primera.total {
                        cancelado(cancelar)?;
                        let (pagina, html_pagina) = if numero == 1 {
                            (primera.clone(), html.clone())
                        } else {
                            pausa(lote.pausa_ms, cancelar)?;
                            let siguientes = siguiente_consulta(&pares, &perfil, &primera, numero)?;
                            let html = cuerpo(&pedir(conexion, "search", json!({"pairs":siguientes,"timeout":60000}))?, false)?;
                            let Respuesta::ConRegistros(p) = validar_respuesta(&html, &siguientes, &perfil, Some((numero, &primera)))?
                            else {
                                return Err("La consulta cambió durante la descarga.".into());
                            };
                            (p, html)
                        };
                        let previo = resumen.archivos.iter().find(|a| a.consulta == indice && a.pagina == numero);
                        // Al retomar, el nombre de una página ya verificada es el del avance (también en lotes con los
                        // nombres anteriores «consulta-NNN-pNNN-de-NNN.xls»).
                        let nombre = match previo {
                            Some(a) => a.nombre.clone(),
                            None => nombre_siguiente(&resumen, catalogo, indice, primera.total, "xls"),
                        };
                        let datos = if let Some(a) = previo {
                            if a.total != primera.total {
                                return Err("Cambió el total de páginas del avance guardado.".into());
                            }
                            fs::read(ruta_archivo(carpeta, &a.nombre)?).map_err(|_| "No se pudo releer una descarga verificada.")?
                        } else {
                            cancelado(cancelar)?;
                            cuerpo(&pedir(conexion, "download", json!({"url":pagina.excel_url,"timeout":60000}))?, true)?
                        };
                        let descarga = verificador.verificar(datos.clone(), &pagina, &perfil)?;
                        if previo.is_none() {
                            let ruta = ruta_archivo(carpeta, &nombre)?;
                            // Si el proceso terminó después de escribir y antes del checkpoint, se coteja el archivo huérfano.
                            if ruta.exists() {
                                if fs::read(&ruta).map(|d| huella(&d)).ok() != Some(descarga.libro.sha256.clone()) {
                                    return Err("Existe un archivo distinto sin checkpoint; inicia otro lote.".into());
                                }
                            } else {
                                exclusivo(&ruta, &datos)?;
                            }
                            resumen.archivos.push(Archivo {
                                nombre,
                                sha256: descarga.libro.sha256,
                                consulta: indice,
                                pagina: numero,
                                total: primera.total,
                                registros: pagina.registros.values().sum(),
                            });
                            resumen.guardar(carpeta)?;
                        }
                        progreso(format!(
                            "Consulta {} de {} · página {numero} de {} verificada",
                            indice + 1,
                            lote.consultas.len(),
                            primera.total
                        ));
                        // Como `motor.search_page`: en Seguimiento los vínculos del listado se guardan siempre, aunque
                        // no se pidan lecturas por ingreso.
                        if s.screen == "seguimiento" {
                            recoleccion.pagina(
                                conexion,
                                catalogo,
                                &mut resumen,
                                crate::recoleccion::PaginaActual { consulta: indice, numero, html: &html_pagina },
                                crate::recoleccion::Control { cancelar, progreso },
                            )?;
                        }
                    }
                }
            }
            if !resumen.terminadas.contains(&indice) {
                resumen.terminadas.push(indice);
            }
            resumen.guardar(carpeta)?;
        }
        recoleccion.terminar(&mut resumen)?;
        Ok(())
    })();
    if let Some(aviso) = aviso_restauracion(&conexion.llamar_con_codigo("unlock", json!({}), Duration::from_secs(8))) {
        resumen.avisos.push(aviso);
    }
    resumen.estado = if resultado.is_ok() { "VALIDADA" } else { "INCOMPLETA" }.into();
    if let Err(e) = &resultado {
        // El motivo queda en el avance del lote, no solo en pantalla. Todos los mensajes de este módulo son textos
        // fijos del programa: no incluyen valores de SITFA ni datos de personas.
        resumen.avisos.push(format!("Lote detenido: {e}"));
    }
    resumen.guardar(carpeta)?;
    resultado?;
    Ok(resumen)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn nombre_ya_usado_por_otra_base_no_choca() {
        // Tribunal «Ficticio 1» (base «… Ficticio 1», sola) y tribunal «Ficticio» con dos páginas («… Ficticio 1/2»).
        let catalogo = json!({"tribunales": {"901": "Juzgado de Familia Ficticio 1", "902": "Juzgado de Familia Ficticio"}});
        let s = |t: &str| Seleccion {
            tribunal: t.into(),
            screen: "seguimiento".into(),
            tab: "Espera".into(),
            modality: "3".into(),
            report: None,
            state: None,
            month: None,
            year: None,
        };
        let mut r: Resumen = serde_json::from_value(json!({"version":1,"lote":{"consultas":[s("901"), s("902")],"fechas":null,
            "conservar_filtros":false,"evidencia":false,"pausa_ms":250},"estado":"EN_CURSO","terminadas":[0],"vacias":[],
            "archivos":[{"nombre":"ESP FAE Jdo Familia Ficticio 1.xls","sha256":"0".repeat(64),"consulta":0,"pagina":1,"total":1,"registros":1}],
            "avisos":[]}))
        .unwrap();
        assert_eq!(nombre_siguiente(&r, &catalogo, 1, 2, "xls"), "ESP FAE Jdo Familia Ficticio 2.xls");
        r.archivos.clear();
        assert_eq!(nombre_siguiente(&r, &catalogo, 1, 2, "xls"), "ESP FAE Jdo Familia Ficticio 1.xls");
    }
}
