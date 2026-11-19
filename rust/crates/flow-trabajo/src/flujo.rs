//! «Consultar y crear paquete»: como `descargador/flujo_paquete.py::run`. Consulta Seguimiento con las lecturas por
//! ingreso activas (las bitácoras siempre, como `DiaryRunner`), el calendario de informes por vencer de los próximos
//! 60 días si hay Cumplimiento (`_informes`) y al final `Captura_CSMP.zip`, todo dentro de una carpeta
//! «Paquete CSMP <fecha>» con los originales verificados en `respaldo/` y el avance en `respaldo/proceso.json`.
//!
//! El lote de Seguimiento y cada mes del calendario son lotes separados dentro de `respaldo/` (un lote tiene una sola
//! pantalla). Retomar reutiliza los lotes validados y continúa los cortados; el estado final es COMPLETO o PARCIAL, e
//! INCOMPLETO si el proceso se detuvo antes del ZIP.

use crate::exportar::{self, Lote as LoteExportable, Opciones};
use chrono::{Datelike, NaiveDate};
use flow_captura::{
    lotes::{self, Conexion, Lote, Resumen},
    recoleccion,
    sitfa::Seleccion,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub const PROCESO: &str = "proceso.json";
const RESPALDO: &str = "respaldo";

/// Avance del paquete (`respaldo/proceso.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proceso {
    /// EN_CURSO, COMPLETO, PARCIAL o INCOMPLETO.
    pub estado: String,
    pub fase: String,
    /// Fase → carpeta del lote, relativa a la carpeta del paquete (siempre dentro de `respaldo/`).
    pub carpetas: BTreeMap<String, String>,
    /// Alcance pedido: el lote de Seguimiento con sus lecturas.
    pub seleccion: Option<Lote>,
    #[serde(default)]
    pub reintentos: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paquete: Option<String>,
    /// Historial anónimo del paquete (E3D-05, Mulchén): instante, fase, decisión sobre cada lote, error literal y resumen
    /// final. Sin nombres, RUT ni rutas personales; sirve para diagnosticar un corte compartiendo solo este archivo.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub historial: Vec<String>,
}

/// Agrega un evento al historial (los últimos 200).
fn anotar(proceso: &mut Proceso, evento: impl AsRef<str>) {
    proceso.historial.push(format!("{} {}", chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z"), evento.as_ref()));
    let sobra = proceso.historial.len().saturating_sub(200);
    proceso.historial.drain(..sobra);
}

/// Preferencias del Descargador que usa el flujo (`descargador/preferencias.py`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferencias {
    pub leer_historia: bool,
    pub leer_calendarios: bool,
    pub historia_en_paquete: bool,
    pub informes_por_vencer: bool,
    pub meses_atras: u32,
    pub pdf_sin_registros: bool,
    pub pausa_ms: u64,
}

impl Default for Preferencias {
    fn default() -> Self {
        Self {
            leer_historia: false,
            leer_calendarios: false,
            historia_en_paquete: true,
            informes_por_vencer: true,
            meses_atras: 4,
            pdf_sin_registros: true,
            pausa_ms: 1000,
        }
    }
}

#[derive(Debug)]
pub struct Resultado {
    pub carpeta: PathBuf,
    pub paquete: PathBuf,
    pub parcial: bool,
    pub avisos: Vec<String>,
    pub bitacoras: String,
}

/// Lo que el flujo pide a SITFA. En Chrome es [`EnChrome`]; las pruebas usan lotes ficticios.
pub trait Sitfa {
    /// Ejecuta (o retoma) un lote en su carpeta.
    fn lote(&self, lote: &Lote, carpeta: &Path, retomar: bool) -> Result<Resumen, String>;
    /// Abre la pantalla del calendario de informes por vencer y entrega sus opciones.
    fn catalogo_informes(&self) -> Result<Value, String>;
    /// Vuelve a Seguimiento (sin error si no se puede).
    fn volver(&self);
    fn cancelado(&self) -> bool;
    fn avance(&self, mensaje: String);
}

/// SITFA a través de la extensión de Chrome, con el catálogo de Seguimiento ya cargado.
pub struct EnChrome<'a> {
    pub conexion: &'a dyn Conexion,
    pub catalogo: &'a Value,
    pub cancelar: &'a AtomicBool,
    pub progreso: &'a dyn Fn(String),
    pub informes: std::sync::Mutex<Option<Value>>,
}

impl Sitfa for EnChrome<'_> {
    fn lote(&self, lote: &Lote, carpeta: &Path, retomar: bool) -> Result<Resumen, String> {
        let informes = self.informes.lock().map_err(|_| "Estado interno no disponible.")?.clone();
        let catalogo = match lote.consultas.first() {
            Some(s) if s.screen == "calendario_informes" => informes.ok_or("Faltan las opciones del calendario de informes.")?,
            _ => self.catalogo.clone(),
        };
        lotes::ejecutar(self.conexion, &catalogo, lote, carpeta, retomar, self.cancelar, self.progreso)
    }
    fn catalogo_informes(&self) -> Result<Value, String> {
        let c = lotes::catalogo(self.conexion, "calendario_informes")?;
        *self.informes.lock().map_err(|_| "Estado interno no disponible.")? = Some(c.clone());
        Ok(c)
    }
    fn volver(&self) {
        let _ = self.conexion.llamar("catalog", json!({"screen":"seguimiento"}), lotes::PLAZO);
    }
    fn cancelado(&self) -> bool {
        self.cancelar.load(Ordering::SeqCst)
    }
    fn avance(&self, mensaje: String) {
        (self.progreso)(mensaje)
    }
}

/// Meses (mes, año) del calendario que cubren desde hoy hasta hoy + 60 días: a veces dos, a veces tres.
pub fn meses_informes(hoy: NaiveDate) -> Vec<(String, String)> {
    let fin = hoy + chrono::Duration::days(60);
    let mut actual = hoy.with_day(1).unwrap();
    let mut meses = Vec::new();
    while actual <= fin {
        meses.push((actual.month().to_string(), actual.year().to_string()));
        actual = actual.checked_add_months(chrono::Months::new(1)).unwrap();
    }
    meses
}

fn escribir(raiz: &Path, proceso: &Proceso) -> Result<(), String> {
    let ruta = raiz.join(RESPALDO).join(PROCESO);
    let temporal = ruta.with_extension("tmp");
    let datos = serde_json::to_vec_pretty(proceso).map_err(|_| "No se pudo preparar el avance del paquete.")?;
    std::fs::write(&temporal, datos)
        .and_then(|_| std::fs::rename(&temporal, &ruta))
        .map_err(|_| "No se pudo guardar el avance del paquete (respaldo/proceso.json).".into())
}

fn leer(raiz: &Path) -> Result<Proceso, String> {
    let texto = std::fs::read(raiz.join(RESPALDO).join(PROCESO))
        .map_err(|_| "Elige una carpeta «Paquete CSMP …» creada por «Consultar y crear paquete».")?;
    let proceso: Proceso = serde_json::from_slice(&texto).map_err(|_| "El control del paquete (respaldo/proceso.json) no es válido.")?;
    if proceso.estado == "COMPLETO" {
        return Err("Ese paquete ya está completo; no hay nada que retomar.".into());
    }
    if proceso.carpetas.values().any(|c| carpeta_relativa(c).is_none()) {
        return Err("El control del paquete apunta fuera de su respaldo.".into());
    }
    Ok(proceso)
}

/// `respaldo/<lote>`, sin rutas absolutas ni `..`.
fn carpeta_relativa(texto: &str) -> Option<PathBuf> {
    let ruta = PathBuf::from(texto);
    let partes: Vec<Component> = ruta.components().collect();
    (partes.len() == 2 && partes[0] == Component::Normal(RESPALDO.as_ref()) && matches!(partes[1], Component::Normal(_))).then_some(ruta)
}

/// Paquetes cortados en `destino` (de los últimos `dias`), del más reciente al más antiguo: (carpeta, fase).
/// Solo los que quedaron EN_CURSO o INCOMPLETO: los PARCIALES terminaron y se revisan en el Asistente.
pub fn pendientes(destino: &Path, dias: u64) -> Vec<(PathBuf, String)> {
    let limite = std::time::SystemTime::now() - std::time::Duration::from_secs(dias * 86_400);
    let mut encontrados = Vec::new();
    for entrada in std::fs::read_dir(destino).into_iter().flatten().flatten() {
        if !entrada.file_name().to_string_lossy().starts_with("Paquete CSMP ") {
            continue;
        }
        let control = entrada.path().join(RESPALDO).join(PROCESO);
        let Ok(modificado) = std::fs::metadata(&control).and_then(|m| m.modified()) else { continue };
        if modificado < limite {
            continue;
        }
        let Ok(p) = serde_json::from_slice::<Proceso>(&std::fs::read(&control).unwrap_or_default()) else { continue };
        if matches!(p.estado.as_str(), "EN_CURSO" | "INCOMPLETO") {
            encontrados.push((modificado, entrada.path(), p.fase));
        }
    }
    encontrados.sort_by(|a, b| b.0.cmp(&a.0));
    encontrados.into_iter().map(|(_, c, f)| (c, f)).collect()
}

/// Nueva carpeta «Paquete CSMP <fecha>» en `destino` y el flujo completo.
pub fn ejecutar(sitfa: &dyn Sitfa, lote: &Lote, destino: &Path, prefs: &Preferencias, hoy: NaiveDate) -> Result<Resultado, String> {
    ejecutar_en(sitfa, lote, destino, prefs, hoy).1
}

/// Como [`ejecutar`], con la raíz del intento creada (E3B-13): ante un error, quien llama sabe qué carpeta retomar y
/// no confunde este intento con otro paquete pendiente del mismo destino. `None` si no alcanzó a crearse.
pub fn ejecutar_en(
    sitfa: &dyn Sitfa,
    lote: &Lote,
    destino: &Path,
    prefs: &Preferencias,
    hoy: NaiveDate,
) -> (Option<PathBuf>, Result<Resultado, String>) {
    if lote.consultas.iter().any(|s| s.screen != "seguimiento") {
        return (None, Err("Consultar y crear paquete requiere Seguimiento; las otras consultas siguen en «Descargar».".into()));
    }
    if std::fs::create_dir_all(destino).is_err() {
        return (None, Err("No se pudo crear la carpeta de destino.".into()));
    }
    let nombre = format!("Paquete CSMP {}", chrono::Local::now().format("%Y-%m-%d %H-%M-%S-%6f"));
    let raiz = destino.join(nombre);
    if std::fs::create_dir(&raiz).is_err() {
        return (None, Err("No se pudo crear la carpeta del paquete.".into()));
    }
    if std::fs::create_dir(raiz.join(RESPALDO)).is_err() {
        return (Some(raiz), Err("No se pudo crear la carpeta de respaldo del paquete.".into()));
    }
    let proceso = Proceso {
        estado: "EN_CURSO".into(),
        fase: "consultas".into(),
        carpetas: BTreeMap::new(),
        seleccion: Some(lote.clone()),
        reintentos: 0,
        paquete: None,
        historial: Vec::new(),
    };
    let r = correr(sitfa, &raiz, proceso, prefs, hoy);
    (Some(raiz), r)
}

/// Texto para el usuario cuando el flujo falla (E3B-13): nombra la carpeta real del intento y dice si se puede retomar.
pub fn mensaje_fallo(error: &str, raiz: Option<&Path>) -> String {
    match raiz {
        // E3C-11: solo se sugiere retomar si el control es legible y retomable; si está dañado o el paquete ya está
        // completo, se dice eso mismo.
        Some(c) if c.join(RESPALDO).join(PROCESO).is_file() => match leer(c) {
            Ok(_) => format!(
                "{error} El avance quedó en {}: usa «Retomar paquete…» y elige esa carpeta para continuar sin repetir lo ya hecho.",
                c.display()
            ),
            Err(motivo) => format!("{error} La carpeta {} no se puede retomar: {motivo} Crea un paquete nuevo.", c.display()),
        },
        Some(c) => format!(
            "{error} No se alcanzó a guardar el control (respaldo/{PROCESO}) en {}; ese intento no se puede retomar. Crea un paquete nuevo.",
            c.display()
        ),
        None => error.to_string(),
    }
}

/// Continúa un paquete cortado: no repite consultas validadas ni lecturas ya hechas, y vuelve a crear el ZIP.
pub fn retomar(sitfa: &dyn Sitfa, raiz: &Path, prefs: &Preferencias, hoy: NaiveDate) -> Result<Resultado, String> {
    let mut proceso = leer(raiz)?;
    if proceso.seleccion.is_none() {
        // Paquetes cortados antes de guardar su alcance: el lote de consultas lo conserva.
        proceso.seleccion = proceso.carpetas.get("consultas").and_then(|c| Resumen::leer(&raiz.join(c)).ok()).map(|r| r.lote);
        if proceso.seleccion.is_none() {
            return Err("Este paquete no guardó su alcance (se cortó en las consultas). Crea un paquete nuevo.".into());
        }
    }
    proceso.estado = "EN_CURSO".into();
    proceso.reintentos += 1;
    correr(sitfa, raiz, proceso, prefs, hoy)
}

/// E3B-11: bitácoras pedidas que no llegaron al paquete, por tribunal (sin vínculo, pendientes, descartadas o ingresos
/// verificados sin lectura). Las usan «Consultar y crear paquete» y «Crear paquete de este lote».
pub fn avisos_de_bitacoras(resumen: &Resumen, producto: &exportar::Producto) -> Vec<String> {
    let mut avisos = recoleccion::avisos_bitacoras_faltantes(resumen);
    let detalle = |lista: &[String]| -> String {
        lista
            .iter()
            .map(|t| {
                let (codigo, n) = t.split_once(": ").unwrap_or((t.as_str(), ""));
                match resumen.tribunales.get(codigo) {
                    Some(nombre) => format!("{nombre} ({codigo}): {n}"),
                    None => format!("tribunal {codigo}: {n}"),
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    if !producto.sin_bitacora.is_empty() {
        avisos.push(format!(
            "Ingresos verificados sin bitácora en el paquete: {}. Ese ingreso no quedó anotado para reintento: crea un paquete nuevo para leerlo.",
            detalle(&producto.sin_bitacora)
        ));
    }
    if !producto.recortadas.is_empty() {
        // E3C-10: la ventana se leyó, pero mostró paginador u otro indicio de recorte; no acredita el historial completo.
        avisos.push(format!(
            "Bitácoras leídas con posible recorte de la ventana (paginador o carga parcial): {}. Revísalas en SITFA.",
            detalle(&producto.recortadas)
        ));
    }
    avisos
}

/// Fuentes del paquete que fallaron, quedaron parciales o no se pudieron atribuir. Las bitácoras (y las consultas que las
/// agregan) son PARCIALES por diseño y se informan aparte con [`avisos_de_bitacoras`] (E3C-10).
pub fn aviso_incompletas(producto: &exportar::Producto) -> Option<String> {
    let por_diseno = ["bitacoras: PARCIAL", "consultas: PARCIAL"];
    let lista: Vec<&str> = producto.incompletas.iter().map(String::as_str).filter(|t| !por_diseno.contains(t)).collect();
    (!lista.is_empty()).then(|| format!("Fuentes no completas en el paquete: {}.", lista.join("; ")))
}

/// Lote validado sin lecturas pendientes: se reutiliza tal cual.
fn terminado(r: &Resumen) -> bool {
    r.estado == "VALIDADA" && recoleccion::lecturas_pendientes(r) == 0
}

/// Ejecuta, retoma o reutiliza el lote de una fase.
fn fase_lote(sitfa: &dyn Sitfa, raiz: &Path, proceso: &mut Proceso, fase: &str, lote: &Lote) -> Result<(PathBuf, Resumen), String> {
    let carpeta = match proceso.carpetas.get(fase) {
        Some(rel) => raiz.join(carpeta_relativa(rel).ok_or("El control del paquete apunta fuera de su respaldo.")?),
        None => {
            let c = lotes::carpeta_nueva(&raiz.join(RESPALDO))?;
            let nombre = c.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            proceso.carpetas.insert(fase.into(), format!("{RESPALDO}/{nombre}"));
            escribir(raiz, proceso)?;
            c
        }
    };
    if carpeta.join("resumen.json").exists() {
        let r = Resumen::leer(&carpeta)?;
        // E3B-12: un lote validado se reutiliza solo si todas sus lecturas siguen íntegras; si falta alguna, se retoma
        // y `lotes::ejecutar` vuelve a leer solo esos ingresos.
        let integras = r.lecturas.iter().all(|l| recoleccion::leer(&carpeta, l).is_ok());
        anotar(
            proceso,
            format!(
                "{fase}: lote {} {}; pendientes {}; lecturas {} ({})",
                carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                r.estado,
                recoleccion::lecturas_pendientes(&r),
                r.lecturas.len(),
                if integras { "íntegras" } else { "alguna falta o cambió" }
            ),
        );
        if terminado(&r) && integras {
            if fase == "consultas" {
                sitfa.avance("Consultas ya validadas en el intento anterior: se reutilizan.".into());
            }
            return Ok((carpeta, r));
        }
        // El alcance guardado manda: retomar no cambia el plan del lote.
        let lote = r.lote.clone();
        return Ok((carpeta.clone(), sitfa.lote(&lote, &carpeta, true)?));
    }
    let r = sitfa.lote(lote, &carpeta, false)?;
    Ok((carpeta, r))
}

fn correr(sitfa: &dyn Sitfa, raiz: &Path, mut proceso: Proceso, prefs: &Preferencias, hoy: NaiveDate) -> Result<Resultado, String> {
    let mut lote = proceso.seleccion.clone().ok_or("Falta el alcance del paquete.")?;
    // Las bitácoras siempre se leen (como `DiaryRunner`); historia y calendario del ingreso según las preferencias.
    lote.lecturas.bitacoras = true;
    lote.lecturas.historia = prefs.leer_historia;
    lote.lecturas.calendario = prefs.leer_calendarios;
    lote.lecturas.meses = prefs.meses_atras;
    lote.evidencia = prefs.pdf_sin_registros;
    lote.pausa_ms = prefs.pausa_ms;
    let inicio = format!("inicio; reintento {}", proceso.reintentos);
    anotar(&mut proceso, inicio);
    escribir(raiz, &proceso)?;
    let resultado = (|| -> Result<Resultado, String> {
        let mut fase = |proceso: &mut Proceso, nombre: &str| -> Result<(), String> {
            if sitfa.cancelado() {
                return Err("Proceso detenido. Los originales verificados se conservan en respaldo.".into());
            }
            proceso.fase = nombre.into();
            anotar(proceso, format!("fase {nombre}"));
            escribir(raiz, proceso)
        };
        fase(&mut proceso, "consultas")?;
        let (carpeta, resumen) = fase_lote(sitfa, raiz, &mut proceso, "consultas", &lote)?;
        let mut avisos = Vec::new();
        let (informes, completos) = informes_por_vencer(sitfa, raiz, &mut proceso, &resumen.lote, prefs, hoy, &mut avisos, &mut fase)?;
        fase(&mut proceso, "paquete")?;
        let destino = raiz.join(exportar::NOMBRE);
        if destino.exists() {
            // El ZIP parcial del intento anterior se guarda en respaldo; el nuevo ocupa su lugar.
            let anterior = format!("Captura_CSMP anterior {}.zip", chrono::Local::now().format("%Y-%m-%d %H-%M-%S-%6f"));
            std::fs::rename(&destino, raiz.join(RESPALDO).join(anterior)).map_err(|_| "No se pudo apartar el paquete anterior.")?;
        }
        let mut lotes_paquete = vec![LoteExportable { carpeta: &carpeta, resumen: &resumen }];
        lotes_paquete.extend(informes.iter().map(|(c, r)| LoteExportable { carpeta: c, resumen: r }));
        let mut opciones = Opciones { historia: prefs.historia_en_paquete, ..Default::default() };
        if !completos {
            opciones.avisos.push((
                "CALENDARIO_INCOMPLETO".into(),
                "Se pidió el calendario de informes por vencer y no quedó completo en el paquete.".into(),
            ));
        }
        let mut completos = completos;
        let producto = match exportar::exportar(&lotes_paquete, &destino, &opciones) {
            // El calendario de informes nunca impide el paquete de siempre: se crea sin él y queda PARCIAL.
            Err(e) if lotes_paquete.len() > 1 => {
                let aviso = format!("El calendario de informes por vencer no pudo incluirse en el paquete: {e}");
                sitfa.avance(aviso.clone());
                avisos.push(aviso);
                completos = false;
                lotes_paquete.truncate(1);
                if !opciones.avisos.iter().any(|(c, _)| c == "CALENDARIO_INCOMPLETO") {
                    opciones.avisos.push((
                        "CALENDARIO_INCOMPLETO".into(),
                        "Se pidió el calendario de informes por vencer y no quedó completo en el paquete.".into(),
                    ));
                }
                exportar::exportar(&lotes_paquete, &destino, &opciones)?
            }
            otro => otro?,
        };
        avisos.extend(producto.avisos.iter().cloned());
        // E3B-11: las bitácoras pedidas que no se leyeron (sin vínculo, pendientes o descartadas) y las tablas que no
        // quedaron completas se informan por tribunal y dejan el paquete PARCIAL.
        let faltantes = avisos_de_bitacoras(&resumen, &producto);
        avisos.extend(faltantes.iter().cloned());
        avisos.extend(aviso_incompletas(&producto));
        // Lecturas por ingreso pendientes, bitácoras pedidas que faltan, un calendario incompleto o la historia fuera del
        // paquete lo dejan PARCIAL.
        let parcial =
            recoleccion::lecturas_pendientes(&resumen) > 0 || !completos || !producto.avisos.is_empty() || !faltantes.is_empty();
        proceso.estado = if parcial { "PARCIAL" } else { "COMPLETO" }.into();
        proceso.paquete = Some(exportar::NOMBRE.into());
        let fin = format!("fin {}; {}", proceso.estado, producto.bitacoras);
        anotar(&mut proceso, fin);
        for aviso in &faltantes {
            anotar(&mut proceso, aviso);
        }
        escribir(raiz, &proceso)?;
        Ok(Resultado { carpeta: raiz.to_path_buf(), paquete: destino, parcial, avisos, bitacoras: producto.bitacoras })
    })();
    if let Err(e) = &resultado {
        proceso.estado = "INCOMPLETO".into();
        let fase = proceso.fase.clone();
        anotar(&mut proceso, format!("error en fase {fase}: {e}"));
        let _ = escribir(raiz, &proceso);
    }
    resultado
}

/// `_informes`: calendario de informes por vencer de los próximos 60 días, solo con la hoja de Cumplimiento. Un mes
/// que falla no descarta los ya descargados, pero el paquete queda parcial. Devuelve (lotes, completo).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn informes_por_vencer(
    sitfa: &dyn Sitfa,
    raiz: &Path,
    proceso: &mut Proceso,
    lote: &Lote,
    prefs: &Preferencias,
    hoy: NaiveDate,
    avisos: &mut Vec<String>,
    fase: &mut dyn FnMut(&mut Proceso, &str) -> Result<(), String>,
) -> Result<(Vec<(PathBuf, Resumen)>, bool), String> {
    if !prefs.informes_por_vencer || !lote.consultas.iter().any(|s| s.tab == "Cumplimiento") {
        return Ok((vec![], true));
    }
    fase(proceso, "informes")?;
    let meses = meses_informes(hoy);
    let mut tribunales: Vec<String> = Vec::new();
    for s in &lote.consultas {
        if !tribunales.contains(&s.tribunal) {
            tribunales.push(s.tribunal.clone());
        }
    }
    let mut lotes_informes = Vec::new();
    let resultado = (|| -> Result<(), String> {
        sitfa.catalogo_informes()?;
        for (mes, año) in &meses {
            let nombre = format!("informes-{año}-{mes}");
            proceso.fase = nombre.clone();
            escribir(raiz, proceso)?;
            let plan = plan_informes(&tribunales, mes, año, prefs);
            lotes_informes.push(fase_lote(sitfa, raiz, proceso, &nombre, &plan)?);
        }
        Ok(())
    })();
    // La consulta siguiente parte en Seguimiento; si no se puede volver, el aviso de la próxima corrida lo dirá.
    sitfa.volver();
    if let Err(e) = resultado {
        if sitfa.cancelado() {
            return Err(e);
        }
        let aviso = format!("No se incluyó completo el calendario de informes por vencer: {e}");
        sitfa.avance(aviso.clone());
        avisos.push(aviso);
    }
    let completo = lotes_informes.len() == meses.len();
    Ok((lotes_informes, completo))
}

/// Lote del calendario de informes por vencer de un mes, para los tribunales del paquete.
pub fn plan_informes(tribunales: &[String], mes: &str, año: &str, prefs: &Preferencias) -> Lote {
    Lote {
        consultas: tribunales
            .iter()
            .map(|t| Seleccion {
                tribunal: t.clone(),
                screen: "calendario_informes".into(),
                tab: String::new(),
                modality: String::new(),
                report: None,
                state: None,
                month: Some(mes.into()),
                year: Some(año.into()),
            })
            .collect(),
        fechas: None,
        conservar_filtros: false,
        evidencia: prefs.pdf_sin_registros,
        pausa_ms: prefs.pausa_ms,
        lecturas: Default::default(),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn meses_del_calendario_cubren_sesenta_dias() {
        let m = |s: &str| meses_informes(s.parse().unwrap());
        assert_eq!(m("2026-11-16"), [("11".into(), "2026".into()), ("12".into(), "2026".into()), ("1".into(), "2027".into())]);
        assert_eq!(m("2026-03-01"), [("3".to_string(), "2026".to_string()), ("4".into(), "2026".into())]);
    }

    #[test]
    fn carpetas_del_control_quedan_dentro_del_respaldo() {
        assert!(carpeta_relativa("respaldo/Flow Captura 2026-11-16 10-00-00").is_some());
        assert!(carpeta_relativa("respaldo/../otra").is_none());
        assert!(carpeta_relativa("/tmp/x").is_none());
        assert!(carpeta_relativa("otra/x").is_none());
    }
}
