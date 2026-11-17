//! Clasificador puro: la ausencia exige evidencia positiva por ingreso e intervalo.
//! El adaptador debe conservar los metadatos crudos antes de reducirlos a Observacion.
use flow_dominio::Fecha;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Clasificacion { SinObservacion, ConObservacion, NoVerificable, FueraDeAlcance }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Autor { Centro, Otro, Desconocido }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evento {
    pub fecha: Option<Fecha>,
    pub texto: String,
    pub autor: Autor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidencia {
    pub admitido: bool,
    pub identidad_verificada: bool,
    pub contradictoria: bool,
    /// Solo Some cuando se acredita completitud de la lectura de ESTE ingreso.
    pub cobertura_completa: Option<(Fecha, Fecha)>,
    /// None significa desconocido/no leído; Some([]) conserva una lectura vacía.
    pub eventos: Option<Vec<Evento>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    pub clasificacion: Clasificacion,
    pub motivo: String,
    pub eventos_elegibles: usize,
    pub ultima_fecha: Option<Fecha>,
    pub cobertura_parcial: bool,
    pub duplicados_identicos: usize,
}

/// Límites inclusivos, fechas civiles de captura; no se convierte una hora sin zona a UTC.
/// Texto vacío no es observación. No se interpretan marcadores libres como ausencia.
pub fn clasificar(e: &Evidencia, desde: Fecha, hasta: Fecha) -> Result<Revision, String> {
    if desde > hasta { return Err("El inicio del período es posterior al término.".into()); }
    let completa = e.cobertura_completa.is_some_and(|(a, b)| a <= desde && hasta <= b);
    let mut r = Revision {
        clasificacion: Clasificacion::NoVerificable,
        motivo: String::new(),
        eventos_elegibles: 0,
        ultima_fecha: None,
        cobertura_parcial: !completa,
        duplicados_identicos: 0,
    };
    if !e.admitido {
        r.clasificacion = Clasificacion::FueraDeAlcance;
        r.motivo = "Categoría no admitida por el motor.".into();
    } else if !e.identidad_verificada || e.contradictoria {
        r.motivo = "Identidad no verificada o evidencia contradictoria.".into();
    } else if let Some(eventos) = &e.eventos {
        let mut unicos: Vec<&Evento> = Vec::new();
        let mut incierto = false;
        for evento in eventos {
            if unicos.contains(&evento) { r.duplicados_identicos += 1; continue; }
            unicos.push(evento);
            if evento.texto.trim().is_empty() { continue; }
            match evento.fecha {
                None => incierto = true,
                Some(d) if desde <= d && d <= hasta => match evento.autor {
                    Autor::Centro => {
                        r.eventos_elegibles += 1;
                        r.ultima_fecha = Some(r.ultima_fecha.map_or(d, |previa| previa.max(d)));
                    }
                    Autor::Desconocido => incierto = true,
                    Autor::Otro => {}
                },
                _ => {}
            }
        }
        if r.eventos_elegibles > 0 {
            r.clasificacion = Clasificacion::ConObservacion;
            r.motivo = "Existe observación sustantiva atribuida al centro dentro del período.".into();
        } else if completa && !incierto {
            r.clasificacion = Clasificacion::SinObservacion;
            r.motivo = "Sin observación del centro en el período verificado.".into();
        } else {
            r.motivo = "Cobertura insuficiente, fecha inválida o autoría desconocida.".into();
        }
    } else {
        r.motivo = "No hay lectura acreditada para este ingreso.".into();
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(n: u32) -> Fecha { format!("2026-11-{n:02}").parse().unwrap() }
    fn base() -> Evidencia {
        Evidencia { admitido: true, identidad_verificada: true, contradictoria: false,
            cobertura_completa: Some((d(1), d(17))), eventos: Some(vec![]) }
    }
    fn evento(autor: Autor, fecha: Option<Fecha>, texto: &str) -> Evento {
        Evento { autor, fecha, texto: texto.into() }
    }
    fn clase(e: &Evidencia) -> Clasificacion { clasificar(e, d(1), d(17)).unwrap().clasificacion }
    #[test]
    fn vacio_acreditado_es_distinto_de_lectura_ausente() {
        let mut e = base();
        assert_eq!(clase(&e), Clasificacion::SinObservacion);
        e.eventos = None;
        assert_eq!(clase(&e), Clasificacion::NoVerificable);
    }
    #[test]
    fn existencia_positiva_no_exige_cobertura_completa() {
        let mut e = base(); e.cobertura_completa = None;
        e.eventos = Some(vec![evento(Autor::Centro, Some(d(1)), "Nota"),
            evento(Autor::Centro, Some(d(17)), "Otra nota")]);
        let r = clasificar(&e, d(1), d(17)).unwrap();
        assert_eq!(r.clasificacion, Clasificacion::ConObservacion);
        assert_eq!(r.eventos_elegibles, 2); assert!(r.cobertura_parcial);
        assert_eq!(r.ultima_fecha, Some(d(17)));
    }
    #[test]
    fn ausencia_exige_periodo_completo_y_autoria_conocida() {
        let mut e = base(); e.eventos = Some(vec![evento(Autor::Otro, Some(d(3)), "Nota")]);
        assert_eq!(clase(&e), Clasificacion::SinObservacion);
        e.eventos.as_mut().unwrap()[0].autor = Autor::Desconocido;
        assert_eq!(clase(&e), Clasificacion::NoVerificable);
        e.eventos.as_mut().unwrap()[0].autor = Autor::Otro;
        e.cobertura_completa = Some((d(2), d(17)));
        assert_eq!(clase(&e), Clasificacion::NoVerificable);
    }
    #[test]
    fn texto_vacio_fecha_invalida_y_fuera_de_intervalo() {
        let mut e = base();
        e.eventos = Some(vec![evento(Autor::Centro, None, "  ")]);
        assert_eq!(clase(&e), Clasificacion::SinObservacion);
        e.eventos = Some(vec![evento(Autor::Centro, None, "Nota")]);
        assert_eq!(clase(&e), Clasificacion::NoVerificable);
        e.eventos = Some(vec![evento(Autor::Centro, Some(d(18)), "Nota")]);
        assert_eq!(clase(&e), Clasificacion::SinObservacion);
    }
    #[test]
    fn precedencia_y_deduplicacion() {
        let mut e = base();
        let v = evento(Autor::Centro, Some(d(3)), "Nota");
        e.eventos = Some(vec![v.clone(), v]);
        let r = clasificar(&e, d(1), d(17)).unwrap();
        assert_eq!(r.eventos_elegibles, 1); assert_eq!(r.duplicados_identicos, 1);
        e.contradictoria = true;
        assert_eq!(clase(&e), Clasificacion::NoVerificable);
        e.admitido = false;
        assert_eq!(clase(&e), Clasificacion::FueraDeAlcance);
        assert!(clasificar(&e, d(17), d(1)).is_err());
    }
}


