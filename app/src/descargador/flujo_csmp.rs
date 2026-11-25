//! Descarga conjunta CSMP: selección y recuperación por fase.

use chrono::{Datelike, NaiveDate};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use super::{lote, paquete, Alcance, Catalogo, Edicion, Pantalla, Perfil, PlanLote, SeleccionLote};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use calamine::{Data, Reader};
use super::excel::{self, Hoja};
use super::xls::Celda;

pub const FLOW_DONE: &str = "PREPARADO_SIN_CONSULTA_FIRMAS";
static SOLICITUD_REANUDAR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

pub fn solicitar_retomar(carpeta: PathBuf) {
    if let Ok(mut solicitud) = SOLICITUD_REANUDAR.lock() { *solicitud = Some(carpeta); }
}

pub fn tomar_solicitud() -> Option<PathBuf> {
    SOLICITUD_REANUDAR.lock().ok().and_then(|mut solicitud| solicitud.take())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FasePlan {
    pub clave: String,
    pub modo: String,
    pub mes: Option<(u32, i32)>,
}

/// Equivalente de `months`: mes de corte y el siguiente.
pub fn meses(hoy: NaiveDate) -> [(u32, i32); 2] {
    let siguiente = if hoy.month() == 12 { (1, hoy.year() + 1) } else { (hoy.month() + 1, hoy.year()) };
    [(hoy.month(), hoy.year()), siguiente]
}

/// Valida el alcance y conserva el orden de las fases de Python.
pub fn plan(tribunales: &[String], modalidades: &[String], modo: &str, hoy: NaiveDate, dias: i32) -> Result<Vec<FasePlan>, String> {
    if !matches!(modo, "Espera" | "Cumplimiento") {
        return Err("El flujo conjunto admite Espera o Cumplimiento.".into());
    }
    if !(60..=730).contains(&dias) {
        return Err("Metadato legado de firmas fuera de rango (60–730).".into());
    }
    if tribunales.is_empty() || modalidades.is_empty() {
        return Err("Selecciona tribunales y modalidades.".into());
    }
    let mut fases = vec![FasePlan { clave: "principal".into(), modo: modo.into(), mes: None }];
    fases.extend(meses(hoy).into_iter().map(|(mes, anio)| FasePlan {
        clave: format!("informes-{anio}-{mes}"), modo: "calendario_informes".into(), mes: Some((mes, anio)),
    }));
    Ok(fases)
}

fn clave_seleccion(seleccion: &Value) -> String {
    let campos = ["tribunal", "tab", "modality", "report", "screen", "state", "month", "year", "start", "end"];
    serde_json::to_string(&campos.map(|campo| seleccion.get(campo).cloned().unwrap_or(Value::Null))).unwrap_or_default()
}

/// Reúne consultas completas sin modificar sus lotes de origen.
pub fn reunir_fase(lotes: &[paquete::Lote], esperadas: &[Value], destino: &Path) -> Result<PathBuf, String> {
    let esperado: BTreeSet<_> = esperadas.iter().map(clave_seleccion).collect();
    let nombre = format!("Reunido {}", chrono::Local::now().format("%Y%m%d_%H%M%S_%6f"));
    let carpeta = destino.join(nombre);
    std::fs::create_dir_all(&carpeta).map_err(|e| e.to_string())?;
    let mut vistas = BTreeSet::new();
    let mut consultas = Vec::new();
    let mut plan = Vec::new();
    for lote_origen in lotes {
        for (consulta, manifiesto) in &lote_origen.consultas {
            if !matches!(consulta["estado"].as_str(), Some("VALIDADA" | "SIN_RESULTADOS")) { continue; }
            let seleccion = consulta["seleccion"].clone();
            let clave = clave_seleccion(&seleccion);
            if !esperado.contains(&clave) || !vistas.insert(clave) {
                return Err("Consulta duplicada o fuera del alcance al recuperar la fase.".into());
            }
            let indice = consultas.len() + 1;
            let mut control = manifiesto.clone();
            if let Some(archivos) = control["archivos"].as_array_mut() {
                for item in archivos {
                    if item["estado"] != "OK" { return Err("Una consulta completa contiene un archivo pendiente.".into()); }
                    copiar_original(&lote_origen.carpeta, &carpeta, indice, item)?;
                }
            }
            if control.get("evidencia").is_some_and(|v| !v.is_null()) {
                copiar_original(&lote_origen.carpeta, &carpeta, indice, &mut control["evidencia"])?;
            }
            let archivo = format!("verificacion {indice:03}.json");
            escribir_json(&carpeta.join(&archivo), &control)?;
            let mut consulta = consulta.clone();
            consulta["verificacion"] = json!(archivo);
            consultas.push(consulta);
            plan.push(seleccion);
        }
    }
    if vistas != esperado { return Err("La recuperación no contiene todas las consultas de la fase.".into()); }
    escribir_json(&carpeta.join("resumen.json"), &json!({
        "version_contrato": 1, "estado": "VALIDADA", "consultas": consultas, "plan": plan,
        "consultas_esperadas": esperado.len(), "consultas_completadas": esperado.len(),
        "otros_filtros": "todos los registros", "filtro_fecha": null,
        "fecha_hora": chrono::Local::now().naive_local().to_string(),
    }))?;
    paquete::cargar_lote(&carpeta)?;
    Ok(carpeta)
}

fn copiar_original(origen: &Path, destino: &Path, indice: usize, item: &mut Value) -> Result<(), String> {
    let bytes = paquete::comprobar(origen, item)?;
    let nombre = item["archivo"].as_str().ok_or("El manifiesto contiene una ruta fuera del lote.")?;
    let nuevo = format!("{indice:03}-{nombre}");
    lote::guardar_exclusivo(&destino.join(&nuevo), &bytes)?;
    item["archivo"] = json!(nuevo);
    Ok(())
}

fn escribir_json(ruta: &Path, valor: &Value) -> Result<(), String> {
    let mut datos = serde_json::to_vec_pretty(valor).map_err(|e| e.to_string())?;
    datos.push(b'\n');
    lote::guardar_exclusivo(ruta, &datos)
}

/// Plan de consultas concretas de una fase a partir del catálogo de esa pantalla.
pub fn consultas_fase(fase: &FasePlan, tribunales: &[String], modalidades: &[String], catalogo: &Catalogo) -> Result<PlanLote, String> {
    if let Some((mes, anio)) = fase.mes {
        let mut consultas = Vec::new();
        for tribunal in tribunales {
            let seleccion = super::Seleccion { tribunal: tribunal.clone(), mes: mes.to_string(), anio: anio.to_string(), ..Default::default() };
            consultas.push(catalogo.preparar(Edicion::Integral, &seleccion)?);
        }
        Ok(PlanLote { consultas, fechas: None })
    } else {
        let seleccion = SeleccionLote { tribunales: tribunales.to_vec(), modalidades: modalidades.to_vec(), pestanas: vec![fase.modo.clone()], medidas: vec![super::TODAS_LAS_MEDIDAS.into()], ..Default::default() };
        catalogo.planificar(Edicion::Integral, &seleccion)
    }
}

fn catalogo_fase(conexion: &dyn lote::Conexion, fase: &FasePlan) -> Result<Catalogo, String> {
    let pantalla = if fase.mes.is_some() { "calendario_informes" } else { "seguimiento" };
    let datos = conexion.llamar("catalog", json!({"screen": pantalla}), Duration::from_secs(75))?;
    if datos["pantalla"] != pantalla { return Err("Chrome devolvió un catálogo de otra pantalla.".into()); }
    serde_json::from_value(datos).map_err(|_| "Chrome no devolvió un catálogo válido.".into())
}

fn seleccion_de(perfil: &Perfil) -> Value {
    let mut seleccion = lote::seleccion_edicion(perfil, Edicion::Integral);
    // Los campos de identidad son los de `perfiles.Selection`.
    if perfil.pantalla == Pantalla::Seguimiento {
        if let Alcance::Seguimiento { reporte: None, .. } = &perfil.alcance { seleccion["report"] = Value::Null; }
    }
    seleccion
}

fn guardar_estado(ruta: &Path, datos: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(datos).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    let temporal = ruta.with_extension("tmp");
    std::fs::write(&temporal, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(temporal, ruta).map_err(|e| e.to_string())
}

/// Ejecuta o recupera una fase, reutilizando las consultas validadas ya guardadas.
#[allow(clippy::too_many_arguments)]
pub fn ejecutar_fase(
    conexion: &dyn lote::Conexion, fase: &FasePlan, tribunales: &[String], modalidades: &[String],
    raiz: &Path, estado: &mut Value, opciones: &lote::Opciones, cancelar: &AtomicBool,
    emitir: &mut dyn FnMut(lote::Evento),
) -> Result<PathBuf, String> {
    if cancelar.load(Ordering::Relaxed) { return Err("Flujo cancelado; conserva las fases comprobadas.".into()); }
    let previo = estado["fases"][&fase.clave].clone();
    if previo["estado"] == "VALIDADA" {
        let carpeta = super::archivo_local::ruta_fase(raiz, &previo["carpeta"])?;
        paquete::cargar_lote(&carpeta)?;
        return Ok(carpeta);
    }
    let catalogo = catalogo_fase(conexion, fase)?;
    let plan_completo = consultas_fase(fase, tribunales, modalidades, &catalogo)?;
    let esperadas: Vec<_> = plan_completo.consultas.iter().map(seleccion_de).collect();
    let mut nombres: Vec<String> = previo["carpetas_parciales"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
    if let Some(carpeta) = previo["carpeta"].as_str().filter(|s| !s.is_empty()) {
        if !nombres.iter().any(|n| n == carpeta) { nombres.push(carpeta.to_string()); }
    }
    let anteriores = nombres.iter().map(|nombre| {
        let carpeta = super::archivo_local::ruta_fase(raiz, &json!(nombre))?;
        paquete::cargar_lote_con(&carpeta, true)
    }).collect::<Result<Vec<_>, String>>()?;
    let terminadas: BTreeSet<_> = anteriores.iter().flat_map(|l| &l.consultas)
        .filter(|(q, _)| matches!(q["estado"].as_str(), Some("VALIDADA" | "SIN_RESULTADOS")))
        .map(|(q, _)| clave_seleccion(&q["seleccion"])).collect();
    let pendientes: Vec<_> = plan_completo.consultas.iter().filter(|p| !terminadas.contains(&clave_seleccion(&seleccion_de(p)))).cloned().collect();
    let destino = raiz.join(&fase.clave);
    std::fs::create_dir_all(&destino).map_err(|e| e.to_string())?;
    let mut carpeta_nueva = None;
    let resultado = if pendientes.is_empty() {
        reunir_fase(&anteriores, &esperadas, &destino)
    } else {
        let plan_pendiente = PlanLote { consultas: pendientes, fechas: plan_completo.fechas };
        let ejecucion = lote::ejecutar(conexion, &catalogo, Edicion::Integral, &plan_pendiente, &destino, opciones, cancelar, &mut |evento| {
            if let lote::Evento::Carpeta(carpeta) = &evento { carpeta_nueva = Some(carpeta.clone()); }
            emitir(evento);
        });
        ejecucion.and_then(|_| {
            let carpeta = carpeta_nueva.clone().ok_or("La fase no identifica su carpeta de fuentes.")?;
            if anteriores.is_empty() { return Ok(carpeta); }
            let mut lotes = anteriores;
            lotes.push(paquete::cargar_lote(&carpeta)?);
            reunir_fase(&lotes, &esperadas, &destino)
        })
    };
    match &resultado {
        Ok(carpeta) => {
            let relativa = carpeta.strip_prefix(raiz).map_err(|_| "La fuente de recuperación está fuera del flujo.")?.to_string_lossy().replace('\\', "/");
            estado["fases"][&fase.clave] = json!({"estado": "VALIDADA", "carpeta": relativa, "consultas": esperadas.len()});
        }
        Err(_) => {
            let carpeta = carpeta_nueva.as_ref().and_then(|p| p.strip_prefix(raiz).ok()).map(|p| p.to_string_lossy().replace('\\', "/"))
                .or_else(|| previo["carpeta"].as_str().map(str::to_string)).unwrap_or_default();
            estado["fases"][&fase.clave] = json!({"estado": "INCOMPLETA", "carpeta": carpeta, "carpetas_parciales": nombres});
        }
    }
    guardar_estado(&raiz.join("flujo.json"), estado)?;
    resultado
}

/// Forma el libro conjunto y agrega la cobertura de firmas no consultadas.
pub fn ensamblar(lotes: &[paquete::Lote], tribunales: &[String], hoy: NaiveDate, dias: u32, destino: &Path) -> Result<PathBuf, String> {
    let ahora = chrono::Local::now().fixed_offset();
    let base = super::resultados::libro_consolidado(lotes, &ahora)?;
    let mut libro = calamine::open_workbook_auto_from_rs(std::io::Cursor::new(base)).map_err(|e| e.to_string())?;
    let mut hojas: Vec<Hoja> = libro.worksheets().into_iter().map(|(nombre, rango)| {
        let filas = rango.rows().map(|fila| fila.iter().map(|valor| match valor {
            Data::Empty | Data::Error(_) => Celda::Vacia,
            Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => Celda::Texto(s.clone()),
            Data::Float(x) => Celda::Real(*x),
            Data::Int(x) => Celda::Entero(*x),
            Data::Bool(x) => Celda::Logico(*x),
            Data::DateTime(x) => {
                let serial = x.as_f64();
                if (1.0..2_958_466.0).contains(&serial) {
                    let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();
                    Celda::Fecha(base + chrono::Duration::days(serial.floor() as i64) + chrono::Duration::seconds(((serial - serial.floor()) * 86_400.0).round() as i64))
                } else { Celda::Real(serial) }
            },
        }).collect()).collect();
        Hoja::simple(&nombre, filas)
    }).collect();
    completar_rut_informes(&mut hojas)?;
    let desde = hoy.checked_sub_days(chrono::Days::new(u64::from(dias.saturating_sub(1)))).ok_or("Fecha fuera de rango.")?;
    let texto = |s: &str| Celda::Texto(s.to_string());
    let mut cobertura = vec![["tribunal_codigo", "desde", "hasta", "estado", "criterio_temporal", "sin_coincidencias"].into_iter().map(texto).collect()];
    for tribunal in tribunales {
        cobertura.push(vec![texto(tribunal), texto(&desde.to_string()), texto(&hoy.to_string()), texto("NO_CONSULTADA"),
            texto("consulta_firmas_retirada_del_flujo"), texto("Resoluciones firmadas no consultadas; no acredita ausencia de movimientos.")]);
    }
    hojas.push(Hoja::simple("SITFA_COBERTURA", cobertura));
    let bytes = excel::libro(&hojas)?;
    super::resultados::publicar(destino, "Flujo CSMP", ".xlsx", &ahora, &bytes)
}

fn completar_rut_informes(hojas: &mut [Hoja]) -> Result<(), String> {
    use std::collections::{BTreeMap, BTreeSet};
    let Some(principal) = hojas.iter().position(|h| matches!(h.nombre.as_str(), "ESPERA" | "CUMPLIMIENTO")) else { return Ok(()) };
    let Some(auxiliar) = hojas.iter().position(|h| h.nombre == "INFORMES") else { return Ok(()) };
    let cabeceras = |h: &Hoja| h.filas.first().into_iter().flatten().map(Celda::texto_python).collect::<Vec<_>>();
    let p = paquete::mapeo(&cabeceras(&hojas[principal]))?;
    let a = paquete::mapeo(&cabeceras(&hojas[auxiliar]))?;
    let claves = ["rit", "nombre", "tribunal", "programa"];
    if !claves.iter().all(|k| p.contains_key(k) && a.contains_key(k)) || !p.contains_key("rut") || a.contains_key("rut") { return Ok(()); }
    let llave = |fila: &[Celda], indices: &std::collections::BTreeMap<&str, usize>| -> Vec<String> {
        claves.iter().map(|k| paquete::normalizado(&fila.get(indices[k]).map(Celda::texto_python).unwrap_or_default())).collect()
    };
    let mut indice: BTreeMap<Vec<String>, BTreeSet<String>> = BTreeMap::new();
    for fila in hojas[principal].filas.iter().skip(1) {
        let clave = llave(fila, &p);
        if clave.iter().all(|s| !s.is_empty()) {
            indice.entry(clave).or_default().insert(fila.get(p["rut"]).map(Celda::texto_python).unwrap_or_default().trim().to_string());
        }
    }
    let hoja = &mut hojas[auxiliar];
    if let Some(cabecera) = hoja.filas.first_mut() { cabecera.extend([Celda::Texto("RUT".into()), Celda::Texto("SITFA_ORIGEN_RUT".into())]); }
    for fila in hoja.filas.iter_mut().skip(1) {
        let coincidencias = indice.get(&llave(fila, &a));
        let rut = coincidencias.filter(|v| v.len() == 1).and_then(|v| v.first()).filter(|s| !s.is_empty());
        fila.push(rut.map(|v| Celda::Texto(v.clone())).unwrap_or(Celda::Vacia));
        fila.push(if rut.is_some() { Celda::Texto("Identidad única RIT/tribunal/persona/centro de hoja principal".into()) } else { Celda::Vacia });
    }
    Ok(())
}

/// Ejecuta el conjunto o lo retoma desde su control local.
#[allow(clippy::too_many_arguments)]
pub fn ejecutar(
    conexion: &dyn lote::Conexion, tribunales: &[String], modalidades: &[String], modo: &str,
    destino: &Path, hoy: NaiveDate, dias: u32, recuperar: Option<&Path>, opciones: &lote::Opciones,
    cancelar: &AtomicBool, emitir: &mut dyn FnMut(lote::Evento),
) -> Result<PathBuf, String> {
    use sha2::{Digest, Sha256};
    let fases = plan(tribunales, modalidades, modo, hoy, dias as i32)?;
    let identidad = json!({"tribunales": tribunales, "modalidades": modalidades, "modo": modo,
        "corte": hoy.to_string(), "dias_firmas": dias, "consultar_firmas": false});
    let (raiz, mut estado) = if let Some(carpeta) = recuperar {
        let raiz = carpeta.canonicalize().map_err(|e| e.to_string())?;
        let bytes = std::fs::read(raiz.join("flujo.json")).map_err(|e| e.to_string())?;
        let mut estado: Value = serde_json::from_slice(&bytes).map_err(|error| {
            if error.is_eof() && bytes.iter().rev().find(|b| !b.is_ascii_whitespace()) == Some(&b'{') {
                let texto = String::from_utf8_lossy(&bytes);
                let posicion = texto.chars().count();
                let columna = texto.rsplit('\n').next().unwrap_or("").chars().count() + 1;
                format!("Expecting property name enclosed in double quotes: line {} column {columna} (char {posicion})", texto.lines().count().max(1))
            } else { format!("Resumen JSON inválido: {error}") }
        })?;
        if !estado.is_object() || !estado["fases"].is_object() || !estado["seleccion"].is_object() {
            return Err("Resumen JSON inválido.".into());
        }
        if estado["seleccion"].get("consultar_firmas").is_none() { estado["seleccion"]["consultar_firmas"] = json!(false); }
        if estado["seleccion"] != identidad { return Err("La recuperación no coincide con la selección y fecha de corte.".into()); }
        let retiradas: Vec<_> = estado["fases"].as_object().unwrap().keys().filter(|k| k.starts_with("carga-")).cloned().collect();
        for clave in retiradas {
            let fase = estado["fases"].as_object_mut().unwrap().remove(&clave).unwrap();
            if !estado["fases_fuera_del_flujo"].is_object() { estado["fases_fuera_del_flujo"] = json!({}); }
            estado["fases_fuera_del_flujo"][clave] = fase;
        }
        (raiz, estado)
    } else {
        let raiz = destino.join(format!("Flujo CSMP {}", chrono::Local::now().format("%Y-%m-%d %H-%M-%S-%6f")));
        std::fs::create_dir_all(&raiz).map_err(|e| e.to_string())?;
        let raiz = raiz.canonicalize().map_err(|e| e.to_string())?;
        (raiz, json!({"version_contrato": 1, "seleccion": identidad, "estado": "EN_CURSO", "fases": {}}))
    };
    guardar_estado(&raiz.join("flujo.json"), &estado)?;
    let resultado = (|| {
        let mut fuentes = Vec::new();
        for fase in &fases {
            let carpeta = ejecutar_fase(conexion, fase, tribunales, modalidades, &raiz, &mut estado, opciones, cancelar, emitir)?;
            fuentes.push(paquete::cargar_lote(&carpeta)?);
        }
        let libro = ensamblar(&fuentes, tribunales, hoy, dias, &raiz)?;
        let bytes = std::fs::read(&libro).map_err(|e| e.to_string())?;
        let desde = hoy.checked_sub_days(chrono::Days::new(u64::from(dias - 1))).ok_or("Fecha fuera de rango.")?;
        let cobertura: serde_json::Map<String, Value> = tribunales.iter().map(|t| (t.clone(), json!({
            "desde": desde.to_string(), "hasta": hoy.to_string(), "estado": "NO_CONSULTADA",
            "criterio_temporal": "consulta_firmas_retirada_del_flujo",
            "sin_coincidencias": "Resoluciones firmadas no consultadas; no acredita ausencia de movimientos."
        }))).collect();
        estado["estado"] = json!(FLOW_DONE);
        estado["archivo"] = json!(libro.file_name().unwrap_or_default().to_string_lossy());
        estado["sha256"] = json!(format!("{:x}", Sha256::digest(bytes)));
        estado["cobertura"] = json!(cobertura);
        guardar_estado(&raiz.join("flujo.json"), &estado)?;
        emitir(lote::Evento::Carpeta(raiz.clone()));
        Ok(libro)
    })();
    if resultado.is_err() {
        estado["estado"] = json!("INCOMPLETO");
        guardar_estado(&raiz.join("flujo.json"), &estado)?;
        emitir(lote::Evento::Carpeta(raiz));
    }
    resultado
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Dos pantallas ficticias con planillas compatibles con CSMP.
    struct SitfaConjunto {
        seguimiento: super::super::sitfa_ficticio::SitfaFicticio,
        informes: super::super::sitfa_ficticio::SitfaFicticio,
        pantalla: std::cell::RefCell<String>,
        consultas: std::cell::RefCell<Vec<(String, String)>>,
        corte: std::cell::RefCell<Option<(String, usize)>>,
    }

    impl SitfaConjunto {
        fn nuevo(corte: Option<(&str, usize)>) -> Self {
            Self { seguimiento: super::super::sitfa_ficticio::SitfaFicticio::nuevo("seguimiento", &[]),
                informes: super::super::sitfa_ficticio::SitfaFicticio::nuevo("calendario_informes", &[]),
                pantalla: std::cell::RefCell::new(String::new()), consultas: std::cell::RefCell::new(Vec::new()),
                corte: std::cell::RefCell::new(corte.map(|(s, n)| (s.to_string(), n))) }
        }
    }

    impl lote::Conexion for SitfaConjunto {
        fn llamar(&self, comando: &str, carga: Value, plazo: Duration) -> Result<Value, String> {
            use base64::Engine;
            if comando == "catalog" {
                let pantalla = carga["screen"].as_str().unwrap_or("");
                *self.pantalla.borrow_mut() = pantalla.to_string();
                return Ok(if pantalla == "seguimiento" { json!({"pantalla":pantalla,"tribunales":{"901":"Juzgado de Familia Ficticio Norte","902":"Juzgado de Familia Ficticio Sur"},"modalidades":{"2":"Ambulatorio"},"pestanas":["Espera","Cumplimiento"],"medidas":{"0":"Todas"}}) }
                    else { json!({"pantalla":pantalla,"tribunales":{"901":"Juzgado de Familia Ficticio Norte","902":"Juzgado de Familia Ficticio Sur"},"meses":{"10":"Octubre","11":"Noviembre"},"anios":{"2026":"2026"}}) });
            }
            let pantalla = self.pantalla.borrow().clone();
            if comando == "prepare" {
                let tribunal = carga["selection"]["tribunal"].as_str().unwrap_or("").to_string();
                self.consultas.borrow_mut().push((pantalla.clone(), tribunal));
                let mut corte = self.corte.borrow_mut();
                if let Some((fase, faltan)) = corte.as_mut() {
                    if *fase == pantalla {
                        if *faltan == 0 { *corte = None; return Err("Corte ficticio".into()); }
                        *faltan -= 1;
                    }
                }
            }
            let conectado = if pantalla == "seguimiento" { &self.seguimiento } else { &self.informes };
            let mut respuesta = conectado.llamar(comando, carga, plazo)?;
            if comando == "download" {
                let contenido = base64::engine::general_purpose::STANDARD.decode(respuesta["body"].as_str().unwrap()).unwrap();
                let texto = String::from_utf8(contenido).unwrap();
                let mut filas = texto.split("</tr>");
                let cabecera = filas.next().unwrap_or("");
                let extra = if pantalla == "seguimiento" { "<td>NOMBRE CENTRO</td><td>T ESPERA</td><td>DIAS DE CUMPLIMIENTO</td><td>DIAS PARA EGRESAR</td>" } else { "<td>NOMBRE CENTRO</td>" };
                let valores = if pantalla == "seguimiento" { "<td>PRM ficticio</td><td>20</td><td>50</td><td>90</td>" } else { "<td>PRM ficticio</td>" };
                let mut nuevo = format!("{cabecera}{extra}</tr>");
                for fila in filas {
                    if fila.contains("<tr>") { nuevo.push_str(fila); nuevo.push_str(valores); nuevo.push_str("</tr>"); }
                    else { nuevo.push_str(fila); }
                }
                respuesta["body"] = json!(base64::engine::general_purpose::STANDARD.encode(nuevo.as_bytes()));
            }
            Ok(respuesta)
        }
    }

    #[test]
    fn flujo_csmp_integral_con_cortes_y_reanudacion() {
        for (modo, corte) in [("Espera", None), ("Cumplimiento", None),
            ("Cumplimiento", Some(("seguimiento", 1))), ("Cumplimiento", Some(("calendario_informes", 0)))] {
            let base = std::env::temp_dir().join(format!("flow-csmp-integral-{}-{}", std::process::id(), chrono::Local::now().timestamp_nanos_opt().unwrap_or_default()));
            std::fs::create_dir_all(&base).unwrap();
            let puente = SitfaConjunto::nuevo(corte);
            let tribunales = vec!["901".to_string(), "902".to_string()];
            let modalidades = vec!["2".to_string()];
            let fecha = NaiveDate::from_ymd_opt(2026, 10, 2).unwrap();
            let opciones = lote::Opciones { evidencia: false, ..Default::default() };
            let cancelar = AtomicBool::new(false);
            let primera = ejecutar(&puente, &tribunales, &modalidades, modo, &base, fecha, 60, None, &opciones, &cancelar, &mut |_| {});
            let libro = if corte.is_some() {
                assert!(primera.is_err(), "el corte debe interrumpir el flujo");
                let carpeta = std::fs::read_dir(&base).unwrap().next().unwrap().unwrap().path();
                let previo: Value = serde_json::from_slice(&std::fs::read(carpeta.join("flujo.json")).unwrap()).unwrap();
                assert_eq!(previo["estado"], "INCOMPLETO");
                ejecutar(&puente, &tribunales, &modalidades, modo, &base, fecha, 60, Some(&carpeta), &opciones, &cancelar, &mut |_| {}).unwrap()
            } else { primera.unwrap() };
            let estado: Value = serde_json::from_slice(&std::fs::read(libro.parent().unwrap().join("flujo.json")).unwrap()).unwrap();
            assert_eq!(estado["estado"], FLOW_DONE);
            assert_eq!(estado["fases"].as_object().unwrap().len(), 3);
            assert!(estado["fases"].as_object().unwrap().values().all(|f| f["estado"] == "VALIDADA"));
            let mut xlsx = calamine::open_workbook_auto(&libro).unwrap();
            assert_eq!(xlsx.worksheet_range("SITFA_COBERTURA").unwrap().height(), 3);
            if corte == Some(("seguimiento", 1)) {
                let consultas = puente.consultas.borrow();
                assert_eq!(consultas.iter().filter(|(pantalla, tribunal)| pantalla == "seguimiento" && tribunal == "901").count(), 1);
            }
            let _ = std::fs::remove_dir_all(base);
        }
    }

    fn comparar_caso(caso: &Value) -> Result<(), String> {
        use base64::Engine;
        let nombre = caso["caso"].as_str().unwrap();
        let hoy = NaiveDate::parse_from_str(caso["fecha"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        let lista = |clave: &str| caso[clave].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect::<Vec<_>>();
        let tribunales = lista("tribunales");
        let modalidades = lista("modalidades");
        let dias = caso["dias"].as_i64().unwrap() as i32;
        let fases = match plan(&tribunales, &modalidades, caso["modo"].as_str().unwrap(), hoy, dias) {
            Err(error) => return if caso["error"] == error { Ok(()) } else { Err(format!("{nombre}: mensaje {error}")) },
            Ok(fases) => fases,
        };
        if nombre == "control_danado" {
            struct SinConexion;
            impl lote::Conexion for SinConexion {
                fn llamar(&self, _: &str, _: Value, _: Duration) -> Result<Value, String> { Err("No se esperaban consultas.".into()) }
            }
            let carpeta = std::env::temp_dir().join(format!("flujo-danado-{}-{}", std::process::id(), chrono::Local::now().timestamp_nanos_opt().unwrap_or_default()));
            std::fs::create_dir_all(&carpeta).map_err(|e| e.to_string())?;
            std::fs::write(carpeta.join("flujo.json"), b"{").map_err(|e| e.to_string())?;
            let resultado = ejecutar(&SinConexion, &tribunales, &modalidades, caso["modo"].as_str().unwrap(), &carpeta, hoy, dias as u32, Some(&carpeta), &lote::Opciones::default(), &AtomicBool::new(false), &mut |_| {});
            let _ = std::fs::remove_dir_all(carpeta);
            return match resultado { Err(error) if caso["error"] == error => Ok(()), Err(error) => Err(format!("{nombre}: {error} != {}", caso["error"])), Ok(_) => Err("Se aceptó el control dañado".into()) };
        }
        let esperadas = caso["estado"]["fases"].as_array().ok_or("Sin fases esperadas")?;
        if fases.len() != esperadas.len() || fases.iter().zip(esperadas).any(|(a, b)| b["clave"] != a.clave || b["estado"] != "VALIDADA") {
            return Err(format!("{nombre}: fases distintas"));
        }
        let base = std::env::temp_dir().join(format!("flow-csmp-oraculo-{}-{}", std::process::id(), chrono::Local::now().timestamp_nanos_opt().unwrap_or_default()));
        std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        let resultado = (|| {
            struct SinConsultas;
            impl lote::Conexion for SinConsultas {
                fn llamar(&self, _: &str, _: Value, _: Duration) -> Result<Value, String> { Err("Una fase validada no debe volver a consultarse.".into()) }
            }
            let mut estado = json!({"version_contrato": 1, "estado":"INCOMPLETO", "seleccion": {
                "tribunales":tribunales, "modalidades":modalidades, "modo":caso["modo"], "corte":caso["fecha"],
                "dias_firmas":dias, "consultar_firmas":false}, "fases":{}});
            for (i, archivos) in caso["fuentes"].as_array().ok_or("Sin fuentes")?.iter().enumerate() {
                let carpeta = base.join(format!("fase-{i}"));
                for (ruta, contenido) in archivos.as_object().ok_or("Fuentes inválidas")? {
                    let archivo = carpeta.join(ruta);
                    std::fs::create_dir_all(archivo.parent().unwrap()).map_err(|e| e.to_string())?;
                    std::fs::write(archivo, base64::engine::general_purpose::STANDARD.decode(contenido.as_str().unwrap()).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                }
                paquete::cargar_lote(&carpeta)?;
                estado["fases"][&fases[i].clave] = json!({"estado":"VALIDADA", "carpeta":format!("fase-{i}"), "consultas":esperadas[i]["consultas"]});
            }
            guardar_estado(&base.join("flujo.json"), &estado)?;
            let ruta = ejecutar(&SinConsultas, &tribunales, &modalidades, caso["modo"].as_str().unwrap(), &base, hoy, dias as u32,
                Some(&base), &lote::Opciones::default(), &AtomicBool::new(false), &mut |_| {})?;
            let resultado_estado: Value = serde_json::from_slice(&std::fs::read(base.join("flujo.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            if resultado_estado["estado"] != caso["estado"]["estado"] { return Err(format!("{nombre}: estado final distinto")); }
            let mut libro = calamine::open_workbook_auto(&ruta).map_err(|e| e.to_string())?;
            for (nombre_hoja, esperado) in caso["hojas"].as_object().ok_or("Sin hojas esperadas")? {
                let rango = libro.worksheet_range(nombre_hoja).map_err(|e| e.to_string())?;
                let ancho = if matches!(nombre_hoja.as_str(), "ESPERA" | "CUMPLIMIENTO") { 8 } else { 6 };
                let filas: Vec<Vec<Value>> = rango.rows().map(|fila| (0..ancho.min(fila.len())).map(|i| match &fila[i] {
                    Data::Empty => Value::Null,
                    Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => json!(s),
                    Data::Int(n) => json!(n),
                    Data::Float(n) if n.fract() == 0.0 => json!(*n as i64),
                    Data::Float(n) => json!(n),
                    Data::Bool(b) => json!(b),
                    Data::DateTime(n) => json!(n.as_f64()),
                    Data::Error(e) => json!(format!("{e:?}")),
                }).collect()).collect();
                if json!(filas) != *esperado { return Err(format!("{nombre}: hoja {nombre_hoja} distinta\nRust: {}\nPython: {esperado}", json!(filas))); }
            }
            Ok(())
        })();
        let _ = std::fs::remove_dir_all(base);
        resultado
    }

    #[test]
    fn flujo_csmp_igual_a_python() {
        let oracle: Value = serde_json::from_str(include_str!("../../tests/datos/flujo_csmp.json")).unwrap();
        let casos = oracle["casos"].as_array().unwrap();
        assert_eq!(casos.len(), 8);
        for caso in casos { comparar_caso(caso).unwrap(); }
    }

    #[test]
    fn oraculo_de_flujo_csmp_detecta_alteraciones() {
        let oracle: Value = serde_json::from_str(include_str!("../../tests/datos/flujo_csmp.json")).unwrap();
        let casos = oracle["casos"].as_array().unwrap();
        let mut fase = casos.iter().find(|v| v["caso"] == "cumplimiento").unwrap().clone();
        fase["estado"]["fases"][0]["estado"] = json!("ALTERADA");
        assert!(comparar_caso(&fase).is_err());
        let mut mensaje = casos.iter().find(|v| v["caso"] == "modo_invalido").unwrap().clone();
        mensaje["error"] = json!("ALTERADO");
        assert!(comparar_caso(&mensaje).is_err());
        let mut celda = casos.iter().find(|v| v["caso"] == "cumplimiento").unwrap().clone();
        celda["hojas"]["SITFA_COBERTURA"][1][3] = json!("ALTERADA");
        assert!(comparar_caso(&celda).is_err());
    }

    #[test]
    fn flujo_csmp_reune_solo_consultas_comprobadas() {
        use base64::Engine;
        let oracle: Value = serde_json::from_str(include_str!("../../tests/datos/flujo_csmp.json")).unwrap();
        let caso = oracle["casos"].as_array().unwrap().iter().find(|v| v["caso"] == "retomar_principal").unwrap();
        let base = std::env::temp_dir().join(format!("flow-csmp-reunir-{}-{}", std::process::id(), chrono::Local::now().timestamp_nanos_opt().unwrap_or_default()));
        let fuente = base.join("fuente");
        for (ruta, contenido) in caso["fuentes"][0].as_object().unwrap() {
            let archivo = fuente.join(ruta);
            std::fs::create_dir_all(archivo.parent().unwrap()).unwrap();
            std::fs::write(archivo, base64::engine::general_purpose::STANDARD.decode(contenido.as_str().unwrap()).unwrap()).unwrap();
        }
        let original = paquete::cargar_lote(&fuente).unwrap();
        assert_eq!(original.consultas.len(), 2);
        let esperadas: Vec<_> = original.consultas.iter().map(|(q, _)| q["seleccion"].clone()).collect();
        let partes: Vec<_> = original.consultas.iter().map(|c| paquete::Lote { carpeta: original.carpeta.clone(), resumen: original.resumen.clone(), consultas: vec![c.clone()], parcial: true }).collect();
        let reunido = reunir_fase(&partes, &esperadas, &base.join("salida")).unwrap();
        assert_eq!(paquete::cargar_lote(&reunido).unwrap().consultas.len(), 2);
        let duplicados = vec![paquete::Lote { carpeta: original.carpeta.clone(), resumen: original.resumen.clone(), consultas: vec![original.consultas[0].clone(), original.consultas[0].clone()], parcial: true }];
        assert_eq!(reunir_fase(&duplicados, &esperadas, &base.join("duplicado")).unwrap_err(), "Consulta duplicada o fuera del alcance al recuperar la fase.");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn flujo_csmp_retomar_consulta_solo_tribunal_pendiente() {
        use base64::Engine;
        use std::cell::RefCell;
        use super::super::sitfa_ficticio::SitfaFicticio;
        struct PuenteFicticio { interno: SitfaFicticio, preparados: RefCell<Vec<String>> }
        impl lote::Conexion for PuenteFicticio {
            fn llamar(&self, comando: &str, carga: Value, plazo: Duration) -> Result<Value, String> {
                if comando == "catalog" { return Ok(json!({"pantalla":"seguimiento", "tribunales":{"113":"Jgdo. L. y G. de Laja", "119":"Jgdo. L. y G. de Mulchén"}, "pestanas":["Cumplimiento"], "modalidades":{"2":"Ambulatorio"}, "medidas":{"0":"Todas"}})); }
                if comando == "prepare" { self.preparados.borrow_mut().push(carga["selection"]["tribunal"].as_str().unwrap_or("").to_string()); }
                self.interno.llamar(comando, carga, plazo)
            }
        }
        let oracle: Value = serde_json::from_str(include_str!("../../tests/datos/flujo_csmp.json")).unwrap();
        let caso = oracle["casos"].as_array().unwrap().iter().find(|v| v["caso"] == "retomar_principal").unwrap();
        let base = std::env::temp_dir().join(format!("flow-csmp-reanudar-{}-{}", std::process::id(), chrono::Local::now().timestamp_nanos_opt().unwrap_or_default()));
        let raiz = base.join("conjunto");
        let anterior = raiz.join("anterior");
        for (ruta, contenido) in caso["fuentes"][0].as_object().unwrap() {
            let archivo = anterior.join(ruta);
            std::fs::create_dir_all(archivo.parent().unwrap()).unwrap();
            std::fs::write(archivo, base64::engine::general_purpose::STANDARD.decode(contenido.as_str().unwrap()).unwrap()).unwrap();
        }
        let resumen_path = anterior.join("resumen.json");
        let mut resumen: Value = serde_json::from_slice(&std::fs::read(&resumen_path).unwrap()).unwrap();
        resumen["consultas"].as_array_mut().unwrap().truncate(1);
        resumen["estado"] = json!("INCOMPLETA");
        resumen["consultas_completadas"] = json!(1);
        std::fs::write(&resumen_path, serde_json::to_vec(&resumen).unwrap()).unwrap();
        let raiz = raiz.canonicalize().unwrap();
        let mut estado = json!({"fases":{"principal":{"estado":"INCOMPLETA", "carpeta":"anterior", "carpetas_parciales":[]}}});
        let puente = PuenteFicticio { interno: SitfaFicticio::nuevo("seguimiento", &[]), preparados: RefCell::new(Vec::new()) };
        let tribunales = vec!["113".into(), "119".into()];
        let modalidades = vec!["2".into()];
        let fase = plan(&tribunales, &modalidades, "Cumplimiento", NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(), 60).unwrap().remove(0);
        let salida = ejecutar_fase(&puente, &fase, &tribunales, &modalidades, &raiz, &mut estado, &lote::Opciones::default(), &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(*puente.preparados.borrow(), ["119"]);
        assert_eq!(paquete::cargar_lote(&salida).unwrap().consultas.len(), 2);
        assert_eq!(estado["fases"]["principal"]["estado"], "VALIDADA");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn flujo_csmp_retomar_informes_completos_sin_repetir_consultas() {
        use base64::Engine;
        struct SoloCatalogo;
        impl lote::Conexion for SoloCatalogo {
            fn llamar(&self, comando: &str, _: Value, _: Duration) -> Result<Value, String> {
                if comando != "catalog" { return Err("No se debe repetir una consulta validada.".into()); }
                Ok(json!({"pantalla":"calendario_informes", "tribunales":{"113":"Jgdo. L. y G. de Laja"}, "meses":{"10":"Octubre", "11":"Noviembre"}, "anios":{"2026":"2026"}}))
            }
        }
        let oracle: Value = serde_json::from_str(include_str!("../../tests/datos/flujo_csmp.json")).unwrap();
        let caso = oracle["casos"].as_array().unwrap().iter().find(|v| v["caso"] == "retomar_informes").unwrap();
        let base = std::env::temp_dir().join(format!("flow-csmp-informes-{}-{}", std::process::id(), chrono::Local::now().timestamp_nanos_opt().unwrap_or_default()));
        let anterior = base.join("anterior");
        for (ruta, contenido) in caso["fuentes"][1].as_object().unwrap() {
            let archivo = anterior.join(ruta);
            std::fs::create_dir_all(archivo.parent().unwrap()).unwrap();
            std::fs::write(archivo, base64::engine::general_purpose::STANDARD.decode(contenido.as_str().unwrap()).unwrap()).unwrap();
        }
        let raiz = base.canonicalize().unwrap();
        let tribunales = vec!["113".into()];
        let modalidades = vec!["2".into()];
        let fase = plan(&tribunales, &modalidades, "Cumplimiento", NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(), 60).unwrap().remove(1);
        let mut estado = json!({"fases":{}});
        estado["fases"][&fase.clave] = json!({"estado":"INCOMPLETA", "carpeta":"anterior", "carpetas_parciales":[]});
        let salida = ejecutar_fase(&SoloCatalogo, &fase, &tribunales, &modalidades, &raiz, &mut estado, &lote::Opciones::default(), &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(paquete::cargar_lote(&salida).unwrap().consultas.len(), 1);
        assert_eq!(estado["fases"][&fase.clave]["estado"], "VALIDADA");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn flujo_csmp_planifica_fases_y_valida_alcance() {
        let fecha = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let tribunales = vec!["113".to_string()];
        let modalidades = vec!["2".to_string()];
        let fases = plan(&tribunales, &modalidades, "Espera", fecha, 60).unwrap();
        assert_eq!(fases.iter().map(|f| f.clave.as_str()).collect::<Vec<_>>(), ["principal", "informes-2026-12", "informes-2027-1"]);
        assert_eq!(plan(&tribunales, &modalidades, "Carga", fecha, 60).unwrap_err(), "El flujo conjunto admite Espera o Cumplimiento.");
        assert_eq!(plan(&tribunales, &modalidades, "Espera", fecha, 59).unwrap_err(), "Metadato legado de firmas fuera de rango (60–730).");
        assert_eq!(plan(&[], &modalidades, "Espera", fecha, 60).unwrap_err(), "Selecciona tribunales y modalidades.");
    }
}
