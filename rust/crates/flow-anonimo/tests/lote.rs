use flow_anonimo::{lote, Modo, Opciones};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
fn temp() -> PathBuf {
    let mut r = [0u8; 8];
    getrandom::fill(&mut r).unwrap();
    let p = std::env::temp_dir().join(format!("flow-anon-prueba-{}", u64::from_le_bytes(r)));
    std::fs::create_dir(&p).unwrap();
    p
}
#[test]
fn carpeta_deduplicada_sesion_comun_y_reporte_sin_identidades() {
    let p = temp();
    let entrada = p.join("Nombrecanario");
    let salida = p.join("salida");
    std::fs::create_dir(&entrada).unwrap();
    std::fs::create_dir(&salida).unwrap();
    let a = entrada.join("a.json");
    std::fs::write(&a, br#"{"Nombre":"Identcanario Apelcanario"}"#).unwrap();
    std::fs::write(entrada.join("b.txt"), b"Se conversa con Identcanario Apelcanario.").unwrap();
    std::fs::write(entrada.join("c.pdf"), b"%PDF-1.7 prueba").unwrap();
    let opciones = Opciones { modo: Modo::Conservador, semilla: "SEMILLA_FICTICIA_NO_PUBLICAR".into(), ..Default::default() };
    let r = lote::ejecutar(&[entrada.clone(), a.clone()], &salida, opciones, &AtomicBool::new(false), |_| {}).unwrap();
    assert_eq!(r.informe.archivos.len(), 2);
    assert_eq!(r.informe.excluidos, 1);
    for archivo in std::fs::read_dir(r.carpeta).unwrap() {
        let s = std::fs::read_to_string(archivo.unwrap().path()).unwrap();
        for privado in ["Identcanario", "Apelcanario", "Nombrecanario", "SEMILLA_FICTICIA_NO_PUBLICAR"] {
            assert!(!s.contains(privado));
        }
    }
    assert_eq!(std::fs::read(a).unwrap(), br#"{"Nombre":"Identcanario Apelcanario"}"#);
    std::fs::remove_dir_all(p).unwrap();
}
#[test]
fn cancelacion_y_rechazo_no_dejan_copias_parciales() {
    let p = temp();
    let entrada = p.join("entrada");
    let salida = p.join("salida");
    std::fs::create_dir(&entrada).unwrap();
    std::fs::create_dir(&salida).unwrap();
    std::fs::write(entrada.join("a.json"), br#"{"Nombre":"Identcanario Apelcanario"}"#).unwrap();
    let cancelar = AtomicBool::new(false);
    let r = lote::ejecutar(std::slice::from_ref(&entrada), &salida, Opciones::default(), &cancelar, |a| {
        if a.fase == "Guardando copias" {
            cancelar.store(true, Ordering::Relaxed);
        }
    });
    assert!(r.is_err());
    assert_eq!(std::fs::read_dir(&salida).unwrap().count(), 0);
    cancelar.store(false, Ordering::Relaxed);
    std::fs::write(entrada.join("b.pdf"), b"%PDF-1.7 prueba").unwrap();
    assert!(lote::ejecutar(&[entrada], &salida, Opciones { rechazar_no_admitidos: true, ..Default::default() }, &cancelar, |_| {}).is_err());
    assert_eq!(std::fs::read_dir(salida).unwrap().count(), 0);
    std::fs::remove_dir_all(p).unwrap();
}
#[cfg(unix)]
#[test]
fn enlaces_de_carpeta_no_se_siguen() {
    let p = temp();
    std::os::unix::fs::symlink(&p, p.join("ciclo")).unwrap();
    assert!(lote::ejecutar(std::slice::from_ref(&p), &p, Opciones::default(), &AtomicBool::new(false), |_| {}).is_err());
    std::fs::remove_dir_all(p).unwrap();
}
