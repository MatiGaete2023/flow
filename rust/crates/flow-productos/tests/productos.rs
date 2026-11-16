//! Productos completos a partir de un libro ficticio: Excel de observaciones, borradores con CC fijo y Word.

use flow_dominio::{Etapa, CC_FIJO};
use flow_motor::Config;
use flow_productos::correos::{self, Contactos, Opciones, Tipo};
use flow_productos::{excel, word};
use flow_trabajo::{analizar_tabla, Contexto, Trabajo};
use std::path::PathBuf;

fn trabajo(etapa: Etapa) -> Trabajo {
    let ruta = format!("{}/../../../docs/evidencia_p0/golden/Origen_{}.xlsx", env!("CARGO_MANIFEST_DIR"), etapa.nombre());
    let libro = flow_tabular::leer(std::path::Path::new(&ruta)).unwrap();
    let tabla = flow_tabular::leer_tabla(&libro, Some(etapa), None).unwrap();
    analizar_tabla(&libro, &tabla, "2026-10-08".parse().unwrap(), &Config::default(), &Contexto::default())
}

fn carpeta(nombre: &str) -> PathBuf {
    let c = std::env::temp_dir().join(format!("flow-prueba-{}-{nombre}", std::process::id()));
    let _ = std::fs::remove_dir_all(&c);
    std::fs::create_dir_all(&c).unwrap();
    c
}

#[test]
fn excel_de_observaciones_se_relee() {
    let t = trabajo(Etapa::Espera);
    let dir = carpeta("excel");
    let destino = excel::ruta_nueva(&dir, "Observaciones ESPERA", "xlsx");
    excel::observaciones(&t, &destino).unwrap();
    assert!(excel::observaciones(&t, &destino).is_err(), "no sobrescribe");
    let libro = flow_tabular::leer(&destino).unwrap();
    let hoja = &libro.hojas[0];
    assert_eq!(hoja.nombre, "ESPERA");
    let cabeceras: Vec<String> = hoja.filas[0].iter().map(|c| c.texto()).collect();
    let col = cabeceras.iter().position(|c| c == "OBSERVACION").unwrap();
    assert_eq!(hoja.filas[1][col].texto(), t.filas[0].observacion);
    assert_eq!(hoja.filas[1][cabeceras.iter().position(|c| c == "RES").unwrap()].texto(), "PC_IE");
    // El libro generado sigue siendo una tabla de Espera legible por Flow.
    assert_eq!(flow_tabular::leer_tabla(&libro, None, None).unwrap().filas.len(), t.filas.len());
    // Las tres etapas y una tabla sin filas.
    for etapa in [Etapa::Espera, Etapa::Cumplimiento, Etapa::Informes] {
        let mut t = trabajo(etapa);
        let destino = excel::ruta_nueva(&dir, &format!("Observaciones {}", etapa.nombre()), "xlsx");
        excel::observaciones(&t, &destino).unwrap();
        let cabeceras: Vec<String> = flow_tabular::leer(&destino).unwrap().hojas[0].filas[0].iter().map(|c| c.texto()).collect();
        for titulo in excel::ANOTADAS {
            assert_eq!(cabeceras.iter().filter(|c| excel::cabecera_normalizada(c) == excel::cabecera_normalizada(titulo)).count(), 1);
        }
        t.filas.clear();
        excel::observaciones(&t, &excel::ruta_nueva(&dir, "Vacía", "xlsx")).unwrap();
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn borradores_llevan_cc_fijo_y_nomina() {
    let t = trabajo(Etapa::Espera);
    let dir = carpeta("correos");
    let mut contactos = Contactos::default();
    contactos.programas.insert("aft ficticio".into(), "programa@example.invalid".into());
    contactos.cc_adicional = "otra@example.invalid".into();
    let lista = correos::preparar(&t, Tipo::ProgramaEspera, &Opciones::default(), &contactos, &dir).unwrap();
    assert!(!lista.is_empty());
    for b in &lista {
        assert!(b.cc.starts_with(CC_FIJO), "{}", b.cc);
        assert_eq!(b.adjuntos.len(), 1);
        assert!(b.adjuntos[0].exists());
        assert!(b.asunto.contains("Lista de espera"));
    }
    let aft = lista.iter().find(|b| b.programa == "AFT FICTICIO").unwrap();
    assert_eq!(aft.para, "programa@example.invalid");
    assert!(aft.faltantes().is_empty());
    let sin_contacto = lista.iter().find(|b| b.programa != "AFT FICTICIO").unwrap();
    assert_eq!(sin_contacto.faltantes(), vec!["destinatario"]);
    let eml = String::from_utf8(correos::eml(aft).unwrap()).unwrap();
    assert!(eml.starts_with("X-Unsent: 1\r\n"));
    assert!(eml.contains(&format!("Cc: {CC_FIJO}, otra@example.invalid")));
    // Al tribunal: exige las modalidades y no adjunta.
    assert!(correos::preparar(&t, Tipo::Informativo, &Opciones::default(), &contactos, &dir).is_err());
    let opciones = Opciones { modalidades: "Intervención ambulatoria de reparación".into(), ..Opciones::default() };
    let informativos = correos::preparar(&t, Tipo::Informativo, &opciones, &contactos, &dir).unwrap();
    assert!(informativos.iter().all(|b| b.adjuntos.is_empty() && b.cuerpo.contains("Intervención ambulatoria")));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn proyectos_word_por_tribunal() {
    let t = trabajo(Etapa::Espera);
    let dir = carpeta("word");
    let hoy = "2026-10-08".parse().unwrap();
    let mut lista = word::proyectos(&t, hoy, None);
    assert!(lista.iter().all(|p| p.tipo == "PC_IE"));
    let laja = lista.iter().find(|p| p.tribunal == flow_dominio::Tribunal::Laja).unwrap();
    assert!(laja.listo(), "{:?}", laja.faltantes);
    assert_eq!(laja.valores["FECHA"], "ocho de octubre de dos mil veintiséis");
    // Lo que falta queda como «[COMPLETAR X]» y no impide generar (como el Asistente Python).
    for p in lista.iter_mut().filter(|p| !p.faltantes.is_empty()) {
        assert!(p.listo() || p.sin_plantilla, "{:?}", p.faltantes);
        assert!(p.faltantes.iter().all(|v| p.valores[v].starts_with("[COMPLETAR ")));
        for clave in p.faltantes.clone() {
            p.valores.insert(clave, "dato completado a mano".into());
        }
        p.actualizar(None);
        assert!(p.faltantes.is_empty());
    }
    let rutas = word::generar(&lista, &dir, None).unwrap();
    assert!(!rutas.is_empty());
    for ruta in &rutas {
        let datos = std::fs::read(ruta).unwrap();
        assert!(datos.starts_with(b"PK"));
    }
    println!("{}", rutas.iter().map(|r| r.display().to_string()).collect::<Vec<_>>().join("\n"));
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn consultas_auxiliares_no_producen_correos_ni_proyectos() {
    let mut t = trabajo(Etapa::Espera);
    let dir = carpeta("consulta-sin-acciones");
    for tipo in ["EGRESADOS", "BUSQUEDA_LITIGANTES"] {
        t.fuente = Some(tipo.into());
        for correo in Tipo::TODOS {
            assert!(correos::preparar(
                &t,
                correo,
                &Opciones { modalidades: "FAE".into(), ..Default::default() },
                &Contactos::default(),
                &dir
            )
            .unwrap()
            .is_empty());
        }
        assert!(word::proyectos(&t, t.corte, None).is_empty());
    }
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn contactos_ilegibles_no_se_reemplazan_y_guardado_se_relee() {
    let p = carpeta("contactos-protegidos");
    let ruta = p.join("contactos.json");
    assert_eq!(Contactos::cargar(&ruta).unwrap(), Contactos::default());
    let malo = br#"{"programas": {"programa ficticio":"ficticio@example.invalid"}"#;
    std::fs::write(&ruta, malo).unwrap();
    assert!(Contactos::cargar(&ruta).is_err());
    assert!(Contactos::default().guardar(&ruta).is_err());
    assert_eq!(std::fs::read(&ruta).unwrap(), malo);
    std::fs::remove_file(&ruta).unwrap();
    let mut contactos = Contactos::default();
    contactos.firma = "Firma ficticia".into();
    contactos.guardar(&ruta).unwrap();
    assert_eq!(Contactos::cargar(&ruta).unwrap(), contactos);
    contactos.cc_adicional = "ficticio@example.invalid".into();
    contactos.guardar(&ruta).unwrap();
    assert_eq!(Contactos::cargar(&ruta).unwrap(), contactos);
    assert_eq!(std::fs::read_dir(&p).unwrap().count(), 1);
    std::fs::remove_dir_all(p).unwrap();
}

#[test]
fn modalidad_como_python() {
    // Casos de `src/nurus/personal/modalities.py::modality`.
    for (texto, clave) in [
        ("Cuidado alternativo residencial", "RES"),
        ("Diagnóstico Clínico Especializado (DCE)", "DCE"),
        ("DCE Ficticio", "DCE"),
        ("FAS-Ficticio", "FAE"),
        ("REMxx Ficticia", "RES"),
        ("RLP Ficticia", "RES"),
        ("Residencia Ficticia", "RES"),
        ("PRM Ficticio", "AMB"),
        ("aft ficticio", "AMB"),
        ("PRMX Ficticio", ""),
        ("2", ""),
        ("", ""),
    ] {
        assert_eq!(correos::modalidad(texto), clave, "{texto}");
    }
    assert_eq!(correos::claves_modalidad(["Intervención ambulatoria de reparación"]), Some(["AMB"].into()));
    assert_eq!(correos::claves_modalidad(["Otra ficticia"]), None, "una modalidad desconocida no filtra");
    assert_eq!(correos::claves_modalidad(correos::MODALIDADES.map(|m| m.1)), None, "todas = sin filtro");
    assert_eq!(
        correos::claves_de_texto("Cuidado alternativo residencial y Diagnóstico Clínico Especializado (DCE)"),
        Some(["RES", "DCE"].into())
    );
}

#[test]
fn las_modalidades_elegidas_filtran_las_filas() {
    let mut t = trabajo(Etapa::Espera);
    let dir = carpeta("modalidades");
    let contactos = Contactos::default();
    let todas = Opciones { modalidades: "todas".into(), ..Opciones::default() };
    let filas = |t: &flow_trabajo::Trabajo, o: &Opciones| -> Vec<String> {
        correos::preparar(t, Tipo::Informativo, o, &contactos, &dir).unwrap().into_iter().flat_map(|b| b.filas).collect()
    };
    let total = filas(&t, &todas);
    // La modalidad de la consulta de SITFA es un código («2»): no se reconoce y decide el prefijo del programa.
    for f in &mut t.filas {
        f.registro.poner("modalidad", flow_dominio::Celda::Texto("2".into()));
    }
    let amb = Opciones { modalidades: "la modalidad ambulatoria".into(), claves_modalidad: Some(["AMB"].into()), ..Opciones::default() };
    let elegidas = filas(&t, &amb);
    let esperadas: Vec<String> = t
        .filas
        .iter()
        .filter(|f| total.contains(&f.id) && correos::modalidad(&f.registro.texto("programa")) == "AMB")
        .map(|f| f.id.clone())
        .collect();
    assert!(!elegidas.is_empty() && elegidas.len() < total.len(), "{} de {}", elegidas.len(), total.len());
    let mut a = elegidas.clone();
    let mut b = esperadas.clone();
    a.sort();
    b.sort();
    assert_eq!(a, b);
    // Una fila con modalidad explícita reconocida usa esa y no el programa.
    let otra = t.filas.iter().position(|f| !elegidas.contains(&f.id) && total.contains(&f.id)).unwrap();
    t.filas[otra].registro.poner("modalidad", flow_dominio::Celda::Texto("Intervención ambulatoria de reparación".into()));
    let id = t.filas[otra].id.clone();
    assert!(filas(&t, &amb).contains(&id));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn nombres_de_archivo_como_program_filename() {
    assert_eq!(excel::nombre_archivo("  PRM: Ficticio/Uno.  "), "PRM_ Ficticio_Uno");
    assert_eq!(excel::nombre_archivo("con"), "_con");
    assert_eq!(excel::nombre_archivo("LPT1.Ficticio"), "_LPT1.Ficticio");
    assert_eq!(excel::nombre_archivo("COM0"), "COM0");
    assert_eq!(excel::nombre_archivo(" . "), "Programa no informado");
    let largo = format!("{}. x", "ñ".repeat(119));
    let n = excel::nombre_archivo(&largo);
    assert_eq!(n, "ñ".repeat(119), "se corta en 120 caracteres sin partir una letra y sin punto final");
    let dir = carpeta("nombres");
    let ruta = excel::ruta_nueva(&dir, &"Nómina ".repeat(40), "xlsx");
    assert!(ruta.file_stem().unwrap().to_string_lossy().chars().count() <= 120);
    std::fs::remove_dir_all(dir).unwrap();
}

/// Textos visibles de los párrafos del cuerpo de un .docx.
fn parrafos_docx(ruta: &std::path::Path) -> Vec<String> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::fs::File::open(ruta).unwrap()).unwrap();
    let mut xml = String::new();
    zip.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
    let nodo = regex::Regex::new(r"<w:t(\s[^>]*)?>([^<]*)</w:t>").unwrap();
    xml.split("</w:p>")
        .map(|p| nodo.captures_iter(p).map(|c| c[2].replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")).collect::<String>())
        .filter(|t| !t.trim().is_empty())
        .collect()
}

/// Una causa ficticia con tres filas (la misma persona dos veces) y dos tipos en las acciones de la primera.
fn causa_ficticia(t: &mut Trabajo) -> [String; 3] {
    use flow_dominio::Celda;
    let datos = [
        ("Persona Ficticia Uno (hija)", "11.111.111-1", "PRM Ficticio", Some("2099-02-01")),
        ("Persona Ficticia Dos", "22.222.222-2", "AFT Ficticio", Some("2099-02-03")),
        ("Persona Ficticia Uno", "11.111.111-1", "PRM Ficticio", None),
    ];
    for (i, (nombre, rut, programa, resolucion)) in datos.into_iter().enumerate() {
        let f = &mut t.filas[i];
        for (campo, v) in [("rit", "X-9-2099"), ("tribunal", "Mulchén"), ("nombre", nombre), ("rut", rut), ("programa", programa)] {
            f.registro.poner(campo, Celda::Texto(v.into()));
        }
        f.registro.poner("resolucion", resolucion.map(|r| Celda::Fecha(r.parse().unwrap())).unwrap_or_default());
        f.propuesta.excluido = false;
        f.propuesta.acciones = if i == 0 { vec!["PC_INFO".into(), "NOMENCL".into()] } else { vec![] };
        f.propuesta.reglas.clear();
        f.observacion = "Observación ficticia sin tipo.".into();
    }
    [t.filas[0].id.clone(), t.filas[1].id.clone(), t.filas[2].id.clone()]
}

#[test]
fn proyectos_agrupados_por_causa_como_python() {
    let mut t = trabajo(Etapa::Espera);
    let ids = causa_ficticia(&mut t);
    // Todas las clases de proyecto de las acciones de la fila (`automatic_project_selections`).
    let sel = word::selecciones(&t);
    let de_causa: Vec<&(String, String)> = sel.iter().filter(|(id, _)| ids.contains(id)).collect();
    assert_eq!(de_causa, vec![&(ids[0].clone(), "PC_INFO".to_string()), &(ids[0].clone(), "NOMENCL".to_string())]);
    let hoy = "2099-01-15".parse().unwrap();
    let lista: Vec<word::Proyecto> = word::proyectos(&t, hoy, None).into_iter().filter(|p| p.rit == "X-9-2099").collect();
    assert_eq!(lista.iter().map(|p| p.tipo.as_str()).collect::<Vec<_>>(), ["PC_INFO", "NOMENCL"]);
    let info = &lista[0];
    // Todas las filas de la causa, aunque solo una tenga la acción (`prepare_projects`).
    assert_eq!(info.filas, ids.to_vec());
    // Valores de `grouped_values` obtenidos con el Python del Asistente sobre los mismos datos ficticios.
    assert_eq!(info.valores["NOMBRE"], "Persona Ficticia Uno y Persona Ficticia Dos");
    assert_eq!(info.valores["RUT"], "11.111.111-1 y 22.222.222-2");
    assert_eq!(info.valores["PROGRAMA"], "PRM Ficticio y AFT Ficticio");
    assert_eq!(info.valores["FECHA_RESOLUCION"], "uno de febrero de dos mil noventa y nueve y tres de febrero de dos mil noventa y nueve");
    assert!(info.plural.plural && info.plural.varios_programas && info.plural.varias_fechas);
    // La NOMENCLATURA sin regla que la defina queda por completar y no impide generar.
    let nomencl = &lista[1];
    assert_eq!(nomencl.valores["NOMENCLATURA"], "[COMPLETAR NOMENCLATURA]");
    assert_eq!(nomencl.faltantes, vec!["NOMENCLATURA".to_string()]);
    assert!(nomencl.listo());
    // Una observación que nombra el tipo manda sobre las acciones.
    t.filas[0].observacion = "Se solicita PC IE ficticio.".into();
    let sel = word::selecciones(&t);
    assert_eq!(sel.iter().filter(|(id, _)| ids.contains(id)).collect::<Vec<_>>(), vec![&(ids[0].clone(), "PC_IE".to_string())]);

    let dir = carpeta("word-plural");
    let rutas = word::generar(&lista[..1], &dir, None).unwrap();
    // Texto de `render_project` (Python) sobre MULCHEN_PC_INFO con los mismos valores.
    let esperado = [
        "Mulchén, quince de enero de dos mil noventa y nueve.",
        "Atendido que el plazo para la remisión del informe de avance pertinente se encuentra vencido y teniendo presente las facultades oficiosas del Tribunal contempladas en el artículo 13 de la Ley 19.968, se resuelve:",
        "Pídase cuenta a los organismos interventores, PRM Ficticio y AFT Ficticio, a fin de dar estricto cumplimiento a lo ordenado en la presente causa, en cuanto a la remisión del informe de cumplimiento, correspondiente a Persona Ficticia Uno, cédula de identidad N° 11.111.111-1 y Persona Ficticia Dos, cédula de identidad N° 22.222.222-2.",
        "Profesionales de dichos programas, deberán remitir dicho informe y dar cumplimiento en el término de 5 días desde su notificación o solicitar una prórroga para su entrega en caso de ser necesario.",
        "Notifíquese la presente resolución por correo electrónico a los programas intervinientes.",
        "Sirva la presente resolución de suficiente y atento oficio remisor.",
        "Personas comprendidas: Persona Ficticia Uno, cédula de identidad N° 11.111.111-1, programa PRM Ficticio; Persona Ficticia Dos, cédula de identidad N° 22.222.222-2, programa AFT Ficticio.",
        "RIT: X-9-2099",
        "Se proveyó y firmó mediante firma electrónica avanzada, según lo dispuesto en la ley 20.886 y acta 71-2016 de la Excelentísima Corte Suprema.",
        "En Mulchén, quince de enero de dos mil noventa y nueve, notifiqué por el estado diario la resolución precedente.",
    ];
    assert_eq!(parrafos_docx(&rutas[0]), esperado);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn word_se_genera_con_marcas_completar_y_conserva_ediciones() {
    let mut t = trabajo(Etapa::Espera);
    let ids = causa_ficticia(&mut t);
    t.filas[0].propuesta.acciones = vec!["NOMENCL".into()];
    let hoy = "2099-01-15".parse().unwrap();
    let mut lista: Vec<word::Proyecto> = word::proyectos(&t, hoy, None).into_iter().filter(|p| p.filas == ids.to_vec()).collect();
    assert_eq!(lista.len(), 1);
    assert!(lista[0].listo(), "las variables sin dato no bloquean: {:?}", lista[0].faltantes);
    let dir = carpeta("word-completar");
    let rutas = word::generar(&lista, &dir, None).unwrap();
    assert!(parrafos_docx(&rutas[0]).iter().any(|p| p.contains("\"[COMPLETAR NOMENCLATURA]\"")));
    // Una variable que el usuario deja vacía no se puede generar (`fill_docx`), sin impedir las demás.
    lista[0].valores.insert("NOMENCLATURA".into(), " ".into());
    lista[0].actualizar(None);
    assert_eq!(lista[0].vacias(), vec!["NOMENCLATURA"]);
    assert!(!lista[0].listo());
    // Al volver a preparar se conservan los valores editados (`merge_project_edits`).
    lista[0].valores.insert("NOMENCLATURA".into(), "Nomenclatura ficticia".into());
    let mut nuevos: Vec<word::Proyecto> = word::proyectos(&t, hoy, None).into_iter().filter(|p| p.filas == ids.to_vec()).collect();
    word::conservar_ediciones(&lista, &mut nuevos, None);
    assert_eq!(nuevos[0].valores["NOMENCLATURA"], "Nomenclatura ficticia");
    assert!(nuevos[0].faltantes.is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

fn cabeceras_y_filas(ruta: &std::path::Path) -> (Vec<String>, Vec<Vec<String>>) {
    let libro = flow_tabular::leer(ruta).unwrap();
    let filas: Vec<Vec<String>> = libro.hojas[0].filas.iter().map(|f| f.iter().map(|c| c.texto()).collect()).collect();
    (filas[0].clone(), filas[1..].to_vec())
}

#[test]
fn medidas_lleva_un_adjunto_por_programa() {
    let mut t = trabajo(Etapa::Cumplimiento);
    // Dos programas ficticios en la misma causa de un tribunal, con la acción de medidas.
    for (i, programa) in ["PRM Ficticio", "AFT Ficticio", "PRM Ficticio"].into_iter().enumerate() {
        let f = &mut t.filas[i];
        f.registro.poner("tribunal", flow_dominio::Celda::Texto("Laja".into()));
        f.registro.poner("programa", flow_dominio::Celda::Texto(programa.into()));
        f.propuesta.excluido = false;
        if !f.tiene_accion("medidas") {
            f.propuesta.acciones.push("medidas".into());
        }
    }
    let dir = carpeta("medidas");
    let opciones = Opciones { modalidades: "las modalidades ficticias".into(), ..Opciones::default() };
    let lista = correos::preparar(&t, Tipo::Medidas, &opciones, &Contactos::default(), &dir).unwrap();
    let laja = lista.iter().find(|b| b.filas.contains(&t.filas[0].id)).unwrap();
    let programas: Vec<String> = {
        let mut v: Vec<String> = Vec::new();
        for id in &laja.filas {
            let p = t.filas.iter().find(|f| &f.id == id).unwrap().registro.texto("programa").trim().to_string();
            if !v.contains(&p) {
                v.push(p);
            }
        }
        v
    };
    assert!(programas.len() >= 2);
    assert_eq!(laja.adjuntos.len(), programas.len(), "un adjunto por programa en el mismo borrador");
    for (ruta, programa) in laja.adjuntos.iter().zip(&programas) {
        assert_eq!(ruta.file_stem().unwrap().to_string_lossy(), excel::nombre_archivo(programa));
        let (cabeceras, filas) = cabeceras_y_filas(ruta);
        let col = cabeceras.iter().position(|c| c == "PROGRAMA").unwrap();
        assert!(filas.iter().all(|f| &f[col] == programa));
        assert_eq!(cabeceras[5], "F. EGRESO PROYECTADO");
    }
    let total: usize = laja.adjuntos.iter().map(|r| cabeceras_y_filas(r).1.len()).sum();
    assert_eq!(total, laja.filas.len());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn reporte_de_proyectos_y_gestion_particular() {
    let mut t = trabajo(Etapa::Espera);
    for f in &mut t.filas[..2] {
        f.registro.poner("tribunal", flow_dominio::Celda::Texto("Laja".into()));
        f.propuesta.excluido = false;
    }
    let dir = carpeta("proyectos-especial");
    let mut contactos = Contactos::default();
    contactos.tribunales.insert("LAJA".into(), "tribunal@example.test".into());
    assert!(!Tipo::AUTOMATICOS.contains(&Tipo::Proyectos) && !Tipo::AUTOMATICOS.contains(&Tipo::Especial));
    // Sin Word generado no hay reporte (`prepare_drafts`: solo filas con recibo «word»).
    let sin_word = Opciones { periodo: "01/2099".into(), ..Opciones::default() };
    assert!(correos::preparar(&t, Tipo::Proyectos, &sin_word, &contactos, &dir).unwrap().is_empty());
    let tribunal = |f: &flow_trabajo::FilaTrabajo| flow_dominio::Tribunal::desde(&f.registro.texto("tribunal"));
    let con: Vec<&flow_trabajo::FilaTrabajo> =
        t.filas.iter().filter(|f| !f.propuesta.excluido && tribunal(f) == Some(flow_dominio::Tribunal::Laja)).take(2).collect();
    assert_eq!(con.len(), 2);
    let opciones = Opciones { periodo: "01/2099".into(), con_word: con.iter().map(|f| f.id.clone()).collect(), ..Opciones::default() };
    let lista = correos::preparar(&t, Tipo::Proyectos, &opciones, &contactos, &dir).unwrap();
    assert_eq!(lista.len(), 1);
    let b = &lista[0];
    assert_eq!(b.filas.len(), 2);
    assert_eq!(b.para, "tribunal@example.test");
    assert!(b.cc.starts_with(CC_FIJO));
    assert!(b.asunto.starts_with("Reporte de proyectos de resolución — ") && b.asunto.ends_with(" — 01/2099"), "{}", b.asunto);
    assert!(b.cuerpo.contains("elaborados durante 01/2099"));
    assert!(!b.adjuntos.is_empty());
    let (cabeceras, _) = cabeceras_y_filas(&b.adjuntos[0]);
    assert_eq!(cabeceras, ["RIT", "TRIBUNAL", "RUT", "NOMBRE", "PROGRAMA", "T ESPERA", "OBSERVACION"]);
    // El período es obligatorio en esta plantilla.
    let sin_periodo = Opciones { periodo: String::new(), ..opciones.clone() };
    assert!(correos::preparar(&t, Tipo::Proyectos, &sin_periodo, &contactos, &dir).unwrap_err().contains("PERIODO"));
    // Gestión particular: sin modalidades, sin adjunto obligatorio, todas las filas del tribunal.
    let especial = correos::preparar(&t, Tipo::Especial, &Opciones::default(), &contactos, &dir).unwrap();
    assert!(!especial.is_empty());
    assert!(especial
        .iter()
        .all(|b| b.asunto.starts_with("Gestión particular RUS — ") && b.adjuntos.is_empty() && b.cc.starts_with(CC_FIJO)));
    let filas_especial: usize = especial.iter().map(|b| b.filas.len()).sum();
    let elegibles =
        t.filas.iter().filter(|f| !f.propuesta.excluido && flow_dominio::Tribunal::desde(&f.registro.texto("tribunal")).is_some()).count();
    assert_eq!(filas_especial, elegibles);
    let eml = String::from_utf8(correos::eml(&especial[0]).unwrap_or_else(|_| b"".to_vec())).unwrap();
    assert!(eml.is_empty() || eml.starts_with("X-Unsent: 1\r\n"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn excel_reutiliza_columnas_del_origen_y_conserva_tt_cc() {
    use flow_dominio::Celda;
    let mut t = trabajo(Etapa::Espera);
    let usadas = t.cabeceras.iter().rposition(|c| !c.is_empty()).map_or(0, |p| p + 1);
    t.cabeceras.truncate(usadas);
    t.cabeceras.extend(["OBSERVACION", "TT", "res"].map(String::from));
    for (i, f) in t.filas.iter_mut().enumerate() {
        f.celdas.resize(usadas, Celda::Vacia);
        let previa = if i < 2 { Celda::Texto(format!("Observación previa ficticia {i}")) } else { Celda::Vacia };
        f.celdas.extend([previa, Celda::Texto("1".into()), Celda::Vacia]);
    }
    // La fila 1 la edita el usuario (con «=» al comienzo: debe quedar como texto); la 0 no.
    t.filas[1].observacion = "=Edición ficticia del usuario".into();
    let dir = carpeta("excel-columnas");
    let destino = excel::ruta_nueva(&dir, "Observaciones", "xlsx");
    excel::observaciones(&t, &destino).unwrap();
    let (cabeceras, filas) = cabeceras_y_filas(&destino);
    let mut esperadas = t.cabeceras.clone();
    esperadas.extend(["FECHA_OBS", "CC", "REVISAR"].map(String::from));
    assert_eq!(cabeceras, esperadas, "OBSERVACION, TT y RES se reutilizan; no se duplican");
    let col = |n: &str| cabeceras.iter().position(|c| c == n).unwrap();
    assert_eq!(filas[0][col("OBSERVACION")], "Observación previa ficticia 0", "no se pisa una observación del origen");
    assert!(filas[0][col("REVISAR")].contains("Se conservó la OBSERVACION del origen; propuesta de Flow:"));
    assert!(!filas[2][col("REVISAR")].contains("Se conservó"));
    assert_eq!(filas[0][col("FECHA_OBS")], "");
    assert_eq!(filas[1][col("OBSERVACION")], "=Edición ficticia del usuario");
    assert_eq!(filas[1][col("FECHA_OBS")], "08/10/2026");
    assert_eq!(filas[2][col("OBSERVACION")], t.filas[2].observacion);
    assert!(filas.iter().all(|f| f[col("TT")] == "1" && f[col("CC")].is_empty()), "TT y CC no se calculan");
    assert_eq!(filas[0][col("res")], t.filas[0].proyecto().unwrap_or(""));
    std::fs::remove_dir_all(dir).unwrap();
}
