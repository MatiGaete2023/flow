//! Huella de las reglas con que se calcula la propuesta: fuentes del motor y del dominio, reiteración, mapeo de
//! columnas, textos y umbrales.
//! Una sesión guardada con otra huella se recalcula al recuperarla (como `ruleset_version` en `capture_input.py`).

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn archivos(carpeta: &Path, salida: &mut Vec<PathBuf>) {
    let Ok(entradas) = std::fs::read_dir(carpeta) else { return };
    for entrada in entradas.flatten() {
        let ruta = entrada.path();
        if ruta.is_dir() {
            archivos(&ruta, salida);
        } else if ruta.extension().is_some_and(|x| x == "rs") {
            salida.push(ruta);
        }
    }
}

fn main() {
    let raiz = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut rutas = Vec::new();
    for crate_ in ["flow-motor", "flow-dominio"] {
        let src = raiz.join("..").join(crate_).join("src");
        println!("cargo:rerun-if-changed={}", src.display());
        archivos(&src, &mut rutas);
    }
    // Reiteración por bitácora y mapeo de columnas también deciden la propuesta.
    for fuente in ["flow-trabajo/src/cronologia.rs", "flow-tabular/src/columnas.rs"] {
        rutas.push(raiz.join("..").join(fuente));
    }
    for dato in ["textos.json", "umbrales.json"] {
        rutas.push(raiz.join("../../datos").join(dato));
    }
    rutas.sort();
    let mut h = Sha256::new();
    for ruta in &rutas {
        println!("cargo:rerun-if-changed={}", ruta.display());
        let nombre = ruta.strip_prefix(raiz.join("..")).unwrap_or(ruta).to_string_lossy().replace('\\', "/");
        h.update(nombre.as_bytes());
        h.update([0]);
        // Fin de línea normalizado: la misma fuente en Windows y Linux da la misma huella.
        let texto = std::fs::read(ruta).unwrap_or_default();
        h.update(String::from_utf8_lossy(&texto).replace("\r\n", "\n").as_bytes());
        h.update([0]);
    }
    let huella: String = h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect();
    println!("cargo:rustc-env=FLOW_HUELLA_MOTOR={huella}");
}
