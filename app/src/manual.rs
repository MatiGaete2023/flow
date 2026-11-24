//! Correo y Word manuales, independientes de cualquier Excel (`manual.py`, `word_quality.py` y `outputs.fill_docx` de Flow
//! Python): un correo libre con adjuntos, un Word de texto libre o con una matriz `{{CAMPO}}`, bloques de texto favoritos
//! con variables y la recuperación de lo que se estaba escribiendo.

use crate::config::{self, atomic_write};
use crate::personalizar;
use crate::productos::{nueva_ruta, Borrador};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Variables de un bloque favorito (`manual.BLOCK_FIELDS`).
pub const CAMPOS_BLOQUE: [&str; 5] = ["NOMBRE", "RIT", "TRIBUNAL", "PROGRAMA", "FECHA"];

pub fn variables_bloque(texto: &str) -> Result<BTreeSet<String>, String> {
    personalizar::variables(texto, &CAMPOS_BLOQUE)
}

/// `render_block`: el bloque con sus variables completas.
pub fn render_bloque(texto: &str, valores: &BTreeMap<String, String>) -> Result<String, String> {
    let necesarias = variables_bloque(texto)?;
    if necesarias.iter().any(|k| valores.get(k).is_none_or(|v| v.trim().is_empty())) {
        return Err("Completa todas las variables del bloque favorito.".into());
    }
    let c: Vec<char> = texto.chars().collect();
    let (mut i, mut salida) = (0, String::new());
    while i < c.len() {
        match c[i] {
            '{' if c.get(i + 1) == Some(&'{') => {
                salida.push('{');
                i += 2;
            }
            '}' if c.get(i + 1) == Some(&'}') => {
                salida.push('}');
                i += 2;
            }
            '{' => {
                let fin = c[i..].iter().position(|x| *x == '}').map_or(c.len(), |p| i + p);
                let nombre: String = c[i + 1..fin].iter().collect();
                salida.push_str(valores.get(&nombre).map_or("", String::as_str));
                i = fin + 1;
            }
            otro => {
                salida.push(otro);
                i += 1;
            }
        }
    }
    Ok(salida)
}

/// `word_quality.unresolved_text`: campos `{{X}}` o `[COMPLETAR …]` que siguen sin completarse.
pub fn pendientes(texto: &str) -> Vec<String> {
    let marcas = Regex::new(r"(?i)\{\{[^{}]+\}\}|\[\s*COMPLETAR[^\]]*\]").unwrap();
    marcas.find_iter(texto).map(|m| m.as_str().to_string()).collect::<BTreeSet<_>>().into_iter().collect()
}

fn desescapar(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

fn escapar(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn parrafos_xml(xml: &str) -> Vec<(usize, usize)> {
    Regex::new(r"(?s)<w:p[ >].*?</w:p>").unwrap().find_iter(xml).map(|m| (m.start(), m.end())).collect()
}

fn nodos_texto(parrafo: &str) -> Vec<(usize, usize, String)> {
    // (inicio y fin del contenido dentro del párrafo, texto sin escapes)
    let re = Regex::new(r"(?s)<w:t(?: [^>]*)?>(.*?)</w:t>").unwrap();
    re.captures_iter(parrafo)
        .map(|c| {
            let g = c.get(1).unwrap();
            (g.start(), g.end(), desescapar(g.as_str()))
        })
        .collect()
}

fn partes_word(paquete: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(paquete)).map_err(|_| "La matriz no es un documento Word válido.".to_string())?;
    let mut v = Vec::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|_| "La matriz Word está dañada.".to_string())?;
        let mut d = Vec::new();
        f.read_to_end(&mut d).map_err(|_| "La matriz Word está dañada.".to_string())?;
        v.push((f.name().to_string(), d));
    }
    if !v.iter().any(|(n, _)| n == "word/document.xml") {
        return Err("La matriz no contiene word/document.xml.".into());
    }
    Ok(v)
}

fn es_parte_word(nombre: &str) -> bool {
    nombre.starts_with("word/") && nombre.ends_with(".xml")
}

/// `template_variables`: los `{{CAMPO}}` de una matriz, también cuando Word los partió en varios fragmentos.
pub fn variables_matriz(paquete: &[u8]) -> Result<BTreeSet<String>, String> {
    let campo = Regex::new(r"\{\{([A-Z_]+)\}\}").unwrap();
    let mut r = BTreeSet::new();
    for (nombre, datos) in partes_word(paquete)? {
        if !es_parte_word(&nombre) {
            continue;
        }
        let xml = String::from_utf8_lossy(&datos).to_string();
        for (a, b) in parrafos_xml(&xml) {
            let texto: String = nodos_texto(&xml[a..b]).into_iter().map(|n| n.2).collect();
            r.extend(campo.captures_iter(&texto).map(|c| c[1].to_string()));
        }
    }
    Ok(r)
}

/// Texto de cada párrafo del documento principal.
#[cfg(test)]
pub fn textos_parrafos(paquete: &[u8]) -> Result<Vec<String>, String> {
    let (_, datos) = partes_word(paquete)?.into_iter().find(|(n, _)| n == "word/document.xml").ok_or("Sin documento.")?;
    let xml = String::from_utf8_lossy(&datos).to_string();
    Ok(parrafos_xml(&xml).into_iter().map(|(a, b)| nodos_texto(&xml[a..b]).into_iter().map(|n| n.2).collect()).collect())
}

/// `fill_docx`: sustituye los campos entre fragmentos sin rehacer párrafos ni perder estilos.
fn rellenar_parrafo(parrafo: &str, valores: &BTreeMap<String, String>) -> String {
    let campo = Regex::new(r"\{\{([A-Z_]+)\}\}").unwrap();
    let mut nodos = nodos_texto(parrafo);
    let completo: String = nodos.iter().map(|n| n.2.as_str()).collect();
    let mut tocados = BTreeSet::new();
    let coincidencias: Vec<_> = campo.captures_iter(&completo).map(|c| (c.get(0).unwrap().start(), c.get(0).unwrap().end(), c[1].to_string())).collect();
    for (ini, fin, clave) in coincidencias.into_iter().rev() {
        let reemplazo = valores.get(&clave).map_or("", |v| v.trim()).to_string();
        // Posiciones en bytes dentro del texto concatenado de los nodos.
        let (mut pos, mut afectados) = (0usize, Vec::new());
        for (i, n) in nodos.iter().enumerate() {
            let (a, b) = (pos, pos + n.2.len());
            if a < fin && b > ini {
                afectados.push((i, a, b));
            }
            pos = b;
        }
        let Some(&(primero, a, _)) = afectados.first() else { continue };
        let &(ultimo, la, _) = afectados.last().unwrap();
        let prefijo = nodos[primero].2[..ini - a].to_string();
        let sufijo = nodos[ultimo].2[fin - la..].to_string();
        nodos[primero].2 = format!("{prefijo}{reemplazo}{}", if primero == ultimo { sufijo.clone() } else { String::new() });
        for &(i, _, _) in afectados.iter().skip(1) {
            nodos[i].2 = if i == ultimo { sufijo.clone() } else { String::new() };
        }
        tocados.extend(afectados.iter().map(|x| x.0));
    }
    // Reconstruye el párrafo reemplazando el contenido de cada nodo de atrás hacia adelante.
    let originales = nodos_texto(parrafo);
    let mut salida = parrafo.to_string();
    for (i, (ini, fin, _)) in originales.iter().enumerate().rev() {
        salida.replace_range(*ini..*fin, &escapar(&nodos[i].2));
        if tocados.contains(&i) {
            // El nodo debe conservar sus espacios.
            let apertura = salida[..*ini].rfind("<w:t").unwrap();
            let cierre = apertura + salida[apertura..].find('>').unwrap();
            if !salida[apertura..cierre].contains("xml:space") {
                salida.insert_str(cierre, " xml:space=\"preserve\"");
            }
        }
    }
    salida
}

fn rellenar_matriz(paquete: &[u8], valores: &BTreeMap<String, String>) -> Result<Vec<u8>, String> {
    let mut salida = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (nombre, mut datos) in partes_word(paquete)? {
        if es_parte_word(&nombre) {
            let xml = String::from_utf8(datos.clone()).map_err(|_| "La matriz no está en UTF-8.".to_string())?;
            let mut nuevo = xml.clone();
            for (a, b) in parrafos_xml(&xml).into_iter().rev() {
                let cambiado = rellenar_parrafo(&xml[a..b], valores);
                nuevo.replace_range(a..b, &cambiado);
            }
            datos = nuevo.into_bytes();
        }
        salida.start_file(nombre, o).map_err(|e| e.to_string())?;
        salida.write_all(&datos).map_err(|e| e.to_string())?;
    }
    Ok(salida.finish().map_err(|e| e.to_string())?.into_inner())
}

fn docx_libre(titulo: &str, cuerpo: &str) -> Result<Vec<u8>, String> {
    let parrafo = |t: &str, titulo: bool| {
        let (props, run) = if titulo {
            ("<w:pPr><w:jc w:val=\"center\"/></w:pPr>", "<w:rPr><w:b/><w:sz w:val=\"36\"/></w:rPr>")
        } else {
            ("", "")
        };
        format!("<w:p>{props}<w:r>{run}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>", escapar(t))
    };
    let mut cuerpo_xml = String::new();
    if !titulo.trim().is_empty() {
        cuerpo_xml.push_str(&parrafo(titulo, true));
    }
    for linea in cuerpo.split('\n') {
        cuerpo_xml.push_str(&parrafo(linea, false));
    }
    let doc = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{cuerpo_xml}<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1417\" w:right=\"1701\" w:bottom=\"1417\" w:left=\"1701\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>"
    );
    let ct = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (nombre, datos) in [("[Content_Types].xml", ct), ("_rels/.rels", rels), ("word/document.xml", doc.as_str())] {
        z.start_file(nombre, o).map_err(|e| e.to_string())?;
        z.write_all(datos.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(z.finish().map_err(|e| e.to_string())?.into_inner())
}

/// `generate_word`: un Word de texto libre o con una matriz, validado antes de escribir y sin reemplazar un archivo.
pub fn generar_word(destino: &Path, titulo: &str, cuerpo: &str, matriz: Option<&Path>, valores: &BTreeMap<String, String>) -> Result<PathBuf, String> {
    let bytes = match matriz {
        Some(m) => {
            let paquete = std::fs::read(m).map_err(|e| format!("No se pudo leer la matriz: {e}"))?;
            let faltan: Vec<String> = variables_matriz(&paquete)?.into_iter().filter(|k| valores.get(k).is_none_or(|v| v.trim().is_empty())).collect();
            if !faltan.is_empty() {
                return Err(format!("Completa los campos de la matriz: {}", faltan.join(", ")));
            }
            rellenar_matriz(&paquete, valores)?
        }
        None => {
            if cuerpo.trim().is_empty() {
                return Err("Escribe el contenido del documento.".into());
            }
            let faltan = pendientes(&format!("{titulo}\n{cuerpo}"));
            if !faltan.is_empty() {
                return Err(format!("Completa los campos Word pendientes: {}", faltan.join(", ")));
            }
            docx_libre(titulo, cuerpo)?
        }
    };
    if destino.exists() {
        return Err("El archivo ya existe; elige un destino nuevo.".into());
    }
    if let Some(d) = destino.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let temporal = destino.with_extension("tmp");
    std::fs::write(&temporal, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temporal, destino).map_err(|e| e.to_string())?;
    Ok(destino.to_path_buf())
}

/// Lo que se estaba escribiendo en los formularios manuales (`manual_actual.json`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actual {
    pub para: String,
    pub cc: String,
    pub asunto: String,
    pub cuerpo: String,
    pub adjuntos: Vec<String>,
    pub titulo_word: String,
    pub cuerpo_word: String,
    pub matriz: String,
    pub valores: BTreeMap<String, String>,
}

fn ruta_actual() -> PathBuf {
    config::data_dir().join("manual_actual.json")
}

fn ruta_favoritos() -> PathBuf {
    config::data_dir().join("bloques_favoritos.json")
}

impl Actual {
    pub fn cargar() -> Self {
        std::fs::read(ruta_actual()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn guardar(&self) -> Result<(), String> {
        let datos = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        atomic_write(&ruta_actual(), &datos).map_err(|e| e.to_string())
    }
}

pub fn favoritos() -> BTreeMap<String, String> {
    std::fs::read(ruta_favoritos()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Guarda un bloque favorito solo si sus variables son válidas.
pub fn guardar_favorito(nombre: &str, texto: &str) -> Result<(), String> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err("Indica un nombre para el bloque.".into());
    }
    variables_bloque(texto)?;
    let mut datos = favoritos();
    datos.insert(nombre.to_string(), texto.to_string());
    atomic_write(&ruta_favoritos(), &serde_json::to_vec_pretty(&datos).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn huella_archivo(ruta: &Path) -> Result<String, String> {
    let datos = std::fs::read(ruta).map_err(|e| format!("No se pudo leer el adjunto {}: {e}", ruta.display()))?;
    Ok(format!("{:x}", Sha256::digest(datos)))
}

/// Un correo manual como borrador: debe tener destinatarios válidos; sus adjuntos quedan con la huella de lo que se revisó.
pub fn borrador_manual(a: &Actual) -> Result<Borrador, String> {
    if personalizar::correos(&a.para)?.is_empty() {
        return Err("Indica al menos un destinatario.".into());
    }
    personalizar::correos(&a.cc)?;
    if a.asunto.trim().is_empty() {
        return Err("Indica el asunto.".into());
    }
    let mut adjuntos = Vec::new();
    for ruta in &a.adjuntos {
        adjuntos.push((ruta.clone(), huella_archivo(Path::new(ruta))?));
    }
    Ok(Borrador {
        kind: "manual".into(),
        titulo: format!("Manual · {}", a.asunto.trim()),
        para: a.para.clone(),
        cc: a.cc.clone(),
        asunto: a.asunto.clone(),
        cuerpo: a.cuerpo.clone(),
        incluir: true,
        adjuntos,
        ..Default::default()
    })
}

/// Comprueba que cada adjunto sea el mismo archivo que se revisó (`_reviewed_attachments`).
pub fn verificar_adjuntos(b: &Borrador) -> Result<(), String> {
    for (ruta, huella) in &b.adjuntos {
        if huella_archivo(Path::new(ruta))? != *huella {
            return Err("El adjunto cambió después de la revisión. Prepara una nueva versión.".into());
        }
    }
    Ok(())
}

/// Ruta nueva para un Word manual en una carpeta de productos.
pub fn ruta_word(dir: &Path) -> PathBuf {
    nueva_ruta(dir, "Documento_manual", "docx")
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::Value;

    fn texto(v: &Value) -> String {
        v.as_str().unwrap_or_default().to_string()
    }

    /// Una matriz con los párrafos y fragmentos (runs) que Python armó.
    fn matriz(parrafos: &Value) -> Vec<u8> {
        let mut cuerpo = String::new();
        for p in parrafos.as_array().unwrap() {
            cuerpo.push_str("<w:p>");
            for r in p.as_array().unwrap() {
                cuerpo.push_str(&format!("<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r>", escapar(r.as_str().unwrap())));
            }
            cuerpo.push_str("</w:p>");
        }
        let doc = format!("<?xml version=\"1.0\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{cuerpo}</w:body></w:document>");
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        z.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(doc.as_bytes()).unwrap();
        z.finish().unwrap().into_inner()
    }

    fn palabra(caso: &Value, raiz: &Path, n: usize) -> Vec<String> {
        let destino = raiz.join(format!("salida{n}.docx"));
        let ruta_matriz = raiz.join(format!("matriz{n}.docx"));
        let usa_matriz = caso["matriz"].is_array();
        if usa_matriz {
            std::fs::write(&ruta_matriz, matriz(&caso["matriz"])).unwrap();
        }
        let valores: BTreeMap<String, String> = caso["valores"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
        let r = generar_word(&destino, caso["titulo"].as_str().unwrap(), caso["cuerpo"].as_str().unwrap(), usa_matriz.then_some(ruta_matriz.as_path()), &valores);
        match (r, caso.get("error"), caso.get("parrafos")) {
            (Err(e), Some(esperado), _) if e == texto(esperado) && !destino.exists() => vec![],
            (Ok(_), None, Some(esperados)) => {
                let obtenidos = textos_parrafos(&std::fs::read(&destino).unwrap()).unwrap();
                let previstos: Vec<String> = esperados.as_array().unwrap().iter().map(texto).collect();
                if obtenidos == previstos { vec![] } else { vec![format!("párrafos Rust {obtenidos:?} · Python {previstos:?}")] }
            }
            (otro, e, p) => vec![format!("Rust {otro:?} · Python error {e:?} párrafos {p:?}")],
        }
    }

    fn bloque(caso: &Value) -> Vec<String> {
        let valores: Option<BTreeMap<String, String>> = caso["valores"].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), texto(v))).collect());
        let vars = variables_bloque(caso["texto"].as_str().unwrap());
        let render = valores.as_ref().map(|v| render_bloque(caso["texto"].as_str().unwrap(), v));
        match (vars, render, caso.get("error"), caso.get("variables"), caso.get("render")) {
            (Err(e), _, Some(esperado), _, _) if e == texto(esperado) => vec![],
            (Ok(_), Some(Err(e)), Some(esperado), _, _) if e == texto(esperado) => vec![],
            (Ok(v), r, None, Some(esperadas), esperado_render) => {
                let mut d = Vec::new();
                if v.iter().cloned().collect::<Vec<_>>() != esperadas.as_array().unwrap().iter().map(texto).collect::<Vec<_>>() {
                    d.push(format!("variables Rust {v:?} · Python {esperadas}"));
                }
                match (r, esperado_render) {
                    (Some(Ok(t)), Some(e)) if t == texto(e) => {}
                    (None, None) => {}
                    (otro, e) => d.push(format!("render Rust {otro:?} · Python {e:?}")),
                }
                d
            }
            (v, r, e, ..) => vec![format!("Rust {v:?}/{r:?} · Python error {e:?}")],
        }
    }

    /// E-T2: Word manual (libre y con matriz) y bloques favoritos de Rust contra `manual.generate_word`,
    /// `block_variables` y `render_block` de Flow Python (`app/qa/generar_personalizacion.py`).
    #[test]
    fn formularios_manuales_iguales_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/personalizacion.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let raiz = perfil.join("manual");
        let _ = std::fs::remove_dir_all(&raiz);
        std::fs::create_dir_all(&raiz).unwrap();
        let (mut total, mut fallos) = (0, Vec::new());
        for (i, c) in datos["palabras"].as_array().unwrap().iter().enumerate() {
            total += 1;
            let d = palabra(c, &raiz, i);
            if !d.is_empty() {
                fallos.push(format!("Word {i}:\n  {}", d.join("\n  ")));
            }
        }
        for (i, c) in datos["bloques"].as_array().unwrap().iter().enumerate() {
            total += 1;
            let d = bloque(c);
            if !d.is_empty() {
                fallos.push(format!("bloque {i}:\n  {}", d.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("MANUAL: {}/{total}, fallos {}", total - fallos.len(), fallos.len());
        let _ = std::fs::remove_dir_all(&raiz);
        assert_eq!(total, 19, "el oráculo debe traer 19 casos");
        assert!(fallos.is_empty());
    }

    /// Prueba permanente del oráculo: un párrafo esperado alterado debe detectarse.
    #[test]
    fn oraculo_del_word_manual_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/personalizacion.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let raiz = perfil.join("manual-mutacion");
        std::fs::create_dir_all(&raiz).unwrap();
        let mut caso = datos["palabras"][7].clone();
        assert!(palabra(&caso, &raiz, 0).is_empty());
        caso["parrafos"][0] = Value::String("otro".into());
        assert!(!palabra(&caso, &raiz, 1).is_empty(), "un párrafo esperado alterado no se detecta");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn un_correo_manual_exige_destinatario_y_conserva_la_huella_del_adjunto() {
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let adjunto = perfil.join("manual-adjunto.txt");
        std::fs::write(&adjunto, b"contenido revisado").unwrap();
        let mut a = Actual { asunto: "Asunto".into(), cuerpo: "Cuerpo".into(), adjuntos: vec![adjunto.display().to_string()], ..Default::default() };
        assert_eq!(borrador_manual(&a).unwrap_err(), "Indica al menos un destinatario.");
        a.para = "no es un correo".into();
        assert!(borrador_manual(&a).unwrap_err().starts_with("Dirección inválida"));
        a.para = "a@ejemplo.invalid; b@ejemplo.invalid".into();
        let b = borrador_manual(&a).unwrap();
        assert_eq!((b.kind.as_str(), b.adjuntos.len()), ("manual", 1));
        verificar_adjuntos(&b).unwrap();
        std::fs::write(&adjunto, b"otro contenido").unwrap();
        assert_eq!(verificar_adjuntos(&b).unwrap_err(), "El adjunto cambió después de la revisión. Prepara una nueva versión.");
        let _ = std::fs::remove_file(&adjunto);
    }

    #[test]
    fn lo_que_se_escribe_y_los_favoritos_se_recuperan() {
        let (_candado, _perfil) = crate::prueba_entorno::perfil_aislado();
        let a = Actual { para: "a@ejemplo.invalid".into(), asunto: "Hola".into(), titulo_word: "Título".into(), valores: BTreeMap::from([("NOMBRE".into(), "Ana".into())]), ..Default::default() };
        a.guardar().unwrap();
        assert_eq!(Actual::cargar(), a);
        assert!(guardar_favorito("", "x").is_err());
        assert!(guardar_favorito("Malo", "{DESCONOCIDA}").is_err());
        guardar_favorito("Saludo", "Hola {NOMBRE}").unwrap();
        assert_eq!(favoritos().get("Saludo").map(String::as_str), Some("Hola {NOMBRE}"));
    }
}
