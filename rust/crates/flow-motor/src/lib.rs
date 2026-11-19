//! Motor de reglas CSMP: de un registro de RUS a la observación propuesta.
//! Sin E/S y sin reloj: la fecha de corte entra como parámetro.

pub mod ctx;
pub mod hist;
pub mod reglas;

pub use ctx::{Config, Duplicidad, Incidencia, Marca, OtroPrograma};

use chrono::{Duration, NaiveDate};
use ctx::Ctx;
use flow_dominio::texto::{clave, fecha_es, fecha_mayoria, identidad, rut_clave};
use flow_dominio::{Escritos, Etapa, EvidenciaInforme, Historia, Historico, Observacion, Registro, Tribunal};
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
    /// Escritos de la causa (bandeja de la causa), Ingreso Histórico Protección de la persona y asociación explícita
    /// del informe con su evidencia (plan 18-nov-2026). `None` = fuente no aportada.
    pub escritos: Option<&'a Escritos>,
    pub historico: Option<&'a Historico>,
    pub evidencia: Option<&'a EvidenciaInforme>,
    /// Claves de programa que el funcionario ya consignó para este niño.
    pub consignados: &'a [String],
    /// La bitácora puede estar truncada: no acredita ausencia de una constancia previa.
    pub bitacora_parcial: bool,
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
    /// Otros programas vigentes cuya constancia propuso el motor.
    pub otros_programas: Vec<OtroPrograma>,
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
        if let Some(a) =
            audiencias.into_iter().find(|a| e.corte <= *a && *a < venc && venc <= *a + Duration::days(VENTANA_INFORME_AUDIENCIA))
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
        // Una lectura posterior al corte no existía al corte: no decide nada (E3-06).
        escritos: e.escritos.filter(|x| x.leido.is_none_or(|l| l <= e.corte)).cloned(),
        historico: e.historico.filter(|x| x.leido.is_none_or(|l| l <= e.corte)).cloned(),
        consignados: e.consignados.iter().cloned().collect(),
        evidencia: e.evidencia.cloned(),
        bitacora_parcial: e.bitacora_parcial,
        otros_programas: Vec::new(),
    };
    let mut avisos = Vec::new();
    if e.etapa == Etapa::Cumplimiento {
        let negativo = |campo: &str| r.celda(campo).and_then(|c| c.entero_laxo()).is_some_and(|n| n < 0);
        if r.fecha("egreso_proy").is_some_and(|f| f > e.corte) && (negativo("dias_cumpl") || negativo("dias_egresar")) {
            avisos.push("Días negativos y egreso futuro: verifica la contradicción; no se afirma que la medida esté vencida.".to_string());
            ctx.invalidas.push("CUMPLIMIENTO.C04_VENCIDA");
        }
    }
    let observacion = match Tribunal::desde(&r.texto("tribunal")) {
        None => {
            avisos.push("Tribunal fuera de Laja, Mulchén y Tomé: sin reglas configuradas; requiere revisión manual.".to_string());
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
        otros_programas: std::mem::take(&mut ctx.otros_programas),
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
    causas_con_varios_por_ingreso(registros, &vec![String::new(); registros.len()])
}

/// Como `causas_con_varios`, con el identificador de ingreso de SITFA de cada fila (vacío si no viene de un paquete):
/// dos ingresos de la misma persona al mismo programa en una causa no se distinguen por la historia, que es de la causa.
pub fn causas_con_varios_por_ingreso(registros: &[Registro], ingresos: &[String]) -> Vec<Option<Duplicidad>> {
    let mut causas: HashMap<(String, String), BTreeSet<(String, String, String)>> = HashMap::new();
    let mut de_fila = vec![None; registros.len()];
    for (i, r) in registros.iter().enumerate() {
        let rit = clave(r.texto("rit").trim());
        if rit.is_empty() {
            continue;
        }
        let causa = (clave(r.texto("tribunal").trim()), rit);
        let ingreso = ingresos.get(i).map(|s| s.trim().to_string()).unwrap_or_default();
        causas.entry(causa.clone()).or_default().insert((rut_clave(r.texto("rut").trim()), clave(r.texto("programa").trim()), ingreso));
        de_fila[i] = Some(causa);
    }
    de_fila.into_iter().map(|c| c.filter(|causa| causas[causa].len() > 1).map(|_| Duplicidad::OtroRegistro)).collect()
}

/// Pre-pase de Informes: informes vencidos del mismo ingreso → (cantidad, vencimiento más antiguo).
pub fn acumulados(registros: &[Registro], corte: NaiveDate) -> Vec<Option<(i64, NaiveDate)>> {
    acumulados_por_ingreso(registros, &vec![String::new(); registros.len()], corte)
}

/// Como `acumulados`, con el identificador de ingreso de SITFA de cada fila (vacío si no viene de un paquete): separa
/// dos ingresos del mismo NNA al mismo programa. Cuenta vencimientos distintos, no filas: una fila repetida no es otro
/// informe.
pub fn acumulados_por_ingreso(registros: &[Registro], ingresos: &[String], corte: NaiveDate) -> Vec<Option<(i64, NaiveDate)>> {
    let mut grupos: HashMap<[String; 5], Vec<(usize, NaiveDate)>> = HashMap::new();
    for (i, r) in registros.iter().enumerate() {
        let Some(vence) = r.fecha("vencimiento").filter(|v| *v < corte) else { continue };
        let [tribunal, rit, rut, programa] = ["tribunal", "rit", "rut", "programa"].map(|c| identidad(r.texto(c).trim()));
        let ingreso = ingresos.get(i).map(|s| s.trim().to_string()).unwrap_or_default();
        if !rit.is_empty() {
            grupos.entry([tribunal, rit, rut, programa, ingreso]).or_default().push((i, vence));
        }
    }
    let mut salida = vec![None; registros.len()];
    for items in grupos.values() {
        let vencimientos: BTreeSet<NaiveDate> = items.iter().map(|(_, v)| *v).collect();
        if vencimientos.len() > 1 {
            let mas_antiguo = *vencimientos.first().unwrap();
            for (i, _) in items {
                salida[*i] = Some((vencimientos.len() as i64, mas_antiguo));
            }
        }
    }
    salida
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use flow_dominio::Celda;

    fn informe(rit: &str, vence: &str) -> Registro {
        let mut r = Registro::default();
        for (campo, valor) in [("tribunal", "Laja"), ("rit", rit), ("rut", "11111111-K"), ("programa", "PRM Laja"), ("vencimiento", vence)]
        {
            r.poner(campo, Celda::Texto(valor.into()));
        }
        r
    }

    /// `work.py` (11-nov-2026): se cuentan vencimientos distintos, no filas, y el ingreso de SITFA separa grupos.
    #[test]
    fn acumulados_cuentan_vencimientos_distintos_por_ingreso() {
        let corte: NaiveDate = "2026-11-12".parse().unwrap();
        let antiguo: NaiveDate = "2026-09-01".parse().unwrap();
        let filas = [
            informe("X-1-2099", "2026-10-01"),
            informe("X-1-2099", "2026-10-01"),
            informe("X-2-2099", "2026-10-01"),
            informe("X-2-2099", "2026-09-01"),
            informe("X-2-2099", "2026-12-01"),
            informe("X-3-2099", "2026-10-01"),
            informe("X-3-2099", "2026-09-01"),
        ];
        let ingresos: Vec<String> = ["", "", "", "", "", "201", "202"].map(String::from).to_vec();
        let salida = acumulados_por_ingreso(&filas, &ingresos, corte);
        assert_eq!(salida[0], None, "una fila repetida no es otro informe");
        assert_eq!(salida[1], None);
        assert_eq!(salida[2], Some((2, antiguo)));
        assert_eq!(salida[3], Some((2, antiguo)));
        assert_eq!(salida[4], None, "un informe por vencer no se acumula");
        assert_eq!(salida[5], None, "ingresos distintos no se acumulan");
        assert_eq!(salida[6], None);
        assert_eq!(acumulados(&filas, corte)[5], Some((2, antiguo)));
    }
}
