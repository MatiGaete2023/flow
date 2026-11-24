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
    let manifiestos = buscar_manifiestos(raices);
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
        let lote = paquete::cargar_lote(&carpeta).map_err(|e| format!("{e} ({nombre})"))?;
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
            return Err("Los lotes no contienen registros de los tribunales elegidos.".into());
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
