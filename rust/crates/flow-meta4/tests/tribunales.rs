//! Reconocimiento de tribunales contra el oráculo Python (`rust/oraculo/generar_tribunales_meta4.py`): textos
//! ficticios, alias «LA» de Los Ángeles incluido.

#[test]
fn textos_de_tribunal_coinciden_con_python() {
    let casos: Vec<serde_json::Value> = serde_json::from_str(include_str!("datos/tribunales_python.json")).unwrap();
    assert!(casos.len() > 100);
    let distintos: Vec<String> = casos
        .iter()
        .filter_map(|c| {
            let texto = c["texto"].as_str().unwrap();
            let rust = flow_meta4::tribunales::tribunal(texto);
            (rust != c["tribunal"].as_str()).then(|| format!("{texto:?}: Rust {rust:?}, Python {}", c["tribunal"]))
        })
        .collect();
    assert!(distintos.is_empty(), "{distintos:#?}");
}
