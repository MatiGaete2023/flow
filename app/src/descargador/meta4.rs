//! E-R1 · Meta 4 a partir de los lotes del Descargador de `app/` (y de Flow Python): `resumen.json` con `plan` y
//! `consultas`, `verificacion NNN.json` con `vinculos_ingreso` y `bitacoras.json` (`BITACORAS_LECTURA` v1) con sus HTML
//! verificados por SHA-256. Porta `META4_Bitacoras/scripts/meta4_bitacoras.py` (`_lote`, `_fuentes`, `_leer_bitacoras`,
//! `buscar`, `enriquecer`, `sin_bitacoras`) y entrega el mismo `flow_meta4::Meta4` que consumen sus productos
//! (`flow_meta4::salida`, `plantilla` y `regenerar`). No se conecta a SITFA ni a RUS: solo lee carpetas.
//!
//! Diferencias declaradas con el crate histórico: lee el formato del Descargador de Flow Python (no el nativo de
//! `flow_captura::lotes`), no acepta `bitacoras_meta4.json` y no distingue la ventana guardada de la bitácora
//! (`PERIODO_INCOMPLETO`), igual que Python.

use super::paquete::{self, Clave, Fuente, Lote};
use super::xls::Celda as CeldaXls;
use chrono::{NaiveDate, NaiveDateTime};
use flow_dominio::Celda;
use flow_meta4::tribunales::{categoria, norma, tribunal};
use flow_meta4::{
    expandir_lotes, normalizar_tribunales, periodo, ultima, Fuente as FuenteMeta4, Lectura, Meta4, ObservacionMes, Registro4, ALERTA,
    AMBIGUA, IDENTIDAD_VISIBLE, LEIDA_ANTES, NOMBRE_VISIBLE, NO_COMPLETADA, NO_VERIFICABLE, SIN_BITACORA, SIN_TRIBUNAL,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

const OK: &str = "OK";
const MAX_MANIFIESTO: u64 = 64 * 1024 * 1024;
const IDENTIDAD_REMOTA: [&str; 3] = ["tribunal_codigo", "causa_id", "ingreso_id"];

fn celda(c: &CeldaXls) -> Celda {
    match c {
        CeldaXls::Vacia => Celda::Vacia,
        CeldaXls::Texto(s) if s.is_empty() => Celda::Vacia,
        CeldaXls::Texto(s) => Celda::Texto(s.clone()),
        CeldaXls::Entero(n) => Celda::Numero(*n as f64),
        CeldaXls::Real(n) => Celda::Numero(*n),
        CeldaXls::Fecha(f) => Celda::Fecha(f.date()),
        CeldaXls::Logico(b) => Celda::Texto(if *b { "VERDADERO" } else { "FALSO" }.into()),
    }
}

fn texto(v: &Value, k: &str) -> String {
    paquete::texto_de(&v[k])
}

fn momento(t: &str) -> Option<NaiveDateTime> {
    let t = t.trim().trim_end_matches('Z');
    NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S"))
        .ok()
        .or_else(|| t.get(..10)?.parse::<NaiveDate>().ok()?.and_hms_opt(0, 0, 0))
}

fn tokens(s: &str) -> BTreeSet<String> {
    norma(s).split(' ').filter(|p| !p.is_empty()).map(String::from).collect()
}

/// `nombres_compatibles`: mismo nombre con otro orden, tildes o un nombre o apellido de menos (dos palabras en común).
fn nombres_compatibles(a: &str, b: &str) -> bool {
    let (x, y) = (tokens(a), tokens(b));
    !x.is_empty() && !y.is_empty() && (x == y || ((x.is_subset(&y) || y.is_subset(&x)) && x.intersection(&y).count() >= 2))
}

fn rut_norma(s: &str) -> String {
    norma(s).replace(' ', "")
}

/// `_tribunal`: el tribunal canónico de Meta 4 o, si no se reconoce, el texto normalizado.
fn clave_tribunal(s: &str) -> String {
    tribunal(s).map(String::from).unwrap_or_else(|| norma(s))
}

/// Una bitácora de `bitacoras.json` con su control (`_leer_bitacoras`).
struct Diario {
    tribunal: String,
    tribunal_canonico: Option<&'static str>,
    rit: String,
    rut: String,
    nombre: String,
    programa: String,
    id: Clave,
    estado: String,
    capturada: String,
    control: String,
    entradas: Vec<(NaiveDateTime, String, String, String)>,
}

#[derive(Default)]
struct LecturaBitacoras {
    diarios: Vec<Diario>,
    estados: Vec<String>,
    cubiertas: BTreeSet<(String, String, String)>,
    declara_alcance: bool,
    avisos: Vec<String>,
}

fn dia_captura(c: &str) -> Option<NaiveDate> {
    chrono::DateTime::parse_from_rfc3339(&c.replace(' ', "T")).ok().map(|f| f.with_timezone(&chrono::Local).date_naive())
}

/// `control_identidad`: un RIT visible distinto bloquea; un nombre distinto solo pide revisión (la fecha se usa).
fn control_identidad(r: &Value, leida: &Value) -> Option<&'static str> {
    let avisos = r["advertencias"].as_array().into_iter().flatten().filter_map(Value::as_str).collect::<Vec<_>>().join(" ");
    if avisos.contains("RIT visible") || norma(&texto(leida, "rit")) != norma(&texto(r, "rit")) {
        return Some(IDENTIDAD_VISIBLE);
    }
    if nombres_compatibles(&texto(leida, "persona"), &texto(r, "nombre")) {
        None
    } else {
        Some(NOMBRE_VISIBLE)
    }
}

fn diario(carpeta: &Path, r: &Value, corte: NaiveDate) -> Diario {
    let id: Clave = IDENTIDAD_REMOTA.map(|k| texto(r, k));
    let nombre_tribunal = texto(r, "tribunal_nombre");
    let mut d = Diario {
        tribunal: clave_tribunal(&nombre_tribunal),
        tribunal_canonico: tribunal(&nombre_tribunal),
        rit: texto(r, "rit"),
        rut: texto(r, "rut"),
        nombre: texto(r, "nombre"),
        programa: texto(r, "programa"),
        id: id.clone(),
        estado: r["estado"].as_str().unwrap_or("").to_string(),
        capturada: texto(r, "capturada"),
        control: NO_COMPLETADA.to_string(),
        entradas: Vec::new(),
    };
    if d.estado != "LEIDA" {
        return d;
    }
    let (inicio, fin) = periodo(corte);
    let leido = paquete::leer_registro(carpeta, r, &id).ok().and_then(|leida| {
        let mut entradas = Vec::new();
        for e in leida["entries"].as_array()? {
            let f = momento(e["fecha"].as_str()?)?;
            if inicio <= f.date() && f.date() <= fin {
                entradas.push((f, texto(e, "etapa"), texto(e, "tipo"), texto(e, "texto")));
            }
        }
        Some((leida, entradas))
    });
    d.control = match leido {
        None => NO_VERIFICABLE.to_string(),
        Some((leida, entradas)) => {
            d.entradas = entradas;
            match (control_identidad(r, &leida), dia_captura(&d.capturada)) {
                (Some(c), _) => c.to_string(),
                (None, Some(leida)) if leida < corte => LEIDA_ANTES.to_string(),
                (None, None) => NO_VERIFICABLE.to_string(),
                _ => OK.to_string(),
            }
        }
    };
    d
}

/// `buscar_archivos(roots, 'bitacoras.json')`: sin carpetas de resultados de Meta 4 ni repeticiones.
fn buscar_manifiestos(raices: &[PathBuf]) -> Vec<PathBuf> {
    fn recorrer(c: &Path, profundidad: usize, salida: &mut Vec<PathBuf>) {
        let p = c.join("bitacoras.json");
        if p.is_file() && !salida.contains(&p) {
            salida.push(p);
        }
        if profundidad == 0 {
            return;
        }
        let mut hijas: Vec<PathBuf> = std::fs::read_dir(c)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                !(["Resultado META4", "Salida META4", "META4 regenerado"].iter().any(|p| n.starts_with(p)) || n.contains(".en-curso-"))
            })
            .map(|e| e.path())
            .collect();
        hijas.sort();
        for h in hijas {
            recorrer(&h, profundidad - 1, salida);
        }
    }
    let mut salida = Vec::new();
    for r in raices {
        recorrer(r, 8, &mut salida);
    }
    salida
}

fn consulta_de(q: &Value) -> (String, String, String) {
    (texto(q, "tribunal"), texto(q, "tab"), texto(q, "modality"))
}

fn leer_bitacoras(raices: &[PathBuf], corte: NaiveDate) -> Result<LecturaBitacoras, String> {
    // Rutas canónicas, como `diary_paths`: `copia_local` compara la carpeta con la de la copia ya resuelta.
    let manifiestos = buscar_manifiestos(&paquete::rutas_bitacoras(raices));
    if manifiestos.is_empty() {
        return Err("Faltan bitácoras. Selecciona la carpeta de bitácoras del Descargador (bitacoras.json y sus HTML).".into());
    }
    let mut lectura = LecturaBitacoras::default();
    for ruta in manifiestos {
        if std::fs::metadata(&ruta).map_err(|e| e.to_string())?.len() > MAX_MANIFIESTO {
            return Err("Un lote de bitácoras excede 64 MB; revisa la carpeta elegida.".into());
        }
        let datos: Value = std::fs::read(&ruta)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .filter(Value::is_object)
            .ok_or("El control de bitácoras no es un JSON válido.")?;
        if datos["tipo"] != "BITACORAS_LECTURA" || datos["version"] != 1 {
            return Err("Formato de bitácoras no reconocido.".into());
        }
        let carpeta = ruta.parent().ok_or("Control de bitácoras sin carpeta.")?;
        lectura.estados.push(datos["estado"].as_str().filter(|s| !s.is_empty()).unwrap_or("SIN ESTADO").to_string());
        if let Some(consultas) = datos["consultas"].as_array() {
            lectura.declara_alcance = true;
            lectura.cubiertas.extend(
                consultas.iter().filter(|q| matches!(q["estado"].as_str(), Some("ENUMERADA" | "SIN_REGISTROS"))).map(consulta_de),
            );
        }
        let seleccion = &datos["seleccion"];
        let con_fechas = seleccion["dates"].as_object().is_some_and(|o| !o.is_empty()) || seleccion["dates"].as_str().is_some_and(|s| !s.is_empty());
        if con_fechas || seleccion["keep_filters"] == true {
            lectura.avisos.push(format!(
                "El lote de bitácoras {} usó filtros de fecha o del formulario; puede no incluir todos los ingresos.",
                carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            ));
        }
        // Un lote retomado conserva lecturas previas aún no reenumeradas: también valen.
        for r in datos["registros"].as_array().into_iter().flatten().chain(datos["anteriores_no_reenumeradas"].as_array().into_iter().flatten()) {
            lectura.diarios.push(diario(carpeta, r, corte));
        }
    }
    Ok(lectura)
}

/// `buscar`: la bitácora de una fila (identidad visible y remota) y su control; `None` con el motivo si no hay.
fn buscar<'a>(
    indice: &'a HashMap<(String, String), Vec<&'a Diario>>,
    tribunal_fila: &str,
    rit: &str,
    rut: &str,
    nombre: &str,
    programa: &str,
    remota: Option<&Clave>,
) -> Result<&'a Diario, String> {
    let (rut, nombre, programa) = (rut_norma(rut), norma(nombre), norma(programa));
    let candidatos: Vec<&Diario> = indice
        .get(&(clave_tribunal(tribunal_fila), norma(rit)))
        .into_iter()
        .flatten()
        .copied()
        .filter(|d| if rut.is_empty() { !nombre.is_empty() && norma(&d.nombre) == nombre } else { rut_norma(&d.rut) == rut })
        .filter(|d| !programa.is_empty() && norma(&d.programa) == programa)
        .filter(|d| remota.is_none_or(|r| &d.id == r))
        .collect();
    let identidades: BTreeSet<&Clave> = candidatos.iter().map(|d| &d.id).collect();
    if identidades.len() != 1 || identidades.iter().any(|c| c.iter().any(String::is_empty)) {
        return Err(if candidatos.is_empty() { SIN_BITACORA } else { AMBIGUA }.to_string());
    }
    // Un archivo alterado o una identidad visible distinta siempre detienen la asociación.
    if let Some(d) = candidatos.iter().find(|d| d.control == NO_VERIFICABLE || d.control == IDENTIDAD_VISIBLE) {
        return Err(d.control.clone());
    }
    // La misma identidad puede venir de varios lotes: una lectura fallida no anula la verificada de otro.
    let leidas: Vec<&&Diario> = candidatos.iter().filter(|d| [OK, LEIDA_ANTES, NOMBRE_VISIBLE].contains(&d.control.as_str())).collect();
    let Some(nueva) = leidas.into_iter().max_by(|a, b| a.capturada.cmp(&b.capturada)) else {
        return Err(candidatos[0].control.clone());
    };
    if nueva.control == LEIDA_ANTES {
        return Err(LEIDA_ANTES.to_string());
    }
    Ok(nueva)
}

fn etiqueta(carpeta: &Path) -> String {
    carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn modalidad(codigo: &str) -> Option<&'static str> {
    match codigo {
        "1" => Some("Residencia"),
        "3" => Some("FAE"),
        _ => None,
    }
}

fn seleccion_de(q: &Value, m: &Value) -> Value {
    if q.get("seleccion").is_some_and(|s| !s.is_null()) { q["seleccion"].clone() } else { m["seleccion"].clone() }
}

/// Consulta de Seguimiento de Espera o Cumplimiento de FAE o Residencia.
fn es_de_meta4(seleccion: &Value) -> bool {
    seleccion["screen"].as_str().is_none_or(|s| s == "seguimiento")
        && matches!(seleccion["tab"].as_str(), Some("Espera" | "Cumplimiento"))
        && modalidad(seleccion["modality"].as_str().unwrap_or("")).is_some()
}

/// Fecha del último día con lecturas en los lotes de bitácoras (`fecha_lectura`): propone el corte.
pub fn fecha_lectura(carpetas: &[PathBuf]) -> Option<NaiveDate> {
    buscar_manifiestos(carpetas)
        .iter()
        .filter_map(|p| serde_json::from_slice::<Value>(&std::fs::read(p).ok()?).ok())
        .flat_map(|d| {
            let mut v = d["registros"].as_array().cloned().unwrap_or_default();
            v.extend(d["anteriores_no_reenumeradas"].as_array().cloned().unwrap_or_default());
            v
        })
        .filter(|r| r["estado"] == "LEIDA")
        .filter_map(|r| dia_captura(r["capturada"].as_str()?))
        .max()
}

/// Tribunales de Meta 4 consultados en los lotes de registros, en el orden oficial (`tribunales_disponibles`).
pub fn tribunales_disponibles(carpetas: &[PathBuf]) -> Result<Vec<&'static str>, String> {
    let mut presentes = BTreeSet::new();
    for c in expandir_lotes(carpetas)? {
        let resumen = paquete::leer_json(&c.join("resumen.json"))?;
        for q in resumen["consultas"].as_array().into_iter().flatten() {
            if let Some(t) = tribunal(&texto(&q["seleccion"], "tribunal_nombre")) {
                presentes.insert(t);
            }
        }
    }
    Ok(flow_meta4::tribunales::TRIBUNALES.into_iter().filter(|t| presentes.contains(t)).collect())
}

struct ConsultaMeta4 {
    etiqueta: String,
    consulta: Value,
    seleccion: Value,
    lista: &'static str,
    categoria: &'static str,
    tribunal: Option<&'static str>,
    evidencia: Option<String>,
}

/// Lee los lotes de registros y las bitácoras, y calcula Meta 4. No escribe nada.
///
/// `cargar_lote` (el `load_lot` del Descargador) rechaza lo mismo que `_lote` de Meta 4, con otras palabras: se usan las
/// de Meta 4, que es lo que ve el usuario en Python.
fn mensaje_de_lote(error: &str) -> String {
    match error {
        "Un archivo cambió después de la descarga. No se exportó el lote." => "Un Excel del descargador cambió después de su verificación",
        "Faltan páginas en una consulta validada." => "Faltan páginas del descargador",
        "El estado de una consulta no coincide con el resumen." => "Una consulta del descargador no está validada",
        otro => otro,
    }
    .to_string()
}

/// `bitacoras` vacío busca los `bitacoras.json` dentro de las carpetas de registros.
pub fn calcular(
    carpetas: &[PathBuf],
    bitacoras: &[PathBuf],
    corte: NaiveDate,
    emision: NaiveDate,
    elegidos: &[String],
) -> Result<Meta4, String> {
    let tribunales_elegidos = normalizar_tribunales(elegidos)?;
    if carpetas.is_empty() {
        return Err("Agrega al menos un lote de registros.".into());
    }
    let raices_bitacoras: Vec<PathBuf> = if bitacoras.is_empty() { carpetas.to_vec() } else { bitacoras.to_vec() };
    let lectura = leer_bitacoras(&raices_bitacoras, corte)?;

    let mut m = Meta4 {
        corte,
        emision,
        tribunales: tribunales_elegidos,
        registros: Vec::new(),
        observaciones: Vec::new(),
        lecturas: Vec::new(),
        sin_bitacoras: Vec::new(),
        filtros: Vec::new(),
        lotes: Vec::new(),
        avisos: lectura.avisos.clone(),
        fuentes: Vec::new(),
        hojas_previas: Vec::new(),
    };

    // -- lotes de registros (`_lote`)
    let mut consultas: Vec<ConsultaMeta4> = Vec::new();
    let mut vistas: HashMap<(String, String, String), String> = HashMap::new();
    let mut lotes: Vec<(String, Lote)> = Vec::new();
    for carpeta in expandir_lotes(carpetas)? {
        let nombre = etiqueta(&carpeta);
        let resumen = paquete::leer_json(&carpeta.join("resumen.json"))?;
        if resumen["estado"] != "VALIDADA" {
            return Err(format!("La descarga de registros está incompleta ({nombre}). Completa el lote antes de generar Meta 4."));
        }
        let lote = paquete::cargar_lote(&carpeta).map_err(|e| format!("{} ({nombre}){}", mensaje_de_lote(&e), if mensaje_de_lote(&e) == e { "" } else { "." }))?;
        m.lotes.push(nombre.clone());
        if let Some(rango) = resumen["filtro_fecha"].as_object().filter(|r| !r.is_empty()) {
            let dato = |k: &str| rango.get(k).and_then(Value::as_str).unwrap_or("?").to_string();
            m.filtros.push(format!(
                "Filtro de fechas {} a {}: el lote incluye solo los ingresos de ese rango. [{nombre}]",
                dato("desde"),
                dato("hasta")
            ));
        }
        if resumen["otros_filtros"].as_str().is_some_and(|f| f != "todos los registros") {
            m.filtros.push(format!(
                "Filtros del formulario de SITFA conservados: el lote puede no incluir todos los registros. [{nombre}]"
            ));
        }
        for (q, manifiesto) in &lote.consultas {
            let seleccion = seleccion_de(q, manifiesto);
            if !es_de_meta4(&seleccion) {
                continue;
            }
            let tab = texto(&seleccion, "tab");
            let nombre_consulta = {
                let c = texto(q, "consulta");
                if c.is_empty() { "Cumplimiento".to_string() } else { c }
            };
            if tab == "Cumplimiento" && !matches!(seleccion["report"].as_str(), None | Some("") | Some("0")) && !seleccion["report"].is_null() {
                m.filtros.push(format!("Filtro de vencimiento en {nombre_consulta}: incluye solo esas medidas. [{nombre}]"));
            }
            let clave = (
                {
                    let c = texto(&seleccion, "tribunal");
                    if c.is_empty() { tribunal(&texto(&seleccion, "tribunal_nombre")).unwrap_or("").to_string() } else { c }
                },
                tab.clone(),
                texto(&seleccion, "modality"),
            );
            if !clave.0.is_empty() {
                if let Some(previo) = vistas.get(&clave).filter(|p| **p != nombre) {
                    return Err(format!(
                        "La consulta {nombre_consulta} está en dos lotes ({previo} y {nombre}). Quita uno de los dos para no duplicar registros."
                    ));
                }
                vistas.insert(clave, nombre.clone());
            }
            let evidencia = manifiesto["archivos"]
                .as_array()
                .is_none_or(Vec::is_empty)
                .then(|| manifiesto["evidencia"]["archivo"].as_str().map(String::from))
                .flatten();
            consultas.push(ConsultaMeta4 {
                etiqueta: nombre.clone(),
                consulta: q.clone(),
                lista: if tab == "Espera" { "Espera" } else { "Cumplimiento" },
                categoria: modalidad(seleccion["modality"].as_str().unwrap_or("")).unwrap(),
                tribunal: tribunal(&texto(&seleccion, "tribunal_nombre")),
                seleccion,
                evidencia,
            });
        }
        lotes.push((nombre, lote));
    }

    // -- filas de las páginas verificadas (`_fuentes`)
    let indice: HashMap<(String, String), Vec<&Diario>> = lectura.diarios.iter().fold(HashMap::new(), |mut i, d| {
        i.entry((d.tribunal.clone(), norma(&d.rit))).or_default().push(d);
        i
    });
    let seleccionado = |t: Option<&'static str>, m: &Meta4| m.tribunales.as_ref().is_none_or(|e| t.is_some_and(|t| e.contains(&t)));
    let mut origen_de_consulta: BTreeMap<(String, String, String), BTreeSet<String>> = BTreeMap::new();
    for (nombre, lote) in &lotes {
        let fuentes: Vec<Fuente> = paquete::tablas_lote(lote)?;
        for f in fuentes.iter().filter(|f| es_de_meta4(&f.seleccion)) {
            let c = consultas
                .iter()
                .find(|c| c.etiqueta == *nombre && texto(&c.consulta, "verificacion") == f.verificacion)
                .ok_or("Una fuente del lote no corresponde a una consulta de Meta 4.")?;
            origen_de_consulta
                .entry((texto(&f.seleccion, "tribunal_nombre"), c.lista.to_string(), c.categoria.to_string()))
                .or_default()
                .insert(nombre.clone());
            let cols = paquete::mapeo(&f.cabecera)?;
            let (cabecera, _) = f.original(f.filas.first().map(|(n, _)| *n).unwrap_or(0)).unzip();
            let cabecera: Vec<String> = cabecera.map(<[String]>::to_vec).unwrap_or_default();
            let archivo = texto(&f.item, "archivo");
            let col = |nombre: &str| f.cabecera.iter().position(|h| h == nombre);
            let (c_causa, c_ingreso, c_trib) = (col("SITFA_CAUSA_RUS"), col("SITFA_INGRESO_RUS"), col("SITFA_TRIBUNAL_CODIGO"));
            let mut registros_fuente = 0usize;
            for (numero, valores) in &f.filas {
                let Some((cab, tipada)) = f.original(*numero) else { continue };
                let cab: Vec<&String> = cab.iter().collect();
                let mut columnas: Vec<(String, Celda)> = cab
                    .iter()
                    .zip(tipada.iter())
                    .filter(|(t, _)| !t.trim().is_empty())
                    .map(|(t, c)| ((*t).clone(), celda(c)))
                    .collect();
                let valor = |k: &str| cols.get(k).map(|&i| valores[i].clone()).unwrap_or_default();
                let tribunal_texto = valor("tribunal");
                if !cabecera.iter().any(|h| norma(h) == "TRIBUNAL") && !tribunal_texto.is_empty() {
                    columnas.push(("TRIBUNAL".into(), Celda::Texto(tribunal_texto.clone())));
                }
                // Identidad visible original (antes de quitar paréntesis del nombre) y remota del vínculo.
                let originales = |k: &str| cols.get(k).and_then(|&i| tipada.get(i)).map(|c| c.texto_python()).unwrap_or_default();
                let (rit, rut, nombre_fila, programa) = (originales("rit"), originales("rut"), originales("nombre"), originales("programa"));
                let remota: Option<Clave> = match (c_trib, c_causa, c_ingreso) {
                    (Some(t), Some(c), Some(i)) if !valores[c].is_empty() && !valores[i].is_empty() => {
                        Some([valores[t].clone(), valores[c].clone(), valores[i].clone()])
                    }
                    _ => None,
                };
                let (fecha_obs, etapa_ultima, control) = match buscar(&indice, &tribunal_texto, &rit, &rut, &nombre_fila, &programa, remota.as_ref())
                {
                    Err(motivo) => (None, String::new(), motivo),
                    Ok(d) => {
                        let etapas: Vec<(NaiveDateTime, String)> = d.entradas.iter().map(|e| (e.0, e.1.clone())).collect();
                        let (dia, etapa, resultado) = ultima(&etapas, corte);
                        let control = if d.control == OK { resultado.to_string() } else { format!("{resultado} · {}", d.control) };
                        (dia, etapa.to_string(), control)
                    }
                };
                let alerta = if c.lista == "Cumplimiento" && etapa_ultima == "Espera" { ALERTA.to_string() } else { String::new() };
                m.registros.push(Registro4 {
                    lista: c.lista,
                    categoria: categoria(&programa),
                    tribunal: tribunal(&tribunal_texto),
                    columnas,
                    archivo: archivo.clone(),
                    lote: nombre.clone(),
                    fecha_obs,
                    etapa_ultima,
                    control,
                    alerta,
                });
                registros_fuente += 1;
            }
            m.fuentes.push(FuenteMeta4 {
                archivo,
                lote: nombre.clone(),
                tipo: "EXCEL",
                tribunal: c.tribunal,
                categoria: c.categoria,
                lista: c.lista,
                registros: registros_fuente,
                resultado: "PROCESADO",
                seleccionado: seleccionado(c.tribunal, &m),
            });
        }
    }
    // Una consulta sin registros solo es evidencia con su PDF verificado (`inspect_zero_record_file`).
    let mut sin_evidencia: Vec<String> = Vec::new();
    let mut vacias = 0usize;
    for c in consultas.iter().filter(|c| c.evidencia.is_some() || texto(&c.consulta, "estado") == "SIN_RESULTADOS") {
        match &c.evidencia {
            Some(pdf) => {
                vacias += 1;
                m.fuentes.push(FuenteMeta4 {
                    archivo: pdf.clone(),
                    lote: c.etiqueta.clone(),
                    tipo: "PDF",
                    tribunal: c.tribunal,
                    categoria: c.categoria,
                    lista: c.lista,
                    registros: 0,
                    resultado: if c.tribunal.is_some() { "SIN REGISTROS" } else { "NO CLASIFICADO" },
                    seleccionado: c.tribunal.is_some() && seleccionado(c.tribunal, &m),
                });
            }
            None => sin_evidencia.push(format!("{} · {} · {}", c.etiqueta, c.lista, c.categoria)),
        }
    }
    for (clave, lotes_de) in &origen_de_consulta {
        if lotes_de.len() > 1 {
            return Err(format!(
                "{} · {} · {} aparece en varios lotes ({}). Quita uno de los lotes para no duplicar registros.",
                clave.0,
                clave.1,
                clave.2,
                lotes_de.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if m.registros.is_empty() && vacias == 0 {
        return Err("Los lotes no traen registros de Espera o Cumplimiento de FAE o Residencia.".into());
    }

    // -- bitácoras: lecturas y observaciones del mes
    let (inicio, fin) = periodo(corte);
    for d in &lectura.diarios {
        let tribunal_diario = d.tribunal_canonico.unwrap_or(SIN_TRIBUNAL).to_string();
        m.lecturas.push(Lectura {
            tribunal: tribunal_diario.clone(),
            rit: d.rit.clone(),
            rut: d.rut.clone(),
            nombre: d.nombre.clone(),
            programa: d.programa.clone(),
            causa: d.id[1].clone(),
            ingreso: d.id[2].clone(),
            lectura: d.estado.clone(),
            control: d.control.clone(),
        });
        for (fecha, etapa, tipo, texto_obs) in d.entradas.iter().filter(|e| inicio <= e.0.date() && e.0.date() <= fin) {
            m.observaciones.push(ObservacionMes {
                tribunal: tribunal_diario.clone(),
                rit: d.rit.clone(),
                rut: d.rut.clone(),
                nombre: d.nombre.clone(),
                programa: d.programa.clone(),
                causa: d.id[1].clone(),
                ingreso: d.id[2].clone(),
                fecha: *fecha,
                etapa: etapa.clone(),
                tipo: tipo.clone(),
                texto: texto_obs.clone(),
            });
        }
    }

    // -- consultas con registros que ningún lote de bitácoras enumeró (`sin_bitacoras`)
    if lectura.declara_alcance {
        for c in &consultas {
            if texto(&c.consulta, "estado") == "VALIDADA" && !lectura.cubiertas.contains(&consulta_de(&c.seleccion)) {
                let etiqueta = ["tribunal_nombre", "tab", "modalidad_nombre"].map(|k| texto(&c.seleccion, k)).join(" / ");
                m.sin_bitacoras.push((etiqueta, c.tribunal.unwrap_or(SIN_TRIBUNAL).to_string()));
            }
        }
    }

    // -- filtro de tribunales y avisos
    if let Some(elegidos) = &m.tribunales {
        let con_candidatos = m.registros.iter().filter_map(|r| r.tribunal).chain(consultas.iter().filter(|c| c.evidencia.is_some()).filter_map(|c| c.tribunal));
        if !con_candidatos.into_iter().any(|t| elegidos.contains(&t)) {
            return Err("El lote no contiene registros de los tribunales elegidos.".into());
        }
        m.registros.retain(|r| r.tribunal.is_some_and(|t| elegidos.contains(&t)));
        m.observaciones.retain(|o| elegidos.iter().any(|t| *t == o.tribunal));
        m.lecturas.retain(|l| elegidos.iter().any(|t| *t == l.tribunal));
        m.sin_bitacoras.retain(|(_, t)| elegidos.iter().any(|e| e == t));
    }
    if !sin_evidencia.is_empty() {
        m.avisos.push(format!(
            "{} consultas sin registros no traen su PDF de evidencia ({}): no cuentan como SIN REGISTROS y en la matriz de cobertura \
             su combinación queda FALTANTE. Descarga el lote con evidencia para acreditarlas.",
            sin_evidencia.len(),
            sin_evidencia.join("; ")
        ));
    }
    let sin_atribuir = consultas.iter().filter(|c| c.evidencia.is_some() && c.tribunal.is_none()).count();
    if sin_atribuir > 0 {
        m.avisos.push(format!(
            "{sin_atribuir} consultas sin registros no se pudieron atribuir a un tribunal; quedan como NO CLASIFICADO en el inventario."
        ));
    }
    let fuera = m.registros.iter().filter(|r| r.categoria.is_none() || r.tribunal.is_none()).count();
    if fuera > 0 {
        m.avisos.push(format!("{fuera} registros sin tribunal o modalidad reconocidos quedan fuera de META 4 (están en Registros)."));
    }
    if !m.sin_bitacoras.is_empty() {
        m.avisos.push(format!("{} consultas con registros no tienen bitácoras leídas.", m.sin_bitacoras.len()));
    }
    let no_leidas = lectura.diarios.iter().filter(|d| d.control == NO_COMPLETADA).count();
    if no_leidas > 0 {
        m.avisos.push(format!(
            "{no_leidas} bitácoras no se pudieron leer; sus registros quedan como «{NO_COMPLETADA}». Reintenta las lecturas fallidas en el Descargador."
        ));
    }
    if !m.filtros.is_empty() {
        m.avisos.push(format!("Lotes con filtros: META 4 contiene solo lo que entregó cada filtro ({}).", m.filtros.join("; ")));
    }
    let antes = m.registros.iter().filter(|r| r.control.contains(LEIDA_ANTES)).count();
    if antes > 0 {
        m.avisos.push(format!("{antes} bitácoras se leyeron antes de la fecha de corte: usa como corte el día de lectura o vuelve a leerlas."));
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn reconstruir(destino: &Path, archivos: &Value) {
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    /// El lote de Tomé del escenario «contexto» de Flow Python, pasado de Ambulatorio a Residencia (los JSON no entran
    /// en las huellas: solo los XLS y los HTML), con su carpeta de bitácoras.
    fn lote_residencia(nombre: &str) -> (PathBuf, PathBuf, PathBuf) {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/paquete.json")).unwrap();
        let e = datos["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "contexto").unwrap();
        let base = std::env::temp_dir().join(format!("flow-meta4-{}-{nombre}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (lote, bitacoras) = (base.join("lote"), base.join("bitacoras"));
        reconstruir(&lote, &e["lote"]["archivos"]);
        reconstruir(&bitacoras, &e["bitacoras"]["archivos"]);
        for ruta in [lote.join("resumen.json"), lote.join("verificacion 001.json"), bitacoras.join("bitacoras.json")] {
            let t = std::fs::read_to_string(&ruta).unwrap();
            let t = t.replace("\"modality\": \"2\"", "\"modality\": \"1\"").replace("\"modalidad\": \"2\"", "\"modalidad\": \"1\"");
            let t = t.replace("\"modalities\": [\"2\"]", "\"modalities\": [\"1\"]").replace(": \"Ambulatorio\"", ": \"Residencia\"").replace("/ Ambulatorio\"", "/ Residencia\"");
            let t = t.replace("\"modalities\": [\n      \"2\"\n    ]", "\"modalities\": [\n      \"1\"\n    ]");
            std::fs::write(ruta, t).unwrap();
        }
        (base, lote, bitacoras)
    }

    #[test]
    fn meta4_lee_el_lote_y_las_bitacoras_del_descargador() {
        let (base, lote, bitacoras) = lote_residencia("lee");
        let corte = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let m = calcular(&[lote.clone()], &[bitacoras.clone()], corte, corte, &[]).unwrap();
        assert_eq!(m.lotes, ["lote"]);
        assert_eq!(m.registros.len(), 2);
        assert!(m.registros.iter().all(|r| r.lista == "Cumplimiento" && r.tribunal == Some("TOMÉ")));
        // El HTML de las bitácoras nombra otro RIT: la identidad visible bloquea y la fecha queda vacía.
        assert!(m.registros.iter().all(|r| r.control == IDENTIDAD_VISIBLE && r.fecha_obs.is_none()), "{:?}", m.registros[0].control);
        assert_eq!(m.lecturas.len(), 2);
        assert!(m.lecturas.iter().all(|l| l.lectura == "LEIDA" && l.control == IDENTIDAD_VISIBLE));
        assert_eq!(m.fuentes.iter().filter(|f| f.tipo == "EXCEL").map(|f| f.registros).sum::<usize>(), 2);
        assert!(m.sin_bitacoras.is_empty());
        assert_eq!(fecha_lectura(&[lote.clone()]), None, "las bitácoras no están dentro del lote de registros");
        assert_eq!(fecha_lectura(&[bitacoras.clone()]), Some(corte));
        assert_eq!(tribunales_disponibles(&[lote.clone()]).unwrap(), ["TOMÉ"]);
        // Los productos de Meta 4 se publican desde este cálculo.
        let salida = base.join("Resultado META4");
        let archivos = flow_meta4::salida::publicar(&m, &salida).unwrap();
        assert!(archivos.iter().any(|a| a.file_name().is_some_and(|n| n.to_string_lossy().starts_with("META 4"))), "{archivos:?}");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Una carpeta de bitácoras elegida por un enlace o por su nombre corto de Windows (`RUNNER~1`) no es la ruta
    /// canónica: la lectura debe seguir verificando sus HTML (falló en la CI de Windows con `NO_VERIFICABLE`).
    #[cfg(unix)]
    #[test]
    fn meta4_lee_bitacoras_elegidas_por_una_ruta_no_canonica() {
        let (base, lote, bitacoras) = lote_residencia("enlace");
        let enlace = base.join("enlace");
        std::os::unix::fs::symlink(&bitacoras, &enlace).unwrap();
        let corte = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let m = calcular(&[lote], &[enlace], corte, corte, &[]).unwrap();
        assert!(m.registros.iter().all(|r| r.control == IDENTIDAD_VISIBLE), "{:?}", m.registros.iter().map(|r| &r.control).collect::<Vec<_>>());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn meta4_rechaza_lote_duplicado_filtrado_ajeno_o_incompleto() {
        let (base, lote, bitacoras) = lote_residencia("rechaza");
        let corte = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        // La misma consulta en dos carpetas de lote distintas duplicaría sus registros.
        let copia = base.join("otro").join("lote2");
        std::fs::create_dir_all(&copia).unwrap();
        for e in std::fs::read_dir(&lote).unwrap().flatten() {
            std::fs::copy(e.path(), copia.join(e.file_name())).unwrap();
        }
        let e = calcular(&[lote.clone(), copia.clone()], &[bitacoras.clone()], corte, corte, &[]).unwrap_err();
        assert!(e.contains("está en dos lotes"), "{e}");
        // Un tribunal que el lote no trae es un error; un nombre desconocido también.
        assert!(calcular(&[lote.clone()], &[bitacoras.clone()], corte, corte, &["Laja".into()]).unwrap_err().contains("tribunales elegidos"));
        assert!(calcular(&[lote.clone()], &[bitacoras.clone()], corte, corte, &["Narnia".into()]).unwrap_err().contains("no reconocido"));
        // Lote incompleto y bitácoras ausentes.
        let resumen = std::fs::read_to_string(lote.join("resumen.json")).unwrap();
        std::fs::write(lote.join("resumen.json"), resumen.replace("\"estado\": \"VALIDADA\",\n  \"plan\"", "\"estado\": \"INCOMPLETA\",\n  \"plan\"")).unwrap();
        assert!(calcular(&[lote.clone()], &[bitacoras.clone()], corte, corte, &[]).unwrap_err().contains("incompleta"));
        std::fs::write(lote.join("resumen.json"), resumen).unwrap();
        assert!(calcular(&[lote], &[], corte, corte, &[]).unwrap_err().contains("Faltan bitácoras"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn nombres_y_ultima_observacion_siguen_las_reglas_de_meta4() {
        assert!(nombres_compatibles("PEREZ GOMEZ, JUAN", "Juan Pérez Gómez"));
        assert!(nombres_compatibles("Juan Pérez Gómez", "Juan Pérez"));
        assert!(!nombres_compatibles("Juan Pérez", "Ana Pérez"));
        let f = |d: u32, h: u32| NaiveDate::from_ymd_opt(2026, 10, d).unwrap().and_hms_opt(h, 0, 0).unwrap();
        let corte = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
        let (dia, etapa, _) = ultima(&[(f(10, 9), "Cumplimiento".into()), (f(12, 9), "Espera".into()), (f(14, 9), "Informes".into())], corte);
        assert_eq!((dia, etapa), (NaiveDate::from_ymd_opt(2026, 10, 12), "Espera"));
    }
}

/// E-R1b · Oráculo contra `meta4_bitacoras.generar` (Flow Python): los lotes sintéticos de `app/qa/generar_meta4.py`
/// se leen con `calcular` y se comparan fila a fila, junto con lecturas, filtros, avisos de lectura y errores.
#[cfg(test)]
mod pruebas_oraculo {
    use super::*;
    use base64::Engine;

    fn reconstruir(destino: &Path, archivos: &Value) {
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    fn texto_de(v: &Value) -> String {
        v.as_str().unwrap_or_default().to_string()
    }

    /// Lo que entrega Rust con la forma del oráculo de Python.
    fn resultado(caso: &Value, base: &Path) -> Result<Value, String> {
        let lotes: Vec<PathBuf> = caso["lotes"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(nombre, archivos)| {
                reconstruir(&base.join(nombre), archivos);
                base.join(nombre)
            })
            .collect();
        let bitacoras: Vec<PathBuf> = if caso["bitacoras"].is_null() {
            Vec::new()
        } else {
            reconstruir(&base.join("bitacoras"), &caso["bitacoras"]);
            vec![base.join("bitacoras")]
        };
        let corte: NaiveDate = caso["corte"].as_str().unwrap().parse().unwrap();
        let tribunales: Vec<String> = caso["tribunales"].as_array().unwrap().iter().map(texto_de).collect();
        let m = calcular(&lotes, &bitacoras, corte, corte, &tribunales)?;
        // Una fila con una derivación que no es FAE ni Residencia queda fuera de META 4: Python la deja también fuera
        // de Registros y Rust la conserva en su libro Registros (`fuera_de_meta4` del reporte). Se compara aparte.
        let todas: Vec<&Registro4> = m.registros.iter().collect();
        let (fuera, dentro): (Vec<&Registro4>, Vec<&Registro4>) = todas.into_iter().partition(|r| r.categoria.is_none());
        let filas: Vec<Value> = dentro
            .iter()
            .map(|r| {
                serde_json::json!({
                    "lista": r.lista, "categoria": r.categoria, "tribunal": r.tribunal,
                    "rit": r.texto("RIT"), "rut": r.texto("RUT"), "nombre": r.texto("NOMBRE"), "derivacion": r.texto("DERIVACIÓN"),
                    "fecha_obs": r.fecha_obs.map(|f| f.to_string()), "etapa": r.etapa_ultima, "control": r.control, "alerta": r.alerta,
                })
            })
            .collect();
        let lecturas: Vec<Value> = m
            .lecturas
            .iter()
            .map(|l| serde_json::json!({"rit": l.rit, "ingreso": l.ingreso, "lectura": l.lectura, "control": l.control}))
            .collect();
        Ok(serde_json::json!({
            "filas": filas, "lecturas": lecturas, "filtros": m.filtros, "lotes": m.lotes,
            "sin_bitacoras": m.sin_bitacoras.iter().map(|(a, b)| [a, b]).collect::<Vec<_>>(),
            "avisos": m.avisos, "fuera_de_meta4": fuera.len(),
        }))
    }

    /// Diferencias entre Rust y Python para un caso (vacío si coinciden).
    fn comparar(caso: &Value, base: &Path, distintas: &mut BTreeSet<String>) -> Vec<String> {
        let nombre = caso["caso"].as_str().unwrap();
        let python = &caso["python"];
        let rust = match resultado(caso, base) {
            Err(e) => {
                return match python["error"].as_str() {
                    Some(p) if p == e => Vec::new(),
                    Some(p) => vec![format!("{nombre}: error distinto · Rust «{e}» · Python «{p}»")],
                    None => vec![format!("{nombre}: Rust falló («{e}») y Python no")],
                };
            }
            Ok(r) => r,
        };
        if let Some(p) = python["error"].as_str() {
            return vec![format!("{nombre}: Python falló («{p}») y Rust no")];
        }
        let mut dif = Vec::new();
        let ordenadas = |v: &Value, clave: &[&str]| {
            let mut filas = v.as_array().cloned().unwrap_or_default();
            filas.sort_by_key(|f| clave.iter().map(|k| f[*k].to_string()).collect::<Vec<_>>());
            filas
        };
        for (campo, clave) in [("filas", &["lista", "rit", "rut"][..]), ("lecturas", &["rit", "ingreso"][..])] {
            let (a, b) = (ordenadas(&rust[campo], clave), ordenadas(&python[campo], clave));
            if a.len() != b.len() {
                dif.push(format!("{nombre}.{campo}: {} filas en Rust y {} en Python", a.len(), b.len()));
            }
            for (i, (x, y)) in a.iter().zip(&b).enumerate() {
                if x != y {
                    dif.push(format!("{nombre}.{campo}[{i}]: Rust {x} · Python {y}"));
                }
            }
        }
        for campo in ["filtros", "lotes", "sin_bitacoras"] {
            if rust[campo] != python[campo] {
                dif.push(format!("{nombre}.{campo}: Rust {} · Python {}", rust[campo], python[campo]));
            }
        }
        if rust["fuera_de_meta4"] != python["fuera_de_alcance"] {
            dif.push(format!(
                "{nombre}: {} filas fuera de FAE/Residencia en Rust y {} en Python",
                rust["fuera_de_meta4"], python["fuera_de_alcance"]
            ));
        }
        for aviso in python["avisos_de_lectura"].as_array().unwrap() {
            if !rust["avisos"].as_array().unwrap().contains(aviso) {
                dif.push(format!("{nombre}: falta el aviso de lectura {aviso}"));
            }
        }
        if python.get("productos").is_some() {
            dif.extend(comparar_productos(caso, base, distintas));
        }
        dif
    }

    /// Hojas de los productos de `flow_meta4::salida` cuyos valores difieren de los de `generar` de Python, ya identificadas
    /// (`archivo | hoja`). La prueba falla si aparece otra o si una de estas deja de diferir: se revisan, no se acumulan.
    const HOJAS_DECLARADAS: [&str; 9] = [
        "Bitacoras_del_mes.xlsx | Control de lecturas",
        "Bitacoras_del_mes.xlsx | Observaciones del mes",
        "META 4 - actualizado 2026-08 - parcial.xlsx | Control bitácoras",
        "META 4 - actualizado 2026-08.xlsx | Control bitácoras",
        "Registros consolidados.xlsx | Alertas y revisión",
        "Registros consolidados.xlsx | Cumplimiento",
        "Registros.xlsx | CUMPLIMIENTO",
        "Registros.xlsx | ERRORES_Y_ADVERTENCIAS",
        "Registros.xlsx | INVENTARIO_ARCHIVOS",
    ];

    /// Los libros y el reporte de `flow_meta4::salida::publicar` contra los de `generar` de Python: devuelve las diferencias
    /// nuevas (hojas con valores distintos que no están declaradas) y las hojas distintas encontradas. El diseño (anchos,
    /// filtros y paneles) es de cada producto y no se compara.
    fn comparar_productos(caso: &Value, base: &Path, distintas: &mut BTreeSet<String>) -> Vec<String> {
        let nombre = caso["caso"].as_str().unwrap();
        let lotes: Vec<PathBuf> = caso["lotes"].as_object().unwrap().keys().map(|n| base.join(n)).collect();
        let corte: NaiveDate = caso["corte"].as_str().unwrap().parse().unwrap();
        let tribunales: Vec<String> = caso["tribunales"].as_array().unwrap().iter().map(texto_de).collect();
        let m = calcular(&lotes, &[base.join("bitacoras")], corte, corte, &tribunales).unwrap();
        let salida = base.join("Productos META4");
        let archivos = match flow_meta4::salida::publicar(&m, &salida) {
            Ok(a) => a,
            Err(e) => return vec![format!("{nombre}: Rust no publicó: {e}")],
        };
        let rust: BTreeMap<String, PathBuf> =
            archivos.into_iter().filter_map(|a| Some((a.file_name()?.to_string_lossy().into_owned(), a))).collect();
        let mut dif = comparar_libros(nombre, &caso["python"]["productos"], &rust, &HOJAS_DECLARADAS, distintas, false);
        // `Reporte META4.json`: las cifras de control del crate (otros nombres de campo) coinciden con las de Python. Fuera de
        // la comparación: `requieren_revision` (el crate suma las alertas; `revision_bitacora` de Python no).
        let reporte: Value = serde_json::from_slice(&std::fs::read(salida.join("Reporte META4.json")).unwrap()).unwrap();
        let python = &caso["python"]["productos"];
        for (rust_campo, python_campo) in [
            ("/periodo/desde", "desde"), ("/periodo/hasta", "hasta"), ("/registros", "registros"), ("/alertas", "alertas"),
            ("/conteo_por_control", "controles_bitacora"),
            ("/fuera_de_meta4", "excluidos_meta4"), ("/consultas_sin_bitacoras", "consultas_sin_bitacoras"),
            ("/lotes", "lotes_registros"), ("/filtros_registros", "filtros_registros"),
        ] {
            let propio = reporte.pointer(rust_campo).cloned().unwrap_or(Value::Null);
            if propio != python["reporte"][python_campo] {
                dif.push(format!("{nombre} · Reporte META4.json · {rust_campo}: Rust {propio} · Python {}", python["reporte"][python_campo]));
            }
        }
        dif
    }

    /// Compara los libros XLSX de una carpeta de productos de Rust con los volcados de Python: nombres, hojas y valores.
    /// Toda hoja con valores distintos queda en `distintas`; solo las declaradas dejan de ser una diferencia.
    /// `emision` normaliza la fila «Fecha Emisión» (ver la diferencia declarada en el oráculo de regeneración).
    fn comparar_libros(
        nombre: &str,
        python: &Value,
        rust: &BTreeMap<String, PathBuf>,
        declaradas: &[&str],
        distintas: &mut BTreeSet<String>,
        emision: bool,
    ) -> Vec<String> {
        use super::super::excel::volcado::{diferencias, volcar};
        let mut dif = Vec::new();
        let mut esperados: Vec<&String> = python["libros"].as_object().unwrap().keys().collect();
        let mut obtenidos: Vec<&String> = rust.keys().filter(|n| n.ends_with(".xlsx")).collect();
        esperados.sort();
        obtenidos.sort();
        if esperados != obtenidos {
            dif.push(format!("{nombre}: libros Rust {obtenidos:?} · Python {esperados:?}"));
        }
        // La fecha de emisión se compara aparte en el oráculo de regeneración: Python la reduce al día 1 del mes.
        let sin_emision = |hoja: &Value| {
            let mut hoja = hoja.clone();
            if let Some(filas) = hoja["celdas"].as_array_mut() {
                for fila in filas.iter_mut().filter_map(Value::as_array_mut) {
                    for celda in fila.iter_mut().filter_map(Value::as_array_mut) {
                        if celda.first().and_then(Value::as_str) == Some("s")
                            && celda.get(1).and_then(Value::as_str).is_some_and(|t| t.starts_with("Fecha Emisión"))
                        {
                            celda[1] = Value::from("<EMISIÓN>");
                        }
                    }
                }
            }
            hoja
        };
        for (archivo, volcado_python) in python["libros"].as_object().unwrap() {
            let Some(ruta) = rust.get(archivo) else { continue };
            let volcado_rust = volcar(&std::fs::read(ruta).unwrap(), "<BASE>");
            let titulos = |v: &Value| v["hojas"].as_array().unwrap().iter().map(|h| h["titulo"].clone()).collect::<Vec<_>>();
            if titulos(&volcado_rust) != titulos(volcado_python) {
                dif.push(format!("{nombre} · {archivo}: hojas Rust {:?} · Python {:?}", titulos(&volcado_rust), titulos(volcado_python)));
                continue;
            }
            for (r, p) in volcado_rust["hojas"].as_array().unwrap().iter().zip(volcado_python["hojas"].as_array().unwrap()) {
                let hoja = format!("{archivo} | {}", p["titulo"].as_str().unwrap());
                let valores = |h: &Value| {
                    let mut h = if emision { sin_emision(h) } else { h.clone() };
                    for k in ["congelar", "filtro", "anchos"] {
                        h[k] = Value::Null;
                    }
                    serde_json::json!({"hojas": [h]})
                };
                let d = diferencias(&valores(r), &valores(p));
                if !d.is_empty() {
                    distintas.insert(hoja.clone());
                    if !declaradas.contains(&hoja.as_str()) {
                        dif.push(format!("{nombre} · {hoja}: {}", d.join("\n")));
                    }
                }
            }
        }
        dif
    }

    fn casos() -> Vec<Value> {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/meta4.json")).unwrap();
        datos["casos"].as_array().unwrap().clone()
    }

    fn carpeta(nombre: &str) -> PathBuf {
        // Serie propia: dos pruebas que usan el mismo nombre no deben compartir carpeta temporal.
        static SERIE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serie = SERIE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("flow-meta4-oraculo-{}-{nombre}-{serie}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        base
    }

    #[test]
    fn meta4_del_descargador_igual_a_python() {
        let casos = casos();
        let (mut filas, mut errores, mut diferencias, mut distintas) = (0usize, 0usize, Vec::new(), BTreeSet::new());
        for caso in &casos {
            let base = carpeta(caso["caso"].as_str().unwrap());
            diferencias.extend(comparar(caso, &base, &mut distintas));
            let _ = std::fs::remove_dir_all(&base);
            match caso["python"]["error"].as_str() {
                Some(_) => errores += 1,
                None => filas += caso["python"]["filas"].as_array().unwrap().len(),
            }
        }
        assert!(diferencias.is_empty(), "{}", diferencias.join("\n"));
        let declaradas: BTreeSet<String> = HOJAS_DECLARADAS.iter().map(|h| h.to_string()).collect();
        assert_eq!(distintas, declaradas, "hojas con valores distintos de Python (Rust ⊃ declaradas = nuevas; Rust ⊂ = ya no difieren)");
        assert_eq!((casos.len(), errores, filas), (33, 7, 55), "conteos del oráculo");
        println!("oraculo meta4: {} casos, {errores} errores de Python reproducidos, {filas} filas", casos.len());
    }

    #[test]
    fn oraculo_de_meta4_detecta_alteraciones() {
        let casos = casos();
        let mut caso = casos.iter().find(|c| c["caso"] == "observacion_en_el_mes").unwrap().clone();
        caso["python"]["filas"][0]["fecha_obs"] = Value::from("2026-08-13");
        let base = carpeta("alterado");
        let dif = comparar(&caso, &base, &mut BTreeSet::new());
        let _ = std::fs::remove_dir_all(&base);
        assert!(dif.iter().any(|d| d.contains("filas[")), "{dif:?}");
    }

    fn datos() -> Value {
        serde_json::from_str(include_str!("../../tests/datos/meta4.json")).unwrap()
    }

    /// Meta 4 del caso indicado del oráculo, reconstruyendo sus lotes y bitácoras.
    fn meta4_del_caso(caso: &Value, base: &Path) -> Meta4 {
        let lotes: Vec<PathBuf> = caso["lotes"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(nombre, archivos)| {
                reconstruir(&base.join(nombre), archivos);
                base.join(nombre)
            })
            .collect();
        let bitacoras: Vec<PathBuf> = if caso["bitacoras"].is_null() {
            Vec::new()
        } else {
            reconstruir(&base.join("bitacoras"), &caso["bitacoras"]);
            vec![base.join("bitacoras")]
        };
        let corte: NaiveDate = caso["corte"].as_str().unwrap().parse().unwrap();
        let tribunales: Vec<String> = caso["tribunales"].as_array().unwrap().iter().map(texto_de).collect();
        calcular(&lotes, &bitacoras, corte, corte, &tribunales).unwrap()
    }

    /// Hojas de los productos regenerados cuyos valores difieren de `regenerar` de Python, ya identificadas
    /// (`archivo | hoja`): la prueba falla si aparece otra o si alguna deja de diferir. Las dos son auxiliares:
    /// «Control bitácoras» escribe las fechas y las cifras como texto y no agrega la línea «Regenerado desde…», y
    /// «ERRORES_Y_ADVERTENCIAS» la reescribe Flow con sus avisos en vez de conservar la del libro revisado.
    const HOJAS_REGENERADAS: [&str; 2] = [
        "META 4 - actualizado 2026-08.xlsx | Control bitácoras",
        "Registros.xlsx | ERRORES_Y_ADVERTENCIAS",
    ];

    /// E-R2 · Regeneración desde un `Registros.xlsx` revisado: `regenerar` de Python contra
    /// `flow_meta4::regenerar` con las mismas celdas editadas, con y sin `Bitacoras_del_mes.xlsx` y con plantilla.
    fn comparar_regeneracion(caso: &Value) -> (Vec<String>, BTreeSet<String>, usize) {
        let nombre = caso["escenario"].as_str().unwrap();
        let mut diferencias = Vec::new();
        let mut distintas = BTreeSet::new();
        let base = carpeta(&format!("regeneracion-{nombre}"));
        let entrada = base.join("entrada");
        reconstruir(&entrada, &caso["entrada"]);
        let salida = base.join("Regenerado");
        let archivos = match flow_meta4::regenerar::ejecutar(&entrada.join("Registros.xlsx"), &salida) {
            Ok(a) => a,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&base);
                return (vec![format!("{nombre}: Rust no regeneró: {e}")], distintas, 0);
            }
        };
        let mut rust: BTreeMap<String, PathBuf> =
            archivos.into_iter().filter_map(|a| Some((a.file_name()?.to_string_lossy().into_owned(), a))).collect();
        let libros = rust.values().filter(|p| p.extension().is_some_and(|e| e == "xlsx")).count();
        let python = &caso["python"];
        // `Bitacoras_del_mes.xlsx` se copia sin reinterpretar: se compara byte a byte, no por valores (el libro
        // copiado lo escribió openpyxl y el volcado de valores solo lee libros de rust_xlsxwriter).
        let original = caso["entrada"]["Bitacoras_del_mes.xlsx"].as_str();
        let copia = rust.remove("Bitacoras_del_mes.xlsx");
        let mut libros_python = python["productos"].clone();
        libros_python["libros"].as_object_mut().unwrap().remove("Bitacoras_del_mes.xlsx");
        match (original, copia) {
            (Some(_), Some(destino)) if std::fs::read(&entrada.join("Bitacoras_del_mes.xlsx")).unwrap() != std::fs::read(&destino).unwrap() => {
                diferencias.push(format!("{nombre}: la copia de Bitacoras_del_mes.xlsx cambió de bytes"));
            }
            (Some(_), None) => diferencias.push(format!("{nombre}: Rust no conservó Bitacoras_del_mes.xlsx")),
            (None, Some(_)) => diferencias.push(format!("{nombre}: Rust copió un Bitacoras_del_mes.xlsx que no existía")),
            _ => {}
        }
        diferencias.extend(comparar_libros(nombre, &libros_python, &rust, &HOJAS_REGENERADAS, &mut distintas, true));
        // Diferencia declarada: al regenerar, Python escribe el día 1 del mes de emisión (su reporte solo guarda
        // AAAA-MM); Rust conserva la fecha de corte de su propio reporte.
        if let (Some(ruta), Some(celda_python)) = (
            rust.get("META 4 - actualizado 2026-08.xlsx"),
            libros_python["libros"]["META 4 - actualizado 2026-08.xlsx"]["hojas"][0]["celdas"][2][0][1].as_str(),
        ) {
            let volcado = super::super::excel::volcado::volcar(&std::fs::read(ruta).unwrap(), "<BASE>");
            let celda_rust = volcado["hojas"][0]["celdas"][2][0][1].as_str().unwrap_or("");
            if !celda_rust.contains("31-08-2026") || !celda_python.contains("01-08-2026") {
                diferencias.push(format!(
                    "{nombre} · fecha de emisión: Rust «{celda_rust}» · Python «{celda_python}» (se esperaba 31-08 en Rust y 01-08 en Python)"
                ));
            }
        }
        let reporte: Value = serde_json::from_slice(&std::fs::read(salida.join("Reporte META4.json")).unwrap()).unwrap();
        let p = &python["reporte"];
        for (campo, esperado) in [
            ("/periodo/desde", &p["desde"]), ("/periodo/hasta", &p["hasta"]),
            ("/registros", &p["registros"]), ("/alertas", &p["alertas"]),
            ("/conteo_por_control", &p["controles_bitacora"]), ("/resumen_por_tribunal", &p["resumen_tribunales"]),
            ("/totales_por_hoja", &p["totales_meta4"]), ("/fuera_de_meta4", &p["excluidos_meta4"]),
        ] {
            let propio = reporte.pointer(campo).cloned().unwrap_or(Value::Null);
            if &propio != esperado {
                diferencias.push(format!("{nombre} · reporte {campo}: Rust {propio} · Python {esperado}"));
            }
        }
        let emision = reporte["fecha_emision"].as_str().unwrap_or("");
        if emision.get(..7) != p["mes_emision"].as_str() {
            diferencias.push(format!("{nombre} · mes de emisión: Rust {emision} · Python {}", p["mes_emision"]));
        }
        let tribunales = if p["tribunales"] == "TODOS" { Value::from("Todos") } else { p["tribunales"].clone() };
        if reporte["tribunales_incluidos"] != tribunales {
            diferencias.push(format!("{nombre} · tribunales: Rust {} · Python {tribunales}", reporte["tribunales_incluidos"]));
        }
        let _ = std::fs::remove_dir_all(&base);
        (diferencias, distintas, libros)
    }

    #[test]
    fn regeneracion_del_registros_revisado_igual_a_python() {
        let datos = datos();
        let (mut diferencias, mut distintas, mut libros) = (Vec::new(), BTreeSet::new(), 0usize);
        for caso in datos["regeneraciones"].as_array().unwrap() {
            let (dif, hojas, n) = comparar_regeneracion(caso);
            diferencias.extend(dif);
            distintas.extend(hojas);
            libros += n;
        }
        assert!(diferencias.is_empty(), "{}", diferencias.join("\n"));
        assert_eq!(libros, 11, "libros regenerados");
        let declaradas: BTreeSet<String> = HOJAS_REGENERADAS.iter().map(|h| h.to_string()).collect();
        assert_eq!(distintas, declaradas, "hojas con valores distintos al regenerar (Rust ⊃ declaradas = nuevas; Rust ⊂ = ya no difieren)");
        println!("oraculo de regeneración: {} escenarios, {libros} libros", datos["regeneraciones"].as_array().unwrap().len());
    }

    /// Una cifra del reporte y una celda del libro regenerado alteradas a mano deben hacer fallar el comparador.
    #[test]
    fn oraculo_de_regeneracion_detecta_alteraciones() {
        let datos = datos();
        let mut caso = datos["regeneraciones"][0].clone();
        caso["python"]["reporte"]["registros"] = Value::from(99);
        let (dif, _, _) = comparar_regeneracion(&caso);
        assert!(dif.iter().any(|d| d.contains("/registros")), "{dif:?}");
        let mut caso = datos["regeneraciones"][0].clone();
        caso["python"]["productos"]["libros"]["Registros.xlsx"]["hojas"][1]["celdas"][1][0][1] = Value::from("OTRO");
        let (dif, _, _) = comparar_regeneracion(&caso);
        assert!(dif.iter().any(|d| d.contains("Registros.xlsx | CUMPLIMIENTO")), "{dif:?}");
    }

    /// E-R2 · Plantilla propia: `validar_plantilla` de Python (que la revisa antes de generar) contra lo que Rust
    /// acepta al publicar con plantilla. Una plantilla con celdas combinadas la acepta Python y Rust la rechaza: es
    /// una diferencia declarada (Rust no copia combinaciones ni dibujos para no alterar los datos).
    fn comparar_plantillas(datos: &Value) -> (Vec<String>, usize, usize) {
        let base_caso = datos["casos"].as_array().unwrap().iter().find(|c| c["caso"] == "observacion_en_el_mes").unwrap().clone();
        let mut resultados = Vec::new();
        for caso in datos["plantillas"].as_array().unwrap() {
            let nombre = caso["caso"].as_str().unwrap();
            let base = carpeta(&format!("plantilla-{nombre}"));
            let m = meta4_del_caso(&base_caso, &base);
            let archivo = base.join("plantilla.xlsx");
            std::fs::write(&archivo, base64::engine::general_purpose::STANDARD.decode(caso["archivo"].as_str().unwrap()).unwrap())
                .unwrap();
            let salida = base.join("Productos con plantilla");
            let resultado = flow_meta4::salida::publicar_con_plantilla(&m, &salida, Some(&archivo));
            resultados.push((nombre.to_string(), caso["python"]["acepta"].as_bool().unwrap(), resultado));
            let _ = std::fs::remove_dir_all(&base);
        }
        let mut problemas = Vec::new();
        let mut publicados = 0usize;
        for (nombre, python_acepta, resultado) in &resultados {
            let rust_acepta = resultado.is_ok();
            if nombre == "plantilla_con_combinadas" {
                // Diferencia declarada: Python la acepta y Rust la rechaza con su motivo.
                if !*python_acepta || rust_acepta {
                    problemas.push(format!("{nombre}: se esperaba Python sí y Rust no"));
                }
                continue;
            }
            if *python_acepta != rust_acepta {
                problemas.push(format!("{nombre}: Python acepta {python_acepta} y Rust {rust_acepta} ({:?})", resultado.as_ref().err()));
            }
            if rust_acepta {
                publicados += 1;
            }
            match (nombre.as_str(), resultado) {
                ("plantilla_sin_una_hoja", Err(e)) if !e.contains("FAE Cumplimiento") => {
                    problemas.push(format!("{nombre}: el error no nombra la hoja que falta ({e})"));
                }
                ("plantilla_ilegible", Err(e)) if !e.contains("plantilla") => {
                    problemas.push(format!("{nombre}: el error no explica que la plantilla no se pudo abrir ({e})"));
                }
                _ => {}
            }
        }
        (problemas, resultados.len(), publicados)
    }

    #[test]
    fn plantilla_propia_igual_a_python() {
        let (problemas, plantillas, publicados) = comparar_plantillas(&datos());
        assert!(problemas.is_empty(), "{}", problemas.join("\n"));
        assert_eq!((plantillas, publicados), (5, 2), "plantillas contrastadas y aceptadas por ambos");
        println!("oraculo de plantilla: {plantillas} plantillas, {publicados} aceptadas");
    }

    /// Aceptar en Python una plantilla que Rust rechaza debe hacer fallar el comparador (salvo la declarada).
    #[test]
    fn oraculo_de_plantilla_detecta_alteraciones() {
        let mut datos = datos();
        let sin_hoja = datos["plantillas"].as_array().unwrap().iter().position(|c| c["caso"] == "plantilla_sin_una_hoja").unwrap();
        datos["plantillas"][sin_hoja]["python"]["acepta"] = Value::from(true);
        let (problemas, _, _) = comparar_plantillas(&datos);
        assert!(problemas.iter().any(|p| p.contains("plantilla_sin_una_hoja")), "{problemas:?}");
        let mut datos = self::datos();
        let valida = datos["plantillas"].as_array().unwrap().iter().position(|c| c["caso"] == "plantilla_valida").unwrap();
        datos["plantillas"][valida]["python"]["acepta"] = Value::from(false);
        let (problemas, _, _) = comparar_plantillas(&datos);
        assert!(problemas.iter().any(|p| p.contains("plantilla_valida")), "{problemas:?}");
    }
}
