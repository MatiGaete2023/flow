//! Ayuda incluida en el ejecutable: una sección por vista y las reglas que no cambian. Texto fijo, sin red.

pub const SECCIONES: [(&str, &str); 9] = [
    ("Trabajo", "Abre la nómina (Excel, HTML/XML de SITFA o el ZIP del Descargador), elige la etapa y revisa cada fila: la observación sale de las reglas y puedes editarla, excluir la fila o consignar. «Actualizar desde Excel» trae los cambios de un libro editado y «Conciliar capturas» compara dos paquetes. La edición masiva se deshace."),
    ("Correos", "Prepara borradores por tribunal, modalidad o filas elegidas, con firma y CC fijo. Solo se guardan en Borradores de Outlook (Windows con Outlook de escritorio) o como .eml; nada se envía. Un borrador cuyo origen cambió se marca como obsoleto."),
    ("Resoluciones", "Genera los proyectos de resolución Word desde las matrices institucionales o desde tus plantillas propias (Ajustes). Cada proyecto se guarda en una carpeta nueva."),
    ("Resultados", "Resumen del trabajo, estadísticas de revisión, órdenes de búsqueda y los productos: libro de observaciones en la estructura FAE_RES, vencimientos y trazabilidad. Los archivos nuevos nunca reemplazan a otros."),
    ("Contactos", "Correos por programa y por tribunal, con alias. Se guardan en tu carpeta de datos."),
    ("Anonimizador", "Copia con identidades ficticias de tablas, texto, calendarios, Word, ZIP y paquetes de captura. Estricto es lo recomendado. Cada copia se verifica antes de publicarse y el original no se modifica. Revisa a mano toda copia antes de compartirla."),
    ("Descargador", "Lee SITFA con tu sesión (nunca guarda credenciales) y arma el paquete del lote con su consolidado, el Libro CSMP, el cruce y las bitácoras. Solo lectura."),
    ("Ajustes", "Umbrales y textos de las observaciones (los valores base son los del original), carpeta de productos, cuenta de Outlook, matrices Word propias y el autodiagnóstico."),
    ("Reglas que no cambian", "Flow no escribe en RUS ni en SATURNO. Outlook solo recibe borradores con CC a ucc_concepcion@pjud.cl; nunca se envía. No hay credenciales, cookies ni tokens en archivos. Todo el procesamiento es local. Nada de lo que muestra Flow es una aceptación institucional."),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hay_una_seccion_por_cada_vista_y_las_reglas_fijas() {
        for (_, _, nombre) in crate::ui::VISTAS.iter() {
            assert!(SECCIONES.iter().any(|(n, t)| n == nombre && !t.is_empty()), "falta la ayuda de {nombre}");
        }
        let reglas = SECCIONES.iter().find(|(n, _)| *n == "Reglas que no cambian").unwrap().1;
        assert!(reglas.contains("RUS") && reglas.contains("SATURNO") && reglas.contains("ucc_concepcion@pjud.cl"));
    }
}
