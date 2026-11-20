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
pub(super) use escritos::informe_pendiente;

const SERVICIO: &str = "ing serv proteccion especializada y otras redes";
const INGRESO_EFECTIVO: &str = "ingreso efectivo";

fn tramites<'a>(
    captura: &'a ContextoFila,
    nombre: &'a str,
) -> impl Iterator<Item = &'a TramiteCausa> {
    captura
        .tramites
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter(move |t| normalizar(&t.descripcion) == nombre)
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
    let captura = ctx.captura.filter(|c| !c.varios_ingresos)?;
    let t = tramites(captura, SERVICIO)
        .filter(|t| novedad(t, &captura.notas_centro))
        .max_by_key(|t| (t.fecha, t.folio.parse::<u64>().ok()))?;
    let mut salida = ctx.render(
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
    let denominacion = match t.tipo.as_str() {
        "escrito" => "en escrito de fecha",
        "actuacion" => "en actuación de fecha",
        _ => return Some(salida),
    };
    for (texto, _) in &mut salida.partes {
        *texto = texto.replacen("en resolución de fecha", denominacion, 1);
    }
    Some(salida)
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
        salida.extend(ctx.render(
            "HISTORIA.OTRO_PROGRAMA",
            &[("PNOMBRE", primero.into()), ("OTRO_PROGRAMA", otro.clone())],
        ));
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
}
