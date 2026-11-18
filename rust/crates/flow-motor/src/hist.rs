//! Constancias de la historia de la causa (`motor/reglas_historia.py` y `motor/analisis_historia.py`).
//! Cada función devuelve fragmentos (vacío si no aplica).

use crate::ctx::{Ctx, Duplicidad, Frag, OtroPrograma};
use chrono::{Datelike, Duration, NaiveDate};
use flow_dominio::historia::ultimo;
use flow_dominio::texto::{clave, contiene_token, es_dce, fecha_es, fecha_mayoria, normalizar, titulo_programa};
use flow_dominio::evidencia::{self, clave_programa, clave_texto, constancia_en_bitacora, mencion_previa, programas_vigentes_otros, ESTADO_PENDIENTE};
use flow_dominio::{Escrito, Tramite};
use regex::Regex;
use std::sync::LazyLock;

// Nombres de nomenclatura (Desc. Trámite). «Ingreso» es la resolución que ordena el ingreso (Espera);
// «Ingreso efectivo» es el trámite con que el programa informa la intervención (Cumplimiento).
pub const INGRESO_EFECTIVO: &str = "Ingreso efectivo";
pub const PRORROGA: &str = "Prorroga";
pub const INGRESO_INFORME_X: &str = "Ingreso informe de cumplimiento X";
pub const ORDEN_BUSQUEDA: &str = "Orden de búsqueda";
pub const PIDE_CUENTA_BUSQUEDA: &str = "Pide cuenta búsqueda";
pub const TERMINO_BUSQUEDA: &str = "Termino de búsqueda";
pub const SIN_EFECTO_BUSQUEDA: &str = "Deja sin efecto búsqueda";
pub const SIN_EFECTO_MN: &str = "Deja sin efecto medida interconexión M.N.";
pub const SIN_EFECTO_DR: &str = "Deja sin efecto derivación D.R.";
pub const CONTINUIDAD: &str = "Resp. continuidad de atención";
pub const SOLICITUD_DCE: &str = "SPE. Resp. Solicitud DCE (escrito)";
pub const DERECHO_OIDO: &str = "Derecho a ser oído";
pub const SERVICIO: &str = "Ing.Serv. protección especializada y otras redes";
pub const CONCILIACION: &str = "Conciliacion";
pub const DICTACION_SENTENCIA: &str = "Dictación de sentencia";
pub const EGRESO_SERVICIO: &str = "Egreso de servicio protección especializada y otras redes";
pub const DESIGNA_ABOGADO: &str = "Designa abogado";
pub const SIN_EFECTO_CURADOR: &str = "Deja sin efecto designación C. Ad-litem";
const RESIDENCIA_PREFIJOS: [&str; 5] = ["rta", "rtt", "res", "rfa", "rva"];

/// Sin regla ni marca por decisión del usuario: no cuentan como «trámites sin regla».
const IGNORADAS: [&str; 6] = [
    "Resp. Diagnóstico clínico especializado",
    "Resp. Rectificación DCE (escrito)",
    "SPE. Resp. Rectificación DCE (escrito)",
    "Derivación Dirección Reg.",
    "Pide cuenta Derivación D.R.",
    "Resp. Derivación dir. regional",
];
/// Nomenclaturas que alguna regla reconoce (`vocabulario.py::CONOCIDAS`).
const CONOCIDAS: [&str; 21] = [
    INGRESO_EFECTIVO,
    PRORROGA,
    INGRESO_INFORME_X,
    ORDEN_BUSQUEDA,
    PIDE_CUENTA_BUSQUEDA,
    SIN_EFECTO_BUSQUEDA,
    TERMINO_BUSQUEDA,
    SIN_EFECTO_MN,
    SIN_EFECTO_DR,
    CONTINUIDAD,
    SOLICITUD_DCE,
    DERECHO_OIDO,
    SERVICIO,
    CONCILIACION,
    DICTACION_SENTENCIA,
    EGRESO_SERVICIO,
    DESIGNA_ABOGADO,
    SIN_EFECTO_CURADOR,
    "Inf. Cump Programa Ambulatorio",
    "Inf. Cump FAE/FAS",
    "Inf. Cump Centro Residencial",
];
/// Último trámite anterior a la ventana que el Descargador conserva como contexto (`vocabulario.py::CONTEXTO`).
pub const CONTEXTO: [&str; 9] = [
    ORDEN_BUSQUEDA,
    TERMINO_BUSQUEDA,
    SIN_EFECTO_BUSQUEDA,
    CONCILIACION,
    DICTACION_SENTENCIA,
    EGRESO_SERVICIO,
    DESIGNA_ABOGADO,
    SIN_EFECTO_CURADOR,
    PRORROGA,
];

const RESOLUCION: &str = "en resolución de fecha";
const MESES: [&str; 12] =
    ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"];
static PROYECCION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)proyecta\s+(?:el\s+)?ingreso(?:\s+efectivo)?\s+para\s+(?:el\s+mes\s+de\s+|el\s+|)({}|setiembre)\b(?:\s+(?:de\s+|del\s+)?(\d{{4}}))?",
        MESES.join("|")
    ))
    .unwrap()
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Materia {
    Curador,
    SinFicha,
    SinFae,
}

impl Materia {
    fn original(self) -> (&'static str, &'static str) {
        match self {
            Materia::Curador => ("COMUN.CURADOR", "MATERIA_CURADOR"),
            Materia::SinFicha => ("CUMPLIMIENTO.C07_SIN_FICHA", "MATERIA_SIN_FICHA"),
            Materia::SinFae => ("CUMPLIMIENTO.C08_FICHA_FAE", "MATERIA_SIN_FAE"),
        }
    }
    fn patron(self) -> &'static str {
        match self {
            Materia::Curador => "no registra curador ad litem",
            Materia::SinFicha => "no registra ficha individual",
            Materia::SinFae => "no registra ficha fae",
        }
    }
}

pub fn es_residencial(programa: &str) -> bool {
    let n = normalizar(programa);
    RESIDENCIA_PREFIJOS.iter().any(|p| n.starts_with(p))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intervencion {
    Dce,
    Residencial,
    Fae,
    Ambulatorio,
}

pub fn intervencion(programa: &str) -> Intervencion {
    if es_dce(programa) {
        Intervencion::Dce
    } else if es_residencial(programa) {
        Intervencion::Residencial
    } else if contiene_token(programa, &["fae", "fas"]) {
        Intervencion::Fae
    } else {
        Intervencion::Ambulatorio
    }
}

fn escrito_de(tipo: Intervencion) -> Option<&'static str> {
    match tipo {
        Intervencion::Ambulatorio => Some("Inf. Cump Programa Ambulatorio"),
        Intervencion::Fae => Some("Inf. Cump FAE/FAS"),
        Intervencion::Residencial => Some("Inf. Cump Centro Residencial"),
        Intervencion::Dce => None,
    }
}

fn refs(lista: &[Tramite]) -> Vec<&Tramite> {
    lista.iter().collect()
}

fn folio_fecha(t: &Tramite) -> [(&'static str, String); 2] {
    [("FOLIO", t.folio_visible().to_string()), ("FECHA", fecha_es(t.fecha))]
}

/// La causa tiene más de un registro en la nómina: la historia, que es de la causa, no se puede atribuir a esta fila.
/// No se afirman hechos personales ni se retiran gestiones por un trámite.
pub fn varios_registros(ctx: &Ctx) -> bool {
    ctx.duplicidad.is_some()
}

/// Nombra el trámite por lo que es: «en resolución de fecha» pasa a «en escrito de fecha» o «en actuación de fecha»
/// según la columna «Trámite» de SITFA. Sin ese dato se conserva «resolución».
fn cita(fragmento: Frag, tramite: &Tramite) -> Frag {
    let nuevo = match clave(&tramite.tipo).as_str() {
        "escrito" => "en escrito de fecha",
        "actuacion" => "en actuación de fecha",
        _ => return fragmento,
    };
    if fragmento.es_vacio() || !fragmento.texto.contains(RESOLUCION) {
        return fragmento;
    }
    Frag {
        texto: fragmento.texto.replacen(RESOLUCION, nuevo, 1),
        grupo: fragmento.grupo,
        partes: fragmento.partes.iter().map(|(t, g)| (t.replacen(RESOLUCION, nuevo, 1), *g)).collect(),
        dato: fragmento.dato,
    }
}

/// Fecha de la última observación de la bitácora del ingreso, o None si no hay bitácora.
pub fn ultima_observacion(ctx: &Ctx) -> Option<NaiveDate> {
    ctx.bitacora.last().map(|o| o.fecha)
}

/// Una observación de la bitácora, del día del trámite o posterior, ya cita su folio («folio N»).
fn citado(ctx: &Ctx, tramite: &Tramite) -> bool {
    !tramite.folio_txt.is_empty()
        && ctx.bitacora.iter().any(|o| o.fecha >= tramite.fecha && cita_folio(&clave(&o.texto), &tramite.folio_txt))
}

/// Una constancia de historia se dice solo si el trámite es de la fecha de la última observación o posterior y esa
/// bitácora no citó ya su folio. Sin bitácora, todo trámite de la ventana es novedad.
pub fn novedad(ctx: &Ctx, tramite: &Tramite) -> bool {
    if ultima_observacion(ctx).is_some_and(|u| tramite.fecha < u) {
        return false;
    }
    !citado(ctx, tramite)
}

/// Trámites aún no citados en esta fila y que son novedad.
fn nuevos(ctx: &Ctx, nombres: &[&str]) -> Vec<Tramite> {
    ctx.tramites(nombres).into_iter().filter(|t| !ctx.emitidos.contains(&t.id()) && novedad(ctx, t)).collect()
}

/// Texto base de uno o varios trámites del mismo nombre; omite los ya citados.
fn constancia(ctx: &mut Ctx, lista: Vec<Tramite>) -> Frag {
    let lista: Vec<Tramite> = lista.into_iter().filter(|t| !ctx.emitidos.contains(&t.id())).collect();
    let Some(ult) = ultimo(&refs(&lista)).cloned() else { return Frag::vacio() };
    ctx.emitidos.extend(lista.iter().map(Tramite::id));
    let mut datos = vec![("DESC", ult.desc.clone()), ("FECHA", fecha_es(ult.fecha)), ("FOLIO", ult.folio_visible().to_string())];
    let texto = if lista.len() == 1 {
        ctx.render("HISTORIA", "BASE_TRAMITE", &datos)
    } else {
        datos.push(("N", lista.len().to_string()));
        ctx.render("HISTORIA", "VARIOS", &datos)
    };
    cita(texto, &ult)
}

/// Regla 1: folio del trámite «Ing.Serv. …» fechado el día de la resolución de RUS; None si no hay o hay varios.
pub fn folio_resolucion(ctx: &mut Ctx, fecha: NaiveDate) -> Option<String> {
    let ingresos: Vec<Tramite> = ctx.tramites(&[SERVICIO]).into_iter().filter(|t| t.fecha == fecha && !t.folio_txt.is_empty()).collect();
    let mut folios: Vec<&str> = ingresos.iter().map(|t| t.folio_txt.as_str()).collect();
    folios.sort_unstable();
    folios.dedup();
    if folios.len() != 1 {
        return None;
    }
    let folio = ingresos[0].folio_txt.clone();
    ctx.emitidos.extend(ingresos.iter().map(Tramite::id));
    Some(folio)
}

/// Espera: «Ingreso efectivo» en la historia con RUS en Espera solo marca para revisión.
pub fn ingreso_efectivo_en_espera(ctx: &mut Ctx, corte: NaiveDate, espera: Option<i64>) {
    let mut lista = ctx.tramites(&[INGRESO_EFECTIVO]);
    if let Some(dias) = espera.filter(|d| *d >= 0) {
        if let Some(inicio) = corte.checked_sub_signed(Duration::days(dias)) {
            lista.retain(|t| t.fecha >= inicio);
        }
    }
    if !lista.is_empty() {
        ctx.marcar(2, "R3_INGRESO_EFECTIVO", "Historia con ingreso efectivo y RUS en Espera");
    }
}

/// Espera: si la última observación de la bitácora dice «…proyecta ingreso efectivo para el mes de M», devuelve esa
/// oración tal como está escrita mientras M sea posterior al mes del corte; si no, vacío.
pub fn proyeccion_ingreso(ctx: &Ctx, corte: NaiveDate) -> String {
    let Some(ultima) = ctx.bitacora.last() else { return String::new() };
    let texto = ultima.texto.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(hallazgo) = PROYECCION.captures(&texto) else { return String::new() };
    let entero = hallazgo.get(0).unwrap();
    let nombre = hallazgo[1].to_lowercase();
    let mes = if nombre == "setiembre" { 9 } else { MESES.iter().position(|m| *m == nombre).map_or(0, |p| p as u32 + 1) };
    let anio = match hallazgo.get(2).and_then(|a| a.as_str().parse::<i32>().ok()) {
        Some(a) => a,
        None => ultima.fecha.year() + i32::from(mes < ultima.fecha.month()),
    };
    if (anio, mes) <= (corte.year(), corte.month()) {
        return String::new();
    }
    let previo = &texto[..entero.start()];
    let corte_previo = [previo.rfind(". "), previo.rfind(": ")].into_iter().flatten().max();
    let inicio = corte_previo.map_or(0, |c| c + 2);
    let fin = texto[entero.end()..].find('.').map_or(texto.len(), |p| entero.end() + p + 1);
    texto[inicio..fin].trim().to_string()
}

pub fn marcar_duplicidad(ctx: &mut Ctx) {
    if ctx.duplicidad == Some(Duplicidad::MismoPrograma) {
        ctx.marcar(1, "R6_DUPLICIDAD", "Más de un ingreso del mismo NNA al mismo programa");
    }
}

/// Materias de REITERA presentes en una observación previa, por oración: una oración que reitera no es la original.
fn materias(texto: &str) -> Vec<Materia> {
    let oraciones: Vec<String> = crate::ctx::oraciones(texto).iter().map(|o| clave(o)).filter(|o| !o.contains("se reitera")).collect();
    [Materia::Curador, Materia::SinFicha, Materia::SinFae]
        .into_iter()
        .filter(|m| oraciones.iter().any(|o| o.contains(m.patron())))
        .collect()
}

/// REITERA: vacío si no hay observación previa de la materia o la regla original está desactivada. Con un trámite de
/// curador posterior a la observación citada la situación cambió y no se reitera.
pub fn reitera(ctx: &mut Ctx, materia: Materia) -> Frag {
    let (original, clave_materia) = materia.original();
    if ctx.desactivada(original) {
        return Frag::vacio();
    }
    let hoy = ctx.hoy;
    let Some(fecha) = ctx.bitacora.iter().find(|o| o.fecha < hoy && materias(&o.texto).contains(&materia)).map(|o| o.fecha) else {
        return Frag::vacio();
    };
    if materia == Materia::Curador && ctx.tramites_con(&[DESIGNA_ABOGADO, SIN_EFECTO_CURADOR], true).iter().any(|t| t.fecha > fecha) {
        return Frag::vacio();
    }
    let texto_materia = ctx.render("HISTORIA", clave_materia, &[]).texto;
    let frag = ctx.render("HISTORIA", "REITERA", &[("FECHA_ANTERIOR", fecha_es(fecha)), ("MATERIA", texto_materia)]);
    if frag.es_vacio() {
        return frag;
    }
    ctx.eventos.push(original.to_string());
    // Reiterar la ausencia de curador o de ficha es gestión del Tribunal.
    Frag::simple(frag.texto, 'T')
}

/// (trámite, es_baja) del último trámite de curador de la causa, incluido el contexto anterior a la ventana.
fn ultimo_curador(ctx: &Ctx) -> Option<(Tramite, bool)> {
    let designa = ultimo(&refs(&ctx.tramites_con(&[DESIGNA_ABOGADO], true))).cloned();
    let baja = ultimo(&refs(&ctx.tramites_con(&[SIN_EFECTO_CURADOR], true))).cloned();
    match (designa, baja) {
        (None, None) => None,
        (None, Some(b)) => Some((b, true)),
        (Some(d), None) => Some((d, false)),
        (Some(d), Some(b)) => Some(if d.posterior_a(&b) { (d, false) } else { (b, true) }),
    }
}

/// RUS no registra curador y lo último de la causa es «Deja sin efecto designación C. Ad-litem», en una causa de un
/// solo registro: no hay a quién asociar y la observación no lo sugiere.
pub fn curador_sin_efecto(ctx: &Ctx, rus_sin_curador: bool) -> bool {
    if !rus_sin_curador || varios_registros(ctx) {
        return false;
    }
    ultimo_curador(ctx).is_some_and(|(_, baja)| baja)
}

/// Último trámite de curador de la historia, solo si RUS no registra curador y la causa tiene un solo registro.
/// Se cita con su nombre literal: la nomenclatura no acredita que sea curador ni de quién.
pub fn curador_designaciones(ctx: &mut Ctx, rus_sin_curador: bool) -> Vec<Frag> {
    if !rus_sin_curador || varios_registros(ctx) {
        return Vec::new();
    }
    let Some((ult, es_baja)) = ultimo_curador(ctx) else { return Vec::new() };
    let Some(h) = ctx.h() else { return Vec::new() };
    if !h.en_ventana(&ult) || ctx.emitidos.contains(&ult.id()) || !novedad(ctx, &ult) {
        return Vec::new();
    }
    let llave = if es_baja { "CURADOR_SIN_EFECTO" } else { "CURADOR_DESIGNADO" };
    ctx.marcar(2, llave, "Curador ad litem en la historia y RUS sin curador registrado");
    let texto = constancia(ctx, vec![ult]);
    if texto.es_vacio() {
        Vec::new()
    } else {
        vec![texto]
    }
}

/// Categorías distintas tras «Institución:» en una celda «(RUT)NOMBRE - Institución: X#(RUT)…».
fn instituciones(celda: &str) -> usize {
    let mut categorias: Vec<String> = Vec::new();
    for entrada in celda.split('#') {
        let entrada = entrada.trim();
        let bajo = entrada.to_lowercase();
        let posicion = bajo.find("institución:").map(|p| p + "institución:".len());
        let posicion = posicion.or_else(|| bajo.find("institucion:").map(|p| p + "institucion:".len()));
        if let Some(p) = posicion {
            let resto = entrada.get(p..).unwrap_or("").trim_start();
            let resto = resto.lines().next().unwrap_or("");
            if !resto.is_empty() {
                let c = clave(resto);
                if !categorias.contains(&c) {
                    categorias.push(c);
                }
            }
        }
    }
    categorias.len()
}

pub fn curador_instituciones(ctx: &mut Ctx, celda: &str, pnombre: &str) -> Vec<Frag> {
    let cantidad = instituciones(celda);
    if cantidad < 2 {
        return Vec::new();
    }
    ctx.marcar(1, "T5_INSTITUCIONES", "Curador con varias instituciones en RUS");
    vec![ctx.render("HISTORIA", "T5_INSTITUCIONES", &[("N", cantidad.to_string()), ("PNOMBRE", pnombre.to_string())])]
}

fn ultimo_nuevo(ctx: &mut Ctx, nombre: &str) -> Option<Tramite> {
    let ult = ultimo(&refs(&nuevos(ctx, &[nombre]))).cloned()?;
    ctx.emitidos.insert(ult.id());
    Some(ult)
}

/// Espera: resolución que dispone el ingreso al programa («Ing.Serv. …»); el programa es el de la fila de RUS, por
/// eso solo se dice en causas de un registro.
pub fn servicio(ctx: &mut Ctx, programa: &str) -> Vec<Frag> {
    if varios_registros(ctx) {
        return Vec::new();
    }
    let Some(ult) = ultimo_nuevo(ctx, SERVICIO) else { return Vec::new() };
    let [folio, fecha] = folio_fecha(&ult);
    let texto = ctx.render("HISTORIA", "SERVICIO_INGRESO", &[("PROGRAMA", titulo_programa(programa)), folio, fecha]);
    vec![cita(texto, &ult)]
}

pub fn continuidad(ctx: &mut Ctx) -> Vec<Frag> {
    let Some(ult) = ultimo_nuevo(ctx, CONTINUIDAD) else { return Vec::new() };
    let texto = ctx.render("HISTORIA", "CONTINUIDAD_ATENCION", &folio_fecha(&ult));
    vec![cita(texto, &ult)]
}

/// Cumplimiento DCE con la medida vencida: informe clínico diagnóstico ordenado en autos.
pub fn solicitud_dce(ctx: &mut Ctx, programa: &str, vencida: bool) -> Vec<Frag> {
    if !vencida || intervencion(programa) != Intervencion::Dce || varios_registros(ctx) {
        return Vec::new();
    }
    let Some(ult) = ultimo_nuevo(ctx, SOLICITUD_DCE) else { return Vec::new() };
    let texto = ctx.render("HISTORIA", "SOLICITUD_DCE", &folio_fecha(&ult));
    vec![cita(texto, &ult)]
}

pub fn marcas_sin_efecto(ctx: &mut Ctx) {
    if !ctx.tramites(&[SIN_EFECTO_DR]).is_empty() {
        ctx.marcar(2, "DEJA_SIN_EFECTO_DR", "Deja sin efecto derivación D.R. en la historia");
    }
    if !ctx.tramites(&[SIN_EFECTO_MN]).is_empty() {
        ctx.marcar(2, "DEJA_SIN_EFECTO_MN", "Deja sin efecto medida interconexión M.N. en la historia");
    }
    tramites_sin_regla(ctx);
}

/// Deja rastro de lo que el motor no reconoce: trámites nuevos de la ventana cuyo nombre no está en el vocabulario.
/// Solo marca (nivel 3); nunca va al texto para RUS.
fn tramites_sin_regla(ctx: &mut Ctx) {
    let Some(h) = ctx.h() else { return };
    let conocidas: Vec<String> = CONOCIDAS.iter().chain(IGNORADAS.iter()).map(|n| clave(n)).collect();
    let mut nombres: Vec<(String, String)> = Vec::new();
    for t in &h.tramites {
        if h.en_ventana(t) && !conocidas.contains(&t.clave) && novedad(ctx, t) {
            match nombres.iter_mut().find(|(c, _)| *c == t.clave) {
                Some(par) => par.1 = t.desc.clone(),
                None => nombres.push((t.clave.clone(), t.desc.clone())),
            }
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

/// «folio 12», «folio n 12» o «folio no 12» como palabra, sin más dígitos después.
fn cita_folio(texto: &str, folio: &str) -> bool {
    let mut desde = 0;
    while let Some(p) = texto[desde..].find("folio ") {
        let inicio = desde + p;
        let previo = texto[..inicio].chars().next_back();
        if previo.is_none_or(|c| !(c.is_alphanumeric() || c == '_')) {
            let resto = &texto[inicio + 6..];
            for candidato in [Some(resto), resto.strip_prefix("n "), resto.strip_prefix("no ")].into_iter().flatten() {
                if let Some(cola) = candidato.strip_prefix(folio) {
                    if !cola.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                        return true;
                    }
                }
            }
        }
        desde = inicio + 6;
    }
    false
}

/// Una observación de la bitácora, posterior al trámite, ya lo mencionó: habla de la materia y cita su folio o fecha.
fn en_bitacora(ctx: &Ctx, tramite: &Tramite, materia: &str) -> bool {
    let fecha = clave(&fecha_es(tramite.fecha));
    ctx.bitacora.iter().filter(|o| o.fecha >= tramite.fecha).any(|o| {
        let texto = clave(&o.texto);
        texto.contains(materia) && (texto.contains(&fecha) || (!tramite.folio_txt.is_empty() && cita_folio(&texto, &tramite.folio_txt)))
    })
}

const CONECTORES: [&str; 2] = ["Posteriormente, ", "Luego, "];

fn frase_busqueda(ctx: &mut Ctx, tramite: &Tramite, posicion: usize) -> Frag {
    let llave = if tramite.clave == clave(PIDE_CUENTA_BUSQUEDA) {
        "R11_PIDE_CUENTA"
    } else if tramite.clave == clave(SIN_EFECTO_BUSQUEDA) {
        "R11_SIN_EFECTO"
    } else {
        "R11_TERMINO"
    };
    let frag = ctx.render("HISTORIA", llave, &folio_fecha(tramite));
    let Some(actual) = CONECTORES.iter().find(|c| frag.texto.starts_with(**c)) else { return frag };
    let deseado = CONECTORES[usize::from(posicion != 0)];
    Frag::simple(format!("{deseado}{}", &frag.texto[actual.len()..]), 'C')
}

/// Órdenes de búsqueda vigentes: las que ningún cierre tiene como su orden más cercana (un término cierra una sola
/// orden). Cuenta también el último trámite anterior a la ventana conservado como contexto.
fn busquedas_vigentes(ctx: &Ctx) -> Vec<Tramite> {
    let ordenes = ctx.tramites_con(&[ORDEN_BUSQUEDA], true);
    let cierres = ctx.tramites_con(&[TERMINO_BUSQUEDA, SIN_EFECTO_BUSQUEDA], true);
    let cerradas: Vec<&Tramite> =
        cierres.iter().filter_map(|c| ultimo(&ordenes.iter().filter(|o| c.posterior_a(o)).collect::<Vec<_>>())).collect();
    ordenes.iter().filter(|o| !cerradas.contains(o)).cloned().collect()
}

/// R11: primero la orden de búsqueda y luego sus trámites. Con bitácora, se dice si su último trámite es novedad; sin
/// bitácora, si cae en los últimos `ventana_busqueda` días. La orden puede ser anterior a la ventana (contexto).
pub fn busqueda_historia(ctx: &mut Ctx) -> Vec<Frag> {
    let Some(h) = ctx.h() else { return Vec::new() };
    let h = h.clone();
    if !busquedas_vigentes(ctx).is_empty() {
        ctx.marcar(1, "R11_BUSQUEDA_VIGENTE", "Orden de búsqueda vigente");
    }
    let desde = ctx.hoy - Duration::days(ctx.umbral("ventana_busqueda"));
    let con_bitacora = ultima_observacion(ctx).is_some();
    let reciente = |ctx: &Ctx, t: &Tramite| h.en_ventana(t) && if con_bitacora { novedad(ctx, t) } else { t.fecha >= desde };
    let ordenes = ctx.tramites_con(&[ORDEN_BUSQUEDA], true);
    let seguimiento = ctx.tramites(&[PIDE_CUENTA_BUSQUEDA, SIN_EFECTO_BUSQUEDA, TERMINO_BUSQUEDA]);
    let mut cadenas: Vec<Vec<Tramite>> = ordenes.iter().map(|o| vec![o.clone()]).collect();
    let mut huerfanos = Vec::new();
    for t in &seguimiento {
        // Órdenes con el mismo identificador comparten cadena (la última gana, como en el diccionario original).
        let mut mejor: Option<usize> = None;
        for (i, o) in ordenes.iter().enumerate() {
            if t.posterior_a(o) && mejor.is_none_or(|m| (o.fecha, o.folio) > (ordenes[m].fecha, ordenes[m].folio)) {
                mejor = Some(i);
            }
        }
        match mejor {
            Some(i) => {
                let destino = ordenes.iter().rposition(|o| o.id() == ordenes[i].id()).unwrap_or(i);
                cadenas[destino].push(t.clone());
            }
            None => huerfanos.push(t.clone()),
        }
    }
    let mut salida = Vec::new();
    for (i, orden) in ordenes.iter().enumerate() {
        let destino = ordenes.iter().rposition(|o| o.id() == orden.id()).unwrap_or(i);
        let cadena = cadenas[destino].clone();
        let final_cadena = cadena.last().unwrap();
        if !reciente(ctx, final_cadena) || ctx.emitidos.contains(&orden.id()) || en_bitacora(ctx, final_cadena, "busqueda") {
            continue;
        }
        ctx.emitidos.extend(cadena.iter().map(Tramite::id));
        let texto = ctx.render("HISTORIA", "R11_ORDEN", &folio_fecha(orden));
        if texto.es_vacio() {
            continue;
        }
        salida.push(texto);
        for (n, t) in cadena[1..].iter().enumerate() {
            let frase = frase_busqueda(ctx, t, n);
            if !frase.es_vacio() {
                salida.push(frase);
            }
        }
    }
    for t in huerfanos {
        if reciente(ctx, &t) && !en_bitacora(ctx, &t, "busqueda") {
            let texto = constancia(ctx, vec![t]);
            if !texto.es_vacio() {
                salida.push(texto);
            }
        }
    }
    salida
}

/// Una observación anterior de la bitácora ya dejó constancia del oído en esa fecha.
pub fn oido_en_bitacora(ctx: &Ctx, fecha: NaiveDate) -> bool {
    let formas = [clave(&fecha_es(fecha)), fecha.format("%d/%m/%Y").to_string()];
    ctx.bitacora.iter().any(|o| {
        let texto = clave(&o.texto);
        texto.contains("oido") && formas.iter().any(|f| texto.contains(f.as_str()))
    })
}

/// Solo si RUS no tiene fecha de oído. La historia acredita que se aplicó la nomenclatura en la causa, no la fecha en
/// que la persona fue oída: se cita como trámite.
pub fn derecho_oido(ctx: &mut Ctx) -> Vec<Frag> {
    let lista = ctx.tramites(&[DERECHO_OIDO]);
    let Some(ult) = ultimo(&refs(&lista)).cloned() else { return Vec::new() };
    if oido_en_bitacora(ctx, ult.fecha) || !novedad(ctx, &ult) {
        return Vec::new();
    }
    let texto = constancia(ctx, vec![ult]);
    if texto.es_vacio() {
        Vec::new()
    } else {
        vec![texto]
    }
}

/// Prórroga: la historia acredita el trámite, no su término; se cita como trámite y el egreso de RUS se dice aparte.
/// Con la medida vencida, por vencer o sin reflejar en RUS solo se dice si es reciente y la causa tiene un registro.
pub fn prorroga(
    ctx: &mut Ctx,
    corte: NaiveDate,
    egreso: Option<NaiveDate>,
    vencida: bool,
    por_vencer: bool,
    egreso_dicho: bool,
) -> Vec<Frag> {
    let lista: Vec<Tramite> = ctx.tramites(&[PRORROGA]).into_iter().filter(|t| !ctx.emitidos.contains(&t.id())).collect();
    let (Some(ult), Some(egreso)) = (ultimo(&refs(&lista)).cloned(), egreso) else { return Vec::new() };
    if en_bitacora(ctx, &ult, "prorrog") {
        return Vec::new();
    }
    let reciente = (corte - ult.fecha).num_days() <= ctx.umbral("prorroga_reciente");
    let sin_reflejar = egreso < ult.fecha;
    let pendiente = vencida || por_vencer || sin_reflejar;
    if reciente && pendiente {
        ctx.marcar(1, "PRORROGA_REVISAR", "Prórroga reciente con medida vencida o por vencer: verificar su registro en RUS");
    }
    // La marca avisa de un registro pendiente en RUS y se mantiene; la frase solo se dice si el trámite es novedad.
    if !novedad(ctx, &ult) || (pendiente && (!reciente || varios_registros(ctx))) {
        ctx.emitidos.extend(lista.iter().map(Tramite::id));
        return Vec::new();
    }
    let mut salida = vec![constancia(ctx, vec![ult])];
    ctx.emitidos.extend(lista.iter().map(Tramite::id));
    if !pendiente && !egreso_dicho && !varios_registros(ctx) {
        salida.push(ctx.render("HISTORIA", "EGRESO_PROYECTADO", &[("FECHA_EGRESO_PROYECTADO", fecha_es(egreso))]));
    }
    salida.retain(|f| !f.es_vacio());
    salida
}

/// C10/I3: último escrito de informe sin «Ingreso informe de cumplimiento X» posterior. Con varios registros no se
/// sabe de quién es el informe y no se propone el proyecto. `folio`: asociación explícita con una proyección.
pub fn escrito_sin_ingreso(ctx: &mut Ctx, programa: &str, desde: Option<NaiveDate>, folio: &str) -> Vec<Frag> {
    if varios_registros(ctx) && folio.is_empty() {
        return Vec::new();
    }
    let mut candidatos = escrito_de(intervencion(programa)).map(|n| ctx.tramites(&[n])).unwrap_or_default();
    if !folio.is_empty() {
        candidatos.retain(|t| t.folio_txt == folio);
    } else if let Some(d) = desde {
        candidatos.retain(|t| t.fecha >= d);
    }
    let Some(escrito) = ultimo(&refs(&candidatos)).cloned() else { return Vec::new() };
    if ctx.emitidos.contains(&escrito.id()) || ctx.tramites(&[INGRESO_INFORME_X]).iter().any(|i| i.posterior_a(&escrito)) {
        return Vec::new();
    }
    ctx.emitidos.insert(escrito.id());
    let texto = ctx.render(
        "HISTORIA",
        "C10_ESCRITO",
        &[("FOLIO_ESCRITO", escrito.folio_visible().to_string()), ("FECHA_ESCRITO", fecha_es(escrito.fecha))],
    );
    vec![cita(texto, &escrito)]
}

/// Informes: «Ingreso informe de cumplimiento X». Vencido: solo cuenta uno del día del vencimiento o posterior. Si
/// no se puede atribuir a este informe (varios registros o varios vencidos), se cita como trámite de la causa.
/// Atribuido y con RUS aún vencido (el calendario marca pendiente), se informa lo que consta y se sugiere revisar su
/// asociación con la fecha proyectada, sin pedir de nuevo el informe. `folio`: asociación explícita con la proyección.
pub fn informe_historia(ctx: &mut Ctx, vencimiento: NaiveDate, vencido: bool, atribuible: bool, programa: &str, folio: &str) -> Vec<Frag> {
    let mut lista: Vec<Tramite> = ctx.tramites(&[INGRESO_INFORME_X]);
    if !folio.is_empty() {
        lista.retain(|t| t.folio_txt == folio);
    } else if vencido {
        lista.retain(|t| t.fecha >= vencimiento);
    }
    lista.retain(|t| !ctx.emitidos.contains(&t.id()));
    let Some(ult) = ultimo(&refs(&lista)).cloned() else { return Vec::new() };
    if !atribuible && folio.is_empty() {
        let texto = if novedad(ctx, &ult) { constancia(ctx, vec![ult]) } else { Frag::vacio() };
        ctx.emitidos.extend(lista.iter().map(Tramite::id));
        return if texto.es_vacio() { Vec::new() } else { vec![texto] };
    }
    ctx.emitidos.extend(lista.iter().map(Tramite::id));
    if vencido {
        let texto = ctx.render(
            "HISTORIA",
            "INFORME_REVISAR_ASOCIACION",
            &[
                ("PROGRAMA", titulo_programa(programa)),
                ("FOLIO", ult.folio_visible().to_string()),
                ("FECHA_PROYECTADA", fecha_es(vencimiento)),
            ],
        );
        if !texto.es_vacio() {
            return vec![texto];
        }
    }
    let texto = ctx.render("HISTORIA", "INFORME_INGRESADO", &folio_fecha(&ult));
    vec![cita(texto, &ult)]
}

/// Presencia de un ingreso o escrito anterior al vencimiento, sin inferir a qué período corresponde.
pub fn informe_previo(ctx: &Ctx, programa: &str, vencimiento: NaiveDate) -> bool {
    let mut nombres = vec![INGRESO_INFORME_X];
    nombres.extend(escrito_de(intervencion(programa)));
    ctx.tramites(&nombres).iter().any(|t| t.fecha < vencimiento)
}

/// El pre-pase encontró más de un informe vencido (fechas distintas) del mismo ingreso.
pub fn varios_vencidos(ctx: &Ctx) -> bool {
    ctx.acumulado.is_some_and(|(n, _)| n >= 2)
}

/// J2: informes vencidos acumulados del mismo ingreso.
pub fn informes_acumulados(ctx: &mut Ctx) -> Vec<Frag> {
    match ctx.acumulado {
        Some((n, fecha)) if n >= 2 => {
            vec![ctx.render("HISTORIA", "J2_ACUMULADOS", &[("N", n.to_string()), ("FECHA", fecha_es(fecha))])]
        }
        _ => Vec::new(),
    }
}

pub fn egreso_posterior_mayoria(ctx: &mut Ctx, nacimiento: Option<NaiveDate>, egreso: Option<NaiveDate>, mayoria_dicha: bool) -> Vec<Frag> {
    let (Some(nac), Some(egreso)) = (nacimiento, egreso) else { return Vec::new() };
    let mayoria = fecha_mayoria(nac);
    if egreso <= mayoria {
        return Vec::new();
    }
    if mayoria_dicha {
        vec![ctx.render("HISTORIA", "EGRESO_POSTERIOR_MAYORIA", &[])]
    } else {
        vec![ctx.render("HISTORIA", "EGRESO_POSTERIOR_MAYORIA_FECHA", &[("FECHA_MAYORIA", fecha_es(mayoria))])]
    }
}

/// Causa P con conciliación o sentencia y sin egreso de servicio en esa fecha o posterior. El cierre puede ser el
/// último trámite anterior a la ventana conservado como contexto.
pub fn causa_finalizada(ctx: &mut Ctx, rit: &str) -> Vec<Frag> {
    if !rit.trim().to_uppercase().starts_with('P') {
        return Vec::new();
    }
    let Some(cierre) = ultimo(&refs(&ctx.tramites_con(&[CONCILIACION, DICTACION_SENTENCIA], true))).cloned() else {
        return Vec::new();
    };
    if ctx.tramites_con(&[EGRESO_SERVICIO], true).iter().any(|t| t.fecha >= cierre.fecha) {
        return Vec::new();
    }
    vec![ctx.render("HISTORIA", "P_FINALIZADA", &[])]
}

/// La medida vencida sigue siendo afirmable (sin contradicción entre días negativos y egreso futuro).
pub fn vencida_valida(ctx: &Ctx, vencida: bool) -> bool {
    vencida && !ctx.invalidas.contains(&"CUMPLIMIENTO.C04_VENCIDA")
}

/// Todo lo emitido hasta ahora en la fila proviene de reglas de historia.
pub fn solo_historia(ctx: &Ctx) -> bool {
    let mut eventos = ctx.eventos.iter().filter(|e| !ctx.desactivada(e)).peekable();
    eventos.peek().is_some() && eventos.all(|e| e.starts_with("HISTORIA."))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn folio_citado() {
        assert!(cita_folio("consta en folio 12 la prorroga", "12"));
        assert!(cita_folio("folio no 12.", "12"));
        assert!(!cita_folio("folio 123", "12"));
        assert!(!cita_folio("portafolio 12", "12"));
    }

    #[test]
    fn cuenta_instituciones() {
        assert_eq!(instituciones("(1-9)ANA - Institución: CAJ#(2-7)LUIS - Institución: ODL"), 2);
        assert_eq!(instituciones("(1-9)ANA - Institución: CAJ#(2-7)LUIS - Institución: caj"), 1);
        assert_eq!(instituciones("NO POSEE"), 0);
    }
}

// --- Integración 18-nov-2026: otro programa vigente e informe de avance pendiente en Escritos ---------------------

/// Espera y Cumplimiento: constancia única de que el niño cuenta con ingreso efectivo a otro programa vigente
/// (otra denominación completa en el Ingreso Histórico Protección, egreso `--` y causal `SIN INFORMACION` en la misma
/// fila). No se repite si la bitácora (de cualquier etapa) ya la tiene o si el funcionario la marcó como consignada.
pub fn otro_programa(ctx: &mut Ctx, programa: &str, pnombre: &str) -> Vec<Frag> {
    let Some(h) = ctx.historico.clone() else { return Vec::new() };
    if h.cobertura == "FALLIDA" {
        ctx.marcar(3, "HISTORICO_NO_DISPONIBLE", "Ingreso Histórico Protección no se pudo leer: no se verificó si hay otro programa vigente");
        return Vec::new();
    }
    if evidencia::con_filas(&h.cobertura) && h.identidad != "VERIFICADA" {
        ctx.marcar(2, "HISTORICO_IDENTIDAD", "Ingreso Histórico Protección sin pertenencia verificada a esta persona: no se afirma otro programa");
        return Vec::new();
    }
    let otros = programas_vigentes_otros(Some(&h), programa);
    if otros.is_empty() {
        return Vec::new();
    }
    if clave_texto(pnombre).is_empty() {
        ctx.marcar(2, "OTRO_PROGRAMA_SIN_NOMBRE", "Otro programa vigente en el histórico, pero no se pudo determinar el primer nombre: revisar esta fila");
        return Vec::new();
    }
    let mut salida = Vec::new();
    for nombre in otros {
        let llave = clave_programa(&nombre);
        if ctx.consignados.contains(&llave) || constancia_en_bitacora(&ctx.bitacora, pnombre, &nombre).is_some() {
            continue;
        }
        if ctx.bitacora_parcial {
            ctx.marcar(2, "OTRO_PROGRAMA_BITACORA_PARCIAL", &format!("Bitácora incompleta: no se puede descartar que el ingreso efectivo a «{nombre}» ya se haya consignado"));
        } else if mencion_previa(&ctx.bitacora, &nombre).is_some() {
            ctx.marcar(3, "OTRO_PROGRAMA_MENCION_PREVIA", &format!("La bitácora menciona «{nombre}» sin una constancia reconocible: revisar si ya se consignó"));
        }
        let texto = ctx.render("HISTORIA", "OTRO_PROGRAMA", &[("PNOMBRE", pnombre.to_string()), ("OTRO_PROGRAMA", nombre.clone())]);
        if !texto.es_vacio() {
            salida.push(texto);
            ctx.otros_programas.push(OtroPrograma { clave: llave, programa: nombre });
        }
    }
    salida
}

fn es_pendiente(programa: &str, e: &Escrito) -> bool {
    escrito_de(intervencion(programa)).is_some_and(|n| clave_texto(&e.tipo) == clave_texto(n) && clave_texto(&e.estado) == ESTADO_PENDIENTE)
}

/// Escritos de la causa con el tipo de la modalidad del programa y estado Pendiente en la misma fila. Una bandeja
/// vacía comprobada, no solicitada o fallida no contiene nada: no se afirma ausencia.
pub fn escritos_pendientes(ctx: &Ctx, programa: &str) -> Vec<Escrito> {
    match &ctx.escritos {
        Some(e) if e.con_filas() => e.filas.iter().filter(|f| es_pendiente(programa, f)).cloned().collect(),
        _ => Vec::new(),
    }
}

/// Informe de avance pendiente de resolver, atribuible a este informe (asociación explícita con la fecha proyectada o
/// único candidato en una causa sin otros vencidos). Si la historia acredita algo posterior a la lectura de la bandeja
/// prevalece la historia; con estados incompatibles sin secuencia fiable, revisión.
pub fn escrito_pendiente(ctx: &mut Ctx, programa: &str, vencimiento: Option<NaiveDate>, atribuible: bool) -> Option<Escrito> {
    let candidatos = escritos_pendientes(ctx, programa);
    if candidatos.is_empty() {
        return None;
    }
    let propio = candidatos.iter().find(|c| c.vinculo == "EXPLICITO" && vencimiento.is_some() && c.fecha_proyectada == vencimiento).cloned();
    let elegido = propio.clone().or_else(|| {
        (atribuible && candidatos.len() == 1 && (vencimiento.is_none() || candidatos[0].vinculo != "EXPLICITO")).then(|| candidatos[0].clone())
    });
    let Some(elegido) = elegido else {
        ctx.marcar(2, "ESCRITO_PENDIENTE_SIN_ATRIBUIR", "Escrito de informe pendiente en la causa que no se puede atribuir a este informe: se conservan las acciones");
        return None;
    };
    if propio.is_some() {
        ctx.marcar(2, "ESCRITO_PENDIENTE", "Informe de avance pendiente de resolver en la bandeja de Escritos");
        return Some(elegido);
    }
    let mut nombres = vec![INGRESO_INFORME_X];
    nombres.extend(escrito_de(intervencion(programa)));
    let en_historia: Vec<Tramite> = ctx.tramites(&nombres).into_iter().filter(|t| vencimiento.is_none_or(|v| t.fecha >= v)).collect();
    if !en_historia.is_empty() {
        let lectura = ctx.escritos.as_ref().and_then(|e| e.leido);
        let hasta = ctx.h().and_then(|h| h.hasta);
        if let (Some(l), Some(hh)) = (lectura, hasta) {
            if hh > l {
                return None; // la incorporación posterior acreditada en Historia prevalece sobre la bandeja
            }
        }
        ctx.marcar(2, "ESCRITO_ESTADOS_INCOMPATIBLES", "Escrito pendiente en la bandeja y escrito ya incorporado en la historia sin secuencia fiable: revisar");
        return None;
    }
    ctx.marcar(2, "ESCRITO_PENDIENTE", "Informe de avance pendiente de resolver en la bandeja de Escritos");
    Some(elegido)
}

/// Fragmentos de la regla (una sola frase, una vez). Vacío si no corresponde.
pub fn informe_pendiente(ctx: &mut Ctx, programa: &str, vencimiento: Option<NaiveDate>, atribuible: bool) -> Vec<Frag> {
    if escrito_pendiente(ctx, programa, vencimiento, atribuible).is_none() {
        return Vec::new();
    }
    let texto = ctx.render("HISTORIA", "INFORME_AVANCE_PENDIENTE", &[]);
    if texto.es_vacio() {
        Vec::new()
    } else {
        vec![texto]
    }
}
