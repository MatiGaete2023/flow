//! Flow: lee RUS, propone observaciones y prepara Excel, Word y borradores.
//! No escribe en RUS ni SATURNO y no envía correos.
//!
//! Sin argumentos (o con un Excel como único argumento) abre la ventana; con un comando, trabaja por consola.

#![cfg_attr(windows, windows_subsystem = "windows")]

mod anonimo_ui;
mod captura_ui;
mod cli;
mod meta4_ui;
mod cronologia_ui;
mod ui;

#[cfg(windows)]
extern "system" {
    fn AttachConsole(proceso: u32) -> i32;
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let archivo = match args.as_slice() {
        [] => None,
        [unico] if std::path::Path::new(unico).is_file() => Some(std::path::PathBuf::from(unico)),
        _ => {
            // Programa de ventana: para escribir en la consola que lo llamó hay que adjuntarse a ella.
            #[cfg(windows)]
            unsafe {
                AttachConsole(u32::MAX);
            }
            std::process::exit(cli::ejecutar(&args));
        }
    };
    std::process::exit(ui::abrir(archivo));
}
