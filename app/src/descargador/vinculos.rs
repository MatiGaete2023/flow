//! Identidad de los enlaces del listado, sin ejecutar JavaScript ni deducir identificadores (E-Q2, desde
//! `flow-captura::vinculos`). Las aperturas de ventana por el puente (`abrir_bitacora`, `abrir_historia`,
//! `abrir_calendario`, E-Q3) solo funcionan con la extensión Integral verificada.

use super::lecturas::texto;
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
/// Como `vinculos.extract` (bitácora) del Descargador: cualquier RIT `[A-Z]+-\d+-\d{4}`.
static RIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A[A-Z]+-\d+-\d{4}\z").unwrap());
/// Como `vinculos.extract_historia`: la historia solo se abre con el formato acotado.
static RIT_HISTORIA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A[A-Z]{1,3}-\d{1,7}-\d{4}\z").unwrap());

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
    // Como `ast.literal_eval` en `vinculos.calls`: comillas simples o dobles con los mismos escapes.
    super::sitfa::cadena_literal(s)
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
    /// `listing_rows`: diagnóstico solo de la bitácora (`vinculo_diagnostico`), sin mezclar la historia.
    pub diagnostico_bitacora: String,
}

impl Vinculo {
    /// La entrada de bitácora con la forma de `vinculos.extract` (`persona_id`/`centro_id` salen del calendario).
    pub fn bitacora_python(&self) -> serde_json::Value {
        let mut v = serde_json::json!({
            "rit": self.rit, "nombre": self.nombre, "rut": self.rut, "programa": self.programa,
            "tribunal_codigo": self.tribunal, "causa_id": self.causa, "ingreso_id": self.ingreso, "etapa": self.etapa,
            "antiguo": self.antiguo, "tipo_popup": self.tipo_popup,
        });
        if let Some(cal) = &self.calendario {
            v["persona_id"] = serde_json::json!(cal.get(1).cloned().unwrap_or_default());
            v["centro_id"] = serde_json::json!(cal.get(3).cloned().unwrap_or_default());
            v["calendario"] = serde_json::json!(cal);
        }
        v
    }

    /// La entrada de historia con la forma de `vinculos.extract_historia`.
    pub fn historia_python(&self) -> serde_json::Value {
        serde_json::json!({
            "rit": self.rit, "nombre": self.nombre, "rut": self.rut, "programa": self.programa,
            "tribunal_codigo": self.tribunal, "causa_id": self.causa, "ingreso_id": self.ingreso,
            "identidad": if self.historia_verificada { "VERIFICADA" } else { "INCIERTA" }, "rit_enlace": self.rit_enlace,
        })
    }

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
/// Vínculos de cada fila del listado, cada uno juzgado solo con su fila (`lectura_bitacoras.listing_rows`).
pub fn extraer_filas(datos: &[u8], seleccion: &Seleccion) -> Result<Vec<Vinculo>, String> {
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
    // El listado ya pasó `validar_respuesta`: se decodifica igual que allí (sin el tope de 8 MiB ni la lista corta de
    // codificaciones de las ventanas de lectura).
    if datos.is_empty() || datos.len() > super::sitfa::LIMITE_BYTES {
        return Err("Respuesta HTML vacía o demasiado grande.".into());
    }
    let doc = Html::parse_document(&super::sitfa::decodificar(datos));
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
            diagnostico_bitacora: "SIN_ACCION_OBSERVACIONES".into(),
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
        // La concordancia de ShowHistoria acredita la causa para la bitácora (como `extract`); la historia además exige
        // el RIT acotado (como `extract_historia`).
        let historia_concordante = hist.len() == 1
            && hist[0].len() == 3
            && hist[0][0] == seleccion.tribunal
            && normalizar(&hist[0][1]) == normalizar(&v.rit)
            && numerico(&hist[0][2]);
        if historia_concordante {
            v.causa = hist[0][2].clone();
            v.rit_enlace = hist[0][1].clone();
            v.historia_verificada = RIT_HISTORIA.is_match(&normalizar(&v.rit));
        }
        if obs.len() == 1
            && obs[0].len() == 6
            && obs[0].iter().all(|s| numerico(s))
            && obs[0][1] == seleccion.tribunal
            && matches!(obs[0][3].as_str(), "11" | "12" | "13")
            && (hist.is_empty() || historia_concordante && v.causa == obs[0][0])
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
        v.diagnostico_bitacora = if v.bitacora_verificada {
            "VERIFICADA"
        } else if obs.is_empty() {
            "SIN_ACCION_OBSERVACIONES"
        } else if obs.len() > 1 {
            "MULTIPLES_ACCIONES_OBSERVACIONES"
        } else {
            "ACCION_OBSERVACIONES_NO_VERIFICADA"
        }
        .into();
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
    Ok(resultado)
}

/// Como `extraer`, más el descarte de causas e ingresos ambiguos entre filas hermanas (lote de listados).
pub fn extraer(datos: &[u8], seleccion: &Seleccion) -> Result<Vec<Vinculo>, String> {
    let mut resultado = extraer_filas(datos, seleccion)?;
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

/// Lectura de una ventana de contexto por el puente (E-Q3): identidad, huella y fecha de captura comprobadas.
fn recibir(conexion: &dyn super::lote::Conexion, comando: &str, payload: serde_json::Value) -> Result<(Vec<u8>, String), String> {
    // La versión de la extensión se comprueba una vez por lote en `lotes::ejecutar`, como el Descargador.
    let respuesta = conexion.llamar(comando, payload.clone(), super::lote::PLAZO)?;
    let campo = if comando == "calendar_open" { "args" } else { "params" };
    if respuesta[campo] != payload[campo] {
        return Err("La extensión devolvió otra identidad de lectura.".into());
    }
    let datos = super::lote::cuerpo(&respuesta, false)?;
    if respuesta["digest"].as_str() != Some(super::xls::huella(&datos).as_str()) {
        return Err("La ventana de lectura no coincide con su huella.".into());
    }
    let capturada = respuesta["startedAt"]
        .as_str()
        .filter(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
        .ok_or("La lectura no tiene una fecha de captura comprobable.")?;
    Ok((datos, capturada.to_string()))
}

pub fn abrir_bitacora(conexion: &dyn super::lote::Conexion, v: &Vinculo, modalidad: &str) -> Result<serde_json::Value, String> {
    let params = v.params_bitacora(modalidad)?;
    let (datos, capturada) = recibir(conexion, "diary_open", serde_json::json!({"params":params}))?;
    super::lecturas::leer_bitacora_en_captura(&datos, &params, &capturada)
}

pub fn abrir_historia(conexion: &dyn super::lote::Conexion, v: &Vinculo, tribunal_nombre: &str) -> Result<serde_json::Value, String> {
    if !v.historia_verificada {
        return Err("La historia no tiene un vínculo comprobado con el listado.".into());
    }
    let (datos, capturada) =
        recibir(conexion, "history_open", serde_json::json!({"params":{"tribunal":v.tribunal,"rit":v.rit_enlace,"causa":v.causa}}))?;
    let historia = super::lecturas::leer_historia(&datos, "", &capturada)?;
    if historia["encabezado"]["RIT"].as_str().map(normalizar) != Some(normalizar(&v.rit)) {
        return Err("El RIT visible de la historia no coincide con la causa solicitada.".into());
    }
    if tribunal_nombre.trim().is_empty() || historia["encabezado"]["Tribunal"].as_str().map(normalizar) != Some(normalizar(tribunal_nombre))
    {
        return Err("El tribunal visible de la historia no coincide con el tribunal solicitado.".into());
    }
    Ok(historia)
}

pub fn abrir_calendario(conexion: &dyn super::lote::Conexion, v: &Vinculo) -> Result<serde_json::Value, String> {
    if !v.bitacora_verificada {
        return Err("El ingreso no tiene un vínculo comprobado con el listado.".into());
    }
    let args = v.calendario.as_ref().ok_or("No consta un enlace de calendario comprobado para el ingreso.")?;
    let (datos, capturada) = recibir(conexion, "calendar_open", serde_json::json!({"args":args}))?;
    super::lecturas::leer_calendario(&datos, args, "", &capturada)
}

/// Selección de la consulta de la que salió el listado (`flow-captura::sitfa::Seleccion`, solo lo que usa `extraer`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seleccion {
    pub tribunal: String,
    pub screen: String,
    pub tab: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn seleccion() -> Seleccion {
        Seleccion { tribunal: "901".into(), screen: "seguimiento".into(), tab: "Espera".into() }
    }

    fn casos() -> Vec<Value> {
        serde_json::from_str(include_str!("../../tests/datos/vinculos.json")).unwrap()
    }

    /// E-Q2: mismas bitácoras e historias que `descargador/vinculos.py` sobre los listados ficticios
    /// (`app/qa/generar_vinculos.py`), con `descartar_ambiguos` aplicado como en el lote.
    #[test]
    fn vinculos_iguales_a_python() {
        let (mut iguales, mut fallos) = (0usize, Vec::new());
        let casos = casos();
        for caso in &casos {
            let mut filas = extraer(caso["html"].as_str().unwrap().as_bytes(), &seleccion()).unwrap();
            descartar_ambiguos(&mut filas);
            let bitacoras: Vec<Value> = filas.iter().filter(|v| v.bitacora_verificada).map(Vinculo::bitacora_python).collect();
            // `extract_historia` solo lista las filas con enlace `ShowHistoria` (rit_enlace).
            let historias: Vec<Value> = filas.iter().filter(|v| !v.rit_enlace.is_empty()).map(Vinculo::historia_python).collect();
            let esp_b = caso["bitacoras"].as_array().unwrap();
            let esp_h = caso["historias"].as_array().unwrap();
            if &bitacoras == esp_b && &historias == esp_h {
                iguales += 1;
            } else {
                fallos.push(format!(
                    "{}:\n  esp bit {esp_b:?}\n  obt bit {bitacoras:?}\n  esp his {esp_h:?}\n  obt his {historias:?}",
                    caso["nombre"]
                ));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("VINCULOS: {iguales}/{}, fallos {}", casos.len(), fallos.len());
        assert_eq!(casos.len(), 9, "el fixture cambió de tamaño: regenerar y revisar");
        assert!(fallos.is_empty(), "{} listados difieren de Flow Python", fallos.len());
    }

    #[test]
    fn parser_de_llamadas_nunca_ejecuta_expresiones_ni_confunde_texto_con_codigo() {
        let accion = "ShowHistoria(901,'X-1-2099',100)";
        for falso in [
            format!("\"{accion}\""),
            format!("`{accion}`"),
            format!("/* {accion} */"),
            format!("// {accion}"),
            format!("if(x) /{accion}/.test(x)"),
            format!("obj.{accion}"),
        ] {
            assert!(llamadas(&falso, "ShowHistoria").is_empty(), "{falso}");
        }
        assert_eq!(llamadas(&format!("/* texto */ {accion}"), "ShowHistoria"), vec![vec!["901", "X-1-2099", "100"]]);
        assert!(llamadas("ShowHistoria(document.value,123)", "ShowHistoria").is_empty());
        assert!(llamadas("ShowHistoria(obtener(),123)", "ShowHistoria").is_empty());
        assert_eq!(
            llamadas("ShowObservaciones(\"texto (con, comas)\",123)", "ShowObservaciones"),
            vec![vec!["texto (con, comas)", "123"]]
        );
    }

    #[test]
    fn conserva_filas_sin_enlace_y_no_confunde_rut_con_codigo_embebido() {
        let caso = casos().remove(0);
        let html = caso["html"].as_str().unwrap().replace("ShowHistoria", "OtraAccion").replace("ShowObservaciones", "OtraAccion");
        let filas = extraer(html.as_bytes(), &seleccion()).unwrap();
        assert_eq!(filas.len(), 1);
        assert!(!filas[0].bitacora_verificada && !filas[0].historia_verificada);
        assert!(filas[0].params_bitacora("2").is_err());
        let html = caso["html"].as_str().unwrap().replace("11111111-0</td>", "11111111-0<script>ajeno()</script></td>");
        assert_eq!(extraer(html.as_bytes(), &seleccion()).unwrap()[0].rut, "11111111-0");
    }

    #[test]
    fn causas_ambiguas_entre_paginas_y_hermanos_no_se_atribuyen_a_un_ingreso() {
        let caso = casos().remove(0);
        let html = caso["html"].as_str().unwrap();
        let mut filas = extraer(html.as_bytes(), &seleccion()).unwrap();
        filas.extend(extraer(html.replace("'100'", "'101'").as_bytes(), &seleccion()).unwrap());
        descartar_ambiguos(&mut filas);
        assert!(filas.iter().all(|v| !v.historia_verificada));
        let mut filas = extraer(html.as_bytes(), &seleccion()).unwrap();
        let mut hermano = filas[0].clone();
        hermano.nombre = "Otra Persona Ficticia".into();
        hermano.rut = "22222222-0".into();
        filas.push(hermano);
        descartar_ambiguos(&mut filas);
        assert!(filas.iter().all(|v| !v.bitacora_verificada && v.calendario.is_none()));
        assert!(filas.iter().all(|v| v.historia_verificada));
    }

    #[test]
    fn identidad_usa_los_cuatro_campos_y_rit_largo_y_charset_como_el_descargador() {
        assert_eq!(
            identidad("X-1-2099", "11.111.111-0", "Niña Ficticia", "PRM Ficticio"),
            identidad("x-1-2099", "11111111-0", "NINA FICTICIA", "prm ficticio")
        );
        assert_ne!(identidad("X-1-2099", "1-9", "Persona Uno", "PRM"), identidad("X-1-2099", "1-9", "Persona Dos", "PRM"));
        let caso = casos().remove(0);
        let html = caso["html"].as_str().unwrap().replace("X-1-2099", "ABCD-12345678-2099");
        let filas = extraer(html.as_bytes(), &seleccion()).unwrap();
        assert!(filas.len() == 1 && filas[0].bitacora_verificada && !filas[0].historia_verificada);
        let html = caso["html"].as_str().unwrap().replace("<html>", "<html><head><meta charset=\"us-ascii\"></head>");
        assert!(extraer(html.as_bytes(), &seleccion()).unwrap()[0].bitacora_verificada);
        let html = caso["html"].as_str().unwrap().replace("ShowObservaciones('100'", "ShowObservaciones('\\x3100'");
        assert!(extraer(html.as_bytes(), &seleccion()).unwrap()[0].bitacora_verificada);
    }

    /// E-Q3: una lectura por el puente exige identidad, huella y fecha antes de aceptar datos, y pide una sola vez.
    #[test]
    fn lectura_exige_identidad_huella_y_fecha_antes_de_aceptar_datos() {
        use super::super::lote::Conexion;
        use base64::{engine::general_purpose::STANDARD, Engine};
        use serde_json::json;
        struct Ficticia {
            mutacion: &'static str,
            datos: String,
            lecturas: std::sync::atomic::AtomicUsize,
        }
        impl Conexion for Ficticia {
            fn llamar(&self, comando: &str, payload: Value, _plazo: std::time::Duration) -> Result<Value, String> {
                assert_eq!(comando, "diary_open");
                self.lecturas.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let datos = if self.mutacion == "html_ajeno" {
                    self.datos.replace("name=\"ID_Ingreso\" value=\"201\"", "name=\"ID_Ingreso\" value=\"999\"")
                } else {
                    self.datos.clone()
                };
                let mut r = json!({"status":200,"type":"text/html","body":STANDARD.encode(datos.as_bytes()),
                    "params":payload["params"],"digest":super::super::xls::huella(datos.as_bytes()),"startedAt":"2026-11-12T12:00:00Z"});
                match self.mutacion {
                    "params" => r["params"]["ID_Ingreso"] = json!("999"),
                    "digest" => r["digest"] = json!("alterada"),
                    "startedAt" => r["startedAt"] = json!("ilegible"),
                    _ => (),
                }
                Ok(r)
            }
        }
        let c = casos().remove(0);
        let v = extraer(c["html"].as_str().unwrap().as_bytes(), &seleccion()).unwrap().remove(0);
        let lecturas: Vec<Value> = serde_json::from_str(include_str!("../../tests/datos/lecturas.json")).unwrap();
        let datos = lecturas[0]["html"].as_str().unwrap()
            .replace("name=\"COD_Tribunal\" value=\"11\"", "name=\"COD_Tribunal\" value=\"901\"")
            .replace("name=\"CRR_IdCausa\" value=\"22\"", "name=\"CRR_IdCausa\" value=\"100\"")
            .replace("name=\"ID_Ingreso\" value=\"33\"", "name=\"ID_Ingreso\" value=\"201\"");
        let positiva = Ficticia { mutacion: "ninguna", datos: datos.clone(), lecturas: Default::default() };
        let leida = abrir_bitacora(&positiva, &v, "2").unwrap();
        assert_eq!(leida["coverage"], "PARCIAL");
        assert_eq!(leida["alcance"], "COMPLETA");
        assert!(leida["alcance_revision_fuente"].is_object(), "la lectura por el puente acredita su alcance de revisión");
        for mutacion in ["params", "digest", "startedAt", "html_ajeno"] {
            let f = Ficticia { mutacion, datos: datos.clone(), lecturas: Default::default() };
            assert!(abrir_bitacora(&f, &v, "2").is_err(), "{mutacion}");
            assert_eq!(f.lecturas.load(std::sync::atomic::Ordering::SeqCst), 1);
        }
        let mut sin_historia = v.clone();
        sin_historia.historia_verificada = false;
        assert!(abrir_historia(&positiva, &sin_historia, "Juzgado").is_err());
        let mut sin_calendario = v.clone();
        sin_calendario.calendario = None;
        assert!(abrir_calendario(&positiva, &sin_calendario).is_err());
    }
}
