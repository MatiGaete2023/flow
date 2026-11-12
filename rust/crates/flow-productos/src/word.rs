//! Proyectos de resolución en Word a partir de plantillas `.docx` con variables `{{NOMBRE}}`.
//! Todos los proyectos de un mismo tribunal y tipo van en un solo documento, cada uno en página nueva.

use crate::excel::{ruta_nueva, valor};
use crate::numeros::fecha_en_palabras;
use flow_dominio::texto::normalizar_columna;
use flow_dominio::{Fecha, Tribunal};
use flow_trabajo::{FilaTrabajo, Trabajo};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

pub const TIPOS: [(&str, &str); 3] = [
    ("PC_IE", "Pide cuenta ingreso efectivo"),
    ("PC_INFO", "Pide cuenta informe"),
    ("NOMENCL", "Nomenclatura"),
];

static NODO_T: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<w:t(\s[^>]*)?>([^<]*)</w:t>").unwrap());
static VARIABLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{([A-Z_]+)\}\}").unwrap());
const SALTO_PAGINA: &str = r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#;

/// Plantillas incluidas en el programa (las mismas del Asistente). Tomé no trae: el usuario agrega las suyas.
fn incluida(tribunal: Tribunal, tipo: &str) -> Option<&'static [u8]> {
    Some(match (tribunal, tipo) {
        (Tribunal::Laja, "PC_IE") => include_bytes!("../../../datos/plantillas/LAJA_PC_IE.docx"),
        (Tribunal::Laja, "PC_INFO") => include_bytes!("../../../datos/plantillas/LAJA_PC_INFO.docx"),
        (Tribunal::Laja, "NOMENCL") => include_bytes!("../../../datos/plantillas/LAJA_NOMENCL.docx"),
        (Tribunal::Mulchen, "PC_IE") => include_bytes!("../../../datos/plantillas/MULCHEN_PC_IE.docx"),
        (Tribunal::Mulchen, "PC_INFO") => include_bytes!("../../../datos/plantillas/MULCHEN_PC_INFO.docx"),
        (Tribunal::Mulchen, "NOMENCL") => include_bytes!("../../../datos/plantillas/MULCHEN_NOMENCL.docx"),
        _ => return None,
    })
}

/// Plantilla del usuario (`<carpeta>/TOME_PC_IE.docx`) si existe; si no, la incluida.
pub fn plantilla(tribunal: Tribunal, tipo: &str, carpeta_usuario: Option<&Path>) -> Option<Vec<u8>> {
    let propia = carpeta_usuario.map(|c| c.join(format!("{}_{tipo}.docx", tribunal.nombre())));
    if let Some(datos) = propia.and_then(|ruta| std::fs::read(ruta).ok()) {
        return Some(datos);
    }
    incluida(tribunal, tipo).map(<[u8]>::to_vec)
}

fn sin_xml(texto: &str) -> String {
    texto.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

fn a_xml(texto: &str) -> String {
    texto.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Sustituye las variables de un párrafo aunque estén partidas entre varios tramos de texto, sin tocar estilos.
fn rellenar_parrafo(parrafo: &str, valores: &BTreeMap<String, String>, estricto: bool) -> Result<String, String> {
    let nodos: Vec<(std::ops::Range<usize>, String)> =
        NODO_T.captures_iter(parrafo).map(|c| (c.get(0).unwrap().range(), sin_xml(&c[2]))).collect();
    let completo: String = nodos.iter().map(|(_, t)| t.as_str()).collect();
    if !VARIABLE.is_match(&completo) {
        return Ok(parrafo.to_string());
    }
    let mut textos: Vec<String> = nodos.iter().map(|(_, t)| t.clone()).collect();
    let mut tocados = vec![false; textos.len()];
    let hallazgos: Vec<(usize, usize, String)> =
        VARIABLE.captures_iter(&completo).map(|c| (c.get(0).unwrap().start(), c.get(0).unwrap().end(), c[1].to_string())).collect();
    for (inicio, fin, clave) in hallazgos.into_iter().rev() {
        let reemplazo = valores.get(&clave).map(|v| v.trim()).unwrap_or("");
        if reemplazo.is_empty() {
            if estricto {
                return Err(format!("Falta la variable Word: {clave}"));
            }
            continue;
        }
        // Tramos que tocan la variable, con sus posiciones en el texto completo actual.
        let mut pos = 0;
        let mut afectados = Vec::new();
        for (i, t) in textos.iter().enumerate() {
            let (a, b) = (pos, pos + t.len());
            if a < fin && b > inicio {
                afectados.push((i, a));
            }
            pos = b;
        }
        let (primero, a) = afectados[0];
        let (ultimo, la) = *afectados.last().unwrap();
        let prefijo = textos[primero][..inicio - a].to_string();
        let sufijo = textos[ultimo][fin - la..].to_string();
        for (i, _) in &afectados[1..] {
            textos[*i] = if *i == ultimo { sufijo.clone() } else { String::new() };
            tocados[*i] = true;
        }
        textos[primero] = if primero == ultimo { format!("{prefijo}{reemplazo}{sufijo}") } else { format!("{prefijo}{reemplazo}") };
        tocados[primero] = true;
    }
    let mut salida = String::with_capacity(parrafo.len());
    let mut cursor = 0;
    for (i, (rango, _)) in nodos.iter().enumerate() {
        salida.push_str(&parrafo[cursor..rango.start]);
        if tocados[i] {
            salida.push_str(&format!("<w:t xml:space=\"preserve\">{}</w:t>", a_xml(&textos[i])));
        } else {
            salida.push_str(&parrafo[rango.clone()]);
        }
        cursor = rango.end;
    }
    salida.push_str(&parrafo[cursor..]);
    Ok(salida)
}

fn rellenar_xml(xml: &str, valores: &BTreeMap<String, String>, estricto: bool) -> Result<String, String> {
    let mut salida = String::with_capacity(xml.len());
    for parrafo in xml.split_inclusive("</w:p>") {
        salida.push_str(&rellenar_parrafo(parrafo, valores, estricto)?);
    }
    Ok(salida)
}

fn partes(docx: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(docx)).map_err(|e| format!("La plantilla no es un .docx válido: {e}"))?;
    let mut salida = Vec::new();
    for i in 0..zip.len() {
        let mut entrada = zip.by_index(i).map_err(|e| e.to_string())?;
        if entrada.size() > 50 * 1024 * 1024 {
            return Err("La plantilla tiene una parte demasiado grande.".into());
        }
        let mut datos = Vec::new();
        entrada.read_to_end(&mut datos).map_err(|e| e.to_string())?;
        salida.push((entrada.name().to_string(), datos));
    }
    Ok(salida)
}

/// Variables `{{X}}` que usa una plantilla.
pub fn variables(docx: &[u8]) -> Result<BTreeSet<String>, String> {
    let mut salida = BTreeSet::new();
    for (nombre, datos) in partes(docx)? {
        if nombre.starts_with("word/") && nombre.ends_with(".xml") {
            let xml = String::from_utf8_lossy(&datos);
            for parrafo in xml.split_inclusive("</w:p>") {
                let texto: String = NODO_T.captures_iter(parrafo).map(|c| sin_xml(&c[2])).collect();
                salida.extend(VARIABLE.captures_iter(&texto).map(|c| c[1].to_string()));
            }
        }
    }
    Ok(salida)
}

/// Un documento con un proyecto por juego de valores, cada uno en página nueva.
pub fn componer(docx: &[u8], lote: &[BTreeMap<String, String>]) -> Result<Vec<u8>, String> {
    if lote.is_empty() {
        return Err("No hay proyectos para generar.".into());
    }
    let mut salida = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opciones = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (nombre, datos) in partes(docx)? {
        let nuevo = if nombre == "word/document.xml" {
            let xml = String::from_utf8(datos).map_err(|_| "document.xml no es UTF-8.".to_string())?;
            let inicio = xml.find("<w:body>").map(|p| p + "<w:body>".len()).ok_or("La plantilla no tiene cuerpo.")?;
            let fin_cuerpo = xml.rfind("</w:body>").ok_or("La plantilla no tiene cuerpo.")?;
            let fin = xml[inicio..fin_cuerpo].rfind("<w:sectPr").map_or(fin_cuerpo, |p| inicio + p);
            let cuerpo = &xml[inicio..fin];
            let mut armado = String::from(&xml[..inicio]);
            for (i, valores) in lote.iter().enumerate() {
                if i > 0 {
                    armado.push_str(SALTO_PAGINA);
                }
                armado.push_str(&rellenar_xml(cuerpo, valores, true)?);
            }
            armado.push_str(&xml[fin..]);
            armado.into_bytes()
        } else if nombre.starts_with("word/") && nombre.ends_with(".xml") {
            // Encabezados y pies: valores del primer proyecto; una variable sin valor se deja como está.
            match String::from_utf8(datos.clone()) {
                Ok(xml) if xml.contains("{{") => rellenar_xml(&xml, &lote[0], false)?.into_bytes(),
                _ => datos,
            }
        } else {
            datos
        };
        salida.start_file(nombre, opciones).map_err(|e| e.to_string())?;
        salida.write_all(&nuevo).map_err(|e| e.to_string())?;
    }
    Ok(salida.finish().map_err(|e| e.to_string())?.into_inner())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proyecto {
    pub fila: String,
    pub tribunal: Tribunal,
    /// PC_IE, PC_INFO o NOMENCL.
    pub tipo: String,
    pub rit: String,
    pub valores: BTreeMap<String, String>,
    /// Variables de la plantilla que no tienen valor: el usuario las completa antes de generar.
    pub faltantes: Vec<String>,
    /// No hay plantilla para este tribunal y tipo.
    pub sin_plantilla: bool,
    #[serde(default)]
    pub omitido: bool,
}

/// Nomenclatura que el proyecto pide aplicar, según la regla que lo originó.
fn nomenclatura(fila: &FilaTrabajo) -> &'static str {
    if fila.propuesta.reglas.iter().any(|r| r == "HISTORIA.P_FINALIZADA") {
        "Egreso de servicio protección especializada y otras redes"
    } else if fila.propuesta.reglas.iter().any(|r| r == "HISTORIA.C10_ESCRITO") {
        "Ingreso Informe de Cumplimiento X"
    } else {
        ""
    }
}

fn valores_de(trabajo: &Trabajo, fila: &FilaTrabajo, hoy: Fecha) -> BTreeMap<String, String> {
    let mut v = BTreeMap::new();
    for campo in ["rit", "rut", "nombre", "programa"] {
        v.insert(campo.to_uppercase(), valor(fila, campo));
    }
    v.insert("FECHA".into(), fecha_en_palabras(hoy));
    v.insert("FECHA_RESOLUCION".into(), fila.registro.fecha("resolucion").map(fecha_en_palabras).unwrap_or_default());
    let duraciones: Vec<String> = trabajo
        .cabeceras
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(normalizar_columna(c).trim(), "duracion" | "plazo" | "vigencia"))
        .filter_map(|(i, _)| fila.celdas.get(i).map(|c| c.texto().trim().to_string()))
        .collect();
    v.insert("DURACION".into(), if duraciones.len() == 1 { duraciones[0].clone() } else { String::new() });
    v.insert("NOMENCLATURA".into(), nomenclatura(fila).to_string());
    v
}

fn revisar_faltantes(p: &mut Proyecto, carpeta_usuario: Option<&Path>) {
    match plantilla(p.tribunal, &p.tipo, carpeta_usuario).and_then(|d| variables(&d).ok()) {
        None => {
            p.sin_plantilla = true;
            p.faltantes.clear();
        }
        Some(vars) => {
            p.sin_plantilla = false;
            p.faltantes = vars.into_iter().filter(|v| p.valores.get(v).is_none_or(|x| x.trim().is_empty())).collect();
        }
    }
}

/// Proyectos que corresponden al trabajo: uno por fila con acción de proyecto.
pub fn proyectos(trabajo: &Trabajo, hoy: Fecha, carpeta_usuario: Option<&Path>) -> Vec<Proyecto> {
    let mut salida = Vec::new();
    for fila in trabajo.filas.iter().filter(|f| !f.propuesta.excluido) {
        let (Some(tipo), Some(tribunal)) = (fila.proyecto(), Tribunal::desde(&valor(fila, "tribunal"))) else { continue };
        let mut p = Proyecto {
            fila: fila.id.clone(),
            tribunal,
            tipo: tipo.to_string(),
            rit: valor(fila, "rit"),
            valores: valores_de(trabajo, fila, hoy),
            faltantes: Vec::new(),
            sin_plantilla: false,
            omitido: false,
        };
        revisar_faltantes(&mut p, carpeta_usuario);
        salida.push(p);
    }
    salida
}

impl Proyecto {
    pub fn listo(&self) -> bool {
        !self.omitido && !self.sin_plantilla && self.faltantes.is_empty()
    }
    /// Tras editar un valor a mano.
    pub fn actualizar(&mut self, carpeta_usuario: Option<&Path>) {
        revisar_faltantes(self, carpeta_usuario);
    }
}

/// Genera un documento por tribunal y tipo con los proyectos listos. Devuelve las rutas creadas.
pub fn generar(lista: &[Proyecto], carpeta: &Path, carpeta_usuario: Option<&Path>) -> Result<Vec<PathBuf>, String> {
    let mut grupos: BTreeMap<(Tribunal, String), Vec<&Proyecto>> = BTreeMap::new();
    for p in lista.iter().filter(|p| p.listo()) {
        grupos.entry((p.tribunal, p.tipo.clone())).or_default().push(p);
    }
    if grupos.is_empty() {
        return Err("No hay proyectos listos para generar.".into());
    }
    std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
    let mut rutas = Vec::new();
    for ((tribunal, tipo), grupo) in grupos {
        let docx = plantilla(tribunal, &tipo, carpeta_usuario).ok_or("Falta la plantilla.")?;
        let lote: Vec<BTreeMap<String, String>> = grupo.iter().map(|p| p.valores.clone()).collect();
        let datos = componer(&docx, &lote)?;
        let ruta = ruta_nueva(carpeta, &format!("Proyectos {tipo} - {} ({})", tribunal.titulo(), grupo.len()), "docx");
        std::fs::write(&ruta, datos).map_err(|e| format!("No se pudo guardar {}: {e}", ruta.display()))?;
        rutas.push(ruta);
    }
    Ok(rutas)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn valores(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
        pares.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn variable_partida_entre_tramos() {
        let p = r#"<w:p><w:r><w:t>RIT: {{</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>RI</w:t></w:r><w:r><w:t>T}} fin</w:t></w:r></w:p>"#;
        let r = rellenar_parrafo(p, &valores(&[("RIT", "X-1-2099 <a&b>")]), true).unwrap();
        assert!(r.contains(r#"<w:t xml:space="preserve">RIT: X-1-2099 &lt;a&amp;b&gt;</w:t>"#), "{r}");
        assert!(r.contains(r#"<w:t xml:space="preserve"> fin</w:t>"#), "{r}");
        assert!(r.contains("<w:rPr><w:b/></w:rPr>"), "conserva estilos");
        assert!(rellenar_parrafo(p, &valores(&[]), true).is_err());
    }

    #[test]
    fn plantillas_incluidas_declaran_sus_variables() {
        let v = variables(incluida(Tribunal::Mulchen, "PC_IE").unwrap()).unwrap();
        let esperadas: BTreeSet<String> =
            ["DURACION", "FECHA", "FECHA_RESOLUCION", "NOMBRE", "PROGRAMA", "RIT", "RUT"].map(String::from).into();
        assert_eq!(v, esperadas);
        assert!(incluida(Tribunal::Tome, "PC_IE").is_none());
    }

    #[test]
    fn compone_un_proyecto_por_pagina() {
        let docx = incluida(Tribunal::Laja, "PC_IE").unwrap();
        let lote = vec![
            valores(&[("PROGRAMA", "PRM Uno"), ("NOMBRE", "Sofía Pérez"), ("RUT", "11111111-K"), ("RIT", "X-1-2099")]),
            valores(&[("PROGRAMA", "PRM Dos"), ("NOMBRE", "Matías Rojas"), ("RUT", "22222222-0"), ("RIT", "X-2-2099")]),
        ];
        let salida = componer(docx, &lote).unwrap();
        let xml = partes(&salida).unwrap().into_iter().find(|(n, _)| n == "word/document.xml").unwrap().1;
        let xml = String::from_utf8(xml).unwrap();
        assert!(xml.contains("X-1-2099") && xml.contains("X-2-2099") && xml.contains("Matías Rojas"));
        assert!(!xml.contains("{{"), "no quedan variables sin sustituir");
        assert_eq!(xml.matches(r#"<w:br w:type="page"/>"#).count(), 1);
        assert_eq!(xml.matches("<w:sectPr").count(), 1, "una sola sección al final");
    }
}
