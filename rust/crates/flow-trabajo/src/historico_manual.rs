//! Ingreso Histórico Protección: captura MANUAL de la tabla visible de la persona seleccionada (plan 18-nov-2026).
//! Espejo de `descargador/historico_manual.py` (mismo `historico.json`). No consulta nada ni sigue enlaces: el funcionario
//! abre la vista y la entrega. Sin prueba de pertenencia a la persona de la fila, la captura queda INCIERTA.

use flow_captura::lecturas::leer_historico;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const MANIFIESTO: &str = "historico.json";
const MAX_BYTES: usize = 8 * 1024 * 1024;

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

fn estado(carpeta: &Path) -> Value {
    std::fs::read(carpeta.join(MANIFIESTO))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .filter(|v| v["tipo"] == "HISTORICO_PROTECCION" && v["version"] == 1)
        .unwrap_or_else(|| json!({"version":1,"tipo":"HISTORICO_PROTECCION","capturas":{}}))
}

/// Guarda una captura manual y devuelve su entrada del manifiesto. Capturar de nuevo a la misma persona reemplaza la anterior.
pub fn capturar(carpeta: &Path, html: &[u8], persona: &Persona, capturada: &str) -> Result<Value, String> {
    if html.is_empty() || html.len() > MAX_BYTES {
        return Err("La vista del histórico está vacía o supera 8 MiB.".into());
    }
    if persona.tribunal.trim().is_empty() || persona.person_id.trim().is_empty() {
        return Err("Falta el tribunal o la persona seleccionada: no se puede asociar el histórico.".into());
    }
    let sha = huella(html);
    let archivo = format!("historico-{}.html", &sha[..24]);
    let mut entrada = json!({"tribunal":persona.tribunal,"tribunal_nombre":persona.tribunal_nombre,"person_id":persona.person_id,
        "archivo":archivo,"sha256":sha,"leida":capturada});
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
    std::fs::create_dir_all(&carpeta_html).map_err(|_| "No se pudo crear la carpeta del histórico.")?;
    let ruta = carpeta_html.join(entrada["archivo"].as_str().unwrap());
    if !ruta.exists() {
        std::fs::write(&ruta, html).map_err(|_| "No se pudo guardar la vista del histórico.")?;
    }
    let mut manifiesto = estado(carpeta);
    manifiesto["capturas"][format!("{}|{}", persona.tribunal, persona.person_id)] = entrada.clone();
    std::fs::write(carpeta.join(MANIFIESTO), serde_json::to_vec_pretty(&manifiesto).map_err(|_| "Manifiesto ilegible.")?)
        .map_err(|_| "No se pudo guardar el manifiesto del histórico.")?;
    Ok(entrada)
}

/// Capturas del manifiesto con su HTML respaldado verificado contra la huella; una discrepancia es error, no vacío.
pub fn cargar(carpeta: &Path) -> Result<Vec<Value>, String> {
    let manifiesto = estado(carpeta);
    let mut salida = vec![];
    for e in manifiesto["capturas"].as_object().into_iter().flatten().map(|(_, v)| v) {
        let ruta = carpeta.join("historico").join(e["archivo"].as_str().unwrap_or(""));
        let datos = std::fs::read(&ruta).map_err(|_| "Falta el respaldo de un histórico.")?;
        if Some(huella(&datos).as_str()) != e["sha256"].as_str() {
            return Err("El respaldo de un histórico no coincide con su huella.".into());
        }
        salida.push(e.clone());
    }
    Ok(salida)
}
