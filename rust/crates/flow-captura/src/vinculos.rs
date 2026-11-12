//! Identidad de los enlaces del listado, sin ejecutar JavaScript ni deducir identificadores.

use crate::{lecturas::texto, sitfa::Seleccion};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

static LEXICO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r#"//[^\n]*|/\*[\s\S]*?(?:\*/|$)|"(?:\\[\s\S]|[^"\\])*(?:"|$)|'(?:\\[\s\S]|[^'\\])*(?:'|$)|`(?:\\[\s\S]|[^`\\])*(?:`|$)"#,
        r#"|/(?:\\[^\n]|\[(?:\\[^\n]|[^\]\\\n])*\]|[^/\\\n])+/[a-z]*"#
    ))
    .unwrap()
});
static RIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A[A-Z]{1,3}-\d{1,7}-\d{4}\z").unwrap());

pub fn normalizar(s: &str) -> String {
    s.nfkd().filter(|c| !is_combining_mark(*c)).collect::<String>().to_uppercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn identidad(rit: &str, rut: &str, nombre: &str, programa: &str) -> [String; 4] {
    [
        normalizar(rit),
        normalizar(rut).chars().filter(|c| *c != '.' && !c.is_whitespace()).collect(),
        normalizar(nombre),
        normalizar(programa),
    ]
}

fn literal(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    if s.bytes().all(|b| b.is_ascii_digit()) && s.len() <= 20 {
        return Some(s.into());
    }
    if s.starts_with('"') {
        return serde_json::from_str(s).ok();
    }
    if !s.starts_with('\'') || !s.ends_with('\'') || s.len() < 2 {
        return None;
    }
    serde_json::from_str(&format!("\"{}\"", s[1..s.len() - 1].replace("\\'", "'").replace('"', "\\\""))).ok()
}

/// Solo llamadas directas con argumentos literales. Descarta comentarios, cadenas, plantillas y regex.
pub fn llamadas(texto: &str, nombre: &str) -> Vec<Vec<String>> {
    if !matches!(nombre, "ShowObservaciones" | "ShowHistoria" | "ShowInformesProgramados") {
        return vec![];
    }
    let codigo = LEXICO.replace_all(texto, |c: &regex::Captures<'_>| " ".repeat(c[0].len()));
    let patron = Regex::new(&format!(r"{}\s*\(", nombre)).unwrap();
    let mut resultado = vec![];
    for m in patron.find_iter(&codigo) {
        if texto[..m.start()].chars().next_back().is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '.')) {
            continue;
        }
        let (mut quote, mut escape, mut inicio) = (None, false, m.end());
        let mut args = vec![];
        for (pos, c) in texto[m.end()..].char_indices() {
            let pos = pos + m.end();
            if let Some(q) = quote {
                if escape {
                    escape = false;
                } else if c == '\\' {
                    escape = true;
                } else if c == q {
                    quote = None;
                }
            } else if matches!(c, '\'' | '"') {
                quote = Some(c);
            } else if matches!(c, ',' | ')') {
                let Some(valor) = literal(texto[inicio..pos].trim()) else {
                    break;
                };
                args.push(valor);
                inicio = pos + c.len_utf8();
                if c == ')' {
                    resultado.push(args);
                    break;
                }
            }
        }
    }
    resultado
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vinculo {
    pub fila: usize,
    pub rit: String,
    pub rut: String,
    pub nombre: String,
    pub programa: String,
    pub tribunal: String,
    pub causa: String,
    pub ingreso: String,
    pub etapa: String,
    pub antiguo: String,
    pub tipo_popup: String,
    pub rit_enlace: String,
    pub calendario: Option<Vec<String>>,
    pub bitacora_verificada: bool,
    pub historia_verificada: bool,
    pub diagnostico: String,
}

impl Vinculo {
    pub fn clave(&self) -> [String; 4] {
        identidad(&self.rit, &self.rut, &self.nombre, &self.programa)
    }
    pub fn params_bitacora(&self, modalidad: &str) -> Result<BTreeMap<String, String>, String> {
        if !self.bitacora_verificada || !matches!(modalidad, "1" | "2" | "3" | "4") {
            return Err("La bitácora no tiene un vínculo comprobado con el listado.".into());
        }
        Ok(BTreeMap::from([
            ("tipo_popUp".into(), self.tipo_popup.clone()),
            ("CRR_IdCausa".into(), self.causa.clone()),
            ("COD_Tribunal".into(), self.tribunal.clone()),
            ("TIP_Consulta".into(), modalidad.into()),
            ("ID_Ingreso".into(), self.ingreso.clone()),
            ("COD_Etapa".into(), self.etapa.clone()),
            ("FLG_MejorNinez".into(), self.antiguo.clone()),
        ]))
    }
}

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("selector constante")
}
fn numerico(s: &str) -> bool {
    !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit())
}
fn celdas(fila: ElementRef<'_>) -> Vec<String> {
    fila.children().filter_map(ElementRef::wrap).filter(|e| matches!(e.value().name(), "td" | "th")).map(texto).collect()
}

/// Conserva todas las filas de personas, incluso sin enlace comprobado; nunca desaparecen del control.
/// Debe llamarse sobre el HTML que ya pasó `validar_respuesta`.
pub fn extraer(datos: &[u8], seleccion: &Seleccion) -> Result<Vec<Vinculo>, String> {
    if seleccion.screen != "seguimiento" {
        return Err("La lectura por ingreso solo corresponde a Seguimiento.".into());
    }
    let tabla_id = match seleccion.tab.as_str() {
        "Espera" => "tablaEspera",
        "Cumplimiento" => "tablaCumplimiento",
        "Informes" => "tablaInforme",
        "Egresados" => "tablaEgresados",
        _ => return Err("Pestaña no permitida.".into()),
    };
    let html = crate::lecturas::decodificar(datos)?;
    let doc = Html::parse_document(&html);
    let tablas = doc.select(&sel(&format!("form[name='InformesPpalForm'] table[id='{tabla_id}']"))).collect::<Vec<_>>();
    if tablas.len() != 1 {
        return Err("No se identifica una tabla única de registros.".into());
    }
    let (mut cabecera, mut resultado) = (None, vec![]);
    for (pos, fila) in tablas[0].select(&sel("tr")).enumerate() {
        if fila.ancestors().filter_map(ElementRef::wrap).find(|e| e.value().name() == "table").map(|e| e.id()) != Some(tablas[0].id()) {
            continue;
        }
        let valores = celdas(fila);
        let normalizados = valores.iter().map(|s| normalizar(s)).collect::<Vec<_>>();
        let unico = |nombres: &[&str]| {
            let encontrados =
                normalizados.iter().enumerate().filter(|(_, s)| nombres.contains(&s.as_str())).map(|(i, _)| i).collect::<Vec<_>>();
            (encontrados.len() == 1).then(|| encontrados[0])
        };
        if normalizados.iter().any(|s| s == "RIT") && normalizados.iter().any(|s| matches!(s.as_str(), "NOMBRE" | "NOMBRE MENOR")) {
            cabecera = Some([
                unico(&["RIT"]),
                unico(&["RUT", "RUT (->RCEI)"]),
                unico(&["NOMBRE", "NOMBRE MENOR"]),
                unico(&["DERIVACION", "NOMBRE CENTRO", "PROGRAMA"]),
            ]);
            continue;
        }
        let Some(cols) = cabecera else {
            continue;
        };
        let valor = |i: usize| cols[i].and_then(|i| valores.get(i)).cloned().unwrap_or_default();
        let rit = valor(0);
        if !RIT.is_match(&normalizar(&rit)) {
            continue;
        }
        let mut v = Vinculo {
            fila: pos + 1,
            rit,
            rut: valor(1),
            nombre: valor(2),
            programa: valor(3),
            tribunal: seleccion.tribunal.clone(),
            causa: String::new(),
            ingreso: String::new(),
            etapa: String::new(),
            antiguo: String::new(),
            tipo_popup: String::new(),
            rit_enlace: String::new(),
            calendario: None,
            bitacora_verificada: false,
            historia_verificada: false,
            diagnostico: "SIN_VINCULO_COMPROBADO".into(),
        };
        if cols.iter().any(Option::is_none) {
            v.diagnostico = "COLUMNAS_IDENTIDAD_INCOMPLETAS".into();
            resultado.push(v);
            continue;
        }
        let comandos = fila
            .select(&sel("[onclick],[href]"))
            .flat_map(|e| [e.value().attr("onclick").unwrap_or(""), e.value().attr("href").unwrap_or("")])
            .collect::<Vec<_>>()
            .join(" ");
        let obs = llamadas(&comandos, "ShowObservaciones");
        let hist = llamadas(&comandos, "ShowHistoria");
        let cal = llamadas(&comandos, "ShowInformesProgramados");
        if hist.len() == 1
            && hist[0].len() == 3
            && hist[0][0] == seleccion.tribunal
            && normalizar(&hist[0][1]) == normalizar(&v.rit)
            && numerico(&hist[0][2])
        {
            v.causa = hist[0][2].clone();
            v.rit_enlace = hist[0][1].clone();
            v.historia_verificada = true;
        }
        if obs.len() == 1
            && obs[0].len() == 6
            && obs[0].iter().all(|s| numerico(s))
            && obs[0][1] == seleccion.tribunal
            && matches!(obs[0][3].as_str(), "11" | "12" | "13")
            && (hist.is_empty() || v.historia_verificada && v.causa == obs[0][0])
        {
            v.causa = obs[0][0].clone();
            v.ingreso = obs[0][2].clone();
            v.etapa = obs[0][4].clone();
            v.tipo_popup = obs[0][3].clone();
            v.antiguo = obs[0][5].clone();
            v.bitacora_verificada = true;
            if cal.len() == 1
                && cal[0].len() == 12
                && cal[0][0] == v.causa
                && cal[0][2] == v.tribunal
                && cal[0][10] == v.ingreso
                && numerico(&cal[0][1])
                && numerico(&cal[0][3])
            {
                v.calendario = Some(cal[0].clone());
            }
        }
        v.diagnostico = if v.bitacora_verificada || v.historia_verificada {
            "VERIFICADA"
        } else if obs.is_empty() {
            "SIN_ACCION_OBSERVACIONES"
        } else if obs.len() > 1 {
            "MULTIPLES_ACCIONES_OBSERVACIONES"
        } else {
            "ACCION_OBSERVACIONES_NO_VERIFICADA"
        }
        .into();
        resultado.push(v);
    }
    descartar_ambiguos(&mut resultado);
    Ok(resultado)
}

/// Se aplica también al conjunto de páginas: un RIT con dos causas o una causa con dos RIT no acredita historia.
pub fn descartar_ambiguos(filas: &mut [Vinculo]) {
    let (mut causas, mut rits): (BTreeMap<_, BTreeSet<_>>, BTreeMap<_, BTreeSet<_>>) = (BTreeMap::new(), BTreeMap::new());
    let mut ingresos: BTreeMap<(String, String), BTreeSet<[String; 4]>> = BTreeMap::new();
    for v in filas.iter().filter(|v| v.historia_verificada || v.bitacora_verificada) {
        causas.entry((v.tribunal.clone(), normalizar(&v.rit))).or_default().insert(v.causa.clone());
        rits.entry((v.tribunal.clone(), v.causa.clone())).or_default().insert(normalizar(&v.rit));
        if v.bitacora_verificada {
            ingresos.entry((v.tribunal.clone(), v.ingreso.clone())).or_default().insert(v.clave());
        }
    }
    for v in filas {
        if causas.get(&(v.tribunal.clone(), normalizar(&v.rit))).is_some_and(|s| s.len() > 1)
            || rits.get(&(v.tribunal.clone(), v.causa.clone())).is_some_and(|s| s.len() > 1)
        {
            v.historia_verificada = false;
            v.diagnostico = "HISTORIA_IDENTIDAD_INCIERTA".into();
        }
        if ingresos.get(&(v.tribunal.clone(), v.ingreso.clone())).is_some_and(|s| s.len() > 1) {
            v.bitacora_verificada = false;
            v.calendario = None;
            v.diagnostico = "INGRESO_IDENTIDAD_INCIERTA".into();
        }
    }
}

fn recibir(conexion: &dyn crate::lotes::Conexion, comando: &str, payload: serde_json::Value) -> Result<(Vec<u8>, String), String> {
    crate::lotes::comprobar_capacidades(&conexion.llamar("capabilities", serde_json::json!({}), std::time::Duration::from_secs(10))?)?;
    let respuesta = conexion.llamar(comando, payload.clone(), std::time::Duration::from_secs(70))?;
    let campo = if comando == "calendar_open" { "args" } else { "params" };
    if respuesta[campo] != payload[campo] {
        return Err("La extensión devolvió otra identidad de lectura.".into());
    }
    let datos = crate::lotes::cuerpo(&respuesta, false)?;
    if respuesta["digest"].as_str() != Some(crate::lotes::huella(&datos).as_str()) {
        return Err("La ventana de lectura no coincide con su huella.".into());
    }
    let capturada = respuesta["startedAt"]
        .as_str()
        .filter(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
        .ok_or("La lectura no tiene una fecha de captura comprobable.")?;
    Ok((datos, capturada.to_string()))
}

pub fn abrir_bitacora(conexion: &dyn crate::lotes::Conexion, v: &Vinculo, modalidad: &str) -> Result<serde_json::Value, String> {
    let params = v.params_bitacora(modalidad)?;
    let (datos, capturada) = recibir(conexion, "diary_open", serde_json::json!({"params":params}))?;
    crate::lecturas::leer_bitacora(&datos, &params, "", &capturada)
}

pub fn abrir_historia(conexion: &dyn crate::lotes::Conexion, v: &Vinculo, tribunal_nombre: &str) -> Result<serde_json::Value, String> {
    if !v.historia_verificada {
        return Err("La historia no tiene un vínculo comprobado con el listado.".into());
    }
    let (datos, capturada) =
        recibir(conexion, "history_open", serde_json::json!({"params":{"tribunal":v.tribunal,"rit":v.rit_enlace,"causa":v.causa}}))?;
    let historia = crate::lecturas::leer_historia(&datos, "", &capturada)?;
    if historia["encabezado"]["RIT"].as_str().map(normalizar) != Some(normalizar(&v.rit)) {
        return Err("El RIT visible de la historia no coincide con la causa solicitada.".into());
    }
    if tribunal_nombre.trim().is_empty() || historia["encabezado"]["Tribunal"].as_str().map(normalizar) != Some(normalizar(tribunal_nombre))
    {
        return Err("El tribunal visible de la historia no coincide con el tribunal solicitado.".into());
    }
    Ok(historia)
}

pub fn abrir_calendario(conexion: &dyn crate::lotes::Conexion, v: &Vinculo) -> Result<serde_json::Value, String> {
    if !v.bitacora_verificada {
        return Err("El ingreso no tiene un vínculo comprobado con el listado.".into());
    }
    let args = v.calendario.as_ref().ok_or("No consta un enlace de calendario comprobado para el ingreso.")?;
    let (datos, capturada) = recibir(conexion, "calendar_open", serde_json::json!({"args":args}))?;
    crate::lecturas::leer_calendario(&datos, args, "", &capturada)
}
