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
    // Lo que no esté listo (sin plantilla o con datos faltantes) no se genera y no impide el resto.
    for p in lista.iter_mut().filter(|p| !p.faltantes.is_empty()) {
        for clave in p.faltantes.clone() {
            p.valores.insert(clave, "dato completado a mano".into());
        }
        p.actualizar(None);
        assert!(p.listo());
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
