//! Elección de hoja y encabezado con libros ficticios en memoria (paridad con `src/nurus/rus/reader.py`).

use flow_dominio::{Celda, Etapa};
use flow_tabular::{leer_tabla, leer_todas, Formato, Hoja, Libro};

fn fila(celdas: &[&str]) -> Vec<Celda> {
    celdas.iter().map(|c| if c.is_empty() { Celda::Vacia } else { Celda::Texto(c.to_string()) }).collect()
}

fn hoja(nombre: &str, filas: Vec<Vec<Celda>>) -> Hoja {
    Hoja { nombre: nombre.into(), filas }
}

fn libro(hojas: Vec<Hoja>) -> Libro {
    Libro { nombre: "consolidado_ficticio.xlsx".into(), sha256: "0".repeat(64), formato: Formato::Xlsx, hojas }
}

const CUMPLIMIENTO: &[&str] = &["RIT", "RUT", "NOMBRE", "DERIVACION", "TRIBUNAL", "DIAS DE CUMPLIMIENTO", "DIAS PARA EGRESAR"];

fn cumplimiento() -> Hoja {
    hoja(
        "Cumplimiento",
        vec![fila(CUMPLIMIENTO), fila(&["X-1-2099", "11111111-1", "Persona Ficticia", "PRM Ficticio", "Laja", "100", "50"])],
    )
}

/// Hoja de cruce: sus seis columnas también satisfacen las obligatorias de Informes.
fn hoja2() -> Hoja {
    hoja(
        "Hoja2",
        vec![
            fila(&["RIT", "RUT MENOR", "NOMBRE MENOR", "TRIBUNAL", "NOMBRE CENTRO", "FECHA VENCIMIENTO"]),
            fila(&["X-1-2099", "11111111-1", "Persona Ficticia", "Laja", "PRM Ficticio", "2099-01-15"]),
        ],
    )
}

fn informes() -> Hoja {
    hoja(
        "Informes",
        vec![
            fila(&["RIT", "RUT", "NOMBRE", "PROGRAMA", "TRIBUNAL", "FEC.VENCIMIENTO"]),
            fila(&["X-2-2099", "22222222-2", "Otra Persona Ficticia", "PRM Ficticio", "Laja", "2099-02-01"]),
            fila(&["X-3-2099", "33333333-3", "Tercera Persona Ficticia", "PRM Ficticio", "Laja", "2099-03-01"]),
        ],
    )
}

#[test]
fn la_hoja_de_cruce_no_se_toma_como_informes() {
    // Orden [Cumplimiento, Hoja2, Informes]: la tabla de Informes es la hoja «Informes», no la de cruce.
    let tablas = leer_todas(&libro(vec![cumplimiento(), hoja2(), informes()]));
    let etapas: Vec<(Etapa, &str)> = tablas.iter().map(|t| (t.etapa, t.hoja.as_str())).collect();
    assert_eq!(etapas, vec![(Etapa::Cumplimiento, "Cumplimiento"), (Etapa::Informes, "Informes")]);
    assert_eq!(tablas[1].filas.len(), 2);
    assert_eq!(tablas[0].cruce.len(), 1, "la Hoja2 sigue siendo el cruce del Cumplimiento");

    // Orden [Cumplimiento, Hoja2]: no hay tabla de Informes.
    let tablas = leer_todas(&libro(vec![cumplimiento(), hoja2()]));
    let etapas: Vec<Etapa> = tablas.iter().map(|t| t.etapa).collect();
    assert_eq!(etapas, vec![Etapa::Cumplimiento]);
    assert_eq!(tablas[0].cruce.len(), 1);

    // Pedida expresamente, la Hoja2 se puede leer como Informes.
    let t = leer_tabla(&libro(vec![cumplimiento(), hoja2()]), Some(Etapa::Informes), Some("Hoja2")).unwrap();
    assert_eq!(t.hoja, "Hoja2");
}

/// Informes con DERIVACION: sus columnas también forman una hoja de cruce completa.
fn informes_con_derivacion() -> Hoja {
    let mut h = informes();
    h.filas[0][3] = Celda::Texto("DERIVACION".into());
    h
}

#[test]
fn una_hoja_informes_con_columnas_de_cruce_sigue_siendo_informes() {
    // [Cumplimiento, Informes]: la hoja «Informes» es la única que podría ser cruce, pero se llama como su etapa.
    let tablas = leer_todas(&libro(vec![cumplimiento(), informes_con_derivacion()]));
    let etapas: Vec<(Etapa, &str)> = tablas.iter().map(|t| (t.etapa, t.hoja.as_str())).collect();
    assert_eq!(etapas, vec![(Etapa::Cumplimiento, "Cumplimiento"), (Etapa::Informes, "Informes")]);
    let t = leer_tabla(&libro(vec![cumplimiento(), informes_con_derivacion()]), Some(Etapa::Informes), None).unwrap();
    assert_eq!(t.hoja, "Informes");
    // [Cumplimiento, Hoja2, Informes con DERIVACION]: dos hojas de cruce posibles, como `_select_cross` no se usa
    // ninguna; la tabla de Informes es la hoja «Informes».
    let tablas = leer_todas(&libro(vec![cumplimiento(), hoja2(), informes_con_derivacion()]));
    let etapas: Vec<(Etapa, &str)> = tablas.iter().map(|t| (t.etapa, t.hoja.as_str())).collect();
    assert_eq!(etapas, vec![(Etapa::Cumplimiento, "Cumplimiento"), (Etapa::Informes, "Informes")]);
    assert!(tablas[0].cruce.is_empty());
    assert!(tablas[0].avisos.iter().any(|a| a.contains("más de una hoja de cruce")));
}

#[test]
fn encabezado_repetido_en_la_misma_hoja_usa_la_primera_fila() {
    let mut filas = informes().filas;
    // Exportación pegada dos veces: el encabezado reaparece más abajo.
    filas.push(filas[0].clone());
    filas.push(fila(&["X-4-2099", "44444444-4", "Cuarta Persona Ficticia", "PRM Ficticio", "Laja", "2099-04-01"]));
    let t = leer_tabla(&libro(vec![hoja("Informes", filas)]), None, None).unwrap();
    assert_eq!(t.fila_cabecera, 1);
    // Como `reader.py::_records`, la fila de encabezado repetida queda como registro (tiene datos mapeados).
    assert_eq!(t.filas.len(), 4);
}

#[test]
fn sin_tabla_se_conserva_el_error_de_mapeo() {
    let ambigua = hoja(
        "Espera",
        vec![
            fila(&["RIT", "RUT", "RUT MENOR", "NOMBRE", "PROGRAMA", "TRIBUNAL", "T ESPERA"]),
            fila(&["X-1-2099", "11111111-1", "11111111-1", "Persona Ficticia", "PRM Ficticio", "Laja", "10"]),
        ],
    );
    let error = leer_tabla(&libro(vec![ambigua]), None, None).unwrap_err();
    assert!(error.contains("Columna ambigua para rut"), "{error}");
    let duplicada = hoja("Espera", vec![fila(&["RIT", "NOMBRE", "PROGRAMA", "TRIBUNAL", "T ESPERA", "Tribunal"])]);
    let error = leer_tabla(&libro(vec![duplicada]), Some(Etapa::Espera), None).unwrap_err();
    assert!(error.contains("duplicados"), "{error}");
    // Un archivo sin nada de RUS conserva el mensaje general.
    let ajena = hoja("Hoja1", vec![fila(&["Título", "Título"]), fila(&["a", "b"])]);
    assert!(leer_tabla(&libro(vec![ajena]), None, None).unwrap_err().starts_with("No se identifica"));
}
