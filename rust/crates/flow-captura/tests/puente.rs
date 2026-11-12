//! El puente solo atiende a la extensión que presenta el código correcto, y entrega y recibe tareas.

use flow_captura::Puente;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

const EXTENSION: &str = "abcdefghijklmnopabcdefghijklmnop";

fn pedir(puerto: u16, metodo: &str, ruta: &str, codigo: &str, extension: &str, cuerpo: Option<&Value>) -> (u16, Value) {
    let mut s = TcpStream::connect(("127.0.0.1", puerto)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
    let cuerpo = cuerpo.map(Value::to_string).unwrap_or_default();
    let mut texto = format!("{metodo} {ruta} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nX-Sitfa-Code: {codigo}\r\nX-Sitfa-Extension: {extension}\r\n");
    if !cuerpo.is_empty() {
        texto.push_str(&format!("Content-Type: application/json\r\nContent-Length: {}\r\n", cuerpo.len()));
    }
    texto.push_str("\r\n");
    texto.push_str(&cuerpo);
    s.write_all(texto.as_bytes()).unwrap();
    let mut respuesta = String::new();
    let _ = s.read_to_string(&mut respuesta);
    let estado = respuesta.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
    let json = respuesta.split("\r\n\r\n").nth(1).and_then(|c| serde_json::from_str(c.trim()).ok()).unwrap_or(Value::Null);
    (estado, json)
}

#[test]
fn rechaza_sin_codigo_u_origen() {
    let p = Puente::abrir().unwrap();
    let codigo = p.codigo_conexion();
    let (puerto, secreto) = codigo.split_once('-').unwrap();
    assert_eq!(puerto.parse::<u16>().unwrap(), p.puerto());
    assert_eq!(secreto.len(), 32);
    let malo = "0".repeat(32);
    assert_eq!(pedir(p.puerto(), "POST", "/connect", &malo, EXTENSION, Some(&json!({}))).1["error"], "CONNECTION_CODE");
    assert_eq!(pedir(p.puerto(), "POST", "/connect", secreto, "no-es-extension", Some(&json!({}))).1["error"], "EXTENSION_ORIGIN");
    assert!(!p.conectado());
    assert_eq!(p.llamar("catalog", json!({}), Duration::from_millis(50)).unwrap_err().codigo, "SIN_CONEXION");
    assert_eq!(p.llamar("escribir_rus", json!({}), Duration::from_millis(50)).unwrap_err().codigo, "NO_PERMITIDA");
}

#[test]
fn entrega_tarea_y_recibe_resultado() {
    let p = Arc::new(Puente::abrir().unwrap());
    let codigo = p.codigo_conexion();
    let secreto = codigo.split_once('-').unwrap().1.to_string();
    let puerto = p.puerto();
    assert_eq!(pedir(puerto, "POST", "/connect", &secreto, EXTENSION, Some(&json!({}))).0, 200);
    assert!(p.conectado());
    // Otra copia de la extensión no puede tomar el enlace ya establecido.
    let otra = "ponmlkjihgfedcbaponmlkjihgfedcba";
    assert_eq!(pedir(puerto, "GET", "/next", &secreto, otra, None).1["error"], "EXTENSION_CHANGED");
    let extension = {
        let secreto = secreto.clone();
        std::thread::spawn(move || {
            let (_, tarea) = pedir(puerto, "GET", "/next", &secreto, EXTENSION, None);
            assert_eq!(tarea["command"], "catalog");
            let respuesta = json!({"id": tarea["id"], "ok": true, "result": {"pantalla": "seguimiento"}});
            assert_eq!(pedir(puerto, "POST", "/complete", &secreto, EXTENSION, Some(&respuesta)).0, 200);
            // Repetir la misma respuesta ya no encuentra la tarea.
            assert_eq!(pedir(puerto, "POST", "/complete", &secreto, EXTENSION, Some(&respuesta)).1["error"], "TASK_EXPIRED");
        })
    };
    let resultado = p.llamar("catalog", json!({"screen": "seguimiento"}), Duration::from_secs(6)).unwrap();
    assert_eq!(resultado["pantalla"], "seguimiento");
    extension.join().unwrap();
    // Sin nadie que responda, vence el plazo; y un error de la extensión llega con su código.
    assert_eq!(p.llamar("search", json!({}), Duration::from_millis(100)).unwrap_err().codigo, "TIEMPO");
    let extension = std::thread::spawn(move || {
        loop {
            let (_, tarea) = pedir(puerto, "GET", "/next", &secreto, EXTENSION, None);
            if tarea["idle"] == true {
                continue;
            }
            let respuesta = json!({"id": tarea["id"], "ok": false, "error": "FOLLOWUP_REQUIRED"});
            pedir(puerto, "POST", "/complete", &secreto, EXTENSION, Some(&respuesta));
            break;
        }
    });
    assert_eq!(p.llamar("catalog", json!({}), Duration::from_secs(8)).unwrap_err().codigo, "FOLLOWUP_REQUIRED");
    extension.join().unwrap();
}
