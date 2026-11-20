//! Ingreso Histórico Protección: captura MANUAL de la tabla visible de la persona seleccionada (plan 18-nov-2026).
//! Espejo de `descargador/historico_manual.py` (mismo `historico.json`). No consulta nada ni sigue enlaces: el funcionario
//! abre la vista y la entrega. Sin prueba de pertenencia a la persona de la fila, la captura queda INCIERTA.

use flow_captura::lecturas::leer_historico;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Read, Write},
    path::Path,
};

pub const MANIFIESTO: &str = "historico.json";
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_CONTROL: usize = 32 * 1024 * 1024;

/// Persona seleccionada: `tribunal` (código), `person_id` (SITFA_PERSONA_RUS de la fila), `rut` y `nombre` (solo para comprobar
/// la pertenencia en el texto visible; no se guardan).
pub struct Persona<'a> {
    pub tribunal: &'a str,
    pub tribunal_nombre: &'a str,
    pub person_id: &'a str,
    pub rut: &'a str,
    pub nombre: &'a str,
    /// Dirección de la ventana, solo para leer el RUT que pide (no se guarda). Vacía si no se conoce.
    pub url: &'a str,
}

fn rut_limpio(t: &str) -> String {
    t.chars().filter(|c| c.is_ascii_digit() || matches!(c, 'k' | 'K')).collect::<String>().to_uppercase()
}

/// RUT (sin puntos ni guion) que la propia dirección de la vista pide (`RUT_Litigante` + `RUT_DV`), o vacío. Es la prueba más
/// fuerte de pertenencia: la ventana se abrió para esa persona.
pub fn rut_de_url(url: &str) -> String {
    let valor = |clave: &str| -> Option<&str> {
        let patron = format!("{clave}=");
        let inicio = url.find(&format!("?{patron}")).or_else(|| url.find(&format!("&{patron}")))? + 1 + patron.len();
        let fin = url[inicio..].find('&').map(|f| inicio + f).unwrap_or(url.len());
        Some(&url[inicio..fin])
    };
    match (valor("RUT_Litigante"), valor("RUT_DV")) {
        (Some(c), Some(d)) if (1..=9).contains(&c.len()) && c.bytes().all(|b| b.is_ascii_digit()) && d.len() == 1 && rut_limpio(d) == d.to_uppercase() => {
            format!("{c}{}", d.to_uppercase())
        }
        _ => String::new(),
    }
}

fn huella(datos: &[u8]) -> String {
    Sha256::digest(datos).iter().map(|b| format!("{b:02x}")).collect()
}

fn leer_acotado(ruta: &Path, limite: usize) -> std::io::Result<Vec<u8>> {
    let meta = fs::symlink_metadata(ruta)?;
    if !meta.is_file() || meta.len() > limite as u64 {
        return Err(ErrorKind::InvalidData.into());
    }
    let mut datos = Vec::new();
    File::open(ruta)?.take(limite as u64 + 1).read_to_end(&mut datos)?;
    if datos.len() > limite {
        return Err(ErrorKind::InvalidData.into());
    }
    Ok(datos)
}

fn estado(carpeta: &Path) -> Result<Value, String> {
    let error = || "Control de histórico inválido.".to_string();
    let datos = match leer_acotado(&carpeta.join(MANIFIESTO), MAX_CONTROL) {
        Ok(datos) => datos,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Ok(json!({"version": 1, "tipo": "HISTORICO_PROTECCION", "capturas": {}}));
        }
        Err(_) => return Err(error()),
    };
    let control: Value = serde_json::from_slice(&datos).map_err(|_| error())?;
    if control["tipo"] != "HISTORICO_PROTECCION"
        || control["version"] != 1
        || !control["capturas"].is_object()
    {
        return Err(error());
    }
    Ok(control)
}

fn respaldo_valido(carpeta: &Path, entrada: &Value) -> bool {
    let (Some(archivo), Some(sha)) = (entrada["archivo"].as_str(), entrada["sha256"].as_str()) else {
        return false;
    };
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit())
        || archivo != format!("historico-{}.html", &sha[..24])
    {
        return false;
    }
    let carpeta = carpeta.join("historico");
    fs::symlink_metadata(&carpeta).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        && leer_acotado(&carpeta.join(archivo), MAX_BYTES).is_ok_and(|datos| huella(&datos) == sha)
}

/// Guarda una captura manual y devuelve su entrada del manifiesto. Capturar de nuevo a la misma persona reemplaza la anterior.
pub fn capturar(carpeta: &Path, html: &[u8], persona: &Persona, capturada: &str) -> Result<Value, String> {
    if html.is_empty() || html.len() > MAX_BYTES {
        return Err("La vista del histórico está vacía o supera 8 MiB.".into());
    }
    if persona.tribunal.trim().is_empty() || persona.person_id.trim().is_empty() {
        return Err("Falta el tribunal o la persona seleccionada: no se puede asociar el histórico.".into());
    }
    let mut manifiesto = estado(carpeta)?;
    let sha = huella(html);
    let archivo = format!("historico-{}.html", &sha[..24]);
    let mut entrada = json!({"tribunal":persona.tribunal,"tribunal_nombre":persona.tribunal_nombre,"person_id":persona.person_id,
        "archivo":archivo,"sha256":sha,"leida":capturada,"origen":"MANUAL"});
    match leer_historico(html, "", capturada, &[persona.rut, persona.nombre]) {
        Ok(l) => {
            entrada["estado"] = json!(if l["vacio_explicito"] == true { "VACIA_COMPROBADA" } else { "LEIDA" });
            entrada["error"] = json!("");
            let (de_url, esperado) = (rut_de_url(persona.url), rut_limpio(persona.rut));
            entrada["ownership"] = if !de_url.is_empty() && !esperado.is_empty() {
                json!(if de_url == esperado { "VERIFICADA" } else { "INCIERTA" })
            } else {
                l["ownership"].clone()
            };
            entrada["filas"] = l["filas"].clone();
            entrada["advertencias"] = json!(l["advertencias"].as_array().map(|a| a.iter().map(|x| x["codigo"].clone()).collect::<Vec<_>>()).unwrap_or_default());
        }
        Err(e) => {
            entrada["estado"] = json!("FALLIDA");
            entrada["error"] = json!(e);
            entrada["ownership"] = json!("INCIERTA");
            entrada["filas"] = json!([]);
            entrada["advertencias"] = json!([]);
        }
    }
    let carpeta_html = carpeta.join("historico");
    fs::create_dir_all(&carpeta_html).map_err(|_| "No se pudo crear la carpeta del histórico.")?;
    if !fs::symlink_metadata(&carpeta_html).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink()) {
        return Err("Carpeta de respaldo de histórico inválida.".into());
    }
    let ruta = carpeta_html.join(&archivo);
    match OpenOptions::new().write(true).create_new(true).open(&ruta) {
        Ok(mut archivo) => archivo.write_all(html).map_err(|_| "No se pudo guardar la vista del histórico.")?,
        Err(e) if e.kind() == ErrorKind::AlreadyExists && respaldo_valido(carpeta, &entrada) => {}
        Err(_) => return Err("No se pudo guardar o verificar el respaldo del histórico.".into()),
    }
    manifiesto["capturas"][format!("{}|{}", persona.tribunal, persona.person_id)] = entrada.clone();
    let datos = serde_json::to_vec_pretty(&manifiesto).map_err(|_| "Manifiesto ilegible.")?;
    let temporal = carpeta.join(format!(
        ".historico-{}-{}.tmp",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let mut control = OpenOptions::new().write(true).create_new(true).open(&temporal)
        .map_err(|_| "No se pudo preparar el manifiesto del histórico.")?;
    let escrito = control.write_all(&datos).and_then(|()| control.sync_all());
    drop(control);
    if escrito.and_then(|()| fs::rename(&temporal, carpeta.join(MANIFIESTO))).is_err() {
        let _ = fs::remove_file(&temporal);
        return Err("No se pudo guardar el manifiesto del histórico.".into());
    }
    Ok(entrada)
}

/// Comprueba cada respaldo por separado. Un respaldo irresoluble deja fallida esa persona.
pub fn cargar(carpeta: &Path) -> Result<Vec<Value>, String> {
    let manifiesto = estado(carpeta)?;
    let capturas = manifiesto["capturas"].as_object().ok_or("Control de histórico inválido.")?;
    let mut salida = Vec::with_capacity(capturas.len());
    for (clave, e) in capturas {
        let tribunal = e["tribunal"].as_str().filter(|s| !s.trim().is_empty());
        let persona = e["person_id"].as_str().filter(|s| !s.trim().is_empty());
        if !tribunal.zip(persona).is_some_and(|(t, p)| clave == &format!("{t}|{p}")) {
            return Err("Entrada sin identidad coherente en el control de histórico.".into());
        }
        let leida = e["leida"].as_str().is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok());
        let filas = e["filas"].as_array().is_some_and(|filas| {
            filas.iter().all(|f| {
                f["fila"].as_u64().is_some_and(|n| n > 0)
                    && ["programa", "fecha_egreso", "causal_egreso"].iter().all(|k| f[*k].is_string())
            })
        });
        let estado = e["estado"].as_str();
        let exige_respaldo = matches!(estado, Some("LEIDA" | "VACIA_COMPROBADA"));
        let tiene_respaldo = e["archivo"].as_str().is_some_and(|s| !s.is_empty());
        let vacio_coherente = estado != Some("VACIA_COMPROBADA")
            || e["filas"].as_array().is_some_and(Vec::is_empty)
                && e["advertencias"].as_array().is_some_and(Vec::is_empty);
        let mut entrada = e.clone();
        if !leida || !filas || !vacio_coherente
            || (exige_respaldo || tiene_respaldo) && !respaldo_valido(carpeta, e)
        {
            entrada["estado"] = json!("FALLIDA");
            entrada["error"] = json!("HISTORICO_RESPALDO_INVALIDO");
            entrada["ownership"] = json!("INCIERTA");
            entrada["filas"] = json!([]);
            if !leida {
                entrada["leida"] = json!(chrono::Utc::now().to_rfc3339());
            }
        }
        salida.push(entrada);
    }
    Ok(salida)
}
