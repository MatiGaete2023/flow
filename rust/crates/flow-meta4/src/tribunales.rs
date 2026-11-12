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

/// Abreviaturas inequívocas. «LA» (Los Ángeles) no se acepta en una celda: es también un artículo.
const ALIAS: [(&str, &str); 4] = [("THNO", "TALCAHUANO"), ("CCP", "CONCEPCIÓN"), ("LOSA", "LOS ANGELES"), ("LGS", "LOS ANGELES")];

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

/// Tribunal que nombra la celda, solo si es exactamente uno.
pub fn tribunal(celda: &str) -> Option<&'static str> {
    let texto = norma(celda);
    let mut hallados: Vec<&'static str> = Vec::new();
    for (patron, nombre) in PATRONES {
        if contiene_palabra(&texto, patron) && !hallados.contains(&nombre) {
            hallados.push(nombre);
        }
    }
    for palabra in texto.split(' ') {
        if let Some((_, nombre)) = ALIAS.iter().find(|(a, _)| *a == palabra) {
            if !hallados.contains(nombre) {
                hallados.push(nombre);
            }
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

    #[test]
    fn categorias() {
        assert_eq!(categoria("FAE PRO Biobío"), Some("FAE"));
        assert_eq!(categoria("RLP Hogar"), Some("Residencia"));
        assert_eq!(categoria("CREAD Lactantes"), Some("Residencia"));
        assert_eq!(categoria("PRM Uno"), None);
    }
}
