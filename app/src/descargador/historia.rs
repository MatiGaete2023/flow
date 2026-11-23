//! E-Q4c · Historia de la causa en la misma pasada del listado: `descargador/lectura_historia.py::LectorHistoria`.
//! Una lectura por causa (`history_open`, solo con la extensión Integral verificada), cotejo del RIT y del tribunal
//! visibles, ventana de 1 a 4 meses, estado de la tabla Escritos y `historia/historias.json` con el formato de Python.
//! Un error de la historia nunca detiene las bitácoras. Pendiente en E-Q4d: el Excel `Historia_SITFA.xlsx`.

use super::lecturas;
use super::lote::{self, Conexion};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

pub const MANIFIESTO: &str = "historias.json";
pub const MAX_HISTORIAS_POR_LOTE: usize = 5000;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const LEIDAS: [&str; 2] = ["LEIDA", "VACIA_COMPROBADA"];
/// `CODIGOS_ERROR`: códigos que se conservan; cualquier otro error queda como `HISTORIA_LECTURA_ERROR`.
const CODIGOS_ERROR: [&str; 16] = [
    "HISTORY_CONTEXT", "HISTORY_LINK", "HISTORY_IDENTITY", "HISTORY_RESPONSE", "HISTORY_REFLECTED", "HISTORY_TIMEOUT",
    "HISTORY_NETWORK", "HISTORIA_IDENTIDAD_VISIBLE", "HISTORIA_TRIBUNAL_VISIBLE", "HISTORIA_RESPUESTA_INCOMPLETA",
    "HISTORIA_SESION", "HISTORIA_TABLA_NO_RECONOCIDA", "HISTORIA_ENCABEZADO_DISTINTO", "HISTORIA_TABLA_AMBIGUA",
    "HISTORIA_VACIO_NO_COMPROBADO", "HISTORIA_PAGINACION",
];

fn sha256(datos: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(datos))
}

/// `motor.normalized`: sin tildes, mayúsculas y espacios simples.
fn normalizado(s: &str) -> String {
    super::vinculos::normalizar(&s.replace('\u{a0}', " "))
}

/// `codigo_error`: el código si es uno de los conocidos; si no, el genérico.
fn codigo_error(error: &str, generico: &str) -> String {
    if CODIGOS_ERROR.contains(&error) { error.to_string() } else { generico.to_string() }
}

/// `estado_escritos`: estado de la tabla Escritos en la MISMA respuesta de la causa.
pub fn estado_escritos(datos: &[u8]) -> Value {
    match lecturas::leer_escritos_ventana(datos, "", "") {
        Err(e) => {
            let estado = match e.as_str() {
                "ESCRITOS_NO_VERIFICADA" => "SIN_TABLA",
                "ESCRITOS_AUSENTE" => "AUSENTE",
                _ => "ILEGIBLE",
            };
            json!({"estado": estado, "filas": 0, "pendientes": 0})
        }
        Ok(lectura) => {
            let filas = lectura["filas"].as_array().cloned().unwrap_or_default();
            let pendientes = filas.iter().filter(|f| lecturas::normalizar(f["Estado"].as_str().unwrap_or("")).trim() == "pendiente").count();
            json!({"estado": if filas.is_empty() { "VACIA" } else { "CON_FILAS" }, "filas": filas.len(), "pendientes": pendientes})
        }
    }
}

/// `resumen_escritos`: conteo anónimo por estado de Escritos de las causas leídas y total de pendientes.
fn resumen_escritos(causas: &Map<String, Value>) -> Value {
    let mut salida: BTreeMap<String, Value> = BTreeMap::new();
    let mut pendientes = 0u64;
    for c in causas.values() {
        if LEIDAS.contains(&c["estado"].as_str().unwrap_or("")) {
            let estado = c["escritos"]["estado"].as_str().unwrap_or("NO_REGISTRADO").to_string();
            let n = salida.get(&estado).and_then(Value::as_u64).unwrap_or(0) + 1;
            salida.insert(estado, json!(n));
        }
        pendientes += c["escritos"]["pendientes"].as_u64().unwrap_or(0);
    }
    salida.insert("pendientes".into(), json!(pendientes));
    json!(salida)
}

// ---------------------------------------------------------------- estructura anónima de Escritos

const FUNCIONES: [&str; 4] = ["ShowHistoriaRut", "ShowHistorialFichas", "verEscritos", "verDoc"];
const RUTAS: [&str; 3] =
    ["/SITFAWEB/IrPopUpPublicoAccion.do", "/SITFAWEB/EscritosAccion.do", "/SITFAWEB/jsp/Tramitacion/PopUp/PopUpHistorial.jsp"];
const ATRIBUTOS: [&str; 12] = ["id", "class", "name", "value", "href", "onclick", "src", "type", "title", "style", "target", "alt"];
const PARAMETROS: [&str; 6] = ["id", "clave", "crr_causa", "tipo_popUp", "RUT_Litigante", "RUT_DV"];

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("selector constante")
}

fn catalogo<'a>(valores: impl IntoIterator<Item = &'a str>, permitidos: &[&str]) -> Vec<String> {
    let valores: BTreeSet<&str> = valores.into_iter().collect();
    let mut salida: Vec<String> = valores.iter().filter(|v| permitidos.contains(v)).map(|v| v.to_string()).collect();
    if valores.iter().any(|v| !permitidos.contains(v)) {
        salida.push("otro".into());
    }
    salida
}

/// `tipo_valor`: vacío, numérico, opaco o texto, sin el valor.
fn tipo_valor(valor: &str) -> String {
    static OPACO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9+/=%]{16,}$").unwrap());
    if valor.is_empty() {
        "vacio".into()
    } else if valor.chars().all(|c| c.is_ascii_digit()) {
        format!("numerico({})", valor.chars().count())
    } else if OPACO.is_match(valor) {
        format!("opaco({})", valor.chars().count())
    } else {
        "texto".into()
    }
}

/// `forma_enlace`: etiqueta, atributos del catálogo y forma de `onclick` y `href`.
fn forma_enlace(nodo: ElementRef<'_>) -> Value {
    static FUNCION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)^\s*([A-Za-z_][\w.]*)\s*\((.*)\)").unwrap());
    static ESQUEMA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z]+:").unwrap());
    static PARAMETRO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&]([^=&#]+)=([^&#]*)").unwrap());
    let mut salida = json!({"etiqueta": nodo.value().name(),
        "atributos": catalogo(nodo.value().attrs().map(|(k, _)| k), &ATRIBUTOS)});
    let onclick = nodo.value().attr("onclick").unwrap_or("");
    if let Some(c) = FUNCION.captures(onclick) {
        let args = c.get(2).unwrap().as_str();
        let argumentos: Vec<String> = if args.trim().is_empty() {
            vec![]
        } else {
            args.split(',').map(|a| tipo_valor(a.trim().trim_matches(|c| c == '\'' || c == '"'))).collect()
        };
        let funcion = c.get(1).unwrap().as_str();
        salida["onclick"] =
            json!({"funcion": if FUNCIONES.contains(&funcion) { funcion } else { "otra" }, "argumentos": argumentos});
    }
    if let Some(href) = nodo.value().attr("href") {
        let mut esquema = if ESQUEMA.is_match(href) {
            href.split(':').next().unwrap_or("").to_lowercase()
        } else if href.starts_with('#') {
            "ancla".into()
        } else {
            "relativo".into()
        };
        if !["https", "http", "javascript", "data", "ancla", "relativo"].contains(&esquema.as_str()) {
            esquema = "otro".into();
        }
        let parametros: Vec<Value> = PARAMETRO
            .captures_iter(href)
            .map(|c| {
                let n = c.get(1).unwrap().as_str();
                json!({"nombre": if PARAMETROS.contains(&n) { n } else { "otro" }, "tipo": tipo_valor(c.get(2).unwrap().as_str())})
            })
            .collect();
        salida["href"] = json!({"esquema": esquema, "parametros": parametros});
    }
    salida
}

/// `estructura_pestana`: forma ANÓNIMA de la pestaña Escritos (sin datos de la causa).
pub fn estructura_pestana(datos: &[u8]) -> Result<Value, String> {
    let doc = Html::parse_document(&lecturas::decodificar(datos)?);
    let texto = |e: ElementRef<'_>| e.text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
    let controles: Vec<Value> = doc
        .select(&sel("a, li, button, span, td, th, input, div"))
        .filter(|e| {
            lecturas::normalizar(&texto(*e)) == "escritos"
                || lecturas::normalizar(e.value().attr("value").unwrap_or("")) == "escritos"
        })
        .take(10)
        .map(forma_enlace)
        .collect();
    let codigo = doc.select(&sel("script")).map(|e| e.text().collect::<String>()).collect::<Vec<_>>().join(" ");
    static FUNCION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"function\s+([A-Za-z_]\w*)").unwrap());
    static RUTA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/[A-Za-z0-9_/]+\.(?:do|jsp)").unwrap());
    static PARAMETRO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&]([A-Za-z_]\w{0,40})=").unwrap());
    let funciones: BTreeSet<&str> = FUNCION
        .captures_iter(&codigo)
        .map(|c| c.get(1).unwrap().as_str())
        .filter(|f| f.to_lowercase().contains("escrit"))
        .collect();
    let rutas: BTreeSet<&str> = RUTA.find_iter(&codigo).map(|m| m.as_str()).collect();
    let parametros: BTreeSet<&str> = PARAMETRO.captures_iter(&codigo).map(|c| c.get(1).unwrap().as_str()).collect();
    Ok(json!({
        "tipo": "ESTRUCTURA_PESTANA_ESCRITOS", "version": 2, "controles": controles,
        "funciones_escritos": catalogo(funciones, &FUNCIONES), "rutas_en_scripts": catalogo(rutas, &RUTAS),
        "nombres_de_parametros": catalogo(parametros, &PARAMETROS),
        "ajax": {"jquery": codigo.matches("$.ajax").count(), "xhr": codigo.matches("XMLHttpRequest").count(),
            "fetch": codigo.matches("fetch(").count()},
        "contiene_identificadores_personales": false,
    }))
}

/// `json.dumps(params)` de Python (separadores «, » y «: », claves en el orden dado).
fn json_objeto_python(pares: &[(&str, &str)]) -> String {
    let partes: Vec<String> = pares
        .iter()
        .map(|(k, v)| format!("{}: {}", serde_json::to_string(k).unwrap(), serde_json::to_string(v).unwrap()))
        .collect();
    format!("{{{}}}", partes.join(", "))
}

/// `LectorHistoria`.
pub struct LectorHistoria {
    pub meses: u32,
    pub pausa: Duration,
    pub anterior: Option<PathBuf>,
    pub estado: Value,
    carpeta: PathBuf,
    anteriores: BTreeMap<String, Value>,
    causas_por_rit: BTreeMap<(String, String), BTreeSet<String>>,
    rits_por_causa: BTreeMap<String, BTreeSet<String>>,
    fallo_persistencia: bool,
    /// Desfase de la hora local con que Python expresa `leida` (`astimezone()`).
    desfase: chrono::FixedOffset,
}

impl LectorHistoria {
    pub fn nuevo(meses: u32, pausa: Duration, anterior: Option<PathBuf>, desfase: chrono::FixedOffset) -> Result<Self, String> {
        if !(1..=4).contains(&meses) {
            return Err("El período de la historia va de 1 a 4 meses.".into());
        }
        Ok(Self {
            meses,
            pausa,
            anterior,
            estado: Value::Null,
            carpeta: PathBuf::new(),
            anteriores: BTreeMap::new(),
            causas_por_rit: BTreeMap::new(),
            rits_por_causa: BTreeMap::new(),
            fallo_persistencia: false,
            desfase,
        })
    }

    fn persistir(&mut self) {
        let ruta = self.carpeta.join(MANIFIESTO);
        let temporal = ruta.with_extension("tmp");
        let escrito = serde_json::to_vec_pretty(&self.estado)
            .map_err(std::io::Error::other)
            .and_then(|d| std::fs::write(&temporal, d))
            .and_then(|_| std::fs::rename(&temporal, &ruta));
        if escrito.is_err() {
            self.fallo_persistencia = true;
            if self.estado["estado"] != "EN_CURSO" {
                self.estado["estado"] = json!("INCOMPLETA");
            }
            self.estado["persistencia_error"] = json!("HISTORIA_PERSISTENCIA");
        }
    }

    /// `abrir`: carpeta `historia/` del lote, manifiesto inicial y lecturas anteriores íntegras.
    pub fn abrir(&mut self, carpeta_lote: &Path) -> Result<(), String> {
        self.carpeta = carpeta_lote.join("historia");
        self.estado = json!({
            "version": 1, "tipo": "HISTORIA_LECTURA", "estado": "EN_CURSO",
            "criterio_temporal": format!("ventana_{}_meses", self.meses), "meses": self.meses, "causas": {},
            "vinculos": [], "lecturas": 0, "errores_listado": [],
            "descargador": {"version": super::extension::version(super::Edicion::Integral), "commit": "desarrollo"},
            "extension_version": super::extension::version(super::Edicion::Integral),
        });
        std::fs::create_dir_all(&self.carpeta).map_err(|e| format!("No se pudo crear la carpeta de la historia: {e}"))?;
        self.cargar_anterior();
        self.persistir();
        Ok(())
    }

    /// `_cargar_anterior`: solo causas ya leídas cuyo HTML sigue íntegro; FALLIDA/PENDIENTE se vuelven a pedir.
    fn cargar_anterior(&mut self) {
        let Some(ruta) = self.anterior.clone() else { return };
        let Ok(datos) = std::fs::read(&ruta).map_err(|_| ()).and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|_| ())) else {
            return;
        };
        if datos["tipo"] != "HISTORIA_LECTURA" || datos["meses"].as_u64().unwrap_or(4) != u64::from(self.meses) {
            return;
        }
        let padre = ruta.parent().unwrap_or(Path::new("."));
        for (clave, causa) in datos["causas"].as_object().into_iter().flatten() {
            let archivo = causa["archivo"].as_str().unwrap_or("");
            let origen = padre.join(archivo);
            let enlace = std::fs::symlink_metadata(&origen).map(|m| m.file_type().is_symlink()).unwrap_or(true);
            let integra = LEIDAS.contains(&causa["estado"].as_str().unwrap_or(""))
                && !archivo.is_empty()
                && !archivo.contains(['/', '\\'])
                && !enlace
                && std::fs::read(&origen).is_ok_and(|b| b.len() <= MAX_BYTES && Some(sha256(&b).as_str()) == causa["sha256"].as_str());
            if integra {
                let destino = self.carpeta.join(archivo);
                if !destino.exists() && std::fs::copy(&origen, &destino).is_err() {
                    self.anteriores.clear();
                    return;
                }
                let mut c = causa.clone();
                c["reutilizada"] = json!(true);
                self.anteriores.insert(clave.clone(), c);
            }
        }
    }

    /// `pagina`: los vínculos de historia de una página del listado y la lectura de cada causa nueva.
    #[allow(clippy::too_many_arguments)]
    pub fn pagina(
        &mut self,
        conexion: &dyn Conexion,
        cancelar: &std::sync::atomic::AtomicBool,
        tribunal: &str,
        pestana: &str,
        modalidad: &str,
        tribunal_nombre: &str,
        numero: usize,
        listado: &[u8],
    ) {
        let seleccion = super::vinculos::Seleccion { tribunal: tribunal.into(), screen: "seguimiento".into(), tab: pestana.into() };
        let resultado = (|| -> Result<(), String> {
            let filas = super::vinculos::extraer(listado, &seleccion)?;
            for v in filas.iter().filter(|v| !v.rit_enlace.is_empty()) {
                let fila = v.historia_python();
                let clave = format!("RUS|{tribunal}|{}", fila["causa_id"].as_str().unwrap_or(""));
                let mut vinculo = fila.clone();
                vinculo["pestana"] = json!(pestana);
                vinculo["modalidad"] = json!(modalidad);
                vinculo["clave"] = json!(clave);
                self.estado["vinculos"].as_array_mut().unwrap().push(vinculo);
                if self.estado["causas"].get(&clave).is_none() {
                    if let Some(a) = self.anteriores.get(&clave) {
                        self.estado["causas"][&clave] = a.clone();
                    }
                }
                if self.estado["causas"].get(&clave).is_none() {
                    self.estado["causas"][&clave] = json!({"tribunal": tribunal, "tribunal_nombre": tribunal_nombre,
                        "rit": fila["rit"], "rit_enlace": fila["rit_enlace"], "causa_id": fila["causa_id"], "estado": "PENDIENTE"});
                }
                let rit = normalizado(fila["rit"].as_str().unwrap_or(""));
                let rit_clave = (tribunal.to_string(), rit.clone());
                self.causas_por_rit.entry(rit_clave.clone()).or_default().insert(clave.clone());
                self.rits_por_causa.entry(clave.clone()).or_default().insert(rit.clone());
                let causa_rit = normalizado(self.estado["causas"][&clave]["rit"].as_str().unwrap_or(""));
                let conflicto = fila["identidad"] != "VERIFICADA"
                    || self.causas_por_rit[&rit_clave].len() > 1
                    || self.rits_por_causa[&clave].len() > 1
                    || causa_rit != rit;
                if conflicto {
                    let mut afectadas = self.causas_por_rit[&rit_clave].clone();
                    afectadas.insert(clave.clone());
                    for otra in &afectadas {
                        let incierta = self.estado["causas"][otra].as_object_mut().unwrap();
                        for campo in ["en_ventana", "anteriores", "incompletas", "advertencias", "desde", "hasta"] {
                            incierta.remove(campo);
                        }
                        incierta.insert("estado".into(), json!("IDENTIDAD_INCIERTA"));
                        incierta.insert("error".into(), json!("HISTORIA_IDENTIDAD_INCIERTA"));
                    }
                    for vinculo in self.estado["vinculos"].as_array_mut().unwrap() {
                        if afectadas.contains(vinculo["clave"].as_str().unwrap_or("")) {
                            vinculo["identidad"] = json!("INCIERTA");
                        }
                    }
                    continue;
                }
                if self.estado["causas"][&clave]["estado"] == "PENDIENTE"
                    && !cancelar.load(std::sync::atomic::Ordering::SeqCst)
                    && !self.fallo_persistencia
                {
                    self.leer(conexion, cancelar, &clave);
                }
            }
            Ok(())
        })();
        if let Err(e) = resultado {
            let codigo = codigo_error(&e, "HISTORIA_LISTADO_ERROR");
            self.estado["errores_listado"].as_array_mut().unwrap().push(json!({"pagina": numero, "error": codigo}));
        }
        self.persistir();
    }

    /// `_leer`: una ventana de historia, con cotejo de huella, parámetros, RIT y tribunal visibles.
    fn leer(&mut self, conexion: &dyn Conexion, cancelar: &std::sync::atomic::AtomicBool, clave: &str) {
        if self.estado["lecturas"].as_u64().unwrap_or(0) as usize >= MAX_HISTORIAS_POR_LOTE {
            self.estado["causas"][clave]["estado"] = json!("PENDIENTE");
            self.estado["causas"][clave]["error"] = json!("HISTORIA_TOPE");
            self.estado["tope_alcanzado"] = json!(true);
            return;
        }
        self.estado["lecturas"] = json!(self.estado["lecturas"].as_u64().unwrap_or(0) + 1);
        let causa = self.estado["causas"][clave].clone();
        let tribunal = causa["tribunal"].as_str().unwrap_or("").to_string();
        let rit_pedido = causa["rit_enlace"].as_str().filter(|r| !r.is_empty()).or(causa["rit"].as_str()).unwrap_or("").to_string();
        let causa_id = causa["causa_id"].as_str().unwrap_or("").to_string();
        let params = json!({"tribunal": tribunal, "rit": rit_pedido, "causa": causa_id});
        let carpeta = self.carpeta.clone();
        let (meses, desfase) = (self.meses, self.desfase);
        let resultado = (|| -> Result<Map<String, Value>, String> {
            let respuesta = conexion.llamar("history_open", json!({"params": params, "timeout": 60000}), lote::PLAZO)?;
            let datos = lote::cuerpo(&respuesta, false)?;
            let huella = sha256(&datos);
            if datos.len() > MAX_BYTES || respuesta["digest"].as_str() != Some(huella.as_str()) || respuesta["params"] != params {
                return Err("HISTORY_RESPONSE".into());
            }
            let leida = chrono::DateTime::parse_from_rfc3339(&respuesta["startedAt"].as_str().unwrap_or("").replace('Z', "+00:00"))
                .map_err(|_| "HISTORIA_LECTURA_ERROR".to_string())?
                .with_timezone(&desfase);
            let lectura = lecturas::leer_historia(&datos, "", "")?;
            if normalizado(lectura["encabezado"]["RIT"].as_str().unwrap_or("")) != normalizado(causa["rit"].as_str().unwrap_or("")) {
                return Err("HISTORIA_IDENTIDAD_VISIBLE".into());
            }
            let tribunal_visible = lectura["encabezado"]["Tribunal"].as_str().unwrap_or("");
            let nombre_tribunal = causa["tribunal_nombre"].as_str().unwrap_or("");
            if tribunal_visible.is_empty() || nombre_tribunal.is_empty() || normalizado(tribunal_visible) != normalizado(nombre_tribunal) {
                return Err("HISTORIA_TRIBUNAL_VISIBLE".into());
            }
            let base = json_objeto_python(&[("tribunal", &tribunal), ("rit", &rit_pedido), ("causa", &causa_id)]);
            let nombre = format!("historia-{}-{}.html", &sha256(base.as_bytes())[..24], &huella[..16]);
            let ruta = carpeta.join(&nombre);
            if !ruta.exists() {
                lote::guardar_exclusivo(&ruta, &datos)?;
            } else if std::fs::read(&ruta).map_err(|e| e.to_string())? != datos {
                return Err("El archivo local de historia cambió.".into());
            }
            let filas = lectura["filas"].as_array().cloned().unwrap_or_default();
            let hoy = leida.date_naive();
            let [dentro, anteriores, posteriores, sin_fecha] = lecturas::separar_ventana(&filas, hoy, meses)?;
            let escritos = estado_escritos(&datos);
            if escritos["estado"] == "SIN_TABLA" {
                let destino = carpeta.join("estructura_escritos.json");
                if !destino.exists() {
                    if let Ok(e) = estructura_pestana(&datos) {
                        let _ = serde_json::to_vec_pretty(&e).map(|b| std::fs::write(&destino, b));
                    }
                }
            }
            let desde = hoy.checked_sub_months(chrono::Months::new(meses)).unwrap_or(hoy);
            let advertencias: Vec<Value> = lectura["advertencias"].as_array().cloned().unwrap_or_default();
            let mut codigos: Vec<Value> = advertencias.iter().map(|a| a["codigo"].clone()).collect();
            if !posteriores.is_empty() {
                codigos.push(json!("HISTORIA_FECHA_POSTERIOR"));
            }
            let mut m = Map::new();
            m.insert("escritos".into(), escritos);
            for (k, v) in [
                ("estado", json!(if lectura["vacio_explicito"] == true { "VACIA_COMPROBADA" } else { "LEIDA" })),
                ("error", json!("")),
                ("archivo", json!(nombre)),
                ("sha256", json!(huella)),
                ("leida", json!(leida.to_rfc3339())),
                ("desde", json!(desde.format("%Y-%m-%d").to_string())),
                ("hasta", json!(hoy.format("%Y-%m-%d").to_string())),
                ("en_ventana", json!(dentro.len() + posteriores.len() + sin_fecha.len())),
                ("anteriores", json!(anteriores.len())),
                ("incompletas", json!(advertencias.iter().filter(|a| a["codigo"] == "HISTORIA_FILA_INCOMPLETA").count())),
                ("advertencias", json!(codigos)),
            ] {
                m.insert(k.into(), v);
            }
            Ok(m)
        })();
        let destino = self.estado["causas"][clave].as_object_mut().unwrap();
        match resultado {
            Ok(m) => {
                destino.remove("reutilizada");
                destino.extend(m);
            }
            Err(e) => {
                destino.insert("estado".into(), json!("FALLIDA"));
                destino.insert("error".into(), json!(codigo_error(&e, "HISTORIA_LECTURA_ERROR")));
            }
        }
        if !self.pausa.is_zero() {
            let fin = std::time::Instant::now() + self.pausa;
            while std::time::Instant::now() < fin && !cancelar.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep((fin - std::time::Instant::now()).min(Duration::from_millis(20)));
            }
        }
    }

    /// `cerrar`: estado final, conteo y diagnóstico anónimo (el Excel es de E-Q4d). Nunca falla.
    pub fn cerrar(&mut self, lote_estado: &str, cancelada: bool) {
        if self.estado.is_null() || self.estado["estado"] != "EN_CURSO" {
            return;
        }
        let causas = self.estado["causas"].as_object().cloned().unwrap_or_default();
        let mut conteo: BTreeMap<String, u64> = BTreeMap::new();
        for c in causas.values() {
            *conteo.entry(c["estado"].as_str().unwrap_or("").to_string()).or_default() += 1;
        }
        let completa = lote_estado == "COMPLETA"
            && self.estado["errores_listado"].as_array().is_some_and(Vec::is_empty)
            && !self.fallo_persistencia
            && causas.values().all(|c| LEIDAS.contains(&c["estado"].as_str().unwrap_or("")));
        self.estado["estado"] = json!(if cancelada { "CANCELADA" } else if completa { "COMPLETA" } else { "INCOMPLETA" });
        self.estado["conteo"] = json!(conteo);
        self.persistir();
        let mut codigos: BTreeMap<String, u64> = BTreeMap::new();
        for c in causas.values() {
            if let Some(e) = c["error"].as_str().filter(|e| !e.is_empty()) {
                *codigos.entry(e.to_string()).or_default() += 1;
            }
        }
        let diagnostico = json!({
            "estado": self.estado["estado"], "causas_por_estado": conteo, "codigos_error": codigos,
            "errores_listado": self.estado["errores_listado"].as_array().map_or(0, Vec::len),
            "tope_alcanzado": self.estado["tope_alcanzado"] == true, "escritos": resumen_escritos(&causas),
        });
        let ruta = self.carpeta.join("diagnostico_historia.json");
        let escrito = serde_json::to_vec_pretty(&diagnostico)
            .map_err(std::io::Error::other)
            .and_then(|b| std::fs::write(ruta.with_extension("tmp"), b))
            .and_then(|_| std::fs::rename(ruta.with_extension("tmp"), &ruta));
        if escrito.is_err() {
            self.estado["diagnostico_error"] = json!("HISTORIA_PERSISTENCIA");
            self.estado["estado"] = json!("INCOMPLETA");
        }
        self.persistir();
    }
}
