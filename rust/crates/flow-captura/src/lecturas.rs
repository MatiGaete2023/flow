//! Lectura local de ventanas de bitácora, historia y calendario. Nunca ejecuta scripts ni sigue enlaces.
//! La identidad remota se coteja antes de leer. Una estructura desconocida es error, nunca una lista vacía.

use chrono::{NaiveDate, NaiveDateTime};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

const IDENTIDAD: [&str; 5] = ["COD_Tribunal", "CRR_IdCausa", "ID_Ingreso", "COD_Etapa", "TIP_Consulta"];
const OCULTOS: [&str; 5] = ["HOBS_Centro_", "HOBS_Tribunal_", "HCOD_Estado_", "HFUN_Tribunal_", "HFLG_Envio_"];
const BITACORA: [&str; 9] = [
    "Tipo Observación",
    "Etapa",
    "Fecha Centro",
    "Usuario Centro",
    "Observación Centro",
    "Fecha Tribunal",
    "Usuario Tribunal",
    "Observación Tribunal",
    "Acción",
];
const HISTORIA: [&str; 9] = ["Folio", "Doc.", "Sel", "Anexo", "Etapa", "Trámite", "Desc. Trámite", "Referencia", "Fecha Tram"];
const CABECERA: [&str; 10] =
    ["RIT", "RUC", "Caratulado", "F. Ingreso", "Proc.", "Forma Inicio", "Est. Adm.", "Etapa", "Estado Proc.", "Tribunal"];
static CHARSET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)<meta\b[^>]*\bcharset\s*=\s*["']?\s*([a-zA-Z0-9_-]+)"#).unwrap());
static CIERRE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</html\s*>").unwrap());

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("selector constante")
}
fn normalizar(s: &str) -> String {
    flow_dominio::texto::normalizar_columna(s)
}
pub(crate) fn texto(e: ElementRef<'_>) -> String {
    let partes = e
        .descendants()
        .filter(|n| !n.ancestors().filter_map(ElementRef::wrap).any(|e| matches!(e.value().name(), "script" | "style" | "noscript" | "template")))
        .filter_map(|n| n.value().as_text())
        .map(|t| t.as_ref())
        .collect::<Vec<_>>()
        .join(" ");
    partes.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Respeta la codificación declarada y falla si faltan bytes o hay caracteres no decodificables.
pub fn decodificar(datos: &[u8]) -> Result<String, String> {
    if datos.is_empty() || datos.len() > 8 * 1024 * 1024 {
        return Err("Respuesta HTML vacía o demasiado grande.".into());
    }
    let prefijo = String::from_utf8_lossy(&datos[..datos.len().min(8192)]);
    let charset = CHARSET.captures(&prefijo).map(|c| c[1].to_ascii_lowercase()).unwrap_or_default();
    if !["", "utf-8", "utf8", "windows-1252", "cp1252", "iso-8859-1", "latin1", "latin-1"].contains(&charset.as_str()) {
        return Err("Codificación HTML de captura no admitida.".into());
    }
    let datos = datos.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(datos);
    if charset == "utf-8" || charset == "utf8" {
        return std::str::from_utf8(datos).map(String::from).map_err(|_| "No se pudo decodificar íntegramente el HTML.".into());
    }
    if charset.is_empty() {
        if let Ok(s) = std::str::from_utf8(datos) {
            return Ok(s.into());
        }
    }
    encoding_rs::WINDOWS_1252
        .decode_without_bom_handling_and_without_replacement(datos)
        .map(|s| s.into_owned())
        .ok_or("No se pudo decodificar íntegramente el HTML.".into())
}

fn documento(datos: &[u8]) -> Result<Html, String> {
    let texto = decodificar(datos)?;
    if !CIERRE.is_match(&texto) {
        return Err("La respuesta HTML está incompleta.".into());
    }
    let doc = Html::parse_document(&texto);
    if doc.select(&sel("input")).any(|e| e.value().attr("type").is_some_and(|t| t.eq_ignore_ascii_case("password"))) {
        return Err("La respuesta es una página de acceso; revisa la sesión de SITFA.".into());
    }
    Ok(doc)
}

fn formulario(doc: &Html) -> Result<ElementRef<'_>, String> {
    let forms = doc.select(&sel("form[name='InformesPpalForm']")).collect::<Vec<_>>();
    if forms.len() != 1 {
        return Err("No se encontró un formulario único de la ventana solicitada.".into());
    }
    Ok(forms[0])
}

fn input(form: ElementRef<'_>, nombre: &str) -> Result<String, String> {
    let campos = form.select(&sel("input[name]")).filter(|e| e.value().attr("name") == Some(nombre)).collect::<Vec<_>>();
    if campos.len() != 1 {
        return Err("Campo ausente o repetido en la ventana solicitada.".into());
    }
    Ok(campos[0].value().attr("value").unwrap_or("").into())
}

fn filas(tabla: ElementRef<'_>) -> Vec<ElementRef<'_>> {
    tabla
        .select(&sel("tr"))
        .filter(|r| r.ancestors().filter_map(ElementRef::wrap).find(|e| e.value().name() == "table").map(|e| e.id()) == Some(tabla.id()))
        .collect()
}
fn celdas(fila: ElementRef<'_>) -> Vec<ElementRef<'_>> {
    fila.children().filter_map(ElementRef::wrap).filter(|e| matches!(e.value().name(), "td" | "th")).collect()
}
fn encabezado(fila: ElementRef<'_>, esperado: &[&str]) -> bool {
    celdas(fila).iter().map(|e| normalizar(&texto(*e))).eq(esperado.iter().map(|s| normalizar(s)))
}
fn positivo(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && s.bytes().any(|b| b != b'0')
}

fn fecha_hora(s: &str, opcional: bool) -> Result<String, String> {
    let s = s.trim();
    if opcional && matches!(s, "" | "---") {
        return Ok(String::new());
    }
    for formato in ["%d/%m/%Y %H:%M:%S", "%d/%m/%Y %H:%M"] {
        if let Ok(f) = NaiveDateTime::parse_from_str(s, formato) {
            return Ok(f.format("%Y-%m-%dT%H:%M:%S").to_string());
        }
    }
    NaiveDate::parse_from_str(s, "%d/%m/%Y")
        .map(|f| format!("{f}T00:00:00"))
        .map_err(|_| "La captura contiene una fecha de bitácora ilegible.".into())
}

pub fn leer_bitacora(datos: &[u8], params: &BTreeMap<String, String>, fuente: &str, capturada: &str) -> Result<Value, String> {
    let tipo = params.get("tipo_popUp").map(String::as_str).unwrap_or("");
    if !matches!(tipo, "11" | "12" | "13") {
        return Err("La captura no corresponde a una bitácora RUS.".into());
    }
    let doc = documento(datos)?;
    let form = formulario(&doc)?;
    if !matches!(form.value().attr("action"), Some("/SITFAWEB/InformesDAction.do" | "https://familia.pjud.cl/SITFAWEB/InformesDAction.do"))
    {
        return Err("El formulario de bitácora no tiene el destino esperado.".into());
    }
    let mut contexto = BTreeMap::new();
    for (i, k) in IDENTIDAD.iter().enumerate() {
        let valor = input(form, k)?;
        if valor.is_empty() || params.get(*k) != Some(&valor) || i < 4 && !positivo(&valor) {
            return Err("La respuesta no corresponde al ingreso solicitado.".into());
        }
        contexto.insert(*k, valor);
    }
    let recibido = input(form, "tipo_popUp")?;
    if !recibido.is_empty() && recibido != tipo {
        return Err("El tipo de ventana no corresponde a bitácora.".into());
    }
    let tablas = form.select(&sel("table[id='TablaInforme']")).collect::<Vec<_>>();
    if tablas.len() != 1 {
        return Err("La captura no contiene una tabla de bitácora única.".into());
    }
    let tabla = tablas[0];
    let filas = filas(tabla);
    if filas.is_empty() || !encabezado(filas[0], &BITACORA) {
        return Err("Cambió la estructura de la tabla de bitácora.".into());
    }
    let mut ocultos = BTreeMap::new();
    for campo in tabla.select(&sel("input")) {
        let nombre = campo.value().attr("name").unwrap_or("");
        if !OCULTOS.iter().any(|p| nombre.starts_with(p)) {
            continue;
        }
        if !campo.value().attr("type").is_some_and(|t| t.eq_ignore_ascii_case("hidden"))
            || ocultos.insert(nombre.to_string(), campo.value().attr("value").unwrap_or("").to_string()).is_some()
        {
            return Err("Campo de texto completo inválido o repetido.".into());
        }
    }
    let base = json!({"tribunal_codigo":contexto["COD_Tribunal"],"causa_id":contexto["CRR_IdCausa"],"ingreso_id":contexto["ID_Ingreso"],"rit":input(form,"RIT_Causa")?,"persona":input(form,"GLS_Nombre")?,"centro":input(form,"GLS_Centro")?});
    let (mut entries, mut metadata, mut advertencias) = (vec![], vec![], vec![]);
    let (mut ids, mut usados) = (BTreeSet::new(), BTreeSet::new());
    for (numero, fila) in filas.iter().skip(1).enumerate() {
        let cells = celdas(*fila);
        if cells.len() != 9 {
            return Err("Fila de bitácora incompleta.".into());
        }
        let valores = cells.iter().map(|e| texto(*e)).collect::<Vec<_>>();
        let encontrados = cells[8].select(&sel("img[id]")).map(|e| e.value().attr("id").unwrap().to_string()).collect::<BTreeSet<_>>();
        if encontrados.len() != 1 {
            return Err("No se puede vincular la fila con su observación.".into());
        }
        let id = encontrados.into_iter().next().unwrap();
        if !positivo(&id) || !ids.insert(id.clone()) {
            return Err("La bitácora contiene una identidad de observación inválida o repetida.".into());
        }
        let mut valores_ocultos = vec![];
        for prefijo in OCULTOS {
            let nombre = format!("{prefijo}{id}");
            valores_ocultos.push(ocultos.get(&nombre).cloned().ok_or("Falta el texto completo o los metadatos de una observación.")?);
            usados.insert(nombre);
        }
        let respuesta = if valores_ocultos[1].trim() == "---" { "" } else { &valores_ocultos[1] };
        let fecha_respuesta = fecha_hora(&valores[5], true)?;
        if !respuesta.trim().is_empty() != !fecha_respuesta.is_empty() {
            advertencias.push(format!("Entrada {id}: respuesta y fecha del tribunal no concuerdan."));
        }
        let mut entrada = base.clone();
        let objeto = entrada.as_object_mut().unwrap();
        for (k,v) in json!({"entry_id":id,"fecha":fecha_hora(&valores[2],false)?,"autor":valores[3],"tipo":valores[0],"etapa":valores[1],"texto":valores_ocultos[0],"origen":"centro","respuesta":respuesta,"fecha_respuesta":fecha_respuesta,"respuesta_comprobada":true,"fuente":fuente,"fila_fuente":numero+1}).as_object().unwrap(){objeto.insert(k.clone(),v.clone());}
        entries.push(entrada);
        metadata.push(
            json!({"entry_id":id,"state":valores_ocultos[2],"send_to_tribunal":valores_ocultos[4],"tribunal_user":valores_ocultos[3]}),
        );
    }
    if ocultos.keys().cloned().collect::<BTreeSet<_>>() != usados {
        return Err("Hay textos ocultos sin una fila correspondiente.".into());
    }
    for nodo in doc.select(&sel("input, img")) {
        if nodo.ancestors().any(|p| p.id() == tabla.id()) {
            continue;
        }
        if nodo.value().attr("name").is_some_and(|n| OCULTOS.iter().any(|p| n.starts_with(p)))
            || nodo.value().name() == "img" && nodo.value().attr("id").is_some_and(positivo) && nodo.value().attr("onclick").is_some()
        {
            return Err("Hay observaciones fuera de la tabla de bitácora; estructura no reconocida.".into());
        }
    }
    let mut resultado = base;
    let objeto = resultado.as_object_mut().unwrap();
    for (k,v) in json!({"entries":entries,"coverage":"PARCIAL","captured_at":capturada,"stage_code":contexto["COD_Etapa"],"source":fuente,"row_metadata":metadata,"warnings":advertencias,"html_sha256":crate::lotes::huella(datos),"captures":1,"error":"Tabla recuperada de una respuesta de bitácora. No acredita historial remoto completo."}).as_object().unwrap(){objeto.insert(k.clone(),v.clone());}
    Ok(resultado)
}

pub fn leer_historia(datos: &[u8], fuente: &str, capturada: &str) -> Result<Value, String> {
    let doc = documento(datos)?;
    let mut candidatas = vec![];
    let mut parecidas = false;
    for tabla in doc.select(&sel("table")) {
        let filas = filas(tabla);
        if let Some(primera) = filas.first() {
            if encabezado(*primera, &HISTORIA) {
                candidatas.push(filas);
            } else if celdas(*primera).first().is_some_and(|e| normalizar(&texto(*e)) == "folio") {
                parecidas = true;
            }
        }
    }
    if candidatas.len() != 1 {
        return Err(if parecidas && candidatas.is_empty() { "HISTORIA_ENCABEZADO_DISTINTO" } else { "HISTORIA_TABLA_NO_RECONOCIDA" }.into());
    }
    let filas = candidatas.pop().unwrap();
    let (mut resultado, mut advertencias) = (vec![], vec![]);
    for (i, fila) in filas.iter().skip(1).enumerate() {
        let c = celdas(*fila);
        let posicion = i + 1;
        if encabezado(*fila, &HISTORIA) {
            advertencias.push(json!({"codigo":"HISTORIA_ENCABEZADO_REPETIDO","fila":posicion,"folio":""}));
            continue;
        }
        if c.len() == 4 && posicion == filas.len() - 1 {
            advertencias.push(json!({"codigo":"HISTORIA_FILA_INCOMPLETA","fila":posicion,"folio":texto(c[0])}));
            continue;
        }
        if c.len() != 9 {
            return Err("HISTORIA_ENCABEZADO_DISTINTO".into());
        }
        let valores = c.iter().map(|e| texto(*e)).collect::<Vec<_>>();
        let fecha = NaiveDate::parse_from_str(&valores[8], "%d/%m/%Y").map(|f| f.to_string()).unwrap_or_default();
        if fecha.is_empty() {
            advertencias.push(json!({"codigo":"HISTORIA_FECHA_ILEGIBLE","fila":posicion,"folio":valores[0]}));
        }
        resultado.push(json!({"fila":posicion,"folio":valores[0],"etapa":valores[4],"tramite":valores[5],"descripcion":valores[6],"referencia":valores[7],"fecha_texto":valores[8],"fecha_iso":fecha,"anexo":c[3].select(&sel("img")).next().is_some()}));
    }
    let paginacion = doc.select(&sel(".pagination,.paginacion,.paginador,.pager")).next().is_some()
        || doc.select(&sel("input[name]")).any(|e| {
            let n = e.value().attr("name").unwrap();
            n.contains("pagina") || n.contains("nroPag") || n.contains("numPag")
        })
        || doc.select(&sel("a")).any(|e| texto(e).to_lowercase() == "siguiente");
    if paginacion {
        advertencias.push(json!({"codigo":"HISTORIA_POSIBLE_PAGINACION","fila":0,"folio":""}));
    }
    let mut cabecera = BTreeMap::new();
    for etiqueta in doc.select(&sel("label")) {
        let clave = normalizar(texto(etiqueta).trim_end_matches([':', ' ']));
        if let Some(k) = CABECERA.iter().find(|k| normalizar(k) == clave) {
            let valor = etiqueta
                .next_sibling()
                .and_then(|n| n.value().as_text())
                .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            cabecera.entry(*k).or_insert(valor);
        }
    }
    let reservada = doc.select(&sel(".panel-heading")).any(|e| normalizar(&texto(e)).contains("reservada"));
    Ok(
        json!({"encabezado":cabecera,"reservada":reservada,"filas":resultado,"vacio_explicito":filas.len()==1,"advertencias":advertencias,"sha256":crate::lotes::huella(datos),"source":fuente,"captured_at":capturada}),
    )
}

fn fecha_calendario(s: &str, opcional: bool) -> Result<String, String> {
    let s = s.trim();
    if opcional && matches!(s, "" | "-" | "--" | "---") {
        return Ok(String::new());
    }
    static FORMA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A\d{2}/\d{2}/\d{4}(?: \d{1,2}:\d{2}(?::\d{2})?)?\z").unwrap());
    if !FORMA.is_match(s) {
        return Err("El calendario contiene una fecha ilegible.".into());
    }
    let fecha = fecha_hora(s, false)?;
    Ok(fecha[..10].to_string())
}

/// Ventana de meses naturales (1 a 4), sin convertir fechas ilegibles en ausencia de trámites.
/// Devuelve: dentro, anteriores, posteriores y sin fecha.
pub fn separar_ventana(filas: &[Value], hasta: NaiveDate, meses: u32) -> Result<[Vec<Value>; 4], String> {
    if !(1..=4).contains(&meses) {
        return Err("El período a recolectar va de 1 a 4 meses.".into());
    }
    let desde = hasta.checked_sub_months(chrono::Months::new(meses)).ok_or("La ventana de historia no es válida.")?;
    let mut salida: [Vec<Value>; 4] = Default::default();
    for fila in filas {
        let fecha = fila["fecha_iso"].as_str().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
        let indice = match fecha {
            None => 3,
            Some(f) if f < desde => 1,
            Some(f) if f > hasta => 2,
            _ => 0,
        };
        salida[indice].push(fila.clone());
    }
    Ok(salida)
}

pub fn leer_calendario(datos: &[u8], args: &[String], fuente: &str, capturada: &str) -> Result<Value, String> {
    if args.len() != 12 {
        return Err("El enlace del calendario no tiene la estructura esperada.".into());
    }
    let doc = documento(datos)?;
    let form = formulario(&doc)?;
    let (tribunal, causa, ingreso) = (input(form, "COD_Tribunal")?, input(form, "CRR_IdCausa")?, input(form, "ID_Ingreso")?);
    if (&tribunal, &causa, &ingreso) != (&args[2], &args[0], &args[10]) || ![&tribunal, &causa, &ingreso].into_iter().all(|s| positivo(s)) {
        return Err("La respuesta del calendario no corresponde al ingreso solicitado.".into());
    }
    let mut candidatas = vec![];
    for tabla in doc.select(&sel("table")) {
        let filas = filas(tabla);
        if filas.first().is_some_and(|f| encabezado(*f, &["Fecha Proyección", "Fecha Informe", "Estado"])) {
            candidatas.push(filas);
        }
    }
    if candidatas.len() != 1 {
        return Err("Cambió la estructura del calendario; no se leyó para no perder proyecciones.".into());
    }
    let mut proyecciones = vec![];
    for (i, fila) in candidatas[0].iter().skip(1).enumerate() {
        let c = celdas(*fila).iter().map(|e| texto(*e)).collect::<Vec<_>>();
        if c.iter().all(|s| s.is_empty()) {
            continue;
        }
        if c.len() != 3 {
            return Err("Fila de calendario incompleta.".into());
        }
        let estado = c[2].to_uppercase();
        if !matches!(estado.as_str(), "ENVIADO" | "PENDIENTE") {
            return Err("Estado de calendario no reconocido.".into());
        }
        let informe = fecha_calendario(&c[1], true)?;
        if (estado == "ENVIADO") != !informe.is_empty() {
            return Err("Estado y fecha de informe del calendario no concuerdan.".into());
        }
        proyecciones.push(json!({"fila":i+1,"fecha_proyeccion":fecha_calendario(&c[0],false)?,"fecha_informe":informe,"estado":estado}));
    }
    let plazo = |etiqueta: &str| -> Option<u32> {
        let patron = Regex::new(r"(\d+)\s*mes").unwrap();
        for e in doc.select(&sel("td,th,span,label,b,strong")) {
            let t = normalizar(&texto(e));
            if t.starts_with(etiqueta) {
                if let Some(c) = patron.captures(&t) {
                    return c[1].parse().ok();
                }
            }
        }
        None
    };
    Ok(
        json!({"tribunal_codigo":tribunal,"causa_id":causa,"ingreso_id":ingreso,"proyecciones":proyecciones,"plazo_remision_meses":plazo("plazo remision"),"plazo_intervencion_meses":plazo("plazo intervencion"),"source":fuente,"captured_at":capturada,"html_sha256":crate::lotes::huella(datos)}),
    )
}
