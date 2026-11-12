//! Validación local de respuestas de SITFA Lite (paso 7.2). No hace peticiones ni ejecuta scripts.
//! Los errores son mensajes controlados: no devuelven HTML, nombres, rutas remotas ni valores de campos.

mod filtro;
mod perfil;
mod registros;

pub use perfil::{Perfil, Seleccion};
use perfil::{DESTINO, DESTINO_ABSOLUTO, PESTANAS};
use registros::{clave, columnas, contar, grilla, normalizar, registro};

use chrono::Datelike;
use flow_tabular::{leer_bytes, Libro};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

type Error = String;
pub type Pares = Vec<(String, String)>;
/// La cantidad importa: dos medidas pueden tener el mismo RIT y nombre.
pub type Registros = BTreeMap<(String, String), usize>;
type FilasCompletas = BTreeMap<(Vec<String>, Vec<String>), usize>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pagina {
    pub actual: usize,
    pub total: usize,
    pub excel_url: String,
    pub registros: Registros,
    pub paginacion: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Respuesta {
    SinRegistros,
    ConRegistros(Pagina),
}

fn selector(s: &str) -> Selector {
    Selector::parse(s).expect("selector constante del programa")
}

fn es_paginacion(k: &str) -> bool {
    PESTANAS.iter().any(|t| k == t.2 || k == t.3) || k == "NUM_Total"
}

fn campos(formulario: ElementRef<'_>) -> Result<BTreeMap<String, String>, Error> {
    let mut grupos: BTreeMap<String, Vec<ElementRef<'_>>> = BTreeMap::new();
    for el in formulario.select(&selector("input[name], select[name], textarea[name]")) {
        if el.value().name() == "input"
            && matches!(el.value().attr("type").unwrap_or("").to_ascii_lowercase().as_str(), "button" | "reset" | "image")
        {
            continue;
        }
        grupos.entry(el.value().attr("name").unwrap().into()).or_default().push(el);
    }
    let mut resultado = BTreeMap::new();
    for (nombre, elementos) in grupos {
        let radios = elementos
            .iter()
            .all(|e| e.value().name() == "input" && e.value().attr("type").is_some_and(|t| t.eq_ignore_ascii_case("radio")));
        if elementos.len() != 1 && !radios {
            return Err("Formulario ambiguo: campos repetidos.".into());
        }
        let el = if radios {
            let marcados: Vec<_> = elementos.iter().filter(|e| e.value().attr("checked").is_some()).collect();
            if marcados.len() > 1 {
                return Err("Formulario ambiguo: opciones de consulta repetidas.".into());
            }
            *marcados.first().copied().unwrap_or(&elementos[0])
        } else {
            elementos[0]
        };
        let valor = match el.value().name() {
            "select" => {
                let opciones: Vec<_> = el.select(&selector("option")).collect();
                let marcadas: Vec<_> = opciones.iter().filter(|o| o.value().attr("selected").is_some()).collect();
                if marcadas.len() > 1 {
                    return Err("Formulario ambiguo: selector con varias opciones elegidas.".into());
                }
                marcadas
                    .first()
                    .copied()
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

static SHOW_EXCEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bfunction\s+ShowExcel\s*\(\s*\)\s*\{").unwrap());
static FUNCION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bfunction\s+\w+\s*\(").unwrap());
static ABRIR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"window\.open\s*\(\s*(?:"([^"\r\n]+)"|'([^'\r\n]+)')\s*(?:,\s*(?:"[^"\r\n]*"|'[^'\r\n]*')\s*)?\)"#).unwrap()
});
static URL_XLS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\Ahttps://familia\.pjud\.cl/sitfa/reportes/[A-Za-z0-9_-]+\.xls\z").unwrap());

/// Devuelve la plantilla del enlace validado; no decodifica ni sigue la URL.
fn plantilla_url<'a>(url: &'a str, perfil: &Perfil) -> Result<&'a str, Error> {
    if !URL_XLS.is_match(url) {
        return Err("El enlace Excel está fuera del sitio permitido.".into());
    }
    let nombre = url.rsplit('/').next().unwrap().strip_suffix(".xls").unwrap();
    let plantilla = nombre.rsplit_once('_').map(|(_, p)| p).ok_or("El nombre Excel no se puede reconocer.")?;
    if !plantilla.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || plantilla.is_empty() && !(perfil.pantalla == "seguimiento" && perfil.objetivo["TIP_Consulta"] == "4")
        || perfil.plantilla.is_some_and(|p| p != plantilla)
    {
        return Err("El Excel corresponde a otra pantalla o no se reconoce su nombre.".into());
    }
    Ok(plantilla)
}

fn exportacion(scripts: &str, perfil: &Perfil) -> Result<String, Error> {
    let funciones: Vec<_> = SHOW_EXCEL.find_iter(scripts).collect();
    if funciones.len() != 1 {
        return Err("SITFA no entregó un único enlace Excel para esta consulta.".into());
    }
    let bloque = &scripts[funciones[0].end()..];
    let bloque = &bloque[..FUNCION.find(bloque).map(|m| m.start()).unwrap_or(bloque.len())];
    let enlaces: Vec<_> = ABRIR.captures_iter(bloque).collect();
    if enlaces.len() != 1 {
        return Err("No se encontró una única exportación XLS auditada.".into());
    }
    let url = enlaces[0].get(1).or_else(|| enlaces[0].get(2)).unwrap().as_str();
    plantilla_url(url, perfil)?;
    Ok(url.into())
}

fn literal_js(literal: &str) -> Result<String, Error> {
    // Los calendarios auditados contienen solo literales de texto, nunca expresiones.
    let json = if literal.starts_with('\'') {
        format!("\"{}\"", literal[1..literal.len() - 1].replace("\\'", "'").replace('"', "\\\""))
    } else {
        literal.to_string()
    };
    serde_json::from_str::<String>(&json)
        .map(|s| flow_tabular::html::sin_entidades(&s))
        .map_err(|_| "No se pueden comprobar los registros del calendario.".into())
}

fn registros_calendario(raiz: &Html, scripts: &str) -> Result<Registros, Error> {
    let mut valores = Vec::new();
    for nombre in ["Ritxdias", "Nombresxdias"] {
        let asignacion = Regex::new(&format!(r"\b{nombre}\[dia\]\s*=")).unwrap();
        let literal = Regex::new(&format!(r#"\b{nombre}\[dia\]\s*=\s*("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')\s*;"#)).unwrap();
        let textos: Vec<_> = literal.captures_iter(scripts).map(|c| literal_js(&c[1])).collect::<Result<_, _>>()?;
        if textos.len() != asignacion.find_iter(scripts).count() {
            return Err("El calendario contiene valores dinámicos no auditados.".into());
        }
        valores.push(textos);
    }
    if valores[0].len() != valores[1].len() {
        return Err("El calendario contiene registros incompletos.".into());
    }
    for id in ["TablaCalendarioSemana", "TablaCalendarioDia"] {
        let tablas: Vec<_> = raiz.select(&selector(&format!("table[id='{id}']"))).collect();
        if tablas.len() != 1 || grilla(tablas[0])?.len() != valores[0].len() {
            return Err("SITFA cambió la estructura del calendario.".into());
        }
    }
    let mut registros = Registros::new();
    for (rit, nombre) in valores[0].iter().zip(&valores[1]) {
        *registros.entry(clave(rit, nombre)?).or_default() += 1;
    }
    Ok(registros)
}

fn tabla_registros(form: ElementRef<'_>, perfil: &Perfil) -> Result<Registros, Error> {
    let mut candidatas = Vec::new();
    for tabla in form.select(&selector("table")) {
        if perfil.pantalla == "litigantes" {
            let filas = grilla(tabla)?;
            if filas.iter().map(|f| columnas(f)).collect::<Result<Vec<_>, _>>()?.iter().any(Option::is_some) {
                candidatas.push(filas);
            }
        } else if tabla.value().attr("id") == Some(perfil.tabla) {
            candidatas.push(grilla(tabla)?);
        }
    }
    if candidatas.len() != 1 {
        return Err("No se encontró un único listado de la consulta seleccionada.".into());
    }
    contar(&candidatas[0], true)
}

/// Valida la página recibida. Para continuar se exige el número pedido y la primera página validada:
/// el total y la plantilla no pueden cambiar. Una respuesta vacía a mitad de consulta siempre es un error.
pub fn validar_respuesta(
    datos: &[u8],
    pares: &[(String, String)],
    perfil: &Perfil,
    esperada: Option<(usize, &Pagina)>,
) -> Result<Respuesta, Error> {
    perfil.validar_consulta(pares)?;
    if datos.is_empty() || datos.len() > flow_tabular::libro::LIMITE_BYTES {
        return Err("Respuesta HTML vacía o demasiado grande.".into());
    }
    let raiz = Html::parse_document(&flow_tabular::html::decodificar(datos));
    if raiz.select(&selector("input")).any(|e| e.value().attr("type").is_some_and(|t| t.eq_ignore_ascii_case("password"))) {
        return Err("Sesión expirada: se recibió una página de acceso.".into());
    }
    let formularios: Vec<_> = raiz.select(&selector("form[name='InformesPpalForm']")).collect();
    if formularios.len() != 1 {
        return Err("SITFA no devolvió un único formulario esperado.".into());
    }
    let form = formularios[0];
    if !matches!(form.value().attr("action"), Some(DESTINO | DESTINO_ABSOLUTO))
        || !form.value().attr("method").is_some_and(|m| m.eq_ignore_ascii_case("post"))
    {
        return Err("El destino del formulario cambió; descarga detenida.".into());
    }
    let scripts = raiz.select(&selector("script")).map(|e| e.text().collect::<String>()).collect::<Vec<_>>().join("\n");
    let mut fields = campos(form)?;
    if perfil.pantalla == "seguimiento" && perfil.objetivo.contains_key("TIP_Informe") {
        if let Some(informe) = filtro::seleccionado(&scripts, perfil.pestaña)? {
            if !fields.contains_key("TIP_Informe") {
                return Err("Falta el selector de vencimiento de la respuesta.".into());
            }
            fields.insert("TIP_Informe".into(), informe);
        }
    }
    if fields.keys().any(|k| ["csrf", "token", "nonce"].iter().any(|p| k.to_ascii_lowercase().contains(p))) {
        return Err("Apareció un campo dinámico no auditado; requiere revisión.".into());
    }
    if perfil
        .objetivo
        .iter()
        .any(|(k, v)| k != "COD_Lengueta" && !(perfil.pantalla == "litigantes" && k == "TIP_Consulta") && fields.get(k) != Some(v))
    {
        return Err("La respuesta no corresponde a la selección realizada.".into());
    }
    let pestaña = fields.get("COD_Lengueta").map(String::as_str);
    if perfil.pantalla == "seguimiento" && pestaña != Some("") && pestaña != Some(perfil.pestaña) {
        return Err("La respuesta pertenece a otra pestaña.".into());
    }
    let registros = if perfil.calendario() { registros_calendario(&raiz, &scripts)? } else { tabla_registros(form, perfil)? };
    if registros.is_empty() {
        if esperada.is_some() {
            return Err("Una página de continuación quedó vacía; repetir la consulta.".into());
        }
        if !perfil.calendario() {
            if !matches!(fields.get(perfil.total).map(String::as_str), None | Some("" | "0" | "1"))
                || !matches!(fields.get(perfil.pagina).map(String::as_str), None | Some("" | "0" | "1"))
            {
                return Err("La respuesta vacía tiene una paginación incoherente.".into());
            }
            if perfil.pantalla == "seguimiento" && pestaña == Some("") {
                for tabla in form.select(&selector("table")) {
                    if PESTANAS.iter().any(|t| t.4 != perfil.tabla && tabla.value().attr("id") == Some(t.4))
                        && !contar(&grilla(tabla)?, true)?.is_empty()
                    {
                        return Err("La respuesta vacía no coincide con la pestaña seleccionada.".into());
                    }
                }
            }
        }
        return Ok(Respuesta::SinRegistros);
    }
    let (actual, total) = if perfil.calendario() {
        (1, 1)
    } else {
        let numero =
            |k| fields.get(k).and_then(|v| v.trim().parse::<usize>().ok()).ok_or("No se puede determinar la paginación.".to_string());
        (numero(perfil.pagina)?, numero(perfil.total)?)
    };
    if !(1..=500).contains(&total) || actual == 0 || actual > total {
        return Err("Paginación fuera de rango; descarga detenida.".into());
    }
    let url = exportacion(&scripts, perfil)?;
    if let Some((numero, primera)) = esperada {
        if actual != numero || total != primera.total || plantilla_url(&url, perfil)? != plantilla_url(&primera.excel_url, perfil)? {
            return Err("Cambió la página, el total o la exportación durante la consulta.".into());
        }
    } else if actual != 1 {
        return Err("La consulta no comenzó en la primera página.".into());
    }
    Ok(Respuesta::ConRegistros(Pagina {
        actual,
        total,
        excel_url: url,
        registros,
        paginacion: fields.into_iter().filter(|(k, _)| es_paginacion(k)).collect(),
    }))
}

/// Construye la consulta siguiente conservando todos los filtros originales.
pub fn siguiente_consulta(pares: &[(String, String)], perfil: &Perfil, primera: &Pagina, numero: usize) -> Result<Pares, Error> {
    perfil.validar_consulta(pares)?;
    if perfil.calendario() || numero < 2 || numero > primera.total {
        return Err("Página siguiente fuera de rango.".into());
    }
    let mut resultado: Pares = pares.iter().filter(|(k, _)| !es_paginacion(k)).cloned().collect();
    resultado.extend(primera.paginacion.iter().map(|(k, v)| (k.clone(), v.clone())));
    resultado.retain(|(k, _)| k != perfil.pagina && k != perfil.total);
    resultado.push((perfil.pagina.into(), numero.to_string()));
    resultado.push((perfil.total.into(), primera.total.to_string()));
    perfil.validar_consulta(&resultado)?;
    Ok(resultado)
}

/// La vista del PDF debe identificar la consulta y mostrar los controles efectivamente seleccionados.
pub fn validar_pantalla_evidencia(datos: &[u8], pares: &Pares, perfil: &Perfil, contexto: &[[String; 2]]) -> Result<(), String> {
    if validar_respuesta(datos, pares, perfil, None)? != Respuesta::SinRegistros {
        return Err("La consulta cambió y ahora tiene registros; repite el lote.".into());
    }
    let raiz = Html::parse_document(&flow_tabular::html::decodificar(datos));
    let banners: Vec<_> = raiz.select(&selector("#sitfaContextoConsulta")).collect();
    let texto = format!("Consulta solicitada · {}", contexto.iter().map(|[k, v]| format!("{k}: {v}")).collect::<Vec<_>>().join(" · "));
    let json = serde_json::to_string(contexto).map_err(|_| "Contexto de consulta inválido.")?;
    if banners.len() != 1
        || banners[0].value().attr("data-context") != Some(json.as_str())
        || banners[0].text().collect::<String>() != texto
    {
        return Err("La pantalla de evidencia no identifica la consulta solicitada.".into());
    }
    let form = raiz.select(&selector("form[name='InformesPpalForm']")).next().ok_or("Falta el formulario de evidencia.")?;
    let campos = campos(form)?;
    if perfil.objetivo.iter().any(|(k, v)| campos.get(k) != Some(v))
        || pares.iter().filter(|(k, _)| matches!(k.as_str(), "FEC_Inicio" | "FEC_Fin")).any(|(k, v)| campos.get(k) != Some(v))
    {
        return Err("Los controles de la pantalla a capturar no corresponden a la selección.".into());
    }
    Ok(())
}

#[derive(Debug)]
pub struct DescargaVerificada {
    pub libro: Libro,
    pub registros: Registros,
}

/// Estado de cotejo de una consulta. Solo retiene huellas y filas normalizadas en RAM.
#[derive(Default)]
pub struct VerificadorDescargas {
    huellas: BTreeSet<String>,
    filas: Vec<FilasCompletas>,
}

impl VerificadorDescargas {
    pub fn verificar(&mut self, datos: Vec<u8>, pagina: &Pagina, perfil: &Perfil) -> Result<DescargaVerificada, Error> {
        let libro = leer_bytes("descarga.xls", datos).map_err(|_| "No se pudo abrir el Excel de la consulta.".to_string())?;
        let mut candidatas = Vec::new();
        for hoja in &libro.hojas {
            let filas: Vec<Vec<String>> = hoja.filas.iter().map(|f| f.iter().map(|c| c.texto()).collect()).collect();
            if filas.len() > 200_000 || filas.iter().any(|f| f.len() > 200) {
                return Err("El Excel supera los límites de filas o columnas.".into());
            }
            let registros = contar(&filas, false)?;
            if !registros.is_empty() {
                candidatas.push((filas, registros));
            }
        }
        if candidatas.len() != 1 {
            return Err("No se reconoce un único listado de causas y nombres en el Excel.".into());
        }
        let (filas, registros) = candidatas.pop().unwrap();
        if registros != pagina.registros {
            return Err("Los registros XLS no coinciden con la página HTML.".into());
        }
        if perfil.calendario() {
            verificar_mes(&filas, perfil)?;
        }
        let mut completas = BTreeMap::new();
        let (mut cols, mut cabeceras) = (None, Vec::new());
        for fila in &filas {
            if let Some(c) = columnas(fila)? {
                cols = Some(c);
                cabeceras = fila.iter().map(|s| normalizar(s)).collect();
            } else if cols.as_ref().and_then(|c| registro(fila, c)).is_some() {
                *completas.entry((cabeceras.clone(), fila.iter().map(|s| normalizar(s)).collect())).or_insert(0) += 1;
            }
        }
        if self.huellas.contains(&libro.sha256) || self.filas.contains(&completas) {
            return Err("Dos páginas contienen el mismo archivo o las mismas filas; descarga detenida.".into());
        }
        self.huellas.insert(libro.sha256.clone());
        self.filas.push(completas);
        Ok(DescargaVerificada { libro, registros })
    }
}

fn verificar_mes(filas: &[Vec<String>], perfil: &Perfil) -> Result<(), Error> {
    let mut columnas_registro = None;
    let mut posicion = 0;
    let aliases =
        ["FECHA VENCIMIENTO", "FEC.VENCIMIENTO", "FEC. VENCIMIENTO", "F. VENCIMIENTO", "F.VENCIMIENTO", "F.VENC", "F. VENC", "VENCIMIENTO"];
    let egreso = ["FEC.EGRESO PROYECTADO", "FEC. EGRESO PROYECTADO", "FEC EGRESO PROYECTADO", "FECHA EGRESO PROYECTADO"];
    for fila in filas {
        if let Some(c) = columnas(fila)? {
            let fechas: Vec<_> = fila
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    aliases.contains(&normalizar(s).as_str())
                        || perfil.pantalla == "calendario_medidas" && egreso.contains(&normalizar(s).as_str())
                })
                .collect();
            if fechas.len() != 1 {
                return Err("El calendario XLS no permite comprobar sus fechas.".into());
            }
            columnas_registro = Some(c);
            posicion = fechas[0].0;
        } else if columnas_registro.as_ref().and_then(|c| registro(fila, c)).is_some() {
            let fecha = fila
                .get(posicion)
                .and_then(|s| flow_dominio::celda::fecha_de_texto(s))
                .ok_or("La fecha del calendario XLS está vacía o no se reconoce.")?;
            if fecha.month().to_string() != perfil.objetivo["COD_Mes_sel"] || fecha.year().to_string() != perfil.objetivo["COD_Anio_Sel"] {
                return Err("La fecha del calendario XLS no corresponde al mes consultado.".into());
            }
        }
    }
    Ok(())
}
