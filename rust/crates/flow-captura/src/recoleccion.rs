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

/// Lecturas por corrida (bitácoras y calendarios comparten el tope; las historias tienen uno igual aparte).
/// Usuario, 13-nov-2026: subido de 600 a 2000. No se elimina: evita una corrida sin fin si la sesión de SITFA se cae.
pub const TOPE: usize = 2000;
/// Como `MAX_FALLAS_SEGUIDAS` de `descargador/lectura_bitacoras.py`: tantas bitácoras fallidas seguidas detienen el lote
/// (la sesión de SITFA probablemente cayó). Solo cuentan las bitácoras; una leída reinicia la cuenta.
pub const MAX_FALLAS_SEGUIDAS: usize = 5;

static PENDIENTE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(
        r"\AConsulta (\d+), página \d+(?:, fila \d+: (?:bitacora|historia|calendario) pendiente|: vínculos del listado pendientes)\.",
    )
    .unwrap()
});
const RESUMEN_PENDIENTES: &str = "Los Excel verificados se conservan. Quedaron ";


/// Prefijo del aviso de una lectura por ingreso (`tipo`: bitacora, historia o calendario). Meta 4 lo usa para
/// reconocer las lecturas pendientes: no redactar este texto en otro lugar.
pub fn prefijo_lectura(consulta: usize, pagina: usize, fila: usize, tipo: &str) -> String {
    format!("Consulta {}, página {pagina}, fila {fila}: {tipo} ", consulta + 1)
}

/// Prefijo del aviso de los vínculos del listado de una página.
pub fn prefijo_listado(consulta: usize, pagina: usize) -> String {
    format!("Consulta {}, página {pagina}: vínculos del listado ", consulta + 1)
}

/// Consultas (índice desde 0) con lecturas por ingreso pendientes según los avisos del avance. Sus XLS ya están
/// verificados; al retomar se vuelven a recorrer solo para reintentar esas lecturas.
pub fn consultas_pendientes(resumen: &Resumen) -> BTreeSet<usize> {
    resumen
        .avisos
        .iter()
        .filter_map(|a| PENDIENTE.captures(a)?[1].parse::<usize>().ok()?.checked_sub(1))
        .filter(|i| *i < resumen.lote.consultas.len())
        .collect()
}

/// Lecturas (o páginas de vínculos) pendientes anotadas en el avance.
pub fn lecturas_pendientes(resumen: &Resumen) -> usize {
    resumen.avisos.iter().filter(|a| PENDIENTE.is_match(a)).count()
}

/// Nomenclaturas de contexto de `nurus/personal/motor/vocabulario.py::CONTEXTO` (igual a `flow_motor::hist::CONTEXTO`).
const CONTEXTO: [&str; 9] = [
    "Orden de búsqueda",
    "Termino de búsqueda",
    "Deja sin efecto búsqueda",
    "Conciliacion",
    "Dictación de sentencia",
    "Egreso de servicio protección especializada y otras redes",
    "Designa abogado",
    "Deja sin efecto designación C. Ad-litem",
    "Prorroga",
];

/// `nurus.bitacora_html.normalize`: NFKD sin lo no ASCII, minúsculas, `-().` como espacio y espacios colapsados
/// (el recorte es previo al reemplazo, como en Python).
fn normalizar_tramite(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let ascii: String = s.nfkd().filter(char::is_ascii).collect::<String>().to_lowercase().trim().to_string();
    let sin_signos: String = ascii.chars().map(|c| if matches!(c, '-' | '(' | ')' | '.') { ' ' } else { c }).collect();
    let mut salida = String::with_capacity(sin_signos.len());
    let mut espacio = false;
    for c in sin_signos.chars() {
        if c.is_whitespace() {
            if !espacio {
                salida.push(' ');
            }
            espacio = true;
        } else {
            salida.push(c);
            espacio = false;
        }
    }
    salida
}

/// Como `nurus.capture.historias.contexto_anterior`: de las filas anteriores a la ventana, solo el último trámite
/// (por fecha y folio) de cada nomenclatura de contexto, en el orden de la historia. Sirve para que una orden de
/// búsqueda, una sentencia o una designación antiguas sigan viéndose; nunca para redactar una constancia nueva.
pub fn contexto_anterior(anteriores: &[Value]) -> Vec<Value> {
    let nombres: BTreeSet<String> = CONTEXTO.iter().map(|n| normalizar_tramite(n)).collect();
    let mut ultimos: BTreeMap<String, ((String, i128), &Value)> = BTreeMap::new();
    for fila in anteriores {
        let nombre = normalizar_tramite(fila["descripcion"].as_str().unwrap_or(""));
        let fecha = fila["fecha_iso"].as_str().unwrap_or("");
        if !nombres.contains(&nombre) || fecha.is_empty() {
            continue;
        }
        let folio = fila["folio"].as_str().unwrap_or("");
        let numero = if !folio.is_empty() && folio.bytes().all(|b| b.is_ascii_digit()) { folio.parse().unwrap_or(i128::MAX) } else { -1 };
        let orden = (fecha.to_string(), numero);
        if ultimos.get(&nombre).is_none_or(|(o, _)| orden > *o) {
            ultimos.insert(nombre, (orden, fila));
        }
    }
    let mut salida: Vec<&Value> = ultimos.into_values().map(|(_, f)| f).collect();
    salida.sort_by_key(|f| f["fila"].as_u64().unwrap_or(u64::MAX));
    salida.into_iter().cloned().collect()
}

fn anotar(resumen: &mut Resumen, aviso: String) {
    if !resumen.avisos.contains(&aviso) {
        resumen.avisos.push(aviso);
    }
}

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

fn evidencia_revision(valor: &Value, capturada: &str, meses: u32) -> Result<Value, String> {
    let captura = DateTime::parse_from_rfc3339(capturada).map_err(|_| "Fecha de captura inválida.")?;
    let hasta = captura.with_timezone(&Local).date_naive();
    let desde = hasta.checked_sub_months(chrono::Months::new(meses)).ok_or("Período inválido.")?;
    let fecha = |k: &str, defecto: chrono::NaiveDate| -> Result<chrono::NaiveDate, String> {
        match valor.get(k) {
            None => Ok(defecto),
            Some(v) => v.as_str().and_then(|s| s.parse().ok()).ok_or_else(|| format!("Límite {k} inválido.")),
        }
    };
    let desde = fecha("desde", desde)?;
    let hasta = fecha("hasta", hasta)?;
    if desde > hasta { return Err("Período invertido en la lectura.".into()); }
    let entries = valor["entries"].as_array().ok_or("La bitácora no conserva entradas.")?;
    let fecha_entrada=|e:&Value|e["fecha"].as_str().and_then(|s|s.get(..10)).and_then(|s|s.parse::<chrono::NaiveDate>().ok());
    let ilegibles=entries.iter().filter(|e|fecha_entrada(e).is_none()).count();
    let posteriores=entries.iter().filter(|e|fecha_entrada(e).is_some_and(|f|f>hasta)).count();
    let anteriores=entries.iter().filter(|e|fecha_entrada(e).is_some_and(|f|f<desde)).count();
    let dentro=entries.iter().filter(|e|fecha_entrada(e).is_some_and(|f|f>=desde&&f<=hasta)).cloned().collect::<Vec<_>>();
    let mut avisos=valor["warnings"].as_array().cloned().unwrap_or_else(||vec![json!("Advertencias de origen no disponibles.")]);
    if ilegibles>0 {avisos.push(json!(format!("{ilegibles} entradas sin fecha acreditable: se omite su contenido y no se acredita ausencia.")));}
    if posteriores>0 {avisos.push(json!(format!("{posteriores} entradas posteriores a la captura: se omite su contenido y no se acredita ausencia.")));}
    let fechas_validas = ilegibles==0&&posteriores==0;
    let sin_avisos = avisos.is_empty();
    let completa = matches!(valor["coverage"].as_str(), Some("COMPLETA" | "VACIA_COMPROBADA")) && sin_avisos && fechas_validas;
    Ok(json!({"version":1,"desde":desde.to_string(),"hasta":hasta.to_string(),
        "captured_at":capturada,"coverage":valor["coverage"],"completa":completa,
        "entries":dentro,"warnings":avisos,"anteriores_omitidas":anteriores,"ilegibles_omitidas":ilegibles,
        "posteriores_omitidas":posteriores,"time_zone":"FECHA_CIVIL_DE_LA_CAPTURA"}))
}

/// Aplica el límite vigente a copias V1 anteriores sin modificar el archivo original
/// ni convertir una evidencia incompleta en completa.
pub fn evidencia_para_exportar(valor:&Value,capturada:&str,meses:u32)->Result<Value,String>{
    if valor["version"]!=1{return Err("Versión de evidencia desconocida.".into())}
    let captura=DateTime::parse_from_rfc3339(capturada).map_err(|_|"Fecha de captura inválida.")?;
    let fin=captura.with_timezone(&Local).date_naive();
    let limite=fin.checked_sub_months(chrono::Months::new(meses)).ok_or("Período inválido.")?;
    let desde=valor["desde"].as_str().and_then(|s|s.parse::<chrono::NaiveDate>().ok()).ok_or("Falta el comienzo de la evidencia.")?;
    let hasta=valor["hasta"].as_str().and_then(|s|s.parse::<chrono::NaiveDate>().ok()).ok_or("Falta el término de la evidencia.")?;
    let mut copia=valor.clone();copia["desde"]=json!(desde.max(limite).to_string());copia["hasta"]=json!(hasta.min(fin).to_string());
    let mut salida=evidencia_revision(&copia,capturada,meses)?;
    salida["completa"]=json!(valor["completa"]==true&&salida["completa"]==true);
    for campo in ["anteriores_omitidas","ilegibles_omitidas","posteriores_omitidas"] {
        salida[campo]=json!(salida[campo].as_u64().unwrap_or(0).saturating_add(valor[campo].as_u64().unwrap_or(0)));
    }
    if salida["ilegibles_omitidas"].as_u64().unwrap_or(0)>0 || salida["posteriores_omitidas"].as_u64().unwrap_or(0)>0 {salida["completa"]=json!(false);}
    // No se reescribe la fecha de origen para resolver una discrepancia.
    salida["captured_at"]=valor["captured_at"].clone();
    Ok(salida)
}

pub struct Recoleccion {
    hoy: chrono::NaiveDate,
    carpeta: PathBuf,
    previas: BTreeMap<String, (PathBuf, Referencia)>,
    aperturas: usize,
    historias: usize,
    fallas_seguidas: usize,
}

impl Recoleccion {
    fn reservar(&mut self, tipo: &str) -> Result<(), String> {
        if tipo == "historia" {
            if self.historias >= TOPE {
                return Err(format!("Se alcanzó el máximo de {TOPE} historias de esta corrida. Retoma el lote para continuar."));
            }
            self.historias += 1;
        } else {
            if self.aperturas >= TOPE {
                return Err(format!(
                    "Se alcanzó el máximo de {TOPE} lecturas de bitácoras/calendarios de esta corrida. Retoma el lote para continuar."
                ));
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
            let hermanas = |padre: &Path| {
                std::fs::read_dir(padre)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()) && e.path() != carpeta)
                    .filter_map(|e| e.metadata().and_then(|m| m.modified()).ok().map(|t| (e.path(), t)))
                    .collect::<Vec<_>>()
            };
            let padre = carpeta.parent().unwrap_or(carpeta);
            let mut carpetas = hermanas(padre);
            // Dentro de «Paquete CSMP …/respaldo/», como `reuse_base` del Descargador: también los lotes de los
            // paquetes anteriores de la misma carpeta de destino.
            if padre.file_name().is_some_and(|n| n == "respaldo") {
                if let Some(paquete) = padre.parent() {
                    for (otro, _) in paquete.parent().map(hermanas).unwrap_or_default() {
                        let es_paquete = otro.file_name().is_some_and(|n| n.to_string_lossy().starts_with("Paquete CSMP "));
                        if es_paquete && otro != paquete {
                            carpetas.extend(hermanas(&otro.join("respaldo")));
                        }
                    }
                }
            }
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
        Ok(Self { hoy, carpeta: carpeta.to_path_buf(), previas, aperturas: 0, historias: 0, fallas_seguidas: 0 })
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
        // Una lectura reutilizada puede estar recortada: nunca reconstruir su evidencia como completa.
        // Los metadatos respetan el mismo límite histórico y no retienen texto sin fecha.
        if tipo == "bitacora" && !reutilizada && valor.get("revision_bitacoras").is_none() {
            valor["revision_bitacoras"] = evidencia_revision(&valor, &capturada, meses)?;
        } else if tipo=="bitacora" && valor["revision_bitacoras"].is_object() {
            match evidencia_para_exportar(&valor["revision_bitacoras"],&capturada,meses) {
                Ok(acotada)=>valor["revision_bitacoras"]=acotada,
                Err(_)=>{valor.as_object_mut().unwrap().remove("revision_bitacoras");}
            }
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
            // Lotes anteriores no traen este campo: quien lo lea debe tratar su ausencia como lista vacía.
            valor["contexto_anterior"] = json!(contexto_anterior(&anteriores));
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
        // Una captura explícita debe obtener la evidencia que el formato legado no conservó.
        // No se convierte la ausencia de metadatos en una cobertura comprobada.
        if tipo == "bitacora"
            && (lectura["datos"]["revision_bitacoras"]["version"].as_u64() != Some(1)
                || !lectura["datos"]["revision_bitacoras"]["entries"].is_array())
        {
            return Ok(false);
        }
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
        // Como el Descargador (`motor.search_page` y `lectura_historia.pagina`): un listado cuyos vínculos no se pueden
        // leer queda anotado aparte y nunca detiene el lote; los XLS verificados siguen disponibles.
        let prefijo_listado = prefijo_listado(actual.consulta, actual.numero);
        resumen.avisos.retain(|s| !s.starts_with(&prefijo_listado));
        let enlaces = match vinculos::extraer(actual.html, &seleccion) {
            Ok(enlaces) => enlaces,
            Err(e) => {
                let estado = if opciones.activa() { "pendientes" } else { "no leídos" };
                anotar(resumen, format!("{prefijo_listado}{estado}. {e}"));
                resumen.guardar(&self.carpeta)?;
                return Ok(());
            }
        };
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
        if !opciones.activa() {
            return Ok(());
        }
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
                let prefijo = prefijo_lectura(actual.consulta, actual.numero, v.fila, tipo);
                resumen.avisos.retain(|s| {
                    !s.starts_with(&format!("{prefijo}pendiente.")) && !s.starts_with(&format!("{prefijo}sin vínculo comprobado;"))
                });
                if !vinculada {
                    let aviso = format!("{prefijo}sin vínculo comprobado; no evaluable.");
                    if !resumen.avisos.contains(&aviso) {
                        resumen.avisos.push(aviso);
                    }
                    continue;
                }
                // Como `session.reutilizar` del Descargador: una copia previa que no se puede reutilizar se lee en SITFA.
                if self.reutilizar(resumen, tipo, &v, opciones.meses).unwrap_or(false) {
                    if tipo == "bitacora" {
                        self.fallas_seguidas = 0;
                    }
                    continue;
                }
                self.reservar(tipo)?;
                lotes::pausa(pausa, control.cancelar)?;
                (control.progreso)(format!(
                    "Consulta {} · página {} · fila {} · leyendo {tipo}",
                    actual.consulta + 1,
                    actual.numero,
                    v.fila
                ));
                let lectura = match tipo {
                    "bitacora" => vinculos::abrir_bitacora(conexion, &v, &seleccion.modality),
                    "historia" => vinculos::abrir_historia(conexion, &v, tribunal),
                    _ => vinculos::abrir_calendario(conexion, &v),
                };
                match lectura {
                    Ok(datos) => {
                        self.persistir(resumen, tipo, &v, datos, opciones.meses, false)?;
                        if tipo == "bitacora" {
                            self.fallas_seguidas = 0;
                        }
                    }
                    Err(e) => {
                        anotar(resumen, format!("{prefijo}pendiente. {e}"));
                        resumen.guardar(&self.carpeta)?;
                        if tipo == "bitacora" {
                            self.fallas_seguidas += 1;
                            if self.fallas_seguidas >= MAX_FALLAS_SEGUIDAS {
                                return Err(format!(
                                    "{MAX_FALLAS_SEGUIDAS} bitácoras seguidas fallaron ({e}). Se detuvo el lote para no esperar en cada \
                                     causa. Revisa la sesión SITFA y usa «Retomar lote…»: no se vuelve a leer lo ya leído."
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Anota cuántas lecturas quedaron pendientes. No quita consultas de `terminadas`: sus XLS están verificados y
    /// las lecturas fallidas se reintentan al retomar (`consultas_pendientes`).
    pub fn terminar(&self, resumen: &mut Resumen) -> Result<(), String> {
        resumen.avisos.retain(|s| !s.starts_with(RESUMEN_PENDIENTES));
        let pendientes = lecturas_pendientes(resumen);
        if pendientes != 0 {
            resumen
                .avisos
                .push(format!("{RESUMEN_PENDIENTES}{pendientes} lecturas por ingreso pendientes; retoma el lote para reintentarlas."));
        }
        resumen.guardar(&self.carpeta)
    }
}

#[cfg(test)]
mod pruebas_limite {
    use super::*;

    #[test]
    fn revision_preserva_incertidumbre_sin_retener_texto_de_fecha_ilegible() {
        let valor = json!({"coverage":"COMPLETA","warnings":[],"desde":"2026-10-17","hasta":"2026-11-17",
            "entries":[{"fecha":"ilegible","origen":"centro","texto":"Nota ficticia"}]});
        let r = evidencia_revision(&valor, "2026-11-17T12:00:00-03:00", 1).unwrap();
        assert_eq!(r["entries"], json!([]));
        assert_eq!(r["ilegibles_omitidas"],1);
        assert!(!r.to_string().contains("Nota ficticia"));
        assert_eq!(r["completa"], false);
    }

    #[test]
    fn revision_no_reintroduce_textos_fuera_de_la_ventana_historica() {
        let valor=json!({"coverage":"COMPLETA","warnings":[],"entries":[
            {"fecha":"2026-01-01","texto":"ANTIGUA_NO_PERSISTIR"},
            {"fecha":"2026-10-17","texto":"Límite inclusivo"},
            {"fecha":"2026-11-17","texto":"Dentro"}]});
        let r=evidencia_revision(&valor,"2026-11-17T12:00:00-03:00",1).unwrap();
        assert_eq!(r["entries"].as_array().unwrap().len(),2);assert_eq!(r["anteriores_omitidas"],1);
        assert!(!r.to_string().contains("ANTIGUA_NO_PERSISTIR"));assert_eq!(r["completa"],true);
    }

    #[test]
    fn revision_acredita_vacio_y_conserva_advertencias() {
        let mut valor = json!({"coverage":"VACIA_COMPROBADA","warnings":[],"entries":[]});
        assert_eq!(evidencia_revision(&valor, "2026-11-17T12:00:00-03:00", 1).unwrap()["completa"], true);
        valor["warnings"] = json!(["Lectura incompleta"]);
        assert_eq!(evidencia_revision(&valor, "2026-11-17T12:00:00-03:00", 1).unwrap()["completa"], false);
        valor["desde"] = json!("2026-12-01");
        assert!(evidencia_revision(&valor, "2026-11-17T12:00:00-03:00", 1).is_err());
    }

    #[test]
    fn contexto_anterior_conserva_el_ultimo_de_cada_nomenclatura_como_python() {
        let fila = |fila: u64, folio: &str, descripcion: &str, fecha: &str| json!({"fila":fila,"folio":folio,"descripcion":descripcion,"fecha_iso":fecha,"tramite":"Resolución"});
        let anteriores = vec![
            fila(3, "50", "Orden de busqueda", "2099-01-10"),
            fila(4, "40", "ORDEN DE BÚSQUEDA", "2099-01-05"),
            fila(5, "60", "Dictación de sentencia", "2099-01-10"),
            fila(6, "61", "Dictacion  de sentencia", "2099-01-10"),
            fila(7, "70", "Acompaña documentos", "2099-02-01"),
            fila(8, "80", "Designa abogado", ""),
            fila(9, "s/n", "Deja sin efecto designación C. Ad-litem", "2099-01-01"),
        ];
        let contexto = contexto_anterior(&anteriores);
        let filas: Vec<u64> = contexto.iter().map(|f| f["fila"].as_u64().unwrap()).collect();
        assert_eq!(filas, vec![3, 6, 9]);
    }
    #[test]
    fn tope_comparte_bitacoras_calendario_y_separa_historias() {
        let mut r = Recoleccion {
            hoy: Local::now().date_naive(),
            carpeta: PathBuf::new(),
            previas: BTreeMap::new(),
            aperturas: 0,
            historias: 0,
            fallas_seguidas: 0,
        };
        for _ in 0..TOPE / 2 {
            r.reservar("bitacora").unwrap();
            r.reservar("calendario").unwrap();
        }
        assert!(r.reservar("bitacora").is_err());
        assert!(r.reservar("calendario").is_err());
        for _ in 0..TOPE {
            r.reservar("historia").unwrap();
        }
        assert!(r.reservar("historia").is_err());
    }
}


