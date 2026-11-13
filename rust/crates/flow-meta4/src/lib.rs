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

#[derive(Debug, Clone, PartialEq)]
pub struct Lectura {
    pub tribunal: String,
    pub rit: String,
    pub nombre: String,
    pub programa: String,
    pub causa: String,
    pub ingreso: String,
    pub control: String,
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

/// Lee uno o varios lotes de Flow y calcula Meta 4. No escribe nada.
pub fn calcular(carpetas: &[PathBuf], corte: NaiveDate, emision: NaiveDate, elegidos: &[String]) -> Result<Meta4, String> {
    if carpetas.is_empty() {
        return Err("Agrega al menos un lote de registros.".into());
    }
    let tribunales_elegidos = normalizar_tribunales(elegidos)?;
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
    };
    let (inicio, fin) = periodo(corte);
    let mut consultas_vistas: HashMap<(String, String, String), String> = HashMap::new();
    let mut exportadas: BTreeSet<String> = BTreeSet::new();
    for carpeta in carpetas {
        let resumen = Resumen::leer(carpeta)?;
        let etiqueta = carpeta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        m.lotes.push(etiqueta.clone());
        if resumen.lote.fechas.is_some() {
            m.filtros.push(format!("{etiqueta}: filtro de fechas"));
        }
        if resumen.lote.conservar_filtros {
            m.filtros.push(format!("{etiqueta}: filtros del formulario"));
        }
        let mut lecturas: HashMap<String, Result<serde_json::Value, String>> = HashMap::new();
        let mut capturadas: HashMap<String, String> = HashMap::new();
        for r in resumen.lecturas.iter().filter(|r| r.tipo == "bitacora") {
            lecturas.insert(r.clave.clone(), recoleccion::leer(carpeta, r));
            capturadas.insert(r.clave.clone(), r.capturada.clone());
        }
        let mut todos: Vec<Vinculo> = resumen.paginas_lectura.iter().flat_map(|p| p.vinculos.clone()).collect();
        vinculos::descartar_ambiguos(&mut todos);
        let mut indice: HashMap<(usize, [String; 4]), Vec<Vinculo>> = HashMap::new();
        let mut siguiente = todos.into_iter();
        for pagina in &resumen.paginas_lectura {
            for _ in &pagina.vinculos {
                let v = siguiente.next().expect("vínculo");
                indice.entry((pagina.consulta, v.clave())).or_default().push(v);
            }
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
            let clave = (seleccion.tribunal.clone(), lista.to_string(), seleccion.modality.clone());
            if let Some(previo) = consultas_vistas.insert(clave, etiqueta.clone()) {
                return Err(format!(
                    "La consulta {nombre_consulta} está en dos lotes («{previo}» y «{etiqueta}»). Quita uno para no duplicar registros."
                ));
            }
            if seleccion.report.as_deref().is_some_and(|r| !r.is_empty() && r != "0") {
                m.filtros.push(format!("{etiqueta}: filtro de vencimiento en {nombre_consulta}"));
            }
            let mut filas_consulta = 0usize;
            let mut tribunal_consulta = String::new();
            for archivo in resumen.archivos.iter().filter(|a| a.consulta == numero && a.nombre.ends_with(".xls")) {
                let datos = std::fs::read(ruta_archivo(carpeta, &archivo.nombre)?)
                    .map_err(|_| format!("Falta una descarga del lote «{etiqueta}»."))?;
                if huella(&datos) != archivo.sha256 {
                    return Err(format!("Una descarga del lote «{etiqueta}» cambió desde su verificación."));
                }
                let libro = flow_tabular::leer_bytes(&archivo.nombre, datos)?;
                let tabla = flow_tabular::leer_tabla(&libro, Some(etapa), None)?;
                for fila in &tabla.filas {
                    filas_consulta += 1;
                    let columnas: Vec<(String, Celda)> = tabla
                        .cabeceras
                        .iter()
                        .cloned()
                        .zip(fila.celdas.iter().cloned())
                        .filter(|(t, _)| !t.trim().is_empty())
                        .collect();
                    let mut r = Registro4 {
                        lista,
                        categoria: categoria(&fila.registro.texto("programa")),
                        tribunal: tribunal(&fila.registro.texto("tribunal")),
                        columnas,
                        archivo: archivo.nombre.clone(),
                        lote: etiqueta.clone(),
                        fecha_obs: None,
                        etapa_ultima: String::new(),
                        control: String::new(),
                        alerta: String::new(),
                    };
                    if tribunal_consulta.is_empty() {
                        tribunal_consulta = r.tribunal_o_sin().to_string();
                    }
                    let identidad = vinculos::identidad(
                        &fila.registro.texto("rit"),
                        &fila.registro.texto("rut"),
                        &fila.registro.texto("nombre"),
                        &fila.registro.texto("programa"),
                    );
                    let candidatos: Vec<&Vinculo> = indice
                        .get(&(numero, identidad))
                        .map(|v| v.iter().filter(|v| v.tribunal == seleccion.tribunal && v.bitacora_verificada).collect())
                        .unwrap_or_default();
                    let remotos: BTreeSet<(&str, &str, &str)> =
                        candidatos.iter().map(|v| (v.causa.as_str(), v.ingreso.as_str(), v.etapa.as_str())).collect();
                    r.control = match remotos.len() {
                        0 => SIN_BITACORA.to_string(),
                        1 => {
                            let v = candidatos[0];
                            let clave = recoleccion::clave("bitacora", v);
                            let (control, datos) = match lecturas.get(&clave) {
                                None => (NO_COMPLETADA.to_string(), None),
                                Some(Err(_)) => (NO_VERIFICABLE.to_string(), None),
                                Some(Ok(lectura)) => {
                                    let datos = &lectura["datos"];
                                    let rit_visible = datos["rit"].as_str().unwrap_or("");
                                    let leida = chrono::DateTime::parse_from_rfc3339(capturadas.get(&clave).map(String::as_str).unwrap_or(""))
                                        .map(|f| f.with_timezone(&chrono::Local).date_naive())
                                        .ok();
                                    if !rit_visible.is_empty() && norma(rit_visible) != norma(&fila.registro.texto("rit")) {
                                        (IDENTIDAD_VISIBLE.to_string(), None)
                                    } else if leida.is_none_or(|d| d < corte) {
                                        (LEIDA_ANTES.to_string(), None)
                                    } else if !matches!(
                                        (datos["desde"].as_str().and_then(|s|s.parse::<NaiveDate>().ok()),
                                         datos["hasta"].as_str().and_then(|s|s.parse::<NaiveDate>().ok())),
                                        (Some(desde),Some(hasta)) if desde<=inicio&&hasta>=fin
                                    ) {
                                        (PERIODO_INCOMPLETO.to_string(), None)
                                    } else {
                                        let persona = datos["persona"].as_str().unwrap_or("");
                                        let aviso = (!persona.is_empty() && !nombres_compatibles(persona, &fila.registro.texto("nombre")))
                                            .then_some(NOMBRE_VISIBLE);
                                        (aviso.unwrap_or("OK").to_string(), Some(datos))
                                    }
                                }
                            };
                            let mut final_control = control.clone();
                            if let Some(datos) = datos {
                                let mut entradas = Vec::new();
                                for e in datos["entries"].as_array().into_iter().flatten() {
                                    let fecha = momento(e["fecha"].as_str().unwrap_or(""))
                                        .ok_or(format!("Fecha de bitácora no reconocida en el lote «{etiqueta}»."))?;
                                    let etapa_texto = e["etapa"].as_str().unwrap_or("").to_string();
                                    if inicio <= fecha.date() && fecha.date() <= fin && exportadas.insert(format!("{clave}|{}|{fecha}", e["entry_id"])) {
                                        m.observaciones.push(ObservacionMes {
                                            tribunal: r.tribunal_o_sin().to_string(),
                                            rit: fila.registro.texto("rit"),
                                            rut: fila.registro.texto("rut"),
                                            nombre: fila.registro.texto("nombre"),
                                            programa: fila.registro.texto("programa"),
                                            causa: v.causa.clone(),
                                            ingreso: v.ingreso.clone(),
                                            fecha,
                                            etapa: etapa_texto.clone(),
                                            tipo: e["tipo"].as_str().unwrap_or("").to_string(),
                                            texto: e["texto"].as_str().unwrap_or("").to_string(),
                                        });
                                    }
                                    entradas.push((fecha, etapa_texto));
                                }
                                let (dia, etapa_ultima, resultado) = ultima(&entradas, corte);
                                r.fecha_obs = dia;
                                r.etapa_ultima = etapa_ultima.to_string();
                                // El aviso de nombre visible se conserva junto al resultado: la fila queda en amarillo.
                                final_control = if control == "OK" { resultado.to_string() } else { format!("{resultado} · {control}") };
                            }
                            m.lecturas.push(Lectura {
                                tribunal: r.tribunal_o_sin().to_string(),
                                rit: fila.registro.texto("rit"),
                                nombre: fila.registro.texto("nombre"),
                                programa: fila.registro.texto("programa"),
                                causa: v.causa.clone(),
                                ingreso: v.ingreso.clone(),
                                control: control.clone(),
                            });
                            final_control
                        }
                        _ => AMBIGUA.to_string(),
                    };
                    if r.lista == "Cumplimiento" && r.etapa_ultima == "Espera" {
                        r.alerta = ALERTA.to_string();
                    }
                    m.registros.push(r);
                }
            }
            if filas_consulta > 0 && !resumen.paginas_lectura.iter().any(|p| p.consulta == numero) {
                m.sin_bitacoras.push((format!("{etiqueta}: {nombre_consulta}"), tribunal_consulta));
            }
        }
    }
    if m.registros.is_empty() {
        return Err("Los lotes no traen registros de Espera o Cumplimiento de FAE o Residencia.".into());
    }
    if let Some(elegidos) = &m.tribunales {
        for t in elegidos {
            if !m.registros.iter().any(|r| r.tribunal == Some(*t)) {
                return Err(format!("El tribunal {t} no tiene registros en los lotes agregados."));
            }
        }
        m.registros.retain(|r| r.tribunal.is_some_and(|t| elegidos.contains(&t)));
        m.observaciones.retain(|o| elegidos.iter().any(|t| *t == o.tribunal));
        m.lecturas.retain(|l| elegidos.iter().any(|t| *t == l.tribunal));
        m.sin_bitacoras.retain(|(_, t)| elegidos.iter().any(|e| e == t));
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
