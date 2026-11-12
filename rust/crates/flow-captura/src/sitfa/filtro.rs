//! Reconoce el inicializador auditado de TIP_Informe, sin ejecutar JavaScript.

use super::Error;
use regex::Regex;
use std::sync::LazyLock;

static DEFINICION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bfunction\s+cargarOpciones\s*\(").unwrap());
// Se conservan las cadenas; los comentarios y espacios fuera de ellas no cambian el código reconocido.
static TOKENS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|/\*.*?\*/|//[^\r\n]*|\s+|."#).unwrap());
static CONOCIDO: LazyLock<Regex> = LazyLock::new(|| {
    let patron = regex::escape(&compactar(include_str!("selector.js")))
        .replace(&regex::escape("'__VALUE__'"), r#"(?P<selected>'[0-5]'|"[0-5]")"#)
        .replace(&regex::escape("\"__LABEL__\""), r#"(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')"#);
    Regex::new(&format!("^{patron}")).unwrap()
});

fn compactar(s: &str) -> String {
    TOKENS
        .find_iter(s)
        .map(|m| m.as_str())
        .filter(|t| !t.starts_with("//") && !t.starts_with("/*") && !t.chars().all(char::is_whitespace))
        .collect()
}

pub(super) fn seleccionado(scripts: &str, pestaña: &str) -> Result<Option<String>, Error> {
    let definiciones: Vec<_> = DEFINICION.find_iter(scripts).collect();
    if definiciones.is_empty() {
        return Ok(None);
    }
    if definiciones.len() != 1 {
        return Err("El selector dinámico de vencimiento es ambiguo.".into());
    }
    let compacto = compactar(&scripts[definiciones[0].start()..]);
    let valor =
        CONOCIDO.captures(&compacto).ok_or("Cambió el selector dinámico de vencimiento; no se pudo comprobar el filtro aplicado.")?;
    let seleccionado = &valor["selected"][1..2];
    let permitidos: &[&str] = match pestaña {
        "tdCumplimiento" => &["0", "4", "5"],
        "tdInforme" => &["1", "2", "3"],
        _ => &["0"],
    };
    if !permitidos.contains(&seleccionado) {
        return Err("El filtro de vencimiento recibido no corresponde a la pestaña seleccionada.".into());
    }
    Ok(Some(seleccionado.into()))
}
