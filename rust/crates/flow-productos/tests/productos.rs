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
    let opciones = Opciones { modalidades: "Intervención ambulatoria de reparación".into(), hoy: None };
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
