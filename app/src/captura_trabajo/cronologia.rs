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

    const CRONOLOGIA_ORACULO: &str =
        include_str!("../../../rust/crates/flow-trabajo/tests/datos/cronologia.json");

    /// Entradas del oráculo con el mismo mapeo que `temporal.py`: ventana `period_from..=period_to`, centro desde
    /// `origin`/`origin_verified`/`coverage` (temporal.py:60-67), instante desde `time_precision`/`time_zone`
    /// (temporal.py:50-57), `al_tribunal` desde `event_type` (temporal.py:129) y `sin_respuesta` desde `response`
    /// (temporal.py:130). Los eventos del oráculo no traen `captured_at`, por eso no se usa `leer`.
    fn entradas_del_caso(caso: &serde_json::Value, desde: NaiveDate, hasta: NaiveDate) -> Vec<Entrada> {
        caso["eventos"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|evento| {
                let at = evento["event_at"].as_str().unwrap_or("");
                let fecha = at.get(..10)?.parse::<NaiveDate>().ok()?;
                if !(desde..=hasta).contains(&fecha) {
                    return None;
                }
                let instante = if evento["time_precision"].as_str() == Some("SEGUNDO")
                    && !matches!(evento["time_zone"].as_str().unwrap_or(""), "" | "DESCONOCIDA")
                {
                    DateTime::parse_from_rfc3339(at).ok().map(|t| t.to_utc())
                } else {
                    None
                };
                Some(Entrada {
                    fecha,
                    texto: evento["text"].as_str().unwrap_or("").to_string(),
                    instante,
                    id: evento["event_id"].as_str().unwrap_or("").to_string(),
                    centro: evento["origin"].as_str() == Some("CENTRO")
                        && evento["origin_verified"].as_str() == Some("true")
                        && matches!(
                            evento["coverage"].as_str(),
                            Some("COMPLETA" | "PARCIAL")
                        ),
                    al_tribunal: util::normalizar(evento["event_type"].as_str().unwrap_or(""))
                        == "al tribunal",
                    sin_respuesta: evento["response"]
                        .as_str()
                        .unwrap_or("")
                        .trim()
                        .is_empty(),
                })
            })
            .collect()
    }

    fn config_del_caso(caso: &serde_json::Value) -> Config {
        let mut config = Config {
            redaccion_registro: false,
            ..Config::default()
        };
        for regla in caso["desactivadas"].as_array().unwrap() {
            config
                .desactivadas
                .insert(regla.as_str().unwrap().to_string());
        }
        config
    }

    fn resultado_del_caso(caso: &serde_json::Value) -> Resultado {
        Resultado {
            observacion: caso["propuesta"].as_str().unwrap().to_string(),
            eventos: caso["reglas"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_str().unwrap().to_string())
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn oraculo_cronologia_reiteracion() {
        let raiz: serde_json::Value =
            serde_json::from_str(CRONOLOGIA_ORACULO).expect("cronologia.json válido");
        let casos = raiz["casos"].as_array().unwrap();
        let (mut coinciden, mut reiteradas, mut reiteradas_esperadas) = (0usize, 0usize, 0usize);
        let (mut desactivada, mut desactivadas_esperadas) = (0usize, 0usize);
        let (mut excepciones, mut no_reiteradas) = (0usize, 0usize);
        let mut fallos = Vec::new();

        for (indice, caso) in casos.iter().enumerate() {
            let display = &caso["display"];
            let desde: NaiveDate = display["period_from"].as_str().unwrap().parse().unwrap();
            let hasta: NaiveDate = display["period_to"].as_str().unwrap().parse().unwrap();
            let leida = caso["leida"].as_bool().unwrap();
            let cronologia = ordenar(
                entradas_del_caso(caso, desde, hasta),
                desde,
                hasta,
                leida,
            );
            let config = config_del_caso(caso);
            let mut resultado = resultado_del_caso(caso);
            cronologia.aplicar_reiteracion(&mut resultado, &config);

            let esperado = &caso["reiteracion"];
            let esperado_obs = esperado["observacion"].as_str().unwrap();
            let esperado_reglas: Vec<&str> = esperado["reglas"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|r| r.as_str())
                .collect();
            let esperado_avisos: Vec<&str> = esperado["avisos"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|r| r.as_str())
                .collect();
            let obtenidas: Vec<&str> = resultado.eventos.iter().map(String::as_str).collect();
            let obtenidos: Vec<&str> = resultado.avisos.iter().map(String::as_str).collect();
            let espera_reiteracion = esperado_reglas.contains(&REITERACION);
            let espera_desactivada = caso["desactivadas"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d.as_str() == Some(REITERACION));
            if espera_reiteracion {
                reiteradas_esperadas += 1;
            }
            if espera_desactivada {
                desactivadas_esperadas += 1;
            }

            let coincide = resultado.observacion == esperado_obs
                && obtenidas == esperado_reglas
                && obtenidos == esperado_avisos;
            if coincide {
                coinciden += 1;
                if espera_reiteracion {
                    reiteradas += 1;
                }
                if espera_desactivada {
                    desactivada += 1;
                }
            } else {
                fallos.push(format!(
                    "caso {indice}: esperada {esperado_obs:?} / obtenida {:?}; reglas esp {esperado_reglas:?} / obt {obtenidas:?}; avisos esp {esperado_avisos:?} / obt {obtenidos:?}",
                    resultado.observacion
                ));
            }

            // R5 parcial (redaccion_registro: true): las dos excepciones del usuario no se reiteran.
            let cuerpo_propuesta = cuerpo(caso["propuesta"].as_str().unwrap());
            let es_excepcion = cuerpo_propuesta == cuerpo("Medida revisada, sin observaciones.")
                || (cuerpo_propuesta.starts_with("la derivacion ")
                    && cuerpo_propuesta
                        .ends_with(" no se encuentra sujeta a seguimiento por este centro"));
            if es_excepcion {
                excepciones += 1;
                let mut parcial = resultado_del_caso(caso);
                let mut config_parcial = config_del_caso(caso);
                config_parcial.redaccion_registro = true;
                cronologia.aplicar_reiteracion(&mut parcial, &config_parcial);
                if !parcial.eventos.iter().any(|r| r.as_str() == REITERACION)
                    && parcial.observacion == caso["propuesta"].as_str().unwrap()
                {
                    no_reiteradas += 1;
                }
            }
        }

        for fallo in &fallos {
            println!("FALLA {fallo}");
        }
        println!(
            "CRONOLOGIA: casos {}, coinciden {}, reiteradas {}/{}, desactivada {}/{}, fallos {}",
            casos.len(),
            coinciden,
            reiteradas,
            reiteradas_esperadas,
            desactivada,
            desactivadas_esperadas,
            fallos.len()
        );
        if excepciones == 0 {
            println!("R5_PARCIAL: SIN_COBERTURA");
        } else {
            println!("R5_PARCIAL: excepciones {excepciones}, no reiteradas {no_reiteradas}");
        }
        println!(
            "SIMILITUDES: {} declaradas; no aplica en app/ (la cronología no usa una función de similitud)",
            raiz["similitudes"].as_array().map_or(0, |v| v.len())
        );

        assert_eq!(casos.len(), 181, "el oráculo debe traer 181 casos");
        assert_eq!(coinciden, casos.len(), "hay casos que no coinciden");
        assert_eq!(reiteradas_esperadas, 18, "el oráculo debe traer 18 reiteraciones");
        assert_eq!(reiteradas, 18, "no coinciden las 18 reiteraciones");
        assert_eq!(
            desactivadas_esperadas, 1,
            "el oráculo debe traer 1 caso con la regla desactivada"
        );
        assert_eq!(desactivada, 1, "no coincide el caso con la regla desactivada");
        assert!(fallos.is_empty(), "{} casos no coinciden", fallos.len());
        assert_eq!(no_reiteradas, excepciones, "R5 parcial: alguna excepción se reiteró");
        assert_eq!(
            raiz["similitudes"].as_array().map_or(0, |v| v.len()),
            256
        );
    }
}
