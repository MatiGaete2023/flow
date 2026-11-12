//! Valida dimensiones declaradas y coordenadas antes de que el lector de Excel reserve sus matrices.
use crate::formatos::decodificar;
use quick_xml::{events::Event, Reader};

fn celda(s: &str) -> Result<(u64, u64), String> {
    let s = s.trim().replace('$', "");
    let i = s.find(|c: char| c.is_ascii_digit()).ok_or("Coordenada Excel no admitida.")?;
    let col = s[..i]
        .bytes()
        .try_fold(0u64, |n, c| {
            if c.is_ascii_alphabetic() {
                n.checked_mul(26)?.checked_add((c.to_ascii_uppercase() - b'A' + 1) as u64)
            } else {
                None
            }
        })
        .ok_or("Columna Excel fuera de rango.")?;
    let row = s[i..].parse::<u64>().map_err(|_| "Fila Excel fuera de rango.")?;
    if col == 0 || col > 16384 || row == 0 || row > 1_048_576 {
        return Err("Coordenada Excel fuera de rango.".into());
    }
    Ok((row, col))
}
pub fn xml_excel(raw: &[u8]) -> Result<(), String> {
    let xml = decodificar(raw)?;
    let mut reader = Reader::from_str(&xml);
    let (mut root, mut hoja, mut maxfila, mut maxcol, mut elementos) = (true, false, 0u64, 0u64, 0usize);
    loop {
        match reader.read_event().map_err(|_| "XML de Excel inválido.")? {
            Event::Start(e) | Event::Empty(e) => {
                elementos += 1;
                if elementos > 6_000_000 {
                    return Err("XML de Excel excesivo.".into());
                }
                let local = e.local_name();
                if root {
                    hoja = local.as_ref() == b"worksheet";
                    root = false;
                }
                for attr in e.attributes() {
                    let attr = attr.map_err(|_| "Atributo XML de Excel inválido.")?;
                    let key = attr.key.local_name();
                    if matches!(key.as_ref(), b"count" | b"uniqueCount") {
                        let n = attr
                            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                            .map_err(|_| "Conteo de Excel ilegible.")?
                            .parse::<u64>()
                            .map_err(|_| "Conteo de Excel inválido.")?;
                        if n > 2_000_000 {
                            return Err("Un conteo de Excel supera los límites.".into());
                        }
                    }
                    if !hoja {
                        continue;
                    }
                    if local.as_ref() == b"dimension" && key.as_ref() == b"ref" || local.as_ref() == b"c" && key.as_ref() == b"r" {
                        let valor = attr
                            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                            .map_err(|_| "Referencia Excel ilegible.")?;
                        for extremo in valor.split(':') {
                            let (r, c) = celda(extremo)?;
                            maxfila = maxfila.max(r);
                            maxcol = maxcol.max(c);
                        }
                    } else if local.as_ref() == b"row" && key.as_ref() == b"r" {
                        let n = attr
                            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                            .map_err(|_| "Fila Excel ilegible.")?
                            .parse::<u64>()
                            .map_err(|_| "Fila Excel inválida.")?;
                        if n == 0 || n > 1_048_576 {
                            return Err("Fila Excel fuera de rango.".into());
                        }
                        maxfila = maxfila.max(n);
                    }
                }
                if maxfila.saturating_mul(maxcol) > 2_000_000 {
                    return Err("Las dimensiones de Excel superan dos millones de celdas.".into());
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}
