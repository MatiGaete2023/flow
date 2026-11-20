use std::{path::Path, process::ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let Some(ruta) = args.next().filter(|_| args.next().is_none()) else {
        eprintln!("Uso: verificar_paquete <captura.zip>");
        return ExitCode::from(2);
    };
    match flow_rust::captura::leer_archivo(Path::new(&ruta))
        .and_then(|bytes| flow_rust::captura::leer_captura(&bytes))
    {
        Ok(captura) => {
            println!(
                "CSMP_CAPTURE {} validado, {} registros.",
                captura.manifiesto.schema_version,
                captura.filas("registros").count(),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
