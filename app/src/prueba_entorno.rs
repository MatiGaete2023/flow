//! Ayuda de pruebas que dependen de variables de entorno (E-L).
//!
//! Sin la variable la prueba debe fallar al pedirse con `--include-ignored`; nunca terminar con `return` en silencio.

use std::ffi::OsString;

/// Devuelve la variable o falla con el mensaje único de las pruebas condicionadas.
pub(crate) fn requerida(nombre: &str) -> OsString {
    std::env::var_os(nombre).unwrap_or_else(|| {
        panic!("{nombre} es obligatoria para esta prueba (ver docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md)")
    })
}
