//! Captura desde SITFA con la sesión del usuario en su Chrome, a través de la extensión.
//! Solo lectura: no existe ningún comando que escriba en RUS. No se leen cookies ni contraseñas.

pub mod puente;

pub use puente::{ErrorPuente, Puente};
