//! Un solo documento Word con proyectos de distintas plantillas, como `generate_projects` de Python
//! (`docxcompose.Composer(docs[0], preserve_styles=True)` y `word_quality.improve_pagination`).
//!
//! Se porta lo que usan las matrices: el primer documento da el paquete (estilos, encabezados, sección); de cada uno
//! de los siguientes se agrega el cuerpo sin su sección, con el formato de su estilo de párrafo por defecto
//! (`retain_formatting_from_default_styles`) y sus estilos copiados o renombrados (`add_styles`, `increment_name`).
//! Un cuerpo de otra plantilla con imágenes, vínculos o numeración no se puede unir: el llamador lo deja aparte.

use regex::Regex;
use std::sync::LazyLock;

static ESTILO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<w:style\b[^>]*>.*?</w:style>").unwrap());
static ID_ESTILO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"w:styleId="([^"]*)""#).unwrap());
static NOMBRE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<w:name w:val="([^"]*)"\s*/>"#).unwrap());
static IGNORADOS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<w:(name|rsid)\b[^>]*/>").unwrap());
static REFERENCIA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<w:(pStyle|rStyle|tblStyle) w:val="([^"]*)""#).unwrap());
static VINCULO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<w:link w:val="([^"]*)""#).unwrap());
static RUN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<w:r(\s[^>]*)?>").unwrap());
static PARRAFO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<w:p(\s[^>]*)?>").unwrap());
static RPR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<w:rPr>(.*?)</w:rPr>").unwrap());
static PPR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<w:pPr>(.*?)</w:pPr>").unwrap());
static HIJO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<(w:[A-Za-z0-9]+)\b[^>]*/>").unwrap());
static ETIQUETA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<(w:[A-Za-z0-9]+)\b").unwrap());
static PSTYLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<w:pStyle w:val="([^"]*)""#).unwrap());
static ENCABEZADOS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<w:(headerReference|footerReference)\b[^>]*/>").unwrap());
static TEXTO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<w:t(\s[^>]*)?>([^<]*)</w:t>").unwrap());

#[derive(Clone)]
struct Estilo {
    id: String,
    nombre: Option<String>,
    xml: String,
}

fn estilos(xml: &str) -> Vec<Estilo> {
    ESTILO
        .find_iter(xml)
        .map(|m| {
            let x = m.as_str();
            let apertura = &x[..x.find('>').unwrap_or(0)];
            Estilo {
                id: ID_ESTILO.captures(apertura).map(|c| c[1].to_string()).unwrap_or_default(),
                nombre: NOMBRE.captures(x).map(|c| c[1].to_string()),
                xml: x.to_string(),
            }
        })
        .collect()
}

fn por_defecto(lista: &[Estilo]) -> Option<&Estilo> {
    lista.iter().find(|e| {
        let apertura = &e.xml[..e.xml.find('>').unwrap_or(0)];
        apertura.contains(r#"w:type="paragraph""#) && apertura.contains(r#"w:default="1""#)
    })
}

/// `xml_elements_equal(..., ignored_tags={name, rsid})`.
fn firma(xml: &str) -> String {
    IGNORADOS.replace_all(xml, "").to_string()
}

/// `docxcompose.utils.increment_name`.
pub fn incrementar(nombre: &str) -> String {
    let parte = nombre.rsplit('_').next().unwrap_or("");
    match parte.parse::<i64>() {
        Ok(n) => format!("{}{}", &nombre[..nombre.len() - parte.len()], n + 1),
        _ => format!("{nombre}_1"),
    }
}

fn hijos(bloque: &str) -> Vec<(String, String)> {
    HIJO.captures_iter(bloque).map(|c| (c[1].to_string(), c[0].to_string())).collect()
}

fn bloque<'a>(re: &Regex, xml: &'a str) -> &'a str {
    re.captures(xml).and_then(|c| c.get(1)).map_or("", |m| m.as_str())
}

/// Un cuerpo de otra plantilla que no se puede unir sin copiar partes relacionadas.
pub fn necesita_partes(cuerpo: &str) -> bool {
    ["r:id=\"", "r:embed=\"", "r:link=\"", "<w:numPr", "<w:numId", "w:footnoteReference", "w:endnoteReference", "<w:object", "<v:"]
        .iter()
        .any(|x| cuerpo.contains(x))
}

/// `retain_formatting_from_default_styles`: si el estilo de párrafo por defecto difiere, sus propiedades pasan a los
/// tramos y párrafos sin estilo propio. A diferencia de Python, no repite una propiedad que el tramo ya define.
fn conservar_formato(cuerpo: &str, suyos: &[Estilo], nuestros: &[Estilo]) -> String {
    let Some(suyo) = por_defecto(suyos) else { return cuerpo.to_string() };
    if por_defecto(nuestros).is_some_and(|n| n.xml == suyo.xml) {
        return cuerpo.to_string();
    }
    let de_tramo = hijos(bloque(&RPR, &suyo.xml));
    let de_parrafo = hijos(bloque(&PPR, &suyo.xml));
    let mut salida = String::with_capacity(cuerpo.len());
    for parrafo in cuerpo.split_inclusive("</w:p>") {
        let mut p = parrafo.to_string();
        if !de_tramo.is_empty() {
            // Tramos sin propiedades: se les agrega un <w:rPr> vacío.
            let mut cambiado = String::new();
            let mut cursor = 0;
            for m in RUN.find_iter(&p) {
                cambiado.push_str(&p[cursor..m.end()]);
                if !p[m.end()..].starts_with("<w:rPr") {
                    cambiado.push_str("<w:rPr></w:rPr>");
                }
                cursor = m.end();
            }
            cambiado.push_str(&p[cursor..]);
            p = cambiado.replace("<w:rPr/>", "<w:rPr></w:rPr>");
            let estilo_parrafo = PSTYLE.captures(&p).map(|c| c[1].to_string());
            let del_estilo: Vec<String> = estilo_parrafo
                .and_then(|id| suyos.iter().find(|e| e.id == id))
                .map(|e| {
                    let mut t: Vec<String> = hijos(bloque(&RPR, &e.xml)).into_iter().map(|h| h.0).collect();
                    t.extend(hijos(bloque(&PPR, &e.xml)).into_iter().map(|h| h.0));
                    t
                })
                .unwrap_or_default();
            p = RPR
                .replace_all(&p, |c: &regex::Captures| {
                    let dentro = &c[1];
                    if dentro.contains("<w:rStyle") {
                        return c[0].to_string();
                    }
                    let presentes: Vec<String> = ETIQUETA.captures_iter(dentro).map(|x| x[1].to_string()).collect();
                    let extra: String = de_tramo
                        .iter()
                        .filter(|(t, _)| !del_estilo.contains(t) && !presentes.contains(t))
                        .map(|(_, x)| x.as_str())
                        .collect();
                    format!("<w:rPr>{dentro}{extra}</w:rPr>")
                })
                .to_string();
        }
        if !de_parrafo.is_empty() {
            if let Some(m) = PARRAFO.find(&p) {
                if !p[m.end()..].starts_with("<w:pPr") {
                    p.insert_str(m.end(), "<w:pPr></w:pPr>");
                }
            }
            p = p.replacen("<w:pPr/>", "<w:pPr></w:pPr>", 1);
            p = PPR
                .replace(&p, |c: &regex::Captures| {
                    let dentro = &c[1];
                    if dentro.contains("<w:pStyle") {
                        return c[0].to_string();
                    }
                    let presentes: Vec<String> = ETIQUETA.captures_iter(dentro).map(|x| x[1].to_string()).collect();
                    let extra: String = de_parrafo.iter().filter(|(t, _)| !presentes.contains(t)).map(|(_, x)| x.as_str()).collect();
                    format!("<w:pPr>{dentro}{extra}</w:pPr>")
                })
                .to_string();
        }
        salida.push_str(&p);
    }
    salida
}

fn cambiar_referencias(cuerpo: &str, de: &str, a: &str) -> String {
    REFERENCIA
        .replace_all(cuerpo, |c: &regex::Captures| if &c[2] == de { format!(r#"<w:{} w:val="{a}""#, &c[1]) } else { c[0].to_string() })
        .to_string()
}

/// Estilos del documento compuesto, con las copias preservadas de cada estilo (`_preserved_styles`).
pub struct Compositor {
    estilos_xml: String,
    preservados: Vec<(String, Vec<String>)>,
}

impl Compositor {
    pub fn nuevo(estilos_base: &str) -> Self {
        Compositor { estilos_xml: estilos_base.to_string(), preservados: Vec::new() }
    }

    pub fn estilos(&self) -> &str {
        &self.estilos_xml
    }

    fn agregar_estilo(&mut self, xml: &str) {
        match self.estilos_xml.rfind("</w:styles>") {
            Some(p) => self.estilos_xml.insert_str(p, xml),
            None => self.estilos_xml.push_str(xml),
        }
    }

    /// Prepara el cuerpo de un documento agregado (`Composer.insert`): formato por defecto, estilos y sin
    /// referencias a encabezados ni pies.
    pub fn agregar(&mut self, cuerpo: &str, estilos_suyos: &str) -> String {
        let suyos = estilos(estilos_suyos);
        let nuestros = estilos(&self.estilos_xml);
        let mut cuerpo = conservar_formato(cuerpo, &suyos, &nuestros);
        let nombre_de: Vec<(String, Option<String>)> = suyos.iter().map(|e| (e.id.clone(), e.nombre.clone())).collect();
        let mut usados: Vec<String> = Vec::new();
        for c in REFERENCIA.captures_iter(&cuerpo) {
            if !usados.contains(&c[2].to_string()) {
                usados.push(c[2].to_string());
            }
        }
        for id in usados {
            let nuestros = estilos(&self.estilos_xml);
            // `mapped_style_id`: el mismo nombre, aunque el identificador sea de otro idioma.
            let nuestro_id = nombre_de
                .iter()
                .find(|(i, _)| *i == id)
                .and_then(|(_, n)| n.as_ref())
                .and_then(|n| nuestros.iter().find(|e| e.nombre.as_deref() == Some(n.as_str())))
                .map(|e| e.id.clone())
                .unwrap_or_else(|| id.clone());
            let Some(suyo) = suyos.iter().find(|e| e.id == id) else { continue };
            if nuestros.iter().any(|e| e.id == nuestro_id) {
                let candidatos: Vec<String> = self
                    .preservados
                    .iter()
                    .find(|(k, _)| *k == nuestro_id)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| vec![nuestro_id.clone()]);
                let igual = candidatos.iter().any(|c| nuestros.iter().any(|e| &e.id == c && firma(&e.xml) == firma(&suyo.xml)));
                let elegido = if igual {
                    suyo.id.clone()
                } else {
                    let mut nuevo = incrementar(&nuestro_id);
                    let mut nombre = suyo.nombre.as_deref().map(incrementar);
                    while nuestros.iter().any(|e| e.id == nuevo) {
                        nuevo = incrementar(&nuevo);
                        nombre = nombre.as_deref().map(incrementar);
                    }
                    let mut copia = suyo.xml.replacen(&format!(r#"w:styleId="{}""#, suyo.id), &format!(r#"w:styleId="{nuevo}""#), 1);
                    if let (Some(viejo), Some(n)) = (&suyo.nombre, &nombre) {
                        copia = copia.replacen(&format!(r#"<w:name w:val="{viejo}""#), &format!(r#"<w:name w:val="{n}""#), 1);
                    }
                    self.agregar_estilo(&copia);
                    self.agregar_vinculado(&copia, &suyos);
                    match self.preservados.iter_mut().find(|(k, _)| *k == nuestro_id) {
                        Some((_, v)) => v.push(nuevo.clone()),
                        None => self.preservados.push((nuestro_id.clone(), vec![nuestro_id.clone(), nuevo.clone()])),
                    }
                    nuevo
                };
                cuerpo = cambiar_referencias(&cuerpo, &id, &elegido);
                if elegido == id && nuestro_id != id {
                    cuerpo = cambiar_referencias(&cuerpo, &id, &nuestro_id);
                }
            } else {
                self.agregar_estilo(&suyo.xml.clone());
                self.agregar_vinculado(&suyo.xml.clone(), &suyos);
                if nuestro_id != id {
                    cuerpo = cambiar_referencias(&cuerpo, &id, &nuestro_id);
                }
            }
        }
        ENCABEZADOS.replace_all(&cuerpo, "").to_string()
    }

    /// `add_linked_styles`.
    fn agregar_vinculado(&mut self, xml: &str, suyos: &[Estilo]) {
        let Some(id) = VINCULO.captures(xml).map(|c| c[1].to_string()) else { return };
        if estilos(&self.estilos_xml).iter().any(|e| e.id == id) {
            return;
        }
        if let Some(e) = suyos.iter().find(|e| e.id == id) {
            let x = e.xml.clone();
            self.agregar_estilo(&x);
        }
    }
}

/// `word_quality.improve_pagination`: control de líneas viudas en los párrafos con texto que no lo definen.
pub fn mejorar_paginacion(cuerpo: &str) -> String {
    let mut salida = String::with_capacity(cuerpo.len());
    for parrafo in cuerpo.split_inclusive("</w:p>") {
        let texto: String = TEXTO.captures_iter(parrafo).map(|c| c[2].to_string()).collect();
        let Some(m) = PARRAFO.find(parrafo).filter(|_| parrafo.ends_with("</w:p>") && !texto.trim().is_empty()) else {
            salida.push_str(parrafo);
            continue;
        };
        let mut p = parrafo.to_string();
        let resto = &p[m.end()..];
        if resto.starts_with("<w:pPr/>") {
            p.replace_range(m.end()..m.end() + "<w:pPr/>".len(), "<w:pPr><w:widowControl/></w:pPr>");
        } else if resto.starts_with("<w:pPr>") {
            let inicio = m.end() + "<w:pPr>".len();
            let fin = inicio + p[inicio..].find("</w:pPr>").unwrap_or(0);
            let dentro = &p[inicio..fin];
            if !dentro.contains("<w:widowControl") {
                // Orden del esquema: pStyle, keepNext, keepLines, pageBreakBefore, framePr, widowControl.
                let mut pos = 0;
                for (etiqueta, x) in hijos(dentro) {
                    if matches!(etiqueta.as_str(), "w:pStyle" | "w:keepNext" | "w:keepLines" | "w:pageBreakBefore" | "w:framePr") {
                        pos = dentro.find(&x).map_or(pos, |i| (i + x.len()).max(pos));
                    }
                }
                p.insert_str(inicio + pos, "<w:widowControl/>");
            }
        } else {
            p.insert_str(m.end(), "<w:pPr><w:widowControl/></w:pPr>");
        }
        salida.push_str(&p);
    }
    salida
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn incrementar_como_docxcompose() {
        // `increment_name` de docxcompose.
        assert_eq!(incrementar("Normal"), "Normal_1");
        assert_eq!(incrementar("Normal_1"), "Normal_2");
        assert_eq!(incrementar("Titulo_9"), "Titulo_10");
    }

    #[test]
    fn paginacion_agrega_control_de_viudas() {
        let c = r#"<w:p><w:r><w:t>Uno</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="X"/><w:jc w:val="both"/></w:pPr><w:r><w:t>Dos</w:t></w:r></w:p><w:p></w:p>"#;
        let r = mejorar_paginacion(c);
        assert!(r.starts_with("<w:p><w:pPr><w:widowControl/></w:pPr><w:r>"), "{r}");
        assert!(r.contains(r#"<w:pStyle w:val="X"/><w:widowControl/><w:jc"#), "{r}");
        assert!(r.ends_with("<w:p></w:p>"), "un párrafo vacío no cambia");
    }
}
