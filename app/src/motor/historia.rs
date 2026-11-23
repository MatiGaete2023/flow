//! Reglas que requieren evidencia acreditada del paquete de Trabajo.

use super::{Ctx, Frag};
use crate::captura_trabajo::{clave_texto, ContextoFila, NotaCentro, TramiteCausa};
use crate::util::{fecha_es, limpiar_nombre, normalizar};
use chrono::{Days, NaiveDate};
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

mod escritos;
pub(super) use escritos::{
    escrito_sin_ingreso, informe_atribuible, informe_historia, informe_pendiente, informe_previo,
};

const SERVICIO: &str = "ing serv proteccion especializada y otras redes";
const INGRESO_EFECTIVO: &str = "ingreso efectivo";
const PRORROGA: &str = "prorroga";
const CIERRES: [&str; 2] = ["conciliacion", "dictacion de sentencia"];
const EGRESO_SERVICIO: [&str; 1] = ["egreso de servicio proteccion especializada y otras redes"];
const DESIGNA: [&str; 1] = ["designa abogado"];
const BAJA_CURADOR: [&str; 1] = ["deja sin efecto designacion c ad litem"];
const CURADOR: [&str; 2] = [DESIGNA[0], BAJA_CURADOR[0]];

/// Materias de `HISTORIA.REITERA`: regla original, clave de la materia y patrón de la observación previa.
#[derive(Clone, Copy)]
pub(super) enum Materia {
    Curador,
    SinFicha,
    SinFae,
}

impl Materia {
    fn original(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Materia::Curador => ("COMUN.CURADOR", "HISTORIA.MATERIA_CURADOR", "no registra curador ad litem"),
            Materia::SinFicha => ("CUMPLIMIENTO.C07_SIN_FICHA", "HISTORIA.MATERIA_SIN_FICHA", "no registra ficha individual"),
            Materia::SinFae => ("CUMPLIMIENTO.C08_FICHA_FAE", "HISTORIA.MATERIA_SIN_FAE", "no registra ficha fae"),
        }
    }
}

fn tramites<'a>(
    captura: &'a ContextoFila,
    nombre: &'static str,
) -> impl Iterator<Item = &'a TramiteCausa> {
    captura
        .tramites
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter(move |t| !t.contexto && clave_texto(&t.descripcion) == nombre)
}

/// `Historia.buscar(*nombres, contexto=True)`: incluye el trámite anterior a la ventana.
fn tramites_con_contexto<'a>(
    captura: &'a ContextoFila,
    nombres: &'a [&'static str],
) -> impl Iterator<Item = &'a TramiteCausa> {
    captura
        .tramites
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter(move |t| nombres.contains(&clave_texto(&t.descripcion).as_str()))
}

pub(super) fn advertir_ingreso_efectivo(ctx: &mut Ctx, espera: Option<i64>) {
    let Some(captura) = ctx.captura else { return };
    let desde = espera.filter(|d| *d >= 0).map(|d| {
        ctx.hoy
            .checked_sub_days(Days::new(d as u64))
            .unwrap_or(NaiveDate::MIN)
    });
    let efectivo =
        tramites(captura, INGRESO_EFECTIVO).any(|t| desde.is_none_or(|fecha| t.fecha >= fecha));
    if efectivo {
        ctx.marcar(
            2,
            "R3_INGRESO_EFECTIVO",
            "Historia con ingreso efectivo y RUS en Espera",
        );
    }
    if captura.varios_ingresos && (efectivo || tramites(captura, SERVICIO).next().is_some()) {
        ctx.aviso("HISTORIA.R3_MULTI: la historia es de la causa y no permite atribuir el trámite a un ingreso determinado.");
    }
}

fn folio_resolucion(captura: &ContextoFila, fecha: NaiveDate) -> Option<&str> {
    if captura.varios_ingresos {
        return None;
    }
    let mut folios = tramites(captura, SERVICIO)
        .filter(|t| t.fecha == fecha && !t.folio.is_empty())
        .map(|t| t.folio.as_str());
    let primero = folios.next()?;
    (primero.bytes().all(|c| c.is_ascii_digit()) && folios.all(|folio| folio == primero))
        .then_some(primero)
}

pub(super) fn resolucion(ctx: &mut Ctx, programa: &str, fecha: NaiveDate) -> Option<Frag> {
    let mut vars = vec![
        ("PROGRAMA", programa.to_owned()),
        ("FECHA_RESOLUCION", fecha_es(fecha)),
    ];
    if let Some(folio) = ctx.captura.and_then(|c| folio_resolucion(c, fecha)) {
        vars.push(("FOLIO", folio.into()));
        return ctx.render("HISTORIA.E04_CON_FOLIO", &vars);
    }
    ctx.render("ESPERA.E04_RESOLUCION_RECIENTE", &vars)
}

fn novedad(t: &TramiteCausa, notas: &[NotaCentro]) -> bool {
    if notas.iter().any(|n| n.fecha > t.fecha) {
        return false;
    }
    static FOLIO: OnceLock<Regex> = OnceLock::new();
    let folio = FOLIO.get_or_init(|| Regex::new(r"\bfolio (?:no? )?(\d+)\b").unwrap());
    t.folio.is_empty()
        || !notas.iter().filter(|n| n.fecha == t.fecha).any(|n| {
            folio
                .captures_iter(&normalizar(&n.texto))
                .any(|c| c[1] == t.folio)
        })
}

pub(super) fn servicio_ingreso(ctx: &mut Ctx, programa: &str) -> Option<Frag> {
    let captura = ctx.captura.filter(|c| !c.varios_ingresos)?;
    let t = tramites(captura, SERVICIO)
        .filter(|t| novedad(t, &captura.notas_centro))
        .max_by_key(|t| (t.fecha, t.folio.parse::<u64>().ok()))?;
    let salida = ctx.render(
        "HISTORIA.SERVICIO_INGRESO",
        &[
            ("PROGRAMA", programa.to_owned()),
            (
                "FOLIO",
                if t.folio.is_empty() {
                    "s/f".into()
                } else {
                    t.folio.clone()
                },
            ),
            ("FECHA", fecha_es(t.fecha)),
        ],
    )?;
    Some(cita(salida, t))
}

/// R2: la historia es de la causa; con otro registro o ingreso no se atribuye a esta fila.
fn causa_compartida(ctx: &Ctx, captura: &ContextoFila) -> bool {
    captura.varios_ingresos || ctx.duplicidad.is_some()
}

fn ultimo<'a>(lista: impl Iterator<Item = &'a TramiteCausa>) -> Option<&'a TramiteCausa> {
    lista.max_by_key(|t| (t.fecha, t.folio.parse::<u64>().ok()))
}

fn cita_folio(texto: &str, folio: &str) -> bool {
    static FOLIO: OnceLock<Regex> = OnceLock::new();
    let re = FOLIO.get_or_init(|| Regex::new(r"\bfolio (?:no? )?(\d+)").unwrap());
    re.captures_iter(texto).any(|c| &c[1] == folio)
}

/// `_en_bitacora`: una observación del centro, del día del trámite o posterior, ya habló de la materia y citó su
/// folio o su fecha completa.
fn en_bitacora(t: &TramiteCausa, notas: &[NotaCentro], materia: &str) -> bool {
    let fecha = clave_texto(&fecha_es(t.fecha));
    notas.iter().filter(|n| n.fecha >= t.fecha).any(|n| {
        let texto = clave_texto(&n.texto);
        texto.contains(materia)
            && (texto.contains(&fecha) || (!t.folio.is_empty() && cita_folio(&texto, &t.folio)))
    })
}

/// `constancia([t])`: constancia neutra de un trámite, nombrado según la columna «Trámite».
fn constancia_tramite(ctx: &mut Ctx, t: &TramiteCausa) -> Option<Frag> {
    let folio = if t.folio.is_empty() { "s/f".into() } else { t.folio.clone() };
    let frag = ctx.render(
        "HISTORIA.BASE_TRAMITE",
        &[
            ("DESC", t.descripcion.clone()),
            ("FECHA", fecha_es(t.fecha)),
            ("FOLIO", folio),
        ],
    )?;
    Some(cita(frag, t))
}

/// `analisis_historia.materias`: por oración; una oración que ya reitera no es la observación original.
fn trata_materia(texto: &str, patron: &str) -> bool {
    static ORACION: OnceLock<Regex> = OnceLock::new();
    let oracion = ORACION.get_or_init(|| Regex::new(r"\.\s+").unwrap());
    oracion
        .split(texto)
        .map(clave_texto)
        .any(|o| !o.contains("se reitera") && o.contains(patron))
}

/// `reglas_historia.reitera`: la sugerencia de la primera observación del centro sobre la materia, anterior al
/// corte, se reitera en una frase autosuficiente de gestión del Tribunal. Con la regla original desactivada no se
/// reitera; tampoco el curador si la causa registra un trámite de curador posterior a esa observación.
pub(super) fn reitera(ctx: &mut Ctx, materia: Materia) -> Option<Frag> {
    let (original, clave, patron) = materia.original();
    let captura = ctx.captura?;
    if ctx.cfg.desactivadas.contains(original) {
        return None;
    }
    let previa = captura
        .notas_centro
        .iter()
        .filter(|n| n.fecha < ctx.hoy && trata_materia(&n.texto, patron))
        .min_by_key(|n| n.fecha)?;
    if matches!(materia, Materia::Curador)
        && tramites_con_contexto(captura, &CURADOR).any(|t| t.fecha > previa.fecha)
    {
        return None;
    }
    let materia_txt = ctx.render(clave, &[])?.partes.into_iter().map(|(t, _)| t).collect::<Vec<_>>().join(" ");
    let mut frag = ctx.render(
        "HISTORIA.REITERA",
        &[("MATERIA", materia_txt), ("FECHA_ANTERIOR", fecha_es(previa.fecha))],
    )?;
    ctx.eventos.push(original.into());
    for (_, grupo) in &mut frag.partes {
        *grupo = 'T';
    }
    Some(frag)
}

/// `analisis_historia.posterior`: por folio si ambos son numéricos; si no, por fecha.
fn posterior(a: &TramiteCausa, b: &TramiteCausa) -> bool {
    match (a.folio.parse::<u64>(), b.folio.parse::<u64>()) {
        (Ok(x), Ok(y)) => x > y,
        _ => a.fecha > b.fecha,
    }
}

/// `_ultimo_curador`: último trámite de curador de la causa, incluido el de contexto, y si es la baja.
fn ultimo_curador(captura: &ContextoFila) -> Option<(&TramiteCausa, bool)> {
    let designa = ultimo(tramites_con_contexto(captura, &DESIGNA));
    let baja = ultimo(tramites_con_contexto(captura, &BAJA_CURADOR));
    match (designa, baja) {
        (None, None) => None,
        (Some(d), None) => Some((d, false)),
        (None, Some(b)) => Some((b, true)),
        (Some(d), Some(b)) if posterior(d, b) => Some((d, false)),
        (_, Some(b)) => Some((b, true)),
    }
}

/// `curador_sin_efecto`: RUS sin curador y lo último de la causa es la baja de la designación (causa de un registro).
pub(super) fn curador_sin_efecto(ctx: &Ctx, rus_sin_curador: bool) -> bool {
    let Some(captura) = ctx.captura else { return false };
    rus_sin_curador
        && !causa_compartida(ctx, captura)
        && ultimo_curador(captura).is_some_and(|(_, baja)| baja)
}

/// `curador_designaciones`: con RUS sin curador y causa de un registro, cita el último trámite de curador de la
/// ventana con su nombre literal; la nomenclatura no acredita que sea curador ni de quién.
pub(super) fn curador_designaciones(ctx: &mut Ctx, rus_sin_curador: bool) -> Option<Frag> {
    let captura = ctx.captura?;
    if !rus_sin_curador || causa_compartida(ctx, captura) {
        return None;
    }
    let (ult, baja) = ultimo_curador(captura)?;
    if ult.contexto || !novedad(ult, &captura.notas_centro) {
        return None;
    }
    ctx.marcar(
        2,
        if baja { "CURADOR_SIN_EFECTO" } else { "CURADOR_DESIGNADO" },
        "Curador ad litem en la historia y RUS sin curador registrado",
    );
    constancia_tramite(ctx, ult)
}

/// `reglas_historia.causa_finalizada`: causa P con «Conciliacion» o «Dictación de sentencia» sin «Egreso de servicio…»
/// en esa fecha o después. El cierre puede ser el trámite de contexto anterior a la ventana: la regla no caduca
/// cuando la sentencia sale de ella.
pub(super) fn causa_finalizada(ctx: &mut Ctx, rit: &str) -> Option<Frag> {
    if !rit.trim().to_uppercase().starts_with('P') {
        return None;
    }
    let captura = ctx.captura?;
    let cierre = ultimo(tramites_con_contexto(captura, &CIERRES))?;
    if tramites_con_contexto(captura, &EGRESO_SERVICIO).any(|t| t.fecha >= cierre.fecha) {
        return None;
    }
    ctx.render("HISTORIA.P_FINALIZADA", &[])
}

/// `reglas_historia.prorroga` (Cumplimiento, 11-nov-2026). La historia acredita el trámite, no su término: se cita
/// como trámite y el egreso de RUS se dice aparte. Con la medida vencida, por vencer o con el egreso de RUS anterior a
/// la prórroga, marca para revisión si es reciente (`prorroga_reciente`) y solo se dice en causas de un registro.
/// Una mención previa en la bitácora suprime frase y marca. R2: también cuenta como causa compartida que la causa
/// tenga varios ingresos en el paquete.
pub(super) fn prorroga(
    ctx: &mut Ctx,
    egreso: Option<NaiveDate>,
    vencida: bool,
    por_vencer: bool,
    egreso_dicho: bool,
) -> Vec<Frag> {
    let (Some(captura), Some(egreso)) = (ctx.captura, egreso) else {
        return Vec::new();
    };
    let Some(ult) = ultimo(tramites(captura, PRORROGA)) else {
        return Vec::new();
    };
    if en_bitacora(ult, &captura.notas_centro, "prorrog") {
        return Vec::new();
    }
    let reciente = (ctx.hoy - ult.fecha).num_days() <= ctx.p("prorroga_reciente");
    let pendiente = vencida || por_vencer || egreso < ult.fecha;
    if reciente && pendiente {
        ctx.marcar(
            1,
            "PRORROGA_REVISAR",
            "Prórroga reciente con medida vencida o por vencer: verificar su registro en RUS",
        );
    }
    let compartida = causa_compartida(ctx, captura);
    if !novedad(ult, &captura.notas_centro) || (pendiente && (!reciente || compartida)) {
        return Vec::new();
    }
    let mut salida: Vec<Frag> = constancia_tramite(ctx, ult).into_iter().collect();
    if !pendiente && !egreso_dicho && !compartida {
        salida.extend(ctx.render(
            "HISTORIA.EGRESO_PROYECTADO",
            &[("FECHA_EGRESO_PROYECTADO", fecha_es(egreso))],
        ));
    }
    salida
}

fn cita(mut frag: Frag, tramite: &TramiteCausa) -> Frag {
    let denominacion = match tramite.tipo.as_str() {
        "escrito" => "en escrito de fecha",
        "actuacion" => "en actuación de fecha",
        _ => return frag,
    };
    for (texto, _) in &mut frag.partes {
        *texto = texto.replacen("en resolución de fecha", denominacion, 1);
    }
    frag
}

fn sujeto(prefijo: &str) -> Vec<String> {
    static TOKENS: OnceLock<Regex> = OnceLock::new();
    const CONECTORES: &str = "que se hace presente informa informo indica indico senala senalo comunica refiere menciona consta da cuenta el la los las un una nino nina nna adolescente joven menor y e o de del al a con por para en segun ya actualmente hoy tambien asimismo programa";
    const PARTICULAS: [&str; 6] = ["de", "del", "la", "las", "los", "san"];
    let tokens = TOKENS.get_or_init(|| Regex::new(r"\p{L}[\p{L}\p{M}]*|\d+|[^\s]").unwrap());
    let mut palabras: Vec<_> = tokens.find_iter(prefijo).map(|t| t.as_str()).collect();
    if palabras.last() == Some(&",") {
        palabras.pop();
    }
    let es_palabra = |t: &str| t.chars().next().is_some_and(char::is_alphabetic);
    let es_conector = |t: &str| CONECTORES.split_whitespace().any(|p| p == t);
    let mut salida = Vec::new();
    for (i, palabra) in palabras.iter().enumerate().rev().take(6) {
        if !es_palabra(palabra) {
            break;
        }
        let clave = clave_texto(palabra);
        if es_conector(&clave) {
            let entre_nombres = i
                .checked_sub(1)
                .and_then(|p| palabras.get(p))
                .filter(|p| es_palabra(p))
                .is_some_and(|p| {
                    let p = clave_texto(p);
                    !es_conector(&p) || PARTICULAS.contains(&p.as_str())
                });
            if salida.is_empty() || !PARTICULAS.contains(&clave.as_str()) || !entre_nombres {
                break;
            }
        }
        salida.push(clave);
    }
    salida.reverse();
    salida
}

fn constancia(texto: &str, nombre: &[&str], programa: &str) -> bool {
    static CONSTANCIA: OnceLock<Regex> = OnceLock::new();
    static NEGACION: OnceLock<Regex> = OnceLock::new();
    let constancia = CONSTANCIA.get_or_init(|| Regex::new(
        r"(?i)\b(?:cuenta\s+con\s+)?ingres(?:o\s+efectivo|[oó]\s+efectivamente)\s+(?:al|a)\s+(?:programa\s+)?([^.;:\n]+)",
    ).unwrap());
    let negacion = NEGACION.get_or_init(|| Regex::new(r"\b(?:no|nunca|ni|sin)\b.{0,40}$").unwrap());
    let texto: String = texto.nfc().collect();
    texto.split(['.', '\n']).any(|oracion| {
        constancia.captures_iter(oracion).any(|hallazgo| {
            let previo = &oracion[..hallazgo.get(0).unwrap().start()];
            if clave_texto(&hallazgo[1]) != programa || negacion.is_match(&clave_texto(previo)) {
                return false;
            }
            let sujeto = sujeto(previo);
            !sujeto.is_empty()
                && nombre.starts_with(&sujeto.iter().map(String::as_str).collect::<Vec<_>>())
        })
    })
}

fn mencion(texto: &str, programa: &str) -> bool {
    let texto = clave_texto(texto);
    texto.match_indices(programa).any(|(i, coincidencia)| {
        let es_palabra = |c: char| c.is_alphanumeric() || c == '_';
        !texto[..i].chars().next_back().is_some_and(es_palabra)
            && !texto[i + coincidencia.len()..]
                .chars()
                .next()
                .is_some_and(es_palabra)
    })
}

pub(super) fn otro_programa(ctx: &mut Ctx, actual: &str, primero: &str, nombre: &str) -> Vec<Frag> {
    let Some(captura) = ctx.captura else {
        return Vec::new();
    };
    let Some(historico) = &captura.historico else {
        return Vec::new();
    };
    if !historico.verificada {
        match historico.cobertura.as_str() {
            "FALLIDA" => ctx.marcar(3, "HISTORICO_NO_DISPONIBLE", "Ingreso Histórico Protección no se pudo leer: no se verificó si hay otro programa vigente"),
            "COMPLETA" | "PARCIAL" | "IDENTIDAD_INCIERTA" => ctx.marcar(2, "HISTORICO_IDENTIDAD", "Ingreso Histórico Protección sin pertenencia verificada a esta persona: no se afirma otro programa"),
            _ => {}
        }
        return Vec::new();
    }
    let parcial = captura
        .bitacora
        .as_ref()
        .is_some_and(|b| !matches!(b.cobertura.as_str(), "COMPLETA" | "VACIA_COMPROBADA"));
    let actual = clave_texto(actual);
    let nombre = clave_texto(&limpiar_nombre(nombre));
    let nombre: Vec<_> = nombre.split_whitespace().collect();
    let mut vistos = HashSet::new();
    let mut salida = Vec::new();
    for otro in &historico.vigentes {
        let clave = clave_texto(otro);
        if clave.is_empty() || clave == actual || !vistos.insert(clave.clone()) {
            continue;
        }
        if clave_texto(primero).is_empty() {
            ctx.marcar(2, "OTRO_PROGRAMA_SIN_NOMBRE", "Otro programa vigente en el histórico, pero no se pudo determinar el primer nombre: revisar esta fila");
            break;
        }
        let notas = captura
            .notas_persona
            .iter()
            .flat_map(|ingreso| ingreso.iter());
        if notas.clone().any(|n| constancia(&n.texto, &nombre, &clave)) {
            continue;
        }
        if parcial {
            ctx.marcar(2, "OTRO_PROGRAMA_BITACORA_PARCIAL", &format!("Bitácora incompleta: no se puede descartar que el ingreso efectivo a «{otro}» ya se haya consignado"));
        } else if notas.clone().any(|n| mencion(&n.texto, &clave)) {
            ctx.marcar(3, "OTRO_PROGRAMA_MENCION_PREVIA", &format!("La bitácora menciona «{otro}» sin una constancia reconocible: revisar si ya se consignó"));
        }
        salida.extend(ctx.render(
            "HISTORIA.OTRO_PROGRAMA",
            &[("PNOMBRE", primero.into()), ("OTRO_PROGRAMA", otro.clone())],
        ));
    }
    salida
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motor::{acciones, evaluar_con_captura, Fila, Mode, Prepaso, Resultado};
    use crate::{config::Config, util::Cell};

    fn evaluar_espera(captura: &ContextoFila, resolucion: Cell) -> Resultado {
        let cols = [
            ("tribunal", 0),
            ("programa", 1),
            ("nombre", 2),
            ("espera", 3),
            ("resolucion", 4),
        ]
        .into();
        let values = [
            Cell::Text("Juzgado de Letras de Laja".into()),
            Cell::Text("DCE Ficticio".into()),
            Cell::Text("Persona Ficticia".into()),
            Cell::Number(40.0),
            resolucion,
        ];
        evaluar_con_captura(
            &Config::default(),
            Mode::Espera,
            &Fila {
                values: &values,
                cols: &cols,
            },
            "2026-10-19".parse().unwrap(),
            (&Prepaso::default(), 0),
            None,
            Some(captura),
        )
    }

    #[test]
    fn espera_con_folio_y_correo_dice_el_encabezado_una_vez_r3() {
        let fecha = "2026-10-13".parse().unwrap();
        let tramite = TramiteCausa {
            fecha,
            descripcion: "Ing.Serv. protección especializada y otras redes".into(),
            folio: "42".into(),
            referencia: String::new(),
            tipo: "resolucion".into(),
            contexto: false,
        };
        let mut captura = ContextoFila {
            tramites: Some(vec![tramite.clone(), tramite.clone()].into()),
            ..Default::default()
        };
        let r = evaluar_espera(&captura, Cell::Date(fecha));
        assert_eq!(r.observacion, "Persona DCE: Medida revisada, a la espera de ingreso efectivo. Ingreso al programa DCE Ficticio ordenado en resolución de fecha 13 de octubre de 2026 (folio 42). Se remite correo al programa consultando fecha estimada de ingreso.");
        assert_eq!(
            r.eventos,
            ["HISTORIA.E04_CON_FOLIO", "ESPERA.E05_SOLO_CORREO"]
        );
        assert_eq!(
            acciones(&r.eventos, None, "2026-10-19".parse().unwrap()),
            ["programa_espera"]
        );
        let conflicting = TramiteCausa {
            folio: "43".into(),
            ..tramite.clone()
        };
        captura.tramites = Some(vec![tramite.clone(), conflicting].into());
        let r = evaluar_espera(&captura, Cell::Date(fecha));
        assert_eq!(r.eventos[0], "ESPERA.E04_RESOLUCION_RECIENTE");
        captura.tramites = Some(
            vec![TramiteCausa {
                descripcion: "Cita a audiencia".into(),
                ..tramite
            }]
            .into(),
        );
        let r = evaluar_espera(&captura, Cell::Date(fecha));
        assert!(!r.observacion.contains("folio"));
    }

    #[test]
    fn espera_con_servicio_y_causa_compartida_aplica_r2_y_r4() {
        let fecha = "2026-10-13".parse().unwrap();
        let tramite = TramiteCausa {
            fecha,
            descripcion: "Ing.Serv. protección especializada y otras redes".into(),
            folio: "42".into(),
            referencia: String::new(),
            tipo: "resolucion".into(),
            contexto: false,
        };
        let mut captura = ContextoFila {
            tramites: Some(vec![tramite].into()),
            ..Default::default()
        };
        let r = evaluar_espera(&captura, Cell::Empty);
        assert!(r.observacion.contains("Tribunal dispone ingreso a DCE Ficticio, en resolución de fecha 13 de octubre de 2026 (folio 42)."));
        assert!(!r.observacion.contains("días de espera"));
        captura.notas_centro = vec![NotaCentro {
            fecha,
            texto: "Ya se hizo constar en folio 420.".into(),
        }]
        .into();
        let r = evaluar_espera(&captura, Cell::Empty);
        assert!(r.eventos.iter().any(|e| e == "HISTORIA.SERVICIO_INGRESO"));
        captura.notas_centro = vec![NotaCentro {
            fecha,
            texto: "Ya se hizo constar en folio N° 42.".into(),
        }]
        .into();
        let r = evaluar_espera(&captura, Cell::Empty);
        assert_eq!(r.eventos, ["ESPERA.E05_SOLO_CORREO"]);
        captura.varios_ingresos = true;
        let r = evaluar_espera(&captura, Cell::Date(fecha));
        assert_eq!(
            r.eventos,
            ["ESPERA.E04_RESOLUCION_RECIENTE", "ESPERA.E05_SOLO_CORREO"]
        );
        assert!(!r.observacion.contains("atribuir"));
        assert!(!r.observacion.contains("folio"));
        assert_eq!(
            r.avisos
                .iter()
                .filter(|a| a.starts_with("HISTORIA.R3_MULTI:"))
                .count(),
            1
        );
    }

    #[test]
    fn constancia_exige_ingreso_afirmado_del_mismo_sujeto_y_programa() {
        let nombre = ["ana", "maria", "del", "rio"];
        let programa = "prm centro";
        for texto in [
            "Se hace presente que Ana María cuenta con ingreso efectivo al programa PRM Centro.",
            "Ana María del Río ingreso\u{301} efectivamente a PRM Centro.",
        ] {
            assert!(constancia(texto, &nombre, programa), "{texto}");
        }
        for texto in [
            "Ana María no cuenta con ingreso efectivo al programa PRM Centro.",
            "Ana informa que Beto cuenta con ingreso efectivo al programa PRM Centro.",
            "Ana Luisa cuenta con ingreso efectivo al programa PRM Centro.",
            "Ana María cuenta con ingreso efectivo al programa PRM Centro Sur.",
            "PRM Centro comunicó proyección de ingreso de Ana.",
        ] {
            assert!(!constancia(texto, &nombre, programa), "{texto}");
        }
    }

    fn evaluar_cumplimiento(captura: &ContextoFila, egreso: &str, dpe: f64) -> Resultado {
        let cols = [
            ("tribunal", 0),
            ("programa", 1),
            ("nombre", 2),
            ("dias_cumpl", 3),
            ("dias_egresar", 4),
            ("ingreso", 5),
            ("egreso_proy", 6),
        ]
        .into();
        let values = [
            Cell::Text("Juzgado de Letras de Laja".into()),
            Cell::Text("PRM Ficticio".into()),
            Cell::Text("Persona Ficticia".into()),
            Cell::Number(if dpe >= 0.0 { 100.0 } else { dpe }),
            Cell::Number(dpe),
            Cell::Date("2026-06-01".parse().unwrap()),
            Cell::Date(egreso.parse().unwrap()),
        ];
        evaluar_con_captura(
            &Config::default(),
            Mode::Cumplimiento,
            &Fila {
                values: &values,
                cols: &cols,
            },
            "2026-10-14".parse().unwrap(),
            (&Prepaso::default(), 0),
            None,
            Some(captura),
        )
    }

    fn con_prorroga(fecha: &str, notas: Vec<NotaCentro>) -> ContextoFila {
        ContextoFila {
            tramites: Some(
                vec![TramiteCausa {
                    fecha: fecha.parse().unwrap(),
                    descripcion: "Prorroga".into(),
                    folio: "3".into(),
                    referencia: String::new(),
                    tipo: String::new(),
                    contexto: false,
                }]
                .into(),
            ),
            notas_centro: notas.into(),
            ..Default::default()
        }
    }

    fn texto(r: &Resultado) -> &str {
        r.observacion.split_once(": ").unwrap().1
    }

    fn marcas(r: &Resultado) -> Vec<&str> {
        r.marcas.iter().map(|m| m.codigo.as_str()).collect()
    }

    #[test]
    fn prorroga_vigente_se_cita_y_el_egreso_de_rus_va_aparte() {
        let r = evaluar_cumplimiento(&con_prorroga("2026-10-07", vec![]), "2027-03-01", 100.0);
        assert_eq!(texto(&r), "Medida revisada. Se hace presente que en resolución de fecha 7 de octubre de 2026 (folio 3), consta el trámite «Prorroga». RUS registra como fecha de egreso proyectado el 1 de marzo de 2027.");
        assert!(r.marcas.is_empty());
        let mut compartida = con_prorroga("2026-10-07", vec![]);
        compartida.varios_ingresos = true;
        let r = evaluar_cumplimiento(&compartida, "2027-03-01", 100.0);
        assert!(texto(&r).contains("«Prorroga»") && !texto(&r).contains("egreso proyectado"));
    }

    #[test]
    fn prorroga_con_medida_por_vencer_o_vencida_marca_segun_umbral() {
        let r = evaluar_cumplimiento(&con_prorroga("2026-10-07", vec![]), "2026-10-29", 15.0);
        assert_eq!(texto(&r), "Medida próxima a vencer en RUS el 29 de octubre de 2026. Se hace presente que en resolución de fecha 7 de octubre de 2026 (folio 3), consta el trámite «Prorroga».");
        assert_eq!(marcas(&r), ["PRORROGA_REVISAR"]);
        let antigua = evaluar_cumplimiento(&con_prorroga("2026-07-01", vec![]), "2026-10-29", 15.0);
        assert!(!antigua.observacion.contains("Prorroga") && antigua.marcas.is_empty());
        let limite = evaluar_cumplimiento(&con_prorroga("2026-07-16", vec![]), "2026-10-29", 15.0);
        assert_eq!(marcas(&limite), ["PRORROGA_REVISAR"]);
        let fuera = evaluar_cumplimiento(&con_prorroga("2026-07-15", vec![]), "2026-10-29", 15.0);
        assert!(fuera.marcas.is_empty());
        let vencida = evaluar_cumplimiento(&con_prorroga("2026-08-01", vec![]), "2026-10-01", -13.0);
        assert_eq!(texto(&vencida), "Medida vencida en RUS desde el 1 de octubre de 2026. Se hace presente que en resolución de fecha 1 de agosto de 2026 (folio 3), consta el trámite «Prorroga».");
        assert_eq!(marcas(&vencida), ["PRORROGA_REVISAR"]);
        let vieja = evaluar_cumplimiento(&con_prorroga("2026-06-20", vec![]), "2026-10-01", -13.0);
        assert_eq!(texto(&vieja), "Medida vencida en RUS desde el 1 de octubre de 2026.");
        assert!(vieja.marcas.is_empty());
        let sin_reflejar = evaluar_cumplimiento(&con_prorroga("2026-10-07", vec![]), "2026-09-01", 100.0);
        assert_eq!(marcas(&sin_reflejar), ["PRORROGA_REVISAR"]);
        assert!(sin_reflejar.observacion.contains("«Prorroga»") && !sin_reflejar.observacion.contains("hasta"));
        let mut compartida = con_prorroga("2026-10-07", vec![]);
        compartida.varios_ingresos = true;
        let r = evaluar_cumplimiento(&compartida, "2026-10-29", 15.0);
        assert_eq!(texto(&r), "Medida próxima a vencer en RUS el 29 de octubre de 2026.");
        assert_eq!(marcas(&r), ["PRORROGA_REVISAR"]);
    }

    #[test]
    fn prorroga_mencionada_en_la_bitacora_suprime_frase_y_marca() {
        let nota = |texto: &str| NotaCentro {
            fecha: "2026-10-08".parse().unwrap(),
            texto: texto.into(),
        };
        let previa = con_prorroga("2026-10-07", vec![nota("Persona X: Se hace presente que a folio 3 se prorroga la medida.")]);
        let r = evaluar_cumplimiento(&previa, "2026-10-29", 15.0);
        assert_eq!(texto(&r), "Medida próxima a vencer en RUS el 29 de octubre de 2026.");
        assert!(r.marcas.is_empty());
        let otra = con_prorroga("2026-10-07", vec![nota("Persona X: Se hace presente que a folio 99 se prorroga la medida.")]);
        assert_eq!(marcas(&evaluar_cumplimiento(&otra, "2026-10-29", 15.0)), ["PRORROGA_REVISAR"]);
    }

    /// Evalúa un caso del corpus de Flow Python con su historia y bitácora como contexto del paquete.
    fn caso_corpus(c: &serde_json::Value, mode: Mode) -> (Resultado, bool) {
        let fecha = |v: &serde_json::Value| v.as_str().and_then(|s| s.parse::<NaiveDate>().ok());
        let corte = fecha(&c["corte"]).unwrap();
        let h = &c["historia"];
        let desde = fecha(&h["desde"]);
        let vigente = desde.is_none_or(|d| corte >= d);
        let tramites: Vec<TramiteCausa> = if vigente && matches!(h["cobertura"].as_str(), Some("COMPLETA" | "PARCIAL")) {
            h["tramites"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|t| {
                    let f = fecha(&t["fecha"])?;
                    (f <= corte).then(|| TramiteCausa {
                        fecha: f,
                        descripcion: t["desc"].as_str().unwrap_or("").into(),
                        folio: t["folio_txt"].as_str().unwrap_or("").into(),
                        referencia: String::new(),
                        tipo: clave_texto(t["tipo"].as_str().unwrap_or("")),
                        contexto: desde.is_some_and(|d| f < d),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let con_prorroga = tramites.iter().any(|t| !t.contexto && clave_texto(&t.descripcion) == PRORROGA);
        let notas: Vec<NotaCentro> = c["bitacora"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|o| Some(NotaCentro { fecha: fecha(&o["fecha"])?, texto: o["texto"].as_str()?.into() }))
                    .filter(|n| n.fecha <= corte)
                    .collect()
            })
            .unwrap_or_default();
        let captura = ContextoFila { tramites: Some(tramites.into()), notas_centro: notas.into(), ..Default::default() };
        let mut cfg = Config { redaccion_registro: false, ..Config::default() };
        for d in c["desactivadas"].as_array().unwrap() {
            cfg.desactivadas.insert(d.as_str().unwrap().to_string());
        }
        let mut cols = crate::motor::Cols::new();
        let mut values = Vec::new();
        for (k, v) in c["campos"].as_object().unwrap() {
            cols.insert(Box::leak(k.clone().into_boxed_str()), values.len());
            values.push(match v["t"].as_str().unwrap() {
                "fecha" => Cell::Date(fecha(&v["v"]).unwrap()),
                "numero" => Cell::Number(v["v"].as_f64().unwrap()),
                "vacia" => Cell::Empty,
                _ => Cell::Text(v["v"].as_str().unwrap_or("").to_string()),
            });
        }
        let mut pre = Prepaso::default();
        if let Some(d) = c["duplicidad"].as_str() {
            pre.duplicidad.insert(0, if d == "MISMO_PROGRAMA" { "MISMO_PROGRAMA" } else { "OTRO_REGISTRO" });
        }
        let r = evaluar_con_captura(&cfg, mode, &Fila { values: &values, cols: &cols }, corte, (&pre, 0), fecha(&c["fecha_hoja2"]), Some(&captura));
        (r, con_prorroga)
    }

    /// Contraste focalizado con el corpus de Flow Python (`FLOW_CASOS`) en Espera y Cumplimiento con historia o
    /// bitácora. Coinciden: `HISTORIA.P_FINALIZADA` (observación completa y NOMENCL); con una «Prorroga» en la
    /// ventana, la marca PRORROGA_REVISAR y su constancia; y en todos, los eventos de curador, fichas y
    /// `HISTORIA.REITERA` en su orden, junto con las marcas de curador.
    #[test]
    fn oraculo_historia() {
        let Ok(dir) = std::env::var("FLOW_CASOS") else { return };
        const FAMILIA: [&str; 7] = ["COMUN.CURADOR", "HISTORIA.REITERA", "HISTORIA.MATERIA_", "CUMPLIMIENTO.C07_SIN_FICHA", "CUMPLIMIENTO.C08_FICHA_FAE", "HISTORIA.BASE_TRAMITE", "HISTORIA.P_FINALIZADA"];
        let familia = |e: &str| FAMILIA.iter().any(|p| e.starts_with(p));
        let marca_curador = |m: &str| m.starts_with("CURADOR_");
        let (mut total, mut finales, mut evaluados, mut fallos) = (0, 0, 0, Vec::new());
        for (archivo, mode) in [("espera", Mode::Espera), ("cumplimiento", Mode::Cumplimiento)] {
            let txt = std::fs::read_to_string(std::path::Path::new(&dir).join(format!("{archivo}.json"))).unwrap();
            let casos: Vec<serde_json::Value> = serde_json::from_str(&txt).unwrap();
            for c in &casos {
                if c["historia"].is_null() && c["bitacora"].as_array().is_none_or(|a| a.is_empty()) {
                    continue;
                }
                evaluados += 1;
                let (r, con_prorroga) = caso_corpus(c, mode);
                let e = &c["esperado"];
                let reglas: Vec<&str> = e["reglas"].as_array().unwrap().iter().filter_map(|r| r.as_str()).collect();
                let esp: Vec<&str> = reglas.iter().copied().filter(|e| familia(e)).collect();
                let obt: Vec<&str> = r.eventos.iter().map(String::as_str).filter(|e| familia(e)).collect();
                let me: Vec<&str> = e["marcas"].as_array().unwrap().iter().filter_map(|m| m["codigo"].as_str()).filter(|m| marca_curador(m)).collect();
                let mo: Vec<&str> = r.marcas.iter().map(|m| m.codigo.as_str()).filter(|m| marca_curador(m)).collect();
                // BASE_TRAMITE también lo emiten reglas no portadas (búsqueda, continuidad): se compara la constancia
                // del curador y la de la prórroga por su texto.
                fn sin_base<'a>(v: &[&'a str]) -> Vec<&'a str> {
                    v.iter().copied().filter(|e| *e != "HISTORIA.BASE_TRAMITE").collect()
                }
                let curador_txt = |t: &str| t.contains("«Designa abogado»") || t.contains("«Deja sin efecto designación");
                let (ce, co) = (curador_txt(e["observacion"].as_str().unwrap()), curador_txt(&r.observacion));
                if sin_base(&esp) != sin_base(&obt) || me != mo || ce != co {
                    fallos.push(format!("{} eventos {esp:?}/{obt:?} marcas {me:?}/{mo:?}\n  esp: {}\n  obt: {}", c["id"], e["observacion"], r.observacion));
                }
                let pe = reglas.contains(&"HISTORIA.P_FINALIZADA");
                if pe {
                    finales += 1;
                    if e["observacion"].as_str() != Some(r.observacion.as_str()) || !acciones(&r.eventos, None, "2026-01-01".parse().unwrap()).contains(&"NOMENCL") {
                        fallos.push(format!("{}: P_FINALIZADA\n  esp: {}\n  obt: {}", c["id"], e["observacion"], r.observacion));
                    }
                }
                if !con_prorroga || mode != Mode::Cumplimiento {
                    continue;
                }
                total += 1;
                let esperada = e["marcas"].as_array().unwrap().iter().any(|m| m["codigo"] == "PRORROGA_REVISAR");
                let obtenida = r.marcas.iter().any(|m| m.codigo == "PRORROGA_REVISAR");
                let frase = |t: &str| t.contains("consta el trámite «Prorroga»") || t.contains("trámites «Prorroga»");
                let (fe, fo) = (frase(e["observacion"].as_str().unwrap()), frase(&r.observacion));
                if esperada != obtenida || fe != fo {
                    fallos.push(format!("{}: prórroga marca {esperada}/{obtenida} frase {fe}/{fo}\n  esp: {}\n  obt: {}", c["id"], e["observacion"], r.observacion));
                }
            }
        }
        println!("oraculo_historia: {evaluados} casos, {total} con prórroga, {finales} causas finalizadas");
        assert!(total > 0 && finales > 0);
        assert!(fallos.is_empty(), "{} casos difieren:\n{}", fallos.len(), fallos.iter().take(10).cloned().collect::<Vec<_>>().join("\n"));
    }
}
