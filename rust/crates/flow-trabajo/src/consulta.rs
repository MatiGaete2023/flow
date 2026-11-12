//! Consulta auxiliar local: conserva sus filas y muestra avisos; no ejecuta reglas del motor tradicional.
use crate::{analizar_tabla, controles::Origen, Contexto, Trabajo};
use flow_dominio::{texto::normalizar_columna, Fecha};
use flow_motor::Config;
use flow_tabular::Libro;

pub fn detectar(libro: &Libro) -> Option<&'static str> {
    if libro
        .hojas
        .iter()
        .flat_map(|h| h.filas.iter().take(30))
        .any(|f| f.iter().any(|c| normalizar_columna(&c.texto()).trim() == "estado o b"))
    {
        Some("BUSQUEDA_LITIGANTES")
    } else if normalizar_columna(&libro.nombre).contains("egresados")
        || libro.hojas.iter().any(|h| normalizar_columna(&h.nombre) == "egresados")
    {
        Some("EGRESADOS")
    } else {
        None
    }
}

pub fn analizar(libro: &Libro, tipo: &str, corte: Fecha, config: &Config) -> Result<Trabajo, String> {
    let tabla = flow_tabular::leer_consulta(libro, tipo)?;
    let mut contexto = Contexto::default();
    for fila in &tabla.filas {
        contexto.origenes.insert(
            fila.id.clone(),
            Origen {
                sistema: "RUS".into(),
                tribunal: String::new(),
                causa: String::new(),
                ingreso: String::new(),
                tipo: if tipo == "EGRESADOS" { "EGRESADOS" } else { "OTRO" }.into(),
                pantalla: if tipo == "BUSQUEDA_LITIGANTES" { "litigantes" } else { "seguimiento" }.into(),
                verificada: false,
                valores: tabla.cabeceras.iter().cloned().zip(fila.celdas.iter().cloned()).collect(),
            },
        );
    }
    Ok(analizar_tabla(libro, &tabla, corte, config, &contexto))
}
