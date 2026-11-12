//! Archivos y carpetas en una sesión común. Las copias se publican solo tras verificarlas todas.
use crate::{formatos, texto::Sesion, Opciones, Salida, MAX_ARCHIVO};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Debug)]
pub struct Avance {
    pub fase: &'static str,
    pub completados: usize,
    pub total: usize,
}
#[derive(Serialize)]
pub struct Informe {
    pub version: u32,
    pub modo: String,
    pub archivos: Vec<String>,
    pub excluidos: usize,
    pub avisos: Vec<String>,
    pub revision_manual: String,
}
pub struct Resultado {
    pub carpeta: PathBuf,
    pub informe: Informe,
}
fn comprobar(cancelar: &AtomicBool) -> Result<(), String> {
    if cancelar.load(Ordering::Relaxed) {
        Err("Anonimización cancelada; no se publicaron copias parciales.".into())
    } else {
        Ok(())
    }
}
fn listar(
    p: &Path,
    archivos: &mut BTreeSet<PathBuf>,
    dirs: &mut BTreeSet<PathBuf>,
    nivel: usize,
    cancelar: &AtomicBool,
) -> Result<(), String> {
    comprobar(cancelar)?;
    if nivel > 32 {
        return Err("La carpeta supera la profundidad permitida.".into());
    }
    let meta = std::fs::symlink_metadata(p).map_err(|_| "No se pudo leer una de las entradas elegidas.")?;
    if meta.file_type().is_symlink() {
        return Err("No se siguen enlaces simbólicos; elige los archivos directamente.".into());
    }
    let real = p.canonicalize().map_err(|_| "No se pudo resolver una entrada elegida.")?;
    if meta.is_dir() {
        if !dirs.insert(real.clone()) {
            return Ok(());
        }
        if dirs.len() > 4000 {
            return Err("Demasiadas carpetas en la selección.".into());
        }
        for hijo in std::fs::read_dir(real).map_err(|_| "No se pudo enumerar una carpeta.")? {
            listar(&hijo.map_err(|_| "No se pudo enumerar una entrada.")?.path(), archivos, dirs, nivel + 1, cancelar)?;
        }
    } else if meta.is_file() {
        archivos.insert(real);
        if archivos.len() > 4000 {
            return Err("La selección supera 4.000 archivos.".into());
        }
    } else {
        return Err("La selección contiene un objeto que no es archivo o carpeta.".into());
    }
    Ok(())
}
fn nueva_carpeta(destino: &Path) -> Result<PathBuf, String> {
    for _ in 0..100 {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).map_err(|_| "No se pudo preparar la salida.")?;
        let p = destino.join(format!("Anonimizados_{}", bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()));
        match std::fs::create_dir(&p) {
            Ok(()) => return Ok(p),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("No se pudo crear una carpeta de resultados en el destino.".into()),
        }
    }
    Err("No se encontró un nombre de carpeta disponible.".into())
}
pub fn ejecutar(
    entradas: &[PathBuf],
    destino: &Path,
    opciones: Opciones,
    cancelar: &AtomicBool,
    mut progreso: impl FnMut(Avance),
) -> Result<Resultado, String> {
    if !destino.is_dir() {
        return Err("El destino debe ser una carpeta existente.".into());
    }
    let (mut archivos, mut dirs) = (BTreeSet::new(), BTreeSet::new());
    for p in entradas {
        listar(p, &mut archivos, &mut dirs, 0, cancelar)?;
    }
    if archivos.is_empty() {
        return Err("No hay archivos para anonimizar.".into());
    }
    let total = archivos.len();
    let mut cargados = vec![];
    let mut excluidos = 0;
    let mut presupuesto = 200 * 1024 * 1024usize;
    for (i, p) in archivos.into_iter().enumerate() {
        comprobar(cancelar)?;
        progreso(Avance { fase: "Leyendo", completados: i, total });
        let f = std::fs::File::open(&p).map_err(|_| "No se pudo leer un archivo seleccionado.")?;
        let mut raw = vec![];
        f.take((MAX_ARCHIVO + 1) as u64).read_to_end(&mut raw).map_err(|_| "No se pudo completar la lectura.")?;
        presupuesto = presupuesto.checked_sub(raw.len()).ok_or("La selección supera 200 MiB; divídela en grupos menores.")?;
        if raw.len() > MAX_ARCHIVO {
            return Err("Un archivo supera 80 MiB; no se procesa el lote.".into());
        }
        let nombre = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        match formatos::cargar(nombre, &raw) {
            Ok(e) => cargados.push(e),
            Err(_) => {
                if opciones.rechazar_no_admitidos {
                    return Err("Una entrada no es admitida o está dañada; no se publicó el lote.".into());
                }
                excluidos += 1;
            }
        }
    }
    if cargados.is_empty() {
        return Err("Ningún archivo seleccionado es admitido.".into());
    }
    let modo = format!("{:?}", opciones.modo);
    let mut sesion = Sesion::nueva(opciones)?;
    for (i, e) in cargados.iter().enumerate() {
        comprobar(cancelar)?;
        progreso(Avance { fase: "Reconociendo identidades", completados: i, total: cargados.len() });
        formatos::recolectar(&mut sesion, e)?;
    }
    let mut salidas: Vec<Salida> = vec![];
    let mut bytes = 0usize;
    let mut avisos = BTreeSet::new();
    for (i, e) in cargados.iter().enumerate() {
        comprobar(cancelar)?;
        progreso(Avance { fase: "Anonimizando y verificando", completados: i, total: cargados.len() });
        let s = formatos::transformar(&mut sesion, e)?;
        bytes += s.datos().len();
        if bytes > 200 * 1024 * 1024 {
            return Err("Las copias superan el tamaño permitido; divide la selección.".into());
        }
        avisos.extend(s.avisos().iter().cloned());
        salidas.push(s);
    }
    comprobar(cancelar)?;
    let carpeta = nueva_carpeta(destino)?;
    let publicacion = (|| {
        let mut nombres = vec![];
        for (i, s) in salidas.iter().enumerate() {
            comprobar(cancelar)?;
            progreso(Avance { fase: "Guardando copias", completados: i, total: salidas.len() });
            let p = crate::publicar(&carpeta, s)?;
            nombres.push(p.file_name().unwrap().to_string_lossy().into_owned());
        }
        let informe=Informe{version:1,modo,archivos:nombres,excluidos,avisos:avisos.into_iter().collect(),revision_manual:"Revise las copias y el informe antes de compartir. La revisión automática no garantiza anonimato absoluto. Los originales no se modificaron.".into()};
        let raw = serde_json::to_vec_pretty(&informe).map_err(|_| "No se pudo generar el informe.")?;
        std::fs::write(carpeta.join("INFORME.json"), raw).map_err(|_| "No se pudo guardar el informe.")?;
        comprobar(cancelar)?;
        Ok(informe)
    })();
    match publicacion {
        Ok(informe) => {
            progreso(Avance { fase: "Terminado", completados: salidas.len(), total: salidas.len() });
            Ok(Resultado { carpeta, informe })
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(carpeta);
            Err(e)
        }
    }
}
