//! Datos locales del usuario: sesiones de trabajo y configuración. Nada sale del equipo.
//! Retención (decisión D4): 60 días y hasta 10 sesiones anteriores.

use crate::Trabajo;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const DIAS_RETENCION: u64 = 60;
pub const SESIONES_MAXIMAS: usize = 10;

/// Carpeta de datos: `FLOW_DATOS` si está definida; en Windows `%APPDATA%\Flow`; si no, `~/.local/share/flow`.
pub fn carpeta_datos() -> PathBuf {
    if let Some(ruta) = std::env::var_os("FLOW_DATOS").filter(|v| !v.is_empty()) {
        return PathBuf::from(ruta);
    }
    if let Some(appdata) = std::env::var_os("APPDATA").filter(|v| !v.is_empty()) {
        return PathBuf::from(appdata).join("Flow");
    }
    let casa = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    casa.join(".local").join("share").join("flow")
}

fn sesiones(datos: &Path) -> PathBuf {
    datos.join("sesiones")
}

fn ruta_sesion(datos: &Path, trabajo: &Trabajo) -> PathBuf {
    sesiones(datos).join(format!(
        "{}_{}_{}.json",
        trabajo.corte.format("%Y%m%d"),
        trabajo.nombre(),
        &trabajo.sha256[..12.min(trabajo.sha256.len())]
    ))
}
/// Recupera la sesión del mismo origen y corte antes de activar un análisis recién abierto.
/// Un archivo dañado o una colisión de nombre detienen la apertura y conservan la copia anterior.
pub fn recuperar(datos: &Path, nuevo: &Trabajo) -> Result<Option<Trabajo>, String> {
    let ruta = ruta_sesion(datos, nuevo);
    if !ruta.try_exists().map_err(|_| "No se pudo comprobar la sesión anterior; no se reemplazará.")? {
        return Ok(None);
    }
    let anterior = cargar(&ruta)?;
    if anterior.sha256 != nuevo.sha256
        || anterior.corte != nuevo.corte
        || anterior.etapa != nuevo.etapa
        || anterior.fuente != nuevo.fuente
        || anterior.hoja != nuevo.hoja
        || anterior.cabeceras != nuevo.cabeceras
        || anterior.filas.iter().map(|f| &f.id).ne(nuevo.filas.iter().map(|f| &f.id))
    {
        return Err("Existe una sesión distinta con el mismo nombre. Consérvela antes de volver a analizar; no se reemplazó.".into());
    }
    Ok(Some(anterior))
}

/// Guarda el trabajo como sesión. El nombre depende del origen y del corte: volver a guardar reemplaza la misma.
pub fn guardar(datos: &Path, trabajo: &Trabajo) -> Result<PathBuf, String> {
    let carpeta = sesiones(datos);
    std::fs::create_dir_all(&carpeta).map_err(|e| format!("No se pudo crear la carpeta de sesiones: {e}"))?;
    let destino = ruta_sesion(datos, trabajo);
    let temporal = destino.with_extension("json.tmp");
    let texto = serde_json::to_string(trabajo).map_err(|e| e.to_string())?;
    std::fs::write(&temporal, texto).map_err(|e| format!("No se pudo guardar la sesión: {e}"))?;
    std::fs::rename(&temporal, &destino).map_err(|e| format!("No se pudo guardar la sesión: {e}"))?;
    Ok(destino)
}

pub fn cargar(ruta: &Path) -> Result<Trabajo, String> {
    let texto = std::fs::read_to_string(ruta).map_err(|e| format!("No se pudo leer la sesión: {e}"))?;
    serde_json::from_str(&texto).map_err(|e| format!("La sesión guardada no se puede leer: {e}"))
}

/// Sesiones guardadas, la más reciente primero.
pub fn listar(datos: &Path) -> Vec<(PathBuf, SystemTime)> {
    let mut salida: Vec<(PathBuf, SystemTime)> = std::fs::read_dir(sesiones(datos))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| e.metadata().and_then(|m| m.modified()).ok().map(|t| (e.path(), t)))
        .collect();
    salida.sort_by_key(|a| std::cmp::Reverse(a.1));
    salida
}

/// Borra las sesiones de más de 60 días y las que exceden las 10 más recientes. Devuelve cuántas borró.
pub fn aplicar_retencion(datos: &Path, ahora: SystemTime) -> usize {
    let limite = Duration::from_secs(DIAS_RETENCION * 24 * 3600);
    let mut borradas = 0;
    for (i, (ruta, modificada)) in listar(datos).into_iter().enumerate() {
        let vieja = ahora.duration_since(modificada).is_ok_and(|edad| edad > limite);
        if (vieja || i >= SESIONES_MAXIMAS) && std::fs::remove_file(&ruta).is_ok() {
            borradas += 1;
        }
    }
    borradas
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn retencion_por_cantidad_y_antiguedad() {
        let datos = std::env::temp_dir().join(format!("flow-almacen-{}", std::process::id()));
        let carpeta = sesiones(&datos);
        std::fs::create_dir_all(&carpeta).unwrap();
        for i in 0..13 {
            std::fs::write(carpeta.join(format!("s{i:02}.json")), "{}").unwrap();
        }
        assert_eq!(aplicar_retencion(&datos, SystemTime::now()), 3);
        assert_eq!(listar(&datos).len(), SESIONES_MAXIMAS);
        let futuro = SystemTime::now() + Duration::from_secs(61 * 24 * 3600);
        assert_eq!(aplicar_retencion(&datos, futuro), 10);
        assert!(listar(&datos).is_empty());
        std::fs::remove_dir_all(datos).unwrap();
    }
}
