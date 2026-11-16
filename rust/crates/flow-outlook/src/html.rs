//! Cuerpo HTML del borrador a partir del texto revisado, sin recursos externos (`nurus/personal/mail_presentation.py`).
//! Lógica pura: se prueba en cualquier sistema; la usa la automatización de Outlook en Windows.

/// Escapa como `html.escape` de Python (también comillas dobles y simples).
pub fn escapar(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len());
    for c in texto.chars() {
        match c {
            '&' => salida.push_str("&amp;"),
            '<' => salida.push_str("&lt;"),
            '>' => salida.push_str("&gt;"),
            '"' => salida.push_str("&quot;"),
            '\'' => salida.push_str("&#x27;"),
            otro => salida.push(otro),
        }
    }
    salida
}

/// `body_html`: un párrafo por bloque separado por una línea en blanco; los saltos simples son `<br>`.
pub fn cuerpo_html(texto: &str) -> String {
    let normal = texto.replace("\r\n", "\n").replace('\r', "\n");
    let parrafos: String = normal
        .split("\n\n")
        .map(|p| format!("<p style=\"margin:0 0 12px;line-height:1.45\">{}</p>", escapar(p).replace('\n', "<br>")))
        .collect();
    format!("<div style=\"font-family:Calibri,Arial,sans-serif;font-size:11pt;color:#202020\">{parrafos}</div>")
}

/// Fin de la etiqueta de apertura `<body …>` (sin distinguir mayúsculas), como `re.search(r'<body\b[^>]*>', …)`.
fn fin_de_body(html: &str) -> Option<usize> {
    let bajo = html.to_ascii_lowercase();
    let mut desde = 0;
    while let Some(i) = bajo[desde..].find("<body") {
        let inicio = desde + i;
        let tras = inicio + "<body".len();
        // `\b`: lo que sigue a «body» no puede ser letra, dígito ni guion bajo.
        let limite = bajo[tras..].chars().next().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if limite {
            if let Some(cierre) = bajo[tras..].find('>') {
                return Some(tras + cierre + 1);
            }
            return None;
        }
        desde = tras;
    }
    None
}

/// `insert_before_signature`: el texto va al comienzo del `<body>` del HTML que Outlook preparó con la firma.
/// Sin `<body>`, va antes de todo. `con_estilo` usa `cuerpo_html`; si no, un `<div>` simple.
pub fn insertar_antes_de_firma(texto: &str, firma: &str, con_estilo: bool) -> String {
    let agregado = if con_estilo { cuerpo_html(texto) } else { format!("<div>{}</div>", escapar(texto).replace('\n', "<br>")) } + "<br>";
    match fin_de_body(firma) {
        Some(fin) => format!("{}{agregado}{}", &firma[..fin], &firma[fin..]),
        None => agregado + firma,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn escapa_como_python() {
        assert_eq!(escapar(r#"<a href="x">Tom & 'Ana'</a>"#), "&lt;a href=&quot;x&quot;&gt;Tom &amp; &#x27;Ana&#x27;&lt;/a&gt;");
        assert_eq!(escapar("ñandú"), "ñandú");
    }

    #[test]
    fn parrafos_y_saltos() {
        let html = cuerpo_html("Buen día:\r\n\r\nLínea 1\nLínea <2>");
        assert_eq!(
            html,
            "<div style=\"font-family:Calibri,Arial,sans-serif;font-size:11pt;color:#202020\">\
             <p style=\"margin:0 0 12px;line-height:1.45\">Buen día:</p>\
             <p style=\"margin:0 0 12px;line-height:1.45\">Línea 1<br>Línea &lt;2&gt;</p></div>"
        );
    }

    #[test]
    fn inserta_al_comienzo_del_body_y_conserva_la_firma() {
        let firma = "<html><head><style>p{}</style></head><BODY lang=ES-CL class=x><p>Firma ficticia</p></BODY></html>";
        let html = insertar_antes_de_firma("Hola", firma, false);
        assert_eq!(
            html,
            "<html><head><style>p{}</style></head><BODY lang=ES-CL class=x><div>Hola</div><br><p>Firma ficticia</p></BODY></html>"
        );
        let con_estilo = insertar_antes_de_firma("Hola", firma, true);
        assert!(con_estilo.contains("class=x><div style=") && con_estilo.ends_with("<br><p>Firma ficticia</p></BODY></html>"));
        // `<bodyx>` no es `<body>`; sin body el texto va antes de todo.
        assert_eq!(insertar_antes_de_firma("A", "<bodyx><body>F", false), "<bodyx><body><div>A</div><br>F");
        assert_eq!(insertar_antes_de_firma("A", "<p>F</p>", false), "<div>A</div><br><p>F</p>");
        assert_eq!(insertar_antes_de_firma("A", "", false), "<div>A</div><br>");
    }
}
