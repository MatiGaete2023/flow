//! Libros Excel de salida: observaciones y nóminas. Siempre archivos nuevos: nunca se sobrescribe.

use flow_dominio::{Celda, Etapa};
use flow_trabajo::{FilaTrabajo, Trabajo};
use rust_xlsxwriter::{Color, ExcelDateTime, Format, FormatAlign, FormatBorder, Workbook, Worksheet, XlsxError};
use std::path::{Path, PathBuf};

use chrono::Datelike;

/// Ruta que no existe todavía: agrega « (2)», « (3)»… si el nombre está ocupado.
pub fn ruta_nueva(carpeta: &Path, nombre: &str, extension: &str) -> PathBuf {
    let limpio: String =
        nombre.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c }).collect();
    let limpio = limpio.trim().trim_end_matches('.').to_string();
    let mut ruta = carpeta.join(format!("{limpio}.{extension}"));
    let mut n = 2;
    while ruta.exists() {
        ruta = carpeta.join(format!("{limpio} ({n}).{extension}"));
        n += 1;
    }
    ruta
}

/// Un nombre no lleva paréntesis ni lo escrito en ellos.
pub fn sin_parentesis(texto: &str) -> String {
    let mut salida = String::new();
    let mut nivel = 0usize;
    for c in texto.chars() {
        match c {
            '(' => {
                nivel += 1;
                salida.push(' ');
            }
            ')' => {
                nivel = nivel.saturating_sub(1);
                salida.push(' ');
            }
            _ if nivel == 0 => salida.push(c),
            _ => {}
        }
    }
    salida.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Texto de un campo para correos, nóminas y Word.
pub fn valor(fila: &FilaTrabajo, campo: &str) -> String {
    let texto = fila.registro.texto(campo).trim().to_string();
    if campo == "nombre" {
        sin_parentesis(&texto)
    } else {
        texto
    }
}

struct Estilos {
    cabecera: Format,
    texto: Format,
    fecha: Format,
    observacion: Format,
}

fn estilos() -> Estilos {
    let base = Format::new().set_border(FormatBorder::Thin).set_border_color(Color::RGB(0xDDE2E9)).set_align(FormatAlign::Top);
    Estilos {
        cabecera: Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x2F5BD3))
            .set_font_color(Color::White)
            .set_border(FormatBorder::Thin)
            .set_text_wrap()
            .set_align(FormatAlign::VerticalCenter),
        fecha: base.clone().set_num_format("dd/mm/yyyy"),
        observacion: base.clone().set_text_wrap(),
        texto: base,
    }
}

fn escribir(hoja: &mut Worksheet, fila: u32, col: u16, celda: &Celda, e: &Estilos) -> Result<(), XlsxError> {
    match celda {
        Celda::Vacia => hoja.write_blank(fila, col, &e.texto)?,
        // write_string nunca interpreta fórmulas: un texto que empieza con «=» queda como texto.
        Celda::Texto(t) => hoja.write_string_with_format(fila, col, t, &e.texto)?,
        Celda::Numero(n) => hoja.write_number_with_format(fila, col, *n, &e.texto)?,
        Celda::Fecha(f) => match ExcelDateTime::from_ymd(f.year() as u16, f.month() as u8, f.day() as u8) {
            Ok(fecha) => hoja.write_datetime_with_format(fila, col, &fecha, &e.fecha)?,
            Err(_) => hoja.write_string_with_format(fila, col, &celda.texto(), &e.texto)?,
        },
    };
    Ok(())
}

fn ancho(cabecera: &str) -> f64 {
    match cabecera {
        "OBSERVACION" => 90.0,
        "REVISAR" => 50.0,
        c => (c.chars().count() as f64 + 4.0).clamp(12.0, 34.0),
    }
}

fn tabla(hoja: &mut Worksheet, cabeceras: &[String], filas: &[Vec<Celda>], e: &Estilos) -> Result<(), XlsxError> {
    for (c, cabecera) in cabeceras.iter().enumerate() {
        hoja.write_string_with_format(0, c as u16, cabecera, &e.cabecera)?;
        hoja.set_column_width(c as u16, ancho(cabecera))?;
    }
    for (f, celdas) in filas.iter().enumerate() {
        for (c, celda) in celdas.iter().enumerate().take(cabeceras.len()) {
            if matches!(cabeceras[c].as_str(), "OBSERVACION" | "REVISAR") {
                hoja.write_string_with_format(f as u32 + 1, c as u16, celda.texto(), &e.observacion)?;
            } else {
                escribir(hoja, f as u32 + 1, c as u16, celda, e)?;
            }
        }
    }
    hoja.set_freeze_panes(1, 0)?;
    if !filas.is_empty() && !cabeceras.is_empty() {
        hoja.autofilter(0, 0, filas.len() as u32, cabeceras.len() as u16 - 1)?;
    }
    Ok(())
}

fn guardar(mut libro: Workbook, destino: &Path) -> Result<(), String> {
    if destino.exists() {
        return Err(format!("Ya existe {}: no se sobrescribe.", destino.display()));
    }
    libro.save(destino).map_err(|e| format!("No se pudo guardar {}: {e}", destino.display()))
}

/// Lo que hay que mirar en una fila: marcas de revisión y avisos, en una sola celda.
pub fn revisar(fila: &FilaTrabajo) -> String {
    let mut partes: Vec<String> =
        fila.propuesta.marcas.iter().map(|m| format!("[{}] {}", m.nivel, m.texto)).collect();
    partes.extend(fila.propuesta.avisos.iter().cloned());
    partes.join("\n")
}

/// Libro con las columnas originales de RUS más OBSERVACION, FECHA_OBS, RES y REVISAR.
pub fn observaciones(trabajo: &Trabajo, destino: &Path) -> Result<(), String> {
    let e = estilos();
    let mut libro = Workbook::new();
    let usadas = trabajo.cabeceras.iter().rposition(|c| !c.is_empty()).map_or(0, |p| p + 1);
    let mut cabeceras: Vec<String> = trabajo.cabeceras[..usadas].to_vec();
    cabeceras.extend(["OBSERVACION", "FECHA_OBS", "RES", "REVISAR"].map(String::from));
    let filas: Vec<Vec<Celda>> = trabajo
        .filas
        .iter()
        .map(|f| {
            let mut celdas: Vec<Celda> = (0..usadas).map(|i| f.celdas.get(i).cloned().unwrap_or_default()).collect();
            celdas.push(Celda::Texto(f.observacion.clone()));
            celdas.push(if f.observacion.is_empty() { Celda::Vacia } else { Celda::Fecha(trabajo.corte) });
            celdas.push(f.proyecto().map(|p| Celda::Texto(p.to_string())).unwrap_or_default());
            celdas.push(Celda::Texto(revisar(f)));
            celdas
        })
        .collect();
    let hoja = libro.add_worksheet();
    let en = |x: XlsxError| x.to_string();
    hoja.set_name(trabajo.etapa.nombre()).map_err(en)?;
    tabla(hoja, &cabeceras, &filas, &e).map_err(en)?;
    let resumen = libro.add_worksheet();
    resumen.set_name("Resumen").map_err(en)?;
    let datos = [
        ("Etapa", trabajo.etapa.nombre().to_string()),
        ("Fecha de corte", trabajo.corte.format("%d/%m/%Y").to_string()),
        ("Archivo de origen", trabajo.archivo.clone()),
        ("Huella SHA-256 del origen", trabajo.sha256.clone()),
        ("Registros", trabajo.filas.len().to_string()),
        ("Con observación", trabajo.con_observacion().to_string()),
        ("Observaciones editadas", trabajo.filas.iter().filter(|f| f.editada()).count().to_string()),
        ("Estado", "Propuesta: requiere revisión del usuario antes de registrarla".to_string()),
    ];
    for (i, (campo, v)) in datos.iter().enumerate() {
        resumen.write_string_with_format(i as u32, 0, *campo, &e.cabecera).map_err(en)?;
        resumen.write_string_with_format(i as u32, 1, v, &e.texto).map_err(en)?;
    }
    resumen.set_column_width(0, 30).map_err(en)?;
    resumen.set_column_width(1, 70).map_err(en)?;
    guardar(libro, destino)
}

/// Dato que acompaña a cada registro en una nómina: días de espera, vencimiento o egreso proyectado.
pub fn metrica(trabajo: &Trabajo, fila: &FilaTrabajo, tipo: &str) -> String {
    let texto = match tipo {
        "programa_espera" => valor(fila, "espera"),
        "programa_vencido" | "programa_por_vencer" => {
            let propio = valor(fila, "vencimiento");
            if propio.is_empty() && trabajo.etapa == Etapa::Cumplimiento {
                fila.proximo_informe.map(|f| f.format("%d/%m/%Y").to_string()).unwrap_or_default()
            } else {
                propio
            }
        }
        _ => valor(fila, "egreso_proy"),
    };
    if texto.is_empty() {
        "Sin dato".into()
    } else {
        texto
    }
}

/// Nómina adjunta a un correo. A programas: RIT, tribunal, nombre, derivación y el dato. Al tribunal: además RUT y
/// la observación.
pub fn nomina(trabajo: &Trabajo, filas: &[&FilaTrabajo], tipo: &str, destino: &Path) -> Result<(), String> {
    let t = |s: String| Celda::Texto(s);
    let (cabeceras, datos): (Vec<&str>, Vec<Vec<Celda>>) = match tipo {
        "programa_espera" | "programa_vencido" | "programa_por_vencer" => (
            vec!["RIT", "TRIBUNAL", "NOMBRE", "DERIVACION", if tipo == "programa_espera" { "T ESPERA" } else { "F. VENCIMIENTO" }],
            filas
                .iter()
                .map(|f| {
                    vec![
                        t(valor(f, "rit")),
                        t(valor(f, "tribunal")),
                        t(valor(f, "nombre")),
                        t(valor(f, "programa")),
                        t(metrica(trabajo, f, tipo)),
                    ]
                })
                .collect(),
        ),
        _ => (
            vec!["RIT", "TRIBUNAL", "RUT", "NOMBRE", "PROGRAMA", "F. EGRESO PROYECTADO", "OBSERVACION"],
            filas
                .iter()
                .map(|f| {
                    vec![
                        t(valor(f, "rit")),
                        t(valor(f, "tribunal")),
                        t(valor(f, "rut")),
                        t(valor(f, "nombre")),
                        t(valor(f, "programa")),
                        t(metrica(trabajo, f, tipo)),
                        t(f.observacion.clone()),
                    ]
                })
                .collect(),
        ),
    };
    let cabeceras: Vec<String> = cabeceras.into_iter().map(String::from).collect();
    let mut libro = Workbook::new();
    let hoja = libro.add_worksheet();
    hoja.set_name("Nómina").map_err(|e| e.to_string())?;
    tabla(hoja, &cabeceras, &datos, &estilos()).map_err(|e| e.to_string())?;
    guardar(libro, destino)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn nombres_sin_parentesis() {
        assert_eq!(sin_parentesis("SOFÍA (hija) PÉREZ (2)"), "SOFÍA PÉREZ");
        assert_eq!(sin_parentesis("ANA ) SOTO"), "ANA SOTO");
    }
}
