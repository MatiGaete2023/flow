//! Actualización en el lugar (E-I2): una corrección llega como un `flow_rust.exe` nuevo y reemplaza al que corre, sin
//! reinstalar nada más. Flow vigila `%LOCALAPPDATA%\FlowRust\actualizaciones`; si ahí hay un `flow_rust.exe` distinto del
//! que está corriendo, avisa, y solo con el botón verifica su huella SHA-256 contra `SHA256.txt` (el que entrega la CI),
//! deja el programa en uso como `flow_rust.exe.anterior` (Windows permite renombrar un ejecutable en uso), copia el nuevo
//! en su lugar y se reinicia. La extensión de Chrome, los datos, la sesión y la configuración no se tocan: viven en otras
//! carpetas y la extensión solo cambia cuando cambia su propio código. Nada se descarga: el archivo lo deja la persona.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const NOMBRE: &str = "flow_rust.exe";

/// Una versión nueva lista para aplicar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pendiente {
    pub ruta: PathBuf,
    pub huella: String,
    /// El commit que declara `SHA256.txt`, si lo trae.
    pub commit: String,
}

/// Carpeta donde se deja el ejecutable nuevo.
pub fn carpeta() -> PathBuf {
    flow_rust::descargador::extension::carpeta_base().join("actualizaciones")
}

pub fn huella(ruta: &Path) -> Result<String, String> {
    let bytes = std::fs::read(ruta).map_err(|e| format!("No se pudo leer {}: {e}", ruta.display()))?;
    Ok(crate::trabajo::hex(&Sha256::digest(bytes)))
}

/// `SHA256.txt` de la CI: «<huella>  flow_rust.exe (…)» y una línea «commit <sha>».
fn declarada(carpeta: &Path) -> (Option<String>, String) {
    let Ok(texto) = std::fs::read_to_string(carpeta.join("SHA256.txt")) else { return (None, String::new()) };
    let primera = texto.lines().next().and_then(|l| l.split_whitespace().next()).map(str::to_lowercase);
    let huella = primera.filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()));
    let commit = texto.lines().find_map(|l| l.strip_prefix("commit ")).map(|c| c.trim().to_string()).unwrap_or_default();
    (huella, commit)
}

/// La versión nueva que espera en `carpeta`, si la hay y difiere del programa que corre (`actual`).
pub fn buscar(carpeta: &Path, actual: &Path) -> Result<Option<Pendiente>, String> {
    let nuevo = carpeta.join(NOMBRE);
    if !nuevo.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(&nuevo).map_err(|e| format!("No se pudo leer la actualización: {e}"))?;
    if !bytes.starts_with(b"MZ") {
        return Err(format!("{NOMBRE} no parece un ejecutable de Windows; vuelve a copiarlo desde la entrega de la CI."));
    }
    let h = crate::trabajo::hex(&Sha256::digest(&bytes));
    if huella(actual).is_ok_and(|a| a == h) {
        return Ok(None);
    }
    let (esperada, commit) = declarada(carpeta);
    if esperada.is_some_and(|e| e != h) {
        return Err("La huella de la actualización no coincide con SHA256.txt: el archivo está incompleto o cambió. No se aplicó.".into());
    }
    Ok(Some(Pendiente { ruta: nuevo, huella: h, commit }))
}

fn anterior_de(actual: &Path) -> PathBuf {
    let nombre = actual.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| NOMBRE.into());
    actual.with_file_name(format!("{nombre}.anterior"))
}

/// La copia del programa anterior, si existe (para volver atrás).
pub fn anterior(actual: &Path) -> Option<PathBuf> {
    Some(anterior_de(actual)).filter(|p| p.is_file())
}

/// Reemplaza `actual` por la versión pendiente. Si algo falla antes de terminar, el programa queda como estaba.
pub fn aplicar(p: &Pendiente, actual: &Path) -> Result<PathBuf, String> {
    if huella(&p.ruta)? != p.huella {
        return Err("La actualización cambió después de verificarse; vuelve a buscarla.".into());
    }
    let anterior = anterior_de(actual);
    let _ = std::fs::remove_file(&anterior);
    let sin_permiso = |e: std::io::Error| {
        format!("No se pudo reemplazar el programa en {}: {e}. Mueve Flow a una carpeta propia, por ejemplo %LOCALAPPDATA%\\FlowRust.", actual.display())
    };
    std::fs::rename(actual, &anterior).map_err(sin_permiso)?;
    let restaurar = |motivo: String| {
        let _ = std::fs::remove_file(actual);
        let _ = std::fs::rename(&anterior, actual);
        motivo
    };
    if let Err(e) = std::fs::copy(&p.ruta, actual) {
        return Err(restaurar(format!("No se pudo copiar la actualización: {e}")));
    }
    if huella(actual).ok().as_deref() != Some(p.huella.as_str()) {
        return Err(restaurar("La copia del programa nuevo no coincide con la verificada; se conservó el anterior.".into()));
    }
    // La actualización ya aplicada se guarda aparte para que no se vuelva a ofrecer.
    if let Some(carpeta) = p.ruta.parent() {
        let aplicadas = carpeta.join("aplicadas");
        if std::fs::create_dir_all(&aplicadas).is_ok() {
            let corto = &p.huella[..8.min(p.huella.len())];
            let _ = std::fs::rename(&p.ruta, aplicadas.join(format!("flow_rust-{corto}.exe")));
            let _ = std::fs::rename(carpeta.join("SHA256.txt"), aplicadas.join(format!("SHA256-{corto}.txt")));
        }
    }
    Ok(anterior)
}

/// Intercambia el programa actual con la copia anterior.
pub fn volver(actual: &Path) -> Result<(), String> {
    let anterior = anterior_de(actual);
    if !anterior.is_file() {
        return Err("No hay una versión anterior guardada.".into());
    }
    let temporal = actual.with_file_name("flow_rust.exe.intercambio");
    let paso = |a: &Path, b: &Path| std::fs::rename(a, b).map_err(|e| format!("No se pudo volver a la versión anterior: {e}"));
    paso(actual, &temporal)?;
    if let Err(e) = paso(&anterior, actual) {
        let _ = std::fs::rename(&temporal, actual);
        return Err(e);
    }
    paso(&temporal, &anterior)
}

/// Abre el programa que quedó en `actual` (el nuevo) para que continúe el trabajo.
pub fn reiniciar(actual: &Path) -> Result<(), String> {
    std::process::Command::new(actual).spawn().map(|_| ()).map_err(|e| format!("No se pudo reiniciar Flow: {e}"))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn carpeta_temporal(nombre: &str) -> PathBuf {
        let c = std::env::temp_dir().join(format!("flow-actualizacion-{}-{nombre}", std::process::id()));
        let _ = std::fs::remove_dir_all(&c);
        std::fs::create_dir_all(&c).unwrap();
        c
    }

    fn exe(ruta: &Path, contenido: &str) {
        std::fs::write(ruta, format!("MZ{contenido}")).unwrap();
    }

    #[test]
    fn la_actualizacion_reemplaza_el_programa_y_deja_el_anterior() {
        let base = carpeta_temporal("aplica");
        let (programa, nuevas) = (base.join("programa"), base.join("actualizaciones"));
        std::fs::create_dir_all(&programa).unwrap();
        std::fs::create_dir_all(&nuevas).unwrap();
        let actual = programa.join(NOMBRE);
        exe(&actual, "version vieja");
        // Sin nada en la carpeta no hay aviso.
        assert_eq!(buscar(&nuevas, &actual).unwrap(), None);
        exe(&nuevas.join(NOMBRE), "version nueva");
        let h = huella(&nuevas.join(NOMBRE)).unwrap();
        std::fs::write(nuevas.join("SHA256.txt"), format!("{h}  flow_rust.exe (debug)\ncommit abc1234\n")).unwrap();
        let pendiente = buscar(&nuevas, &actual).unwrap().expect("hay una versión nueva");
        assert_eq!((pendiente.huella.as_str(), pendiente.commit.as_str()), (h.as_str(), "abc1234"));
        // Buscar no cambia nada: solo avisa.
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");
        let anterior = aplicar(&pendiente, &actual).unwrap();
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion nueva");
        assert_eq!(std::fs::read(&anterior).unwrap(), b"MZversion vieja");
        assert!(!nuevas.join(NOMBRE).exists(), "la actualización aplicada no se vuelve a ofrecer");
        assert!(nuevas.join("aplicadas").is_dir());
        assert_eq!(buscar(&nuevas, &actual).unwrap(), None);
        // Volver atrás intercambia los dos y se puede repetir.
        volver(&actual).unwrap();
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");
        volver(&actual).unwrap();
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion nueva");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn una_actualizacion_igual_incompleta_o_ajena_no_se_aplica() {
        let base = carpeta_temporal("rechaza");
        let nuevas = base.join("actualizaciones");
        std::fs::create_dir_all(&nuevas).unwrap();
        let actual = base.join(NOMBRE);
        exe(&actual, "version vieja");
        // Igual al programa que corre: está al día.
        exe(&nuevas.join(NOMBRE), "version vieja");
        assert_eq!(buscar(&nuevas, &actual).unwrap(), None);
        // La huella declarada no coincide: archivo incompleto o cambiado.
        exe(&nuevas.join(NOMBRE), "version nueva");
        std::fs::write(nuevas.join("SHA256.txt"), format!("{}  flow_rust.exe\n", "0".repeat(64))).unwrap();
        assert!(buscar(&nuevas, &actual).unwrap_err().contains("no coincide"));
        // No es un ejecutable.
        std::fs::remove_file(nuevas.join("SHA256.txt")).unwrap();
        std::fs::write(nuevas.join(NOMBRE), "texto").unwrap();
        assert!(buscar(&nuevas, &actual).unwrap_err().contains("ejecutable"));
        // Cambió entre la verificación y la aplicación: el programa queda como estaba.
        exe(&nuevas.join(NOMBRE), "version nueva");
        let pendiente = buscar(&nuevas, &actual).unwrap().unwrap();
        exe(&nuevas.join(NOMBRE), "otra cosa");
        assert!(aplicar(&pendiente, &actual).unwrap_err().contains("cambió"));
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");
        assert!(anterior(&actual).is_none());
        assert!(volver(&actual).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }
}
