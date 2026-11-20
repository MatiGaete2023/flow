//! Entrada de prueba que usa el mismo lector y exportador que la ventana.
use flow_productos::revision;
use flow_trabajo::paquete;
use std::{path::Path, sync::atomic::AtomicBool};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argumentos: Vec<_> = std::env::args().skip(1).collect();
    let [paquete_zip, salida, desde, hasta, corte] = argumentos.as_slice() else {
        return Err("Uso: contrastar_revision_python ZIP DIRECTORIO DESDE HASTA CORTE".into());
    };
    let datos = std::fs::read(paquete_zip)?;
    let informe = paquete::revisar_bitacoras(&datos, desde.parse()?, hasta.parse()?, corte.parse()?)?;
    revision::exportar(&informe, None, Path::new(salida), &AtomicBool::new(false))?;
    Ok(())
}
