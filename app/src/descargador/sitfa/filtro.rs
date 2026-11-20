//! Inicializador auditado de `TIP_Informe` (`filtros_respuesta.selected_report` de Flow Python), sin ejecutar JavaScript.

use regex::Regex;
use std::sync::LazyLock;

static DEFINICION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bfunction\s+cargarOpciones\s*\(").unwrap());
// Se conservan las cadenas; comentarios y espacios fuera de ellas no cambian el código reconocido.
static TOKENS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|/\*.*?\*/|//[^\r\n]*|\s+|."#).unwrap()
});
static CONOCIDO: LazyLock<Regex> = LazyLock::new(|| {
    let patron = regex::escape(&compactar(include_str!("selector.js")))
        .replace(
            &regex::escape("'__VALUE__'"),
            r#"(?P<selected>'[0-5]'|"[0-5]")"#,
        )
        .replace(
            &regex::escape("\"__LABEL__\""),
            r#"(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')"#,
        );
    Regex::new(&format!("^{patron}")).unwrap()
});

fn compactar(s: &str) -> String {
    TOKENS
        .find_iter(s)
        .map(|m| m.as_str())
        .filter(|t| !t.starts_with("//") && !t.starts_with("/*") && !t.chars().all(char::is_whitespace))
        .collect()
}

/// Valor literal validado, o `None` cuando la página usa un selector HTML simple.
pub(super) fn seleccionado(scripts: &str, pestana: &str) -> Result<Option<String>, String> {
    let definiciones: Vec<_> = DEFINICION.find_iter(scripts).collect();
    if definiciones.is_empty() {
        return Ok(None);
    }
    if definiciones.len() != 1 {
        return Err("El selector dinámico de vencimiento es ambiguo.".into());
    }
    let compacto = compactar(&scripts[definiciones[0].start()..]);
    let valor = CONOCIDO.captures(&compacto).ok_or(
        "Cambió el selector dinámico de vencimiento; no se pudo comprobar el filtro aplicado.",
    )?;
    let elegido = &valor["selected"][1..2];
    let permitidos: &[&str] = match pestana {
        "tdCumplimiento" => &["0", "4", "5"],
        "tdInforme" => &["1", "2", "3"],
        _ => &["0"],
    };
    if !permitidos.contains(&elegido) {
        return Err(
            "El filtro de vencimiento recibido no corresponde a la pestaña seleccionada.".into(),
        );
    }
    Ok(Some(elegido.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconoce_el_inicializador_auditado_y_rechaza_cambios() {
        let auditado = include_str!("selector.js")
            .replace("__VALUE__", "5")
            .replace("__LABEL__", "Vencidos");
        assert_eq!(
            seleccionado(&auditado, "tdCumplimiento").unwrap().as_deref(),
            Some("5")
        );
        assert!(seleccionado(&auditado, "tdInforme").is_err());
        assert!(seleccionado(&format!("{auditado}\n{auditado}"), "tdCumplimiento").is_err());
        let alterado = auditado.replace("select.appendChild", "select.prepend");
        assert!(seleccionado(&alterado, "tdCumplimiento").is_err());
        assert_eq!(seleccionado("var x = 1;", "tdCumplimiento").unwrap(), None);
    }
}
