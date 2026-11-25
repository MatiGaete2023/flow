//! Referencias locales equivalentes a `descargador/archivo_local.py`.

use super::paquete::{self, Lote};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Bitacoras,
    Conjunto,
    Descarga,
}

impl Tipo {
    pub fn etiqueta(self) -> &'static str {
        match self {
            Self::Bitacoras => "Bitácoras",
            Self::Conjunto => "Conjunto",
            Self::Descarga => "Descarga",
        }
    }
}

const TIPO_INVALIDO: &str =
    "La carpeta debe conservar resumen.json, flujo.json o bitacoras.json y sus originales.";
const INCOMPLETO: &str =
    "El conjunto está incompleto. Permite resultados incompletos para conservar su alcance explícito.";
const CARPETA_FASE_AUSENTE: &str = "Falta la carpeta de fuentes de la fase.";
const FUERA_DEL_CONJUNTO: &str = "La fuente de recuperación está fuera del flujo.";

pub fn tipo(carpeta: &Path) -> Result<Tipo, String> {
    for (nombre, tipo) in [
        ("bitacoras.json", Tipo::Bitacoras),
        ("flujo.json", Tipo::Conjunto),
        ("resumen.json", Tipo::Descarga),
    ] {
        if carpeta.join(nombre).is_file() {
            return Ok(tipo);
        }
    }
    Err(TIPO_INVALIDO.into())
}

fn ruta_relativa_normalizada(nombre: &str) -> Result<PathBuf, String> {
    let texto = nombre.replace('\\', "/");
    let ruta = Path::new(&texto);
    let mut salida = PathBuf::new();
    for parte in ruta.components() {
        match parte {
            Component::CurDir => {}
            Component::ParentDir => {
                if !salida.pop() {
                    return Err(FUERA_DEL_CONJUNTO.into());
                }
            }
            Component::Normal(_) => salida.push(parte.as_os_str()),
            Component::Prefix(_) | Component::RootDir => return Err(FUERA_DEL_CONJUNTO.into()),
        }
    }
    Ok(salida)
}

fn resolver_tolerante(ruta: &Path) -> Result<PathBuf, String> {
    let mut existente = ruta.to_path_buf();
    let mut sufijos = Vec::new();
    while !existente.exists() {
        let nombre = existente.file_name().ok_or_else(|| FUERA_DEL_CONJUNTO.to_string())?;
        sufijos.push(nombre.to_os_string());
        existente = existente.parent().ok_or_else(|| FUERA_DEL_CONJUNTO.to_string())?.to_path_buf();
    }
    let mut resuelta = std::fs::canonicalize(existente).map_err(|_| FUERA_DEL_CONJUNTO.to_string())?;
    for sufijo in sufijos.iter().rev() {
        resuelta.push(sufijo);
    }
    Ok(resuelta)
}

fn ruta_fase(raiz: &Path, nombre: &Value) -> Result<PathBuf, String> {
    let Some(nombre) = nombre.as_str().filter(|s| !s.is_empty()) else {
        return Err(CARPETA_FASE_AUSENTE.into());
    };
    // `Path.resolve()` de Python interpreta rutas POSIX incluso en Windows. También rechazamos
    // raíces Windows cuando se ejecuta en Linux, donde PathBuf por sí solo las vería relativas.
    let normalizado = nombre.replace('\\', "/");
    if normalizado.starts_with('/')
        || normalizado.starts_with("//")
        || normalizado.as_bytes().get(1) == Some(&b':')
    {
        return Err(FUERA_DEL_CONJUNTO.into());
    }
    let raiz = std::fs::canonicalize(raiz).map_err(|_| FUERA_DEL_CONJUNTO.to_string())?;
    let relativa = ruta_relativa_normalizada(&normalizado)?;
    if relativa.as_os_str().is_empty() {
        return Err(FUERA_DEL_CONJUNTO.into());
    }
    let lexical = raiz.join(&relativa);
    if lexical == raiz || !lexical.starts_with(&raiz) {
        return Err(FUERA_DEL_CONJUNTO.into());
    }
    // Canonicalize existing targets to reproduce Path.resolve for symlinks and Windows aliases
    // such as RUNNER~1. For a missing phase, retain the validated lexical target as Python does.
    let real = resolver_tolerante(&lexical)?;
    if real == raiz || !real.starts_with(&raiz) {
        return Err(FUERA_DEL_CONJUNTO.into());
    }
    Ok(real)
}

pub fn fases_conjunto(carpeta: &Path, parcial: bool) -> Result<Vec<PathBuf>, String> {
    let raiz = std::fs::canonicalize(carpeta).map_err(|e| format!("No se pudo abrir el lote: {e}"))?;
    let ruta = raiz.join("flujo.json");
    let datos = std::fs::read(&ruta).map_err(|_| "Falta flujo.json.".to_string())?;
    if datos.len() > 4 * 1024 * 1024 {
        return Err("El resumen excede el límite de lectura.".into());
    }
    let flujo: Value = serde_json::from_slice(&datos).map_err(|_| "Resumen JSON inválido.".to_string())?;
    if !flujo.is_object() {
        return Err("Resumen JSON inválido.".into());
    }
    let estado = flujo["estado"].as_str().unwrap_or("");
    if !parcial && !matches!(estado, "PREPARADO_SIN_CONSULTA_FIRMAS" | "VALIDADA" | "COMPLETA") {
        return Err(INCOMPLETO.into());
    }
    let mut salida = Vec::new();
    if let Some(fases) = flujo["fases"].as_object() {
        for fase in fases.values() {
            let valor = &fase["carpeta"];
            let tiene_valor = match valor {
                Value::Null => false,
                Value::Bool(v) => *v,
                Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
                Value::String(s) => !s.is_empty(),
                Value::Array(_) | Value::Object(_) => true,
            };
            if tiene_valor {
                salida.push(ruta_fase(&raiz, valor)?);
            }
        }
    }
    Ok(salida)
}

pub fn cargar_fuentes(carpetas: &[PathBuf], parcial: bool) -> Result<Vec<Lote>, String> {
    let mut resultado = Vec::new();
    let mut vistas = HashSet::new();
    for carpeta in carpetas {
        let carpeta = std::fs::canonicalize(carpeta).unwrap_or_else(|_| carpeta.clone());
        match tipo(&carpeta)? {
            Tipo::Bitacoras => continue,
            Tipo::Conjunto => {
                for fase in fases_conjunto(&carpeta, parcial)? {
                    let fase = std::fs::canonicalize(&fase).unwrap_or_else(|_| fase.clone());
                    if vistas.insert(fase.clone()) {
                        resultado.push(paquete::cargar_lote_con(&fase, parcial)?);
                    }
                }
            }
            Tipo::Descarga => {
                if vistas.insert(carpeta.clone()) {
                    resultado.push(paquete::cargar_lote_con(&carpeta, parcial)?);
                }
            }
        }
    }
    if resultado.is_empty() {
        return Err("Añade al menos una descarga de hojas RUS para crear el paquete del Asistente.".into());
    }
    Ok(resultado)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use base64::Engine;
    use serde_json::{json, Map};

    fn reconstruir(destino: &Path, archivos: &Value) {
        for (ruta, contenido) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(contenido.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    fn ruta_relativa(base: &Path, ruta: &Path) -> String {
        ruta.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/")
    }

    fn seleccionar(base: &Path, caso: &Value) -> Vec<PathBuf> {
        caso["seleccion"].as_array().map(|v| v.iter().map(|p| base.join(p.as_str().unwrap())).collect())
            .unwrap_or_else(|| vec![base.join("fuente")])
    }

    fn contenido_tablas(zip_path: &Path) -> Result<Map<String, Value>, String> {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(zip_path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let mut salida = Map::new();
        for index in 0..zip.len() {
            let mut entrada = zip.by_index(index).map_err(|e| e.to_string())?;
            let nombre = entrada.name().to_string();
            let Some(tabla) = nombre.strip_prefix("data/").and_then(|s| s.strip_suffix(".csv")) else { continue };
            let mut lector = csv::Reader::from_reader(&mut entrada);
            let cabeceras: Vec<String> = lector.headers().map_err(|e| e.to_string())?.iter().map(str::to_owned).collect();
            let mut filas = Vec::new();
            for fila in lector.records() {
                let fila = fila.map_err(|e| e.to_string())?;
                let valores: Map<String, Value> = cabeceras.iter().zip(fila.iter()).map(|(k, v)| (k.clone(), json!(v))).collect();
                filas.push(Value::Object(valores));
            }
            if !filas.is_empty() {
                salida.insert(tabla.into(), Value::Array(filas));
            }
        }
        Ok(salida)
    }

    fn comparar_caso(caso: &Value) -> Result<(), String> {
        let base = std::env::temp_dir().join(format!("flow-es2-{}-{}", std::process::id(), caso["caso"].as_str().unwrap_or("x")));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        let base = std::fs::canonicalize(&base).map_err(|e| e.to_string())?;
        reconstruir(&base, &caso["archivos"]);
        let partial = caso["parcial"].as_bool().unwrap_or(false);
        let seleccionadas = seleccionar(&base, caso);
        let fuente = seleccionadas.first().cloned().unwrap_or_else(|| base.join("fuente"));
        match tipo(&fuente) {
            Ok(t) => {
                if let Some(expected) = caso["tipo"].as_str() {
                    if t.etiqueta() != expected { return Err(format!("{} tipo: {} != {expected}", caso["caso"], t.etiqueta())); }
                } else { return Err(format!("{} esperaba error de tipo", caso["caso"])); }
            }
            Err(error) => {
                if caso["tipo_error"].as_str() != Some(&error) { return Err(format!("{} tipo_error: {error}", caso["caso"])); }
            }
        }
        if caso["tipo"] == "Conjunto" && (caso.get("fases").is_some() || caso.get("fases_error").is_some()) {
            match fases_conjunto(&fuente, partial) {
                Ok(fases) => {
                    let recibidas: Vec<String> = fases.iter().map(|p| ruta_relativa(&base, p)).collect();
                    if json!(recibidas) != caso["fases"] { return Err(format!("{} fases: {recibidas:?} != {}", caso["caso"], caso["fases"])); }
                }
                Err(error) => if caso["fases_error"].as_str() != Some(error.as_str()) { return Err(format!("{} fases_error: {error}", caso["caso"])); },
            }
        }
        if let Some(expected) = caso["direct_error"].as_str() {
            let error = ruta_fase(&fuente, &json!("")).unwrap_err();
            if error != expected { return Err(format!("{} direct_error: {error} != {expected}", caso["caso"])); }
        }
        if caso.get("fuentes").is_some() || caso.get("fuentes_error").is_some() {
            match cargar_fuentes(&seleccionadas, partial) {
                Ok(lotes) => {
                    let recibidas: Vec<String> = lotes.iter().map(|l| ruta_relativa(&base, &l.carpeta)).collect();
                    if json!(recibidas) != caso["fuentes"] { return Err(format!("{} fuentes: {recibidas:?} != {}", caso["caso"], caso["fuentes"])); }
                }
                Err(error) => if caso["fuentes_error"].as_str() != Some(error.as_str()) { return Err(format!("{} fuentes_error: {error}", caso["caso"])); },
            }
        }
        if caso.get("paquete").is_some() {
            let carpetas: Vec<PathBuf> = cargar_fuentes(&seleccionadas, partial)?.iter().map(|l| l.carpeta.clone()).collect();
            let diario = base.join("controles").join("bitacoras.json");
            let destino = base.join("captura.zip");
            paquete::exportar_con(&carpetas, &destino, &paquete::Opciones {
                bitacoras: vec![diario], emparejar: true, ..Default::default()
            })?;
            let tablas = contenido_tablas(&destino)?;
            for (tabla, esperado) in caso["paquete"].as_object().into_iter().flat_map(|m| m.iter()) {
                let obtenido = tablas.get(tabla).cloned().unwrap_or_else(|| json!([]));
                if obtenido != *esperado {
                    return Err(format!("{} tabla {tabla} distinta: Rust {obtenido:?}; Python {esperado:?}", caso["caso"]));
                }
            }
            if tablas["incidencias"] != caso["incidencias"] { return Err(format!("{} incidencias distintas", caso["caso"])); }
        }
        let _ = std::fs::remove_dir_all(&base);
        Ok(())
    }

    #[test]
    fn conjuntos_iguales_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/archivo_local.json")).unwrap();
        let casos = datos["casos"].as_array().unwrap();
        assert_eq!(casos.len(), 22, "el oráculo de archivo_local debe conservar sus 22 casos");
        let mut fallos = Vec::new();
        for caso in casos {
            if let Err(error) = comparar_caso(caso) { fallos.push(error); }
        }
        assert!(fallos.is_empty(), "{} casos difieren de Python:\n{}", fallos.len(), fallos.join("\n"));
    }

    #[test]
    fn oraculo_de_conjuntos_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/archivo_local.json")).unwrap();
        let casos = datos["casos"].as_array().unwrap();
        let mut ruta = casos.iter().find(|c| c["caso"] == "estado_validada").unwrap().clone();
        ruta["fases"][0] = json!("fuente/fase-alterada");
        assert!(comparar_caso(&ruta).is_err(), "un directorio de fase alterado debe fallar");
        let mut mensaje = casos.iter().find(|c| c["caso"] == "incompleto_sin_parcial").unwrap().clone();
        mensaje["fuentes_error"] = json!("mensaje alterado");
        assert!(comparar_caso(&mensaje).is_err(), "un mensaje alterado debe fallar");
        let mut tabla = casos.iter().find(|c| c["caso"] == "paquete_coincidencia_total").unwrap().clone();
        tabla["paquete"]["registros"][0]["court_code"] = json!("alterado");
        assert!(comparar_caso(&tabla).is_err(), "una tabla alterada debe fallar");
    }
}

