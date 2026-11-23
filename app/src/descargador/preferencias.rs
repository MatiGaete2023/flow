//! Preferencias locales del Descargador (`descargador/preferencias.py`): sin filtros privados ni autenticación.

use super::{Catalogo, Pantalla, SeleccionLote};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Consulta favorita (`Preferences.favorite`): el alcance sin filtros privados ni fechas fijas. Las fechas se
/// recalculan al aplicarla (últimos 30 días) y los calendarios guardan el mes relativo (actual o siguiente).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Favorito {
    pub pantalla: Pantalla,
    pub tribunales: Vec<String>,
    pub pestanas: Vec<String>,
    pub modalidades: Vec<String>,
    pub informes: Vec<String>,
    pub medidas: Vec<String>,
    pub estados: Vec<String>,
    pub con_fechas: bool,
    pub desplazamiento_mes: i32,
}

fn mes_absoluto(anio: i32, mes: u32) -> i32 {
    anio * 12 + mes as i32 - 1
}

/// Pausa entre consultas a SITFA (`PAUSA_MINIMA_S`, `PAUSA_DEFECTO_S`, `PAUSA_MAXIMA_S`).
pub const PAUSA_MINIMA_S: f64 = 0.5;
pub const PAUSA_DEFECTO_S: f64 = 1.0;
pub const PAUSA_MAXIMA_S: f64 = 10.0;

/// `clamp_pause`: nunca menos de 0,5 s ni más de 10 s.
pub fn limitar_pausa(valor: f64) -> f64 {
    if valor.is_nan() {
        return PAUSA_DEFECTO_S;
    }
    valor.clamp(PAUSA_MINIMA_S, PAUSA_MAXIMA_S)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferencias {
    pub destino: String,
    pub pdf_sin_registros: bool,
    pub pausa_consultas: f64,
    pub favoritos: BTreeMap<String, Favorito>,
}

impl Default for Preferencias {
    fn default() -> Self {
        Self {
            destino: String::new(),
            pdf_sin_registros: true,
            pausa_consultas: PAUSA_DEFECTO_S,
            favoritos: BTreeMap::new(),
        }
    }
}

impl Preferencias {
    /// `%LOCALAPPDATA%\FlowRust\descargador_preferencias.json`.
    pub fn ruta() -> PathBuf {
        super::extension::carpeta_base().join("descargador_preferencias.json")
    }

    /// Lee las preferencias; un archivo ilegible o con tipos inválidos vuelve a los valores por omisión.
    pub fn cargar(ruta: &Path) -> Self {
        let mut p: Self = std::fs::read(ruta)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        p.pausa_consultas = limitar_pausa(p.pausa_consultas);
        p
    }

    /// Guarda el alcance de `lote` como favorito. Rechaza los filtros privados y, en calendarios, todo lo que no
    /// sea un único mes, el actual o el siguiente.
    pub fn guardar_favorito(
        &mut self,
        nombre: &str,
        pantalla: Pantalla,
        lote: &SeleccionLote,
        conservar_filtros: bool,
        hoy: NaiveDate,
    ) -> Result<(), String> {
        let nombre: String = nombre.trim().chars().take(80).collect();
        if nombre.is_empty() {
            return Err("Escribe un nombre para el favorito.".into());
        }
        if conservar_filtros {
            return Err("Los favoritos no guardan filtros privados preparados en SITFA.".into());
        }
        let mut desplazamiento_mes = 0;
        if matches!(pantalla, Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas) {
            let fuera = || "Los perfiles automáticos admiten el mes actual o siguiente.".to_string();
            let [mes] = lote.meses.as_slice() else { return Err(fuera()) };
            let (mes, anio) = (mes.parse::<u32>().map_err(|_| fuera())?, lote.anio.parse::<i32>().map_err(|_| fuera())?);
            desplazamiento_mes = mes_absoluto(anio, mes) - mes_absoluto(hoy.year(), hoy.month());
            if !(0..=1).contains(&desplazamiento_mes) {
                return Err(fuera());
            }
        }
        self.favoritos.insert(
            nombre,
            Favorito {
                pantalla,
                tribunales: lote.tribunales.clone(),
                pestanas: lote.pestanas.clone(),
                modalidades: lote.modalidades.clone(),
                informes: lote.informes.clone(),
                medidas: lote.medidas.clone(),
                estados: lote.estados.clone(),
                con_fechas: lote.fechas.is_some() || pantalla == Pantalla::Carga,
                desplazamiento_mes,
            },
        );
        Ok(())
    }

    /// `Preferences.restore`: arma el lote del favorito contra el catálogo cargado. Devuelve también los tribunales
    /// retirados porque ya no están en el catálogo.
    pub fn aplicar_favorito(
        &self,
        nombre: &str,
        catalogo: &Catalogo,
        hoy: NaiveDate,
    ) -> Result<(SeleccionLote, Vec<String>), String> {
        let f = self.favoritos.get(nombre).ok_or("Selecciona un favorito.")?;
        if f.pantalla != catalogo.pantalla {
            return Err("Selecciona la pantalla del favorito y carga sus opciones antes de aplicarlo.".into());
        }
        let (tribunales, retirados): (Vec<String>, Vec<String>) =
            f.tribunales.iter().cloned().partition(|t| catalogo.tribunales.contains_key(t));
        let mut lote = SeleccionLote {
            tribunales,
            pestanas: f.pestanas.clone(),
            modalidades: f.modalidades.clone(),
            informes: f.informes.clone(),
            medidas: f.medidas.clone(),
            estados: f.estados.clone(),
            ..Default::default()
        };
        if matches!(f.pantalla, Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas) {
            let absoluto = mes_absoluto(hoy.year(), hoy.month()) + f.desplazamiento_mes;
            let (mes, anio) = ((absoluto % 12 + 1).to_string(), (absoluto / 12).to_string());
            if !catalogo.meses.contains_key(&mes) || !catalogo.anios.contains_key(&anio) {
                return Err("El período relativo del perfil no está disponible en SITFA.".into());
            }
            lote.meses = vec![mes];
            lote.anio = anio;
        }
        if f.con_fechas {
            lote.fechas = Some((hoy - chrono::Duration::days(29), hoy));
        }
        Ok((lote, retirados))
    }

    /// Escritura atómica: archivo temporal y reemplazo.
    pub fn guardar(&self, ruta: &Path) -> Result<(), String> {
        if let Some(carpeta) = ruta.parent() {
            std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
        }
        let datos = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let temporal = ruta.with_extension("tmp");
        std::fs::write(&temporal, datos).map_err(|e| e.to_string())?;
        std::fs::rename(&temporal, ruta).map_err(|e| e.to_string())
    }
}
