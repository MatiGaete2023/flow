//! Tablas de un HTML sencillo (lo que SITFA guarda con extensión `.xls`). Sin scripts, sin red.

use flow_dominio::Celda;

/// UTF-8 si es válido; si no, Windows-1252 (lo habitual en SITFA).
pub fn decodificar(datos: &[u8]) -> String {
    let datos = datos.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(datos);
    match std::str::from_utf8(datos) {
        Ok(t) => t.to_string(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(datos).0.into_owned(),
    }
}

/// Valor de una entidad (`amp`, `#225`, `#xE1`, `aacute`…).
pub fn entidad(nombre: &str) -> Option<String> {
    if let Some(num) = nombre.strip_prefix('#') {
        let codigo = match num.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => num.parse().ok()?,
        };
        return char::from_u32(codigo).map(String::from);
    }
    ENTIDADES.binary_search_by(|(n, _)| n.cmp(&nombre)).ok().map(|i| ENTIDADES[i].1.to_string())
}

/// Entidades con nombre de HTML 4 (lo que reconoce lxml/libxml2), más `apos`. Ordenadas para búsqueda binaria.
const ENTIDADES: [(&str, char); 253] = [
    ("AElig", '\u{c6}'),
    ("Aacute", '\u{c1}'),
    ("Acirc", '\u{c2}'),
    ("Agrave", '\u{c0}'),
    ("Alpha", '\u{391}'),
    ("Aring", '\u{c5}'),
    ("Atilde", '\u{c3}'),
    ("Auml", '\u{c4}'),
    ("Beta", '\u{392}'),
    ("Ccedil", '\u{c7}'),
    ("Chi", '\u{3a7}'),
    ("Dagger", '\u{2021}'),
    ("Delta", '\u{394}'),
    ("ETH", '\u{d0}'),
    ("Eacute", '\u{c9}'),
    ("Ecirc", '\u{ca}'),
    ("Egrave", '\u{c8}'),
    ("Epsilon", '\u{395}'),
    ("Eta", '\u{397}'),
    ("Euml", '\u{cb}'),
    ("Gamma", '\u{393}'),
    ("Iacute", '\u{cd}'),
    ("Icirc", '\u{ce}'),
    ("Igrave", '\u{cc}'),
    ("Iota", '\u{399}'),
    ("Iuml", '\u{cf}'),
    ("Kappa", '\u{39a}'),
    ("Lambda", '\u{39b}'),
    ("Mu", '\u{39c}'),
    ("Ntilde", '\u{d1}'),
    ("Nu", '\u{39d}'),
    ("OElig", '\u{152}'),
    ("Oacute", '\u{d3}'),
    ("Ocirc", '\u{d4}'),
    ("Ograve", '\u{d2}'),
    ("Omega", '\u{3a9}'),
    ("Omicron", '\u{39f}'),
    ("Oslash", '\u{d8}'),
    ("Otilde", '\u{d5}'),
    ("Ouml", '\u{d6}'),
    ("Phi", '\u{3a6}'),
    ("Pi", '\u{3a0}'),
    ("Prime", '\u{2033}'),
    ("Psi", '\u{3a8}'),
    ("Rho", '\u{3a1}'),
    ("Scaron", '\u{160}'),
    ("Sigma", '\u{3a3}'),
    ("THORN", '\u{de}'),
    ("Tau", '\u{3a4}'),
    ("Theta", '\u{398}'),
    ("Uacute", '\u{da}'),
    ("Ucirc", '\u{db}'),
    ("Ugrave", '\u{d9}'),
    ("Upsilon", '\u{3a5}'),
    ("Uuml", '\u{dc}'),
    ("Xi", '\u{39e}'),
    ("Yacute", '\u{dd}'),
    ("Yuml", '\u{178}'),
    ("Zeta", '\u{396}'),
    ("aacute", '\u{e1}'),
    ("acirc", '\u{e2}'),
    ("acute", '\u{b4}'),
    ("aelig", '\u{e6}'),
    ("agrave", '\u{e0}'),
    ("alefsym", '\u{2135}'),
    ("alpha", '\u{3b1}'),
    ("amp", '\u{26}'),
    ("and", '\u{2227}'),
    ("ang", '\u{2220}'),
    ("apos", '\u{27}'),
    ("aring", '\u{e5}'),
    ("asymp", '\u{2248}'),
    ("atilde", '\u{e3}'),
    ("auml", '\u{e4}'),
    ("bdquo", '\u{201e}'),
    ("beta", '\u{3b2}'),
    ("brvbar", '\u{a6}'),
    ("bull", '\u{2022}'),
    ("cap", '\u{2229}'),
    ("ccedil", '\u{e7}'),
    ("cedil", '\u{b8}'),
    ("cent", '\u{a2}'),
    ("chi", '\u{3c7}'),
    ("circ", '\u{2c6}'),
    ("clubs", '\u{2663}'),
    ("cong", '\u{2245}'),
    ("copy", '\u{a9}'),
    ("crarr", '\u{21b5}'),
    ("cup", '\u{222a}'),
    ("curren", '\u{a4}'),
    ("dArr", '\u{21d3}'),
    ("dagger", '\u{2020}'),
    ("darr", '\u{2193}'),
    ("deg", '\u{b0}'),
    ("delta", '\u{3b4}'),
    ("diams", '\u{2666}'),
    ("divide", '\u{f7}'),
    ("eacute", '\u{e9}'),
    ("ecirc", '\u{ea}'),
    ("egrave", '\u{e8}'),
    ("empty", '\u{2205}'),
    ("emsp", '\u{2003}'),
    ("ensp", '\u{2002}'),
    ("epsilon", '\u{3b5}'),
    ("equiv", '\u{2261}'),
    ("eta", '\u{3b7}'),
    ("eth", '\u{f0}'),
    ("euml", '\u{eb}'),
    ("euro", '\u{20ac}'),
    ("exist", '\u{2203}'),
    ("fnof", '\u{192}'),
    ("forall", '\u{2200}'),
    ("frac12", '\u{bd}'),
    ("frac14", '\u{bc}'),
    ("frac34", '\u{be}'),
    ("frasl", '\u{2044}'),
    ("gamma", '\u{3b3}'),
    ("ge", '\u{2265}'),
    ("gt", '\u{3e}'),
    ("hArr", '\u{21d4}'),
    ("harr", '\u{2194}'),
    ("hearts", '\u{2665}'),
    ("hellip", '\u{2026}'),
    ("iacute", '\u{ed}'),
    ("icirc", '\u{ee}'),
    ("iexcl", '\u{a1}'),
    ("igrave", '\u{ec}'),
    ("image", '\u{2111}'),
    ("infin", '\u{221e}'),
    ("int", '\u{222b}'),
    ("iota", '\u{3b9}'),
    ("iquest", '\u{bf}'),
    ("isin", '\u{2208}'),
    ("iuml", '\u{ef}'),
    ("kappa", '\u{3ba}'),
    ("lArr", '\u{21d0}'),
    ("lambda", '\u{3bb}'),
    ("lang", '\u{2329}'),
    ("laquo", '\u{ab}'),
    ("larr", '\u{2190}'),
    ("lceil", '\u{2308}'),
    ("ldquo", '\u{201c}'),
    ("le", '\u{2264}'),
    ("lfloor", '\u{230a}'),
    ("lowast", '\u{2217}'),
    ("loz", '\u{25ca}'),
    ("lrm", '\u{200e}'),
    ("lsaquo", '\u{2039}'),
    ("lsquo", '\u{2018}'),
    ("lt", '\u{3c}'),
    ("macr", '\u{af}'),
    ("mdash", '\u{2014}'),
    ("micro", '\u{b5}'),
    ("middot", '\u{b7}'),
    ("minus", '\u{2212}'),
    ("mu", '\u{3bc}'),
    ("nabla", '\u{2207}'),
    ("nbsp", '\u{a0}'),
    ("ndash", '\u{2013}'),
    ("ne", '\u{2260}'),
    ("ni", '\u{220b}'),
    ("not", '\u{ac}'),
    ("notin", '\u{2209}'),
    ("nsub", '\u{2284}'),
    ("ntilde", '\u{f1}'),
    ("nu", '\u{3bd}'),
    ("oacute", '\u{f3}'),
    ("ocirc", '\u{f4}'),
    ("oelig", '\u{153}'),
    ("ograve", '\u{f2}'),
    ("oline", '\u{203e}'),
    ("omega", '\u{3c9}'),
    ("omicron", '\u{3bf}'),
    ("oplus", '\u{2295}'),
    ("or", '\u{2228}'),
    ("ordf", '\u{aa}'),
    ("ordm", '\u{ba}'),
    ("oslash", '\u{f8}'),
    ("otilde", '\u{f5}'),
    ("otimes", '\u{2297}'),
    ("ouml", '\u{f6}'),
    ("para", '\u{b6}'),
    ("part", '\u{2202}'),
    ("permil", '\u{2030}'),
    ("perp", '\u{22a5}'),
    ("phi", '\u{3c6}'),
    ("pi", '\u{3c0}'),
    ("piv", '\u{3d6}'),
    ("plusmn", '\u{b1}'),
    ("pound", '\u{a3}'),
    ("prime", '\u{2032}'),
    ("prod", '\u{220f}'),
    ("prop", '\u{221d}'),
    ("psi", '\u{3c8}'),
    ("quot", '\u{22}'),
    ("rArr", '\u{21d2}'),
    ("radic", '\u{221a}'),
    ("rang", '\u{232a}'),
    ("raquo", '\u{bb}'),
    ("rarr", '\u{2192}'),
    ("rceil", '\u{2309}'),
    ("rdquo", '\u{201d}'),
    ("real", '\u{211c}'),
    ("reg", '\u{ae}'),
    ("rfloor", '\u{230b}'),
    ("rho", '\u{3c1}'),
    ("rlm", '\u{200f}'),
    ("rsaquo", '\u{203a}'),
    ("rsquo", '\u{2019}'),
    ("sbquo", '\u{201a}'),
    ("scaron", '\u{161}'),
    ("sdot", '\u{22c5}'),
    ("sect", '\u{a7}'),
    ("shy", '\u{ad}'),
    ("sigma", '\u{3c3}'),
    ("sigmaf", '\u{3c2}'),
    ("sim", '\u{223c}'),
    ("spades", '\u{2660}'),
    ("sub", '\u{2282}'),
    ("sube", '\u{2286}'),
    ("sum", '\u{2211}'),
    ("sup", '\u{2283}'),
    ("sup1", '\u{b9}'),
    ("sup2", '\u{b2}'),
    ("sup3", '\u{b3}'),
    ("supe", '\u{2287}'),
    ("szlig", '\u{df}'),
    ("tau", '\u{3c4}'),
    ("there4", '\u{2234}'),
    ("theta", '\u{3b8}'),
    ("thetasym", '\u{3d1}'),
    ("thinsp", '\u{2009}'),
    ("thorn", '\u{fe}'),
    ("tilde", '\u{2dc}'),
    ("times", '\u{d7}'),
    ("trade", '\u{2122}'),
    ("uArr", '\u{21d1}'),
    ("uacute", '\u{fa}'),
    ("uarr", '\u{2191}'),
    ("ucirc", '\u{fb}'),
    ("ugrave", '\u{f9}'),
    ("uml", '\u{a8}'),
    ("upsih", '\u{3d2}'),
    ("upsilon", '\u{3c5}'),
    ("uuml", '\u{fc}'),
    ("weierp", '\u{2118}'),
    ("xi", '\u{3be}'),
    ("yacute", '\u{fd}'),
    ("yen", '\u{a5}'),
    ("yuml", '\u{ff}'),
    ("zeta", '\u{3b6}'),
    ("zwj", '\u{200d}'),
    ("zwnj", '\u{200c}'),
];

pub fn sin_entidades(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(p) = resto.find('&') {
        salida.push_str(&resto[..p]);
        let cola = &resto[p + 1..];
        match cola.find(';').filter(|f| *f <= 10).and_then(|f| entidad(&cola[..f]).map(|v| (f, v))) {
            Some((f, valor)) => {
                salida.push_str(&valor);
                resto = &cola[f + 1..];
            }
            None => {
                salida.push('&');
                resto = cola;
            }
        }
    }
    salida.push_str(resto);
    salida
}

#[derive(Default)]
struct Tabla {
    orden: usize,
    filas: Vec<Vec<String>>,
    fila: Option<Vec<String>>,
    celda: Option<String>,
}

impl Tabla {
    fn cerrar_celda(&mut self) {
        if let Some(c) = self.celda.take() {
            self.fila.get_or_insert_with(Vec::new).push(c);
        }
    }
    fn cerrar_fila(&mut self) {
        self.cerrar_celda();
        if let Some(f) = self.fila.take() {
            self.filas.push(f);
        }
    }
}

/// Texto de cada celda de cada tabla, en orden de aparición. Una página con formularios o scripts no es un Excel.
pub fn tablas_texto(datos: &[u8]) -> Result<Vec<Vec<Vec<String>>>, String> {
    let texto = decodificar(datos);
    let mut pila: Vec<Tabla> = Vec::new();
    let mut listas: Vec<Tabla> = Vec::new();
    let mut abiertas = 0usize;
    let mut resto = texto.as_str();
    let mut omitir: Option<&str> = None;
    while !resto.is_empty() {
        let Some(p) = resto.find('<') else {
            if let Some(c) = pila.last_mut().and_then(|t| t.celda.as_mut()) {
                c.push_str(&sin_entidades(resto));
            }
            break;
        };
        if omitir.is_none() {
            if let Some(c) = pila.last_mut().and_then(|t| t.celda.as_mut()) {
                c.push_str(&sin_entidades(&resto[..p]));
            }
        }
        let cola = &resto[p..];
        if cola.starts_with("<!--") {
            resto = cola.find("-->").map(|f| &cola[f + 3..]).unwrap_or("");
            continue;
        }
        let Some(fin) = cola.find('>') else { break };
        let etiqueta = &cola[1..fin];
        resto = &cola[fin + 1..];
        let cierre = etiqueta.starts_with('/');
        let nombre: String =
            etiqueta.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
        if let Some(esperada) = omitir {
            if cierre && nombre == esperada {
                omitir = None;
            }
            continue;
        }
        match (nombre.as_str(), cierre) {
            ("form", false) | ("script", false) => return Err("El archivo es una página web activa, no un Excel verificable.".into()),
            ("style", false) => omitir = Some("style"),
            ("table", false) => {
                pila.push(Tabla { orden: abiertas, ..Tabla::default() });
                abiertas += 1;
            }
            ("table", true) => {
                if let Some(mut t) = pila.pop() {
                    t.cerrar_fila();
                    listas.push(t);
                }
            }
            ("tr", false) => {
                if let Some(t) = pila.last_mut() {
                    t.cerrar_fila();
                    t.fila = Some(Vec::new());
                }
            }
            ("tr", true) => {
                if let Some(t) = pila.last_mut() {
                    t.cerrar_fila();
                }
            }
            ("td", false) | ("th", false) => {
                if let Some(t) = pila.last_mut() {
                    t.cerrar_celda();
                    t.celda = Some(String::new());
                }
            }
            ("td", true) | ("th", true) => {
                if let Some(t) = pila.last_mut() {
                    t.cerrar_celda();
                }
            }
            _ => {}
        }
    }
    while let Some(mut t) = pila.pop() {
        t.cerrar_fila();
        listas.push(t);
    }
    listas.sort_by_key(|t| t.orden);
    let total: usize = listas.iter().map(|t| t.filas.iter().map(Vec::len).sum::<usize>()).sum();
    if total > 2_000_000 {
        return Err("La tabla supera los límites de filas y celdas.".into());
    }
    Ok(listas.into_iter().map(|t| t.filas).collect())
}

/// Tablas como celdas: texto recortado; lo vacío queda vacío.
pub fn tablas(datos: &[u8]) -> Result<Vec<Vec<Vec<Celda>>>, String> {
    let convertir = |t: String| {
        let limpio = t.replace('\u{a0}', " ").trim().to_string();
        if limpio.is_empty() {
            Celda::Vacia
        } else {
            Celda::Texto(limpio)
        }
    };
    Ok(tablas_texto(datos)?.into_iter().map(|filas| filas.into_iter().map(|f| f.into_iter().map(convertir).collect()).collect()).collect())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lee_tabla_simple() {
        let html = b"<html><body><TABLE><tr><th>RIT</th><th>NOMBRE</th></tr>\
            <tr><td>X-1-2099</td><td> Sof&iacute;a&nbsp;</td></tr></table></body></html>";
        let t = tablas(html).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0][1][1], Celda::Texto("Sofía".into()));
    }

    #[test]
    fn rechaza_pagina_activa() {
        assert!(tablas(b"<html><form><table><tr><td>a</td></tr></table></form></html>").is_err());
    }

    #[test]
    fn entidades_latinas_como_lxml() {
        // Paridad con lxml: todas las entidades con nombre de HTML 4 (ficticio).
        assert_eq!(
            sin_entidades("Zo&euml; Fran&ccedil;a Jo&atilde;o C&ocirc;te &szlig; &AElig; &yuml; &euro; &mdash;"),
            "Zoë França João Côte ß Æ ÿ € —"
        );
        assert_eq!(sin_entidades("&thetasym; &Yacute; &copy; &frac12; &times;"), "ϑ Ý © ½ ×");
        assert_eq!(sin_entidades("&noexiste; &amp"), "&noexiste; &amp");
        let t = tablas(b"<table><tr><td>No&euml;l</td></tr></table>").unwrap();
        assert_eq!(t[0][0][0], Celda::Texto("Noël".into()));
    }

    #[test]
    fn decodifica_latin1() {
        assert_eq!(decodificar(&[b'a', 0xf1, b'o']), "año");
    }
}
