//! Lecturas adicionales dentro de la página vigente. Persistencia por huella, sin HTML ni valores cifrados.

use crate::{
    lotes::{self, Conexion, Resumen},
    vinculos::{self, Vinculo},
};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub const TOPE: usize = 600;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Opciones {
    pub bitacoras: bool,
    pub historia: bool,
    pub calendario: bool,
    pub meses: u32,
}
impl Default for Opciones {
    fn default() -> Self {
        Self { bitacoras: false, historia: false, calendario: false, meses: 4 }
    }
}
impl Opciones {
    pub fn activa(&self) -> bool {
        self.bitacoras || self.historia || self.calendario
    }
    pub fn validar(&self) -> Result<(), String> {
        if !(1..=4).contains(&self.meses) {
            Err("El período de lectura va de 1 a 4 meses.".into())
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagina {
    pub consulta: usize,
    pub numero: usize,
    pub vinculos: Vec<Vinculo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Referencia {
    pub tipo: String,
    pub clave: String,
    pub archivo: String,
    pub sha256: String,
    pub capturada: String,
    pub meses: u32,
    pub tribunal: String,
    pub causa: String,
    pub ingreso: String,
    pub reutilizada: bool,
}

pub struct PaginaActual<'a> {
    pub consulta: usize,
    pub numero: usize,
    pub html: &'a [u8],
}
pub struct Control<'a> {
    pub cancelar: &'a AtomicBool,
    pub progreso: &'a dyn Fn(String),
}

pub fn clave(tipo: &str, v: &Vinculo) -> String {
    if tipo == "historia" {
        format!("H|{}|{}", v.tribunal, v.causa)
    } else {
        format!("{tipo}|{}|{}|{}|{}", v.tribunal, v.causa, v.ingreso, v.etapa)
    }
}

fn del_dia(capturada: &str, hoy: chrono::NaiveDate) -> bool {
    DateTime::parse_from_rfc3339(capturada).is_ok_and(|f| f.with_timezone(&Local).date_naive() == hoy)
}

pub fn leer(carpeta: &Path, r: &Referencia) -> Result<Value, String> {
    let ruta = lotes::ruta_archivo(carpeta, &r.archivo)?;
    let archivo = std::fs::File::open(&ruta).map_err(|_| "Falta una lectura verificada del lote.")?;
    let mut datos = vec![];
    archivo.take(32 * 1024 * 1024 + 1).read_to_end(&mut datos).map_err(|_| "No se pudo leer un contexto del lote.")?;
    if datos.len() > 32 * 1024 * 1024 || lotes::huella(&datos) != r.sha256 {
        return Err("Cambió una lectura verificada del lote.".into());
    }
    let valor: Value = serde_json::from_slice(&datos).map_err(|_| "No se reconoce una lectura guardada.")?;
    if valor["version"] != 1 || valor["tipo"] != r.tipo || valor["clave"] != r.clave {
        return Err("La identidad de una lectura guardada no coincide con su manifiesto.".into());
    }
    Ok(valor)
}

pub struct Recoleccion {
    hoy: chrono::NaiveDate,
    carpeta: PathBuf,
    previas: BTreeMap<String, (PathBuf, Referencia)>,
    aperturas: usize,
    historias: usize,
    pendientes: usize,
    consultas_incompletas: BTreeSet<usize>,
}

impl Recoleccion {
    fn reservar(&mut self, tipo: &str) -> Result<(), String> {
        if tipo == "historia" {
            if self.historias >= TOPE {
                return Err("Se alcanzó el máximo de 600 historias de esta corrida. Retoma el lote para continuar.".into());
            }
            self.historias += 1;
        } else {
            if self.aperturas >= TOPE {
                return Err(
                    "Se alcanzó el máximo de 600 lecturas de bitácoras/calendarios de esta corrida. Retoma el lote para continuar.".into(),
                );
            }
            self.aperturas += 1;
        }
        Ok(())
    }

    pub fn nueva(carpeta: &Path, resumen: &Resumen) -> Result<Self, String> {
        Self::con_fecha(carpeta, resumen, Local::now().date_naive())
    }

    pub fn con_fecha(carpeta: &Path, resumen: &Resumen, hoy: chrono::NaiveDate) -> Result<Self, String> {
        let mut previas = BTreeMap::new();
        if resumen.lote.lecturas.activa() {
            // Primero las anteriores y al final el mismo lote: siempre prevalece su checkpoint.
            let mut carpetas = std::fs::read_dir(carpeta.parent().unwrap_or(carpeta))
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()) && e.path() != carpeta)
                .filter_map(|e| e.metadata().and_then(|m| m.modified()).ok().map(|t| (e.path(), t)))
                .collect::<Vec<_>>();
            carpetas.sort_by_key(|a| std::cmp::Reverse(a.1));
            for (c, _) in carpetas.into_iter().take(10).rev() {
                if let Ok(r) = Resumen::leer(&c) {
                    for l in r.lecturas.into_iter().filter(|l| del_dia(&l.capturada, hoy)) {
                        if leer(&c, &l).is_ok() {
                            previas.insert(l.clave.clone(), (c.clone(), l));
                        }
                    }
                }
            }
            for r in &resumen.lecturas {
                leer(carpeta, r)?;
                previas.insert(r.clave.clone(), (carpeta.to_path_buf(), r.clone()));
            }
        }
        Ok(Self {
            hoy,
            carpeta: carpeta.to_path_buf(),
            previas,
            aperturas: 0,
            historias: 0,
            pendientes: 0,
            consultas_incompletas: BTreeSet::new(),
        })
    }

    fn persistir(
        &mut self,
        resumen: &mut Resumen,
        tipo: &str,
        v: &Vinculo,
        mut valor: Value,
        meses: u32,
        reutilizada: bool,
    ) -> Result<(), String> {
        let capturada = valor["captured_at"].as_str().unwrap_or("").to_string();
        if DateTime::parse_from_rfc3339(&capturada).is_err() {
            return Err("La lectura no conserva una fecha de captura válida.".into());
        }
        if tipo == "bitacora" && valor.get("desde").is_none() {
            let hasta = DateTime::parse_from_rfc3339(&capturada).unwrap().with_timezone(&Local).date_naive();
            let desde = hasta.checked_sub_months(chrono::Months::new(meses)).ok_or("Período de lectura inválido.")?;
            let entries = valor["entries"].as_array().ok_or("La bitácora no conserva sus entradas.")?;
            let dentro = entries
                .iter()
                .filter(|e| {
                    e["fecha"]
                        .as_str()
                        .and_then(|s| s.get(..10))
                        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                        .is_some_and(|f| f >= desde)
                })
                .cloned()
                .collect::<Vec<_>>();
            valor["entries"] = json!(dentro);
            valor["desde"] = json!(desde.to_string());
            valor["hasta"] = json!(hasta.to_string());
        }
        if tipo == "historia" && valor.get("desde").is_none() {
            let hasta = DateTime::parse_from_rfc3339(&capturada).unwrap().with_timezone(&Local).date_naive();
            let desde = hasta.checked_sub_months(chrono::Months::new(meses)).ok_or("Período de lectura inválido.")?;
            let filas = valor["filas"].as_array().ok_or("La historia no conserva sus filas.")?;
            let [dentro, anteriores, posteriores, sin_fecha] = crate::lecturas::separar_ventana(filas, hasta, meses)?;
            let futura = !posteriores.is_empty();
            valor["filas"] = json!(dentro.into_iter().chain(posteriores).chain(sin_fecha).collect::<Vec<_>>());
            valor["desde"] = json!(desde.to_string());
            valor["hasta"] = json!(hasta.to_string());
            valor["anteriores"] = json!(anteriores.len());
            if futura {
                valor["advertencias"]
                    .as_array_mut()
                    .ok_or("La historia no conserva sus advertencias.")?
                    .push(json!({"codigo":"HISTORIA_FECHA_POSTERIOR","fila":0,"folio":""}));
            }
        }
        let clave = clave(tipo, v);
        let datos = serde_json::to_vec(&json!({"version":1,"tipo":tipo,"clave":clave,"vinculo":v,"datos":valor}))
            .map_err(|_| "No se pudo preparar la lectura local.")?;
        let sha256 = lotes::huella(&datos);
        let archivo = format!("lectura-{}-{}.json", &lotes::huella(clave.as_bytes())[..20], &sha256[..20]);
        let ruta = lotes::ruta_archivo(&self.carpeta, &archivo)?;
        if ruta.exists() {
            if std::fs::read(&ruta).map(|d| lotes::huella(&d)).ok() != Some(sha256.clone()) {
                return Err("Existe una lectura local distinta con el mismo nombre.".into());
            }
        } else {
            lotes::exclusivo(&ruta, &datos)?;
        }
        let r = Referencia {
            tipo: tipo.into(),
            clave: clave.clone(),
            archivo,
            sha256,
            capturada,
            meses,
            tribunal: v.tribunal.clone(),
            causa: v.causa.clone(),
            ingreso: v.ingreso.clone(),
            reutilizada,
        };
        resumen.lecturas.retain(|l| l.clave != clave);
        resumen.lecturas.push(r.clone());
        resumen.guardar(&self.carpeta)?;
        self.previas.insert(clave, (self.carpeta.clone(), r));
        Ok(())
    }

    fn reutilizar(&mut self, resumen: &mut Resumen, tipo: &str, v: &Vinculo, meses: u32) -> Result<bool, String> {
        let Some((carpeta, r)) = self.previas.get(&clave(tipo, v)).cloned() else {
            return Ok(false);
        };
        if !del_dia(&r.capturada, self.hoy) || tipo != "calendario" && r.meses != meses {
            return Ok(false);
        }
        let lectura = leer(&carpeta, &r)?;
        let anterior: Vinculo =
            serde_json::from_value(lectura["vinculo"].clone()).map_err(|_| "El vínculo de una lectura guardada no se reconoce.")?;
        if vinculos::normalizar(&anterior.rit) != vinculos::normalizar(&v.rit) || tipo != "historia" && anterior.clave() != v.clave() {
            return Ok(false);
        }
        if carpeta == self.carpeta {
            if !resumen.lecturas.iter().any(|l| l.clave == r.clave) {
                resumen.lecturas.push(r);
                resumen.guardar(&self.carpeta)?;
            }
        } else {
            self.persistir(resumen, tipo, v, lectura["datos"].clone(), meses, true)?;
        }
        Ok(true)
    }

    pub fn pagina(
        &mut self,
        conexion: &dyn Conexion,
        catalogo: &Value,
        resumen: &mut Resumen,
        actual: PaginaActual<'_>,
        control: Control<'_>,
    ) -> Result<(), String> {
        let seleccion = resumen.lote.consultas[actual.consulta].clone();
        let opciones = resumen.lote.lecturas.clone();
        let pausa = resumen.lote.pausa_ms;
        let enlaces = vinculos::extraer(actual.html, &seleccion)?;
        resumen.paginas_lectura.retain(|p| p.consulta != actual.consulta || p.numero != actual.numero);
        resumen.paginas_lectura.push(Pagina { consulta: actual.consulta, numero: actual.numero, vinculos: enlaces });
        let mut todos = resumen.paginas_lectura.iter().flat_map(|p| p.vinculos.clone()).collect::<Vec<_>>();
        vinculos::descartar_ambiguos(&mut todos);
        let mut todos = todos.into_iter();
        for pagina in &mut resumen.paginas_lectura {
            for v in &mut pagina.vinculos {
                *v = todos.next().unwrap();
            }
        }
        resumen.guardar(&self.carpeta)?;
        let enlaces =
            resumen.paginas_lectura.iter().find(|p| p.consulta == actual.consulta && p.numero == actual.numero).unwrap().vinculos.clone();
        for v in enlaces {
            if control.cancelar.load(Ordering::SeqCst) {
                return Err("Lote cancelado. Las lecturas verificadas se conservan.".into());
            }
            let tribunal = catalogo["tribunales"][&seleccion.tribunal].as_str().unwrap_or("");
            for (tipo, habilitada, vinculada) in [
                ("bitacora", opciones.bitacoras, v.bitacora_verificada),
                ("historia", opciones.historia, v.historia_verificada),
                (
                    "calendario",
                    opciones.calendario && matches!(seleccion.tab.as_str(), "Cumplimiento" | "Informes"),
                    v.bitacora_verificada && v.calendario.is_some(),
                ),
            ] {
                if !habilitada {
                    continue;
                }
                let prefijo = format!("Consulta {}, fila {}: {tipo} ", actual.consulta + 1, v.fila);
                resumen.avisos.retain(|s| {
                    !s.starts_with(&format!("{prefijo}pendiente.")) && !s.starts_with(&format!("{prefijo}sin vínculo comprobado;"))
                });
                if !vinculada {
                    let aviso = format!("Consulta {}, fila {}: {tipo} sin vínculo comprobado; no evaluable.", actual.consulta + 1, v.fila);
                    if !resumen.avisos.contains(&aviso) {
                        resumen.avisos.push(aviso);
                    }
                    continue;
                }
                if self.reutilizar(resumen, tipo, &v, opciones.meses)? {
                    continue;
                }
                self.reservar(tipo)?;
                lotes::pausa(pausa, control.cancelar)?;
                (control.progreso)(format!("Consulta {} · fila {} · leyendo {tipo}", actual.consulta + 1, v.fila));
                let lectura = match tipo {
                    "bitacora" => vinculos::abrir_bitacora(conexion, &v, &seleccion.modality),
                    "historia" => vinculos::abrir_historia(conexion, &v, tribunal),
                    _ => vinculos::abrir_calendario(conexion, &v),
                };
                match lectura {
                    Ok(datos) => self.persistir(resumen, tipo, &v, datos, opciones.meses, false)?,
                    Err(e) => {
                        self.pendientes += 1;
                        self.consultas_incompletas.insert(actual.consulta);
                        let aviso = format!("Consulta {}, fila {}: {tipo} pendiente. {e}", actual.consulta + 1, v.fila);
                        if !resumen.avisos.contains(&aviso) {
                            resumen.avisos.push(aviso);
                        }
                        resumen.guardar(&self.carpeta)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn terminar(&self, resumen: &mut Resumen) -> Result<(), String> {
        resumen.terminadas.retain(|i| !self.consultas_incompletas.contains(i));
        if self.pendientes != 0 {
            Err(format!(
                "Los Excel verificados se conservan. Quedaron {} lecturas pendientes; retoma el lote para reintentarlas.",
                self.pendientes
            ))
        } else {
            Ok(())
        }
    }
    pub fn completa(&self, consulta: usize) -> bool {
        !self.consultas_incompletas.contains(&consulta)
    }
}

#[cfg(test)]
mod pruebas_limite {
    use super::*;
    #[test]
    fn tope_comparte_bitacoras_calendario_y_separa_historias() {
        let mut r = Recoleccion {
            hoy: Local::now().date_naive(),
            carpeta: PathBuf::new(),
            previas: BTreeMap::new(),
            aperturas: 0,
            historias: 0,
            pendientes: 0,
            consultas_incompletas: BTreeSet::new(),
        };
        for _ in 0..300 {
            r.reservar("bitacora").unwrap();
            r.reservar("calendario").unwrap();
        }
        assert!(r.reservar("bitacora").is_err());
        assert!(r.reservar("calendario").is_err());
        for _ in 0..600 {
            r.reservar("historia").unwrap();
        }
        assert!(r.reservar("historia").is_err());
    }
}
