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

pub trait Conexion: Send + Sync {
    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String>;
}

impl Conexion for Puente {
    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
        Puente::llamar(self, comando, payload, plazo).map_err(|e| e.mensaje)
    }
}

fn pedir(c: &dyn Conexion, comando: &str, payload: Value) -> Result<Value, String> {
    c.llamar(comando, payload, Duration::from_secs(70))
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
    comprobar_capacidades(&pedir(conexion, "capabilities", json!({}))?)?;
    let catalogo = pedir(conexion, "catalog", json!({"screen":pantalla}))?;
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

pub fn carpeta_nueva(destino: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(destino).map_err(|_| "No se pudo crear la carpeta de destino.")?;
    let nombre = chrono::Local::now().format("Flow Captura %Y-%m-%d %H-%M-%S").to_string();
    for n in 0..1000 {
        let ruta = destino.join(if n == 0 { nombre.clone() } else { format!("{nombre} ({n})") });
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
        let r = Resumen::leer(carpeta)?;
        if r.lote != *lote {
            return Err("El plan de consultas cambió; inicia un lote nuevo.".into());
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
        }
    };
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
            if resumen.terminadas.contains(&indice) {
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
                        let nombre = format!("consulta-{:03}-SIN-REGISTROS.pdf", indice + 1);
                        exclusivo(&ruta_archivo(carpeta, &nombre)?, &pdf)?;
                        resumen.archivos.push(Archivo {
                            nombre,
                            sha256: huella(&pdf),
                            consulta: indice,
                            pagina: 1,
                            total: 1,
                            registros: 0,
                        });
                    }
                    resumen.vacias.push(indice);
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
                        let nombre = format!("consulta-{:03}-p{numero:03}-de-{:03}.xls", indice + 1, primera.total);
                        let datos = if let Some(a) = previo {
                            if a.total != primera.total || a.nombre != nombre {
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
                        if lote.lecturas.activa() && s.screen == "seguimiento" {
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
            if recoleccion.completa(indice) {
                resumen.terminadas.push(indice);
            }
            resumen.guardar(carpeta)?;
        }
        recoleccion.terminar(&mut resumen)?;
        Ok(())
    })();
    match conexion.llamar("unlock", json!({}), Duration::from_secs(8)) {
        Ok(v) if v["restored"].as_bool() != Some(false) => (),
        _ => resumen.avisos.push("No se pudo confirmar la restauración de los filtros de SITFA. Revisa el formulario.".into()),
    }
    resumen.estado = if resultado.is_ok() { "VALIDADA" } else { "INCOMPLETA" }.into();
    resumen.guardar(carpeta)?;
    resultado?;
    Ok(resumen)
}
