//! Archivos comprimidos reconstruidos. Verifica cada declaración y conserva claves foráneas ficticias.
use crate::{
    documentos,
    formatos::{self, Entrada},
    texto::Sesion,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};

static CONTRATO: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(include_str!("../../../datos/captura_anonima.json")).unwrap());
type Filas = (Vec<String>, Vec<BTreeMap<String, String>>);
pub struct Miembro {
    nombre: String,
    entrada: Option<Entrada>,
    tipo: String,
}
pub enum Archivo {
    General { miembros: Vec<Miembro>, omitidos: usize },
    Captura { manifest: Value, tablas: BTreeMap<String, Filas>, miembros: Vec<Miembro> },
}
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn en(lista: &str, key: &str) -> bool {
    CONTRATO[lista].as_array().unwrap().iter().any(|v| v == key)
}
fn tabla(raw: &[u8], tipo: &str) -> Result<Filas, String> {
    let text = std::str::from_utf8(raw).map_err(|_| "Tabla de captura no UTF-8.")?.trim_start_matches('\u{feff}');
    let mut reader = csv::ReaderBuilder::new().from_reader(text.as_bytes());
    let campos = reader.headers().map_err(|_| "Tabla sin encabezados válidos.")?.iter().map(str::to_string).collect::<Vec<_>>();
    let esperados = CONTRATO["tablas"][tipo].as_array().ok_or("Tabla de captura desconocida.")?;
    if campos.len() < esperados.len()
        || !campos.iter().zip(esperados).all(|(a, b)| b == a)
        || campos.iter().collect::<BTreeSet<_>>().len() != campos.len()
    {
        return Err("Encabezados de captura incompatibles.".into());
    }
    let mut filas = vec![];
    for r in reader.records() {
        if filas.len() > 200_000 {
            return Err("Tabla de captura excesiva.".into());
        }
        let r = r.map_err(|_| "Fila CSV de captura irregular.")?;
        filas.push(campos.iter().cloned().zip(r.iter().map(str::to_string)).collect());
    }
    Ok((campos, filas))
}
fn estado(s: &str) -> bool {
    [
        "NO_SOLICITADA",
        "NO_CONSULTADA",
        "COMPLETA",
        "VACIA_COMPROBADA",
        "PARCIAL",
        "FALLIDA",
        "CANCELADA",
        "IDENTIDAD_INCIERTA",
        "NO_DISPONIBLE",
    ]
    .contains(&s)
}
fn fecha_valida(s: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(s).is_ok()
        || chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
        || chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").is_ok()
}
fn validar_campos(r: &BTreeMap<String, String>) -> Result<(), String> {
    for (k, v) in r {
        if v.is_empty() {
            continue;
        }
        if en("tiempo", k) && !fecha_valida(v) {
            return Err("Fecha de tabla de captura inválida.".into());
        }
        let valido = match k.as_str() {
            "coverage" => estado(v),
            "identity_status" => ["VERIFICADA", "INCIERTA", "CAUSA"].contains(&v.as_str()),
            "origin" => ["CENTRO", "OTRO", "DESCONOCIDO"].contains(&v.as_str()),
            "origin_verified" | "history_complete" | "institutional_verified" | "recoverable" | "content_read" => {
                ["true", "false"].contains(&v.as_str())
            }
            "scope" => ["INGRESO", "CAUSA"].contains(&v.as_str()),
            "time_precision" => ["DIA", "MINUTO", "SEGUNDO", "DESCONOCIDA"].contains(&v.as_str()),
            "expected_pages" | "saved_pages" | "original_row" => v.bytes().all(|b| b.is_ascii_digit()),
            _ => true,
        };
        if !valido {
            return Err("Control de captura inválido.".into());
        }
    }
    Ok(())
}
fn referencias(tablas: &BTreeMap<String, Filas>) -> Result<(), String> {
    let ids = |tabla: &str, key: &str| -> Result<BTreeSet<String>, String> {
        let mut encontrados = BTreeSet::new();
        if let Some((_, rows)) = tablas.get(tabla) {
            for r in rows {
                let id = r.get(key).ok_or("Falta un identificador de captura.")?;
                if id.is_empty() || !encontrados.insert(id.clone()) {
                    return Err("Identificador de captura vacío o repetido.".into());
                }
            }
        }
        Ok(encontrados)
    };
    let queries = ids("consultas", "query_id")?;
    let records = ids("registros", "record_id")?;
    let docs = ids("documentos", "document_id")?;
    let mut events = ids("bitacoras", "event_id")?;
    for id in ids("historia_tramites", "event_id")? {
        if !events.insert(id) {
            return Err("Evento de captura repetido.".into());
        }
    }
    for (tabla, (_, rows)) in tablas {
        for r in rows {
            for (key, set) in [("query_id", &queries), ("record_id", &records), ("parent_event_id", &events), ("document_id", &docs)] {
                if let Some(id) = r.get(key).filter(|s| !s.is_empty()) {
                    if !set.contains(id) {
                        return Err("Referencia de captura sin destino.".into());
                    }
                }
            }
            if tabla == "documentos" && r.get("parent_event_id").is_none_or(|s| s.is_empty()) {
                return Err("Documento sin evento padre.".into());
            }
        }
    }
    Ok(())
}
pub fn leer(entries: Vec<(String, Vec<u8>)>, nivel: usize, presupuesto: &mut usize) -> Result<Archivo, String> {
    let manifest = entries
        .iter()
        .find(|(n, _)| n == "manifest.json")
        .and_then(|(_, r)| serde_json::from_slice::<Value>(r).ok())
        .filter(|m| m["schema_id"] == "CSMP_CAPTURE");
    if let Some(m) = manifest {
        if !m["schema_version"].as_str().unwrap_or("").starts_with("1.")
            || m["required_features"].as_array().is_some_and(|x| !x.is_empty())
            || !m["table_states"].is_object()
        {
            return Err("Contrato de captura no admitido.".into());
        }
        let estados = m["table_states"].as_object().unwrap();
        if estados.len() != CONTRATO["tablas"].as_object().unwrap().len()
            || estados.iter().any(|(k, v)| CONTRATO["tablas"].get(k).is_none() || !estado(v.as_str().unwrap_or("")))
        {
            return Err("Estados de tablas de captura inválidos.".into());
        }
        if !fecha_valida(m["captured_at"].as_str().unwrap_or("")) {
            return Err("Fecha de captura inválida.".into());
        }
        let declarados = m["files"].as_array().ok_or("Captura sin lista de archivos.")?;
        let mapa = entries.iter().map(|(n, r)| (n.as_str(), r)).collect::<BTreeMap<_, _>>();
        let mut vistos = BTreeSet::new();
        let (mut tablas, mut miembros) = (BTreeMap::new(), vec![]);
        for f in declarados {
            let nombre = f["path"].as_str().ok_or("Captura sin ruta declarada.")?;
            if !vistos.insert(nombre.to_string()) {
                return Err("Declaración de captura repetida.".into());
            }
            let raw = mapa.get(nombre).ok_or("Falta un archivo declarado de captura.")?;
            if f["bytes"].as_u64() != Some(raw.len() as u64) || f["sha256"] != hash(raw) {
                return Err("La huella de captura no coincide; no se anonimiza un paquete alterado.".into());
            }
            match f["type"].as_str() {
                Some("normalized_table") => {
                    let tipo = f["table"].as_str().ok_or("Tabla sin tipo.")?;
                    let t = tabla(raw, tipo)?;
                    for r in &t.1 {
                        validar_campos(r)?;
                    }
                    if f["rows"].as_u64() != Some(t.1.len() as u64) || tablas.insert(tipo.into(), t).is_some() {
                        return Err("Conteos o tablas repetidas en captura.".into());
                    }
                }
                Some("original" | "consolidated_workbook") => miembros.push(Miembro {
                    nombre: nombre.into(),
                    tipo: f["type"].as_str().unwrap().into(),
                    entrada: formatos::cargar_interno(nombre, raw, nivel + 1, presupuesto).ok(),
                }),
                _ => return Err("Archivo de captura con tipo desconocido.".into()),
            }
        }
        if !tablas.contains_key("consultas") || vistos.len() + 1 != entries.len() {
            return Err("Los archivos no coinciden con el manifiesto de captura.".into());
        }
        referencias(&tablas)?;
        return Ok(Archivo::Captura { manifest: m, tablas, miembros });
    }
    let (mut miembros, mut omitidos) = (vec![], 0);
    for (nombre, raw) in entries {
        let base = nombre.rsplit('/').next().unwrap_or("").to_lowercase();
        if matches!(base.as_str(), "manifiesto.json" | "indice.txt") {
            continue;
        }
        match formatos::cargar_interno(&nombre, &raw, nivel + 1, presupuesto) {
            Ok(e) => miembros.push(Miembro { nombre, entrada: Some(e), tipo: String::new() }),
            Err(_) => omitidos += 1,
        }
    }
    if miembros.is_empty() {
        return Err("El ZIP no contiene archivos que puedan anonimizarse.".into());
    }
    Ok(Archivo::General { miembros, omitidos })
}
pub fn semillar(sesion: &mut Sesion, a: &Archivo) -> Result<(), String> {
    let miembros = match a {
        Archivo::General { miembros, .. } => miembros,
        Archivo::Captura { tablas, miembros, .. } => {
            for (_, filas) in tablas.values() {
                for r in filas {
                    for (k, v) in r {
                        sesion.reservar(v);
                        if k == "author" && !v.is_empty() {
                            sesion.persona(v);
                        }
                        if k.ends_with("_json") {
                            if let Ok(j) = serde_json::from_str(v) {
                                formatos::seed_json(sesion, &j, "", 0)?;
                            }
                        }
                    }
                }
            }
            miembros
        }
    };
    for m in miembros {
        if let Some(e) = &m.entrada {
            formatos::recolectar(sesion, e)?;
        }
    }
    Ok(())
}
fn ext(e: &Entrada) -> &str {
    match e {
        Entrada::Json(_) => "json",
        Entrada::JsonLineas(_) => "jsonl",
        Entrada::Texto(_, e) => e,
        Entrada::Csv(_, b) if *b == b'\t' => "tsv",
        Entrada::Csv(_, _) => "csv",
        Entrada::Excel(..) => "xlsx",
        Entrada::Html(_) => "html",
        Entrada::Ics(_) => "ics",
        Entrada::Word(_) => "docx",
        Entrada::Zip(_) => "zip",
    }
}
fn nuevos(miembros: &[Miembro], sesion: &mut Sesion, captura: bool) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let mut bases: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (i, item) in miembros.iter().enumerate() {
        if let Some(e) = &item.entrada {
            let nuevo = if captura && item.tipo == "consolidated_workbook" {
                "consolidado/consolidado.xlsx".into()
            } else {
                format!("{}archivo_{}.{}", if captura { "originales/" } else { "" }, i + 1, ext(e))
            };
            m.insert(item.nombre.clone(), nuevo.clone());
            bases.entry(item.nombre.rsplit('/').next().unwrap_or("").into()).or_default().push(nuevo);
        }
    }
    for (k, v) in bases {
        if v.len() == 1 {
            sesion.rutas.insert(k, v[0].clone());
        }
    }
    sesion.rutas.extend(m.clone());
    m
}
fn campo(sesion: &mut Sesion, key: &str, value: &str, capture_id: &str, textos: &mut Vec<(String, bool)>) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    if key.ends_with("_json") {
        let v = serde_json::from_str(value).map_err(|_| "JSON de captura inválido.")?;
        return serde_json::to_string(&json_control(sesion, &v, "", key == "filters_json", textos)?)
            .map_err(|_| "No se pudo reconstruir JSON de captura.".into());
    }
    let nuevo = match key {
        "capture_id" => capture_id.into(),
        "rit_original" => sesion.pseudo("rit", value),
        "author" => sesion.valor("Nombre", value)?,
        "text" | "response" | "description" | "reference" | "message" | "error" => sesion.valor("observacion", value)?,
        "source_sha256" | "sha256" => sesion.pseudo("hash", value),
        "original_file" | "original_sheet" | "local_file" => sesion.valor(key, value)?,
        _ if key.ends_with("_id") => sesion.token(key, value),
        _ if en("tiempo", key) => sesion.desplazar(value, false),
        _ if en("control", key) || key == "court_name" => sesion.texto(value, false, true)?,
        _ => sesion.valor(key, value)?,
    };
    textos.push((nuevo.clone(), en("control", key)));
    Ok(nuevo)
}
fn json_control(sesion: &mut Sesion, v: &Value, key: &str, filtros: bool, textos: &mut Vec<(String, bool)>) -> Result<Value, String> {
    if !filtros {
        return formatos::json(sesion, v, key, 0, textos);
    }
    Ok(match v {
        Value::Object(m) => {
            let mut out = serde_json::Map::new();
            for (k, v) in m {
                let dummy = Value::Object(serde_json::Map::from_iter([(k.clone(), Value::Null)]));
                let tratado = formatos::json(sesion, &dummy, "", 0, textos)?;
                let mut nuevo = tratado.as_object().unwrap().keys().next().unwrap().clone();
                while out.contains_key(&nuevo) {
                    nuevo.push('_');
                }
                out.insert(nuevo, json_control(sesion, v, k, filtros, textos)?);
            }
            Value::Object(out)
        }
        Value::Array(xs) => Value::Array(xs.iter().map(|v| json_control(sesion, v, key, filtros, textos)).collect::<Result<Vec<_>, _>>()?),
        Value::String(s) if filtros && (en("filtros", key) || en("control", key)) => {
            let t = sesion.texto(s, false, true)?;
            textos.push((t.clone(), true));
            Value::String(t)
        }
        _ => formatos::json(sesion, v, key, 0, textos)?,
    })
}
pub fn transformar(sesion: &mut Sesion, a: &Archivo, textos: &mut Vec<(String, bool)>) -> Result<Vec<u8>, String> {
    let previas = sesion.rutas.clone();
    let resultado = procesar(sesion, a, textos);
    sesion.rutas = previas;
    resultado
}
fn procesar(sesion: &mut Sesion, a: &Archivo, textos: &mut Vec<(String, bool)>) -> Result<Vec<u8>, String> {
    let miembros = match a {
        Archivo::General { miembros, .. } | Archivo::Captura { miembros, .. } => miembros,
    };
    let nombres = nuevos(miembros, sesion, matches!(a, Archivo::Captura { .. }));
    let mut payload: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut incluidos: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut omitidos = match a {
        Archivo::General { omitidos, .. } => *omitidos,
        _ => 0,
    };
    let mut files = vec![];
    for m in miembros {
        let Some(e) = &m.entrada else {
            omitidos += 1;
            continue;
        };
        match formatos::transformar(sesion, e) {
            Ok(s) => {
                let nombre = nombres[&m.nombre].clone();
                let sha = hash(s.datos());
                incluidos.insert(m.nombre.clone(), (nombre.clone(), sha.clone()));
                files.push(json!({"path":nombre,"type":if m.tipo.is_empty(){"archivo"}else{&m.tipo},"bytes":s.datos().len(),"sha256":sha}));
                payload.insert(nombre, s.datos);
            }
            Err(_) => omitidos += 1,
        }
    }
    if omitidos > 0 {
        if sesion.opciones.rechazar_no_admitidos {
            return Err("El ZIP contiene archivos no admitidos o no depurables y se pidió rechazarlo completo.".into());
        }
        sesion
            .avisos
            .insert(format!("{omitidos} archivo(s) no admitidos o no depurables se excluyeron del ZIP; no se copiaron sin tratar."));
    }
    if let Archivo::Captura { manifest, tablas, .. } = a {
        let id = sesion.token("capture_id", manifest["capture_id"].as_str().unwrap_or("captura"));
        let mut consultas = vec![];
        for (tipo, (campos, filas)) in tablas {
            let mut writer = csv::WriterBuilder::new().from_writer(vec![]);
            writer.write_record(campos).map_err(|_| "No se pudo reconstruir CSV de captura.")?;
            for fila in filas {
                let mut nueva = BTreeMap::new();
                for (k, v) in fila {
                    nueva.insert(k.clone(), campo(sesion, k, v, &id, textos)?);
                }
                if let Some(original) = fila.get("original_file").filter(|s| incluidos.contains_key(*s)) {
                    if let Some(Entrada::Excel(libro, _)) = miembros.iter().find(|m| &m.nombre == original).and_then(|m| m.entrada.as_ref())
                    {
                        if let Some(i) = libro.hojas.iter().position(|h| Some(&h.nombre) == fila.get("original_sheet")) {
                            nueva.insert("original_sheet".into(), formatos::nombres_hojas(&libro.hojas)[i].clone());
                        }
                    }
                }
                if tipo == "documentos" {
                    if let Some((n, h)) = incluidos.get(fila.get("local_file").map(String::as_str).unwrap_or("")) {
                        nueva.insert("local_file".into(), n.clone());
                        nueva.insert("sha256".into(), h.clone());
                    } else {
                        nueva.insert("local_file".into(), String::new());
                        nueva.insert("sha256".into(), String::new());
                        nueva.insert("download_state".into(), "NO_DISPONIBLE".into());
                        nueva.insert("content_read".into(), "false".into());
                    }
                }
                if tipo == "consultas" {
                    consultas.push(serde_json::to_value(&nueva).map_err(|_| "Consulta ilegible.")?);
                }
                writer
                    .write_record(campos.iter().map(|k| nueva.get(k).map(String::as_str).unwrap_or("")))
                    .map_err(|_| "No se pudo escribir CSV de captura.")?;
            }
            let raw = writer.into_inner().map_err(|_| "No se pudo cerrar CSV de captura.")?;
            let path = format!("data/{tipo}.csv");
            files
                .push(json!({"path":path,"table":tipo,"type":"normalized_table","rows":filas.len(),"bytes":raw.len(),"sha256":hash(&raw)}));
            payload.insert(path, raw);
        }
        let manifest = json!({"schema_id":"CSMP_CAPTURE","schema_version":if payload.contains_key("consolidado/consolidado.xlsx"){"1.1.0"}else{"1.0.0"},"capture_id":id,"captured_at":sesion.desplazar(manifest["captured_at"].as_str().unwrap_or(""),false),"downloader":{"version":"anonimizador-local","commit":"COPIA_LOCAL"},"contains_sensitive_data":true,"originals_authorized":!incluidos.is_empty(),"queries":consultas,"table_states":manifest["table_states"],"mapping_version":"anonimizado-local","required_features":[],"files":files});
        payload.insert("manifest.json".into(), serde_json::to_vec_pretty(&manifest).map_err(|_| "No se pudo reconstruir el manifiesto.")?);
    } else {
        if payload.is_empty() {
            return Err("El ZIP no conserva archivos que puedan publicarse.".into());
        }
        payload.insert(
            "MANIFIESTO.json".into(),
            serde_json::to_vec_pretty(&json!({"tipo":"ANONIMIZADO_LOCAL","archivos":files,"omitidos":omitidos}))
                .map_err(|_| "No se pudo construir manifiesto.")?,
        );
        let indice = files
            .iter()
            .map(|f| format!("{} {} {}", f["path"].as_str().unwrap_or(""), f["bytes"], f["sha256"].as_str().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n");
        payload.insert("INDICE.txt".into(), indice.into_bytes());
    }
    let raw = documentos::zip(&payload.into_iter().collect::<Vec<_>>())?;
    if matches!(a, Archivo::Captura { .. }) {
        formatos::cargar("comprobacion.zip", &raw)?;
    }
    Ok(raw)
}
