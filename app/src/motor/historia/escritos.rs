//! Atribución de pendientes de la bandeja de Escritos por ingreso y fecha proyectada.

use super::super::{Ctx, RESIDENCIAS};
use crate::captura_trabajo::{clave_texto, ContextoFila, EscritoCausa};
use crate::util::{contiene_token, es_dce};
use chrono::NaiveDate;

const INGRESO_INFORME: &str = "ingreso informe de cumplimiento x";
const AMBULATORIO: &str = "inf cump programa ambulatorio";
const FAE: &str = "inf cump fae/fas";
const RESIDENCIAL: &str = "inf cump centro residencial";

fn tipo_escrito(programa: &str) -> Option<&'static str> {
    if es_dce(programa) {
        return None;
    }
    let clave = clave_texto(programa);
    Some(if RESIDENCIAS.iter().any(|p| clave.starts_with(p)) {
        RESIDENCIAL
    } else if contiene_token(programa, &["fae", "fas"]) {
        FAE
    } else {
        AMBULATORIO
    })
}

fn historia_del_informe(
    captura: &ContextoFila,
    escritos: &[EscritoCausa],
    vencimiento: NaiveDate,
) -> bool {
    // Una referencia documental es prueba de atribución cuando todos sus vínculos apuntan a la misma proyección.
    captura.tramites.as_deref().unwrap_or(&[]).iter().any(|t| {
        if t.referencia.is_empty()
            || t.folio.is_empty()
            || !matches!(
                clave_texto(&t.descripcion).as_str(),
                INGRESO_INFORME | AMBULATORIO | FAE | RESIDENCIAL
            )
        {
            return false;
        }
        let mut duenos = escritos
            .iter()
            .filter(|e| e.documento == t.referencia)
            .filter_map(|e| e.vinculo.as_ref());
        let propio =
            |v: &(String, Option<NaiveDate>)| v.0 == captura.ingreso_id && v.1 == Some(vencimiento);
        duenos.next().is_some_and(propio) && duenos.all(propio)
    })
}

pub(in crate::motor) fn informe_pendiente(
    ctx: &mut Ctx,
    programa: &str,
    vencimiento: Option<NaiveDate>,
) -> bool {
    let Some(captura) = ctx.captura else {
        return false;
    };
    let Some(escritos) = &captura.escritos else {
        return false;
    };
    if escritos.discordante {
        ctx.marcar(2, "ESCRITOS_DISCORDANTES", "Dos lecturas simultáneas de la bandeja de Escritos no coinciden: no se atribuye ningún escrito y se conservan las acciones");
        return false;
    }
    let Some(tipo) = tipo_escrito(programa) else {
        return false;
    };
    if !matches!(escritos.cobertura.as_str(), "COMPLETA" | "PARCIAL") {
        return false;
    }
    let candidatos: Vec<_> = escritos
        .datos
        .iter()
        .filter(|e| e.tipo == tipo && e.estado == "pendiente")
        .filter(|e| {
            e.vinculo
                .as_ref()
                .is_none_or(|(ingreso, _)| ingreso == &captura.ingreso_id)
        })
        .collect();
    if candidatos.is_empty() {
        return false;
    }
    let explicito = vencimiento.is_some()
        && candidatos.iter().any(|e| {
            e.vinculo
                .as_ref()
                .is_some_and(|(_, fecha)| *fecha == vencimiento)
        });
    if vencimiento.is_some_and(|fecha| historia_del_informe(captura, &escritos.datos, fecha)) {
        if explicito {
            ctx.marcar(2, "ESCRITO_ESTADOS_INCOMPATIBLES", "Escrito pendiente en la bandeja y escrito o resolución del mismo informe ya en la historia: prevalece la historia; revisar");
        }
        return false;
    }
    if !explicito {
        let atribuible =
            !captura.varios_ingresos && ctx.duplicidad.is_none() && ctx.acumulado.is_none();
        let unico = atribuible
            && escritos.cobertura == "COMPLETA"
            && candidatos.len() == 1
            && candidatos[0]
                .vinculo
                .as_ref()
                .is_none_or(|(_, fecha)| vencimiento.is_none() && fecha.is_some());
        if !unico {
            ctx.marcar(2, "ESCRITO_PENDIENTE_SIN_ATRIBUIR", "Escrito de informe pendiente en la causa que no se puede atribuir a este informe: se conservan las acciones");
            return false;
        }
        let en_historia = captura.tramites.as_deref().unwrap_or(&[]).iter().any(|t| {
            let descripcion = clave_texto(&t.descripcion);
            (descripcion == tipo || descripcion == INGRESO_INFORME)
                && vencimiento.is_none_or(|fecha| t.fecha >= fecha)
        });
        if en_historia {
            let leido = escritos.leido.parse::<NaiveDate>().ok();
            if !captura
                .historia_hasta
                .zip(leido)
                .is_some_and(|(hasta, leido)| hasta > leido)
            {
                ctx.marcar(2, "ESCRITO_ESTADOS_INCOMPATIBLES", "Escrito pendiente en la bandeja y escrito ya incorporado en la historia sin secuencia fiable: revisar");
            }
            return false;
        }
    }
    ctx.marcar(
        2,
        "ESCRITO_PENDIENTE",
        "Informe de avance pendiente de resolver en la bandeja de Escritos",
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::captura_trabajo::ResumenEscritos;
    use crate::config::Config;
    use crate::motor::{evaluar_con_captura, Fila, Mode, Prepaso, Resultado};
    use crate::util::Cell;

    fn analizar(captura: &ContextoFila, vencimiento: NaiveDate) -> Resultado {
        let values = [
            Cell::Text("Laja".into()),
            Cell::Text("PRM Ficticio".into()),
            Cell::Date(vencimiento),
        ];
        let cols = [("tribunal", 0), ("programa", 1), ("vencimiento", 2)].into();
        evaluar_con_captura(
            &Config::default(),
            Mode::Informes,
            &Fila {
                values: &values,
                cols: &cols,
            },
            "2026-10-08".parse().unwrap(),
            (&Prepaso::default(), 0),
            None,
            Some(captura),
        )
    }

    #[test]
    fn la_bandeja_parcial_o_compartida_exige_asociacion_explicita() {
        let vencimiento = "2026-09-20".parse().unwrap();
        let otra_fecha = "2026-09-21".parse().unwrap();
        let escrito = |vinculo| EscritoCausa {
            tipo: AMBULATORIO.into(),
            estado: "pendiente".into(),
            documento: "ficticio".into(),
            vinculo,
        };
        let mut captura = ContextoFila {
            ingreso_id: "ingreso-ficticio".into(),
            varios_ingresos: true,
            escritos: Some(ResumenEscritos {
                cobertura: "PARCIAL".into(),
                datos: vec![escrito(None)].into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let r = analizar(&captura, vencimiento);
        assert_eq!(r.eventos, ["INFORMES.I01_VENCIDO_GENERAL"]);
        assert_eq!(r.marcas[0].codigo, "ESCRITO_PENDIENTE_SIN_ATRIBUIR");
        captura.escritos.as_mut().unwrap().datos =
            vec![escrito(Some(("otro-ingreso".into(), Some(vencimiento))))].into();
        let r = analizar(&captura, vencimiento);
        assert_eq!(r.eventos, ["INFORMES.I01_VENCIDO_GENERAL"]);
        assert!(r.marcas.is_empty());
        captura.escritos.as_mut().unwrap().datos = vec![escrito(Some((
            "ingreso-ficticio".into(),
            Some(vencimiento),
        )))]
        .into();
        let r = analizar(&captura, otra_fecha);
        assert_eq!(r.eventos, ["INFORMES.I01_VENCIDO_GENERAL"]);
        let r = analizar(&captura, vencimiento);
        assert_eq!(r.eventos, ["HISTORIA.INFORME_AVANCE_PENDIENTE"]);
        captura.escritos.as_mut().unwrap().discordante = true;
        let r = analizar(&captura, vencimiento);
        assert_eq!(r.eventos, ["INFORMES.I01_VENCIDO_GENERAL"]);
        assert_eq!(r.marcas[0].codigo, "ESCRITOS_DISCORDANTES");
    }
}
