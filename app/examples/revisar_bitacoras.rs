//! Utilidad de contraste del clasificador con los resultados de Flow Python.

use chrono::NaiveDate;
use flow_rust::captura::{leer_archivo, leer_captura, revision::Revision};
use serde_json::json;
use std::path::Path;
use std::process::ExitCode;

fn revisar() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [ruta, desde, hasta] = args.as_slice() else {
        return Err("Uso: revisar_bitacoras <captura.zip> <AAAA-MM-DD> <AAAA-MM-DD>".into());
    };
    let fecha = |s: &std::ffi::OsStr| -> Result<NaiveDate, String> {
        s.to_str()
            .and_then(|s| s.parse().ok())
            .ok_or("Fecha inválida.".into())
    };
    let (desde, hasta) = (fecha(desde)?, fecha(hasta)?);
    let captura = leer_captura(&leer_archivo(Path::new(ruta))?)?;
    let revision = Revision::nueva(&captura);
    let filas = captura.filas("registros").map(|r| {
        let resultado = revision.clasificar(r, desde, hasta)?;
        Ok(json!({"record_id": r.get("record_id"), "causa": r.get("cause_id"), "resultado": resultado}))
    }).collect::<Result<Vec<_>, String>>()?;
    println!("{}", json!(filas));
    Ok(())
}

fn main() -> ExitCode {
    match revisar() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
