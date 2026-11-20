//! Vista temporal de las lecturas acreditadas del mismo ingreso.

use crate::{config::Config, motor::Resultado, util};
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
    ultima_no_reiterada: Option<(NaiveDate, String)>,
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

const REITERACION: &str = "COMUN.REITERA_OBSERVACION";
const NO_REITERAR_PYTHON: &[&str] = &[
    "CUMPLIMIENTO.C09_SIN_OBSERVACIONES",
    "ESPERA.E05_",
    "ESPERA.E07_",
    "INFORMES.I01_",
    "INFORMES.I02_",
    "HISTORIA.C10_ESCRITO",
    "HISTORIA.P_FINALIZADA",
    "HISTORIA.REITERA",
    "COMUN.NO_SEGUIMIENTO",
];

fn prefijo(texto: &str) -> Option<(&str, &str)> {
    texto
        .split_once(": ")
        .filter(|(cabeza, _)| cabeza.chars().count() <= 40)
}

fn cuerpo(texto: &str) -> String {
    let normalizado = util::normalizar(texto);
    prefijo(&normalizado)
        .map_or(normalizado.as_str(), |(_, cuerpo)| cuerpo)
        .to_owned()
}

impl Cronologia {
    pub fn aplicar_reiteracion(&self, resultado: &mut Resultado, config: &Config) {
        let Some((fecha, anterior)) = &self.ultima_no_reiterada else {
            return;
        };
        let propuesta = cuerpo(&resultado.observacion);
        if propuesta.is_empty()
            || &propuesta != anterior
            || config.desactivadas.contains(REITERACION)
            || resultado
                .eventos
                .iter()
                .any(|regla| regla == "COMUN.NO_SEGUIMIENTO")
        {
            return;
        }
        let excepcion = if config.redaccion_registro {
            propuesta == cuerpo("Medida revisada, sin observaciones.")
                || (propuesta.starts_with("la derivacion ")
                    && propuesta.ends_with(" no se encuentra sujeta a seguimiento por este centro"))
        } else {
            resultado
                .eventos
                .iter()
                .any(|regla| NO_REITERAR_PYTHON.iter().any(|p| regla.starts_with(p)))
        };
        if excepcion {
            return;
        }
        let texto = config
            .texto(REITERACION)
            .replace("{FECHA_OBSERVACION_ANTERIOR}", &util::fecha_es(*fecha));
        resultado.avisos.push(format!(
            "CT-07: la observación propuesta repetía la del {}; se consigna la reiteración. Texto propuesto originalmente: {}",
            fecha.format("%d/%m/%Y"), resultado.observacion
        ));
        resultado.observacion = if let Some((nombre, _)) = prefijo(&resultado.observacion) {
            format!("{nombre}: {texto}")
        } else {
            texto
        };
        resultado.eventos.push(REITERACION.into());
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
        ultima_no_reiterada: ultima_no_reiterada(&entradas),
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

fn ultima_no_reiterada(entradas: &[Entrada]) -> Option<(NaiveDate, String)> {
    let centros = entradas.iter().filter(|e| {
        e.centro && !util::normalizar(&e.texto).contains("se reitera observacion anterior")
    });
    let fecha = centros.clone().map(|e| e.fecha).max()?;
    let candidatas = centros.filter(|e| e.fecha == fecha);
    let ultima_hora = if candidatas.clone().all(|e| e.instante.is_some()) {
        candidatas.clone().filter_map(|e| e.instante).max()
    } else {
        None
    };
    let mut textos = candidatas
        .filter(|e| ultima_hora.is_none_or(|hora| e.instante == Some(hora)))
        .map(|e| cuerpo(&e.texto));
    let anterior = textos.next()?;
    textos.all(|t| t == anterior).then_some((fecha, anterior))
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

    #[test]
    fn aplica_r5_al_texto_identico_y_respeta_sus_dos_excepciones() {
        let corte = "2026-10-20".parse().unwrap();
        let fecha = "2026-09-12T09:00:00-03:00";
        for (texto, regla, cambia) in [
            ("La medida del programa vence el 20 de noviembre de 2026.", "ESPERA.E05_SOLO_CORREO", true),
            ("Medida revisada, sin observaciones.", "CUMPLIMIENTO.C09_SIN_OBSERVACIONES", false),
            ("La derivación Programa Ficticio no se encuentra sujeta a seguimiento por este Centro.", "COMUN.NO_SEGUIMIENTO", false),
        ] {
            let entrada = Entrada { texto: texto.into(), ..evento("centro", fecha, true, true) };
            let crono = ordenar(vec![entrada], "2026-06-20".parse().unwrap(), corte, true);
            let propuesta = format!("Persona ficticia: {texto}");
            let mut resultado = Resultado { observacion: propuesta.clone(), eventos: vec![regla.into()], ..Default::default() };
            let referencia = Config { redaccion_registro: false, ..Config::default() };
            crono.aplicar_reiteracion(&mut resultado, &referencia);
            assert_eq!(resultado.observacion, propuesta);
            crono.aplicar_reiteracion(&mut resultado, &Config::default());
            assert_eq!(resultado.eventos.iter().any(|r| r == REITERACION), cambia);
            let esperado = if cambia { "Persona ficticia: Se reitera observación anterior del 12 de septiembre de 2026." }
                else { &propuesta };
            assert_eq!(resultado.observacion, esperado);
            let mut distinta = Resultado { observacion: format!("{texto} Cambio ficticio."), ..Default::default() };
            crono.aplicar_reiteracion(&mut distinta, &Config::default());
            assert!(distinta.eventos.is_empty());
        }
    }

    #[test]
    fn no_reitera_una_lectura_con_orden_ambiguo_o_sin_autoria_acreditada() {
        let (desde, corte) = ("2026-06-20".parse().unwrap(), "2026-10-20".parse().unwrap());
        let crono = ordenar(
            vec![
                evento("antecedente", "2026-09-12", true, false),
                evento("texto distinto", "2026-09-12", true, false),
            ],
            desde,
            corte,
            true,
        );
        let mut resultado = Resultado {
            observacion: "antecedente".into(),
            ..Default::default()
        };
        crono.aplicar_reiteracion(&mut resultado, &Config::default());
        let sin_autoria = ordenar(
            vec![evento("antecedente", "2026-09-12", false, false)],
            desde,
            corte,
            true,
        );
        sin_autoria.aplicar_reiteracion(&mut resultado, &Config::default());
        assert!(resultado.eventos.is_empty());
    }
}
