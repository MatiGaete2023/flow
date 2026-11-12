//! Lectura de los Excel de RUS/SITFA: formato por contenido, encabezados, columnas y registros.

pub mod columnas;
pub mod html;
pub mod libro;
pub mod tabla;

pub use libro::{leer, leer_bytes, Formato, Hoja, Libro};
pub use tabla::{candidatas, leer_consulta, leer_tabla, leer_todas, Candidata, Fila, Tabla};
