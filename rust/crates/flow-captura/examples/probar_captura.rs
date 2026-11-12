//! Ensayo de extremo a extremo: puente Rust → extensión MV3 → SITFA ficticio en Chromium.
//! El código de conexión pasa al proceso hijo por stdin y nunca se imprime ni se escribe en disco.

use flow_captura::{
    lotes::{self, Lote},
    Puente,
};
use serde_json::{json, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

fn main() -> Result<(), String> {
    let vacia = std::env::args().any(|a| a == "--vacia");
    let lecturas = std::env::args().any(|a| a == "--lecturas");
    let puente = Puente::abrir()?;
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut navegador = Command::new("node")
        .arg(raiz.join("pruebas/captura_chrome.cjs"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|_| "No se pudo iniciar Node/Playwright.")?;
    writeln!(navegador.stdin.as_mut().unwrap(), "{}", json!({"codigo":puente.codigo_conexion(),"vacia":vacia,"lecturas":lecturas}))
        .map_err(|_| "No se pudo enlazar el ensayo con Chromium.")?;
    let limite = Instant::now() + Duration::from_secs(25);
    while !puente.conectado() && Instant::now() < limite {
        if navegador.try_wait().map_err(|_| "No se pudo comprobar Chromium.")?.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if !puente.conectado() {
        let _ = navegador.kill();
        let salida = navegador.wait_with_output().map_err(|_| "No se pudo cerrar Chromium.")?;
        return Err(format!("Chromium no conectó: {}", String::from_utf8_lossy(&salida.stderr)));
    }
    let casos: Vec<Value> = serde_json::from_str(include_str!("../tests/datos/sitfa_ficticio.json")).unwrap();
    let caso = casos
        .iter()
        .find(|c| {
            c["nombre"]
                == if vacia {
                    "seguimiento/Espera/vacia"
                } else if lecturas {
                    "seguimiento/Cumplimiento"
                } else {
                    "seguimiento/Espera"
                }
        })
        .unwrap();
    let lote = Lote {
        consultas: vec![serde_json::from_value(caso["seleccion"].clone()).unwrap()],
        fechas: None,
        conservar_filtros: false,
        evidencia: vacia,
        pausa_ms: 250,
        lecturas: flow_captura::recoleccion::Opciones { bitacoras: lecturas, historia: lecturas, calendario: lecturas, meses: 4 },
    };
    let padre = std::env::temp_dir();
    let carpeta = lotes::carpeta_nueva(&padre)?;
    let resultado = (|| {
        let catalogo = lotes::catalogo(&puente, "seguimiento")?;
        lotes::ejecutar(&puente, &catalogo, &lote, &carpeta, false, &AtomicBool::new(false), &|_| {})
    })();
    let _ = writeln!(navegador.stdin.as_mut().unwrap(), "fin");
    let salida = navegador.wait_with_output().map_err(|_| "No se pudo cerrar el ensayo Chromium.")?;
    let _ = std::fs::remove_dir_all(&carpeta);
    let resumen = resultado?;
    if !salida.status.success() {
        return Err(format!("Falló el ensayo Chromium: {}", String::from_utf8_lossy(&salida.stderr)));
    }
    if resumen.archivos.len() != if vacia { 1 } else { 2 } {
        return Err("Cantidad inesperada de productos del ensayo.".into());
    }
    if lecturas && resumen.lecturas.len() != 18 {
        return Err(format!("Se esperaban 18 lecturas; se guardaron {}.", resumen.lecturas.len()));
    }
    println!(
        "Captura MV3 ficticia PASA: {} consultas, {} archivos verificados; filtros restaurados.",
        resumen.terminadas.len(),
        resumen.archivos.len()
    );
    Ok(())
}
