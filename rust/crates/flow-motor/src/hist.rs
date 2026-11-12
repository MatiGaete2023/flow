//! Constancias de la historia de la causa (`motor/reglas_historia.py` y `motor/analisis_historia.py`).
//! Cada función devuelve fragmentos (vacío si no aplica).

use crate::ctx::{Ctx, Duplicidad, Frag};
use chrono::{Duration, NaiveDate};
use flow_dominio::historia::ultimo;
use flow_dominio::texto::{clave, contiene_token, es_dce, fecha_es, fecha_mayoria, normalizar, titulo_programa};
use flow_dominio::Tramite;

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

/// Constancias que citan trámites: con ellas, una causa de varios registros lleva la aclaración final.
const CITAN: [&str; 15] = [
    "HISTORIA.BASE_TRAMITE",
    "HISTORIA.VARIOS",
    "HISTORIA.E04_CON_FOLIO",
    "HISTORIA.C2_PRORROGA_HASTA",
    "HISTORIA.SERVICIO_INGRESO",
    "HISTORIA.CURADOR_SIN_EFECTO",
    "HISTORIA.CURADOR_DESIGNADO",
    "HISTORIA.R11_ORDEN",
    "HISTORIA.R11_PIDE_CUENTA",
    "HISTORIA.R11_SIN_EFECTO",
    "HISTORIA.R11_TERMINO",
    "HISTORIA.SOLICITUD_DCE",
    "HISTORIA.CONTINUIDAD_ATENCION",
    "HISTORIA.INFORME_INGRESADO",
    "HISTORIA.C10_ESCRITO",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Materia {
    Curador,
    SinFicha,
    SinFae,
    Vencida,
}

impl Materia {
    fn original(self) -> (&'static str, &'static str) {
        match self {
            Materia::Curador => ("COMUN.CURADOR", "MATERIA_CURADOR"),
            Materia::SinFicha => ("CUMPLIMIENTO.C07_SIN_FICHA", "MATERIA_SIN_FICHA"),
            Materia::SinFae => ("CUMPLIMIENTO.C08_FICHA_FAE", "MATERIA_SIN_FAE"),
            Materia::Vencida => ("CUMPLIMIENTO.C04_VENCIDA", "MATERIA_VENCIDA"),
        }
    }
    fn patron(self) -> &'static str {
        match self {
            Materia::Curador => "no registra curador ad litem",
            Materia::SinFicha => "no registra ficha individual",
            Materia::SinFae => "no registra ficha fae",
            Materia::Vencida => "medida vencida en rus",
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

/// Tramites aún no citados en esta fila.
fn pendientes(ctx: &Ctx, nombres: &[&str]) -> Vec<Tramite> {
    ctx.tramites(nombres).into_iter().filter(|t| !ctx.emitidos.contains(&t.id())).collect()
}

/// Texto base de uno o varios trámites del mismo nombre; omite los ya citados.
fn constancia(ctx: &mut Ctx, lista: Vec<Tramite>) -> Frag {
    let lista: Vec<Tramite> = lista.into_iter().filter(|t| !ctx.emitidos.contains(&t.id())).collect();
    let Some(ult) = ultimo(&refs(&lista)).cloned() else { return Frag::vacio() };
    ctx.emitidos.extend(lista.iter().map(Tramite::id));
    let mut datos = vec![("DESC", ult.desc.clone()), ("FECHA", fecha_es(ult.fecha)), ("FOLIO", ult.folio_visible().to_string())];
    if lista.len() == 1 {
        ctx.render("HISTORIA", "BASE_TRAMITE", &datos)
    } else {
        datos.push(("N", lista.len().to_string()));
        ctx.render("HISTORIA", "VARIOS", &datos)
    }
}

/// Regla 1: folio del trámite «Ing.Serv. …» fechado el día de la resolución de RUS; None si no hay o hay varios.
pub fn folio_resolucion(ctx: &mut Ctx, fecha: NaiveDate) -> Option<String> {
    let ingresos: Vec<Tramite> =
        ctx.tramites(&[SERVICIO]).into_iter().filter(|t| t.fecha == fecha && !t.folio_txt.is_empty()).collect();
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

pub fn lista_larga(ctx: &Ctx, programa: &str, espera: Option<i64>) -> bool {
    espera.is_some_and(|e| intervencion(programa) == Intervencion::Residencial && e > ctx.umbral("lista_espera_larga"))
}

pub fn lista_espera_larga(ctx: &mut Ctx, programa: &str, espera: Option<i64>) -> Vec<Frag> {
    if lista_larga(ctx, programa, espera) {
        vec![ctx.render("HISTORIA", "E10_LISTA", &[("N", espera.unwrap().to_string())])]
    } else {
        Vec::new()
    }
}

pub fn marcar_duplicidad(ctx: &mut Ctx) {
    if ctx.duplicidad == Some(Duplicidad::MismoPrograma) {
        ctx.marcar(1, "R6_DUPLICIDAD", "Más de un ingreso del mismo NNA al mismo programa");
    }
}

fn materias(texto: &str) -> Vec<Materia> {
    let limpio = clave(texto);
    if limpio.starts_with("se reitera") {
        return Vec::new();
    }
    [Materia::Curador, Materia::SinFicha, Materia::SinFae, Materia::Vencida]
        .into_iter()
        .filter(|m| limpio.contains(m.patron()))
        .collect()
}

/// REITERA: vacío si no hay observación previa de la materia o la regla original está desactivada.
pub fn reitera(ctx: &mut Ctx, materia: Materia) -> Frag {
    let (original, clave_materia) = materia.original();
    if ctx.desactivada(original) {
        return Frag::vacio();
    }
    let hoy = ctx.hoy;
    let Some(fecha) = ctx.bitacora.iter().find(|o| o.fecha < hoy && materias(&o.texto).contains(&materia)).map(|o| o.fecha)
    else {
        return Frag::vacio();
    };
    let texto_materia = ctx.render("HISTORIA", clave_materia, &[]).texto;
    let frag = ctx.render("HISTORIA", "REITERA", &[("FECHA_ANTERIOR", fecha_es(fecha)), ("MATERIA", texto_materia)]);
    if frag.es_vacio() {
        return frag;
    }
    ctx.eventos.push(original.to_string());
    // Reiterar la ausencia de curador o de ficha es gestión del Tribunal; reiterar el vencimiento es constancia.
    Frag::simple(frag.texto, if materia == Materia::Vencida { 'C' } else { 'T' })
}

/// Última designación o baja de curador de la historia, solo si RUS no registra curador.
pub fn curador_designaciones(ctx: &mut Ctx, rus_sin_curador: bool) -> Vec<Frag> {
    if !rus_sin_curador {
        return Vec::new();
    }
    let designa = ultimo(&refs(&ctx.tramites(&[DESIGNA_ABOGADO]))).cloned();
    let baja = ultimo(&refs(&ctx.tramites(&[SIN_EFECTO_CURADOR]))).cloned();
    let (ult, es_baja) = match (designa, baja) {
        (None, None) => return Vec::new(),
        (None, Some(b)) => (b, true),
        (Some(d), None) => (d, false),
        (Some(d), Some(b)) => {
            if d.posterior_a(&b) {
                (d, false)
            } else {
                (b, true)
            }
        }
    };
    if !ctx.emitidos.insert(ult.id()) {
        return Vec::new();
    }
    let llave = if es_baja { "CURADOR_SIN_EFECTO" } else { "CURADOR_DESIGNADO" };
    ctx.marcar(2, llave, "Curador ad litem en la historia y RUS sin curador registrado");
    vec![ctx.render("HISTORIA", llave, &folio_fecha(&ult))]
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

fn ultimo_pendiente(ctx: &mut Ctx, nombre: &str) -> Option<Tramite> {
    let ult = ultimo(&refs(&pendientes(ctx, &[nombre]))).cloned()?;
    ctx.emitidos.insert(ult.id());
    Some(ult)
}

/// Espera: resolución que dispone el ingreso al programa («Ing.Serv. …»).
pub fn servicio(ctx: &mut Ctx, programa: &str) -> Vec<Frag> {
    let Some(ult) = ultimo_pendiente(ctx, SERVICIO) else { return Vec::new() };
    let [folio, fecha] = folio_fecha(&ult);
    vec![ctx.render("HISTORIA", "SERVICIO_INGRESO", &[("PROGRAMA", titulo_programa(programa)), folio, fecha])]
}

pub fn continuidad(ctx: &mut Ctx) -> Vec<Frag> {
    let Some(ult) = ultimo_pendiente(ctx, CONTINUIDAD) else { return Vec::new() };
    vec![ctx.render("HISTORIA", "CONTINUIDAD_ATENCION", &folio_fecha(&ult))]
}

/// Cumplimiento DCE con la medida vencida: informe clínico diagnóstico ordenado en autos.
pub fn solicitud_dce(ctx: &mut Ctx, programa: &str, vencida: bool) -> Vec<Frag> {
    if !vencida || intervencion(programa) != Intervencion::Dce {
        return Vec::new();
    }
    let Some(ult) = ultimo_pendiente(ctx, SOLICITUD_DCE) else { return Vec::new() };
    vec![ctx.render("HISTORIA", "SOLICITUD_DCE", &folio_fecha(&ult))]
}

pub fn marcas_sin_efecto(ctx: &mut Ctx) {
    if !ctx.tramites(&[SIN_EFECTO_DR]).is_empty() {
        ctx.marcar(2, "DEJA_SIN_EFECTO_DR", "Deja sin efecto derivación D.R. en la historia");
    }
    if !ctx.tramites(&[SIN_EFECTO_MN]).is_empty() {
        ctx.marcar(2, "DEJA_SIN_EFECTO_MN", "Deja sin efecto medida interconexión M.N. en la historia");
    }
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
        texto.contains(materia)
            && (texto.contains(&fecha) || (!tramite.folio_txt.is_empty() && cita_folio(&texto, &tramite.folio_txt)))
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

/// R11: primero la orden de búsqueda y luego sus trámites; solo si el último es reciente y la bitácora no lo observó.
pub fn busqueda_historia(ctx: &mut Ctx) -> Vec<Frag> {
    if ctx.h().is_none() {
        return Vec::new();
    }
    let ordenes = ctx.tramites(&[ORDEN_BUSQUEDA]);
    let cierres = ctx.tramites(&[TERMINO_BUSQUEDA, SIN_EFECTO_BUSQUEDA]);
    if ordenes.iter().any(|o| !cierres.iter().any(|c| c.posterior_a(o))) {
        ctx.marcar(1, "R11_BUSQUEDA_VIGENTE", "Orden de búsqueda vigente");
    }
    let desde = ctx.hoy - Duration::days(ctx.umbral("ventana_busqueda"));
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
        if final_cadena.fecha < desde || ctx.emitidos.contains(&orden.id()) || en_bitacora(ctx, final_cadena, "busqueda") {
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
        if t.fecha >= desde && !en_bitacora(ctx, &t, "busqueda") {
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

/// Solo si RUS no tiene fecha de oído: usa la última aplicación de la historia.
pub fn derecho_oido(ctx: &mut Ctx) -> Vec<Frag> {
    let lista = ctx.tramites(&[DERECHO_OIDO]);
    let Some(ult) = ultimo(&refs(&lista)).cloned() else { return Vec::new() };
    if oido_en_bitacora(ctx, ult.fecha) {
        return Vec::new();
    }
    vec![ctx.render("COMUN", "OIDO", &[("FECHA_OIDO", fecha_es(ult.fecha))])]
}

/// Cumplimiento sin ingreso reciente: si la historia confirma el ingreso, se dice la fecha de RUS.
pub fn ingreso_informado(ctx: &mut Ctx, fecha_ingreso: Option<NaiveDate>) -> Vec<Frag> {
    let Some(inicio) = fecha_ingreso else { return Vec::new() };
    let limite = inicio + Duration::days(ctx.umbral("ventana_ingreso"));
    if !ctx.tramites(&[INGRESO_EFECTIVO]).iter().any(|t| inicio <= t.fecha && t.fecha <= limite) {
        return Vec::new();
    }
    vec![ctx.render("HISTORIA", "INGRESO_INFORMADO", &[("FECHA_INGRESO", fecha_es(inicio))])]
}

/// Prórroga: no se afirma con la medida vencida, por vencer o si RUS no la refleja; reciente → marca prioritaria.
pub fn prorroga(
    ctx: &mut Ctx,
    corte: NaiveDate,
    egreso: Option<NaiveDate>,
    vencida: bool,
    por_vencer: bool,
    egreso_dicho: bool,
) -> Vec<Frag> {
    let lista = pendientes(ctx, &[PRORROGA]);
    let (Some(ult), Some(egreso)) = (ultimo(&refs(&lista)).cloned(), egreso) else { return Vec::new() };
    if en_bitacora(ctx, &ult, "prorrog") {
        return Vec::new();
    }
    ctx.emitidos.extend(lista.iter().map(Tramite::id));
    let reciente = (corte - ult.fecha).num_days() <= ctx.umbral("prorroga_reciente");
    let sin_reflejar = egreso < ult.fecha;
    if reciente && (vencida || por_vencer || sin_reflejar) {
        ctx.marcar(
            1,
            "PRORROGA_REVISAR",
            "Prórroga reciente con medida vencida o por vencer: verificar su registro en RUS",
        );
    }
    if vencida || por_vencer || sin_reflejar || egreso_dicho {
        return Vec::new();
    }
    let [folio, fecha] = folio_fecha(&ult);
    vec![ctx.render("HISTORIA", "C2_PRORROGA_HASTA", &[folio, fecha, ("FECHA_EGRESO_PROYECTADO", fecha_es(egreso))])]
}

/// C10/I3: último escrito de informe sin «Ingreso informe de cumplimiento X» posterior.
pub fn escrito_sin_ingreso(ctx: &mut Ctx, programa: &str, desde: Option<NaiveDate>) -> Vec<Frag> {
    let mut candidatos = escrito_de(intervencion(programa)).map(|n| ctx.tramites(&[n])).unwrap_or_default();
    if let Some(d) = desde {
        candidatos.retain(|t| t.fecha >= d);
    }
    let Some(escrito) = ultimo(&refs(&candidatos)).cloned() else { return Vec::new() };
    if ctx.emitidos.contains(&escrito.id()) || ctx.tramites(&[INGRESO_INFORME_X]).iter().any(|i| i.posterior_a(&escrito)) {
        return Vec::new();
    }
    ctx.emitidos.insert(escrito.id());
    vec![ctx.render(
        "HISTORIA",
        "C10_ESCRITO",
        &[("FOLIO_ESCRITO", escrito.folio_visible().to_string()), ("FECHA_ESCRITO", fecha_es(escrito.fecha))],
    )]
}

/// Informes: «Ingreso informe de cumplimiento X». Vencido: solo cuenta uno del día del vencimiento o posterior.
pub fn informe_historia(ctx: &mut Ctx, vencimiento: NaiveDate, vencido: bool) -> Vec<Frag> {
    let mut lista = pendientes(ctx, &[INGRESO_INFORME_X]);
    if vencido {
        lista.retain(|t| t.fecha >= vencimiento);
    }
    let Some(ult) = ultimo(&refs(&lista)).cloned() else { return Vec::new() };
    ctx.emitidos.extend(lista.iter().map(Tramite::id));
    vec![ctx.render("HISTORIA", "INFORME_INGRESADO", &folio_fecha(&ult))]
}

/// Presencia de un ingreso o escrito anterior al vencimiento, sin inferir a qué período corresponde.
pub fn informe_previo(ctx: &Ctx, programa: &str, vencimiento: NaiveDate) -> bool {
    let mut nombres = vec![INGRESO_INFORME_X];
    nombres.extend(escrito_de(intervencion(programa)));
    ctx.tramites(&nombres).iter().any(|t| t.fecha < vencimiento)
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

pub fn egreso_posterior_mayoria(
    ctx: &mut Ctx,
    nacimiento: Option<NaiveDate>,
    egreso: Option<NaiveDate>,
    mayoria_dicha: bool,
) -> Vec<Frag> {
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

/// La historia es de la causa: con varios registros en la nómina no se atribuye a un ingreso.
pub fn aclaracion_causa(ctx: &mut Ctx) -> Vec<Frag> {
    if ctx.duplicidad.is_none() {
        return Vec::new();
    }
    if !ctx.eventos.iter().any(|e| CITAN.contains(&e.as_str()) && !ctx.desactivada(e)) {
        return Vec::new();
    }
    let texto = ctx.render("HISTORIA", "R3_MULTI", &[]);
    if texto.es_vacio() {
        Vec::new()
    } else {
        vec![texto]
    }
}

/// Causa P con conciliación o sentencia y sin egreso de servicio en esa fecha o posterior.
pub fn causa_finalizada(ctx: &mut Ctx, rit: &str) -> Vec<Frag> {
    if !rit.trim().to_uppercase().starts_with('P') {
        return Vec::new();
    }
    let Some(cierre) = ultimo(&refs(&ctx.tramites(&[CONCILIACION, DICTACION_SENTENCIA]))).cloned() else {
        return Vec::new();
    };
    if ctx.tramites(&[EGRESO_SERVICIO]).iter().any(|t| t.fecha >= cierre.fecha) {
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
