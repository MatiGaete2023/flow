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

/// Qué hizo la instalación con la carpeta de la extensión.
#[derive(Debug, PartialEq, Eq)]
pub enum Cambio {
    /// La carpeta no existía: hay que cargarla una vez en Chrome.
    Nueva,
    /// Cambió su código: la carpeta es la misma y en Chrome basta «Recargar», sin volver a cargarla.
    Actualizada { de: String, a: String },
    /// Ya estaba al día: no se escribió nada y no hay nada que hacer en Chrome.
    SinCambios,
}

#[derive(Debug)]
pub struct Instalacion {
    pub carpeta: PathBuf,
    pub cambio: Cambio,
    /// Carpetas de instalaciones anteriores con la versión en el nombre (`extension-integral-2.9.0`): siguen sirviendo.
    pub anteriores: Vec<PathBuf>,
}

fn nombre(edicion: Edicion) -> &'static str {
    match edicion {
        Edicion::Lite => "lite",
        Edicion::Integral => "integral",
    }
}

fn archivos_de(carpeta: &Path) -> Vec<PathBuf> {
    let mut salida = Vec::new();
    let mut pendientes = vec![carpeta.to_path_buf()];
    while let Some(c) = pendientes.pop() {
        for e in std::fs::read_dir(&c).into_iter().flatten().flatten() {
            let ruta = e.path();
            if ruta.is_dir() {
                pendientes.push(ruta);
            } else {
                salida.push(ruta);
            }
        }
    }
    salida
}

/// Escribe la extensión de la edición en `base/extension-<edición>`, una carpeta fija que se actualiza en el lugar:
/// solo se reescriben los archivos que cambiaron y se quitan los que dejaron de existir. Comprueba lo escrito.
pub fn instalar_en_el_lugar(edicion: Edicion, base: &Path) -> Result<Instalacion, String> {
    let carpeta = base.join(format!("extension-{}", nombre(edicion)));
    let existia = carpeta.join("manifest.json").is_file();
    let version_previa = std::fs::read(carpeta.join("manifest.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|m| m["version"].as_str().map(str::to_string))
        .unwrap_or_default();
    let mut escribio = false;
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
        escribio = true;
    }
    // Un archivo que ya no forma parte de la extensión se quita para que Chrome no cargue restos de otra versión.
    let vigentes: Vec<PathBuf> = archivos(edicion).iter().map(|(r, _)| carpeta.join(r)).collect();
    for sobrante in archivos_de(&carpeta).into_iter().filter(|a| !vigentes.contains(a)) {
        std::fs::remove_file(&sobrante).map_err(|e| format!("No se pudo quitar {}: {e}", sobrante.display()))?;
        escribio = true;
    }
    for (ruta, datos) in archivos(edicion) {
        if std::fs::read(carpeta.join(ruta)).ok().as_deref() != Some(*datos) {
            return Err("La copia de la extensión no coincide con la incluida en Flow. Revisa permisos o antivirus.".into());
        }
    }
    let cambio = match (existia, escribio) {
        (false, _) => Cambio::Nueva,
        (true, false) => Cambio::SinCambios,
        (true, true) => Cambio::Actualizada { de: version_previa, a: version(edicion) },
    };
    let prefijo = format!("extension-{}-", nombre(edicion));
    let mut anteriores: Vec<PathBuf> = std::fs::read_dir(base)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&prefijo)))
        .collect();
    anteriores.sort();
    Ok(Instalacion { carpeta, cambio, anteriores })
}

/// `instalar_en_el_lugar` sin el detalle de qué cambió.
pub fn instalar(edicion: Edicion, base: &Path) -> Result<PathBuf, String> {
    instalar_en_el_lugar(edicion, base).map(|i| i.carpeta)
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
        let base = carpeta_temporal();
        assert_eq!(version(Edicion::Lite), "1.1.0");
        assert_eq!(version(Edicion::Integral), "2.9.0");
        let lite = instalar(Edicion::Lite, &base).unwrap();
        assert!(lite.ends_with("extension-lite"), "la carpeta es fija: no lleva la versión");
        assert!(lite.join("icons/icono128.png").is_file());
        assert_eq!(instalar(Edicion::Lite, &base).unwrap(), lite);
        let integral = instalar(Edicion::Integral, &base).unwrap();
        assert!(integral.join("manifest.json").is_file());
        assert!(!lite.to_string_lossy().contains("_internal"));
        let _ = std::fs::remove_dir_all(&base);
    }

    fn carpeta_temporal() -> PathBuf {
        let mut sal = [0u8; 8];
        getrandom::fill(&mut sal).unwrap();
        std::env::temp_dir().join(format!("flow-ext-{}", sal.iter().map(|b| format!("{b:02x}")).collect::<String>()))
    }

    /// Una corrección de la extensión no obliga a cargarla de nuevo: la carpeta es la misma, solo cambian los archivos que
    /// cambiaron, se quitan los sobrantes, y sin cambios no se escribe nada.
    #[test]
    fn la_extension_se_actualiza_en_el_lugar_sin_cambiar_de_carpeta() {
        let base = carpeta_temporal();
        std::fs::create_dir_all(base.join("extension-integral-2.9.0")).unwrap();
        let primera = instalar_en_el_lugar(Edicion::Integral, &base).unwrap();
        assert_eq!(primera.cambio, Cambio::Nueva);
        assert_eq!(primera.anteriores, [base.join("extension-integral-2.9.0")], "se avisa de la carpeta con versión en el nombre");
        assert_eq!(instalar_en_el_lugar(Edicion::Integral, &base).unwrap().cambio, Cambio::SinCambios);
        // Una copia vieja: otro manifest, un archivo alterado y uno que ya no existe.
        std::fs::write(primera.carpeta.join("background.js"), "codigo viejo").unwrap();
        std::fs::write(primera.carpeta.join("restos.js"), "ya no va").unwrap();
        let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(primera.carpeta.join("manifest.json")).unwrap()).unwrap();
        manifest["version"] = "2.8.0".into();
        std::fs::write(primera.carpeta.join("manifest.json"), manifest.to_string()).unwrap();
        let segunda = instalar_en_el_lugar(Edicion::Integral, &base).unwrap();
        assert_eq!(segunda.carpeta, primera.carpeta, "la carpeta no cambia");
        assert_eq!(segunda.cambio, Cambio::Actualizada { de: "2.8.0".into(), a: "2.9.0".into() });
        assert!(!segunda.carpeta.join("restos.js").exists());
        assert_eq!(instalar_en_el_lugar(Edicion::Integral, &base).unwrap().cambio, Cambio::SinCambios);
        let _ = std::fs::remove_dir_all(&base);
    }
}
