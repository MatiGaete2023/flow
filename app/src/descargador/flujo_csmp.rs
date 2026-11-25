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
            Data::DateTime(x) => Celda::Real(x.as_f64()),
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
        let mut estado: Value = serde_json::from_slice(&std::fs::read(raiz.join("flujo.json")).map_err(|e| e.to_string())?)
            .map_err(|_| "Resumen JSON inválido.".to_string())?;
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
