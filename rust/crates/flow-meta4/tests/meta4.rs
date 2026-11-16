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
        "estado": "VALIDADA", "terminadas": [0, 1], "vacias": [], "archivos": [], "avisos": []
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
        (
            &v1,
            "Pérez Soto Sofía",
            json!([
            {"entry_id":"1","fecha":"2026-10-05T10:00:00","etapa":"Cumplimiento","tipo":"Al tribunal","texto":"Observación ficticia uno"},
            {"entry_id":"2","fecha":"2026-10-12T09:30:00","etapa":"Espera","tipo":"Al tribunal","texto":"Observación ficticia dos"},
            {"entry_id":"3","fecha":"2026-08-01T09:30:00","etapa":"Cumplimiento","tipo":"Al tribunal","texto":"Fuera del período"}]),
        ),
        (
            &v2,
            "Matías Rojas Díaz",
            json!([
            {"entry_id":"4","fecha":"2026-10-08T11:00:00","etapa":"Cumplimiento","tipo":"Al tribunal","texto":"Observación ficticia tres"},
            {"entry_id":"5","fecha":"2026-10-14T11:00:00","etapa":"Informes","tipo":"Al tribunal","texto":"De informes, no participa"}]),
        ),
    ];
    for (n, (v, persona, entries)) in entradas.into_iter().enumerate() {
        let clave = recoleccion::clave("bitacora", v);
        let hasta = chrono::DateTime::parse_from_rfc3339(capturada).unwrap().with_timezone(&chrono::Local).date_naive();
        let desde = flow_meta4::periodo(hasta).0;
        let datos = json!({"rit": v.rit, "persona": persona, "entries": entries,"desde":desde,"hasta":hasta});
        let bytes = serde_json::to_vec(&json!({"version":1,"tipo":"bitacora","clave":clave,"vinculo":v,"datos":datos})).unwrap();
        let archivo = format!("bitacora-{n}.json");
        std::fs::write(carpeta.join(&archivo), &bytes).unwrap();
        r.lecturas.push(Referencia {
            tipo: "bitacora".into(),
            clave,
            archivo,
            sha256: huella(&bytes),
            capturada: capturada.into(),
            meses: 1,
            tribunal: "901".into(),
            causa: v.causa.clone(),
            ingreso: v.ingreso.clone(),
            reutilizada: false,
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
    assert_eq!(
        (total["TOTAL REGISTROS"].as_i64(), total["ALERTAS"].as_i64(), total["REVISAR BITÁCORA"].as_i64()),
        (Some(4), Some(1), Some(2))
    );
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
    assert!(laja.lecturas.is_empty(), "las lecturas de Tomé están fuera del filtro de Laja");
    let libro = flow_tabular::leer(&t.0.join("Parcial").join("Bitacoras_del_mes.xlsx")).unwrap();
    for h in libro.hojas {
        for fila in h.filas {
            for celda in fila {
                let s = celda.texto();
                assert!(
                    !s.contains("X-1-2099")
                        && !s.contains("SOFÍA")
                        && !s.contains("TOMÉ")
                        && !s.contains("RLP Hogar Ficticio")
                        && !s.contains("c201")
                );
            }
        }
    }

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

#[test]
fn lectura_posterior_al_corte_con_ventana_insuficiente_no_afirma_ausencia() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    lote(&carpeta, "2026-10-30T15:00:00-03:00");
    let m = calcular(std::slice::from_ref(&carpeta), dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    let fila = m.registros.iter().find(|r| r.texto("RIT") == "X-1-2099").unwrap();
    assert!(fila.fecha_obs.is_none());
    assert_eq!(fila.control, flow_meta4::PERIODO_INCOMPLETO);
    assert!(fila.etapa_ultima.is_empty());
    assert!(m.avisos.iter().any(|s| s.contains("no cubren todo el período")));
    assert!(m.observaciones.is_empty());
    // Una ventana ausente tampoco permite afirmar que no hubo observaciones.
    let mut resumen = Resumen::leer(&carpeta).unwrap();
    for referencia in &mut resumen.lecturas {
        let path = carpeta.join(&referencia.archivo);
        let mut datos: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        datos["datos"].as_object_mut().unwrap().remove("desde");
        let raw = serde_json::to_vec(&datos).unwrap();
        std::fs::write(path, &raw).unwrap();
        referencia.sha256 = huella(&raw);
    }
    resumen.guardar(&carpeta).unwrap();
    let m = calcular(&[carpeta], dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    assert_eq!(m.registros[0].control, flow_meta4::PERIODO_INCOMPLETO);
}
#[test]
fn cuatrocientos_cincuenta_calculos_coinciden_con_python() {
    let casos: Vec<serde_json::Value> = serde_json::from_str(include_str!("datos/calculo_python.json")).unwrap();
    for c in casos {
        let corte = dia(c["corte"].as_str().unwrap());
        assert_eq!(flow_meta4::periodo(corte).0.to_string(), c["desde"]);
        let entradas = c["entradas"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    chrono::NaiveDateTime::parse_from_str(e["fecha"].as_str().unwrap(), "%Y-%m-%dT%H:%M:%S").unwrap(),
                    e["etapa"].as_str().unwrap().to_string(),
                )
            })
            .collect::<Vec<_>>();
        let (fecha, etapa, control) = flow_meta4::ultima(&entradas, corte);
        assert_eq!(fecha.map(|f| f.to_string()), c["fecha"].as_str().map(str::to_string));
        assert_eq!(etapa, c["etapa"].as_str().unwrap_or(""));
        assert_eq!(control, c["control"]);
    }
}

/// Reescribe los datos de una lectura guardada del lote y actualiza su huella en el resumen.
fn editar_lectura(carpeta: &Path, indice: usize, cambio: impl Fn(&mut serde_json::Value)) {
    let mut resumen = Resumen::leer(carpeta).unwrap();
    let referencia = &mut resumen.lecturas[indice];
    let ruta = carpeta.join(&referencia.archivo);
    let mut valor: serde_json::Value = serde_json::from_slice(&std::fs::read(&ruta).unwrap()).unwrap();
    cambio(&mut valor["datos"]);
    let bytes = serde_json::to_vec(&valor).unwrap();
    std::fs::write(&ruta, &bytes).unwrap();
    referencia.sha256 = huella(&bytes);
    resumen.guardar(carpeta).unwrap();
}

/// M1: una lectura fallida deja la consulta en `terminadas` y un aviso en el resumen (Captura 8c2dffb). Meta 4 no se
/// bloquea: la fila queda «LECTURA NO COMPLETADA», se avisa y la bitácora fallida figura en Control de lecturas. Un lote
/// INCOMPLETA se rechaza como en Python (`estado == VALIDADA`).
#[test]
fn m1_lectura_fallida_genera_con_aviso_y_lote_incompleto_se_rechaza() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    let mut r = lote(&carpeta, "2026-10-16T15:00:00-03:00");
    r.avisos.push("Consulta 1, página 1, fila 1: bitacora pendiente. Sesión ficticia vencida.".into());
    r.avisos
        .push("Los Excel verificados se conservan. Quedaron 1 lecturas por ingreso pendientes; retoma el lote para reintentarlas.".into());
    r.guardar(&carpeta).unwrap();
    let m = calcular(std::slice::from_ref(&carpeta), dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    let emilia = m.registros.iter().find(|r| r.texto("RIT") == "X-3-2099").unwrap();
    assert_eq!(emilia.control, NO_COMPLETADA);
    assert!(m.avisos.iter().any(|a| a.contains("1 bitácoras no se pudieron leer") && a.contains("consultas 1")), "{:?}", m.avisos);
    let fallida = m.lecturas.iter().find(|l| l.rit == "X-3-2099").unwrap();
    assert_eq!((fallida.lectura.as_str(), fallida.control.as_str()), ("FALLIDA", NO_COMPLETADA));
    r.estado = "INCOMPLETA".into();
    r.guardar(&carpeta).unwrap();
    let error = calcular(&[carpeta], dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap_err();
    assert!(error.contains("incompleta") && error.contains("Lote ficticio"), "{error}");
}

/// M1: un listado cuyos vínculos quedaron pendientes con bitácoras pedidas no se informa como «sin bitácora».
#[test]
fn m1_listado_pendiente_queda_como_lectura_no_completada() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    let mut r = lote(&carpeta, "2026-10-16T15:00:00-03:00");
    r.paginas_lectura.clear();
    r.lecturas.clear();
    r.avisos.push("Consulta 1, página 1: vínculos del listado pendientes. Listado ficticio ilegible.".into());
    r.guardar(&carpeta).unwrap();
    let m = calcular(&[carpeta], dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    assert!(m.registros.iter().filter(|r| r.lista == "Cumplimiento").all(|r| r.control == NO_COMPLETADA));
    assert_eq!(m.sin_bitacoras.len(), 1, "solo la consulta de Espera, que no tiene vínculos ni pendientes");
    assert!(m.sin_bitacoras[0].0.contains("Espera"));
}

/// M4: dos consultas del mismo lote con la misma clave (p. ej. otro filtro de vencimiento) no son «dos lotes».
#[test]
fn m4_consulta_repetida_en_el_mismo_lote_no_se_rechaza() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    let mut r = lote(&carpeta, "2026-10-16T15:00:00-03:00");
    let mut otra = r.lote.consultas[0].clone();
    otra.report = Some("2".into());
    r.lote.consultas.push(otra);
    r.terminadas.push(2);
    let mut archivo = r.archivos[0].clone();
    archivo.nombre = "cumplimiento-vencimiento.xls".into();
    archivo.consulta = 2;
    std::fs::copy(carpeta.join("cumplimiento.xls"), carpeta.join(&archivo.nombre)).unwrap();
    r.archivos.push(archivo);
    r.guardar(&carpeta).unwrap();
    let m = calcular(&[carpeta], dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    assert_eq!(m.registros.len(), 7);
    assert!(m.filtros.iter().any(|f| f.contains("vencimiento")));
    // Dos lotes distintos con el mismo nombre de carpeta siguen siendo dos lotes.
    let (a, b) = (t.0.join("a").join("Lote homónimo"), t.0.join("b").join("Lote homónimo"));
    lote(&a, "2026-10-16T15:00:00-03:00");
    lote(&b, "2026-10-16T15:00:00-03:00");
    let error = calcular(&[a, b], dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap_err();
    assert!(error.contains("dos lotes"), "{error}");
}

/// Lote de un tribunal ficticio (código 905): FAE Cumplimiento con un registro de Yumbel y FAE Espera sin registros
/// con evidencia PDF.
fn lote_con_vacia(carpeta: &Path, con_registros: bool) {
    std::fs::create_dir_all(carpeta).unwrap();
    let consulta = |tab: &str| json!({"tribunal":"905","screen":"seguimiento","tab":tab,"modality":"3","report":null,"state":null,"month":null,"year":null});
    let mut r: Resumen = serde_json::from_value(json!({
        "version": 1,
        "lote": {"consultas": [consulta("Cumplimiento"), consulta("Espera")], "fechas": null, "conservar_filtros": false,
                 "evidencia": true, "pausa_ms": 250, "lecturas": {"bitacoras": false, "historia": false, "calendario": false, "meses": 1}},
        "estado": "VALIDADA", "terminadas": [0, 1], "vacias": [1], "archivos": [], "avisos": []
    }))
    .unwrap();
    if con_registros {
        let html = "<html><table><tr><th>RIT</th><th>Tribunal</th><th>RUT</th><th>Nombre</th><th>Tipo</th><th>Sexo</th>\
            <th>Derivación</th><th>Fec. Resolución</th><th>Fec. Ingreso Efectivo</th><th>DIAS DE CUMPLIMIENTO</th><th>DIAS PARA EGRESAR</th></tr>\
            <tr><td>X-7-2099</td><td>Juzgado de Letras de Yumbel</td><td>77777777-7</td><td>PERSONA FICTICIA</td><td>Protección</td><td>F</td><td>FAE Ficticio</td><td>01/08/2026</td><td>10/08/2026</td><td>60</td><td>100</td></tr>\
            </table></html>";
        std::fs::write(carpeta.join("cumplimiento.xls"), html).unwrap();
        r.archivos.push(Archivo {
            nombre: "cumplimiento.xls".into(),
            sha256: huella(html.as_bytes()),
            consulta: 0,
            pagina: 1,
            total: 1,
            registros: 1,
        });
    } else {
        r.vacias = vec![0, 1];
    }
    std::fs::write(carpeta.join("consulta-002-SIN-REGISTROS.pdf"), b"%PDF-1.4 ficticio").unwrap();
    r.archivos.push(Archivo {
        nombre: "consulta-002-SIN-REGISTROS.pdf".into(),
        sha256: huella(b"%PDF-1.4 ficticio"),
        consulta: 1,
        pagina: 1,
        total: 1,
        registros: 0,
    });
    r.guardar(carpeta).unwrap();
}

/// M5: solo falla si ninguno de los tribunales elegidos tiene candidatos; una consulta sin registros es un candidato
/// (SIN REGISTROS en la planilla, PDF en la matriz) y un lote solo con consultas vacías genera la planilla.
#[test]
fn m5_tribunal_sin_registros_y_consultas_vacias_no_bloquean() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    lote(&carpeta, "2026-10-16T15:00:00-03:00");
    let m = calcular(std::slice::from_ref(&carpeta), dia("2026-10-16"), dia("2026-10-16"), &["Laja".into(), "Yumbel".into()]).unwrap();
    assert_eq!(m.registros.len(), 1);
    assert_eq!(m.tribunales, Some(vec!["LAJA", "YUMBEL"]));
    assert!(calcular(&[carpeta], dia("2026-10-16"), dia("2026-10-16"), &["Yumbel".into()]).is_err(), "ninguno elegido tiene candidatos");

    let yumbel = t.0.join("Lote Yumbel");
    lote_con_vacia(&yumbel, true);
    assert_eq!(flow_meta4::tribunales_en_lotes(std::slice::from_ref(&yumbel)).unwrap(), vec!["YUMBEL"]);
    let m = calcular(std::slice::from_ref(&yumbel), dia("2026-10-16"), dia("2026-10-16"), &["Yumbel".into()]).unwrap();
    let destino = t.0.join("Salida Yumbel");
    salida::publicar(&m, &destino).unwrap();
    let libro = flow_tabular::leer(&destino.join("Registros.xlsx")).unwrap();
    let matriz = libro.hojas.iter().find(|h| h.nombre == "MATRIZ_COBERTURA").unwrap();
    let fila = matriz.filas.iter().find(|f| f.first().is_some_and(|c| c.texto() == "YUMBEL")).unwrap();
    assert_eq!(fila[1].texto(), "PDF · 0 registros");
    assert_eq!(fila[2].texto(), "EXCEL · 1 registro(s)");
    assert_eq!(fila[3].texto(), "FALTANTE");

    let vacio = t.0.join("Lote vacío");
    lote_con_vacia(&vacio, false);
    let m = calcular(&[vacio], dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    assert!(m.registros.is_empty());
    assert!(m.avisos.iter().any(|a| a.contains("no se pudieron atribuir")));
    let rutas = salida::publicar(&m, &t.0.join("Salida vacía")).unwrap();
    assert_eq!(rutas.len(), 5);
}

/// M6: Registros.xlsx trae INVENTARIO_ARCHIVOS y MATRIZ_COBERTURA; «Control de lecturas» va por bitácora (también las
/// que no corresponden a una fila) con RUT y LECTURA; «Observaciones del mes» incluye bitácoras leídas antes del corte.
#[test]
fn m6_salidas_con_inventario_matriz_y_control_por_bitacora() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    let mut r = lote(&carpeta, "2026-10-14T15:00:00-03:00");
    r.paginas_lectura[0].vinculos.push(vinculo("X-9-2099", "99999999-9", "OTRA PERSONA FICTICIA", "RLP Hogar Ficticio", "209"));
    r.guardar(&carpeta).unwrap();
    let m = calcular(std::slice::from_ref(&carpeta), dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    assert_eq!(m.lecturas.len(), 4, "una por bitácora del listado");
    assert_eq!(m.observaciones.len(), 4, "las leídas antes del corte también se exportan");
    let destino = t.0.join("Salida");
    salida::publicar(&m, &destino).unwrap();
    let registros = flow_tabular::leer(&destino.join("Registros.xlsx")).unwrap();
    let nombres: Vec<&str> = registros.hojas.iter().map(|h| h.nombre.as_str()).collect();
    assert_eq!(nombres, ["ESPERA", "CUMPLIMIENTO", "INVENTARIO_ARCHIVOS", "MATRIZ_COBERTURA", "ERRORES_Y_ADVERTENCIAS"]);
    let inventario = &registros.hojas[2];
    assert_eq!(inventario.filas.len(), 3);
    assert_eq!(inventario.filas[1][1].texto(), "cumplimiento.xls");
    assert_eq!(inventario.filas[1][4].texto(), "TOMÉ");
    let matriz = &registros.hojas[3];
    let tome = matriz.filas.iter().find(|f| f.first().is_some_and(|c| c.texto() == "TOMÉ")).unwrap();
    assert_eq!(tome[4].texto(), "EXCEL · 3 registro(s)");
    let bitacoras = flow_tabular::leer(&destino.join("Bitacoras_del_mes.xlsx")).unwrap();
    let control = bitacoras.hojas.iter().find(|h| h.nombre == "Control de lecturas").unwrap();
    let cabecera: Vec<String> = control.filas[0].iter().map(|c| c.texto()).collect();
    assert_eq!(cabecera, ["TRIBUNAL", "RIT", "RUT", "NOMBRE", "PROGRAMA", "CAUSA ID", "INGRESO ID", "LECTURA", "CONTROL"]);
    assert_eq!(control.filas.len(), 5);
    // La regeneración relee el control nuevo y conserva inventario y matriz.
    let regenerado = flow_meta4::regenerar::cargar(&destino.join("Registros.xlsx")).unwrap();
    assert_eq!(regenerado.meta.lecturas.iter().filter(|l| l.lectura == "PENDIENTE").count(), 2);
    assert_eq!(regenerado.meta.hojas_previas.len(), 2);
    let copia = t.0.join("Regenerado");
    flow_meta4::regenerar::ejecutar(&destino.join("Registros.xlsx"), &copia).unwrap();
    let libro = flow_tabular::leer(&copia.join("Registros.xlsx")).unwrap();
    let matriz = libro.hojas.iter().find(|h| h.nombre == "MATRIZ_COBERTURA").unwrap();
    let tome = matriz.filas.iter().find(|f| f.first().is_some_and(|c| c.texto() == "TOMÉ")).unwrap();
    assert_eq!(tome[4].texto(), "EXCEL · 3 registro(s)", "la matriz se conserva al regenerar");
}

/// M9: RIT visible vacío marca identidad; persona vacía marca nombre; una fecha ilegible deja esa bitácora como no
/// verificable sin abortar; la consulta sin bitácoras se atribuye al tribunal de la selección; un texto de más de
/// 32767 caracteres se trunca como openpyxl.
#[test]
fn m9_identidad_vacia_fecha_ilegible_atribucion_y_textos_largos() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    let mut r = lote(&carpeta, "2026-10-16T15:00:00-03:00");
    // Lucas (Espera, Laja): una primera fila con un tribunal que no se reconoce.
    let espera = "<html><table><tr><th>RIT</th><th>Tribunal</th><th>RUT</th><th>Nombre</th><th>Tipo</th><th>Sexo</th>\
        <th>Derivación</th><th>T Espera</th><th>Fec. Resolución</th></tr>\
        <tr><td>X-5-2099</td><td>Juzgado Ficticio</td><td>55555555-5</td><td>ANA FICTICIA</td><td>Protección</td><td>F</td><td>FAE PRO Ficticio</td><td>10</td><td>05/09/2026</td></tr>\
        <tr><td>X-4-2099</td><td>Jgdo. L. y G. de Laja</td><td>44444444-2</td><td>LUCAS ARAYA</td><td>Protección</td><td>M</td><td>FAE PRO Ficticio</td><td>35</td><td>05/09/2026</td></tr>\
        </table></html>";
    std::fs::write(carpeta.join("espera.xls"), espera).unwrap();
    r.archivos[1].sha256 = huella(espera.as_bytes());
    r.archivos[1].registros = 2;
    r.guardar(&carpeta).unwrap();
    editar_lectura(&carpeta, 0, |d| d["persona"] = json!(""));
    editar_lectura(&carpeta, 1, |d| d["rit"] = json!(""));
    let m = calcular(std::slice::from_ref(&carpeta), dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    let de = |rit: &str| m.registros.iter().find(|r| r.texto("RIT") == rit).unwrap().control.clone();
    assert_eq!(de("X-1-2099"), format!("{VERIFICADA} · {}", flow_meta4::NOMBRE_VISIBLE));
    assert_eq!(de("X-2-2099"), flow_meta4::IDENTIDAD_VISIBLE);
    assert_eq!(m.sin_bitacoras, vec![("Lote ficticio: tribunal 901 · Espera · FAE".to_string(), "LAJA".to_string())]);

    editar_lectura(&carpeta, 1, |d| {
        d["rit"] = json!("X-2-2099");
        d["entries"][0]["fecha"] = json!("fecha ilegible");
    });
    editar_lectura(&carpeta, 0, |d| d["entries"][0]["texto"] = json!("ñ".repeat(40_000)));
    let m = calcular(std::slice::from_ref(&carpeta), dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    let matias = m.registros.iter().find(|r| r.texto("RIT") == "X-2-2099").unwrap();
    assert_eq!((matias.control.as_str(), matias.fecha_obs), (flow_meta4::NO_VERIFICABLE, None));
    let destino = t.0.join("Salida");
    salida::publicar(&m, &destino).unwrap();
    let libro = flow_tabular::leer(&destino.join("Bitacoras_del_mes.xlsx")).unwrap();
    let largo = libro.hojas[0].filas.iter().flat_map(|f| f.iter()).map(|c| c.texto().chars().count()).max().unwrap();
    assert_eq!(largo, 32_767);
}

/// M2: el corte propuesto es el día de lectura de las bitácoras del lote.
#[test]
fn m2_fecha_de_lectura_de_las_bitacoras() {
    let t = Temporal::nueva();
    let carpeta = t.0.join("Lote ficticio");
    lote(&carpeta, "2026-10-14T15:00:00-03:00");
    let esperado = chrono::DateTime::parse_from_rfc3339("2026-10-14T15:00:00-03:00").unwrap().with_timezone(&chrono::Local).date_naive();
    assert_eq!(flow_meta4::fecha_lectura(&[carpeta]), Some(esperado));
    assert_eq!(flow_meta4::fecha_lectura(&[t.0.join("no existe")]), None);
}

/// M3: una carpeta con varios lotes se expande a ellos (sin carpetas de resultados); sin lotes, mensaje claro.
#[test]
fn m3_carpeta_con_varios_lotes() {
    let t = Temporal::nueva();
    let raiz = t.0.join("Capturas");
    lote(&raiz.join("Lote ficticio"), "2026-10-16T15:00:00-03:00");
    lote_con_vacia(&raiz.join("Lote Yumbel"), true);
    lote(&raiz.join("Resultado META4 2026-10-16").join("Copia"), "2026-10-16T15:00:00-03:00");
    let m = calcular(std::slice::from_ref(&raiz), dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
    assert_eq!(m.lotes, ["Lote Yumbel", "Lote ficticio"]);
    assert_eq!(m.registros.len(), 5);
    let vacia = t.0.join("Sin lotes");
    std::fs::create_dir_all(&vacia).unwrap();
    let error = calcular(&[vacia], dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap_err();
    assert!(error.contains("no contiene un lote"), "{error}");
}
