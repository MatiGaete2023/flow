//! Autodiagnóstico del ejecutable: comprueba con datos sintéticos, en una carpeta temporal aislada y sin red, que el
//! paquete trae lo que necesita (matrices, reglas, lectura, producto Excel, Anonimizador y CC fijo). No toca los datos
//! del usuario ni abre RUS, SATURNO u Outlook.

use crate::{anon, config, demo, motor::Mode, producto_excel, productos, trabajo::Trabajo};
use chrono::NaiveDate;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub struct Comprobacion {
    pub nombre: &'static str,
    pub ok: bool,
    pub detalle: String,
}

pub struct Informe {
    pub comprobaciones: Vec<Comprobacion>,
}

impl Informe {
    pub fn ok(&self) -> bool {
        self.comprobaciones.iter().all(|c| c.ok)
    }

    pub fn json(&self) -> Value {
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "commit": option_env!("GITHUB_SHA").unwrap_or("desarrollo"),
            "ok": self.ok(),
            "outlook_disponible": cfg!(target_os = "windows"),
            "comprobaciones": self.comprobaciones.iter().map(|c| json!({"nombre": c.nombre, "ok": c.ok, "detalle": c.detalle})).collect::<Vec<_>>(),
        })
    }
}

fn paso(nombre: &'static str, r: Result<String, String>) -> Comprobacion {
    match r {
        Ok(detalle) => Comprobacion { nombre, ok: true, detalle },
        Err(detalle) => Comprobacion { nombre, ok: false, detalle },
    }
}

fn matrices() -> Result<String, String> {
    for (t, k, datos) in productos::MATRICES.iter() {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(*datos)).map_err(|e| format!("{t}/{k}: {e}"))?;
        z.by_name("word/document.xml").map_err(|e| format!("{t}/{k}: {e}"))?;
    }
    Ok(format!("{} matrices Word incluidas", productos::MATRICES.len()))
}

fn flujo(modo: Mode, tmp: &Path) -> Result<String, String> {
    let hoy = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
    let libro = demo::libro(modo, hoy)?;
    let cfg = config::Config::default();
    let t = Trabajo::abrir_archivo(libro.clone(), Some(modo), hoy, &cfg)?;
    let _ = std::fs::remove_file(libro);
    if t.registros.is_empty() {
        return Err("el ejemplo no produjo filas".into());
    }
    let salida = producto_excel::exportar(&t, &tmp.join(format!("producto_{}", modo.label())), None)?;
    if !salida.is_file() {
        return Err("el producto Excel no se escribió".into());
    }
    Ok(format!("{} filas, producto Excel escrito", t.registros.len()))
}

fn anonimizador(tmp: &Path) -> Result<String, String> {
    let entrada = tmp.join("nota.txt");
    std::fs::write(&entrada, "Se conversa con Identcanario Apelcanario, RUT 11.111.111-1.").map_err(|e| e.to_string())?;
    let o = anon::Opciones { modo: anon::Modo::Conservador, semilla: "AUTODIAGNOSTICO".into(), ..Default::default() };
    let inf = anon::ejecutar(&[entrada], &tmp.join("anonimo"), &o);
    if let Some(e) = inf.errores.first() {
        return Err(e.clone());
    }
    let copia = inf.creados.first().ok_or("sin copia")?;
    let t = std::fs::read_to_string(copia).map_err(|e| e.to_string())?;
    if t.contains("Identcanario") || t.contains("11.111.111-1") {
        return Err("la copia conserva datos del ejemplo".into());
    }
    Ok("copia sin identidades".into())
}

/// Corre todas las comprobaciones dentro de una carpeta temporal propia, que se borra al terminar.
pub fn ejecutar() -> Informe {
    let tmp = std::env::temp_dir().join(format!("flow-autodiagnostico-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let preparada = std::fs::create_dir_all(&tmp).map_err(|e| format!("carpeta temporal: {e}"));
    let mut v = vec![paso("Carpeta temporal", preparada.map(|_| "lista".to_string()))];
    v.push(paso("Matrices Word", matrices()));
    v.push(paso(
        "CC obligatorio",
        if config::CC_OBLIGATORIO == "ucc_concepcion@pjud.cl" { Ok(config::CC_OBLIGATORIO.to_string()) } else { Err("CC distinto del fijo".into()) },
    ));
    for modo in Mode::ALL {
        let nombre = match modo {
            Mode::Espera => "Flujo Espera",
            Mode::Cumplimiento => "Flujo Cumplimiento",
            Mode::Informes => "Flujo Informes",
        };
        v.push(paso(nombre, flujo(modo, &tmp)));
    }
    v.push(paso("Anonimizador", anonimizador(&tmp)));
    let _ = std::fs::remove_dir_all(&tmp);
    Informe { comprobaciones: v }
}

/// `--self-test <informe.json>`: escribe el informe en una ruta nueva (si existe, no la reemplaza) y devuelve el código de
/// salida: 0 pasa, 1 falla una comprobación, 2 el informe ya existe o no se pudo escribir.
pub fn por_linea_de_comandos(destino: &Path) -> i32 {
    if destino.exists() {
        return 2;
    }
    let inf = ejecutar();
    let texto = serde_json::to_string_pretty(&inf.json()).unwrap_or_default();
    match std::fs::OpenOptions::new().write(true).create_new(true).open(destino) {
        Ok(mut f) => {
            use std::io::Write;
            if f.write_all(texto.as_bytes()).is_err() {
                return 2;
            }
        }
        Err(_) => return 2,
    }
    if inf.ok() {
        0
    } else {
        1
    }
}

pub fn ruta_desde_argumentos(args: &[String]) -> Option<PathBuf> {
    args.iter().position(|a| a == "--self-test").and_then(|i| args.get(i + 1)).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_autodiagnostico_pasa_con_datos_sinteticos_y_no_sobrescribe() {
        let d = std::env::temp_dir().join(format!("flow-autodiag-prueba-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let informe = d.join("informe.json");
        assert_eq!(por_linea_de_comandos(&informe), 0, "{}", std::fs::read_to_string(&informe).unwrap_or_default());
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&informe).unwrap()).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["comprobaciones"].as_array().unwrap().len(), 7);
        let antes = std::fs::read(&informe).unwrap();
        assert_eq!(por_linea_de_comandos(&informe), 2);
        assert_eq!(std::fs::read(&informe).unwrap(), antes);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn una_comprobacion_fallida_se_informa() {
        let inf = Informe { comprobaciones: vec![paso("a", Ok("x".into())), paso("b", Err("y".into()))] };
        assert!(!inf.ok());
        assert_eq!(inf.json()["ok"], false);
    }

    #[test]
    fn ruta_de_la_linea_de_comandos() {
        let a: Vec<String> = ["flow", "--self-test", "x.json"].iter().map(|s| s.to_string()).collect();
        assert_eq!(ruta_desde_argumentos(&a), Some(PathBuf::from("x.json")));
        assert_eq!(ruta_desde_argumentos(&a[..2]), None);
    }
}
