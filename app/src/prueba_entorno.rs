//! Ayuda de pruebas que dependen de variables de entorno (E-L).
//!
//! Sin la variable la prueba debe fallar al pedirse con `--include-ignored`; nunca terminar con `return` en silencio.

use std::ffi::OsString;

/// Devuelve la variable o falla con el mensaje único de las pruebas condicionadas. `FLOW_CASOS`, `FLOW_FIXTURES` y
/// `FLOW_REFERENCIA` tienen ruta por omisión en el repositorio (E-V2): la variable solo la reemplaza.
pub(crate) fn requerida(nombre: &str) -> OsString {
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let defecto = match nombre {
        "FLOW_CASOS" => Some(raiz.join("../rust/casos")),
        "FLOW_FIXTURES" => Some(raiz.join("../rust/crates/flow-tabular/tests/datos")),
        "FLOW_REFERENCIA" => Some(raiz.join("..")),
        _ => None,
    };
    std::env::var_os(nombre).or_else(|| defecto.map(OsString::from)).unwrap_or_else(|| {
        panic!("{nombre} es obligatoria para esta prueba (ver docs/migracion_python_rust/VERIFICACION.md)")
    })
}

/// Perfil aislado de las pruebas de interfaz (E-D): `APPDATA` y `LOCALAPPDATA` apuntan a una carpeta temporal antes de
/// que la prueba lea o escriba el perfil, así nunca se toca el perfil real del usuario. El candado serializa las pruebas
/// que lo usan; la carpeta se fija una sola vez por proceso porque las variables de entorno son globales.
#[allow(dead_code)]
pub(crate) fn perfil_aislado() -> (std::sync::MutexGuard<'static, ()>, std::path::PathBuf) {
    use std::sync::{Mutex, OnceLock};
    static CANDADO: Mutex<()> = Mutex::new(());
    static CARPETA: OnceLock<std::path::PathBuf> = OnceLock::new();
    let guardia = CANDADO.lock().unwrap_or_else(|e| e.into_inner());
    let carpeta = CARPETA
        .get_or_init(|| {
            let ruta = std::env::temp_dir().join(format!("flow-rust-perfil-pruebas-{}", std::process::id()));
            std::fs::create_dir_all(&ruta).unwrap();
            std::env::set_var("APPDATA", &ruta);
            std::env::set_var("LOCALAPPDATA", &ruta);
            ruta
        })
        .clone();
    let _ = std::fs::remove_dir_all(carpeta.join("FlowRust"));
    (guardia, carpeta)
}
