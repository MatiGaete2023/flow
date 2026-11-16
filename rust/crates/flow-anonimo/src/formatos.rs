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

/// Tipo original de una celda de Excel que `Celda` no representa: se conserva al escribir la copia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tipo {
    Logico(bool),
    FechaHora(chrono::NaiveDateTime),
}
/// (fila, columna) -> tipo original, por hoja.
pub type Tipos = std::collections::BTreeMap<(usize, usize), Tipo>;

pub enum Entrada {
    /// JSON y extensión de salida (`json`, `jsonl` o `ndjson`).
    Json(Value, String),
    JsonLineas(Vec<Value>, String),
    Texto(String, String),
    Csv(Vec<Vec<String>>, u8),
    Excel(Libro, bool, Vec<Tipos>),
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
    let (mut hojas, mut formulas, mut celdas, mut tipos) = (vec![], false, 0usize, vec![]);
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
        let mut tipos_hoja = Tipos::new();
        celdas += ancho.saturating_mul(inicio_fila as usize);
        for fila in rango.rows() {
            let r = filas.len();
            for (c, d) in fila.iter().enumerate() {
                let c = c + inicio_col as usize;
                match d {
                    Data::Bool(b) => {
                        tipos_hoja.insert((r, c), Tipo::Logico(*b));
                    }
                    Data::DateTime(d) if !d.is_duration() => {
                        if let Some(d) = d.as_datetime() {
                            tipos_hoja.insert((r, c), Tipo::FechaHora(d));
                        }
                    }
                    _ => {}
                }
            }
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
        tipos.push(tipos_hoja);
    }
    Ok(Entrada::Excel(
        Libro {
            nombre: nombre.into(),
            sha256: format!("{:x}", Sha256::digest(datos)),
            formato: if biff { flow_tabular::Formato::XlsBiff } else { flow_tabular::Formato::Xlsx },
            hojas,
        },
        formulas,
        tipos,
    ))
}

pub fn decodificar(datos: &[u8]) -> Result<String, String> {
    decodificar_con(datos, true)
}
/// Texto, CSV y HTML: como Python, admite caracteres de control salvo NUL (los saltos de página se tratan como líneas).
fn decodificar_texto(datos: &[u8]) -> Result<String, String> {
    decodificar_con(datos, false)
}
fn decodificar_con(datos: &[u8], estricto: bool) -> Result<String, String> {
    let control = |c: char| c == '\0' || (estricto && c.is_control() && !matches!(c, '\t' | '\r' | '\n'));
    let (encoding, contenido) = if datos.starts_with(&[0xff, 0xfe]) {
        (encoding_rs::UTF_16LE, &datos[2..])
    } else if datos.starts_with(&[0xfe, 0xff]) {
        (encoding_rs::UTF_16BE, &datos[2..])
    } else {
        if let Ok(s) = std::str::from_utf8(datos) {
            if s.chars().any(control) {
                return Err("El texto contiene controles no admitidos.".into());
            }
            return Ok(s.trim_start_matches('\u{feff}').to_string());
        }
        (encoding_rs::WINDOWS_1252, datos)
    };
    let (s, mal) = encoding.decode_without_bom_handling(contenido);
    if mal || s.chars().any(control) {
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

/// `safezip.safe_name`: `\\` como `/`, sin `..`, sin raíz absoluta ni unidad (`:` en el primer segmento), sin `.` ni vacíos.
fn nombre_zip(nombre: &str) -> Option<String> {
    let limpio = nombre.replace('\\', "/");
    if limpio.starts_with('/') || limpio.contains('\0') || limpio.split('/').any(|p| p == "..") {
        return None;
    }
    let partes = limpio.split('/').filter(|p| !p.is_empty() && *p != ".").collect::<Vec<_>>();
    if partes.is_empty() || partes[0].contains(':') {
        return None;
    }
    Some(partes.join("/"))
}
pub fn validar_zip(datos: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(datos)).map_err(|_| "El ZIP no se reconoce.")?;
    if zip.len() > 4000 {
        return Err("El ZIP contiene demasiados archivos.".into());
    }
    let (mut total, mut vistos, mut out) = (0u64, BTreeSet::new(), vec![]);
    for i in 0..zip.len() {
        let mut miembro = zip.by_index(i).map_err(|_| "El ZIP está cifrado o contiene una entrada no admitida.")?;
        if miembro.is_dir() {
            continue;
        }
        let nombre = nombre_zip(miembro.name()).ok_or("El ZIP contiene rutas inseguras.")?;
        if !vistos.insert(nombre.to_lowercase()) {
            return Err("El ZIP contiene rutas repetidas.".into());
        }
        if miembro.unix_mode().is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000) {
            return Err("El ZIP contiene enlaces u objetos especiales.".into());
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
    let s = decodificar_texto(datos)?;
    let inicio = s.trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n']);
    let minus = inicio.chars().take(4096).collect::<String>().to_ascii_lowercase();
    if minus.starts_with("<?xml") || minus.starts_with("<workbook") || minus.starts_with("<ss:workbook") {
        if !xml_seguro(datos) {
            return Err("El XML contiene declaraciones no admitidas.".into());
        }
        static TIPOS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"((?:ss:)?Type\s*=\s*["'])DateTime(["'])"#).unwrap());
        let xml = TIPOS.replace_all(&s, "${1}String${2}");
        return flow_tabular::leer_bytes(nombre, xml.as_bytes().to_vec())
            .map(|l| Entrada::Excel(l, s.contains("Formula="), vec![]))
            .map_err(|_| "No se pudo leer Excel XML.".into());
    }
    // `formats.sniff`: HTML si empieza con «<» y nombra una etiqueta de documento en los primeros 4096 bytes.
    static HTML: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<(?:!doctype html|html|table|body|head|div|p)\b").unwrap());
    if minus.starts_with('<') && HTML.is_match(&minus) {
        if matches!(ext.as_str(), "xls" | "xlsx" | "xlsm") {
            // SITFA guarda tablas HTML con extensión .xls; sin tablas, el archivo se trata como documento.
            if let Ok(l) = flow_tabular::leer_bytes(nombre, datos.to_vec()) {
                if !l.hojas.is_empty() {
                    return Ok(Entrada::Excel(l, false, vec![]));
                }
            }
        }
        return crate::documentos::leer_html(&s).map(Entrada::Html);
    }
    let sin_extension = ext.is_empty();
    if matches!(ext.as_str(), "json" | "jsonl" | "ndjson") || sin_extension && inicio.starts_with(['{', '[']) {
        return leer_json(&s, if sin_extension { "json" } else { &ext });
    }
    if ext == "ics" || minus.starts_with("begin:vcalendar") {
        return crate::calendario::lineas(&s).map(Entrada::Ics);
    }
    if matches!(ext.as_str(), "csv" | "tsv") {
        let cabecera = s.lines().next().unwrap_or("");
        let cuenta = |c: u8| cabecera.bytes().filter(|b| *b == c).count();
        // Como `csv.Sniffer` con «,;\t|»: el separador más frecuente en la primera línea; con empate, la coma.
        let delimitador = if ext == "tsv" {
            b'\t'
        } else {
            b";\t|".iter().copied().fold(b',', |mejor, c| if cuenta(c) > cuenta(mejor) { c } else { mejor })
        };
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
    if matches!(ext.as_str(), "txt" | "md" | "log" | "text") || sin_extension {
        return Ok(Entrada::Texto(s, if sin_extension { "txt".into() } else { ext }));
    }
    Err("Formato no admitido; el archivo original se conserva.".into())
}

/// `json.loads(..., parse_constant=None)`: NaN, Infinity y -Infinity fuera de cadenas pasan a `null`.
fn sin_constantes(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains("NaN") && !s.contains("Infinity") {
        return s.into();
    }
    let (mut out, mut cadena, mut escape, mut i) = (String::with_capacity(s.len()), false, false, 0);
    let b = s.as_bytes();
    while i < b.len() {
        let c = b[i];
        if cadena {
            if escape {
                escape = false;
            } else if c == b'\\' {
                escape = true;
            } else if c == b'"' {
                cadena = false;
            }
        } else if c == b'"' {
            cadena = true;
        } else if let Some(t) = ["-Infinity", "Infinity", "NaN"].iter().find(|t| s[i..].starts_with(**t)) {
            let antes = i.checked_sub(1).map(|j| b[j]);
            let despues = b.get(i + t.len()).copied();
            if !antes.is_some_and(|c| c.is_ascii_alphanumeric()) && !despues.is_some_and(|c| c.is_ascii_alphanumeric()) {
                out.push_str("null");
                i += t.len();
                continue;
            }
        }
        // Se copian bytes completos de UTF-8: las marcas buscadas son ASCII y nunca cortan un carácter.
        let largo = s[i..].chars().next().map(char::len_utf8).unwrap_or(1);
        out.push_str(&s[i..i + largo]);
        i += largo;
    }
    out.into()
}
/// `jsonfmt.anonymize_bytes`: JSON completo o, si no lo es y hay al menos dos líneas, JSON por líneas.
fn leer_json(s: &str, ext: &str) -> Result<Entrada, String> {
    let lineas = || -> Result<Vec<Value>, String> {
        let mut values = vec![];
        for linea in s.lines().filter(|l| !l.trim().is_empty()) {
            if values.len() >= 2_000_000 {
                return Err("JSONL contiene demasiadas entradas.".into());
            }
            let v = serde_json::from_str(&sin_constantes(linea)).map_err(|_| "JSONL inválido.")?;
            limite_json(&v)?;
            values.push(v);
        }
        Ok(values)
    };
    let entero = || serde_json::from_str::<Value>(&sin_constantes(s)).map_err(|_| String::from("JSON inválido."));
    if matches!(ext, "jsonl" | "ndjson") {
        if let Ok(v) = lineas() {
            return Ok(Entrada::JsonLineas(v, ext.into()));
        }
        return entero().map(|v| Entrada::Json(v, ext.into()));
    }
    match entero() {
        Ok(v) => Ok(Entrada::Json(v, ext.into())),
        Err(e) if s.lines().filter(|l| !l.trim().is_empty()).count() < 2 => Err(e),
        Err(_) => lineas().map(|v| Entrada::JsonLineas(v, ext.into())),
    }
}

fn tipo_persona(c: Option<Clase>) -> bool {
    matches!(c, Some(Clase::Persona | Clase::Curador))
}
/// `grid.looks_like_person`: celda con aspecto de nombre propio que no es un título de columna conocido.
fn parece_persona(s: &str) -> bool {
    texto::clase(s).is_none() && !texto::protegido(s) && texto::parece_identidad(s)
}
/// `grid.header_row`: primera fila (de 60) con al menos dos títulos de texto de hasta 40 caracteres, sin pares
/// «Etiqueta:» ni nombres propios, y con dos columnas de identidad o un título RIT/RUT/RUN/Nombre….
fn cabecera(filas: &[Vec<Celda>]) -> Option<usize> {
    filas.iter().take(60).position(|fila| {
        let textos = fila
            .iter()
            .filter_map(|c| match c {
                Celda::Texto(s) if !s.trim().is_empty() => Some(s.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let nombres = textos.iter().map(|s| texto::normal(s)).collect::<Vec<_>>();
        if nombres.len() < 2 || nombres.iter().any(|n| n.chars().count() > 40) {
            return false;
        }
        if textos.iter().any(|s| s.trim().ends_with(':') || parece_persona(s)) {
            return false;
        }
        let identidad =
            textos.iter().filter(|s| matches!(texto::clase(s), Some(Clase::Persona | Clase::Curador | Clase::Rut | Clase::Rit))).count();
        identidad >= 2
            || nombres
                .iter()
                .any(|n| ["rit", "rut", "run"].contains(&n.as_str()) || ["nombre", "rut ", "rit ", "run "].iter().any(|p| n.starts_with(p)))
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
/// `textfmt.KV_RE`: etiqueta de 2 a 40 caracteres hasta el primer «:» o «=».
fn par(linea: &str) -> Option<(&str, &str)> {
    let corte = linea.find([':', '='])?;
    let (label, valor) = (linea[..corte].trim(), &linea[corte + 1..]);
    let largo = label.chars().count();
    ((2..=40).contains(&largo) && (texto::clase(label).is_some() || texto::protegido(label) || texto::credencial(label)))
        .then_some((label, valor.trim()))
}
/// `str.splitlines` de Python: también separa en \x0b, \x0c, \x1c-\x1e, \x85, U+2028 y U+2029.
pub(crate) fn lineas_texto(s: &str) -> Vec<&str> {
    let mut out = vec![];
    let mut inicio = 0;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if matches!(c, '\n' | '\r' | '\x0b' | '\x0c' | '\x1c' | '\x1d' | '\x1e' | '\u{85}' | '\u{2028}' | '\u{2029}') {
            out.push(&s[inicio..i]);
            inicio = i + c.len_utf8();
            if c == '\r' && chars.peek().is_some_and(|(_, n)| *n == '\n') {
                chars.next();
                inicio += 1;
            }
        }
    }
    if inicio < s.len() {
        out.push(&s[inicio..]);
    }
    out
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
        // La etiqueta se publica: si contiene un RIT, RUT, correo o nombre conocido, la línea es texto libre.
        if sesion.texto(label, false, true)? == label {
            return Ok(format!("{label}: {}", sesion.valor(label, valor)?));
        }
    }
    sesion.valor("", l)
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
        Entrada::Json(v, _) => seed_json(sesion, v, "", 0)?,
        Entrada::JsonLineas(xs, _) => {
            for v in xs {
                seed_json(sesion, v, "", 0)?;
            }
        }
        Entrada::Texto(s, _) => {
            for l in lineas_texto(s) {
                semillar_linea(sesion, l);
            }
        }
        Entrada::Csv(rows, _) => {
            seed_tabla(sesion, &rows.iter().map(|r| r.iter().cloned().map(Celda::Texto).collect()).collect::<Vec<_>>())?
        }
        Entrada::Excel(libro, formulas, _) => {
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
    tabla_tipada(sesion, filas, &Tipos::new(), &mut Tipos::new(), textos)
}
/// Segunda pasada de una tabla. Las celdas con tipo original (fecha y hora, lógico) conservan su tipo como Python:
/// la fecha solo se desplaza o generaliza y el lógico no cambia.
fn tabla_tipada(
    sesion: &mut Sesion,
    filas: &[Vec<Celda>],
    tipos: &Tipos,
    tipos_salida: &mut Tipos,
    textos: &mut Vec<(String, bool)>,
) -> Result<Vec<Vec<Celda>>, String> {
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
            // `grid.transform`: el encabezado se conserva y se revisa como celda protegida (solo nombres completos).
            let protegida = Some(r) == encabezado || texto::protegido(&key);
            if let Some(tipo) = tipos.get(&(r, c)) {
                let tipo = match *tipo {
                    Tipo::FechaHora(d) if Some(r) != encabezado => {
                        let nacimiento = encabezado.is_some_and(|h| r > h) && texto::normal(&key).contains("nacimiento");
                        Tipo::FechaHora(if nacimiento && sesion.opciones.generalizar_nacimiento {
                            chrono::NaiveDate::from_ymd_opt(chrono::Datelike::year(&d), 1, 1)
                                .unwrap_or(d.date())
                                .and_time(chrono::NaiveTime::MIN)
                        } else {
                            d.checked_add_signed(chrono::Duration::days(sesion.opciones.desplazar_dias))
                                .ok_or("Fecha desplazada fuera de rango.")?
                        })
                    }
                    otro => otro,
                };
                let celda = Celda::Texto(match tipo {
                    Tipo::Logico(b) => b.to_string(),
                    Tipo::FechaHora(d) => d.format("%Y-%m-%dT%H:%M:%S").to_string(),
                });
                tipos_salida.insert((r, c), tipo);
                textos.push((celda.texto(), protegida));
                nueva.push(celda);
                continue;
            }
            let transformada = match valor {
                Celda::Vacia => Celda::Vacia,
                Celda::Texto(s) if Some(r) == encabezado => {
                    Celda::Texto(if parece_persona(s) { format!("CAMPO {}", c + 1) } else { sesion.texto(s, false, true)? })
                }
                Celda::Texto(s)
                    if key.is_empty()
                        && encabezado.is_none_or(|h| r < h)
                        && (s.trim_end().ends_with(':') || s.split_whitespace().count() <= 3)
                        && (texto::clase(s).is_some() || texto::protegido(s)) =>
                {
                    clase_par = Some(s.trim_end_matches(':').to_string());
                    Celda::Texto(sesion.texto(s, false, true)?)
                }
                Celda::Texto(s) if matches!(texto::clase(&key), Some(Clase::Json)) => match serde_json::from_str(&sin_constantes(s)) {
                    Ok(v) => Celda::Texto(
                        serde_json::to_string(&json(sesion, &v, "", 0, textos)?).map_err(|_| "No se pudo reconstruir una columna JSON.")?,
                    ),
                    // `jsonfmt.text`: lo que no es JSON válido se trata como texto libre.
                    Err(_) => Celda::Texto(sesion.texto(s, true, false)?),
                },
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
fn xlsx(hojas: &[Hoja], tipos: &[Tipos]) -> Result<Vec<u8>, String> {
    let mut libro = rust_xlsxwriter::Workbook::new();
    let (fecha, fecha_hora) =
        (rust_xlsxwriter::Format::new().set_num_format("yyyy-mm-dd"), rust_xlsxwriter::Format::new().set_num_format("yyyy-mm-dd hh:mm:ss"));
    let vacio = Tipos::new();
    for (i, h) in hojas.iter().enumerate() {
        let tipos = tipos.get(i).unwrap_or(&vacio);
        let hoja = libro.add_worksheet();
        hoja.set_name(&h.nombre).map_err(|_| "No se pudo crear una hoja de salida.")?;
        for (r, fila) in h.filas.iter().enumerate() {
            for (c, valor) in fila.iter().enumerate() {
                if r > 1_048_575 || c > 16_383 {
                    return Err("La tabla supera las dimensiones de Excel.".into());
                }
                let resultado = match (tipos.get(&(r, c)), valor) {
                    (Some(Tipo::Logico(b)), _) => hoja.write_boolean(r as u32, c as u16, *b),
                    (Some(Tipo::FechaHora(d)), _) => {
                        use chrono::{Datelike, Timelike};
                        let dia = rust_xlsxwriter::ExcelDateTime::from_ymd(d.year() as u16, d.month() as u8, d.day() as u8)
                            .map_err(|_| "Fecha fuera del rango de Excel.")?;
                        if d.time() == chrono::NaiveTime::MIN {
                            hoja.write_datetime_with_format(r as u32, c as u16, &dia, &fecha)
                        } else {
                            let completa = dia
                                .and_hms(d.hour() as u16, d.minute() as u8, d.second() as f64)
                                .map_err(|_| "Hora fuera del rango de Excel.")?;
                            hoja.write_datetime_with_format(r as u32, c as u16, &completa, &fecha_hora)
                        }
                    }
                    (None, valor) => match valor {
                        Celda::Vacia => continue,
                        Celda::Texto(s) => hoja.write_string(r as u32, c as u16, s),
                        Celda::Numero(n) => hoja.write_number(r as u32, c as u16, *n),
                        Celda::Fecha(f) => hoja.write_string(r as u32, c as u16, f.to_string()),
                    },
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
        Entrada::Json(v, ext) => {
            (serde_json::to_vec_pretty(&json(sesion, v, "", 0, &mut textos)?).map_err(|_| "No se pudo reconstruir JSON.")?, ext.as_str())
        }
        Entrada::JsonLineas(xs, ext) => {
            let mut s = String::new();
            for v in xs {
                s.push_str(&serde_json::to_string(&json(sesion, v, "", 0, &mut textos)?).map_err(|_| "No se pudo reconstruir JSONL.")?);
                s.push('\n');
            }
            (s.into_bytes(), ext.as_str())
        }
        Entrada::Texto(s, ext) => {
            let mut out = String::new();
            for l in lineas_texto(s) {
                let nuevo = transformar_linea(sesion, l)?;
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
        Entrada::Excel(libro, _, tipos) => {
            let (mut hojas, mut tipos_salida) = (vec![], vec![]);
            let nombres = nombres_hojas(&libro.hojas);
            let vacio = Tipos::new();
            for (i, (h, nombre)) in libro.hojas.iter().zip(nombres).enumerate() {
                textos.push((nombre.clone(), true));
                let mut salida = Tipos::new();
                hojas.push(Hoja {
                    nombre,
                    filas: tabla_tipada(sesion, &h.filas, tipos.get(i).unwrap_or(&vacio), &mut salida, &mut textos)?,
                });
                tipos_salida.push(salida);
            }
            (xlsx(&hojas, &tipos_salida)?, "xlsx")
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
