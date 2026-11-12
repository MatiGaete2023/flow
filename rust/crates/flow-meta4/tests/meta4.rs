//! Meta 4 de punta a punta sobre un lote ficticio en el formato de captura de Flow.

use flow_captura::lotes::{huella, Archivo, Resumen};
use flow_captura::recoleccion::{self, Pagina, Referencia};
use flow_captura::vinculos::Vinculo;
use flow_meta4::{calcular, salida, ALERTA, LEIDA_ANTES, NO_COMPLETADA, SIN_BITACORA, SIN_OBSERVACION, VERIFICADA};
use serde_json::json;
use std::path::{Path, PathBuf};

static SECUENCIA: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct Temporal(PathBuf);
impl Temporal {
    fn nueva() -> Self {
        let p = std::env::temp_dir().join(format!(
            "flow-meta4-ficticio-{}-{}",
            std::process::id(),
            SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn vinculo(rit: &str, rut: &str, nombre: &str, programa: &str, ingreso: &str) -> Vinculo {
    serde_json::from_value(json!({
        "fila": 1, "rit": rit, "rut": rut, "nombre": nombre, "programa": programa, "tribunal": "901",
        "causa": format!("c{ingreso}"), "ingreso": ingreso, "etapa": "3", "antiguo": "", "tipo_popup": "12",
        "rit_enlace": rit, "calendario": null, "bitacora_verificada": true, "historia_verificada": false, "diagnostico": ""
    }))
    .unwrap()
}

/// Lote con dos consultas: Residencia en Cumplimiento (3 registros) y FAE en Espera (1 registro).
fn lote(carpeta: &Path, capturada: &str) -> Resumen {
    std::fs::create_dir_all(carpeta).unwrap();
    let consulta = |tab: &str, modalidad: &str| json!({"tribunal":"901","screen":"seguimiento","tab":tab,"modality":modalidad,"report":null,"state":null,"month":null,"year":null});
    let mut r: Resumen = serde_json::from_value(json!({
        "version": 1,
        "lote": {"consultas": [consulta("Cumplimiento", "1"), consulta("Espera", "3")], "fechas": null, "conservar_filtros": false,
                 "evidencia": false, "pausa_ms": 250, "lecturas": {"bitacoras": true, "historia": false, "calendario": false, "meses": 1}},
        "estado": "TERMINADO", "terminadas": [0, 1], "vacias": [], "archivos": [], "avisos": []
    }))
    .unwrap();
    let cumplimiento = "<html><table><tr><th>RIT</th><th>Tribunal</th><th>RUT</th><th>Nombre</th><th>Tipo</th><th>Sexo</th>\
        <th>Derivación</th><th>Fec. Resolución</th><th>Fec. Ingreso Efectivo</th><th>DIAS DE CUMPLIMIENTO</th><th>DIAS PARA EGRESAR</th></tr>\
        <tr><td>X-1-2099</td><td>Juzgado de Familia de Tomé</td><td>11111111-K</td><td>SOFÍA PÉREZ SOTO</td><td>Protección</td><td>F</td><td>RLP Hogar Ficticio</td><td>01/08/2026</td><td>10/08/2026</td><td>60</td><td>100</td></tr>\
        <tr><td>X-2-2099</td><td>Juzgado de Familia de Tomé</td><td>22222222-0</td><td>MATÍAS ROJAS DÍAZ</td><td>Protección</td><td>M</td><td>REM Residencia Ficticia</td><td>02/08/2026</td><td>12/08/2026</td><td>58</td><td>90</td></tr>\
        <tr><td>X-3-2099</td><td>Juzgado de Familia de Tomé</td><td>33333333-1</td><td>EMILIA TORO</td><td>Protección</td><td>F</td><td>=RES con signo igual</td><td>03/08/2026</td><td>13/08/2026</td><td>57</td><td>80</td></tr>\
        </table></html>";
    let espera = "<html><table><tr><th>RIT</th><th>Tribunal</th><th>RUT</th><th>Nombre</th><th>Tipo</th><th>Sexo</th>\
        <th>Derivación</th><th>T Espera</th><th>Fec. Resolución</th></tr>\
        <tr><td>X-4-2099</td><td>Jgdo. L. y G. de Laja</td><td>44444444-2</td><td>LUCAS ARAYA</td><td>Protección</td><td>M</td><td>FAE PRO Ficticio</td><td>35</td><td>05/09/2026</td></tr>\
        </table></html>";
    for (i, (nombre, html, registros)) in [("cumplimiento.xls", cumplimiento, 3), ("espera.xls", espera, 1)].into_iter().enumerate() {
        std::fs::write(carpeta.join(nombre), html).unwrap();
        r.archivos.push(Archivo { nombre: nombre.into(), sha256: huella(html.as_bytes()), consulta: i, pagina: 1, total: 1, registros });
    }
    // Sofía: última observación del mes en Espera estando en Cumplimiento → alerta.
    // Matías: observación de Informes más reciente, que no participa → cuenta la de Cumplimiento.
    // Emilia: vínculo sin lectura. Lucas (otra consulta): sin vínculo.
    let v1 = vinculo("X-1-2099", "11111111-K", "SOFÍA PÉREZ SOTO", "RLP Hogar Ficticio", "201");
    let v2 = vinculo("X-2-2099", "22222222-0", "MATÍAS ROJAS DÍAZ", "REM Residencia Ficticia", "202");
    let v3 = vinculo("X-3-2099", "33333333-1", "EMILIA TORO", "=RES con signo igual", "203");
    r.paginas_lectura.push(Pagina { consulta: 0, numero: 1, vinculos: vec![v1.clone(), v2.clone(), v3] });
    let entradas = [
        (&v1, "Pérez Soto Sofía", json!([
            {"entry_id":"1","fecha":"2026-10-05T10:00:00","etapa":"Cumplimiento","tipo":"Al tribunal","texto":"Observación ficticia uno"},
            {"entry_id":"2","fecha":"2026-10-12T09:30:00","etapa":"Espera","tipo":"Al tribunal","texto":"Observación ficticia dos"},
            {"entry_id":"3","fecha":"2026-08-01T09:30:00","etapa":"Cumplimiento","tipo":"Al tribunal","texto":"Fuera del período"}])),
        (&v2, "Matías Rojas Díaz", json!([
            {"entry_id":"4","fecha":"2026-10-08T11:00:00","etapa":"Cumplimiento","tipo":"Al tribunal","texto":"Observación ficticia tres"},
            {"entry_id":"5","fecha":"2026-10-14T11:00:00","etapa":"Informes","tipo":"Al tribunal","texto":"De informes, no participa"}])),
    ];
    for (n, (v, persona, entries)) in entradas.into_iter().enumerate() {
        let clave = recoleccion::clave("bitacora", v);
        let datos = json!({"rit": v.rit, "persona": persona, "entries": entries});
        let bytes = serde_json::to_vec(&json!({"version":1,"tipo":"bitacora","clave":clave,"vinculo":v,"datos":datos})).unwrap();
        let archivo = format!("bitacora-{n}.json");
        std::fs::write(carpeta.join(&archivo), &bytes).unwrap();
        r.lecturas.push(Referencia {
            tipo: "bitacora".into(), clave, archivo, sha256: huella(&bytes), capturada: capturada.into(), meses: 1,
            tribunal: "901".into(), causa: v.causa.clone(), ingreso: v.ingreso.clone(), reutilizada: false,
        });
    }
    r.guardar(carpeta).unwrap();
    r
}

fn dia(texto: &str) -> chrono::NaiveDate {
    texto.parse().unwrap()
}

#[test]
fn calcula_fechas_alertas_y_controles() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    lote(&carpeta, "2026-10-16T15:00:00-03:00");
    let m = calcular(&[carpeta.clone()], dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    assert_eq!(m.registros.len(), 4);
    let de = |rit: &str| m.registros.iter().find(|r| r.texto("RIT") == rit).unwrap();
    let sofia = de("X-1-2099");
    assert_eq!((sofia.fecha_obs, sofia.etapa_ultima.as_str(), sofia.control.as_str()), (Some(dia("2026-10-12")), "Espera", VERIFICADA));
    assert_eq!(sofia.alerta, ALERTA);
    assert_eq!((sofia.categoria, sofia.tribunal), (Some("Residencia"), Some("TOMÉ")));
    let matias = de("X-2-2099");
    assert_eq!((matias.fecha_obs, matias.etapa_ultima.as_str()), (Some(dia("2026-10-08")), "Cumplimiento"));
    assert!(matias.alerta.is_empty() && !matias.requiere_revision());
    assert_eq!(de("X-3-2099").control, NO_COMPLETADA);
    let lucas = de("X-4-2099");
    assert_eq!((lucas.control.as_str(), lucas.categoria, lucas.tribunal), (SIN_BITACORA, Some("FAE"), Some("LAJA")));
    // Solo las observaciones del período, incluidas las de Informes, van a «Bitácoras del mes».
    assert_eq!(m.observaciones.len(), 4);
    // La consulta de Espera tiene registros y ninguna bitácora leída.
    assert_eq!(m.sin_bitacoras.len(), 1);
    let resumen = salida::resumen(&m);
    let total = resumen.last().unwrap();
    assert_eq!((total["TOTAL REGISTROS"].as_i64(), total["ALERTAS"].as_i64(), total["REVISAR BITÁCORA"].as_i64()), (Some(4), Some(1), Some(2)));
    assert_eq!(resumen[0]["TRIBUNAL"], "LAJA");

    // Publicación: cinco archivos en carpeta nueva; no se sobrescribe.
    let destino = t.0.join("Resultado META4");
    let rutas = salida::publicar(&m, &destino).unwrap();
    assert_eq!(rutas.len(), 5);
    assert!(rutas.iter().all(|r| r.is_file()));
    assert!(salida::publicar(&m, &destino).is_err());
    assert!(!t.0.join(".en-curso-Resultado META4").exists());
    // La planilla se relee: bloque de TOMÉ en «Residencia Cumplimiento» con los tres registros y su fecha.
    let libro = flow_tabular::leer(&rutas[0]).unwrap();
    let hoja = libro.hojas.iter().find(|h| h.nombre == "Residencia Cumplimiento").unwrap();
    let texto = |f: usize, c: usize| hoja.filas.get(f).and_then(|x| x.get(c)).map(|c| c.texto()).unwrap_or_default();
    let inicio = hoja.filas.iter().position(|f| f.first().is_some_and(|c| c.texto() == "TOMÉ")).unwrap();
    assert_eq!(texto(inicio + 2, 0), "Fecha Emisión   : 16-10-2026");
    assert_eq!(texto(inicio + 4, 0), "RIT");
    assert_eq!(texto(inicio + 5, 0), "X-1-2099");
    assert_eq!(hoja.filas[inicio + 5][7].fecha(), Some(dia("2026-10-12")));
    assert_eq!(texto(inicio + 7, 4), "=RES con signo igual", "queda como texto, no como fórmula");
    let ultimo = hoja.filas.iter().rposition(|f| f.first().is_some_and(|c| c.texto() == "TOTAL:")).unwrap();
    assert_eq!(hoja.filas[ultimo][1].entero(), Some(3));
    assert!(libro.hojas.iter().any(|h| h.nombre == "Control bitácoras"));
    let reporte: serde_json::Value = serde_json::from_slice(&std::fs::read(&rutas[4]).unwrap()).unwrap();
    assert_eq!(reporte["alertas"], 1);
    assert_eq!(reporte["periodo"]["desde"], "2026-09-16");
    assert!(!reporte.to_string().contains("SOFÍA"), "el reporte no lleva datos de personas");
}

#[test]
fn lectura_anterior_al_corte_filtro_y_lotes_repetidos() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    lote(&carpeta, "2026-10-14T15:00:00-03:00");
    // Leída el 14 y corte el 16: lo escrito entre medio no está; se pide revisión y no se toma fecha.
    let m = calcular(&[carpeta.clone()], dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    let sofia = m.registros.iter().find(|r| r.texto("RIT") == "X-1-2099").unwrap();
    assert_eq!((sofia.control.as_str(), sofia.fecha_obs), (LEIDA_ANTES, None));
    // Filtro de tribunales: solo Laja; uno desconocido o sin registros es un error.
    let laja = calcular(&[carpeta.clone()], dia("2026-10-14"), dia("2026-10-14"), &["Laja".into()]).unwrap();
    assert_eq!(laja.registros.len(), 1);
    assert_eq!(laja.registros[0].control, SIN_BITACORA);
    assert!(calcular(&[carpeta.clone()], dia("2026-10-14"), dia("2026-10-14"), &["Atlántida".into()]).is_err());
    assert!(calcular(&[carpeta.clone()], dia("2026-10-14"), dia("2026-10-14"), &["Yumbel".into()]).is_err());
    let rutas = salida::publicar(&laja, &t.0.join("Parcial")).unwrap();
    assert!(rutas[0].to_string_lossy().ends_with("META 4 - actualizado 2026-10 - parcial.xlsx"));
    // La misma consulta en dos lotes se rechaza: duplicaría registros.
    let copia = t.0.join("Lote repetido");
    lote(&copia, "2026-10-14T15:00:00-03:00");
    let error = calcular(&[carpeta.clone(), copia], dia("2026-10-14"), dia("2026-10-14"), &[]).unwrap_err();
    assert!(error.contains("dos lotes"), "{error}");
    // Una descarga alterada después de verificada detiene todo.
    std::fs::write(carpeta.join("espera.xls"), "<html><table><tr><td>alterado</td></tr></table></html>").unwrap();
    assert!(calcular(&[carpeta], dia("2026-10-14"), dia("2026-10-14"), &[]).is_err());
}

#[test]
fn sin_observacion_en_el_mes_deja_la_fecha_vacia() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    lote(&carpeta, "2026-12-20T10:00:00-03:00");
    let m = calcular(&[carpeta], dia("2026-12-20"), dia("2026-12-20"), &[]).unwrap();
    let sofia = m.registros.iter().find(|r| r.texto("RIT") == "X-1-2099").unwrap();
    assert_eq!((sofia.fecha_obs, sofia.control.as_str()), (None, SIN_OBSERVACION));
    assert!(sofia.alerta.is_empty());
}
