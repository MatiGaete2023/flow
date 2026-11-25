//! E-Q9 · «Consultar y crear paquete» en una sola acción (equivalente de `descargador/flujo_paquete.py`).
//!
//! Encadena las fases de Flow Python en una carpeta nueva «Paquete CSMP <fecha y hora>»: consultas (`lote::ejecutar`),
//! bitácoras con calendarios, historia e histórico (`bitacoras::ejecutar_con_avance`), informes por vencer de los
//! próximos 60 días (`Pantalla::CalendarioInformes`) y el paquete CSMP (`paquete::exportar_con`). Deja
//! `respaldo/proceso.json` con estado, fase, carpetas, opciones, historial y reintentos, y `Captura_CSMP.zip` junto a
//! `respaldo/`. Con [`retomar`] continúa un paquete cortado: reutiliza las consultas validadas y las bitácoras ya
//! leídas y vuelve a crear el ZIP. Solo lee de SITFA con la sesión del usuario; no escribe en RUS ni SATURNO.

use crate::descargador::bitacoras::{self, LoteBitacoras, MANIFIESTO};
use crate::descargador::lote::{self, Conexion, Evento as EventoLote, Opciones as OpcionesLote};
use crate::descargador::paquete::{self, Opciones as OpcionesPaquete};
use crate::descargador::{Catalogo, Edicion, Pantalla, PlanLote, Seleccion, TODAS_LAS_MEDIDAS};
use chrono::{Datelike, NaiveDate};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// `PROCESO`: control del paquete dentro de `respaldo/`.
pub const PROCESO: &str = "proceso.json";
/// `CAPTURA_CSMP_ZIP`: nombre del paquete en la raíz de la carpeta del flujo.
pub const ZIP: &str = "Captura_CSMP.zip";
/// Prefijo de la carpeta que crea el flujo (y que `pendientes` reconoce).
pub const PREFIJO: &str = "Paquete CSMP ";
const HISTORIAL_MAXIMO: usize = 200;
const CANCELADO: &str = "Proceso detenido. Los originales verificados se conservan en respaldo.";
const NO_IDENTIFICO: &str = "La consulta no identificó sus originales; no se creó el paquete.";
const SEGUIMIENTO: &str =
    "Consultar y crear paquete requiere Seguimiento; las otras consultas siguen en Avanzado.";
const OPCIONES_INVALIDAS: &str = "Las opciones guardadas del paquete no son válidas.";
const CONTROL_INVALIDO: &str = "El control del paquete (respaldo/proceso.json) no es válido.";
const CONTROL_AJENO: &str =
    "Elige una carpeta «Paquete CSMP …» creada por «Consultar y crear paquete».";
const YA_COMPLETO: &str = "Ese paquete ya está completo; no hay nada que retomar.";
const FUERA_DE_RESPALDO: &str = "El control del paquete apunta fuera de su respaldo.";
const SIN_ALCANCE: &str =
    "Este paquete no guardó su alcance (se cortó en las consultas). Crea un paquete nuevo.";
const FUENTES_FUERA: &str = "La carpeta de fuentes quedó fuera de respaldo.";
const CONTROL_FUERA: &str = "El control de bitácoras quedó fuera de respaldo.";
const HISTORIA_AVISO: &str = "La historia no pudo incluirse en el paquete; las bitácoras se conservan.";
const HISTORICO_AVISO: &str =
    "Quedaron lecturas del Ingreso Histórico Protección sin verificar. El paquete queda parcial.";
const CALENDARIO_AVISO: &str =
    "No se incluyó completo el calendario de informes por vencer: ";
const CALENDARIO_INCOMPLETO: &str =
    "Se pidió el calendario de informes por vencer y no quedó completo en el paquete.";

/// Opciones del flujo, con los mismos nombres y validación que `_validar_opciones` de Python.
#[derive(Clone, Debug, PartialEq)]
pub struct Opciones {
    pub consolidado: bool,
    pub calendarios: bool,
    pub historia: bool,
    pub historico: bool,
    pub historia_en_paquete: bool,
    pub meses: u32,
    pub informes: bool,
    pub fecha_informes: NaiveDate,
    pub pdf_vacias: bool,
    pub pausa: Duration,
    /// `reuse`: reutiliza bitácoras de otros lotes del mismo día (solo en un paquete nuevo).
    pub reutilizar: bool,
}

impl Default for Opciones {
    fn default() -> Self {
        Self {
            consolidado: true,
            calendarios: false,
            historia: false,
            historico: false,
            historia_en_paquete: true,
            meses: 4,
            informes: false,
            fecha_informes: chrono::Local::now().date_naive(),
            pdf_vacias: true,
            pausa: Duration::from_secs(1),
            reutilizar: true,
        }
    }
}

impl Opciones {
    /// `state.setdefault('opciones', {...})` y `_validar_opciones`.
    pub fn desde_json(v: &Value) -> Result<Self, String> {
        let b = |k: &str| v[k].as_bool().ok_or_else(|| OPCIONES_INVALIDAS.to_string());
        let meses = v["meses"].as_u64().filter(|m| (1..=4).contains(m)).ok_or_else(|| OPCIONES_INVALIDAS.to_string())?;
        let fecha = v["fecha_informes"]
            .as_str()
            .and_then(|f| NaiveDate::parse_from_str(f, "%Y-%m-%d").ok())
            .ok_or_else(|| OPCIONES_INVALIDAS.to_string())?;
        Ok(Self {
            consolidado: b("consolidado")?,
            calendarios: b("calendarios")?,
            historia: b("historia")?,
            historico: b("historico")?,
            historia_en_paquete: b("historia_en_paquete")?,
            meses: meses as u32,
            informes: b("informes")?,
            fecha_informes: fecha,
            pdf_vacias: b("pdf_vacias")?,
            pausa: Default::default(),
            reutilizar: Default::default(),
        })
    }

    pub fn a_json(&self) -> Value {
        json!({
            "consolidado": self.consolidado, "calendarios": self.calendarios, "historia": self.historia,
            "historico": self.historico, "historia_en_paquete": self.historia_en_paquete, "meses": self.meses,
            "informes": self.informes, "fecha_informes": self.fecha_informes.to_string(),
            "pdf_vacias": self.pdf_vacias,
        })
    }
}

/// Eventos del flujo: los mismos de `pipeline_folder`, `pipeline_source`, `phase`, `status`, `warning` y
/// `package_done` de Flow Python, más el avance por fila del lote de bitácoras (E-Q7).
#[derive(Clone, Debug, PartialEq)]
pub enum Evento {
    /// `pipeline_folder`: la carpeta «Paquete CSMP …» recién creada.
    Carpeta(PathBuf),
    /// `phase`: consultas, bitácoras, informes, paquete (o `informes-AAAA-MM`).
    Fase(String),
    /// `pipeline_source`: subcarpeta de fuentes de una fase, dentro de `respaldo/`.
    Origen { fase: String, carpeta: PathBuf },
    /// `status`: mensaje de avance.
    Estado(String),
    /// `query`, `diary_progress` o `warning` del lote de bitácoras.
    Avance(String, Value),
    /// `warning`: aviso visible al terminar.
    Aviso(String),
    /// `package_done`: resultado del paquete.
    Paquete(Value),
}

/// Resultado del flujo, equivalente al diccionario de `flujo_paquete.run` de Python.
#[derive(Clone, Debug)]
pub struct Resultado {
    pub paquete: PathBuf,
    pub parcial: bool,
    pub consultas: PathBuf,
    pub manifiesto: PathBuf,
    pub historia: Option<Value>,
    pub historico_incompleto: bool,
}

/// `_proceso`: control leído y validado de un paquete que se retoma.
struct Control {
    datos: Value,
}

fn ahora() -> chrono::DateTime<chrono::Local> {
    chrono::Local::now()
}

/// `«Paquete CSMP » + now().strftime('%Y-%m-%d %H-%M-%S-%f')`.
fn nombre_carpeta() -> String {
    format!("{PREFIJO}{}", ahora().format("%Y-%m-%d %H-%M-%S-%6f"))
}

/// `anotar`: últimos 200 eventos de vocabulario controlado, con hora local y segundos.
fn anotar(estado: &mut Value, evento: &str) {
    let texto = evento.split_whitespace().collect::<Vec<_>>().join(" ");
    let texto: String = texto.chars().take(500).collect();
    let linea = format!("{} {}", ahora().to_rfc3339_opts(chrono::SecondsFormat::Secs, false), texto);
    if !estado["historial"].is_array() {
        estado["historial"] = json!([]);
    }
    let historial = estado["historial"].as_array_mut().unwrap();
    historial.push(json!(linea));
    let sobran = historial.len().saturating_sub(HISTORIAL_MAXIMO);
    if sobran > 0 {
        historial.drain(..sobran);
    }
}

/// `write(control, state)`: JSON con temporada y renombrado (nunca deja un control a medias).
fn escribir_control(ruta: &Path, estado: &Value) -> Result<(), String> {
    let temporal = ruta.with_extension("tmp");
    let datos = serde_json::to_vec_pretty(estado).map_err(|e| e.to_string())?;
    std::fs::write(&temporal, datos).map_err(|e| format!("No se pudo guardar el control del paquete: {e}"))?;
    std::fs::rename(&temporal, ruta).map_err(|e| format!("No se pudo guardar el control del paquete: {e}"))
}

fn checkpoint(cancelar: &AtomicBool) -> Result<(), String> {
    if cancelar.load(Ordering::SeqCst) {
        Err(CANCELADO.into())
    } else {
        Ok(())
    }
}

fn fase(
    estado: &mut Value,
    ruta: &Path,
    nombre: &str,
    cancelar: &AtomicBool,
    emitir: &mut dyn FnMut(Evento),
) -> Result<(), String> {
    checkpoint(cancelar)?;
    estado["fase"] = json!(nombre);
    anotar(estado, &format!("fase {nombre}"));
    escribir_control(ruta, estado)?;
    emitir(Evento::Fase(nombre.to_string()));
    Ok(())
}

/// `relay('folder', …)`: la subcarpeta de la fase queda dentro de `respaldo/` y se anota relativa a la raíz.
fn registrar_origen(
    estado: &mut Value,
    control: &Path,
    respaldo: &Path,
    raiz: &Path,
    nombre: &str,
    carpeta: &Path,
    emitir: &mut dyn FnMut(Evento),
) -> Result<(), String> {
    let resuelta = carpeta.canonicalize().unwrap_or_else(|_| carpeta.to_path_buf());
    let dentro = respaldo.canonicalize().map(|r| resuelta.starts_with(&r)).unwrap_or(false);
    if !dentro {
        return Err(FUENTES_FUERA.into());
    }
    // canonicalize en Windows devuelve \\?\C:\…: la raíz se compara canonicalizada para que la carpeta quede relativa.
    let raiz_canonica = raiz.canonicalize().unwrap_or_else(|_| raiz.to_path_buf());
    let relativa = resuelta
        .strip_prefix(&raiz_canonica)
        .or_else(|_| resuelta.strip_prefix(raiz))
        .unwrap_or(&resuelta)
        .to_string_lossy()
        .into_owned();
    estado["carpetas"][nombre] = json!(relativa);
    escribir_control(control, estado)?;
    emitir(Evento::Origen { fase: nombre.to_string(), carpeta: resuelta });
    Ok(())
}

fn consultas_validadas(carpeta: Option<&PathBuf>) -> bool {
    carpeta
        .and_then(|c| std::fs::read(c.join("resumen.json")).ok())
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .is_some_and(|r| r["estado"] == "VALIDADA")
}

fn carpeta_del_estado(raiz: &Path, estado: &Value, fase: &str) -> Option<PathBuf> {
    estado["carpetas"][fase].as_str().map(|c| raiz.join(relativa(c)))
}

/// El `proceso.json` puede venir de Windows (`respaldo\\...`) y leerse en Linux: los separadores se normalizan.
fn relativa(texto: &str) -> PathBuf {
    PathBuf::from(texto.replace('\\', "/"))
}

/// `_proceso`: valida el control de un paquete antes de retomarlo.
fn leer_control(raiz: &Path) -> Result<Control, String> {
    let respaldo = raiz.join("respaldo");
    let ruta = respaldo.join(PROCESO);
    let datos = std::fs::read(&ruta).map_err(|_| CONTROL_AJENO.to_string())?;
    let estado: Value = serde_json::from_slice(&datos).map_err(|_| CONTROL_AJENO.to_string())?;
    if !estado.is_object() || !estado["carpetas"].is_object() {
        return Err(CONTROL_INVALIDO.into());
    }
    if estado["estado"] == "COMPLETO" {
        return Err(YA_COMPLETO.into());
    }
    for carpeta in estado["carpetas"].as_object().map(|c| c.values()).into_iter().flatten() {
        let Some(relativa_texto) = carpeta.as_str() else {
            return Err(FUERA_DE_RESPALDO.into());
        };
        let destino = raiz.join(relativa(relativa_texto));
        let dentro = destino
            .canonicalize()
            .map(|d| d.starts_with(respaldo.canonicalize().unwrap_or_else(|_| respaldo.clone())))
            .unwrap_or_else(|_| {
                // Una carpeta que ya no existe no puede apuntar fuera: se comprueba la ruta escrita.
                normalize(&destino).starts_with(normalize(&respaldo))
            });
        if !dentro {
            return Err(FUERA_DE_RESPALDO.into());
        }
    }
    Ok(Control { datos: estado })
}

/// Normaliza `..` y `.` sin tocar el disco (para rutas que aún no existen).
fn normalize(ruta: &Path) -> PathBuf {
    let mut salida = PathBuf::new();
    for parte in ruta.components() {
        match parte {
            std::path::Component::ParentDir => {
                salida.pop();
            }
            std::path::Component::CurDir => {}
            otro => salida.push(otro.as_os_str()),
        }
    }
    salida
}

/// `_alcance`: el alcance guardado (`seleccion`) o, en paquetes anteriores, el de su lote de bitácoras.
fn alcance_guardado(raiz: &Path, estado: &Value) -> Result<LoteBitacoras, String> {
    let lista = |v: &Value, k: &str| -> Option<Vec<String>> {
        v[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
    };
    if estado["seleccion"].is_object() {
        let s = &estado["seleccion"];
        match (lista(s, "tribunals"), lista(s, "modalities"), lista(s, "tabs")) {
            (Some(t), Some(m), Some(p)) => return Ok(LoteBitacoras { tribunales: t, modalidades: m, pestanas: p }),
            _ => return Err(SIN_ALCANCE.into()),
        }
    }
    if let Some(carpeta) = carpeta_del_estado(raiz, estado, "bitacoras") {
        if carpeta.join(MANIFIESTO).is_file() {
            return LoteBitacoras::guardado(&carpeta);
        }
    }
    Err(SIN_ALCANCE.into())
}

/// `Batch.plan(catalog)` para el alcance guardado, con el catálogo de Seguimiento ya cargado.
fn plan_del_alcance(alcance: &LoteBitacoras, catalogo: &Catalogo, edicion: Edicion) -> Result<PlanLote, String> {
    let lote = crate::descargador::SeleccionLote {
        tribunales: alcance.tribunales.clone(),
        pestanas: alcance.pestanas.clone(),
        modalidades: alcance.modalidades.clone(),
        informes: vec!["1".into()],
        medidas: vec![TODAS_LAS_MEDIDAS.into()],
        ..Default::default()
    };
    catalogo.planificar(edicion, &lote)
}

/// `meses_informes`: meses (mes, año) desde hoy hasta hoy + 60 días; a veces dos, a veces tres.
pub fn meses_informes(hoy: NaiveDate, dias: u32) -> Vec<(String, String)> {
    let fin = hoy + chrono::Days::new(u64::from(dias));
    let mut actual = hoy.with_day(1).unwrap_or(hoy);
    let mut meses = Vec::new();
    while actual <= fin {
        meses.push((actual.month().to_string(), actual.year().to_string()));
        actual = if actual.month() == 12 {
            NaiveDate::from_ymd_opt(actual.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(actual.year(), actual.month() + 1, 1)
        }
        .unwrap_or(fin + chrono::Days::new(1));
    }
    meses
}

/// `bridge.call('catalog', {'screen': …})` + `catalogo_para`: el catálogo de otra pantalla, sin pasar por `Puente`.
fn catalogo_de(conexion: &dyn Conexion, pantalla: Pantalla) -> Result<Catalogo, String> {
    let codigo = match pantalla {
        Pantalla::Seguimiento => "seguimiento",
        Pantalla::Litigantes => "litigantes",
        Pantalla::CalendarioInformes => "calendario_informes",
        Pantalla::CalendarioMedidas => "calendario_medidas",
        Pantalla::Carga => "carga",
    };
    let datos = conexion.llamar("catalog", json!({"screen": codigo}), Duration::from_secs(75))?;
    if datos["pantalla"].as_str() != Some(codigo) {
        return Err("Chrome devolvió un catálogo de otra pantalla.".into());
    }
    serde_json::from_value(datos).map_err(|_| "Chrome no devolvió un catálogo válido.".into())
}

/// Plan de una consulta de calendario por tribunal (equivalente a `Batch(tribunals, …, screen='calendario_informes')`).
fn plan_calendario(
    catalogo: &Catalogo,
    alcance: &LoteBitacoras,
    mes: &str,
    anio: &str,
    edicion: Edicion,
) -> Result<PlanLote, String> {
    let mut consultas = Vec::new();
    for tribunal in &alcance.tribunales {
        let seleccion = Seleccion {
            tribunal: tribunal.clone(),
            mes: mes.to_string(),
            anio: anio.to_string(),
            ..Default::default()
        };
        consultas.push(catalogo.preparar(edicion, &seleccion)?);
    }
    Ok(PlanLote { consultas, fechas: None })
}

/// `aviso_faltantes`: un texto por tribunal; con `nombre` = false usa «tribunal N» (anotación del historial).
fn aviso_faltantes(faltantes: &Value, con_nombre: bool) -> Vec<String> {
    let Some(mapa) = faltantes.as_object() else { return Vec::new() };
    let mut claves: Vec<&String> = mapa.keys().collect();
    claves.sort();
    let mut avisos = Vec::new();
    for codigo in claves {
        if !con_nombre && !codigo.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let item = &mapa[codigo];
        let mut partes = Vec::new();
        for (clave, etiqueta) in [("SIN_VINCULO", "sin vínculo"), ("FALLIDA", "fallidas"), ("PENDIENTE", "pendientes")] {
            if let Some(n) = item[clave].as_u64().filter(|n| *n > 0) {
                partes.push(format!("{n} {etiqueta}"));
            }
        }
        let nombre = item["nombre"].as_str().unwrap_or("");
        let tribunal = if con_nombre && !nombre.is_empty() {
            format!("{nombre} ({codigo})")
        } else {
            format!("tribunal {codigo}")
        };
        avisos.push(format!("Bitácoras no leídas en {tribunal}: {}.", partes.join(", ")));
    }
    avisos
}

/// Fases del flujo y estado a partir del cual se puede retomar. `ejecutar` crea y ejecuta la carpeta; `retomar`
/// continúa una cortada.
#[allow(clippy::too_many_arguments)]
fn correr(
    conexion: &dyn Conexion,
    catalogo: &Catalogo,
    edicion: Edicion,
    plan: &PlanLote,
    alcance: &LoteBitacoras,
    destino_usuario: Option<&Path>,
    raiz: &Path,
    previo: Option<Control>,
    opciones: &Opciones,
    cancelar: &AtomicBool,
    emitir: &mut dyn FnMut(Evento),
) -> Result<Resultado, String> {
    let retomando = previo.is_some();
    if catalogo.pantalla != Pantalla::Seguimiento
        || plan.consultas.iter().any(|p| p.pantalla != Pantalla::Seguimiento)
    {
        return Err(SEGUIMIENTO.into());
    }
    let mut estado = match previo {
        Some(control) => control.datos,
        None => json!({
            "estado": "EN_CURSO", "fase": "consultas", "carpetas": {},
            "seleccion": {"tribunals": alcance.tribunales, "modalities": alcance.modalidades, "tabs": alcance.pestanas},
        }),
    };
    let respaldo = raiz.join("respaldo");
    let control = respaldo.join(PROCESO);
    let guardadas = estado.get("opciones").cloned();
    let op = match guardadas {
        Some(v) => Opciones::desde_json(&v)?,
        None => {
            estado["opciones"] = opciones.a_json();
            opciones.clone()
        }
    };
    std::fs::create_dir_all(&respaldo)
        .map_err(|e| format!("No se pudo crear la carpeta del paquete: {e}"))?;
    if estado["historial_version"] != 2 {
        estado["historial"] = json!([]);
        estado["historial_version"] = json!(2);
    }
    let reintentos = estado["reintentos"].as_u64().unwrap_or(0);
    anotar(&mut estado, &if retomando { format!("inicio; reintento {reintentos}") } else { "inicio".into() });
    escribir_control(&control, &estado)?;
    emitir(Evento::Carpeta(raiz.to_path_buf()));
    let anterior: BTreeMap<String, PathBuf> = if retomando {
        estado["carpetas"]
            .as_object()
            .map(|m| m.iter().filter_map(|(k, v)| Some((k.clone(), raiz.join(relativa(v.as_str()?))))).collect())
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };

    let corrida = (|| -> Result<(Resultado, Value), String> {
        // -- consultas (`BatchRunner.run`)
        fase(&mut estado, &control, "consultas", cancelar, emitir)?;
        if consultas_validadas(anterior.get("consultas")) {
            anotar(&mut estado, "consultas reutilizadas del intento anterior");
            emitir(Evento::Estado("Consultas ya validadas en el intento anterior: se reutilizan.".into()));
        } else {
            if let Some(carpetas) = estado["carpetas"].as_object_mut() {
                carpetas.remove("consultas");
            }
            let opciones_lote = OpcionesLote { conservar_filtros: false, evidencia: op.pdf_vacias, pausa: op.pausa };
            let mut carpeta = None;
            let mut relé = |evento: EventoLote| match evento {
                EventoLote::Carpeta(c) => carpeta = Some(c),
                EventoLote::Aviso(a) => emitir(Evento::Aviso(a)),
                EventoLote::Consulta { indice, total, etiqueta } => {
                    emitir(Evento::Estado(format!("Consultando {indice} de {total}: {etiqueta}")))
                }
                EventoLote::Pagina { pagina, total, .. } => emitir(Evento::Estado(format!("Página {pagina} de {total}"))),
                _ => {}
            };
            lote::ejecutar(conexion, catalogo, edicion, plan, &respaldo, &opciones_lote, cancelar, &mut relé)?;
            let carpeta = carpeta.ok_or_else(|| NO_IDENTIFICO.to_string())?;
            registrar_origen(&mut estado, &control, &respaldo, raiz, "consultas", &carpeta, emitir)?;
        }
        let consultas = carpeta_del_estado(raiz, &estado, "consultas").ok_or_else(|| NO_IDENTIFICO.to_string())?;
        if !consultas.is_dir() {
            return Err(NO_IDENTIFICO.into());
        }
        checkpoint(cancelar)?;

        // -- bitácoras, calendarios, historia e histórico (`DiaryRunner.run`)
        fase(&mut estado, &control, "bitacoras", cancelar, emitir)?;
        let previas = anterior.get("bitacoras").filter(|c| c.join(MANIFIESTO).is_file()).cloned();
        let historia_anterior = previas.as_ref().map(|c| c.join("historia").join("historias.json")).filter(|h| h.is_file());
        let opciones_bit = bitacoras::Opciones {
            pausa: op.pausa,
            retomar: previas.clone(),
            reutilizar_base: (op.reutilizar && !retomando).then(|| destino_usuario.map(Path::to_path_buf)).flatten(),
            calendarios: op.calendarios,
            historico: op.historico,
            historia: op.historia.then_some(op.meses),
            historia_anterior,
            ahora: None,
        };
        let carpeta_bitacoras = std::cell::RefCell::new(None::<PathBuf>);
        let diarios = {
            // `Avance` es un `Fn`: el emisor y la carpeta se comparten por dentro (RefCell) para no pedir `FnMut`.
            let emisor = std::cell::RefCell::new(&mut *emitir);
            let avance = |nombre: &str, valor: Value| {
                if nombre == "folder" {
                    *carpeta_bitacoras.borrow_mut() = valor.as_str().map(PathBuf::from);
                } else {
                    let mut emitir = emisor.borrow_mut();
                    emitir(Evento::Avance(nombre.to_string(), valor));
                }
            };
            bitacoras::ejecutar_con_avance(conexion, catalogo, alcance, &respaldo, &opciones_bit, cancelar, &avance)?
        };
        checkpoint(cancelar)?;
        let carpeta_bitacoras = carpeta_bitacoras
            .into_inner()
            .or_else(|| diarios["folder"].as_str().map(PathBuf::from))
            .ok_or_else(|| "El lote de bitácoras no identificó su carpeta; no se creó el paquete.".to_string())?;
        registrar_origen(&mut estado, &control, &respaldo, raiz, "bitacoras", &carpeta_bitacoras, emitir)?;
        let n = |k: &str| diarios[k].as_u64().unwrap_or(0);
        let mut resumen_bit = format!(
            "bitácoras {}; leídas {}; no leídas {}; consultas fallidas {}",
            diarios["estado"].as_str().unwrap_or("?"),
            n("leidas"),
            n("fallidas"),
            n("consultas_fallidas"),
        );
        let historicos = &diarios["historicos"];
        let texto_historicos = match historicos {
            Value::Object(m) if !m.is_empty() => {
                let partes: Vec<String> = m.iter().map(|(k, v)| format!("'{k}': {}", v.as_u64().unwrap_or(0))).collect();
                format!("{{{}}}", partes.join(", "))
            }
            _ => String::new(),
        };
        if !texto_historicos.is_empty() {
            resumen_bit.push_str(&format!("; históricos {texto_historicos}"));
        }
        anotar(&mut estado, &resumen_bit);
        let faltantes = diarios["faltantes_por_tribunal"].clone();
        for aviso in aviso_faltantes(&faltantes, true) {
            emitir(Evento::Aviso(aviso));
        }
        for aviso in aviso_faltantes(&faltantes, false) {
            anotar(&mut estado, &aviso);
        }
        escribir_control(&control, &estado)?;

        // -- informes por vencer (`_informes`): solo con la hoja de Cumplimiento
        let (carpetas_informes, informes_completos) =
            informes(conexion, edicion, alcance, &respaldo, raiz, &mut estado, &control, &op, cancelar, emitir)?;

        // -- paquete (`package_current_lot`)
        fase(&mut estado, &control, "paquete", cancelar, emitir)?;
        let manifiesto = PathBuf::from(diarios["manifest"].as_str().unwrap_or(""));
        let dentro = manifiesto
            .canonicalize()
            .map(|m| m.starts_with(respaldo.canonicalize().unwrap_or_else(|_| respaldo.clone())))
            .unwrap_or(false);
        if !dentro {
            return Err(CONTROL_FUERA.into());
        }
        let resumen_historia = if op.historia {
            std::fs::read(carpeta_bitacoras.join("historia").join("historias.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        } else {
            None
        };
        let mut historia_ok = resumen_historia.as_ref().is_some_and(|r| r["estado"] == "COMPLETA");
        let ruta_historia = if op.historia && op.historia_en_paquete {
            let candidata = carpeta_bitacoras.join("historia").join("historias.json");
            candidata.is_file().then_some(candidata)
        } else {
            None
        };
        let mut avisos = Vec::new();
        if !informes_completos {
            avisos.push(("CALENDARIO_INCOMPLETO".to_string(), CALENDARIO_INCOMPLETO.to_string()));
        }
        let mut lotes = vec![consultas.clone()];
        lotes.extend(carpetas_informes.iter().cloned());
        let opciones_paquete = |historia: Option<&Path>| OpcionesPaquete {
            bitacoras: vec![manifiesto.clone()],
            emparejar: true,
            historia: historia.map(Path::to_path_buf),
            historico: op.historico.then(|| manifiesto.parent().map(Path::to_path_buf)).flatten(),
            meses: op.meses,
            avisos: avisos.clone(),
            consolidado: op.consolidado,
        };
        let objetivo = raiz.join(ZIP);
        let temporal = respaldo.join(format!(".captura-{}", ahora().format("%Y%m%d%H%M%S%6f")));
        std::fs::create_dir_all(&temporal).map_err(|e| format!("No se pudo preparar el paquete: {e}"))?;
        let nuevo = temporal.join(ZIP);
        let resultado = match paquete::exportar_con(&lotes, &nuevo, &opciones_paquete(ruta_historia.as_deref())) {
            Ok(e) => Ok(e),
            Err(_) if ruta_historia.is_some() => {
                emitir(Evento::Aviso(HISTORIA_AVISO.into()));
                historia_ok = false;
                let _ = std::fs::remove_file(&nuevo);
                paquete::exportar_con(&lotes, &nuevo, &opciones_paquete(None))
            }
            Err(e) => Err(e),
        };
        let resultado = match resultado {
            Ok(e) => e,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&temporal);
                return Err(e);
            }
        };
        checkpoint(cancelar)?;
        if objetivo.exists() {
            let anterior_zip = respaldo.join(format!("Captura_CSMP anterior {}.zip", ahora().format("%Y-%m-%d %H-%M-%S-%6f")));
            std::fs::copy(&objetivo, &anterior_zip).map_err(|e| format!("No se pudo conservar el paquete anterior: {e}"))?;
        }
        std::fs::rename(&nuevo, &objetivo).map_err(|e| format!("No se pudo publicar el paquete: {e}"))?;
        let _ = std::fs::remove_dir_all(&temporal);
        let historico_fallido = op.historico && resultado.historico_incompleto;
        if historico_fallido {
            emitir(Evento::Aviso(HISTORICO_AVISO.into()));
        }
        let mut parcial = diarios["estado"] != "COMPLETA" || n("fallidas") > 0 || n("consultas_fallidas") > 0;
        parcial |= n("calendarios_fallidos") > 0;
        parcial |= !informes_completos || historico_fallido;
        if op.historia {
            parcial |= !historia_ok;
        }
        estado["estado"] = json!(if parcial { "PARCIAL" } else { "COMPLETO" });
        estado["paquete"] = json!(objetivo.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        let fin = format!(
            "fin {}{}",
            if parcial { "PARCIAL" } else { "COMPLETO" },
            if informes_completos { "" } else { "; calendario de informes incompleto" }
        );
        anotar(&mut estado, &fin);
        escribir_control(&control, &estado)?;
        let historico_incompleto = resultado.historico_incompleto;
        let mut valor = resultado.respuesta_interfaz();
        valor["path"] = json!(objetivo.to_string_lossy().into_owned());
        valor["sheets_folder"] = json!(consultas.to_string_lossy().into_owned());
        valor["diary_manifest"] = json!(manifiesto.to_string_lossy().into_owned());
        valor["partial"] = json!(parcial);
        if op.historia {
            valor["historia"] = resumen_historia.clone().unwrap_or_else(|| json!({"estado": "INCOMPLETA"}));
        }
        Ok((
            Resultado {
                paquete: objetivo,
                parcial,
                consultas,
                manifiesto,
                historia: resumen_historia,
                historico_incompleto,
            },
            valor,
        ))
    })();

    match corrida {
        Ok((resultado, valor)) => {
            emitir(Evento::Paquete(valor));
            Ok(resultado)
        }
        Err(e) => {
            estado["estado"] = json!("INCOMPLETO");
            let codigo = "ERROR_OPERACION";
            let fase_actual = estado["fase"].as_str().unwrap_or("").to_string();
            anotar(&mut estado, &format!("error en fase {fase_actual} ({codigo})"));
            let _ = escribir_control(&control, &estado);
            Err(e)
        }
    }
}

/// `_informes`: calendario de informes por vencer de los próximos 60 días, solo con la hoja de Cumplimiento.
/// Devuelve (carpetas, completo): un mes que falla no descarta los ya descargados, pero el paquete queda parcial.
#[allow(clippy::too_many_arguments)]
fn informes(
    conexion: &dyn Conexion,
    edicion: Edicion,
    alcance: &LoteBitacoras,
    respaldo: &Path,
    raiz: &Path,
    estado: &mut Value,
    control: &Path,
    opciones: &Opciones,
    cancelar: &AtomicBool,
    emitir: &mut dyn FnMut(Evento),
) -> Result<(Vec<PathBuf>, bool), String> {
    if !opciones.informes || !alcance.pestanas.iter().any(|p| p == "Cumplimiento") {
        return Ok((Vec::new(), true));
    }
    fase(estado, control, "informes", cancelar, emitir)?;
    let meses = meses_informes(opciones.fecha_informes, 60);
    let mut carpetas = Vec::new();
    let corrida = (|| -> Result<(), String> {
        let catalogo = catalogo_de(conexion, Pantalla::CalendarioInformes)?;
        for (mes, anio) in &meses {
            let nombre = format!("informes-{anio}-{mes}");
            estado["fase"] = json!(nombre);
            escribir_control(control, estado)?;
            let plan = plan_calendario(&catalogo, alcance, mes, anio, edicion)?;
            let opciones_lote = OpcionesLote { conservar_filtros: false, evidencia: opciones.pdf_vacias, pausa: opciones.pausa };
            let mut carpeta = None;
            let mut relé = |evento: EventoLote| match evento {
                EventoLote::Carpeta(c) => carpeta = Some(c),
                EventoLote::Aviso(a) => emitir(Evento::Aviso(a)),
                _ => {}
            };
            lote::ejecutar(conexion, &catalogo, edicion, &plan, respaldo, &opciones_lote, cancelar, &mut relé)?;
            let carpeta = carpeta.ok_or_else(|| NO_IDENTIFICO.to_string())?;
            registrar_origen(estado, control, respaldo, raiz, &nombre, &carpeta, emitir)?;
            carpetas.push(carpeta);
        }
        Ok(())
    })();
    // `finally`: la consulta siguiente parte en Seguimiento; si no se puede volver, el aviso lo dirá.
    let _ = conexion.llamar("catalog", json!({"screen": "seguimiento"}), Duration::from_secs(75));
    match corrida {
        Ok(()) => Ok((carpetas.clone(), carpetas.len() == meses.len())),
        Err(e) => {
            if cancelar.load(Ordering::SeqCst) {
                return Err(CANCELADO.into());
            }
            emitir(Evento::Aviso(format!("{CALENDARIO_AVISO}{e}")));
            Ok((carpetas.clone(), false))
        }
    }
}

/// `run`: consulta, bitácoras, informes y ZIP en una carpeta nueva dentro de `destino`. No escribe en SITFA.
#[allow(clippy::too_many_arguments)]
pub fn ejecutar(
    conexion: &dyn Conexion,
    catalogo: &Catalogo,
    edicion: Edicion,
    plan: &PlanLote,
    alcance: &LoteBitacoras,
    destino: &Path,
    opciones: &Opciones,
    cancelar: &AtomicBool,
    emitir: &mut dyn FnMut(Evento),
) -> Result<Resultado, String> {
    let raiz = destino.join(nombre_carpeta());
    correr(conexion, catalogo, edicion, plan, alcance, Some(destino), &raiz, None, opciones, cancelar, emitir)
}

/// `retomar`: continúa un paquete cortado en `carpeta` («Paquete CSMP …»): no repite consultas validadas ni
/// bitácoras ya leídas y vuelve a crear el ZIP.
pub fn retomar(
    conexion: &dyn Conexion,
    catalogo: &Catalogo,
    edicion: Edicion,
    carpeta: &Path,
    opciones: &Opciones,
    cancelar: &AtomicBool,
    emitir: &mut dyn FnMut(Evento),
) -> Result<Resultado, String> {
    let raiz = carpeta.canonicalize().map_err(|_| CONTROL_AJENO.to_string())?;
    let mut control = leer_control(&raiz)?;
    let alcance = alcance_guardado(&raiz, &control.datos)?;
    let plan = plan_del_alcance(&alcance, catalogo, edicion)?;
    control.datos["estado"] = json!("EN_CURSO");
    let reintentos = control.datos["reintentos"].as_u64().unwrap_or(0) + 1;
    control.datos["reintentos"] = json!(reintentos);
    correr(conexion, catalogo, edicion, &plan, &alcance, None, &raiz, Some(control), opciones, cancelar, emitir)
}

/// `pendientes`: paquetes cortados en `destino` de los últimos `dias`, del más reciente al más antiguo.
pub fn pendientes(destino: &Path, dias: u32) -> Vec<(PathBuf, String)> {
    let limite = std::time::SystemTime::now()
        .checked_sub(Duration::from_secs(u64::from(dias) * 86_400))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let mut encontrados = Vec::new();
    for entrada in std::fs::read_dir(destino).into_iter().flatten().flatten() {
        if !entrada.file_name().to_string_lossy().starts_with(PREFIJO) {
            continue;
        }
        let carpeta = entrada.path();
        let control = carpeta.join("respaldo").join(PROCESO);
        let Ok(meta) = std::fs::metadata(&control) else { continue };
        let Ok(modificado) = meta.modified() else { continue };
        if modificado < limite {
            continue;
        }
        let Ok(estado) = std::fs::read(&control).map_err(|_| ()).and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|_| ())) else {
            continue;
        };
        if matches!(estado["estado"].as_str(), Some("EN_CURSO" | "INCOMPLETO")) {
            encontrados.push((modificado, carpeta, estado["fase"].as_str().unwrap_or("").to_string()));
        }
    }
    encontrados.sort_by_key(|e| std::cmp::Reverse(e.0));
    encontrados.into_iter().map(|(_, carpeta, fase)| (carpeta, fase)).collect()
}


#[cfg(test)]
pub(crate) mod pruebas {
    use super::*;
    use crate::descargador::sitfa_ficticio::SitfaFicticio;
    use crate::descargador::SeleccionLote;
    use base64::Engine;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    const PERSONAS: [(&str, &str, &str); 3] = [
        ("X-1-2026", "11111111-0", "Persona Ficticia 1"),
        ("X-2-2026", "11111112-0", "Persona Ficticia 2"),
        ("X-3-2026", "11111113-0", "Persona Ficticia 3"),
    ];

    /// Listado (con enlaces) o XLS (sin ellos) de tres personas ficticias, como el oráculo de Python.
    fn tabla(con_enlaces: bool, id: &str) -> String {
        let mut filas = String::new();
        for (n, (rit, rut, nombre)) in PERSONAS.iter().enumerate() {
            let (causa, ingreso) = (100 + n + 1, 200 + n + 1);
            let acciones = if con_enlaces {
                format!(
                    "<a onclick=\"ShowHistoria('901','{rit}','{causa}')\">Causa</a>\
                     <a onclick=\"ShowObservaciones('{causa}','901','{ingreso}','12','3','0')\">Obs</a>\
                     <a onclick=\"ShowInformesProgramados('{causa}','300','901','400','{rit}','01/03/2026',\
                     'PRM Centro Ficticio','{nombre}','6 Meses','3 Meses','{ingreso}','1')\">Inf</a>"
                )
            } else {
                String::new()
            };
            filas.push_str(&format!(
                "<tr><td>{rit}</td><td>{rut}</td><td>{nombre}</td><td>PRM Centro Ficticio</td><td>{acciones}</td></tr>"
            ));
        }
        format!("<table id=\"{id}\"><tr><td>RIT</td><td>RUT</td><td>Nombre</td><td>Derivacion</td><td>Acciones</td></tr>{filas}</table>")
    }

    /// Listado del calendario de informes: dos tablas y los RIT en el script, como SITFA.
    fn pagina_calendario() -> String {
        let filas: String = PERSONAS.iter().map(|(rit, _, nombre)| format!("<tr><td>{rit}</td><td>{nombre}</td></tr>")).collect();
        let tablas: String = ["TablaCalendarioSemana", "TablaCalendarioDia"]
            .iter()
            .map(|i| format!("<table id=\"{i}\">{filas}</table>"))
            .collect();
        let scripts: String = PERSONAS
            .iter()
            .map(|(rit, _, nombre)| format!("Ritxdias[dia] = \"{rit}\"; Nombresxdias[dia] = \"{nombre}\";"))
            .collect();
        format!("{tablas}<script>{scripts}</script>")
    }

    /// Bitácora sintética con una observación del centro; la identidad visible no siempre coincide.
    fn popup(params: &BTreeMap<String, String>) -> Vec<u8> {
        let cabecera = "<td>Tipo Observación</td><td>Etapa</td><td>Fecha Centro</td><td>Usuario Centro</td>\
                        <td>Observación Centro</td><td>Fecha Tribunal</td><td>Usuario Tribunal</td>\
                        <td>Observación Tribunal</td><td>Acción</td>";
        let fila = "<td>Al Tribunal</td><td>Cumplimiento</td><td>12/08/2026 10:13</td><td>Usuario ficticio</td>\
                    <td>Texto recortado</td><td>---</td><td>---</td><td>---</td><td><img id=\"44\"></td>";
        let mut campos: String = params
            .iter()
            .filter(|(k, _)| !["tipo_popUp", "RIT_Causa", "GLS_Nombre", "GLS_Centro"].contains(&k.as_str()))
            .map(|(k, v)| format!("<input type=\"hidden\" name=\"{k}\" value=\"{v}\">"))
            .collect();
        campos.push_str("<input type=\"hidden\" name=\"tipo_popUp\" value=\"\">");
        campos.push_str("<input type=\"hidden\" name=\"RIT_Causa\" value=\"X-1-2026\">");
        campos.push_str("<input type=\"hidden\" name=\"GLS_Nombre\" value=\"Persona ficticia\">");
        campos.push_str("<input type=\"hidden\" name=\"GLS_Centro\" value=\"PRM Centro ficticio\">");
        let ocultos = "<input type=\"hidden\" name=\"HOBS_Centro_44\" value=\"Texto completo\nSegunda línea\">\
                       <input type=\"hidden\" name=\"HOBS_Tribunal_44\" value=\"---\">\
                       <input type=\"hidden\" name=\"HCOD_Estado_44\" value=\"6\">\
                       <input type=\"hidden\" name=\"HFUN_Tribunal_44\" value=\"\">\
                       <input type=\"hidden\" name=\"HFLG_Envio_44\" value=\"1\">";
        format!(
            "<html><form name=\"InformesPpalForm\" action=\"/SITFAWEB/InformesDAction.do\">{campos}\
             <table id=\"TablaInforme\"><tr>{cabecera}</tr><tr>{fila}</tr>{ocultos}</table></form></html>"
        )
        .into_bytes()
    }

    /// Calendario del ingreso: formulario con la identidad del enlace y dos proyecciones.
    fn calendario(args: &[String]) -> Vec<u8> {
        let campos = format!(
            "<input type=\"hidden\" name=\"COD_Tribunal\" value=\"{}\">\
             <input type=\"hidden\" name=\"COD_Tribunal_sel\" value=\"{}\">\
             <input type=\"hidden\" name=\"CRR_IdCausa\" value=\"{}\">\
             <input type=\"hidden\" name=\"ID_Ingreso\" value=\"{}\">",
            args[2], args[2], args[0], args[10]
        );
        let filas = "<tr><td>01/03/2026</td><td>15/03/2026</td><td>ENVIADO</td></tr>\
                     <tr><td>01/04/2026</td><td>---</td><td>PENDIENTE</td></tr>";
        format!(
            "<html><form name=\"InformesPpalForm\" action=\"/SITFAWEB/InformesDAction.do\">{campos}\
             <table><tr><td>Fecha Proyección</td><td>Fecha Informe</td><td>Estado</td></tr>{filas}</table>\
             <span>Plazo remisión: 6 meses</span></form></html>"
        )
        .into_bytes()
    }

    pub(crate) fn catalogo_json_publico(pantalla: &str) -> Value {
        catalogo_json(pantalla)
    }

    fn catalogo_json(pantalla: &str) -> Value {
        let tribunales = json!({"901": "Juzgado de Familia Ficticio Norte", "902": "Juzgado de Familia Ficticio Sur"});
        match pantalla {
            "seguimiento" => json!({
                "pantalla": "seguimiento", "tribunales": tribunales,
                "modalidades": {"1": "Residencia", "2": "Ambulatorio", "3": "FAE", "4": "DCE"},
                "informes": {"1": "Por vencer (a 25 días)", "2": "Vencidos", "3": "Recibidos"},
                "medidas": {"4": "Por vencer (a 45 días)", "5": "Vencidos"},
                "pestanas": ["Espera", "Cumplimiento", "Informes", "Egresados"],
            }),
            _ => json!({
                "pantalla": pantalla, "tribunales": tribunales,
                "meses": {"1": "Enero", "2": "Febrero", "3": "Marzo", "4": "Abril", "5": "Mayo", "6": "Junio",
                          "7": "Julio", "8": "Agosto", "9": "Septiembre", "10": "Octubre", "11": "Noviembre", "12": "Diciembre"},
                "anios": {"2026": "2026", "2027": "2027"},
            }),
        }
    }

    /// SITFA ficticio del flujo: `SitfaFicticio` responde `prepare`; los listados, bitácoras y calendarios son los
    /// mismos documentos sintéticos que sirve el oráculo de Python.
    pub(crate) struct Doble {
        seguimiento: SitfaFicticio,
        calendario: SitfaFicticio,
        pantalla: RefCell<&'static str>,
        pares: RefCell<BTreeMap<String, String>>,
        mes: RefCell<String>,
        /// Cancela el flujo en la primera descarga de las consultas (caso cancelado_tras_consultas).
        cancelar_tras_consultas: bool,
        cancelar: Option<std::sync::Arc<AtomicBool>>,
        /// Ingreso cuya primera bitácora falla (caso itacora_fallida).
        fallar_bitacora: RefCell<Option<String>>,
    }

    impl Doble {
        pub(crate) fn nuevo() -> Self {
            Self {
                seguimiento: SitfaFicticio::nuevo("seguimiento", &[]),
                calendario: SitfaFicticio::nuevo("calendario_informes", &[]),
                pantalla: RefCell::new("seguimiento"),
                pares: RefCell::new(BTreeMap::new()),
                mes: RefCell::new(String::new()),
                cancelar_tras_consultas: false,
                cancelar: None,
                fallar_bitacora: RefCell::new(None),
            }
        }

        fn documento(&self, con_enlaces: bool) -> Vec<u8> {
            let pantalla = *self.pantalla.borrow();
            let pares = self.pares.borrow();
            let formulario: String = pares
                .iter()
                .map(|(k, v)| {
                    // La paginación del listado va con valores; prepare la entrega vacía para que la ponga la página.
                    let valor = if k.starts_with("NUM_Pagina") || k.starts_with("NUM_Total") { "1" } else { v.as_str() };
                    if ["COD_Tribunal_sel", "TIP_Consulta", "COD_Mes_sel", "COD_Anio_Sel"].contains(&k.as_str()) {
                        format!("<select name=\"{k}\"><option value=\"{valor}\" selected>{valor}</option></select>")
                    } else {
                        format!("<input name=\"{k}\" value=\"{valor}\">")
                    }
                })
                .collect();
            if pantalla.starts_with("calendario_") {
                format!(
                    "<html><body><form name=\"InformesPpalForm\" method=\"post\" action=\"/SITFAWEB/InformesDAction.do\">\
                     {formulario}{}</form><script>function ShowExcel(){{window.open(\"https://familia.pjud.cl/sitfa/reportes/u1_InfoPorVencer.xls\");}}</script></body></html>",
                    pagina_calendario()
                )
                .into_bytes()
            } else {
                let id = match pantalla {
                    "litigantes" => "tablaLitigantes",
                    _ => "tablaCumplimiento",
                };
                format!(
                    "<html><body><form name=\"InformesPpalForm\" method=\"post\" action=\"/SITFAWEB/InformesDAction.do\">\
                     {formulario}{}</form><script>function ShowExcel(){{window.open(\"https://familia.pjud.cl/sitfa/reportes/u1_listdemo.xls\");}}</script></body></html>",
                    tabla(con_enlaces, id)
                )
                .into_bytes()
            }
        }
    }

    fn digest(datos: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(datos))
    }

    impl Conexion for Doble {
        fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
            match comando {
                "capabilities" => Ok(json!({
                    "tipo": "CSMP_RUS_CAPACIDADES", "version": "2.9.0", "escritura_rus": false,
                    "protocolos": {"bitacoras_lectura": 1, "descarga_conjunta": 1, "pdf_seleccion": 1,
                                   "pdf_copia_fija": 1, "calendario_lectura": 1, "historia_lectura": 1,
                                   "historico_lectura": 2},
                })),
                "lock" | "unlock" => Ok(json!({"ok": true})),
                "catalog" => {
                    let pantalla = match payload["screen"].as_str() {
                        Some("calendario_informes") => "calendario_informes",
                        Some("calendario_medidas") => "calendario_medidas",
                        Some("litigantes") => "litigantes",
                        _ => "seguimiento",
                    };
                    *self.pantalla.borrow_mut() = pantalla;
                    Ok(catalogo_json(pantalla))
                }
                "prepare" => {
                    let base = match *self.pantalla.borrow() {
                        "calendario_informes" => &self.calendario,
                        "calendario_medidas" => &self.calendario,
                        _ => &self.seguimiento,
                    };
                    let respuesta = base.llamar("prepare", payload.clone(), plazo)?;
                    let mut pares = BTreeMap::new();
                    for par in respuesta["pairs"].as_array().into_iter().flatten() {
                        if let Some([Value::String(k), Value::String(v)]) = par.as_array().map(Vec::as_slice) {
                            pares.insert(k.clone(), v.clone());
                        }
                    }
                    if let Some(mes) = payload["selection"]["month"].as_str().filter(|m| !m.is_empty()) {
                        *self.mes.borrow_mut() = mes.to_string();
                    }
                    *self.pares.borrow_mut() = pares;
                    Ok(respuesta)
                }
                "search" | "evidence_open" => {
                    for par in payload["pairs"].as_array().into_iter().flatten() {
                        if let Some([Value::String(k), Value::String(v)]) = par.as_array().map(Vec::as_slice) {
                            self.pares.borrow_mut().insert(k.clone(), v.clone());
                        }
                    }
                    let cuerpo = self.documento(comando == "search");
                    let mut respuesta = json!({"status": 200, "type": "text/html",
                        "body": base64::engine::general_purpose::STANDARD.encode(&cuerpo)});
                    if comando == "evidence_open" {
                        respuesta["digest"] = json!(digest(&cuerpo));
                    }
                    Ok(respuesta)
                }
                "download" => {
                    if self.cancelar_tras_consultas {
                        if let Some(c) = &self.cancelar {
                            c.store(true, Ordering::SeqCst);
                        }
                    }
                    let pantalla = *self.pantalla.borrow();
                    let cuerpo = if pantalla.starts_with("calendario_") {
                        let mes: u32 = self.mes.borrow().parse().unwrap_or(1);
                        let filas: String = PERSONAS
                            .iter()
                            .map(|(rit, _, nombre)| format!("<tr><td>{rit}</td><td>{nombre}</td><td>15/{mes:02}/2026</td></tr>"))
                            .collect();
                        format!("<html><meta charset=\"utf-8\"><table><tr><td>RIT</td><td>Nombre</td><td>Fecha Vencimiento</td></tr>{filas}</table></html>")
                    } else {
                        format!("<html><meta charset=\"utf-8\">{}</html>", tabla(false, "tablaCumplimiento"))
                    };
                    Ok(json!({"status": 200, "type": "application/vnd.ms-excel",
                              "body": base64::engine::general_purpose::STANDARD.encode(cuerpo)}))
                }
                "diary_open" => {
                    let params: BTreeMap<String, String> = serde_json::from_value(payload["params"].clone())
                        .map_err(|e| e.to_string())?;
                    let falla = self.fallar_bitacora.borrow().as_deref() == params.get("ID_Ingreso").map(String::as_str);
                    if falla {
                        *self.fallar_bitacora.borrow_mut() = None;
                        return Err("La bitácora no cargó.".into());
                    }
                    let cuerpo = popup(&params);
                    Ok(json!({"status": 200, "type": "text/html",
                              "body": base64::engine::general_purpose::STANDARD.encode(&cuerpo),
                              "digest": digest(&cuerpo), "params": payload["params"],
                              "startedAt": "2026-10-08T12:00:00Z"}))
                }
                "calendar_open" => {
                    let args: Vec<String> = serde_json::from_value(payload["args"].clone()).map_err(|e| e.to_string())?;
                    let cuerpo = calendario(&args);
                    Ok(json!({"status": 200, "type": "text/html",
                              "body": base64::engine::general_purpose::STANDARD.encode(&cuerpo),
                              "digest": digest(&cuerpo), "args": payload["args"],
                              "startedAt": "2026-10-08T12:00:00Z"}))
                }
                "history_open" | "historico_open" => Err("El oráculo no sirve historia ni histórico.".into()),
                otro => Err(format!("comando no previsto en el doble: {otro}")),
            }
        }
    }

    fn datos() -> Value {
        serde_json::from_str(include_str!("../../tests/datos/flujo_paquete.json")).unwrap()
    }

    fn carpeta(nombre: &str) -> PathBuf {
        static SERIE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serie = SERIE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("flow-flujo-{}-{nombre}-{serie}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        base
    }

    fn reconstruir(destino: &Path, archivos: &Value) {
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    /// Fecha y hora de las carpetas e historial: `<FECHA>`; separadores a `/` para comparar Windows y Linux.
    fn normalizar(texto: &str) -> String {
        static RELOJ: std::sync::LazyLock<regex::Regex> =
            std::sync::LazyLock::new(|| regex::Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}-\d{2}-\d{2}(-\d{6})?").unwrap());
        static MARCA: std::sync::LazyLock<regex::Regex> =
            std::sync::LazyLock::new(|| regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[+-]\d{2}:\d{2} ").unwrap());
        let texto = MARCA.replace(texto, "");
        RELOJ.replace_all(&texto, "<FECHA>").replace('\\', "/")
    }

    fn volcado_proceso(carpeta: &Path) -> Value {
        let Ok(crudo) = std::fs::read_to_string(carpeta.join("respaldo").join(PROCESO)) else {
            return json!({"invalido": true});
        };
        let Ok(datos) = serde_json::from_str::<Value>(&crudo) else { return json!({"invalido": true}) };
        if !datos.is_object() {
            return json!({"invalido": true});
        }
        let carpetas: BTreeMap<String, String> = datos["carpetas"]
            .as_object()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), normalizar(v.as_str().unwrap_or("")))).collect())
            .unwrap_or_default();
        json!({
            "estado": datos["estado"], "fase": datos["fase"], "reintentos": datos["reintentos"].as_u64().unwrap_or(0),
            "paquete": normalizar(datos["paquete"].as_str().unwrap_or("")),
            "carpetas": carpetas,
            "historial": datos["historial"].as_array().into_iter().flatten()
                .map(|h| normalizar(h.as_str().unwrap_or(""))).collect::<Vec<_>>(),
            "opciones": datos["opciones"],
        })
    }

    fn zip(carpeta: &Path) -> Result<Value, String> {
        let ruta = carpeta.join(ZIP);
        let archivo = std::fs::File::open(&ruta).map_err(|e| format!("sin ZIP: {e}"))?;
        let mut zip = zip::ZipArchive::new(archivo).map_err(|e| e.to_string())?;
        let manifiesto: Value = {
            use std::io::Read;
            let mut texto = String::new();
            zip.by_name("manifest.json").map_err(|e| e.to_string())?.read_to_string(&mut texto).map_err(|e| e.to_string())?;
            serde_json::from_str(&texto).map_err(|e| e.to_string())?
        };
        let mut tablas = serde_json::Map::new();
        for nombre in ["consultas", "registros", "bitacoras", "calendarios", "historia_tramites", "documentos",
                       "incidencias", "escritos", "historico_proteccion"] {
            let filas = manifiesto["files"].as_array().into_iter().flatten()
                .find(|f| f["table"] == nombre).map(|f| f["rows"].clone()).unwrap_or_else(|| json!(0));
            tablas.insert(nombre.into(), filas);
        }
        let mut codigos: Vec<String> = Vec::new();
        {
            use std::io::Read;
            if let Ok(mut csv) = zip.by_name("data/incidencias.csv") {
                let mut texto = String::new();
                if csv.read_to_string(&mut texto).is_ok() {
                    let mut lineas = texto.lines();
                    if let Some(cabecera) = lineas.next() {
                        let columnas: Vec<&str> = cabecera.split(',').collect();
                        if let Some(i) = columnas.iter().position(|c| c.trim_matches('"') == "code") {
                            for linea in lineas {
                                if let Some(v) = linea.split(',').nth(i) {
                                    codigos.push(v.trim_matches('"').to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        codigos.sort();
        codigos.dedup();
        let estados = manifiesto["table_states"].clone();
        Ok(json!({
            "nombre": ZIP, "tablas": tablas, "estados": estados,
            "consolidado": manifiesto.get("consolidated").is_some_and(|v| !v.is_null()),
            "schema": manifiesto["schema_version"], "incidencias": codigos,
        }))
    }

    fn opciones(caso: &Value) -> Opciones {
        let mut op = Opciones {
            fecha_informes: chrono::NaiveDate::from_ymd_opt(2026, 10, 14).unwrap(),
            pausa: Duration::ZERO,
            ..Default::default()
        };
        if let Some(o) = caso["opciones"].as_object() {
            if let Some(v) = o.get("calendars").and_then(Value::as_bool) { op.calendarios = v; }
            if let Some(v) = o.get("historia").and_then(Value::as_bool) { op.historia = v; }
            if let Some(v) = o.get("historico").and_then(Value::as_bool) { op.historico = v; }
            if let Some(v) = o.get("informes").and_then(Value::as_bool) { op.informes = v; }
            if let Some(v) = o.get("consolidated").and_then(Value::as_bool) { op.consolidado = v; }
        }
        op
    }

    /// Corre un caso del oráculo y devuelve lo observable con la forma del JSON de Python.
    fn correr(caso: &Value) -> (Result<Resultado, String>, Vec<String>, Option<Value>, Option<Value>) {
        let nombre = caso["caso"].as_str().unwrap();
        let base = carpeta(nombre);
        let mut doble = Doble::nuevo();
        doble.cancelar_tras_consultas = caso["caso"] == "cancelado_tras_consultas";
        let cancelar = std::sync::Arc::new(AtomicBool::new(false));
        doble.cancelar = Some(cancelar.clone());
        if caso["caso"] == "bitacora_fallida" {
            *doble.fallar_bitacora.borrow_mut() = Some("201".into());
        }
        let catalogo: Catalogo = serde_json::from_value(catalogo_json("seguimiento")).unwrap();
        // La pantalla que no es Seguimiento se rechaza antes de planificar: se le pasa el catálogo equivocado.
        let ejecucion: Catalogo = if caso["otra_pantalla"].as_bool().unwrap_or(false) {
            serde_json::from_value(catalogo_json("calendario_informes")).unwrap()
        } else {
            catalogo.clone()
        };
        let alcance = LoteBitacoras {
            tribunales: vec!["901".into()], modalidades: vec!["2".into()], pestanas: vec!["Cumplimiento".into()],
        };
        let mut eventos: Vec<Evento> = Vec::new();
        let mut emitir = |e: Evento| eventos.push(e);
        let op = opciones(caso);
        let resultado = if caso["retomar"].as_bool().unwrap_or(false) {
            let entrada = base.join("entrada");
            reconstruir(&entrada, &caso["entrada"]);
            let paquete = std::fs::read_dir(&entrada).unwrap().flatten().map(|e| e.path())
                .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(PREFIJO))).unwrap();
            retomar(&doble, &ejecucion, Edicion::Integral, &paquete, &op, cancelar.as_ref(), &mut emitir)
        } else {
            let lote = SeleccionLote {
                tribunales: vec!["901".into()], pestanas: vec!["Cumplimiento".into()], modalidades: vec!["2".into()],
                informes: vec!["1".into()], medidas: vec![TODAS_LAS_MEDIDAS.into()], ..Default::default()
            };
            let plan = catalogo.planificar(Edicion::Integral, &lote).unwrap();
            let destino = base.join("destino");
            std::fs::create_dir_all(&destino).unwrap();
            ejecutar(&doble, &ejecucion, Edicion::Integral, &plan, &alcance, &destino, &op, cancelar.as_ref(), &mut emitir)
        };
        let fases = eventos.iter().filter_map(|e| match e {
            Evento::Fase(f) => Some(f.clone()),
            _ => None,
        }).collect::<Vec<_>>();
        let buscar = |dir: &Path| {
            std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path())
                .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(PREFIJO)))
        };
        let paquete = match &resultado {
            Ok(r) => r.paquete.parent().map(Path::to_path_buf),
            Err(_) => buscar(&base.join("destino")).or_else(|| buscar(&base.join("entrada"))),
        };
        let proceso = paquete.as_ref().map(|p| volcado_proceso(p));
        let volcado_zip = paquete.as_ref().and_then(|p| zip(p).ok());
        let _ = std::fs::remove_dir_all(&base);
        (resultado, fases, proceso, volcado_zip)
    }

    /// Diferencias conocidas del escritor de paquetes (paquete.rs, fuera de este encargo): el manifiesto de Rust no
    /// lista escritos/historico_proteccion cuando el paquete no los trae, y agrega CONSOLIDADO_OMITIDO cuando el
    /// lote no es apto. Se comparan las tablas y los estados comunes y el resto de las incidencias.
    fn normalizar_zip(rust: &Value, python: &Value) -> (Value, Value) {
        let depurar = |v: &Value, claves: Option<Vec<String>>| {
            let mut v = v.clone();
            if let Some(tablas) = v["tablas"].as_object_mut() {
                for (_, fila) in tablas.iter_mut() {
                    if fila.is_null() {
                        *fila = json!(0);
                    }
                }
            }
            if let Some(estados) = v["estados"].as_object_mut() {
                if let Some(claves) = &claves {
                    estados.retain(|k, _| claves.contains(k));
                }
            }
            if let Some(codigos) = v["incidencias"].as_array_mut() {
                let extra = codigos.iter().filter(|c| *c == "CONSOLIDADO_OMITIDO").count();
                codigos.retain(|c| c != "CONSOLIDADO_OMITIDO");
                if extra > 0 {
                    let filas = v["tablas"]["incidencias"].as_u64().unwrap_or(0);
                    v["tablas"]["incidencias"] = json!(filas.saturating_sub(extra as u64));
                }
            }
            v
        };
        let comunes: Option<Vec<String>> = rust["estados"].as_object().map(|r| {
            r.keys().filter(|k| python["estados"].get(*k).is_some()).cloned().collect()
        });
        (depurar(rust, comunes.clone()), depurar(python, comunes))
    }

    fn comparar_caso(caso: &Value) -> Vec<String> {
        let nombre = caso["caso"].as_str().unwrap();
        let (resultado, fases, proceso, volcado) = correr(caso);
        let mut dif = Vec::new();
        match (&resultado, caso["error"].as_str()) {
            (Err(e), Some(p)) if e == p => {}
            (Err(e), Some(p)) => dif.push(format!("{nombre}: error distinto · Rust «{e}» · Python «{p}»")),
            (Err(e), None) => dif.push(format!("{nombre}: Rust falló («{e}») y Python no")),
            (Ok(_), Some(p)) => dif.push(format!("{nombre}: Python falló («{p}») y Rust no")),
            (Ok(_), None) => {}
        }
        if fases != caso["fases"].as_array().unwrap().iter().map(|f| f.as_str().unwrap().to_string()).collect::<Vec<_>>() {
            dif.push(format!("{nombre}.fases: Rust {fases:?} · Python {}", caso["fases"]));
        }
        let proceso_rust = proceso.unwrap_or(Value::Null);
        if &proceso_rust != &caso["proceso"] {
            dif.push(format!("{nombre}.proceso: Rust {proceso_rust} · Python {}", caso["proceso"]));
        }
        if let Ok(r) = &resultado {
            let esperado = &caso["resultado"];
            if Some(r.parcial) != esperado["parcial"].as_bool() {
                dif.push(format!("{nombre}.parcial: Rust {} · Python {}", r.parcial, esperado["parcial"]));
            }
            let historia = r.historia.as_ref().and_then(|h| h["estado"].as_str().map(str::to_string));
            if historia.as_deref() != esperado["historia"].as_str() {
                dif.push(format!("{nombre}.historia: Rust {historia:?} · Python {}", esperado["historia"]));
            }
            match &volcado {
                Some(obtenido) => {
                    let (a, b) = normalizar_zip(obtenido, &caso["zip"]);
                    if a != b {
                        dif.push(format!("{nombre}.zip: Rust {a} · Python {b}"));
                    }
                }
                None => dif.push(format!("{nombre}: el ZIP de Rust no se pudo leer")),
            }
        }
        dif
    }

    /// E-Q9 · El flujo completo de Rust contra `flujo_paquete.run`/`retomar` de Python sobre el mismo mundo sintético.
    #[test]
    fn flujo_paquete_igual_a_python() {
        let datos = datos();
        let casos = datos["casos"].as_array().unwrap();
        let (mut errores, mut fases) = (0usize, 0usize);
        let mut diferencias = Vec::new();
        for caso in casos {
            diferencias.extend(comparar_caso(caso));
            if caso["error"].is_string() {
                errores += 1;
            }
            fases += caso["fases"].as_array().unwrap().len();
        }
        assert!(diferencias.is_empty(), "{}", diferencias.join("\n"));
        assert_eq!((casos.len(), errores, fases), (12, 4, 26), "conteos del oráculo");
        println!("oraculo flujo de paquete: {} casos, {errores} errores, {fases} fases", casos.len());
    }

    /// Alterar una fase, un mensaje de error y una tabla del ZIP debe hacer fallar el comparador.
    #[test]
    fn oraculo_de_flujo_paquete_detecta_alteraciones() {
        let datos = datos();
        let mut caso = datos["casos"][0].clone();
        caso["fases"][0] = json!("otra");
        assert!(comparar_caso(&caso).iter().any(|d| d.contains("fases")), "fase alterada no detectada");
        let mut caso = datos["casos"].as_array().unwrap().iter().find(|c| c["caso"] == "otra_pantalla").unwrap().clone();
        caso["error"] = json!("otro mensaje");
        assert!(comparar_caso(&caso).iter().any(|d| d.contains("error")), "mensaje alterado no detectado");
        let mut caso = datos["casos"][0].clone();
        caso["zip"]["tablas"]["bitacoras"] = json!(99);
        assert!(comparar_caso(&caso).iter().any(|d| d.contains(".zip")), "tabla alterada no detectada");
    }

    /// Retomar un paquete cuyo `proceso.json` trae `respaldo/<fecha>` o `respaldo\\<fecha>` debe resolver la carpeta y
    /// reutilizar las consultas validadas en cualquier sistema (Linux incluido), como `_consultas_validadas` de Python.
    #[test]
    fn retomar_resuelve_separadores_y_reutiliza_consultas() {
        let datos = datos();
        let caso = datos["casos"].as_array().unwrap().iter().find(|c| c["caso"] == "retomado").unwrap().clone();
        for (nombre, separador) in [("barra", "/"), ("inversa", "\\")] {
            let base = carpeta(&format!("retomar-{nombre}"));
            let entrada = base.join("entrada");
            reconstruir(&entrada, &caso["entrada"]);
            let paquete = std::fs::read_dir(&entrada).unwrap().flatten().map(|e| e.path())
                .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(PREFIJO)))
                .unwrap();
            let control = paquete.join("respaldo").join(PROCESO);
            let mut proceso: Value = serde_json::from_slice(&std::fs::read(&control).unwrap()).unwrap();
            for (_, valor) in proceso["carpetas"].as_object_mut().unwrap().iter_mut() {
                *valor = json!(valor.as_str().unwrap_or("").replace('/', separador));
            }
            std::fs::write(&control, serde_json::to_vec_pretty(&proceso).unwrap()).unwrap();
            let doble = Doble::nuevo();
            let catalogo: Catalogo = serde_json::from_value(catalogo_json("seguimiento")).unwrap();
            let (mut eventos, cancelar) = (Vec::new(), AtomicBool::new(false));
            let resultado = retomar(
                &doble, &catalogo, Edicion::Integral, &paquete, &Opciones { pausa: Duration::ZERO, ..Default::default() },
                &cancelar, &mut |e| eventos.push(e),
            );
            assert!(resultado.is_ok(), "{separador}: {:?}", resultado.err());
            let volcado = volcado_proceso(&paquete);
            let historial = volcado["historial"].as_array().cloned().unwrap_or_default();
            assert!(
                historial.iter().any(|h| h.as_str().unwrap_or("").contains("consultas reutilizadas del intento anterior")),
                "{separador}: {historial:?}"
            );
            let _ = std::fs::remove_dir_all(&base);
        }
    }
}

#[cfg(test)]
mod pruebas_flujo {
    use super::*;
    use crate::descargador::SeleccionLote;

    /// E-Q9 · El flujo deja una sola carpeta «Paquete CSMP …» con `respaldo/` (las fuentes) y el ZIP, y nada suelto
    /// en el destino; el proceso queda COMPLETO y el resultado informa la ruta.
    #[test]
    fn deja_una_sola_carpeta_con_las_fuentes_y_el_zip() {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/flujo_paquete.json")).unwrap();
        let caso = &datos["casos"][0];
        let base = std::env::temp_dir().join(format!("flow-flujo-vista-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let catalogo: Catalogo = serde_json::from_value(super::pruebas::catalogo_json_publico("seguimiento")).unwrap();
        let lote = SeleccionLote {
            tribunales: vec!["901".into()], pestanas: vec!["Cumplimiento".into()], modalidades: vec!["2".into()],
            informes: vec!["1".into()], medidas: vec![TODAS_LAS_MEDIDAS.into()], ..Default::default()
        };
        let plan = catalogo.planificar(Edicion::Integral, &lote).unwrap();
        let alcance = LoteBitacoras {
            tribunales: vec!["901".into()], modalidades: vec!["2".into()], pestanas: vec!["Cumplimiento".into()],
        };
        let doble = super::pruebas::Doble::nuevo();
        let mut eventos = Vec::new();
        let cancelar = AtomicBool::new(false);
        let resultado = ejecutar(
            &doble, &catalogo, Edicion::Integral, &plan, &alcance, &base, &Opciones { pausa: Duration::ZERO, ..Default::default() },
            &cancelar, &mut |e| eventos.push(e),
        )
        .unwrap();
        let mut entradas: Vec<PathBuf> = std::fs::read_dir(&base).unwrap().flatten().map(|e| e.path()).collect();
        entradas.retain(|p| p.is_dir());
        assert_eq!(entradas.len(), 1, "una sola carpeta en el destino: {entradas:?}");
        let paquete = &entradas[0];
        assert!(paquete.file_name().unwrap().to_string_lossy().starts_with(PREFIJO));
        assert!(paquete.join("respaldo").is_dir());
        assert!(paquete.join(ZIP).is_file());
        assert_eq!(paquete.join(ZIP), resultado.paquete);
        assert!(!resultado.parcial);
        let proceso: Value = serde_json::from_slice(&std::fs::read(paquete.join("respaldo").join(PROCESO)).unwrap()).unwrap();
        assert_eq!(proceso["estado"], "COMPLETO");
        assert!(eventos.iter().any(|e| matches!(e, Evento::Paquete(_))));
        let _ = caso;
        let _ = std::fs::remove_dir_all(&base);
    }
}
