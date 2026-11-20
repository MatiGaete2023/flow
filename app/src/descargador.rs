//! Perfiles locales del Descargador, a partir de las opciones anunciadas por cada catálogo.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Edicion {
    #[default]
    Integral,
    Lite,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pantalla {
    #[default]
    Seguimiento,
    Litigantes,
    CalendarioInformes,
    CalendarioMedidas,
    Carga,
}

impl Pantalla {
    pub const LITE: [Self; 4] = [
        Self::Seguimiento,
        Self::Litigantes,
        Self::CalendarioInformes,
        Self::CalendarioMedidas,
    ];

    pub const INTEGRAL: [Self; 5] = [
        Self::Seguimiento,
        Self::Litigantes,
        Self::CalendarioInformes,
        Self::CalendarioMedidas,
        Self::Carga,
    ];

    pub fn etiqueta(self) -> &'static str {
        match self {
            Self::Seguimiento => "Seguimiento",
            Self::Litigantes => "Búsqueda de litigantes",
            Self::CalendarioInformes => "Informes por vencer",
            Self::CalendarioMedidas => "Medidas por vencer",
            Self::Carga => "Carga · resoluciones firmadas",
        }
    }

    fn disponible(self, edicion: Edicion) -> bool {
        match edicion {
            Edicion::Integral => Self::INTEGRAL.contains(&self),
            Edicion::Lite => Self::LITE.contains(&self),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Catalogo {
    #[serde(default)]
    pub pantalla: Pantalla,
    pub tribunales: BTreeMap<String, String>,
    #[serde(default)]
    pub pestanas: Vec<String>,
    #[serde(default)]
    pub modalidades: BTreeMap<String, String>,
    #[serde(default)]
    pub informes: BTreeMap<String, String>,
    #[serde(default)]
    pub medidas: BTreeMap<String, String>,
    #[serde(default)]
    pub estados: BTreeMap<String, String>,
    #[serde(default)]
    pub meses: BTreeMap<String, String>,
    #[serde(default)]
    pub anios: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seleccion {
    pub tribunal: String,
    pub pestana: String,
    pub modalidad: String,
    pub reporte: String,
    pub estado: String,
    pub mes: String,
    pub anio: String,
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Perfil {
    pub pantalla: Pantalla,
    pub tribunal: String,
    pub alcance: Alcance,
    pub etiqueta: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub enum Alcance {
    Seguimiento {
        pestana: String,
        modalidad: String,
        reporte: Option<String>,
    },
    Litigantes {
        estado: String,
    },
    Calendario {
        mes: String,
        anio: String,
    },
    Carga {
        desde: NaiveDate,
        hasta: NaiveDate,
    },
}

impl Catalogo {
    pub fn preparar(&self, edicion: Edicion, s: &Seleccion) -> Result<Perfil, String> {
        if !self.pantalla.disponible(edicion) {
            return Err(format!(
                "{} está disponible en Integral.",
                self.pantalla.etiqueta()
            ));
        }
        let tribunal = self
            .tribunales
            .get(&s.tribunal)
            .ok_or("Selecciona un tribunal del catálogo.")?;
        let (alcance, detalle) = match self.pantalla {
            Pantalla::Seguimiento => self.seguimiento(s)?,
            Pantalla::Litigantes => {
                let estado = self
                    .estados
                    .get(&s.estado)
                    .ok_or("Selecciona un estado de litigantes.")?;
                (
                    Alcance::Litigantes {
                        estado: s.estado.clone(),
                    },
                    estado.to_owned(),
                )
            }
            Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas => {
                let mes = self
                    .meses
                    .get(&s.mes)
                    .filter(|_| s.mes.parse::<u32>().is_ok_and(|n| (1..=12).contains(&n)));
                let anio = self
                    .anios
                    .get(&s.anio)
                    .filter(|_| s.anio.parse::<u32>().is_ok_and(|n| (1..=9999).contains(&n)));
                let (mes, anio) = mes
                    .zip(anio)
                    .ok_or("Selecciona un mes y año disponibles.")?;
                (
                    Alcance::Calendario {
                        mes: s.mes.clone(),
                        anio: s.anio.clone(),
                    },
                    format!("{mes} {anio}"),
                )
            }
            Pantalla::Carga => {
                let (desde, hasta) = s
                    .desde
                    .zip(s.hasta)
                    .filter(|(a, b)| (0..=30).contains(&(*b - *a).num_days()))
                    .ok_or("Selecciona un período válido de hasta 31 días inclusivos.")?;
                (
                    Alcance::Carga { desde, hasta },
                    format!(
                        "{} a {}",
                        desde.format("%d/%m/%Y"),
                        hasta.format("%d/%m/%Y")
                    ),
                )
            }
        };
        Ok(Perfil {
            pantalla: self.pantalla,
            tribunal: s.tribunal.clone(),
            alcance,
            etiqueta: format!("{tribunal} · {} · {detalle}", self.pantalla.etiqueta()),
        })
    }

    fn seguimiento(&self, s: &Seleccion) -> Result<(Alcance, String), String> {
        if !matches!(
            s.pestana.as_str(),
            "Espera" | "Cumplimiento" | "Informes" | "Egresados"
        ) || !self.pestanas.contains(&s.pestana)
        {
            return Err("Selecciona una pestaña disponible.".into());
        }
        let modalidad = self
            .modalidades
            .get(&s.modalidad)
            .ok_or("Selecciona una modalidad disponible.")?;
        let filtro = match s.pestana.as_str() {
            "Informes" => Some(
                self.informes
                    .get(&s.reporte)
                    .ok_or("Selecciona un tipo de informe.")?,
            ),
            "Cumplimiento" => Some(
                self.medidas
                    .get(&s.reporte)
                    .ok_or("Selecciona un filtro de medidas.")?,
            ),
            _ => None,
        };
        let mut detalle = format!("{} · {modalidad}", s.pestana);
        if let Some(filtro) = filtro {
            detalle.push_str(" · ");
            detalle.push_str(filtro);
        }
        let alcance = Alcance::Seguimiento {
            pestana: s.pestana.clone(),
            modalidad: s.modalidad.clone(),
            reporte: filtro.map(|_| s.reporte.clone()),
        };
        Ok((alcance, detalle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalogo(pantalla: Pantalla) -> Catalogo {
        serde_json::from_value(serde_json::json!({
            "pantalla": pantalla, "tribunales": {"ficticio": "Tribunal ficticio"},
            "pestanas": ["Espera", "Cumplimiento", "Informes", "Egresados"],
            "modalidades": {"2": "Ambulatorio"}, "informes": {"i": "Por vencer"},
            "medidas": {"m": "Vigentes"}, "estados": {"1": "Vigentes"},
            "meses": {"10": "Octubre", "20": "Mes ficticio"}, "anios": {"2026": "2026"},
        }))
        .unwrap()
    }

    #[test]
    fn lite_admite_cuatro_consultas_y_carga_respeta_el_limite_inclusivo() {
        assert_eq!(Pantalla::LITE.len(), 4);
        assert!(!Pantalla::Carga.disponible(Edicion::Lite));
        assert!(serde_json::from_str::<Pantalla>(r#""bitacoras""#).is_err());
        let modelo = catalogo(Pantalla::Carga);
        let mut s = Seleccion {
            tribunal: "ficticio".into(),
            desde: "2026-10-01".parse().ok(),
            hasta: "2026-10-31".parse().ok(),
            ..Default::default()
        };
        assert!(modelo.preparar(Edicion::Integral, &s).is_ok());
        assert!(modelo.preparar(Edicion::Lite, &s).is_err());
        s.hasta = "2026-11-01".parse().ok();
        assert!(modelo.preparar(Edicion::Integral, &s).is_err());
    }

    #[test]
    fn seguimiento_valida_el_filtro_propio_de_cada_pestana() {
        let modelo = catalogo(Pantalla::Seguimiento);
        let mut s = Seleccion {
            tribunal: "ficticio".into(),
            modalidad: "2".into(),
            pestana: "Informes".into(),
            ..Default::default()
        };
        assert!(modelo
            .preparar(Edicion::Lite, &s)
            .unwrap_err()
            .contains("tipo de informe"));
        s.reporte = "i".into();
        assert!(modelo.preparar(Edicion::Lite, &s).is_ok());
        s.pestana = "Cumplimiento".into();
        assert!(modelo
            .preparar(Edicion::Lite, &s)
            .unwrap_err()
            .contains("medidas"));
        s.reporte = "m".into();
        assert!(modelo.preparar(Edicion::Integral, &s).is_ok());
        s.pestana = "Espera".into();
        let perfil = modelo.preparar(Edicion::Lite, &s).unwrap();
        assert!(matches!(
            perfil.alcance,
            Alcance::Seguimiento { reporte: None, .. }
        ));
    }

    #[test]
    fn valida_estado_de_litigantes_y_mes_anio_de_calendarios() {
        let mut s = Seleccion {
            tribunal: "ficticio".into(),
            estado: "1".into(),
            mes: "10".into(),
            anio: "2026".into(),
            ..Default::default()
        };
        assert!(catalogo(Pantalla::Litigantes)
            .preparar(Edicion::Lite, &s)
            .is_ok());
        for pantalla in [Pantalla::CalendarioInformes, Pantalla::CalendarioMedidas] {
            assert!(catalogo(pantalla).preparar(Edicion::Lite, &s).is_ok());
        }
        s.mes = "20".into();
        assert!(catalogo(Pantalla::CalendarioInformes)
            .preparar(Edicion::Integral, &s)
            .is_err());
        s.estado = "99".into();
        assert!(catalogo(Pantalla::Litigantes)
            .preparar(Edicion::Integral, &s)
            .is_err());
    }
}
