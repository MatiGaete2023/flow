//! Actualización en el lugar (E-I2): una corrección llega como un `flow_rust.exe` nuevo y reemplaza al que corre, sin
//! reinstalar nada más. Flow vigila `%LOCALAPPDATA%\FlowRust\actualizaciones`; si ahí hay un `flow_rust.exe` distinto del
//! que está corriendo, avisa, y solo con el botón verifica su huella SHA-256 contra `SHA256.txt` (el que entrega la CI),
//! deja el programa en uso como `flow_rust.exe.anterior` (Windows permite renombrar un ejecutable en uso), copia el nuevo
//! en su lugar y se reinicia. La extensión de Chrome, los datos, la sesión y la configuración no se tocan: viven en otras
//! carpetas y la extensión solo cambia cuando cambia su propio código. Nada se descarga: el archivo lo deja la persona.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const NOMBRE: &str = "flow_rust.exe";

pub const AVISO_SINCRONIZANDO: &str =
    "La actualización de la carpeta adicional todavía se está sincronizando; vuelve a intentar en unos minutos.";

pub const PAUSA_SINCRONIZACION: std::time::Duration = std::time::Duration::from_secs(2);

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

/// Comprueba si el archivo está disponible localmente (no es un marcador de solo nube de OneDrive)
/// y se puede abrir para lectura.
#[cfg(windows)]
fn archivo_disponible_localmente(ruta: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    if let Ok(meta) = std::fs::metadata(ruta) {
        let attrs = meta.file_attributes();
        // FILE_ATTRIBUTE_OFFLINE (0x1000)
        // FILE_ATTRIBUTE_RECALL_ON_OPEN (0x00040000)
        // FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS (0x00400000)
        if attrs & (0x1000 | 0x0004_0000 | 0x0040_0000) != 0 {
            return false;
        }
    }
    std::fs::File::open(ruta).is_ok()
}

#[cfg(not(windows))]
fn archivo_disponible_localmente(ruta: &Path) -> bool {
    std::fs::File::open(ruta).is_ok()
}

/// Comprueba que el tamaño y la fecha de modificación del archivo permanezcan estables entre dos lecturas.
fn verificar_estabilidad(ruta: &Path, pausa: std::time::Duration) -> bool {
    let Ok(m1) = std::fs::metadata(ruta) else { return false };
    let len1 = m1.len();
    let mod1 = m1.modified().ok();
    if !pausa.is_zero() {
        std::thread::sleep(pausa);
    }
    let Ok(m2) = std::fs::metadata(ruta) else { return false };
    let len2 = m2.len();
    let mod2 = m2.modified().ok();
    len1 == len2 && mod1 == mod2
}

/// Copia `flow_rust.exe` y `SHA256.txt` desde la carpeta adicional a la carpeta fija
/// usando archivos temporales y renombrado atómico para no sobrescribir a medias.
fn copiar_de_adicional_a_fija(
    fija: &Path,
    exe_origen: &Path,
    sha_origen: &Path,
    huella_esperada: &str,
) -> Result<(), String> {
    std::fs::create_dir_all(fija)
        .map_err(|e| format!("No se pudo crear la carpeta fija de actualizaciones {}: {e}", fija.display()))?;

    let tmp_exe = fija.join("flow_rust.exe.tmp");
    let tmp_sha = fija.join("SHA256.txt.tmp");
    let _ = std::fs::remove_file(&tmp_exe);
    let _ = std::fs::remove_file(&tmp_sha);

    let limpiar = |e: String| {
        let _ = std::fs::remove_file(&tmp_exe);
        let _ = std::fs::remove_file(&tmp_sha);
        e
    };

    std::fs::copy(exe_origen, &tmp_exe)
        .map_err(|e| limpiar(format!("No se pudo copiar el ejecutable a {}: {e}", fija.display())))?;

    let h_copia = huella(&tmp_exe).map_err(|e| limpiar(format!("No se pudo verificar la copia: {e}")))?;
    if h_copia != huella_esperada {
        return Err(limpiar(AVISO_SINCRONIZANDO.into()));
    }

    std::fs::copy(sha_origen, &tmp_sha)
        .map_err(|e| limpiar(format!("No se pudo copiar SHA256.txt a {}: {e}", fija.display())))?;

    std::fs::rename(&tmp_exe, fija.join(NOMBRE))
        .map_err(|e| limpiar(format!("No se pudo mover el ejecutable en {}: {e}", fija.display())))?;
    std::fs::rename(&tmp_sha, fija.join("SHA256.txt"))
        .map_err(|e| limpiar(format!("No se pudo mover SHA256.txt en {}: {e}", fija.display())))?;

    Ok(())
}

/// Busca en la carpeta fija y, si está configurada, en la adicional con una pausa configurable para la verificación de sincronización.
pub fn buscar_con_adicional_pausa(
    fija: &Path,
    adicional: Option<&Path>,
    actual: &Path,
    pausa: std::time::Duration,
) -> Result<Option<Pendiente>, String> {
    if let Some(adicional) = adicional.filter(|p| !p.as_os_str().is_empty()) {
        let nuevo_adicional = adicional.join(NOMBRE);
        if nuevo_adicional.is_file() {
            let h_actual = huella(actual).ok();

            // (c) Verificar si el archivo se puede abrir en lectura exclusiva y no es marcador sólo en la nube
            if !archivo_disponible_localmente(&nuevo_adicional) {
                return Err(AVISO_SINCRONIZANDO.into());
            }

            // (b) Verificar si el tamaño o fecha de modificación cambió entre dos lecturas separadas por al menos 2s
            if !verificar_estabilidad(&nuevo_adicional, pausa) {
                return Err(AVISO_SINCRONIZANDO.into());
            }

            // Leer bytes para verificar si es ejecutable
            let bytes = std::fs::read(&nuevo_adicional).map_err(|_| AVISO_SINCRONIZANDO.to_string())?;
            if !bytes.starts_with(b"MZ") {
                return Err(format!("{NOMBRE} no parece un ejecutable de Windows; vuelve a copiarlo desde la entrega de la CI."));
            }
            let h_nuevo = crate::trabajo::hex(&Sha256::digest(&bytes));

            // Si es idéntico al programa que ya está corriendo, no hay nueva versión en la adicional
            if h_actual.as_deref() == Some(&h_nuevo) {
                return buscar(fija, actual);
            }

            // (a) Falta SHA256.txt o no coincide
            let sha_adicional = adicional.join("SHA256.txt");
            if !sha_adicional.is_file() {
                return Err(AVISO_SINCRONIZANDO.into());
            }
            let (esperada, _) = declarada(adicional);
            if esperada.as_deref() != Some(&h_nuevo) {
                return Err(AVISO_SINCRONIZANDO.into());
            }

            // Copiar a la fija (por temporal y renombrado, nunca sobrescribe a medias)
            copiar_de_adicional_a_fija(fija, &nuevo_adicional, &sha_adicional, &h_nuevo)?;
        }
    }

    buscar(fija, actual)
}

/// Busca en la carpeta fija (%LOCALAPPDATA%\FlowRust\actualizaciones) y, si está configurada, en la adicional.
pub fn buscar_con_adicional(
    fija: &Path,
    adicional: Option<&Path>,
    actual: &Path,
) -> Result<Option<Pendiente>, String> {
    buscar_con_adicional_pausa(fija, adicional, actual, PAUSA_SINCRONIZACION)
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

    #[test]
    fn la_actualizacion_de_la_carpeta_adicional_se_copia_y_aplica() {
        let base = carpeta_temporal("adicional-aplica");
        let (programa, fija, adicional) = (base.join("programa"), base.join("fija"), base.join("adicional"));
        std::fs::create_dir_all(&programa).unwrap();
        std::fs::create_dir_all(&fija).unwrap();
        std::fs::create_dir_all(&adicional).unwrap();

        let actual = programa.join(NOMBRE);
        exe(&actual, "version vieja");

        exe(&adicional.join(NOMBRE), "version nueva");
        let h = huella(&adicional.join(NOMBRE)).unwrap();
        std::fs::write(adicional.join("SHA256.txt"), format!("{h}  flow_rust.exe\ncommit def5678\n")).unwrap();

        assert!(!fija.join(NOMBRE).exists());
        let pendiente = buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::ZERO)
            .unwrap()
            .expect("debe detectar la versión nueva de la carpeta adicional");

        assert_eq!((pendiente.huella.as_str(), pendiente.commit.as_str()), (h.as_str(), "def5678"));
        // Se copió a la fija antes de aplicar
        assert_eq!(std::fs::read(fija.join(NOMBRE)).unwrap(), b"MZversion nueva");
        assert!(fija.join("SHA256.txt").is_file());

        // Aplicar desde la fija
        let anterior = aplicar(&pendiente, &actual).unwrap();
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion nueva");
        assert_eq!(std::fs::read(&anterior).unwrap(), b"MZversion vieja");
        assert!(fija.join("aplicadas").is_dir());

        // Una búsqueda posterior ve que el programa en uso ya tiene esa versión y no vuelve a ofrecerla ni a copiarla
        assert_eq!(buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::ZERO).unwrap(), None);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn una_actualizacion_a_medio_sincronizar_no_se_aplica() {
        let base = carpeta_temporal("adicional-mediosync");
        let (programa, fija, adicional) = (base.join("programa"), base.join("fija"), base.join("adicional"));
        std::fs::create_dir_all(&programa).unwrap();
        std::fs::create_dir_all(&fija).unwrap();
        std::fs::create_dir_all(&adicional).unwrap();

        let actual = programa.join(NOMBRE);
        exe(&actual, "version vieja");

        exe(&adicional.join(NOMBRE), "version parcial");
        let h = huella(&adicional.join(NOMBRE)).unwrap();
        std::fs::write(adicional.join("SHA256.txt"), format!("{h}  flow_rust.exe\n")).unwrap();

        // Simular que el tamaño cambia durante la verificación (entre dos lecturas)
        let exe_adicional = adicional.join(NOMBRE);
        let h_thread = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let mut f = std::fs::OpenOptions::new().append(true).open(&exe_adicional).unwrap();
            use std::io::Write;
            let _ = f.write_all(b"mas datos que cambian tamano");
        });

        let res = buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::from_millis(150));
        h_thread.join().unwrap();

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), AVISO_SINCRONIZANDO);
        assert!(!fija.join(NOMBRE).exists(), "no debe copiarse a la fija si está a medio sincronizar");
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn sin_sha256_en_la_carpeta_adicional_no_se_aplica() {
        let base = carpeta_temporal("adicional-sinsha");
        let (programa, fija, adicional) = (base.join("programa"), base.join("fija"), base.join("adicional"));
        std::fs::create_dir_all(&programa).unwrap();
        std::fs::create_dir_all(&fija).unwrap();
        std::fs::create_dir_all(&adicional).unwrap();

        let actual = programa.join(NOMBRE);
        exe(&actual, "version vieja");

        exe(&adicional.join(NOMBRE), "version nueva");
        // No se crea SHA256.txt en la carpeta adicional

        let res = buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::ZERO);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), AVISO_SINCRONIZANDO);
        assert!(!fija.join(NOMBRE).exists(), "no debe copiarse a la fija si falta SHA256.txt");
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");

        // Caso con SHA256.txt que no coincide
        std::fs::write(adicional.join("SHA256.txt"), format!("{}  flow_rust.exe\n", "1".repeat(64))).unwrap();
        let res2 = buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::ZERO);
        assert!(res2.is_err());
        assert_eq!(res2.unwrap_err(), AVISO_SINCRONIZANDO);
        assert!(!fija.join(NOMBRE).exists());

        let _ = std::fs::remove_dir_all(&base);
    }
}
