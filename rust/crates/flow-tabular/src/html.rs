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
    let c = match nombre {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "aacute" => 'á',
        "eacute" => 'é',
        "iacute" => 'í',
        "oacute" => 'ó',
        "uacute" => 'ú',
        "Aacute" => 'Á',
        "Eacute" => 'É',
        "Iacute" => 'Í',
        "Oacute" => 'Ó',
        "Uacute" => 'Ú',
        "ntilde" => 'ñ',
        "Ntilde" => 'Ñ',
        "uuml" => 'ü',
        "Uuml" => 'Ü',
        "iquest" => '¿',
        "iexcl" => '¡',
        "ordm" => 'º',
        "ordf" => 'ª',
        "deg" => '°',
        "laquo" => '«',
        "raquo" => '»',
        _ => return None,
    };
    Some(c.to_string())
}

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
        let nombre: String = etiqueta
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        if let Some(esperada) = omitir {
            if cierre && nombre == esperada {
                omitir = None;
            }
            continue;
        }
        match (nombre.as_str(), cierre) {
            ("form", false) | ("script", false) => {
                return Err("El archivo es una página web activa, no un Excel verificable.".into())
            }
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
    Ok(tablas_texto(datos)?
        .into_iter()
        .map(|filas| filas.into_iter().map(|f| f.into_iter().map(convertir).collect()).collect())
        .collect())
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
    fn decodifica_latin1() {
        assert_eq!(decodificar(&[b'a', 0xf1, b'o']), "año");
    }
}
