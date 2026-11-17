//! Proyectos de resolución en Word a partir de plantillas `.docx` con variables `{{NOMBRE}}`.
//! Todos los proyectos van en un solo documento (`generate_projects`), cada uno en página nueva.

use crate::excel::{ruta_nueva, valor};
use crate::numeros::fecha_en_palabras;
use flow_dominio::texto::normalizar_columna;
use flow_dominio::{Celda, Fecha, Tribunal};
use flow_trabajo::{FilaTrabajo, Trabajo};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

pub const TIPOS: [(&str, &str); 3] =
    [("PC_IE", "Pide cuenta ingreso efectivo"), ("PC_INFO", "Pide cuenta informe"), ("NOMENCL", "Nomenclatura")];

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

/// Reemplaza tramos `(inicio, fin, texto)` del texto visible de un párrafo, aunque crucen varios `<w:t>`, sin tocar
/// estilos: el reemplazo queda en el primer tramo tocado (como `fill_docx`). Los tramos no se solapan.
fn reemplazar_tramos(parrafo: &str, nodos: &[(std::ops::Range<usize>, String)], cambios: &[(usize, usize, String)]) -> String {
    let mut textos: Vec<String> = nodos.iter().map(|(_, t)| t.clone()).collect();
    let mut tocados = vec![false; textos.len()];
    for (inicio, fin, reemplazo) in cambios.iter().rev() {
        let (inicio, fin) = (*inicio, *fin);
        // Tramos que tocan el cambio, con sus posiciones en el texto completo actual.
        let mut pos = 0;
        let mut afectados = Vec::new();
        for (i, t) in textos.iter().enumerate() {
            let (a, b) = (pos, pos + t.len());
            if a < fin && b > inicio {
                afectados.push((i, a));
            }
            pos = b;
        }
        let Some(&(primero, a)) = afectados.first() else { continue };
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
    salida
}

fn nodos(parrafo: &str) -> Vec<(std::ops::Range<usize>, String)> {
    NODO_T.captures_iter(parrafo).map(|c| (c.get(0).unwrap().range(), sin_xml(&c[2]))).collect()
}

fn texto_de(nodos: &[(std::ops::Range<usize>, String)]) -> String {
    nodos.iter().map(|(_, t)| t.as_str()).collect()
}

/// Sustituye las variables de un párrafo aunque estén partidas entre varios tramos de texto, sin tocar estilos.
fn rellenar_parrafo(parrafo: &str, valores: &BTreeMap<String, String>, estricto: bool) -> Result<String, String> {
    let nodos = nodos(parrafo);
    let completo = texto_de(&nodos);
    if !VARIABLE.is_match(&completo) {
        return Ok(parrafo.to_string());
    }
    let mut cambios = Vec::new();
    for c in VARIABLE.captures_iter(&completo) {
        let m = c.get(0).unwrap();
        let reemplazo = valores.get(&c[1]).map(|v| v.trim()).unwrap_or("");
        if reemplazo.is_empty() {
            if estricto {
                return Err(format!("Falta la variable Word: {}", &c[1]));
            }
            continue;
        }
        cambios.push((m.start(), m.end(), reemplazo.to_string()));
    }
    Ok(reemplazar_tramos(parrafo, &nodos, &cambios))
}

/// `str.replace(viejo, nuevo)` sobre el texto visible del párrafo.
fn reemplazar_texto(parrafo: &str, viejo: &str, nuevo: &str) -> String {
    if viejo.is_empty() {
        return parrafo.to_string();
    }
    let nodos = nodos(parrafo);
    let completo = texto_de(&nodos);
    let cambios: Vec<(usize, usize, String)> =
        completo.match_indices(viejo).map(|(i, _)| (i, i + viejo.len(), nuevo.to_string())).collect();
    if cambios.is_empty() {
        return parrafo.to_string();
    }
    reemplazar_tramos(parrafo, &nodos, &cambios)
}

/// Reemplazos de plural de `resolutions.py::render_project`, en orden.
fn reemplazos_plural(p: &Plural) -> Vec<(&'static str, &'static str)> {
    let mut r = vec![
        ("la persona aludida", "las personas individualizadas"),
        ("del niño sujeto de protección", "de los niños, niñas o adolescentes individualizados"),
        ("del niño, niña o adolescente", "de los niños, niñas o adolescentes"),
    ];
    if p.varios_programas {
        r.extend([
            ("al organismo interventor", "a los organismos interventores"),
            ("al programa interventor", "a los programas intervinientes"),
            ("a la institución", "a las instituciones correspondientes"),
            ("de dicho programa", "de dichos programas"),
        ]);
    }
    if p.varias_fechas {
        r.push(("con fecha ", "con fechas "));
    }
    r
}

/// Párrafo `<w:p>` de un fragmento de `split_inclusive("</w:p>")`: posición de su apertura, si la hay.
fn apertura_parrafo(fragmento: &str) -> Option<usize> {
    let a = fragmento.rfind("<w:p>");
    let b = fragmento.rfind("<w:p ");
    a.max(b)
}

fn aperturas_tabla(texto: &str) -> isize {
    (texto.matches("<w:tbl>").count() + texto.matches("<w:tbl ").count()) as isize - texto.matches("</w:tbl>").count() as isize
}

/// Cuerpo de un proyecto: variables y, con varias personas, los reemplazos de plural y el párrafo «Personas
/// comprendidas» (`render_project`).
fn cuerpo_proyecto(cuerpo: &str, valores: &BTreeMap<String, String>, plural: Option<&Plural>) -> Result<String, String> {
    let lleno = rellenar_xml(cuerpo, valores, true)?;
    let Some(plural) = plural.filter(|p| p.plural) else { return Ok(lleno) };
    let nombre = valores.get("NOMBRE").map(String::as_str).unwrap_or("");
    let rut = valores.get("RUT").map(String::as_str).unwrap_or("");
    let reemplazos = reemplazos_plural(plural);
    let mut salida = String::with_capacity(lleno.len());
    let mut profundidad: isize = 0;
    let mut ancla: Option<usize> = None;
    for fragmento in lleno.split_inclusive("</w:p>") {
        let mut parrafo = fragmento.to_string();
        for marca in ["cédula de identidad N°", "cédula de identidad N.º"] {
            let par = format!("{nombre}, {marca} {rut}");
            if texto_de(&nodos(&parrafo)).contains(&par) {
                parrafo = reemplazar_texto(&parrafo, &par, &plural.personas.replace("cédula de identidad N°", marca));
            }
        }
        for (viejo, nuevo) in &reemplazos {
            parrafo = reemplazar_texto(&parrafo, viejo, nuevo);
        }
        if let Some(pos) = apertura_parrafo(&parrafo).filter(|_| fragmento.ends_with("</w:p>")) {
            let antes = profundidad + aperturas_tabla(&parrafo[..pos]);
            let texto = texto_de(&nodos(&parrafo));
            if ancla.is_none() && antes == 0 && !texto.contains("{{") && texto.trim().starts_with("RIT") {
                ancla = Some(salida.len() + pos);
            }
        }
        profundidad += aperturas_tabla(&parrafo);
        salida.push_str(&parrafo);
    }
    if plural.varios_programas {
        let detalle = format!(
            "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            a_xml(&format!("Personas comprendidas: {}.", plural.detalle))
        );
        match ancla {
            Some(pos) => salida.insert_str(pos, &detalle),
            None => salida.push_str(&detalle),
        }
    }
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
    let lote: Vec<(&BTreeMap<String, String>, Option<&Plural>)> = lote.iter().map(|v| (v, None)).collect();
    componer_con(docx, &lote)
}

fn componer_con(docx: &[u8], lote: &[(&BTreeMap<String, String>, Option<&Plural>)]) -> Result<Vec<u8>, String> {
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
            for (i, (valores, plural)) in lote.iter().enumerate() {
                if i > 0 {
                    armado.push_str(SALTO_PAGINA);
                }
                armado.push_str(&crate::compositor::mejorar_paginacion(&cuerpo_proyecto(cuerpo, valores, *plural)?));
            }
            armado.push_str(&xml[fin..]);
            armado.into_bytes()
        } else if nombre.starts_with("word/") && nombre.ends_with(".xml") {
            // Encabezados y pies: valores del primer proyecto; una variable sin valor se deja como está.
            match String::from_utf8(datos.clone()) {
                Ok(xml) if xml.contains("{{") => rellenar_xml(&xml, lote[0].0, false)?.into_bytes(),
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

/// Varias personas en un mismo proyecto (`grouped_values`: `_PLURAL`, `PERSONAS`, `DETALLE_PERSONAS`…).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Plural {
    pub plural: bool,
    pub varios_programas: bool,
    pub varias_fechas: bool,
    /// «A, cédula de identidad N° 1 y B, cédula de identidad N° 2».
    pub personas: String,
    /// «A, cédula de identidad N° 1, programa P; B, …».
    pub detalle: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proyecto {
    /// Todas las filas no excluidas de la causa (tribunal + RIT), en el orden del trabajo.
    pub filas: Vec<String>,
    pub tribunal: Tribunal,
    /// PC_IE, PC_INFO o NOMENCL.
    pub tipo: String,
    pub rit: String,
    pub valores: BTreeMap<String, String>,
    /// Valores al prepararlo: lo que difiere es edición del usuario y se conserva al volver a preparar.
    #[serde(default)]
    pub originales: BTreeMap<String, String>,
    /// Variables de la plantilla sin dato, que quedan como «[COMPLETAR X]» en el Word (no impiden generarlo).
    pub faltantes: Vec<String>,
    /// No hay plantilla para este tribunal y tipo.
    pub sin_plantilla: bool,
    #[serde(default)]
    pub omitido: bool,
    #[serde(default)]
    pub plural: Plural,
    /// Huella de los registros y la plantilla de los que depende (`dependency_hash`, `template_hash`).
    #[serde(default)]
    pub huella: String,
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
    let mut duraciones: Vec<String> = trabajo
        .cabeceras
        .iter()
        .enumerate()
        .filter(|(_, c)| flow_trabajo::controles::columna_duracion(c))
        .filter_map(|(i, _)| fila.celdas.get(i).map(|c| c.texto().trim().to_string()))
        .collect();
    // Paquete Captura_CSMP.zip: la tabla del motor no trae la columna; se toma del registro (diferencia deliberada
    // con `capture_input.py`, que la descarta y deja DURACION vacía).
    if duraciones.is_empty() {
        if let Some(o) = &fila.origen {
            duraciones = o
                .columnas_word
                .iter()
                .filter(|(k, _)| flow_trabajo::controles::columna_duracion(k))
                .map(|(_, c)| c.texto().trim().to_string())
                .collect();
        }
    }
    v.insert("DURACION".into(), if duraciones.len() == 1 { duraciones[0].clone() } else { String::new() });
    v.insert("NOMENCLATURA".into(), nomenclatura(fila).to_string());
    v
}

/// `resolutions.py::join_names`: sin vacíos ni repetidos; «A», «A y B», «A, B y C».
pub fn unir<I: IntoIterator<Item = S>, S: AsRef<str>>(valores: I) -> String {
    let mut lista: Vec<String> = Vec::new();
    for v in valores {
        let v = v.as_ref().trim();
        if !v.is_empty() && !lista.iter().any(|x| x == v) {
            lista.push(v.to_string());
        }
    }
    match lista.len() {
        0 => String::new(),
        1 => lista.remove(0),
        n => format!("{} y {}", lista[..n - 1].join(", "), lista[n - 1]),
    }
}

/// `resolutions.py::grouped_values`: valores de la primera fila de la causa y NOMBRE, RUT, PROGRAMA,
/// FECHA_RESOLUCION y DURACION de todas, unidos con «A y B».
fn valores_agrupados(trabajo: &Trabajo, filas: &[&FilaTrabajo], tipo: &str, hoy: Fecha) -> (BTreeMap<String, String>, Plural) {
    let de_cada: Vec<BTreeMap<String, String>> = filas.iter().map(|f| valores_de(trabajo, f, hoy)).collect();
    let mut v = de_cada[0].clone();
    for clave in ["NOMBRE", "RUT", "PROGRAMA", "FECHA_RESOLUCION", "DURACION"] {
        v.insert(clave.into(), unir(de_cada.iter().map(|x| x[clave].as_str())));
    }
    // NOMENCLATURA automática (propia de Flow): de la fila de la causa que originó la nomenclatura.
    if tipo == "NOMENCL" {
        let n = filas.iter().filter(|f| f.tiene_accion("NOMENCL")).map(|f| nomenclatura(f)).find(|n| !n.is_empty());
        v.insert("NOMENCLATURA".into(), n.unwrap_or("").to_string());
    }
    // Clave (nombre y RUT normalizados) → nombre, RUT y programas de esa persona.
    type Persona = ((String, String), (String, String, Vec<String>));
    let mut personas: Vec<Persona> = Vec::new();
    for f in filas {
        let (nombre, rut, programa) = (valor(f, "nombre"), valor(f, "rut"), valor(f, "programa"));
        let clave = (normalizar_columna(&nombre), normalizar_columna(&rut));
        match personas.iter_mut().find(|(k, _)| *k == clave) {
            Some((_, p)) => p.2.push(programa),
            None => personas.push((clave, (nombre, rut, vec![programa]))),
        }
    }
    let identidad =
        |nombre: &str, rut: &str| format!("{nombre}, cédula de identidad N° {}", if rut.is_empty() { "[COMPLETAR RUT]" } else { rut });
    let programas: BTreeSet<String> = filas.iter().map(|f| normalizar_columna(&valor(f, "programa"))).collect();
    let fechas: BTreeSet<&str> = de_cada.iter().map(|x| x["FECHA_RESOLUCION"].as_str()).filter(|x| !x.is_empty()).collect();
    let plural = Plural {
        plural: personas.len() > 1,
        varios_programas: programas.len() > 1,
        varias_fechas: fechas.len() > 1,
        personas: unir(personas.iter().map(|(_, (n, r, _))| identidad(n, r))),
        detalle: personas
            .iter()
            .map(|(_, (n, r, ps))| format!("{}, programa {}", identidad(n, r), unir(ps)))
            .collect::<Vec<_>>()
            .join("; "),
    };
    (v, plural)
}

fn completar(variable: &str) -> String {
    format!("[COMPLETAR {}]", variable.replace('_', " "))
}

fn revisar_faltantes(p: &mut Proyecto, carpeta_usuario: Option<&Path>) {
    match plantilla(p.tribunal, &p.tipo, carpeta_usuario).and_then(|d| variables(&d).ok()) {
        None => {
            p.sin_plantilla = true;
            p.faltantes.clear();
        }
        Some(vars) => {
            p.sin_plantilla = false;
            p.faltantes = vars
                .into_iter()
                .filter(|v| p.valores.get(v).is_none_or(|x| x.trim().is_empty() || x.to_uppercase().contains("[COMPLETAR")))
                .collect();
        }
    }
}

/// `resolutions.py::observation_resolution_kind`: tipo que la observación nombra, si lo hace.
pub fn tipo_por_observacion(observacion: &str) -> Option<&'static str> {
    let t = normalizar_columna(observacion);
    if t.is_empty() {
        None
    } else if t.contains("nomencl") {
        Some("NOMENCL")
    } else if t.contains("pc info") {
        Some("PC_INFO")
    } else if t.contains("pc ie") {
        Some("PC_IE")
    } else if ["informe", "diagnostico", "diagnostico clinico"].iter().any(|p| t.contains(p)) {
        Some("PC_INFO")
    } else if ["ingreso efectivo", "fecha estimada de ingreso", "ingreso al programa"].iter().any(|p| t.contains(p)) {
        Some("PC_IE")
    } else {
        None
    }
}

/// Alias de las columnas humanas (`importing.py`): se toma una columna solo si hay exactamente una.
pub const ALIAS_RES: [&str; 6] =
    ["res", "resolucion generada", "resolucion", "generar resolucion", "proyecto resolucion", "proyecto de resolucion"];
pub const ALIAS_OBSERVACION: [&str; 2] = ["observacion", "observaciones"];

/// Posición de la columna humana del origen con alguno de esos nombres, si hay exactamente una.
pub fn columna_humana(trabajo: &Trabajo, alias: &[&str]) -> Option<usize> {
    let alias: Vec<String> = alias.iter().map(|a| normalizar_columna(a)).collect();
    let halladas: Vec<usize> =
        trabajo.cabeceras.iter().enumerate().filter(|(_, c)| alias.contains(&normalizar_columna(c))).map(|(i, _)| i).collect();
    (halladas.len() == 1).then(|| halladas[0])
}

/// Observación que revisa el usuario (`review['OBSERVACION']`): la editada en Flow o, si el origen trae la columna,
/// la del origen; si no, la propuesta.
pub fn observacion_revisada(trabajo: &Trabajo, fila: &FilaTrabajo) -> String {
    if fila.editada() {
        return fila.observacion.clone();
    }
    match columna_humana(trabajo, &ALIAS_OBSERVACION) {
        Some(i) => fila.celdas.get(i).map(|c| c.texto()).unwrap_or_default(),
        None => fila.observacion.clone(),
    }
}

/// `resolutions.py::resolution_kind`.
pub fn tipo_res(celda: &Celda) -> Option<&'static str> {
    match normalizar_columna(&celda.texto()).replace('_', " ").as_str() {
        "pc ie" => Some("PC_IE"),
        "pc info" => Some("PC_INFO"),
        "nomencl" | "nomenclatura" => Some("NOMENCL"),
        _ => None,
    }
}

/// `resolutions.py::resolution_marked` sobre una celda con valor (`_has_explicit_value`).
pub fn res_marcado(celda: &Celda) -> bool {
    if tipo_res(celda).is_some() {
        return true;
    }
    match celda {
        Celda::Vacia | Celda::Fecha(_) => false,
        Celda::Numero(n) => *n != 0.0,
        Celda::Texto(t) => {
            let crudo = t.trim().to_lowercase().replace(',', ".");
            if crudo == "1" || crudo == "1.0" {
                return true;
            }
            if crudo == "0" || crudo == "0.0" {
                return false;
            }
            matches!(normalizar_columna(t).as_str(), "si" | "s" | "x" | "true" | "verdadero" | "res" | "resolucion" | "con resolucion")
        }
    }
}

fn explicito(celda: &Celda) -> bool {
    match celda {
        Celda::Vacia => false,
        Celda::Texto(t) => !t.trim().is_empty(),
        _ => true,
    }
}

/// `_case_key`: tribunal y RIT normalizados.
fn clave_causa(fila: &FilaTrabajo) -> (String, String) {
    let crudo = valor(fila, "tribunal");
    let tribunal = Tribunal::desde(&crudo).map(|t| t.nombre().to_string()).unwrap_or_else(|| normalizar_columna(&crudo));
    (tribunal, normalizar_columna(&valor(fila, "rit")))
}

/// `resolutions.py::automatic_project_selections`.
///
/// - Si el origen trae una columna RES (aunque esté vacía), manda la selección humana: entran las causas con RES
///   marcado y su tipo sale de RES, si no de la observación, si no de las acciones (y en último caso PC_IE), para todas
///   las filas de la causa.
/// - Si no, cada fila con acciones de proyecto aporta el tipo que nombra la observación o, si no nombra ninguno, todos
///   los tipos de sus acciones.
///
/// La rama `structured` de Python depende de decisiones por fila (`NURUS_DECISIONES`) que Flow no tiene.
pub fn selecciones(trabajo: &Trabajo) -> Vec<(String, String)> {
    if trabajo.fuente.is_some() {
        return Vec::new();
    }
    let mut salida = Vec::new();
    if let Some(col) = columna_humana(trabajo, &ALIAS_RES) {
        let mut por_causa: Vec<((String, String), Vec<&'static str>)> = Vec::new();
        for fila in trabajo.filas.iter().filter(|f| !f.propuesta.excluido) {
            let res = fila.celdas.get(col).cloned().unwrap_or_default();
            if !(explicito(&res) && res_marcado(&res)) {
                continue;
            }
            let mut tipos = match (tipo_res(&res), tipo_por_observacion(&observacion_revisada(trabajo, fila))) {
                (Some(t), _) | (None, Some(t)) => vec![t],
                _ => fila.proyectos().into_iter().filter_map(|t| TIPOS.iter().find(|(x, _)| *x == t).map(|(x, _)| *x)).collect(),
            };
            if tipos.is_empty() {
                tipos.push("PC_IE");
            }
            let clave = clave_causa(fila);
            let i = match por_causa.iter().position(|(k, _)| *k == clave) {
                Some(i) => i,
                None => {
                    por_causa.push((clave, Vec::new()));
                    por_causa.len() - 1
                }
            };
            for t in tipos {
                if !por_causa[i].1.contains(&t) {
                    por_causa[i].1.push(t);
                }
            }
        }
        for fila in trabajo.filas.iter().filter(|f| !f.propuesta.excluido) {
            let clave = clave_causa(fila);
            if let Some((_, tipos)) = por_causa.iter().find(|(k, _)| *k == clave) {
                salida.extend(tipos.iter().map(|t| (fila.id.clone(), t.to_string())));
            }
        }
        return salida;
    }
    for fila in trabajo.filas.iter().filter(|f| !f.propuesta.excluido) {
        let sugeridos = fila.proyectos();
        if sugeridos.is_empty() {
            continue;
        }
        let tipos = match tipo_por_observacion(&observacion_revisada(trabajo, fila)) {
            Some(t) => vec![t],
            None => sugeridos,
        };
        salida.extend(tipos.into_iter().map(|t| (fila.id.clone(), t.to_string())));
    }
    salida
}

fn causa(fila: &FilaTrabajo) -> Option<(Tribunal, String)> {
    Some((Tribunal::desde(&valor(fila, "tribunal"))?, normalizar_columna(&valor(fila, "rit"))))
}

/// Proyectos que corresponden al trabajo (`selecciones` + `preparar`).
pub fn proyectos(trabajo: &Trabajo, hoy: Fecha, carpeta_usuario: Option<&Path>) -> Vec<Proyecto> {
    preparar(trabajo, &selecciones(trabajo), hoy, carpeta_usuario, &trabajo.version_reglas)
}

/// `resolutions.py::prepare_projects`: un proyecto por tribunal + RIT + tipo con todas las filas no excluidas de la
/// causa. Las variables sin dato quedan como «[COMPLETAR X]».
/// `reglas`: las vigentes en el programa, para la huella de vigencia.
pub fn preparar(
    trabajo: &Trabajo,
    selecciones: &[(String, String)],
    hoy: Fecha,
    carpeta_usuario: Option<&Path>,
    reglas: &str,
) -> Vec<Proyecto> {
    if trabajo.fuente.is_some() {
        return Vec::new();
    }
    let mut por_causa: Vec<((Tribunal, String), Vec<&FilaTrabajo>)> = Vec::new();
    for fila in trabajo.filas.iter().filter(|f| !f.propuesta.excluido) {
        let Some(clave) = causa(fila) else { continue };
        match por_causa.iter_mut().find(|(k, _)| *k == clave) {
            Some((_, filas)) => filas.push(fila),
            None => por_causa.push((clave, vec![fila])),
        }
    }
    let mut grupos: Vec<((Tribunal, String), String)> = Vec::new();
    for (id, tipo) in selecciones {
        if !TIPOS.iter().any(|(t, _)| t == tipo) {
            continue;
        }
        let Some(fila) = trabajo.filas.iter().find(|f| &f.id == id && !f.propuesta.excluido) else { continue };
        let Some(clave) = causa(fila) else { continue };
        let grupo = (clave, tipo.clone());
        if !grupos.contains(&grupo) {
            grupos.push(grupo);
        }
    }
    let huellas = crate::vigencia::Huellas::nuevas(trabajo, reglas);
    let mut salida = Vec::new();
    for (clave, tipo) in grupos {
        let Some((_, filas)) = por_causa.iter().find(|(k, _)| *k == clave) else { continue };
        let (valores, plural) = valores_agrupados(trabajo, filas, &tipo, hoy);
        let mut p = Proyecto {
            filas: filas.iter().map(|f| f.id.clone()).collect(),
            tribunal: clave.0,
            tipo,
            rit: valor(filas[0], "rit"),
            valores,
            originales: BTreeMap::new(),
            faltantes: Vec::new(),
            sin_plantilla: false,
            omitido: false,
            plural,
            huella: String::new(),
        };
        revisar_faltantes(&mut p, carpeta_usuario);
        for v in p.faltantes.clone() {
            if p.valores.get(&v).is_none_or(|x| x.trim().is_empty()) {
                p.valores.insert(v.clone(), completar(&v));
            }
        }
        p.originales = p.valores.clone();
        p.huella = huellas.de_proyecto(&p, plantilla(p.tribunal, &p.tipo, carpeta_usuario).as_deref());
        salida.push(p);
    }
    salida
}

/// `session.py::merge_project_edits`: al volver a preparar, los valores que el usuario cambió en un proyecto del mismo
/// tribunal, RIT y tipo se conservan, y también si lo había omitido.
pub fn conservar_ediciones(previos: &[Proyecto], nuevos: &mut [Proyecto], carpeta_usuario: Option<&Path>) {
    for p in nuevos.iter_mut() {
        let Some(viejo) = previos.iter().find(|v| v.tribunal == p.tribunal && v.rit == p.rit && v.tipo == p.tipo) else { continue };
        let editados: Vec<(String, String)> = viejo
            .valores
            .iter()
            .filter(|(k, v)| viejo.originales.get(*k).is_some_and(|o| o != *v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if !editados.is_empty() {
            p.valores.extend(editados);
            p.actualizar(carpeta_usuario);
        }
        p.omitido = viejo.omitido;
    }
}

impl Proyecto {
    /// Fila principal (la primera de la causa).
    pub fn fila(&self) -> &str {
        self.filas.first().map(String::as_str).unwrap_or("")
    }
    /// Variables de la plantilla que el usuario dejó vacías: `fill_docx` no puede generarlas.
    pub fn vacias(&self) -> Vec<&str> {
        self.faltantes.iter().filter(|v| self.valores.get(*v).is_none_or(|x| x.trim().is_empty())).map(String::as_str).collect()
    }
    /// Se genera aunque queden «[COMPLETAR X]» (`require_complete(..., allow_review_markers=True)`).
    pub fn listo(&self) -> bool {
        !self.omitido && !self.sin_plantilla && self.vacias().is_empty()
    }
    /// Tras editar un valor a mano.
    pub fn actualizar(&mut self, carpeta_usuario: Option<&Path>) {
        revisar_faltantes(self, carpeta_usuario);
    }
}

/// Partes del cuerpo de `document.xml`: antes del cuerpo, cuerpo sin la sección final, y sección con el cierre.
fn dividir(xml: &str) -> Result<(&str, &str, &str), String> {
    let inicio = xml.find("<w:body>").map(|p| p + "<w:body>".len()).ok_or("La plantilla no tiene cuerpo.")?;
    let fin_cuerpo = xml.rfind("</w:body>").ok_or("La plantilla no tiene cuerpo.")?;
    let fin = xml[inicio..fin_cuerpo].rfind("<w:sectPr").map_or(fin_cuerpo, |p| inicio + p);
    Ok((&xml[..inicio], &xml[inicio..fin], &xml[fin..]))
}

fn parte_texto(docx: &[u8], nombre: &str) -> Result<String, String> {
    let datos = partes(docx)?.into_iter().find(|(n, _)| n == nombre).map(|(_, d)| d).unwrap_or_default();
    String::from_utf8(datos).map_err(|_| format!("{nombre} no es UTF-8."))
}

/// `require_complete(..., allow_review_markers=True)`: no puede quedar ninguna `{{VARIABLE}}`.
fn exigir_completo(docx: &[u8]) -> Result<(), String> {
    let mut pendientes = BTreeSet::new();
    for (nombre, datos) in partes(docx)? {
        if nombre.starts_with("word/") && nombre.ends_with(".xml") {
            let xml = String::from_utf8_lossy(&datos);
            for parrafo in xml.split_inclusive("</w:p>") {
                let texto: String = NODO_T.captures_iter(parrafo).map(|c| sin_xml(&c[2])).collect();
                pendientes.extend(VARIABLE.find_iter(&texto).map(|m| m.as_str().to_string()));
            }
        }
    }
    if pendientes.is_empty() {
        Ok(())
    } else {
        Err(format!("Completa los campos Word pendientes: {}", pendientes.into_iter().collect::<Vec<_>>().join(", ")))
    }
}

/// Un documento con proyectos de distintas plantillas (`generate_projects`): el paquete del primero y, de los demás, su
/// cuerpo con sus estilos (`compositor`). Encabezados y pies con los valores del primero.
fn componer_varias(lote: &[(&[u8], &Proyecto)]) -> Result<Vec<u8>, String> {
    let (base, primero) = lote.first().ok_or("No hay proyectos para generar.")?;
    let xml = parte_texto(base, "word/document.xml")?;
    let (cabeza, cuerpo, cola) = dividir(&xml)?;
    let mut compositor = crate::compositor::Compositor::nuevo(&parte_texto(base, "word/styles.xml")?);
    let mut armado = String::from(cabeza);
    armado.push_str(&crate::compositor::mejorar_paginacion(&cuerpo_proyecto(cuerpo, &primero.valores, Some(&primero.plural))?));
    for (docx, p) in &lote[1..] {
        let suyo = parte_texto(docx, "word/document.xml")?;
        let (_, cuerpo, _) = dividir(&suyo)?;
        let mut cuerpo = crate::compositor::mejorar_paginacion(&cuerpo_proyecto(cuerpo, &p.valores, Some(&p.plural))?);
        if docx != base {
            cuerpo = compositor.agregar(&cuerpo, &parte_texto(docx, "word/styles.xml")?);
        }
        armado.push_str(SALTO_PAGINA);
        armado.push_str(&cuerpo);
    }
    armado.push_str(cola);
    let mut salida = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opciones = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (nombre, datos) in partes(base)? {
        let nuevo = if nombre == "word/document.xml" {
            armado.clone().into_bytes()
        } else if nombre == "word/styles.xml" {
            compositor.estilos().as_bytes().to_vec()
        } else if nombre.starts_with("word/") && nombre.ends_with(".xml") {
            match String::from_utf8(datos.clone()) {
                Ok(xml) if xml.contains("{{") => rellenar_xml(&xml, &primero.valores, false)?.into_bytes(),
                _ => datos,
            }
        } else {
            datos
        };
        salida.start_file(nombre, opciones).map_err(|e| e.to_string())?;
        salida.write_all(&nuevo).map_err(|e| e.to_string())?;
    }
    let datos = salida.finish().map_err(|e| e.to_string())?.into_inner();
    exigir_completo(&datos)?;
    Ok(datos)
}

/// `generate_projects`: un solo Word `Resoluciones_<fecha y hora>.docx` con todos los proyectos listos, en su orden y
/// cada uno en página nueva, aunque sean de distintos tribunales y tipos. Un proyecto cuya plantilla (distinta de la
/// primera) usa imágenes, vínculos o numeración en el cuerpo no se puede unir: va en otro documento con los de su
/// plantilla. Devuelve las rutas creadas, la principal primero.
pub fn generar(lista: &[Proyecto], carpeta: &Path, carpeta_usuario: Option<&Path>) -> Result<Vec<PathBuf>, String> {
    let listos: Vec<&Proyecto> = lista.iter().filter(|p| p.listo()).collect();
    if listos.is_empty() {
        return Err("No hay proyectos listos para generar.".into());
    }
    let mut lote: Vec<(Vec<u8>, &Proyecto)> = Vec::new();
    for p in listos {
        let docx = plantilla(p.tribunal, &p.tipo, carpeta_usuario)
            .ok_or_else(|| format!("{}: falta matriz {}/{}", p.rit, p.tribunal.nombre(), p.tipo))?;
        lote.push((docx, p));
    }
    let base = lote[0].0.clone();
    let mut unidos: Vec<(&[u8], &Proyecto)> = Vec::new();
    let mut aparte: Vec<(&[u8], Vec<&Proyecto>)> = Vec::new();
    for (docx, p) in &lote {
        let cuerpo = parte_texto(docx, "word/document.xml")?;
        if *docx == base || !crate::compositor::necesita_partes(dividir(&cuerpo)?.1) {
            unidos.push((docx, p));
        } else {
            match aparte.iter_mut().find(|(d, _)| *d == docx.as_slice()) {
                Some((_, v)) => v.push(p),
                None => aparte.push((docx, vec![p])),
            }
        }
    }
    std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
    let mut rutas = Vec::new();
    let nombre = format!("Resoluciones_{}", chrono::Local::now().format("%Y%m%d_%H%M%S_%6f"));
    let ruta = ruta_nueva(carpeta, &nombre, "docx");
    let datos = componer_varias(&unidos)?;
    std::fs::write(&ruta, datos).map_err(|e| format!("No se pudo guardar {}: {e}", ruta.display()))?;
    rutas.push(ruta);
    for (docx, grupo) in aparte {
        let lote: Vec<(&BTreeMap<String, String>, Option<&Plural>)> = grupo.iter().map(|p| (&p.valores, Some(&p.plural))).collect();
        let datos = componer_con(docx, &lote)?;
        exigir_completo(&datos)?;
        let ruta = ruta_nueva(carpeta, &format!("{nombre} - {} {} aparte", grupo[0].tribunal.titulo(), grupo[0].tipo), "docx");
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
    fn union_y_tipo_por_observacion_como_python() {
        // `join_names(['A','B','A',' ','C'])` y `observation_resolution_kind` del Asistente Python.
        assert_eq!(unir(["A", "B", "A", " ", "C"]), "A, B y C");
        assert_eq!(unir(["A"]), "A");
        assert_eq!(unir([""; 0]), "");
        assert_eq!(tipo_por_observacion("Observación ficticia sin tipo."), None);
        assert_eq!(tipo_por_observacion("Se solicita PC IE ficticio."), Some("PC_IE"));
        assert_eq!(tipo_por_observacion("Pedir informe ficticio"), Some("PC_INFO"));
        assert_eq!(tipo_por_observacion("Aplicar NOMENCLATURA ficticia"), Some("NOMENCL"));
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
