use flow_anonimo::{documentos, formatos, Modo, Opciones};

fn opciones() -> Opciones {
    Opciones { modo: Modo::Conservador, semilla: "SEMILLA_FICTICIA".into(), ..Default::default() }
}

#[test]
fn html_solo_contiene_texto_y_tablas_tratados() {
    let original=br#"<!DOCTYPE html><html><head><title>Cabecera ficticia</title><script>CLAVE_FICTICIA</script></head><body><p>Nombre: Identcanario Apelcanario</p><table onclick="CLAVE_FICTICIA"><tr><th>RIT</th><th>Nombre</th><th>T ESPERA</th></tr><tr><td>P-1-2099</td><td>Identcanario Apelcanario</td><td>45</td></tr></table><input name="password" value="CLAVE_FICTICIA"><a href="https://example.invalid/privado">Enlace</a><img src="https://example.invalid/imagen.png"></body></html>"#;
    let salida = formatos::anonimizar("pagina.html", original, opciones()).unwrap();
    assert_eq!(salida.extension(), "html");
    let s = std::str::from_utf8(salida.datos()).unwrap();
    assert!(s.contains("<table>") && s.contains("<td>45</td>"));
    for privado in ["Identcanario", "Apelcanario", "CLAVE_FICTICIA", "onclick", "href=", "<img", "<script", "<input"] {
        assert!(!s.contains(privado));
    }
}

#[test]
fn ics_preserva_hora_zona_y_relaciones_sin_asistentes_ni_alarmas() {
    let original="BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:evento-ficticio\r\nDTSTART;TZID=America/Santiago:20261008T151617\r\nDTEND;TZID=America/Santiago:20261008T161617\r\nSUMMARY:Nombre: Identcanario Apelcanario\r\nDESCRIPTION:Texto de prueba con ñññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññññ\r\nATTENDEE;CN=Identcanario:mailto:ficticio@example.invalid\r\nBEGIN:VALARM\r\nACTION:EMAIL\r\nDESCRIPTION:CLAVE_FICTICIA\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let out = formatos::anonimizar("agenda.ics", original.as_bytes(), Opciones { desplazar_dias: 10, ..opciones() }).unwrap();
    let s = std::str::from_utf8(out.datos()).unwrap();
    assert!(s.contains("DTSTART;TZID=America/Santiago:20261018T151617"));
    assert!(s.contains("DTEND;TZID=America/Santiago:20261018T161617"));
    assert!(s.contains("DTSTAMP:19700101T000000Z"));
    for privado in ["evento-ficticio", "Identcanario", "Apelcanario", "ATTENDEE", "VALARM", "CLAVE_FICTICIA"] {
        assert!(!s.contains(privado));
    }
    assert!(s.lines().all(|l| l.len() <= 75));
    assert!(!out.avisos().is_empty());
    assert!(formatos::anonimizar("mala.ics", b"BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART:ilegible\nEND:VEVENT\nEND:VCALENDAR", opciones())
        .is_err());
}

fn word(media: bool) -> Vec<u8> {
    let mut entradas=vec![
        ("word/document.xml".into(),br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Nom</w:t></w:r><w:r><w:t>bre: Identcanario &amp; Apelcanario</w:t></w:r></w:p><w:p><w:r><w:t>Se conversa con Identcanario.</w:t></w:r></w:p></w:body></w:document>"#.to_vec()),
        ("word/header1.xml".into(),br#"<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>RIT: P-1-2099</w:t></w:r></w:p></w:hdr>"#.to_vec()),
        ("word/comments.xml".into(),br#"<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:comment w:author="AUTOR_FICTICIO"><w:p><w:r><w:t>Nombre: Identcanario Apelcanario</w:t></w:r></w:p></w:comment></w:comments>"#.to_vec()),
        ("docProps/core.xml".into(),b"<metadata>AUTOR_FICTICIO CLAVE_FICTICIA</metadata>".to_vec()),
        ("word/_rels/document.xml.rels".into(),br#"<Relationships><Relationship Target="https://example.invalid/privado" TargetMode="External"/></Relationships>"#.to_vec()),
    ];
    if media {
        entradas.push(("word/media/imagen.png".into(), b"imagen ficticia".to_vec()));
    }
    documentos::zip(&entradas).unwrap()
}

#[test]
fn word_trata_parrafos_entre_runs_y_descarta_metadatos_y_relaciones() {
    let salida = formatos::anonimizar("proyecto.docx", &word(false), opciones()).unwrap();
    assert_eq!(salida.extension(), "docx");
    let entries = formatos::validar_zip(salida.datos()).unwrap();
    assert_eq!(entries.len(), 3);
    let todos = entries.iter().map(|(_, s)| String::from_utf8_lossy(s)).collect::<Vec<_>>().join("\n");
    for privado in ["Identcanario", "Apelcanario", "AUTOR_FICTICIO", "CLAVE_FICTICIA", "TargetMode=\"External\"", "example.invalid/privado"]
    {
        assert!(!todos.contains(privado));
    }
    let parrafos = documentos::parrafos_docx(&entries).unwrap();
    assert!(parrafos.len() >= 4);
    assert!(parrafos.iter().any(|s| s.contains("Nombre:")));
    assert!(formatos::anonimizar("imagen.docx", &word(true), opciones()).is_err());
}

#[test]
fn dimensiones_excel_excesivas_se_rechazan_antes_del_lector() {
    for xml in [
        r#"<worksheet><dimension ref="A1:XFD1048576"/></worksheet>"#,
        r#"<worksheet><row r="1048576"><c r="XFD1048576"/></row></worksheet>"#,
        r#"<sst uniqueCount="18446744073709551615"/>"#,
    ] {
        let zip = documentos::zip(&[
            ("xl/workbook.xml".into(), b"<workbook/>".to_vec()),
            ("xl/worksheets/sheet1.xml".into(), xml.as_bytes().to_vec()),
        ])
        .unwrap();
        let error = match formatos::cargar("grande.xlsx", &zip) {
            Ok(_) => panic!("No rechazó el archivo ficticio."),
            Err(e) => e,
        };
        assert!(error.contains("supera") || error.contains("dimensiones"));
    }
}

#[test]
fn estructuras_e_identidades_excesivas_no_llegan_a_publicarse() {
    let html = format!("<html><body>{}texto{}</body></html>", "<div>".repeat(1000), "</div>".repeat(1000));
    assert!(formatos::cargar("profundo.html", html.as_bytes()).is_err());
    let json = serde_json::json!({"Nombre":"Identcanario ".repeat(100)});
    assert!(formatos::anonimizar("nombre.json", &serde_json::to_vec(&json).unwrap(), opciones()).is_err());
}
#[test]
fn excel_preserva_coordenadas_de_tabla_desplazada(){
 use calamine::Reader;
 let mut w=rust_xlsxwriter::Workbook::new();let h=w.add_worksheet();h.write_string(2,1,"RIT").unwrap();h.write_string(2,2,"Nombre").unwrap();h.write_string(3,1,"P-1-2099").unwrap();h.write_string(3,2,"Identcanario Apelcanario").unwrap();
 let salida=formatos::anonimizar("desplazado.xlsx",&w.save_to_buffer().unwrap(),opciones()).unwrap();let mut libro=calamine::open_workbook_auto_from_rs(std::io::Cursor::new(salida.datos().to_vec())).unwrap();let rango=libro.worksheet_range_at(0).unwrap().unwrap();assert_eq!(rango.start(),Some((2,1)));assert_eq!(rango.get_value((2,1)),Some(&calamine::Data::String("RIT".into())));assert!(matches!(rango.get_value((3,2)),Some(calamine::Data::String(s)) if !s.contains("Identcanario")));
}
