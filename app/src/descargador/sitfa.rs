//! Validación local de consultas y respuestas de SITFA (`perfiles.py` y `motor.py` de Flow Python Lite).
//! No hace peticiones ni ejecuta scripts. Los errores son textos fijos: nunca HTML, URL XLS ni datos de personas.

mod filtro;
pub(crate) mod registros;

use super::{Alcance, Pantalla, Perfil};
use registros::{clave, columnas, contar, contar_listado, filas_de_datos, grilla, grilla_con_cuerpo};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub type Pares = Vec<(String, String)>;
/// La cantidad importa: dos medidas pueden tener el mismo RIT y nombre.
pub type Registros = BTreeMap<(String, String), usize>;

pub const DESTINO: &str = "/SITFAWEB/InformesDAction.do";
pub const DESTINO_ABSOLUTO: &str = "https://familia.pjud.cl/SITFAWEB/InformesDAction.do";
pub const LIMITE_BYTES: usize = 80 * 1024 * 1024;
/// Pestaña, control, campo de página, campo de total y tabla (`perfiles.TABS`).
pub const PESTANAS_SITFA: [(&str, &str, &str, &str, &str); 4] = [
    ("Espera", "tdEspera", "NUM_PaginaEspera", "NUM_TotalEspera", "tablaEspera"),
    (
        "Cumplimiento",
        "tdCumplimiento",
        "NUM_PaginaCumplimiento",
        "NUM_TotalCumplimiento",
        "tablaCumplimiento",
    ),
    ("Informes", "tdInforme", "NUM_PaginaInforme", "NUM_TotalInforme", "tablaInforme"),
    ("Egresados", "tdEgreso", "NUM_PaginaEgreso", "NUM_TotalEgreso", "tablaEgresados"),
];

fn selector(s: &str) -> Selector {
    Selector::parse(s).expect("selector constante del programa")
}

static FORMULARIO: LazyLock<Selector> =
    LazyLock::new(|| selector("form[name='InformesPpalForm']"));
static SCRIPTS: LazyLock<Selector> = LazyLock::new(|| selector("script"));
static CONTROLES: LazyLock<Selector> =
    LazyLock::new(|| selector("input[name], select[name], textarea[name]"));
static TABLAS: LazyLock<Selector> = LazyLock::new(|| selector("table"));
static ENTRADAS: LazyLock<Selector> = LazyLock::new(|| selector("input"));
static BANNER: LazyLock<Selector> = LazyLock::new(|| selector("#sitfaContextoConsulta"));
static TEXTAREA: LazyLock<Selector> = LazyLock::new(|| selector("textarea"));

/// Perfil de lectura de una consulta: campos que la respuesta debe reproducir y dónde están página, total y tabla.
#[derive(Debug, Clone)]
pub struct Consulta {
    pub pantalla: Pantalla,
    pub objetivo: BTreeMap<String, String>,
    pub accion: &'static str,
    pub pestana: &'static str,
    pub pagina: &'static str,
    pub total: &'static str,
    pub tabla: &'static str,
    pub plantilla: Option<&'static str>,
}

impl Consulta {
    pub fn desde(perfil: &Perfil) -> Result<Self, String> {
        let mut c = Consulta {
            pantalla: perfil.pantalla,
            objetivo: BTreeMap::from([("COD_Tribunal_sel".into(), perfil.tribunal.clone())]),
            accion: "",
            pestana: "",
            pagina: "",
            total: "",
            tabla: "",
            plantilla: None,
        };
        match (&perfil.alcance, perfil.pantalla) {
            (
                Alcance::Seguimiento {
                    pestana,
                    modalidad,
                    reporte,
                },
                Pantalla::Seguimiento,
            ) => {
                let t = PESTANAS_SITFA
                    .iter()
                    .find(|t| t.0 == pestana)
                    .ok_or("Pestaña no permitida.")?;
                c.objetivo.insert("TIP_Consulta".into(), modalidad.clone());
                c.objetivo.insert("COD_Lengueta".into(), t.1.into());
                if let Some(r) = reporte.as_ref().filter(|_| matches!(t.0, "Informes" | "Cumplimiento")) {
                    c.objetivo.insert("TIP_Informe".into(), r.clone());
                }
                (c.pestana, c.pagina, c.total, c.tabla) = (t.1, t.2, t.3, t.4);
                c.accion = "Buscar medida";
            }
            (Alcance::Litigantes { estado }, Pantalla::Litigantes) => {
                c.objetivo.insert("TIP_Consulta".into(), "2".into());
                c.objetivo.insert("COD_EstBusqueda".into(), estado.clone());
                (c.accion, c.pagina, c.total, c.plantilla) =
                    ("Consulta Informe", "NUM_PaginaInforme", "NUM_Total", Some("BusquedaLit"));
            }
            (
                Alcance::Calendario { mes, anio },
                Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas,
            ) => {
                c.objetivo.insert("TIP_Consulta".into(), "1".into());
                c.objetivo.insert("COD_Mes_sel".into(), mes.clone());
                c.objetivo.insert("COD_Anio_Sel".into(), anio.clone());
                (c.accion, c.plantilla) = if perfil.pantalla == Pantalla::CalendarioInformes {
                    ("Buscar Inf.", Some("InfoPorVencer"))
                } else {
                    ("Buscar", Some("MedPorVencer"))
                };
            }
            _ => return Err("Esta consulta todavía no se ejecuta desde Flow Rust.".into()),
        }
        Ok(c)
    }

    pub fn calendario(&self) -> bool {
        matches!(
            self.pantalla,
            Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas
        )
    }

    /// `motor.query_pairs`: campos únicos, auditados por pantalla, acción de lectura y objetivo exacto.
    pub fn validar_consulta(&self, pares: &[(String, String)]) -> Result<(), String> {
        if pares.is_empty() || pares.len() > 100 {
            return Err("La consulta tiene parametros no auditados; requiere revision local.".into());
        }
        let mut unicos = BTreeMap::new();
        for (k, v) in pares {
            if unicos.insert(k.as_str(), v.as_str()).is_some() {
                return Err("Consulta con parametros repetidos; descarga detenida.".into());
            }
            if !self.permitido(k) {
                return Err("La consulta tiene parametros no auditados; requiere revision local.".into());
            }
        }
        if unicos.get("irAccion").copied() != Some(self.accion)
            || self
                .objetivo
                .iter()
                .any(|(k, v)| unicos.get(k.as_str()).copied() != Some(v.as_str()))
        {
            return Err("La solicitud no corresponde a la consulta de lectura seleccionada.".into());
        }
        Ok(())
    }

    fn permitido(&self, campo: &str) -> bool {
        const COMUNES: &str = "COD_Centro GLS_TribunalOrigen FLG_Consulta COD_Tribunal_sel TIP_Consulta TIP_Causa ROL_Causa ERA_Causa RUT_Consulta RUT_DvConsulta CHK_Consulta FEC_Inicio FEC_Fin irAccion";
        let adicionales = match self.pantalla {
            Pantalla::Seguimiento => "COD_Lengueta COD_CentroResidencial COD_PlazoIntervencion TIP_Informe COD_TiempoEspera COD_TipLitigante NUM_PaginaEspera NUM_TotalEspera NUM_PaginaCumplimiento NUM_TotalCumplimiento NUM_PaginaInforme NUM_TotalInforme NUM_PaginaEgreso NUM_TotalEgreso",
            Pantalla::Litigantes => "COD_Medida COD_EstBusqueda NUM_PaginaInforme NUM_Total",
            Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas => {
                return "FLG_Busqueda COD_TribunalActual GLS_TribunalActual busquedaPorTribunal TIP_Consulta COD_Tribunal_sel FEC_Inicio COD_Mes_sel COD_Anio_Sel CHK_Contrae irAccion"
                    .split_whitespace()
                    .any(|k| k == campo)
            }
            Pantalla::Carga => return false,
        };
        COMUNES
            .split_whitespace()
            .chain(adicionales.split_whitespace())
            .any(|k| k == campo)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pagina {
    pub actual: usize,
    pub total: usize,
    pub excel_url: String,
    pub plantilla: String,
    pub registros: Registros,
    pub paginacion: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Respuesta {
    SinRegistros,
    ConRegistros(Pagina),
}

fn es_paginacion(k: &str) -> bool {
    PESTANAS_SITFA.iter().any(|t| k == t.2 || k == t.3) || k == "NUM_Total"
}

/// UTF-8 si es válido; si no, Windows-1252.
pub fn decodificar(datos: &[u8]) -> String {
    let datos = datos.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(datos);
    match std::str::from_utf8(datos) {
        Ok(t) => t.to_string(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(datos).0.into_owned(),
    }
}

/// `motor.fields_from_form`: valor de cada control con nombre; radios por grupo; el resto debe ser único.
fn campos(formulario: ElementRef<'_>) -> Result<BTreeMap<String, String>, String> {
    let mut grupos: BTreeMap<String, Vec<ElementRef<'_>>> = BTreeMap::new();
    for el in formulario.select(&CONTROLES) {
        let tipo = el.value().attr("type").unwrap_or("").to_ascii_lowercase();
        if el.value().name() == "input" && matches!(tipo.as_str(), "button" | "reset" | "image") {
            continue;
        }
        grupos
            .entry(el.value().attr("name").unwrap_or("").into())
            .or_default()
            .push(el);
    }
    let mut resultado = BTreeMap::new();
    for (nombre, elementos) in grupos {
        let es_radio = |e: &ElementRef<'_>| {
            e.value().name() == "input"
                && e.value().attr("type").is_some_and(|t| t.eq_ignore_ascii_case("radio"))
        };
        let radios = elementos.iter().filter(|e| es_radio(e)).count();
        if radios > 0 && radios != elementos.len() {
            return Err("Formulario ambiguo: tipos de campo repetidos.".into());
        }
        let el = if radios > 0 {
            let marcados: Vec<_> = elementos
                .iter()
                .filter(|e| e.value().attr("checked").is_some())
                .collect();
            if marcados.len() > 1 {
                return Err("Formulario ambiguo: opciones de consulta repetidas.".into());
            }
            *marcados.first().copied().unwrap_or(&elementos[0])
        } else if elementos.len() != 1 {
            return Err("Formulario ambiguo: campos repetidos.".into());
        } else {
            elementos[0]
        };
        let valor = match el.value().name() {
            "select" => {
                let opciones: Vec<_> = el
                    .children()
                    .filter_map(ElementRef::wrap)
                    .filter(|o| o.value().name() == "option")
                    .collect();
                opciones
                    .iter()
                    .find(|o| o.value().attr("selected").is_some())
                    .or(opciones.first())
                    .map(|o| o.value().attr("value").unwrap_or("").to_string())
                    .unwrap_or_default()
            }
            "textarea" => el.text().collect(),
            _ => el.value().attr("value").unwrap_or("").into(),
        };
        resultado.insert(nombre, valor);
    }
    Ok(resultado)
}

static SHOW_EXCEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"function\s+ShowExcel\s*\(\s*\)\s*\{").unwrap());
static FUNCION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bfunction\s+\w+\s*\(").unwrap());
static ABRIR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"window\.open\s*\(\s*["']([^"']+)["']"#).unwrap());
static URL_XLS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\Ahttps://familia\.pjud\.cl/sitfa/reportes/[A-Za-z0-9_-]+\.xls\z").unwrap()
});

/// `motor.validate_export_url`: solo el sitio y la ruta de reportes. Devuelve la plantilla, sin seguir la URL.
pub fn plantilla_url(url: &str) -> Result<String, String> {
    if !URL_XLS.is_match(url) {
        return Err("El enlace Excel esta fuera del sitio permitido.".into());
    }
    let nombre = url
        .rsplit('/')
        .next()
        .and_then(|n| n.strip_suffix(".xls"))
        .unwrap_or("");
    Ok(nombre.rsplit_once('_').map_or(nombre, |(_, p)| p).to_string())
}

/// `ChromeTransport.download`: la URL debe terminar en `_<plantilla>.xls` de la primera página validada.
pub fn validar_descarga(url: &str, plantilla: &str) -> Result<(), String> {
    plantilla_url(url)?;
    let sufijo = format!("_{plantilla}.xls");
    let nombre = url.rsplit('/').next().unwrap_or("");
    if !nombre.ends_with(&sufijo) || nombre.len() == sufijo.len() {
        return Err("El enlace Excel no coincide con la consulta actual.".into());
    }
    Ok(())
}

fn exportacion(scripts: &str, consulta: &Consulta) -> Result<(String, String), String> {
    let inicio = SHOW_EXCEL
        .find(scripts)
        .ok_or("SITFA no entrego un enlace Excel para esta consulta.")?;
    let bloque = &scripts[inicio.end()..];
    let bloque = &bloque[..FUNCION.find(bloque).map(|m| m.start()).unwrap_or(bloque.len())];
    let url = ABRIR
        .captures(bloque)
        .and_then(|c| c.get(1))
        .ok_or("No se encontro la exportacion XLS auditada.")?
        .as_str()
        .to_string();
    let plantilla = plantilla_url(&url)?;
    let modalidad_dce = consulta.pantalla == Pantalla::Seguimiento
        && consulta.objetivo.get("TIP_Consulta").map(String::as_str) == Some("4");
    if !plantilla.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || plantilla.is_empty() && !modalidad_dce
    {
        return Err("El nombre Excel no se puede reconocer.".into());
    }
    if consulta.plantilla.is_some_and(|p| p != plantilla) {
        return Err("El Excel corresponde a otra pantalla.".into());
    }
    Ok((url, plantilla))
}

/// Cadena JavaScript literal con los escapes que acepta `ast.literal_eval`; no evalúa expresiones.
fn cadena_literal(literal: &str) -> Option<String> {
    let comilla = literal.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let cuerpo = literal.strip_prefix(comilla)?.strip_suffix(comilla)?;
    let mut salida = String::with_capacity(cuerpo.len());
    let mut chars = cuerpo.chars().peekable();
    let hex = |chars: &mut std::iter::Peekable<std::str::Chars<'_>>, n: usize| -> Option<char> {
        let digitos: String = (0..n)
            .map(|_| chars.next().filter(char::is_ascii_hexdigit))
            .collect::<Option<_>>()?;
        char::from_u32(u32::from_str_radix(&digitos, 16).ok()?)
    };
    while let Some(c) = chars.next() {
        if c == comilla || matches!(c, '\n' | '\r') {
            return None;
        }
        if c != '\\' {
            salida.push(c);
            continue;
        }
        match chars.next()? {
            '\n' => {}
            '\\' => salida.push('\\'),
            '\'' => salida.push('\''),
            '"' => salida.push('"'),
            'a' => salida.push('\u{7}'),
            'b' => salida.push('\u{8}'),
            'f' => salida.push('\u{c}'),
            'n' => salida.push('\n'),
            'r' => salida.push('\r'),
            't' => salida.push('\t'),
            'v' => salida.push('\u{b}'),
            'x' => salida.push(hex(&mut chars, 2)?),
            'u' => salida.push(hex(&mut chars, 4)?),
            'U' => salida.push(hex(&mut chars, 8)?),
            d @ '0'..='7' => {
                let mut valor = d.to_digit(8)?;
                for _ in 0..2 {
                    match chars.peek().and_then(|c| c.to_digit(8)) {
                        Some(o) => {
                            valor = valor * 8 + o;
                            chars.next();
                        }
                        None => break,
                    }
                }
                salida.push(char::from_u32(valor)?);
            }
            otro => {
                salida.push('\\');
                salida.push(otro);
            }
        }
    }
    Some(salida)
}

/// `html.unescape`: las entidades se resuelven con el tokenizador HTML5 en un contenido RCDATA, sin interpretar etiquetas.
fn sin_entidades(texto: &str) -> Option<String> {
    if !texto.contains('&') {
        return Some(texto.to_string());
    }
    if texto.to_ascii_lowercase().contains("</textarea") {
        return None;
    }
    let fragmento = Html::parse_fragment(&format!("<textarea>\n{texto}</textarea>"));
    let valor = fragmento.select(&TEXTAREA).next()?.text().collect::<String>();
    Some(valor)
}

fn literal_js(literal: &str) -> Result<String, String> {
    cadena_literal(&literal.replace("\\/", "/"))
        .and_then(|s| sin_entidades(&s))
        .ok_or_else(|| "No se pueden comprobar los registros del calendario.".into())
}

fn registros_calendario(raiz: &Html, scripts: &str) -> Result<Registros, String> {
    let mut valores = Vec::new();
    for nombre in ["Ritxdias", "Nombresxdias"] {
        let literal = Regex::new(&format!(
            r#"{nombre}\[dia\]\s*=\s*("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')\s*;"#
        ))
        .unwrap();
        let textos: Vec<_> = literal
            .captures_iter(scripts)
            .map(|c| literal_js(&c[1]))
            .collect::<Result<_, _>>()?;
        valores.push(textos);
    }
    if valores[0].len() != valores[1].len() {
        return Err("El calendario contiene registros incompletos.".into());
    }
    let tablas: Vec<_> = raiz
        .select(&TABLAS)
        .filter(|t| {
            matches!(
                t.value().attr("id"),
                Some("TablaCalendarioSemana" | "TablaCalendarioDia")
            )
        })
        .collect();
    if tablas.len() != 2
        || tablas
            .iter()
            .map(|t| filas_de_datos(*t))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|n| *n != valores[0].len())
    {
        return Err("SITFA cambio la estructura del calendario.".into());
    }
    let mut registros = Registros::new();
    for (rit, nombre) in valores[0].iter().zip(&valores[1]) {
        *registros.entry(clave(rit, nombre)?).or_default() += 1;
    }
    Ok(registros)
}

fn tabla_registros(form: ElementRef<'_>, consulta: &Consulta) -> Result<Registros, String> {
    let mut candidatas = Vec::new();
    for tabla in form.select(&TABLAS) {
        if consulta.pantalla == Pantalla::Litigantes {
            let filas = grilla_con_cuerpo(tabla)?;
            let mut con_cabecera = false;
            for (fila, _) in &filas {
                con_cabecera |= columnas(fila)?.is_some();
            }
            if con_cabecera {
                candidatas.push(filas);
            }
        } else if tabla.value().attr("id") == Some(consulta.tabla) {
            candidatas.push(grilla_con_cuerpo(tabla)?);
        }
    }
    if candidatas.len() != 1 {
        return Err("No se encontro un unico listado de la consulta seleccionada.".into());
    }
    contar_listado(&candidatas[0])
}

fn texto_scripts(raiz: &Html) -> String {
    raiz.select(&SCRIPTS)
        .map(|e| e.text().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn formulario_unico<'a>(raiz: &'a Html) -> Result<ElementRef<'a>, String> {
    if raiz.select(&ENTRADAS).any(|e| {
        e.value()
            .attr("type")
            .is_some_and(|t| t.eq_ignore_ascii_case("password"))
    }) {
        return Err("La sesion expiro. Cierre la sesion y vuelva a ingresar en Chrome.".into());
    }
    let formularios: Vec<_> = raiz.select(&FORMULARIO).collect();
    if formularios.len() != 1 {
        return Err("SITFA no devolvio el formulario esperado.".into());
    }
    let form = formularios[0];
    if !matches!(form.value().attr("action"), Some(DESTINO | DESTINO_ABSOLUTO))
        || !form
            .value()
            .attr("method")
            .is_some_and(|m| m.eq_ignore_ascii_case("post"))
    {
        return Err("SITFA no devolvio el formulario esperado.".into());
    }
    Ok(form)
}

/// `perfiles.response_profile` y `motor.search_page`. Con `esperada`, la página pedida debe conservar total y plantilla
/// de la primera; una continuación vacía siempre es un error.
pub fn validar_respuesta(
    datos: &[u8],
    pares: &[(String, String)],
    consulta: &Consulta,
    esperada: Option<(usize, &Pagina)>,
) -> Result<Respuesta, String> {
    consulta.validar_consulta(pares)?;
    if datos.is_empty() || datos.len() > LIMITE_BYTES {
        return Err("Respuesta HTML vacia o ilegible.".into());
    }
    let raiz = Html::parse_document(&decodificar(datos));
    let form = formulario_unico(&raiz)?;
    let scripts = texto_scripts(&raiz);
    let mut fields = campos(form)?;
    if consulta.pantalla == Pantalla::Seguimiento && consulta.objetivo.contains_key("TIP_Informe") {
        if let Some(informe) = filtro::seleccionado(&scripts, consulta.pestana)? {
            if !fields.contains_key("TIP_Informe") {
                return Err("Falta el selector de vencimiento de la respuesta.".into());
            }
            fields.insert("TIP_Informe".into(), informe);
        }
    }
    if consulta.objetivo.iter().any(|(k, v)| {
        k != "COD_Lengueta"
            && !(consulta.pantalla == Pantalla::Litigantes && k == "TIP_Consulta")
            && fields.get(k) != Some(v)
    }) {
        return Err("La respuesta no corresponde a la seleccion realizada.".into());
    }
    let lengueta = fields.get("COD_Lengueta").map(String::as_str);
    if consulta.pantalla == Pantalla::Seguimiento
        && lengueta != Some("")
        && lengueta != Some(consulta.pestana)
    {
        return Err("La respuesta pertenece a otra pestaña.".into());
    }
    if fields.keys().any(|k| {
        let k = k.to_ascii_lowercase();
        ["csrf", "token", "nonce"].iter().any(|p| k.contains(p))
    }) {
        return Err("Aparecio un campo dinamico no auditado; requiere revision.".into());
    }
    let registros = if consulta.calendario() {
        registros_calendario(&raiz, &scripts)?
    } else {
        tabla_registros(form, consulta)?
    };
    if registros.is_empty() {
        if esperada.is_some() {
            return Err("No se pudieron identificar todos los registros de la pagina.".into());
        }
        if !consulta.calendario() {
            let otras_filas = if consulta.pantalla == Pantalla::Seguimiento && lengueta == Some("") {
                let mut hay = false;
                for tabla in form.select(&TABLAS) {
                    if PESTANAS_SITFA
                        .iter()
                        .any(|t| t.4 != consulta.tabla && tabla.value().attr("id") == Some(t.4))
                    {
                        hay |= !contar(&grilla(tabla)?)?.is_empty();
                    }
                }
                hay
            } else {
                false
            };
            if otras_filas
                || !matches!(
                    fields.get(consulta.total).map(String::as_str),
                    None | Some("" | "0" | "1")
                )
            {
                return Err("La respuesta vacia no coincide con la pestaña seleccionada.".into());
            }
        }
        return Ok(Respuesta::SinRegistros);
    }
    let (actual, total) = if consulta.calendario() {
        (1, 1)
    } else {
        let numero = |k: &str| {
            fields
                .get(k)
                .and_then(|v| v.trim().parse::<usize>().ok())
                .ok_or_else(|| {
                    "No se puede determinar la paginacion de la pestaña seleccionada.".to_string()
                })
        };
        (numero(consulta.pagina)?, numero(consulta.total)?)
    };
    if !(1..=500).contains(&total) || actual == 0 || actual > total {
        return Err("Paginacion fuera de rango; descarga detenida.".into());
    }
    let (url, plantilla) = exportacion(&scripts, consulta)?;
    match esperada {
        Some((numero, primera)) => {
            if actual != numero {
                return Err("El servidor devolvio una pagina distinta de la solicitada.".into());
            }
            if total != primera.total {
                return Err("Cambio el total de paginas durante la consulta; repetir en una ejecucion nueva.".into());
            }
            if plantilla != primera.plantilla {
                return Err("El enlace Excel no coincide con la consulta actual.".into());
            }
        }
        None if actual != 1 => {
            return Err("La consulta inicial no corresponde a la primera pagina.".into())
        }
        None => {}
    }
    Ok(Respuesta::ConRegistros(Pagina {
        actual,
        total,
        excel_url: url,
        plantilla,
        registros,
        paginacion: fields
            .into_iter()
            .filter(|(k, _)| es_paginacion(k))
            .collect(),
    }))
}

/// `motor.run_sequence`: la página siguiente toma la paginación de la anterior y conserva el resto de la consulta.
pub fn siguiente_consulta(
    pares: &[(String, String)],
    consulta: &Consulta,
    anterior: &Pagina,
    numero: usize,
) -> Result<Pares, String> {
    if consulta.calendario() || numero < 2 || numero > anterior.total {
        return Err("Paginacion fuera de rango; descarga detenida.".into());
    }
    let resultado: Pares = pares
        .iter()
        .map(|(k, v)| {
            let valor = if k == consulta.pagina {
                numero.to_string()
            } else if es_paginacion(k) {
                anterior.paginacion.get(k).cloned().unwrap_or_default()
            } else {
                v.clone()
            };
            (k.clone(), valor)
        })
        .collect();
    let fijo = |p: &[(String, String)]| -> Pares {
        p.iter().filter(|(k, _)| !es_paginacion(k)).cloned().collect()
    };
    if fijo(&resultado) != fijo(pares) {
        return Err("Cambio la consulta entre paginas; descarga detenida.".into());
    }
    consulta.validar_consulta(&resultado)?;
    Ok(resultado)
}

/// La vista del PDF debe identificar la consulta y mostrar los controles de la selección (`capture_empty`).
pub fn validar_pantalla_evidencia(
    datos: &[u8],
    pares: &[(String, String)],
    consulta: &Consulta,
    contexto: &[[String; 2]],
    fechas: Option<(&str, &str)>,
) -> Result<(), String> {
    if validar_respuesta(datos, pares, consulta, None)? != Respuesta::SinRegistros {
        return Err(
            "La consulta cambió y ahora tiene registros. Repite el lote; no se guardó un PDF vacío.".into(),
        );
    }
    let raiz = Html::parse_document(&decodificar(datos));
    let banners: Vec<_> = raiz.select(&BANNER).collect();
    let texto = format!(
        "Consulta solicitada · {}",
        contexto
            .iter()
            .map(|[k, v]| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join(" · ")
    );
    let json = serde_json::to_string(contexto).map_err(|_| "Contexto de consulta inválido.")?;
    if banners.len() != 1
        || banners[0].value().attr("data-context") != Some(json.as_str())
        || banners[0].text().collect::<String>() != texto
    {
        return Err("La pantalla no identifica la consulta solicitada. Recarga la extensión Lite.".into());
    }
    let campos = campos(formulario_unico(&raiz)?)?;
    if consulta.objetivo.iter().any(|(k, v)| campos.get(k) != Some(v)) {
        return Err("Los controles de la pantalla a capturar no corresponden a la selección.".into());
    }
    if fechas.is_some_and(|(desde, hasta)| {
        campos.get("FEC_Inicio").map(String::as_str) != Some(desde)
            || campos.get("FEC_Fin").map(String::as_str) != Some(hasta)
    }) {
        return Err("Las fechas de la pantalla a capturar no corresponden al lote.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perfil_espera() -> Perfil {
        Perfil {
            pantalla: Pantalla::Seguimiento,
            tribunal: "901".into(),
            alcance: Alcance::Seguimiento {
                pestana: "Espera".into(),
                modalidad: "2".into(),
                reporte: None,
            },
            etiqueta: String::new(),
        }
    }

    fn pares(lista: &[(&str, &str)]) -> Pares {
        lista.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn pagina(filas: &str, pagina: usize, total: usize, extra: &str) -> Vec<u8> {
        format!(
            r#"<html><body><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">
            <select name="COD_Tribunal_sel"><option value="901" selected>901</option></select>
            <select name="TIP_Consulta"><option value="2" selected>2</option></select>
            <input name="COD_Lengueta" value=""><input name="NUM_PaginaEspera" value="{pagina}">
            <input name="NUM_TotalEspera" value="{total}">{extra}
            <table id="tablaEspera"><tr><td>RIT</td><td>Nombre</td></tr>{filas}</table>
            <table id="tablaEgresados"><tr><td>RIT</td><td>Nombre</td></tr></table>
            </form><script>function ShowExcel(){{window.open("https://familia.pjud.cl/sitfa/reportes/u1_listdemo.xls");}}</script>
            </body></html>"#
        )
        .into_bytes()
    }

    fn consulta_base() -> Pares {
        pares(&[
            ("COD_Tribunal_sel", "901"),
            ("COD_Lengueta", "tdEspera"),
            ("TIP_Consulta", "2"),
            ("NUM_PaginaEspera", ""),
            ("NUM_TotalEspera", ""),
            ("FLG_Consulta", "0"),
            ("irAccion", "Buscar medida"),
        ])
    }

    #[test]
    fn valida_pagina_con_registros_y_arma_la_siguiente() {
        let c = Consulta::desde(&perfil_espera()).unwrap();
        let p = consulta_base();
        let filas = "<tr><td>X-10-2026</td><td>Persona Ficticia Uno</td></tr><tr><td>X-11-2026</td><td>Persona Ficticia Dos</td></tr>";
        let Respuesta::ConRegistros(primera) =
            validar_respuesta(&pagina(filas, 1, 2, ""), &p, &c, None).unwrap()
        else {
            panic!("se esperaban registros");
        };
        assert_eq!((primera.actual, primera.total), (1, 2));
        assert_eq!(primera.plantilla, "listdemo");
        assert_eq!(primera.registros.values().sum::<usize>(), 2);
        let siguiente = siguiente_consulta(&p, &c, &primera, 2).unwrap();
        assert!(siguiente.contains(&("NUM_PaginaEspera".into(), "2".into())));
        assert!(siguiente.contains(&("NUM_TotalEspera".into(), "2".into())));
        assert!(validar_respuesta(&pagina(filas, 1, 2, ""), &siguiente, &c, Some((2, &primera))).is_err());
        assert!(validar_respuesta(&pagina(filas, 2, 3, ""), &siguiente, &c, Some((2, &primera))).is_err());
        assert!(validar_respuesta(&pagina(filas, 2, 2, ""), &siguiente, &c, Some((2, &primera))).is_ok());
    }

    #[test]
    fn rechaza_filas_desconocidas_campos_dinamicos_sesion_y_consultas_ajenas() {
        let c = Consulta::desde(&perfil_espera()).unwrap();
        let p = consulta_base();
        let desconocida = "<tr><td>X-10-2026</td><td>Persona Ficticia</td></tr><tr><td>texto</td><td>libre</td></tr>";
        assert!(validar_respuesta(&pagina(desconocida, 1, 1, ""), &p, &c, None)
            .unwrap_err()
            .contains("todos los registros"));
        let token = r#"<input name="csrf_token" value="x">"#;
        assert!(validar_respuesta(&pagina("", 1, 1, token), &p, &c, None).is_err());
        let acceso = r#"<input type="password" name="clave">"#;
        assert!(validar_respuesta(&pagina("", 1, 1, acceso), &p, &c, None)
            .unwrap_err()
            .contains("sesion"));
        let mut ajena = p.clone();
        ajena[0].1 = "902".into();
        assert!(validar_respuesta(&pagina("", 1, 1, ""), &ajena, &c, None).is_err());
        let mut no_auditada = p.clone();
        no_auditada.push(("navegar".into(), "1".into()));
        assert!(c.validar_consulta(&no_auditada).is_err());
        assert_eq!(
            validar_respuesta(&pagina("<tr><td>Sin registros</td></tr>", 1, 1, ""), &p, &c, None).unwrap(),
            Respuesta::SinRegistros
        );
        assert!(validar_respuesta(&pagina("", 1, 4, ""), &p, &c, None).is_err());
    }

    #[test]
    fn la_url_de_descarga_queda_en_el_sitio_y_la_plantilla() {
        assert!(validar_descarga("https://familia.pjud.cl/sitfa/reportes/u1_listdemo.xls", "listdemo").is_ok());
        assert!(validar_descarga("https://familia.pjud.cl/sitfa/reportes/u1_otra.xls", "listdemo").is_err());
        assert!(validar_descarga("https://otro.invalid/sitfa/reportes/u1_listdemo.xls", "listdemo").is_err());
        assert!(validar_descarga("https://familia.pjud.cl/sitfa/reportes/u1_listdemo.xls?x=1", "listdemo").is_err());
        assert_eq!(sin_entidades("Mu&ntilde;oz &amp; Cia").unwrap(), "Muñoz & Cia");
        assert_eq!(cadena_literal(r#""Año""#).unwrap(), "Año");
    }
}
