//! El paquete ficticio con historia, importado por Rust, da lo mismo que `import_capture` del Asistente Python
//! (`rust/oraculo/generar_paquete.py`). Un paquete alterado se rechaza.

use flow_motor::{Config, Marca};
use flow_trabajo::paquete;
use serde::Deserialize;

#[derive(Deserialize)]
struct FilaEsperada {
    id: String,
    observacion: String,
    reglas: Vec<String>,
    acciones: Vec<String>,
    marcas: Vec<Marca>,
}

#[derive(Deserialize)]
struct Esperado {
    corte: chrono::NaiveDate,
    filas: Vec<FilaEsperada>,
}

fn datos(nombre: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/datos/{nombre}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn paquete_con_historia_igual_que_python() {
    let esperado: Esperado = serde_json::from_slice(&datos("paquete_historia.json")).unwrap();
    let zip = datos("paquete_historia.zip");
    assert!(paquete::es_paquete(&zip));
    let p = paquete::importar("paquete_historia.zip", &zip, esperado.corte, &Config::default()).unwrap();
    let filas: Vec<_> = p.trabajos.iter().flat_map(|t| t.filas.iter()).collect();
    assert_eq!(filas.len(), esperado.filas.len());
    let mut con_historia = 0;
    for e in &esperado.filas {
        let f = filas.iter().find(|f| f.id == e.id).unwrap_or_else(|| panic!("falta {}", e.id));
        assert_eq!(f.propuesta.observacion, e.observacion, "{}", e.id);
        assert_eq!(f.propuesta.reglas, e.reglas, "{}", e.id);
        assert_eq!(f.propuesta.acciones, e.acciones, "{}", e.id);
        assert_eq!(f.propuesta.marcas, e.marcas, "{}", e.id);
        con_historia += usize::from(e.reglas.iter().any(|r| r.starts_with("HISTORIA.E04") || r.starts_with("HISTORIA.R11")));
    }
    assert!(con_historia > 0, "el caso debe ejercitar reglas de historia");
}

#[test]
fn paquete_alterado_se_rechaza() {
    let mut zip = datos("paquete_historia.zip");
    // Altera un byte de los datos comprimidos del primer CSV: falla la lectura o deja de coincidir la huella.
    zip[120] ^= 0x55;
    assert!(paquete::importar("x.zip", &zip, "2026-08-20".parse().unwrap(), &Config::default()).is_err());
    assert!(!paquete::es_paquete(b"PK no es zip"));
}
