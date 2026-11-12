//! Borradores en Outlook de escritorio. La única operación que existe es guardar un borrador.
//! Este crate no tiene, ni puede tener, una función que despache correos: lo controla `tests/sin_envio.rs`.

use flow_dominio::CC_FIJO;
use flow_productos::correos::{direcciones, Borrador};

#[cfg(windows)]
mod com;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resultado {
    /// Quedó en la carpeta Borradores de Outlook.
    Guardado,
    /// Outlook falló después de pedirle guardar: puede existir. Revisar Borradores antes de reintentar.
    Incierto(String),
}

/// Revisa el borrador antes de tocar Outlook. El CC fijo es obligatorio.
pub fn validar(borrador: &Borrador) -> Result<(), String> {
    let faltan = borrador.faltantes();
    if !faltan.is_empty() {
        return Err(format!("Al borrador le falta: {}.", faltan.join(", ")));
    }
    if !direcciones(&borrador.cc).iter().any(|d| d.eq_ignore_ascii_case(CC_FIJO)) {
        return Err("El borrador no lleva el CC fijo.".into());
    }
    if let Some(falta) = borrador.adjuntos.iter().find(|a| !a.is_file()) {
        return Err(format!("No existe el adjunto {}.", falta.display()));
    }
    Ok(())
}

/// Hay Outlook de escritorio en este equipo.
pub fn disponible() -> bool {
    #[cfg(windows)]
    {
        com::disponible()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Guarda el borrador en Outlook. No lo despacha: queda en Borradores para que el usuario lo revise.
pub fn guardar_borrador(borrador: &Borrador) -> Result<Resultado, String> {
    validar(borrador)?;
    #[cfg(windows)]
    {
        com::guardar(borrador)
    }
    #[cfg(not(windows))]
    {
        Err("Outlook de escritorio solo existe en Windows. Usa los archivos .eml.".into())
    }
}
