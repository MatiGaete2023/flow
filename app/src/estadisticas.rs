//! Conteos de constancias (`statistics.py`) y métricas locales opcionales (`review_metrics.py`) de Flow Python.
//! Una propuesta o un borrador no equivalen a una gestión: solo cuenta lo que la persona completó (fecha, TT y CC). Las
//! métricas guardan únicamente conteos y marcas de tiempo, nunca datos de casos.

use crate::conciliar::{Cambios, Tipo};
use crate::config;
use crate::preparados::Recibo;
use crate::productos::{res_tipo, nueva_ruta};
use crate::trabajo::Trabajo;
use crate::util::{get_date, get_int, Cell};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Lo que las estadísticas leen de una fila.
#[derive(Clone, Debug, Default)]
pub struct FilaEstadistica {
    pub excluida: bool,
    /// La persona tomó alguna decisión sobre la fila (`review` no vacío).
    pub con_revision: bool,
    pub observacion: String,
    pub fecha: String,
    pub tt: String,
    pub cc: String,
    pub res: String,
    pub res_humana: bool,
    pub tribunal: String,
    pub rit: String,
}

/// `res_is_human`: RES con un valor que no es el que el motor sugirió.
pub fn res_humana(res: &str, acciones: &[&str]) -> bool {
    if res.trim().is_empty() {
        return false;
    }
    let tipo = res_tipo(res).map_or_else(|| res.trim().to_string(), str::to_string);
    let sugerida = acciones.iter().find(|a| matches!(**a, "PC_IE" | "PC_INFO" | "NOMENCL")).map(|a| a.to_string());
    Some(tipo) != sugerida
}

pub fn filas_de(w: &Trabajo) -> Vec<FilaEstadistica> {
    w.registros
        .iter()
        .map(|r| {
            let campo = |k: &str| r.campos.get(k).cloned().unwrap_or_default();
            FilaEstadistica {
                excluida: r.excluido,
                con_revision: r.editada.is_some() || !r.campos.is_empty() || !r.res.is_empty(),
                observacion: r.editada.clone().unwrap_or_default(),
                fecha: campo("FECHA_OBS"),
                tt: campo("TT"),
                cc: campo("CC"),
                res: r.res.clone(),
                res_humana: res_humana(&r.res, &r.acciones),
                tribunal: w.value(r, "tribunal"),
                rit: w.value(r, "rit"),
            }
        })
        .collect()
}

fn entero(texto: &str) -> Option<i64> {
    get_int(&Cell::Text(texto.to_string()))
}

fn constancia(f: &FilaEstadistica) -> Option<(i64, i64)> {
    let (tt, cc) = (entero(&f.tt)?, entero(&f.cc)?);
    (!f.excluida && !f.observacion.is_empty() && get_date(&Cell::Text(f.fecha.clone())).is_some() && tt == 1 && matches!(cc, 0 | 1)).then_some((tt, cc))
}

/// `summarize`: propuestas, constancias, datos incompletos y productos creados.
pub fn resumir(filas: &[FilaEstadistica], recibos: &[Recibo]) -> BTreeMap<&'static str, usize> {
    let mut m = BTreeMap::from([
        ("propuestas", filas.len()),
        ("excluidos", filas.iter().filter(|f| f.excluida).count()),
        ("constancias", 0),
        ("con_carga", 0),
        ("sin_carga", 0),
        ("datos_incompletos", 0),
    ]);
    for f in filas.iter().filter(|f| !f.excluida && f.con_revision) {
        if let Some((tt, cc)) = constancia(f) {
            *m.get_mut("constancias").unwrap() += 1;
            *m.get_mut("con_carga").unwrap() += cc as usize;
            *m.get_mut("sin_carga").unwrap() += (tt - cc) as usize;
        } else if [&f.fecha, &f.tt, &f.cc].iter().any(|v| !v.is_empty()) || f.res_humana {
            *m.get_mut("datos_incompletos").unwrap() += 1;
        }
    }
    let word: Vec<&Recibo> = recibos.iter().filter(|r| r.tipo == "word").collect();
    m.insert("word_generados", word.iter().map(|r| r.ruta.as_str()).collect::<BTreeSet<_>>().len());
    m.insert("proyectos_generados", word.len());
    m.insert("borradores_creados", recibos.iter().filter(|r| r.tipo == "borrador" && r.estado == "creado").count());
    m
}

/// Hoja «Constancias»: las filas completas (fecha, TT = 1 y CC 0 o 1), con TT y CC como números.
pub fn constancias(filas: &[FilaEstadistica]) -> Vec<(String, String, String, i64, i64, i64, String)> {
    filas
        .iter()
        .filter_map(|f| constancia(f).map(|(tt, cc)| (f.tribunal.clone(), f.rit.clone(), f.fecha.clone(), tt, cc, tt - cc, f.res.clone())))
        .collect()
}

/// `export_summary`: un libro nuevo con el resumen y las constancias; no reemplaza un archivo.
pub fn exportar_resumen(filas: &[FilaEstadistica], recibos: &[Recibo], destino: &Path) -> Result<PathBuf, String> {
    if destino.exists() {
        return Err("El archivo ya existe; elige un destino nuevo.".into());
    }
    let err = |e: rust_xlsxwriter::XlsxError| e.to_string();
    let mut libro = rust_xlsxwriter::Workbook::new();
    let cab = rust_xlsxwriter::Format::new().set_bold();
    let hoja = libro.add_worksheet();
    hoja.set_name("Resumen").map_err(err)?;
    hoja.write_string_with_format(0, 0, "Categoría", &cab).map_err(err)?;
    hoja.write_string_with_format(0, 1, "Cantidad", &cab).map_err(err)?;
    let orden = ["propuestas", "excluidos", "constancias", "con_carga", "sin_carga", "datos_incompletos", "word_generados", "proyectos_generados", "borradores_creados"];
    let resumen = resumir(filas, recibos);
    for (i, k) in orden.iter().enumerate() {
        hoja.write_string(i as u32 + 1, 0, k.replace('_', " ")).map_err(err)?;
        hoja.write_number(i as u32 + 1, 1, resumen[k] as f64).map_err(err)?;
    }
    hoja.set_freeze_panes(1, 0).map_err(err)?;
    hoja.set_column_width(0, 25).map_err(err)?;
    hoja.set_column_width(1, 25).map_err(err)?;
    let hoja = libro.add_worksheet();
    hoja.set_name("Constancias").map_err(err)?;
    for (c, t) in ["Tribunal", "RIT", "Fecha", "TT", "CC", "Sin carga", "RES"].iter().enumerate() {
        hoja.write_string_with_format(0, c as u16, *t, &cab).map_err(err)?;
        hoja.set_column_width(c as u16, 25).map_err(err)?;
    }
    for (i, (trib, rit, fecha, tt, cc, sin, res)) in constancias(filas).iter().enumerate() {
        let n = i as u32 + 1;
        hoja.write_string(n, 0, trib).map_err(err)?;
        hoja.write_string(n, 1, rit).map_err(err)?;
        hoja.write_string(n, 2, fecha).map_err(err)?;
        hoja.write_number(n, 3, *tt as f64).map_err(err)?;
        hoja.write_number(n, 4, *cc as f64).map_err(err)?;
        hoja.write_number(n, 5, *sin as f64).map_err(err)?;
        hoja.write_string(n, 6, res).map_err(err)?;
    }
    hoja.set_freeze_panes(1, 0).map_err(err)?;
    if let Some(d) = destino.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    libro.save(destino).map_err(|e| format!("No se pudo guardar {}: {e}", destino.display()))?;
    Ok(destino.to_path_buf())
}

pub fn ruta_resumen(dir: &Path) -> PathBuf {
    nueva_ruta(dir, "Resumen_constancias", "xlsx")
}

// ── métricas locales ─────────────────────────────────────────────────────────

fn ahora() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

fn leer(dir: &Path) -> Value {
    std::fs::read(dir.join("metricas.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .filter(|v| v.get("ciclos").is_some_and(Value::is_array))
        .unwrap_or_else(|| json!({"version": 1, "ciclos": []}))
}

fn escribir(dir: &Path, datos: &Value) {
    // Una métrica que no se puede guardar nunca interrumpe el trabajo.
    let _ = std::fs::create_dir_all(dir);
    if let Ok(bytes) = serde_json::to_vec_pretty(datos) {
        let _ = config::atomic_write(&dir.join("metricas.json"), &bytes);
    }
}

/// `record_import`: un ciclo nuevo con lo que cambió frente a la captura anterior y lo ya revisado.
pub fn registrar_importacion(dir: &Path, activa: bool, filas: &[(String, bool)], cambios: Option<&Cambios>) {
    if !activa {
        return;
    }
    let mut datos = leer(dir);
    let conteo = |t: Tipo| cambios.map_or(0, |c| c.filas.values().filter(|f| f.tipo == t).count());
    let revisadas: Vec<&(String, bool)> = filas.iter().filter(|f| f.1).collect();
    let ciclo = json!({
        "importado": ahora(), "exportado": "", "primer_borrador": "", "ultimo_borrador": "",
        "filas": filas.len(),
        "nuevas": conteo(Tipo::Nuevo), "cambio_fuente": conteo(Tipo::Fuente), "cambio_reglas": conteo(Tipo::Reglas),
        "egresadas": cambios.map_or(0, |c| c.egresados.len()),
        "revisadas": revisadas.len(),
        "revisadas_sin_cambio": revisadas.iter().filter(|(id, _)| cambios.and_then(|c| c.filas.get(id)).is_some_and(|f| f.tipo == Tipo::SinCambio)).count(),
        "ediciones_despues_de_exportar": 0,
    });
    let ciclos = datos["ciclos"].as_array_mut().unwrap();
    ciclos.push(ciclo);
    let exceso = ciclos.len().saturating_sub(60);
    ciclos.drain(..exceso);
    escribir(dir, &datos);
}

fn marcar_ultimo(dir: &Path, activa: bool, campos: &[(&str, bool)]) {
    if !activa {
        return;
    }
    let mut datos = leer(dir);
    let Some(ciclo) = datos["ciclos"].as_array_mut().and_then(|c| c.last_mut()) else { return };
    let marca = ahora();
    for (campo, solo_si_vacio) in campos {
        if !*solo_si_vacio || ciclo[*campo].as_str().is_none_or(str::is_empty) {
            ciclo[*campo] = Value::String(marca.clone());
        }
    }
    escribir(dir, &datos);
}

pub fn registrar_exportacion(dir: &Path, activa: bool) {
    marcar_ultimo(dir, activa, &[("exportado", false)]);
}

pub fn registrar_borrador(dir: &Path, activa: bool) {
    marcar_ultimo(dir, activa, &[("primer_borrador", true), ("ultimo_borrador", false)]);
}

pub fn directorio_metricas() -> PathBuf {
    config::data_dir()
}

/// Los últimos 60 ciclos, sin rutas ni datos de personas.
#[cfg(test)]
pub fn resumen_metricas(dir: &Path) -> Value {
    let d = leer(dir);
    let ciclos: Vec<Value> = d["ciclos"].as_array().cloned().unwrap_or_default();
    let desde = ciclos.len().saturating_sub(60);
    json!({"version": 1, "ciclos": ciclos[desde..]})
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::conciliar::CambioFila;

    fn texto(v: &Value) -> String {
        v.as_str().unwrap_or_default().to_string()
    }

    fn fila(v: &Value) -> FilaEstadistica {
        let review = v["review"].as_object().unwrap();
        let campo = |k: &str| review.get(k).map(texto).unwrap_or_default();
        let acciones: Vec<&str> = v["acciones"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
        FilaEstadistica {
            excluida: v["excluido"].as_bool().unwrap(),
            con_revision: !review.is_empty(),
            observacion: campo("OBSERVACION"),
            fecha: campo("FECHA_OBS"),
            tt: campo("TT"),
            cc: campo("CC"),
            res: campo("RES"),
            res_humana: res_humana(&campo("RES"), &acciones),
            ..Default::default()
        }
    }

    fn estadistica(caso: &Value, raiz: &Path, n: usize) -> Vec<String> {
        let mut filas: Vec<FilaEstadistica> = caso["filas"].as_array().unwrap().iter().map(fila).collect();
        // Mismos nombres que en el oráculo: el tribunal y el RIT salen de la posición de la fila.
        for (i, f) in filas.iter_mut().enumerate() {
            f.tribunal = format!("Tribunal {}", (i + 1) % 2);
            f.rit = format!("X-{}-2099", i + 1);
        }
        let recibos: Vec<Recibo> = caso["recibos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| Recibo {
                tipo: if r["kind"] == "draft" { "borrador".into() } else { texto(&r["kind"]) },
                estado: if r["state"] == "created" { "creado".into() } else { texto(&r["state"]) },
                ruta: texto(&r["path"]),
                ..Default::default()
            })
            .collect();
        let mut d = Vec::new();
        let obtenido = resumir(&filas, &recibos);
        for (k, v) in caso["resumen"].as_object().unwrap() {
            if obtenido.get(k.as_str()).copied() != v.as_u64().map(|x| x as usize) {
                d.push(format!("{k}: Rust {:?} · Python {v}", obtenido.get(k.as_str())));
            }
        }
        if obtenido.len() != caso["resumen"].as_object().unwrap().len() {
            d.push(format!("claves Rust {} · Python {}", obtenido.len(), caso["resumen"].as_object().unwrap().len()));
        }
        let destino = raiz.join(format!("resumen{n}.xlsx"));
        exportar_resumen(&filas, &recibos, &destino).unwrap();
        let hojas = crate::lectura::read_sheets(&std::fs::read(&destino).unwrap()).unwrap();
        let nombres: Vec<&str> = hojas.iter().map(|h| h.name.as_str()).collect();
        let previstas: Vec<String> = caso["hojas"].as_array().unwrap().iter().map(texto).collect();
        if nombres != previstas {
            d.push(format!("hojas Rust {nombres:?} · Python {previstas:?}"));
        }
        let celdas: Vec<Vec<String>> = hojas[1].rows.iter().map(|f| f.iter().map(Cell::display).collect()).collect();
        let esperadas: Vec<Vec<String>> = caso["constancias"].as_array().unwrap().iter().map(|f| f.as_array().unwrap().iter().map(texto).collect()).collect();
        if celdas != esperadas {
            d.push(format!("constancias Rust {celdas:?} · Python {esperadas:?}"));
        }
        let filas_resumen: Vec<(String, usize)> = hojas[0].rows.iter().skip(1).map(|f| (f[0].display(), f[1].display().parse().unwrap())).collect();
        let previstas: Vec<(String, usize)> = caso["resumen_filas"].as_array().unwrap().iter().skip(1).map(|f| (texto(&f[0]), f[1].as_u64().unwrap() as usize)).collect();
        if filas_resumen != previstas {
            d.push(format!("hoja Resumen Rust {filas_resumen:?} · Python {previstas:?}"));
        }
        d
    }

    fn cambios(v: &Value) -> Option<Cambios> {
        let v = v.as_object()?;
        let tipo = |t: &str| match t {
            "NUEVO" => Tipo::Nuevo,
            "CAMBIO_FUENTE" => Tipo::Fuente,
            "CAMBIO_REGLAS" => Tipo::Reglas,
            _ => Tipo::SinCambio,
        };
        Some(Cambios {
            filas: v["filas"].as_object().unwrap().iter().map(|(k, f)| (k.clone(), CambioFila { tipo: tipo(f["tipo"].as_str().unwrap()), campos: vec![], reglas_antes: vec![], reglas_despues: vec![] })).collect(),
            egresados: v["egresados"].as_array().unwrap().iter().map(|e| crate::conciliar::Egresado { id_anterior: texto(&e["id_anterior"]), rit: String::new(), nombre: String::new(), tribunal: String::new(), programa: String::new(), ausencia: crate::conciliar::Ausencia::SinCobertura }).collect(),
        })
    }

    fn normalizar(doc: &Value) -> Value {
        let ciclos: Vec<Value> = doc["ciclos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                let mut c = c.clone();
                for k in ["importado", "exportado", "primer_borrador", "ultimo_borrador"] {
                    c[k] = Value::Bool(c[k].as_str().is_some_and(|s| !s.is_empty()));
                }
                c
            })
            .collect();
        json!(ciclos)
    }

    fn metrica(caso: &Value, raiz: &Path, n: usize) -> Vec<String> {
        let dir = raiz.join(format!("metricas{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        match &caso["previo"] {
            Value::Null => {}
            Value::String(s) => std::fs::write(dir.join("metricas.json"), s).unwrap(),
            otro => std::fs::write(dir.join("metricas.json"), otro.to_string()).unwrap(),
        }
        let activa = caso["activa"].as_bool().unwrap();
        let filas: Vec<(String, bool)> = caso["filas"].as_array().unwrap().iter().map(|f| (texto(&f["id"]), f["editada"].as_bool().unwrap())).collect();
        let cambios = cambios(&caso["cambios"]);
        for paso in caso["pasos"].as_array().unwrap() {
            match paso.as_str().unwrap() {
                "importar" => registrar_importacion(&dir, activa, &filas, cambios.as_ref()),
                "exportar" => registrar_exportacion(&dir, activa),
                _ => registrar_borrador(&dir, activa),
            }
        }
        let existe = dir.join("metricas.json").exists();
        let mut d = Vec::new();
        match (&caso["archivo"], existe) {
            (Value::Null, false) => {}
            (esperado, true) if !esperado.is_null() => {
                let obtenido = normalizar(&leer(&dir));
                if obtenido != *esperado {
                    d.push(format!("archivo Rust {obtenido} · Python {esperado}"));
                }
                if normalizar(&resumen_metricas(&dir)) != *esperado {
                    d.push("el resumen difiere del archivo".into());
                }
            }
            (esperado, e) => d.push(format!("existe Rust {e} · Python archivo {esperado}")),
        }
        d
    }

    /// E-T3: estadísticas y métricas de Rust contra `statistics` y `review_metrics` de Flow Python
    /// (`app/qa/generar_estadisticas.py`).
    #[test]
    fn estadisticas_y_metricas_iguales_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/estadisticas.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let raiz = perfil.join("estadisticas");
        let _ = std::fs::remove_dir_all(&raiz);
        std::fs::create_dir_all(&raiz).unwrap();
        let (mut total, mut fallos) = (0, Vec::new());
        for (i, c) in datos["estadisticas"].as_array().unwrap().iter().enumerate() {
            total += 1;
            let d = estadistica(c, &raiz, i);
            if !d.is_empty() {
                fallos.push(format!("estadística {}:\n  {}", c["nombre"], d.join("\n  ")));
            }
        }
        for (i, c) in datos["metricas"].as_array().unwrap().iter().enumerate() {
            total += 1;
            let d = metrica(c, &raiz, i);
            if !d.is_empty() {
                fallos.push(format!("métrica {}:\n  {}", c["nombre"], d.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("ESTADISTICAS: {}/{total}, fallos {}", total - fallos.len(), fallos.len());
        let _ = std::fs::remove_dir_all(&raiz);
        assert_eq!(total, 14, "el oráculo debe traer 14 casos");
        assert!(fallos.is_empty());
    }

    /// Prueba permanente del oráculo: un conteo esperado alterado debe detectarse.
    #[test]
    fn oraculo_de_estadisticas_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/estadisticas.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let raiz = perfil.join("estadisticas-mutacion");
        std::fs::create_dir_all(&raiz).unwrap();
        let mut caso = datos["estadisticas"][4].clone();
        assert!(estadistica(&caso, &raiz, 0).is_empty());
        caso["resumen"]["constancias"] = Value::from(99);
        assert!(!estadistica(&caso, &raiz, 1).is_empty(), "un conteo esperado alterado no se detecta");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn el_resumen_de_un_trabajo_cuenta_lo_que_la_persona_completo() {
        let mut w = Trabajo::default();
        w.registros = vec![crate::trabajo::Registro::default(), crate::trabajo::Registro::default()];
        w.registros[0].editada = Some("Obs".into());
        for (k, v) in [("FECHA_OBS", "14/11/2026"), ("TT", "1"), ("CC", "1")] {
            w.registros[0].campos.insert(k.into(), v.into());
        }
        w.registros[1].res = "PC_IE".into();
        let r = resumir(&filas_de(&w), &[]);
        assert_eq!((r["propuestas"], r["constancias"], r["con_carga"], r["datos_incompletos"]), (2, 1, 1, 1));
    }
}
