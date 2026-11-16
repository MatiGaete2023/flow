//! Datos locales del usuario: sesiones de trabajo y configuración. Nada sale del equipo.
//! Retención (decisión D4): 60 días y hasta 10 sesiones anteriores.

use crate::Trabajo;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const DIAS_RETENCION: u64 = 60;
pub const SESIONES_MAXIMAS: usize = 10;

/// Claves de `Trabajo::sesion` con productos preparados. Si cambian las reglas se conservan, como en Python: su
/// huella ya no coincide (`flow_productos::vigencia`), quedan obsoletos y hay que prepararlos de nuevo antes de
/// guardarlos o generarlos.
pub const PRODUCTOS_SESION: [&str; 3] = ["borradores", "proyectos", "seleccion_preparada"];

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
/// Recupera la sesión del mismo origen y corte antes de activar un análisis recién abierto. Devuelve si había una y,
/// si corresponde, un aviso para mostrar una vez (no se guarda en el trabajo, para no repetirlo en cada apertura).
///
/// - Calculada con las mismas reglas: reemplaza a `nuevo` tal como quedó.
/// - Con otras reglas (`version_reglas`): queda la propuesta recién calculada y se traen las ediciones de la
///   observación, las marcas de revisada del usuario y el estado de la sesión; los borradores y proyectos preparados
///   (`PRODUCTOS_SESION`) quedan obsoletos, con aviso.
/// - Ilegible (dañada o de un formato anterior): se aparta con otro nombre, sin borrarla, y se analiza de nuevo.
/// - Una colisión de nombre con otro origen detiene la apertura y conserva la copia anterior.
pub fn recuperar(datos: &Path, nuevo: &mut Trabajo) -> Result<(bool, Option<String>), String> {
    let ruta = ruta_sesion(datos, nuevo);
    if !ruta.try_exists().map_err(|_| "No se pudo comprobar la sesión anterior; no se reemplazará.")? {
        return Ok((false, None));
    }
    let anterior = match cargar(&ruta) {
        Ok(t) => t,
        Err(e) => {
            let apartada = apartar(&ruta)?;
            let aviso = format!("{e} Se analizó de nuevo; la sesión anterior se conserva sin cambios en {}.", apartada.display());
            return Ok((false, Some(aviso)));
        }
    };
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
    if anterior.version_reglas == nuevo.version_reglas {
        *nuevo = anterior;
        return Ok((true, None));
    }
    for (fila, previa) in nuevo.filas.iter_mut().zip(&anterior.filas) {
        // Una observación igual a la propuesta anterior era texto del motor: se reemplaza por la nueva.
        if previa.editada() {
            fila.observacion = previa.observacion.clone();
        }
        fila.revisada = previa.revisada;
    }
    let mut aviso =
        "La sesión guardada se calculó con otras reglas: se recalcularon las propuestas y se conservaron tus ediciones y marcas."
            .to_string();
    if let Some(serde_json::Value::Object(estado)) = anterior.sesion {
        let hay = PRODUCTOS_SESION.iter().any(|c| estado.get(*c).is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty())));
        if hay {
            aviso.push_str(" Los borradores y proyectos preparados quedan obsoletos: prepáralos de nuevo antes de guardarlos.");
        }
        nuevo.sesion = Some(serde_json::Value::Object(estado));
    }
    Ok((true, Some(aviso)))
}

/// Mueve una sesión ilegible a `<nombre>.json.ilegible` (o `.ilegible-2`…): fuera de `listar` y de la retención.
fn apartar(ruta: &Path) -> Result<PathBuf, String> {
    let base = ruta.with_extension("json.ilegible");
    let mut destino = base.clone();
    let mut n = 2;
    while destino.try_exists().unwrap_or(true) {
        if n > 100 {
            return Err("No se pudo apartar la sesión ilegible; se conserva y no se reemplazará.".into());
        }
        destino = PathBuf::from(format!("{}-{n}", base.display()));
        n += 1;
    }
    std::fs::rename(ruta, &destino).map_err(|_| "No se pudo apartar la sesión ilegible; se conserva y no se reemplazará.")?;
    Ok(destino)
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
    use flow_dominio::Celda;
    use flow_tabular::{Formato, Hoja, Libro};

    fn temporal(nombre: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "flow-almacen-{nombre}-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn analizado() -> Trabajo {
        let texto = |xs: &[&str]| xs.iter().map(|x| Celda::Texto(x.to_string())).collect::<Vec<_>>();
        let mut filas = vec![texto(&["RIT", "NOMBRE", "PROGRAMA", "TRIBUNAL", "T ESPERA"])];
        for i in 1..=3 {
            filas.push(texto(&[&format!("X-{i}-2099"), &format!("Persona Ficticia {i}"), "PRM Ficticio", "Laja", "200"]));
        }
        let libro = Libro {
            nombre: "espera_ficticia.xlsx".into(),
            sha256: "a".repeat(64),
            formato: Formato::Xlsx,
            hojas: vec![Hoja { nombre: "Espera".into(), filas }],
        };
        let tabla = flow_tabular::leer_tabla(&libro, None, None).unwrap();
        crate::analizar_tabla(&libro, &tabla, "2026-10-20".parse().unwrap(), &flow_motor::Config::default(), &Default::default())
    }

    #[test]
    fn otras_reglas_recalculan_y_conservan_solo_ediciones_y_marcas() {
        let datos = temporal("reglas");
        let mut viejo = analizado();
        // Sesión de antes del campo: sin versión, con propuestas de un motor anterior.
        viejo.version_reglas = String::new();
        for f in &mut viejo.filas {
            f.propuesta.observacion = "Texto ficticio del motor anterior.".into();
            f.observacion = f.propuesta.observacion.clone();
        }
        viejo.filas[0].observacion = "Edición ficticia del usuario.".into();
        viejo.filas[1].revisada = true;
        viejo.sesion = Some(serde_json::json!({
            "version": 2,
            "borradores": [{"asunto": "Borrador ficticio"}],
            "proyectos": [],
            "modalidades": ["Modalidad ficticia"],
        }));
        guardar(&datos, &viejo).unwrap();
        let mut nuevo = analizado();
        let recien = nuevo.clone();
        let (recuperada, aviso) = recuperar(&datos, &mut nuevo).unwrap();
        assert!(recuperada && aviso.is_some_and(|a| a.contains("otras reglas") && a.contains("quedan obsoletos")));
        // Los productos preparados se conservan (quedan obsoletos por su huella), como en Python.
        assert_eq!(nuevo.sesion, viejo.sesion);
        assert_eq!(nuevo.version_reglas, recien.version_reglas);
        assert_eq!(nuevo.filas[0].observacion, "Edición ficticia del usuario.");
        assert!(nuevo.filas[1].revisada && !nuevo.filas[0].revisada);
        assert_eq!(nuevo.filas[2].observacion, recien.filas[2].observacion);
        assert!(nuevo.filas.iter().zip(&recien.filas).all(|(a, b)| a.propuesta == b.propuesta));
        assert_eq!(nuevo.avisos, recien.avisos, "el aviso no queda guardado en el trabajo");
        // Con las mismas reglas se recupera tal cual y sin volver a avisar.
        guardar(&datos, &nuevo).unwrap();
        let mut otra = analizado();
        assert_eq!(recuperar(&datos, &mut otra).unwrap(), (true, None));
        assert_eq!(otra.sesion, nuevo.sesion, "con las mismas reglas la sesión vuelve tal cual");
        assert_eq!(otra.filas[0].observacion, "Edición ficticia del usuario.");
        assert_eq!(otra.avisos, recien.avisos);
        std::fs::remove_dir_all(datos).unwrap();
    }

    #[test]
    fn sesion_ilegible_se_aparta_sin_borrarla_y_no_bloquea() {
        let datos = temporal("ilegible");
        let trabajo = analizado();
        let ruta = guardar(&datos, &trabajo).unwrap();
        std::fs::write(&ruta, b"sesion incompleta ficticia").unwrap();
        let mut nuevo = analizado();
        let (recuperada, aviso) = recuperar(&datos, &mut nuevo).unwrap();
        assert!(!recuperada && aviso.is_some_and(|a| a.contains("se conserva sin cambios")));
        assert!(!ruta.exists());
        let apartada = ruta.with_extension("json.ilegible");
        assert_eq!(std::fs::read(&apartada).unwrap(), b"sesion incompleta ficticia");
        assert!(listar(&datos).is_empty(), "la copia apartada no entra en la retención");
        std::fs::remove_dir_all(datos).unwrap();
    }

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
