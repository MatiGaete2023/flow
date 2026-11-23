//! E-Q4c · Ingreso Histórico Protección del lote de bitácoras: `descargador/historico_manual.py` (manifiesto
//! `historico.json`, respaldo `historico/historico-*.html` y entradas por tribunal y persona) y la estructura ANÓNIMA de
//! `extraer_estructura_escritos.estructura_ventana` para una ventana sin la tabla esperada. Sin datos personales en la
//! estructura: solo cantidades y literales de catálogos cerrados.

use super::lecturas;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

pub const MANIFIESTO: &str = "historico.json";
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("selector constante")
}

fn sha256(datos: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(datos))
}

/// `escritos_html.MENSAJES` / `mensaje`.
pub fn mensaje(codigo: &str) -> String {
    match codigo {
        "ESCRITOS_NO_VERIFICADA" => "La pestaña Escritos no trae una tabla legible; la fuente queda sin verificar, no vacía.",
        "ESCRITOS_AUSENTE" => {
            "La respuesta no incluye la pestaña Escritos (otra versión de SITFA o no cargada): fuente no solicitada."
        }
        "HISTORICO_NO_VERIFICADA" => "La vista de Ingreso Histórico Protección no trae la tabla esperada; queda sin verificar.",
        "ESCRITOS_RESPUESTA_INCOMPLETA" => {
            "La respuesta llegó cortada (sin cierre del documento); se reintenta, no se lee como vacía."
        }
        "ESCRITOS_SESION" => "La respuesta parece la página de ingreso de SITFA; revisa la sesión.",
        otro => otro,
    }
    .to_string()
}

/// `_estado`: el manifiesto o uno vacío.
fn estado(carpeta: &Path) -> Result<Value, String> {
    let ruta = carpeta.join(MANIFIESTO);
    if !ruta.is_file() {
        return Ok(json!({"version": 1, "tipo": "HISTORICO_PROTECCION", "capturas": {}}));
    }
    let datos: Value = serde_json::from_slice(&std::fs::read(&ruta).map_err(|e| e.to_string())?)
        .map_err(|_| "Control de histórico inválido.".to_string())?;
    if datos["tipo"] != "HISTORICO_PROTECCION" || datos["version"] != 1 || !datos["capturas"].is_object() {
        return Err("Control de histórico inválido.".into());
    }
    Ok(datos)
}

fn escribir(ruta: &Path, valor: &Value) -> Result<(), String> {
    let temporal = ruta.with_extension("tmp");
    std::fs::write(&temporal, serde_json::to_vec_pretty(valor).map_err(|e| e.to_string())?)
        .and_then(|_| std::fs::rename(&temporal, ruta))
        .map_err(|e| format!("No se pudo guardar el control de histórico: {e}"))
}

/// `guardar_entrada`: la última captura de la persona manda; un ID remoto en conflicto solo respalda una incidencia.
pub fn guardar_entrada(carpeta: &Path, entrada: &mut Value) -> Result<(), String> {
    let mut datos = estado(carpeta)?;
    let clave = format!("{}|{}", entrada["tribunal"].as_str().unwrap_or(""), entrada["person_id"].as_str().unwrap_or(""));
    let anterior = datos["capturas"].get(&clave).cloned();
    if let Some(a) = anterior.as_ref().filter(|a| a.is_object()) {
        let con_huella = a["rut_hash"].as_str().is_some_and(|h| !h.is_empty())
            && entrada["rut_hash"].as_str().is_some_and(|h| !h.is_empty());
        if con_huella && (a["conflicto_rut"].as_bool().unwrap_or(false) || a["rut_hash"] != entrada["rut_hash"]) {
            entrada["estado"] = json!("FALLIDA");
            entrada["error"] = json!("HISTORICO_IDENTITY");
            entrada["ownership"] = json!("INCIERTA");
            entrada["filas"] = json!([]);
            entrada["rut_hash"] = a["rut_hash"].clone();
            entrada["conflicto_rut"] = json!(true);
        }
    }
    if anterior.as_ref() != Some(entrada) {
        datos["capturas"][&clave] = entrada.clone();
        escribir(&carpeta.join(MANIFIESTO), &datos)?;
    }
    Ok(())
}

/// `cargar`: entradas del manifiesto; un respaldo discrepante degrada solo su entrada.
pub fn cargar(carpeta: &Path, ahora: &str) -> Result<Vec<Value>, String> {
    static NOMBRE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^historico-[a-f0-9]{24}\.html$").unwrap());
    let datos = estado(carpeta)?;
    let mut entradas = Vec::new();
    for entrada in datos["capturas"].as_object().into_iter().flat_map(Map::values) {
        let texto = |k: &str| entrada[k].as_str().is_some_and(|s| !s.is_empty());
        if !entrada.is_object() || !["tribunal", "person_id", "leida", "estado"].iter().all(|k| texto(k)) {
            return Err("Entrada de histórico inválida.".into());
        }
        let valida = (|| -> Option<()> {
            if let Some(nombre) = entrada["archivo"].as_str().filter(|n| !n.is_empty()) {
                let ruta = carpeta.join("historico").join(nombre);
                let enlace = std::fs::symlink_metadata(&ruta).ok()?.file_type().is_symlink();
                if !NOMBRE.is_match(nombre) || enlace || sha256(&std::fs::read(&ruta).ok()?) != entrada["sha256"].as_str()? {
                    return None;
                }
            } else if matches!(entrada["estado"].as_str(), Some("LEIDA" | "VACIA_COMPROBADA")) {
                return None;
            }
            chrono::DateTime::parse_from_rfc3339(&entrada["leida"].as_str()?.replace('Z', "+00:00")).ok()?;
            let filas = entrada["filas"].as_array()?;
            filas
                .iter()
                .all(|f| ["fila", "programa", "fecha_egreso", "causal_egreso"].iter().all(|k| f.get(*k).is_some()))
                .then_some(())
        })();
        let mut e = entrada.clone();
        if valida.is_none() {
            e["estado"] = json!("FALLIDA");
            e["error"] = json!("HISTORICO_RESPALDO_INVALIDO");
            e["ownership"] = json!("INCIERTA");
            e["filas"] = json!([]);
            e["leida"] = json!(ahora);
        }
        if !matches!(e["origen"].as_str(), Some("AUTOMATICA" | "MANUAL")) || e["ownership"] != "VERIFICADA" {
            e["ownership"] = json!("INCIERTA");
        }
        entradas.push(e);
    }
    Ok(entradas)
}

/// `capturar` (captura automática del lote, con `rut_hash`): guarda el respaldo, lee la tabla y registra la entrada.
pub fn capturar(carpeta: &Path, datos: &[u8], persona: &Value, capturada: &str, rut_hash: &str) -> Result<Value, String> {
    if datos.is_empty() || datos.len() > MAX_BYTES {
        return Err("La vista del histórico está vacía o supera 8 MiB.".into());
    }
    for clave in ["tribunal", "person_id"] {
        if persona[clave].as_str().unwrap_or("").trim().is_empty() {
            return Err(format!("Falta {clave} de la persona seleccionada: no se puede asociar el histórico."));
        }
    }
    let huella = sha256(datos);
    let mut entrada = json!({
        "tribunal": persona["tribunal"], "tribunal_nombre": persona["tribunal_nombre"].as_str().unwrap_or(""),
        "person_id": persona["person_id"], "archivo": format!("historico-{}.html", &huella[..24]), "sha256": huella,
        "origen": if rut_hash.is_empty() { "MANUAL" } else { "AUTOMATICA" }, "rut_hash": rut_hash, "leida": capturada,
    });
    let rut = persona["rut"].as_str().unwrap_or("");
    let nombre = persona["nombre"].as_str().unwrap_or("");
    let rut_visible = if rut_hash.is_empty() { "" } else { rut };
    match lecturas::leer_historico(datos, "", capturada, &[rut, nombre], rut_visible) {
        Err(e) => {
            entrada["estado"] = json!("FALLIDA");
            entrada["advertencias"] = json!([mensaje(&e)]);
            entrada["error"] = json!(e);
            entrada["ownership"] = json!("INCIERTA");
            entrada["filas"] = json!([]);
        }
        Ok(lectura) => {
            entrada["estado"] = json!(if lectura["vacio_explicito"] == true { "VACIA_COMPROBADA" } else { "LEIDA" });
            entrada["error"] = json!("");
            entrada["ownership"] = lectura["ownership"].clone();
            entrada["filas"] = json!(lectura["filas"]
                .as_array()
                .map(|f| f
                    .iter()
                    .map(|f| json!({"fila": f["fila"], "programa": f["Programa"], "fecha_egreso": f["Fecha egreso"],
                        "causal_egreso": f["Causal egreso"]}))
                    .collect::<Vec<_>>())
                .unwrap_or_default());
            entrada["advertencias"] =
                json!(lectura["advertencias"].as_array().map(|a| a.iter().map(|x| x["codigo"].clone()).collect::<Vec<_>>()));
        }
    }
    let ruta = carpeta.join("historico").join(entrada["archivo"].as_str().unwrap());
    std::fs::create_dir_all(ruta.parent().unwrap()).map_err(|e| format!("No se pudo crear la carpeta del histórico: {e}"))?;
    if !ruta.exists() {
        super::lote::guardar_exclusivo(&ruta, datos)?;
    }
    guardar_entrada(carpeta, &mut entrada)?;
    Ok(entrada)
}

// ---------------------------------------------------------------- estructura anónima (`estructura_ventana`)

const ROTULOS: [&str; 33] = [
    "Historial Menores", "Despliegue de Pop Up", "Ingreso Historico Protección", "Ingreso Historico(LRPA)",
    "Ingreso Histórico", "Protección", "Escritos", "Doc.", "Tipo Escrito", "Solicitante", "Estado", "Nombre Proyecto",
    "Programa", "Fecha Ingreso", "Fecha Egreso", "Causal Egreso", "Causal Ingreso", "Tribunal", "RUC", "RIT", "RUT",
    "RUN", "Sexo", "Nombre del niño", "Nombre del menor", "Folio", "Etapa", "Trámite", "Referencia", "Código",
    "Entidad", "Proyecto", "Quién Egresa",
];
const FUNCIONES: [&str; 4] = ["ShowHistoriaRut", "ShowHistorialFichas", "verEscritos", "verDoc"];
const RUTAS: [&str; 3] =
    ["/SITFAWEB/IrPopUpPublicoAccion.do", "/SITFAWEB/EscritosAccion.do", "/SITFAWEB/jsp/Tramitacion/PopUp/PopUpHistorial.jsp"];
const PARAMETROS: [&str; 6] = ["id", "clave", "crr_causa", "tipo_popUp", "RUT_Litigante", "RUT_DV"];

/// `lxml text_content()`: todo el texto descendiente, incluido el de scripts.
fn texto_completo(e: ElementRef<'_>) -> String {
    e.text().collect::<String>()
}

/// `_rotulo_seguro`: el literal del catálogo cerrado o nada.
fn rotulo_seguro(texto: &str) -> Option<&'static str> {
    // Como Python: se recorta el texto buscado (`strip(': ')`), no las claves del catálogo (`normalize(r)`).
    let t = lecturas::normalizar(texto);
    let t = t.trim_matches(|c| c == ':' || c == ' ');
    ROTULOS.iter().find(|r| lecturas::normalizar(r) == t).copied()
}

/// `_ruta_segura`: la ruta solo si es una del catálogo; si no, «otra».
fn ruta_segura(valor: Option<&str>) -> String {
    let valor = valor.unwrap_or("");
    let sin_fragmento = valor.split(['#', '?']).next().unwrap_or("");
    let ruta = match sin_fragmento.split_once("://") {
        Some((_, resto)) => resto.find('/').map_or("", |i| &resto[i..]),
        None => sin_fragmento,
    };
    if ruta.is_empty() || RUTAS.contains(&ruta) { ruta.to_string() } else { "otra".into() }
}

/// `_catalogo`: los permitidos presentes, ordenados, más «otro» si había alguno fuera del catálogo.
fn catalogo<'a>(valores: impl IntoIterator<Item = &'a str>, permitidos: &[&str]) -> Vec<String> {
    let valores: BTreeSet<&str> = valores.into_iter().collect();
    let mut salida: Vec<String> = valores.iter().filter(|v| permitidos.contains(v)).map(|v| v.to_string()).collect();
    if valores.iter().any(|v| !permitidos.contains(v)) {
        salida.push("otro".into());
    }
    salida
}

fn filas_directas(tabla: ElementRef<'_>) -> Vec<ElementRef<'_>> {
    tabla
        .children()
        .filter_map(ElementRef::wrap)
        .flat_map(|h| match h.value().name() {
            "tr" => vec![h],
            "thead" | "tbody" => h.children().filter_map(ElementRef::wrap).filter(|f| f.value().name() == "tr").collect(),
            _ => vec![],
        })
        .collect()
}

fn celdas(fila: ElementRef<'_>) -> Vec<ElementRef<'_>> {
    fila.children().filter_map(ElementRef::wrap).filter(|c| matches!(c.value().name(), "td" | "th")).collect()
}

/// `estructura_ventana`: forma ANÓNIMA de una ventana SITFA cuyo lector no encontró la tabla esperada.
pub fn estructura_ventana(datos: &[u8]) -> Result<Value, String> {
    let doc = Html::parse_document(&lecturas::decodificar(datos)?);
    let titulo = rotulo_seguro(&doc.select(&sel("title")).map(texto_completo).collect::<Vec<_>>().join(" "));
    let marcos: Vec<Value> = doc
        .select(&sel("iframe, frame"))
        .map(|e| json!({"etiqueta": e.value().name(), "ruta": ruta_segura(e.value().attr("src"))}))
        .take(10)
        .collect();
    let formularios: Vec<Value> = doc
        .select(&sel("form"))
        .map(|f| {
            let metodo = f.value().attr("method").unwrap_or("").to_lowercase();
            let campos: Vec<&str> = f.select(&sel("input, select")).filter_map(|c| c.value().attr("name")).collect();
            json!({"accion": ruta_segura(f.value().attr("action")),
                "metodo": if matches!(metodo.as_str(), "get" | "post" | "") { metodo } else { "otro".into() },
                "campos": catalogo(campos, &PARAMETROS)})
        })
        .take(10)
        .collect();
    let tablas: Vec<Value> = doc
        .select(&sel("table"))
        .map(|t| {
            let filas = filas_directas(t);
            let mut encabezados: Vec<Vec<&str>> = Vec::new();
            for fila in filas.iter().take(4) {
                let c = celdas(*fila);
                let textos: Vec<Option<&str>> = c.iter().map(|x| rotulo_seguro(&texto_completo(*x))).collect();
                let tiene_th = c.iter().any(|x| x.value().name() == "th");
                if tiene_th || (!c.is_empty() && textos.iter().all(Option::is_some)) {
                    encabezados.push(textos.iter().map(|t| t.unwrap_or("·")).collect());
                }
            }
            let anidada = t.ancestors().filter_map(ElementRef::wrap).any(|a| a.value().name() == "table");
            json!({"filas": filas.len(), "columnas_max": filas.iter().map(|f| celdas(*f).len()).max().unwrap_or(0),
                "anidada": anidada, "filas_encabezado": encabezados.into_iter().take(3).collect::<Vec<_>>()})
        })
        .take(40)
        .collect();
    let secciones: Vec<&str> = doc
        .select(&sel("td, th, div, span, b, font, h1, h2, h3, p, caption"))
        .filter_map(|e| rotulo_seguro(&texto_completo(e)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(30)
        .collect();
    let codigo = doc.select(&sel("script")).map(texto_completo).collect::<Vec<_>>().join(" ");
    static FUNCION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"function\s+([A-Za-z_]\w*)").unwrap());
    static RUTA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/[A-Za-z0-9_/]+\.(?:do|jsp)").unwrap());
    let funciones: Vec<&str> = FUNCION.captures_iter(&codigo).map(|c| c.get(1).unwrap().as_str()).collect();
    let rutas: Vec<&str> = RUTA.find_iter(&codigo).map(|m| m.as_str()).collect();
    Ok(json!({
        "tipo": "ESTRUCTURA_VENTANA_SITFA", "version": 2, "bytes": datos.len(), "titulo": titulo, "marcos": marcos,
        "formularios": formularios, "tablas": tablas, "secciones": secciones,
        "funciones": catalogo(funciones, &FUNCIONES), "rutas_en_scripts": catalogo(rutas, &RUTAS),
        "ajax": {"jquery": codigo.matches("$.ajax").count(), "xhr": codigo.matches("XMLHttpRequest").count(),
            "fetch": codigo.matches("fetch(").count()},
        "contiene_identificadores_personales": false,
    }))
}

/// `_estructura_historico`: deja `historico/estructura_historico.json` la primera vez (sin pisar uno previo).
pub fn guardar_estructura(carpeta: &Path, datos: &[u8]) {
    let destino = carpeta.join("historico").join("estructura_historico.json");
    if destino.exists() {
        return;
    }
    if let Ok(e) = estructura_ventana(datos) {
        if std::fs::create_dir_all(destino.parent().unwrap()).is_ok() {
            let _ = escribir(&destino, &e);
        }
    }
}
