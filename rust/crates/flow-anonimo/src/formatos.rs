use crate::{
    texto::{self, Clase, Sesion},
    Opciones, Salida, MAX_ARCHIVO,
};
use flow_dominio::Celda;
use flow_tabular::{Hoja, Libro};
use regex::Regex;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    io::{Cursor, Read},
    sync::LazyLock,
};

pub enum Entrada {
    Json(Value),
    JsonLineas(Vec<Value>),
    Texto(String, String),
    Csv(Vec<Vec<String>>, u8),
    Excel(Libro, bool),
    Html(Vec<crate::documentos::Bloque>),
    Ics(Vec<String>),
    Word(Vec<String>),
    Zip(crate::archivos::Archivo),
}
fn extension(nombre: &str) -> String {
    std::path::Path::new(nombre).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

fn leer_excel(nombre: &str, datos: &[u8], biff: bool) -> Result<Entrada, String> {
    use calamine::{Data, Reader};
    use sha2::{Digest, Sha256};
    let mut libro = calamine::open_workbook_auto_from_rs(Cursor::new(datos.to_vec())).map_err(|_| "No se pudo leer el Excel.")?;
    let (mut hojas, mut formulas, mut celdas) = (vec![], false, 0usize);
    for nombre in libro.sheet_names().to_vec() {
        let rango = libro.worksheet_range(&nombre).map_err(|_| "No se pudo leer una hoja Excel.")?;
        if rango.width() > 16384 || rango.height() > 1_048_576 || rango.get_size().0.saturating_mul(rango.get_size().1) > 2_000_000 {
            return Err("La hoja supera los límites de lectura.".into());
        }
        formulas |= libro.worksheet_formula(&nombre).map(|r| r.rows().flatten().any(|s| !s.is_empty())).unwrap_or(false);
        let (inicio_fila, inicio_col) = rango.start().unwrap_or_default();
        let ancho = inicio_col as usize + rango.width();
        let alto = inicio_fila as usize + rango.height();
        if ancho.saturating_mul(alto) > 2_000_000 {
            return Err("La hoja supera los límites de lectura.".into());
        }
        let mut filas = vec![vec![Celda::Vacia; ancho]; inicio_fila as usize];
        celdas += ancho.saturating_mul(inicio_fila as usize);
        for fila in rango.rows() {
            celdas += fila.len() + inicio_col as usize;
            if celdas > 2_000_000 {
                return Err("El libro contiene demasiadas celdas.".into());
            }
            let fila = std::iter::repeat_n(Celda::Vacia, inicio_col as usize)
                .chain(fila.iter().map(|d| match d {
                    Data::Empty | Data::Error(_) => Celda::Vacia,
                    Data::String(s) => Celda::Texto(s.clone()),
                    Data::Int(n) => Celda::Numero(*n as f64),
                    Data::Float(n) => Celda::Numero(*n),
                    Data::Bool(b) => Celda::Texto(b.to_string()),
                    Data::DateTime(d) => {
                        d.as_datetime().map(|d| Celda::Texto(d.format("%Y-%m-%dT%H:%M:%S").to_string())).unwrap_or(Celda::Vacia)
                    }
                    Data::DateTimeIso(s) | Data::DurationIso(s) => Celda::Texto(s.clone()),
                }))
                .collect();
            filas.push(fila);
        }
        hojas.push(Hoja { nombre, filas });
    }
    Ok(Entrada::Excel(
        Libro {
            nombre: nombre.into(),
            sha256: format!("{:x}", Sha256::digest(datos)),
            formato: if biff { flow_tabular::Formato::XlsBiff } else { flow_tabular::Formato::Xlsx },
            hojas,
        },
        formulas,
    ))
}

pub fn decodificar(datos: &[u8]) -> Result<String, String> {
    let (encoding, contenido) = if datos.starts_with(&[0xff, 0xfe]) {
        (encoding_rs::UTF_16LE, &datos[2..])
    } else if datos.starts_with(&[0xfe, 0xff]) {
        (encoding_rs::UTF_16BE, &datos[2..])
    } else {
        if let Ok(s) = std::str::from_utf8(datos) {
            if s.chars().any(|c| c == '\0' || (c.is_control() && !matches!(c, '\t' | '\r' | '\n'))) {
                return Err("El texto contiene controles no admitidos.".into());
            }
            return Ok(s.trim_start_matches('\u{feff}').to_string());
        }
        (encoding_rs::WINDOWS_1252, datos)
    };
    let (s, mal) = encoding.decode_without_bom_handling(contenido);
    if mal || s.chars().any(|c| c == '\0' || (c.is_control() && !matches!(c, '\t' | '\r' | '\n'))) {
        return Err("La codificación de texto no se reconoce con seguridad.".into());
    }
    Ok(s.into_owned())
}
fn xml_seguro(datos: &[u8]) -> bool {
    decodificar(datos).is_ok_and(|s| {
        let texto = s.to_ascii_uppercase();
        !texto.contains("<!DOCTYPE") && !texto.contains("<!ENTITY")
    })
}

pub fn validar_zip(datos: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(datos)).map_err(|_| "El ZIP no se reconoce.")?;
    if zip.len() > 4000 {
        return Err("El ZIP contiene demasiados archivos.".into());
    }
    let (mut total, mut vistos, mut out) = (0u64, BTreeSet::new(), vec![]);
    for i in 0..zip.len() {
        let mut miembro = zip.by_index(i).map_err(|_| "El ZIP está cifrado o contiene una entrada no admitida.")?;
        let nombre = miembro.name().to_string();
        if nombre.contains(['\\', ':', '\0'])
            || nombre.starts_with('/')
            || nombre.split('/').any(|p| p == ".." || p == ".")
            || !vistos.insert(nombre.to_lowercase())
        {
            return Err("El ZIP contiene rutas inseguras o repetidas.".into());
        }
        if miembro.unix_mode().is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000 && m & 0o170000 != 0o040000) {
            return Err("El ZIP contiene enlaces u objetos especiales.".into());
        }
        if miembro.is_dir() {
            continue;
        }
        total = total.checked_add(miembro.size()).ok_or("Expansión de ZIP excesiva.")?;
        if miembro.size() > MAX_ARCHIVO as u64 || total > 200 * 1024 * 1024 || miembro.size() > miembro.compressed_size().max(1) * 300 {
            return Err("La expansión del ZIP excede los límites.".into());
        }
        let mut raw = vec![];
        miembro.by_ref().take(MAX_ARCHIVO as u64 + 1).read_to_end(&mut raw).map_err(|_| "No se pudo leer un miembro del ZIP.")?;
        if raw.len() > MAX_ARCHIVO {
            return Err("Un miembro del ZIP excede los límites.".into());
        }
        if nombre.to_lowercase().ends_with(".xml") && !xml_seguro(&raw) {
            return Err("El XML contiene declaraciones no admitidas.".into());
        }
        out.push((nombre, raw));
    }
    Ok(out)
}

pub fn cargar(nombre: &str, datos: &[u8]) -> Result<Entrada, String> {
    cargar_interno(nombre, datos, 0, &mut (200 * 1024 * 1024))
}
pub(crate) fn cargar_interno(nombre: &str, datos: &[u8], nivel: usize, presupuesto: &mut usize) -> Result<Entrada, String> {
    if nivel > 3 || datos.len() > *presupuesto {
        return Err("Archivo anidado o volumen excesivo.".into());
    }
    *presupuesto -= datos.len();
    if datos.is_empty() || datos.len() > MAX_ARCHIVO {
        return Err("Archivo vacío o superior a 80 MiB.".into());
    }
    let ext = extension(nombre);
    if datos.starts_with(b"PK") {
        let miembros = validar_zip(datos)?;
        if miembros.iter().any(|(n, _)| n == "xl/workbook.xml") {
            for (n, raw) in &miembros {
                if n.to_lowercase().ends_with(".xml") {
                    crate::limites::xml_excel(raw)?;
                }
            }
            return leer_excel(nombre, datos, false);
        }
        if miembros.iter().any(|(n, _)| n == "word/document.xml") {
            return Ok(Entrada::Word(crate::documentos::parrafos_docx(&miembros)?));
        }
        return crate::archivos::leer(miembros, nivel, presupuesto).map(Entrada::Zip);
    }
    if datos.starts_with(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]) {
        return leer_excel(nombre, datos, true);
    }
    if datos.starts_with(b"%PDF") || datos.starts_with(b"MZ") || datos.starts_with(b"\x89PNG") || datos.starts_with(&[0xff, 0xd8]) {
        return Err("PDF, imágenes y ejecutables no se anonimizan como texto.".into());
    }
    let s = decodificar(datos)?;
    let inicio = s.trim_start();
    let minus = inicio.to_ascii_lowercase();
    if minus.starts_with("<?xml") || minus.starts_with("<workbook") || minus.starts_with("<ss:workbook") {
        if !xml_seguro(datos) {
            return Err("El XML contiene declaraciones no admitidas.".into());
        }
        static TIPOS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"((?:ss:)?Type\s*=\s*["'])DateTime(["'])"#).unwrap());
        let xml = TIPOS.replace_all(&s, "${1}String${2}");
        return flow_tabular::leer_bytes(nombre, xml.as_bytes().to_vec())
            .map(|l| Entrada::Excel(l, s.contains("Formula=")))
            .map_err(|_| "No se pudo leer Excel XML.".into());
    }
    if minus.starts_with("<html") || minus.starts_with("<table") || minus.starts_with("<!doctype html") {
        if !matches!(ext.as_str(), "xls" | "xlsx" | "xlsm") {
            return crate::documentos::leer_html(&s).map(Entrada::Html);
        }
        return flow_tabular::leer_bytes(nombre, datos.to_vec())
            .map(|l| Entrada::Excel(l, false))
            .map_err(|_| "No se pudo leer el Excel HTML.".into());
    }
    if ext == "ics" || minus.starts_with("begin:vcalendar") {
        return crate::calendario::lineas(&s).map(Entrada::Ics);
    }
    if ext == "jsonl" {
        let mut values = vec![];
        for linea in s.lines().filter(|l| !l.trim().is_empty()) {
            if values.len() >= 2_000_000 {
                return Err("JSONL contiene demasiadas entradas.".into());
            }
            let v = serde_json::from_str(linea).map_err(|_| "JSONL inválido.")?;
            limite_json(&v)?;
            values.push(v);
        }
        return Ok(Entrada::JsonLineas(values));
    }
    if matches!(inicio.chars().next(), Some('{' | '[')) || ext == "json" {
        return serde_json::from_str(&s).map(Entrada::Json).map_err(|_| "JSON inválido.".into());
    }
    if matches!(ext.as_str(), "csv" | "tsv") {
        let cabecera = s.lines().next().unwrap_or("");
        let delimitador =
            if ext == "tsv" { b'\t' } else { b",;\t".iter().copied().max_by_key(|c| cabecera.bytes().filter(|b| b == c).count()).unwrap() };
        let mut lector = csv::ReaderBuilder::new().has_headers(false).flexible(true).delimiter(delimitador).from_reader(s.as_bytes());
        let mut filas = vec![];
        let mut total = 0usize;
        for registro in lector.records() {
            let registro = registro.map_err(|_| "CSV inválido.")?;
            total += registro.len();
            if total > 2_000_000 {
                return Err("La tabla contiene demasiadas celdas.".into());
            }
            filas.push(registro.iter().map(str::to_string).collect());
        }
        return Ok(Entrada::Csv(filas, delimitador));
    }
    if matches!(ext.as_str(), "txt" | "md" | "log") {
        return Ok(Entrada::Texto(s, ext));
    }
    Err("Formato no admitido; el archivo original se conserva.".into())
}

fn tipo_persona(c: Option<Clase>) -> bool {
    matches!(c, Some(Clase::Persona | Clase::Curador))
}
fn cabecera(filas: &[Vec<Celda>]) -> Option<usize> {
    filas.iter().take(60).position(|fila| {
        let no_vacias = fila.iter().filter(|c| !c.es_vacia()).collect::<Vec<_>>();
        no_vacias.len() >= 2
            && no_vacias.iter().all(|c| {
                c.texto().len() <= 80
                    && !c.texto().trim_end().ends_with(':')
                    && !(texto::clase(&c.texto()).is_none() && !texto::protegido(&c.texto()) && texto::parece_nombre(&c.texto()))
            })
            && (no_vacias
                .iter()
                .filter(|c| matches!(texto::clase(&c.texto()), Some(Clase::Persona | Clase::Curador | Clase::Rut | Clase::Rit)))
                .count()
                >= 2
                || no_vacias.iter().any(|c| matches!(texto::clase(&c.texto()), Some(Clase::Rut | Clase::Rit))))
    })
}
fn limite_json(v: &Value) -> Result<(), String> {
    let mut pila = vec![(v, 0)];
    let mut elementos = 0;
    while let Some((v, prof)) = pila.pop() {
        elementos += 1;
        if elementos > 2_000_000 || prof > 60 {
            return Err("El JSON supera los límites de estructura.".into());
        }
        match v {
            Value::Object(m) => pila.extend(m.values().map(|v| (v, prof + 1))),
            Value::Array(xs) => pila.extend(xs.iter().map(|v| (v, prof + 1))),
            _ => {}
        }
    }
    Ok(())
}
pub(crate) fn seed_json(sesion: &mut Sesion, v: &Value, key: &str, prof: usize) -> Result<(), String> {
    if prof == 0 {
        limite_json(v)?;
    }
    if prof > 60 {
        return Err("JSON demasiado profundo.".into());
    }
    match v {
        Value::Object(m) => {
            for (k, v) in m {
                sesion.reservar(k);
                seed_json(sesion, v, k, prof + 1)?;
            }
        }
        Value::Array(xs) => {
            for v in xs {
                seed_json(sesion, v, key, prof + 1)?;
            }
        }
        Value::String(s) => {
            sesion.reservar(s);
            if tipo_persona(texto::clase(key)) {
                sesion.persona(s);
            } else if let Ok(v) = serde_json::from_str::<Value>(s) {
                seed_json(sesion, &v, "", prof + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn par(linea: &str) -> Option<(&str, &str)> {
    let (label, valor) = linea.split_once(':').or_else(|| linea.split_once('='))?;
    let label = label.trim();
    (label.len() <= 80 && (!label.is_empty()) && (texto::clase(label).is_some() || texto::protegido(label) || texto::credencial(label)))
        .then_some((label, valor.trim()))
}
pub(crate) fn seed_tabla(sesion: &mut Sesion, filas: &[Vec<Celda>]) -> Result<(), String> {
    let encabezado = cabecera(filas);
    for (n, fila) in filas.iter().enumerate() {
        for (i, valor) in fila.iter().enumerate() {
            let s = valor.texto();
            sesion.reservar(&s);
            if let Some(header) = encabezado.filter(|h| n > *h).and_then(|h| filas[h].get(i)) {
                if tipo_persona(texto::clase(&header.texto())) {
                    sesion.persona(&s);
                }
                if matches!(texto::clase(&header.texto()), Some(Clase::Json)) {
                    if let Ok(v) = serde_json::from_str(&s) {
                        seed_json(sesion, &v, "", 0)?;
                    }
                }
            } else if let Some((label, contenido)) = par(&s) {
                if tipo_persona(texto::clase(label)) {
                    sesion.persona(contenido);
                }
            }
            if n < encabezado.unwrap_or(filas.len())
                && (s.trim_end().ends_with(':') || s.split_whitespace().count() <= 3)
                && tipo_persona(texto::clase(&s))
            {
                if let Some(valor) = fila[i + 1..].iter().find(|c| !c.es_vacia()) {
                    sesion.persona(&valor.texto());
                }
            }
        }
    }
    Ok(())
}
pub(crate) fn semillar_linea(sesion: &mut Sesion, l: &str) {
    sesion.reservar(l);
    if let Some((label, valor)) = par(l) {
        if tipo_persona(texto::clase(label)) {
            sesion.persona(valor);
        }
    }
}
pub(crate) fn transformar_linea(sesion: &mut Sesion, l: &str) -> Result<String, String> {
    if let Some((label, valor)) = par(l) {
        Ok(format!("{label}: {}", sesion.valor(label, valor)?))
    } else {
        sesion.valor("", l)
    }
}
pub fn recolectar(sesion: &mut Sesion, entrada: &Entrada) -> Result<(), String> {
    match entrada {
        Entrada::Zip(a) => crate::archivos::semillar(sesion, a)?,
        Entrada::Html(b) => crate::documentos::semillar_html(sesion, b)?,
        Entrada::Ics(l) => crate::calendario::semillar(sesion, l),
        Entrada::Word(p) => {
            for l in p {
                semillar_linea(sesion, l);
            }
        }
        Entrada::Json(v) => seed_json(sesion, v, "", 0)?,
        Entrada::JsonLineas(xs) => {
            for v in xs {
                seed_json(sesion, v, "", 0)?;
            }
        }
        Entrada::Texto(s, _) => {
            for l in s.lines() {
                sesion.reservar(l);
                if let Some((label, valor)) = par(l) {
                    if tipo_persona(texto::clase(label)) {
                        sesion.persona(valor);
                    }
                }
            }
        }
        Entrada::Csv(rows, _) => {
            seed_tabla(sesion, &rows.iter().map(|r| r.iter().cloned().map(Celda::Texto).collect()).collect::<Vec<_>>())?
        }
        Entrada::Excel(libro, formulas) => {
            if *formulas {
                sesion.avisos.insert(
                    "El libro contiene fórmulas: se conservan solo valores calculados; las fórmulas sin caché quedan vacías.".into(),
                );
            }
            for hoja in &libro.hojas {
                sesion.reservar(&hoja.nombre);
                seed_tabla(sesion, &hoja.filas)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn json(sesion: &mut Sesion, v: &Value, key: &str, prof: usize, textos: &mut Vec<(String, bool)>) -> Result<Value, String> {
    if prof > 60 {
        return Err("JSON demasiado profundo.".into());
    }
    Ok(match v {
        Value::Object(m) => {
            let mut out = serde_json::Map::new();
            for (k, v) in m {
                let mut nuevo = sesion.texto(k, false, false)?;
                static NOMBRES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\p{Lu}\p{Ll}{2,}\s+\p{Lu}\p{Ll}{2,}\b").unwrap());
                if (NOMBRES.is_match(&nuevo) || nuevo.contains(char::is_whitespace)) && texto::clase(k).is_none() && !texto::protegido(k) {
                    nuevo = format!("clave_{}", out.len() + 1);
                }
                while out.contains_key(&nuevo) {
                    nuevo.push('_');
                }
                textos.push((nuevo.clone(), false));
                out.insert(nuevo, json(sesion, v, k, prof + 1, textos)?);
            }
            Value::Object(out)
        }
        Value::Array(xs) => Value::Array(xs.iter().map(|v| json(sesion, v, key, prof + 1, textos)).collect::<Result<Vec<_>, _>>()?),
        Value::String(s) => {
            let salida = if let Some(interno) = serde_json::from_str::<Value>(s).ok().filter(|v| v.is_object() || v.is_array()) {
                serde_json::to_string(&json(sesion, &interno, "", prof + 1, textos)?).map_err(|_| "No se pudo reconstruir JSON anidado.")?
            } else {
                sesion.valor(key, s)?
            };
            textos.push((salida.clone(), texto::protegido(key)));
            Value::String(salida)
        }
        Value::Number(n)
            if texto::clase(key).is_some_and(|c| matches!(c, Clase::Rut | Clase::Identificador | Clase::Tecnico | Clase::Huella))
                || n.to_string().trim_start_matches('-').len() >= 7 =>
        {
            Value::String(sesion.valor(if texto::clase(key).is_some() { key } else { "identificador" }, &n.to_string())?)
        }
        other => other.clone(),
    })
}

pub(crate) fn tabla(sesion: &mut Sesion, filas: &[Vec<Celda>], textos: &mut Vec<(String, bool)>) -> Result<Vec<Vec<Celda>>, String> {
    let encabezado = cabecera(filas);
    let mut out = vec![];
    for (r, fila) in filas.iter().enumerate() {
        let mut nueva = vec![];
        let mut clase_par = None;
        for (c, valor) in fila.iter().enumerate() {
            let key = encabezado
                .filter(|h| r > *h)
                .and_then(|h| filas[h].get(c))
                .map(Celda::texto)
                .or_else(|| if valor.es_vacia() { None } else { clase_par.take() })
                .unwrap_or_default();
            let protegida = texto::protegido(&key);
            let transformada = match valor {
                Celda::Vacia => Celda::Vacia,
                Celda::Texto(s) if Some(r) == encabezado => Celda::Texto(
                    if texto::clase(s).is_none()
                        && !texto::protegido(s)
                        && (!texto::estructural(s))
                        && (sesion.opciones.modo == crate::Modo::Estricto || texto::parece_nombre(s))
                    {
                        format!("CAMPO {}", c + 1)
                    } else {
                        sesion.texto(s, false, true)?
                    },
                ),
                Celda::Texto(s)
                    if key.is_empty()
                        && encabezado.is_none_or(|h| r < h)
                        && (s.trim_end().ends_with(':') || s.split_whitespace().count() <= 3)
                        && (texto::clase(s).is_some() || texto::protegido(s)) =>
                {
                    clase_par = Some(s.trim_end_matches(':').to_string());
                    Celda::Texto(sesion.texto(s, false, true)?)
                }
                Celda::Texto(s) if matches!(texto::clase(&key), Some(Clase::Json)) => {
                    let v = serde_json::from_str(s).map_err(|_| "Una columna JSON no se reconoce.")?;
                    Celda::Texto(
                        serde_json::to_string(&json(sesion, &v, "", 0, textos)?).map_err(|_| "No se pudo reconstruir una columna JSON.")?,
                    )
                }
                Celda::Texto(s) => Celda::Texto(sesion.valor(&key, s)?),
                Celda::Numero(n) if !n.is_finite() => return Err("Un número no es finito.".into()),
                Celda::Numero(n)
                    if texto::clase(&key)
                        .is_some_and(|c| matches!(c, Clase::Rut | Clase::Tecnico | Clase::Identificador | Clase::Huella))
                        || n.abs() >= 1_000_000.0 =>
                {
                    Celda::Texto(sesion.valor(if texto::clase(&key).is_some() { &key } else { "identificador" }, &valor.texto())?)
                }
                Celda::Fecha(f) => {
                    let s = sesion.desplazar(&f.to_string(), texto::normal(&key).contains("nacimiento"));
                    Celda::Fecha(s.parse().map_err(|_| "Fecha desplazada fuera de rango.")?)
                }
                other => other.clone(),
            };
            textos.push((transformada.texto(), protegida));
            nueva.push(transformada);
        }
        out.push(nueva);
    }
    Ok(out)
}

pub(crate) fn nombres_hojas(hojas: &[Hoja]) -> Vec<String> {
    let mut usados = BTreeSet::new();
    hojas
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let mut nombre = if texto::titulo_seguro(&h.nombre) { h.nombre.clone() } else { format!("Hoja{}", i + 1) };
            let mut n = i + 1;
            while !usados.insert(nombre.to_lowercase()) {
                n += 1;
                nombre = format!("Hoja{n}");
            }
            nombre
        })
        .collect()
}
fn xlsx(hojas: &[Hoja]) -> Result<Vec<u8>, String> {
    let mut libro = rust_xlsxwriter::Workbook::new();
    for h in hojas {
        let hoja = libro.add_worksheet();
        hoja.set_name(&h.nombre).map_err(|_| "No se pudo crear una hoja de salida.")?;
        for (r, fila) in h.filas.iter().enumerate() {
            for (c, valor) in fila.iter().enumerate() {
                if r > 1_048_575 || c > 16_383 {
                    return Err("La tabla supera las dimensiones de Excel.".into());
                }
                let resultado = match valor {
                    Celda::Vacia => continue,
                    Celda::Texto(s) => hoja.write_string(r as u32, c as u16, s),
                    Celda::Numero(n) => hoja.write_number(r as u32, c as u16, *n),
                    Celda::Fecha(f) => hoja.write_string(r as u32, c as u16, f.to_string()),
                };
                resultado.map_err(|_| "No se pudo escribir una celda de salida.")?;
            }
        }
    }
    libro.save_to_buffer().map_err(|_| "No se pudo reconstruir el libro de salida.".into())
}
pub fn transformar(sesion: &mut Sesion, entrada: &Entrada) -> Result<Salida, String> {
    recolectar(sesion, entrada)?;
    sesion.verificar(&[])?;
    let mut textos = vec![];
    let (datos, extension) = match entrada {
        Entrada::Zip(a) => (crate::archivos::transformar(sesion, a, &mut textos)?, "zip"),
        Entrada::Html(b) => (crate::documentos::html(sesion, b, &mut textos)?, "html"),
        Entrada::Ics(l) => (crate::calendario::transformar(sesion, l, &mut textos)?, "ics"),
        Entrada::Word(p) => (crate::documentos::docx(sesion, p, &mut textos)?, "docx"),
        Entrada::Json(v) => {
            (serde_json::to_vec_pretty(&json(sesion, v, "", 0, &mut textos)?).map_err(|_| "No se pudo reconstruir JSON.")?, "json")
        }
        Entrada::JsonLineas(xs) => {
            let mut s = String::new();
            for v in xs {
                s.push_str(&serde_json::to_string(&json(sesion, v, "", 0, &mut textos)?).map_err(|_| "No se pudo reconstruir JSONL.")?);
                s.push('\n');
            }
            (s.into_bytes(), "jsonl")
        }
        Entrada::Texto(s, ext) => {
            let mut out = String::new();
            for l in s.lines() {
                let nuevo = if let Some((label, valor)) = par(l) {
                    format!("{label}: {}", sesion.valor(label, valor)?)
                } else {
                    sesion.valor("", l)?
                };
                textos.push((nuevo.clone(), false));
                out.push_str(&nuevo);
                out.push('\n');
            }
            (out.into_bytes(), ext.as_str())
        }
        Entrada::Csv(rows, delim) => {
            let filas = rows.iter().map(|r| r.iter().cloned().map(Celda::Texto).collect()).collect::<Vec<_>>();
            let filas = tabla(sesion, &filas, &mut textos)?;
            let mut writer = csv::WriterBuilder::new().delimiter(*delim).from_writer(vec![]);
            for f in filas {
                let f = f
                    .iter()
                    .map(Celda::texto)
                    .map(|s| {
                        if s.starts_with(['=', '+', '-', '@', '\t', '\r']) && s.parse::<f64>().ok().is_none_or(|n| !n.is_finite()) {
                            format!("'{s}")
                        } else {
                            s
                        }
                    })
                    .collect::<Vec<_>>();
                writer.write_record(f).map_err(|_| "No se pudo reconstruir CSV.")?;
            }
            (writer.into_inner().map_err(|_| "No se pudo cerrar CSV.")?, if *delim == b'\t' { "tsv" } else { "csv" })
        }
        Entrada::Excel(libro, _) => {
            let mut hojas = vec![];
            let nombres = nombres_hojas(&libro.hojas);
            for (h, nombre) in libro.hojas.iter().zip(nombres) {
                textos.push((nombre.clone(), true));
                hojas.push(Hoja { nombre, filas: tabla(sesion, &h.filas, &mut textos)? });
            }
            (xlsx(&hojas)?, "xlsx")
        }
    };
    sesion.verificar(&textos)?;
    Ok(Salida { datos, extension: extension.into(), avisos: sesion.avisos.iter().cloned().collect() })
}
pub fn anonimizar(nombre: &str, datos: &[u8], opciones: Opciones) -> Result<Salida, String> {
    let entrada = cargar(nombre, datos)?;
    let mut sesion = Sesion::nueva(opciones)?;
    transformar(&mut sesion, &entrada)
}
