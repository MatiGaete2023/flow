//! Importa el paquete ZIP `CSMP_CAPTURE_V1` que crea el Descargador: registros, bitácoras, historia de la causa y
//! calendario de informes. El ZIP no se extrae al disco y cada archivo se verifica contra su huella del manifiesto.

use crate::{analizar_filas, Contexto, Trabajo};
use flow_dominio::celda::fecha_de_texto;
use flow_dominio::{Celda, Etapa, Fecha, Historia, Observacion, Registro, Tramite};
use flow_motor::Config;
use flow_tabular::columnas;
use flow_tabular::{Fila, Tabla};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};
use std::path::Path;

const LIMITE: u64 = 80 * 1024 * 1024;
const ENTRADAS_MAXIMAS: usize = 2048;
const REGISTROS_MAXIMOS: usize = 20_000;
/// Calendario de informes por vencer: solo cuenta lo que vence dentro de 60 días desde el corte.
const VENTANA_INFORMES_DIAS: i64 = 60;

type FilaCsv = HashMap<String, String>;
type Identidad = [String; 4];

#[derive(Debug)]
pub struct Paquete {
    /// Un trabajo por etapa presente (Espera, Cumplimiento, Informes).
    pub trabajos: Vec<Trabajo>,
    pub avisos: Vec<String>,
}

fn hex(datos: &[u8]) -> String {
    Sha256::digest(datos).iter().map(|b| format!("{b:02x}")).collect()
}

/// CSV RFC 4180: comas, comillas dobles y saltos de línea dentro de comillas.
pub fn leer_csv(texto: &str) -> Vec<FilaCsv> {
    let texto = texto.strip_prefix('\u{feff}').unwrap_or(texto);
    let mut filas: Vec<Vec<String>> = Vec::new();
    let mut fila: Vec<String> = Vec::new();
    let mut campo = String::new();
    let mut en_comillas = false;
    let mut letras = texto.chars().peekable();
    while let Some(c) = letras.next() {
        if en_comillas {
            match c {
                '"' if letras.peek() == Some(&'"') => {
                    letras.next();
                    campo.push('"');
                }
                '"' => en_comillas = false,
                _ => campo.push(c),
            }
        } else {
            match c {
                '"' => en_comillas = true,
                ',' => fila.push(std::mem::take(&mut campo)),
                '\r' => {}
                '\n' => {
                    fila.push(std::mem::take(&mut campo));
                    filas.push(std::mem::take(&mut fila));
                }
                _ => campo.push(c),
            }
        }
    }
    if !campo.is_empty() || !fila.is_empty() {
        fila.push(campo);
        filas.push(fila);
    }
    let Some(cabeceras) = filas.first().cloned() else { return Vec::new() };
    filas[1..]
        .iter()
        .filter(|f| f.len() > 1 || f.first().is_some_and(|c| !c.is_empty()))
        .map(|f| cabeceras.iter().cloned().zip(f.iter().cloned().chain(std::iter::repeat(String::new()))).collect())
        .collect()
}

fn ruta_segura(nombre: &str) -> bool {
    !nombre.is_empty()
        && !nombre.starts_with('/')
        && !nombre.contains('\\')
        && !nombre.contains(':')
        && nombre.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

struct Contenido {
    manifiesto: Value,
    tablas: HashMap<String, Vec<FilaCsv>>,
}

fn abrir(datos: &[u8]) -> Result<Contenido, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(datos)).map_err(|e| format!("El paquete no es un ZIP válido: {e}"))?;
    if zip.len() > ENTRADAS_MAXIMAS {
        return Err("El paquete tiene demasiados archivos.".into());
    }
    let mut leer = |nombre: &str| -> Result<Vec<u8>, String> {
        let mut entrada = zip.by_name(nombre).map_err(|_| format!("Falta {nombre} en el paquete."))?;
        if entrada.size() > LIMITE {
            return Err(format!("{nombre} excede el tamaño admitido."));
        }
        let mut salida = Vec::new();
        entrada.by_ref().take(LIMITE + 1).read_to_end(&mut salida).map_err(|e| format!("No se pudo leer {nombre}: {e}"))?;
        Ok(salida)
    };
    let manifiesto: Value =
        serde_json::from_slice(&leer("manifest.json")?).map_err(|e| format!("Manifiesto ilegible: {e}"))?;
    if manifiesto["schema_id"].as_str() != Some("CSMP_CAPTURE") {
        return Err("El archivo no es un paquete de captura CSMP.".into());
    }
    if !manifiesto["schema_version"].as_str().unwrap_or("").starts_with("1.") {
        return Err("Versión de paquete no admitida.".into());
    }
    if manifiesto["required_features"].as_array().is_some_and(|f| !f.is_empty()) {
        return Err("El paquete exige funciones que esta versión de Flow no tiene.".into());
    }
    let mut tablas = HashMap::new();
    for archivo in manifiesto["files"].as_array().ok_or("Manifiesto sin lista de archivos.")? {
        let ruta = archivo["path"].as_str().unwrap_or("");
        if !ruta_segura(ruta) {
            return Err("El paquete declara una ruta insegura.".into());
        }
        let bytes = leer(ruta)?;
        if Some(bytes.len() as u64) != archivo["bytes"].as_u64() || Some(hex(&bytes).as_str()) != archivo["sha256"].as_str() {
            return Err(format!("{ruta} no coincide con la huella del manifiesto: el paquete fue alterado o está dañado."));
        }
        if archivo["type"].as_str() == Some("normalized_table") {
            let texto = String::from_utf8(bytes).map_err(|_| format!("{ruta} no es UTF-8."))?;
            let filas = leer_csv(&texto);
            if Some(filas.len() as u64) != archivo["rows"].as_u64() {
                return Err(format!("{ruta} no tiene la cantidad de filas declarada."));
            }
            tablas.insert(archivo["table"].as_str().unwrap_or("").to_string(), filas);
        }
    }
    Ok(Contenido { manifiesto, tablas })
}

fn dato<'a>(fila: &'a FilaCsv, clave: &str) -> &'a str {
    fila.get(clave).map(String::as_str).unwrap_or("")
}

fn identidad(fila: &FilaCsv) -> Identidad {
    ["source_system", "court_code", "cause_id", "ingreso_id"].map(|k| dato(fila, k).to_string())
}

fn verificada(fila: &FilaCsv) -> bool {
    dato(fila, "identity_status") == "VERIFICADA"
}

fn dia(texto: &str) -> Option<Fecha> {
    texto.get(..10).and_then(|d| d.parse().ok())
}

fn celda(valor: &Value) -> Celda {
    match valor {
        Value::Null => Celda::Vacia,
        Value::String(s) if s.is_empty() => Celda::Vacia,
        Value::String(s) => Celda::Texto(s.clone()),
        Value::Number(n) => n.as_f64().map(Celda::Numero).unwrap_or_default(),
        Value::Bool(b) => Celda::Texto(b.to_string()),
        otro => Celda::Texto(otro.to_string()),
    }
}

/// Columnas del registro tal como las entregó RUS (nombre → valor).
fn normalizado(fila: &FilaCsv) -> Vec<(String, Celda)> {
    match serde_json::from_str::<Value>(dato(fila, "normalized_json")) {
        Ok(Value::Object(mapa)) => mapa.iter().map(|(k, v)| (k.clone(), celda(v))).collect(),
        _ => Vec::new(),
    }
}

/// Historia por causa: de varias consultas de la misma causa gana la de mejor cobertura.
fn historias(c: &Contenido) -> HashMap<[String; 3], Historia> {
    let vacio = Vec::new();
    let filas = c.tablas.get("historia_tramites").unwrap_or(&vacio);
    let mut salida: HashMap<[String; 3], (u8, Historia)> = HashMap::new();
    for consulta in c.manifiesto["queries"].as_array().into_iter().flatten() {
        if consulta["screen"].as_str() != Some("historia_tramites") {
            continue;
        }
        let Ok(filtros) = serde_json::from_str::<Value>(consulta["filters_json"].as_str().unwrap_or("")) else { continue };
        let causa = &filtros["historia_causa"];
        if !causa.is_object() {
            continue;
        }
        let clave = ["source_system", "court_code", "cause_id"].map(|k| causa[k].as_str().unwrap_or("").to_string());
        let cobertura = consulta["coverage"].as_str().unwrap_or("");
        let puntaje = match cobertura {
            "COMPLETA" => 3,
            "PARCIAL" => 2,
            "VACIA_COMPROBADA" => 1,
            _ => 0,
        };
        if salida.get(&clave).is_some_and(|(mejor, _)| *mejor >= puntaje) {
            continue;
        }
        let id = consulta["query_id"].as_str().unwrap_or("");
        let tramites = filas
            .iter()
            .filter(|f| dato(f, "query_id") == id)
            .filter_map(|f| dia(dato(f, "event_at")).map(|fecha| Tramite::nuevo(dato(f, "description"), fecha, dato(f, "folio"))))
            .collect();
        let ventana = |k: &str| filtros["ventana"][k].as_str().and_then(dia);
        salida.insert(clave, (puntaje, Historia::nueva(cobertura, ventana("desde"), ventana("hasta"), tramites)));
    }
    salida.into_iter().map(|(k, (_, h))| (k, h)).collect()
}

fn etapa_de(tipo: &str) -> Option<Etapa> {
    match tipo {
        "ESPERA" => Some(Etapa::Espera),
        "CUMPLIMIENTO" => Some(Etapa::Cumplimiento),
        "INFORMES" => Some(Etapa::Informes),
        _ => None,
    }
}

/// Lee, verifica y analiza un paquete. Devuelve un trabajo por etapa.
pub fn importar(nombre: &str, datos: &[u8], corte: Fecha, config: &Config) -> Result<Paquete, String> {
    if datos.len() as u64 > LIMITE {
        return Err("El paquete supera 80 MB.".into());
    }
    let c = abrir(datos)?;
    let huella = hex(datos);
    let vacio = Vec::new();
    let tabla = |n: &str| c.tablas.get(n).unwrap_or(&vacio);
    let pantallas: HashMap<&str, &str> = tabla("consultas").iter().map(|q| (dato(q, "query_id"), dato(q, "screen"))).collect();
    let de_calendario = |r: &FilaCsv| pantallas.get(dato(r, "query_id")) == Some(&"calendario_informes");
    // Con registros de Seguimiento, el calendario de informes solo aporta fechas al cruce: no duplica personas.
    let solo_cruce = tabla("registros").iter().any(|r| !de_calendario(r));
    let registros: Vec<&FilaCsv> = tabla("registros").iter().filter(|r| !(solo_cruce && de_calendario(r))).collect();
    if registros.len() > REGISTROS_MAXIMOS {
        return Err("La captura excede 20.000 registros; divide la exportación en lotes.".into());
    }
    let por_causa = historias(&c);
    let solicitadas = c.manifiesto["table_states"]["bitacoras"] != "NO_SOLICITADA";
    let mut lecturas: HashMap<Identidad, bool> = HashMap::new();
    for q in tabla("consultas").iter().filter(|q| dato(q, "screen") == "bitacoras") {
        if let Ok(filtros) = serde_json::from_str::<Value>(dato(q, "filters_json")) {
            if filtros["remote_identity"].is_object() {
                let ident = ["source_system", "court_code", "cause_id", "ingreso_id"]
                    .map(|k| filtros["remote_identity"][k].as_str().unwrap_or("").to_string());
                let leida = !matches!(dato(q, "coverage"), "FALLIDA" | "NO_CONSULTADA");
                lecturas.entry(ident).and_modify(|l| *l |= leida).or_insert(leida);
            }
        }
    }
    let mut eventos_cronologia: HashMap<Identidad, Vec<Value>> = HashMap::new();
    for e in tabla("bitacoras").iter().filter(|e| verificada(e) && e.get("scope").is_none_or(|s| s == "INGRESO")) {
        eventos_cronologia.entry(identidad(e)).or_default().push(serde_json::to_value(e).map_err(|_| "Evento de bitácora ilegible.")?);
    }
    let mut bitacoras: HashMap<Identidad, Vec<Observacion>> = HashMap::new();
    for evento in tabla("bitacoras") {
        let de_ingreso = evento.get("scope").is_none_or(|s| s == "INGRESO");
        if let (true, true, Some(fecha)) = (verificada(evento), de_ingreso, dia(dato(evento, "event_at"))) {
            bitacoras.entry(identidad(evento)).or_default().push(Observacion { fecha, texto: dato(evento, "text").to_string() });
        }
    }
    let mut calendario: HashMap<Identidad, Vec<Fecha>> = HashMap::new();
    for fila in tabla("calendarios") {
        let tipo = dato(fila, "event_type").trim().to_uppercase();
        if dato(fila, "association") != "VERIFICADA" || !(tipo == "INFORME" || tipo == "INFORMES") {
            continue;
        }
        let columnas_fila = normalizado(fila);
        let nombres: Vec<String> = columnas_fila.iter().map(|(k, _)| k.clone()).collect();
        let vence = columnas::mapear(&nombres, Etapa::Informes)
            .ok()
            .and_then(|m| m.get("vencimiento").copied())
            .and_then(|i| columnas_fila[i].1.fecha());
        if let Some(v) = vence.filter(|v| corte < *v && (*v - corte).num_days() <= VENTANA_INFORMES_DIAS) {
            calendario.entry(identidad(fila)).or_default().push(v);
        }
    }
    let mut proyectados: HashMap<Identidad, Vec<crate::controles::Proyeccion>> = HashMap::new();
    for fila in
        tabla("calendarios").iter().filter(|f| dato(f, "association") == "VERIFICADA" && dato(f, "event_type") == "INFORME_PROYECTADO")
    {
        if let Some(fecha) = dia(dato(fila, "event_at")) {
            proyectados
                .entry(identidad(fila))
                .or_default()
                .push(crate::controles::Proyeccion { fecha, estado: dato(fila, "status").into() });
        }
    }
    for items in proyectados.values_mut() {
        items.sort();
        items.dedup();
    }
    let mut avisos = Vec::new();
    let mut por_etapa: BTreeMap<(u8, String), (Etapa, Vec<Fila>)> = BTreeMap::new();
    let mut contexto = Contexto { proximos: Some(HashMap::new()), ..Contexto::default() };
    for (n, r) in registros.iter().enumerate() {
        let tipo = dato(r, "record_type");
        let etapa = etapa_de(tipo).unwrap_or(Etapa::Espera);
        let id = dato(r, "record_id").to_string();
        let columnas_fila = normalizado(r);
        let nombres: Vec<String> = columnas_fila.iter().map(|(k, _)| k.clone()).collect();
        let pantalla = pantallas.get(dato(r, "query_id")).copied().unwrap_or("");
        let origen = crate::controles::Origen {
            sistema: dato(r, "source_system").into(),
            tribunal: dato(r, "court_code").into(),
            causa: dato(r, "cause_id").into(),
            ingreso: dato(r, "ingreso_id").into(),
            tipo: tipo.into(),
            pantalla: pantalla.into(),
            verificada: verificada(r),
            valores: if etapa_de(tipo).is_none() { columnas_fila.iter().cloned().collect() } else { Default::default() },
        };
        contexto.origenes.insert(id.clone(), origen);

        let mapeo = columnas::mapear(&nombres, etapa).map_err(|e| format!("Registro {id}: {e}"))?;
        let mut registro = Registro::default();
        for (campo, i) in &mapeo {
            registro.poner(campo, columnas_fila[*i].1.clone());
        }
        // Celdas en el orden canónico de la etapa; lo no capturado queda vacío.
        let mut celdas: Vec<Celda> =
            columnas::canonicas(etapa).iter().map(|(campo, _)| registro.celda(campo).cloned().unwrap_or_default()).collect();
        celdas.push(Celda::Texto(dato(r, "modality_raw").to_string()));
        if verificada(r) {
            let ident = identidad(r);
            if ident.iter().all(|k| !k.is_empty()) {
                contexto.cronologias.insert(
                    id.clone(),
                    crate::cronologia::mostrar(
                        eventos_cronologia.get(&ident).map(Vec::as_slice).unwrap_or(&[]),
                        corte,
                        solicitadas,
                        if lecturas.is_empty() { true } else { lecturas.get(&ident).copied().unwrap_or(false) },
                    ),
                );
            }
            if let Some(h) = por_causa.get(&[ident[0].clone(), ident[1].clone(), ident[2].clone()]) {
                contexto.historias.insert(id.clone(), h.clone());
            }
            if let Some(b) = bitacoras.get(&ident) {
                contexto.bitacoras.insert(id.clone(), b.clone());
            }
            let hoja2 = calendario.get(&ident).and_then(|fechas| fechas.iter().all(|f| *f == fechas[0]).then_some(fechas[0]));
            let items = proyectados.get(&ident).map(Vec::as_slice).unwrap_or(&[]);
            contexto.calendarios.insert(id.clone(), items.to_vec());
            if matches!(etapa, Etapa::Cumplimiento | Etapa::Informes) {
                let vencimiento = if etapa == Etapa::Informes { registro.fecha("vencimiento") } else { None };
                contexto.avisos.insert(id.clone(), crate::controles::cotejar(items, corte, vencimiento, hoja2));
            }
            if let Some(d) = hoja2.or_else(|| crate::controles::proxima(items, corte)) {
                contexto.proximos.as_mut().unwrap().insert(id.clone(), d);
            }
        }
        let numero = dato(r, "original_row").parse().unwrap_or(n + 2);
        por_etapa.entry((etapa as u8, tipo.into())).or_insert_with(|| (etapa, Vec::new())).1.push(Fila { id, numero, celdas, registro });
    }
    if let Some(estados) = c.manifiesto["table_states"].as_object() {
        let faltan: Vec<String> = estados
            .iter()
            .filter(|(_, v)| !matches!(v.as_str(), Some("COMPLETA" | "VACIA_COMPROBADA" | "NO_SOLICITADA")))
            .map(|(k, v)| format!("{k}: {}", v.as_str().unwrap_or("?")))
            .collect();
        if !faltan.is_empty() {
            avisos.push(format!("Fuentes faltantes: {}.", faltan.join(", ")));
        }
    }
    let mut trabajos = Vec::new();
    for ((_, tipo), (etapa, mut filas)) in por_etapa {
        let canonicas = columnas::canonicas(etapa);
        // Solo se declaran las columnas que algún registro trae: una columna ausente no es un dato vacío.
        let presentes: Vec<bool> = canonicas.iter().map(|(campo, _)| filas.iter().any(|f| f.registro.tiene(campo))).collect();
        let mut cabeceras: Vec<String> = canonicas.iter().map(|(_, titulo)| titulo.to_string()).collect();
        cabeceras.push("MODALIDAD".into());
        let mut mapeo = canonicas.iter().enumerate().filter(|(i, _)| presentes[*i]).map(|(i, (campo, _))| (campo.to_string(), i)).collect();
        if etapa_de(&tipo).is_none() {
            cabeceras = filas
                .iter()
                .flat_map(|f| contexto.origenes[&f.id].valores.keys().cloned())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            mapeo = columnas::mapear(&cabeceras, Etapa::Espera)?;
            for fila in &mut filas {
                fila.celdas = cabeceras.iter().map(|k| contexto.origenes[&fila.id].valores.get(k).cloned().unwrap_or_default()).collect();
            }
        }
        let tabla =
            Tabla { etapa, hoja: "Paquete".into(), fila_cabecera: 1, cabeceras, mapeo, filas, cruce: Vec::new(), avisos: avisos.clone() };
        trabajos.push(analizar_filas(nombre, &huella, &tabla, corte, config, &contexto));
    }
    crate::controles::paquete(&mut trabajos);
    if trabajos.is_empty() {
        return Err("El paquete no trae registros para mostrar.".into());
    }
    Ok(Paquete { trabajos, avisos })
}

pub fn importar_archivo(ruta: &Path, corte: Fecha, config: &Config) -> Result<Paquete, String> {
    let datos = std::fs::read(ruta).map_err(|e| format!("No se pudo leer {}: {e}", ruta.display()))?;
    let nombre = ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    importar(&nombre, &datos, corte, config)
}

/// Un archivo es un paquete de captura si es un ZIP con `manifest.json`.
pub fn es_paquete(datos: &[u8]) -> bool {
    datos.starts_with(b"PK") && zip::ZipArchive::new(Cursor::new(datos)).is_ok_and(|mut z| z.by_name("manifest.json").is_ok())
}

#[allow(dead_code)]
fn _fecha(texto: &str) -> Option<Fecha> {
    fecha_de_texto(texto)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn csv_con_comillas_y_saltos() {
        let filas = leer_csv("a,b\n1,\"x, \"\"y\"\"\nz\"\n2,\n");
        assert_eq!(filas.len(), 2);
        assert_eq!(filas[0]["b"], "x, \"y\"\nz");
        assert_eq!(filas[1]["b"], "");
    }

    #[test]
    fn rutas_inseguras() {
        assert!(ruta_segura("data/registros.csv"));
        assert!(!ruta_segura("../x") && !ruta_segura("/x") && !ruta_segura("C:\\x") && !ruta_segura("a//b"));
    }
}
