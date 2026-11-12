//! Consultas de lectura auditadas de SITFA Lite. Ningún valor del catálogo amplía la lista de operaciones.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const DESTINO: &str = "/SITFAWEB/InformesDAction.do";
pub const DESTINO_ABSOLUTO: &str = "https://familia.pjud.cl/SITFAWEB/InformesDAction.do";
pub const PESTANAS: [(&str, &str, &str, &str, &str); 4] = [
    ("Espera", "tdEspera", "NUM_PaginaEspera", "NUM_TotalEspera", "tablaEspera"),
    ("Cumplimiento", "tdCumplimiento", "NUM_PaginaCumplimiento", "NUM_TotalCumplimiento", "tablaCumplimiento"),
    ("Informes", "tdInforme", "NUM_PaginaInforme", "NUM_TotalInforme", "tablaInforme"),
    ("Egresados", "tdEgreso", "NUM_PaginaEgreso", "NUM_TotalEgreso", "tablaEgresados"),
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Seleccion {
    pub tribunal: String,
    #[serde(default = "seguimiento")]
    pub screen: String,
    #[serde(default)]
    pub tab: String,
    #[serde(default)]
    pub modality: String,
    pub report: Option<String>,
    pub state: Option<String>,
    pub month: Option<String>,
    pub year: Option<String>,
}

fn seguimiento() -> String {
    "seguimiento".into()
}

#[derive(Debug, Clone)]
pub struct Perfil {
    pub(super) pantalla: String,
    pub(super) objetivo: BTreeMap<String, String>,
    pub(super) accion: &'static str,
    pub(super) pestaña: &'static str,
    pub(super) pagina: &'static str,
    pub(super) total: &'static str,
    pub(super) tabla: &'static str,
    pub(super) plantilla: Option<&'static str>,
}

impl Perfil {
    pub fn desde_catalogo(s: &Seleccion, catalogo: &Value) -> Result<Self, String> {
        let disponible = |grupo: &str, valor: &str| catalogo[grupo].as_object().is_some_and(|g| g.contains_key(valor));
        if catalogo["pantalla"].as_str().unwrap_or("seguimiento") != s.screen || !disponible("tribunales", &s.tribunal) {
            return Err("Carga las opciones de la pantalla y selecciona un tribunal disponible.".into());
        }
        let mut p = Perfil {
            pantalla: s.screen.clone(),
            objetivo: BTreeMap::from([("COD_Tribunal_sel".into(), s.tribunal.clone())]),
            accion: "",
            pestaña: "",
            pagina: "",
            total: "",
            tabla: "",
            plantilla: None,
        };
        match s.screen.as_str() {
            "seguimiento" => {
                let tab = PESTANAS.iter().find(|t| t.0 == s.tab).ok_or("Pestaña no permitida.")?;
                if !disponible("modalidades", &s.modality)
                    || !catalogo["pestanas"].as_array().is_some_and(|ts| ts.iter().any(|t| t.as_str() == Some(&s.tab)))
                {
                    return Err("Selecciona una pestaña y modalidad disponibles.".into());
                }
                p.objetivo.insert("TIP_Consulta".into(), s.modality.clone());
                p.objetivo.insert("COD_Lengueta".into(), tab.1.into());
                let grupo = if s.tab == "Informes" { "informes" } else { "medidas" };
                if s.tab == "Informes" && s.report.is_none()
                    || matches!(s.tab.as_str(), "Informes" | "Cumplimiento") && s.report.as_ref().is_some_and(|r| !disponible(grupo, r))
                {
                    return Err("Selecciona un filtro de vencimiento disponible.".into());
                }
                if matches!(s.tab.as_str(), "Informes" | "Cumplimiento") {
                    if let Some(r) = &s.report {
                        p.objetivo.insert("TIP_Informe".into(), r.clone());
                    }
                }
                (p.pestaña, p.pagina, p.total, p.tabla) = (tab.1, tab.2, tab.3, tab.4);
                p.accion = "Buscar medida";
            }
            "litigantes" => {
                let estado = s.state.as_deref().unwrap_or("1");
                if !disponible("estados", estado) {
                    return Err("Selecciona un estado de litigantes disponible.".into());
                }
                p.objetivo.insert("TIP_Consulta".into(), "2".into());
                p.objetivo.insert("COD_EstBusqueda".into(), estado.into());
                (p.accion, p.pagina, p.total, p.plantilla) = ("Consulta Informe", "NUM_PaginaInforme", "NUM_Total", Some("BusquedaLit"));
            }
            "calendario_informes" | "calendario_medidas" => {
                let (mes, año) = (s.month.as_deref().unwrap_or(""), s.year.as_deref().unwrap_or(""));
                if !disponible("meses", mes)
                    || !disponible("anios", año)
                    || !mes.parse::<u32>().is_ok_and(|n| (1..=12).contains(&n))
                    || !año.parse::<i32>().is_ok_and(|n| (1..=9999).contains(&n))
                {
                    return Err("Selecciona un mes y año disponibles.".into());
                }
                p.objetivo.insert("TIP_Consulta".into(), "1".into());
                p.objetivo.insert("COD_Mes_sel".into(), mes.into());
                p.objetivo.insert("COD_Anio_Sel".into(), año.into());
                (p.accion, p.plantilla) = if s.screen == "calendario_informes" {
                    ("Buscar Inf.", Some("InfoPorVencer"))
                } else {
                    ("Buscar", Some("MedPorVencer"))
                };
            }
            _ => return Err("Pantalla de consulta no permitida.".into()),
        }
        Ok(p)
    }

    pub(super) fn calendario(&self) -> bool {
        self.pantalla.starts_with("calendario_")
    }

    pub fn validar_consulta(&self, pares: &[(String, String)]) -> Result<(), String> {
        let mut unicos = BTreeMap::new();
        if pares.is_empty() || pares.len() > 100 {
            return Err("Consulta vacía o con demasiados campos.".into());
        }
        for (k, v) in pares {
            if !self.permitido(k) || unicos.insert(k.as_str(), v.as_str()).is_some() {
                return Err("Consulta con campos repetidos o no auditados.".into());
            }
        }
        if unicos.get("irAccion").copied() != Some(self.accion)
            || self.objetivo.iter().any(|(k, v)| unicos.get(k.as_str()).copied() != Some(v.as_str()))
        {
            return Err("La solicitud no corresponde a la consulta de lectura seleccionada.".into());
        }
        Ok(())
    }

    fn permitido(&self, campo: &str) -> bool {
        let comunes = "COD_Centro GLS_TribunalOrigen FLG_Consulta COD_Tribunal_sel TIP_Consulta TIP_Causa ROL_Causa ERA_Causa RUT_Consulta RUT_DvConsulta CHK_Consulta FEC_Inicio FEC_Fin irAccion";
        let adicionales = match self.pantalla.as_str() {
            "seguimiento" => "COD_Lengueta COD_CentroResidencial COD_PlazoIntervencion TIP_Informe COD_TiempoEspera COD_TipLitigante NUM_PaginaEspera NUM_TotalEspera NUM_PaginaCumplimiento NUM_TotalCumplimiento NUM_PaginaInforme NUM_TotalInforme NUM_PaginaEgreso NUM_TotalEgreso",
            "litigantes" => "COD_Medida COD_EstBusqueda NUM_PaginaInforme NUM_Total",
            _ => return "FLG_Busqueda COD_TribunalActual GLS_TribunalActual busquedaPorTribunal TIP_Consulta COD_Tribunal_sel FEC_Inicio COD_Mes_sel COD_Anio_Sel CHK_Contrae irAccion".split_whitespace().any(|k| k == campo),
        };
        comunes.split_whitespace().chain(adicionales.split_whitespace()).any(|k| k == campo)
    }
}
