//! Libros Excel de salida: observaciones y nóminas. Siempre archivos nuevos: nunca se sobrescribe.

use flow_dominio::{Celda, Etapa};
use flow_trabajo::{FilaTrabajo, Trabajo};
use rust_xlsxwriter::{Color, ExcelDateTime, Format, FormatAlign, FormatBorder, Workbook, Worksheet, XlsxError};
use std::path::{Path, PathBuf};

use chrono::Datelike;

/// Ruta que no existe todavía: agrega « (2)», « (3)»… si el nombre está ocupado.
pub fn ruta_nueva(carpeta: &Path, nombre: &str, extension: &str) -> PathBuf {
    let limpio = nombre_archivo(nombre);
    let mut ruta = carpeta.join(format!("{limpio}.{extension}"));
    let mut n = 2;
    while ruta.exists() {
        ruta = carpeta.join(format!("{limpio} ({n}).{extension}"));
        n += 1;
    }
    ruta
}

/// Nombre de archivo válido en Windows (`outputs.py::program_filename`): sin caracteres prohibidos ni de control,
/// sin puntos ni espacios al final, sin nombres reservados (CON, PRN, AUX, NUL, COM1–9, LPT1–9) y de 120 caracteres
/// como máximo.
pub fn nombre_archivo(nombre: &str) -> String {
    let limpio: String = nombre.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || (c as u32) < 0x20 { '_' } else { c }).collect();
    let mut limpio = limpio.trim().trim_end_matches(['.', ' ']).to_string();
    if limpio.is_empty() {
        limpio = "Programa no informado".into();
    }
    let raiz = limpio.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reservado = matches!(raiz.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (raiz.len() == 4 && (raiz.starts_with("COM") || raiz.starts_with("LPT")) && matches!(raiz.as_bytes()[3], b'1'..=b'9'));
    if reservado {
        limpio.insert(0, '_');
    }
    limpio.chars().take(120).collect::<String>().trim_end_matches(['.', ' ']).to_string()
}

/// Carpeta nueva dentro de `base`: agrega « (2)», « (3)»… si el nombre está ocupado. La crea.
pub fn carpeta_nueva(base: &Path, nombre: &str) -> Result<PathBuf, String> {
    let mut carpeta = base.join(nombre);
    let mut n = 2;
    while carpeta.exists() {
        carpeta = base.join(format!("{nombre} ({n})"));
        n += 1;
    }
    std::fs::create_dir_all(&carpeta).map_err(|e| format!("No se pudo crear {}: {e}", carpeta.display()))?;
    Ok(carpeta)
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
    numero: Format,
}

/// `personal/excel_presentation.py`: Calibri 11, borde fino negro, ajuste de texto arriba; cabecera azul petróleo.
fn estilos() -> Estilos {
    let base = Format::new()
        .set_font_name("Calibri")
        .set_font_size(11)
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::Black)
        .set_text_wrap()
        .set_align(FormatAlign::Top);
    Estilos {
        cabecera: base.clone().set_bold().set_font_color(Color::White).set_background_color(Color::RGB(0x195F7B)),
        fecha: base.clone().set_num_format("dd/mm/yyyy"),
        texto: base.clone().set_num_format("@"),
        numero: base,
    }
}

fn escribir(hoja: &mut Worksheet, fila: u32, col: u16, celda: &Celda, e: &Estilos) -> Result<(), XlsxError> {
    match celda {
        Celda::Vacia => hoja.write_blank(fila, col, &e.numero)?,
        // write_string nunca interpreta fórmulas: un texto que empieza con «=» queda como texto.
        Celda::Texto(t) => hoja.write_string_with_format(fila, col, t, &e.texto)?,
        Celda::Numero(n) => hoja.write_number_with_format(fila, col, *n, &e.numero)?,
        Celda::Fecha(f) => match ExcelDateTime::from_ymd(f.year() as u16, f.month() as u8, f.day() as u8) {
            Ok(fecha) => hoja.write_datetime_with_format(fila, col, &fecha, &e.fecha)?,
            Err(_) => hoja.write_string_with_format(fila, col, celda.texto(), &e.texto)?,
        },
    };
    Ok(())
}

/// `excel_presentation.py::column_width`.
pub fn ancho(cabecera: &str) -> f64 {
    let t = cabecera.to_uppercase();
    let tiene = |xs: &[&str]| xs.iter().any(|x| t.contains(x));
    if tiene(&["OBSERVACION", "OBSERVACIÓN", "DETALLE", "TEXTO", "TRÁMITE", "TRAMITE"]) {
        65.0
    } else if tiene(&["NOMBRE", "PERSONA"]) {
        35.0
    } else if tiene(&["PROGRAMA", "DERIVACION"]) {
        30.0
    } else if t.contains("TRIBUNAL") {
        28.0
    } else if t == "RIT" {
        17.0
    } else if t.contains("RUT") {
        16.0
    } else if tiene(&["FECHA", "F.", "VENCIMIENTO", "EGRESO"]) {
        19.0
    } else if tiene(&["ESPERA", "DÍAS", "DIAS"]) {
        14.0
    } else {
        24.0
    }
}

/// Líneas que ocupa una fila (`format_sheet`): cada parte separada por salto, según el ancho de su columna.
fn lineas(cabeceras: &[String], celdas: &[Celda]) -> usize {
    celdas
        .iter()
        .zip(cabeceras)
        .map(|(c, cab)| {
            let util = (ancho(cab) - 3.0).max(8.0);
            c.texto().split('\n').map(|parte| ((parte.chars().count() as f64 / util).ceil() as usize).max(1)).sum::<usize>()
        })
        .max()
        .unwrap_or(1)
}

fn tabla(hoja: &mut Worksheet, cabeceras: &[String], filas: &[Vec<Celda>], e: &Estilos) -> Result<(), XlsxError> {
    for (c, cabecera) in cabeceras.iter().enumerate() {
        hoja.write_string_with_format(0, c as u16, cabecera, &e.cabecera)?;
        hoja.set_column_width(c as u16, ancho(cabecera))?;
    }
    for (f, celdas) in filas.iter().enumerate() {
        for (c, celda) in celdas.iter().enumerate().take(cabeceras.len()) {
            escribir(hoja, f as u32 + 1, c as u16, celda, e)?;
        }
        // Un texto muy largo queda al ajuste automático de Excel en vez de forzar una altura recortada.
        let n = lineas(cabeceras, celdas);
        if n <= 12 {
            hoja.set_row_height(f as u32 + 1, (15 * n + 5).max(22) as f64)?;
        }
    }
    hoja.set_freeze_panes(1, 0)?;
    hoja.set_repeat_rows(0, 0)?;
    hoja.set_landscape();
    hoja.set_paper_size(9);
    hoja.set_print_fit_to_pages(1, 0);
    hoja.set_margins(0.3, 0.3, 0.45, 0.45, 0.2, 0.2);
    hoja.set_footer("&CPágina &P de &N");
    hoja.set_print_center_horizontally(true);
    hoja.set_row_height(0, 32)?;
    if !cabeceras.is_empty() {
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
    let mut partes: Vec<String> = fila.propuesta.marcas.iter().map(|m| format!("[{}] {}", m.nivel, m.texto)).collect();
    partes.extend(fila.propuesta.avisos.iter().cloned());
    partes.join("\n")
}

/// `services/exports.py::_normalized_header`: mayúsculas, «_» como espacio y espacios simples (conserva tildes).
pub fn cabecera_normalizada(texto: &str) -> String {
    texto.trim().to_uppercase().replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `services/exports.py::_column_plan`: cada título usa la columna existente con ese nombre o una nueva al final, en
/// orden. Una columna repetida es un error. Posiciones desde 0.
pub fn plan_columnas(cabeceras: &[String], titulos: &[&str]) -> Result<Vec<usize>, String> {
    let posiciones = |nombre: &str| -> Vec<usize> {
        cabeceras.iter().enumerate().filter(|(_, c)| cabecera_normalizada(c) == nombre).map(|(i, _)| i).collect()
    };
    for t in titulos {
        let nombre = cabecera_normalizada(t);
        if posiciones(&nombre).len() > 1 {
            return Err(format!("Hay más de una columna '{nombre}'; no se puede anotar sin ambigüedad."));
        }
    }
    let mut ultima = cabeceras.len();
    let mut salida = Vec::new();
    for t in titulos {
        let pos = posiciones(&cabecera_normalizada(t)).first().copied().unwrap_or(ultima);
        salida.push(pos);
        ultima = ultima.max(pos + 1);
    }
    Ok(salida)
}

/// Columnas que Flow anota, en el orden de la copia revisada de Python, más REVISAR (propia de Flow).
pub const ANOTADAS: [&str; 6] = ["OBSERVACION", "FECHA_OBS", "TT", "CC", "RES", "REVISAR"];

fn en_blanco(celda: &Celda) -> bool {
    celda.texto().trim().is_empty()
}

/// Libro con las columnas originales de RUS y OBSERVACION, FECHA_OBS, TT, CC, RES y REVISAR. Una columna que ya
/// existe en el origen se reutiliza en su lugar (`_column_plan`); las demás se agregan al final.
///
/// Como la copia de Python: una OBSERVACION o un RES ya escritos en el origen se conservan, salvo que el usuario
/// haya editado la observación en Flow; TT y CC no se calculan (quedan los del origen). FECHA_OBS (propio de Flow)
/// toma la fecha de corte solo donde Flow escribe la observación y la celda está vacía.
pub fn observaciones(trabajo: &Trabajo, destino: &Path) -> Result<(), String> {
    let e = estilos();
    let mut libro = Workbook::new();
    let usadas = trabajo.cabeceras.iter().rposition(|c| !c.is_empty()).map_or(0, |p| p + 1);
    let mut cabeceras: Vec<String> = trabajo.cabeceras[..usadas].to_vec();
    let plan = plan_columnas(&cabeceras, &ANOTADAS)?;
    let total = plan.iter().map(|p| p + 1).max().unwrap_or(0).max(usadas);
    cabeceras.resize(total, String::new());
    for (titulo, &pos) in ANOTADAS.iter().zip(&plan) {
        if cabeceras[pos].trim().is_empty() {
            cabeceras[pos] = titulo.to_string();
        }
    }
    let [obs, fecha, _tt, _cc, res, rev] = [plan[0], plan[1], plan[2], plan[3], plan[4], plan[5]];
    let res_origen = crate::word::columna_humana(trabajo, &crate::word::ALIAS_RES);
    let filas: Vec<Vec<Celda>> = trabajo
        .filas
        .iter()
        .map(|f| {
            let mut celdas: Vec<Celda> =
                (0..total).map(|i| if i < usadas { f.celdas.get(i).cloned().unwrap_or_default() } else { Celda::Vacia }).collect();
            if f.editada() || en_blanco(&celdas[obs]) {
                celdas[obs] = if f.observacion.is_empty() { Celda::Vacia } else { Celda::Texto(f.observacion.clone()) };
                if en_blanco(&celdas[fecha]) && !f.observacion.is_empty() {
                    celdas[fecha] = Celda::Fecha(trabajo.corte);
                }
            }
            // `resolution_value`: el RES del origen si trae esa columna (aunque esté vacía); si no, el tipo sugerido.
            if en_blanco(&celdas[res]) {
                celdas[res] = match res_origen {
                    Some(i) => f.celdas.get(i).cloned().unwrap_or_default(),
                    None => f.proyecto().map(|p| Celda::Texto(p.to_string())).unwrap_or_default(),
                };
            }
            let mut nota = revisar(f);
            let conservada = celdas[obs].texto();
            if conservada.trim() != f.observacion.trim() {
                // Lo que se ve en Trabajo no es lo que queda en el archivo: se dice en la misma fila.
                let aviso = format!("Se conservó la OBSERVACION del origen; propuesta de Flow: {}", f.observacion);
                nota = if nota.is_empty() { aviso } else { format!("{nota}\n{aviso}") };
            }
            celdas[rev] = Celda::Texto(nota);
            celdas
        })
        .collect();
    let hoja = libro.add_worksheet();
    let en = |x: XlsxError| x.to_string();
    hoja.set_name(trabajo.nombre()).map_err(en)?;
    tabla(hoja, &cabeceras, &filas, &e).map_err(en)?;
    if !filas.is_empty() {
        // Lista cerrada para RES (`_portable_resolution_validation`); vacío significa que no corresponde proyecto.
        let validacion = rust_xlsxwriter::DataValidation::new()
            .allow_list_strings(&["PC_IE", "PC_INFO", "NOMENCL"])
            .and_then(|v| v.set_error_title("Tipo RES no válido"))
            .and_then(|v| v.set_error_message("Usa PC_IE, PC_INFO o NOMENCL, o deja la celda vacía."))
            .and_then(|v| v.set_input_title("Tipo de resolución"))
            .and_then(|v| v.set_input_message("Selecciona PC_IE, PC_INFO o NOMENCL. Vacío significa que no corresponde proyecto."))
            .map_err(en)?
            .ignore_blank(true);
        hoja.add_data_validation(1, res as u16, filas.len() as u32, res as u16, &validacion).map_err(en)?;
    }
    let resumen = libro.add_worksheet();
    resumen.set_name("Resumen").map_err(en)?;
    let datos = [
        ("Etapa", trabajo.nombre().to_string()),
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

/// `outputs.py::mail_metric_key`: qué dato lleva la nómina (días de espera, vencimiento o egreso proyectado). Los
/// tipos sin dato propio (reporte de proyectos) usan el de la etapa.
pub fn clave_metrica(etapa: Etapa, tipo: &str) -> &'static str {
    match tipo {
        "espera" | "programa_espera" => "espera",
        "informes" | "programa_vencido" | "programa_por_vencer" => "vencimiento",
        "cumplimiento" | "medidas" => "egreso_proy",
        _ => match etapa {
            Etapa::Cumplimiento => "egreso_proy",
            Etapa::Informes => "vencimiento",
            Etapa::Espera => "espera",
        },
    }
}

/// Dato que acompaña a cada registro en una nómina: días de espera, vencimiento o egreso proyectado.
pub fn metrica(trabajo: &Trabajo, fila: &FilaTrabajo, tipo: &str) -> String {
    let texto = match clave_metrica(trabajo.etapa, tipo) {
        "espera" => valor(fila, "espera"),
        "vencimiento" => {
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
            vec![
                "RIT",
                "TRIBUNAL",
                "RUT",
                "NOMBRE",
                "PROGRAMA",
                match clave_metrica(trabajo.etapa, tipo) {
                    "espera" => "T ESPERA",
                    "vencimiento" => "F. VENCIMIENTO",
                    _ => "F. EGRESO PROYECTADO",
                },
                "OBSERVACION",
            ],
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

    fn cab(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn plan_de_columnas_como_column_plan() {
        // Resultados de `services/exports.py::_column_plan` (base 0) con los mismos encabezados ficticios.
        assert_eq!(plan_columnas(&cab(&["RIT", "NOMBRE"]), &ANOTADAS).unwrap(), [2, 3, 4, 5, 6, 7]);
        assert_eq!(plan_columnas(&cab(&["RIT", "observacion", "x", "Fecha obs", "RES"]), &ANOTADAS).unwrap(), [1, 3, 5, 6, 4, 7]);
        assert_eq!(plan_columnas(&cab(&["RIT", "TT", "  cc ", "RES", "OBSERVACIÓN"]), &ANOTADAS).unwrap(), [5, 6, 1, 2, 3, 7]);
        assert_eq!(
            plan_columnas(&cab(&["A", "RES", "res"]), &ANOTADAS).unwrap_err(),
            "Hay más de una columna 'RES'; no se puede anotar sin ambigüedad."
        );
    }

    #[test]
    fn anchos_como_excel_presentation() {
        let titulos = [
            "RIT",
            "TRIBUNAL",
            "RUT",
            "NOMBRE",
            "PROGRAMA",
            "T ESPERA",
            "OBSERVACION",
            "REVISAR",
            "F. VENCIMIENTO",
            "DERIVACION",
            "FEC.EGRESO PROYECTADO",
            "DIAS DE CUMPLIMIENTO",
        ];
        let python = [17.0, 28.0, 16.0, 35.0, 30.0, 14.0, 65.0, 24.0, 19.0, 30.0, 19.0, 14.0];
        assert_eq!(titulos.map(ancho), python);
    }

    #[test]
    fn nombres_sin_parentesis() {
        assert_eq!(sin_parentesis("SOFÍA (hija) PÉREZ (2)"), "SOFÍA PÉREZ");
        assert_eq!(sin_parentesis("ANA ) SOTO"), "ANA SOTO");
    }
}
