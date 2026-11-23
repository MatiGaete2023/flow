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

const ORDEN_BUSQUEDA: &str = "orden de busqueda";
const PIDE_CUENTA_BUSQUEDA: &str = "pide cuenta busqueda";
const SIN_EFECTO_BUSQUEDA: &str = "deja sin efecto busqueda";
const TERMINO_BUSQUEDA: &str = "termino de busqueda";
const CIERRES_BUSQUEDA: [&str; 2] = [TERMINO_BUSQUEDA, SIN_EFECTO_BUSQUEDA];
const CONTINUIDAD: &str = "resp continuidad de atencion";
const SOLICITUD_DCE: &str = "spe resp solicitud dce escrito";
const DERECHO_OIDO: &str = "derecho a ser oido";
const SIN_EFECTO_DR: &str = "deja sin efecto derivacion d r";
const SIN_EFECTO_MN: &str = "deja sin efecto medida interconexion m n";
/// `vocabulario.CONOCIDAS`, con su nombre en SITFA: lo demás deja la marca TRAMITES_SIN_REGLA.
const CONOCIDAS: [&str; 26] = [
    "Ingreso efectivo",
    "Prorroga",
    "Ingreso Informe de Cumplimiento X",
    "Orden de búsqueda",
    "Pide cuenta búsqueda",
    "Deja sin efecto búsqueda",
    "Termino de búsqueda",
    "Deja sin efecto medida interconexión M.N.",
    "Deja sin efecto derivación D.R.",
    "Resp. continuidad de atención",
    "SPE. Resp. Solicitud DCE (escrito)",
    "Derecho a ser oído",
    "Ing.Serv. protección especializada y otras redes",
    "Conciliacion",
    "Dictación de sentencia",
    "Egreso de servicio protección especializada y otras redes",
    "Designa abogado",
    "Deja sin efecto designación C. Ad-litem",
    "Inf. Cump Programa Ambulatorio",
    "Inf. Cump FAE/FAS",
    "Inf. Cump Centro Residencial",
    "Resp. Diagnóstico clínico especializado",
    "Resp. Rectificación DCE (escrito)",
    "SPE. Resp. Rectificación DCE (escrito)",
    "Derivación Dirección Reg.",
    "Pide cuenta Derivación D.R.",
];
const CONOCIDAS_EXTRA: [&str; 1] = ["Resp. Derivación dir. regional"];

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
    let captura = ctx.captura.filter(|c| !causa_compartida(ctx, c))?;
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

fn folio_o_sf(t: &TramiteCausa) -> String {
    if t.folio.is_empty() {
        "s/f".into()
    } else {
        t.folio.clone()
    }
}

fn orden_tramite(t: &TramiteCausa) -> (NaiveDate, Option<u64>) {
    (t.fecha, t.folio.parse::<u64>().ok())
}

/// `_id`: nombre, fecha y folio.
fn mismo_tramite(a: &TramiteCausa, b: &TramiteCausa) -> bool {
    a.fecha == b.fecha && a.folio == b.folio && clave_texto(&a.descripcion) == clave_texto(&b.descripcion)
}

/// Último trámite nuevo (`_nuevos`) de un nombre, citado con su plantilla.
fn ultima_constancia(ctx: &mut Ctx, nombre: &'static str, clave: &str) -> Option<Frag> {
    let captura = ctx.captura?;
    let ult = ultimo(tramites(captura, nombre).filter(|t| novedad(t, &captura.notas_centro)))?;
    let frag = ctx.render(clave, &[("FOLIO", folio_o_sf(ult)), ("FECHA", fecha_es(ult.fecha))])?;
    Some(cita(frag, ult))
}

/// `continuidad`: respuesta sobre la continuidad de atención (Espera y Cumplimiento).
pub(super) fn continuidad(ctx: &mut Ctx) -> Option<Frag> {
    ultima_constancia(ctx, CONTINUIDAD, "HISTORIA.CONTINUIDAD_ATENCION")
}

/// `solicitud_dce`: Cumplimiento DCE con la medida vencida y causa de un registro.
pub(super) fn solicitud_dce(ctx: &mut Ctx, programa: &str, vencida: bool) -> Option<Frag> {
    let captura = ctx.captura?;
    if !vencida || !crate::util::es_dce(programa) || causa_compartida(ctx, captura) {
        return None;
    }
    ultima_constancia(ctx, SOLICITUD_DCE, "HISTORIA.SOLICITUD_DCE")
}

/// `marcas_sin_efecto` y `tramites_sin_regla`: solo marcas, nunca texto para RUS.
pub(super) fn marcas_sin_efecto(ctx: &mut Ctx) {
    let Some(captura) = ctx.captura else { return };
    if tramites(captura, SIN_EFECTO_DR).next().is_some() {
        ctx.marcar(2, "DEJA_SIN_EFECTO_DR", "Deja sin efecto derivación D.R. en la historia");
    }
    if tramites(captura, SIN_EFECTO_MN).next().is_some() {
        ctx.marcar(2, "DEJA_SIN_EFECTO_MN", "Deja sin efecto medida interconexión M.N. en la historia");
    }
    let conocidas: HashSet<String> = CONOCIDAS.iter().chain(&CONOCIDAS_EXTRA).map(|n| clave_texto(n)).collect();
    let mut nombres: Vec<(String, String)> = Vec::new();
    for t in captura.tramites.as_deref().unwrap_or(&[]) {
        let clave = clave_texto(&t.descripcion);
        if t.contexto || conocidas.contains(&clave) || !novedad(t, &captura.notas_centro) {
            continue;
        }
        match nombres.iter_mut().find(|(c, _)| *c == clave) {
            Some((_, desc)) => *desc = t.descripcion.clone(),
            None => nombres.push((clave, t.descripcion.clone())),
        }
    }
    if nombres.is_empty() {
        return;
    }
    let mut lista: Vec<String> = nombres.into_iter().map(|(_, d)| d).collect();
    lista.sort();
    let mut detalle = lista.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    if lista.len() > 5 {
        detalle.push_str(&format!(" y {} más", lista.len() - 5));
    }
    ctx.marcar(3, "TRAMITES_SIN_REGLA", &format!("Trámites sin regla en la historia ({}): {detalle}", lista.len()));
}

/// `oido_en_bitacora`: una observación previa ya dejó constancia del oído en esa fecha.
pub(super) fn oido_en_bitacora(ctx: &Ctx, fecha: NaiveDate) -> bool {
    let Some(captura) = ctx.captura else { return false };
    let formas = [clave_texto(&fecha_es(fecha)), fecha.format("%d/%m/%Y").to_string()];
    captura.notas_centro.iter().any(|n| {
        let texto = clave_texto(&n.texto);
        texto.contains("oido") && formas.iter().any(|f| texto.contains(f.as_str()))
    })
}

/// `derecho_oido`: sin fecha de oído en RUS, la historia acredita la nomenclatura aplicada en la causa y se cita como
/// trámite.
pub(super) fn derecho_oido(ctx: &mut Ctx) -> Option<Frag> {
    let captura = ctx.captura?;
    let ult = ultimo(tramites(captura, DERECHO_OIDO))?;
    if oido_en_bitacora(ctx, ult.fecha) || !novedad(ult, &captura.notas_centro) {
        return None;
    }
    constancia_tramite(ctx, ult)
}

const CONECTORES: [&str; 2] = ["Posteriormente, ", "Luego, "];

/// `_frase_busqueda`: el primer trámite posterior a la orden abre con «Posteriormente» y los siguientes con «Luego».
fn frase_busqueda(ctx: &mut Ctx, t: &TramiteCausa, posicion: usize) -> Option<Frag> {
    let clave = match clave_texto(&t.descripcion).as_str() {
        PIDE_CUENTA_BUSQUEDA => "HISTORIA.R11_PIDE_CUENTA",
        SIN_EFECTO_BUSQUEDA => "HISTORIA.R11_SIN_EFECTO",
        _ => "HISTORIA.R11_TERMINO",
    };
    let mut frag = ctx.render(clave, &[("FECHA", fecha_es(t.fecha)), ("FOLIO", folio_o_sf(t))])?;
    let deseado = CONECTORES[usize::from(posicion != 0)];
    if let Some((texto, grupo)) = frag.partes.first_mut() {
        if let Some(actual) = CONECTORES.iter().find(|c| texto.starts_with(*c)) {
            *texto = format!("{deseado}{}", &texto[actual.len()..]);
            *grupo = 'C';
        }
    }
    Some(frag)
}

/// `busqueda_historia` (R11): la orden de búsqueda y luego sus trámites (pide cuenta, deja sin efecto, término). Con
/// bitácora se dice si el último trámite de la cadena es novedad; sin ella, si cae en los últimos `ventana_busqueda`
/// días. La orden puede ser el trámite de contexto. Una orden sin cierre posterior marca R11_BUSQUEDA_VIGENTE.
pub(super) fn busqueda_historia(ctx: &mut Ctx) -> Vec<Frag> {
    let Some(captura) = ctx.captura else { return Vec::new() };
    if captura.tramites.is_none() {
        return Vec::new();
    }
    // `analisis_historia.busqueda`: cada cierre cierra la orden anterior más cercana.
    let todas: Vec<&TramiteCausa> = tramites_con_contexto(captura, &[ORDEN_BUSQUEDA]).collect();
    let cerradas: Vec<&TramiteCausa> = tramites_con_contexto(captura, &CIERRES_BUSQUEDA)
        .filter_map(|c| todas.iter().copied().filter(|o| posterior(c, o)).max_by_key(|o| orden_tramite(o)))
        .collect();
    if todas.iter().any(|o| !cerradas.contains(o)) {
        ctx.marcar(1, "R11_BUSQUEDA_VIGENTE", "Orden de búsqueda vigente");
    }
    let desde = ctx.hoy - chrono::Duration::days(ctx.p("ventana_busqueda"));
    let notas = &captura.notas_centro;
    let reciente = |t: &TramiteCausa| {
        !t.contexto && if notas.is_empty() { t.fecha >= desde } else { novedad(t, notas) }
    };
    let mut ordenes: Vec<&TramiteCausa> = Vec::new();
    for o in todas {
        if !ordenes.iter().any(|x| mismo_tramite(x, o)) {
            ordenes.push(o);
        }
    }
    ordenes.sort_by_key(|o| orden_tramite(o));
    let mut seguimiento: Vec<&TramiteCausa> = [PIDE_CUENTA_BUSQUEDA, SIN_EFECTO_BUSQUEDA, TERMINO_BUSQUEDA]
        .into_iter()
        .flat_map(|n| tramites(captura, n))
        .collect();
    seguimiento.sort_by_key(|t| orden_tramite(t));
    let mut cadenas: Vec<Vec<&TramiteCausa>> = ordenes.iter().map(|o| vec![*o]).collect();
    let mut huerfanos: Vec<&TramiteCausa> = Vec::new();
    for t in seguimiento {
        let previa = (0..ordenes.len())
            .filter(|&i| posterior(t, ordenes[i]))
            .max_by_key(|&i| orden_tramite(ordenes[i]));
        match previa {
            Some(i) => cadenas[i].push(t),
            None if !huerfanos.iter().any(|x| mismo_tramite(x, t)) => huerfanos.push(t),
            None => {}
        }
    }
    let mut salida = Vec::new();
    for cadena in cadenas {
        let (orden, ult) = (cadena[0], cadena[cadena.len() - 1]);
        if !reciente(ult) || en_bitacora(ult, notas, "busqueda") {
            continue;
        }
        let Some(frag) = ctx.render(
            "HISTORIA.R11_ORDEN",
            &[("FECHA", fecha_es(orden.fecha)), ("FOLIO", folio_o_sf(orden))],
        ) else {
            continue;
        };
        salida.push(frag);
        for (n, t) in cadena[1..].iter().enumerate() {
            salida.extend(frase_busqueda(ctx, t, n));
        }
    }
    for t in huerfanos {
        if reciente(t) && !en_bitacora(t, notas, "busqueda") {
            salida.extend(constancia_tramite(ctx, t));
        }
    }
    salida
}

const MESES: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre",
    "diciembre",
];

/// `proyeccion_ingreso` (Espera, 11-nov-2026): si la última observación dice «…proyecta ingreso efectivo para el mes
/// de M», devuelve esa oración tal como está escrita mientras M sea posterior al mes del corte.
pub(super) fn proyeccion_ingreso(ctx: &Ctx) -> Option<String> {
    use chrono::Datelike;
    static PROYECCION: OnceLock<Regex> = OnceLock::new();
    let re = PROYECCION.get_or_init(|| {
        Regex::new(&format!(
            r"(?i)proyecta\s+(?:el\s+)?ingreso(?:\s+efectivo)?\s+para\s+(?:el\s+mes\s+de\s+|el\s+|)({}|setiembre)\b(?:\s+(?:de\s+|del\s+)?(\d{{4}}))?",
            MESES.join("|")
        ))
        .unwrap()
    });
    let ultima = ctx.captura?.notas_centro.iter().max_by_key(|n| n.fecha)?;
    let texto = ultima.texto.split_whitespace().collect::<Vec<_>>().join(" ");
    let hallazgo = re.captures(&texto)?;
    let nombre = hallazgo[1].to_lowercase();
    let mes = if nombre == "setiembre" { 9 } else { MESES.iter().position(|m| *m == nombre)? as u32 + 1 };
    let anio = match hallazgo.get(2) {
        Some(a) => a.as_str().parse().ok()?,
        None => ultima.fecha.year() + i32::from(mes < ultima.fecha.month()),
    };
    if (anio, mes) <= (ctx.hoy.year(), ctx.hoy.month()) {
        return None;
    }
    let (inicio, fin) = (hallazgo.get(0)?.start(), hallazgo.get(0)?.end());
    let previo = [texto[..inicio].rfind(". "), texto[..inicio].rfind(": ")].into_iter().flatten().max();
    let desde = previo.map_or(0, |p| p + 2);
    let hasta = texto[fin..].find('.').map_or(texto.len(), |p| fin + p + 1);
    Some(texto[desde..hasta].trim().to_owned())
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
        // Consignada por el funcionario (P1): la propuesta no repite la constancia.
        if ctx.consignados.contains(&clave) {
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
        if let Some(frag) = ctx.render(
            "HISTORIA.OTRO_PROGRAMA",
            &[("PNOMBRE", primero.into()), ("OTRO_PROGRAMA", otro.clone())],
        ) {
            salida.push(frag);
            ctx.otros_programas.push(otro.clone());
        }
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
    fn caso_corpus_con_redaccion(c: &serde_json::Value, mode: Mode, redaccion: bool) -> (Resultado, bool) {
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
        let mut cfg = Config {
            redaccion_registro: redaccion,
            ..Config::default()
        };
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
        if let Some(a) = c["acumulado"].as_array() {
            pre.acumulado.insert(0, (a[0].as_u64().unwrap() as usize, fecha(&a[1]).unwrap()));
        }
        let r = evaluar_con_captura(&cfg, mode, &Fila { values: &values, cols: &cols }, corte, (&pre, 0), fecha(&c["fecha_hoja2"]), Some(&captura));
        (r, con_prorroga)
    }

    fn caso_corpus(c: &serde_json::Value, mode: Mode) -> (Resultado, bool) {
        caso_corpus_con_redaccion(c, mode, false)
    }

    /// Contraste con el corpus de Flow Python (`FLOW_CASOS`): en Espera, Cumplimiento e Informes, cada caso con
    /// historia o bitácora coincide en observación, reglas y marcas; `HISTORIA.P_FINALIZADA` activa NOMENCL.
    /// `FLOW_DIFF=1` imprime cada diferencia completa.
    #[test]
    fn oraculo_historia() {
        let Ok(dir) = std::env::var("FLOW_CASOS") else { return };
        let (mut evaluados, mut con_prorroga, mut finales, mut fallos) = (0, 0, 0, Vec::new());
        for (archivo, mode) in [("espera", Mode::Espera), ("cumplimiento", Mode::Cumplimiento), ("informes", Mode::Informes)] {
            let txt = std::fs::read_to_string(std::path::Path::new(&dir).join(format!("{archivo}.json"))).unwrap();
            let casos: Vec<serde_json::Value> = serde_json::from_str(&txt).unwrap();
            for c in &casos {
                if c["historia"].is_null() && c["bitacora"].as_array().is_none_or(|a| a.is_empty()) {
                    continue;
                }
                evaluados += 1;
                let (r, prorroga) = caso_corpus(c, mode);
                con_prorroga += usize::from(prorroga);
                let e = &c["esperado"];
                let reglas: Vec<&str> = e["reglas"].as_array().unwrap().iter().filter_map(|r| r.as_str()).collect();
                let marcas: Vec<&str> = e["marcas"].as_array().unwrap().iter().filter_map(|m| m["codigo"].as_str()).collect();
                let obtenidas: Vec<&str> = r.marcas.iter().map(|m| m.codigo.as_str()).collect();
                let mut igual = e["observacion"].as_str() == Some(r.observacion.as_str())
                    && reglas == r.eventos.iter().map(String::as_str).collect::<Vec<_>>()
                    && marcas == obtenidas;
                if reglas.contains(&"HISTORIA.P_FINALIZADA") {
                    finales += 1;
                    igual &= acciones(&r.eventos, None, "2026-01-01".parse().unwrap()).contains(&"NOMENCL");
                }
                if !igual {
                    let detalle = format!("{}\n  reglas {reglas:?}\n  obten. {:?}\n  marcas {marcas:?} / {obtenidas:?}\n  esp: {}\n  obt: {}", c["id"], r.eventos, e["observacion"], r.observacion);
                    if std::env::var_os("FLOW_DIFF").is_some() {
                        println!("DIFF {detalle}");
                    }
                    fallos.push(detalle);
                }
            }
        }
        println!("oraculo_historia: {} de {evaluados} casos coinciden ({con_prorroga} con prórroga, {finales} causas finalizadas)", evaluados - fallos.len());
        assert!(evaluados > 2500 && con_prorroga > 0 && finales > 0);
        assert!(fallos.is_empty(), "{} casos difieren:\n{}", fallos.len(), fallos.iter().take(10).cloned().collect::<Vec<_>>().join("\n"));
    }

    /// Casos evaluados del corpus `rust/casos` (espera, cumplimiento e informes con historia o bitácora).
    /// Si se regenera el corpus con `rust/oraculo/generar_casos.py`, actualizar este valor en el mismo commit.
    const CASOS_PAQUETE_B: u64 = 2582;

    #[derive(Default)]
    struct PaqueteB {
        evaluados: u64,
        identicos: u64,
        solo_r1: u64,
        solo_r6: u64,
        r6_guion: u64,
        r1_y_r6: u64,
        r2_ok: u64,
        r2_aplica: u64,
        r2_cobertura: u64,
        r3_ok: u64,
        r3_aplica: u64,
        r4_ok: u64,
        r4_aplica: u64,
        r5_motor: u64,
        fallos: Vec<String>,
    }

    fn evaluar_paquete_b() -> PaqueteB {
        let dir = match std::env::var("FLOW_CASOS") {
            Ok(d) => d,
            Err(_) => panic!("FLOW_CASOS es obligatorio para el paquete B"),
        };
        let re_c10 = Regex::new(r"\s*Próximo informe de avance de [^.]* vence el [^.]*\.").unwrap();
        let mut p = PaqueteB::default();

        for (archivo, mode) in [
            ("espera", Mode::Espera),
            ("cumplimiento", Mode::Cumplimiento),
            ("informes", Mode::Informes),
        ] {
            let txt = std::fs::read_to_string(std::path::Path::new(&dir).join(format!("{archivo}.json"))).unwrap();
            let casos: Vec<serde_json::Value> = serde_json::from_str(&txt).unwrap();
            for c in &casos {
                if c["historia"].is_null()
                    && c["bitacora"].as_array().is_none_or(|a| a.is_empty())
                {
                    continue;
                }
                p.evaluados += 1;
                let (r, _) = caso_corpus_con_redaccion(c, mode, true);
                let e = &c["esperado"];
                let reglas: Vec<&str> = e["reglas"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|r| r.as_str())
                    .collect();
                let marcas: Vec<&str> = e["marcas"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|m| m["codigo"].as_str())
                    .collect();
                let obtenidas: Vec<&str> = r.marcas.iter().map(|m| m.codigo.as_str()).collect();

                // R5: la reiteración (COMUN.REITERA_OBSERVACION) la aplica la cronología, no el motor por fila.
                // Aquí solo se acredita que el motor por fila no la emite; R5 se coteja con la cronología.
                const REITERA: &str = "COMUN.REITERA_OBSERVACION";
                if reglas.contains(&REITERA) || r.eventos.iter().any(|ev| ev == REITERA) {
                    p.r5_motor += 1;
                    p.fallos.push(format!("{}: R5 emitida por el motor por fila", c["id"]));
                }

                let reglas_iguales = reglas == r.eventos.iter().map(String::as_str).collect::<Vec<_>>();
                let marcas_iguales = marcas == obtenidas;
                if !reglas_iguales || !marcas_iguales {
                    p.fallos.push(format!("{}: reglas o marcas difieren", c["id"]));
                    continue;
                }

                let esp_obs = e["observacion"].as_str().unwrap_or("");
                let obt_obs = r.observacion.as_str();

                // ---- R2: sin la oración de historia de causa ni R3_MULTI, en todos los casos ----
                // HISTORIA.R3_MULTI no es regla del motor Python: en Rust es un aviso interno. La cobertura de R2
                // son los casos en que Rust emite ese aviso (historia de la causa con varios ingresos).
                if r.avisos.iter().any(|a| a.starts_with("HISTORIA.R3_MULTI:")) {
                    p.r2_cobertura += 1;
                }
                p.r2_aplica += 1;
                if !obt_obs.contains("La historia es de la causa y no permite atribuirlo a un ingreso determinado.")
                    && !obt_obs.contains("R3_MULTI")
                {
                    p.r2_ok += 1;
                } else {
                    p.fallos.push(format!("{}: R2", c["id"]));
                }

                // ---- R3: con E04_CON_FOLIO y E05_SOLO_CORREO ----
                if reglas.contains(&"HISTORIA.E04_CON_FOLIO") && reglas.contains(&"ESPERA.E05_SOLO_CORREO") {
                    p.r3_aplica += 1;
                    let cuenta_medida =
                        obt_obs.match_indices("Medida revisada, a la espera de ingreso efectivo.").count();
                    let tiene_dias = obt_obs.contains("RUS registra") && obt_obs.contains("días de espera");
                    if cuenta_medida == 1 && !tiene_dias {
                        p.r3_ok += 1;
                    } else {
                        p.fallos.push(format!("{}: R3", c["id"]));
                    }
                }

                // ---- R4: si tiene "Tribunal dispone ingreso a", no días de espera ----
                if obt_obs.contains("Tribunal dispone ingreso a") {
                    p.r4_aplica += 1;
                    let tiene_dias = obt_obs.contains("RUS registra") && obt_obs.contains("días de espera");
                    if !tiene_dias {
                        p.r4_ok += 1;
                    } else {
                        p.fallos.push(format!("{}: R4", c["id"]));
                    }
                }

                // ---- R1 y R6: comparación byte a byte de la observación ----
                if esp_obs == obt_obs {
                    p.identicos += 1;
                    continue;
                }

                let tiene_r1 = r.eventos.iter().any(|ev| ev == "CUMPLIMIENTO.C05_POR_VENCER")
                    && r.eventos.iter().any(|ev| ev == "CUMPLIMIENTO.C10_HOJA2");

                let mut base_esp = esp_obs.to_string();
                if tiene_r1 {
                    assert!(
                        re_c10.is_match(esp_obs),
                        "{} C10: la oración C10 no existe en la esperada",
                        c["id"]
                    );
                    assert!(
                        !re_c10.is_match(obt_obs),
                        "{} C10: la oración C10 existe en la obtenida",
                        c["id"]
                    );
                    base_esp = re_c10.replace(&base_esp, "").to_string();
                }

                let nombre = c["campos"]["programa"]["v"].as_str().unwrap_or("");
                let py = crate::util::titulo_programa(nombre);
                let rs = crate::util::titulo_programa_registro(nombre);
                let esp_ajustada = base_esp.replace(&py, &rs);
                let es_r6 = esp_ajustada != base_esp;
                let es_guion = es_r6 && rs.contains(" - ") && !py.contains(" - ");

                if esp_ajustada == obt_obs {
                    match (tiene_r1, es_r6) {
                        (true, true) => p.r1_y_r6 += 1,
                        (true, false) => p.solo_r1 += 1,
                        (false, true) if es_guion => p.r6_guion += 1,
                        (false, true) => p.solo_r6 += 1,
                        (false, false) => p.fallos.push(format!("{}: diferencia sin regla", c["id"])),
                    }
                } else {
                    let primera = primera_palabra_distinta(&esp_ajustada, obt_obs);
                    p.fallos.push(format!(
                        "{}: diff inesperado (primera palabra distinta: {})",
                        c["id"], primera
                    ));
                }
            }
        }
        p
    }

    fn cobertura(ok: u64, aplica: u64) -> String {
        if aplica == 0 {
            "SIN_COBERTURA 0/0".into()
        } else {
            format!("{ok}/{aplica}")
        }
    }

    #[test]
    fn paquete_b_redaccion_mensual() {
        let p = evaluar_paquete_b();
        let diferencias = p.solo_r1 + p.solo_r6 + p.r6_guion + p.r1_y_r6;
        println!(
            "PAQUETE_B: evaluados {}, identicos {}, R1 {}, R6 {}, R6_GUION {}, R1+R6 {}, suma {}, R2 {} (con aviso R3_MULTI {}), R3 {}, R4 {}, R5 fuera del motor por fila (emitidas {}), fallos {}",
            p.evaluados,
            p.identicos,
            p.solo_r1,
            p.solo_r6,
            p.r6_guion,
            p.r1_y_r6,
            diferencias,
            cobertura(p.r2_ok, p.r2_aplica),
            p.r2_cobertura,
            cobertura(p.r3_ok, p.r3_aplica),
            cobertura(p.r4_ok, p.r4_aplica),
            p.r5_motor,
            p.fallos.len()
        );
        for f in p.fallos.iter().take(5) {
            println!("  FALLO: {f}");
        }
        assert_eq!(p.evaluados, CASOS_PAQUETE_B, "cambió la cantidad de casos evaluados del corpus");
        assert_eq!(diferencias + p.identicos, p.evaluados, "hay casos sin clasificar");
        assert!(p.fallos.is_empty(), "Hubo fallos inesperados en Paquete B");
    }

    /// Exige que el corpus tenga casos de R3 (HISTORIA.E04_CON_FOLIO + ESPERA.E05_SOLO_CORREO).
    /// R2 no se exige aquí: HISTORIA.R3_MULTI no existe en el motor Python (en Rust es un aviso interno) y la cubre
    /// `espera_con_servicio_y_causa_compartida_aplica_r2_y_r4`. Hoy falla; se activa al cerrar el encargo E-J.
    #[test]
    #[ignore = "E-J: el corpus rust/casos no tiene casos de R3 (E04_CON_FOLIO + E05_SOLO_CORREO)"]
    fn paquete_b_cobertura_r3() {
        let p = evaluar_paquete_b();
        println!(
            "PAQUETE_B_COBERTURA: R3 {}, R4 {}, fallos {}",
            cobertura(p.r3_ok, p.r3_aplica),
            cobertura(p.r4_ok, p.r4_aplica),
            p.fallos.len()
        );
        assert!(p.r3_aplica > 0, "R3 sin cobertura: ningún caso con E04_CON_FOLIO y E05_SOLO_CORREO");
        assert!(p.fallos.is_empty(), "Hubo fallos inesperados en Paquete B");
    }

    fn primera_palabra_distinta(a: &str, b: &str) -> String {
        let pa: Vec<&str> = a.split_whitespace().collect();
        let pb: Vec<&str> = b.split_whitespace().collect();
        let n = pa.len().min(pb.len());
        for i in 0..n {
            if pa[i] != pb[i] {
                return format!("esp={} obt={}", pa[i], pb[i]);
            }
        }
        if pa.len() != pb.len() {
            if pa.len() > n {
                return format!("esp={} obt=<fin>", pa[n]);
            }
            return format!("esp=<fin> obt={}", pb[n]);
        }
        String::new()
    }

    #[test]
    fn a4_busqueda_historia() {
        // 1. Una orden de contexto con su término en la ventana se dice completa
        let orden_ctx = TramiteCausa {
            fecha: "2026-09-01".parse().unwrap(),
            descripcion: "Orden de búsqueda".into(),
            folio: "10".into(),
            referencia: String::new(),
            tipo: "resolucion".into(),
            contexto: true,
        };
        let termino_en_ventana = TramiteCausa {
            fecha: "2026-10-10".parse().unwrap(),
            descripcion: "Termino de búsqueda".into(),
            folio: "15".into(),
            referencia: String::new(),
            tipo: "resolucion".into(),
            contexto: false,
        };
        let captura1 = ContextoFila {
            tramites: Some(vec![orden_ctx.clone(), termino_en_ventana.clone()].into()),
            ..Default::default()
        };
        let r1 = evaluar_espera(&captura1, Cell::Empty);
        assert!(r1.observacion.contains("Se hace presente que en resolución de fecha 1 de septiembre de 2026 (folio 10), se decretó orden de búsqueda."));
        assert!(r1.observacion.contains("Posteriormente, en resolución de fecha 10 de octubre de 2026 (folio 15), se dispuso el término de la búsqueda."));
        assert!(!r1.marcas.iter().any(|m| m.codigo == "R11_BUSQUEDA_VIGENTE"));

        // 2. Con dos trámites posteriores, el primero abre con «Posteriormente» y el segundo con «Luego»
        let pide_cuenta = TramiteCausa {
            fecha: "2026-10-05".parse().unwrap(),
            descripcion: "Pide cuenta búsqueda".into(),
            folio: "12".into(),
            referencia: String::new(),
            tipo: "resolucion".into(),
            contexto: false,
        };
        let captura2 = ContextoFila {
            tramites: Some(vec![orden_ctx.clone(), pide_cuenta.clone(), termino_en_ventana.clone()].into()),
            ..Default::default()
        };
        let r2 = evaluar_espera(&captura2, Cell::Empty);
        assert!(r2.observacion.contains("Posteriormente, en resolución de fecha 5 de octubre de 2026 (folio 12), se pidió cuenta de la búsqueda."));
        assert!(r2.observacion.contains("Luego, en resolución de fecha 10 de octubre de 2026 (folio 15), se dispuso el término de la búsqueda."));

        // 3. Un cierre sin orden se dice como constancia neutra
        let captura3 = ContextoFila {
            tramites: Some(vec![termino_en_ventana.clone()].into()),
            ..Default::default()
        };
        let r3 = evaluar_espera(&captura3, Cell::Empty);
        assert!(r3.observacion.contains("Se hace presente que en resolución de fecha 10 de octubre de 2026 (folio 15), consta el trámite «Termino de búsqueda»."));
        assert!(!r3.observacion.contains("Posteriormente"));
        assert!(!r3.marcas.iter().any(|m| m.codigo == "R11_BUSQUEDA_VIGENTE"));

        // 4. Una orden repetida con el mismo folio se dice una sola vez
        let orden_reciente = TramiteCausa {
            fecha: "2026-10-02".parse().unwrap(),
            descripcion: "Orden de búsqueda".into(),
            folio: "20".into(),
            referencia: String::new(),
            tipo: "resolucion".into(),
            contexto: false,
        };
        let captura4 = ContextoFila {
            tramites: Some(vec![orden_reciente.clone(), orden_reciente.clone()].into()),
            ..Default::default()
        };
        let r4 = evaluar_espera(&captura4, Cell::Empty);
        assert_eq!(r4.observacion.matches("se decretó orden de búsqueda").count(), 1);

        // 5. Sin bitácora rige ventana_busqueda (60 días por omisión: desde 2026-08-20 hasta 2026-10-19)
        let orden_fuera_ventana = TramiteCausa {
            fecha: "2026-08-01".parse().unwrap(),
            descripcion: "Orden de búsqueda".into(),
            folio: "5".into(),
            referencia: String::new(),
            tipo: "resolucion".into(),
            contexto: false,
        };
        let captura5 = ContextoFila {
            tramites: Some(vec![orden_fuera_ventana].into()),
            ..Default::default()
        };
        let r5 = evaluar_espera(&captura5, Cell::Empty);
        assert!(!r5.observacion.contains("se decretó orden de búsqueda"), "Trámite fuera de ventana no debe decirse");

        // 6. La marca R11_BUSQUEDA_VIGENTE aparece solo con una orden sin cierre
        assert!(r4.marcas.iter().any(|m| m.codigo == "R11_BUSQUEDA_VIGENTE"));
        assert!(!r1.marcas.iter().any(|m| m.codigo == "R11_BUSQUEDA_VIGENTE"));
    }

    #[test]
    fn a5_proyeccion_ingreso() {
        let fecha_nota = "2026-10-10".parse().unwrap();

        // 1. Mes sin año (mes posterior al corte 2026-10-19 -> noviembre 2026)
        let captura1 = ContextoFila {
            notas_centro: vec![NotaCentro {
                fecha: fecha_nota,
                texto: "El programa proyecta ingreso para el mes de noviembre.".into(),
            }].into(),
            ..Default::default()
        };
        let r1 = evaluar_espera(&captura1, Cell::Date("2026-10-12".parse().unwrap()));
        assert!(r1.eventos.contains(&"ESPERA.E07_PROYECCION".to_string()));
        assert!(r1.observacion.contains("El programa proyecta ingreso para el mes de noviembre."));
        // Con proyección vigente no se emiten E04 ni E05
        assert!(!r1.eventos.contains(&"ESPERA.E04_RESOLUCION_RECIENTE".to_string()));
        assert!(!r1.eventos.contains(&"HISTORIA.E04_CON_FOLIO".to_string()));
        assert!(!r1.eventos.contains(&"ESPERA.E05_SOLO_CORREO".to_string()));

        // 2. Mes con año (enero 2027)
        let captura2 = ContextoFila {
            notas_centro: vec![NotaCentro {
                fecha: fecha_nota,
                texto: "Se proyecta el ingreso efectivo para el mes de enero de 2027.".into(),
            }].into(),
            ..Default::default()
        };
        let r2 = evaluar_espera(&captura2, Cell::Empty);
        assert!(r2.eventos.contains(&"ESPERA.E07_PROYECCION".to_string()));
        assert!(r2.observacion.contains("Se proyecta el ingreso efectivo para el mes de enero de 2027."));

        // 3. «setiembre» (setiembre 2027)
        let captura3 = ContextoFila {
            notas_centro: vec![NotaCentro {
                fecha: fecha_nota,
                texto: "Coordinación proyecta ingreso para setiembre 2027.".into(),
            }].into(),
            ..Default::default()
        };
        let r3 = evaluar_espera(&captura3, Cell::Empty);
        assert!(r3.eventos.contains(&"ESPERA.E07_PROYECCION".to_string()));
        assert!(r3.observacion.contains("Coordinación proyecta ingreso para setiembre 2027."));

        // 4. Mes igual al del corte (octubre 2026): no se copia
        let captura4 = ContextoFila {
            notas_centro: vec![NotaCentro {
                fecha: fecha_nota,
                texto: "Programa proyecta ingreso para el mes de octubre.".into(),
            }].into(),
            ..Default::default()
        };
        let r4 = evaluar_espera(&captura4, Cell::Date("2026-10-12".parse().unwrap()));
        assert!(!r4.eventos.contains(&"ESPERA.E07_PROYECCION".to_string()));
        assert!(r4.eventos.contains(&"ESPERA.E04_RESOLUCION_RECIENTE".to_string()));

        // 5. Mes pasado (agosto 2026): no se copia
        let captura5 = ContextoFila {
            notas_centro: vec![NotaCentro {
                fecha: fecha_nota,
                texto: "Programa proyecta ingreso para agosto de 2026.".into(),
            }].into(),
            ..Default::default()
        };
        let r5 = evaluar_espera(&captura5, Cell::Empty);
        assert!(!r5.eventos.contains(&"ESPERA.E07_PROYECCION".to_string()));

        // 6. Precedida por «Nombre SIGLA: »
        let captura6 = ContextoFila {
            notas_centro: vec![NotaCentro {
                fecha: fecha_nota,
                texto: "Persona DCE: Se proyecta ingreso para el mes de diciembre.".into(),
            }].into(),
            ..Default::default()
        };
        let r6 = evaluar_espera(&captura6, Cell::Empty);
        assert!(r6.eventos.contains(&"ESPERA.E07_PROYECCION".to_string()));
        assert!(r6.observacion.contains("Se proyecta ingreso para el mes de diciembre."));
        assert!(!r6.observacion.contains("efectivo. Persona DCE:"));

        // 7. Precedida por otra oración
        let captura7 = ContextoFila {
            notas_centro: vec![NotaCentro {
                fecha: fecha_nota,
                texto: "Caso revisado en reunión del centro. Se proyecta ingreso para el mes de diciembre.".into(),
            }].into(),
            ..Default::default()
        };
        let r7 = evaluar_espera(&captura7, Cell::Empty);
        assert!(r7.eventos.contains(&"ESPERA.E07_PROYECCION".to_string()));
        assert!(r7.observacion.contains("Se proyecta ingreso para el mes de diciembre."));
        assert!(!r7.observacion.contains("Caso revisado"));
    }
}
