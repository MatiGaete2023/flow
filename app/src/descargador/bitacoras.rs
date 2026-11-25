//! E-Q4a · Lote de bitácoras: el comportamiento de `descargador/lectura_bitacoras.py::DiaryRunner` (Flow Python) para
//! la ruta principal. Enumera cada consulta de Seguimiento, lee la bitácora de cada fila con vínculo comprobado por el
//! puente (solo con la extensión Integral) y escribe `bitacoras.json` y las instantáneas `bitacora-*.html` con el mismo
//! formato que Python. Solo lectura de SITFA: no hay operaciones de escritura en RUS.
//!
//! Pendiente en E-Q4b/c/d: retomar y reutilizar lecturas, calendario, historia e histórico, el diagnóstico anónimo y el
//! Excel consolidado.

use super::lote::{self, Conexion};
use super::sitfa::{self, Consulta, Pares, Respuesta};
use super::{lecturas, vinculos, Alcance, Catalogo, Edicion, Pantalla, Perfil};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const MANIFIESTO: &str = "bitacoras.json";
/// `PAUSA_SITFA_S`, `CHECKPOINT_CADA`, `MAX_LECTURAS_POR_LOTE` y `MAX_FALLAS_SEGUIDAS` de `lectura_bitacoras.py`.
pub const PAUSA_SITFA: Duration = Duration::from_secs(1);
const CHECKPOINT_CADA: usize = 25;
pub const MAX_LECTURAS_POR_LOTE: usize = 5000;
pub const MAX_FALLAS_SEGUIDAS: usize = 5;
const ESPERA_MS: u64 = 60_000;
const MODALIDADES: [(&str, &str); 4] = [("1", "Residencia"), ("2", "Ambulatorio"), ("3", "FAE"), ("4", "DCE")];

/// Opciones de `DiaryRunner`: pausa, carpeta a retomar, destino donde buscar lecturas recientes y reloj (pruebas).
#[derive(Clone, Debug, Default)]
pub struct Opciones {
    pub pausa: Duration,
    /// `resume`: carpeta de un lote anterior; lo ya leído con su huella intacta no se vuelve a pedir.
    pub retomar: Option<PathBuf>,
    /// `reuse_base`: destino del usuario donde buscar lecturas de otros lotes del mismo día (EF-04).
    pub reutilizar_base: Option<PathBuf>,
    /// Reloj fijo para las pruebas; `None` usa la hora local.
    pub ahora: Option<chrono::DateTime<chrono::FixedOffset>>,
    /// `calendars`: calendario de informes de cada ingreso de Cumplimiento e Informes (cuenta en el tope de lecturas).
    pub calendarios: bool,
    /// `historico`: Ingreso Histórico Protección de cada persona del listado, una vez por persona y lote (P3c).
    pub historico: bool,
    /// `historia`: historia de la causa en la misma pasada (`LectorHistoria`), con su ventana de 1 a 4 meses.
    pub historia: Option<u32>,
    /// `LectorHistoria(anterior=…)`: `historias.json` de un lote anterior para reutilizar lo ya leído.
    pub historia_anterior: Option<PathBuf>,
}

/// `MAX_HISTORICOS_FALLIDOS_SEGUIDOS`: tras tantas ventanas fallidas seguidas se suspende el histórico.
pub const MAX_HISTORICOS_FALLIDOS_SEGUIDOS: usize = 5;

/// `PESTANAS_CALENDARIO`.
const PESTANAS_CALENDARIO: [&str; 2] = ["Cumplimiento", "Informes"];

/// `REUTILIZAR_HORAS` y `REUTILIZAR_CARPETAS`.
const REUTILIZAR_HORAS: i64 = 12;
const REUTILIZAR_CARPETAS: usize = 10;

/// `lotes.Batch` de Seguimiento para bitácoras (informe por omisión «1», sin fechas ni filtros conservados).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoteBitacoras {
    pub tribunales: Vec<String>,
    pub modalidades: Vec<String>,
    pub pestanas: Vec<String>,
}

impl LoteBitacoras {
    /// `saved_batch(load(carpeta / 'bitacoras.json'))`: el alcance guardado de un lote de bitácoras, para retomarlo.
    pub fn guardado(carpeta: &Path) -> Result<Self, String> {
        let datos = cargar(&carpeta.join(MANIFIESTO))?;
        let s = &datos["seleccion"];
        if s["screen"] != "seguimiento" {
            return Err("Selecciona Seguimiento para leer bitácoras.".into());
        }
        let lista = |k: &str| -> Result<Vec<String>, String> {
            s[k].as_array().ok_or("El lote recuperado no trae su alcance.")?.iter()
                .map(|v| v.as_str().map(str::to_string).ok_or_else(|| "El lote recuperado no trae su alcance.".to_string()))
                .collect()
        };
        Ok(Self { tribunales: lista("tribunals")?, modalidades: lista("modalities")?, pestanas: lista("tabs")? })
    }

    /// `asdict(batch)`.
    fn seleccion(&self) -> Value {
        json!({"tribunals": self.tribunales, "modalities": self.modalidades, "tabs": self.pestanas, "report": "1",
            "dates": null, "keep_filters": false, "screen": "seguimiento", "state": "1", "month": null, "year": null})
    }

    /// `Batch.plan`: tribunal × pestaña × modalidad, con informe solo en Informes.
    fn plan(&self, catalogo: &Catalogo) -> Result<Vec<Perfil>, String> {
        if self.tribunales.is_empty() || self.modalidades.is_empty() || self.pestanas.is_empty() {
            return Err("Seleccione tribunales, modalidades y pestañas.".into());
        }
        for valores in [&self.tribunales, &self.modalidades, &self.pestanas] {
            if valores.iter().collect::<BTreeSet<_>>().len() != valores.len() {
                return Err("El lote tiene opciones repetidas.".into());
            }
        }
        if self.modalidades.iter().any(|m| !MODALIDADES.iter().any(|(c, _)| c == m)) {
            return Err("Modalidad no auditada.".into());
        }
        let mut plan = Vec::new();
        for t in &self.tribunales {
            if !catalogo.tribunales.contains_key(t) {
                return Err("Tribunal fuera del catálogo.".into());
            }
            for tab in &self.pestanas {
                for m in &self.modalidades {
                    plan.push(Perfil {
                        pantalla: Pantalla::Seguimiento,
                        tribunal: t.clone(),
                        alcance: Alcance::Seguimiento {
                            pestana: tab.clone(),
                            modalidad: m.clone(),
                            reporte: (tab == "Informes").then(|| "1".to_string()),
                        },
                        etiqueta: String::new(),
                    });
                }
            }
        }
        Ok(plan)
    }
}

enum Detencion {
    /// `LoteDetenido` / `LimiteLecturas`: detiene todo el lote.
    Lote(String),
    /// Error de una consulta: queda FALLIDA y el lote sigue.
    Consulta(String),
}

impl From<String> for Detencion {
    fn from(e: String) -> Self {
        Detencion::Consulta(e)
    }
}

struct Sesion<'a> {
    conexion: &'a dyn Conexion,
    carpeta: PathBuf,
    estado: Value,
    pendiente: usize,
    aperturas: usize,
    fallas_seguidas: usize,
    cancelar: &'a AtomicBool,
    pausa: Duration,
    /// Registros del lote retomado (`registros` + `anteriores_no_reenumeradas`) y sus LEIDA por clave.
    previos: Vec<Value>,
    antiguos: BTreeMap<String, Value>,
    /// Lecturas reutilizables de otros lotes: clave → (carpeta, registro).
    cruzadas: BTreeMap<String, (PathBuf, Value)>,
    calendarios: bool,
    historico: bool,
    /// Históricos ya resueltos en este lote por huella de persona (`_historicos`), fallas seguidas y suspensión.
    historicos: Option<BTreeMap<String, Value>>,
    historicos_fallidos: usize,
    historico_suspendido: bool,
    ahora: chrono::DateTime<chrono::FixedOffset>,
    historia: Option<super::historia::LectorHistoria>,
    /// `emit` de `DiaryRunner`: `query`, `diary_progress` y `warning` para la interfaz (E-Q7).
    avance: Avance<'a>,
}

/// Canal de avance del lote de bitácoras: nombre del evento de Python y su valor.
pub type Avance<'a> = &'a dyn Fn(&str, Value);

impl Sesion<'_> {
    /// `_Lectura.persistir`: escritura atómica (`bitacoras.tmp` y reemplazo); con `perezosa`, cada `CHECKPOINT_CADA`.
    fn persistir(&mut self, perezosa: bool) -> Result<(), String> {
        if perezosa {
            self.pendiente += 1;
            if self.pendiente < CHECKPOINT_CADA {
                return Ok(());
            }
            self.pendiente = 0;
        }
        if !self.previos.is_empty() {
            // Al retomar, se conserva lo ya leído que aún no se volvió a enumerar (`anteriores_no_reenumeradas`).
            let actuales: BTreeSet<String> = self.estado["registros"]
                .as_array()
                .map(|r| r.iter().filter_map(|x| x["clave"].as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let (mut orden, mut ultimo): (Vec<String>, BTreeMap<String, Value>) = (Vec::new(), BTreeMap::new());
            for r in &self.previos {
                let clave = r["clave"].as_str().unwrap_or("").to_string();
                if actuales.contains(&clave) {
                    continue;
                }
                if !ultimo.contains_key(&clave) {
                    orden.push(clave.clone());
                }
                ultimo.insert(clave, r.clone());
            }
            self.estado["anteriores_no_reenumeradas"] = json!(orden.iter().map(|c| ultimo[c].clone()).collect::<Vec<_>>());
        }
        let datos = serde_json::to_vec_pretty(&self.estado).map_err(|e| e.to_string())?;
        let temporal = self.carpeta.join("bitacoras.tmp");
        std::fs::write(&temporal, datos)
            .and_then(|_| std::fs::rename(&temporal, self.carpeta.join(MANIFIESTO)))
            .map_err(|e| format!("No se pudo guardar el manifiesto de bitácoras: {e}"))
    }

    fn detener_si_cancelada(&self) -> Result<(), Detencion> {
        if self.cancelar.load(Ordering::SeqCst) {
            return Err(Detencion::Lote("Lectura cancelada. Se conserva lo ya comprobado.".into()));
        }
        Ok(())
    }

    fn pedir(&self, comando: &str, payload: Value) -> Result<Value, String> {
        self.conexion.llamar(comando, payload, lote::PLAZO)
    }
}

/// `json.dumps([...], ensure_ascii=False)` de Python: separadores «, » y cadenas sin escapar lo no ASCII.
fn json_python(v: &Value) -> String {
    match v {
        Value::Array(a) => format!("[{}]", a.iter().map(json_python).collect::<Vec<_>>().join(", ")),
        otro => serde_json::to_string(otro).unwrap_or_default(),
    }
}

fn sha256(datos: &[u8]) -> String {
    format!("{:x}", Sha256::digest(datos))
}

/// `bitacora_html.visible_identity_warnings`.
pub(crate) fn advertencias(leida: &Value, rit: &str, nombre: &str) -> Vec<String> {
    let mut a = Vec::new();
    if lecturas::normalizar(leida["rit"].as_str().unwrap_or("")) != lecturas::normalizar(rit) {
        a.push("Advertencia: RIT visible de bitácora distinto al listado; revisar el ingreso.".to_string());
    }
    if lecturas::normalizar(leida["persona"].as_str().unwrap_or("")) != lecturas::normalizar(nombre) {
        a.push("Advertencia: nombre visible de bitácora distinto al listado; revisar el ingreso.".to_string());
    }
    a
}

/// `faltantes_por_tribunal`: bitácoras que no quedaron LEIDA, por tribunal y estado.
pub fn faltantes_por_tribunal(registros: &[Value]) -> Value {
    let mut salida: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for r in registros.iter().filter(|r| r["estado"] != "LEIDA") {
        let codigo = r["tribunal_codigo"].as_str().unwrap_or("").to_string();
        let item = salida.entry(codigo).or_insert_with(|| {
            Map::from_iter([("nombre".to_string(), json!(r["tribunal_nombre"].as_str().unwrap_or("")))])
        });
        let estado = r["estado"].as_str().unwrap_or("FALLIDA").to_string();
        let n = item.get(&estado).and_then(Value::as_u64).unwrap_or(0) + 1;
        item.insert(estado, json!(n));
    }
    json!(salida)
}

fn ahora_local() -> chrono::DateTime<chrono::Local> {
    chrono::Local::now()
}

/// `DiaryRunner.run` (ruta principal). Devuelve el resultado de `_resultado` sin el Excel ni el diagnóstico (E-Q4d).
pub fn ejecutar(
    conexion: &dyn Conexion,
    catalogo: &Catalogo,
    lote_bitacoras: &LoteBitacoras,
    destino: &Path,
    opciones: &Opciones,
    cancelar: &AtomicBool,
) -> Result<Value, String> {
    ejecutar_con_avance(conexion, catalogo, lote_bitacoras, destino, opciones, cancelar, &|_, _| {})
}

/// `DiaryRunner.run` con su `emit`: una consulta por perfil (`query`), una fila por registro (`diary_progress`) y los
/// avisos (`warning`).
pub fn ejecutar_con_avance(
    conexion: &dyn Conexion,
    catalogo: &Catalogo,
    lote_bitacoras: &LoteBitacoras,
    destino: &Path,
    opciones: &Opciones,
    cancelar: &AtomicBool,
    avance: Avance<'_>,
) -> Result<Value, String> {
    let plan = lote_bitacoras.plan(catalogo)?;
    let inicio = opciones.ahora.unwrap_or_else(|| ahora_local().fixed_offset());
    let (mut previos, mut antiguos) = (Vec::new(), BTreeMap::new());
    let carpeta = if let Some(retomar) = &opciones.retomar {
        let carpeta = retomar.canonicalize().map_err(|e| format!("No se pudo abrir el lote recuperado: {e}"))?;
        let anterior = cargar(&carpeta.join(MANIFIESTO))?;
        if anterior["seleccion"] != lote_bitacoras.seleccion() {
            return Err("El alcance no coincide con el lote recuperado.".into());
        }
        for clave in ["registros", "anteriores_no_reenumeradas"] {
            previos.extend(anterior[clave].as_array().cloned().unwrap_or_default());
        }
        for r in previos.iter().rev() {
            if r["estado"] == "LEIDA" {
                antiguos.insert(r["clave"].as_str().unwrap_or("").to_string(), r.clone());
            }
        }
        carpeta
    } else {
        let carpeta = destino.join(format!("Bitacoras {}", inicio.format("%Y-%m-%d %H-%M-%S-%6f")));
        std::fs::create_dir_all(destino).map_err(|e| format!("No se pudo crear la carpeta de destino: {e}"))?;
        std::fs::create_dir(&carpeta).map_err(|e| format!("No se pudo crear la carpeta del lote de bitácoras: {e}"))?;
        carpeta
    };
    let cruzadas = match &opciones.reutilizar_base {
        Some(base) => lecturas_reutilizables(&carpetas_previas(base, Some(&carpeta)), inicio),
        None => BTreeMap::new(),
    };
    let pausa = opciones.pausa;
    let mut s = Sesion {
        conexion,
        carpeta: carpeta.clone(),
        estado: json!({
            "version": 1, "tipo": "BITACORAS_LECTURA", "estado": "EN_CURSO", "seleccion": lote_bitacoras.seleccion(),
            "fecha": inicio.format("%Y-%m-%dT%H:%M:%S%.6f%:z").to_string(), "consultas": [], "registros": [],
            "origen": {"version": super::extension::version(Edicion::Integral), "commit": "desarrollo"},
        }),
        pendiente: 0,
        aperturas: 0,
        fallas_seguidas: 0,
        cancelar,
        pausa,
        previos,
        antiguos,
        cruzadas,
        calendarios: opciones.calendarios,
        historico: opciones.historico,
        historicos: None,
        historicos_fallidos: 0,
        historico_suspendido: false,
        ahora: inicio,
        historia: match opciones.historia {
            Some(meses) => Some(super::historia::LectorHistoria::nuevo(
                meses,
                opciones.pausa,
                opciones.historia_anterior.clone(),
                *inicio.offset(),
            )?),
            None => None,
        },
        avance,
    };
    s.persistir(false)?;
    let mut bloqueado = false;
    let corrida = (|| -> Result<(), Detencion> {
        let version = s.pedir("capabilities", json!({}))?;
        super::comprobar_extension(Edicion::Integral, &version)?;
        conexion.habilitar_lecturas(true);
        s.estado["extension"] = json!({"version": version["version"], "protocolos": version["protocolos"]});
        s.persistir(false)?;
        s.pedir("lock", json!({}))?;
        bloqueado = true;
        // `_historia('abrir', …)`: la historia es opcional; su falla no afecta las bitácoras.
        let carpeta_lote = s.carpeta.clone();
        if let Some(h) = s.historia.as_mut() {
            if h.abrir(&carpeta_lote).is_err() {
                h.estado["estado"] = json!("INCOMPLETA");
            }
        }
        for (indice, perfil) in plan.iter().enumerate() {
            (s.avance)("query", json!({"index": indice + 1, "total": plan.len(), "label": perfil.etiqueta}));
            if !enumerar(&mut s, catalogo, perfil)? {
                break;
            }
        }
        Ok(())
    })();
    let completa = |e: &Value| {
        e["consultas"].as_array().is_some_and(|c| {
            c.len() == plan.len() && c.iter().all(|q| matches!(q["estado"].as_str(), Some("ENUMERADA" | "SIN_REGISTROS")))
        }) && e["registros"].as_array().is_some_and(|r| r.iter().all(|r| r["estado"] == "LEIDA"))
    };
    let cancelada = cancelar.load(Ordering::SeqCst);
    match corrida {
        Ok(()) => {
            s.estado["estado"] =
                json!(if cancelada { "CANCELADA" } else if completa(&s.estado) { "COMPLETA" } else { "INCOMPLETA" });
        }
        Err(Detencion::Lote(e)) | Err(Detencion::Consulta(e)) => {
            s.estado["estado"] = json!(if cancelada { "CANCELADA" } else { "INCOMPLETA" });
            s.estado["error"] = json!(e);
        }
    }
    let estado_lote = s.estado["estado"].as_str().unwrap_or("").to_string();
    if let Some(h) = s.historia.as_mut() {
        h.cerrar(&estado_lote, cancelar.load(Ordering::SeqCst));
    }
    // `_cerrar`
    if s.estado["estado"] == "EN_CURSO" {
        s.estado["estado"] = json!("INCOMPLETA");
    }
    if s.estado.get("anteriores_no_reenumeradas").is_none() {
        s.estado["anteriores_no_reenumeradas"] = json!([]);
    }
    let guardado = s.persistir(false).and_then(|()| {
        // Archivo independiente para soporte técnico, sin datos judiciales de NNA.
        let ruta = s.carpeta.join("diagnostico_bitacoras.json");
        let temporal = s.carpeta.join("diagnostico_bitacoras.tmp");
        serde_json::to_vec_pretty(&diagnostico_anonimo(&s.estado))
            .map_err(|e| e.to_string())
            .and_then(|b| std::fs::write(&temporal, b).map_err(|e| e.to_string()))
            .and_then(|()| std::fs::rename(&temporal, &ruta).map_err(|e| e.to_string()))
            .and_then(|()| {
                let ruta = s.carpeta.join("diagnostico_calendarios.json");
                let temporal = s.carpeta.join("diagnostico_calendarios.tmp");
                serde_json::to_vec_pretty(&diagnostico_calendarios(&s.estado))
                    .map_err(|e| e.to_string())
                    .and_then(|b| std::fs::write(&temporal, b).map_err(|e| e.to_string()))
                    .and_then(|()| std::fs::rename(&temporal, &ruta).map_err(|e| e.to_string()))
            })
    });
    if bloqueado {
        let _ = conexion.llamar("unlock", json!({}), Duration::from_secs(8));
    }
    conexion.habilitar_lecturas(false);
    guardado?;
    let registros = s.estado["registros"].as_array().cloned().unwrap_or_default();
    let consultas = s.estado["consultas"].as_array().cloned().unwrap_or_default();
    let manifiesto = carpeta.join(MANIFIESTO);
    let detalle = consultas
        .iter()
        .find(|q| q["estado"] == "FALLIDA")
        .map(|q| q["error"].as_str().unwrap_or("").to_string())
        .unwrap_or_else(|| s.estado["error"].as_str().unwrap_or("").to_string());
    let mut resultado = json!({
        "folder": carpeta.display().to_string(),
        "manifest": manifiesto.display().to_string(),
        "sha256": sha256(&std::fs::read(&manifiesto).map_err(|e| e.to_string())?),
        "estado": s.estado["estado"],
        "leidas": registros.iter().filter(|r| r["estado"] == "LEIDA").count(),
        "fallidas": registros.iter().filter(|r| r["estado"] != "LEIDA").count(),
        "consultas_fallidas": consultas.iter().filter(|q| q["estado"] == "FALLIDA").count(),
        "calendarios_leidos": registros.iter().filter(|r| r["calendario_lectura"]["estado"] == "LEIDA").count(),
        "calendarios_fallidos": registros
            .iter()
            .filter(|r| matches!(r["calendario_lectura"]["estado"].as_str(), Some("FALLIDA" | "PENDIENTE")))
            .count(),
        "detalle": detalle,
        "faltantes_por_tribunal": faltantes_por_tribunal(&registros),
        "historicos": conteo_historicos(&registros),
        "diagnostico_anonimo": carpeta.join("diagnostico_bitacoras.json").display().to_string(),
    });
    // `export_diaries`: copia íntegra en `Bitacoras_RUS.xlsx` (o con sufijo si existe); un fallo no anula la lectura.
    let destino = super::excel::nombre_libre(&carpeta, "Bitacoras_RUS");
    match super::excel::exportar_bitacoras(&[manifiesto], &destino) {
        Ok(r) => resultado["excel"] = json!(r.display().to_string()),
        Err(e) => resultado["excel_error"] = json!(e),
    }
    Ok(resultado)
}

/// `enumerar_listado`: formulario, páginas del listado y lectura de cada fila. `Ok(false)` detiene el lote.
fn enumerar(s: &mut Sesion<'_>, catalogo: &Catalogo, perfil: &Perfil) -> Result<bool, Detencion> {
    s.detener_si_cancelada()?;
    let Alcance::Seguimiento { pestana, modalidad, reporte } = &perfil.alcance else {
        return Err(Detencion::Lote("Las bitácoras se consultan desde Seguimiento.".into()));
    };
    let mut consulta_json = lote::seleccion_edicion(perfil, Edicion::Integral);
    consulta_json["tribunal_nombre"] = json!(catalogo.tribunales.get(&perfil.tribunal).cloned().unwrap_or_default());
    consulta_json["modalidad_nombre"] =
        json!(MODALIDADES.iter().find(|(c, _)| c == modalidad).map_or("", |(_, n)| n));
    consulta_json["estado"] = json!("EN_CURSO");
    consulta_json["paginas"] = json!(0);
    consulta_json["registros"] = json!(0);
    s.estado["consultas"].as_array_mut().unwrap().push(consulta_json);
    let q = s.estado["consultas"].as_array().unwrap().len() - 1;
    s.persistir(false)?;
    let _ = reporte;
    let resultado = (|| -> Result<(), Detencion> {
        let consulta = Consulta::desde(perfil)?;
        let preparado = s.pedir(
            "prepare",
            json!({"selection": lote::seleccion_edicion(perfil, Edicion::Integral), "dates": null, "keep_filters": false}),
        )?;
        let pares: Pares = preparado["pairs"]
            .as_array()
            .filter(|a| a.len() <= 100)
            .and_then(|a| {
                a.iter()
                    .map(|par| match par.as_array().map(Vec::as_slice) {
                        Some([Value::String(k), Value::String(v)]) => Some((k.clone(), v.clone())),
                        _ => None,
                    })
                    .collect()
            })
            .ok_or("El formulario no entregó parámetros válidos.".to_string())?;
        consulta.validar_consulta(&pares)?;
        if pares.iter().find(|(k, _)| k == "FLG_Consulta").map(|(_, v)| v.as_str()) != Some("0") {
            return Err(Detencion::Consulta("El filtro de fechas no coincide.".into()));
        }
        let html = lote::cuerpo(&s.pedir("search", json!({"pairs": pares, "timeout": ESPERA_MS}))?, false)?;
        let primera = match sitfa::validar_respuesta(&html, &pares, &consulta, None)? {
            Respuesta::SinRegistros => {
                s.estado["consultas"][q]["estado"] = json!("SIN_REGISTROS");
                s.persistir(false)?;
                return Ok(());
            }
            Respuesta::ConRegistros(p) => p,
        };
        if primera.actual != 1 {
            return Err(Detencion::Consulta("La consulta no comenzó en la primera página.".into()));
        }
        let mut vistos = BTreeSet::new();
        let (mut actuales, mut pagina, mut cuerpo_html) = (pares.clone(), primera.clone(), html);
        for numero in 1..=primera.total {
            s.detener_si_cancelada()?;
            if numero > 1 {
                actuales = sitfa::siguiente_consulta(&actuales, &consulta, &pagina, numero)?;
                cuerpo_html = lote::cuerpo(&s.pedir("search", json!({"pairs": actuales, "timeout": ESPERA_MS}))?, false)?;
                pagina = match sitfa::validar_respuesta(&cuerpo_html, &actuales, &consulta, Some((numero, &primera)))? {
                    Respuesta::ConRegistros(p) => p,
                    Respuesta::SinRegistros => {
                        return Err(Detencion::Consulta(
                            "No se pudieron identificar todos los registros de la pagina.".into(),
                        ))
                    }
                };
            }
            let seleccion = vinculos::Seleccion {
                tribunal: perfil.tribunal.clone(),
                screen: "seguimiento".into(),
                tab: pestana.clone(),
            };
            let filas = vinculos::extraer_filas(&cuerpo_html, &seleccion)?;
            // `listing_rows`: el control de filas debe coincidir con el listado validado.
            let mut control = sitfa::Registros::new();
            for v in &filas {
                let clave = super::sitfa::registros::clave(&v.rit, &v.nombre)?;
                *control.entry(clave).or_insert(0) += 1;
            }
            if control != pagina.registros {
                return Err(Detencion::Consulta("El control de filas no coincide con el listado validado.".into()));
            }
            for v in &filas {
                leer_fila(s, perfil, pestana, modalidad, q, numero, v, &mut vistos)?;
            }
            let (conexion, cancelar) = (s.conexion, s.cancelar);
            let tribunal_nombre = s.estado["consultas"][q]["tribunal_nombre"].as_str().unwrap_or("").to_string();
            if let Some(h) = s.historia.as_mut() {
                h.pagina(conexion, cancelar, &perfil.tribunal, pestana, modalidad, &tribunal_nombre, numero, &cuerpo_html);
            }
            s.estado["consultas"][q]["paginas"] = json!(numero);
            s.persistir(false)?;
        }
        s.estado["consultas"][q]["estado"] = json!("ENUMERADA");
        s.persistir(false)?;
        Ok(())
    })();
    match resultado {
        Ok(()) => Ok(true),
        Err(Detencion::Lote(e)) => {
            s.estado["consultas"][q]["estado"] = json!("INCOMPLETA");
            Err(Detencion::Lote(e))
        }
        Err(Detencion::Consulta(e)) => {
            s.estado["consultas"][q]["estado"] = json!("FALLIDA");
            s.estado["consultas"][q]["error"] = json!(e);
            s.persistir(false)?;
            Ok(!s.cancelar.load(Ordering::SeqCst))
        }
    }
}

/// `leer_bitacora`: identidad, lectura de la bitácora de la fila y pausa.
#[allow(clippy::too_many_arguments)]
fn leer_fila(
    s: &mut Sesion<'_>,
    perfil: &Perfil,
    pestana: &str,
    modalidad: &str,
    q: usize,
    numero: usize,
    v: &vinculos::Vinculo,
    vistos: &mut BTreeSet<(String, String, String)>,
) -> Result<(), Detencion> {
    s.detener_si_cancelada()?;
    let consulta = &s.estado["consultas"][q];
    let (tribunal_nombre, modalidad_nombre) = (consulta["tribunal_nombre"].clone(), consulta["modalidad_nombre"].clone());
    let mut calendario_abierto = false;
    let mut r = if v.bitacora_verificada {
        v.bitacora_python()
    } else {
        json!({"rit": v.rit, "nombre": v.nombre, "programa": v.programa, "rut": v.rut,
            "vinculo_diagnostico": v.diagnostico_bitacora})
    };
    r["fila"] = json!(v.fila);
    r["tribunal_codigo"] = json!(perfil.tribunal);
    r["tribunal_nombre"] = tribunal_nombre;
    r["modalidad"] = json!(modalidad);
    r["modalidad_nombre"] = modalidad_nombre;
    r["pestana"] = json!(pestana);
    r["pagina"] = json!(numero);
    r["estado"] = json!("PENDIENTE");
    r["error"] = json!("");
    let campo = |r: &Value, k: &str| r[k].as_str().unwrap_or("").to_string();
    let identidad = (perfil.tribunal.clone(), campo(&r, "causa_id"), campo(&r, "ingreso_id"));
    let completa = !identidad.0.is_empty() && !identidad.1.is_empty() && !identidad.2.is_empty();
    if completa && !vistos.insert(identidad.clone()) {
        return Err(Detencion::Consulta("Un ingreso está repetido en el listado; se detuvo esta consulta.".into()));
    }
    let base = json!([pestana, modalidad, [identidad.0, identidad.1, identidad.2], r["rit"], r["nombre"], r["programa"],
        r["rut"], if completa { json!("") } else { r["fila"].clone() }]);
    r["clave"] = json!(sha256(json_python(&base).as_bytes()));
    s.estado["registros"].as_array_mut().unwrap().push(r);
    let i = s.estado["registros"].as_array().unwrap().len() - 1;
    let registros = s.estado["consultas"][q]["registros"].as_u64().unwrap_or(0) + 1;
    s.estado["consultas"][q]["registros"] = json!(registros);
    if !completa {
        s.estado["registros"][i]["estado"] = json!("SIN_VINCULO");
        s.estado["registros"][i]["error"] = json!(format!(
            "La fila no tiene un enlace de bitácora inequívoco. No se construyó una consulta por RIT. Diagnóstico: {}",
            s.estado["registros"][i]["vinculo_diagnostico"].as_str().unwrap_or("NO_VERIFICADO")
        ));
    } else {
        let params = json!({"tipo_popUp": campo(&s.estado["registros"][i], "tipo_popup"), "CRR_IdCausa": identidad.1,
            "COD_Tribunal": perfil.tribunal, "TIP_Consulta": modalidad, "ID_Ingreso": identidad.2,
            "COD_Etapa": campo(&s.estado["registros"][i], "etapa"), "FLG_MejorNinez": campo(&s.estado["registros"][i], "antiguo")});
        s.estado["registros"][i]["params"] = params.clone();
        let reutilizada = match reutilizar_retomada(s, i, &params) {
            Ok(true) => Ok(true),
            Ok(false) => Ok(reutilizar_de_otro_lote(s, i, &params)?),
            Err(e) => Err(e),
        };
        let apertura = match reutilizada {
            Ok(true) => Ok(false),
            Ok(false) => abrir(s, i, &params).map(|()| true),
            Err(e) => Err(e),
        }
        .and_then(|abierta| {
            let con_calendario = s.calendarios
                && PESTANAS_CALENDARIO.contains(&pestana)
                && s.estado["registros"][i]["calendario"].is_array();
            if con_calendario {
                calendario_abierto = leer_calendario(s, i, abierta)?;
            }
            if s.historico {
                calendario_abierto = leer_historico(s, i)? || calendario_abierto;
            }
            Ok(())
        });
        match apertura {
            Ok(()) => {}
            Err(Detencion::Lote(e)) => {
                if s.historico && s.estado["registros"][i].get("historico_lectura").is_none() {
                    s.estado["registros"][i]["historico_lectura"] = json!({"estado": "PENDIENTE", "error": "HISTORICO_TOPE"});
                }
                if s.estado["registros"][i]["estado"] != "LEIDA" {
                    s.estado["registros"][i]["estado"] = json!("PENDIENTE");
                    s.estado["registros"][i]["error"] = json!("Pendiente: se alcanzó el máximo de lecturas del lote.");
                }
                return Err(Detencion::Lote(e));
            }
            Err(Detencion::Consulta(e)) => {
                s.estado["registros"][i]["estado"] = json!("FALLIDA");
                s.estado["registros"][i]["error"] = json!(e);
            }
        }
    }
    if s.historico && s.estado["registros"][i].get("historico_lectura").is_none() {
        s.estado["registros"][i]["historico_lectura"] = json!({"estado": "OMITIDA", "error": "HISTORICO_SIN_VINCULO"});
    }
    let estado = s.estado["registros"][i]["estado"].as_str().unwrap_or("").to_string();
    if estado == "FALLIDA" {
        s.fallas_seguidas += 1;
    } else if estado == "LEIDA" {
        s.fallas_seguidas = 0;
    }
    s.persistir(true)?;
    let r = &s.estado["registros"][i];
    let archivo = r["archivo"].as_str().filter(|a| !a.is_empty()).map(str::to_string).unwrap_or_else(|| estado.clone());
    (s.avance)("diary_progress", json!({"pagina": r["pagina"], "registros": 1, "archivo": archivo, "rit": r["rit"], "estado": estado}));
    if s.fallas_seguidas >= MAX_FALLAS_SEGUIDAS {
        return Err(Detencion::Lote(format!(
            "{MAX_FALLAS_SEGUIDAS} bitácoras seguidas fallaron ({}). Se detuvo el lote para no esperar en cada causa. \
             Revisa la sesión SITFA y usa «Retomar paquete…»: no se vuelve a leer lo ya leído.",
            s.estado["registros"][i]["error"].as_str().unwrap_or("")
        )));
    }
    let reutilizada = s.estado["registros"][i]["reutilizada"] == true;
    let espera =
        if estado != "SIN_VINCULO" && (!reutilizada || calendario_abierto) { s.pausa } else { Duration::from_millis(50) };
    let fin = std::time::Instant::now() + espera;
    while std::time::Instant::now() < fin {
        if s.cancelar.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep((fin - std::time::Instant::now()).min(Duration::from_millis(20)));
    }
    s.detener_si_cancelada()
}

/// `lectura_bitacoras.load`: manifiesto de un lote de lectura de bitácoras.
fn cargar(ruta: &Path) -> Result<Value, String> {
    let tamano = std::fs::metadata(ruta).map_err(|_| "El archivo no es un lote de lectura de bitácoras.".to_string())?.len();
    if tamano > 32 * 1024 * 1024 {
        return Err("El lote excede el tamaño permitido.".into());
    }
    let datos: Value = serde_json::from_slice(&std::fs::read(ruta).map_err(|e| e.to_string())?)
        .map_err(|_| "El archivo no es un lote de lectura de bitácoras.".to_string())?;
    if datos["version"] != 1 || datos["tipo"] != "BITACORAS_LECTURA" {
        return Err("El archivo no es un lote de lectura de bitácoras.".into());
    }
    Ok(datos)
}

/// `_estructura_reutilizable`: ante cualquier duda sobre los tipos, se relee en SITFA.
fn estructura_reutilizable(r: &Value) -> bool {
    let nombre = r["archivo"].as_str().unwrap_or("");
    let huella = r["sha256"].as_str().unwrap_or("");
    nombre.ends_with(".html")
        && !nombre.contains(['/', '\\'])
        && huella.len() == 64
        && huella.chars().all(|c| "0123456789abcdef".contains(c))
        && r["entradas"].is_u64()
        && r["params"].is_object()
        && r["capturada"].is_string()
}

/// `carpetas_previas`: carpetas «Bitacoras *» (y «*/respaldo/Bitacoras *») del destino, más recientes primero.
pub fn carpetas_previas(base: &Path, actual: Option<&Path>) -> Vec<PathBuf> {
    let Ok(raiz) = base.canonicalize() else { return Vec::new() };
    let actual = actual.and_then(|a| a.canonicalize().ok());
    let mut candidatas: Vec<PathBuf> = Vec::new();
    let es_lote = |p: &Path| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("Bitacoras "));
    if let Ok(entradas) = std::fs::read_dir(&raiz) {
        for e in entradas.flatten() {
            let p = e.path();
            if es_lote(&p) {
                candidatas.push(p.clone());
            }
            if let Ok(respaldo) = std::fs::read_dir(p.join("respaldo")) {
                candidatas.extend(respaldo.flatten().map(|r| r.path()).filter(|r| es_lote(r)));
            }
        }
    }
    let mut halladas: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    for c in candidatas {
        let manifiesto = c.join(MANIFIESTO);
        let enlace = std::fs::symlink_metadata(&c).map(|m| m.file_type().is_symlink()).unwrap_or(true);
        if enlace || !c.is_dir() || !manifiesto.is_file() {
            continue;
        }
        let Ok(resuelta) = c.canonicalize() else { continue };
        if !resuelta.starts_with(&raiz) || actual.as_ref() == Some(&resuelta) {
            continue;
        }
        if let Ok(fecha) = std::fs::metadata(&manifiesto).and_then(|m| m.modified()) {
            if !halladas.iter().any(|(p, _)| p == &resuelta) {
                halladas.push((resuelta, fecha));
            }
        }
    }
    halladas.sort_by_key(|h| std::cmp::Reverse(h.1));
    halladas.into_iter().take(REUTILIZAR_CARPETAS).map(|(p, _)| p).collect()
}

/// `lecturas_reutilizables`: lecturas LEIDA de menos de 12 h y del mismo día local; gana la captura más reciente.
pub fn lecturas_reutilizables(
    carpetas: &[PathBuf],
    ahora: chrono::DateTime<chrono::FixedOffset>,
) -> BTreeMap<String, (PathBuf, Value)> {
    let mut mejor: BTreeMap<String, (PathBuf, Value, chrono::DateTime<chrono::FixedOffset>)> = BTreeMap::new();
    for carpeta in carpetas {
        let Ok(datos) = cargar(&carpeta.join(MANIFIESTO)) else { continue };
        let registros = ["registros", "anteriores_no_reenumeradas"]
            .iter()
            .flat_map(|k| datos[*k].as_array().cloned().unwrap_or_default());
        for r in registros {
            if r["estado"] != "LEIDA" || !estructura_reutilizable(&r) {
                continue;
            }
            let clave = r["clave"].as_str().unwrap_or("").to_string();
            if clave.is_empty() {
                continue;
            }
            let Ok(capturada) = chrono::DateTime::parse_from_rfc3339(&r["capturada"].as_str().unwrap_or("").replace('Z', "+00:00"))
            else {
                continue;
            };
            let edad = ahora.signed_duration_since(capturada);
            let mismo_dia = capturada.with_timezone(ahora.offset()).date_naive() == ahora.date_naive();
            if edad < chrono::Duration::zero() || edad > chrono::Duration::hours(REUTILIZAR_HORAS) || !mismo_dia {
                continue;
            }
            if mejor.get(&clave).is_none_or(|(_, _, previa)| capturada > *previa) {
                mejor.insert(clave, (carpeta.clone(), r, capturada));
            }
        }
    }
    mejor.into_iter().map(|(k, (c, r, _))| (k, (c, r))).collect()
}

/// Instantánea local de una lectura anterior: archivo regular, ≤ 8 MiB y con su huella.
fn instantanea(ruta: &Path, huella: &str) -> Option<Vec<u8>> {
    let meta = std::fs::symlink_metadata(ruta).ok()?;
    if !meta.is_file() || meta.len() > 8 * 1024 * 1024 {
        return None;
    }
    let datos = std::fs::read(ruta).ok()?;
    (sha256(&datos) == huella).then_some(datos)
}

fn copiar_lectura(s: &mut Sesion<'_>, i: usize, anterior: &Value, leida: &Value) {
    let r = &s.estado["registros"][i];
    let avisos = advertencias(leida, r["rit"].as_str().unwrap_or(""), r["nombre"].as_str().unwrap_or(""));
    let r = &mut s.estado["registros"][i];
    r["advertencias"] = json!(avisos);
    r["error"] = json!(avisos.join(" "));
    for k in ["archivo", "sha256", "capturada", "entradas"] {
        r[k] = anterior[k].clone();
    }
    r["estado"] = json!("LEIDA");
    r["reutilizada"] = json!(true);
}

/// `_reutilizar_retomada`: la lectura del lote retomado, si su archivo conserva la huella.
fn reutilizar_retomada(s: &mut Sesion<'_>, i: usize, params: &Value) -> Result<bool, Detencion> {
    let clave = s.estado["registros"][i]["clave"].as_str().unwrap_or("").to_string();
    let Some(anterior) = s.antiguos.get(&clave).cloned() else { return Ok(false) };
    if &anterior["params"] != params || !estructura_reutilizable(&anterior) {
        return Ok(false);
    }
    let ruta = s.carpeta.join(anterior["archivo"].as_str().unwrap_or(""));
    let Some(datos) = instantanea(&ruta, anterior["sha256"].as_str().unwrap_or("")) else { return Ok(false) };
    let parametros: BTreeMap<String, String> = serde_json::from_value(params.clone()).map_err(|e| e.to_string())?;
    let leida = lecturas::leer_bitacora(&datos, &parametros, "", "")?;
    copiar_lectura(s, i, &anterior, &leida);
    Ok(true)
}

/// `_reutilizar_de_otro_lote`: copia al lote actual una lectura reciente de otro lote (identidad, parámetros y huella).
fn reutilizar_de_otro_lote(s: &mut Sesion<'_>, i: usize, params: &Value) -> Result<bool, Detencion> {
    let clave = s.estado["registros"][i]["clave"].as_str().unwrap_or("").to_string();
    let Some((carpeta, anterior)) = s.cruzadas.get(&clave).cloned() else { return Ok(false) };
    if &anterior["params"] != params || !estructura_reutilizable(&anterior) {
        return Ok(false);
    }
    let nombre = anterior["archivo"].as_str().unwrap_or("").to_string();
    let Some(datos) = instantanea(&carpeta.join(&nombre), anterior["sha256"].as_str().unwrap_or("")) else {
        return Ok(false);
    };
    let Ok(parametros) = serde_json::from_value::<BTreeMap<String, String>>(params.clone()) else { return Ok(false) };
    // Una copia ajena ilegible se resuelve leyendo en SITFA, no como FALLIDA (E3 #4).
    let Ok(leida) = lecturas::leer_bitacora(&datos, &parametros, "", "") else { return Ok(false) };
    let destino = s.carpeta.join(&nombre);
    if destino.exists() {
        if std::fs::read(&destino).map_err(|e| e.to_string())? != datos {
            return Err(Detencion::Consulta("El archivo local de bitácora cambió.".into()));
        }
    } else {
        lote::guardar_exclusivo(&destino, &datos)?;
    }
    copiar_lectura(s, i, &anterior, &leida);
    s.estado["registros"][i]["reutilizada_de"] = json!({
        "carpeta": carpeta.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "capturada": anterior["capturada"],
    });
    Ok(true)
}

/// `Counter` de los estados del histórico de los registros que lo tienen.
fn conteo_historicos(registros: &[Value]) -> Value {
    let mut c: BTreeMap<String, usize> = BTreeMap::new();
    for r in registros.iter().filter_map(|r| r["historico_lectura"]["estado"].as_str()) {
        *c.entry(r.to_string()).or_default() += 1;
    }
    json!(c)
}

/// `anonymous_diagnostic`: resumen apto para compartir; nunca reproduce filas, identificadores ni HTML.
pub fn diagnostico_anonimo(datos: &Value) -> Value {
    const ESTADOS: [&str; 4] = ["LEIDA", "FALLIDA", "SIN_VINCULO", "PENDIENTE"];
    const VINCULOS: [&str; 3] =
        ["SIN_ACCION_OBSERVACIONES", "MULTIPLES_ACCIONES_OBSERVACIONES", "ACCION_OBSERVACIONES_NO_VERIFICADA"];
    const EXTENSION: [&str; 6] = ["DIARY_CONTEXT", "DIARY_LINK", "DIARY_IDENTITY", "DIARY_NETWORK", "DIARY_TIMEOUT", "DIARY_RESPONSE"];
    static VERSION: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^[0-9]{1,3}(?:\.[0-9]{1,3}){2}(?:\.dev[0-9]{1,5})?$").unwrap());
    static COMMIT: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^[0-9a-fA-F]{40}$").unwrap());
    let version = |v: &Value| {
        let t = v.as_str().unwrap_or("");
        if VERSION.is_match(t) { t.to_string() } else { "NO_VERIFICADA".into() }
    };
    let commit = |v: &Value| {
        let t = v.as_str().unwrap_or("");
        if COMMIT.is_match(t) { t.to_lowercase() } else { "NO_VERIFICADO".into() }
    };
    let estado = |v: &Value| v.as_str().filter(|e| ESTADOS.contains(e)).unwrap_or("OTRO").to_string();
    let contar = |it: &mut dyn Iterator<Item = String>| {
        let mut c: BTreeMap<String, u64> = BTreeMap::new();
        for k in it {
            *c.entry(k).or_default() += 1;
        }
        json!(c)
    };
    let registros = datos["registros"].as_array().cloned().unwrap_or_default();
    let estados = contar(&mut registros.iter().map(|r| estado(&r["estado"])));
    let vinculos = contar(&mut registros.iter().filter(|r| r["estado"] == "SIN_VINCULO").map(|r| {
        r["vinculo_diagnostico"].as_str().filter(|v| VINCULOS.contains(v)).unwrap_or("NO_ESPECIFICADO").to_string()
    }));
    let extension = contar(&mut registros.iter().filter(|r| r["estado"] == "FALLIDA").map(|r| {
        r["diagnostico"]["codigo_extension"].as_str().filter(|c| EXTENSION.contains(c)).unwrap_or("OTRA_FALLA").to_string()
    }));
    let consultas: Vec<Value> = datos["consultas"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|q| {
            let (t, m, p) = (q["tribunal"].as_str().unwrap_or(""), q["modality"].as_str().unwrap_or(""), q["tab"].as_str().unwrap_or(""));
            let subconjunto: Vec<&Value> = registros
                .iter()
                .filter(|r| r["tribunal_codigo"] == t && r["modalidad"] == m && r["pestana"] == p)
                .collect();
            let estado_consulta = q["estado"].as_str().unwrap_or("");
            json!({
                "tribunal_codigo": if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) { t } else { "NO_VERIFICADO" },
                "modalidad_codigo": if ["1", "2", "3", "4"].contains(&m) { m } else { "NO_VERIFICADA" },
                "pestana": if ["Espera", "Cumplimiento", "Informes", "Egresados"].contains(&p) { p } else { "NO_VERIFICADA" },
                "estado_consulta": if ["ENUMERADA", "SIN_REGISTROS", "FALLIDA", "EN_CURSO"].contains(&estado_consulta) {
                    estado_consulta
                } else {
                    "OTRO"
                },
                "paginas": q["paginas"].as_u64().unwrap_or(0),
                "registros": subconjunto.len(),
                "estados": contar(&mut subconjunto.iter().map(|r| estado(&r["estado"]))),
            })
        })
        .collect();
    let lote = datos["estado"].as_str().filter(|e| ["COMPLETA", "INCOMPLETA", "CANCELADA", "EN_CURSO"].contains(e));
    json!({
        "tipo": "BITACORAS_DIAGNOSTICO_ANONIMO", "version": 1, "estado_lote": lote.unwrap_or("OTRO"),
        "descargador_version": version(&datos["origen"]["version"]), "descargador_commit": commit(&datos["origen"]["commit"]),
        "extension_version": version(&datos["extension"]["version"]), "registros": registros.len(), "estados": estados,
        "diagnosticos_sin_vinculo": vinculos, "errores_extension": extension, "consultas": consultas,
        "anteriores_no_reenumeradas": datos["anteriores_no_reenumeradas"].as_array().map_or(0, Vec::len),
        "contiene_identificadores_personales": false,
    })
}

/// Clasifica el detalle de lectura, incluidos los controles guardados con versiones anteriores.
pub(super) fn codigo_calendario(error: &str) -> &'static str {
    const CODIGOS_CALENDARIO: &[(&str, &str)] = &[
        ("Fecha Proyección", "CAL_FECHA_PROYECCION"),
        ("fecha ilegible", "CAL_FECHA_ILEGIBLE"),
        ("no concuerdan", "CAL_ESTADO_FECHA"),
        ("Estado de calendario no reconocido", "CAL_ESTADO"),
        ("estructura del calendario", "CAL_ESTRUCTURA"),
        ("Fila de calendario incompleta", "CAL_FILA"),
        ("máximo de lecturas", "CAL_TOPE"),
        ("no corresponde al ingreso", "CAL_OTRO_INGRESO"),
        ("incompleta", "CAL_INCOMPLETA"),
        ("ingreso de SITFA", "CAL_SESION"),
        ("no coincide con la solicitud", "CAL_RESPUESTA"),
        ("formulario del calendario", "CAL_FORMULARIO"),
        ("Campo ausente o repetido", "CAL_CAMPO"),
        ("enlace del calendario", "CAL_ENLACE"),
    ];
    CODIGOS_CALENDARIO.iter().find(|(fragmento, _)| error.contains(fragmento)).map_or("CAL_OTRA", |(_, codigo)| *codigo)
}

/// Extensión de Flow Rust: estados y códigos fijos de error, para compartir sin identificadores ni texto libre.
pub fn diagnostico_calendarios(datos: &Value) -> Value {
    let registros = datos["registros"].as_array().cloned().unwrap_or_default();
    let contar = |it: &mut dyn Iterator<Item = String>| {
        let mut c: BTreeMap<String, u64> = BTreeMap::new();
        for k in it {
            *c.entry(k).or_default() += 1;
        }
        json!(c)
    };
    let estado_lectura = |v: &Value| ["LEIDA", "FALLIDA", "PENDIENTE"].iter().find(|e| v == **e).map_or("OTRO", |e| *e).to_string();
    let calendarios = contar(&mut registros.iter().filter(|r| r["calendario"].is_array()).map(|r| {
        if r["calendario_lectura"].is_object() { estado_lectura(&r["calendario_lectura"]["estado"]) } else { "NO_INTENTADO".into() }
    }));
    let errores_calendario = contar(&mut registros.iter().filter(|r| matches!(r["calendario_lectura"]["estado"].as_str(), Some("FALLIDA" | "PENDIENTE"))).map(|r| {
        let texto = r["calendario_lectura"]["error"].as_str().unwrap_or("");
        codigo_calendario(texto).to_string()
    }));
    let historicos = contar(&mut registros.iter().filter(|r| r["historico_lectura"].is_object()).map(|r| estado_lectura(&r["historico_lectura"]["estado"])));
    json!({
        "tipo": "CALENDARIOS_DIAGNOSTICO_ANONIMO", "version": 1, "registros": registros.len(), "calendarios": calendarios,
        "errores_calendario": errores_calendario, "historicos": historicos, "contiene_identificadores_personales": false,
    })
}

/// `persona_por_rut`: clave de persona cuando la fila no trae `SITFA_PERSONA_RUS`. Es una huella, no el RUT.
pub fn persona_por_rut(rut: &str) -> String {
    format!("rut-{}", &sha256(rut.as_bytes())[..20])
}

/// `_leer_historico`: el Ingreso Histórico Protección de la persona de la fila. Una falla solo deja el histórico
/// FALLIDO; nunca invalida la bitácora ya leída. Devuelve si se abrió una ventana.
fn leer_historico(s: &mut Sesion<'_>, i: usize) -> Result<bool, Detencion> {
    static RUT: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^[0-9]{6,9}[0-9K]$").unwrap());
    let r = s.estado["registros"][i].clone();
    let rut = lecturas::rut_canonico(r["rut"].as_str().unwrap_or(""));
    if !RUT.is_match(&rut) {
        s.estado["registros"][i]["historico_lectura"] = json!({"estado": "FALLIDA", "error": "HISTORICO_RUT"});
        return Ok(false);
    }
    let huella = persona_por_rut(&rut);
    let person_id = r["persona_id"].as_str().filter(|p| !p.is_empty()).map(str::to_string).unwrap_or_else(|| huella.clone());
    let campos = json!({"tribunal": r["tribunal_codigo"], "tribunal_nombre": r["tribunal_nombre"].as_str().unwrap_or(""),
        "person_id": person_id});
    let persona = json!({"tribunal": r["tribunal_codigo"], "tribunal_nombre": r["tribunal_nombre"].as_str().unwrap_or(""),
        "person_id": person_id, "rut": rut});
    let ahora = chrono::Local::now().fixed_offset().format("%Y-%m-%dT%H:%M:%S%.6f%:z").to_string();
    let mut base = campos.clone();
    for (k, v) in [("origen", json!("AUTOMATICA")), ("rut_hash", json!(huella)), ("leida", json!(ahora)),
        ("ownership", json!("INCIERTA")), ("filas", json!([]))]
    {
        base[k] = v;
    }
    let mut abierto = false;
    let carpeta = s.carpeta.clone();
    let resultado = (|| -> Result<Value, Detencion> {
        if s.historicos.is_none() {
            let previas = super::historico::cargar(&carpeta, &s.ahora.to_rfc3339())?;
            s.historicos = Some(
                previas
                    .into_iter()
                    .filter(|e| {
                        e["rut_hash"].as_str().is_some_and(|h| !h.is_empty())
                            && e["ownership"] == "VERIFICADA"
                            && matches!(e["estado"].as_str(), Some("LEIDA" | "VACIA_COMPROBADA"))
                    })
                    .map(|e| (e["rut_hash"].as_str().unwrap().to_string(), e))
                    .collect(),
            );
        }
        let previa = s.historicos.as_ref().and_then(|h| h.get(&huella)).cloned();
        let mut entrada = if let Some(mut p) = previa {
            for (k, v) in campos.as_object().unwrap() {
                p[k] = v.clone();
            }
            p
        } else if s.historico_suspendido {
            let mut e = base.clone();
            e["estado"] = json!("OMITIDA");
            e["error"] = json!("HISTORICO_SUSPENDIDO");
            e
        } else {
            if s.aperturas >= MAX_LECTURAS_POR_LOTE {
                s.estado["registros"][i]["historico_lectura"] = json!({"estado": "PENDIENTE", "error": "HISTORICO_TOPE"});
                return Err(Detencion::Lote(format!("Se alcanzó el máximo de {MAX_LECTURAS_POR_LOTE} lecturas por lote.")));
            }
            if !s.pausa.is_zero() {
                let fin = std::time::Instant::now() + s.pausa;
                while std::time::Instant::now() < fin && !s.cancelar.load(Ordering::SeqCst) {
                    std::thread::sleep((fin - std::time::Instant::now()).min(Duration::from_millis(20)));
                }
            }
            s.detener_si_cancelada()?;
            s.aperturas += 1;
            abierto = true;
            let params = json!({"tribunal": persona["tribunal"], "rut": rut});
            let respuesta = s.pedir("historico_open", json!({"params": params, "timeout": ESPERA_MS}))?;
            let datos = lote::cuerpo(&respuesta, false)?;
            if datos.len() > super::historico::MAX_BYTES
                || respuesta["digest"].as_str() != Some(sha256(&datos).as_str())
                || respuesta["params"] != params
            {
                return Err(Detencion::Consulta("HISTORICO_RESPONSE".into()));
            }
            let capturada = respuesta["startedAt"].as_str().unwrap_or("").to_string();
            let e = super::historico::capturar(&carpeta, &datos, &persona, &capturada, &huella)?;
            if e["estado"] == "FALLIDA" {
                super::historico::guardar_estructura(&carpeta, &datos);
            }
            return Ok(e);
        };
        super::historico::guardar_entrada(&carpeta, &mut entrada)?;
        Ok(entrada)
    })();
    let entrada = match resultado {
        Ok(e) => e,
        Err(Detencion::Lote(e)) => return Err(Detencion::Lote(e)),
        Err(Detencion::Consulta(e)) => {
            // El puente de Rust no trae el `code` de la extensión: «HISTORICO_RESPONSE» o respaldo inválido, como Python
            // con una excepción sin código.
            let codigo = if e == "HISTORICO_RESPONSE" { e } else { "HISTORICO_RESPALDO_INVALIDO".to_string() };
            let mut entrada = base.clone();
            entrada["estado"] = json!("FALLIDA");
            entrada["error"] = json!(codigo);
            let _ = super::historico::guardar_entrada(&carpeta, &mut entrada);
            entrada
        }
    };
    s.estado["registros"][i]["historico_lectura"] = json!({
        "estado": entrada["estado"], "person_id": person_id, "ownership": entrada["ownership"],
        "error": entrada["error"].as_str().unwrap_or(""), "programas": entrada["filas"].as_array().map_or(0, Vec::len),
    });
    if abierto {
        if entrada["estado"] == "FALLIDA" || entrada["ownership"] != "VERIFICADA" {
            s.historicos_fallidos += 1;
            if s.historicos_fallidos >= MAX_HISTORICOS_FALLIDOS_SEGUIDOS {
                s.historico_suspendido = true;
                (s.avance)("warning", json!(format!("Falló el Ingreso Histórico en {MAX_HISTORICOS_FALLIDOS_SEGUIDOS} ventanas seguidas. \
                    Ahora se suspende el histórico. Las bitácoras siguen.")));
            }
        } else {
            s.historicos_fallidos = 0;
        }
        if let Some(h) = s.historicos.as_mut() {
            h.insert(huella, entrada);
        }
    }
    Ok(abierto)
}

/// `reutilizar_calendario`: al retomar, el calendario ya leído del mismo ingreso si su archivo conserva la huella.
fn reutilizar_calendario(s: &mut Sesion<'_>, i: usize, args: &Value) -> bool {
    let clave = s.estado["registros"][i]["clave"].as_str().unwrap_or("").to_string();
    let anterior = s.antiguos.get(&clave).map(|r| r["calendario_lectura"].clone()).unwrap_or(Value::Null);
    if anterior["estado"] != "LEIDA" || &anterior["args"] != args {
        return false;
    }
    let nombre = anterior["archivo"].as_str().unwrap_or("").to_string();
    static NOMBRE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^calendario-[0-9a-f]{24}-[0-9a-f]{16}\.html$").unwrap());
    if !NOMBRE.is_match(&nombre) {
        return false;
    }
    let Some(datos) = instantanea(&s.carpeta.join(&nombre), anterior["sha256"].as_str().unwrap_or("")) else {
        return false;
    };
    let Ok(argumentos) = serde_json::from_value::<Vec<String>>(args.clone()) else { return false };
    let Ok(leido) = lecturas::leer_calendario(&datos, &argumentos, "", "") else { return false };
    let lectura = &mut s.estado["registros"][i]["calendario_lectura"];
    lectura["estado"] = json!("LEIDA");
    lectura["archivo"] = json!(nombre);
    lectura["sha256"] = anterior["sha256"].clone();
    lectura["capturada"] = json!(anterior["capturada"].as_str().unwrap_or(""));
    lectura["proyecciones"] = leido["proyecciones"].clone();
    lectura["reutilizada"] = json!(true);
    true
}

/// `_leer_calendario`: el calendario del ingreso tras su bitácora; una falla aquí no invalida la bitácora leída.
/// Devuelve si se abrió una ventana.
fn leer_calendario(s: &mut Sesion<'_>, i: usize, pausar: bool) -> Result<bool, Detencion> {
    let args = s.estado["registros"][i]["calendario"].clone();
    s.estado["registros"][i]["calendario_lectura"] = json!({"estado": "PENDIENTE", "args": args});
    if reutilizar_calendario(s, i, &args) {
        return Ok(false);
    }
    if s.aperturas >= MAX_LECTURAS_POR_LOTE {
        s.estado["registros"][i]["calendario_lectura"]["error"] = json!("Pendiente: se alcanzó el máximo de lecturas del lote.");
        return Err(Detencion::Lote(format!(
            "Se alcanzó el máximo de {MAX_LECTURAS_POR_LOTE} lecturas por lote. Usa «Más › Retomar paquete…» (o \
             «Reintentar lecturas fallidas…») para continuar."
        )));
    }
    if pausar && !s.pausa.is_zero() {
        let fin = std::time::Instant::now() + s.pausa;
        while std::time::Instant::now() < fin && !s.cancelar.load(Ordering::SeqCst) {
            std::thread::sleep((fin - std::time::Instant::now()).min(Duration::from_millis(20)));
        }
        s.detener_si_cancelada()?;
    }
    s.aperturas += 1;
    let clave = s.estado["registros"][i]["clave"].as_str().unwrap_or("").to_string();
    let lectura = (|| -> Result<Value, String> {
        let respuesta = s.pedir("calendar_open", json!({"args": args, "timeout": ESPERA_MS}))?;
        let datos = lote::cuerpo(&respuesta, false)?;
        let huella = sha256(&datos);
        if datos.len() > 8 * 1024 * 1024 || respuesta["digest"].as_str() != Some(huella.as_str()) || respuesta["args"] != args {
            return Err("La respuesta del calendario no coincide con la solicitud.".into());
        }
        let capturada = respuesta["startedAt"].as_str().unwrap_or("").to_string();
        chrono::DateTime::parse_from_rfc3339(&capturada.replace('Z', "+00:00"))
            .map_err(|_| "La fecha de la lectura del calendario no es válida.".to_string())?;
        let argumentos: Vec<String> = serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
        let leido = lecturas::leer_calendario(&datos, &argumentos, "", "")?;
        let nombre = format!("calendario-{}-{}.html", &clave[..24], &huella[..16]);
        let ruta = s.carpeta.join(&nombre);
        if ruta.exists() {
            if std::fs::read(&ruta).map_err(|e| e.to_string())? != datos {
                return Err("El archivo local del calendario cambió.".into());
            }
        } else {
            lote::guardar_exclusivo(&ruta, &datos)?;
        }
        Ok(json!({"archivo": nombre, "sha256": huella, "capturada": capturada, "proyecciones": leido["proyecciones"]}))
    })();
    let destino = &mut s.estado["registros"][i]["calendario_lectura"];
    match lectura {
        Ok(v) => {
            destino["estado"] = json!("LEIDA");
            for (k, x) in v.as_object().unwrap() {
                destino[k] = x.clone();
            }
        }
        Err(e) => {
            destino["estado"] = json!("FALLIDA");
            destino["error"] = json!(e);
        }
    }
    Ok(true)
}

/// `_abrir_bitacora`: `diary_open`, cotejo de huella, parámetros y fecha, lectura e instantánea local exclusiva.
fn abrir(s: &mut Sesion<'_>, i: usize, params: &Value) -> Result<(), Detencion> {
    if s.aperturas >= MAX_LECTURAS_POR_LOTE {
        return Err(Detencion::Lote(format!(
            "Se alcanzó el máximo de {MAX_LECTURAS_POR_LOTE} lecturas por lote. Usa «Más › Retomar paquete…» (o \
             «Reintentar lecturas fallidas…») para continuar."
        )));
    }
    s.aperturas += 1;
    let respuesta = s.pedir("diary_open", json!({"params": params, "timeout": ESPERA_MS}))?;
    let datos = lote::cuerpo(&respuesta, false)?;
    let huella = sha256(&datos);
    if datos.len() > 8 * 1024 * 1024 || respuesta["digest"].as_str() != Some(huella.as_str()) || &respuesta["params"] != params {
        return Err(Detencion::Consulta("La respuesta de bitácora no coincide con la solicitud.".into()));
    }
    let capturada = respuesta["startedAt"].as_str().unwrap_or("").to_string();
    chrono::DateTime::parse_from_rfc3339(&capturada.replace('Z', "+00:00"))
        .map_err(|_| Detencion::Consulta("La fecha de la lectura de bitácora no es válida.".into()))?;
    let parametros: BTreeMap<String, String> = serde_json::from_value(params.clone()).map_err(|e| e.to_string())?;
    let leida = lecturas::leer_bitacora(&datos, &parametros, "", &capturada)?;
    let r = &s.estado["registros"][i];
    let avisos = advertencias(&leida, r["rit"].as_str().unwrap_or(""), r["nombre"].as_str().unwrap_or(""));
    let clave = r["clave"].as_str().unwrap_or("").to_string();
    let nombre = format!("bitacora-{}-{}.html", &clave[..24], &huella[..16]);
    let ruta = s.carpeta.join(&nombre);
    if ruta.exists() {
        if std::fs::read(&ruta).map_err(|e| e.to_string())? != datos {
            return Err(Detencion::Consulta("El archivo local de bitácora cambió.".into()));
        }
    } else {
        lote::guardar_exclusivo(&ruta, &datos)?;
    }
    let r = &mut s.estado["registros"][i];
    r["error"] = json!(avisos.join(" "));
    r["advertencias"] = json!(avisos);
    r["estado"] = json!("LEIDA");
    r["archivo"] = json!(nombre);
    r["sha256"] = json!(huella);
    r["capturada"] = json!(capturada);
    r["entradas"] = json!(leida["entries"].as_array().map_or(0, Vec::len));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Extensión de Flow Rust: estados y códigos de calendario e histórico, sin texto libre.
    #[test]
    fn diagnostico_cuenta_calendarios_e_historicos_sin_texto_libre() {
        let datos = json!({"estado": "COMPLETA", "consultas": [], "registros": [
            {"estado": "LEIDA", "calendario": [1], "historico_lectura": {"estado": "LEIDA"},
             "calendario_lectura": {"estado": "FALLIDA", "error": "El calendario contiene una fecha ilegible en «Fecha Informe» (forma «- x -»)."}},
            {"estado": "LEIDA", "calendario": [1], "calendario_lectura": {"estado": "LEIDA"}},
            {"estado": "LEIDA", "calendario": [1], "calendario_lectura": {"estado": "PENDIENTE", "error": "Pendiente: se alcanzó el máximo de lecturas del lote."}},
            {"estado": "LEIDA", "calendario": [1]},
            {"estado": "LEIDA"}]});
        let d = diagnostico_calendarios(&datos);
        assert_eq!(d["calendarios"], json!({"FALLIDA": 1, "LEIDA": 1, "NO_INTENTADO": 1, "PENDIENTE": 1}));
        assert_eq!(d["errores_calendario"], json!({"CAL_FECHA_ILEGIBLE": 1, "CAL_TOPE": 1}));
        assert_eq!(d["historicos"], json!({"LEIDA": 1}));
        assert!(!d.to_string().contains("Fecha Informe"));
    }

    /// Reproduce las respuestas que Flow Python recibió del puente ficticio, en el mismo orden.
    struct Guion {
        pasos: Vec<Value>,
        pedidos: RefCell<Vec<Value>>,
    }

    impl Conexion for Guion {
        fn llamar(&self, comando: &str, payload: Value, _plazo: Duration) -> Result<Value, String> {
            let n = self.pedidos.borrow().len();
            self.pedidos.borrow_mut().push(json!({"comando": comando, "payload": payload}));
            let paso = self.pasos.get(n).ok_or_else(|| format!("orden {n} ({comando}) fuera del guion de Python"))?;
            if paso["comando"] != comando {
                return Err(format!("orden {n}: Rust pide {comando}, Python pidió {}", paso["comando"]));
            }
            match paso.get("respuesta") {
                Some(r) => Ok(r.clone()),
                None => Err(paso["error"].as_str().unwrap_or("error del puente").to_string()),
            }
        }
    }

    /// `null` (Python) y `{}` (Rust) son la misma orden sin datos.
    fn sin_vacio(v: &Value) -> Value {
        if v.is_null() || v.as_object().is_some_and(Map::is_empty) {
            Value::Null
        } else {
            v.clone()
        }
    }

    fn instantaneas(carpeta: &Path) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        for e in std::fs::read_dir(carpeta).unwrap().flatten() {
            let nombre = e.file_name().to_string_lossy().to_string();
            if nombre.ends_with(".html") {
                m.insert(nombre, sha256(&std::fs::read(e.path()).unwrap()));
            }
        }
        m
    }

    /// `historia/`: manifiesto, diagnóstico y estructura anónima de Escritos, más las huellas de las ventanas.
    fn historias(carpeta: &Path) -> Value {
        let mut salida = Map::new();
        for nombre in ["historias.json", "diagnostico_historia.json", "estructura_escritos.json"] {
            if let Ok(b) = std::fs::read(carpeta.join("historia").join(nombre)) {
                salida.insert(nombre.into(), serde_json::from_slice(&b).unwrap());
            }
        }
        let mut ventanas = Map::new();
        if let Ok(d) = std::fs::read_dir(carpeta.join("historia")) {
            for e in d.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with("historia-") && n.ends_with(".html") {
                    ventanas.insert(n, json!(sha256(&std::fs::read(e.path()).unwrap())));
                }
            }
        }
        salida.insert("ventanas".into(), Value::Object(ventanas));
        Value::Object(salida)
    }

    /// `historico.json` (con `leida` declarada en las entradas sin respaldo), estructura anónima y respaldos.
    fn historicos(carpeta: &Path) -> Value {
        let mut salida = Map::new();
        if let Ok(b) = std::fs::read(carpeta.join("historico.json")) {
            let mut datos: Value = serde_json::from_slice(&b).unwrap();
            for e in datos["capturas"].as_object_mut().unwrap().values_mut() {
                if e["archivo"].as_str().is_none_or(str::is_empty) {
                    e["leida"] = json!("<LEIDA>");
                }
            }
            salida.insert("historico.json".into(), datos);
        }
        if let Ok(b) = std::fs::read(carpeta.join("historico").join("estructura_historico.json")) {
            salida.insert("estructura_historico.json".into(), serde_json::from_slice(&b).unwrap());
        }
        let mut respaldos = Map::new();
        if let Ok(d) = std::fs::read_dir(carpeta.join("historico")) {
            for e in d.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with("historico-") && n.ends_with(".html") {
                    respaldos.insert(n, json!(sha256(&std::fs::read(e.path()).unwrap())));
                }
            }
        }
        salida.insert("respaldos".into(), Value::Object(respaldos));
        Value::Object(salida)
    }

    fn diferencia_paquete(captura: &crate::captura::Captura, esperado: &Value) -> Option<Value> {
        let tablas = ["consultas", "registros", "bitacoras", "calendarios", "historico_proteccion", "incidencias"];
        let cobertura: BTreeMap<_, _> = tablas.into_iter().map(|t| (t, captura.estado(t))).collect();
        let incidencias: Vec<_> = captura.filas("incidencias").map(|fila| fila.get("code")).collect();
        let obtenido = json!({"cobertura": cobertura, "incidencias": incidencias});
        (obtenido["cobertura"] != esperado["cobertura"] || obtenido["incidencias"] != esperado["incidencias"]).then_some(obtenido)
    }

    fn contrastar_paquete(e: &Value, destino: &Path, carpeta: &Path, registros: &Value) {
        use super::super::paquete;

        let esperado = &e["paquete"];
        let listados = destino.join("listados");
        std::fs::create_dir(&listados).unwrap();
        for (nombre, datos) in esperado["archivos"].as_object().unwrap() {
            std::fs::write(listados.join(nombre), serde_json::to_vec(datos).unwrap()).unwrap();
        }
        let ruta = destino.join("paquete.zip");
        let manifiesto = carpeta.join(MANIFIESTO);
        let mut opciones = paquete::Opciones {
            bitacoras: vec![manifiesto.clone()],
            historico: (e["historico"] == true).then(|| carpeta.to_path_buf()),
            ..Default::default()
        };
        let exportado = paquete::exportar_con(std::slice::from_ref(&listados), &ruta, &opciones).unwrap();
        let captura = crate::captura::leer_captura(&std::fs::read(&ruta).unwrap()).unwrap();
        let caso = e["escenario"].as_str().unwrap();
        assert_eq!(diferencia_paquete(&captura, esperado), None, "{caso}");
        let (calendario, codigo) = match caso {
            "cal_pendiente_guiones" => ("LEIDA", None),
            "cal_enviado_guiones" => ("FALLIDA", Some("CAL_ESTADO_FECHA")),
            "cal_proyeccion_guiones" => ("FALLIDA", Some("CAL_FECHA_PROYECCION")),
            "cal_estado_desconocido" => ("FALLIDA", Some("CAL_ESTADO")),
            "historico_no_verificado" => ("", Some("HISTORICO_NO_VERIFICADA")),
            _ => panic!("Escenario de paquete sin prueba: {caso}"),
        };
        assert_eq!(registros[0]["estado"], "LEIDA");
        assert_eq!(registros[1]["estado"], "LEIDA");
        if calendario.is_empty() {
            assert_eq!(registros[1]["historico_lectura"]["estado"], "FALLIDA");
        } else {
            assert_eq!(registros[0]["calendario_lectura"]["estado"], "LEIDA");
            assert_eq!(registros[1]["calendario_lectura"]["estado"], calendario);
        }
        let diagnostico = paquete::leer_json(&ruta.with_extension("diagnostico_cobertura.json")).unwrap();
        assert_eq!(diagnostico["motivos"]["BITACORA_VENTANA_VERIFICADA"], 2, "{caso}");
        if let Some(codigo) = codigo {
            assert_eq!(diagnostico["motivos"][codigo], 1);
            assert!(exportado.aviso_cobertura.unwrap().contains(&format!("{codigo} (1)")));
        } else {
            assert!(exportado.aviso_cobertura.is_none());
            let mut alterado = esperado.clone();
            alterado["cobertura"]["calendarios"] = json!("FALLIDA");
            assert!(diferencia_paquete(&captura, &alterado).is_some());

            // Control persistido por el lector Python anterior a 38fa03d con la misma ventana PENDIENTE.
            let mut anterior = paquete::leer_json(&manifiesto).unwrap();
            anterior["registros"][1]["calendario_lectura"] = json!({
                "estado": "FALLIDA",
                "error": "El calendario contiene una fecha ilegible en «Fecha Informe» (forma «- - -»).",
            });
            let control = carpeta.join("control_anterior.json");
            std::fs::write(&control, serde_json::to_vec(&anterior).unwrap()).unwrap();
            opciones.bitacoras = vec![control];
            let ruta = destino.join("anterior.zip");
            let exportado = paquete::exportar_con(&[listados], &ruta, &opciones).unwrap();
            let captura = crate::captura::leer_captura(&std::fs::read(&ruta).unwrap()).unwrap();
            assert_eq!(anterior["registros"][1]["estado"], "LEIDA");
            assert_eq!(anterior["registros"][1]["calendario_lectura"]["estado"], "FALLIDA");
            assert_eq!(captura.estado("calendarios"), "PARCIAL");
            let incidencias: Vec<_> = captura.filas("incidencias").map(|fila| fila.get("code")).collect();
            assert_eq!(incidencias, ["CALENDARIO_FALLIDO"]);
            assert!(exportado.aviso_cobertura.unwrap().contains("CAL_FECHA_ILEGIBLE (1)"));
        }
    }

    /// E-Q4a: mismas órdenes al puente, mismo `bitacoras.json`, mismas instantáneas y mismo resultado que
    /// `DiaryRunner` de Flow Python en los escenarios de `app/qa/generar_bitacoras.py` (`fecha` declarada).
    #[test]
    fn lote_de_bitacoras_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/bitacoras.json")).unwrap();
        let catalogo: Catalogo = serde_json::from_value(datos["catalogo"].clone()).unwrap();
        let (mut iguales, mut fallos) = (0usize, Vec::new());
        let mut avances: Vec<(String, usize, usize, usize)> = Vec::new();
        let escenarios = datos["escenarios"].as_array().unwrap();
        for e in escenarios {
            let nombre = e["escenario"].as_str().unwrap();
            let lista = |k: &str| -> Vec<String> { serde_json::from_value(e["lote"][k].clone()).unwrap() };
            let lote = LoteBitacoras { tribunales: lista("tribunales"), modalidades: lista("modalidades"), pestanas: lista("pestanas") };
            let guion = Guion { pasos: e["guion"].as_array().unwrap().clone(), pedidos: RefCell::default() };
            let destino = std::env::temp_dir().join(format!("flow-eq4-{}-{nombre}", std::process::id()));
            let _ = std::fs::remove_dir_all(&destino);
            std::fs::create_dir_all(&destino).unwrap();
            let mut opciones = Opciones {
                ahora: Some(chrono::DateTime::parse_from_rfc3339(e["ahora"].as_str().unwrap()).unwrap()),
                calendarios: e["calendarios"] == true,
                historico: e["historico"] == true,
                historia: (e["historia"] == true).then_some(4),
                ..Default::default()
            };
            if let Some(inicial) = e.get("inicial") {
                // Estado inicial: la carpeta que dejó la primera corrida de Python, byte a byte.
                use base64::Engine;
                let carpeta = destino.join(inicial["carpeta"].as_str().unwrap());
                std::fs::create_dir(&carpeta).unwrap();
                for (archivo, b64) in inicial["archivos"].as_object().unwrap() {
                    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap();
                    std::fs::write(carpeta.join(archivo), bytes).unwrap();
                }
                if e["modo"] == "retomar" {
                    opciones.retomar = Some(carpeta);
                } else {
                    opciones.reutilizar_base = Some(destino.clone());
                }
            }
            let cancelar = AtomicBool::new(e["cancelada"] == true);
            let eventos = std::cell::RefCell::new(Vec::<(String, Value)>::new());
            let resultado = ejecutar_con_avance(&guion, &catalogo, &lote, &destino, &opciones, &cancelar,
                &|n, v| eventos.borrow_mut().push((n.to_string(), v))).unwrap();
            // E-Q7: una fila de avance por registro enumerado en esta corrida y una consulta por perfil iniciado.
            let eventos = eventos.into_inner();
            let filas = eventos.iter().filter(|(n, _)| n == "diary_progress").count();
            let registros_corrida = serde_json::from_slice::<Value>(&std::fs::read(
                PathBuf::from(resultado["manifest"].as_str().unwrap())).unwrap()).unwrap()["registros"].as_array().map_or(0, Vec::len);
            avances.push((nombre.to_string(), filas, registros_corrida, eventos.iter().filter(|(n, _)| n == "query").count()));
            let carpeta = PathBuf::from(resultado["folder"].as_str().unwrap());
            if carpeta.file_name().and_then(|n| n.to_str()) != e["carpeta"].as_str() {
                fallos.push(format!("{nombre}: carpeta Rust {carpeta:?} / Python {}", e["carpeta"]));
            }
            let mut manifiesto: Value = serde_json::from_slice(&std::fs::read(carpeta.join(MANIFIESTO)).unwrap()).unwrap();
            manifiesto["fecha"] = json!("<FECHA>");
            let mut diferencias = Vec::new();
            let pedidos = guion.pedidos.borrow();
            let esperados = e["guion"].as_array().unwrap();
            if pedidos.len() != esperados.len() {
                diferencias.push(format!("órdenes: Rust {} / Python {}", pedidos.len(), esperados.len()));
            }
            for (k, (p, x)) in pedidos.iter().zip(esperados).enumerate() {
                if p["comando"] != x["comando"] || sin_vacio(&p["payload"]) != sin_vacio(&x["payload"]) {
                    diferencias.push(format!("orden {k}: Rust {p} / Python {}", json!({"comando": x["comando"], "payload": x["payload"]})));
                }
            }
            for clave in manifiesto.as_object().unwrap().keys().chain(e["manifiesto"].as_object().unwrap().keys()) {
                if manifiesto.get(clave) != e["manifiesto"].get(clave) {
                    diferencias.push(format!("manifiesto.{clave}:\n    Rust   {:?}\n    Python {:?}", manifiesto.get(clave), e["manifiesto"].get(clave)));
                }
            }
            let esperadas: BTreeMap<String, String> = serde_json::from_value(e["instantaneas"].clone()).unwrap();
            if instantaneas(&carpeta) != esperadas {
                diferencias.push(format!("instantáneas: Rust {:?} / Python {esperadas:?}", instantaneas(&carpeta)));
            }
            let diagnostico: Value =
                serde_json::from_slice(&std::fs::read(carpeta.join("diagnostico_bitacoras.json")).unwrap()).unwrap();
            if diagnostico != e["diagnostico"] {
                diferencias.push(format!("diagnóstico:\n    Rust   {diagnostico}\n    Python {}", e["diagnostico"]));
            }
            if let Some(esperado) = e.get("historias").filter(|h| !h.is_null()) {
                let obtenido = historias(&carpeta);
                if &obtenido != esperado {
                    diferencias.push(format!("historia:\n    Rust   {obtenido}\n    Python {esperado}"));
                }
            }
            if let Some(esperado) = e.get("historicos") {
                let obtenido = historicos(&carpeta);
                if &obtenido != esperado {
                    diferencias.push(format!("histórico:\n    Rust   {obtenido}\n    Python {esperado}"));
                }
            }
            for (clave, valor) in e["resultado"].as_object().unwrap() {
                if &resultado[clave] != valor {
                    diferencias.push(format!("resultado.{clave}: Rust {} / Python {valor}", resultado[clave]));
                }
            }
            if e.get("paquete").is_some() {
                contrastar_paquete(e, &destino, &carpeta, &manifiesto["registros"]);
            }
            let _ = std::fs::remove_dir_all(&destino);
            if diferencias.is_empty() {
                iguales += 1;
            } else {
                diferencias.dedup();
                fallos.push(format!("{nombre}:\n  {}", diferencias.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("BITACORAS: {iguales}/{}, fallos {}", escenarios.len(), fallos.len());
        let distintos: Vec<_> = avances.iter().filter(|(_, filas, registros, _)| filas != registros).collect();
        println!("AVANCE: {} escenarios con una fila de avance por registro; distintos {distintos:?}", avances.len() - distintos.len());
        assert!(distintos.is_empty(), "el avance por fila no cubre cada registro");
        assert!(avances.iter().all(|(_, _, _, consultas)| *consultas > 0), "falta el evento de consulta");
        assert_eq!(escenarios.len(), 21, "el fixture cambió de tamaño: regenerar y revisar");
        assert_eq!(escenarios.iter().filter(|e| e.get("paquete").is_some()).count(), 5);
        assert!(fallos.is_empty(), "{} escenarios difieren de Flow Python", fallos.len());
    }
}
