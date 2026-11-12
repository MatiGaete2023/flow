use flow_anonimo::{
    formatos, publicar,
    texto::{self, Sesion},
    Modo, Opciones,
};
use serde_json::{json, Value};
use std::io::{Cursor, Write};

fn opciones() -> Opciones {
    Opciones { semilla: "SEMILLA_FICTICIA_DE_PRUEBA".into(), ..Default::default() }
}

#[test]
fn json_preserva_relaciones_y_omite_texto_libre_y_credenciales() {
    let input = json!({"registros":[{"record_id":"registro-ficticio","cause_id":"causa-ficticia","Nombre":"Nombreficticio Apellidoficticio","RUT":"11111111-0","RIT":"X-1-2099","T ESPERA":40,"TRIBUNAL":"Laja","nota":"Relato reservado ficticio"}],"eventos":[{"record_id":"registro-ficticio","cause_id":"causa-ficticia","texto":"Nombre privado ficticio"}],"password":"SOLO-VALOR-FICTICIO"});
    let salida = formatos::anonimizar("datos.json", &serde_json::to_vec(&input).unwrap(), opciones()).unwrap();
    let out: Value = serde_json::from_slice(salida.datos()).unwrap();
    assert_eq!(out["registros"][0]["cause_id"], out["eventos"][0]["cause_id"]);
    assert_eq!(out["registros"][0]["record_id"], out["eventos"][0]["record_id"]);
    assert_eq!(out["registros"][0]["T ESPERA"], 40);
    assert_eq!(out["registros"][0]["TRIBUNAL"], "Laja");
    assert_eq!(out["registros"][0]["nota"], "[texto omitido]");
    assert_eq!(out["password"], "[credencial omitida]");
    let texto = String::from_utf8(salida.datos().to_vec()).unwrap().to_lowercase();
    assert!(!texto.contains("nombreficticio") && !texto.contains("apellidoficticio"));
    assert_ne!(out["registros"][0]["RIT"], input["registros"][0]["RIT"]);
}

#[test]
fn conservador_reemplaza_nombres_catalogados_en_todo_el_lote() {
    let a = formatos::cargar("persona.json", br#"{"nombre":"Nombreficticio Apellidoficticio"}"#).unwrap();
    let b = formatos::cargar("relato.txt", b"Se converso con Nombreficticio y Apellidoficticio.").unwrap();
    let mut s = Sesion::nueva(Opciones { modo: Modo::Conservador, ..opciones() }).unwrap();
    formatos::recolectar(&mut s, &a).unwrap();
    formatos::recolectar(&mut s, &b).unwrap();
    let salida = formatos::transformar(&mut s, &b).unwrap();
    let texto = String::from_utf8(salida.datos().to_vec()).unwrap();
    assert!(!texto.to_lowercase().contains("nombreficticio"));
    assert!(!texto.to_lowercase().contains("apellidoficticio"));
    assert!(texto.contains("Se converso"));
    assert!(!salida.avisos().is_empty());
}

#[test]
fn rut_siempre_invalido_y_revision_final_rechaza_datos_conocidos() {
    let mut s = Sesion::nueva(opciones()).unwrap();
    let base = "8".repeat(8);
    let original = format!("{base}-{}", texto::dv(&base));
    let fake = s.valor("RUT", &original).unwrap();
    let (n, d) = fake.split_once('-').unwrap();
    assert_ne!(texto::dv(n), d.chars().next().unwrap());
    assert!(s.verificar(&[(original, false)]).is_err());
    s.persona("Nombreficticio Apellidoficticio");
    assert!(s.verificar(&[("Nombreficticio Apellidoficticio".into(), true)]).is_err());
}

#[test]
fn fechas_horas_formula_y_numeros_utiles_se_conservan_sin_codigo_activo() {
    let input = "RIT,Nombre,T ESPERA,FECHA CENTRO,PROGRAMA\nX-1-2099,Nombreficticio Apellidoficticio,-10,2026-10-08T15:16:17,=1+1\n";
    let out = formatos::anonimizar("tabla.csv", input.as_bytes(), Opciones { desplazar_dias: 10, ..opciones() }).unwrap();
    let texto = String::from_utf8(out.datos().to_vec()).unwrap();
    assert!(texto.contains("-10") && !texto.contains("'-10"));
    assert!(texto.contains("2026-10-18T15:16:17"));
    assert!(texto.contains("'=1+1"));
    let mut libro = rust_xlsxwriter::Workbook::new();
    let hoja = libro.add_worksheet();
    hoja.set_name("Espera").unwrap();
    for (i, s) in ["RIT", "Nombre", "T ESPERA", "TRIBUNAL", "DERIVACION", "FECHA CENTRO"].iter().enumerate() {
        hoja.write_string(0, i as u16, *s).unwrap();
    }
    hoja.write_string(1, 0, "X-1-2099").unwrap();
    hoja.write_string(1, 1, "Nombreficticio Apellidoficticio").unwrap();
    hoja.write_number(1, 2, 40).unwrap();
    hoja.write_string(1, 3, "Laja").unwrap();
    hoja.write_string(1, 4, "PRM Ficticio").unwrap();
    let fecha = rust_xlsxwriter::ExcelDateTime::parse_from_str("2026-10-08 15:16:17").unwrap();
    let f = rust_xlsxwriter::Format::new().set_num_format("yyyy-mm-dd hh:mm:ss");
    hoja.write_datetime_with_format(1, 5, &fecha, &f).unwrap();
    let out = formatos::anonimizar("tabla.xlsx", &libro.save_to_buffer().unwrap(), opciones()).unwrap();
    let entrada = flow_tabular::leer_bytes("salida.xlsx", out.datos().to_vec()).unwrap();
    assert_eq!(entrada.hojas[0].nombre, "Espera");
    assert_eq!(entrada.hojas[0].filas[1][5].texto(), "2026-10-08T15:16:17");
}

#[test]
fn json_anidado_y_nacimiento_generalizado_son_tratados() {
    let input = json!({"normalized_json":json!({"Nombre":"Nombreficticio Apellidoficticio","FEC NACIMIENTO":"2000-10-08"}).to_string()});
    let out =
        formatos::anonimizar("datos.json", &serde_json::to_vec(&input).unwrap(), Opciones { generalizar_nacimiento: true, ..opciones() })
            .unwrap();
    let v: Value = serde_json::from_slice(out.datos()).unwrap();
    let interno: Value = serde_json::from_str(v["normalized_json"].as_str().unwrap()).unwrap();
    assert_eq!(interno["FEC NACIMIENTO"], "2000-01-01");
    assert_ne!(interno["Nombre"], "Nombreficticio Apellidoficticio");
}

#[test]
fn rechaza_xml_peligroso_zip_inseguro_y_contenido_binario() {
    let xml = b"<?xml version=\"1.0\"?><!DOCTYPE Workbook [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><Workbook/>";
    assert!(formatos::anonimizar("hoja.xls", xml, opciones()).is_err());
    let mut zip = zip::ZipWriter::new(Cursor::new(vec![]));
    zip.start_file("../fuera.txt", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(b"texto ficticio").unwrap();
    let zip = zip.finish().unwrap().into_inner();
    assert!(formatos::cargar("archivo.zip", &zip).is_err());
    for contenido in [b"MZ archivo ficticio".as_slice(), b"%PDF-1.7 ficticio", b"\x89PNG ficticio", b"texto\0binario"] {
        assert!(formatos::cargar("archivo.txt", contenido).is_err());
    }
}

#[test]
fn publicacion_no_sobrescribe_y_no_reutiliza_nombre_personal() {
    let mut azar = [0u8; 8];
    getrandom::fill(&mut azar).unwrap();
    let p = std::env::temp_dir().join(format!("flow-anonimo-prueba-{}", u64::from_ne_bytes(azar)));
    std::fs::create_dir(&p).unwrap();
    let out = formatos::anonimizar("Nombreficticio.txt", b"Nombre: Nombreficticio Apellidoficticio", opciones()).unwrap();
    let a = publicar(&p, &out).unwrap();
    let b = publicar(&p, &out).unwrap();
    assert_ne!(a, b);
    assert!(a.file_name().unwrap().to_string_lossy().starts_with("anonimizado_"));
    assert_eq!(std::fs::read(a).unwrap(), out.datos());
    std::fs::remove_dir_all(p).unwrap();
}

#[test]
fn clasifica_columnas_como_python() {
    use texto::Clase::*;
    let casos: Value = serde_json::from_str(include_str!("datos/columnas.json")).unwrap();
    for c in casos.as_array().unwrap() {
        let label = c["columna"].as_str().unwrap();
        let clase = texto::clase(label).map(|k| match k {
            Persona => "persona",
            Curador => "curador",
            Rut => "rut",
            Rit => "rit",
            Correo => "mail",
            Telefono => "telefono",
            Direccion => "direccion",
            Identificador => "ident",
            Huella => "hash",
            Json => "json",
            Texto => "texto",
            Tecnico => "tech",
        });
        assert_eq!(serde_json::to_value(clase).unwrap(), c["tipo"], "{label}");
        assert_eq!(texto::protegido(label), c["protegido"].as_bool().unwrap(), "{label}");
    }
}

#[test]
fn pares_de_etiqueta_y_celdas_vacias_no_dejan_nombres_en_la_copia() {
    let input =
        b"<html><table><tr><td>Nombre</td><td></td><td>Identcanario Apelcanario</td></tr><tr><td>Edad</td><td>10</td></tr></table></html>";
    let salida = formatos::anonimizar("pares.xls", input, opciones()).unwrap();
    let libro = flow_tabular::leer_bytes("salida.xlsx", salida.datos().to_vec()).unwrap();
    let texto = libro.hojas.iter().flat_map(|h| &h.filas).flatten().map(|v| v.texto()).collect::<Vec<_>>().join(" ");
    assert!(!texto.contains("Identcanario") && !texto.contains("Apelcanario"));
    assert!(texto.contains("Persona Ficticia"));
}

#[test]
fn rit_anonimos_no_se_confunden_con_telefonos_y_conservan_relacion_de_formatos() {
    let mut s = Sesion::nueva(opciones()).unwrap();
    let mut encontro = false;
    for i in 0..100 {
        let original = format!("X-{i}-2099");
        let fake = s.pseudo("rit", &original);
        s.verificar(&[(fake.clone(), false)]).unwrap();
        encontro |= fake.split('-').nth(1).unwrap().len() == 5;
    }
    assert!(encontro);
    assert!(s.pseudo("rit", "P-1-2099").starts_with("P-"));
    assert_eq!(s.pseudo("rit", "X-000123-2099"), s.pseudo("rit", "X 123 2099"));
}

#[test]
fn identidad_con_caracteres_no_admitidos_falla_cerrado() {
    let input = json!({"Nombre":"测试标识"});
    assert!(formatos::anonimizar("datos.json", &serde_json::to_vec(&input).unwrap(), opciones()).is_err());
}

#[test]
fn las_tres_variantes_xls_siguen_siendo_tablas_utiles() {
    for nombre in ["espera_biff.xls", "espera_html.xls", "espera_xml.xls"] {
        let ruta = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../flow-tabular/tests/datos").join(nombre);
        let origen = std::fs::read(ruta).unwrap();
        let original = flow_tabular::leer_bytes(nombre, origen.clone()).unwrap();
        let original = flow_tabular::leer_tabla(&original, Some(flow_dominio::Etapa::Espera), None).unwrap();
        let salida = formatos::anonimizar(nombre, &origen, opciones()).unwrap();
        let libro = flow_tabular::leer_bytes("salida.xlsx", salida.datos().to_vec()).unwrap();
        let tabla = flow_tabular::leer_tabla(&libro, Some(flow_dominio::Etapa::Espera), None).unwrap();
        assert_eq!(tabla.filas.len(), 2);
        assert!(tabla.filas.iter().zip(&original.filas).all(|(n, o)| n.registro.texto("nombre") != o.registro.texto("nombre")));
        assert!(tabla.filas.iter().all(|f| f.registro.entero("espera").is_some()));
    }
}
