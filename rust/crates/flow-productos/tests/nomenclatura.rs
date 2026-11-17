//! Proyecto NOMENCL: {{NOMENCLATURA}} desde RES + observación (o la regla del motor) y Word sin cursiva, como
//! `tests/test_nomenclatura_proyecto_20261116.py` de Python. Datos ficticios; `datos/nomenclatura_python.json` lo
//! genera `rust/oraculo/generar_nomenclatura.py` con el Python del Asistente.

use flow_dominio::{Celda, Etapa};
use flow_motor::Config;
use flow_productos::word::{self, EGRESO_SERVICIO, INGRESO_INFORME_X};
use flow_trabajo::{analizar_tabla, Contexto, Trabajo};
use serde::Deserialize;
use std::io::Read;

fn base() -> Trabajo {
    let ruta = format!("{}/../../../docs/evidencia_p0/golden/Origen_ESPERA.xlsx", env!("CARGO_MANIFEST_DIR"));
    let libro = flow_tabular::leer(std::path::Path::new(&ruta)).unwrap();
    let tabla = flow_tabular::leer_tabla(&libro, Some(Etapa::Espera), None).unwrap();
    analizar_tabla(&libro, &tabla, "2026-10-08".parse().unwrap(), &Config::default(), &Contexto::default())
}

struct Fila<'a> {
    tribunal: &'a str,
    rit: &'a str,
    nombre: &'a str,
    rut: &'a str,
    observacion: &'a str,
    res: Option<&'a str>,
    reglas: Vec<String>,
    acciones: Vec<String>,
}

fn fila<'a>(rit: &'a str, tribunal: &'a str, nombre: &'a str, rut: &'a str, observacion: &'a str, res: &'a str) -> Fila<'a> {
    Fila { tribunal, rit, nombre, rut, observacion, res: Some(res), reglas: vec![], acciones: vec![] }
}

/// Como `libro()` de Python: un origen con columnas RIT, TRIBUNAL, NOMBRE, RUT, DERIVACION, OBSERVACION y RES; la
/// observación queda como la que revisa el usuario y RES en la columna humana.
fn libro(filas: &[Fila]) -> Trabajo {
    let mut t = base();
    let modelo = t.filas[0].clone();
    let usadas = t.cabeceras.iter().rposition(|c| !c.is_empty()).map_or(0, |p| p + 1);
    t.cabeceras.truncate(usadas);
    t.cabeceras.push("RES".into());
    assert_eq!(word::columna_humana(&t, &word::ALIAS_OBSERVACION), None, "la observación revisada sale de la fila");
    t.filas = filas
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let mut f = modelo.clone();
            f.id = format!("f{i}");
            for (campo, v) in [("rit", d.rit), ("tribunal", d.tribunal), ("nombre", d.nombre), ("rut", d.rut), ("programa", "AFT EJEMPLO")]
            {
                f.registro.poner(campo, Celda::Texto(v.into()));
            }
            f.propuesta.excluido = false;
            f.propuesta.reglas = d.reglas.clone();
            f.propuesta.acciones = d.acciones.clone();
            f.propuesta.observacion = d.observacion.into();
            f.observacion = d.observacion.into();
            f.celdas.resize(usadas, Celda::Vacia);
            f.celdas.push(d.res.map_or(Celda::Vacia, |r| Celda::Texto(r.into())));
            f
        })
        .collect();
    t
}

fn hoy() -> flow_dominio::Fecha {
    "2099-01-15".parse().unwrap()
}

fn proyectos(t: &Trabajo) -> Vec<word::Proyecto> {
    let lista = word::proyectos(t, hoy(), None);
    assert!(lista.iter().all(|p| !p.sin_plantilla));
    lista
}

#[derive(Deserialize)]
struct FilaOraculo {
    observacion: String,
    res: Option<String>,
    reglas: Vec<String>,
    acciones: Vec<String>,
}

#[derive(Deserialize)]
struct Caso {
    nombre: String,
    filas: Vec<FilaOraculo>,
    observadas: Vec<Option<String>>,
    esperado: String,
}

#[test]
fn nomenclatura_como_python() {
    let casos: Vec<Caso> = serde_json::from_str(include_str!("datos/nomenclatura_python.json")).unwrap();
    assert!(casos.len() >= 30);
    for caso in casos {
        let filas: Vec<Fila> = caso
            .filas
            .iter()
            .map(|f| Fila {
                res: f.res.as_deref(),
                reglas: f.reglas.clone(),
                acciones: f.acciones.clone(),
                ..fila("X-1-2099", "LAJA", "Persona Uno", "11.111.111-1", &f.observacion, "")
            })
            .collect();
        let t = libro(&filas);
        let observadas: Vec<Option<String>> =
            t.filas.iter().map(|f| word::nomenclatura_observada(&word::observacion_revisada(&t, f))).collect();
        assert_eq!(observadas, caso.observadas, "{}", caso.nombre);
        let refs: Vec<&flow_trabajo::FilaTrabajo> = t.filas.iter().collect();
        assert_eq!(word::nomenclatura_proyecto(&t, &refs), caso.esperado, "{}", caso.nombre);
    }
}

#[test]
fn res_nomencl_con_observacion_de_egreso_usa_egreso() {
    let t = libro(&[fila("X-1-2099", "LAJA", "Persona Uno", "11.111.111-1", "Causa finalizada, aplicar egreso.", "NOMENCL")]);
    let lista = proyectos(&t);
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].tipo, "NOMENCL");
    assert_eq!(lista[0].valores["NOMENCLATURA"], EGRESO_SERVICIO);
    assert!(lista[0].faltantes.is_empty());
    let dir = carpeta("egreso");
    let rutas = word::generar(&lista, &dir, None).unwrap();
    let texto = parrafos(&documento(&rutas[0])).join("\n");
    assert!(texto.contains(&format!("\"{EGRESO_SERVICIO}\"")), "{texto}");
    assert!(!texto.contains("COMPLETAR NOMENCLATURA"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn res_nomencl_con_observacion_de_informe_usa_ingreso() {
    let t = libro(&[fila("X-2-2099", "MULCHEN", "Persona Dos", "22.222.222-2", "Informe de avance a folio 12.", "NOMENCL")]);
    let lista = proyectos(&t);
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].valores["NOMENCLATURA"], INGRESO_INFORME_X);
}

#[test]
fn observacion_con_nomenclatura_entre_comillas_manda() {
    let texto = "Se remite proyecto que aplica nomenclatura «Ingreso informe de diagnóstico» para regularizar.";
    let t = libro(&[fila("X-3-2099", "LAJA", "Persona Tres", "33.333.333-3", texto, "NOMENCL")]);
    let lista = proyectos(&t);
    assert_eq!(lista[0].valores["NOMENCLATURA"], "Ingreso informe de diagnóstico");
}

#[test]
fn la_observacion_de_nomenclatura_elige_el_tipo_y_la_nomenclatura() {
    // RES marcado sin tipo: la observación define NOMENCL y cuál nomenclatura.
    let texto = "Se remite proyecto de nomenclatura «Egreso de servicio protección especializada y otras redes».";
    let t = libro(&[fila("X-4-2099", "LAJA", "Persona Cuatro", "44.444.444-4", texto, "X")]);
    let lista = proyectos(&t);
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].tipo, "NOMENCL");
    assert_eq!(lista[0].valores["NOMENCLATURA"], EGRESO_SERVICIO);
}

#[test]
fn la_columna_observacion_del_origen_manda_sobre_la_propuesta() {
    // Como `libro()` de Python: la observación revisada sale de la columna OBSERVACION del origen, no de la propuesta.
    let mut t = libro(&[fila("X-14-2099", "LAJA", "Persona Catorce", "15.151.515-1", "Aplicar egreso.", "NOMENCL")]);
    t.cabeceras.push("OBSERVACION".into());
    t.filas[0].celdas.push(Celda::Texto("Informe de avance.".into()));
    assert!(!t.filas[0].editada());
    assert_eq!(proyectos(&t)[0].valores["NOMENCLATURA"], INGRESO_INFORME_X);
    // Una observación editada en Flow es la revisada.
    t.filas[0].observacion = "Se resuelve egreso.".into();
    assert_eq!(proyectos(&t)[0].valores["NOMENCLATURA"], EGRESO_SERVICIO);
}

#[test]
fn sin_observacion_util_la_regla_del_motor_decide() {
    let mut t = libro(&[fila("X-5-2099", "LAJA", "Persona Cinco", "55.555.555-5", "Texto.", "NOMENCL")]);
    t.filas[0].propuesta.reglas = vec!["HISTORIA.C10_ESCRITO".into()];
    assert_eq!(proyectos(&t)[0].valores["NOMENCLATURA"], INGRESO_INFORME_X);
    t.filas[0].propuesta.reglas = vec!["HISTORIA.P_FINALIZADA".into()];
    assert_eq!(proyectos(&t)[0].valores["NOMENCLATURA"], EGRESO_SERVICIO);
    // La observación revisada por el usuario prevalece sobre la regla.
    t.filas[0].observacion = "Se recepciona informe de cumplimiento.".into();
    assert_eq!(proyectos(&t)[0].valores["NOMENCLATURA"], INGRESO_INFORME_X);
}

#[test]
fn observacion_ambigua_o_ajena_no_inventa_nomenclatura() {
    let t = libro(&[
        fila("X-6-2099", "LAJA", "Persona Seis", "66.666.666-6", "Informe de avance; evaluar egreso.", "NOMENCL"),
        fila("X-7-2099", "LAJA", "Persona Siete", "77.777.777-7", "Texto.", "NOMENCL"),
    ]);
    let lista = proyectos(&t);
    assert_eq!(lista.len(), 2);
    for p in &lista {
        assert_eq!(p.valores["NOMENCLATURA"], "[COMPLETAR NOMENCLATURA]");
        assert_eq!(p.faltantes, ["NOMENCLATURA"]);
    }
}

#[test]
fn observacion_ambigua_no_cae_a_la_regla() {
    // Revisión Codex (PR #58): con regla C10 y observación que nombra egreso e informe, no se elige por la regla.
    let mut t = libro(&[fila("X-13-2099", "LAJA", "Persona Trece", "14.141.414-1", "Informe de avance; evaluar egreso.", "NOMENCL")]);
    t.filas[0].propuesta.reglas = vec!["HISTORIA.C10_ESCRITO".into()];
    let refs: Vec<&flow_trabajo::FilaTrabajo> = t.filas.iter().collect();
    assert_eq!(word::nomenclatura_proyecto(&t, &refs), "");
    assert_eq!(proyectos(&t)[0].valores["NOMENCLATURA"], "[COMPLETAR NOMENCLATURA]");
}

#[test]
fn nomenclatura_editada_por_el_usuario_se_respeta() {
    // En Python, `word_overrides` de la fila; en Flow, el valor editado del proyecto se conserva al volver a preparar.
    let t = libro(&[fila("X-8-2099", "LAJA", "Persona Ocho", "88.888.888-8", "Egreso.", "NOMENCL")]);
    let mut previos = proyectos(&t);
    assert_eq!(previos[0].valores["NOMENCLATURA"], EGRESO_SERVICIO);
    previos[0].valores.insert("NOMENCLATURA".into(), "Nomenclatura escrita a mano".into());
    let mut nuevos = proyectos(&t);
    word::conservar_ediciones(&previos, &mut nuevos, None);
    assert_eq!(nuevos[0].valores["NOMENCLATURA"], "Nomenclatura escrita a mano");
}

fn carpeta(nombre: &str) -> std::path::PathBuf {
    let c = std::env::temp_dir().join(format!("flow-prueba-{}-nomencl-{nombre}", std::process::id()));
    let _ = std::fs::remove_dir_all(&c);
    std::fs::create_dir_all(&c).unwrap();
    c
}

fn documento(ruta: &std::path::Path) -> String {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(ruta).unwrap()).unwrap();
    let mut xml = String::new();
    zip.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
    xml
}

fn parrafos(xml: &str) -> Vec<String> {
    let nodo = regex::Regex::new(r"<w:t(\s[^>]*)?>([^<]*)</w:t>").unwrap();
    xml.split("</w:p>")
        .map(|p| nodo.captures_iter(p).map(|c| c[2].replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")).collect::<String>())
        .filter(|t| !t.trim().is_empty())
        .collect()
}

/// `_cursivas` de la prueba Python: tramos con texto cuya cursiva no está apagada de forma explícita.
fn cursivas(xml: &str) -> Vec<String> {
    let tramo = regex::Regex::new(r"(?s)<w:r(?:\s[^>]*)?>(.*?)</w:r>").unwrap();
    let nodo = regex::Regex::new(r"<w:t(?:\s[^>]*)?>([^<]*)</w:t>").unwrap();
    let rpr = regex::Regex::new(r"(?s)^<w:rPr>(.*?)</w:rPr>").unwrap();
    let marca = regex::Regex::new(r"<w:(?:i|iCs)(\s[^>]*)?/>").unwrap();
    let mut halladas = Vec::new();
    for c in tramo.captures_iter(xml) {
        let dentro = &c[1];
        let texto: String = nodo.captures_iter(dentro).map(|t| t[1].to_string()).collect();
        if texto.trim().is_empty() {
            continue;
        }
        let marcas: Vec<String> = rpr
            .captures(dentro)
            .map(|r| marca.captures_iter(&r[1]).map(|m| m.get(1).map_or("", |a| a.as_str()).trim().to_string()).collect())
            .unwrap_or_default();
        if marcas.is_empty() || marcas.iter().any(|m| m != r#"w:val="0""# && m != r#"w:val="false""#) {
            halladas.push(texto);
        }
    }
    halladas
}

#[test]
fn word_generado_sin_cursiva_en_ninguna_matriz() {
    let t = libro(&[
        fila("X-9-2099", "LAJA", "Persona Nueve", "99.999.999-9", "Aplicar egreso.", "NOMENCL"),
        fila("X-10-2099", "MULCHEN", "Persona Diez", "10.101.010-1", "Informe de avance.", "NOMENCL"),
        fila("X-11-2099", "LAJA", "Persona Once", "12.121.212-1", "Texto.", "PC_IE"),
        fila("X-12-2099", "MULCHEN", "Persona Doce", "13.131.313-1", "Texto.", "PC_INFO"),
    ]);
    let lista = proyectos(&t);
    let tipos: std::collections::BTreeSet<&str> = lista.iter().map(|p| p.tipo.as_str()).collect();
    assert_eq!(tipos, ["NOMENCL", "PC_IE", "PC_INFO"].into());
    // Las matrices NOMENCL traen <w:i/> en su párrafo principal.
    for (nombre, xml) in [("LAJA", "NOMENCL"), ("MULCHEN", "NOMENCL")] {
        let docx = word::plantilla(flow_dominio::Tribunal::desde(nombre).unwrap(), xml, None).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(docx)).unwrap();
        let mut cuerpo = String::new();
        zip.by_name("word/document.xml").unwrap().read_to_string(&mut cuerpo).unwrap();
        assert!(cuerpo.contains("<w:i/>"));
    }
    let dir = carpeta("sin-cursiva");
    let rutas = word::generar(&lista, &dir, None).unwrap();
    assert_eq!(rutas.len(), 1, "un solo documento");
    let xml = documento(&rutas[0]);
    let texto = parrafos(&xml).join("\n");
    assert!(texto.contains(EGRESO_SERVICIO) && texto.contains(INGRESO_INFORME_X), "{texto}");
    assert_eq!(cursivas(&xml), Vec::<String>::new());
    assert!(!xml.contains("<w:i/>") && !xml.contains("<w:iCs/>"));
    std::fs::remove_dir_all(dir).unwrap();
}
