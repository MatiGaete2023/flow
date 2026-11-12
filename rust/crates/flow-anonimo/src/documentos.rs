//! Reconstruye documentos desde texto visible: ningún atributo, objeto o relación del original se publica.
use crate::{formatos, texto::Sesion, MAX_ARCHIVO};
use flow_dominio::Celda;
use quick_xml::{events::Event, Reader};
use scraper::{ElementRef, Html, Selector};
use std::io::{Cursor, Write};

#[derive(Clone)]
pub enum Bloque {
    Texto(String),
    Tabla(Vec<Vec<Celda>>),
}
fn selector(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}
fn ignorado(e: ElementRef<'_>) -> bool {
    matches!(e.value().name(), "script" | "style" | "noscript" | "template" | "iframe" | "object" | "embed")
}
fn visible(e: ElementRef<'_>) -> String {
    if ignorado(e) {
        return String::new();
    }
    let mut s = String::new();
    for n in e.children() {
        if let Some(t) = n.value().as_text() {
            s.push_str(t);
        } else if let Some(e) = ElementRef::wrap(n) {
            s.push_str(&visible(e));
            if matches!(e.value().name(), "br" | "p" | "div") {
                s.push(' ');
            }
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn html_bloques(e: ElementRef<'_>, out: &mut Vec<Bloque>) {
    if ignorado(e) {
        return;
    }
    if e.value().name() == "table" {
        let filas = e
            .select(&selector("tr"))
            .filter(|r| r.ancestors().filter_map(ElementRef::wrap).find(|e| e.value().name() == "table").is_some_and(|t| t.id() == e.id()))
            .map(|r| {
                r.children()
                    .filter_map(ElementRef::wrap)
                    .filter(|e| matches!(e.value().name(), "td" | "th"))
                    .map(|e| Celda::Texto(visible(e)))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        out.push(Bloque::Tabla(filas));
        return;
    }
    let mut linea = String::new();
    let flush = |s: &mut String, out: &mut Vec<Bloque>| {
        let text = s.split_whitespace().collect::<Vec<_>>().join(" ");
        if !text.is_empty() {
            out.push(Bloque::Texto(text));
        }
        s.clear();
    };
    for n in e.children() {
        if let Some(t) = n.value().as_text() {
            linea.push_str(t);
        } else if let Some(hijo) = ElementRef::wrap(n) {
            if ignorado(hijo) {
                continue;
            }
            if matches!(
                hijo.value().name(),
                "table" | "p" | "div" | "section" | "article" | "main" | "header" | "footer" | "h1" | "h2" | "h3" | "li" | "ul" | "ol"
            ) {
                flush(&mut linea, out);
                html_bloques(hijo, out);
            } else if hijo.value().name() == "br" {
                flush(&mut linea, out);
            } else {
                linea.push_str(&visible(hijo));
            }
        }
    }
    flush(&mut linea, out);
}
pub fn leer_html(s: &str) -> Result<Vec<Bloque>, String> {
    if s.len() > 8 * 1024 * 1024 {
        return Err("El HTML supera 8 MiB.".into());
    }
    let doc = Html::parse_document(s);
    let body = doc.select(&selector("body")).next().unwrap_or(doc.root_element());
    let mut out = vec![];
    let mut pila = vec![(body, 0usize)];
    let mut elementos = 0;
    while let Some((e, prof)) = pila.pop() {
        elementos += 1;
        if prof > 128 || elementos > 200_000 {
            return Err("El HTML supera los límites de estructura.".into());
        }
        pila.extend(e.children().filter_map(ElementRef::wrap).map(|e| (e, prof + 1)));
    }
    html_bloques(body, &mut out);
    if out.is_empty() {
        return Err("El HTML no contiene texto ni tablas recuperables.".into());
    }
    Ok(out)
}
pub fn escapar(s: &str) -> String {
    quick_xml::escape::escape(s).into_owned()
}
pub fn semillar_html(sesion: &mut Sesion, bloques: &[Bloque]) -> Result<(), String> {
    for b in bloques {
        match b {
            Bloque::Texto(s) => formatos::semillar_linea(sesion, s),
            Bloque::Tabla(rows) => {
                formatos::seed_tabla(sesion, rows)?;
            }
        }
    }
    Ok(())
}
pub fn html(sesion: &mut Sesion, bloques: &[Bloque], textos: &mut Vec<(String, bool)>) -> Result<Vec<u8>, String> {
    let mut out = String::from("<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>");
    for b in bloques {
        match b {
            Bloque::Texto(s) => {
                let s = formatos::transformar_linea(sesion, s)?;
                textos.push((s.clone(), false));
                out.push_str(&format!("<p>{}</p>", escapar(&s)));
            }
            Bloque::Tabla(rows) => {
                out.push_str("<table>");
                for row in formatos::tabla(sesion, rows, textos)? {
                    out.push_str("<tr>");
                    for v in row {
                        out.push_str(&format!("<td>{}</td>", escapar(&v.texto())));
                    }
                    out.push_str("</tr>");
                }
                out.push_str("</table>");
            }
        }
    }
    out.push_str("</body></html>");
    Ok(out.into_bytes())
}

pub fn parrafos_docx(miembros: &[(String, Vec<u8>)]) -> Result<Vec<String>, String> {
    let mut partes = miembros
        .iter()
        .filter(|(n, _)| {
            let base = n.strip_prefix("word/glossary/").or_else(|| n.strip_prefix("word/")).unwrap_or("");
            n.ends_with(".xml") && ["document", "header", "footer", "footnotes", "endnotes", "comments"].iter().any(|p| base.starts_with(p))
        })
        .collect::<Vec<_>>();
    partes.sort_by_key(|(n, _)| (n != "word/document.xml", n.clone()));
    if !partes.iter().any(|(n, _)| n == "word/document.xml") {
        return Err("El Word no contiene un documento principal.".into());
    }
    if miembros.iter().any(|(n, _)| {
        let n = n.to_lowercase();
        n.starts_with("word/media/") || n.starts_with("word/embeddings/") || n.starts_with("word/activex/") || n.contains("vbaproject")
    }) {
        return Err("Word con imágenes u objetos incrustados: no se puede anonimizar con garantías.".into());
    }
    let mut out = vec![];
    for (_, raw) in partes {
        let xml = formatos::decodificar(raw)?;
        let mut reader = Reader::from_str(&xml);
        let mut pila = vec![];
        let mut dentro_texto = false;
        loop {
            match reader.read_event().map_err(|_| "El XML del Word no se reconoce.")? {
                Event::Start(e) => match e.local_name().as_ref() {
                    b"p" => {
                        if out.len() > 200_000 || pila.len() > 64 {
                            return Err("El Word supera los límites de párrafos.".into());
                        }
                        pila.push(out.len());
                        out.push(String::new());
                    }
                    b"t" => dentro_texto = true,
                    b"tab" | b"br" => {
                        if let Some(&i) = pila.last() {
                            out[i].push(' ');
                        }
                    }
                    _ => {}
                },
                Event::Empty(e) if matches!(e.local_name().as_ref(), b"tab" | b"br") => {
                    if let Some(&i) = pila.last() {
                        out[i].push(' ');
                    }
                }
                Event::Text(t) if dentro_texto => {
                    let s = t.decode().map_err(|_| "Texto de Word ilegible.")?;
                    let s = quick_xml::escape::unescape(&s).map_err(|_| "Entidad de Word no admitida.")?;
                    if let Some(&i) = pila.last() {
                        out[i].push_str(&s);
                    } else {
                        out.push(s.into_owned());
                    }
                }
                Event::GeneralRef(t) if dentro_texto => {
                    let value = t.decode().map_err(|_| "Referencia de Word ilegible.")?;
                    let value = format!("&{value};");
                    let value = quick_xml::escape::unescape(&value).map_err(|_| "Entidad de Word no admitida.")?;
                    if let Some(&i) = pila.last() {
                        out[i].push_str(&value);
                    } else {
                        out.push(value.into_owned());
                    }
                }
                Event::CData(t) if dentro_texto => {
                    let s = t.decode().map_err(|_| "Texto de Word ilegible.")?;
                    if let Some(&i) = pila.last() {
                        out[i].push_str(&s);
                    } else {
                        out.push(s.into_owned());
                    }
                }
                Event::End(e) => match e.local_name().as_ref() {
                    b"p" => {
                        pila.pop();
                    }
                    b"t" => dentro_texto = false,
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
        }
    }
    Ok(out)
}
pub fn zip(miembros: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut writer = zip::ZipWriter::new(Cursor::new(vec![]));
    for (name, body) in miembros {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated))
            .map_err(|_| "No se pudo preparar el ZIP de salida.")?;
        writer.write_all(body).map_err(|_| "No se pudo escribir el ZIP de salida.")?;
    }
    let data = writer.finish().map_err(|_| "No se pudo cerrar el ZIP de salida.")?.into_inner();
    if data.len() > MAX_ARCHIVO {
        return Err("El ZIP de salida supera 80 MiB.".into());
    }
    Ok(data)
}
pub fn docx(sesion: &mut Sesion, parrafos: &[String], textos: &mut Vec<(String, bool)>) -> Result<Vec<u8>, String> {
    let mut document=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>");
    for s in parrafos {
        let s = formatos::transformar_linea(sesion, s)?;
        textos.push((s.clone(), false));
        document.push_str(&format!("<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>", escapar(&s)));
    }
    document.push_str("<w:sectPr/></w:body></w:document>");
    sesion.avisos.insert("Word reconstruido como texto: incluye párrafos de cuerpo, encabezados, pies y comentarios; sin formato, vínculos ni metadatos del original.".into());
    zip(&[
        ("[Content_Types].xml".into(),br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#.to_vec()),
        ("_rels/.rels".into(),br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_vec()),
        ("word/document.xml".into(),document.into_bytes()),
    ])
}
