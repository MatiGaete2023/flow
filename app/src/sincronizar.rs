//! «Actualizar desde Excel» (`nurus.personal.sync` de Flow Python): trae a Trabajo la revisión hecha en la copia de
//! Excel. Las filas se asocian por identidad, nunca por posición; una edición hecha aquí y otra en Excel sobre el mismo
//! campo es un conflicto que la persona resuelve, y nada se aplica hasta que no hay conflictos.

use crate::config::Config;
use crate::copia_excel::identificadores;
use crate::lectura;
use crate::producto_excel;
use crate::trabajo::{Registro, Trabajo};
use crate::util::{historical_match, normalizar, Cell};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const CAMPOS: [&str; 5] = ["OBSERVACION", "FECHA_OBS", "TT", "CC", "RES"];
const IDENTIDAD: [&str; 5] = ["rit", "rut", "nombre", "tribunal", "programa"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflicto {
    /// Posición de la fila en Trabajo.
    pub fila: usize,
    pub campo: String,
    pub base: String,
    pub local: String,
    pub entrante: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// La copia no corresponde al trabajo; no se aplicó nada.
    Rechazo(String),
    /// Hay campos editados aquí y en Excel; no se aplicó nada.
    Conflictos(Vec<Conflicto>),
}

impl Error {
    pub fn mensaje(&self) -> String {
        match self {
            Error::Rechazo(m) => m.clone(),
            Error::Conflictos(c) => {
                format!("Hay {} campos editados aquí y en Excel. Resuelve las diferencias; no se aplicó ningún cambio.", c.len())
            }
        }
    }
}

/// Fila de Trabajo, valores a aplicar, revisión que ve Excel y cambios en `NURUS_DECISIONES`.
struct Cambio {
    fila: usize,
    fusion: BTreeMap<&'static str, String>,
    excel: BTreeMap<String, String>,
    decisiones: Option<(Option<(BTreeMap<String, String>, BTreeMap<String, String>)>, BTreeMap<String, String>, BTreeMap<String, String>)>,
}

/// Elecciones de la persona ante un conflicto: (fila, campo) → valor.
pub type Resoluciones = BTreeMap<(usize, String), String>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Resumen {
    pub filas_cambiadas: usize,
    /// Filas con datos de origen distintos en Excel, cuya propuesta y reglas se recalcularon.
    pub recalculadas: usize,
    pub avisos: Vec<String>,
}

fn rechazo(m: &str) -> Error {
    Error::Rechazo(m.into())
}

/// Revisión que Excel tiene en una fila: los cinco campos, vacíos si no están.
fn revision_excel(cols: &HashMap<&str, usize>, fila: &[Cell]) -> BTreeMap<String, String> {
    CAMPOS
        .iter()
        .map(|c| (c.to_string(), cols.get(c).and_then(|&i| fila.get(i)).map(|v| v.display().trim().to_string()).unwrap_or_default()))
        .collect()
}

/// Revisión local de una fila con el mismo vocabulario que Excel. `None` en un campo: no hay edición local.
fn revision_local(r: &Registro) -> BTreeMap<&'static str, Option<String>> {
    let mut m = BTreeMap::new();
    m.insert("OBSERVACION", r.editada.clone());
    m.insert("RES", Some(r.res.clone()));
    for c in ["FECHA_OBS", "TT", "CC"] {
        m.insert(c, r.campos.get(c).cloned());
    }
    m
}

/// Texto canónico de un mapa de decisiones (`gestiones_correo` o `anulaciones`).
fn texto_mapa(m: &BTreeMap<String, String>) -> String {
    serde_json::to_string(m).unwrap_or_default()
}

/// Texto canónico de `NURUS_DECISIONES` (gestiones de correo y anulaciones de origen) para comparar y resolver conflictos.
fn texto_decisiones(correo: &BTreeMap<String, String>, anul: &BTreeMap<String, String>) -> String {
    serde_json::to_string(&serde_json::json!({ "mail": correo, "overrides": anul })).unwrap_or_default()
}

fn parsear_decisiones(texto: &str) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(texto) else {
        return (BTreeMap::new(), BTreeMap::new());
    };
    let leer = |obj: Option<&serde_json::Map<String, serde_json::Value>>| -> BTreeMap<String, String> {
        obj.into_iter()
            .flatten()
            .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_string())))
            .collect()
    };
    if v.get("mail").is_some() || v.get("overrides").is_some() {
        (leer(v.get("mail").and_then(serde_json::Value::as_object)), leer(v.get("overrides").and_then(serde_json::Value::as_object)))
    } else {
        (leer(v.as_object()), BTreeMap::new())
    }
}

/// `record_edits.decode`: la celda técnica `NURUS_DECISIONES` de la copia. `Ok(None)` si está vacía; el error es el de
/// Python. Devuelve `(gestiones_correo, overrides)` (sin las `auto`, que son la ausencia de gestión).
fn leer_decisiones(texto: &str) -> Result<Option<(BTreeMap<String, String>, BTreeMap<String, String>)>, Error> {
    let invalida = || rechazo("La columna técnica NURUS_DECISIONES no es válida. No se importaron cambios.");
    if texto.trim().is_empty() {
        return Ok(None);
    }
    let datos: serde_json::Value = serde_json::from_str(texto).map_err(|_| invalida())?;
    let decisiones = datos["decisions"].as_object().ok_or_else(invalida)?;
    let correo = match decisiones.get("mail") {
        None => serde_json::Map::new(),
        Some(m) => m.as_object().ok_or_else(invalida)?.clone(),
    };
    let overrides_obj = datos["overrides"].as_object().ok_or_else(invalida)?;
    let resolucion = decisiones.get("resolution").and_then(serde_json::Value::as_str).unwrap_or("auto");
    if datos["version"] != 1
        || !datos["word_overrides"].is_object()
        || !datos["excluded"].is_boolean()
        || !["auto", "review", "none", "PC_IE", "PC_INFO", "NOMENCL"].contains(&resolucion)
        || correo.values().any(|v| !matches!(v.as_str(), Some("auto" | "include" | "omit")))
        || overrides_obj.values().any(|v| !v.is_string() && !v.is_number() && !v.is_boolean())
    {
        return Err(invalida());
    }
    let mail = correo.into_iter().filter_map(|(k, v)| v.as_str().filter(|v| *v != "auto").map(|v| (k, v.to_string()))).collect();
    let overrides = overrides_obj
        .iter()
        .map(|(k, v)| {
            let s = match v.as_str() {
                Some(s) => s.to_string(),
                None => v.to_string(),
            };
            (k.clone(), s)
        })
        .collect();
    Ok(Some((mail, overrides)))
}

fn pos(headers: &[String], titulo: &str) -> Option<usize> {
    let k = normalizar(titulo);
    headers.iter().position(|h| normalizar(h) == k)
}

fn columnas(headers: &[String]) -> HashMap<&'static str, usize> {
    CAMPOS.iter().filter_map(|c| pos(headers, c).map(|i| (*c, i))).collect()
}

/// Deja anotado qué tenía la copia que se acaba de guardar: el identificador de cada fila y su revisión.
pub fn recordar_exportacion(w: &mut Trabajo, hoja: &str) {
    let h = producto_excel::encabezados(w);
    let cols = columnas(&h);
    let datos = producto_excel::filas(w);
    let ids = identificadores(w, hoja);
    for ((r, fila), id) in w.registros.iter_mut().zip(datos).zip(ids) {
        let base = revision_excel(&cols, &fila);
        r.sync_local = Some(base.clone());
        r.sync_base = Some(base);
        r.sync_correo = Some(texto_mapa(&r.gestiones_correo));
        r.sync_anulaciones = Some(texto_mapa(&r.anulaciones));
        r.id_excel = id;
    }
}

fn identidad(headers: &[String], fila: &[Cell], claves: &[&str], modo: crate::motor::Mode) -> Vec<String> {
    let mapa = lectura::map_columns(headers, modo);
    claves
        .iter()
        .map(|k| mapa.get(k).and_then(|&i| fila.get(i)).map(|c| historical_match(&c.display())).unwrap_or_default())
        .collect()
}

/// `sync.associate`: una fila del trabajo por fila de la copia, validando todo el conjunto antes de devolver.
fn asociar(w: &Trabajo, headers: &[String], filas: &[Vec<Cell>]) -> Result<Vec<usize>, Error> {
    let claves: Vec<&str> = IDENTIDAD.iter().copied().filter(|k| w.cols.contains_key(k)).collect();
    if !["rit", "nombre", "tribunal"].iter().all(|k| claves.contains(k)) {
        return Err(rechazo("Faltan campos de identidad; no se aplicaron cambios."));
    }
    let modo = w.mode();
    let col_id = pos(headers, "NURUS_ID_REGISTRO");
    let por_id: HashMap<&str, usize> = w.registros.iter().enumerate().filter(|(_, r)| !r.id_excel.is_empty()).map(|(i, r)| (r.id_excel.as_str(), i)).collect();
    let mut por_identidad: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (i, r) in w.registros.iter().enumerate() {
        let k: Vec<String> = claves.iter().map(|c| historical_match(&w.raw_value(r, c))).collect();
        por_identidad.entry(k).or_default().push(i);
    }
    let mut resultado = Vec::with_capacity(filas.len());
    let mut vistas = HashSet::new();
    for fila in filas {
        let en_excel = identidad(headers, fila, &claves, modo);
        let i = match col_id {
            Some(c) => {
                let id = fila.get(c).map(|v| v.display().trim().to_string()).unwrap_or_default();
                *por_id.get(id.as_str()).ok_or_else(|| rechazo("Hay identidades ajenas o vacías en la copia; no se aplicaron cambios."))?
            }
            None => match por_identidad.get(&en_excel).map(Vec::as_slice) {
                Some([unico]) => *unico,
                _ => return Err(rechazo("Falta la columna de identidad y hay registros ambiguos o modificados; no se aplicaron cambios.")),
            },
        };
        if !vistas.insert(i) {
            return Err(rechazo("Hay identidades duplicadas en la copia; no se aplicaron cambios."));
        }
        let local: Vec<String> = claves.iter().map(|c| historical_match(&w.raw_value(&w.registros[i], c))).collect();
        if local != en_excel {
            return Err(rechazo("Cambió la identidad de una fila; no se aplicaron cambios."));
        }
        resultado.push(i);
    }
    if vistas.len() != w.registros.len() {
        return Err(rechazo("Faltan registros en la copia; no se aplicaron cambios."));
    }
    Ok(resultado)
}

/// Resultado de combinar un campo: `Ok(Some(v))` toma `v`; `Ok(None)` conserva lo local; `Err` es un conflicto.
fn combinar(
    campo: &str,
    local: &Option<String>,
    base: Option<&String>,
    sync_local: Option<&String>,
    remoto: &str,
    con_base: bool,
) -> Result<Option<String>, (String, String, String)> {
    let base_v = base.map_or("", String::as_str);
    let local_v = local.as_deref().unwrap_or(base_v);
    let sync_local_v = sync_local.map_or(base_v, String::as_str);
    let mut local_cambio = local.is_some() && local_v != sync_local_v;
    if !con_base {
        // Sesiones sin línea base: nunca se sobrescribe en silencio un valor local.
        local_cambio = local.is_some() && local_v != remoto;
    }
    let remoto_cambio = !con_base || remoto != base_v;
    let _ = campo;
    if local_cambio && remoto_cambio && local_v != remoto {
        return Err((base_v.into(), local_v.into(), remoto.into()));
    }
    Ok(remoto_cambio.then(|| remoto.to_string()))
}

/// `sync.refresh`: combina la revisión de la copia con la de Trabajo. Todo o nada.
///
/// F10: si una columna de origen cambió en la copia, se toma el valor de Excel y se recalculan la propuesta y las reglas
/// (`record_edits.recalculate`); la revisión de la persona se conserva. En un trabajo de paquete el contexto de lectura
/// (historia, bitácora, calendario) sigue siendo el de la captura. Diferencia con Python: se vuelven a analizar todas las
/// filas, no solo las editadas, así que también se actualiza una fila cuya duplicidad depende de la editada.
pub fn actualizar(w: &mut Trabajo, cfg: &Config, headers: &[String], filas: &[Vec<Cell>], resoluciones: &Resoluciones) -> Result<Resumen, Error> {
    let asociadas = asociar(w, headers, filas)?;
    let cols = columnas(headers);
    let mut conflictos = Vec::new();
    // (fila de Trabajo, valores a aplicar, revisión que ve Excel)
    let mut cambios: Vec<Cambio> = Vec::new();
    for (&i, fila) in asociadas.iter().zip(filas) {
        let r = &w.registros[i];
        let excel = revision_excel(&cols, fila);
        let local = revision_local(r);
        let mut fusion = BTreeMap::new();
        for campo in CAMPOS {
            let remoto = excel.get(campo).map_or("", String::as_str);
            let base = r.sync_base.as_ref().and_then(|b| b.get(campo));
            let sync_local = r.sync_local.as_ref().and_then(|b| b.get(campo));
            let propio = local.get(campo).cloned().flatten();
            // RES siempre tiene un valor local (la sugerencia del motor); lo demás solo si la persona lo editó.
            match combinar(campo, &propio, base, sync_local, remoto, r.sync_base.is_some()) {
                Ok(Some(v)) => {
                    fusion.insert(campo, v);
                }
                Ok(None) => {}
                Err((b, l, e)) => match resoluciones.get(&(i, campo.to_string())) {
                    Some(v) => {
                        fusion.insert(campo, v.clone());
                    }
                    None => conflictos.push(Conflicto { fila: i, campo: campo.into(), base: b, local: l, entrante: e }),
                },
            }
        }
        // Gestiones de correo y anulaciones (`NURUS_DECISIONES`): Excel solo manda si cambió desde la última copia y la fila no cambió aquí.
        let mut decisiones = None;
        if let Some(c) = pos(headers, "NURUS_DECISIONES") {
            if let Some((rem_correo, rem_anul)) = leer_decisiones(&fila.get(c).map(|v| v.display()).unwrap_or_default())? {
                let parse_mapa = |opt: Option<&String>| -> BTreeMap<String, String> {
                    opt.and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default()
                };
                let con_base = r.sync_correo.is_some() || r.sync_anulaciones.is_some();
                let base_dec = texto_decisiones(&parse_mapa(r.sync_correo.as_ref()), &parse_mapa(r.sync_anulaciones.as_ref()));
                let local_dec = texto_decisiones(&r.gestiones_correo, &r.anulaciones);
                let remoto_dec = texto_decisiones(&rem_correo, &rem_anul);
                let nuevo = match combinar("NURUS_DECISIONES", &Some(local_dec), Some(&base_dec), Some(&base_dec), &remoto_dec, con_base) {
                    Ok(v) => Some(v.map(|_| (rem_correo.clone(), rem_anul.clone()))),
                    Err((b, l, e)) => match resoluciones.get(&(i, "NURUS_DECISIONES".to_string())) {
                        Some(v) => Some(Some(parsear_decisiones(v))),
                        None => {
                            conflictos.push(Conflicto { fila: i, campo: "NURUS_DECISIONES".into(), base: b, local: l, entrante: e });
                            None
                        }
                    },
                };
                decisiones = nuevo.map(|n| (n, rem_correo, rem_anul));
            }
        }
        cambios.push(Cambio { fila: i, fusion, excel, decisiones });
    }
    if !conflictos.is_empty() {
        return Err(Error::Conflictos(conflictos));
    }
    let mut resumen = Resumen::default();
    let mut origen_editado = 0;
    let mut anulaciones_cambiadas = false;
    let mut nuevos_valores: Vec<(usize, Vec<(usize, Cell)>)> = Vec::new();
    for (c, fila) in cambios.into_iter().zip(filas) {
        let i = c.fila;
        let r = &mut w.registros[i];
        let antes = (
            r.editada.clone(),
            r.res.clone(),
            r.campos.clone(),
            r.gestiones_correo.clone(),
            r.anulaciones.clone(),
        );
        for (campo, v) in c.fusion {
            match campo {
                // Una observación vaciada en Excel no borra la local ni la propuesta.
                "OBSERVACION" if !v.is_empty() => r.editada = Some(v),
                "OBSERVACION" => {}
                "RES" => r.res = v,
                otro if v.is_empty() => {
                    r.campos.remove(otro);
                }
                otro => {
                    r.campos.insert(otro.to_string(), v);
                }
            }
        }
        if let Some((nuevas, rem_correo, rem_anul)) = c.decisiones {
            if let Some((correo, anul)) = nuevas {
                if r.anulaciones != anul {
                    anulaciones_cambiadas = true;
                }
                r.gestiones_correo = correo;
                r.anulaciones = anul;
            }
            r.sync_correo = Some(texto_mapa(&rem_correo));
            r.sync_anulaciones = Some(texto_mapa(&rem_anul));
        }
        if antes != (
            r.editada.clone(),
            r.res.clone(),
            r.campos.clone(),
            r.gestiones_correo.clone(),
            r.anulaciones.clone(),
        ) {
            resumen.filas_cambiadas += 1;
        }
        r.sync_base = Some(c.excel.clone());
        r.sync_local = Some(c.excel);
        // Columnas de origen que cambiaron en la copia (sin las de identidad, que `asociar` ya validó).
        let mapa = lectura::map_columns(headers, w.mode());
        let w_ref: &Trabajo = w;
        let editadas: Vec<(usize, usize)> = w_ref
            .cols
            .iter()
            .filter(|(k, _)| !IDENTIDAD.contains(k))
            .filter_map(|(k, &c)| {
                let j = *mapa.get(k)?;
                let en_trabajo = w_ref.registros[i].values.get(c).map(Cell::display).unwrap_or_default();
                let en_excel = fila.get(j).map(Cell::display).unwrap_or_default();
                (normalizar(en_trabajo.trim()) != normalizar(en_excel.trim())).then_some((c, j))
            })
            .collect();
        if !editadas.is_empty() {
            origen_editado += 1;
            nuevos_valores.push((i, editadas.into_iter().map(|(c, j)| (c, fila.get(j).cloned().unwrap_or(Cell::Empty))).collect()));
        }
    }
    if origen_editado > 0 || anulaciones_cambiadas {
        let hoja = w.sheet;
        for (i, valores) in nuevos_valores {
            let fila_hoja = w.registros[i].fila_excel - 1;
            for (c, v) in valores {
                if let Some(celda) = w.sheets[hoja].rows[fila_hoja].get_mut(c) {
                    *celda = v;
                }
            }
        }
        w.analizar(cfg);
        resumen.recalculadas = origen_editado;
    }
    Ok(resumen)
}

/// Lee la copia guardada: la hoja de datos (la del modo o la que trae `NURUS_ID_REGISTRO`) con su encabezado.
pub fn leer_copia(w: &Trabajo, bytes: &[u8]) -> Result<(Vec<String>, Vec<Vec<Cell>>), String> {
    let hojas = lectura::read_sheets(bytes).map_err(|_| "No se pudo leer la copia de Excel.".to_string())?;
    let modo = w.mode().label().to_uppercase();
    let encabezado = |h: &lectura::Sheet| {
        h.rows.iter().take(15).position(|f| {
            let t: Vec<String> = f.iter().map(|c| normalizar(&c.display())).collect();
            let hay = |n: &str| t.contains(&normalizar(n));
            hay("RIT") && (hay("OBSERVACION") || hay("NURUS_ID_REGISTRO"))
        })
    };
    let hoja = hojas
        .iter()
        .find(|h| h.name.to_uppercase() == modo && encabezado(h).is_some())
        .or_else(|| hojas.iter().find(|h| encabezado(h).is_some()))
        .ok_or("La copia no tiene una hoja de datos reconocible.")?;
    let inicio = encabezado(hoja).unwrap();
    let headers = lectura::headers_of(&hoja.rows[inicio]);
    let filas = hoja.rows[inicio + 1..]
        .iter()
        .filter(|f| f.iter().any(|c| !c.is_empty()))
        .map(|f| f.to_vec())
        .collect();
    Ok((headers, filas))
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::config::Config;
    use crate::motor::Mode;
    use chrono::NaiveDate;
    use serde_json::Value;
    use std::path::PathBuf;

    /// El libro de origen que armó Python (título, fila vacía, encabezado y tres filas), escrito aquí con los mismos datos.
    fn trabajo(datos: &Value) -> Trabajo {
        let mut libro = rust_xlsxwriter::Workbook::new();
        let hoja = libro.add_worksheet();
        hoja.set_name("informe_1").unwrap();
        hoja.write_string(0, 0, "Reporte RUS").unwrap();
        for (c, h) in datos["encabezados"].as_array().unwrap().iter().enumerate() {
            hoja.write_string(2, c as u16, h.as_str().unwrap()).unwrap();
        }
        for (i, fila) in datos["filas"].as_array().unwrap().iter().enumerate() {
            for (c, v) in fila.as_array().unwrap().iter().enumerate() {
                match v.as_f64() {
                    Some(n) => hoja.write_number(3 + i as u32, c as u16, n).unwrap(),
                    None => hoja.write_string(3 + i as u32, c as u16, v.as_str().unwrap()).unwrap(),
                };
            }
        }
        let bytes = libro.save_to_buffer().unwrap();
        let corte = NaiveDate::parse_from_str(datos["corte"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        Trabajo::abrir(PathBuf::from("origen.xlsx"), bytes, Some(Mode::Espera), corte, &Config::default()).unwrap()
    }

    fn texto(v: &Value) -> String {
        v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())
    }

    /// Repite el escenario de Python sobre Trabajo y devuelve las diferencias con lo que Python grabó.
    fn comparar(datos: &Value, caso: &Value) -> Vec<String> {
        comparar_con(trabajo(datos), "ESPERA", caso)
    }

    /// Repite el escenario sobre un trabajo ya armado (de un archivo de RUS o de un paquete); `hoja` es la de su copia.
    fn comparar_con(mut w: Trabajo, hoja: &str, caso: &Value) -> Vec<String> {
        recordar_exportacion(&mut w, hoja);
        let sugerida: Vec<String> = w.registros.iter().map(|r| r.res.clone()).collect();
        for l in caso["locales"].as_array().unwrap() {
            let (i, campo, v) = (l[0].as_u64().unwrap() as usize, l[1].as_str().unwrap(), texto(&l[2]));
            let r = &mut w.registros[i];
            match campo {
                "OBSERVACION" => r.editada = Some(v),
                "RES" => r.res = v,
                otro => {
                    r.campos.insert(otro.into(), v);
                }
            }
        }
        for c in caso["correos_locales"].as_array().into_iter().flatten() {
            let mapa = c[1].as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
            w.registros[c[0].as_u64().unwrap() as usize].gestiones_correo = mapa;
        }
        for a in caso["anulaciones_locales"].as_array().into_iter().flatten() {
            let fila = a[0].as_u64().unwrap() as usize;
            let mapa = a[1].as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
            w.aplicar_anulaciones(fila, mapa, &Config::default()).unwrap();
        }
        let mut headers = producto_excel::encabezados(&w);
        let mut filas = producto_excel::filas(&w);
        for e in caso["remotas"].as_array().unwrap() {
            let (i, campo, v) = (e[0].as_u64().unwrap() as usize, e[1].as_str().unwrap(), texto(&e[2]));
            let c = pos(&headers, campo).unwrap();
            filas[i][c] = if e[2].is_number() { Cell::Number(e[2].as_f64().unwrap()) } else { Cell::Text(v) };
        }
        let excel = &caso["excel"];
        if let Some(i) = excel.get("cambiar_identidad").and_then(Value::as_u64) {
            let c = pos(&headers, "NOMBRE").unwrap();
            filas[i as usize][c] = Cell::Text("Otra Persona Ficticia".into());
        }
        if let Some(n) = excel.get("quitar_filas").and_then(Value::as_u64) {
            filas.truncate(filas.len() - n as usize);
        }
        if excel.get("quitar_id").and_then(Value::as_bool) == Some(true) {
            let c = pos(&headers, "NURUS_ID_REGISTRO").unwrap();
            headers.remove(c);
            filas.iter_mut().for_each(|f| {
                f.remove(c);
            });
        }
        let resoluciones: Resoluciones = caso["resoluciones"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| ((r[0].as_u64().unwrap() as usize, r[1].as_str().unwrap().to_string()), texto(&r[2])))
            .collect();
        let resultado = actualizar(&mut w, &Config::default(), &headers, &filas, &resoluciones);
        let mut d = Vec::new();
        match (&resultado, caso.get("error"), caso.get("conflictos")) {
            (Err(Error::Rechazo(m)), Some(e), _) if *m == texto(e) => {}
            (Err(Error::Conflictos(c)), None, Some(esperados)) => {
                // La celda técnica trae todo el estado de la fila en Python: solo se compara qué fila y qué campo chocan.
                let tecnica = |x: &Conflicto, v: &String| if x.campo == "NURUS_DECISIONES" { String::new() } else { v.clone() };
                let mut obtenidos: Vec<Vec<String>> = c
                    .iter()
                    .map(|x| vec![x.fila.to_string(), x.campo.clone(), tecnica(x, &x.base), tecnica(x, &x.local), tecnica(x, &x.entrante)])
                    .collect();
                obtenidos.sort();
                let mut previstos: Vec<Vec<String>> = esperados
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_array().unwrap().iter().map(texto).collect())
                    .collect();
                previstos.sort();
                if obtenidos != previstos {
                    d.push(format!("conflictos Rust {obtenidos:?} · Python {previstos:?}"));
                }
            }
            (Ok(_), None, None) => {}
            (otro, e, c) => d.push(format!("resultado Rust {otro:?} · Python error {e:?} conflictos {c:?}")),
        }
        // F10: lo que Python recalcula al cambiar una columna de origen (y lo que deja igual en las demás filas).
        for (i, propuesta) in caso["propuestas"].as_array().unwrap().iter().enumerate() {
            let r = &w.registros[i];
            let reglas: Vec<String> = caso["reglas"][i].as_array().unwrap().iter().map(texto).collect();
            if r.eventos != reglas {
                d.push(format!("fila {i} reglas: Rust {:?} · Python {reglas:?}", r.eventos));
            }
            // R6 (decisión del 20-nov): el nombre del programa en tipo título («Dos», no «DOS»).
            let esperada = texto(propuesta).replace("AFT Ejemplo DOS", "AFT Ejemplo Dos");
            if r.propuesta != esperada {
                d.push(format!("fila {i} propuesta: Rust {:?} · Python {esperada:?}", r.propuesta));
            }
        }
        for (i, esperado) in caso["correo"].as_array().into_iter().flatten().enumerate() {
            let previsto: BTreeMap<String, String> = esperado.as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
            if w.registros[i].gestiones_correo != previsto {
                d.push(format!("fila {i} gestiones de correo: Rust {:?} · Python {previsto:?}", w.registros[i].gestiones_correo));
            }
        }
        for (i, esperado) in caso["anulaciones"].as_array().into_iter().flatten().enumerate() {
            let previsto: BTreeMap<String, String> = esperado.as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
            if w.registros[i].anulaciones != previsto {
                d.push(format!("fila {i} anulaciones: Rust {:?} · Python {previsto:?}", w.registros[i].anulaciones));
            }
        }
        for (i, esperada) in caso["revision"].as_array().unwrap().iter().enumerate() {
            let r = &w.registros[i];
            for campo in CAMPOS {
                let obtenido = match campo {
                    "OBSERVACION" => r.editada.clone().unwrap_or_default(),
                    "RES" if r.res == sugerida[i] => String::new(),
                    "RES" => r.res.clone(),
                    otro => r.campos.get(otro).cloned().unwrap_or_default(),
                };
                let previsto = texto(&esperada[campo]);
                if obtenido != previsto {
                    d.push(format!("fila {i} {campo}: Rust {obtenido:?} · Python {previsto:?}"));
                }
            }
        }
        d
    }

    /// E-A1: «Actualizar desde Excel» de Rust contra `nurus.personal.sync.refresh` de Flow Python
    /// (`app/qa/generar_sync.py`): la revisión de cada fila, los conflictos y los rechazos con su mensaje.
    #[test]
    fn actualizar_desde_excel_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let casos = datos["casos"].as_array().unwrap();
        let (mut iguales, mut fallos) = (0, Vec::new());
        for caso in casos {
            let d = comparar(&datos, caso);
            if d.is_empty() {
                iguales += 1;
            } else {
                fallos.push(format!("{}:\n  {}", caso["escenario"], d.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("SYNC: {iguales}/{}, fallos {}", casos.len(), fallos.len());
        assert_eq!(casos.len(), 31, "el oráculo debe traer 31 casos");
        assert!(fallos.is_empty());
    }

    /// Prueba permanente del oráculo: si Python hubiera grabado otra revisión, la comparación debe fallar.
    #[test]
    fn oraculo_de_sync_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let mut caso = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "remota").unwrap().clone();
        assert!(comparar(&datos, &caso).is_empty());
        caso["revision"][0]["CC"] = Value::String("0".into());
        assert!(!comparar(&datos, &caso).is_empty(), "un valor esperado alterado no se detecta");
        // También la propuesta que Python recalculó al editar una columna de origen.
        let mut recalculo = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "origen_editado").unwrap().clone();
        assert!(comparar(&datos, &recalculo).is_empty());
        recalculo["reglas"][1] = serde_json::json!(["ESPERA.E05_PROYECTO_Y_CORREO"]);
        assert!(!comparar(&datos, &recalculo).is_empty(), "una regla recalculada alterada no se detecta");
        // Y la gestión de correo que Python combinó desde la celda técnica.
        let mut correo = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "correo_remoto").unwrap().clone();
        assert!(comparar(&datos, &correo).is_empty());
        correo["correo"][0] = serde_json::json!({"medidas": "include"});
        assert!(!comparar(&datos, &correo).is_empty(), "una gestión de correo esperada alterada no se detecta");
        // Y la anulación de origen que Python combinó desde la celda técnica.
        let mut anul = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "anulacion_remota_y_origen_editado_en_excel").unwrap().clone();
        assert!(comparar(&datos, &anul).is_empty());
        anul["anulaciones"][0] = serde_json::json!({"espera": "5"});
        assert!(!comparar(&datos, &anul).is_empty(), "una anulación esperada alterada no se detecta");
    }

    /// El archivo que guarda Flow Rust se puede volver a leer y a importar sin cambios ni conflictos.
    #[test]
    fn la_copia_guardada_se_importa_sin_cambios() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let mut w = trabajo(&datos);
        recordar_exportacion(&mut w, "ESPERA");
        let ruta = producto_excel::exportar(&w, &perfil.join("sync"), None).unwrap();
        let (headers, filas) = leer_copia(&w, &std::fs::read(&ruta).unwrap()).unwrap();
        assert_eq!(filas.len(), 3);
        let resumen = actualizar(&mut w, &Config::default(), &headers, &filas, &Resoluciones::new()).unwrap();
        assert_eq!(resumen.filas_cambiadas, 0, "{resumen:?}");
        assert!(resumen.avisos.is_empty(), "{:?}", resumen.avisos);
    }

    /// Un conflicto en las gestiones de correo se resuelve con el valor que elige la persona (el texto que muestra el
    /// diálogo: el de aquí o el de Excel) y la línea base pasa a la de Excel.
    #[test]
    fn un_conflicto_de_gestion_de_correo_se_resuelve_con_el_valor_elegido() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let celda = |mail: &str| {
            Cell::Text(format!(r#"{{"version":1,"decisions":{{"mail":{{"medidas":"{mail}"}}}},"overrides":{{}},"word_overrides":{{}},"excluded":false}}"#))
        };
        for (elegido, esperado) in [("local", "omit"), ("excel", "include")] {
            let mut w = trabajo(&datos);
            recordar_exportacion(&mut w, "ESPERA");
            w.registros[0].gestiones_correo.insert("medidas".into(), "omit".into());
            let headers = producto_excel::encabezados(&w);
            let mut filas = producto_excel::filas(&w);
            let c = pos(&headers, "NURUS_DECISIONES").unwrap();
            filas[0][c] = celda("include");
            let Err(Error::Conflictos(conflictos)) = actualizar(&mut w, &Config::default(), &headers, &filas, &Resoluciones::new()) else {
                panic!("debía haber un conflicto");
            };
            assert_eq!((conflictos.len(), conflictos[0].campo.as_str()), (1, "NURUS_DECISIONES"));
            assert_eq!(w.registros[0].gestiones_correo["medidas"], "omit", "sin resolver no se aplica nada");
            let texto = if elegido == "local" { conflictos[0].local.clone() } else { conflictos[0].entrante.clone() };
            let resoluciones = Resoluciones::from([((0, "NURUS_DECISIONES".to_string()), texto)]);
            actualizar(&mut w, &Config::default(), &headers, &filas, &resoluciones).unwrap();
            assert_eq!(w.registros[0].gestiones_correo["medidas"], esperado, "{elegido}");
            assert_eq!(w.registros[0].sync_correo.as_deref(), Some(r#"{"medidas":"include"}"#), "la línea base es la de Excel");
        }
    }

    /// E-A9: «Actualizar desde Excel» en un trabajo que viene de un paquete, contra `refresh` de Flow Python
    /// (`app/qa/generar_sync_paquete.py`): el lote de listados con tres columnas de origen, empaquetado por cada lado.
    fn trabajo_de_paquete(datos: &Value, carpeta: &std::path::Path) -> Trabajo {
        use flow_rust::descargador::paquete::{self, Opciones};
        use base64::Engine;
        let lote = &datos["lote"];
        let lote_dir = carpeta.join(lote["carpeta"].as_str().unwrap());
        for (ruta, b64) in lote["archivos"].as_object().unwrap() {
            let destino = lote_dir.join(ruta);
            std::fs::create_dir_all(destino.parent().unwrap()).unwrap();
            std::fs::write(destino, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
        let zip = carpeta.join("captura.zip");
        paquete::exportar_con(&[lote_dir], &zip, &Opciones::default()).unwrap();
        let corte = NaiveDate::parse_from_str(datos["corte"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        Trabajo::abrir_archivo(zip, Some(Mode::Cumplimiento), corte, &Config::default()).unwrap()
    }

    #[test]
    fn actualizar_desde_excel_en_un_paquete_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync_paquete.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let casos = datos["casos"].as_array().unwrap();
        let (mut iguales, mut fallos) = (0, Vec::new());
        for caso in casos {
            let carpeta = perfil.join("sync_paquete").join(caso["escenario"].as_str().unwrap());
            let _ = std::fs::remove_dir_all(&carpeta);
            std::fs::create_dir_all(&carpeta).unwrap();
            let d = comparar_con(trabajo_de_paquete(&datos, &carpeta), "CUMPLIMIENTO", caso);
            if d.is_empty() {
                iguales += 1;
            } else {
                fallos.push(format!("{}:\n  {}", caso["escenario"], d.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("SYNC_PAQUETE: {iguales}/{}, fallos {}", casos.len(), fallos.len());
        assert_eq!(casos.len(), 6, "el oráculo debe traer 6 casos");
        assert!(fallos.is_empty());
    }

    /// Prueba permanente del oráculo: la regla que Python recalculó en el paquete, alterada, debe detectarse.
    #[test]
    fn oraculo_de_sync_en_paquete_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync_paquete.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let carpeta = perfil.join("sync_paquete_alterado");
        let _ = std::fs::remove_dir_all(&carpeta);
        std::fs::create_dir_all(&carpeta).unwrap();
        let mut caso = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "origen_editado").unwrap().clone();
        assert!(comparar_con(trabajo_de_paquete(&datos, &carpeta), "CUMPLIMIENTO", &caso).is_empty());
        caso["reglas"][0] = serde_json::json!(["CUMPLIMIENTO.C05_POR_VENCER"]);
        let carpeta2 = perfil.join("sync_paquete_alterado_2");
        let _ = std::fs::remove_dir_all(&carpeta2);
        std::fs::create_dir_all(&carpeta2).unwrap();
        assert!(!comparar_con(trabajo_de_paquete(&datos, &carpeta2), "CUMPLIMIENTO", &caso).is_empty(), "una regla alterada no se detecta");
    }
}
