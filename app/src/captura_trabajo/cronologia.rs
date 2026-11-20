//! Vista temporal de las lecturas acreditadas del mismo ingreso.

use chrono::{DateTime, Months, NaiveDate, Utc};
use flow_rust::captura::{revision::Revision, Fila};

#[derive(Debug, PartialEq, Eq)]
pub struct Entrada {
    pub fecha: NaiveDate,
    pub texto: String,
    instante: Option<DateTime<Utc>>,
    id: String,
    centro: bool,
    al_tribunal: bool,
    sin_respuesta: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Cronologia {
    pub desde: NaiveDate,
    pub hasta: NaiveDate,
    pub ultimas: Vec<Entrada>,
    pub posteriores: Vec<Entrada>,
    pub orden_incierto: Vec<Entrada>,
    pub sin_autoria: Vec<Entrada>,
    pub sin_respuesta: Vec<NaiveDate>,
    pub estado: Estado,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Estado {
    Acreditada,
    OrigenNoAcreditado,
    NoLeida,
    SinEntradasCapturadas,
}

impl Estado {
    pub fn etiqueta(&self) -> &'static str {
        match self {
            Estado::Acreditada => "Observación del centro acreditada",
            Estado::OrigenNoAcreditado => {
                "Entradas del período con origen del centro sin acreditar"
            }
            Estado::NoLeida => "Bitácora del ingreso sin lectura comprobada",
            Estado::SinEntradasCapturadas => "Sin entradas capturadas en el período",
        }
    }
}

pub(super) fn leer(
    filas: &[Fila<'_>],
    revision: &Revision,
    corte: NaiveDate,
    leida: bool,
) -> Cronologia {
    let desde = corte
        .checked_sub_months(Months::new(4))
        .unwrap_or(NaiveDate::MIN);
    let entradas = filas
        .iter()
        .filter_map(|&f| {
            let fecha = super::fecha_evento(f)?;
            let lectura = super::dia_iso(f.get("captured_at"))?;
            if !(desde..=corte).contains(&fecha) || lectura > corte {
                return None;
            }
            let instante = if f.get("time_precision") == "SEGUNDO"
                && !matches!(f.get("time_zone"), "" | "DESCONOCIDA")
            {
                DateTime::parse_from_rfc3339(f.get("event_at"))
                    .ok()
                    .map(|t| t.to_utc())
            } else {
                None
            };
            Some(Entrada {
                fecha,
                texto: f.get("text").to_owned(),
                instante,
                id: f.get("event_id").to_owned(),
                centro: revision.acredita_centro(f),
                al_tribunal: crate::util::normalizar(f.get("event_type")) == "al tribunal",
                sin_respuesta: f.get("response").trim().is_empty(),
            })
        })
        .collect();
    ordenar(entradas, desde, corte, leida)
}

fn ordenar(
    mut entradas: Vec<Entrada>,
    desde: NaiveDate,
    hasta: NaiveDate,
    leida: bool,
) -> Cronologia {
    let fecha = entradas.iter().filter(|e| e.centro).map(|e| e.fecha).max();
    let ultima_hora = fecha.and_then(|fecha| {
        let dia = entradas.iter().filter(|e| e.centro && e.fecha == fecha);
        dia.clone()
            .all(|e| e.instante.is_some())
            .then(|| dia.filter_map(|e| e.instante).max())
            .flatten()
    });
    let mut salida = Cronologia {
        desde,
        hasta,
        ultimas: Vec::new(),
        posteriores: Vec::new(),
        orden_incierto: Vec::new(),
        sin_autoria: Vec::new(),
        sin_respuesta: Vec::new(),
        estado: if fecha.is_some() {
            Estado::Acreditada
        } else if !entradas.is_empty() {
            Estado::OrigenNoAcreditado
        } else if leida {
            Estado::SinEntradasCapturadas
        } else {
            Estado::NoLeida
        },
    };
    entradas
        .sort_unstable_by(|a, b| (a.fecha, a.instante, &a.id).cmp(&(b.fecha, b.instante, &b.id)));
    for evento in entradas {
        if evento.centro
            && evento.al_tribunal
            && evento.sin_respuesta
            && (hasta - evento.fecha).num_days() >= 30
        {
            salida.sin_respuesta.push(evento.fecha);
        }
        let Some(fecha) = fecha else {
            salida.sin_autoria.push(evento);
            continue;
        };
        if evento.fecha > fecha {
            salida.posteriores.push(evento);
        } else if evento.fecha == fecha {
            if evento.centro && ultima_hora.is_none_or(|hora| evento.instante == Some(hora)) {
                salida.ultimas.push(evento);
            } else if let Some((hora, ultima)) = evento.instante.zip(ultima_hora) {
                if hora > ultima {
                    salida.posteriores.push(evento);
                }
            } else {
                salida.orden_incierto.push(evento);
            }
        }
    }
    salida
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evento(id: &str, valor: &str, centro: bool, precisa: bool) -> Entrada {
        Entrada {
            fecha: valor[..10].parse().unwrap(),
            texto: id.into(),
            id: id.into(),
            centro,
            instante: precisa.then(|| DateTime::parse_from_rfc3339(valor).unwrap().to_utc()),
            al_tribunal: true,
            sin_respuesta: true,
        }
    }

    #[test]
    fn distingue_posteriores_y_orden_incierto_con_la_fecha_civil_y_hora() {
        let desde = "2026-06-20".parse().unwrap();
        let corte = "2026-10-20".parse().unwrap();
        let cronologia = ordenar(
            vec![
                evento("centro-a", "2026-09-12T09:00:00-03:00", true, true),
                evento("centro-b", "2026-09-12T13:00:00Z", true, true),
                evento("tribunal-a", "2026-09-12T11:30:00-03:00", false, true),
                evento("sin-hora", "2026-09-12", false, false),
                evento("otro-dia", "2026-09-13", false, false),
            ],
            desde,
            corte,
            true,
        );
        assert_eq!(cronologia.ultimas[0].id, "centro-b");
        assert_eq!(cronologia.posteriores.len(), 2);
        assert_eq!(cronologia.orden_incierto.len(), 1);
        assert_eq!(
            cronologia.sin_respuesta,
            ["2026-09-12".parse::<NaiveDate>().unwrap(); 2]
        );
        assert_eq!(
            ordenar(Vec::new(), desde, corte, false).estado,
            Estado::NoLeida
        );
    }
}
