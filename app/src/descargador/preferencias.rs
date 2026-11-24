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
    /// Lecturas de contexto del lote de bitácoras (`preferencias.py`, mismas claves y valores por omisión).
    pub reutilizar_bitacoras: bool,
    pub leer_calendarios: bool,
    pub leer_historia: bool,
    pub leer_historico: bool,
    pub historia_en_paquete: bool,
    pub meses_atras: u32,
    /// `paquete_consolidado`: el paquete lleva el XLSX «Para CSMP» (D6).
    pub paquete_consolidado: bool,
}

impl Default for Preferencias {
    fn default() -> Self {
        Self {
            destino: String::new(),
            pdf_sin_registros: true,
            pausa_consultas: PAUSA_DEFECTO_S,
            favoritos: BTreeMap::new(),
            reutilizar_bitacoras: true,
            leer_calendarios: false,
            leer_historia: false,
            leer_historico: false,
            historia_en_paquete: true,
            meses_atras: 4,
            paquete_consolidado: true,
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
        if !(1..=4).contains(&p.meses_atras) {
            p.meses_atras = 4;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descargador::{lote, Edicion};

    #[test]
    fn a3_preferencias_descargador_limites_y_persistencia() {
        // limitar_pausa(0.0) = 0.5, limitar_pausa(30.0) = 10.0 y limitar_pausa(NaN) = 1.0
        assert_eq!(limitar_pausa(0.0), 0.5);
        assert_eq!(limitar_pausa(30.0), 10.0);
        assert_eq!(limitar_pausa(f64::NAN), 1.0);

        let temp_dir = std::env::temp_dir().join(format!("flow_a3_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let ruta_invalida = temp_dir.join("invalido.json");

        // Cargar de un JSON inválido devuelve valores por omisión
        std::fs::write(&ruta_invalida, b"not a valid json").unwrap();
        let cargado = Preferencias::cargar(&ruta_invalida);
        assert_eq!(cargado, Preferencias::default());

        // Cargar con tipos erróneos devuelve valores por omisión
        std::fs::write(&ruta_invalida, br#"{"pausa_consultas": "diez", "pdf_sin_registros": "no"}"#).unwrap();
        let cargado2 = Preferencias::cargar(&ruta_invalida);
        assert_eq!(cargado2, Preferencias::default());

        // Guardar seguido de cargar devuelve los mismos valores, sin archivos .tmp
        let ruta_valida = temp_dir.join("preferencias_guardadas.json");
        let mut prefs = Preferencias {
            destino: "C:\\Descargas".into(),
            pdf_sin_registros: false,
            pausa_consultas: 3.5,
            favoritos: BTreeMap::new(),
            leer_historico: true,
            meses_atras: 2,
            ..Default::default()
        };
        prefs.favoritos.insert("fav1".into(), Favorito {
            pantalla: Pantalla::Seguimiento,
            tribunales: vec!["113".into()],
            ..Default::default()
        });

        prefs.guardar(&ruta_valida).unwrap();
        assert!(!ruta_valida.with_extension("tmp").exists(), "No deben quedar archivos .tmp");

        let recuperadas = Preferencias::cargar(&ruta_valida);
        assert_eq!(recuperadas, prefs);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn a6_favoritos_restricciones_fechas_y_catalogos() {
        let mut prefs = Preferencias::default();
        let hoy = NaiveDate::from_ymd_opt(2026, 12, 15).unwrap();

        let lote = SeleccionLote {
            tribunales: vec!["113".into(), "999".into()],
            meses: vec!["1".into()],
            anio: "2027".into(),
            ..Default::default()
        };

        // Un favorito con conservar_filtros se rechaza
        let err = prefs.guardar_favorito("con_filtros", Pantalla::Seguimiento, &lote, true, hoy);
        assert_eq!(err, Err("Los favoritos no guardan filtros privados preparados en SITFA.".into()));

        // En calendarios: dos meses se rechazan
        let lote_dos_meses = SeleccionLote {
            meses: vec!["12".into(), "1".into()],
            anio: "2026".into(),
            ..Default::default()
        };
        let err_dos = prefs.guardar_favorito("dos_meses", Pantalla::CalendarioInformes, &lote_dos_meses, false, hoy);
        assert!(err_dos.is_err());

        // En calendarios: mes a más de 1 de distancia se rechaza (febrero 2027 desde dic 2026 -> +2)
        let lote_lejos = SeleccionLote {
            meses: vec!["2".into()],
            anio: "2027".into(),
            ..Default::default()
        };
        let err_lejos = prefs.guardar_favorito("lejos", Pantalla::CalendarioInformes, &lote_lejos, false, hoy);
        assert!(err_lejos.is_err());

        // El mes siguiente guardado en diciembre (enero 2027, desplazamiento +1) se guarda y aplica
        let lote_siguiente = SeleccionLote {
            tribunales: vec!["113".into(), "999".into()],
            meses: vec!["1".into()],
            anio: "2027".into(),
            ..Default::default()
        };
        assert!(prefs.guardar_favorito("siguiente_mes", Pantalla::CalendarioInformes, &lote_siguiente, false, hoy).is_ok());

        let mut catalogo = Catalogo {
            pantalla: Pantalla::CalendarioInformes,
            ..Default::default()
        };
        catalogo.tribunales.insert("113".into(), "Tribunal Ficticio".into());
        catalogo.meses.insert("1".into(), "Enero".into());
        catalogo.anios.insert("2027".into(), "2027".into());

        // Tribunal ausente del catálogo se retira y se informa
        let (lote_aplicado, retirados) = prefs.aplicar_favorito("siguiente_mes", &catalogo, hoy).unwrap();
        assert_eq!(lote_aplicado.tribunales, vec!["113"]);
        assert_eq!(retirados, vec!["999"]);
        assert_eq!(lote_aplicado.meses, vec!["1"]);
        assert_eq!(lote_aplicado.anio, "2027");

        // Un favorito con fechas se aplica con los últimos 30 días: hoy menos 29 hasta hoy
        let lote_con_fechas = SeleccionLote {
            tribunales: vec!["113".into()],
            fechas: Some((hoy - chrono::Duration::days(10), hoy)),
            ..Default::default()
        };
        prefs.guardar_favorito("con_fechas", Pantalla::Seguimiento, &lote_con_fechas, false, hoy).unwrap();
        let mut cat_seg = Catalogo {
            pantalla: Pantalla::Seguimiento,
            ..Default::default()
        };
        cat_seg.tribunales.insert("113".into(), "Tribunal Ficticio".into());
        let (lote_fechas_aplicado, _) = prefs.aplicar_favorito("con_fechas", &cat_seg, hoy).unwrap();
        let (desde, hasta) = lote_fechas_aplicado.fechas.unwrap();
        assert_eq!(desde, hoy - chrono::Duration::days(29));
        assert_eq!(hasta, hoy);
    }

    #[test]
    fn a6_recuperacion_pendientes_y_planificar() {
        let temp_dir = std::env::temp_dir().join(format!("flow_a6_lote_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let s1 = serde_json::json!({"screen": "seguimiento", "tribunal": "113", "tab": "Seguimiento", "modality": "Total"});
        let s2 = serde_json::json!({"screen": "seguimiento", "tribunal": "114", "tab": "Seguimiento", "modality": "Total"});
        let s3 = serde_json::json!({"screen": "seguimiento", "tribunal": "115", "tab": "Seguimiento", "modality": "Total"});

        // 1. Tres consultas en plan y dos completadas: devuelve una
        let resumen_1 = serde_json::json!({
            "plan": [&s1, &s2, &s3],
            "consultas": [
                {"seleccion": &s1},
                {"seleccion": &s2}
            ],
            "filtro_fecha": {"desde": "01/10/2026", "hasta": "30/10/2026"}
        });
        std::fs::write(temp_dir.join("resumen.json"), resumen_1.to_string()).unwrap();
        let p1 = lote::pendientes(&temp_dir).unwrap();
        assert_eq!(p1.selecciones.len(), 1);
        assert_eq!(p1.selecciones[0].tribunal, "115");

        // 2. Todas completadas devuelve "No hay consultas pendientes."
        let resumen_todas = serde_json::json!({
            "plan": [&s1, &s2],
            "consultas": [
                {"seleccion": &s1},
                {"seleccion": &s2}
            ]
        });
        std::fs::write(temp_dir.join("resumen.json"), resumen_todas.to_string()).unwrap();
        assert_eq!(lote::pendientes(&temp_dir), Err("No hay consultas pendientes.".into()));

        // 3. Sin plan devuelve "El lote antiguo no tiene plan estructurado."
        let resumen_sin_plan = serde_json::json!({
            "consultas": []
        });
        std::fs::write(temp_dir.join("resumen.json"), resumen_sin_plan.to_string()).unwrap();
        assert_eq!(lote::pendientes(&temp_dir), Err("El lote antiguo no tiene plan estructurado.".into()));

        // 4. Con otros_filtros "formulario SITFA" devuelve el error de Python
        let resumen_privado = serde_json::json!({
            "plan": [&s1],
            "consultas": [],
            "otros_filtros": "formulario SITFA"
        });
        std::fs::write(temp_dir.join("resumen.json"), resumen_privado.to_string()).unwrap();
        assert_eq!(lote::pendientes(&temp_dir), Err("Prepara otra vez los filtros privados; no se guardan para recuperación.".into()));

        // 5. planificar_pendientes con otra pantalla devuelve error de Python
        let mut catalogo_litigantes = Catalogo {
            pantalla: Pantalla::Litigantes,
            ..Default::default()
        };
        catalogo_litigantes.tribunales.insert("115".into(), "Tribunal 115".into());
        let err_pantalla = catalogo_litigantes.planificar_pendientes(Edicion::Lite, &p1);
        assert_eq!(err_pantalla, Err("Selecciona la pantalla del lote y carga sus opciones primero.".into()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    /// E-Q6 · `preferencias.py`: valores por omisión de las lecturas de contexto y `meses_atras` fuera de 1–4 vuelve a 4.
    #[test]
    fn eq6_preferencias_de_lecturas() {
        let d = Preferencias::default();
        assert_eq!(
            (d.reutilizar_bitacoras, d.leer_calendarios, d.leer_historia, d.leer_historico, d.historia_en_paquete, d.meses_atras,
             d.paquete_consolidado),
            (true, false, false, false, true, 4, true)
        );
        let carpeta = std::env::temp_dir().join(format!("flow-eq6-prefs-{}", std::process::id()));
        std::fs::create_dir_all(&carpeta).unwrap();
        let ruta = carpeta.join("p.json");
        for (meses, esperado) in [(0, 4), (1, 1), (4, 4), (9, 4)] {
            std::fs::write(&ruta, format!("{{\"meses_atras\": {meses}, \"leer_historia\": true}}")).unwrap();
            let p = Preferencias::cargar(&ruta);
            assert_eq!((p.meses_atras, p.leer_historia, p.historia_en_paquete), (esperado, true, true), "meses {meses}");
        }
        let _ = std::fs::remove_dir_all(&carpeta);
    }
}
