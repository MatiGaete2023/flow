//! Adapta las filas normalizadas del paquete a las hojas de Trabajo.

use crate::lectura::{self, Sheet};
use crate::motor::{Cols, Mode};
use crate::util::Cell;
use chrono::NaiveDate;
use flow_rust::captura::{self, revision::Revision, Captura, Fila};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use unicode_normalization::UnicodeNormalization;

mod cronologia;

const LIMITE_ANALISIS: usize = 20_000;
const INGRESO: [&str; 4] = ["source_system", "court_code", "cause_id", "ingreso_id"];
const VENTANA_INFORMES_DIAS: i64 = 60;
const PROCEDENCIA: [(&str, &str); 5] = [
    ("SITFA_ARCHIVO", "original_file"),
    ("SITFA_HOJA", "original_sheet"),
    ("SITFA_FILA", "original_row"),
    ("SITFA_SHA256", "source_sha256"),
    ("ID_REGISTRO_CAPTURA", "record_id"),
];

/// Un aviso del paquete con todas sus repeticiones agrupadas (código, severidad y mensaje iguales).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvisoPaquete {
    pub codigo: String,
    pub severidad: String,
    pub mensaje: String,
    pub incidencias: usize,
    pub consultas: usize,
}

pub struct PaqueteTrabajo {
    pub fuente: Captura,
    pub modos: Vec<Mode>,
    pub columnas_por_fila: Vec<Vec<Cols>>,
    pub omitidos: usize,
    /// Fuentes de lectura por `record_id` del paquete.
    pub contextos: HashMap<String, ContextoFila>,
}

impl PaqueteTrabajo {
    /// Incidencias del paquete agrupadas por código y mensaje, de la más repetida a la menos. El paquete conserva una
    /// fila por registro (contrato de Flow Python); aquí se muestran una sola vez, con su cantidad.
    pub fn avisos_agrupados(&self) -> Vec<AvisoPaquete> {
        let mut grupos: Vec<(AvisoPaquete, HashSet<String>)> = Vec::new();
        for i in self.fuente.filas("incidencias") {
            let clave = (i.get("code"), i.get("severity"), i.get("message"));
            let pos = grupos.iter().position(|(a, _)| (a.codigo.as_str(), a.severidad.as_str(), a.mensaje.as_str()) == clave);
            let (aviso, consultas) = match pos {
                Some(p) => &mut grupos[p],
                None => {
                    grupos.push((
                        AvisoPaquete { codigo: clave.0.into(), severidad: clave.1.into(), mensaje: clave.2.into(), incidencias: 0, consultas: 0 },
                        HashSet::new(),
                    ));
                    grupos.last_mut().unwrap()
                }
            };
            aviso.incidencias += 1;
            consultas.insert(i.get("query_id").to_string());
            aviso.consultas = consultas.len();
        }
        let mut avisos: Vec<_> = grupos.into_iter().map(|(a, _)| a).collect();
        avisos.sort_by_key(|a| std::cmp::Reverse(a.incidencias));
        avisos
    }

    pub fn actualizar_contextos(&mut self, corte: NaiveDate) {
        self.contextos = contextos(&self.fuente, corte);
    }

    pub(crate) fn identidades_ingresos(&self) -> HashMap<&str, [&str; 4]> {
        self.fuente
            .filas("registros")
            .filter(|r| r.get("identity_status") == "VERIFICADA")
            .filter_map(|r| {
                let identidad = INGRESO.map(|campo| r.get(campo));
                identidad
                    .iter()
                    .all(|s| !s.is_empty())
                    .then_some((r.get("record_id"), identidad))
            })
            .collect()
    }
}

pub fn es_zip(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

pub fn abrir(bytes: &[u8]) -> Result<(PaqueteTrabajo, Vec<Sheet>), String> {
    let fuente = captura::leer_captura(bytes)?;
    let calendarios: HashSet<_> = fuente
        .filas("consultas")
        .filter(|q| q.get("screen") == "calendario_informes")
        .map(|q| q.get("query_id"))
        .collect();
    let hay_seguimiento = fuente
        .filas("registros")
        .any(|r| !calendarios.contains(r.get("query_id")));
    let filas = fuente
        .filas("registros")
        .filter(|r| !hay_seguimiento || !calendarios.contains(r.get("query_id")));
    let mut grupos = HashMap::<Mode, Vec<Fila<'_>>>::new();
    let mut omitidos = 0;
    for (i, fila) in filas.enumerate() {
        if i >= LIMITE_ANALISIS {
            return Err(
                "La captura supera 20.000 registros de trabajo. Divide la exportación en lotes."
                    .into(),
            );
        }
        let modo = match fila.get("record_type") {
            "ESPERA" => Mode::Espera,
            "CUMPLIMIENTO" => Mode::Cumplimiento,
            "INFORMES" => Mode::Informes,
            _ => {
                omitidos += 1;
                continue;
            }
        };
        grupos.entry(modo).or_default().push(fila);
    }
    let mut modos = Vec::new();
    let mut hojas = Vec::new();
    let mut columnas_por_fila = Vec::new();
    for modo in Mode::ALL {
        if let Some(filas) = grupos.remove(&modo) {
            let (hoja, columnas) = hoja(modo, &filas)?;
            modos.push(modo);
            hojas.push(hoja);
            columnas_por_fila.push(columnas);
        }
    }
    if hojas.is_empty() {
        return Err(
            "El paquete no incluye registros de Espera, Cumplimiento o Informes para Trabajo."
                .into(),
        );
    }
    Ok((
        PaqueteTrabajo {
            contextos: HashMap::new(),
            fuente,
            modos,
            columnas_por_fila,
            omitidos,
        },
        hojas,
    ))
}

fn hoja(modo: Mode, filas: &[Fila<'_>]) -> Result<(Sheet, Vec<Cols>), String> {
    let columnas = lectura::columnas_canonicas(modo);
    let mut mapas = Vec::with_capacity(filas.len());
    for &fila in filas {
        let mapa: serde_json::Map<String, Value> =
            serde_json::from_str(fila.get("normalized_json"))
                .map_err(|_| "No se pudo leer un registro normalizado.")?;
        mapas.push(mapa);
    }
    // Las columnas canónicas van primero (el motor las busca por posición); el resto del registro normalizado
    // (identificadores SITFA, tipo, sexo, nacionalidad…) se conserva a continuación en el orden en que aparece.
    let reservados: HashSet<String> = columnas
        .iter()
        .map(|&(_, titulo)| titulo)
        .chain(PROCEDENCIA.iter().map(|&(titulo, _)| titulo))
        .map(|t| normalizar_titulo(t))
        .collect();
    let mut usados: HashSet<String> = HashSet::new();
    let mut extras: Vec<String> = Vec::new();
    for mapa in &mapas {
        let encabezados: Vec<_> = mapa.keys().cloned().collect();
        let origen = lectura::map_columns(&encabezados, modo);
        usados.extend(columnas.iter().filter_map(|&(clave, _)| origen.get(clave)).map(|&i| encabezados[i].clone()));
        for clave in &encabezados {
            if !usados.contains(clave) && !reservados.contains(&normalizar_titulo(clave)) && !extras.contains(clave) {
                extras.push(clave.clone());
            }
        }
    }
    // Una clave que otra fila sí usó como canónica no es una columna extra.
    extras.retain(|c| !usados.contains(c));
    let mut celdas = Vec::with_capacity(filas.len() + 1);
    celdas.push(
        columnas
            .iter()
            .map(|&(_, titulo)| Cell::Text(titulo.into()))
            .chain(PROCEDENCIA.iter().map(|&(titulo, _)| Cell::Text(titulo.into())))
            .chain(extras.iter().map(|c| Cell::Text(c.clone())))
            .collect(),
    );
    let mut columnas_por_fila = Vec::with_capacity(filas.len());
    for (&fila, mapa) in filas.iter().zip(&mapas) {
        let encabezados: Vec<_> = mapa.keys().cloned().collect();
        let origen = lectura::map_columns(&encabezados, modo);
        let mut disponibles = Cols::new();
        let mut valores = Vec::with_capacity(columnas.len() + PROCEDENCIA.len() + extras.len());
        for (i, &(clave, _)) in columnas.iter().enumerate() {
            let valor = origen.get(clave).map(|&idx| &mapa[&encabezados[idx]]);
            if valor.is_some() {
                disponibles.insert(clave, i);
            }
            valores.push(celda(valor));
        }
        valores.extend(
            PROCEDENCIA
                .iter()
                .map(|&(_, campo)| Cell::Text(fila.get(campo).into())),
        );
        valores.extend(extras.iter().map(|c| celda(mapa.get(c))));
        columnas_por_fila.push(disponibles);
        celdas.push(valores);
    }
    Ok((
        Sheet {
            name: modo.label().into(),
            rows: celdas,
        },
        columnas_por_fila,
    ))
}

fn normalizar_titulo(t: &str) -> String {
    t.nfd().filter(|c| !('\u{300}'..='\u{36f}').contains(c)).collect::<String>().to_uppercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn celda(valor: Option<&Value>) -> Cell {
    match valor {
        None | Some(Value::Null) => Cell::Empty,
        Some(Value::String(s)) => Cell::Text(s.clone()),
        Some(Value::Number(n)) => n.as_f64().map_or(Cell::Empty, Cell::Number),
        Some(Value::Bool(b)) => Cell::Bool(*b),
        Some(v) => Cell::Text(v.to_string()),
    }
}

/// Escritos de la causa leídos en el paquete (`revision_bitacoras.resumen_escritos`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResumenEscritos {
    pub cobertura: String,
    pub leido: String,
    pub filas: usize,
    /// «Tipo · documento» de cada escrito pendiente, sin los vinculados expresamente a otro ingreso.
    pub pendientes: Vec<String>,
    pub discordante: bool,
    pub(crate) datos: Arc<[EscritoCausa]>,
    pub(crate) documentos: Arc<DocumentosInforme>,
}

type DocumentosInforme = HashMap<String, HashSet<(String, NaiveDate)>>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EscritoCausa {
    pub(crate) tipo: String,
    pub(crate) estado: String,
    pub(crate) vinculo: Option<(String, Option<NaiveDate>)>,
}

pub(crate) const INGRESO_INFORME: &str = "ingreso informe de cumplimiento x";
pub(crate) const AMBULATORIO: &str = "inf cump programa ambulatorio";
pub(crate) const FAE: &str = "inf cump fae/fas";
pub(crate) const RESIDENCIAL: &str = "inf cump centro residencial";

#[derive(Default)]
pub(crate) struct EvidenciaInforme<'a> {
    pub(crate) escrito: Option<&'a str>,
    pub(crate) resolucion: Option<&'a str>,
    pub(crate) folios_ajenos: HashSet<&'a str>,
}

/// Ingreso Histórico Protección (Mejor Niñez) de la persona (`escritos.historicos_por_persona`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResumenHistorico {
    pub cobertura: String,
    pub leido: String,
    pub verificada: bool,
    /// Programas vigentes: fecha de egreso «--» y causal «SIN INFORMACION» en la misma fila.
    pub vigentes: Vec<String>,
    pub filas: usize,
    pub discordante: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextoFila {
    pub escritos: Option<ResumenEscritos>,
    pub historico: Option<ResumenHistorico>,
    pub historia: Option<ResumenLectura>,
    pub bitacora: Option<ResumenLectura>,
    pub calendario: Option<ResumenCalendario>,
    pub vencimiento_calendario_91: Option<NaiveDate>,
    pub cronologia: Option<Arc<cronologia::Cronologia>>,
    pub(crate) tramites: Option<Arc<[TramiteCausa]>>,
    pub(crate) varios_ingresos: bool,
    pub(crate) notas_centro: Arc<[NotaCentro]>,
    pub(crate) notas_persona: Arc<[Arc<[NotaCentro]>]>,
    pub(crate) historia_hasta: Option<NaiveDate>,
    pub(crate) ingreso_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TramiteCausa {
    pub(crate) fecha: NaiveDate,
    pub(crate) descripcion: String,
    pub(crate) folio: String,
    pub(crate) referencia: String,
    pub(crate) tipo: String,
    /// Último trámite anterior a la ventana, conservado por el paquete como contexto: sirve para saber si una
    /// situación sigue vigente, nunca para redactar una constancia nueva (`Historia.buscar(contexto=True)`).
    pub(crate) contexto: bool,
}

impl ContextoFila {
    /// Vincula Historia a una proyección por la referencia documental de Escritos.
    pub(crate) fn evidencia_informe(&self, vencimiento: NaiveDate) -> EvidenciaInforme<'_> {
        let mut evidencia = EvidenciaInforme::default();
        let Some(escritos) = &self.escritos else {
            return evidencia;
        };
        for tramite in self.tramites.as_deref().unwrap_or_default() {
            if tramite.contexto {
                continue;
            }
            let folio = tramite.folio.as_str();
            let Some(duenos) = escritos.documentos.get(&tramite.referencia) else {
                continue;
            };
            if folio.is_empty() {
                continue;
            }
            let propio = !escritos.discordante
                && duenos.len() == 1
                && duenos
                    .iter()
                    .any(|(ingreso, fecha)| ingreso == &self.ingreso_id && *fecha == vencimiento);
            if !propio {
                evidencia.folios_ajenos.insert(folio);
                continue;
            }
            match clave_texto(&tramite.descripcion).as_str() {
                INGRESO_INFORME => evidencia.resolucion = Some(folio),
                AMBULATORIO | FAE | RESIDENCIAL => evidencia.escrito = Some(folio),
                _ => {}
            }
        }
        for folio in evidencia.escrito.into_iter().chain(evidencia.resolucion) {
            evidencia.folios_ajenos.remove(folio);
        }
        evidencia
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NotaCentro {
    pub(crate) fecha: NaiveDate,
    pub(crate) texto: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumenLectura {
    pub cobertura: String,
    pub leido: String,
    pub filas: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumenCalendario {
    pub lectura: ResumenLectura,
    pub pendientes: Vec<NaiveDate>,
    proyecciones: usize,
}

impl ResumenCalendario {
    pub fn proxima_pendiente(&self, corte: NaiveDate) -> Option<NaiveDate> {
        self.pendientes.iter().copied().find(|&fecha| fecha > corte)
    }

    pub fn avisos_cotejo(
        &self,
        corte: NaiveDate,
        vencimiento_informes: Option<NaiveDate>,
        fecha_calendario_91: Option<NaiveDate>,
    ) -> Vec<String> {
        if self.proyecciones == 0 {
            return Vec::new();
        }
        let mut avisos = Vec::new();
        let completa = self.lectura.cobertura == "COMPLETA";
        let primera = self.pendientes.first().copied();
        let proxima = self.proxima_pendiente(corte);
        let fecha_es = crate::util::fecha_es;
        if let Some(informes) =
            vencimiento_informes.filter(|&f| Some(f) != primera && (primera.is_some() || completa))
        {
            let contraste = primera.map_or_else(
                || "no tiene informes pendientes".into(),
                |f| format!("proyecta el informe pendiente el {}", fecha_es(f)),
            );
            avisos.push(format!("CAL-03: la pestaña Informes indica vencimiento el {} y el calendario del ingreso {contraste}; revisar en RUS.", fecha_es(informes)));
        }
        if let Some(hoja2) =
            fecha_calendario_91.filter(|&f| Some(f) != proxima && (proxima.is_some() || completa))
        {
            let contraste = proxima.map_or_else(
                || "no tiene un próximo informe pendiente".into(),
                |f| format!("proyecta el próximo informe el {}", fecha_es(f)),
            );
            avisos.push(format!("CAL-03: Hoja2 (calendario 91) indica el {} y el calendario del ingreso {contraste}; revisar en RUS.", fecha_es(hoja2)));
        }
        avisos
    }
}

#[derive(Clone, Debug)]
struct Consulta {
    id: String,
    cobertura: String,
    leido: String,
    instante: chrono::NaiveDateTime,
    discordante: bool,
    lecturas_discordantes: Vec<String>,
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
}

impl Consulta {
    fn resumen(&self, filas: &[Fila<'_>]) -> ResumenLectura {
        ResumenLectura {
            cobertura: self.cobertura.clone(),
            leido: self.leido.clone(),
            filas: filas.len(),
        }
    }
}

pub(crate) fn clave_texto(t: &str) -> String {
    crate::util::normalizar(&t.replace('\u{a0}', " "))
}

fn rango(cobertura: &str) -> u8 {
    match cobertura {
        "COMPLETA" | "VACIA_COMPROBADA" => 3,
        "PARCIAL" => 2,
        _ => 0,
    }
}

/// Aplica el orden de `_mejores_consultas` y, para Escritos e Histórico, la comparación de
/// lecturas del mismo instante. Las demás pantallas muestran el origen del resumen.
fn mejores_consultas<const N: usize>(
    fuente: &Captura,
    pantalla: &str,
    filtro: &str,
    campos: [&str; N],
    corte: NaiveDate,
) -> HashMap<[String; N], Consulta> {
    let mut grupos = HashMap::<[String; N], Vec<Consulta>>::new();
    for q in fuente
        .filas("consultas")
        .filter(|q| q.get("screen") == pantalla)
    {
        let Ok(filtros) = serde_json::from_str::<Value>(q.get("filters_json")) else {
            continue;
        };
        let alcance = &filtros[filtro];
        let clave = campos.map(|k| alcance[k].as_str().unwrap_or("").to_string());
        let fecha = q.get("captured_at");
        let Ok(instante) = chrono::DateTime::parse_from_rfc3339(fecha) else {
            continue;
        };
        if instante.date_naive() > corte || clave.iter().any(String::is_empty) {
            continue;
        }
        let consulta = Consulta {
            id: q.get("query_id").into(),
            cobertura: q.get("coverage").into(),
            leido: instante.date_naive().to_string(),
            instante: instante.naive_utc(),
            discordante: false,
            lecturas_discordantes: Vec::new(),
            desde: filtros["ventana"]["desde"].as_str().and_then(dia_iso),
            hasta: filtros["ventana"]["hasta"].as_str().and_then(dia_iso),
        };
        grupos.entry(clave).or_default().push(consulta);
    }
    let firmas = match pantalla {
        "escritos" => Some(firmas_por_consulta(
            fuente,
            pantalla,
            &[
                "escrito_type",
                "escrito_status",
                "doc",
                "requester",
                "linked_ingreso_id",
                "linked_projection",
            ],
        )),
        "historico_proteccion" => Some(firmas_por_consulta(
            fuente,
            pantalla,
            &[
                "program_name",
                "egreso_date",
                "egreso_cause",
                "ownership",
                "person_id",
            ],
        )),
        "historia_tramites" => Some(firmas_por_consulta(
            fuente,
            pantalla,
            &[
                "event_at",
                "description",
                "folio",
                "reference",
                "raw_json",
                "scope",
                "identity_status",
            ],
        )),
        _ => None,
    };
    grupos
        .into_iter()
        .filter_map(|(clave, mut grupo)| {
            grupo.sort_unstable_by(|a, b| {
                rango(&b.cobertura)
                    .cmp(&rango(&a.cobertura))
                    .then_with(|| b.instante.cmp(&a.instante))
                    .then_with(|| a.id.cmp(&b.id))
            });
            let mut elegida = grupo.first()?.clone();
            if let Some(firmas) = &firmas {
                let simultaneas: Vec<_> = grupo
                    .iter()
                    .filter(|q| rango(&q.cobertura) > 0 && q.instante == elegida.instante)
                    .collect();
                let primera_parcial = simultaneas.iter().find(|q| q.cobertura == "PARCIAL");
                let compatibles = |a: &Consulta, b: &Consulta| {
                    let firma =
                        |q: &Consulta| firmas.get(q.id.as_str()).map(Vec::as_slice).unwrap_or(&[]);
                    firmas_compatibles(&a.cobertura, firma(a), &b.cobertura, firma(b))
                };
                elegida.discordante = simultaneas.iter().any(|q| {
                    !compatibles(&elegida, q)
                        || (q.cobertura == "PARCIAL"
                            && primera_parcial.is_some_and(|p| !compatibles(p, q)))
                });
                if elegida.discordante {
                    elegida.lecturas_discordantes =
                        simultaneas.iter().map(|q| q.id.clone()).collect();
                    elegida.cobertura = "PARCIAL".into();
                }
            }
            Some((clave, elegida))
        })
        .collect()
}

type Firma = Vec<String>;

fn firmas_por_consulta<'a>(
    fuente: &'a Captura,
    tabla: &str,
    campos: &[&str],
) -> HashMap<&'a str, Vec<Firma>> {
    let mut firmas: HashMap<_, Vec<_>> = HashMap::new();
    for fila in fuente.filas(tabla) {
        let firma = campos
            .iter()
            .map(|campo| {
                let valor = fila.get(campo);
                match *campo {
                    "linked_ingreso_id" | "linked_projection" | "person_id" => valor.to_string(),
                    "doc" | "requester" => valor.nfc().collect(),
                    _ => clave_texto(valor),
                }
            })
            .collect();
        firmas.entry(fila.get("query_id")).or_default().push(firma);
    }
    for filas in firmas.values_mut() {
        filas.sort_unstable();
    }
    firmas
}

fn firmas_compatibles(a: &str, firma_a: &[Firma], b: &str, firma_b: &[Firma]) -> bool {
    match (rango(a) == 3, rango(b) == 3) {
        (true, false) => firma_contenida(firma_b, firma_a),
        (false, true) => firma_contenida(firma_a, firma_b),
        _ => a == b && firma_a == firma_b,
    }
}

/// Cada posición del multiconjunto mayor se consume como máximo una vez.
fn firma_contenida(menor: &[Firma], mayor: &[Firma]) -> bool {
    let mut disponibles = mayor.iter();
    menor
        .iter()
        .all(|fila| disponibles.by_ref().any(|otra| fila == otra))
}

fn filas_por_consulta<'a>(fuente: &'a Captura, tabla: &str) -> HashMap<&'a str, Vec<Fila<'a>>> {
    let mut salida: HashMap<_, Vec<_>> = HashMap::new();
    for fila in fuente.filas(tabla) {
        salida.entry(fila.get("query_id")).or_default().push(fila);
    }
    salida
}

fn calendario_91(fuente: &Captura, corte: NaiveDate) -> HashMap<[String; 4], Option<NaiveDate>> {
    let consultas: HashSet<_> = fuente
        .filas("consultas")
        .filter(|q| q.get("screen") == "calendario_informes")
        .filter(|q| matches!(q.get("coverage"), "COMPLETA" | "PARCIAL"))
        .filter(|q| {
            chrono::DateTime::parse_from_rfc3339(q.get("captured_at"))
                .is_ok_and(|leida| leida.date_naive() <= corte)
        })
        .map(|q| q.get("query_id"))
        .collect();
    let mut fechas = HashMap::new();
    for f in fuente.filas("calendarios") {
        if !consultas.contains(f.get("query_id"))
            || f.get("association") != "VERIFICADA"
            || !matches!(f.get("event_type"), "INFORME" | "INFORMES")
        {
            continue;
        }
        let Some(fecha) = fecha_evento(f) else {
            continue;
        };
        if !(1..=VENTANA_INFORMES_DIAS).contains(&(fecha - corte).num_days()) {
            continue;
        }
        let clave = INGRESO.map(|campo| f.get(campo).to_string());
        if clave.iter().any(String::is_empty) {
            continue;
        }
        let guardada = fechas.entry(clave).or_insert(Some(fecha));
        if *guardada != Some(fecha) {
            *guardada = None;
        }
    }
    fechas
}

fn fecha_evento(f: Fila<'_>) -> Option<NaiveDate> {
    dia_iso(f.get("event_at"))
}

fn dia_iso(fecha: &str) -> Option<NaiveDate> {
    fecha.get(..10)?.parse().ok()
}

fn tramites_causa(
    q: &Consulta,
    filas: &[Fila<'_>],
    causa: &[String; 3],
    campos: [&str; 3],
    corte: NaiveDate,
) -> Arc<[TramiteCausa]> {
    // `Historia.al_corte`: una lectura cuya ventana empieza después del corte no acredita nada a esa fecha, ni
    // siquiera con su trámite de contexto.
    if q.discordante
        || !matches!(q.cobertura.as_str(), "COMPLETA" | "PARCIAL")
        || q.desde.is_some_and(|desde| corte < desde)
    {
        return Arc::default();
    }
    filas
        .iter()
        .filter_map(|&f| {
            let fecha = fecha_evento(f)?;
            let descripcion = f.get("description").trim();
            if f.get("scope") != "CAUSA"
                || f.get("identity_status") != "CAUSA"
                || campos.map(|campo| f.get(campo)) != causa.each_ref().map(String::as_str)
                || !dia_iso(f.get("captured_at")).is_some_and(|dia| dia <= corte)
                || fecha > q.hasta.unwrap_or(corte).min(corte)
                || descripcion.is_empty()
            {
                return None;
            }
            let tipo = serde_json::from_str::<Value>(f.get("raw_json"))
                .ok()
                .and_then(|v| v["Trámite"].as_str().map(clave_texto))
                .unwrap_or_default();
            Some(TramiteCausa {
                fecha,
                descripcion: descripcion.into(),
                folio: f.get("folio").trim().into(),
                referencia: clave_texto(f.get("reference")),
                tipo,
                contexto: q.desde.is_some_and(|desde| fecha < desde),
            })
        })
        .collect()
}

fn contextos(fuente: &Captura, corte: NaiveDate) -> HashMap<String, ContextoFila> {
    const CAUSA: [&str; 3] = ["source_system", "court_code", "cause_id"];
    const PERSONA: [&str; 3] = ["source_system", "court_code", "person_id"];
    let escritos_por_consulta = filas_por_consulta(fuente, "escritos");
    let historico_por_consulta = filas_por_consulta(fuente, "historico_proteccion");
    let historia_por_consulta = filas_por_consulta(fuente, "historia_tramites");
    let bitacora_por_consulta = filas_por_consulta(fuente, "bitacoras");
    let calendario_por_consulta = filas_por_consulta(fuente, "calendarios");
    let escritos = mejores_consultas(fuente, "escritos", "escritos_causa", CAUSA, corte);
    let historicos = mejores_consultas(
        fuente,
        "historico_proteccion",
        "historico_persona",
        PERSONA,
        corte,
    );
    let historias = mejores_consultas(fuente, "historia_tramites", "historia_causa", CAUSA, corte);
    let bitacoras = mejores_consultas(fuente, "bitacoras", "remote_identity", INGRESO, corte);
    let calendarios = mejores_consultas(
        fuente,
        "calendario_ingreso",
        "remote_identity",
        INGRESO,
        corte,
    );
    let fechas_calendario_91 = calendario_91(fuente, corte);
    let revision = Revision::al_corte(fuente, corte);
    let solicitada = fuente.estado("bitacoras") != "NO_SOLICITADA";
    let mut eventos_bitacora: HashMap<_, Vec<_>> = HashMap::new();
    let mut notas_por_ingreso: HashMap<_, Vec<_>> = HashMap::new();
    for evento in fuente.filas("bitacoras").filter(|f| {
        f.get("identity_status") == "VERIFICADA" && matches!(f.get("scope"), "" | "INGRESO")
    }) {
        let clave = INGRESO.map(|campo| evento.get(campo));
        eventos_bitacora.entry(clave).or_default().push(evento);
        if let Some(fecha) = fecha_evento(evento).filter(|fecha| *fecha <= corte) {
            if dia_iso(evento.get("captured_at")).is_some_and(|dia| dia <= corte)
                && revision.acredita_centro(evento)
            {
                notas_por_ingreso
                    .entry(clave)
                    .or_default()
                    .push(NotaCentro {
                        fecha,
                        texto: evento.get("text").into(),
                    });
            }
        }
    }
    let notas_por_ingreso: HashMap<_, Arc<[NotaCentro]>> = notas_por_ingreso
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect();
    let verificados = || {
        fuente
            .filas("registros")
            .filter(|r| r.get("identity_status") == "VERIFICADA")
    };
    let mut ingresos_por_causa = HashMap::<_, HashSet<_>>::new();
    let mut ingresos_por_persona = HashMap::<_, HashSet<_>>::new();
    let mut personas_por_ingreso = HashMap::<_, HashSet<_>>::new();
    for r in verificados() {
        let causa = CAUSA.map(|c| r.get(c));
        let ingreso = INGRESO.map(|c| r.get(c));
        let persona = PERSONA.map(|c| r.get(c));
        if ingreso.iter().all(|s| !s.is_empty()) {
            ingresos_por_causa
                .entry(causa)
                .or_default()
                .insert(r.get("ingreso_id"));
            personas_por_ingreso
                .entry(ingreso)
                .or_default()
                .insert(persona);
            if persona.iter().all(|s| !s.is_empty()) {
                ingresos_por_persona
                    .entry(persona)
                    .or_default()
                    .insert(ingreso);
            }
        }
    }
    let notas_por_persona: HashMap<_, Arc<[Arc<[NotaCentro]>]>> = ingresos_por_persona
        .into_iter()
        .map(|(persona, ingresos)| {
            let notas = ingresos
                .into_iter()
                .filter(|i| personas_por_ingreso.get(i).is_some_and(|ps| ps.len() == 1))
                .filter_map(|i| notas_por_ingreso.get(&i).cloned())
                .collect();
            (persona, notas)
        })
        .collect();
    let mut tramites_por_causa = HashMap::new();
    let mut datos_escritos_por_causa = HashMap::new();
    let mut documentos_por_causa = HashMap::new();
    let mut cronologias: HashMap<_, Arc<cronologia::Cronologia>> = HashMap::new();
    let mut salida = HashMap::new();
    for r in verificados() {
        let causa = CAUSA.map(|campo| r.get(campo).to_string());
        let persona = PERSONA.map(|campo| r.get(campo).to_string());
        let ingreso = INGRESO.map(|campo| r.get(campo).to_string());
        let mut ctx = ContextoFila {
            vencimiento_calendario_91: fechas_calendario_91.get(&ingreso).copied().flatten(),
            varios_ingresos: ingresos_por_causa
                .get(&causa.each_ref().map(String::as_str))
                .is_some_and(|ingresos| ingresos.len() > 1),
            notas_centro: notas_por_ingreso
                .get(&ingreso.each_ref().map(String::as_str))
                .cloned()
                .unwrap_or_default(),
            notas_persona: notas_por_persona
                .get(&persona.each_ref().map(String::as_str))
                .cloned()
                .unwrap_or_default(),
            ..Default::default()
        };
        if let Some(q) = escritos.get(&causa) {
            let filas = escritos_por_consulta
                .get(q.id.as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let pendientes = filas
                .iter()
                .filter(|_| {
                    !q.discordante && matches!(q.cobertura.as_str(), "COMPLETA" | "PARCIAL")
                })
                .filter(|e| clave_texto(e.get("escrito_status")) == "pendiente")
                .filter(|e| {
                    e.get("linked_ingreso_id").is_empty()
                        || e.get("linked_ingreso_id") == r.get("ingreso_id")
                })
                .map(|e| format!("{} · {}", e.get("escrito_type"), e.get("doc")))
                .collect();
            let datos = datos_escritos_por_causa
                .entry(causa.clone())
                .or_insert_with(|| {
                    filas
                        .iter()
                        .filter(|f| {
                            f.get("identity_status") == "CAUSA" && f.get("scope") == "CAUSA"
                        })
                        .map(|f| EscritoCausa {
                            tipo: clave_texto(f.get("escrito_type")),
                            estado: clave_texto(f.get("escrito_status")),
                            vinculo: (!f.get("linked_ingreso_id").is_empty()).then(|| {
                                (
                                    f.get("linked_ingreso_id").into(),
                                    dia_iso(f.get("linked_projection")),
                                )
                            }),
                        })
                        .collect::<Arc<[_]>>()
                });
            let documentos = documentos_por_causa
                .entry(causa.clone())
                .or_insert_with(|| {
                    let consultas = if q.discordante {
                        q.lecturas_discordantes.as_slice()
                    } else if matches!(q.cobertura.as_str(), "COMPLETA" | "PARCIAL") {
                        std::slice::from_ref(&q.id)
                    } else {
                        &[]
                    };
                    let mut documentos = DocumentosInforme::new();
                    for f in consultas
                        .iter()
                        .filter_map(|id| escritos_por_consulta.get(id.as_str()))
                        .flatten()
                    {
                        if f.get("identity_status") != "CAUSA" || f.get("scope") != "CAUSA" {
                            continue;
                        }
                        let documento = clave_texto(f.get("doc"));
                        let ingreso = f.get("linked_ingreso_id");
                        if let Some(fecha) = dia_iso(f.get("linked_projection"))
                            .filter(|_| !documento.is_empty() && !ingreso.is_empty())
                        {
                            documentos
                                .entry(documento)
                                .or_default()
                                .insert((ingreso.into(), fecha));
                        }
                    }
                    Arc::new(documentos)
                });
            ctx.escritos = Some(ResumenEscritos {
                cobertura: q.cobertura.clone(),
                leido: q.leido.clone(),
                filas: filas.len(),
                pendientes,
                discordante: q.discordante,
                datos: Arc::clone(datos),
                documentos: Arc::clone(documentos),
            });
            ctx.ingreso_id = r.get("ingreso_id").into();
        }
        if let Some(q) = historicos.get(&persona) {
            let filas = historico_por_consulta
                .get(q.id.as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let verificada = !q.discordante
                && matches!(
                    q.cobertura.as_str(),
                    "COMPLETA" | "PARCIAL" | "VACIA_COMPROBADA"
                )
                && filas.iter().all(|f| f.get("ownership") == "VERIFICADA");
            let mut vigentes: Vec<String> = Vec::new();
            for f in filas.iter().filter(|_| verificada) {
                let fecha = f.get("egreso_date").trim();
                let sin_fecha = fecha.chars().count() == 2
                    && fecha
                        .chars()
                        .all(|c| "-\u{2010}\u{2011}\u{2012}\u{2013}\u{2014}\u{2212}".contains(c));
                let nombre = f
                    .get("program_name")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                if sin_fecha
                    && !nombre.is_empty()
                    && clave_texto(f.get("egreso_cause")) == "sin informacion"
                    && !vigentes.contains(&nombre)
                {
                    vigentes.push(nombre);
                }
            }
            ctx.historico = Some(ResumenHistorico {
                cobertura: q.cobertura.clone(),
                leido: q.leido.clone(),
                verificada,
                vigentes,
                filas: filas.len(),
                discordante: q.discordante,
            });
        }
        if let Some(q) = historias.get(&causa) {
            let filas = historia_por_consulta
                .get(q.id.as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            ctx.historia = Some(q.resumen(filas));
            ctx.historia_hasta = q.hasta.map(|hasta| hasta.min(corte));
            ctx.tramites = Some(Arc::clone(
                tramites_por_causa
                    .entry(causa.clone())
                    .or_insert_with(|| tramites_causa(q, filas, &causa, CAUSA, corte)),
            ));
        }
        ctx.bitacora = bitacoras.get(&ingreso).map(|q| {
            q.resumen(
                bitacora_por_consulta
                    .get(q.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            )
        });
        if solicitada {
            let clave = INGRESO.map(|campo| r.get(campo));
            let cronologia = cronologias.entry(clave).or_insert_with(|| {
                let filas = eventos_bitacora
                    .get(&clave)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let leida = ctx.bitacora.as_ref().is_some_and(|r| {
                    matches!(
                        r.cobertura.as_str(),
                        "COMPLETA" | "PARCIAL" | "VACIA_COMPROBADA"
                    )
                });
                Arc::new(cronologia::leer(filas, &revision, corte, leida))
            });
            ctx.cronologia = Some(Arc::clone(cronologia));
        }
        if let Some(q) = calendarios.get(&ingreso) {
            let filas = calendario_por_consulta
                .get(q.id.as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let proyecciones: Vec<_> = filas
                .iter()
                .filter(|_| matches!(q.cobertura.as_str(), "COMPLETA" | "PARCIAL"))
                .filter(|f| {
                    f.get("association") == "VERIFICADA"
                        && f.get("event_type") == "INFORME_PROYECTADO"
                })
                .filter_map(|f| fecha_evento(*f).map(|fecha| (fecha, f.get("status"))))
                .collect();
            let mut pendientes: Vec<_> = proyecciones
                .iter()
                .filter(|(_, estado)| *estado == "PENDIENTE")
                .map(|(fecha, _)| *fecha)
                .collect();
            pendientes.sort_unstable();
            pendientes.dedup();
            ctx.calendario = Some(ResumenCalendario {
                lectura: q.resumen(filas),
                pendientes,
                proyecciones: proyecciones.len(),
            });
        }
        if ctx != ContextoFila::default() {
            salida.insert(r.get("record_id").to_string(), ctx);
        }
    }
    salida
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requiere FLOW_E3_PAQUETE"]
    fn trabajo_abre_contexto_e3_del_exportador_python() {
        let ruta = crate::prueba_entorno::requerida("FLOW_E3_PAQUETE");
        let ruta = std::path::PathBuf::from(ruta);
        let bytes = std::fs::read(&ruta).unwrap();
        let (mut paquete, hojas) = abrir(&bytes).unwrap();
        paquete.actualizar_contextos("2026-10-19".parse().unwrap());
        assert_eq!(paquete.modos, [Mode::Cumplimiento]);
        assert_eq!(hojas[0].rows.len(), 3);
        let fila = |ingreso| {
            let registro = paquete
                .fuente
                .filas("registros")
                .find(|r| r.get("ingreso_id") == ingreso)
                .unwrap();
            &paquete.contextos[registro.get("record_id")]
        };
        let primera = fila("201");
        let segunda = fila("202");
        let escritos = primera.escritos.as_ref().unwrap();
        assert_eq!(escritos.cobertura, "COMPLETA");
        assert_eq!(escritos.filas, 2);
        assert_eq!(
            escritos.pendientes,
            ["Inf. Cump Programa Ambulatorio · DOC-1"]
        );
        assert!(segunda.escritos.is_none());
        let historico = primera.historico.as_ref().unwrap();
        assert!(historico.verificada);
        assert_eq!(historico.vigentes, ["PRM Ficticio", "FAE Otro Ficticio"]);
        assert_eq!(historico.filas, 3);
        assert_eq!(segunda.historico.as_ref(), Some(historico));
        let historia = primera.historia.as_ref().unwrap();
        assert_eq!(historia.filas, 3);
        assert!(segunda.historia.is_none());
        assert_eq!(primera.bitacora.as_ref().unwrap().filas, 1);
        assert_eq!(segunda.bitacora.as_ref().unwrap().filas, 1);
        let cronologia = primera.cronologia.as_ref().unwrap();
        assert_eq!(cronologia.estado, super::cronologia::Estado::Acreditada);
        assert_eq!(
            cronologia.ultimas[0].fecha,
            "2026-08-12".parse::<NaiveDate>().unwrap()
        );
        let calendario = primera.calendario.as_ref().unwrap();
        assert_eq!(calendario.lectura.filas, 2);
        assert_eq!(
            calendario.pendientes,
            ["2026-11-01".parse::<chrono::NaiveDate>().unwrap()]
        );
        assert!(segunda.calendario.is_none());
        paquete.actualizar_contextos("2026-10-12".parse().unwrap());
        let primera = paquete
            .fuente
            .filas("registros")
            .find(|r| r.get("ingreso_id") == "201")
            .unwrap();
        let contexto = &paquete.contextos[primera.get("record_id")];
        assert!(contexto.bitacora.is_some());
        assert!(contexto.calendario.is_none());
        assert!(contexto.historia.is_none());
        assert!(contexto.escritos.is_none());
        assert!(contexto.historico.is_none());
        paquete.actualizar_contextos("2026-10-07".parse().unwrap());
        let primera = paquete
            .fuente
            .filas("registros")
            .find(|r| r.get("ingreso_id") == "201")
            .unwrap();
        let contexto = &paquete.contextos[primera.get("record_id")];
        assert_eq!(
            contexto.cronologia.as_ref().unwrap().estado,
            super::cronologia::Estado::NoLeida
        );

        let bytes = std::fs::read(ruta.with_file_name("contexto-e3-discordante.zip")).unwrap();
        let (mut discordante, _) = abrir(&bytes).unwrap();
        discordante.actualizar_contextos("2026-10-19".parse().unwrap());
        let registro = discordante
            .fuente
            .filas("registros")
            .find(|r| r.get("ingreso_id") == "201")
            .unwrap();
        let contexto = &discordante.contextos[registro.get("record_id")];
        let escritos = contexto.escritos.as_ref().unwrap();
        assert!(escritos.discordante);
        assert_eq!(escritos.cobertura, "PARCIAL");
        assert!(escritos.pendientes.is_empty());
        let evidencia = contexto.evidencia_informe("2026-11-01".parse().unwrap());
        assert_eq!(evidencia.resolucion, None);
        assert_eq!(evidencia.folios_ajenos, HashSet::from(["9119", "9120"]));
        let historico = contexto.historico.as_ref().unwrap();
        assert!(historico.discordante);
        assert!(!historico.verificada);
        assert!(historico.vigentes.is_empty());
    }

    #[test]
    fn lecturas_en_el_mismo_instante_comparan_multiplicidad_y_cobertura() {
        let registro = vec!["ficticio".to_string()];
        let uno = vec![registro.clone()];
        let dos = vec![registro.clone(), registro];
        assert!(firmas_compatibles("COMPLETA", &dos, "PARCIAL", &uno));
        assert!(!firmas_compatibles("COMPLETA", &uno, "PARCIAL", &dos));
        assert!(!firmas_compatibles("COMPLETA", &dos, "COMPLETA", &uno));
        assert!(!firmas_compatibles("PARCIAL", &dos, "PARCIAL", &uno));
        assert!(!firmas_compatibles(
            "COMPLETA",
            &[],
            "VACIA_COMPROBADA",
            &[]
        ));
    }

    #[test]
    fn a1_tramites_de_contexto() {
        let causa = ["SITFA".to_string(), "113".to_string(), "100".to_string()];
        let campos = ["source_system", "court_code", "cause_id"];

        let consulta = Consulta {
            id: "q1".into(),
            cobertura: "COMPLETA".into(),
            leido: "2026-10-15".into(),
            instante: "2026-10-15T10:00:00".parse().unwrap(),
            discordante: false,
            lecturas_discordantes: Vec::new(),
            desde: Some("2026-10-01".parse().unwrap()),
            hasta: Some("2026-10-31".parse().unwrap()),
        };

        let t1_json = serde_json::json!({"Trámite": "Resolución"}).to_string();
        let mut cols = HashMap::new();
        for (i, c) in [
            "source_system", "court_code", "cause_id", "scope", "identity_status",
            "captured_at", "event_at", "description", "folio", "reference", "raw_json",
        ].iter().enumerate() {
            cols.insert(c.to_string(), i);
        }

        let r1 = csv::StringRecord::from(vec![
            "SITFA", "113", "100", "CAUSA", "CAUSA",
            "2026-10-08T10:00:00Z", "2026-09-15T00:00:00Z",
            "Ingreso Informe de Cumplimiento X", "1", "DOC-1", &t1_json,
        ]);
        let r2 = csv::StringRecord::from(vec![
            "SITFA", "113", "100", "CAUSA", "CAUSA",
            "2026-10-08T10:00:00Z", "2026-10-05T00:00:00Z",
            "Ingreso Informe de Cumplimiento X", "2", "DOC-2", &t1_json,
        ]);

        let f1 = Fila::nueva(&cols, &r1);
        let f2 = Fila::nueva(&cols, &r2);
        let filas = [f1, f2];

        // 1. Un trámite anterior a desde llega con contexto = true, y en ventana con contexto = false
        let corte = "2026-10-10".parse::<NaiveDate>().unwrap();
        let res = tramites_causa(&consulta, &filas, &causa, campos, corte);
        assert_eq!(res.len(), 2);
        assert!(res[0].contexto, "Trámite anterior a desde debe tener contexto = true");
        assert!(!res[1].contexto, "Trámite dentro de la ventana debe tener contexto = false");

        // 2. Una lectura con desde posterior al corte no entrega trámites
        let consulta_futura = Consulta {
            id: "q2".into(),
            cobertura: "COMPLETA".into(),
            leido: "2026-10-25".into(),
            instante: "2026-10-25T10:00:00".parse().unwrap(),
            discordante: false,
            lecturas_discordantes: Vec::new(),
            desde: Some("2026-10-20".parse().unwrap()),
            hasta: Some("2026-10-31".parse().unwrap()),
        };
        let res_vacia = tramites_causa(&consulta_futura, &filas, &causa, campos, corte);
        assert!(res_vacia.is_empty(), "Lectura con desde > corte no debe entregar trámites");

        // 3. evidencia_informe ignora los trámites de contexto
        let mut documentos = DocumentosInforme::new();
        let vencimiento = "2026-11-01".parse::<NaiveDate>().unwrap();
        documentos.insert(clave_texto("DOC-1"), HashSet::from([("ing1".into(), vencimiento)]));
        documentos.insert(clave_texto("DOC-2"), HashSet::from([("ing1".into(), vencimiento)]));

        let escritos = ResumenEscritos {
            cobertura: "COMPLETA".into(),
            leido: "2026-10-15".into(),
            filas: 2,
            pendientes: Vec::new(),
            discordante: false,
            datos: Arc::from([]),
            documentos: Arc::new(documentos),
        };

        let contexto_fila = ContextoFila {
            tramites: Some(res),
            escritos: Some(escritos),
            ingreso_id: "ing1".into(),
            ..Default::default()
        };
        let ev = contexto_fila.evidencia_informe(vencimiento);
        // DOC-1 (trámite con contexto = true, folio 1) se ignora; solo se toma DOC-2 (folio 2)
        assert_eq!(ev.resolucion, Some("2"));
    }
}
