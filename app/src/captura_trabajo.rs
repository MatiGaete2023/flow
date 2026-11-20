//! Adapta las filas normalizadas del paquete a las hojas de Trabajo.

use crate::lectura::{self, Sheet};
use crate::motor::{Cols, Mode};
use crate::util::Cell;
use chrono::NaiveDate;
use flow_rust::captura::{self, Captura, Fila};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

const LIMITE_ANALISIS: usize = 20_000;
const PROCEDENCIA: [(&str, &str); 5] = [
    ("SITFA_ARCHIVO", "original_file"),
    ("SITFA_HOJA", "original_sheet"),
    ("SITFA_FILA", "original_row"),
    ("SITFA_SHA256", "source_sha256"),
    ("ID_REGISTRO_CAPTURA", "record_id"),
];

pub struct PaqueteTrabajo {
    pub fuente: Captura,
    pub modos: Vec<Mode>,
    pub columnas_por_fila: Vec<Vec<Cols>>,
    pub omitidos: usize,
    /// Fuentes de lectura por `record_id` del paquete.
    pub contextos: HashMap<String, ContextoFila>,
}

impl PaqueteTrabajo {
    pub fn actualizar_contextos(&mut self, corte: NaiveDate) {
        self.contextos = contextos(&self.fuente, corte);
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
    let mut celdas = Vec::with_capacity(filas.len() + 1);
    celdas.push(
        columnas
            .iter()
            .map(|&(_, titulo)| Cell::Text(titulo.into()))
            .chain(
                PROCEDENCIA
                    .iter()
                    .map(|&(titulo, _)| Cell::Text(titulo.into())),
            )
            .collect(),
    );
    let mut columnas_por_fila = Vec::with_capacity(filas.len());
    for &fila in filas {
        let mapa: serde_json::Map<String, Value> =
            serde_json::from_str(fila.get("normalized_json"))
                .map_err(|_| "No se pudo leer un registro normalizado.")?;
        let encabezados: Vec<_> = mapa.keys().cloned().collect();
        let origen = lectura::map_columns(&encabezados, modo);
        let mut disponibles = Cols::new();
        let mut valores = Vec::with_capacity(columnas.len() + PROCEDENCIA.len());
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
}

impl ResumenCalendario {
    pub fn proxima_pendiente(&self, corte: NaiveDate) -> Option<NaiveDate> {
        self.pendientes.iter().copied().find(|&fecha| fecha > corte)
    }
}

#[derive(Clone, Debug)]
struct Consulta {
    id: String,
    cobertura: String,
    leido: String,
    instante: chrono::NaiveDateTime,
    discordante: bool,
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

fn clave_texto(t: &str) -> String {
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

fn contextos(fuente: &Captura, corte: NaiveDate) -> HashMap<String, ContextoFila> {
    const CAUSA: [&str; 3] = ["source_system", "court_code", "cause_id"];
    const PERSONA: [&str; 3] = ["source_system", "court_code", "person_id"];
    const INGRESO: [&str; 4] = ["source_system", "court_code", "cause_id", "ingreso_id"];
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
    let mut salida = HashMap::new();
    for r in fuente.filas("registros") {
        if r.get("identity_status") != "VERIFICADA" {
            continue;
        }
        let causa = CAUSA.map(|campo| r.get(campo).to_string());
        let persona = PERSONA.map(|campo| r.get(campo).to_string());
        let ingreso = INGRESO.map(|campo| r.get(campo).to_string());
        let mut ctx = ContextoFila::default();
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
            ctx.escritos = Some(ResumenEscritos {
                cobertura: q.cobertura.clone(),
                leido: q.leido.clone(),
                filas: filas.len(),
                pendientes,
                discordante: q.discordante,
            });
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
                let sin_fecha = f
                    .get("egreso_date")
                    .trim()
                    .trim_matches(|c: char| {
                        "-\u{2010}\u{2011}\u{2012}\u{2013}\u{2014}\u{2212}".contains(c)
                    })
                    .is_empty();
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
        ctx.historia = historias.get(&causa).map(|q| {
            q.resumen(
                historia_por_consulta
                    .get(q.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            )
        });
        ctx.bitacora = bitacoras.get(&ingreso).map(|q| {
            q.resumen(
                bitacora_por_consulta
                    .get(q.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            )
        });
        if let Some(q) = calendarios.get(&ingreso) {
            let filas = calendario_por_consulta
                .get(q.id.as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mut pendientes: Vec<_> = filas
                .iter()
                .filter(|_| matches!(q.cobertura.as_str(), "COMPLETA" | "PARCIAL"))
                .filter(|f| {
                    f.get("association") == "VERIFICADA"
                        && f.get("event_type") == "INFORME_PROYECTADO"
                })
                .filter(|f| f.get("status") == "PENDIENTE")
                .filter_map(|f| f.get("event_at").parse().ok())
                .collect();
            pendientes.sort_unstable();
            pendientes.dedup();
            ctx.calendario = Some(ResumenCalendario {
                lectura: q.resumen(filas),
                pendientes,
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
    fn trabajo_abre_contexto_e3_del_exportador_python() {
        let Some(ruta) = std::env::var_os("FLOW_E3_PAQUETE") else {
            return;
        };
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
}
