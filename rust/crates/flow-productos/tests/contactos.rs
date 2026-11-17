//! Contactos como el Asistente Python (`config.parse_contacts`, `outputs.import_contacts`, `resolve_contact`,
//! `personalization.save_contact`), solo con direcciones ficticias de dominios reservados.

use flow_productos::correos::{correos_validos, leer_lista_csmp, Contactos};
use std::collections::BTreeMap;

fn mapa(pares: &[(&str, &str)]) -> BTreeMap<String, String> {
    pares.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

#[test]
fn lista_de_csmp_acepta_los_dos_formatos_y_descarta_lo_dudoso() {
    let lista = r#"[{"Nombre":"PRM Ficticio Uno","Mail":"uno@example.invalid"},
        {"Nombre":"PRM Ficticio Uno","Mail":"otro@example.invalid"},
        {"Nombre":"FAE Ficticio","Mail":"fae@example.test"},
        {"Nombre":"","Mail":"sin.nombre@example.test"},
        {"Nombre":"Mal Escrito","Mail":"no-es-correo"},
        {"Nombre":"Tribunal: LAJA","Mail":"laja@example.invalid; laja2@example.invalid"}]"#;
    let c = leer_lista_csmp(lista).unwrap();
    assert_eq!(c, mapa(&[("FAE Ficticio", "fae@example.test"), ("Tribunal: LAJA", "laja@example.invalid; laja2@example.invalid")]));
    // Formato histórico: objetos separados por comas, sin corchetes.
    let historico = r#"{"Nombre":"PIE Ficticio","Mail":"pie@example.invalid"},{"Nombre":"DAM Ficticio","Mail":"dam@example.invalid"},"#;
    assert_eq!(leer_lista_csmp(historico).unwrap().len(), 2);
    assert!(leer_lista_csmp("no es json").is_err());
    let d = Contactos::desde_csmp(&c);
    assert_eq!(d.programas, mapa(&[("FAE Ficticio", "fae@example.test")]));
    assert_eq!(d.tribunales["LAJA"], "laja@example.invalid; laja2@example.invalid");
    assert!(correos_validos("a@example.test, b@example.test\nA@example.test").unwrap().len() == 2);
    assert!(correos_validos("a@example").is_err());
}

#[test]
fn leer_csmp_no_modifica_el_archivo() {
    let dir = std::env::temp_dir().join(format!("flow-contactos-csmp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join("contactos.json");
    assert_eq!(Contactos::leer_csmp(&ruta).unwrap(), None);
    let texto = r#"[{"Nombre":"PRM Ficticio","Mail":"prm@example.invalid"}]"#;
    std::fs::write(&ruta, texto).unwrap();
    let c = Contactos::leer_csmp(&ruta).unwrap().unwrap();
    assert_eq!(c.de_programa("prm ficticio"), "prm@example.invalid");
    assert_eq!(std::fs::read_to_string(&ruta).unwrap(), texto);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn importar_json_y_catastro_no_sustituyen_sin_confirmar() {
    let mut actual = Contactos::default();
    actual.programas.insert("PRM Ficticio".into(), "prm@example.invalid".into());
    actual.tribunales.insert("TOME".into(), "tome@example.invalid".into());
    let imp = actual.importar_csmp(&mapa(&[
        ("PRM Ficticio", "cambio@example.invalid"),
        ("FAE Ficticio", "fae@example.invalid"),
        ("Tribunal: TOME", "tome2@example.invalid"),
    ]));
    assert_eq!(imp.conflictos, ["PRM Ficticio"]);
    assert_eq!(imp.nuevos, ["FAE Ficticio"]);
    assert_eq!(imp.tribunales, ["TOME"]);
    assert_eq!(imp.contactos.programas["PRM Ficticio"], "prm@example.invalid", "el conflicto no se sustituye");
    assert_eq!(imp.contactos.tribunales["TOME"], "tome2@example.invalid");
    assert_eq!(actual.programas.len(), 1, "nada cambia hasta confirmar");
    // Catastro: títulos «Programa» y «Correo electrónico»; un nombre igual normalizado con otra dirección es conflicto.
    let cab = ["Programa".to_string(), "Correo electrónico".to_string()];
    let filas = vec![
        vec!["prm ficticio".to_string(), "otra@example.invalid".to_string()],
        vec!["DAM Ficticio".to_string(), "dam@example.invalid; dam2@example.invalid".to_string()],
        vec!["".to_string(), "x@example.invalid".to_string()],
    ];
    let imp = actual.importar_catastro(&cab, &filas).unwrap();
    assert_eq!(imp.conflictos, ["prm ficticio"]);
    assert_eq!(imp.contactos.programas["DAM Ficticio"], "dam@example.invalid; dam2@example.invalid");
    assert!(actual.importar_catastro(&["Nombre".into(), "Teléfono".into()], &filas).unwrap_err().contains("Nombre/Programa"));
    assert!(actual.importar_catastro(&cab, &[vec!["X".into(), "malo".into()]]).unwrap_err().contains("Dirección inválida"));
}

#[test]
fn alias_de_programa_como_resolve_contact() {
    let mut c = Contactos::default();
    // `aliases_base.json` de CSMP: «DCE - KAMULAYAN» es «DCE - KAMULYAN» en el catastro (nombre institucional).
    c.programas.insert("DCE - KAMULYAN".into(), "dce@example.invalid".into());
    assert_eq!(c.de_programa("DCE - Kamulayan"), "dce@example.invalid");
    // Alias del usuario al guardar el contacto; no se toma el nombre de otro contacto ni un alias ajeno.
    c.guardar_programa(None, "PRM Ficticio Norte", "norte@example.invalid", &["PRM Norte".into()]).unwrap();
    assert_eq!(c.de_programa("prm norte"), "norte@example.invalid");
    assert!(c.guardar_programa(None, "PRM Ficticio Norte", "x@example.invalid", &[]).unwrap_err().contains("Ya existe"));
    assert!(c.guardar_programa(None, "Otro", "x@example.invalid", &["PRM Norte".into()]).unwrap_err().contains("pertenece"));
    assert!(c.guardar_programa(None, "PRM Norte", "x@example.invalid", &[]).unwrap_err().contains("alias"));
    // Editar y renombrar conserva el alias; quitar borra los alias propios.
    c.guardar_programa(Some("PRM Ficticio Norte"), "PRM Ficticio Norte Dos", "dos@example.invalid", &["PRM Norte".into()]).unwrap();
    assert_eq!(c.de_programa("PRM Norte"), "dos@example.invalid");
    assert!(!c.programas.contains_key("PRM Ficticio Norte"));
    c.quitar_programa("PRM Ficticio Norte Dos");
    assert!(c.aliases.is_empty());
    assert_eq!(c.de_programa("PRM Norte"), "");
}

#[test]
fn configuracion_de_la_app_python_es_la_fuente_preferida() {
    let dir = std::env::temp_dir().join(format!("flow-contactos-config-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join("configuracion.json");
    // Solo el ejemplo ficticio de Python: no son contactos del usuario.
    std::fs::write(
        &ruta,
        r#"{"version":1,"contactos":{"PRM EJEMPLO UNO":"prm.uno@example.invalid"},"correos":{"tribunales":{"LAJA":{"para":[]}}}}"#,
    )
    .unwrap();
    assert_eq!(Contactos::leer_csmp(&ruta).unwrap(), None);
    let texto = r#"{"version":1,"contactos":{"PRM Ficticio":"prm@example.invalid","Malo":"no-es-correo"},
        "correos":{"tribunales":{"LAJA":{"para":["laja@example.invalid"]},"TOME":{"para":[]}},"cc_adicional":"cc@example.test"},
        "aliases":{"PRM Alias Ficticio":"PRM Ficticio","DCE - KAMULAYAN":"DCE - KAMULYAN"},"firma":"Firma ficticia"}"#;
    std::fs::write(&ruta, texto).unwrap();
    let c = Contactos::leer_csmp(&ruta).unwrap().unwrap();
    assert_eq!(c.programas, mapa(&[("PRM Ficticio", "prm@example.invalid")]));
    assert_eq!(c.tribunales, mapa(&[("LAJA", "laja@example.invalid")]));
    assert_eq!(c.aliases, mapa(&[("PRM Alias Ficticio", "PRM Ficticio")]), "los alias base no se duplican");
    assert_eq!(c.de_programa("prm alias ficticio"), "prm@example.invalid");
    assert_eq!((c.cc_adicional.as_str(), c.firma.as_str()), ("cc@example.test", "Firma ficticia"));
    assert_eq!(std::fs::read_to_string(&ruta).unwrap(), texto, "no se modifica");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn alias_base_igual_al_de_python() {
    let leer = |r: &str| serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(r).unwrap()).unwrap();
    let raiz = env!("CARGO_MANIFEST_DIR");
    assert_eq!(leer(&format!("{raiz}/../../datos/aliases.json")), leer(&format!("{raiz}/../../../src/nurus/personal/aliases_base.json")));
}
