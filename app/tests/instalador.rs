use std::path::{Path, PathBuf};

fn raiz_repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("directorio padre de app/")
        .to_path_buf()
}

#[test]
fn contrato_instalador_por_usuario_y_actualizaciones() {
    let raiz = raiz_repo();
    let iss = std::fs::read_to_string(raiz.join("installer/FlowRust.iss"))
        .expect("debe existir installer/FlowRust.iss");
    let instalar = std::fs::read_to_string(raiz.join("installer/Instalar_Flow.ps1"))
        .expect("debe existir installer/Instalar_Flow.ps1");
    let desinstalar = std::fs::read_to_string(raiz.join("installer/Desinstalar_Flow.ps1"))
        .expect("debe existir installer/Desinstalar_Flow.ps1");
    let cmd = std::fs::read_to_string(raiz.join("installer/Instalar_Flow.cmd"))
        .expect("debe existir installer/Instalar_Flow.cmd");
    let verificador = std::fs::read_to_string(raiz.join("tools/verify_flow_rust_installer.ps1"))
        .expect("debe existir tools/verify_flow_rust_installer.ps1");

    // E-I1: instalación por usuario (sin administrador), respetando actualizaciones (E-I2) y acceso directo al menú.
    let iss_checks = [
        "PrivilegesRequired=lowest",
        "{localappdata}\\Programs\\FlowRust",
        "{localappdata}\\FlowRust\\actualizaciones",
        "uninsneveruninstall",
        "{autoprograms}\\Flow",
        "flow_rust.exe.anterior",
        "CloseApplications=yes",
    ];
    for c in iss_checks {
        assert!(iss.contains(c), "FlowRust.iss debe contener `{c}`");
    }
    assert!(
        !iss.contains("PrivilegesRequired=admin"),
        "FlowRust.iss no debe exigir administrador"
    );

    let instalar_checks = [
        "SHA256]::Create()",
        "actualizaciones",
        "Flow.lnk",
        "Desinstalar_Flow.ps1",
        "Uninstall",
        "flow_rust.exe",
        "SinHuella",
    ];
    for c in instalar_checks {
        assert!(instalar.contains(c), "Instalar_Flow.ps1 debe contener `{c}`");
    }
    assert!(
        !instalar.contains("-Verb RunAs") && !instalar.contains("#Requires -RunAsAdministrator"),
        "Instalar_Flow.ps1 no debe pedir elevación"
    );

    let leeme = std::fs::read_to_string(raiz.join("installer/LEEME.txt"))
        .expect("debe existir installer/LEEME.txt");
    assert!(leeme.contains("OneDrive"));

    assert!(desinstalar.contains("Flow.lnk"));
    assert!(desinstalar.contains("flowDataDir"));
    assert!(!desinstalar.contains("actualizaciones\" -Recurse"));
    assert!(cmd.contains("DisableDelayedExpansion"));
    assert!(cmd.contains("-RequireSha256"));
    assert!(verificador.contains("actualizaciones"));
    assert!(verificador.contains("--self-test"));
}

#[cfg(windows)]
#[test]
fn instalador_instala_reinstala_y_desinstala_preservando_actualizaciones() {
    use sha2::{Digest, Sha256};

    let raiz = raiz_repo();
    let script_instalar = raiz.join("installer/Instalar_Flow.ps1");
    let base = std::env::temp_dir().join(format!(
        "flow-rust-ei1-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let origen = base.join("paquete");
    let destino = base.join("programa").join("FlowRust");
    let datos = base.join("local").join("FlowRust");
    let menu = base.join("menu");
    std::fs::create_dir_all(&origen).unwrap();

    let contenido_exe = b"MZflow-rust-sintetico-ei1";
    let exe_origen = origen.join("flow_rust.exe");
    std::fs::write(&exe_origen, contenido_exe).unwrap();
    let huella = format!("{:x}", Sha256::digest(contenido_exe));
    let sha_origen = origen.join("SHA256.txt");
    std::fs::write(&sha_origen, format!("{huella}  flow_rust.exe\n")).unwrap();

    // 1. Huella manipulada debe fallar antes de instalar.
    std::fs::write(&sha_origen, format!("{}  flow_rust.exe\n", "0".repeat(64))).unwrap();
    let salida_mala = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_instalar.to_str().unwrap(),
            "-SourceDir",
            origen.to_str().unwrap(),
            "-Destination",
            destino.to_str().unwrap(),
            "-DataBaseDir",
            datos.to_str().unwrap(),
            "-StartMenuDir",
            menu.to_str().unwrap(),
            "-SkipRegistry",
            "-RequireSha256",
            "-Silent",
        ])
        .output()
        .expect("ejecutar powershell.exe");
    assert!(
        !salida_mala.status.success(),
        "debe rechazar SHA256.txt manipulado"
    );
    let err_mala = String::from_utf8_lossy(&salida_mala.stderr);
    assert!(
        err_mala.contains("no coincide"),
        "el rechazo por SHA debe explicar la discrepancia, obtuvo: {err_mala}"
    );
    assert!(!destino.join("flow_rust.exe").exists());

    // 2. Restaurar huella correcta e instalar por usuario.
    std::fs::write(&sha_origen, format!("{huella}  flow_rust.exe\n")).unwrap();
    let salida_ok = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_instalar.to_str().unwrap(),
            "-SourceDir",
            origen.to_str().unwrap(),
            "-Destination",
            destino.to_str().unwrap(),
            "-DataBaseDir",
            datos.to_str().unwrap(),
            "-StartMenuDir",
            menu.to_str().unwrap(),
            "-SkipRegistry",
            "-RequireSha256",
            "-Silent",
        ])
        .output()
        .expect("ejecutar powershell.exe");
    assert!(
        salida_ok.status.success(),
        "instalación falló: {}{}",
        String::from_utf8_lossy(&salida_ok.stdout),
        String::from_utf8_lossy(&salida_ok.stderr)
    );
    assert_eq!(std::fs::read(destino.join("flow_rust.exe")).unwrap(), contenido_exe);
    assert!(datos.join("actualizaciones").is_dir());
    assert!(menu.join("Flow.lnk").is_file());
    assert!(destino.join("Desinstalar_Flow.ps1").is_file());

    // 3. Crear datos previos en actualizaciones/ (E-I2), extension-integral/ y flow_rust.exe.anterior; reinstalar y desinstalar.
    let pendiente = datos.join("actualizaciones").join("flow_rust.exe");
    let aplicadas = datos.join("actualizaciones").join("aplicadas");
    let ext = datos.join("extension-integral").join("manifest.json");
    std::fs::create_dir_all(&aplicadas).unwrap();
    std::fs::create_dir_all(ext.parent().unwrap()).unwrap();
    std::fs::write(&pendiente, b"MZpendiente").unwrap();
    std::fs::write(aplicadas.join("v0.txt"), b"v0").unwrap();
    std::fs::write(&ext, b"{\"version\":\"1.0\"}").unwrap();
    std::fs::write(destino.join("flow_rust.exe.anterior"), b"MZanterior").unwrap();

    let salida_re = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_instalar.to_str().unwrap(),
            "-SourceDir",
            origen.to_str().unwrap(),
            "-Destination",
            destino.to_str().unwrap(),
            "-DataBaseDir",
            datos.to_str().unwrap(),
            "-StartMenuDir",
            menu.to_str().unwrap(),
            "-SkipRegistry",
            "-RequireSha256",
            "-Silent",
        ])
        .output()
        .expect("reinstalar");
    assert!(salida_re.status.success());
    assert_eq!(std::fs::read(&pendiente).unwrap(), b"MZpendiente");
    assert_eq!(std::fs::read(aplicadas.join("v0.txt")).unwrap(), b"v0");
    assert_eq!(std::fs::read(&ext).unwrap(), b"{\"version\":\"1.0\"}");

    let script_desinstalar = destino.join("Desinstalar_Flow.ps1");
    let salida_uninst = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_desinstalar.to_str().unwrap(),
            "-Destination",
            destino.to_str().unwrap(),
            "-StartMenuDir",
            menu.to_str().unwrap(),
            "-SkipRegistry",
            "-Silent",
        ])
        .output()
        .expect("desinstalar");
    assert!(
        salida_uninst.status.success(),
        "desinstalación falló: {}{}",
        String::from_utf8_lossy(&salida_uninst.stdout),
        String::from_utf8_lossy(&salida_uninst.stderr)
    );
    assert!(!destino.exists(), "la carpeta del programa debe eliminarse");
    assert!(!menu.join("Flow.lnk").exists(), "Flow.lnk debe eliminarse");
    assert_eq!(std::fs::read(&pendiente).unwrap(), b"MZpendiente");
    assert_eq!(std::fs::read(aplicadas.join("v0.txt")).unwrap(), b"v0");
    assert_eq!(std::fs::read(&ext).unwrap(), b"{\"version\":\"1.0\"}");

    let _ = std::fs::remove_dir_all(&base);
}

#[cfg(windows)]
#[test]
fn instalador_sin_sha256_falla_por_omision() {
    let raiz = raiz_repo();
    let script_instalar = raiz.join("installer/Instalar_Flow.ps1");
    let base = std::env::temp_dir().join(format!(
        "flow-rust-sinsha-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let origen = base.join("paquete");
    let destino = base.join("programa").join("FlowRust");
    let datos = base.join("local").join("FlowRust");
    let menu = base.join("menu");
    std::fs::create_dir_all(&origen).unwrap();

    let contenido_exe = b"MZflow-rust-sintetico-sinsha";
    let exe_origen = origen.join("flow_rust.exe");
    std::fs::write(&exe_origen, contenido_exe).unwrap();
    // Sin crear SHA256.txt

    // 1. Por omisión (sin -RequireSha256 y sin -SinHuella) DEBE fallar porque exige SHA256.txt
    let salida_omision = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_instalar.to_str().unwrap(),
            "-SourceDir",
            origen.to_str().unwrap(),
            "-Destination",
            destino.to_str().unwrap(),
            "-DataBaseDir",
            datos.to_str().unwrap(),
            "-StartMenuDir",
            menu.to_str().unwrap(),
            "-SkipRegistry",
            "-Silent",
        ])
        .output()
        .expect("ejecutar powershell.exe");
    assert!(!salida_omision.status.success(), "debe fallar por omisión si falta SHA256.txt");
    let err_omision = String::from_utf8_lossy(&salida_omision.stderr);
    assert!(err_omision.contains("Falta SHA256.txt"), "debe indicar falta de SHA256.txt: {err_omision}");
    assert!(!destino.join("flow_rust.exe").exists());

    // 2. Con -SinHuella se permite la instalación y el mensaje advierte la falta de verificación
    let salida_sin_huella = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_instalar.to_str().unwrap(),
            "-SourceDir",
            origen.to_str().unwrap(),
            "-Destination",
            destino.to_str().unwrap(),
            "-DataBaseDir",
            datos.to_str().unwrap(),
            "-StartMenuDir",
            menu.to_str().unwrap(),
            "-SkipRegistry",
            "-SinHuella",
        ])
        .output()
        .expect("ejecutar powershell.exe");
    assert!(salida_sin_huella.status.success(), "con -SinHuella debe permitir instalar: {}", String::from_utf8_lossy(&salida_sin_huella.stderr));
    let out_sin_huella = String::from_utf8_lossy(&salida_sin_huella.stdout);
    assert!(
        out_sin_huella.contains("SHA-256 no verificado (instalación sin SHA256.txt)"),
        "debe avisar que no se verificó huella: {out_sin_huella}"
    );
    assert_eq!(std::fs::read(destino.join("flow_rust.exe")).unwrap(), contenido_exe);

    let _ = std::fs::remove_dir_all(&base);
}

#[cfg(windows)]
#[test]
fn desinstalador_rechaza_carpeta_flowrust_sin_ejecutable() {
    let raiz = raiz_repo();
    let script_desinstalar = raiz.join("installer/Desinstalar_Flow.ps1");
    let base = std::env::temp_dir().join(format!(
        "flow-rust-uninst-reject-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    // Carpeta con nombre «FlowRust» pero sin flow_rust.exe ni Desinstalar_Flow.ps1
    let falsa_carpeta = base.join("otra_cosa").join("FlowRust");
    std::fs::create_dir_all(&falsa_carpeta).unwrap();
    std::fs::write(falsa_carpeta.join("archivo_ajeno.txt"), b"datos ajenos").unwrap();

    let salida = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_desinstalar.to_str().unwrap(),
            "-Destination",
            falsa_carpeta.to_str().unwrap(),
            "-SkipRegistry",
            "-Silent",
        ])
        .output()
        .expect("ejecutar desinstalador");

    assert!(!salida.status.success(), "debe rechazar una carpeta sin flow_rust.exe ni Desinstalar_Flow.ps1");
    let err = String::from_utf8_lossy(&salida.stderr);
    assert!(
        err.contains("no corresponde a una instalacion de Flow Rust"),
        "error inesperado: {err}"
    );
    assert!(falsa_carpeta.join("archivo_ajeno.txt").exists(), "no debe borrar la carpeta rechazada");

    let _ = std::fs::remove_dir_all(&base);
}
