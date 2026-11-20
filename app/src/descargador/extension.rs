//! Extensiones de Chrome de Flow Python incluidas en el ejecutable (`descargador.setup` y `SitfaLite.setup`).
//! «Instalar extensión…» deja la carpeta en el perfil local del usuario; Chrome la carga con «Cargar sin empaquetar».

use super::Edicion;
use std::path::{Path, PathBuf};

macro_rules! archivo {
    ($carpeta:literal, $ruta:literal) => {
        (
            $ruta,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../", $carpeta, "/extension/", $ruta)).as_slice(),
        )
    };
}

const LITE: [(&str, &[u8]); 10] = [
    archivo!("descargador_lite", "manifest.json"),
    archivo!("descargador_lite", "background.js"),
    archivo!("descargador_lite", "conexion.css"),
    archivo!("descargador_lite", "conexion.html"),
    archivo!("descargador_lite", "conexion.js"),
    archivo!("descargador_lite", "pagina.js"),
    archivo!("descargador_lite", "icons/icono16.png"),
    archivo!("descargador_lite", "icons/icono32.png"),
    archivo!("descargador_lite", "icons/icono48.png"),
    archivo!("descargador_lite", "icons/icono128.png"),
];

const INTEGRAL: [(&str, &[u8]); 6] = [
    archivo!("descargador", "manifest.json"),
    archivo!("descargador", "background.js"),
    archivo!("descargador", "conexion.css"),
    archivo!("descargador", "conexion.html"),
    archivo!("descargador", "conexion.js"),
    archivo!("descargador", "pagina.js"),
];

pub fn archivos(edicion: Edicion) -> &'static [(&'static str, &'static [u8])] {
    match edicion {
        Edicion::Lite => &LITE,
        Edicion::Integral => &INTEGRAL,
    }
}

/// Versión declarada en el `manifest.json` incluido.
pub fn version(edicion: Edicion) -> String {
    serde_json::from_slice::<serde_json::Value>(archivos(edicion)[0].1)
        .ok()
        .and_then(|m| m["version"].as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Carpeta local del usuario (`%LOCALAPPDATA%\FlowRust`), fuera de Descargas y de la carpeta del programa.
pub fn carpeta_base() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("FlowRust")
}

/// Escribe la extensión de la edición en `base/extension-<edición>-<versión>` y comprueba lo escrito.
pub fn instalar(edicion: Edicion, base: &Path) -> Result<PathBuf, String> {
    let nombre = match edicion {
        Edicion::Lite => "lite",
        Edicion::Integral => "integral",
    };
    let carpeta = base.join(format!("extension-{nombre}-{}", version(edicion)));
    for (ruta, datos) in archivos(edicion) {
        let destino = carpeta.join(ruta);
        if std::fs::read(&destino).is_ok_and(|actual| actual == *datos) {
            continue;
        }
        if let Some(padre) = destino.parent() {
            std::fs::create_dir_all(padre)
                .map_err(|e| format!("No se pudo crear la carpeta de la extensión: {e}"))?;
        }
        std::fs::write(&destino, datos)
            .map_err(|e| format!("No se pudo escribir la extensión en {}: {e}", carpeta.display()))?;
    }
    for (ruta, datos) in archivos(edicion) {
        if std::fs::read(carpeta.join(ruta)).ok().as_deref() != Some(*datos) {
            return Err("La copia de la extensión no coincide con la incluida en Flow. Revisa permisos o antivirus.".into());
        }
    }
    Ok(carpeta)
}

/// Abre `chrome://extensions/` con Chrome (`open_chrome` de Flow Python). Devuelve `false` si no se encontró Chrome.
pub fn abrir_extensiones_chrome() -> bool {
    let candidatos: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|b| PathBuf::from(b).join("Google").join("Chrome").join("Application").join("chrome.exe"))
        .collect();
    candidatos
        .iter()
        .find(|c| c.is_file())
        .is_some_and(|c| std::process::Command::new(c).arg("chrome://extensions/").spawn().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instala_las_dos_ediciones_con_sus_versiones_y_es_idempotente() {
        let mut sal = [0u8; 8];
        getrandom::fill(&mut sal).unwrap();
        let base = std::env::temp_dir().join(format!("flow-ext-{}", sal.iter().map(|b| format!("{b:02x}")).collect::<String>()));
        assert_eq!(version(Edicion::Lite), "1.1.0");
        assert_eq!(version(Edicion::Integral), "2.9.0");
        let lite = instalar(Edicion::Lite, &base).unwrap();
        assert!(lite.ends_with("extension-lite-1.1.0"));
        assert!(lite.join("icons/icono128.png").is_file());
        assert_eq!(instalar(Edicion::Lite, &base).unwrap(), lite);
        let integral = instalar(Edicion::Integral, &base).unwrap();
        assert!(integral.join("manifest.json").is_file());
        assert!(!lite.to_string_lossy().contains("_internal"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
