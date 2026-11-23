//! Contraste de informes entre la bandeja de Escritos, Historia y el vencimiento en RUS.

use super::super::{Ctx, RESIDENCIAS};
use super::{causa_compartida, cita, novedad, tramites};
use crate::captura_trabajo::{
    clave_texto, ContextoFila, EvidenciaInforme, TramiteCausa, AMBULATORIO, FAE, INGRESO_INFORME,
    RESIDENCIAL,
};
use crate::util::{contiene_token, es_dce, fecha_es};
use chrono::NaiveDate;

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

pub(in crate::motor) fn informe_atribuible(ctx: &Ctx) -> bool {
    ctx.captura.is_none_or(|c| !causa_compartida(ctx, c))
        && ctx.acumulado.is_none_or(|(cantidad, _)| cantidad < 2)
}

fn elegir_tramite<'a>(
    captura: &'a ContextoFila,
    nombre: &'static str,
    folio: Option<&str>,
    desde: Option<NaiveDate>,
    evidencia: &EvidenciaInforme,
) -> Option<&'a TramiteCausa> {
    tramites(captura, nombre)
        .filter(|t| match folio {
            Some(folio) => t.folio == folio,
            None => {
                desde.is_none_or(|d| t.fecha >= d)
                    && !evidencia.folios_ajenos.contains(t.folio.as_str())
            }
        })
        .max_by_key(|t| (t.fecha, t.folio.parse::<u64>().ok()))
}

fn folio_visible(t: &TramiteCausa) -> String {
    if t.folio.is_empty() {
        "s/f".into()
    } else {
        t.folio.clone()
    }
}

pub(in crate::motor) fn informe_historia(
    ctx: &mut Ctx,
    desde: Option<NaiveDate>,
    programa: &str,
    evidencia: &EvidenciaInforme,
) -> Option<super::Frag> {
    let captura = ctx.captura?;
    let tramite = elegir_tramite(
        captura,
        INGRESO_INFORME,
        evidencia.resolucion,
        desde,
        evidencia,
    )?;
    let vars = [
        ("FOLIO", folio_visible(tramite)),
        ("FECHA", fecha_es(tramite.fecha)),
    ];
    if evidencia.resolucion.is_none() && !informe_atribuible(ctx) {
        if !novedad(tramite, &captura.notas_centro) {
            return None;
        }
        let [folio, fecha] = vars;
        return ctx
            .render(
                "HISTORIA.BASE_TRAMITE",
                &[folio, fecha, ("DESC", tramite.descripcion.clone())],
            )
            .map(|frag| cita(frag, tramite));
    }
    if let Some(vencimiento) = desde {
        if let Some(frag) = ctx.render(
            "HISTORIA.INFORME_REVISAR_ASOCIACION",
            &[
                ("PROGRAMA", programa.to_owned()),
                ("FOLIO", folio_visible(tramite)),
                ("FECHA_PROYECTADA", fecha_es(vencimiento)),
            ],
        ) {
            return Some(frag);
        }
    }
    ctx.render("HISTORIA.INFORME_INGRESADO", &vars)
        .map(|frag| cita(frag, tramite))
}

pub(in crate::motor) fn escrito_sin_ingreso(
    ctx: &mut Ctx,
    programa: &str,
    desde: Option<NaiveDate>,
    evidencia: &EvidenciaInforme,
) -> Option<super::Frag> {
    let captura = ctx.captura?;
    if evidencia.escrito.is_none() && causa_compartida(ctx, captura) {
        return None;
    }
    let escrito = elegir_tramite(
        captura,
        tipo_escrito(programa)?,
        evidencia.escrito,
        desde,
        evidencia,
    )?;
    let folio = escrito.folio.parse::<u64>().ok();
    if tramites(captura, INGRESO_INFORME).any(|ingreso| {
        match ingreso.folio.parse::<u64>().ok().zip(folio) {
            Some((posterior, anterior)) => posterior > anterior,
            None => ingreso.fecha > escrito.fecha,
        }
    }) {
        return None;
    }
    ctx.render(
        "HISTORIA.C10_ESCRITO",
        &[
            ("FOLIO_ESCRITO", folio_visible(escrito)),
            ("FECHA_ESCRITO", fecha_es(escrito.fecha)),
        ],
    )
    .map(|frag| cita(frag, escrito))
}

pub(in crate::motor) fn informe_previo(ctx: &Ctx, programa: &str, vencimiento: NaiveDate) -> bool {
    ctx.captura.is_some_and(|captura| {
        [Some(INGRESO_INFORME), tipo_escrito(programa)]
            .into_iter()
            .flatten()
            .any(|nombre| tramites(captura, nombre).any(|t| t.fecha < vencimiento))
    })
}

pub(in crate::motor) fn informe_pendiente(
    ctx: &mut Ctx,
    programa: &str,
    vencimiento: Option<NaiveDate>,
    evidencia: &EvidenciaInforme,
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
    if evidencia.escrito.is_some() || evidencia.resolucion.is_some() {
        if explicito {
            ctx.marcar(2, "ESCRITO_ESTADOS_INCOMPATIBLES", "Escrito pendiente en la bandeja y escrito o resolución del mismo informe ya en la historia: prevalece la historia; revisar");
        }
        return false;
    }
    if !explicito {
        let unico = informe_atribuible(ctx)
            && escritos.cobertura == "COMPLETA"
            && candidatos.len() == 1
            && (vencimiento.is_none()
                || candidatos[0]
                    .vinculo
                    .as_ref()
                    .is_none_or(|(_, fecha)| fecha.is_none()));
        if !unico {
            ctx.marcar(2, "ESCRITO_PENDIENTE_SIN_ATRIBUIR", "Escrito de informe pendiente en la causa que no se puede atribuir a este informe: se conservan las acciones");
            return false;
        }
        let en_historia = [tipo, INGRESO_INFORME].into_iter().any(|nombre| {
            tramites(captura, nombre).any(|t| vencimiento.is_none_or(|fecha| t.fecha >= fecha))
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
    use crate::captura_trabajo::{EscritoCausa, ResumenEscritos};
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

    #[test]
    fn una_referencia_con_dos_duenos_no_acredita_la_resolucion_a_ninguno() {
        let vencimiento = "2026-09-20".parse().unwrap();
        let siguiente = "2026-09-21".parse().unwrap();
        let captura = ContextoFila {
            ingreso_id: "ingreso-ficticio".into(),
            escritos: Some(ResumenEscritos {
                cobertura: "COMPLETA".into(),
                documentos: std::sync::Arc::new(
                    [(
                        "referencia ficticia".into(),
                        [
                            ("ingreso-ficticio".into(), vencimiento),
                            ("ingreso-ficticio".into(), siguiente),
                        ]
                        .into(),
                    )]
                    .into(),
                ),
                ..Default::default()
            }),
            tramites: Some(
                vec![TramiteCausa {
                    fecha: siguiente,
                    descripcion: "Ingreso Informe de Cumplimiento X".into(),
                    folio: "42".into(),
                    referencia: "referencia ficticia".into(),
                    tipo: "resolucion".into(),
                    contexto: false,
                }]
                .into(),
            ),
            ..Default::default()
        };
        for fecha in [vencimiento, siguiente] {
            let r = analizar(&captura, fecha);
            assert_eq!(r.eventos, ["INFORMES.I01_VENCIDO_GENERAL"]);
            assert_eq!(
                crate::motor::acciones(&r.eventos, None, fecha),
                ["programa_vencido", "PC_INFO"]
            );
        }
    }
}
