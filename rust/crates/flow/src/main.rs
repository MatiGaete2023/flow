//! Flow: lee RUS, propone observaciones y prepara Excel, Word y borradores.
//! No escribe en RUS ni SATURNO y no envía correos.

mod cli;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(cli::ejecutar(&args));
}
