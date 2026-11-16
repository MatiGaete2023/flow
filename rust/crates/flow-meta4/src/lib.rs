//! Meta 4: registros de Espera y Cumplimiento (FAE y Residencia) con la fecha de la última observación del centro
//! en el mes, a partir de las capturas locales de Flow. No se conecta a SITFA ni a RUS: solo lee carpetas.
//! Reglas tomadas de `META4_Bitacoras/scripts/meta4_bitacoras.py` 2.6.0.

pub mod plantilla;
pub mod regenerar;
pub mod salida;
pub mod tribunales;

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use flow_captura::lotes::{huella, ruta_archivo, Resumen};
use flow_captura::recoleccion;
use flow_captura::vinculos::{self, Vinculo};
use flow_dominio::{Celda, Etapa};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use tribunales::{categoria, norma, tribunal};

pub const ALERTA: &str = "ALERTA: última observación en Espera; registro en Cumplimiento";
pub const VERIFICADA: &str = "VERIFICADA EN EL PERÍODO";
pub const SIN_OBSERVACION: &str = "SIN OBSERVACIÓN DE ESPERA/CUMPLIMIENTO EN EL MES";
pub const EMPATE: &str = "REVISAR: etapas distintas en la misma fecha y hora";
pub const NO_COMPLETADA: &str = "LECTURA NO COMPLETADA";
pub const NO_VERIFICABLE: &str = "BITÁCORA NO VERIFICABLE";
pub const IDENTIDAD_VISIBLE: &str = "REVISAR IDENTIDAD VISIBLE";
pub const NOMBRE_VISIBLE: &str = "REVISAR NOMBRE VISIBLE (fecha tomada de la bitácora)";
pub const PERIODO_INCOMPLETO: &str = "REVISAR: la bitácora guardada no cubre todo el período";
pub const LEIDA_ANTES: &str = "REVISAR: bitácora leída antes de la fecha de corte";
pub const SIN_BITACORA: &str = "SIN BITÁCORA ASOCIADA";
pub const AMBIGUA: &str = "IDENTIDAD AMBIGUA";
pub const SIN_TRIBUNAL: &str = "SIN TRIBUNAL RECONOCIDO";

/// Mes calendario móvil que termina en el corte, ambos extremos incluidos (16/09 a 16/10; 28/02 a 31/03).
pub fn periodo(corte: NaiveDate) -> (NaiveDate, NaiveDate) {
    let (anio, mes) = if corte.month() == 1 { (corte.year() - 1, 12) } else { (corte.year(), corte.month() - 1) };
    let mut dia = corte.day();
    loop {
        if let Some(inicio) = NaiveDate::from_ymd_opt(anio, mes, dia) {
            return (inicio, corte);
        }
        dia -= 1;
    }
}

/// Etapa de una observación según la columna Etapa de la bitácora, nunca según el texto libre.
pub fn etapa_de(columna: &str) -> Option<&'static str> {
    match norma(columna).as_str() {
        "ESPERA" | "EN ESPERA" | "LISTA DE ESPERA" | "LISTA ESPERA" => Some("Espera"),
        "CUMPLIMIENTO" | "EN CUMPLIMIENTO" => Some("Cumplimiento"),
        _ => None,
    }
}

fn momento(texto: &str) -> Option<NaiveDateTime> {
    let t = texto.trim().trim_end_matches('Z');
    NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S"))
        .ok()
        .or_else(|| t.get(..10)?.parse::<NaiveDate>().ok()?.and_hms_opt(0, 0, 0))
}

/// Última observación del centro, de etapa Espera o Cumplimiento, dentro del período: (fecha, etapa, control).
/// Informes no participa. Si dos etapas comparten la última fecha y hora, se pide revisión.
pub fn ultima(entradas: &[(NaiveDateTime, String)], corte: NaiveDate) -> (Option<NaiveDate>, &'static str, &'static str) {
    let (inicio, fin) = periodo(corte);
    let validas: Vec<(NaiveDateTime, &'static str)> = entradas
        .iter()
        .filter(|(f, _)| inicio <= f.date() && f.date() <= fin)
        .filter_map(|(f, e)| etapa_de(e).map(|etapa| (*f, etapa)))
        .collect();
    let Some(mayor) = validas.iter().map(|(f, _)| *f).max() else { return (None, "", SIN_OBSERVACION) };
    let etapas: BTreeSet<&str> = validas.iter().filter(|(f, _)| *f == mayor).map(|(_, e)| *e).collect();
    if etapas.len() > 1 {
        (Some(mayor.date()), "", EMPATE)
    } else {
        (Some(mayor.date()), etapas.into_iter().next().unwrap(), VERIFICADA)
    }
}

fn palabras(texto: &str) -> BTreeSet<String> {
    norma(texto).split(' ').filter(|p| !p.is_empty()).map(String::from).collect()
}

/// Mismo nombre con otro orden, tildes o un nombre o apellido de menos (al menos dos palabras en común).
pub fn nombres_compatibles(a: &str, b: &str) -> bool {
    let (x, y) = (palabras(a), palabras(b));
    !x.is_empty() && !y.is_empty() && (x == y || ((x.is_subset(&y) || y.is_subset(&x)) && x.intersection(&y).count() >= 2))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Registro4 {
    /// «Espera» o «Cumplimiento»: la lista de RUS donde está el registro.
    pub lista: &'static str,
    /// «FAE» o «Residencia» según la derivación; None queda fuera de META 4.
    pub categoria: Option<&'static str>,
    pub tribunal: Option<&'static str>,
    /// Columnas originales del listado (título → valor).
    pub columnas: Vec<(String, Celda)>,
    pub archivo: String,
    pub lote: String,
    pub fecha_obs: Option<NaiveDate>,
    pub etapa_ultima: String,
    pub control: String,
    pub alerta: String,
}

impl Registro4 {
    /// Valor de una columna por título, sin distinguir tildes, mayúsculas ni puntuación.
    pub fn valor(&self, titulo: &str) -> Celda {
        let buscado = match norma(titulo).as_str() {
            "NACIONALIDAD PAIS" => "NACIONALIDAD PAIS".to_string(),
            otro => otro.to_string(),
        };
        self.columnas
            .iter()
            .find(|(t, _)| {
                let n = norma(t);
                n == buscado || (buscado == "RUT" && n == "RUT RCEI") || (buscado == "NOMBRE" && n == "NOMBRE MENOR")
            })
            .map(|(_, c)| c.clone())
            .unwrap_or_default()
    }
    pub fn texto(&self, titulo: &str) -> String {
        self.valor(titulo).texto().split_whitespace().collect::<Vec<_>>().join(" ")
    }
    pub fn revisada(&self) -> bool {
        self.control == VERIFICADA || norma(&self.control).starts_with("REVISADO")
    }
    pub fn requiere_revision(&self) -> bool {
        !self.alerta.is_empty() || !self.revisada()
    }
    pub fn tribunal_o_sin(&self) -> &'static str {
        self.tribunal.unwrap_or(SIN_TRIBUNAL)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservacionMes {
    pub tribunal: String,
    pub rit: String,
    pub rut: String,
    pub nombre: String,
    pub programa: String,
    pub causa: String,
    pub ingreso: String,
    pub fecha: NaiveDateTime,
    pub etapa: String,
    pub tipo: String,
    pub texto: String,
}

/// Una fila de «Control de lecturas»: una por bitácora del lote, también las que no se pudieron leer.
#[derive(Debug, Clone, PartialEq)]
pub struct Lectura {
    pub tribunal: String,
    pub rit: String,
    pub rut: String,
    pub nombre: String,
    pub programa: String,
    pub causa: String,
    pub ingreso: String,
    /// Estado de la lectura con el vocabulario del Descargador: LEIDA, FALLIDA, PENDIENTE o SIN_VINCULO.
    pub lectura: String,
    pub control: String,
}

/// Un archivo (o una consulta sin registros) de los lotes, para INVENTARIO_ARCHIVOS y MATRIZ_COBERTURA.
#[derive(Debug, Clone, PartialEq)]
pub struct Fuente {
    pub archivo: String,
    pub lote: String,
    /// «EXCEL», «PDF» (evidencia sin registros) o «SIN ARCHIVO» (consulta sin registros sin evidencia).
    pub tipo: &'static str,
    pub tribunal: Option<&'static str>,
    pub categoria: &'static str,
    pub lista: &'static str,
    pub registros: usize,
    /// «PROCESADO», «SIN REGISTROS» o «NO CLASIFICADO».
    pub resultado: &'static str,
    pub seleccionado: bool,
}

#[derive(Debug, Clone)]
pub struct Meta4 {
    pub corte: NaiveDate,
    pub emision: NaiveDate,
    /// Tribunales incluidos; None = todos.
    pub tribunales: Option<Vec<&'static str>>,
    pub registros: Vec<Registro4>,
    pub observaciones: Vec<ObservacionMes>,
    pub lecturas: Vec<Lectura>,
    /// Consultas con registros que ningún lote leyó bitácoras: (consulta, tribunal).
    pub sin_bitacoras: Vec<(String, String)>,
    pub filtros: Vec<String>,
    pub lotes: Vec<String>,
    pub avisos: Vec<String>,
    /// Archivos y consultas sin registros de los lotes (inventario y matriz de cobertura).
    pub fuentes: Vec<Fuente>,
    /// Al regenerar: hojas INVENTARIO_ARCHIVOS y MATRIZ_COBERTURA del Registros.xlsx revisado, que se conservan tal cual.
    pub hojas_previas: Vec<(String, Vec<Vec<Celda>>)>,
}

fn modalidad(codigo: &str) -> Option<&'static str> {
    match codigo {
        "1" => Some("Residencia"),
        "3" => Some("FAE"),
        _ => None,
    }
}

/// Tribunales elegidos por el usuario, en el orden oficial. Un nombre que no se reconoce es un error.
pub fn normalizar_tribunales(valores: &[String]) -> Result<Option<Vec<&'static str>>, String> {
    if valores.iter().all(|v| v.trim().is_empty()) {
        return Ok(None);
    }
    let mut elegidos = Vec::new();
    for v in valores.iter().filter(|v| !v.trim().is_empty()) {
        elegidos.push(tribunal(v).ok_or(format!("Tribunal no reconocido: {}.", v.trim()))?);
    }
    Ok(Some(tribunales::TRIBUNALES.into_iter().filter(|t| elegidos.contains(t)).collect()))
}

/// Carpetas de resultados de Meta 4 (terminadas o en curso): nunca son lotes de origen.
fn es_salida(nombre: &str) -> bool {
    ["Resultado META4", "Salida META4", "META4 regenerado"].iter().any(|p| nombre.starts_with(p)) || nombre.contains(".en-curso-")
}

fn buscar_lotes(carpeta: &Path, profundidad: usize, hallados: &mut Vec<PathBuf>) {
    if carpeta.join("resumen.json").is_file() && !hallados.iter().any(|h| h == carpeta) {
        hallados.push(carpeta.to_path_buf());
    }
    if profundidad == 0 {
        return;
    }
    let mut hijas: Vec<PathBuf> = std::fs::read_dir(carpeta)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| !es_salida(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect();
    hijas.sort();
    for hija in hijas {
        buscar_lotes(&hija, profundidad - 1, hallados);
    }
}

/// Como `buscar_archivos` (rglob de `resumen.json`) de Python: una carpeta que contiene varios lotes se expande a
/// todos ellos, sin carpetas de resultados de Meta 4 ni repeticiones. Una carpeta sin ningún lote es un error.
pub fn expandir_lotes(carpetas: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut lotes = Vec::new();
    for carpeta in carpetas {
        let antes = lotes.len();
        let mut hallados = Vec::new();
        buscar_lotes(carpeta, 8, &mut hallados);
        for h in hallados {
            if !lotes.contains(&h) {
                lotes.push(h);
            }
        }
        if lotes.len() == antes && !carpeta.join("resumen.json").is_file() {
            return Err(format!("Esta carpeta no contiene un lote de Captura (resumen.json): {}", carpeta.display()));
        }
    }
    Ok(lotes)
}

/// Día local más reciente con bitácoras leídas en los lotes: propone la fecha de corte (`fecha_lectura` de Python).
pub fn fecha_lectura(carpetas: &[PathBuf]) -> Option<NaiveDate> {
    let lotes = expandir_lotes(carpetas).ok()?;
    lotes
        .iter()
        .filter_map(|c| Resumen::leer(c).ok())
        .flat_map(|r| r.lecturas.into_iter().filter(|l| l.tipo == "bitacora").map(|l| l.capturada))
        .filter_map(|c| chrono::DateTime::parse_from_rfc3339(&c).ok().map(|f| f.with_timezone(&chrono::Local).date_naive()))
        .max()
}

/// Fila del listado, ya leída y verificada.
struct FilaLeida {
    archivo: String,
    pagina: usize,
    columnas: Vec<(String, Celda)>,
    rit: String,
    rut: String,
    nombre: String,
    programa: String,
    tribunal: Option<&'static str>,
}

/// Consulta de Seguimiento de Espera o Cumplimiento de FAE o Residencia de un lote.
struct ConsultaLeida {
    lote: usize,
    numero: usize,
    codigo: String,
    lista: &'static str,
    categoria: &'static str,
    nombre: String,
    vacia: bool,
    evidencia: Option<String>,
    paginas_xls: Vec<(String, usize)>,
    filas: Vec<FilaLeida>,
    /// Tribunal de la selección, derivado de las filas (o de otra consulta del mismo código).
    tribunal: Option<&'static str>,
}

struct LoteLeido {
    carpeta: PathBuf,
    etiqueta: String,
    resumen: Resumen,
}

/// Lee los lotes y las filas de sus consultas de Meta 4; asigna a cada consulta el tribunal de su selección.
/// `Resumen` solo guarda el código del tribunal: el nombre se toma del único tribunal que nombran las filas de las
/// consultas con ese código. Un código cuyas consultas no tienen registros queda sin tribunal.
fn leer_lotes(carpetas: &[PathBuf]) -> Result<(Vec<LoteLeido>, Vec<ConsultaLeida>), String> {
    if carpetas.is_empty() {
        return Err("Agrega al menos un lote de registros.".into());
    }
    let mut lotes = Vec::new();
    let mut consultas = Vec::new();
    for carpeta in expandir_lotes(carpetas)? {
        let resumen = Resumen::leer(&carpeta)?;
        let etiqueta = carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if resumen.estado != "VALIDADA" {
            return Err(format!(
                "La descarga de registros está incompleta («{etiqueta}», estado {}). Complétala con «Retomar lote» en Captura antes de generar Meta 4.",
                resumen.estado
            ));
        }
        for (numero, seleccion) in resumen.lote.consultas.iter().enumerate() {
            let (Some(etapa), Some(categoria_consulta)) = (Etapa::desde(&seleccion.tab), modalidad(&seleccion.modality)) else {
                continue;
            };
            if seleccion.screen != "seguimiento" || etapa == Etapa::Informes {
                continue;
            }
            let lista = if etapa == Etapa::Espera { "Espera" } else { "Cumplimiento" };
            let nombre_consulta = format!("tribunal {} · {lista} · {categoria_consulta}", seleccion.tribunal);
            if !resumen.terminadas.contains(&numero) && !resumen.vacias.contains(&numero) {
                return Err(format!("El lote «{etiqueta}» está incompleto: falta la consulta {nombre_consulta}. Retómalo en Captura."));
            }
            let mut c = ConsultaLeida {
                lote: lotes.len(),
                numero,
                codigo: seleccion.tribunal.clone(),
                lista,
                categoria: categoria_consulta,
                nombre: nombre_consulta,
                vacia: resumen.vacias.contains(&numero),
                evidencia: resumen.archivos.iter().find(|a| a.consulta == numero && a.nombre.ends_with(".pdf")).map(|a| a.nombre.clone()),
                paginas_xls: Vec::new(),
                filas: Vec::new(),
                tribunal: None,
            };
            for archivo in resumen.archivos.iter().filter(|a| a.consulta == numero && a.nombre.ends_with(".xls")) {
                let datos = std::fs::read(ruta_archivo(&carpeta, &archivo.nombre)?)
                    .map_err(|_| format!("Falta una descarga del lote «{etiqueta}»."))?;
                if huella(&datos) != archivo.sha256 {
                    return Err(format!("Una descarga del lote «{etiqueta}» cambió desde su verificación."));
                }
                let libro = flow_tabular::leer_bytes(&archivo.nombre, datos)?;
                let tabla = flow_tabular::leer_tabla(&libro, Some(etapa), None)?;
                c.paginas_xls.push((archivo.nombre.clone(), tabla.filas.len()));
                for fila in &tabla.filas {
                    c.filas.push(FilaLeida {
                        archivo: archivo.nombre.clone(),
                        pagina: archivo.pagina,
                        columnas: tabla
                            .cabeceras
                            .iter()
                            .cloned()
                            .zip(fila.celdas.iter().cloned())
                            .filter(|(t, _)| !t.trim().is_empty())
                            .collect(),
                        rit: fila.registro.texto("rit"),
                        rut: fila.registro.texto("rut"),
                        nombre: fila.registro.texto("nombre"),
                        programa: fila.registro.texto("programa"),
                        tribunal: tribunal(&fila.registro.texto("tribunal")),
                    });
                }
            }
            let nombres: BTreeSet<&'static str> = c.filas.iter().filter_map(|f| f.tribunal).collect();
            c.tribunal = (nombres.len() == 1).then(|| *nombres.iter().next().unwrap());
            consultas.push(c);
        }
        lotes.push(LoteLeido { carpeta, etiqueta, resumen });
    }
    let mut por_codigo: HashMap<String, BTreeSet<&'static str>> = HashMap::new();
    for c in &consultas {
        if let Some(t) = c.tribunal {
            por_codigo.entry(c.codigo.clone()).or_default().insert(t);
        }
    }
    for c in consultas.iter_mut().filter(|c| c.tribunal.is_none()) {
        c.tribunal = por_codigo.get(&c.codigo).filter(|s| s.len() == 1).and_then(|s| s.iter().next().copied());
    }
    Ok((lotes, consultas))
}

/// Tribunales de Meta 4 presentes en los lotes (con registros o con una consulta sin registros), en el orden oficial.
pub fn tribunales_en_lotes(carpetas: &[PathBuf]) -> Result<Vec<&'static str>, String> {
    let (_, consultas) = leer_lotes(carpetas)?;
    let presentes: BTreeSet<&'static str> =
        consultas.iter().flat_map(|c| c.filas.iter().filter_map(|f| f.tribunal).chain(c.tribunal)).collect();
    Ok(tribunales::TRIBUNALES.into_iter().filter(|t| presentes.contains(t)).collect())
}

/// Bitácora de un vínculo del listado, con su control (independiente de la fila, como en `_leer_bitacoras` de Python).
struct Bitacora {
    consulta: usize,
    lectura: Lectura,
    control: String,
    /// Observaciones comprobadas: (fecha, etapa, tipo, texto, id). Vacías si la lectura no es utilizable.
    entradas: Vec<(NaiveDateTime, String, String, String, String)>,
}

/// Controles con los que la bitácora se exporta a «Observaciones del mes» (Python exporta toda bitácora leída).
fn exporta_observaciones(control: &str) -> bool {
    [OK, NOMBRE_VISIBLE, LEIDA_ANTES, IDENTIDAD_VISIBLE].contains(&control)
}
const OK: &str = "OK";

fn controlar_bitacora(
    carpeta: &Path,
    resumen: &Resumen,
    pagina: &recoleccion::Pagina,
    v: &Vinculo,
    tribunal_consulta: &str,
    corte: NaiveDate,
) -> Bitacora {
    let (inicio, fin) = periodo(corte);
    let clave = recoleccion::clave("bitacora", v);
    let referencia = resumen.lecturas.iter().find(|r| r.tipo == "bitacora" && r.clave == clave);
    let fallida = format!("{}pendiente.", flow_captura::recoleccion::prefijo_lectura(pagina.consulta, pagina.numero, v.fila, "bitacora"));
    let mut entradas = Vec::new();
    let (lectura, control) = if !v.bitacora_verificada {
        ("SIN_VINCULO", NO_COMPLETADA.to_string())
    } else {
        match referencia {
            None if resumen.avisos.iter().any(|a| a.starts_with(&fallida)) => ("FALLIDA", NO_COMPLETADA.to_string()),
            None => ("PENDIENTE", NO_COMPLETADA.to_string()),
            Some(r) => match recoleccion::leer(carpeta, r) {
                Err(_) => ("LEIDA", NO_VERIFICABLE.to_string()),
                Ok(valor) => {
                    let datos = &valor["datos"];
                    // Una fecha ilegible deja esta bitácora como no verificable; Meta 4 sigue con las demás.
                    let leidas: Option<Vec<_>> = datos["entries"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|e| {
                            let texto = |k: &str| e[k].as_str().unwrap_or("").to_string();
                            Some((momento(e["fecha"].as_str()?)?, texto("etapa"), texto("tipo"), texto("texto"), e["entry_id"].to_string()))
                        })
                        .collect();
                    match leidas {
                        None => ("LEIDA", NO_VERIFICABLE.to_string()),
                        Some(todas) => {
                            let leida = chrono::DateTime::parse_from_rfc3339(&r.capturada)
                                .map(|f| f.with_timezone(&chrono::Local).date_naive())
                                .ok();
                            let dia = |k: &str| datos[k].as_str().and_then(|s| s.parse::<NaiveDate>().ok());
                            let persona = datos["persona"].as_str().unwrap_or("");
                            // Como `control_identidad`: un RIT visible distinto o ausente bloquea; un nombre distinto o
                            // ausente solo pide revisión.
                            let control = if norma(datos["rit"].as_str().unwrap_or("")) != norma(&v.rit) {
                                IDENTIDAD_VISIBLE
                            } else if leida.is_none_or(|d| d < corte) {
                                LEIDA_ANTES
                            } else if !matches!((dia("desde"), dia("hasta")), (Some(d), Some(h)) if d <= inicio && h >= fin) {
                                PERIODO_INCOMPLETO
                            } else if !nombres_compatibles(persona, &v.nombre) {
                                NOMBRE_VISIBLE
                            } else {
                                OK
                            };
                            if exporta_observaciones(control) {
                                entradas = todas;
                            }
                            ("LEIDA", control.to_string())
                        }
                    }
                }
            },
        }
    };
    Bitacora {
        consulta: pagina.consulta,
        lectura: Lectura {
            tribunal: tribunal_consulta.to_string(),
            rit: v.rit.clone(),
            rut: v.rut.clone(),
            nombre: v.nombre.clone(),
            programa: v.programa.clone(),
            causa: v.causa.clone(),
            ingreso: v.ingreso.clone(),
            lectura: lectura.to_string(),
            control: control.clone(),
        },
        control,
        entradas,
    }
}

/// Lee uno o varios lotes de Flow y calcula Meta 4. No escribe nada.
pub fn calcular(carpetas: &[PathBuf], corte: NaiveDate, emision: NaiveDate, elegidos: &[String]) -> Result<Meta4, String> {
    let tribunales_elegidos = normalizar_tribunales(elegidos)?;
    let (lotes, consultas) = leer_lotes(carpetas)?;
    let mut m = Meta4 {
        corte,
        emision,
        tribunales: tribunales_elegidos,
        registros: Vec::new(),
        observaciones: Vec::new(),
        lecturas: Vec::new(),
        sin_bitacoras: Vec::new(),
        filtros: Vec::new(),
        lotes: Vec::new(),
        avisos: Vec::new(),
        fuentes: Vec::new(),
        hojas_previas: Vec::new(),
    };
    let (inicio, fin) = periodo(corte);
    // Como Python (`_lote`): la misma consulta solo se rechaza si aparece en dos lotes distintos.
    let mut consultas_vistas: HashMap<(String, String, String), (usize, String)> = HashMap::new();
    for (n, lote) in lotes.iter().enumerate() {
        let (carpeta, etiqueta, resumen) = (&lote.carpeta, &lote.etiqueta, &lote.resumen);
        m.lotes.push(etiqueta.clone());
        if resumen.lote.fechas.is_some() {
            m.filtros.push(format!("{etiqueta}: filtro de fechas"));
        }
        if resumen.lote.conservar_filtros {
            m.filtros.push(format!("{etiqueta}: filtros del formulario"));
        }
        let del_lote: Vec<&ConsultaLeida> = consultas.iter().filter(|c| c.lote == n).collect();
        // Captura guarda los vínculos del listado en todo lote de Seguimiento (como el Descargador); sin lectura de
        // bitácoras pedida, la consulta sigue contando como «sin bitácoras leídas».
        let paginas: &[recoleccion::Pagina] = if resumen.lote.lecturas.bitacoras { &resumen.paginas_lectura } else { &[] };
        let mut todos: Vec<Vinculo> = paginas.iter().flat_map(|p| p.vinculos.clone()).collect();
        vinculos::descartar_ambiguos(&mut todos);
        let mut indice: HashMap<(usize, [String; 4]), Vec<Vinculo>> = HashMap::new();
        let mut bitacoras: HashMap<String, Bitacora> = HashMap::new();
        let mut siguiente = todos.into_iter();
        for pagina in paginas {
            let consulta = del_lote.iter().find(|c| c.numero == pagina.consulta);
            for _ in &pagina.vinculos {
                let v = siguiente.next().expect("vínculo");
                if let Some(c) = consulta {
                    if let std::collections::hash_map::Entry::Vacant(hueco) = bitacoras.entry(recoleccion::clave("bitacora", &v)) {
                        let tribunal_consulta = c.tribunal.unwrap_or(SIN_TRIBUNAL);
                        let b = controlar_bitacora(carpeta, resumen, pagina, &v, tribunal_consulta, corte);
                        for (fecha, etapa, tipo, texto, _) in b.entradas.iter().filter(|e| inicio <= e.0.date() && e.0.date() <= fin) {
                            m.observaciones.push(ObservacionMes {
                                tribunal: tribunal_consulta.to_string(),
                                rit: v.rit.clone(),
                                rut: v.rut.clone(),
                                nombre: v.nombre.clone(),
                                programa: v.programa.clone(),
                                causa: v.causa.clone(),
                                ingreso: v.ingreso.clone(),
                                fecha: *fecha,
                                etapa: etapa.clone(),
                                tipo: tipo.clone(),
                                texto: texto.clone(),
                            });
                        }
                        m.lecturas.push(b.lectura.clone());
                        hueco.insert(b);
                    }
                }
                indice.entry((pagina.consulta, v.clave())).or_default().push(v);
            }
        }
        // Bitácoras sin leer (fallidas o pendientes) y páginas sin vínculos: solo las de bitácora, no historia ni calendario.
        let no_leidas: Vec<&Bitacora> =
            bitacoras.values().filter(|b| matches!(b.lectura.lectura.as_str(), "FALLIDA" | "PENDIENTE")).collect();
        let mut pendientes: BTreeSet<usize> = no_leidas.iter().map(|b| b.consulta).collect();
        let mut listados_pendientes = 0usize;
        for c in &del_lote {
            let seleccion = &resumen.lote.consultas[c.numero];
            let clave = (seleccion.tribunal.clone(), c.lista.to_string(), seleccion.modality.clone());
            // Se compara el lote, no su nombre: dos carpetas homónimas en lugares distintos son dos lotes.
            if let Some((indice_previo, previo)) = consultas_vistas.insert(clave, (n, etiqueta.clone())) {
                if indice_previo != n {
                    return Err(format!(
                        "La consulta {} está en dos lotes («{previo}» y «{etiqueta}»). Quita uno para no duplicar registros.",
                        c.nombre
                    ));
                }
            }
            if seleccion.report.as_deref().is_some_and(|r| !r.is_empty() && r != "0") {
                m.filtros.push(format!("{etiqueta}: filtro de vencimiento en {}", c.nombre));
            }
            let seleccionado = |t: Option<&'static str>| m.tribunales.as_ref().is_none_or(|e| t.is_some_and(|t| e.contains(&t)));
            for (archivo, filas) in &c.paginas_xls {
                m.fuentes.push(Fuente {
                    archivo: archivo.clone(),
                    lote: etiqueta.clone(),
                    tipo: "EXCEL",
                    tribunal: c.tribunal,
                    categoria: c.categoria,
                    lista: c.lista,
                    registros: *filas,
                    resultado: "PROCESADO",
                    seleccionado: seleccionado(c.tribunal),
                });
            }
            if c.vacia {
                m.fuentes.push(Fuente {
                    archivo: c.evidencia.clone().unwrap_or_default(),
                    lote: etiqueta.clone(),
                    tipo: if c.evidencia.is_some() { "PDF" } else { "SIN ARCHIVO" },
                    tribunal: c.tribunal,
                    categoria: c.categoria,
                    lista: c.lista,
                    registros: 0,
                    resultado: if c.tribunal.is_some() { "SIN REGISTROS" } else { "NO CLASIFICADO" },
                    seleccionado: c.tribunal.is_some() && seleccionado(c.tribunal),
                });
            }
            // Páginas cuyo listado no entregó vínculos con la lectura de bitácoras pedida: sus filas no se leyeron.
            let listado_pendiente = |pagina: usize| {
                resumen.lote.lecturas.bitacoras
                    && resumen
                        .avisos
                        .iter()
                        .any(|a| a.starts_with(&format!("{}pendientes", flow_captura::recoleccion::prefijo_listado(c.numero, pagina))))
            };
            for fila in &c.filas {
                let mut r = Registro4 {
                    lista: c.lista,
                    categoria: categoria(&fila.programa),
                    tribunal: fila.tribunal,
                    columnas: fila.columnas.clone(),
                    archivo: fila.archivo.clone(),
                    lote: etiqueta.clone(),
                    fecha_obs: None,
                    etapa_ultima: String::new(),
                    control: String::new(),
                    alerta: String::new(),
                };
                let identidad = vinculos::identidad(&fila.rit, &fila.rut, &fila.nombre, &fila.programa);
                let candidatos: Vec<&Vinculo> = indice
                    .get(&(c.numero, identidad))
                    .map(|v| v.iter().filter(|v| v.tribunal == seleccion.tribunal && v.bitacora_verificada).collect())
                    .unwrap_or_default();
                let remotos: BTreeSet<(&str, &str, &str)> =
                    candidatos.iter().map(|v| (v.causa.as_str(), v.ingreso.as_str(), v.etapa.as_str())).collect();
                r.control = match remotos.len() {
                    0 if listado_pendiente(fila.pagina) => NO_COMPLETADA.to_string(),
                    0 => SIN_BITACORA.to_string(),
                    1 => match bitacoras.get(&recoleccion::clave("bitacora", candidatos[0])) {
                        None => NO_COMPLETADA.to_string(),
                        Some(b) if b.control == OK || b.control == NOMBRE_VISIBLE => {
                            let etapas: Vec<(NaiveDateTime, String)> = b.entradas.iter().map(|e| (e.0, e.1.clone())).collect();
                            let (dia, etapa_ultima, resultado) = ultima(&etapas, corte);
                            r.fecha_obs = dia;
                            r.etapa_ultima = etapa_ultima.to_string();
                            // El aviso de nombre visible se conserva junto al resultado: la fila queda en amarillo.
                            if b.control == OK {
                                resultado.to_string()
                            } else {
                                format!("{resultado} · {}", b.control)
                            }
                        }
                        Some(b) => b.control.clone(),
                    },
                    _ => AMBIGUA.to_string(),
                };
                if r.lista == "Cumplimiento" && r.etapa_ultima == "Espera" {
                    r.alerta = ALERTA.to_string();
                }
                m.registros.push(r);
            }
            let listado_sin_leer = c.filas.iter().any(|f| listado_pendiente(f.pagina));
            if listado_sin_leer {
                pendientes.insert(c.numero);
                listados_pendientes +=
                    c.filas.iter().map(|f| f.pagina).collect::<BTreeSet<_>>().into_iter().filter(|p| listado_pendiente(*p)).count();
            }
            if !c.filas.is_empty() && !listado_sin_leer && !paginas.iter().any(|p| p.consulta == c.numero) {
                m.sin_bitacoras.push((format!("{etiqueta}: {}", c.nombre), c.tribunal.unwrap_or(SIN_TRIBUNAL).to_string()));
            }
        }
        if !pendientes.is_empty() {
            let consultas: Vec<String> = pendientes.iter().map(|i| (i + 1).to_string()).collect();
            let listados = if listados_pendientes > 0 {
                format!(" y {listados_pendientes} páginas del listado quedaron sin vínculos")
            } else {
                String::new()
            };
            m.avisos.push(format!(
                "Lote «{etiqueta}»: {} bitácoras no se pudieron leer{listados} (consultas {}); sus registros quedan como \
                 «{NO_COMPLETADA}». Retoma el lote en Captura para reintentarlas.",
                no_leidas.len(),
                consultas.join(", ")
            ));
        }
    }
    // Como Python: sin registros ni consultas sin registros reconocidas no hay Meta 4; una consulta vacía es evidencia.
    if m.registros.is_empty() && !consultas.iter().any(|c| c.vacia) {
        return Err("Los lotes no traen registros de Espera o Cumplimiento de FAE o Residencia.".into());
    }
    if let Some(elegidos) = &m.tribunales {
        let con_candidatos =
            m.registros.iter().filter_map(|r| r.tribunal).chain(consultas.iter().filter(|c| c.vacia).filter_map(|c| c.tribunal));
        if !con_candidatos.into_iter().any(|t| elegidos.contains(&t)) {
            return Err("Los lotes no contienen registros de los tribunales elegidos.".into());
        }
        m.registros.retain(|r| r.tribunal.is_some_and(|t| elegidos.contains(&t)));
        m.observaciones.retain(|o| elegidos.iter().any(|t| *t == o.tribunal));
        m.lecturas.retain(|l| elegidos.iter().any(|t| *t == l.tribunal));
        m.sin_bitacoras.retain(|(_, t)| elegidos.iter().any(|e| e == t));
    }
    let sin_atribuir = consultas.iter().filter(|c| c.vacia && c.tribunal.is_none()).count();
    if sin_atribuir > 0 {
        m.avisos.push(format!(
            "{sin_atribuir} consultas sin registros no se pudieron atribuir a un tribunal (el lote guarda solo su código); \
             quedan como NO CLASIFICADO en el inventario."
        ));
    }
    let fuera = m.registros.iter().filter(|r| r.categoria.is_none() || r.tribunal.is_none()).count();
    if fuera > 0 {
        m.avisos.push(format!("{fuera} registros sin tribunal o modalidad reconocidos quedan fuera de META 4 (están en Registros)."));
    }
    if !m.sin_bitacoras.is_empty() {
        m.avisos.push(format!("{} consultas con registros no tienen bitácoras leídas.", m.sin_bitacoras.len()));
    }
    if !m.filtros.is_empty() {
        m.avisos.push(format!("Lotes con filtros: META 4 contiene solo lo que entregó cada filtro ({}).", m.filtros.join("; ")));
    }
    let incompletas = m.registros.iter().filter(|r| r.control == PERIODO_INCOMPLETO).count();
    if incompletas > 0 {
        m.avisos.push(format!("{incompletas} bitácoras no cubren todo el período de Meta 4; sus fechas quedan vacías. Vuelva a leerlas con una ventana mayor o ajuste el corte."));
    }
    let antes = m.registros.iter().filter(|r| r.control == LEIDA_ANTES).count();
    if antes > 0 {
        m.avisos.push(format!("{antes} bitácoras se leyeron antes de la fecha de corte: usa como corte el día de lectura o vuelve a leerlas."));
    }
    Ok(m)
}

/// Tribunales que aparecen en los lotes, en el orden oficial.
pub fn tribunales_disponibles(m: &Meta4) -> Vec<&'static str> {
    tribunales::TRIBUNALES.into_iter().filter(|t| m.registros.iter().any(|r| r.tribunal == Some(*t))).collect()
}

/// Carpeta de salida nueva; se rechaza si ya existe.
pub fn validar_destino(destino: &Path) -> Result<(), String> {
    if destino.exists() {
        return Err(format!("La carpeta de salida ya existe: {}. Elige una nueva.", destino.display()));
    }
    Ok(())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn f(a: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, d).unwrap()
    }
    fn e(texto: &str, etapa: &str) -> (NaiveDateTime, String) {
        (momento(texto).unwrap(), etapa.to_string())
    }

    #[test]
    fn mes_calendario_movil() {
        assert_eq!(periodo(f(2026, 10, 16)).0, f(2026, 9, 16));
        assert_eq!(periodo(f(2026, 3, 31)).0, f(2026, 2, 28));
        assert_eq!(periodo(f(2026, 1, 15)).0, f(2025, 12, 15));
    }

    #[test]
    fn ultima_ignora_informes_y_fuera_de_periodo() {
        let corte = f(2026, 10, 16);
        let entradas = vec![
            e("2026-10-01T09:00:00", "Espera"),
            e("2026-10-10T10:00:00", "Cumplimiento"),
            e("2026-10-15T10:00:00", "Informes"),
            e("2026-10-20T10:00:00", "Espera"),
            e("2026-08-01", "Espera"),
        ];
        assert_eq!(ultima(&entradas, corte), (Some(f(2026, 10, 10)), "Cumplimiento", VERIFICADA));
        assert_eq!(ultima(&[e("2026-08-01", "Espera")], corte), (None, "", SIN_OBSERVACION));
        let empate = vec![e("2026-10-10T10:00:00", "Espera"), e("2026-10-10T10:00:00", "Cumplimiento")];
        assert_eq!(ultima(&empate, corte), (Some(f(2026, 10, 10)), "", EMPATE));
    }

    #[test]
    fn nombres() {
        assert!(nombres_compatibles("PÉREZ SOTO SOFÍA", "Sofia Perez Soto"));
        assert!(nombres_compatibles("Sofía Pérez", "SOFIA ANDREA PEREZ SOTO"));
        assert!(!nombres_compatibles("Sofía", "Sofía Pérez"));
        assert!(!nombres_compatibles("Matías Rojas", "Sofía Pérez"));
    }
}

#[cfg(test)]
mod pruebas_vinculos_sin_lectura {
    use super::*;
    use serde_json::json;

    /// M3: Captura guarda los vínculos del listado aunque no se pidan bitácoras; Meta 4 sigue informando la consulta
    /// como «sin bitácoras leídas» y la fila como sin bitácora, no como «lectura no completada».
    #[test]
    fn vinculos_sin_lectura_de_bitacoras_cuentan_como_sin_bitacoras() {
        let carpeta = std::env::temp_dir().join(format!("flow-meta4-m3-ficticio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&carpeta);
        std::fs::create_dir_all(&carpeta).unwrap();
        let espera = "<html><table><tr><th>RIT</th><th>Tribunal</th><th>RUT</th><th>Nombre</th><th>Tipo</th><th>Sexo</th>\
            <th>Derivación</th><th>T Espera</th><th>Fec. Resolución</th></tr>\
            <tr><td>X-4-2099</td><td>Jgdo. L. y G. de Laja</td><td>44444444-2</td><td>LUCAS ARAYA</td><td>Protección</td><td>M</td><td>FAE PRO Ficticio</td><td>35</td><td>05/09/2026</td></tr>\
            </table></html>";
        std::fs::write(carpeta.join("espera.xls"), espera).unwrap();
        let vinculo = json!({"fila": 1, "rit": "X-4-2099", "rut": "44444444-2", "nombre": "LUCAS ARAYA", "programa": "FAE PRO Ficticio",
            "tribunal": "901", "causa": "c1", "ingreso": "i1", "etapa": "3", "antiguo": "", "tipo_popup": "12", "rit_enlace": "X-4-2099",
            "calendario": null, "bitacora_verificada": true, "historia_verificada": false, "diagnostico": ""});
        let resumen: Resumen = serde_json::from_value(json!({
            "version": 1,
            "lote": {"consultas": [{"tribunal":"901","screen":"seguimiento","tab":"Espera","modality":"3","report":null,"state":null,"month":null,"year":null}],
                     "fechas": null, "conservar_filtros": false, "evidencia": false, "pausa_ms": 250,
                     "lecturas": {"bitacoras": false, "historia": false, "calendario": false, "meses": 1}},
            "estado": "VALIDADA", "terminadas": [0], "vacias": [],
            "archivos": [{"nombre": "espera.xls", "sha256": huella(espera.as_bytes()), "consulta": 0, "pagina": 1, "total": 1, "registros": 1}],
            "avisos": [], "paginas_lectura": [{"consulta": 0, "numero": 1, "vinculos": [vinculo]}]
        }))
        .unwrap();
        resumen.guardar(&carpeta).unwrap();
        let dia = |s: &str| s.parse::<NaiveDate>().unwrap();
        let m = calcular(std::slice::from_ref(&carpeta), dia("2026-10-16"), dia("2026-10-16"), &[]).unwrap();
        let _ = std::fs::remove_dir_all(&carpeta);
        assert_eq!(m.sin_bitacoras.len(), 1);
        assert_eq!(m.registros[0].control, SIN_BITACORA);
    }
}
