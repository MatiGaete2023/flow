//! Motor de reglas CSMP: de un registro de RUS a la observación propuesta.
//! Sin E/S y sin reloj: la fecha de corte entra como parámetro.

pub mod ctx;
pub mod hist;
pub mod reglas;

pub use ctx::{Config, Duplicidad, Incidencia, Marca};

use chrono::{Duration, NaiveDate};
use ctx::Ctx;
use flow_dominio::texto::{clave, fecha_es, fecha_mayoria, identidad, rut_clave};
use flow_dominio::{Etapa, Historia, Observacion, Registro, Tribunal};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Ventana del correo preventivo de informe por vencer (cruce de Cumplimiento).
const VENTANA_POR_VENCER_DIAS: i64 = 60;
/// Informe que vence poco después de una audiencia: se sugiere pedirlo antes.
const VENTANA_INFORME_AUDIENCIA: i64 = 30;

/// Todo lo que el motor necesita para una fila.
#[derive(Debug, Clone)]
pub struct Entrada<'a> {
    pub etapa: Etapa,
    pub registro: &'a Registro,
    pub corte: NaiveDate,
    pub config: &'a Config,
    pub historia: Option<&'a Historia>,
    pub bitacora: &'a [Observacion],
    pub duplicidad: Option<Duplicidad>,
    /// Informes vencidos acumulados del mismo ingreso: (cantidad, vencimiento más antiguo).
    pub acumulado: Option<(i64, NaiveDate)>,
    /// Próximo informe según el cruce (hoja 2 o calendario). Solo Cumplimiento.
    pub fecha_hoja2: Option<NaiveDate>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Resultado {
    pub observacion: String,
    /// Reglas aplicadas («MODO.CLAVE»), sin repetir, en orden.
    pub reglas: Vec<String>,
    /// correo: programa_espera, programa_vencido, programa_por_vencer, medidas. proyecto: PC_IE, PC_INFO, NOMENCL.
    pub acciones: Vec<String>,
    pub avisos: Vec<String>,
    pub marcas: Vec<Marca>,
    /// Derivación que el Centro no sigue: no genera acciones.
    pub excluido: bool,
}

fn sin_repetir<T: Clone + Eq + std::hash::Hash>(lista: &[T]) -> Vec<T> {
    let mut vistos = HashSet::new();
    lista.iter().filter(|x| vistos.insert((*x).clone())).cloned().collect()
}

/// Acciones según las reglas aplicadas, nunca según palabras de la observación.
pub fn acciones(reglas: &[String], proximo_informe: Option<NaiveDate>, corte: NaiveDate) -> Vec<String> {
    let tiene = |f: &dyn Fn(&str) -> bool| reglas.iter().any(|e| f(e));
    let mut out: Vec<String> = Vec::new();
    if tiene(&|e| e.ends_with("E05_SOLO_CORREO") || e.ends_with("E05_PROYECTO_Y_CORREO")) {
        out.push("programa_espera".into());
    }
    if tiene(&|e| e == "ESPERA.E05_PROYECTO_Y_CORREO") {
        out.push("PC_IE".into());
    }
    if tiene(&|e| e.starts_with("INFORMES.I01_")) {
        out.push("programa_vencido".into());
        out.push("PC_INFO".into());
    }
    // I02 por vencer solo genera correo preventivo; nunca PC_INFO.
    if tiene(&|e| e.starts_with("INFORMES.I02_")) {
        out.push("programa_por_vencer".into());
    }
    if tiene(&|e| e.starts_with("CUMPLIMIENTO.C04_") || e.starts_with("CUMPLIMIENTO.C05_")) {
        out.push("medidas".into());
    }
    if tiene(&|e| e == "CUMPLIMIENTO.C10_HOJA2")
        && !out.iter().any(|a| a == "programa_por_vencer")
        && proximo_informe.is_none_or(|v| (v - corte).num_days() <= VENTANA_POR_VENCER_DIAS)
    {
        out.push("programa_por_vencer".into());
    }
    if tiene(&|e| e == "HISTORIA.C10_ESCRITO" || e == "HISTORIA.P_FINALIZADA") {
        out.push("NOMENCL".into());
    }
    out
}

/// Avisos solo mostrar (N-01, N-02): no cambian la observación ni las acciones.
fn avisos_fila(e: &Entrada) -> Vec<String> {
    let r = e.registro;
    let mut out = Vec::new();
    if e.etapa == Etapa::Cumplimiento {
        if let (Some(nac), Some(egreso)) = (r.fecha("nacimiento"), r.fecha("egreso_proy")) {
            let mayoria = fecha_mayoria(nac);
            if e.corte < mayoria && mayoria < egreso {
                out.push(format!(
                    "N-01: la medida se proyecta hasta el {}, posterior a la mayoría de edad ({}).",
                    fecha_es(egreso),
                    fecha_es(mayoria)
                ));
            }
        }
    }
    let vencimiento = if e.etapa == Etapa::Informes { r.fecha("vencimiento") } else { e.fecha_hoja2 };
    if let (Some(venc), Some(celda)) = (vencimiento, r.celda("prox_aud")) {
        let mut audiencias = celda.fechas_varias().0;
        audiencias.sort_unstable();
        audiencias.dedup();
        if let Some(a) = audiencias
            .into_iter()
            .find(|a| e.corte <= *a && *a < venc && venc <= *a + Duration::days(VENTANA_INFORME_AUDIENCIA))
        {
            out.push(format!(
                "N-02: audiencia el {} y el informe vence el {}; se sugiere solicitarlo antes de la audiencia.",
                fecha_es(a),
                fecha_es(venc)
            ));
        }
    }
    out
}

/// Analiza una fila. Sin tribunal reconocido no hay observación: solo un aviso.
pub fn analizar(e: &Entrada) -> Resultado {
    let r = e.registro;
    let mut bitacora: Vec<Observacion> = e.bitacora.iter().filter(|o| o.fecha <= e.corte).cloned().collect();
    bitacora.sort_by_key(|o| o.fecha);
    let mut ctx = Ctx {
        cfg: e.config,
        hoy: e.corte,
        eventos: Vec::new(),
        invalidas: Vec::new(),
        historia: e.historia.map(|h| h.al_corte(e.corte)),
        bitacora,
        marcas: Vec::new(),
        duplicidad: e.duplicidad,
        acumulado: e.acumulado,
        emitidos: HashSet::new(),
        incidencias: Vec::new(),
    };
    let mut avisos = Vec::new();
    if e.etapa == Etapa::Cumplimiento {
        let negativo = |campo: &str| r.celda(campo).and_then(|c| c.entero_laxo()).is_some_and(|n| n < 0);
        if r.fecha("egreso_proy").is_some_and(|f| f > e.corte) && (negativo("dias_cumpl") || negativo("dias_egresar")) {
            avisos.push(
                "Días negativos y egreso futuro: verifica la contradicción; no se afirma que la medida esté vencida."
                    .to_string(),
            );
            ctx.invalidas.push("CUMPLIMIENTO.C04_VENCIDA");
        }
    }
    let observacion = match Tribunal::desde(&r.texto("tribunal")) {
        None => {
            avisos.push(
                "Tribunal fuera de Laja, Mulchén y Tomé: sin reglas configuradas; requiere revisión manual.".to_string(),
            );
            String::new()
        }
        Some(tribunal) => match e.etapa {
            Etapa::Espera => reglas::espera(&mut ctx, r, tribunal),
            Etapa::Cumplimiento => reglas::cumplimiento(&mut ctx, r, e.fecha_hoja2),
            Etapa::Informes => reglas::informes(&mut ctx, r),
        },
    };
    avisos.extend(ctx.incidencias.iter().map(|i| i.motivo.clone()));
    let eventos: Vec<String> = ctx.eventos.iter().filter(|x| !ctx.desactivada(x)).cloned().collect();
    let excluido = eventos.iter().any(|x| x == "COMUN.NO_SEGUIMIENTO");
    if !observacion.is_empty() && !ctx.eventos.iter().any(|x| x == "COMUN.NO_SEGUIMIENTO") {
        avisos.extend(avisos_fila(e));
    }
    Resultado {
        observacion,
        acciones: if excluido { Vec::new() } else { acciones(&eventos, e.fecha_hoja2, e.corte) },
        reglas: sin_repetir(&eventos),
        avisos,
        marcas: sin_repetir(&ctx.marcas),
        excluido,
    }
}

/// Pre-pase de Espera y Cumplimiento: causas con más de un registro en la nómina.
/// `MismoPrograma`: el mismo NNA (RUT) tiene otro registro en el mismo programa. Sin RUT no se afirma.
pub fn duplicidades(registros: &[Registro]) -> Vec<Option<Duplicidad>> {
    let mut causas: HashMap<(String, String), Vec<(usize, String, String)>> = HashMap::new();
    for (i, r) in registros.iter().enumerate() {
        let rit = clave(r.texto("rit").trim());
        if rit.is_empty() {
            continue;
        }
        let llave = (clave(r.texto("tribunal").trim()), rit);
        causas.entry(llave).or_default().push((i, rut_clave(r.texto("rut").trim()), clave(r.texto("programa").trim())));
    }
    let mut salida = vec![None; registros.len()];
    for items in causas.values().filter(|v| v.len() > 1) {
        for (i, rut, programa) in items {
            let repetido = !rut.is_empty() && items.iter().filter(|(_, r2, p2)| r2 == rut && p2 == programa).count() > 1;
            salida[*i] = Some(if repetido { Duplicidad::MismoPrograma } else { Duplicidad::OtroRegistro });
        }
    }
    salida
}

/// Pre-pase de Informes (cada fila es un informe): la causa tiene más de un niño o programa en la nómina.
pub fn causas_con_varios(registros: &[Registro]) -> Vec<Option<Duplicidad>> {
    let mut causas: HashMap<(String, String), BTreeSet<(String, String)>> = HashMap::new();
    let mut de_fila = vec![None; registros.len()];
    for (i, r) in registros.iter().enumerate() {
        let rit = clave(r.texto("rit").trim());
        if rit.is_empty() {
            continue;
        }
        let causa = (clave(r.texto("tribunal").trim()), rit);
        causas.entry(causa.clone()).or_default().insert((rut_clave(r.texto("rut").trim()), clave(r.texto("programa").trim())));
        de_fila[i] = Some(causa);
    }
    de_fila
        .into_iter()
        .map(|c| c.filter(|causa| causas[causa].len() > 1).map(|_| Duplicidad::OtroRegistro))
        .collect()
}

/// Pre-pase de Informes: informes vencidos del mismo ingreso → (cantidad, vencimiento más antiguo).
pub fn acumulados(registros: &[Registro], corte: NaiveDate) -> Vec<Option<(i64, NaiveDate)>> {
    let mut grupos: HashMap<[String; 4], Vec<(usize, NaiveDate)>> = HashMap::new();
    for (i, r) in registros.iter().enumerate() {
        let Some(vence) = r.fecha("vencimiento").filter(|v| *v < corte) else { continue };
        let ident = ["tribunal", "rit", "rut", "programa"].map(|c| identidad(r.texto(c).trim()));
        if !ident[1].is_empty() {
            grupos.entry(ident).or_default().push((i, vence));
        }
    }
    let mut salida = vec![None; registros.len()];
    for items in grupos.values().filter(|v| v.len() > 1) {
        let mas_antiguo = items.iter().map(|(_, v)| *v).min().unwrap();
        for (i, _) in items {
            salida[*i] = Some((items.len() as i64, mas_antiguo));
        }
    }
    salida
}
