//! Paridad con el Anonimizador Python (`anonimizador_csmp`) con datos exclusivamente ficticios.
//! Los valores esperados se cotejaron con `engine.collect_bytes`/`engine.process_bytes` de Python.
use flow_anonimo::{documentos, formatos, lote, texto::Sesion, Modo, Opciones};
use serde_json::Value;
use std::{io::Cursor, path::PathBuf, sync::atomic::AtomicBool};

fn estricto() -> Opciones {
    Opciones::default()
}
fn conservador() -> Opciones {
    Opciones { modo: Modo::Conservador, ..estricto() }
}
fn texto(nombre: &str, datos: &[u8], o: Opciones) -> (String, String) {
    let s = formatos::anonimizar(nombre, datos, o).unwrap_or_else(|e| panic!("{nombre}: {e}"));
    (String::from_utf8(s.datos().to_vec()).unwrap(), s.extension().to_string())
}
fn sin(s: &str, privados: &[&str]) {
    for p in privados {
        assert!(!s.contains(p), "quedó «{p}» en:\n{s}");
    }
}
fn temp() -> PathBuf {
    let mut r = [0u8; 8];
    getrandom::fill(&mut r).unwrap();
    let p = std::env::temp_dir().join(format!("flow-anon-paridad-{}", u64::from_le_bytes(r)));
    std::fs::create_dir(&p).unwrap();
    p
}

// A1: Python (`add_person`) solo omite registrar; la celda de persona se reemplaza igual.
#[test]
fn a1_identidades_cortas_largas_o_sin_transliteracion_no_abortan() {
    for valor in ["Łukasz Nowak", "Müller Ñandú", "J", "Jo Li", "Ana", "12345", "Sin nombre"] {
        let (s, _) = texto("a.csv", format!("RIT,NOMBRE\nC-1-2099,{valor}\n").as_bytes(), estricto());
        assert!(s.starts_with("RIT,NOMBRE\n"), "{s}");
        assert!(s.contains(",Persona Ficticia "), "{valor}: {s}");
        sin(&s, &[valor]);
    }
    let (s, _) = texto("nn.txt", b"Nombre: NN\nRIT: C-1-2099\n", estricto());
    assert!(s.contains("Nombre: Persona Ficticia "));
    // Más de 512 caracteres con un correo: nunca sale el nombre junto al correo ficticio.
    let largo = format!("RIT,NOMBRE\nC-1-2099,Ana Canario Ficticio ana.canario@example.com {}\n", "palabra ".repeat(80));
    let (s, _) = texto("l.csv", largo.as_bytes(), estricto());
    sin(&s, &["Canario", "ana.canario", "palabra"]);
}
#[test]
fn a1_nombre_sin_transliteracion_se_reemplaza_en_otro_archivo() {
    let a = formatos::cargar("a.csv", "RIT,NOMBRE\nC-1-2099,Łukasz Nowakficticio\n".as_bytes()).unwrap();
    let b = formatos::cargar("b.txt", "Se habló con Łukasz Nowakficticio hoy.\n".as_bytes()).unwrap();
    let mut s = Sesion::nueva(conservador()).unwrap();
    formatos::recolectar(&mut s, &a).unwrap();
    formatos::recolectar(&mut s, &b).unwrap();
    formatos::transformar(&mut s, &a).unwrap();
    let out = String::from_utf8(formatos::transformar(&mut s, &b).unwrap().datos().to_vec()).unwrap();
    assert!(out.starts_with("Se habló con Persona Ficticia "), "{out}");
    sin(&out, &["ukasz", "Nowakficticio"]);
}
#[test]
fn a1_apostrofo_no_deja_apellidos_sueltos() {
    // Python deja «D’Angelo O’Higgins»; Rust reemplaza el nombre completo y sus partes.
    let (s, _) = texto("f.csv", "RIT,NOMBRE\nC-1-2099,Ana María D’Angeloficticio O’Higginsficticio\n".as_bytes(), estricto());
    sin(&s, &["Angeloficticio", "Higginsficticio"]);
}

// A2: un archivo que falla no cancela el lote; el informe lo cita sin su nombre.
#[test]
fn a2_lote_publica_los_validos_e_informa_los_rechazados() {
    let p = temp();
    let (entrada, salida) = (p.join("entrada"), p.join("salida"));
    std::fs::create_dir(&entrada).unwrap();
    std::fs::create_dir(&salida).unwrap();
    std::fs::write(entrada.join("a_bueno.txt"), b"Nombre: Identcanario Apelcanario\n").unwrap();
    std::fs::write(entrada.join("b_Privadocanario.txt"), "Texto con marca \u{e000} interna\n".as_bytes()).unwrap();
    std::fs::write(entrada.join("c.pdf"), b"%PDF-1.7 ficticio").unwrap();
    std::fs::write(entrada.join(".oculto.txt"), b"Nombre: Ocultocanario Apelcanario\n").unwrap();
    std::fs::create_dir(entrada.join(".git")).unwrap();
    std::fs::write(entrada.join(".git").join("x.txt"), b"hola\n").unwrap();
    let r = lote::ejecutar(std::slice::from_ref(&entrada), &salida, estricto(), &AtomicBool::new(false), |_| {}).unwrap();
    assert_eq!(r.informe.archivos.len(), 1);
    assert_eq!(r.informe.excluidos, 2);
    assert_eq!(r.informe.rechazados.iter().map(|x| x.entrada.as_str()).collect::<Vec<_>>(), ["Archivo 2", "Archivo 3"]);
    let informe = std::fs::read_to_string(r.carpeta.join("INFORME.json")).unwrap();
    assert!(informe.contains("\"rechazados\""));
    sin(&informe, &["Privadocanario", "Identcanario", "Ocultocanario", ".pdf"]);
    // Con «rechazar todo», el mismo lote no publica nada.
    let otra = p.join("otra");
    std::fs::create_dir(&otra).unwrap();
    let o = Opciones { rechazar_no_admitidos: true, ..estricto() };
    assert!(lote::ejecutar(&[entrada], &otra, o, &AtomicBool::new(false), |_| {}).is_err());
    assert_eq!(std::fs::read_dir(&otra).unwrap().count(), 0);
    std::fs::remove_dir_all(p).unwrap();
}

// A3, A4, A7: encabezados como `grid.header_row` y `_header`.
#[test]
fn a3_a4_encabezados_se_conservan_y_nombre_basta_para_detectarlos() {
    // Python: «Nombre,Edad,Programa,Comuna / Persona Ficticia 1,12,PRM X,Lota».
    let (s, _) = texto("n.csv", "Nombre,Edad,Programa,Comuna\nAna Canario Soto,12,PRM X,Lota\n".as_bytes(), estricto());
    assert_eq!(s, "Nombre,Edad,Programa,Comuna\nPersona Ficticia 1,12,PRM X,Lota\n");
    let mut libro = rust_xlsxwriter::Workbook::new();
    let hoja = libro.add_worksheet();
    for (c, v) in ["Nombre", "Edad", "Programa"].iter().enumerate() {
        hoja.write_string(0, c as u16, *v).unwrap();
    }
    hoja.write_string(1, 0, "Ana Canario Soto").unwrap();
    hoja.write_number(1, 1, 12).unwrap();
    hoja.write_string(1, 2, "PRM X").unwrap();
    let out = formatos::anonimizar("nom.xlsx", &libro.save_to_buffer().unwrap(), estricto()).unwrap();
    let l = flow_tabular::leer_bytes("s.xlsx", out.datos().to_vec()).unwrap();
    let filas = l.hojas[0].filas.iter().map(|f| f.iter().map(|c| c.texto()).collect::<Vec<_>>()).collect::<Vec<_>>();
    assert_eq!(filas[0], ["Nombre", "Edad", "Programa"]);
    assert_eq!(filas[1][0], "Persona Ficticia 1");
    assert_eq!(filas[1][2], "PRM X");
    // Estricto conserva títulos desconocidos (antes «CAMPO n»).
    let (s, _) = texto("t.csv", b"RIT,Nombre NNA,Observaciones del caso,Comuna\nC-1-2099,Ana Canario Soto,algo,Lota\n", estricto());
    assert!(s.starts_with("RIT,Nombre NNA,Observaciones del caso,Comuna\n"), "{s}");
}
#[test]
fn a7_encabezado_protegido_no_dispara_la_revision_final() {
    let (s, _) = texto("h.csv", b"RIT,NOMBRE\nC-1-2099,Sin nombre\n", estricto());
    assert!(s.starts_with("RIT,NOMBRE\n") && s.contains("Persona Ficticia"));
}

// A5: formatos que Python acepta.
#[test]
fn a5_texto_y_json_como_python() {
    // `.txt/.log/.md/.csv` que empiezan con «[» no son JSON.
    let (s, e) = texto("log1.txt", b"[2026-01-05 10:00] Contactar a Pedro Canario Rojas\nRIT: C-1-2099\n", conservador());
    assert_eq!(e, "txt");
    assert!(s.starts_with("[2026-01-05 10:00] Contactar a [identidad omitida]\nRIT: C-"), "{s}");
    sin(&s, &["Canario", "C-1-2099"]);
    let (s, e) = texto("md1.md", "[enlace](x)\nNombre: Ana Canario Muñoz\n".as_bytes(), conservador());
    assert_eq!((s.as_str(), e.as_str()), ("[enlace](x)\nNombre: Persona Ficticia 1\n", "md"));
    let (s, e) = texto("csv1.csv", b"[x],y\n1,2\n", conservador());
    assert_eq!(e, "csv");
    assert!(s.starts_with("[x],"));
    // NDJSON, JSON por líneas con extensión .json, NaN.
    assert_eq!(
        texto("nd.ndjson", b"{\"a\":\"x\"}\n{\"a\":\"y\"}\n", conservador()),
        ("{\"a\":\"x\"}\n{\"a\":\"y\"}\n".into(), "ndjson".into())
    );
    assert_eq!(
        texto("j1.json", b"{\"a\":\"x\"}\n{\"a\":\"y\"}\n", conservador()),
        ("{\"a\":\"x\"}\n{\"a\":\"y\"}\n".into(), "json".into())
    );
    let (s, _) = texto("nan.json", b"{\"a\":NaN,\"b\":[Infinity,-Infinity],\"c\":\"NaN\"}", conservador());
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v, serde_json::json!({"a":null,"b":[null,null],"c":"NaN"}));
    // Sin extensión y `.text` son texto.
    assert_eq!(texto("noext", "hola Ana Canario\n".as_bytes(), conservador()), ("hola [identidad omitida]\n".into(), "txt".into()));
    assert_eq!(
        texto("x.text", "Nombre: Ana Canario Soto\n".as_bytes(), estricto()),
        ("Nombre: Persona Ficticia 1\n".into(), "text".into())
    );
    // Salto de página: Python `splitlines` lo trata como fin de línea.
    let (s, _) = texto("ff.txt", b"linea uno\x0cNombre: Ana Canario Soto\nRUT: 11.111.111-1\n", conservador());
    assert!(s.starts_with("linea uno\nNombre: Persona Ficticia 1\nRUT: "), "{s}");
    sin(&s, &["11.111.111-1", "11111111", "\x0c"]);
}
#[test]
fn a5_csv_con_barra_html_e_ics_tolerantes() {
    assert_eq!(texto("pipe.csv", "RIT|Nombre\nC-1-2099|Ana Canario Soto\n".as_bytes(), estricto()).1, "csv");
    let (s, _) = texto("pipe.csv", "RIT|Nombre\nC-1-2099|Ana Canario Soto\n".as_bytes(), estricto());
    assert!(s.starts_with("RIT|Nombre\nC-") && s.ends_with("|Persona Ficticia 1\n"), "{s}");
    // HTML detectado como `formats.sniff`; .xls HTML sin tablas sale como documento.
    let (s, e) = texto("h1.html", "<meta charset=utf-8><div>Nombre: Ana Canario Soto</div>".as_bytes(), estricto());
    assert_eq!(e, "html");
    assert!(s.contains("Nombre: Persona Ficticia 1"));
    assert_eq!(texto("h2.html", b"<!-- c --><html><body><p>Hola</p></body></html>", estricto()).1, "html");
    let (s, e) = texto("h3.xls", b"<html><body><p>sin tablas</p></body></html>", conservador());
    assert_eq!(e, "html");
    assert!(s.contains("<p>sin tablas</p>"));
    // ICS de Outlook: zona de Windows, parámetro, componente y parte de regla desconocidos.
    let ics = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:abc\r\nDTSTART;TZID=Pacific SA Standard Time;X-FOO=Identcanario:20260105T100000\r\nRRULE:FREQ=WEEKLY;X-NAME=Identcanario;COUNT=3\r\nSUMMARY:Audiencia\r\nORGANIZER;CN=Identcanario:mailto:a@example.test\r\nEND:VEVENT\r\nBEGIN:VJOURNAL\r\nDESCRIPTION:Identcanario\r\nEND:VJOURNAL\r\nEND:VCALENDAR\r\n";
    let (s, _) = texto("o.ics", ics.as_bytes(), conservador());
    assert!(s.contains("DTSTART;TZID=Pacific SA Standard Time:20260105T100000\r\n"), "{s}");
    assert!(s.contains("RRULE:FREQ=WEEKLY;COUNT=3\r\n"));
    sin(&s, &["Identcanario", "X-FOO", "VJOURNAL", "a@example.test"]);
    // Zona con caracteres fuera del patrón cerrado: se omite el parámetro, no el evento.
    let (s, _) = texto(
        "z.ics",
        b"BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDTSTART;TZID=Ana Canario <a@example.test>:20260105T100000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        conservador(),
    );
    assert!(s.contains("DTSTART:20260105T100000"));
    sin(&s, &["Canario", "a@example.test"]);
}
#[test]
fn a5_zip_normaliza_rutas_como_safezip() {
    let bytes = documentos::zip(&[
        ("./a.txt".into(), b"hola\n".to_vec()),
        ("sub\\b.txt".into(), b"Nombre: Ana Canario Soto\n".to_vec()),
        ("sub/c:d.txt".into(), b"chao\n".to_vec()),
    ])
    .unwrap();
    let entries = formatos::validar_zip(&bytes).unwrap();
    assert_eq!(entries.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["a.txt", "sub/b.txt", "sub/c:d.txt"]);
    let s = formatos::anonimizar("z.zip", &bytes, estricto()).unwrap();
    sin(&String::from_utf8_lossy(s.datos()), &["Canario"]);
    // Siguen rechazados: unidad en el primer segmento, «..», raíz absoluta y repetidos tras normalizar.
    for malos in [vec!["C:x.txt"], vec!["a/../b.txt"], vec!["/a.txt"], vec!["a.txt", "./A.txt"]] {
        let z = documentos::zip(&malos.iter().map(|n| (n.to_string(), b"x".to_vec())).collect::<Vec<_>>()).unwrap();
        assert!(formatos::validar_zip(&z).is_err(), "{malos:?}");
    }
}

// A6: etiqueta de hasta 40 caracteres; la etiqueta con un RIT se depura en vez de rechazar el archivo.
#[test]
fn a6_etiquetas_de_python_y_rit_en_la_etiqueta() {
    let (s, _) = texto("r.txt", b"RIT C-1-2099 de la causa: audiencia fijada\nNombre: Ana Canario Soto\n", conservador());
    assert!(s.contains("Nombre: Persona Ficticia"), "{s}");
    sin(&s, &["C-1-2099", "Canario"]);
    let (s, _) = texto("r.txt", b"RIT C-1-2099 de la causa: audiencia fijada\n", estricto());
    assert_eq!(s, "[texto omitido]\n");
    // Más de 40 caracteres: no es etiqueta (Python `KV_RE`).
    let (s, _) = texto("l.txt", b"Nombre completo del nino que ingresa al programa: Ana Canario\n", estricto());
    assert_eq!(s, "[texto omitido]\n");
}

// A8: resumen.json del Descargador Rust.
#[test]
fn a8_nombre_de_archivo_no_es_persona_ni_contamina_claves() {
    let json = br#"{"estado":"VALIDADA","archivos":[{"nombre":"consulta-001-p001-de-001.xls","consulta":0,"pagina":1}],"paginas_lectura":[{"consulta":0,"vinculos":[{"rit":"C-1-2099","nombre":"Ana Canario Soto","programa":"FAE X"}]}]}"#;
    let (s, _) = texto("resumen.json", json, conservador());
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["archivos"][0]["consulta"], 0);
    assert_eq!(v["paginas_lectura"][0]["consulta"], 0);
    assert!(v["archivos"][0]["nombre"].as_str().unwrap().starts_with("fuente_"));
    assert!(v["paginas_lectura"][0]["vinculos"][0]["nombre"].as_str().unwrap().starts_with("Persona Ficticia "));
    sin(&s, &["Nombre1", "Apellido", "consulta-001", "Canario"]);
    // Un nombre de archivo con un nombre propio no se publica literal y el nombre se reemplaza en otro lugar.
    let (s, _) = texto("r.json", br#"{"nombre":"Ana_Canario_Soto.xlsx","nota":"Ana Canario Soto"}"#, conservador());
    sin(&s, &["Canario"]);
    // Los nombres de archivo del Descargador (`lotes.file_base`, con espacios) también son nombres de archivo.
    let json = br#"{"archivos":[{"nombre":"ESP Ambulatorio Jdo Familia Ficticio Norte 2.xls","consulta":0}]}"#;
    let (s, _) = texto("resumen.json", json, conservador());
    let v: Value = serde_json::from_str(&s).unwrap();
    assert!(v["archivos"][0]["nombre"].as_str().unwrap().starts_with("fuente_"), "{s}");
}

// A9: fechas y lógicos de Excel conservan su tipo.
#[test]
fn a9_excel_conserva_fecha_y_logico() {
    use calamine::{Data, Reader};
    let mut libro = rust_xlsxwriter::Workbook::new();
    let hoja = libro.add_worksheet();
    for (c, v) in ["RIT", "Nombre NNA", "FEC. NACIMIENTO", "Flag"].iter().enumerate() {
        hoja.write_string(0, c as u16, *v).unwrap();
    }
    hoja.write_string(1, 0, "C-12-2099").unwrap();
    hoja.write_string(1, 1, "Ana Canario Soto").unwrap();
    let f = rust_xlsxwriter::Format::new().set_num_format("yyyy-mm-dd");
    hoja.write_datetime_with_format(1, 2, rust_xlsxwriter::ExcelDateTime::from_ymd(2015, 3, 4).unwrap(), &f).unwrap();
    hoja.write_boolean(1, 3, true).unwrap();
    let o = Opciones { desplazar_dias: 2, ..estricto() };
    let out = formatos::anonimizar("std.xlsx", &libro.save_to_buffer().unwrap(), o).unwrap();
    let mut l = calamine::open_workbook_auto_from_rs(Cursor::new(out.datos().to_vec())).unwrap();
    let r = l.worksheet_range_at(0).unwrap().unwrap();
    match r.get_value((1, 2)) {
        Some(Data::DateTime(d)) => assert_eq!(d.as_datetime().unwrap().date().to_string(), "2015-03-06"),
        otro => panic!("{otro:?}"),
    }
    assert_eq!(r.get_value((1, 3)), Some(&Data::Bool(true)));
    let o = Opciones { generalizar_nacimiento: true, ..estricto() };
    let out = formatos::anonimizar("std.xlsx", &libro.save_to_buffer().unwrap(), o).unwrap();
    let mut l = calamine::open_workbook_auto_from_rs(Cursor::new(out.datos().to_vec())).unwrap();
    let r = l.worksheet_range_at(0).unwrap().unwrap();
    assert!(matches!(r.get_value((1, 2)), Some(Data::DateTime(d)) if d.as_datetime().unwrap().date().to_string() == "2015-01-01"));
}

// A10: estado del ZIP genérico.
#[test]
fn a10_zip_conserva_estado_conocido_y_nada_mas_del_manifiesto() {
    let manifiesto = br#"{"estado":"INCOMPLETO","autor":"Identcanario","archivos":[{"archivo":"sub/a.txt"}]}"#;
    let bytes =
        documentos::zip(&[("sub/a.txt".into(), b"Nombre: Ana Canario Soto\n".to_vec()), ("manifiesto.json".into(), manifiesto.to_vec())])
            .unwrap();
    let s = formatos::anonimizar("z.zip", &bytes, estricto()).unwrap();
    let entries = formatos::validar_zip(s.datos()).unwrap();
    let m: Value = serde_json::from_slice(&entries.iter().find(|(n, _)| n == "MANIFIESTO.json").unwrap().1).unwrap();
    assert_eq!(m["estado"], "INCOMPLETO");
    let indice = String::from_utf8(entries.iter().find(|(n, _)| n == "INDICE.txt").unwrap().1.clone()).unwrap();
    assert!(indice.starts_with("INCOMPLETO\n"));
    sin(&String::from_utf8_lossy(s.datos()), &["Identcanario", "Canario"]);
    // Estado desconocido: se publica VERIFICADO, no el texto original.
    let bytes =
        documentos::zip(&[("a.txt".into(), b"hola\n".to_vec()), ("INDICE.txt".into(), b"Identcanario Apelcanario\n".to_vec())]).unwrap();
    let s = formatos::anonimizar("z.zip", &bytes, estricto()).unwrap();
    let entries = formatos::validar_zip(s.datos()).unwrap();
    let indice = String::from_utf8(entries.iter().find(|(n, _)| n == "INDICE.txt").unwrap().1.clone()).unwrap();
    assert!(indice.starts_with("VERIFICADO\n"));
}
