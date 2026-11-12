//! Reglas por etapa (`motor/reglas_espera.py`, `reglas_cumplimiento.py`, `reglas_informes.py`).
//! El orden de los fragmentos es parte de la regla: no reordenar.

use crate::ctx::{componer, Ctx, Frag};
use crate::hist::{self as h, Materia};
use chrono::{Duration, NaiveDate};
use flow_dominio::texto::{
    contiene_token, dias_para_mayoria, edad_exacta, es_dce, es_derivacion_sin_seguimiento, fecha_es, fecha_mayoria,
    lista_fechas_es, prefijo_observacion, primer_nombre, tiene_curador_real, titulo_programa,
};
use flow_dominio::{Registro, Tribunal};

/// Días de margen tras el vencimiento RUS antes de pedir cuenta de un diagnóstico DCE.
const MARGEN_DCE: i64 = 10;

fn audiencia(ctx: &mut Ctx, r: &Registro) -> Option<Frag> {
    let celda = r.celda("prox_aud")?;
    let (fechas, ilegibles) = celda.fechas_varias();
    if ilegibles > 0 {
        ctx.incidencia("T-03", "próxima audiencia con fecha ilegible (G-05)");
    }
    let mut futuras: Vec<NaiveDate> = fechas.into_iter().filter(|f| *f >= ctx.hoy).collect();
    futuras.sort_unstable();
    futuras.dedup();
    let frag = if futuras.len() == 1 {
        ctx.render("COMUN", "PROX_AUDIENCIA", &[("FECHA_AUDIENCIA", fecha_es(futuras[0]))])
    } else if !futuras.is_empty() && !ctx.desactivada("COMUN.PROX_AUDIENCIA") {
        ctx.render("COMUN", "PROX_AUDIENCIAS", &[("FECHAS_AUDIENCIA", lista_fechas_es(&futuras))])
    } else {
        return None;
    };
    (!frag.es_vacio()).then_some(frag)
}

/// Oído de RUS si es reciente; si RUS no tiene fecha, el de la historia.
fn oido(ctx: &mut Ctx, r: &Registro) -> Vec<Frag> {
    match r.fecha("oido") {
        Some(fo) => {
            let d = (ctx.hoy - fo).num_days();
            if 0 <= d && d <= ctx.umbral("oido") && !h::oido_en_bitacora(ctx, fo) {
                vec![ctx.render("COMUN", "OIDO", &[("FECHA_OIDO", fecha_es(fo))])]
            } else {
                Vec::new()
            }
        }
        None => h::derecho_oido(ctx),
    }
}

fn rus_sin_curador(r: &Registro) -> bool {
    r.tiene("curador") && !tiene_curador_real(&r.texto("curador"))
}

fn curador(ctx: &mut Ctx, r: &Registro) -> Vec<Frag> {
    if !r.tiene("curador") {
        return Vec::new();
    }
    let celda = r.texto("curador");
    if !tiene_curador_real(&celda) {
        let previa = h::reitera(ctx, Materia::Curador);
        return vec![if previa.es_vacio() { ctx.render("COMUN", "CURADOR", &[]) } else { previa }];
    }
    h::curador_instituciones(ctx, &celda, &primer_nombre(&r.texto("nombre")))
}

fn complementarias(ctx: &mut Ctx, r: &Registro) -> Vec<Frag> {
    let mut out = oido(ctx, r);
    out.extend(audiencia(ctx, r));
    out.extend(curador(ctx, r));
    out
}

pub fn espera(ctx: &mut Ctx, r: &Registro, tribunal: Tribunal) -> String {
    let programa = r.texto("programa").trim().to_string();
    let nombre = r.texto("nombre").trim().to_string();
    let pfx = prefijo_observacion(&nombre, &programa);
    let pn = primer_nombre(&nombre);
    let prog = titulo_programa(&programa);
    let hoy = ctx.hoy;
    if es_derivacion_sin_seguimiento(&programa) {
        return componer(&pfx, &[ctx.render("COMUN", "NO_SEGUIMIENTO", &[("PROGRAMA", prog)])]);
    }
    let mut frags = Vec::new();
    let nac = r.fecha("nacimiento");
    let mayoria = nac.map(|n| [("PNOMBRE", pn.clone()), ("FECHA_MAYORIA", fecha_es(fecha_mayoria(n)))]);
    if nac.is_some_and(|n| edad_exacta(n, hoy) >= 18) {
        frags.push(ctx.render("COMUN", "MAYORIA_EDAD", mayoria.as_ref().unwrap()));
        ctx.marcar(3, "REVISAR_MAYORIA", "Mayoría de edad");
        return componer(&pfx, &frags);
    }
    let finalizada = h::causa_finalizada(ctx, &r.texto("rit"));
    if !finalizada.is_empty() {
        return componer(&pfx, &finalizada);
    }
    if nac.is_some_and(|n| (1..=ctx.umbral("mayoria")).contains(&dias_para_mayoria(n, hoy))) {
        frags.push(ctx.render("COMUN", "PROXIMA_MAYORIA", mayoria.as_ref().unwrap()));
    }
    let mut principal = false;
    h::marcar_duplicidad(ctx);
    let espera = r.entero("espera");
    h::ingreso_efectivo_en_espera(ctx, hoy, espera);
    h::marcas_sin_efecto(ctx);
    let mut e04 = false;
    if let Some(fres) = r.fecha("resolucion") {
        let d = (hoy - fres).num_days();
        if 0 <= d && d < ctx.umbral("resolucion_reciente") {
            let datos = [("PROGRAMA", prog.clone()), ("FECHA_RESOLUCION", fecha_es(fres))];
            frags.push(match h::folio_resolucion(ctx, fres) {
                Some(folio) => ctx.render("HISTORIA", "E04_CON_FOLIO", &[datos[0].clone(), datos[1].clone(), ("FOLIO", folio)]),
                None => ctx.render("ESPERA", "E04_RESOLUCION_RECIENTE", &datos),
            });
            principal = true;
            e04 = true;
        }
    }
    if r.tiene("espera") && espera.is_none_or(|e| e < 0) {
        ctx.incidencia("E-05/E-06", "tiempo de espera ausente o inválido (G-05)");
    }
    // «Registra N días en lista de espera» dice lo mismo que los días de espera: se dice una vez.
    let larga = h::lista_larga(ctx, &programa, espera);
    let dce = es_dce(&programa);
    let umbral = if dce {
        ctx.umbral("espera_dce")
    } else {
        ctx.umbral(&format!("espera_{}", tribunal.nombre().to_lowercase()))
    };
    if let Some(dias) = espera.filter(|e| *e >= umbral) {
        let texto = if dce {
            "E05_SOLO_CORREO"
        } else if tribunal == Tribunal::Tome && dias < ctx.umbral("proyecto_tome") {
            "E05_SOLO_CORREO"
        } else {
            "E05_PROYECTO_Y_CORREO"
        };
        frags.push(ctx.render("ESPERA", texto, &[]));
        principal = true;
        if !larga {
            frags.push(ctx.render("HISTORIA", "E05_DIAS", &[("CORTE", fecha_es(hoy)), ("N", dias.to_string())]));
        }
    }
    if !principal {
        frags.push(ctx.render("ESPERA", "E06_SIN_RESOLUCION", &[("PROGRAMA", prog.clone())]));
    }
    frags.extend(h::lista_espera_larga(ctx, &programa, espera));
    // Con E-04 la resolución de ingreso ya se dijo: el trámite «Ing.Serv.» no la repite.
    if !e04 {
        frags.extend(h::servicio(ctx, &programa));
    }
    frags.extend(h::busqueda_historia(ctx));
    frags.extend(h::continuidad(ctx));
    frags.extend(h::curador_designaciones(ctx, rus_sin_curador(r)));
    frags.extend(complementarias(ctx, r));
    frags.extend(h::aclaracion_causa(ctx));
    componer(&pfx, &frags)
}

pub fn cumplimiento(ctx: &mut Ctx, r: &Registro, fecha_hoja2: Option<NaiveDate>) -> String {
    let hoy = ctx.hoy;
    let programa = r.texto("programa").trim().to_string();
    let nombre = r.texto("nombre").trim().to_string();
    let pfx = prefijo_observacion(&nombre, &programa);
    let pn = primer_nombre(&nombre);
    let prog = titulo_programa(&programa);
    let fegr = r.fecha("egreso_proy");
    let dpe = r.entero("dias_egresar");
    let dc = r.entero("dias_cumpl");
    let negativos = dpe.is_some_and(|d| d < 0) || dc.is_some_and(|d| d < 0);
    let vencida = fegr.is_some() && negativos;
    let egreso = fegr.map(|f| [("FECHA_EGRESO_PROYECTADO", fecha_es(f))]);
    if es_derivacion_sin_seguimiento(&programa) {
        let mut frags = vec![ctx.render("COMUN", "NO_SEGUIMIENTO", &[("PROGRAMA", prog)])];
        if vencida {
            frags.push(ctx.render("HISTORIA", "MEDIDA_SIN_VIGENCIA", egreso.as_ref().unwrap()));
        }
        return componer(&pfx, &frags);
    }
    let nac = r.fecha("nacimiento");
    let mayoria = nac.map(|n| [("PNOMBRE", pn.clone()), ("FECHA_MAYORIA", fecha_es(fecha_mayoria(n)))]);
    if nac.is_some_and(|n| edad_exacta(n, hoy) >= 18) {
        ctx.marcar(3, "REVISAR_MAYORIA", "Mayoría de edad");
        return componer(&pfx, &[ctx.render("COMUN", "MAYORIA_EDAD", mayoria.as_ref().unwrap())]);
    }
    let finalizada = h::causa_finalizada(ctx, &r.texto("rit"));
    if !finalizada.is_empty() {
        return componer(&pfx, &finalizada);
    }
    h::marcar_duplicidad(ctx);
    h::marcas_sin_efecto(ctx);
    let mut frags = Vec::new();
    let mut principal = false;
    let mut c03 = false;
    let c05 = fegr.is_some() && dpe.is_some_and(|d| 0 <= d && d <= ctx.umbral("medida")) && !vencida;
    let proxima = nac.is_some_and(|n| (1..=ctx.umbral("mayoria")).contains(&dias_para_mayoria(n, hoy)));
    if proxima {
        frags.push(ctx.render("COMUN", "PROXIMA_MAYORIA", mayoria.as_ref().unwrap()));
    }
    frags.extend(h::egreso_posterior_mayoria(ctx, nac, fegr, proxima));
    if r.tiene("dias_cumpl") && dc.is_none() {
        ctx.incidencia("C-03", "días de cumplimiento ausentes o inválidos (G-05)");
    }
    let fing = r.fecha("ingreso");
    if dc.is_some_and(|d| 0 <= d && d <= ctx.umbral("ingreso_reciente")) {
        match fing {
            Some(f) => {
                frags.push(ctx.render(
                    "CUMPLIMIENTO",
                    "C03_INGRESO_RECIENTE",
                    &[("PROGRAMA", prog.clone()), ("FECHA_INGRESO", fecha_es(f))],
                ));
                principal = true;
                c03 = true;
                // El egreso se dice una sola vez: si la medida está vencida o por vencer, lo dice esa frase.
                if fegr.is_some() && !(vencida || c05) {
                    frags.push(ctx.render("HISTORIA", "EGRESO_PROYECTADO", egreso.as_ref().unwrap()));
                }
            }
            None => ctx.incidencia("C-03", "sin fecha de ingreso — regla omitida (G-05)"),
        }
    }
    if r.tiene("dias_egresar") && dpe.is_none() {
        ctx.incidencia("C-04/C-05", "días para egresar ausentes o inválidos (G-05)");
    }
    let vok = h::vencida_valida(ctx, vencida);
    let mut reiterada = false;
    if vencida {
        let previa = if vok { h::reitera(ctx, Materia::Vencida) } else { Frag::vacio() };
        reiterada = !previa.es_vacio();
        frags.push(if reiterada { previa } else { ctx.render("CUMPLIMIENTO", "C04_VENCIDA", egreso.as_ref().unwrap()) });
        principal = true;
    } else if negativos && fegr.is_none() {
        ctx.incidencia("C-04", "sin egreso proyectado (G-05)");
    }
    if c05 {
        let texto = if dpe == Some(0) { "C05_VENCE_HOY" } else { "C05_POR_VENCER" };
        frags.push(ctx.render("CUMPLIMIENTO", texto, egreso.as_ref().unwrap()));
        principal = true;
    }
    // La fecha de egreso se dice una sola vez: la dice la frase de la medida o la de ingreso reciente.
    let egreso_dicho = (vencida && !reiterada) || c05 || (c03 && fegr.is_some());
    if let Some(h2) = fecha_hoja2 {
        if ((!vencida && !c05) || c03) && !(egreso_dicho && fegr == Some(h2)) {
            frags.push(ctx.render(
                "CUMPLIMIENTO",
                "C10_HOJA2",
                &[("PROGRAMA", prog.clone()), ("FECHA_VENCIMIENTO", fecha_es(h2))],
            ));
            principal = true;
        }
    }
    if !c03 {
        frags.extend(h::ingreso_informado(ctx, fing));
    }
    frags.extend(h::prorroga(ctx, hoy, fegr, vencida && vok, c05, egreso_dicho));
    frags.extend(h::solicitud_dce(ctx, &programa, vok));
    frags.extend(h::escrito_sin_ingreso(ctx, &programa, None));
    frags.extend(h::busqueda_historia(ctx));
    frags.extend(h::continuidad(ctx));
    frags.extend(h::curador_designaciones(ctx, rus_sin_curador(r)));
    frags.extend(oido(ctx, r));
    frags.extend(audiencia(ctx, r));
    frags.extend(curador(ctx, r));
    if h::es_residencial(&programa) && r.tiene("ficha_ind") {
        match r.fecha("ficha_ind") {
            None => {
                let previa = h::reitera(ctx, Materia::SinFicha);
                frags.push(if previa.es_vacio() { ctx.render("CUMPLIMIENTO", "C07_SIN_FICHA", &[]) } else { previa });
            }
            Some(fi) => {
                let dd = (hoy - fi).num_days();
                let datos = [("FECHA_FICHA_INDIVIDUAL", fecha_es(fi))];
                if dd > ctx.umbral("ficha_antigua") {
                    frags.push(ctx.render("CUMPLIMIENTO", "C07_FICHA_ANTIGUA", &datos));
                } else if 0 <= dd && dd <= ctx.umbral("ficha_reciente") {
                    frags.push(ctx.render("CUMPLIMIENTO", "C07_FICHA_RECIENTE", &datos));
                }
            }
        }
    }
    if contiene_token(&programa, &["fae", "fas"]) {
        if !r.tiene("ficha_fae") {
            ctx.incidencia("C-08", "ficha FAE no evaluable: sin columna disponible (G-05)");
        } else if fing.is_some_and(|f| (hoy - f).num_days() > ctx.umbral("ficha_fae")) && r.fecha("ficha_fae").is_none() {
            let previa = h::reitera(ctx, Materia::SinFae);
            frags.push(if previa.es_vacio() {
                ctx.render("CUMPLIMIENTO", "C08_FICHA_FAE", &[("PNOMBRE", pn.clone())])
            } else {
                previa
            });
        }
    }
    if !principal && frags.is_empty() {
        return componer(&pfx, &[ctx.render("CUMPLIMIENTO", "C09_SIN_OBSERVACIONES", &[])]);
    }
    if !principal && h::solo_historia(ctx) {
        frags.insert(0, ctx.render("HISTORIA", "MEDIDA_REVISADA", &[]));
    }
    frags.extend(h::aclaracion_causa(ctx));
    componer(&pfx, &frags)
}

/// Informes: solo lo que tiene que ver con informes y la gestión del Centro (correo y proyecto).
pub fn informes(ctx: &mut Ctx, r: &Registro) -> String {
    let programa = r.texto("programa").trim().to_string();
    let nombre = r.texto("nombre").trim().to_string();
    let pfx = prefijo_observacion(&nombre, &programa);
    let prog = titulo_programa(&programa);
    if es_derivacion_sin_seguimiento(&programa) {
        return componer(&pfx, &[ctx.render("COMUN", "NO_SEGUIMIENTO", &[("PROGRAMA", prog)])]);
    }
    let Some(fv) = r.fecha("vencimiento") else {
        ctx.incidencia("I-01/I-02", "sin fecha de vencimiento (G-05)");
        return String::new();
    };
    let dias = (fv - ctx.hoy).num_days();
    let dce = es_dce(&programa);
    let mut fecha = fv;
    let llave = if dce && -MARGEN_DCE <= dias && dias < 0 {
        fecha = fv + Duration::days(MARGEN_DCE);
        "I02_POR_VENCER_DCE"
    } else if dias < 0 {
        if dce {
            "I01_VENCIDO_DCE"
        } else {
            "I01_VENCIDO_GENERAL"
        }
    } else if dias <= ctx.umbral("informe") {
        if dce {
            "I02_POR_VENCER_DCE"
        } else {
            "I02_POR_VENCER_GENERAL"
        }
    } else {
        return String::new();
    };
    let finalizada = h::causa_finalizada(ctx, &r.texto("rit"));
    if !finalizada.is_empty() {
        return componer(&pfx, &finalizada);
    }
    // El vencimiento para contrastar la historia es el de RUS; el margen DCE solo cambia la frase y su plazo.
    let vencido = dias < 0;
    let i1 = h::informe_historia(ctx, fv, vencido);
    let i3 = h::escrito_sin_ingreso(ctx, &programa, vencido.then_some(fv));
    let contradicho = !(i1.is_empty() && i3.is_empty());
    if vencido && !contradicho && h::informe_previo(ctx, &programa, fv) {
        ctx.marcar(
            2,
            "ENTREGA_ANTERIOR",
            "Posible entrega anterior al vencimiento: no se acredita su período; se conservan las acciones para revisión",
        );
    }
    let principal = if vencido && contradicho {
        ctx.marcar(2, "I01_CONTRADICHA", "Informe vencido en RUS contradicho por la historia");
        let texto = if dce { "I01_SIN_ENVIO_DCE" } else { "I01_SIN_ENVIO_GENERAL" };
        ctx.render("HISTORIA", texto, &[("FECHA_VENCIMIENTO", fecha_es(fv))])
    } else {
        ctx.render("INFORMES", llave, &[("PROGRAMA", prog), ("FECHA_VENCIMIENTO", fecha_es(fecha))])
    };
    let mut frags = vec![principal];
    frags.extend(i1);
    frags.extend(i3);
    frags.extend(h::informes_acumulados(ctx));
    frags.extend(h::aclaracion_causa(ctx));
    componer(&pfx, &frags)
}
