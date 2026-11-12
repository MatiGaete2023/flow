//! Regeneración sobre productos ficticios; no se usa información de SITFA real.
use chrono::NaiveDate;
use flow_dominio::Celda;
use flow_meta4::{regenerar, salida, ALERTA};
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};

static SECUENCIA: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Temporal(PathBuf);
impl Temporal {
    fn nueva() -> Self {
        let ruta = std::env::temp_dir().join(format!(
            "flow-meta4-regeneracion-{}-{}", std::process::id(),
            SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&ruta);
        std::fs::create_dir_all(&ruta).unwrap();
        Self(ruta)
    }
}
impl Drop for Temporal {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}
fn fecha() -> NaiveDate { NaiveDate::from_ymd_opt(2026, 10, 16).unwrap() }
fn meta() -> flow_meta4::Meta4 {
    let columnas = vec![
        ("RIT".into(), Celda::Texto("P-1-2099".into())),
        ("TRIBUNAL".into(), Celda::Texto("Juzgado de Familia de Tomé".into())),
        ("RUT".into(), Celda::Texto("11111111-K".into())),
        ("NOMBRE".into(), Celda::Texto("SOFÍA PÉREZ SOTO".into())),
        ("DERIVACIÓN".into(), Celda::Texto("REM Hogar Ficticio".into())),
    ];
    flow_meta4::Meta4 {
        corte: fecha(), emision: fecha(), tribunales: None,
        registros: vec![flow_meta4::Registro4 {
            lista: "Cumplimiento", categoria: Some("Residencia"), tribunal: Some("TOMÉ"),
            columnas, archivo: "captura.xls".into(), lote: "Lote ficticio".into(),
            fecha_obs: Some(NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()),
            etapa_ultima: "Espera".into(), control: "VERIFICADA EN EL PERÍODO".into(), alerta: ALERTA.into(),
        }],
        observaciones: Vec::new(), lecturas: Vec::new(), sin_bitacoras: Vec::new(),
        filtros: Vec::new(), lotes: vec!["Lote ficticio".into()], avisos: Vec::new(),
    }
}
/// Copia un producto de prueba a un libro simulado como revisado. Cambia fecha, etapa y ALERTA;
/// el valor de ALERTA manipulado se debe ignorar para impedir resultados inconsistentes.
fn editar_libro(origen: &Path, destino: &Path, fecha_ilegal: bool) {
    let libro = flow_tabular::leer(origen).unwrap();
    let mut nuevo = Workbook::new();
    for original in libro.hojas.iter().filter(|h| h.nombre == "ESPERA" || h.nombre == "CUMPLIMIENTO") {
        let hoja = nuevo.add_worksheet();
        hoja.set_name(&original.nombre).unwrap();
        let cab = &original.filas[0];
        let col_fecha = cab.iter().position(|c| c.texto() == "FECHA OBS.").unwrap();
        let col_etapa = cab.iter().position(|c| c.texto() == "ETAPA ÚLTIMA OBS.").unwrap();
        let col_alerta = cab.iter().position(|c| c.texto() == "ALERTA").unwrap();
        for (f, fila) in original.filas.iter().enumerate() {
            for (c, valor) in fila.iter().enumerate() {
                let editado = if f == 1 && original.nombre == "CUMPLIMIENTO" {
                    if c == col_fecha {
                        Celda::Texto(if fecha_ilegal { "sin fecha" } else { "2026-10-15" }.into())
                    } else if c == col_etapa {
                        Celda::Texto("Cumplimiento".into())
                    } else if c == col_alerta {
                        Celda::Texto("ALERTA A MANO INCORRECTA".into())
                    } else { valor.clone() }
                } else { valor.clone() };
                match editado {
                    Celda::Vacia => {},
                    Celda::Texto(t) => { hoja.write_string(f as u32, c as u16, &t).unwrap(); },
                    Celda::Numero(n) => { hoja.write_number(f as u32, c as u16, n).unwrap(); },
                    Celda::Fecha(d) => { hoja.write_string(f as u32, c as u16, d.to_string()).unwrap(); },
                }
            }
        }
    }
    nuevo.save(destino).unwrap();
}

#[test]
fn regenera_desde_fecha_editada_sin_alterar_el_original() {
    let temporal = Temporal::nueva();
    let fuente = temporal.0.join("Original");
    let originales = salida::publicar(&meta(), &fuente).unwrap();
    let revisados = temporal.0.join("Revisados");
    std::fs::create_dir(&revisados).unwrap();
    let registros = revisados.join("Registros.xlsx");
    editar_libro(&originales[1], &registros, false);
    for nombre in ["Reporte META4.json", "Bitacoras_del_mes.xlsx"] {
        std::fs::copy(fuente.join(nombre), revisados.join(nombre)).unwrap();
    }
    let antes = std::fs::read(&originales[1]).unwrap();
    let cargado = regenerar::cargar(&registros).unwrap();
    assert_eq!(cargado.meta.registros.len(), 1);
    let r = &cargado.meta.registros[0];
    assert_eq!(r.fecha_obs, Some(NaiveDate::from_ymd_opt(2026, 10, 15).unwrap()));
    assert_eq!(r.etapa_ultima, "Cumplimiento");
    assert!(r.alerta.is_empty(), "la alerta manual no prevalece sobre la regla");
    let destino = temporal.0.join("Regenerado");
    let archivos = regenerar::ejecutar(&registros, &destino).unwrap();
    assert_eq!(archivos.len(), 5);
    assert!(archivos.iter().all(|p| p.is_file()));
    assert_eq!(std::fs::read(&originales[1]).unwrap(), antes);
    assert_eq!(
        std::fs::read(revisados.join("Bitacoras_del_mes.xlsx")).unwrap(),
        std::fs::read(destino.join("Bitacoras_del_mes.xlsx")).unwrap()
    );
    let planilla = flow_tabular::leer(&archivos[0]).unwrap();
    let hoja = planilla.hojas.iter().find(|h| h.nombre == "Residencia Cumplimiento").unwrap();
    assert!(hoja.filas.iter().any(|f| f.get(7).is_some_and(|c| c.fecha() == r.fecha_obs)));
    let reporte: serde_json::Value = serde_json::from_slice(&std::fs::read(&archivos[4]).unwrap()).unwrap();
    assert_eq!(reporte["alertas"], 0);
    assert_eq!(reporte["regenerado_desde"], "Registros.xlsx revisado");
    assert!(regenerar::ejecutar(&registros, &destino).is_err(), "no se sobrescribe");
}

#[test]
fn fecha_ilegible_o_fuentes_faltantes_no_publican() {
    let temporal = Temporal::nueva();
    let fuente = temporal.0.join("Original");
    let originales = salida::publicar(&meta(), &fuente).unwrap();
    let revisados = temporal.0.join("Revisados");
    std::fs::create_dir(&revisados).unwrap();
    let registros = revisados.join("Registros.xlsx");
    editar_libro(&originales[1], &registros, true);
    for nombre in ["Reporte META4.json", "Bitacoras_del_mes.xlsx"] {
        std::fs::copy(fuente.join(nombre), revisados.join(nombre)).unwrap();
    }
    let destino = temporal.0.join("Regenerado");
    let error = regenerar::ejecutar(&registros, &destino).unwrap_err();
    assert!(error.contains("FECHA OBS."), "{error}");
    assert!(!destino.exists());
    editar_libro(&originales[1], &registros, false);
    std::fs::remove_file(revisados.join("Bitacoras_del_mes.xlsx")).unwrap();
    assert!(regenerar::ejecutar(&registros, &destino).is_err());
    assert!(!destino.exists());
}
