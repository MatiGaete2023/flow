//! Trabajo en curso: el libro abierto, la observación propuesta de cada fila y las decisiones del usuario.

use crate::captura_trabajo::{self, PaqueteTrabajo};
use crate::config::{self, Config};
use crate::lectura::{self, Sheet};
use crate::motor::{self, Cols, Fila, Marca, Mode};
use crate::util::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct Registro {
    pub fila_excel: usize,
    pub values: Vec<Cell>,
    pub propuesta: String,
    pub eventos: Vec<String>,
    pub acciones: Vec<&'static str>,
    pub marcas: Vec<Marca>,
    pub avisos: Vec<String>,
    pub excluido: bool,
    pub fecha_hoja2: Option<NaiveDate>,
    // Decisiones del usuario
    pub editada: Option<String>,
    pub revisada: bool,
    pub seleccionada: bool,
    pub res: String,
    /// Programas que la propuesta afirma como otro programa vigente (botón «Marcar como consignada»).
    pub otros_programas: Vec<String>,
}

impl Registro {
    pub fn observacion(&self) -> &str {
        self.editada.as_deref().unwrap_or(&self.propuesta)
    }
    pub fn estado(&self) -> Estado {
        if self.excluido {
            Estado::Excluido
        } else if self.revisada {
            Estado::Revisada
        } else if self.propuesta.is_empty() && self.editada.is_none() {
            Estado::SinObservacion
        } else if !self.avisos.is_empty() || self.marcas.iter().any(|m| m.nivel >= 2) {
            Estado::Atencion
        } else {
            Estado::Propuesta
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    Propuesta,
    Atencion,
    Revisada,
    Excluido,
    SinObservacion,
}

impl Estado {
    pub fn label(self) -> &'static str {
        match self {
            Estado::Propuesta => "Propuesta",
            Estado::Atencion => "Revisar",
            Estado::Revisada => "Revisada",
            Estado::Excluido => "Excluido",
            Estado::SinObservacion => "Sin observación",
        }
    }
}

#[derive(Default)]
pub struct Trabajo {
    pub path: PathBuf,
    pub nombre: String,
    pub hash: String,
    pub bytes: Vec<u8>,
    pub sheets: Vec<Sheet>,
    pub sheet: usize,
    pub header_row: usize,
    pub mode: Option<Mode>,
    pub modos_posibles: Vec<Mode>,
    pub headers: Vec<String>,
    pub cols: Cols,
    pub registros: Vec<Registro>,
    pub corte: Option<NaiveDate>,
    pub avisos: Vec<String>,
    pub cruce: Option<String>,
    pub analisis_ms: u128,
    pub paquete: Option<PaqueteTrabajo>,
    /// P1: otros programas consignados por el funcionario, por identificador de registro del paquete. Se guardan en
    /// la sesión y sobreviven al recálculo; la propuesta deja de repetir esa constancia.
    pub consignaciones: BTreeMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Decision {
    editada: Option<String>,
    revisada: bool,
    seleccionada: bool,
    res: String,
    propuesta: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Sesion {
    archivo: String,
    hash: String,
    modo: String,
    corte: String,
    guardada: String,
    decisiones: BTreeMap<usize, Decision>,
    #[serde(default)]
    consignaciones: BTreeMap<String, Vec<String>>,
}

impl Trabajo {
    pub fn abrir_archivo(path: PathBuf, pedida: Option<Mode>, corte: NaiveDate, cfg: &Config) -> Result<Self, String> {
        let bytes = if captura_trabajo::es_zip(&path) {
            flow_rust::captura::leer_archivo(&path)?
        } else {
            std::fs::read(&path).map_err(|e| format!("No se pudo leer el archivo: {e}"))?
        };
        Self::abrir(path, bytes, pedida, corte, cfg)
    }

    pub fn abrir(path: PathBuf, bytes: Vec<u8>, pedida: Option<Mode>, corte: NaiveDate, cfg: &Config) -> Result<Trabajo, String> {
        let hash = hex(&Sha256::digest(&bytes));
        let (paquete, sheets, sheet, header_row, mode, modos_posibles) = if captura_trabajo::es_zip(&path) {
            let (paquete, hojas) = captura_trabajo::abrir(&bytes)?;
            let modos = paquete.modos.clone();
            let modo = pedida.filter(|m| modos.contains(m)).unwrap_or(modos[0]);
            let sheet = modos.iter().position(|&m| m == modo).unwrap();
            (Some(paquete), hojas, sheet, 0, modo, modos)
        } else {
            let sheets = lectura::read_sheets(&bytes)?;
            let tablas = lectura::detectar(&sheets);
            if tablas.is_empty() {
                return Err("No se identifica una tabla de Espera, Cumplimiento o Informes en el archivo. Revisa sus encabezados (deben estar DERIVACION, TRIBUNAL y T ESPERA, DIAS DE CUMPLIMIENTO/DIAS PARA EGRESAR o FECHA VENCIMIENTO).".into());
            }
            let (ti, mode) = lectura::elegir(&sheets, &tablas, pedida).ok_or("La etapa elegida no corresponde a las columnas del archivo.")?;
            let t = &tablas[ti];
            (None, sheets, t.sheet, t.header_row, mode, t.modes.clone())
        };
        let mut w = Trabajo {
            nombre: path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
            path,
            hash,
            bytes,
            sheet,
            header_row,
            mode: Some(mode),
            modos_posibles,
            sheets,
            paquete,
            corte: Some(corte),
            ..Default::default()
        };
        w.headers = lectura::headers_of(&w.sheets[w.sheet].rows[w.header_row]);
        w.analizar(cfg);
        w.restaurar(cfg);
        Ok(w)
    }

    pub fn cambiar_etapa_paquete(&mut self, modo: Mode, corte: NaiveDate, cfg: &Config) {
        let Some(sheet) = self.paquete.as_ref().and_then(|p| p.modos.iter().position(|&m| m == modo)) else { return; };
        if sheet == self.sheet {
            return;
        }
        self.sheet = sheet;
        self.mode = Some(modo);
        self.corte = Some(corte);
        self.headers = lectura::headers_of(&self.sheets[sheet].rows[self.header_row]);
        self.registros.clear();
        self.analizar(cfg);
        self.restaurar(cfg);
    }

    /// Vuelve a aplicar las reglas a todas las filas. Conserva las decisiones del usuario por fila.
    pub fn analizar(&mut self, cfg: &Config) {
        let t0 = std::time::Instant::now();
        let mode = self.mode.unwrap_or(Mode::Espera);
        let corte = self.corte.unwrap_or_else(|| chrono::Local::now().date_naive());
        self.cols = lectura::map_columns(&self.headers, mode);
        self.avisos.clear();
        if let Some(paquete) = &mut self.paquete {
            paquete.actualizar_contextos(corte);
            if paquete.fuente.estado("registros") != "COMPLETA" {
                self.avisos.push(format!("Cobertura de registros del paquete: {}.", paquete.fuente.estado("registros")));
            }
            if paquete.omitidos > 0 {
                self.avisos.push(format!("{} filas de Egresados u Otros están fuera de las etapas de Trabajo.", paquete.omitidos));
            }
        } else if self.modos_posibles.len() > 1 {
            self.avisos.push(format!(
                "El archivo sirve para varias etapas ({}); se analiza como {}. Puedes cambiarla arriba.",
                self.modos_posibles.iter().map(|m| m.label()).collect::<Vec<_>>().join(", "),
                mode.label()
            ));
        }
        // Cruce (hoja 2) para C-10: identidad completa, fecha futura, la última fila gana.
        let mut cruce: HashMap<Vec<String>, NaiveDate> = HashMap::new();
        self.cruce = None;
        if mode == Mode::Cumplimiento && self.paquete.is_none() {
            if let Some((si, hr, cc)) = lectura::buscar_cruce(&self.sheets, self.sheet) {
                for row in self.sheets[si].rows.iter().skip(hr + 1) {
                    let f = Fila { values: row, cols: &cc };
                    if let Some(due) = f.date("vencimiento") {
                        if due > corte {
                            let k: Vec<String> = ["rit", "rut", "nombre", "tribunal", "programa"].iter().map(|k| historical_match(&f.text(k))).collect();
                            cruce.insert(k, due);
                        }
                    }
                }
                self.cruce = Some(self.sheets[si].name.clone());
            } else {
                self.avisos.push("Sin hoja de cruce utilizable: no se evalúa el próximo informe (C-10). El resto del análisis continúa.".into());
            }
        }
        let viejos: HashMap<usize, Registro> = std::mem::take(&mut self.registros).into_iter().map(|r| (r.fila_excel, r)).collect();
        let sheet = &self.sheets[self.sheet];
        let mapped: Vec<usize> = self.cols.values().copied().collect();
        let columnas_por_fila = self.paquete.as_ref().map(|p| &p.columnas_por_fila[self.sheet]);
        let mut filas_idx = Vec::new();
        for (i, row) in sheet.rows.iter().enumerate().skip(self.header_row + 1) {
            if columnas_por_fila.is_none() && row.iter().all(|c| c.is_empty()) {
                continue;
            }
            // Totales, pies y notas no son registros: solo filas con algún dato de columna reconocida.
            if columnas_por_fila.is_none() && !mapped.iter().any(|c| row.get(*c).is_some_and(|v| !v.is_empty())) {
                continue;
            }
            filas_idx.push(i);
        }
        let filas: Vec<Fila> = filas_idx.iter().map(|i| Fila {
            values: &sheet.rows[*i],
            cols: columnas_por_fila.map_or(&self.cols, |cols| &cols[*i - self.header_row - 1]),
        }).collect();
        let columna_id = self.paquete.as_ref().and_then(|_| {
            self.headers.iter().position(|h| h == "ID_REGISTRO_CAPTURA")
        });
        let ingresos = self.paquete.as_ref().filter(|_| mode == Mode::Informes)
            .map(PaqueteTrabajo::identidades_ingresos).unwrap_or_default();
        let mut pre = motor::prepaso(mode, &filas, corte, |f| {
            let id = f.values.get(columna_id?)?.display();
            ingresos.get(id.as_str()).copied()
        });
        if let Some(col) = columna_id {
            for (n, f) in filas.iter().enumerate() {
                let Some(id) = f.values.get(col).map(Cell::display) else { continue };
                if let Some(programas) = self.consignaciones.get(&id) {
                    pre.consignados.insert(n, programas.iter().map(|p| captura_trabajo::clave_texto(p)).collect());
                }
            }
        }
        let mut registros = Vec::with_capacity(filas.len());
        for (n, (f, i)) in filas.iter().zip(&filas_idx).enumerate() {
            let contexto = columna_id
                .and_then(|col| f.values.get(col))
                .and_then(|id| self.paquete.as_ref()?.contextos.get(&id.display()));
            let fecha_calendario_91 = contexto.and_then(|ctx| ctx.vencimiento_calendario_91);
            let calendario = contexto.and_then(|ctx| ctx.calendario.as_ref());
            let fecha_hoja2 = if mode == Mode::Cumplimiento {
                if self.paquete.is_some() {
                    fecha_calendario_91.map(|fecha| {
                        self.cruce = Some("Hoja2 (calendario 91)".into());
                        fecha
                    }).or_else(|| {
                        let fecha = calendario?.proxima_pendiente(corte)?;
                        self.cruce.get_or_insert_with(|| "Calendario del ingreso".into());
                        Some(fecha)
                    })
                } else {
                    let k: Vec<String> = ["rit", "rut", "nombre", "tribunal", "programa"].iter().map(|k| historical_match(&f.text(k))).collect();
                    cruce.get(&k).copied()
                }
            } else {
                None
            };
            let mut r = motor::evaluar_con_captura(cfg, mode, f, corte, (&pre, n), fecha_hoja2, contexto);
            if let Some(calendario) = calendario.filter(|_| matches!(mode, Mode::Cumplimiento | Mode::Informes)) {
                let vencimiento_informes = (mode == Mode::Informes).then(|| f.date("vencimiento")).flatten();
                r.avisos.extend(calendario.avisos_cotejo(corte, vencimiento_informes, fecha_calendario_91));
            }
            if let Some(cronologia) = contexto.and_then(|ctx| ctx.cronologia.as_ref()) {
                cronologia.aplicar_reiteracion(&mut r, cfg);
            }
            let excluido = r.eventos.iter().any(|e| e == "COMUN.NO_SEGUIMIENTO");
            let acciones = if excluido { vec![] } else { motor::acciones(&r.eventos, fecha_hoja2, corte) };
            let mut reg = Registro {
                fila_excel: i + 1,
                values: f.values.to_vec(),
                propuesta: r.observacion,
                eventos: r.eventos,
                acciones,
                marcas: r.marcas,
                avisos: r.avisos,
                excluido,
                fecha_hoja2,
                otros_programas: r.otros_programas,
                ..Default::default()
            };
            if let Some(res) = self.cols.get("res").and_then(|c| f.values.get(*c)) {
                reg.res = res.display().trim().to_uppercase();
            }
            if let Some(v) = viejos.get(&reg.fila_excel) {
                reg.editada = v.editada.clone();
                reg.revisada = v.revisada;
                reg.seleccionada = v.seleccionada;
                if !v.res.is_empty() {
                    reg.res = v.res.clone();
                }
            }
            registros.push(reg);
        }
        self.registros = registros;
        self.analisis_ms = t0.elapsed().as_millis();
    }

    /// Descarta las ediciones y marcas (por elección del usuario) y vuelve a analizar.
    pub fn reiniciar(&mut self, cfg: &Config) {
        self.consignaciones.clear();
        for r in &mut self.registros {
            r.editada = None;
            r.revisada = false;
            r.seleccionada = false;
        }
        self.analizar(cfg);
    }

    pub fn mode(&self) -> Mode {
        self.mode.unwrap_or(Mode::Espera)
    }

    pub fn value(&self, r: &Registro, key: &str) -> String {
        let v = self.cols.get(key).and_then(|c| r.values.get(*c)).map(|c| c.display().trim().to_string()).unwrap_or_default();
        if key == "nombre" { sin_parentesis(&v) } else { v }
    }

    pub fn date(&self, r: &Registro, key: &str) -> Option<NaiveDate> {
        self.cols.get(key).and_then(|c| r.values.get(*c)).and_then(get_date)
    }

    pub fn tribunal_clave(&self, r: &Registro) -> String {
        let t = self.value(r, "tribunal");
        tribunal(&t).map(|s| s.to_string()).unwrap_or_else(|| normalizar(&t).to_uppercase())
    }

    fn sesion_path(&self) -> PathBuf {
        let corte = self.corte.map(|c| c.to_string()).unwrap_or_default();
        let key = format!("{}-{}-{}", &self.hash[..16.min(self.hash.len())], self.mode().key(), corte);
        config::data_dir().join("sesiones").join(format!("{key}.json"))
    }

    /// Sustituye las marcas por registros elegibles de la etapa actual.
    pub fn reemplazar_seleccion(&mut self, filas: &[usize]) -> Vec<usize> {
        let ids: std::collections::BTreeSet<_> = filas.iter().copied()
            .filter(|&i| self.registros.get(i).is_some_and(|r| !r.excluido))
            .collect();
        if !ids.is_empty() {
            for (i, r) in self.registros.iter_mut().enumerate() {
                r.seleccionada = ids.contains(&i);
            }
        }
        ids.into_iter().collect()
    }

    /// Guarda las decisiones del usuario (ediciones, revisadas, selección, RES).
    /// Identificador del registro en el paquete (`ID_REGISTRO_CAPTURA`); sin paquete no hay consignación.
    pub fn id_registro(&self, r: &Registro) -> Option<String> {
        self.paquete.as_ref()?;
        let col = self.headers.iter().position(|h| h == "ID_REGISTRO_CAPTURA")?;
        Some(r.values.get(col)?.display()).filter(|id| !id.trim().is_empty())
    }

    /// Programas consignados de la fila, tal como se mostraron al marcarlos.
    pub fn consignados(&self, r: &Registro) -> Vec<String> {
        self.id_registro(r).and_then(|id| self.consignaciones.get(&id).cloned()).unwrap_or_default()
    }

    /// P1: marca (o deshace) la constancia de otro programa como consignada y recalcula. Idempotente.
    pub fn consignar(&mut self, fila: usize, programa: &str, consignar: bool, cfg: &Config) -> Option<String> {
        let id = self.id_registro(self.registros.get(fila)?)?;
        let clave = captura_trabajo::clave_texto(programa);
        let lista = self.consignaciones.entry(id.clone()).or_default();
        let ya = lista.iter().any(|p| captura_trabajo::clave_texto(p) == clave);
        let mensaje = match (consignar, ya) {
            (true, false) => {
                lista.push(programa.to_owned());
                format!("«{programa}» quedó como consignada: la propuesta ya no repite esa constancia.")
            }
            (false, true) => {
                lista.retain(|p| captura_trabajo::clave_texto(p) != clave);
                format!("Se deshizo la consignación de «{programa}»: la constancia vuelve a proponerse si corresponde.")
            }
            _ => return None,
        };
        if lista.is_empty() {
            self.consignaciones.remove(&id);
        }
        self.analizar(cfg);
        Some(mensaje)
    }

    pub fn guardar_sesion(&self) -> Result<(), String> {
        self.escribir_sesion(&self.sesion_path())?;
        limpiar_sesiones();
        Ok(())
    }

    fn escribir_sesion(&self, ruta: &std::path::Path) -> Result<(), String> {
        let mut s = Sesion {
            archivo: self.nombre.clone(),
            hash: self.hash.clone(),
            modo: self.mode().key().into(),
            corte: self.corte.map(|c| c.to_string()).unwrap_or_default(),
            guardada: chrono::Local::now().to_rfc3339(),
            consignaciones: self.consignaciones.clone(),
            ..Default::default()
        };
        for r in &self.registros {
            if r.editada.is_some() || r.revisada || r.seleccionada || !r.res.is_empty() {
                s.decisiones.insert(
                    r.fila_excel,
                    Decision { editada: r.editada.clone(), revisada: r.revisada, seleccionada: r.seleccionada, res: r.res.clone(), propuesta: r.propuesta.clone() },
                );
            }
        }
        let data = serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?;
        config::atomic_write(ruta, &data).map_err(|e| e.to_string())
    }

    fn restaurar(&mut self, cfg: &Config) {
        self.restaurar_desde(&self.sesion_path());
        if !self.consignaciones.is_empty() {
            self.analizar(cfg);
        }
    }

    fn restaurar_desde(&mut self, ruta: &std::path::Path) {
        let Ok(t) = std::fs::read_to_string(ruta) else { return };
        let Ok(s) = serde_json::from_str::<Sesion>(&t) else {
            self.avisos.push("La sesión anterior de este archivo no se pudo leer; se conserva sin cambios.".into());
            return;
        };
        self.consignaciones = s.consignaciones;
        let mut n = 0;
        for r in &mut self.registros {
            if let Some(d) = s.decisiones.get(&r.fila_excel) {
                r.editada = d.editada.clone();
                r.revisada = d.revisada;
                r.seleccionada = d.seleccionada;
                if !d.res.is_empty() {
                    r.res = d.res.clone();
                }
                n += 1;
            }
        }
        if n > 0 {
            self.avisos.push(format!("Se recuperaron las decisiones guardadas de {n} filas (mismo archivo, etapa y corte)."));
        }
    }

    pub fn conteos(&self) -> BTreeMap<&'static str, usize> {
        let mut m = BTreeMap::new();
        for r in &self.registros {
            *m.entry(r.estado().label()).or_insert(0) += 1;
        }
        m
    }
}

/// Retención local: sesiones de más de 60 días se eliminan; se conservan hasta 10 anteriores.
fn limpiar_sesiones() {
    let dir = config::data_dir().join("sesiones");
    let Ok(rd) = std::fs::read_dir(&dir) else { return };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    let limite = std::time::Duration::from_secs(config::RETENCION_DIAS as u64 * 86400);
    for (i, (t, p)) in files.iter().enumerate() {
        let viejo = t.elapsed().map(|e| e > limite).unwrap_or(false);
        if i > config::SESIONES_ANTERIORES || viejo {
            let _ = std::fs::remove_file(p);
        }
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conserva_el_conjunto_transferido_al_restaurar_una_sesion() {
        let mut w = Trabajo {
            registros: vec![
                Registro { fila_excel: 2, seleccionada: true, editada: Some("Edición ficticia".into()), ..Default::default() },
                Registro { fila_excel: 3, ..Default::default() },
                Registro { fila_excel: 4, excluido: true, ..Default::default() },
            ],
            ..Default::default()
        };
        assert!(w.reemplazar_seleccion(&[2, 30]).is_empty());
        assert!(w.registros[0].seleccionada);
        assert_eq!(w.reemplazar_seleccion(&[1, 2, 1, 30]), [1]);
        let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test_out");
        let ruta = carpeta.join(format!("seleccion_{}.json", chrono::Utc::now().timestamp_micros()));
        w.escribir_sesion(&ruta).unwrap();
        w.registros[0].editada = None;
        w.registros[1].seleccionada = false;
        w.restaurar_desde(&ruta);
        assert_eq!(w.registros.iter().map(|r| r.seleccionada).collect::<Vec<_>>(), [false, true, false]);
        assert_eq!(w.registros[0].editada.as_deref(), Some("Edición ficticia"));
    }
    use crate::productos;

    fn hoy() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 19).unwrap()
    }

    #[test]
    #[ignore = "requiere FLOW_E3_PAQUETE"]
    fn cumplimiento_usa_contexto_e3_del_exportador_python_para_el_proximo_informe() {
        let ruta = crate::prueba_entorno::requerida("FLOW_E3_PAQUETE");
        let cfg = Config::default();
        let mut trabajo = Trabajo::abrir_archivo(
            ruta.into(),
            Some(Mode::Cumplimiento),
            "2026-10-19".parse().unwrap(),
            &cfg,
        ).unwrap();
        assert_eq!(trabajo.registros.len(), 2);
        assert_eq!(trabajo.registros[0].fecha_hoja2, "2026-11-01".parse().ok());
        assert!(trabajo.registros[0].eventos.iter().any(|e| e == "CUMPLIMIENTO.C10_HOJA2"));
        assert_eq!(trabajo.registros[0].propuesta, "Persona PRM: Próximo informe de avance de PRM Centro vence el 1 de noviembre de 2026. Se hace presente que Persona cuenta con ingreso efectivo al programa PRM Ficticio. Asimismo, se hace presente que Persona cuenta con ingreso efectivo al programa FAE Otro Ficticio.");
        assert!(trabajo.registros[0].eventos.iter().any(|e| e == "HISTORIA.OTRO_PROGRAMA"));
        assert!(trabajo.registros[1].fecha_hoja2.is_none());
        assert_eq!(trabajo.cruce.as_deref(), Some("Calendario del ingreso"));
        for corte in ["2026-10-18", "2026-11-01"] {
            trabajo.corte = corte.parse().ok();
            trabajo.analizar(&cfg);
            assert!(trabajo.registros.iter().all(|r| r.fecha_hoja2.is_none()));
            assert!(trabajo.cruce.is_none());
        }
    }

    #[test]
    #[ignore = "requiere FLOW_E3_PAQUETE"]
    fn a2_consignacion_p1_y_sesion() {
        let ruta = crate::prueba_entorno::requerida("FLOW_E3_PAQUETE");
        let cfg = Config::default();
        let mut trabajo = Trabajo::abrir_archivo(
            ruta.clone().into(),
            Some(Mode::Cumplimiento),
            "2026-10-19".parse().unwrap(),
            &cfg,
        ).unwrap();

        assert!(trabajo.registros[0].eventos.iter().any(|e| e == "HISTORIA.OTRO_PROGRAMA"));
        assert_eq!(trabajo.registros[0].otros_programas.len(), 2);

        // 1. Consignar retira la frase y luego deja otros_programas vacio
        let msg = trabajo.consignar(0, "PRM Ficticio", true, &cfg);
        assert!(msg.is_some());
        assert!(!trabajo.registros[0].propuesta.contains("PRM Ficticio"));
        assert!(trabajo.registros[0].propuesta.contains("FAE Otro Ficticio"));
        assert_eq!(trabajo.registros[0].otros_programas, ["FAE Otro Ficticio"]);

        let msg2 = trabajo.consignar(0, "FAE Otro Ficticio", true, &cfg);
        assert!(msg2.is_some());
        assert!(!trabajo.registros[0].eventos.iter().any(|e| e == "HISTORIA.OTRO_PROGRAMA"));
        assert!(trabajo.registros[0].otros_programas.is_empty());

        // Consignar dos veces no cambia nada (idempotencia)
        let msg_repetido = trabajo.consignar(0, "PRM Ficticio", true, &cfg);
        assert!(msg_repetido.is_none());

        // Deshacer restituye la frase
        let msg_deshacer = trabajo.consignar(0, "PRM Ficticio", false, &cfg);
        assert!(msg_deshacer.is_some());
        assert!(trabajo.registros[0].propuesta.contains("PRM Ficticio"));
        assert_eq!(trabajo.registros[0].otros_programas, ["PRM Ficticio"]);

        // La consignación sobrevive a analizar
        trabajo.analizar(&cfg);
        assert!(trabajo.registros[0].propuesta.contains("PRM Ficticio"));
        assert!(!trabajo.registros[0].propuesta.contains("FAE Otro Ficticio"));

        // La consignación sobrevive al cambio de corte
        trabajo.corte = Some("2026-10-20".parse().unwrap());
        trabajo.analizar(&cfg);
        assert!(!trabajo.registros[0].propuesta.contains("FAE Otro Ficticio"));

        // La consignación sobrevive a guardar y restaurar sesión
        let temp_sesion = std::env::temp_dir().join("test_sesion_consignacion.json");
        trabajo.escribir_sesion(&temp_sesion).unwrap();

        let mut trabajo_restaurado = Trabajo::abrir_archivo(
            ruta.into(),
            Some(Mode::Cumplimiento),
            "2026-10-19".parse().unwrap(),
            &cfg,
        ).unwrap();
        trabajo_restaurado.restaurar_desde(&temp_sesion);
        trabajo_restaurado.analizar(&cfg);
        assert_eq!(trabajo_restaurado.consignaciones, trabajo.consignaciones);
        assert!(!trabajo_restaurado.registros[0].propuesta.contains("FAE Otro Ficticio"));
        let _ = std::fs::remove_file(&temp_sesion);

        // reiniciar la borra
        trabajo.reiniciar(&cfg);
        assert!(trabajo.consignaciones.is_empty());
        assert!(trabajo.registros[0].propuesta.contains("FAE Otro Ficticio"));

        // Una sesión antigua sin el campo consignaciones se lee sin error
        let sesion_antigua = serde_json::json!({
            "archivo": "contexto-e3.zip",
            "hash": "abc",
            "modo": "cumplimiento",
            "corte": "2026-10-19",
            "guardada": "2026-10-19T10:00:00",
            "decisiones": {}
        }).to_string();
        let sesion_parsed = serde_json::from_str::<Sesion>(&sesion_antigua);
        assert!(sesion_parsed.is_ok());
        assert!(sesion_parsed.unwrap().consignaciones.is_empty());
    }

    #[test]
    #[ignore = "requiere FLOW_REFERENCIA"]
    fn informes_atribuyen_la_historia_y_los_escritos_segun_flow_python() {
        let raiz = crate::prueba_entorno::requerida("FLOW_REFERENCIA");
        let ruta = PathBuf::from(raiz).join("rust/crates/flow-trabajo/tests/datos/paquete_estructura.zip");
        let esperado: serde_json::Value = serde_json::from_slice(&std::fs::read(ruta.with_extension("json")).unwrap()).unwrap();
        let corte = esperado["corte"].as_str().unwrap().parse().unwrap();
        let mut cfg = Config { redaccion_registro: false, ..Config::default() };
        let mut trabajo = Trabajo::abrir_archivo(ruta, Some(Mode::Informes), corte, &cfg).unwrap();
        let filas = esperado["filas"].as_array().unwrap();
        assert_eq!(trabajo.registros.len(), filas.len());
        for (r, fila) in trabajo.registros.iter().zip(filas) {
            let id = &fila["id"];
            assert_eq!(r.propuesta, fila["observacion"].as_str().unwrap(), "{id}");
            assert_eq!(serde_json::json!(&r.eventos), fila["reglas"], "{id}");
            assert_eq!(serde_json::json!(&r.acciones), fila["acciones"], "{id}");
            assert_eq!(serde_json::json!(&r.marcas), fila["marcas"], "{id}");
        }
        let proyectos = productos::preparar_proyectos(&trabajo, &cfg, false);
        let nomencl = proyectos.iter().find(|p| p.tipo == "NOMENCL").unwrap();
        assert_eq!(nomencl.vars["NOMENCLATURA"], "Ingreso Informe de Cumplimiento X");
        assert_eq!(nomencl.registros, [2]);
        assert!(productos::texto_proyecto(nomencl).iter()
            .any(|p| p.contains("Ingreso Informe de Cumplimiento X")));
        trabajo.registros[2].editada = Some("Aplicar nomenclatura «Ingreso informe de diagnóstico».".into());
        let proyectos = productos::preparar_proyectos(&trabajo, &cfg, false);
        let nomencl = proyectos.iter().find(|p| p.tipo == "NOMENCL").unwrap();
        assert_eq!(nomencl.vars["NOMENCLATURA"], "Ingreso informe de diagnóstico");
        cfg.desactivadas.insert("HISTORIA.INFORME_AVANCE_PENDIENTE".into());
        trabajo.analizar(&cfg);
        assert!(trabajo.registros[1].acciones.is_empty());
        assert!(!trabajo.registros[1].propuesta.contains("Pendiente de resolver"));
    }

    #[test]
    #[ignore = "requiere FLOW_E3_PAQUETE"]
    fn espera_coteja_folio_y_correo_del_contexto_e3_del_exportador_python() {
        let ruta = crate::prueba_entorno::requerida("FLOW_E3_PAQUETE");
        let trabajo = Trabajo::abrir_archivo(
            PathBuf::from(ruta).with_file_name("contexto-e3-espera.zip"),
            Some(Mode::Espera),
            "2026-10-19".parse().unwrap(),
            &Config::default(),
        ).unwrap();
        let registro = &trabajo.registros[0];
        assert_eq!(registro.propuesta, concat!(
            "Persona DCE: Medida revisada, a la espera de ingreso efectivo. ",
            "Ingreso al programa DCE Ficticio ordenado en resolución de fecha 13 de octubre de 2026 (folio 9120). ",
            "Se remite correo al programa consultando fecha estimada de ingreso."
        ));
        assert_eq!(registro.eventos, ["HISTORIA.E04_CON_FOLIO", "ESPERA.E05_SOLO_CORREO"]);
        assert_eq!(registro.acciones, ["programa_espera"]);
        assert!(!trabajo.registros[1].eventos.iter().any(|e| e.starts_with("HISTORIA.")));
    }

    #[test]
    #[ignore = "requiere FLOW_REFERENCIA"]
    fn calendario_91_conserva_precedencia_y_aviso_al_abrir_trabajo() {
        let raiz = crate::prueba_entorno::requerida("FLOW_REFERENCIA");
        let ruta = PathBuf::from(raiz).join("rust/crates/flow-trabajo/tests/datos/paquete_flujo.zip");
        let trabajo = Trabajo::abrir_archivo(
            ruta,
            Some(Mode::Cumplimiento),
            "2026-10-20".parse().unwrap(),
            &Config::default(),
        ).unwrap();
        assert_eq!(trabajo.registros[0].fecha_hoja2, "2026-11-15".parse().ok());
        assert_eq!(trabajo.cruce.as_deref(), Some("Hoja2 (calendario 91)"));
        assert_eq!(trabajo.registros[0].avisos.iter().filter(|a| a.starts_with("CAL-03")).map(String::as_str).collect::<Vec<_>>(), [
            "CAL-03: Hoja2 (calendario 91) indica el 15 de noviembre de 2026 y el calendario del ingreso no tiene un próximo informe pendiente; revisar en RUS."
        ]);
        assert!(trabajo.registros[1..].iter().all(|r| r.fecha_hoja2.is_none() && r.avisos.iter().all(|a| !a.starts_with("CAL-03"))));
    }

    #[test]
    #[ignore = "requiere FLOW_E3_PAQUETE"]
    fn informes_coteja_contexto_e3_del_exportador_python_con_una_sola_fecha() {
        let ruta = crate::prueba_entorno::requerida("FLOW_E3_PAQUETE");
        let trabajo = Trabajo::abrir_archivo(
            PathBuf::from(ruta).with_file_name("contexto-e3-informes.zip"),
            Some(Mode::Informes),
            "2026-10-19".parse().unwrap(),
            &Config::default(),
        ).unwrap();
        let avisos: Vec<_> = trabajo.registros.iter().flat_map(|r| &r.avisos)
            .filter(|a| a.starts_with("CAL-03")).map(String::as_str).collect();
        assert_eq!(avisos, [
            "CAL-03: la pestaña Informes indica vencimiento el 10 de octubre de 2026 y el calendario del ingreso proyecta el informe pendiente el 1 de noviembre de 2026; revisar en RUS."
        ]);
    }

    #[test]
    #[ignore = "requiere FLOW_E3_PAQUETE"]
    fn trabajo_reitera_contexto_e3_del_exportador_python_solo_para_el_ingreso() {
        let ruta = crate::prueba_entorno::requerida("FLOW_E3_PAQUETE");
        let mut cfg = Config::default();
        let mut w = Trabajo::abrir_archivo(
            PathBuf::from(ruta).with_file_name("contexto-e3-reiteracion.zip"),
            Some(Mode::Cumplimiento),
            "2026-10-19".parse().unwrap(),
            &cfg,
        ).unwrap();
        assert_eq!(w.registros[0].propuesta, "Persona PRM: Se reitera observación anterior del 12 de agosto de 2026.");
        assert!(w.registros[0].eventos.iter().any(|r| r == "COMUN.REITERA_OBSERVACION"));
        assert!(w.registros[1].eventos.iter().all(|r| r != "COMUN.REITERA_OBSERVACION"));
        let acciones = w.registros[0].acciones.clone();
        cfg.desactivadas.insert("COMUN.REITERA_OBSERVACION".into());
        w.analizar(&cfg);
        assert!(w.registros[0].propuesta.ends_with("vence el 1 de noviembre de 2026."));
        assert_eq!(w.registros[0].acciones, acciones);
    }

    #[test]
    fn flujo_completo_con_ejemplo() {
        let cfg = Config::default();
        // Salidas de prueba dentro de target/, en una carpeta nueva por ejecución.
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("test_out").join(format!("{}", chrono::Local::now().timestamp_millis()));
        std::fs::create_dir_all(&dir).unwrap();
        for m in Mode::ALL {
            let p = crate::demo::libro(m, hoy()).unwrap();
            let bytes = std::fs::read(&p).unwrap();
            let w = Trabajo::abrir(p, bytes, Some(m), hoy(), &cfg).unwrap();
            assert_eq!(w.mode(), m);
            assert_eq!(w.registros.len(), 18, "{m:?}");
            assert!(w.registros.iter().filter(|r| !r.propuesta.is_empty()).count() >= 10, "{m:?}");
            let x = productos::excel_observaciones(&w, &dir).unwrap();
            let back = crate::lectura::read_sheets(&std::fs::read(&x).unwrap()).unwrap();
            assert!(back[0].rows[0].iter().any(|c| c.display() == "OBSERVACION"));
            let o = productos::OpcionesCorreo { modalidades: &["RES", "AMB", "FAE", "DCE"], tribunales: &[], solo_seleccionadas: false };
            let bs = productos::preparar_borradores(&w, &cfg, &o);
            assert!(!bs.is_empty(), "{m:?}");
            for b in &bs {
                let e = productos::guardar_eml(b, &dir).unwrap();
                let t = std::fs::read_to_string(&e).unwrap();
                assert!(t.contains("Cc: ucc_concepcion@pjud.cl"));
                assert!(t.starts_with("X-Unsent: 1"));
            }
            let ps = productos::preparar_proyectos(&w, &cfg, false);
            if m != Mode::Cumplimiento {
                assert!(!ps.is_empty(), "{m:?}");
                let d = productos::generar_word(&ps, &dir, hoy()).unwrap();
                let docs: Vec<_> = std::fs::read_dir(&d).unwrap().filter_map(Result::ok).map(|e| e.path()).collect();
                assert_eq!(docs.len(), ps.iter().filter(|p| p.incluir).count(), "{m:?}");
                assert!(docs.iter().all(|p| p.extension().is_some_and(|x| x == "docx") && std::fs::metadata(p).unwrap().len() > 500));
            } else {
                assert!(w.cruce.is_some());
                assert!(w.registros.iter().any(|r| r.eventos.iter().any(|e| e == "CUMPLIMIENTO.C10_HOJA2")));
            }
        }
        let p = crate::demo::libro(Mode::Espera, hoy()).unwrap();
        let inf = crate::anon::ejecutar(&[p], &dir.join("anon"), &crate::anon::Opciones::default());
        assert!(inf.errores.is_empty(), "{:?}", inf.errores);
        let s = crate::lectura::read_sheets(&std::fs::read(&inf.creados[0]).unwrap()).unwrap();
        let todo: String = s[0].rows.iter().flatten().map(|c| c.display()).collect::<Vec<_>>().join("|");
        assert!(!todo.contains("Ejemplo Uno"), "{todo}");
        assert!(todo.contains("Persona Ficticia"));
        assert!(todo.contains("PRM Ficticio Laja"), "las columnas del motor se conservan");
    }

    /// Texto de una columna lógica del registro analizado (para clasificar R6 por el nombre del programa).
    fn texto_de(trabajo: &Trabajo, registro: &Registro, clave: &str) -> String {
        trabajo
            .cols
            .get(clave)
            .and_then(|indice| registro.values.get(*indice))
            .map(Cell::display)
            .unwrap_or_default()
    }

    /// Compara la observación de Python con la de Rust admitiendo solo R1 (C10 omitida) y R6 (nombre de programa).
    /// Es la misma clasificación del paquete B: `titulo_programa`/`titulo_programa_registro` y la oración C10.
    fn clasificar_observacion(
        esperada: &str,
        obtenida: &str,
        programa: &str,
        tiene_r1: bool,
    ) -> Option<&'static str> {
        if esperada == obtenida {
            return Some("identica");
        }
        let mut base = esperada.to_string();
        if tiene_r1 {
            let re_c10 = regex::Regex::new(r"\s*Próximo informe de avance de [^.]* vence el [^.]*\.").unwrap();
            if !re_c10.is_match(&base) || re_c10.is_match(obtenida) {
                return None;
            }
            base = re_c10.replace(&base, "").to_string();
        }
        let py = titulo_programa(programa);
        let rs = titulo_programa_registro(programa);
        let ajustada = if py.is_empty() || py == rs {
            base.clone()
        } else {
            base.replace(&py, &rs)
        };
        let es_r6 = ajustada != base;
        if ajustada == obtenida {
            if tiene_r1 {
                Some("R1")
            } else if es_r6 {
                Some("R6")
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Compara fila a fila observación, reglas y marcas de Trabajo contra la salida de Python del paquete C.
    /// `cuenta` = [idénticas, R1, R6]; toda otra diferencia va a `fallos`.
    fn comparar_paquete_c(
        trabajo: &Trabajo,
        esperadas: &[&serde_json::Value],
        etapa: &str,
        corte: NaiveDate,
        cuenta: &mut [usize; 3],
        fallos: &mut Vec<String>,
    ) {
        for (indice, (registro, fila)) in trabajo.registros.iter().zip(esperadas).enumerate() {
            let esperada = fila["observacion"].as_str().unwrap();
            let reglas_esperadas: Vec<&str> = fila["reglas"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|r| r.as_str())
                .collect();
            let marcas_esperadas = fila["marcas"].clone();
            let reglas_ok = registro
                .eventos
                .iter()
                .map(String::as_str)
                .eq(reglas_esperadas.iter().copied());
            let marcas_ok = serde_json::to_value(&registro.marcas).unwrap() == marcas_esperadas;
            if !reglas_ok || !marcas_ok {
                fallos.push(format!(
                    "{} {etapa} {corte} fila {indice}: reglas/marcas distintas\n  esp reglas {:?} / obt {:?}\n  esp marcas {} / obt {}",
                    fila["record_id"].as_str().unwrap_or(""),
                    reglas_esperadas,
                    registro.eventos,
                    marcas_esperadas,
                    serde_json::to_value(&registro.marcas).unwrap()
                ));
                continue;
            }
            let programa = texto_de(trabajo, registro, "programa");
            let tiene_r1 = registro.eventos.iter().any(|e| e == "CUMPLIMIENTO.C05_POR_VENCER")
                && registro.eventos.iter().any(|e| e == "CUMPLIMIENTO.C10_HOJA2");
            match clasificar_observacion(esperada, &registro.propuesta, &programa, tiene_r1) {
                Some("identica") => cuenta[0] += 1,
                Some("R1") => cuenta[1] += 1,
                Some("R6") => cuenta[2] += 1,
                _ => fallos.push(format!(
                    "{} {etapa} {corte} fila {indice}: observación distinta\n  esp: {esperada}\n  obt: {}",
                    fila["record_id"].as_str().unwrap_or(""),
                    registro.propuesta
                )),
            }
        }
    }

    /// Paquete C: abre `contexto-e3-historia.zip` en Trabajo en las tres etapas y dos cortes, y compara contra la
    /// salida de `app/qa/esperado_contexto_e3.py`. Sin variables falla; no termina con `return`.
    #[test]
    #[ignore = "requiere FLOW_E3_HISTORIA y FLOW_E3_ESPERADO"]
    fn paquete_c_contexto_e3_historia_igual_a_python() {
        let (Ok(zip), Ok(esperado_ruta)) = (
            std::env::var("FLOW_E3_HISTORIA"),
            std::env::var("FLOW_E3_ESPERADO"),
        ) else {
            panic!("FLOW_E3_HISTORIA y FLOW_E3_ESPERADO son obligatorias para el paquete C");
        };
        let esperado: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&esperado_ruta).unwrap()).unwrap();
        let filas = esperado["filas"].as_array().unwrap();
        let corte_fijo: NaiveDate = esperado["corte_fijo"].as_str().unwrap().parse().unwrap();
        let corte_movido: NaiveDate = esperado["corte_movido"].as_str().unwrap().parse().unwrap();
        let cfg = Config::default();
        let mut etapas = 0usize;
        let (mut filas_total, mut identicas, mut r1, mut r6) = (0usize, 0usize, 0usize, 0usize);
        let (mut filas_corte, mut identicas_corte, mut constancias) = (0usize, 0usize, 0usize);
        let mut fallos = Vec::new();

        for etapa in ["ESPERA", "CUMPLIMIENTO", "INFORMES"] {
            let modo = match etapa {
                "ESPERA" => Mode::Espera,
                "CUMPLIMIENTO" => Mode::Cumplimiento,
                _ => Mode::Informes,
            };
            let esperadas: Vec<&serde_json::Value> = filas
                .iter()
                .filter(|f| f["etapa"] == etapa && f["corte"] == corte_fijo.to_string())
                .collect();
            if esperadas.is_empty() {
                if let Some(no_aplica) = esperado["no_aplica"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|n| n["etapa"] == etapa)
                {
                    println!(
                        "PAQUETE_C_NO_APLICA: etapa {etapa}, {}",
                        no_aplica["motivo"].as_str().unwrap_or("")
                    );
                }
                continue;
            }
            etapas += 1;
            filas_total += esperadas.len();
            let trabajo = Trabajo::abrir_archivo(
                PathBuf::from(&zip),
                Some(modo),
                corte_fijo,
                &cfg,
            )
            .unwrap_or_else(|e| panic!("no se pudo abrir el paquete C en {etapa}: {e}"));
            assert_eq!(
                trabajo.registros.len(),
                esperadas.len(),
                "{etapa}: filas distintas a las de Python"
            );
            let mut cuenta = [0usize; 3];
            comparar_paquete_c(&trabajo, &esperadas, etapa, corte_fijo, &mut cuenta, &mut fallos);
            identicas += cuenta[0];
            r1 += cuenta[1];
            r6 += cuenta[2];

            let movidas: Vec<&serde_json::Value> = filas
                .iter()
                .filter(|f| f["etapa"] == etapa && f["corte"] == corte_movido.to_string())
                .collect();
            if movidas.is_empty() {
                continue;
            }
            let trabajo_corte = Trabajo::abrir_archivo(
                PathBuf::from(&zip),
                Some(modo),
                corte_movido,
                &cfg,
            )
            .unwrap_or_else(|e| panic!("no se pudo abrir el paquete C en {etapa} al corte movido: {e}"));
            assert_eq!(
                trabajo_corte.registros.len(),
                movidas.len(),
                "{etapa}: filas distintas al corte movido"
            );
            filas_corte += movidas.len();
            let mut cuenta = [0usize; 3];
            comparar_paquete_c(&trabajo_corte, &movidas, etapa, corte_movido, &mut cuenta, &mut fallos);
            identicas_corte += cuenta[0];
            constancias += trabajo_corte
                .registros
                .iter()
                .filter(|r| r.eventos.iter().any(|e| e.starts_with("HISTORIA.")))
                .count();
        }

        for no_aplica in esperado["no_aplica"].as_array().unwrap() {
            println!(
                "PAQUETE_C_NO_APLICA: etapa {}, {}",
                no_aplica["etapa"].as_str().unwrap_or(""),
                no_aplica["motivo"].as_str().unwrap_or("")
            );
        }
        for fallo in &fallos {
            println!("FALLA {fallo}");
        }
        println!(
            "PAQUETE_C: etapas {etapas}, filas {filas_total}, identicas {identicas}, R1 {r1}, R6 {r6}, fallos {}",
            fallos.len()
        );
        println!(
            "PAQUETE_C_CORTE: filas {filas_corte}, identicas {identicas_corte}, constancias_historia {constancias}, fallos {}",
            fallos.len()
        );
        assert_eq!(etapas, 3, "el paquete C debe abrir Espera, Cumplimiento e Informes");
        assert_eq!(filas_total, 6, "el paquete C debe traer seis filas en el corte fijo");
        assert_eq!(constancias, 0, "el corte movido no debe tener constancias de historia");
        assert_eq!(identicas_corte, filas_corte, "el corte movido debe coincidir con Python en todas las filas");
        assert!(fallos.is_empty(), "el paquete C tiene diferencias no admitidas");
    }

    /// Medición: 20.000 registros (el máximo operativo del original). `cargo test --release -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn velocidad_20000() {
        let cfg = Config::default();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("test_out");
        std::fs::create_dir_all(&dir).unwrap();
        let base = crate::lectura::read_sheets(&std::fs::read(crate::demo::libro(Mode::Espera, hoy()).unwrap()).unwrap()).unwrap();
        let headers: Vec<String> = base[0].rows[0].iter().map(|c| c.display()).collect();
        let mut rows = Vec::new();
        while rows.len() < 20_000 {
            for (k, r) in base[0].rows.iter().skip(1).enumerate() {
                let mut v: Vec<String> = r.iter().map(|c| c.display()).collect();
                v[0] = format!("X-{}-2099", rows.len() + k);
                rows.push(v);
            }
        }
        rows.truncate(20_000);
        let p = dir.join("velocidad_20000.xlsx");
        crate::productos::escribir_xlsx(&[("Espera", headers, rows)], &p).unwrap();
        let t0 = std::time::Instant::now();
        let w = Trabajo::abrir(p, std::fs::read(dir.join("velocidad_20000.xlsx")).unwrap(), Some(Mode::Espera), hoy(), &cfg).unwrap();
        let total = t0.elapsed();
        let t1 = std::time::Instant::now();
        let x = crate::productos::excel_observaciones(&w, &dir).unwrap();
        println!("VELOCIDAD: {} registros leídos y analizados en {:?} (reglas {} ms); Excel de observaciones en {:?} ({})", w.registros.len(), total, w.analisis_ms, t1.elapsed(), x.display());
        assert_eq!(w.registros.len(), 20_000);
    }

    /// Formatos reales de SITFA (.xls HTML, XML 2003 y BIFF): FLOW_FIXTURES apunta a una carpeta con ellos.
    #[test]
    #[ignore = "requiere FLOW_FIXTURES"]
    fn formatos_sitfa() {
        let d = crate::prueba_entorno::requerida("FLOW_FIXTURES");
        let cfg = Config::default();
        for f in ["espera_html.xls", "espera_xml.xls", "espera_biff.xls"] {
            let p = std::path::Path::new(&d).join(f);
            let bytes = std::fs::read(&p).unwrap();
            let w = Trabajo::abrir(p.clone(), bytes, None, hoy(), &cfg).unwrap_or_else(|e| panic!("{f}: {e}"));
            assert_eq!(w.mode(), Mode::Espera, "{f}");
            assert!(w.registros.len() >= 2, "{f}");
            for r in &w.registros {
                println!("{f}: {} | {:?}", r.propuesta, r.avisos);
            }
            assert!(w.registros[0].propuesta.starts_with("Sofía PRM:"), "{f}: {}", w.registros[0].propuesta);
        }
    }
}
