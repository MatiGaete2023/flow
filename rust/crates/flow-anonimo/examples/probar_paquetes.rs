//! Copias exclusivamente ficticias para cotejar el contrato con Python.
fn main() {
    let destino = std::path::PathBuf::from(std::env::args_os().nth(1).expect("carpeta de prueba"));
    std::fs::create_dir_all(&destino).unwrap();
    for (i, bytes) in [
        include_bytes!("../../flow-trabajo/tests/datos/paquete_cronologia.zip").as_slice(),
        include_bytes!("../../flow-trabajo/tests/datos/paquete_controles.zip").as_slice(),
        include_bytes!("../../flow-trabajo/tests/datos/paquete_historia.zip").as_slice(),
    ]
    .iter()
    .enumerate()
    {
        let s = flow_anonimo::formatos::anonimizar(
            "captura.zip",
            bytes,
            flow_anonimo::Opciones { modo: flow_anonimo::Modo::Conservador, ..Default::default() },
        )
        .unwrap();
        std::fs::write(destino.join(format!("ficticio_{i}.zip")), s.datos()).unwrap();
    }
}
