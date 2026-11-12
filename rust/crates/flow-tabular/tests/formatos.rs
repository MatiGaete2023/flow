//! Las tres variantes de `.xls` (BIFF, HTML y XML 2003) con los mismos datos ficticios dan los mismos registros.

use flow_dominio::{Etapa, Fecha};
use flow_tabular::{leer, leer_tabla, Formato};

fn tabla(archivo: &str) -> (Formato, flow_tabular::Tabla) {
    let ruta = format!("{}/tests/datos/{archivo}", env!("CARGO_MANIFEST_DIR"));
    let libro = leer(std::path::Path::new(&ruta)).unwrap();
    let tabla = leer_tabla(&libro, Some(Etapa::Espera), None).unwrap();
    (libro.formato, tabla)
}

#[test]
fn tres_formatos_mismos_registros() {
    let (f1, biff) = tabla("espera_biff.xls");
    let (f2, html) = tabla("espera_html.xls");
    let (f3, xml) = tabla("espera_xml.xls");
    assert_eq!((f1, f2, f3), (Formato::XlsBiff, Formato::Html, Formato::Xml2003));
    assert_eq!(biff.fila_cabecera, 3, "portada y título antes del encabezado");
    assert_eq!(biff.hoja, "Datos");
    for t in [&biff, &html, &xml] {
        assert_eq!(t.filas.len(), 2, "sin filas vacías ni totales");
        let r = &t.filas[0].registro;
        assert_eq!(r.texto("nombre"), "SOFÍA (hija) PÉREZ");
        assert_eq!(r.texto("programa"), "PRM Ñuble");
        assert_eq!(r.entero("espera"), Some(45));
        assert_eq!(r.fecha("resolucion"), Fecha::from_ymd_opt(2026, 10, 1));
        assert_eq!(r.fecha("nacimiento"), Fecha::from_ymd_opt(2015, 3, 2));
        assert_eq!(r.celda("prox_aud").unwrap().fechas_varias().0.len(), 2);
        assert!(t.filas[1].registro.celda("prox_aud").unwrap().es_vacia());
    }
}
