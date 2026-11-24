//! Perfiles locales del Descargador, a partir de las opciones anunciadas por cada catálogo.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

mod evidencia;
pub mod bitacoras;
pub mod extension;
pub mod historia;
pub mod historico;
pub mod lecturas;
pub mod vinculos;
pub mod lote;
pub mod paquete;
pub mod preferencias;
mod puente;
pub mod sitfa;
mod xls;
pub use xls::leer_carga;
#[cfg(test)]
mod sitfa_ficticio;
pub use puente::Puente;

pub const PESTANAS: [&str; 4] = ["Espera", "Cumplimiento", "Informes", "Egresados"];
pub const MODALIDADES: [&str; 4] = ["1", "2", "3", "4"];
pub const TODAS_LAS_MEDIDAS: &str = "0";

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

    fn codigo(self) -> &'static str {
        match self {
            Self::Seguimiento => "seguimiento",
            Self::Litigantes => "litigantes",
            Self::CalendarioInformes => "calendario_informes",
            Self::CalendarioMedidas => "calendario_medidas",
            Self::Carga => "carga",
        }
    }

    fn disponible(self, edicion: Edicion) -> bool {
        match edicion {
            Edicion::Integral => Self::INTEGRAL.contains(&self),
            Edicion::Lite => Self::LITE.contains(&self),
        }
    }
}

impl Puente {
    pub fn catalogo(&self, edicion: Edicion, pantalla: Pantalla) -> Result<Catalogo, String> {
        if !pantalla.disponible(edicion) {
            return Err("Carga está disponible en Integral.".into());
        }
        let datos = self.llamar(
            "catalog",
            serde_json::json!({"screen": pantalla.codigo()}),
            Duration::from_secs(75),
        )?;
        if datos["pantalla"].as_str() != Some(pantalla.codigo()) {
            return Err("Chrome devolvió un catálogo de otra pantalla.".into());
        }
        // La extensión fija el marco del formulario al cargar el catálogo.
        let version = self.llamar(
            "capabilities",
            serde_json::json!({}),
            Duration::from_secs(15),
        )?;
        comprobar_extension(edicion, &version)?;
        serde_json::from_value::<Catalogo>(datos)
            .map_err(|_| "Chrome no devolvió un catálogo válido.".into())
    }
}

impl Puente {
    /// `version.verify_extension`: antes de un lote se confirma otra vez la edición y el modo de solo lectura.
    pub fn verificar(&self, edicion: Edicion) -> Result<(), String> {
        let version = self.llamar(
            "capabilities",
            serde_json::json!({}),
            Duration::from_secs(15),
        )?;
        let resultado = comprobar_extension(edicion, &version);
        // E-Q3: las lecturas de contexto quedan habilitadas solo tras verificar la extensión Integral.
        self.habilitar_lecturas(resultado.is_ok() && edicion == Edicion::Integral);
        resultado
    }
}

pub(crate) fn comprobar_extension(edicion: Edicion, version: &serde_json::Value) -> Result<(), String> {
    let (tipo, numero, protocolos) = match edicion {
        Edicion::Integral => (
            "CSMP_RUS_CAPACIDADES",
            "2.9.0",
            serde_json::json!({
                "bitacoras_lectura": 1, "descarga_conjunta": 1, "pdf_seleccion": 1,
                "pdf_copia_fija": 1, "calendario_lectura": 1, "historia_lectura": 1, "historico_lectura": 2,
            }),
        ),
        Edicion::Lite => (
            "SITFA_LITE_CAPACIDADES",
            "1.1.0",
            serde_json::json!({
                "descarga_xls": 1, "pdf_sin_registros": 1,
            }),
        ),
    };
    if version["tipo"] == tipo
        && version["version"] == numero
        && version["protocolos"] == protocolos
        && version["escritura_rus"] == false
    {
        return Ok(());
    }
    Err(format!("Carga en Chrome la extensión Flow Python de la edición elegida ({numero}) y vuelve a conectar."))
}

#[derive(Clone, Debug, Default, Deserialize)]
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

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SeleccionLote {
    pub tribunales: Vec<String>,
    pub pestanas: Vec<String>,
    pub modalidades: Vec<String>,
    pub informes: Vec<String>,
    pub medidas: Vec<String>,
    pub estados: Vec<String>,
    pub meses: Vec<String>,
    pub anio: String,
    pub fechas: Option<(NaiveDate, NaiveDate)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlanLote {
    pub consultas: Vec<Perfil>,
    pub fechas: Option<(NaiveDate, NaiveDate)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Perfil {
    pub pantalla: Pantalla,
    pub tribunal: String,
    pub alcance: Alcance,
    pub etiqueta: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
    pub fn planificar(&self, edicion: Edicion, lote: &SeleccionLote) -> Result<PlanLote, String> {
        if !self.pantalla.disponible(edicion) {
            return Err(format!(
                "{} está disponible en Integral.",
                self.pantalla.etiqueta()
            ));
        }
        if lote.tribunales.is_empty() {
            return Err("Marca al menos un tribunal.".into());
        }
        let dimensiones = [
            &lote.tribunales,
            &lote.pestanas,
            &lote.modalidades,
            &lote.informes,
            &lote.medidas,
            &lote.estados,
            &lote.meses,
        ];
        if dimensiones
            .iter()
            .any(|v| v.iter().collect::<HashSet<_>>().len() != v.len())
        {
            return Err("El lote tiene opciones repetidas.".into());
        }
        let calendario = matches!(
            self.pantalla,
            Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas
        );
        if let Some((desde, hasta)) = lote.fechas {
            if calendario {
                return Err("El calendario usa mes y año en el lote.".into());
            }
            if desde > hasta
                || [desde, hasta]
                    .iter()
                    .any(|d| !(1900..=2100).contains(&d.year()))
            {
                return Err(
                    "Usa fechas válidas entre 1900 y 2100, con Desde anterior o igual a Hasta."
                        .into(),
                );
            }
        }
        let variantes = match self.pantalla {
            Pantalla::Seguimiento => Self::seguimiento_lote(edicion, lote)?,
            Pantalla::Litigantes => {
                if lote.estados.is_empty() {
                    return Err("Marca al menos un estado.".into());
                }
                lote.estados
                    .iter()
                    .map(|estado| Seleccion {
                        estado: estado.clone(),
                        ..Default::default()
                    })
                    .collect()
            }
            Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas => {
                if lote.meses.is_empty() || lote.anio.is_empty() {
                    return Err("Marca al menos un mes y elige un año.".into());
                }
                lote.meses
                    .iter()
                    .map(|mes| Seleccion {
                        mes: mes.clone(),
                        anio: lote.anio.clone(),
                        ..Default::default()
                    })
                    .collect()
            }
            Pantalla::Carga => {
                let (desde, hasta) = lote.fechas.ok_or("Carga requiere un rango de fechas.")?;
                vec![Seleccion {
                    desde: Some(desde),
                    hasta: Some(hasta),
                    ..Default::default()
                }]
            }
        };
        let mut consultas = Vec::new();
        for tribunal in &lote.tribunales {
            for variante in &variantes {
                let mut seleccion = variante.clone();
                seleccion.tribunal.clone_from(tribunal);
                consultas.push(self.preparar(edicion, &seleccion)?);
            }
        }
        Ok(PlanLote {
            consultas,
            fechas: lote.fechas,
        })
    }

    fn seguimiento_lote(edicion: Edicion, lote: &SeleccionLote) -> Result<Vec<Seleccion>, String> {
        if lote.modalidades.is_empty() || lote.pestanas.is_empty() {
            return Err("Marca al menos una modalidad y una pestaña.".into());
        }
        if lote
            .modalidades
            .iter()
            .any(|m| !MODALIDADES.contains(&m.as_str()))
            || lote
                .pestanas
                .iter()
                .any(|p| !PESTANAS.contains(&p.as_str()))
        {
            return Err("Modalidad o pestaña fuera de las opciones admitidas.".into());
        }
        let pestanas: Vec<&str> = match edicion {
            Edicion::Integral => lote.pestanas.iter().map(String::as_str).collect(),
            Edicion::Lite => PESTANAS
                .into_iter()
                .filter(|p| lote.pestanas.iter().any(|s| s == p))
                .collect(),
        };
        let sin_filtro = [String::new()];
        let mut variantes = Vec::new();
        for pestana in pestanas {
            let reportes = match pestana {
                "Informes" if lote.informes.is_empty() => {
                    return Err("Marca al menos un tipo de informe.".into())
                }
                "Cumplimiento" if lote.medidas.is_empty() => {
                    return Err("Marca al menos un filtro de medidas.".into())
                }
                "Informes" => lote.informes.as_slice(),
                "Cumplimiento" => lote.medidas.as_slice(),
                _ => &sin_filtro,
            };
            for modalidad in &lote.modalidades {
                for reporte in reportes {
                    variantes.push(Seleccion {
                        pestana: pestana.into(),
                        modalidad: modalidad.clone(),
                        reporte: reporte.clone(),
                        ..Default::default()
                    });
                }
            }
        }
        Ok(variantes)
    }

    /// `prepare_pending`: vuelve a preparar contra el catálogo cargado las consultas pendientes de un lote.
    pub fn planificar_pendientes(&self, edicion: Edicion, p: &lote::Pendientes) -> Result<PlanLote, String> {
        if self.pantalla != p.pantalla {
            return Err("Selecciona la pantalla del lote y carga sus opciones primero.".into());
        }
        let consultas = p.selecciones.iter().map(|s| self.preparar(edicion, s)).collect::<Result<Vec<_>, _>>()?;
        Ok(PlanLote { consultas, fechas: p.fechas })
    }

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
        if !PESTANAS.contains(&s.pestana.as_str()) || !self.pestanas.contains(&s.pestana) {
            return Err("Selecciona una pestaña disponible.".into());
        }
        let modalidad = self
            .modalidades
            .get(&s.modalidad)
            .ok_or("Selecciona una modalidad disponible.")?;
        let filtro: Option<&str> = match s.pestana.as_str() {
            "Informes" => Some(
                self.informes
                    .get(&s.reporte)
                    .ok_or("Selecciona un tipo de informe.")?,
            ),
            "Cumplimiento" if s.reporte == TODAS_LAS_MEDIDAS => Some("Todas las medidas"),
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
            reporte: filtro
                .filter(|_| s.pestana != "Cumplimiento" || s.reporte != TODAS_LAS_MEDIDAS)
                .map(|_| s.reporte.clone()),
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

    #[test]
    fn lote_lite_ordena_pestanas_y_combina_tribunales_modalidades_y_filtros() {
        let mut modelo = catalogo(Pantalla::Seguimiento);
        modelo
            .tribunales
            .insert("otro".into(), "Otro tribunal ficticio".into());
        modelo
            .modalidades
            .extend(MODALIDADES.map(|m| (m.into(), format!("Modalidad {m}"))));
        let mut lote = SeleccionLote {
            tribunales: vec!["ficticio".into(), "otro".into()],
            modalidades: MODALIDADES.map(String::from).into(),
            pestanas: vec!["Egresados".into(), "Espera".into()],
            ..Default::default()
        };
        let plan = modelo.planificar(Edicion::Lite, &lote).unwrap();
        let orden: Vec<_> = plan
            .consultas
            .iter()
            .map(|p| match &p.alcance {
                Alcance::Seguimiento {
                    pestana,
                    modalidad,
                    reporte,
                } => {
                    assert!(reporte.is_none());
                    (p.tribunal.as_str(), pestana.as_str(), modalidad.as_str())
                }
                _ => panic!("Consulta fuera de Seguimiento"),
            })
            .collect();
        let esperado: Vec<_> = ["ficticio", "otro"]
            .into_iter()
            .flat_map(|t| {
                ["Espera", "Egresados"]
                    .into_iter()
                    .flat_map(move |p| MODALIDADES.map(|m| (t, p, m)))
            })
            .collect();
        assert_eq!(orden, esperado);

        lote.pestanas = vec!["Informes".into(), "Cumplimiento".into()];
        assert!(modelo
            .planificar(Edicion::Lite, &lote)
            .unwrap_err()
            .contains("medidas"));
        lote.medidas = vec![TODAS_LAS_MEDIDAS.into(), "m".into()];
        assert!(modelo
            .planificar(Edicion::Lite, &lote)
            .unwrap_err()
            .contains("tipo de informe"));
        lote.informes = vec!["i".into()];
        let plan = modelo.planificar(Edicion::Lite, &lote).unwrap();
        assert_eq!(plan.consultas.len(), 24);
        assert!(matches!(&plan.consultas[0].alcance,
            Alcance::Seguimiento { pestana, reporte: None, .. } if pestana == "Cumplimiento"));
        assert!(matches!(&plan.consultas[1].alcance,
            Alcance::Seguimiento { reporte: Some(r), .. } if r == "m"));
        assert!(matches!(&plan.consultas[8].alcance,
            Alcance::Seguimiento { pestana, reporte: Some(r), .. } if pestana == "Informes" && r == "i"));
        let integral = modelo.planificar(Edicion::Integral, &lote).unwrap();
        assert!(matches!(&integral.consultas[0].alcance,
            Alcance::Seguimiento { pestana, .. } if pestana == "Informes"));
        lote.medidas.push("m".into());
        assert!(modelo
            .planificar(Edicion::Lite, &lote)
            .unwrap_err()
            .contains("repetidas"));
    }

    #[test]
    fn lote_valida_estado_meses_y_fechas_segun_pantalla() {
        let mut lote = SeleccionLote {
            tribunales: vec!["ficticio".into()],
            estados: vec!["1".into()],
            meses: vec!["10".into()],
            anio: "2026".into(),
            ..Default::default()
        };
        assert_eq!(
            catalogo(Pantalla::Litigantes)
                .planificar(Edicion::Lite, &lote)
                .unwrap()
                .consultas
                .len(),
            1
        );
        let modelo = catalogo(Pantalla::CalendarioInformes);
        assert_eq!(
            modelo
                .planificar(Edicion::Lite, &lote)
                .unwrap()
                .consultas
                .len(),
            1
        );
        assert!(catalogo(Pantalla::Carga)
            .planificar(Edicion::Integral, &lote)
            .is_err());
        lote.fechas = Some(("2026-10-01".parse().unwrap(), "2026-10-31".parse().unwrap()));
        assert!(modelo.planificar(Edicion::Lite, &lote).is_err());
        let modelo = catalogo(Pantalla::Carga);
        assert!(modelo.planificar(Edicion::Lite, &lote).is_err());
        let plan = modelo.planificar(Edicion::Integral, &lote).unwrap();
        assert_eq!(plan.fechas, lote.fechas);
        assert_eq!(plan.consultas.len(), 1);
        lote.tribunales.push("desconocido".into());
        assert!(modelo.planificar(Edicion::Integral, &lote).is_err());
    }

    #[test]
    fn extension_integral_exige_protocolos_de_solo_lectura_y_edicion_correcta() {
        let mut version = serde_json::json!({
            "tipo": "CSMP_RUS_CAPACIDADES", "version": "2.9.0", "escritura_rus": false,
            "protocolos": {
                "bitacoras_lectura": 1, "descarga_conjunta": 1, "pdf_seleccion": 1,
                "pdf_copia_fija": 1, "calendario_lectura": 1, "historia_lectura": 1, "historico_lectura": 2,
            },
        });
        assert!(comprobar_extension(Edicion::Integral, &version).is_ok());
        assert!(comprobar_extension(Edicion::Lite, &version).is_err());
        version["escritura_rus"] = serde_json::json!(true);
        assert!(comprobar_extension(Edicion::Integral, &version).is_err());
    }
}
