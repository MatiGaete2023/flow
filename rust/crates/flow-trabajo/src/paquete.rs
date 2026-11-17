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
    if datos.len() as u64 > LIMITE { return Err("El paquete supera 80 MB.".into()); }
    let mut zip = zip::ZipArchive::new(Cursor::new(datos)).map_err(|e| format!("El paquete no es un ZIP válido: {e}"))?;
    if zip.len() > ENTRADAS_MAXIMAS {
        return Err("El paquete tiene demasiados archivos.".into());
    }
    let mut rutas = std::collections::HashSet::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let entrada = zip.by_index(i).map_err(|_| "Entrada ZIP ilegible.")?;
        if !ruta_segura(entrada.name().trim_end_matches('/')) || !rutas.insert(entrada.name().to_string()) {
            return Err("El ZIP contiene rutas inseguras o duplicadas.".into());
        }
        total = total.checked_add(entrada.size()).ok_or("Tamaño ZIP inválido.")?;
        if total > LIMITE { return Err("El paquete descomprimido supera 80 MB.".into()); }
    }
    let mut leidos = 0u64;
    let mut leer = |nombre: &str| -> Result<Vec<u8>, String> {
        let mut entrada = zip.by_name(nombre).map_err(|_| format!("Falta {nombre} en el paquete."))?;
        if entrada.size() > LIMITE {
            return Err(format!("{nombre} excede el tamaño admitido."));
        }
        let mut salida = Vec::new();
        entrada.by_ref().take(LIMITE + 1).read_to_end(&mut salida).map_err(|e| format!("No se pudo leer {nombre}: {e}"))?;
        leidos = leidos.checked_add(salida.len() as u64).ok_or("Tamaño descomprimido inválido.")?;
        if leidos > LIMITE { return Err("La lectura descomprimida supera 80 MB.".into()); }
        Ok(salida)
    };
    let manifiesto: Value = serde_json::from_slice(&leer("manifest.json")?).map_err(|e| format!("Manifiesto ilegible: {e}"))?;
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
            if tablas.insert(archivo["table"].as_str().unwrap_or("").to_string(), filas).is_some() {
                return Err("El manifiesto repite una tabla normalizada.".into());
            }
        }
    }
    Ok(Contenido { manifiesto, tablas })
}

/// Comprueba un paquete recién armado con el mismo lector que la importación (huellas, filas y manifiesto).
pub(crate) fn comprobar(datos: &[u8]) -> Result<(), String> {
    abrir(datos).map(|_| ())
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

/// Columna «Trámite» de SITFA (Resolución, Escrito, Actuación); viaja en raw_json. Vacío si no se conoce.
fn tipo_tramite(fila: &FilaCsv) -> String {
    match serde_json::from_str::<Value>(dato(fila, "raw_json")) {
        Ok(Value::Object(mapa)) => match mapa.get("Trámite") {
            Some(Value::String(s)) => s.trim().to_string(),
            Some(Value::Null) | None => String::new(),
            Some(otro) => otro.to_string().trim().to_string(),
        },
        _ => String::new(),
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
            .filter_map(|f| {
                dia(dato(f, "event_at"))
                    .map(|fecha| Tramite::nuevo(dato(f, "description"), fecha, dato(f, "folio")).con_tipo(&tipo_tramite(f)))
            })
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
            // Las tablas del motor solo llevan columnas canónicas; DURACION se guarda aparte para los valores Word.
            columnas_word: if etapa_de(tipo).is_some() {
                columnas_fila.iter().filter(|(k, _)| crate::controles::columna_duracion(k)).cloned().collect()
            } else {
                Default::default()
            },
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
            // Como `capture_input.py`, cada registro se mapea con sus propias columnas: dos alias del mismo campo en
            // registros distintos no son una columna ambigua. El mapeo declarado usa la primera columna vista.
            mapeo = BTreeMap::new();
            for fila in &filas {
                let propias: Vec<String> = contexto.origenes[&fila.id].valores.keys().cloned().collect();
                for (campo, i) in columnas::mapear(&propias, Etapa::Espera).map_err(|e| format!("Registro {}: {e}", fila.id))? {
                    if let Some(posicion) = cabeceras.iter().position(|c| *c == propias[i]) {
                        mapeo.entry(campo).or_insert(posicion);
                    }
                }
            }
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

#[derive(Default)]
struct EvidenciaIngreso {
    lecturas: Vec<Value>,
    eventos: Vec<crate::revision_bitacoras::Evento>,
    conocida: bool,
    incompleta: bool,
    cobertura: Option<(Fecha, Fecha)>,
    contradictoria: bool,
    ids: HashMap<String, String>,
    // Consulta + ID original + instante/precisión disponible + texto.
    esperados: std::collections::HashSet<[String; 4]>,
    normalizados: std::collections::HashSet<[String; 4]>,
    alias_ids: HashMap<(String, String), Option<String>>,
    firmas_normalizadas: HashMap<[String; 4], String>,
}

fn momento_revision(texto: &str, precision: &str) -> Option<String> {
    if texto.len() == 10 {
        if precision == "SEGUNDO" { return None; }
        return texto.parse::<Fecha>().ok().map(|d| format!("DIA:{d}"));
    }
    if let Ok(t) = chrono::DateTime::parse_from_rfc3339(texto) {
        if precision == "DIA" { return None; }
        // ISO 8601 admite zona: Z y +00:00 son equivalentes. Conservamos fecha y
        // hora civiles y el desplazamiento acreditado; nunca suponemos zona para
        // una hora naive ni desplazamos su fecha a UTC para emparejar entradas.
        return Some(format!("SEGUNDO:{}#{}", t.naive_local().format("%Y-%m-%dT%H:%M:%S%.f"), t.offset().local_minus_utc()));
    }
    let t = chrono::NaiveDateTime::parse_from_str(texto, "%Y-%m-%dT%H:%M:%S%.f").ok()?;
    // El exportador convierte exactamente T00:00:00 a fecha civil sin hora.
    if texto.ends_with("T00:00:00") && precision != "SEGUNDO" {
        Some(format!("DIA:{}", t.date()))
    } else if precision == "DIA" {
        None
    } else {
        Some(format!("SEGUNDO:{}", t.format("%Y-%m-%dT%H:%M:%S%.f")))
    }
}

fn alias_revision(d: &mut EvidenciaIngreso, consulta: &str, id: &str) {
    for alias in [id.to_string(), format!("{consulta}-{id}")] {
        d.alias_ids.entry((consulta.to_string(), alias))
            .and_modify(|previo| { if previo.as_deref() != Some(id) { *previo = None; } })
            .or_insert_with(|| Some(id.to_string()));
    }
}

/// Indiza cada consulta y evento una vez. La evidencia no depende del filtro temporal de la UI.
fn evidencias_revision(c: &Contenido) -> HashMap<Identidad, EvidenciaIngreso> {
    use crate::revision_bitacoras::{Autor, Evento};
    let mut salida: HashMap<Identidad, EvidenciaIngreso> = HashMap::new();
    let mut consultas: HashMap<String, Identidad> = HashMap::new();
    let mut ilegible = false;
    for q in c.tablas.get("consultas").into_iter().flatten().filter(|q| dato(q,"screen") == "bitacoras") {
        let Ok(mut filtros) = serde_json::from_str::<Value>(dato(q,"filters_json")) else { ilegible = true; continue; };
        let ident = ["source_system","court_code","cause_id","ingreso_id"].map(|k| filtros["remote_identity"][k].as_str().unwrap_or("").to_string());
        if ident.iter().any(|s| s.trim().is_empty()) { ilegible = true; continue; }
        if let Some(previa) = consultas.insert(dato(q,"query_id").into(),ident.clone()) {
            if previa != ident {
                salida.entry(previa).or_default().contradictoria = true;
                salida.entry(ident.clone()).or_default().contradictoria = true;
            }
        }
        let raw = filtros["revision_bitacoras"].take();
        let d = salida.entry(ident).or_default();
        if raw["version"] != 1 || !raw["entries"].is_array() {
            d.incompleta = true;
        } else {
            if let Some(previa) = d.lecturas.iter().find(|l| l["evidencia"]["version"] == 1) {
                if previa["evidencia"] != raw { d.contradictoria = true; }
            }
            d.conocida = true;
            let a = raw["desde"].as_str().and_then(|s| s.parse::<Fecha>().ok());
            let b = raw["hasta"].as_str().and_then(|s| s.parse::<Fecha>().ok());
            let captura = chrono::DateTime::parse_from_rfc3339(dato(q,"captured_at")).ok().map(|d| d.date_naive());
            let captura_raw = raw["captured_at"].as_str().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|d| d.date_naive());
            if raw["completa"] == true && matches!(dato(q,"coverage"),"COMPLETA"|"VACIA_COMPROBADA") &&
                matches!(raw["coverage"].as_str(),Some("COMPLETA"|"VACIA_COMPROBADA")) &&
                raw["warnings"].as_array().is_some_and(Vec::is_empty) &&
                captura == captura_raw && a.zip(b).is_some_and(|(a,b)| a <= b && captura.is_some_and(|c| b <= c)) {
                d.cobertura = a.zip(b);
            } else { d.incompleta = true; }
            for e in raw["entries"].as_array().into_iter().flatten() {
                let id = e["entry_id"].as_str().unwrap_or("").to_string();
                if !id.is_empty() {
                    let firma = hex(e.to_string().as_bytes());
                    if let Some(previa) = d.ids.insert(id.clone(),firma.clone()) { if previa != firma { d.contradictoria = true; } }
                }
                if !e["texto"].is_string() { d.incompleta = true; }
                let fecha = e["fecha"].as_str().and_then(dia);
                let texto = e["texto"].as_str().unwrap_or("").to_string();
                if fecha.is_some_and(|f| a.is_some_and(|a| f >= a)) {
                    if !texto.trim().is_empty() {
                        let consulta = dato(q,"query_id");
                        if id.is_empty() || consulta.is_empty() {
                            d.contradictoria = true;
                        } else if let Some(momento) = momento_revision(e["fecha"].as_str().unwrap_or(""), "") {
                            alias_revision(d, consulta, &id);
                            d.esperados.insert([consulta.into(), id.clone(), momento, texto.clone()]);
                        } else { d.contradictoria = true; }
                    }
                }
                d.eventos.push(Evento { id, momento:e["fecha"].as_str().unwrap_or("").into(), fecha, texto,
                    autor:match e["origen"].as_str() {
                        Some("centro") => Autor::Centro, Some("tribunal"|"programa") => Autor::Otro, _ => Autor::Desconocido,
                    } });
            }
        }
        d.lecturas.push(serde_json::json!({"query_id":dato(q,"query_id"),"captured_at":dato(q,"captured_at"),"coverage":dato(q,"coverage"),"evidencia":raw}));
    }
    // Cotejo de las dos representaciones. El origen generalizado del CSV legado no decide autoría.
    for e in c.tablas.get("bitacoras").into_iter().flatten() {
        let ident = identidad(e);
        if let Some(consulta) = consultas.get(dato(e,"query_id")) {
            if consulta != &ident {
                salida.entry(consulta.clone()).or_default().contradictoria = true;
                salida.entry(ident.clone()).or_default().contradictoria = true;
            }
        }
        let d = salida.entry(ident).or_default();
        if !verificada(e) { d.contradictoria = true; }
        if !dato(e,"text").trim().is_empty() {
            let consulta = dato(e,"query_id");
            let id = d.alias_ids.get(&(consulta.into(),dato(e,"event_id").into())).cloned().flatten();
            if let (Some(id), Some(momento)) = (id, momento_revision(dato(e,"event_at"), dato(e,"time_precision"))) {
                let clave = [consulta.into(),id.clone(),momento,dato(e,"text").into()];
                let mut campos = e.iter().map(|(k,v)|(k.clone(),v.clone())).collect::<BTreeMap<_,_>>();
                campos.insert("event_id".into(),id);
                let firma = hex(serde_json::to_string(&campos).unwrap_or_default().as_bytes());
                if let Some(previa) = d.firmas_normalizadas.insert(clave.clone(),firma.clone()) {
                    if previa != firma { d.contradictoria = true; }
                }
                d.normalizados.insert(clave);
            } else { d.contradictoria = true; }
        }
    }
    for d in salida.values_mut() {
        if d.conocida && d.esperados != d.normalizados { d.contradictoria = true; }
        if ilegible { d.incompleta = true; }
    }
    salida
}

/// Clasifica con el mismo validador ZIP que Trabajo. La UI conserva estos mismos bytes
/// para aplicar posteriormente el motor, sin releer un archivo que podría haber cambiado.
pub fn revisar_bitacoras(datos: &[u8], desde: Fecha, hasta: Fecha, corte: Fecha) -> Result<crate::revision_informe::Informe, String> {
    use crate::revision_bitacoras::{clasificar, Evidencia};
    use crate::revision_informe::{FilaRevision, Identidad as Id, Informe, Procedencia};
    if desde > hasta || hasta > corte { return Err("Período inválido o posterior al corte.".into()); }
    let c = abrir(datos)?;
    let registros = c.tablas.get("registros").ok_or("El paquete no trae registros.")?;
    if registros.len() > REGISTROS_MAXIMOS { return Err("El paquete excede 20.000 registros.".into()); }
    let vacias = Vec::new();
    let consultas = c.tablas.get("consultas").unwrap_or(&vacias);
    let indice = evidencias_revision(&c);
    let pantallas: HashMap<_, _> = consultas.iter().map(|q| (dato(q,"query_id"), dato(q,"screen"))).collect();
    let solo_cruce = registros.iter().any(|r| pantallas.get(dato(r,"query_id")) != Some(&"calendario_informes"));
    let fuentes = registros.iter().filter(|r| !(solo_cruce && pantallas.get(dato(r,"query_id")) == Some(&"calendario_informes"))).collect::<Vec<_>>();
    let mut filas: Vec<FilaRevision> = Vec::new();
    let mut grupos: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    let mut duplicados = 0;
    let mut ids: HashMap<&str, (Vec<String>, Value, bool)> = HashMap::new();
    for r in &fuentes {
        let mut signature = identidad(r).to_vec(); signature.push(dato(r,"record_type").into());
        let valores = serde_json::from_str::<Value>(dato(r,"normalized_json")).unwrap_or(Value::Null);
        ids.entry(dato(r,"record_id")).and_modify(|(anterior, previos, conflicto)| {
            *conflicto |= *anterior != signature || *previos != valores;
        }).or_insert((signature, valores, false));
    }
    for (numero, r) in fuentes.iter().enumerate() {
        let ident = identidad(r);
        let verificada = verificada(r) && ident.iter().all(|k| !k.trim().is_empty());
        let tipo = dato(r,"record_type").to_string();
        let valores = serde_json::from_str::<Value>(dato(r,"normalized_json")).unwrap_or(Value::Null);
        let mut clave = ident.to_vec(); clave.push(tipo.clone());
        if !verificada { clave.push(format!("fila-{numero}")); }
        let procedencia = Procedencia {
            registro_id:dato(r,"record_id").into(), consulta_id:dato(r,"query_id").into(),
            archivo:dato(r,"original_file").into(), tabla:dato(r,"original_sheet").into(),
            fila:dato(r,"original_row").into(), capturada:dato(r,"captured_at").into(),
            cobertura:dato(r,"coverage").into(), asociacion:dato(r,"identity_status").into(),
        };
        if let Some(&i) = grupos.get(&clave) {
            let previa = &mut filas[i];
            if previa.valores != valores || ids.get(dato(r,"record_id")).is_some_and(|(_,_,c)| *c) {
                previa.evidencia.contradictoria = true;
                previa.revision = clasificar(&previa.evidencia, desde, hasta)?;
            }
            previa.procedencia.push(procedencia);
            duplicados += 1;
            continue;
        }
        let d = indice.get(&ident);
        let lecturas = d.map(|d|d.lecturas.clone()).unwrap_or_default();
        let evidencia = Evidencia {
            admitido:etapa_de(&tipo).is_some(), identidad_verificada:verificada,
            contradictoria:valores.is_null() || dato(r,"record_id").is_empty() ||
                ids.get(dato(r,"record_id")).is_some_and(|(_,_,c)| *c) || d.is_some_and(|d|d.contradictoria),
            cobertura_completa:d.filter(|d|!d.incompleta).and_then(|d|d.cobertura),
            eventos:d.filter(|d|d.conocida).map(|d|d.eventos.clone()),
        };
        let revision = clasificar(&evidencia, desde, hasta)?;
        grupos.insert(clave, filas.len());
        filas.push(FilaRevision { identidad:Id { sistema:ident[0].clone(),tribunal:ident[1].clone(),causa:ident[2].clone(),ingreso:ident[3].clone() },
            tipo, valores, procedencia:vec![procedencia], lecturas, evidencia, revision, propuesta:None, edicion:None, revisada:false });
    }
    if filas.is_empty() { return Err("El paquete no trae registros para revisar.".into()); }
    Ok(Informe { version:1, contrato:c.manifiesto["schema_version"].as_str().unwrap_or("").into(),
        huella_paquete:hex(datos),desde,hasta,corte,filas_fuente:fuentes.len(),duplicados_registros:duplicados,filas,
        motor_aplicado:false,version_reglas:None })
}

/// Sugiere el intervalo explícito de las lecturas del paquete, incluso si son parciales.
/// None en paquetes legados: no se inventa un período a partir de la fecha del archivo.
pub fn periodo_revision(datos: &[u8]) -> Result<Option<(Fecha, Fecha)>, String> {
    let c = abrir(datos)?;
    let mut intervalo: Option<(Fecha,Fecha)> = None;
    for q in c.tablas.get("consultas").into_iter().flatten().filter(|q|dato(q,"screen")=="bitacoras") {
        let Ok(f) = serde_json::from_str::<Value>(dato(q,"filters_json")) else {continue};
        let r = &f["revision_bitacoras"];
        if r["version"] != 1 {continue}
        let desde = r["desde"].as_str().and_then(|s|s.parse::<Fecha>().ok());
        let hasta = r["hasta"].as_str().and_then(|s|s.parse::<Fecha>().ok());
        if let Some((a,b)) = desde.zip(hasta).filter(|(a,b)|a<=b) {
            intervalo = Some(intervalo.map(|(x,y)|(x.min(a),y.max(b))).unwrap_or((a,b)));
        }
    }
    Ok(intervalo)
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

    /// Paquete mínimo ficticio: solo la tabla de registros, con su huella en el manifiesto.
    fn paquete_ficticio(registros: &[(&str, &str, Value)]) -> Vec<u8> {
        use std::io::Write;
        let mut csv = String::from("record_id,record_type,query_id,identity_status,modality_raw,normalized_json\n");
        for (id, tipo, valores) in registros {
            csv.push_str(&format!("{id},{tipo},q1,INCIERTA,,\"{}\"\n", valores.to_string().replace('"', "\"\"")));
        }
        let manifiesto = serde_json::json!({
            "schema_id": "CSMP_CAPTURE", "schema_version": "1.0", "required_features": [], "table_states": {},
            "files": [{"path": "data/registros.csv", "type": "normalized_table", "table": "registros",
                       "rows": registros.len(), "bytes": csv.len(), "sha256": hex(csv.as_bytes())}],
        });
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opciones = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("manifest.json", opciones).unwrap();
        zip.write_all(manifiesto.to_string().as_bytes()).unwrap();
        zip.start_file("data/registros.csv", opciones).unwrap();
        zip.write_all(csv.as_bytes()).unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn egresados_con_alias_distintos_por_registro_no_son_ambiguos() {
        let datos = paquete_ficticio(&[
            ("e1", "EGRESADOS", serde_json::json!({"RIT": "X-1-2099", "RUT": "1-9", "NOMBRE": "Persona Ficticia", "TRIBUNAL": "Laja"})),
            ("e2", "EGRESADOS", serde_json::json!({"RIT": "X-2-2099", "RUT MENOR": "2-7", "NOMBRE": "Otra Ficticia", "TRIBUNAL": "Laja"})),
        ]);
        let corte = "2026-10-20".parse().unwrap();
        let p = importar("ficticio.zip", &datos, corte, &Config::default()).unwrap();
        let t = &p.trabajos[0];
        assert_eq!(t.filas.len(), 2);
        assert_eq!(t.filas[0].registro.texto("rut"), "1-9");
        assert_eq!(t.filas[1].registro.texto("rut"), "2-7");
        assert_eq!(t.cabeceras[t.mapeo["rut"]], "RUT");
        // Dentro de un mismo registro, dos alias del mismo campo siguen siendo un error (como en Python).
        let ambiguo = paquete_ficticio(&[("e3", "EGRESADOS", serde_json::json!({"RUT": "1-9", "RUT MENOR": "2-7", "NOMBRE": "X"}))]);
        assert!(importar("ficticio.zip", &ambiguo, corte, &Config::default()).unwrap_err().contains("Columna ambigua"));
    }

    #[test]
    fn rutas_inseguras() {
        assert!(ruta_segura("data/registros.csv"));
        assert!(!ruta_segura("../x") && !ruta_segura("/x") && !ruta_segura("C:\\x") && !ruta_segura("a//b"));
    }
}


