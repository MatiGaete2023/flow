//! Trabajo en curso: el libro abierto, la observación propuesta de cada fila y las decisiones del usuario.

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
}

impl Trabajo {
    pub fn abrir(path: PathBuf, bytes: Vec<u8>, pedida: Option<Mode>, corte: NaiveDate, cfg: &Config) -> Result<Trabajo, String> {
        let sheets = lectura::read_sheets(&bytes)?;
        let tablas = lectura::detectar(&sheets);
        if tablas.is_empty() {
            return Err("No se identifica una tabla de Espera, Cumplimiento o Informes en el archivo. Revisa sus encabezados (deben estar DERIVACION, TRIBUNAL y T ESPERA, DIAS DE CUMPLIMIENTO/DIAS PARA EGRESAR o FECHA VENCIMIENTO).".into());
        }
        let (ti, mode) = lectura::elegir(&sheets, &tablas, pedida).ok_or("La etapa elegida no corresponde a las columnas del archivo.")?;
        let t = &tablas[ti];
        let hash = hex(&Sha256::digest(&bytes));
        let mut w = Trabajo {
            nombre: path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
            path,
            hash,
            bytes,
            sheet: t.sheet,
            header_row: t.header_row,
            mode: Some(mode),
            modos_posibles: t.modes.clone(),
            sheets,
            corte: Some(corte),
            ..Default::default()
        };
        w.headers = lectura::headers_of(&w.sheets[w.sheet].rows[w.header_row]);
        w.analizar(cfg);
        w.restaurar();
        Ok(w)
    }

    /// Vuelve a aplicar las reglas a todas las filas. Conserva las decisiones del usuario por fila.
    pub fn analizar(&mut self, cfg: &Config) {
        let t0 = std::time::Instant::now();
        let mode = self.mode.unwrap_or(Mode::Espera);
        let corte = self.corte.unwrap_or_else(|| chrono::Local::now().date_naive());
        self.cols = lectura::map_columns(&self.headers, mode);
        self.avisos.clear();
        if self.modos_posibles.len() > 1 {
            self.avisos.push(format!(
                "El archivo sirve para varias etapas ({}); se analiza como {}. Puedes cambiarla arriba.",
                self.modos_posibles.iter().map(|m| m.label()).collect::<Vec<_>>().join(", "),
                mode.label()
            ));
        }
        // Cruce (hoja 2) para C-10: identidad completa, fecha futura, la última fila gana.
        let mut cruce: HashMap<Vec<String>, NaiveDate> = HashMap::new();
        self.cruce = None;
        if mode == Mode::Cumplimiento {
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
        let mut filas_idx = Vec::new();
        for (i, row) in sheet.rows.iter().enumerate().skip(self.header_row + 1) {
            if row.iter().all(|c| c.is_empty()) {
                continue;
            }
            // Totales, pies y notas no son registros: solo filas con algún dato de columna reconocida.
            if !mapped.iter().any(|c| row.get(*c).is_some_and(|v| !v.is_empty())) {
                continue;
            }
            filas_idx.push(i);
        }
        let filas: Vec<Fila> = filas_idx.iter().map(|i| Fila { values: &sheet.rows[*i], cols: &self.cols }).collect();
        let pre = motor::prepaso(mode, &filas, corte);
        let mut registros = Vec::with_capacity(filas.len());
        for (n, (f, i)) in filas.iter().zip(&filas_idx).enumerate() {
            let fecha_hoja2 = if mode == Mode::Cumplimiento {
                let k: Vec<String> = ["rit", "rut", "nombre", "tribunal", "programa"].iter().map(|k| historical_match(&f.text(k))).collect();
                cruce.get(&k).copied()
            } else {
                None
            };
            let r = motor::evaluar(cfg, mode, f, corte, &pre, n, fecha_hoja2);
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

    /// Guarda las decisiones del usuario (ediciones, revisadas, selección, RES).
    pub fn guardar_sesion(&self) -> Result<(), String> {
        let mut s = Sesion {
            archivo: self.nombre.clone(),
            hash: self.hash.clone(),
            modo: self.mode().key().into(),
            corte: self.corte.map(|c| c.to_string()).unwrap_or_default(),
            guardada: chrono::Local::now().to_rfc3339(),
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
        config::atomic_write(&self.sesion_path(), &data).map_err(|e| e.to_string())?;
        limpiar_sesiones();
        Ok(())
    }

    fn restaurar(&mut self) {
        let Ok(t) = std::fs::read_to_string(self.sesion_path()) else { return };
        let Ok(s) = serde_json::from_str::<Sesion>(&t) else {
            self.avisos.push("La sesión anterior de este archivo no se pudo leer; se conserva sin cambios.".into());
            return;
        };
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
    use crate::productos;

    fn hoy() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 19).unwrap()
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
                assert!(std::fs::metadata(d).unwrap().len() > 500);
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
    fn formatos_sitfa() {
        let Ok(d) = std::env::var("FLOW_FIXTURES") else { return };
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
