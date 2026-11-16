//! Tribunales de la jurisdicción y su reconocimiento en el texto de una celda
//! (`META4_Bitacoras/scripts/generate_meta4.py`).

pub const TRIBUNALES: [&str; 17] = [
    "ARAUCO",
    "CABRERO",
    "CAÑETE",
    "CONCEPCIÓN",
    "CORONEL",
    "CURANILAHUE",
    "FLORIDA",
    "LOS ANGELES",
    "LAJA",
    "LEBU",
    "MULCHÉN",
    "NACIMIENTO",
    "SANTA BÁRBARA",
    "SANTA JUANA",
    "TALCAHUANO",
    "TOMÉ",
    "YUMBEL",
];

const PATRONES: [(&str, &str); 19] = [
    ("SANTA BARBARA", "SANTA BÁRBARA"),
    ("SANTA JUANA", "SANTA JUANA"),
    ("LOS ANGELES", "LOS ANGELES"),
    ("TALCAHUANO", "TALCAHUANO"),
    ("CONCEPCION", "CONCEPCIÓN"),
    ("CONCEPCIN", "CONCEPCIÓN"),
    ("CURANILAHUE", "CURANILAHUE"),
    ("NACIMIENTO", "NACIMIENTO"),
    ("CABRERO", "CABRERO"),
    ("CANETE", "CAÑETE"),
    ("CAETE", "CAÑETE"),
    ("CORONEL", "CORONEL"),
    ("FLORIDA", "FLORIDA"),
    ("ARAUCO", "ARAUCO"),
    ("MULCHEN", "MULCHÉN"),
    ("LAJA", "LAJA"),
    ("LEBU", "LEBU"),
    ("TOME", "TOMÉ"),
    ("YUMBEL", "YUMBEL"),
];

/// Abreviaturas aceptadas (`TRIBUNAL_ALIASES` de Python). «LA» es ambigua (`AMBIGUOUS_ALIASES`): también es un
/// artículo, y solo cuenta si el contexto lo permite (ver `CONECTORES` y `VECINOS`).
const ALIAS: [(&str, &str); 5] =
    [("THNO", "TALCAHUANO"), ("CCP", "CONCEPCIÓN"), ("LA", "LOS ANGELES"), ("LOSA", "LOS ANGELES"), ("LGS", "LOS ANGELES")];
const AMBIGUOS: &[&str] = &["LA"];

/// `ALIAS_CONNECTORS` de Python: si el alias ambiguo viene precedido por uno de estos términos, es un artículo.
const CONECTORES: &[&str] = &[
    "DE",
    "DEL",
    "EN",
    "A",
    "AL",
    "CON",
    "POR",
    "PARA",
    "DESDE",
    "SOBRE",
    "HASTA",
    "ENTRE",
    "SEGUN",
    "Y",
    "O",
    "U",
    "ANTE",
    "BAJO",
    "TRAS",
    "TODA",
    "TODAS",
    "ESTA",
    "ESA",
    "AQUELLA",
    "SOLO",
    "SOLAMENTE",
];

/// `ALIAS_NEIGHBOURS` de Python: si el alias ambiguo va seguido por uno de estos términos (o por nada, o por un
/// número), es una abreviatura de tribunal.
const VECINOS: &[&str] = &[
    "FAE",
    "FAES",
    "FAS",
    "RES",
    "RESIDENCIA",
    "RESIDENCIAS",
    "CREAD",
    "ESPERA",
    "ESP",
    "CUMPLIMIENTO",
    "CUMPLIMEINTO",
    "CUMPL",
    "CUMP",
    "CUM",
    "INFORME",
    "INFORMES",
    "JF",
    "JUZGADO",
    "FAMILIA",
    "ENERO",
    "FEBRERO",
    "MARZO",
    "ABRIL",
    "MAYO",
    "JUNIO",
    "JULIO",
    "AGOSTO",
    "SEPTIEMBRE",
    "SETIEMBRE",
    "OCTUBRE",
    "NOVIEMBRE",
    "DICIEMBRE",
    "XLS",
    "XLSX",
    "XLSM",
    "PDF",
    "PNG",
    "JPG",
    "JPEG",
];

/// Mayúsculas, sin tildes, y todo lo que no es letra o dígito pasa a un espacio.
pub fn norma(texto: &str) -> String {
    let ascii = flow_dominio::texto::ascii(&texto.replace('\u{fffd}', "")).to_uppercase();
    let mut salida = String::with_capacity(ascii.len());
    for c in ascii.chars() {
        if c.is_ascii_alphanumeric() {
            salida.push(c);
        } else if !salida.ends_with(' ') {
            salida.push(' ');
        }
    }
    salida.trim().to_string()
}

fn contiene_palabra(texto: &str, patron: &str) -> bool {
    let mut desde = 0;
    while let Some(p) = texto[desde..].find(patron) {
        let inicio = desde + p;
        let fin = inicio + patron.len();
        let antes = texto[..inicio].chars().next_back().is_some_and(|c| c.is_ascii_uppercase());
        let despues = texto[fin..].chars().next().is_some_and(|c| c.is_ascii_uppercase());
        if !antes && !despues {
            return true;
        }
        desde = inicio + 1;
    }
    false
}

/// `re.sub(r"\bL\.\s?A\.?", "LA", valor, flags=re.IGNORECASE)` de Python: «L.A.», «L. A.» o «l.a» pasan a «LA».
fn abreviatura_la(texto: &str) -> String {
    let palabra = |c: char| c.is_alphanumeric() || c == '_';
    let espacio = |c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c);
    let c: Vec<char> = texto.chars().collect();
    let mut salida = String::with_capacity(texto.len());
    let mut i = 0;
    while i < c.len() {
        if matches!(c[i], 'L' | 'l') && (i == 0 || !palabra(c[i - 1])) && c.get(i + 1) == Some(&'.') {
            let mut j = i + 2;
            if c.get(j).is_some_and(|x| espacio(*x)) {
                j += 1;
            }
            if c.get(j).is_some_and(|x| matches!(x, 'A' | 'a')) {
                j += 1;
                if c.get(j) == Some(&'.') {
                    j += 1;
                }
                salida.push_str("LA");
                i = j;
                continue;
            }
        }
        salida.push(c[i]);
        i += 1;
    }
    salida
}

/// Tribunal que nombra la celda, solo si es exactamente uno (`tribunal_name` y `_segment_tribunals_text` de Python):
/// nombres completos y alias se acumulan; el alias ambiguo «LA» solo cuenta si no lo precede un conector y lo sigue
/// nada, un número o un término de `VECINOS`.
pub fn tribunal(celda: &str) -> Option<&'static str> {
    let texto = norma(&abreviatura_la(celda));
    if texto.is_empty() {
        return None;
    }
    let mut hallados: Vec<&'static str> = Vec::new();
    for (patron, nombre) in PATRONES {
        if contiene_palabra(&texto, patron) && !hallados.contains(&nombre) {
            hallados.push(nombre);
        }
    }
    let palabras: Vec<&str> = texto.split(' ').collect();
    for (i, palabra) in palabras.iter().enumerate() {
        let Some((_, nombre)) = ALIAS.iter().find(|(a, _)| a == palabra) else {
            continue;
        };
        if AMBIGUOS.contains(palabra) {
            let anterior = i.checked_sub(1).map(|j| palabras[j]);
            if anterior.is_some_and(|a| CONECTORES.contains(&a)) {
                continue;
            }
            let siguiente = palabras.get(i + 1);
            let abreviatura = siguiente.is_none_or(|s| VECINOS.contains(s) || s.chars().all(|c| c.is_ascii_digit()));
            if !abreviatura {
                continue;
            }
        }
        if !hallados.contains(nombre) {
            hallados.push(nombre);
        }
    }
    (hallados.len() == 1).then(|| hallados[0])
}

/// FAE o Residencia según cómo empieza la derivación.
pub fn categoria(derivacion: &str) -> Option<&'static str> {
    let texto = norma(derivacion);
    if texto.starts_with("FAE") {
        Some("FAE")
    } else if texto.starts_with('R') || texto.starts_with("CREAD") {
        Some("Residencia")
    } else {
        None
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn reconoce_un_solo_tribunal() {
        assert_eq!(tribunal("Juzgado de Familia de Tomé"), Some("TOMÉ"));
        assert_eq!(tribunal("Jgdo. L. y G. de Mulchén"), Some("MULCHÉN"));
        assert_eq!(tribunal("JF Los Ángeles"), Some("LOS ANGELES"));
        assert_eq!(tribunal("TOMELLOSO"), None);
        assert_eq!(tribunal("Tomé / Laja"), None);
        assert_eq!(tribunal("Familia CCP"), Some("CONCEPCIÓN"));
        assert_eq!(tribunal(""), None);
    }

    /// Decisión 9 del usuario: «LA» es Los Ángeles exactamente como en Python, también lo que Python no acepta.
    #[test]
    fn alias_la_como_python() {
        assert_eq!(tribunal("JF LA"), Some("LOS ANGELES"));
        assert_eq!(tribunal("L. A."), Some("LOS ANGELES"));
        assert_eq!(tribunal("La"), Some("LOS ANGELES"));
        assert_eq!(tribunal("FAE ESPERA LA"), Some("LOS ANGELES"));
        assert_eq!(tribunal("LA casa"), None);
        assert_eq!(tribunal("Familia de L.A."), None);
        assert_eq!(tribunal("La Florida"), Some("FLORIDA"));
        assert_eq!(tribunal("Laja LA"), None);
    }

    #[test]
    fn categorias() {
        assert_eq!(categoria("FAE PRO Biobío"), Some("FAE"));
        assert_eq!(categoria("RLP Hogar"), Some("Residencia"));
        assert_eq!(categoria("CREAD Lactantes"), Some("Residencia"));
        assert_eq!(categoria("PRM Uno"), None);
    }
}
