//! Captura desde SITFA con la sesión del usuario en su Chrome, a través de la extensión.
//! Solo lectura: no existe ningún comando que escriba en RUS. No se leen cookies ni contraseñas.

pub mod evidencia;
pub mod lecturas;
pub mod lotes;
pub mod puente;
pub mod recoleccion;
pub mod sitfa;
pub mod vinculos;

pub use puente::{ErrorPuente, Puente};
