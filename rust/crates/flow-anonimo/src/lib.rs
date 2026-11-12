//! Anonimización local en dos pasadas. No publica una copia hasta revisar su contenido y nunca sobrescribe.
pub mod archivos;
pub mod calendario;
pub mod documentos;
pub mod formatos;
mod limites;
pub mod lote;
pub mod texto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modo {
    Estricto,
    Conservador,
}
#[derive(Clone)]
pub struct Opciones {
    pub modo: Modo,
    pub desplazar_dias: i64,
    pub generalizar_nacimiento: bool,
    pub semilla: String,
    pub rechazar_no_admitidos: bool,
}
impl Default for Opciones {
    fn default() -> Self {
        Self {
            modo: Modo::Estricto,
            desplazar_dias: 0,
            generalizar_nacimiento: false,
            semilla: String::new(),
            rechazar_no_admitidos: false,
        }
    }
}
pub const MAX_ARCHIVO: usize = 80 * 1024 * 1024;
pub const OMITIDO: &str = "[texto omitido]";

pub struct Salida {
    pub(crate) datos: Vec<u8>,
    pub(crate) extension: String,
    pub(crate) avisos: Vec<String>,
}
impl Salida {
    pub fn datos(&self) -> &[u8] {
        &self.datos
    }
    pub fn extension(&self) -> &str {
        &self.extension
    }
    pub fn avisos(&self) -> &[String] {
        &self.avisos
    }
}

pub fn publicar(carpeta: &std::path::Path, salida: &Salida) -> Result<std::path::PathBuf, String> {
    use std::io::Write;
    if salida.datos.len() > MAX_ARCHIVO
        || !["xlsx", "csv", "tsv", "json", "jsonl", "txt", "md", "log", "html", "ics", "docx", "zip"].contains(&salida.extension.as_str())
    {
        return Err("La salida excede el formato o tamaño permitidos.".into());
    }
    // Nombre independiente del original; abrir en exclusivo impide sobrescritura incluso entre procesos.
    for _ in 0..100 {
        let mut azar = [0u8; 8];
        getrandom::fill(&mut azar).map_err(|_| "No se pudo crear un nombre de salida seguro.")?;
        let nombre = azar.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let ruta = carpeta.join(format!("anonimizado_{nombre}.{}", salida.extension));
        let temporal = carpeta.join(format!(".anon-{nombre}.tmp"));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&temporal) {
            Ok(mut f) => {
                if f.write_all(&salida.datos).and_then(|_| f.sync_all()).is_err() {
                    drop(f);
                    let _ = std::fs::remove_file(&temporal);
                    return Err("No se pudo preparar la copia; no se conserva un archivo parcial.".into());
                }
                drop(f);
                let resultado = std::fs::hard_link(&temporal, &ruta);
                let _ = std::fs::remove_file(&temporal);
                match resultado {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(_) => {
                        let mut f = std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&ruta)
                            .map_err(|_| "El destino no permite publicar una copia sin sobrescribir.")?;
                        if f.write_all(&salida.datos).and_then(|_| f.sync_all()).is_err() {
                            drop(f);
                            let _ = std::fs::remove_file(&ruta);
                            return Err("No se pudo publicar la copia; no se conserva un archivo parcial.".into());
                        }
                    }
                }
                return Ok(ruta);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("No se pudo crear la copia en la carpeta elegida.".into()),
        }
    }
    Err("No se encontró un nombre de salida disponible.".into())
}
