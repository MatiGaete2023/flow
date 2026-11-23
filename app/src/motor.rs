//! Motor de observaciones: reglas tradicionales de Espera, Cumplimiento e Informes (Laja, Mulchén y Tomé).
//! Reescrito desde la hoja de reglas (MECHANICS.md). No escribe en RUS ni envía nada.

use crate::captura_trabajo::ContextoFila;
use crate::config::Config;
use crate::util::*;
use chrono::{Duration, NaiveDate};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

mod historia;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    Espera,
    Cumplimiento,
    Informes,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Espera, Mode::Cumplimiento, Mode::Informes];
    pub fn label(self) -> &'static str {
        match self {
            Mode::Espera => "Espera",
            Mode::Cumplimiento => "Cumplimiento",
            Mode::Informes => "Informes",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Mode::Espera => "espera",
            Mode::Cumplimiento => "cumplimiento",
            Mode::Informes => "informes",
        }
    }
}

/// Columna lógica → índice de columna del libro.
pub type Cols = HashMap<&'static str, usize>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Marca {
    pub nivel: u8,
    pub codigo: String,
    pub texto: String,
}

#[derive(Clone, Debug, Default)]
pub struct Resultado {
    pub observacion: String,
    pub eventos: Vec<String>,
    pub marcas: Vec<Marca>,
    pub avisos: Vec<String>,
}

/// Datos de toda la nómina que las reglas usan por fila.
#[derive(Clone, Debug, Default)]
pub struct Prepaso {
    /// "MISMO_PROGRAMA" u "OTRO_REGISTRO" por fila.
    pub duplicidad: HashMap<usize, &'static str>,
    /// Informes vencidos acumulados del mismo ingreso: (cantidad, el más antiguo).
    pub acumulado: HashMap<usize, (usize, NaiveDate)>,
}

#[derive(Clone, Debug)]
struct Frag {
    partes: Vec<(String, char)>,
    dato: Option<&'static str>,
}

impl Frag {
    fn grupo(&self) -> char {
        self.partes.first().map(|p| p.1).unwrap_or('C')
    }
    fn texto(&self) -> String {
        self.partes.iter().map(|p| p.0.as_str()).collect::<Vec<_>>().join(" ")
    }
}

fn estructura(key: &str) -> &'static str {
    match key {
        "ESPERA.E05_SOLO_CORREO" | "ESPERA.E05_PROYECTO_Y_CORREO" => "CA",
        "INFORMES.I01_VENCIDO_DCE" | "INFORMES.I01_VENCIDO_GENERAL" | "INFORMES.I02_MARGEN_DCE" | "HISTORIA.C10_ESCRITO" => "CA",
        "INFORMES.I02_POR_VENCER_DCE" | "INFORMES.I02_POR_VENCER_GENERAL" => "A",
        "COMUN.CURADOR" | "CUMPLIMIENTO.C07_SIN_FICHA" | "CUMPLIMIENTO.C07_FICHA_ANTIGUA" | "CUMPLIMIENTO.C08_FICHA_FAE" | "HISTORIA.T5_INSTITUCIONES" => "T",
        "COMUN.MAYORIA_EDAD" => "CT",
        _ => "C",
    }
}

fn dato(key: &str) -> Option<&'static str> {
    match key {
        "CUMPLIMIENTO.C03_INGRESO_RECIENTE" => Some("ingreso_efectivo"),
        "CUMPLIMIENTO.C04_VENCIDA" | "CUMPLIMIENTO.C05_VENCE_HOY" | "CUMPLIMIENTO.C05_POR_VENCER" | "HISTORIA.MEDIDA_SIN_VIGENCIA" | "HISTORIA.EGRESO_PROYECTADO" => Some("vencimiento"),
        _ => None,
    }
}

fn oraciones(t: &str) -> Vec<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = R.get_or_init(|| Regex::new(r"\.\s+").unwrap());
    let mut out = Vec::new();
    let mut last = 0;
    for m in r.find_iter(t.trim()) {
        out.push(t.trim()[last..m.start() + 1].to_string());
        last = m.end();
    }
    let rest = &t.trim()[last..];
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}

fn format_map(plantilla: &str, vars: &[(&str, String)]) -> String {
    let mut s = plantilla.to_string();
    for (k, v) in vars {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

struct Ctx<'a> {
    cfg: &'a Config,
    hoy: NaiveDate,
    eventos: Vec<String>,
    invalidas: BTreeSet<String>,
    marcas: Vec<Marca>,
    avisos: Vec<String>,
    duplicidad: Option<&'static str>,
    acumulado: Option<(usize, NaiveDate)>,
    captura: Option<&'a ContextoFila>,
}

impl<'a> Ctx<'a> {
    fn p(&self, k: &str) -> i64 {
        self.cfg.umbral(k)
    }

    fn render(&mut self, key: &str, vars: &[(&str, String)]) -> Option<Frag> {
        if self.cfg.desactivadas.contains(key) || self.invalidas.contains(key) {
            return None;
        }
        self.eventos.push(key.to_string());
        let plantilla = self.cfg.texto(key);
        let letras: Vec<char> = estructura(key).chars().collect();
        let partes = if letras.len() == 1 {
            vec![(format_map(&plantilla, vars), letras[0])]
        } else {
            let ors = oraciones(&plantilla);
            if ors.len() != letras.len() {
                vec![(format_map(&plantilla, vars), letras[0])]
            } else {
                ors.iter().zip(letras).map(|(o, l)| (format_map(o, vars), l)).collect()
            }
        };
        Some(Frag { partes, dato: dato(key) })
    }

    fn marcar(&mut self, nivel: u8, codigo: &str, texto: &str) {
        self.marcas.push(Marca { nivel, codigo: codigo.into(), texto: texto.into() });
    }

    fn aviso(&mut self, t: impl Into<String>) {
        self.avisos.push(t.into());
    }
}

fn normalizar_fragmento(t: &str) -> String {
    static D: OnceLock<Regex> = OnceLock::new();
    let s = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.is_empty() {
        return s;
    }
    let s = s.trim_end_matches(['.', ' ']).to_string() + ".";
    D.get_or_init(|| Regex::new(r"\.{2,}").unwrap()).replace_all(&s, ".").to_string()
}

/// Ordena por grupo (constancia, acción del Centro, Tribunal, cierre), dice cada dato una vez y no repite oraciones.
fn componer(pfx: &str, frags: &[Frag]) -> String {
    let orden = |g: char| match g {
        'C' => 0,
        'A' => 1,
        'T' => 2,
        _ => 3,
    };
    let mut piezas: Vec<(u8, usize, String)> = Vec::new();
    let mut dichos = BTreeSet::new();
    for f in frags {
        if let Some(d) = f.dato {
            if !dichos.insert(d) {
                continue;
            }
        }
        for (t, g) in &f.partes {
            let n = piezas.len();
            piezas.push((orden(*g), n, t.clone()));
        }
    }
    piezas.sort_by_key(|p| (p.0, p.1));
    let mut vistas = 0;
    let mut textos = Vec::new();
    for (_, _, t) in piezas {
        let t = if let Some(resto) = t.strip_prefix("Se hace presente que ") {
            vistas += 1;
            if vistas > 1 { format!("Asimismo, se hace presente que {resto}") } else { t }
        } else {
            t
        };
        let n = normalizar_fragmento(&t);
        if !n.is_empty() {
            textos.push(n);
        }
    }
    if textos.is_empty() {
        return String::new();
    }
    let texto = textos.join(" ").replace("..", ".");
    let mut vistas_or: Vec<String> = Vec::new();
    for o in oraciones(&texto) {
        if !vistas_or.contains(&o) {
            vistas_or.push(o);
        }
    }
    format!("{pfx}{}", vistas_or.join(" "))
}

/// E-04 declara «no requiere gestión del Tribunal»: sobra si la observación trae una acción del Centro o del Tribunal.
fn sin_parentesis_admin(frags: Vec<Frag>) -> Vec<Frag> {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = R.get_or_init(|| Regex::new(r"\s*\(Observación administrativa[^)]*\)\.?").unwrap());
    let con_gestion = frags.iter().any(|f| f.partes.iter().any(|p| p.1 == 'A' || p.1 == 'T'));
    if !con_gestion {
        return frags;
    }
    frags
        .into_iter()
        .map(|mut f| {
            if f.partes.len() == 1 && r.is_match(&f.partes[0].0) {
                f.partes[0].0 = r.replace_all(&f.partes[0].0, "").trim().to_string();
            }
            f
        })
        .collect()
}

/// Acceso a las celdas de una fila por columna lógica.
pub struct Fila<'a> {
    pub values: &'a [Cell],
    pub cols: &'a Cols,
}

impl<'a> Fila<'a> {
    pub fn has(&self, k: &str) -> bool {
        self.cols.contains_key(k)
    }
    pub fn cell(&self, k: &str) -> &Cell {
        static EMPTY: Cell = Cell::Empty;
        self.cols.get(k).and_then(|i| self.values.get(*i)).unwrap_or(&EMPTY)
    }
    pub fn text(&self, k: &str) -> String {
        self.cell(k).display().trim().to_string()
    }
    pub fn date(&self, k: &str) -> Option<NaiveDate> {
        if self.has(k) { get_date(self.cell(k)) } else { None }
    }
    pub fn int(&self, k: &str) -> Option<i64> {
        if self.has(k) { get_int(self.cell(k)) } else { None }
    }
}

fn audiencia(ctx: &mut Ctx, f: &Fila) -> Option<Frag> {
    if !f.has("prox_aud") {
        return None;
    }
    let (fechas, ilegibles) = fechas_varias(f.cell("prox_aud"));
    if ilegibles > 0 {
        ctx.aviso("próxima audiencia con fecha ilegible (G-05)");
    }
    let mut futuras: Vec<NaiveDate> = fechas.into_iter().filter(|d| *d >= ctx.hoy).collect();
    futuras.sort();
    futuras.dedup();
    if futuras.len() == 1 {
        return ctx.render("COMUN.PROX_AUDIENCIA", &[("FECHA_AUDIENCIA", fecha_es(futuras[0]))]);
    }
    if !futuras.is_empty() && !ctx.cfg.desactivadas.contains("COMUN.PROX_AUDIENCIA") {
        return ctx.render("COMUN.PROX_AUDIENCIAS", &[("FECHAS_AUDIENCIA", lista_fechas_es(&futuras))]);
    }
    None
}

fn oido(ctx: &mut Ctx, f: &Fila) -> Vec<Frag> {
    if let Some(fo) = f.date("oido") {
        let d = (ctx.hoy - fo).num_days();
        if 0 <= d && d <= ctx.p("oido") {
            return ctx.render("COMUN.OIDO", &[("FECHA_OIDO", fecha_es(fo))]).into_iter().collect();
        }
    }
    vec![]
}

fn curador(ctx: &mut Ctx, f: &Fila) -> Vec<Frag> {
    if !f.has("curador") {
        return vec![];
    }
    let celda = f.text("curador");
    if !tiene_curador_real(&celda) {
        return ctx.render("COMUN.CURADOR", &[]).into_iter().collect();
    }
    let n = instituciones(&celda);
    if n >= 2 {
        ctx.marcar(1, "T5_INSTITUCIONES", "Curador con varias instituciones en RUS");
        let pn = primer_nombre(&f.text("nombre"));
        return ctx.render("HISTORIA.T5_INSTITUCIONES", &[("N", n.to_string()), ("PNOMBRE", pn)]).into_iter().collect();
    }
    vec![]
}

fn marcar_duplicidad(ctx: &mut Ctx) {
    if ctx.duplicidad == Some("MISMO_PROGRAMA") {
        ctx.marcar(1, "R6_DUPLICIDAD", "Más de un ingreso del mismo NNA al mismo programa");
    }
}

fn espera(ctx: &mut Ctx, f: &Fila, trib: &str) -> String {
    let programa = f.text("programa");
    let nombre = f.text("nombre");
    let pfx = prefijo_observacion(&nombre, &programa);
    let pn = primer_nombre(&nombre);
    let prog = if ctx.cfg.redaccion_registro { titulo_programa_registro(&programa) } else { titulo_programa(&programa) };
    if es_derivacion_sin_seguimiento(&programa) {
        let fr: Vec<Frag> = ctx.render("COMUN.NO_SEGUIMIENTO", &[("PROGRAMA", prog)]).into_iter().collect();
        return componer(&pfx, &fr);
    }
    let mut frags: Vec<Frag> = Vec::new();
    let nac = f.date("nacimiento");
    if let Some(fnac) = nac {
        if edad(fnac, ctx.hoy) >= 18 {
            frags.extend(ctx.render("COMUN.MAYORIA_EDAD", &[("PNOMBRE", pn.clone()), ("FECHA_MAYORIA", fecha_es(fecha_mayoria(fnac)))]));
            ctx.marcar(3, "REVISAR_MAYORIA", "Mayoría de edad");
            return componer(&pfx, &frags);
        }
        let dm = (fecha_mayoria(fnac) - ctx.hoy).num_days();
        if 1 <= dm && dm <= ctx.p("mayoria") {
            frags.extend(ctx.render("COMUN.PROXIMA_MAYORIA", &[("PNOMBRE", pn.clone()), ("FECHA_MAYORIA", fecha_es(fecha_mayoria(fnac)))]));
        }
    }
    let mut principal = false;
    marcar_duplicidad(ctx);
    let espera = f.int("espera");
    historia::advertir_ingreso_efectivo(ctx, espera);
    let mut resolucion_reciente = false;
    if let Some(fres) = f.date("resolucion") {
        let d = (ctx.hoy - fres).num_days();
        if 0 <= d && d < ctx.p("resolucion_reciente") {
            frags.extend(historia::resolucion(ctx, &prog, fres));
            resolucion_reciente = true;
            principal = true;
        }
    }
    if f.has("espera") && espera.map_or(true, |e| e < 0) {
        ctx.aviso("tiempo de espera ausente o inválido (G-05)");
    }
    let dce = es_dce(&programa);
    let umbral = if dce {
        ctx.p("espera_dce")
    } else if ["LAJA", "MULCHEN", "TOME"].contains(&trib) {
        ctx.p(&format!("espera_{}", trib.to_lowercase()))
    } else {
        30
    };
    if let Some(e) = espera {
        if e >= umbral {
            let key = if dce {
                Some("ESPERA.E05_SOLO_CORREO")
            } else if trib == "LAJA" || trib == "MULCHEN" {
                Some("ESPERA.E05_PROYECTO_Y_CORREO")
            } else if trib == "TOME" {
                Some(if e >= ctx.p("proyecto_tome") { "ESPERA.E05_PROYECTO_Y_CORREO" } else { "ESPERA.E05_SOLO_CORREO" })
            } else {
                ctx.aviso("tribunal no reconocido — regla omitida (G-06)");
                None
            };
            if let Some(k) = key {
                frags.extend(ctx.render(k, &[]));
                principal = true;
            }
        }
    }
    if !principal {
        frags.extend(ctx.render("ESPERA.E06_SIN_RESOLUCION", &[("PROGRAMA", prog.clone())]));
    }
    if !resolucion_reciente {
        frags.extend(historia::servicio_ingreso(ctx, &prog));
    }
    frags.extend(oido(ctx, f));
    frags.extend(audiencia(ctx, f));
    frags.extend(curador(ctx, f));
    frags.extend(historia::otro_programa(ctx, &programa, &pn, &nombre));
    componer(&pfx, &sin_parentesis_admin(frags))
}

const RESIDENCIAS: [&str; 5] = ["rta", "rtt", "res", "rfa", "rva"];

fn cumplimiento(ctx: &mut Ctx, f: &Fila, fecha_hoja2: Option<NaiveDate>) -> String {
    let hoy = ctx.hoy;
    let programa = f.text("programa");
    let nombre = f.text("nombre");
    let pfx = prefijo_observacion(&nombre, &programa);
    let pn = primer_nombre(&nombre);
    let prog = if ctx.cfg.redaccion_registro { titulo_programa_registro(&programa) } else { titulo_programa(&programa) };
    let fegr = f.date("egreso_proy");
    let dpe = f.int("dias_egresar");
    let dc = f.int("dias_cumpl");
    let negativos = dpe.is_some_and(|d| d < 0) || dc.is_some_and(|d| d < 0);
    let vencida = fegr.is_some() && negativos;
    let vence_txt = fegr.map(fecha_es).unwrap_or_default();
    let medida = ctx.p("medida");
    if es_derivacion_sin_seguimiento(&programa) {
        let mut fr: Vec<Frag> = ctx.render("COMUN.NO_SEGUIMIENTO", &[("PROGRAMA", prog)]).into_iter().collect();
        if vencida {
            fr.extend(ctx.render("HISTORIA.MEDIDA_SIN_VIGENCIA", &[("FECHA_EGRESO_PROYECTADO", vence_txt)]));
        }
        return componer(&pfx, &fr);
    }
    let c05 = fegr.is_some() && dpe.is_some_and(|d| 0 <= d && d <= medida) && !vencida;
    let c05_key = if dpe == Some(0) { "CUMPLIMIENTO.C05_VENCE_HOY" } else { "CUMPLIMIENTO.C05_POR_VENCER" };
    let nac = f.date("nacimiento");
    if let Some(fnac) = nac {
        if edad(fnac, hoy) >= 18 {
            ctx.marcar(3, "REVISAR_MAYORIA", "Mayoría de edad");
            let mut fr: Vec<Frag> = ctx
                .render("COMUN.MAYORIA_EDAD", &[("PNOMBRE", pn.clone()), ("FECHA_MAYORIA", fecha_es(fecha_mayoria(fnac)))])
                .into_iter()
                .collect();
            if vencida && !ctx.invalidas.contains("CUMPLIMIENTO.C04_VENCIDA") {
                fr.extend(ctx.render("CUMPLIMIENTO.C04_VENCIDA", &[("FECHA_EGRESO_PROYECTADO", vence_txt.clone())]));
            } else if c05 {
                fr.extend(ctx.render(c05_key, &[("FECHA_EGRESO_PROYECTADO", vence_txt.clone())]));
            }
            return componer(&pfx, &fr);
        }
    }
    if let Some(finalizada) = historia::causa_finalizada(ctx, &f.text("rit")) {
        return componer(&pfx, &[finalizada]);
    }
    marcar_duplicidad(ctx);
    let mut frags: Vec<Frag> = Vec::new();
    let mut principal = false;
    let mut c03 = false;
    if let Some(fnac) = nac {
        let dm = (fecha_mayoria(fnac) - hoy).num_days();
        let proxima = 1 <= dm && dm <= ctx.p("mayoria");
        if proxima {
            frags.extend(ctx.render("COMUN.PROXIMA_MAYORIA", &[("PNOMBRE", pn.clone()), ("FECHA_MAYORIA", fecha_es(fecha_mayoria(fnac)))]));
        }
        if let Some(e) = fegr {
            let may = fecha_mayoria(fnac);
            if e > may {
                if proxima {
                    frags.extend(ctx.render("HISTORIA.EGRESO_POSTERIOR_MAYORIA", &[]));
                } else {
                    frags.extend(ctx.render("HISTORIA.EGRESO_POSTERIOR_MAYORIA_FECHA", &[("FECHA_MAYORIA", fecha_es(may))]));
                }
            }
        }
    }
    if f.has("dias_cumpl") && dc.is_none() {
        ctx.aviso("días de cumplimiento ausentes o inválidos (G-05)");
    }
    let fing = f.date("ingreso");
    if dc.is_some_and(|d| 0 <= d && d <= ctx.p("ingreso_reciente")) {
        if let Some(fi) = fing {
            frags.extend(ctx.render("CUMPLIMIENTO.C03_INGRESO_RECIENTE", &[("PROGRAMA", prog.clone()), ("FECHA_INGRESO", fecha_es(fi))]));
            principal = true;
            c03 = true;
            if fegr.is_some() && !(vencida || c05) {
                frags.extend(ctx.render("HISTORIA.EGRESO_PROYECTADO", &[("FECHA_EGRESO_PROYECTADO", vence_txt.clone())]));
            }
        } else {
            ctx.aviso("sin fecha de ingreso — regla omitida (G-05)");
        }
    }
    if f.has("dias_egresar") && dpe.is_none() {
        ctx.aviso("días para egresar ausentes o inválidos (G-05)");
    }
    if vencida {
        frags.extend(ctx.render("CUMPLIMIENTO.C04_VENCIDA", &[("FECHA_EGRESO_PROYECTADO", vence_txt.clone())]));
        principal = true;
    } else if negativos && fegr.is_none() {
        ctx.aviso("sin egreso proyectado (G-05)");
    }
    if c05 {
        frags.extend(ctx.render(c05_key, &[("FECHA_EGRESO_PROYECTADO", vence_txt.clone())]));
        principal = true;
    }
    let antes_del_egreso = c05 && fecha_hoja2.zip(fegr).is_some_and(|(h, e)| h <= e);
    if let Some(h2) = fecha_hoja2 {
        if (!vencida && !c05) || c03 || antes_del_egreso {
            let frag = ctx.render("CUMPLIMIENTO.C10_HOJA2", &[("PROGRAMA", prog.clone()), ("FECHA_VENCIMIENTO", fecha_es(h2))]);
            // R1 (20-nov-2026): con la medida por vencer, el registro omite la oración del próximo informe; la regla
            // sigue activa para preparar el correo al programa.
            if !(ctx.cfg.redaccion_registro && c05 && c05_key == "CUMPLIMIENTO.C05_POR_VENCER") {
                frags.extend(frag);
                principal = true;
            }
        }
    }
    // El egreso se dice una sola vez (decisión x): lo dice la frase de la medida o la de ingreso reciente.
    let egreso_dicho = vencida || c05 || (c03 && fegr.is_some());
    let vencida_valida = vencida && !ctx.invalidas.contains("CUMPLIMIENTO.C04_VENCIDA");
    frags.extend(historia::prorroga(ctx, fegr, vencida_valida, c05, egreso_dicho));
    let evidencia = Default::default();
    frags.extend(historia::escrito_sin_ingreso(ctx, &programa, None, &evidencia));
    if historia::informe_pendiente(ctx, &programa, None, &evidencia) {
        frags.extend(ctx.render("HISTORIA.INFORME_AVANCE_PENDIENTE", &[]));
    }
    frags.extend(historia::otro_programa(ctx, &programa, &pn, &nombre));
    frags.extend(oido(ctx, f));
    frags.extend(audiencia(ctx, f));
    frags.extend(curador(ctx, f));
    let pnrm = normalizar(&programa);
    if RESIDENCIAS.iter().any(|r| pnrm.starts_with(r)) && f.has("ficha_ind") {
        match f.date("ficha_ind") {
            None => frags.extend(ctx.render("CUMPLIMIENTO.C07_SIN_FICHA", &[])),
            Some(fi) => {
                let dd = (hoy - fi).num_days();
                if dd > ctx.p("ficha_antigua") {
                    frags.extend(ctx.render("CUMPLIMIENTO.C07_FICHA_ANTIGUA", &[("FECHA_FICHA_INDIVIDUAL", fecha_es(fi))]));
                } else if 0 <= dd && dd <= ctx.p("ficha_reciente") {
                    frags.extend(ctx.render("CUMPLIMIENTO.C07_FICHA_RECIENTE", &[("FECHA_FICHA_INDIVIDUAL", fecha_es(fi))]));
                }
            }
        }
    }
    if contiene_token(&programa, &["fae", "fas"]) {
        if !f.has("ficha_fae") {
            ctx.aviso("ficha FAE no evaluable: sin columna disponible (G-05)");
        } else if let Some(fi) = fing {
            if (hoy - fi).num_days() > ctx.p("ficha_fae") && f.date("ficha_fae").is_none() {
                frags.extend(ctx.render("CUMPLIMIENTO.C08_FICHA_FAE", &[("PNOMBRE", pn.clone())]));
            }
        }
    }
    if !principal && frags.is_empty() {
        let fr: Vec<Frag> = ctx.render("CUMPLIMIENTO.C09_SIN_OBSERVACIONES", &[]).into_iter().collect();
        return componer(&pfx, &fr);
    }
    if !principal && ctx.eventos.iter().all(|e| e.starts_with("HISTORIA."))
        && ctx.eventos.iter().any(|e| matches!(e.as_str(), "HISTORIA.OTRO_PROGRAMA" | "HISTORIA.INFORME_AVANCE_PENDIENTE" | "HISTORIA.C10_ESCRITO" | "HISTORIA.BASE_TRAMITE"))
    {
        if let Some(base) = ctx.render("HISTORIA.MEDIDA_REVISADA", &[]) {
            frags.insert(0, base);
        }
    }
    componer(&pfx, &frags)
}

/// contexto_excel.py::VENTANA_INFORME_AUDIENCIA (ev_78c4c6e4).
pub const VENTANA_INFORME_AUDIENCIA: i64 = 30;
/// reglas_informes.py::MARGEN_DCE (ev_097a02df).
pub const MARGEN_DCE: i64 = 10;
/// work.py::VENTANA_POR_VENCER_DIAS (ev_5960f6f7).
pub const VENTANA_POR_VENCER_DIAS: i64 = 60;

fn informes(ctx: &mut Ctx, f: &Fila) -> String {
    let programa = f.text("programa");
    let nombre = f.text("nombre");
    let pfx = prefijo_observacion(&nombre, &programa);
    let prog = if ctx.cfg.redaccion_registro { titulo_programa_registro(&programa) } else { titulo_programa(&programa) };
    if es_derivacion_sin_seguimiento(&programa) {
        let fr: Vec<Frag> = ctx.render("COMUN.NO_SEGUIMIENTO", &[("PROGRAMA", prog)]).into_iter().collect();
        return componer(&pfx, &fr);
    }
    let Some(fv) = f.date("vencimiento") else {
        ctx.aviso("sin fecha de vencimiento (G-05)");
        return String::new();
    };
    let dias = (fv - ctx.hoy).num_days();
    if dias >= 0 && dias > ctx.p("informe") {
        return String::new();
    }
    let dce = es_dce(&programa);
    let vencido = dias < 0;
    let margen = dce && -MARGEN_DCE <= dias && vencido;
    let evidencia = ctx.captura.map(|c| c.evidencia_informe(fv)).unwrap_or_default();
    let mut frags = Vec::new();
    if vencido && historia::informe_pendiente(ctx, &programa, Some(fv), &evidencia) {
        ctx.marcar(2, "I01_ESCRITO_PENDIENTE", "Informe vencido en RUS con escrito de informe pendiente de resolver: no se pide cuenta");
        frags.extend(ctx.render("HISTORIA.INFORME_AVANCE_PENDIENTE", &[]));
    } else {
        let desde = vencido.then_some(fv);
        let atribuible = historia::informe_atribuible(ctx);
        let ingreso = historia::informe_historia(ctx, desde, &prog, &evidencia);
        let escrito = if atribuible || evidencia.escrito.is_some() {
            historia::escrito_sin_ingreso(ctx, &programa, desde, &evidencia)
        } else {
            None
        };
        let con_historia = ingreso.is_some() || escrito.is_some();
        let atribuida = atribuible || evidencia.resolucion.is_some() || evidencia.escrito.is_some();
        if vencido && !atribuida && ingreso.is_some() {
            ctx.marcar(2, "INFORME_SIN_ATRIBUIR", "Ingreso de informe en la causa que no se puede atribuir a este informe: se conservan las acciones");
        }
        if vencido && !con_historia && historia::informe_previo(ctx, &programa, fv) {
            ctx.marcar(2, "ENTREGA_ANTERIOR", "Posible entrega anterior al vencimiento: no se acredita su período; se conservan las acciones para revisión");
        }
        let mut vars = vec![("PROGRAMA", prog), ("FECHA_VENCIMIENTO", fecha_es(fv))];
        let llave = if vencido && atribuida && con_historia {
            ctx.marcar(2, "I01_CONTRADICHA", "Informe vencido en RUS contradicho por la historia");
            if dce { "HISTORIA.I01_SIN_ENVIO_DCE" } else { "HISTORIA.I01_SIN_ENVIO_GENERAL" }
        } else if margen {
            vars.push(("FECHA_LIMITE", fecha_es(fv + Duration::days(MARGEN_DCE))));
            "INFORMES.I02_MARGEN_DCE"
        } else if vencido {
            if dce { "INFORMES.I01_VENCIDO_DCE" } else { "INFORMES.I01_VENCIDO_GENERAL" }
        } else if dce {
            "INFORMES.I02_POR_VENCER_DCE"
        } else {
            "INFORMES.I02_POR_VENCER_GENERAL"
        };
        frags.extend(ctx.render(llave, &vars));
        frags.extend(ingreso);
        frags.extend(escrito);
    }
    if let Some((n, antiguo)) = ctx.acumulado {
        if n >= 2 {
            frags.extend(ctx.render("HISTORIA.J2_ACUMULADOS", &[("N", n.to_string()), ("FECHA", fecha_es(antiguo))]));
        }
    }
    componer(&pfx, &frags)
}

/// Avisos que solo se muestran (no cambian la observación): N-01 y N-02.
fn avisos_fila(ctx: &mut Ctx, f: &Fila, mode: Mode, fecha_hoja2: Option<NaiveDate>) {
    if mode == Mode::Cumplimiento {
        if let (Some(n), Some(e)) = (f.date("nacimiento"), f.date("egreso_proy")) {
            let may = fecha_mayoria(n);
            if ctx.hoy < may && may < e {
                ctx.aviso(format!(
                    "N-01: la medida se proyecta hasta el {}, posterior a la mayoría de edad ({}).",
                    fecha_es(e),
                    fecha_es(may)
                ));
            }
        }
    }
    let venc = if mode == Mode::Informes { f.date("vencimiento") } else { fecha_hoja2 };
    if let Some(v) = venc {
        if f.has("prox_aud") {
            let (mut auds, _) = fechas_varias(f.cell("prox_aud"));
            auds.sort();
            auds.dedup();
            for a in auds {
                if ctx.hoy <= a && a < v && v <= a + Duration::days(VENTANA_INFORME_AUDIENCIA) {
                    ctx.aviso(format!(
                        "N-02: audiencia el {} y el informe vence el {}; se sugiere solicitarlo antes de la audiencia.",
                        fecha_es(a),
                        fecha_es(v)
                    ));
                    break;
                }
            }
        }
    }
}

/// Evalúa una fila. `idx` identifica la fila en el pre-paso.
pub fn evaluar(cfg: &Config, mode: Mode, f: &Fila, hoy: NaiveDate, pre: &Prepaso, idx: usize, fecha_hoja2: Option<NaiveDate>) -> Resultado {
    evaluar_con_captura(cfg, mode, f, hoy, (pre, idx), fecha_hoja2, None)
}

pub(crate) fn evaluar_con_captura(
    cfg: &Config, mode: Mode, f: &Fila, hoy: NaiveDate, fila_preparada: (&Prepaso, usize),
    fecha_hoja2: Option<NaiveDate>, captura: Option<&ContextoFila>,
) -> Resultado {
    let (pre, idx) = fila_preparada;
    let mut ctx = Ctx {
        cfg,
        hoy,
        eventos: vec![],
        invalidas: BTreeSet::new(),
        marcas: vec![],
        avisos: vec![],
        duplicidad: pre.duplicidad.get(&idx).copied(),
        acumulado: pre.acumulado.get(&idx).copied(),
        captura,
    };
    if mode == Mode::Cumplimiento {
        if let Some(e) = f.date("egreso_proy") {
            let neg = f.int("dias_cumpl").is_some_and(|d| d < 0) || f.int("dias_egresar").is_some_and(|d| d < 0);
            if e > hoy && neg {
                ctx.aviso("Días negativos y egreso futuro: verifica la contradicción; no se afirma que la medida esté vencida.");
                ctx.invalidas.insert("CUMPLIMIENTO.C04_VENCIDA".into());
            }
        }
    }
    let trib = tribunal(&f.text("tribunal"));
    let observacion = match trib {
        None => {
            ctx.aviso("Tribunal fuera de Laja, Mulchén y Tomé: sin reglas configuradas; requiere revisión manual.");
            String::new()
        }
        Some(t) => match mode {
            Mode::Espera => espera(&mut ctx, f, t),
            Mode::Cumplimiento => cumplimiento(&mut ctx, f, fecha_hoja2),
            Mode::Informes => informes(&mut ctx, f),
        },
    };
    if !observacion.is_empty() && !ctx.eventos.iter().any(|e| e == "COMUN.NO_SEGUIMIENTO") {
        avisos_fila(&mut ctx, f, mode, fecha_hoja2);
    }
    let mut eventos = Vec::new();
    for e in ctx.eventos {
        if !eventos.contains(&e) {
            eventos.push(e);
        }
    }
    Resultado { observacion, eventos, marcas: ctx.marcas, avisos: ctx.avisos }
}

/// Acciones (correos y proyectos) según las reglas aplicadas, nunca por palabras de la observación.
pub fn acciones(eventos: &[String], vence_hoja2: Option<NaiveDate>, hoy: NaiveDate) -> Vec<&'static str> {
    let has = |p: &str| eventos.iter().any(|e| e.starts_with(p));
    let mut a = Vec::new();
    if has("ESPERA.E05_") {
        a.push("programa_espera");
    }
    if has("ESPERA.E05_PROYECTO_Y_CORREO") {
        a.push("PC_IE");
    }
    if has("INFORMES.I01_") {
        a.push("programa_vencido");
        a.push("PC_INFO");
    }
    if has("INFORMES.I02_") {
        a.push("programa_por_vencer");
    }
    if has("CUMPLIMIENTO.C04_") || has("CUMPLIMIENTO.C05_") {
        a.push("medidas");
    }
    if has("CUMPLIMIENTO.C10_HOJA2") && !a.contains(&"programa_por_vencer") && vence_hoja2.map_or(true, |v| (v - hoy).num_days() <= VENTANA_POR_VENCER_DIAS) {
        a.push("programa_por_vencer");
    }
    // Observaciones que remiten un proyecto para aplicar una nomenclatura (informe ingresado, causa finalizada).
    if eventos.iter().any(|e| e == "HISTORIA.C10_ESCRITO" || e == "HISTORIA.P_FINALIZADA") {
        a.push("NOMENCL");
    }
    a
}

/// Pre-paso sobre toda la nómina: duplicidad por causa (Espera/Cumplimiento) e informes vencidos acumulados.
pub(crate) fn prepaso<'a>(
    mode: Mode,
    filas: &[Fila],
    hoy: NaiveDate,
    identidad_ingreso: impl Fn(&Fila) -> Option<[&'a str; 4]>,
) -> Prepaso {
    let mut pre = Prepaso::default();
    let mut causas: HashMap<(String, String), Vec<(usize, String, String)>> = HashMap::new();
    for (i, f) in filas.iter().enumerate() {
        let rit = clave(&f.text("rit"));
        if rit.is_empty() {
            continue;
        }
        causas
            .entry((clave(&f.text("tribunal")), rit))
            .or_default()
            .push((i, rut_clave(&f.text("rut")), clave(&f.text("programa"))));
    }
    for items in causas.values() {
        if mode == Mode::Informes {
            let distintos: BTreeSet<(&String, &String)> = items.iter().map(|(_, r, p)| (r, p)).collect();
            if distintos.len() > 1 {
                for (i, _, _) in items {
                    pre.duplicidad.insert(*i, "OTRO_REGISTRO");
                }
            }
            continue;
        }
        if items.len() < 2 {
            continue;
        }
        for (i, rut, prog) in items {
            let rep = !rut.is_empty() && items.iter().filter(|(_, r, p)| r == rut && p == prog).count() > 1;
            pre.duplicidad.insert(*i, if rep { "MISMO_PROGRAMA" } else { "OTRO_REGISTRO" });
        }
    }
    if mode == Mode::Informes {
        #[derive(PartialEq, Eq, Hash)]
        enum ClaveInforme<'a> {
            Captura([&'a str; 4]),
            Libro([String; 4]),
        }
        let mut grupos: HashMap<ClaveInforme<'_>, Vec<(usize, NaiveDate)>> = HashMap::new();
        for (i, f) in filas.iter().enumerate() {
            let Some(due) = f.date("vencimiento") else { continue };
            if due >= hoy {
                continue;
            }
            let ident = if let Some(ingreso) = identidad_ingreso(f) {
                ClaveInforme::Captura(ingreso)
            } else {
                let campos = ["tribunal", "rit", "rut", "programa"].map(|k| historical_match(&f.text(k)));
                if campos[1].is_empty() {
                    continue;
                }
                ClaveInforme::Libro(campos)
            };
            grupos.entry(ident).or_default().push((i, due));
        }
        for items in grupos.values() {
            let fechas: BTreeSet<NaiveDate> = items.iter().map(|(_, d)| *d).collect();
            if fechas.len() > 1 {
                let min = *fechas.iter().next().unwrap();
                for (i, _) in items {
                    pre.acumulado.insert(*i, (fechas.len(), min));
                }
            }
        }
    }
    pre
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Casos de referencia del motor Python original (FLOW_CASOS = carpeta con espera/cumplimiento/informes.json).
    /// Solo los que usan datos del Excel (sin historia ni bitácora): deben coincidir exactamente.
    #[test]
    fn oraculo_python() {
        let Ok(dir) = std::env::var("FLOW_CASOS") else { return };
        let (mut total, mut ok) = (0, 0);
        let mut fallos = Vec::new();
        for (archivo, mode) in [("espera", Mode::Espera), ("cumplimiento", Mode::Cumplimiento), ("informes", Mode::Informes)] {
            let txt = std::fs::read_to_string(std::path::Path::new(&dir).join(format!("{archivo}.json"))).unwrap();
            let casos: Vec<serde_json::Value> = serde_json::from_str(&txt).unwrap();
            for c in casos {
                let sin_historia = c["historia"].is_null() && c["bitacora"].as_array().map_or(true, |a| a.is_empty());
                if !sin_historia {
                    continue;
                }
                total += 1;
                // El oráculo compara con los textos de Flow Python, sin las decisiones de redacción del registro.
                let mut cfg = Config { redaccion_registro: false, ..Config::default() };
                for d in c["desactivadas"].as_array().unwrap() {
                    cfg.desactivadas.insert(d.as_str().unwrap().to_string());
                }
                let corte = NaiveDate::parse_from_str(c["corte"].as_str().unwrap(), "%Y-%m-%d").unwrap();
                let mut cols = Cols::new();
                let mut values = Vec::new();
                for (k, v) in c["campos"].as_object().unwrap() {
                    let key: &'static str = Box::leak(k.clone().into_boxed_str());
                    cols.insert(key, values.len());
                    values.push(match v["t"].as_str().unwrap() {
                        "fecha" => Cell::Date(NaiveDate::parse_from_str(v["v"].as_str().unwrap(), "%Y-%m-%d").unwrap()),
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
                    let n = a[0].as_u64().unwrap() as usize;
                    let d = NaiveDate::parse_from_str(a[1].as_str().unwrap(), "%Y-%m-%d").unwrap();
                    pre.acumulado.insert(0, (n, d));
                }
                let h2 = c["fecha_hoja2"].as_str().and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let f = Fila { values: &values, cols: &cols };
                let r = evaluar(&cfg, mode, &f, corte, &pre, 0, h2);
                let excluido = r.eventos.iter().any(|e| e == "COMUN.NO_SEGUIMIENTO");
                let acc: Vec<String> = if excluido { vec![] } else { acciones(&r.eventos, h2, corte).iter().map(|s| s.to_string()).collect() };
                let e = &c["esperado"];
                let strs = |v: &serde_json::Value| -> Vec<String> { v.as_array().unwrap().iter().map(|x| x.as_str().unwrap_or("").to_string()).collect() };
                let marcas: Vec<String> = e["marcas"].as_array().unwrap().iter().map(|m| m["codigo"].as_str().unwrap().to_string()).collect();
                let mis_marcas: Vec<String> = r.marcas.iter().map(|m| m.codigo.clone()).collect();
                let checks = [
                    ("observacion", e["observacion"].as_str().unwrap() == r.observacion),
                    ("reglas", strs(&e["reglas"]) == r.eventos),
                    ("acciones", strs(&e["acciones"]) == acc),
                    ("avisos", strs(&e["avisos"]) == r.avisos),
                    ("marcas", marcas == mis_marcas),
                    ("excluido", e["excluido"].as_bool().unwrap() == excluido),
                ];
                if checks.iter().all(|(_, b)| *b) {
                    ok += 1;
                } else {
                    let malos: Vec<&str> = checks.iter().filter(|(_, b)| !*b).map(|(n, _)| *n).collect();
                    fallos.push(format!(
                        "{} {:?}\n  esperado: {}\n  obtenido: {}\n  reglas esp {:?} / obt {:?}\n  avisos esp {:?} / obt {:?}\n  marcas esp {:?} / obt {:?}",
                        c["id"], malos, e["observacion"], r.observacion, strs(&e["reglas"]), r.eventos, strs(&e["avisos"]), r.avisos, marcas, mis_marcas
                    ));
                }
            }
        }
        for f in fallos.iter().take(12) {
            println!("{f}");
        }
        println!("ORACULO: {ok}/{total} casos coinciden");
        assert_eq!(ok, total);
    }

    fn fila_cols(headers: &[&'static str]) -> Cols {
        headers.iter().enumerate().map(|(i, h)| (*h, i)).collect()
    }

    #[test]
    fn r1_con_medida_por_vencer_omite_el_proximo_informe_y_conserva_el_correo() {
        let cols = fila_cols(&["rit", "tribunal", "nombre", "programa", "egreso_proy", "dias_egresar", "dias_cumpl"]);
        let hoy = NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();
        let vals = vec![
            Cell::Text("X-1-2099".into()),
            Cell::Text("Juzgado de Letras de Laja".into()),
            Cell::Text("Persona Ficticia".into()),
            Cell::Text("AFT el Faro LAJA".into()),
            Cell::Date(hoy + chrono::Duration::days(20)),
            Cell::Number(20.0),
            Cell::Number(200.0),
        ];
        let f = Fila { values: &vals, cols: &cols };
        let hoja2 = Some(hoy + chrono::Duration::days(10));
        let mut cfg = Config::default();
        let r = evaluar(&cfg, Mode::Cumplimiento, &f, hoy, &Prepaso::default(), 0, hoja2);
        assert!(r.observacion.contains("Medida próxima a vencer"), "{}", r.observacion);
        assert!(!r.observacion.contains("Próximo informe"), "{}", r.observacion);
        assert!(r.eventos.iter().any(|e| e == "CUMPLIMIENTO.C10_HOJA2"));
        cfg.redaccion_registro = false;
        let python = evaluar(&cfg, Mode::Cumplimiento, &f, hoy, &Prepaso::default(), 0, hoja2);
        assert!(python.observacion.contains("Próximo informe de avance de AFT el FARO LAJA"), "{}", python.observacion);
    }

    #[test]
    fn espera_laja_proyecto_y_correo() {
        let cfg = Config::default();
        let cols = fila_cols(&["rit", "tribunal", "nombre", "programa", "espera", "curador"]);
        let vals = vec![
            Cell::Text("X-1-2026".into()),
            Cell::Text("Juzgado de Letras de Laja".into()),
            Cell::Text("SOFÍA (hija) PÉREZ".into()),
            Cell::Text("PRM Ñuble".into()),
            Cell::Number(45.0),
            Cell::Text("NO POSEE".into()),
        ];
        let f = Fila { values: &vals, cols: &cols };
        let hoy = NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();
        let r = evaluar(&cfg, Mode::Espera, &f, hoy, &Prepaso::default(), 0, None);
        assert_eq!(
            r.observacion,
            "Sofía PRM: Medida revisada, a la espera de ingreso efectivo. Se remite proyecto de resolución pidiendo cuenta del ingreso efectivo y correo al programa consultando fecha estimada de ingreso. No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente."
        );
        assert_eq!(acciones(&r.eventos, None, hoy), vec!["programa_espera", "PC_IE"]);
    }

    #[test]
    fn informes_vencido() {
        let cfg = Config::default();
        let cols = fila_cols(&["tribunal", "nombre", "programa", "vencimiento"]);
        let vals = vec![
            Cell::Text("TOME".into()),
            Cell::Text("Ana Rojas".into()),
            Cell::Text("PIE Tomé".into()),
            Cell::Text("01/10/2026".into()),
        ];
        let f = Fila { values: &vals, cols: &cols };
        let hoy = NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();
        let r = evaluar(&cfg, Mode::Informes, &f, hoy, &Prepaso::default(), 0, None);
        assert!(r.observacion.starts_with("Ana PIE: Informe de avance vencido desde el 1 de octubre de 2026."), "{}", r.observacion);
    }
}
