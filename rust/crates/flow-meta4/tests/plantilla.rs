//! Plantilla ficticia con formatos distinguibles. No usa información de personas reales.
use chrono::NaiveDate;
use flow_dominio::Celda;
use flow_meta4::{salida, Meta4, Registro4};
use rust_xlsxwriter::{Color, Format, Workbook};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

static SERIE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Temporal(PathBuf);
impl Temporal {
    fn nueva() -> Self {
        let carpeta = std::env::temp_dir().join(format!(
            "flow-meta4-estilo-{}-{}", std::process::id(),
            SERIE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&carpeta);
        std::fs::create_dir_all(&carpeta).unwrap();
        Self(carpeta)
    }
}
impl Drop for Temporal {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}
fn dia(t: &str) -> NaiveDate { t.parse().unwrap() }
fn meta() -> Meta4 {
    Meta4 {
        corte: dia("2026-10-16"), emision: dia("2026-10-16"), tribunales: None,
        registros: vec![Registro4 {
            lista: "Cumplimiento", categoria: Some("Residencia"), tribunal: Some("TOMÉ"),
            columnas: vec![
                ("RIT".into(), Celda::Texto("P-1-2099".into())),
                ("TRIBUNAL".into(), Celda::Texto("Juzgado de Familia de Tomé".into())),
                ("DERIVACIÓN".into(), Celda::Texto("RLP Residencia Ficticia".into())),
                ("FEC. INGRESO EFECTIVO".into(), Celda::Fecha(dia("2026-09-01"))),
                ("FEC. RESOLUCIÓN".into(), Celda::Fecha(dia("2026-08-15"))),
            ],
            archivo: "origen.xls".into(), lote: "Lote ficticio".into(),
            fecha_obs: Some(dia("2026-10-12")), etapa_ultima: "Cumplimiento".into(),
            control: "VERIFICADA EN EL PERÍODO".into(), alerta: String::new(),
        }],
        observaciones: Vec::new(), lecturas: Vec::new(), sin_bitacoras: Vec::new(),
        filtros: Vec::new(), lotes: vec!["Lote ficticio".into()], avisos: Vec::new(),
    }
}
fn plantilla(ruta: &Path, incompleta: bool) {
    let mut libro = Workbook::new();
    let cab = Format::new().set_bold().set_background_color(Color::RGB(0xC5A3FF));
    let titulo = Format::new().set_bold().set_font_color(Color::RGB(0x136F43));
    let dato = Format::new().set_background_color(Color::RGB(0xE2FFEE));
    let tabla = Format::new().set_background_color(Color::RGB(0xEAF2BB));
    for (i, (nombre, _, _, informe)) in salida::HOJAS.into_iter().enumerate() {
        if incompleta && i > 0 { break; }
        let hoja = libro.add_worksheet();
        hoja.set_name(nombre).unwrap();
        hoja.set_column_width(0, 35.0).unwrap();
        hoja.set_column_width(7, 21.0).unwrap();
        hoja.set_row_height(4, 38.0).unwrap();
        hoja.set_row_height(5, 29.0).unwrap();
        hoja.write_string_with_format(0, 0, "ARAUCO", &titulo).unwrap();
        hoja.write_string_with_format(1, 0, informe, &titulo).unwrap();
        hoja.write_string_with_format(2, 0, "Fecha Emisión   : 01-01-2000", &titulo).unwrap();
        for c in 0..8 {
            hoja.write_string_with_format(4, c, if c == 0 { "RIT" } else { "Campo ficticio" }, &cab).unwrap();
            hoja.write_string_with_format(5, c, if c == 0 { "P-0-2099" } else { "Valor ficticio" }, &dato).unwrap();
        }
        hoja.write_string_with_format(7, 0, "SIN REGISTROS", &tabla).unwrap();
        hoja.write_string_with_format(9, 0, "TOTAL:", &titulo).unwrap();
        hoja.write_number_with_format(9, 1, 0.0, &titulo).unwrap();
    }
    libro.save(ruta).unwrap();
}
fn parte(ruta: &Path, archivo: &str) -> String {
    let contenido = std::fs::read(ruta).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(contenido)).unwrap();
    let mut texto = String::new();
    zip.by_name(archivo).unwrap().read_to_string(&mut texto).unwrap();
    texto
}
#[test]
fn plantilla_colores_alturas_y_fechas_sin_alterar_valores() {
    let tmp = Temporal::nueva();
    let modelo = tmp.0.join("plantilla.xlsx");
    plantilla(&modelo, false);
    let antes = std::fs::read(&modelo).unwrap();
    let basico = tmp.0.join("Base");
    let base = salida::publicar(&meta(), &basico).unwrap();
    let libro_original = flow_tabular::leer(&base[0]).unwrap();
    let nueva = tmp.0.join("Resultado");
    let resultados = salida::publicar_con_plantilla(&meta(), &nueva, Some(&modelo)).unwrap();
    assert_eq!(resultados.len(), 5);
    assert_eq!(std::fs::read(&modelo).unwrap(), antes, "la plantilla es de solo lectura");
    let final_libro = flow_tabular::leer(&resultados[0]).unwrap();
    assert_eq!(libro_original.hojas.len(), final_libro.hojas.len());
    for (a, b) in libro_original.hojas.iter().zip(final_libro.hojas.iter()) {
        assert_eq!(a.nombre, b.nombre);
        assert_eq!(a.filas, b.filas, "el formato no cambia los datos");
    }
    let styles = parte(&resultados[0], "xl/styles.xml");
    assert!(styles.contains("C5A3FF"), "el color personalizado de cabecera se importó");
    assert!(styles.contains("dd-mm-yyyy"), "las fechas se formatean en día-mes-año");
    let xml = parte(&resultados[0], "xl/worksheets/sheet2.xml");
    assert!(xml.contains("ht=\"38"), "altura de cabecera personalizada: no se encontró ht=38");
    let xml_origen = parte(&modelo, "xl/worksheets/sheet2.xml");
    let desde = xml_origen.find("<cols>").unwrap();
    let hasta = xml_origen[desde..].find("</cols>").unwrap() + desde + "</cols>".len();
    assert!(xml.contains(&xml_origen[desde..hasta]), "los anchos deben copiarse exactamente del modelo");
    let hoja = final_libro.hojas.iter().find(|h| h.nombre == "Residencia Cumplimiento").unwrap();
    let fila = hoja.filas.iter().find(|f| f.first().is_some_and(|v| v.texto() == "P-1-2099")).unwrap();
    assert_eq!(fila[7].fecha(), Some(dia("2026-10-12")));
    let reporte: serde_json::Value = serde_json::from_slice(&std::fs::read(&resultados[4]).unwrap()).unwrap();
    assert_eq!(reporte["plantilla"], "Plantilla XLSX personalizada, estilos verificados");
}
#[test]
fn plantilla_incompleta_rechazada_sin_publicacion() {
    let tmp = Temporal::nueva();
    let modelo = tmp.0.join("plantilla-incompleta.xlsx");
    plantilla(&modelo, true);
    let destino = tmp.0.join("Resultado");
    let error = salida::publicar_con_plantilla(&meta(), &destino, Some(&modelo)).unwrap_err();
    assert!(error.contains("no tiene la hoja"), "{error}");
    assert!(!destino.exists(), "no se publica ningún producto parcial");
    assert!(!tmp.0.join(".en-curso-Resultado").exists(), "se limpia la carpeta temporal");
}
