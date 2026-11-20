use super::{
    json, Captura, Fila, Tipo, TABLAS_BASE_OPCIONALES, TABLAS_EXTENDIDAS, TABLAS_REQUERIDAS,
};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime};
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const ESTADOS: &[&str] = &[
    "NO_SOLICITADA",
    "NO_CONSULTADA",
    "COMPLETA",
    "VACIA_COMPROBADA",
    "PARCIAL",
    "FALLIDA",
    "CANCELADA",
    "IDENTIDAD_INCIERTA",
    "NO_DISPONIBLE",
];
const SIN_DATOS: &[&str] = &[
    "NO_SOLICITADA",
    "NO_CONSULTADA",
    "NO_DISPONIBLE",
    "FALLIDA",
    "CANCELADA",
];
const FUENTES: &[&str] = &[
    "registros",
    "bitacoras",
    "historia_tramites",
    "calendarios",
    "documentos",
];
const IDENTIDAD: [&str; 4] = ["source_system", "court_code", "cause_id", "ingreso_id"];

fn adecuada(s: &str) -> bool {
    matches!(s, "COMPLETA" | "VACIA_COMPROBADA")
}

fn binario(s: &str) -> bool {
    matches!(s, "true" | "false")
}

fn entero(s: &str) -> Option<u64> {
    if s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

fn huella(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Clone, Copy)]
enum Fecha {
    Dia(NaiveDate),
    Local(NaiveDateTime),
    ConZona(DateTime<chrono::FixedOffset>),
}

impl Fecha {
    fn dia(self) -> NaiveDate {
        match self {
            Self::Dia(d) => d,
            Self::Local(d) => d.date(),
            Self::ConZona(d) => d.date_naive(),
        }
    }

    fn posterior(self, otra: Self) -> bool {
        match (self, otra) {
            (Self::Local(a), Self::Local(b)) => a > b,
            (Self::ConZona(a), Self::ConZona(b)) => a > b,
            (a, b) => a.dia() > b.dia(),
        }
    }
}

fn fecha(s: &str, admite_dia: bool) -> Result<Fecha, &'static str> {
    static ISO: OnceLock<Regex> = OnceLock::new();
    let error = "Fecha ISO 8601 inválida o fuera de 1900–2199.";
    let fecha = if admite_dia && s.len() == 10 {
        Fecha::Dia(NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| error)?)
    } else {
        let patron = ISO.get_or_init(|| Regex::new(
            r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-5][0-9](\.[0-9]{1,6})?(Z|[+-][0-9]{2}:[0-9]{2})?$"
        ).unwrap());
        if !patron.is_match(s) {
            return Err(error);
        }
        match DateTime::parse_from_rfc3339(s) {
            Ok(d) => Fecha::ConZona(d),
            Err(_) => Fecha::Local(
                NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").map_err(|_| error)?,
            ),
        }
    };
    if !(1900..=2199).contains(&fecha.dia().year()) {
        return Err(error);
    }
    Ok(fecha)
}

struct Consulta<'a> {
    fila: Fila<'a>,
    filtros: Map<String, Value>,
    tablas: Vec<&'static str>,
}

impl<'a> Consulta<'a> {
    fn leer(fila: Fila<'a>) -> Result<Self, &'static str> {
        let cobertura = fila.get("coverage");
        if !ESTADOS.contains(&cobertura) || fila.get("query_id").is_empty() {
            return Err("ID o cobertura de consulta inválidos.");
        }
        let guardadas = entero(fila.get("saved_pages")).ok_or("Páginas inválidas.")?;
        let esperadas = entero(fila.get("expected_pages")).ok_or("Páginas inválidas.")?;
        if guardadas > esperadas || (cobertura == "COMPLETA" && guardadas != esperadas) {
            return Err("Cobertura de páginas inconsistente.");
        }
        if !binario(fila.get("history_complete")) || !binario(fila.get("institutional_verified")) {
            return Err("Garantía de fuente inválida.");
        }
        fecha(fila.get("captured_at"), false)?;
        let periodo = ["period_from", "period_to"].map(|k| {
            let s = fila.get(k);
            (!s.is_empty()).then(|| fecha(s, true)).transpose()
        });
        let [inicio, fin] = periodo;
        if let (Some(a), Some(b)) = (inicio?, fin?) {
            if a.posterior(b) {
                return Err("Período de consulta invertido.");
            }
        }
        let filtros = json::objeto(fila.get("filters_json"))?;
        let tablas = Self::tablas(fila.get("screen"), filtros.get("capture_tables"))?;
        if let Some(scope) = filtros.get("remote_identity").filter(|v| !v.is_null()) {
            let scope = scope.as_object().ok_or("Identidad de alcance no válida.")?;
            if scope.len() != IDENTIDAD.len()
                || IDENTIDAD.iter().any(|k| {
                    scope
                        .get(*k)
                        .and_then(Value::as_str)
                        .is_none_or(str::is_empty)
                })
                || scope["source_system"].as_str() != Some(fila.get("source_system"))
                || scope["court_code"].as_str() != Some(fila.get("court_code"))
            {
                return Err("Identidad de alcance incompleta o ajena.");
            }
        }
        Ok(Self {
            fila,
            filtros,
            tablas,
        })
    }

    fn tablas(pantalla: &str, valor: Option<&Value>) -> Result<Vec<&'static str>, &'static str> {
        if let Some(v) = valor.filter(|v| !v.is_null()) {
            let valores = v
                .as_array()
                .filter(|v| !v.is_empty())
                .ok_or("Alcance de tablas inválido.")?;
            let mut tablas = Vec::new();
            for v in valores {
                let nombre = FUENTES
                    .iter()
                    .copied()
                    .find(|t| Some(*t) == v.as_str())
                    .ok_or("Tabla fuera del alcance permitido.")?;
                if tablas.contains(&nombre) {
                    return Err("Tabla repetida en el alcance.");
                }
                tablas.push(nombre);
            }
            return Ok(tablas);
        }
        Ok(match pantalla {
            "seguimiento" | "carga" => vec!["registros"],
            "calendario_informes" => vec!["registros", "calendarios"],
            _ => FUENTES
                .iter()
                .copied()
                .filter(|&t| t == pantalla && t != "registros")
                .collect(),
        })
    }

    fn validar_fila(
        &self,
        tabla: &str,
        fila: Fila<'a>,
        cobertura_tabla: &str,
    ) -> Result<(), &'static str> {
        if ["source_system", "court_code", "modality_code"]
            .iter()
            .any(|k| fila.get(k) != self.fila.get(k))
        {
            return Err("Identidad de consulta cruzada.");
        }
        let cobertura = fila.get("coverage");
        let cobertura_consulta = self.fila.get("coverage");
        if !ESTADOS.contains(&cobertura) {
            return Err("Cobertura de fila inválida.");
        }
        if FUENTES.contains(&tabla) {
            if let Some(scope) = self
                .filtros
                .get("remote_identity")
                .and_then(Value::as_object)
            {
                if IDENTIDAD
                    .iter()
                    .any(|k| scope[*k].as_str() != Some(fila.get(k)))
                {
                    return Err("Fila fuera de la identidad consultada.");
                }
            }
            if adecuada(cobertura_tabla) && (!adecuada(cobertura) || !adecuada(cobertura_consulta))
            {
                return Err("La tabla declara mayor cobertura que sus consultas o filas.");
            }
        }
        if cobertura == "VACIA_COMPROBADA"
            || cobertura_consulta == "VACIA_COMPROBADA"
            || (adecuada(cobertura) && !adecuada(cobertura_consulta))
        {
            return Err("Cobertura de fila incompatible con su consulta.");
        }
        if !huella(fila.get("source_sha256"))
            || fila.get("original_file").is_empty()
            || entero(fila.get("original_row")).unwrap_or(0) == 0
        {
            return Err("Procedencia incompleta o inválida.");
        }
        fecha(fila.get("captured_at"), false)?;
        if let Some(estado) = fila.campo("identity_status") {
            if !matches!(estado, "VERIFICADA" | "INCIERTA" | "CAUSA")
                || (estado == "VERIFICADA" && !identidad_completa(fila))
            {
                return Err("Identidad remota inválida o incompleta.");
            }
        }
        if let Some(p) = fila.campo("time_precision") {
            if !matches!(p, "DIA" | "SEGUNDO" | "DESCONOCIDA") {
                return Err("Precisión de fecha desconocida.");
            }
        }
        for k in ["event_at", "response_at", "linked_projection"] {
            if !fila.get(k).is_empty() {
                fecha(fila.get(k), true)?;
            }
        }
        for k in ["raw_json", "normalized_json"] {
            if let Some(v) = fila.campo(k) {
                json::objeto(v)?;
            }
        }
        Ok(())
    }

    fn alcance_extendido(&self, fila: Fila<'a>, tabla: &str, filtro: &str, campo: &str) -> bool {
        self.fila.get("screen") == tabla
            && self
                .filtros
                .get(filtro)
                .and_then(Value::as_object)
                .is_some_and(|obj| {
                    ["source_system", "court_code", campo]
                        .into_iter()
                        .all(|k| obj.get(k).and_then(Value::as_str) == Some(fila.get(k)))
                })
    }
}

fn identidad_completa(fila: Fila<'_>) -> bool {
    IDENTIDAD[1..].iter().all(|k| !fila.get(k).is_empty())
}

fn validar_tipo<'a>(
    tabla: &str,
    f: Fila<'a>,
    q: &Consulta<'a>,
    originales: &HashMap<&str, &str>,
) -> Result<(&'static str, &'static str), &'static str> {
    match tabla {
        "registros"
            if matches!(
                f.get("record_type"),
                "ESPERA" | "CUMPLIMIENTO" | "INFORMES" | "EGRESADOS" | "OTRO"
            ) =>
        {
            Ok(("registros", "record_id"))
        }
        "bitacoras"
            if matches!(f.get("origin"), "CENTRO" | "OTRO" | "DESCONOCIDO")
                && binario(f.get("origin_verified")) =>
        {
            Ok(("eventos", "event_id"))
        }
        "historia_tramites"
            if f.campo("annex_visible").is_none_or(binario)
                && match f.get("scope") {
                    "" | "INGRESO" => true,
                    "CAUSA" => f.get("ingreso_id").is_empty(),
                    _ => false,
                } =>
        {
            Ok(("eventos", "event_id"))
        }
        "calendarios"
            if match f.get("association") {
                "VERIFICADA" => identidad_completa(f),
                "NO_ASOCIADO" => true,
                _ => false,
            } =>
        {
            Ok(("calendarios", "calendar_id"))
        }
        "escritos"
            if f.get("scope") == "CAUSA"
                && f.get("ingreso_id").is_empty()
                && f.get("linked_projection").is_empty()
                    == f.get("linked_ingreso_id").is_empty()
                && q.alcance_extendido(f, tabla, "escritos_causa", "cause_id") =>
        {
            Ok(("escritos", "escrito_id"))
        }
        "historico_proteccion"
            if (f.get("ownership") == "INCIERTA"
                || (f.get("ownership") == "VERIFICADA" && !f.get("person_id").is_empty()))
                && q.alcance_extendido(f, tabla, "historico_persona", "person_id") =>
        {
            Ok(("historico_proteccion", "historico_id"))
        }
        "documentos"
            if f.get("content_read") == "false"
                && (f.get("sha256").is_empty() || huella(f.get("sha256")))
                && match f.get("download_state") {
                    "NO_SOLICITADA" | "NO_DISPONIBLE" | "FALLIDA" => true,
                    "DESCARGADO" => {
                        originales.get(f.get("local_file")).copied() == Some(f.get("sha256"))
                    }
                    _ => false,
                } =>
        {
            Ok(("documentos", "document_id"))
        }
        "incidencias" if binario(f.get("recoverable")) => Ok(("incidencias", "incident_id")),
        _ => Err("Tipo, identidad o alcance de fila incompatibles."),
    }
}

pub(super) fn validar(captura: &Captura, menor: u64) -> Result<(), String> {
    let m = &captura.manifiesto;
    let base = TABLAS_REQUERIDAS.into_iter().chain(TABLAS_BASE_OPCIONALES);
    let extendido = menor >= 2 && m.table_states.len() == 9;
    let declarables = base
        .clone()
        .chain(TABLAS_EXTENDIDAS.into_iter().filter(|_| extendido));
    if declarables.clone().count() != m.table_states.len()
        || declarables
            .clone()
            .any(|t| !ESTADOS.contains(&captura.estado(t)) || !m.table_states.contains_key(t))
    {
        return Err("Faltan estados de tabla o hay estados desconocidos.".into());
    }
    if m.mapping_version.trim().is_empty()
        || m.contains_sensitive_data.is_none()
        || ["version", "commit"]
            .iter()
            .any(|k| m.downloader[k].as_str().is_none_or(str::is_empty))
    {
        return Err(
            "El manifiesto carece de versión de mapeo, política de datos o productor.".into(),
        );
    }
    fecha(&m.captured_at, false)?;

    let mut consultas = HashMap::new();
    let csv = captura.filas("consultas").collect::<Vec<_>>();
    let manifiesto = m
        .queries
        .as_ref()
        .ok_or("El manifiesto carece de consultas.")?;
    if csv.len() != manifiesto.len() {
        return Err("Consultas CSV/manifiesto no coinciden.".into());
    }
    for (fila, q) in csv.into_iter().zip(manifiesto) {
        if fila.pares().count() != q.len()
            || fila
                .pares()
                .any(|(k, v)| q.get(k).map(String::as_str) != Some(v))
        {
            return Err("Consultas CSV/manifiesto no coinciden.".into());
        }
        let consulta = Consulta::leer(fila)?;
        if consultas.insert(fila.get("query_id"), consulta).is_some() {
            return Err("Consulta duplicada.".into());
        }
    }

    let originales = m
        .files
        .iter()
        .filter_map(|f| match f.tipo {
            Tipo::Original => Some((f.path.as_str(), f.sha256.as_str())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    let mut ids: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut eventos = HashMap::new();
    for tabla in base.chain(TABLAS_EXTENDIDAS) {
        let estado = captura.estado(tabla);
        let filas = captura.filas(tabla);
        let hay_filas = captura.filas(tabla).next().is_some();
        if (!captura.tablas.contains_key(tabla) && !SIN_DATOS.contains(&estado))
            || (hay_filas
                && (estado == "VACIA_COMPROBADA"
                    || (SIN_DATOS.contains(&estado)
                        && (!TABLAS_REQUERIDAS.contains(&tabla) || tabla == "registros"))))
        {
            return Err(format!(
                "Estado de tabla incompatible con el contenido de {tabla}."
            ));
        }
        if FUENTES.contains(&tabla)
            && adecuada(estado)
            && consultas
                .values()
                .any(|q| q.tablas.contains(&tabla) && !adecuada(q.fila.get("coverage")))
        {
            return Err(format!(
                "La tabla {tabla} declara mayor cobertura que su consulta."
            ));
        }
        if tabla == "consultas" {
            continue;
        }
        for (i, fila) in filas.enumerate() {
            let mut validar = || -> Result<(), &str> {
                let consulta = consultas
                    .get(fila.get("query_id"))
                    .ok_or("Consulta inexistente.")?;
                if fila.get("capture_id") != m.capture_id {
                    return Err("Fila de otra captura.");
                }
                if tabla != "incidencias" {
                    consulta.validar_fila(tabla, fila, estado)?;
                }
                let (grupo, clave) = validar_tipo(tabla, fila, consulta, &originales)?;
                let id = fila.get(clave);
                if id.is_empty() || !ids.entry(grupo).or_default().insert(id) {
                    return Err("Identificador vacío o duplicado.");
                }
                if grupo == "eventos" {
                    eventos.insert(id, fila);
                }
                Ok(())
            };
            validar().map_err(|e| format!("{tabla}, registro {}: {e}", i + 1))?;
        }
    }
    for d in captura.filas("documentos") {
        let padre = eventos
            .get(d.get("parent_event_id"))
            .ok_or("Documento sin trámite padre.")?;
        if IDENTIDAD.iter().any(|k| d.get(k) != padre.get(k))
            || d.get("scope") != padre.campo("scope").unwrap_or("INGRESO")
        {
            return Err("Documento de otra identidad o alcance.".into());
        }
    }
    for (tabla, campo, grupo) in [
        ("historia_tramites", "document_id", "documentos"),
        ("incidencias", "record_id", "registros"),
    ] {
        if captura.filas(tabla).any(|f| {
            let id = f.get(campo);
            !id.is_empty() && !ids.get(grupo).is_some_and(|set| set.contains(id))
        }) {
            return Err(format!("Referencia sin destino en {tabla}."));
        }
    }
    Ok(())
}
