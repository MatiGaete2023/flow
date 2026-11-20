use std::{path::Path, process::ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let Some(ruta) = args.next().filter(|_| args.next().is_none()) else {
        eprintln!("Uso: verificar_paquete <captura.zip>");
        return ExitCode::from(2);
    };
    match flow_rust::captura::verificar_archivo(Path::new(&ruta)) {
        Ok(m) => {
            println!(
                "Estructura ZIP y huellas correctas para CSMP_CAPTURE {}.",
                m.schema_version
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
