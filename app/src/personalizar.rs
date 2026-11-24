//! Ajustes que se editan con cuidado (`personalization.py`, `template_validation.py` y `template_package.py` de Flow
//! Python): contactos con alias sin colisiones, plantillas de correo (duplicar, archivar, restaurar), la gramática de
//! variables de una plantilla y la importación de matrices Word. Cada operación devuelve una configuración nueva o un
//! error; nunca deja una configuración a medias.

use crate::config::{plantillas_base, Config, PlantillaCorreo};
use crate::util::normalizar;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Variables que admiten el asunto y el cuerpo de un correo (`config.VARIABLES`).
pub const VARIABLES: [&str; 6] = ["TRIBUNAL", "ALCANCE_MODALIDADES", "PERIODO", "FECHA", "PROGRAMA", "TABLA_REGISTROS"];
/// Comportamientos que una copia de plantilla puede heredar (`mail_category.CATEGORIES`).
pub const CATEGORIAS: [&str; 8] =
    ["espera", "cumplimiento", "informes", "medidas", "proyectos", "programa_espera", "programa_vencido", "programa_por_vencer"];

/// `config.emails`: direcciones separadas por `;`, `,` o salto de línea, sin repetidas y todas válidas.
pub fn correos(texto: &str) -> Result<Vec<String>, String> {
    let valida = Regex::new(r"^[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+$").unwrap();
    let mut v: Vec<String> = Vec::new();
    for parte in texto.split([';', ',', '\n']) {
        let parte = parte.trim();
        if !parte.is_empty() && !v.iter().any(|x| x.to_lowercase() == parte.to_lowercase()) {
            if !valida.is_match(parte) {
                return Err(format!("Dirección inválida: {parte}"));
            }
            v.push(parte.to_string());
        }
    }
    Ok(v)
}

/// `save_contact`: guarda un contacto (o los destinatarios de un tribunal) con sus alias, sin pisar otro contacto.
pub fn guardar_contacto(cfg: &Config, nombre: &str, direccion: &str, alias: &[String], previo: Option<&str>) -> Result<Config, String> {
    let mut r = cfg.clone();
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err("Indica un nombre para el contacto.".into());
    }
    let direccion = correos(direccion)?.join("; ");
    if let Some(clave) = nombre.strip_prefix("Tribunal: ") {
        let t = r.tribunales.get_mut(clave).ok_or("Tribunal no reconocido.")?;
        t.para = direccion;
        return Ok(r);
    }
    if previo.is_some_and(|p| p.starts_with("Tribunal: ")) {
        return Err("La clave del tribunal se conserva; selecciona Nuevo para crear un programa.".into());
    }
    let n = normalizar(nombre);
    if r.aliases.iter().any(|(a, destino)| normalizar(a) == n && Some(destino.as_str()) != previo) {
        return Err(format!("El nombre ya es alias de otro contacto: {nombre}"));
    }
    let otros: BTreeSet<String> = r.contactos.keys().filter(|k| Some(k.as_str()) != previo).map(|k| normalizar(k)).collect();
    if otros.contains(&n) && Some(nombre) != previo {
        return Err("Ya existe otro contacto con ese nombre. No se reemplazó.".into());
    }
    let mut limpios: Vec<String> = Vec::new();
    for a in alias.iter().map(|a| a.trim()).filter(|a| !a.is_empty()) {
        if !limpios.iter().any(|x| x == a) {
            limpios.push(a.to_string());
        }
    }
    let propios: BTreeSet<String> = r.aliases.iter().filter(|(_, d)| Some(d.as_str()) == previo).map(|(a, _)| a.clone()).collect();
    for a in &limpios {
        if otros.contains(&normalizar(a)) {
            return Err(format!("El alias coincide con otro contacto: {a}"));
        }
        if r.aliases.iter().any(|(k, destino)| normalizar(k) == normalizar(a) && !propios.contains(k) && normalizar(destino) != n) {
            return Err(format!("El alias ya pertenece a otro contacto: {a}"));
        }
    }
    if let Some(p) = previo {
        r.contactos.remove(p);
    }
    r.aliases.retain(|k, _| !propios.contains(k));
    r.contactos.insert(nombre.to_string(), direccion);
    for a in limpios {
        r.aliases.insert(a, nombre.to_string());
    }
    Ok(r)
}

/// Resultado de importar un catastro: los contactos que quedan y los nombres que no se sustituyeron.
#[derive(Debug, PartialEq)]
pub struct Catastro {
    pub contactos: BTreeMap<String, String>,
    pub conflictos: Vec<String>,
}

/// `outputs.import_contacts`: filas de un catastro (la primera es el encabezado) con columnas Nombre/Programa y
/// Mail/Correo. Un nombre que ya existe con otro correo no se sustituye y queda como conflicto; una dirección inválida
/// detiene toda la importación. Como en Python, un nombre que solo cambia de mayúsculas se agrega como clave aparte.
pub fn importar_catastro(previos: &BTreeMap<String, String>, filas: &[Vec<String>]) -> Result<Catastro, String> {
    const FALTAN_COLUMNAS: &str = "El catastro necesita columnas Nombre/Programa y Mail/Correo.";
    let cabecera = filas.first().ok_or(FALTAN_COLUMNAS)?;
    let columna = |claves: &[&str]| claves.iter().find_map(|k| cabecera.iter().position(|c| normalizar(c) == *k));
    let nombre = columna(&["nombre", "programa", "nombre programa"]).ok_or(FALTAN_COLUMNAS)?;
    let correo = columna(&["mail", "correo", "email", "correo electronico"]).ok_or(FALTAN_COLUMNAS)?;
    let mut contactos = previos.clone();
    let mut conflictos = Vec::new();
    for fila in &filas[1..] {
        let celda = |i: usize| fila.get(i).map(String::as_str).unwrap_or("");
        let n = celda(nombre).trim();
        let m = correos(celda(correo))?.join("; ");
        if n.is_empty() {
            continue;
        }
        let clave = normalizar(n);
        let anteriores: BTreeSet<&String> = contactos.iter().filter(|(k, _)| normalizar(k) == clave).map(|(_, v)| v).collect();
        if !anteriores.is_empty() && anteriores != BTreeSet::from([&m]) {
            conflictos.push(n.to_string());
            continue;
        }
        contactos.insert(n.to_string(), m);
    }
    Ok(Catastro { contactos, conflictos })
}

fn texto_python(v: Option<&serde_json::Value>) -> String {
    match v {
        None => String::new(),
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Null) => "None".into(),
        Some(otro) => otro.to_string(),
    }
}

/// `config.parse_contacts`: una lista JSON de `{Nombre, Mail}` o el formato histórico de objetos separados por comas.
/// Salta los nombres vacíos y las direcciones inválidas, y descarta los nombres con dos direcciones distintas.
pub fn parse_contactos(raw: &str) -> Result<Vec<(String, String)>, String> {
    let raw = raw.trim();
    let lista = if raw.starts_with('[') { raw.to_string() } else { format!("[{}]", raw.trim_end_matches(',')) };
    let registros: Vec<serde_json::Value> = serde_json::from_str(&lista).map_err(|e| format!("El archivo de contactos no es un JSON válido: {e}"))?;
    let mut contactos: Vec<(String, String)> = Vec::new();
    let mut ambiguos: BTreeSet<String> = BTreeSet::new();
    for r in &registros {
        let nombre = texto_python(r.get("Nombre")).trim().to_string();
        let direccion = texto_python(r.get("Mail")).trim().to_string();
        if nombre.is_empty() || correos(&direccion).is_err() {
            continue;
        }
        match contactos.iter_mut().find(|(k, _)| *k == nombre) {
            Some((_, previa)) if *previa != direccion => {
                ambiguos.insert(nombre);
            }
            Some((_, previa)) => *previa = direccion,
            None => contactos.push((nombre, direccion)),
        }
    }
    contactos.retain(|(k, _)| !ambiguos.contains(k));
    Ok(contactos)
}

/// Contactos importados con la configuración que resulta (la original no cambia si hay un error).
#[derive(Debug)]
pub struct ImportacionContactos {
    pub cfg: Config,
    pub conflictos: Vec<String>,
}

/// `_import_contacts` de Flow Python. Un `.json` entrega contactos con la regla de `parse_contacts` (las entradas
/// «Tribunal: X» de un tribunal conocido actualizan sus destinatarios); un catastro (Excel u otra tabla) pasa por
/// `importar_catastro`. Extensión de Rust: en tablas que no son Excel (CSV, HTML), una fila «Tribunal: X» también
/// actualiza los destinatarios del tribunal, para releer lo que exporta Contactos.
pub fn importar_contactos_archivo(cfg: &Config, extension: &str, bytes: &[u8]) -> Result<ImportacionContactos, String> {
    let mut r = cfg.clone();
    let mut conflictos = Vec::new();
    let extension = extension.trim_start_matches('.').to_lowercase();
    if extension == "json" {
        let texto = std::str::from_utf8(bytes).map_err(|_| "El archivo de contactos no está en UTF-8.".to_string())?;
        for (nombre, direccion) in parse_contactos(texto)? {
            if let Some(t) = nombre.strip_prefix("Tribunal: ").map(str::to_uppercase).and_then(|k| r.tribunales.get_mut(&k)) {
                t.para = correos(&direccion)?.join("; ");
                continue;
            }
            if r.contactos.get(&nombre).is_some_and(|previa| *previa != direccion) {
                conflictos.push(nombre);
                continue;
            }
            r.contactos.insert(nombre, direccion);
        }
        return Ok(ImportacionContactos { cfg: r, conflictos });
    }
    let hojas = crate::lectura::read_sheets(bytes)?;
    let hoja = hojas.first().ok_or("El archivo no tiene hojas.")?;
    let filas: Vec<Vec<String>> = hoja.rows.iter().map(|f| f.iter().map(|c| c.display()).collect()).collect();
    let mut catastro = importar_catastro(&r.contactos, &filas)?;
    if !["xlsx", "xls"].contains(&extension.as_str()) {
        let tribunales: Vec<String> = catastro.contactos.keys().filter_map(|k| k.strip_prefix("Tribunal: ")).map(str::to_uppercase).collect();
        let conocidos: Vec<String> = tribunales.into_iter().filter(|k| r.tribunales.contains_key(k)).collect();
        for clave in conocidos {
            let nombre = catastro.contactos.keys().find(|k| k.strip_prefix("Tribunal: ").is_some_and(|x| x.to_uppercase() == clave)).cloned().unwrap();
            let direccion = catastro.contactos.remove(&nombre).unwrap_or_default();
            if let Some(t) = r.tribunales.get_mut(&clave) {
                t.para = direccion;
            }
        }
    }
    r.contactos = catastro.contactos;
    conflictos.extend(catastro.conflictos);
    Ok(ImportacionContactos { cfg: r, conflictos })
}

/// Alias de un contacto, en orden alfabético.
pub fn alias_de<'a>(cfg: &'a Config, contacto: &str) -> Vec<&'a str> {
    cfg.aliases.iter().filter(|(_, d)| d.as_str() == contacto).map(|(a, _)| a.as_str()).collect()
}

fn clave_nueva(cfg: &Config) -> String {
    let semilla = format!("{}|{}|{}", std::process::id(), chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(), cfg.plantillas.len());
    let hex = format!("{:x}", Sha256::digest(semilla.as_bytes()));
    format!("particular_{}", &hex[..10])
}

/// `duplicate_template`: copia editable de una plantilla, con el comportamiento de la original.
pub fn duplicar_plantilla(cfg: &Config, clave: &str, nombre: &str) -> Result<(Config, String), String> {
    let mut r = cfg.clone();
    let original = r.plantillas.get(clave).ok_or("Plantilla desconocida.")?.clone();
    let operativa = if original.categoria.is_empty() { clave.to_string() } else { original.categoria.clone() };
    let mut copia = original.clone();
    copia.nombre = if nombre.trim().is_empty() { format!("{} · copia", original.nombre) } else { nombre.trim().to_string() };
    copia.archivada = false;
    copia.categoria = if CATEGORIAS.contains(&operativa.as_str()) { operativa } else { String::new() };
    validar_plantilla(&copia)?;
    let destino = clave_nueva(&r);
    r.plantillas.insert(destino.clone(), copia);
    Ok((r, destino))
}

pub fn archivar_plantilla(cfg: &Config, clave: &str, archivada: bool) -> Result<Config, String> {
    let mut r = cfg.clone();
    r.plantillas.get_mut(clave).ok_or("Plantilla desconocida.")?.archivada = archivada;
    Ok(r)
}

/// `restore_template`: vuelve a la versión anterior guardada o, si no hay, a la de fábrica.
pub fn restaurar_plantilla(cfg: &Config, clave: &str) -> Result<Config, String> {
    let mut r = cfg.clone();
    let anterior = r
        .plantillas_anteriores
        .get(clave)
        .cloned()
        .or_else(|| plantillas_base().remove(clave))
        .ok_or("Esta plantilla todavía no tiene una versión anterior guardada.")?;
    let mut anterior = anterior;
    anterior.archivada = false;
    validar_plantilla(&anterior)?;
    r.plantillas.insert(clave.to_string(), anterior);
    Ok(r)
}

/// Guarda la edición de una plantilla y deja la versión anterior para poder restaurarla.
pub fn guardar_plantilla(cfg: &Config, clave: &str, nueva: PlantillaCorreo) -> Result<Config, String> {
    validar_plantilla(&nueva)?;
    let mut r = cfg.clone();
    if let Some(actual) = r.plantillas.get(clave) {
        if actual.asunto != nueva.asunto || actual.cuerpo != nueva.cuerpo || actual.nombre != nueva.nombre {
            r.plantillas_anteriores.insert(clave.to_string(), actual.clone());
        }
    }
    r.plantillas.insert(clave.to_string(), nueva);
    Ok(r)
}

pub fn validar_plantilla(p: &PlantillaCorreo) -> Result<(), String> {
    if !p.categoria.is_empty() && !CATEGORIAS.contains(&p.categoria.as_str()) {
        return Err("Categoría de correo inválida.".into());
    }
    for texto in [&p.asunto, &p.cuerpo] {
        variables(texto, &VARIABLES)?;
    }
    Ok(())
}

/// Un campo `{nombre!c:formato}` de una plantilla, como lo separa `string.Formatter.parse`.
struct Campo {
    nombre: String,
    formato: String,
    conversion: bool,
}

/// `string.Formatter().parse` sobre el texto: solo devuelve los campos (los literales no importan aquí).
fn campos(texto: &str) -> Result<Vec<Campo>, String> {
    let c: Vec<char> = texto.chars().collect();
    let (mut i, mut v) = (0, Vec::new());
    while i < c.len() {
        match c[i] {
            '{' if c.get(i + 1) == Some(&'{') => i += 2,
            '}' if c.get(i + 1) == Some(&'}') => i += 2,
            '}' => return Err("Single '}' encountered in format string".into()),
            '{' => {
                if i + 1 >= c.len() {
                    return Err("Single '{' encountered in format string".into());
                }
                let (mut j, mut nivel) = (i + 1, 1);
                while j < c.len() && nivel > 0 {
                    match c[j] {
                        '{' => nivel += 1,
                        '}' => nivel -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                if nivel > 0 {
                    return Err("expected '}' before end of string".into());
                }
                let dentro: String = c[i + 1..j - 1].iter().collect();
                let fin_nombre = dentro.find([':', '!']).unwrap_or(dentro.len());
                let nombre = dentro[..fin_nombre].to_string();
                let resto = &dentro[fin_nombre..];
                let (conversion, formato) = match resto.chars().next() {
                    Some('!') => {
                        let tras: Vec<char> = resto.chars().skip(1).collect();
                        match tras.as_slice() {
                            [] => return Err("end of string while looking for conversion specifier".into()),
                            [_] => (true, String::new()),
                            [_, ':', rest @ ..] => (true, rest.iter().collect()),
                            _ => return Err("expected ':' after conversion specifier".into()),
                        }
                    }
                    Some(':') => (false, resto[1..].to_string()),
                    _ => (false, String::new()),
                };
                v.push(Campo { nombre, formato, conversion });
                i = j;
            }
            _ => i += 1,
        }
    }
    Ok(v)
}

/// `template_validation.variables`: las variables de un texto, que deben ser de las ofrecidas y sin formato.
pub fn variables(texto: &str, permitidas: &[&str]) -> Result<BTreeSet<String>, String> {
    let mut r = BTreeSet::new();
    for c in campos(texto).map_err(|e| format!("Plantilla con llaves incompletas: {e}"))? {
        if c.nombre.is_empty() || !permitidas.contains(&c.nombre.as_str()) || !c.formato.is_empty() || c.conversion {
            return Err(format!(
                "Variable de plantilla inválida: {{{}}}. Usa solo los nombres ofrecidos, sin formatos ni conversiones.",
                c.nombre
            ));
        }
        r.insert(c.nombre);
    }
    Ok(r)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Importacion {
    pub importadas: Vec<PathBuf>,
    pub sin_clasificar: Vec<String>,
}

/// `import_templates`: un ZIP de matrices Word por tribunal y tipo queda en `directorio` como `TRIBUNAL_TIPO.docx`.
/// Una matriz que ya existía se respalda como `.bak.docx`. Si algo falla, no se escribe ninguna.
pub fn importar_matrices(zip_ruta: &Path, directorio: &Path) -> Result<Importacion, String> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(zip_ruta).map_err(|e| format!("No se pudo abrir el paquete: {e}"))?)
        .map_err(|_| "El archivo no es un paquete ZIP.".to_string())?;
    let tribunales = ["LAJA", "MULCHEN", "TOME"].map(|c| (c, Regex::new(&format!(r"\b{c}\b")).unwrap()));
    let tipos = [("PC_IE", r"\bPC IE\b"), ("PC_INFO", r"\bPC INFO\b"), ("NOMENCL", r"\bNOMENCL(?:ATURA)?\b")].map(|(k, p)| (k, Regex::new(p).unwrap()));
    let mut pendientes: Vec<((&str, &str), Vec<u8>)> = Vec::new();
    let mut resultado = Importacion::default();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        let nombre = f.name().to_string();
        if f.is_dir() || !nombre.to_lowercase().ends_with(".docx") {
            continue;
        }
        let limpio = normalizar(&nombre.replace('_', " ").replace('\\', "/")).to_uppercase();
        let tribunal: Vec<&str> = tribunales.iter().filter(|(_, re)| re.is_match(&limpio)).map(|(c, _)| *c).collect();
        let tipo: Vec<&str> = tipos.iter().filter(|(_, re)| re.is_match(&limpio)).map(|(k, _)| *k).collect();
        let ([tribunal], [tipo]) = (tribunal.as_slice(), tipo.as_slice()) else {
            resultado.sin_clasificar.push(nombre);
            continue;
        };
        if f.size() > 20_000_000 {
            return Err(format!("Matriz demasiado grande: {nombre}"));
        }
        let mut contenido = Vec::new();
        f.read_to_end(&mut contenido).map_err(|e| e.to_string())?;
        let ok = zip::ZipArchive::new(std::io::Cursor::new(&contenido)).ok().is_some_and(|mut d| d.by_name("word/document.xml").is_ok());
        if !ok {
            return Err(format!("La matriz no es un documento Word válido: {nombre}"));
        }
        if pendientes.iter().any(|(k, _)| *k == (*tribunal, *tipo)) {
            return Err(format!("Hay dos matrices para {tribunal}/{tipo}. Conserva una versión por tipo."));
        }
        pendientes.push(((tribunal, tipo), contenido));
    }
    std::fs::create_dir_all(directorio).map_err(|e| format!("No se pudo crear la carpeta de plantillas: {e}"))?;
    for ((tribunal, tipo), contenido) in pendientes {
        let destino = directorio.join(format!("{tribunal}_{tipo}.docx"));
        if destino.exists() {
            std::fs::copy(&destino, destino.with_extension("bak.docx")).map_err(|e| format!("No se pudo respaldar {}: {e}", destino.display()))?;
        }
        let temporal = destino.with_extension("tmp");
        std::fs::write(&temporal, &contenido).map_err(|e| e.to_string())?;
        std::fs::rename(&temporal, &destino).map_err(|e| e.to_string())?;
        resultado.importadas.push(destino);
    }
    resultado.importadas.sort();
    resultado.sin_clasificar.sort();
    Ok(resultado)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::io::Write;

    fn cfg_base(contactos: &Value, aliases: &Value) -> Config {
        let mut c = Config::default();
        c.contactos = contactos.as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect();
        c.aliases = aliases.as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect();
        c
    }

    fn estado(c: &Config) -> Value {
        serde_json::json!({
            "contactos": c.contactos,
            "aliases": c.aliases,
            "tribunales": c.tribunales.iter().filter(|(_, t)| !t.para.is_empty()).map(|(k, t)| (k.clone(), t.para.clone())).collect::<std::collections::BTreeMap<_, _>>(),
        })
    }

    fn texto(v: &Value) -> String {
        v.as_str().unwrap_or_default().to_string()
    }

    fn contacto(caso: &Value) -> Vec<String> {
        let cfg = cfg_base(&caso["contactos"], &caso["aliases"]);
        let alias: Vec<String> = caso["alias"].as_array().unwrap().iter().map(texto).collect();
        let previo = caso["previo"].as_str();
        let r = guardar_contacto(&cfg, caso["nombre"].as_str().unwrap(), caso["direccion"].as_str().unwrap(), &alias, previo);
        match (r, caso.get("error"), caso.get("resultado")) {
            (Err(e), Some(esperado), _) if e == texto(esperado) => vec![],
            (Ok(c), None, Some(esperado)) if estado(&c) == *esperado => vec![],
            (otro, e, r) => vec![format!("Rust {otro:?} · Python error {e:?} resultado {r:?}")],
        }
    }

    fn plantilla(caso: &Value) -> Vec<String> {
        let mut cfg = Config::default();
        let clave = caso["clave"].as_str().unwrap();
        let python = |p: &Value| PlantillaCorreo {
            nombre: texto(&p["nombre"]),
            asunto: texto(&p["asunto"]),
            cuerpo: texto(&p["cuerpo"]),
            destino: "tribunal".into(),
            adjunto: p["adjunto"] == "obligatorio",
            usa_modalidades: p["usa_modalidades"].as_bool().unwrap_or(false),
            archivada: p["archivada"].as_bool().unwrap_or(false),
            categoria: String::new(),
        };
        if caso["respaldo"].is_object() {
            cfg.plantillas_anteriores.insert(clave.to_string(), python(&caso["respaldo"]));
        }
        let (resultado, obtenida): (Result<(), String>, Option<PlantillaCorreo>) = match caso["operacion"].as_str().unwrap() {
            "duplicar" => match duplicar_plantilla(&cfg, clave, caso["nombre"].as_str().unwrap()) {
                Ok((c, k)) => {
                    let prefijo = k.chars().take(k.chars().count() - 10).collect::<String>();
                    assert_eq!((prefijo.as_str(), k.chars().count() - prefijo.chars().count()), (texto(&caso["prefijo"]).as_str(), caso["largo_sufijo"].as_u64().unwrap() as usize));
                    (Ok(()), c.plantillas.get(&k).cloned())
                }
                Err(e) => (Err(e), None),
            },
            "archivar" => archivar_plantilla(&cfg, clave, true).map(|c| ((), c.plantillas[clave].clone())).map_or_else(|e| (Err(e), None), |(_, p)| (Ok(()), Some(p))),
            "desarchivar" => {
                cfg.plantillas.get_mut(clave).unwrap().archivada = true;
                archivar_plantilla(&cfg, clave, false).map(|c| c.plantillas[clave].clone()).map_or_else(|e| (Err(e), None), |p| (Ok(()), Some(p)))
            }
            _ => restaurar_plantilla(&cfg, clave).map(|c| c.plantillas[clave].clone()).map_or_else(|e| (Err(e), None), |p| (Ok(()), Some(p))),
        };
        match (resultado, obtenida, caso.get("error"), caso.get("plantilla")) {
            (Err(_), None, Some(esperado), _) if esperado == "KeyError" => vec![],
            (Err(e), None, Some(esperado), _) if e == texto(esperado) => vec![],
            (Ok(()), Some(p), None, Some(esperada)) => {
                let obtenida = [("nombre", p.nombre.clone()), ("asunto", p.asunto.clone()), ("cuerpo", p.cuerpo.clone())];
                let mut d: Vec<String> = obtenida.iter().filter(|(k, v)| *v != texto(&esperada[*k])).map(|(k, v)| format!("{k}: Rust {v:?}")).collect();
                if p.archivada != esperada["archivada"].as_bool().unwrap() {
                    d.push(format!("archivada Rust {}", p.archivada));
                }
                // Python deja `categoria` sin definir cuando la plantilla no tiene un comportamiento propio.
                if p.categoria != esperada["categoria"].as_str().unwrap_or("") {
                    d.push(format!("categoria Rust {:?} · Python {}", p.categoria, esperada["categoria"]));
                }
                d
            }
            (r, p, e, esperada) => vec![format!("Rust {r:?}/{p:?} · Python error {e:?} plantilla {esperada:?}")],
        }
    }

    fn gramatica(caso: &Value) -> Vec<String> {
        match (variables(caso["texto"].as_str().unwrap(), &VARIABLES), caso.get("error"), caso.get("variables")) {
            (Err(e), Some(esperado), _) if e == texto(esperado) => vec![],
            (Ok(v), None, Some(esperadas)) if v.iter().cloned().collect::<Vec<_>>() == esperadas.as_array().unwrap().iter().map(texto).collect::<Vec<_>>() => vec![],
            (otro, e, v) => vec![format!("{:?}: Rust {otro:?} · Python error {e:?} variables {v:?}", caso["texto"])],
        }
    }

    fn matrices(caso: &Value, raiz: &Path, n: usize) -> Vec<String> {
        let zip_ruta = raiz.join(format!("paquete{n}.zip"));
        let docx = {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            z.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"<w:document/>").unwrap();
            z.finish().unwrap().into_inner()
        };
        let mut z = zip::ZipWriter::new(std::fs::File::create(&zip_ruta).unwrap());
        let invalidos: Vec<String> = caso["invalidos"].as_array().unwrap().iter().map(texto).collect();
        for nombre in caso["nombres"].as_array().unwrap() {
            let nombre = nombre.as_str().unwrap();
            z.start_file(nombre, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(if invalidos.iter().any(|i| i == nombre) { b"no es un docx" } else { docx.as_slice() }).unwrap();
        }
        z.finish().unwrap();
        let destino = raiz.join(format!("plantillas{n}"));
        match (importar_matrices(&zip_ruta, &destino), caso.get("error"), caso.get("importadas")) {
            (Err(e), Some(esperado), _) => {
                // Los mensajes de Python con el nombre de la matriz coinciden; el de un Word dañado es de cada lenguaje.
                if esperado == "BadZipFile" || e == texto(esperado) { vec![] } else { vec![format!("error Rust {e:?} · Python {esperado}")] }
            }
            (Ok(r), None, Some(esperadas)) => {
                // Python escribe `TRIBUNAL/TIPO.docx`; Rust, `TRIBUNAL_TIPO.docx`.
                let rust: Vec<String> = r.importadas.iter().map(|p| p.file_name().unwrap().to_string_lossy().replace("_PC", "/PC").replace("_NOMENCL", "/NOMENCL")).collect();
                let python: Vec<String> = esperadas.as_array().unwrap().iter().map(texto).collect();
                let mut d = Vec::new();
                if rust != python {
                    d.push(format!("importadas Rust {rust:?} · Python {python:?}"));
                }
                if r.sin_clasificar != caso["sin_clasificar"].as_array().unwrap().iter().map(texto).collect::<Vec<_>>() {
                    d.push(format!("sin clasificar Rust {:?} · Python {}", r.sin_clasificar, caso["sin_clasificar"]));
                }
                d
            }
            (r, e, _) => vec![format!("Rust {r:?} · Python error {e:?}")],
        }
    }

    /// E-A4: contactos con alias, plantillas, gramática de variables e importación de matrices de Rust contra Flow Python
    /// (`app/qa/generar_personalizacion.py`).
    #[test]
    fn ajustes_iguales_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/personalizacion.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let raiz = perfil.join("personalizacion");
        let _ = std::fs::remove_dir_all(&raiz);
        std::fs::create_dir_all(&raiz).unwrap();
        let (mut total, mut fallos) = (0, Vec::new());
        let mut anotar = |nombre: String, d: Vec<String>| {
            total += 1;
            if !d.is_empty() {
                fallos.push(format!("{nombre}:\n  {}", d.join("\n  ")));
            }
        };
        for (i, c) in datos["contactos"].as_array().unwrap().iter().enumerate() {
            anotar(format!("contacto {i} {}", c["nombre"]), contacto(c));
        }
        for (i, c) in datos["plantillas"].as_array().unwrap().iter().enumerate() {
            anotar(format!("plantilla {i} {} {}", c["operacion"], c["clave"]), plantilla(c));
        }
        for (i, c) in datos["gramatica"].as_array().unwrap().iter().enumerate() {
            anotar(format!("gramática {i}"), gramatica(c));
        }
        for (i, c) in datos["matrices"].as_array().unwrap().iter().enumerate() {
            anotar(format!("matrices {i}"), matrices(c, &raiz, i));
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("AJUSTES: {}/{total}, fallos {}", total - fallos.len(), fallos.len());
        let _ = std::fs::remove_dir_all(&raiz);
        assert_eq!(total, 44, "el oráculo debe traer 44 casos");
        assert!(fallos.is_empty());
    }

    /// Prueba permanente del oráculo: un resultado esperado alterado debe detectarse.
    #[test]
    fn oraculo_de_ajustes_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/personalizacion.json")).unwrap();
        let mut caso = datos["contactos"][1].clone();
        assert!(contacto(&caso).is_empty());
        caso["resultado"]["contactos"]["Programa Uno"] = Value::String("otro@ejemplo.cl".into());
        assert!(!contacto(&caso).is_empty(), "un contacto esperado alterado no se detecta");
        let mut g = datos["gramatica"][0].clone();
        assert!(gramatica(&g).is_empty());
        g["variables"] = serde_json::json!(["FECHA"]);
        assert!(!gramatica(&g).is_empty());
    }

    #[test]
    fn un_alias_resuelve_el_correo_del_contacto() {
        let mut cfg = Config::default();
        cfg = guardar_contacto(&cfg, "Programa Dos", "dos@ejemplo.cl", &["PD".into()], None).unwrap();
        assert_eq!(cfg.contacto_programa("pd"), "dos@ejemplo.cl");
        assert_eq!(cfg.contacto_programa("Programa Dos"), "dos@ejemplo.cl");
        assert_eq!(cfg.contacto_programa("Otro"), "");
        assert_eq!(alias_de(&cfg, "Programa Dos"), ["PD"]);
    }

    // ---- E-A6: importación de contactos
    fn catastro_de_python(caso: &Value) -> Vec<String> {
        use base64::Engine;
        let previos: BTreeMap<String, String> =
            caso["contactos"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
        let cfg = Config { contactos: previos, ..Config::default() };
        let bytes = base64::engine::general_purpose::STANDARD.decode(caso["xlsx"].as_str().unwrap()).unwrap();
        match (importar_contactos_archivo(&cfg, "xlsx", &bytes), caso.get("error"), caso.get("resultado")) {
            (Err(e), Some(esperado), _) if e == texto(esperado) => vec![],
            (Ok(r), None, Some(esperado)) => {
                let obtenido = serde_json::json!({"contactos": r.cfg.contactos, "conflictos": r.conflictos});
                if obtenido == *esperado { vec![] } else { vec![format!("Rust {obtenido} · Python {esperado}")] }
            }
            (otro, e, r) => vec![format!("Rust {otro:?} · Python error {e:?} resultado {r:?}")],
        }
    }

    fn texto_de_python(caso: &Value) -> Vec<String> {
        let obtenido: BTreeMap<String, String> = parse_contactos(caso["texto"].as_str().unwrap()).unwrap().into_iter().collect();
        let esperado: BTreeMap<String, String> = caso["contactos"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
        if obtenido == esperado { vec![] } else { vec![format!("Rust {obtenido:?} · Python {esperado:?}")] }
    }

    /// E-A6: el catastro Excel (`import_contacts`) y `parse_contacts` de Rust contra Flow Python
    /// (`app/qa/generar_catastro.py`).
    #[test]
    fn importacion_de_contactos_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/catastro.json")).unwrap();
        let (mut total, mut fallos) = (0, Vec::new());
        for (i, c) in datos["catastros"].as_array().unwrap().iter().enumerate() {
            total += 1;
            let d = catastro_de_python(c);
            if !d.is_empty() {
                fallos.push(format!("catastro {i}: {}", d.join(" ")));
            }
        }
        for (i, c) in datos["textos"].as_array().unwrap().iter().enumerate() {
            total += 1;
            let d = texto_de_python(c);
            if !d.is_empty() {
                fallos.push(format!("texto {i}: {}", d.join(" ")));
            }
        }
        let errores = datos["catastros"].as_array().unwrap().iter().filter(|c| c.get("error").is_some()).count();
        let conflictos: usize = datos["catastros"].as_array().unwrap().iter().filter_map(|c| c["resultado"]["conflictos"].as_array()).map(Vec::len).sum();
        println!("CATASTRO: {}/{total}, fallos {}", total - fallos.len(), fallos.len());
        assert!(fallos.is_empty(), "{}", fallos.join("\n"));
        assert_eq!((total, errores, conflictos), (36, 5, 4), "conteos del oráculo");
    }

    /// Prueba permanente del oráculo: un resultado esperado alterado debe detectarse.
    #[test]
    fn oraculo_de_catastro_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/catastro.json")).unwrap();
        let mut caso = datos["catastros"][7].clone();
        assert!(catastro_de_python(&caso).is_empty());
        caso["resultado"]["conflictos"] = serde_json::json!([]);
        assert!(!catastro_de_python(&caso).is_empty(), "un conflicto esperado alterado no se detecta");
        let mut t = datos["textos"][0].clone();
        assert!(texto_de_python(&t).is_empty());
        t["contactos"]["Programa Uno"] = Value::String("otro@ejemplo.cl".into());
        assert!(!texto_de_python(&t).is_empty(), "un texto esperado alterado no se detecta");
    }

    #[test]
    fn importar_json_respeta_tribunales_conflictos_y_no_toca_la_configuracion_original() {
        let mut cfg = Config::default();
        cfg.tribunales.insert("LAJA".into(), crate::config::Tribunal { nombre: "Laja".into(), para: String::new() });
        cfg.contactos.insert("Programa Uno".into(), "uno@ejemplo.cl".into());
        let json = r#"[{"Nombre":"Tribunal: LAJA","Mail":"laja@ejemplo.cl, laja2@ejemplo.cl"},{"Nombre":"Tribunal: OTRO","Mail":"x@ejemplo.cl"},
            {"Nombre":"Programa Uno","Mail":"distinto@ejemplo.cl"},{"Nombre":"Programa Dos","Mail":"dos@ejemplo.cl"}]"#;
        let r = importar_contactos_archivo(&cfg, "json", json.as_bytes()).unwrap();
        assert_eq!(r.cfg.tribunales["LAJA"].para, "laja@ejemplo.cl; laja2@ejemplo.cl");
        assert_eq!(r.conflictos, ["Programa Uno"]);
        assert_eq!(r.cfg.contactos["Programa Uno"], "uno@ejemplo.cl");
        assert_eq!(r.cfg.contactos["Programa Dos"], "dos@ejemplo.cl");
        assert_eq!(r.cfg.contactos["Tribunal: OTRO"], "x@ejemplo.cl", "un tribunal desconocido es un contacto más, como en Python");
        assert!(cfg.tribunales["LAJA"].para.is_empty() && !cfg.contactos.contains_key("Programa Dos"), "la original no cambia");
        assert!(importar_contactos_archivo(&cfg, "json", b"[{").is_err());
    }

    #[test]
    fn importar_tabla_csv_relee_lo_que_exporta_contactos() {
        let mut cfg = Config::default();
        cfg.tribunales.insert("LAJA".into(), crate::config::Tribunal { nombre: "Laja".into(), para: String::new() });
        let csv = "Nombre;Mail\nTribunal: LAJA;laja@ejemplo.cl\nPrograma Uno;uno@ejemplo.cl\n";
        let r = importar_contactos_archivo(&cfg, "csv", csv.as_bytes()).unwrap();
        assert_eq!(r.cfg.tribunales["LAJA"].para, "laja@ejemplo.cl");
        assert_eq!(r.cfg.contactos.keys().collect::<Vec<_>>(), ["Programa Uno"]);
        let malo = "Nombre;Mail\nPrograma Uno;uno@ejemplo.cl\nPrograma Dos;no es correo\n";
        let e = importar_contactos_archivo(&cfg, "csv", malo.as_bytes()).unwrap_err();
        assert_eq!(e, "Dirección inválida: no es correo");
    }
}
