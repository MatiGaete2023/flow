//! Reconstrucción de META 4 desde Registros.xlsx corregido por el usuario.
//! Nunca relee SITFA, modifica el original ni acepta un libro sin cabeceras comprobables.
use crate::salida::{CUMPLIMIENTO, ESPERA, EXTRA};
use crate::tribunales::{categoria, norma, tribunal};
use crate::{etapa_de, normalizar_tribunales, Lectura, Meta4, Registro4, ALERTA, SIN_TRIBUNAL};
use chrono::NaiveDate;
use flow_dominio::Celda;
use flow_tabular::{Formato, Hoja};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Estado comprobado del libro revisado y del reporte original; la bitácora se conserva intacta.
pub struct Regeneracion {
    pub meta: Meta4,
    pub bitacoras: PathBuf,
    pub huella_registros: String,
}

fn fecha_iso(valor: &Value, descripcion: &str) -> Result<NaiveDate, String> {
    let texto = valor.as_str().ok_or(format!("El reporte no trae {descripcion}."))?;
    texto.parse().map_err(|_| format!("Fecha ilegible en {descripcion} del reporte."))
}

fn textos(valor: &Value, nombre: &str) -> Result<Vec<String>, String> {
    let lista = valor.as_array().ok_or(format!("El reporte no trae una lista válida de {nombre}."))?;
    if lista.len() > 10000 {
        return Err(format!("La lista {nombre} del reporte es excesiva."));
    }
    lista.iter().map(|v| v.as_str().map(str::to_string).ok_or(format!("Elemento inválido en {nombre} del reporte."))).collect()
}

fn valor<'a>(fila: &'a [Celda], cabeceras: &BTreeMap<String, usize>, nombre: &str) -> &'a Celda {
    static VACIA: Celda = Celda::Vacia;
    cabeceras.get(&norma(nombre)).and_then(|i| fila.get(*i)).unwrap_or(&VACIA)
}

fn leer_hoja(hoja: &Hoja, lista: &'static str, esperadas: &[&str]) -> Result<Vec<Registro4>, String> {
    let encabezado = hoja.filas.first().ok_or(format!("La hoja {lista} de Registros.xlsx está vacía."))?;
    let mut cabeceras = BTreeMap::new();
    for (i, celda) in encabezado.iter().enumerate() {
        let titulo = norma(&celda.texto());
        if !titulo.is_empty() && cabeceras.insert(titulo.clone(), i).is_some() {
            return Err(format!("La hoja {lista} tiene la columna {titulo} repetida."));
        }
    }
    for titulo in esperadas.iter().copied().chain(EXTRA) {
        if !cabeceras.contains_key(&norma(titulo)) {
            return Err(format!("En la hoja {lista} falta la columna {titulo}."));
        }
    }
    let mut registros = Vec::new();
    for (i, fila) in hoja.filas.iter().enumerate().skip(1) {
        if valor(fila, &cabeceras, "RIT").es_vacia() {
            if fila.iter().any(|c| !c.es_vacia()) {
                return Err(format!("La hoja {lista}, fila {} contiene datos pero no tiene RIT.", i + 1));
            }
            continue;
        }
        let fecha = valor(fila, &cabeceras, "FECHA OBS.");
        let fecha_obs = if fecha.es_vacia() {
            None
        } else {
            Some(fecha.fecha().ok_or(format!("FECHA OBS. ilegible en la hoja {lista}, fila {}.", i + 1))?)
        };
        let etapa = valor(fila, &cabeceras, "ETAPA ÚLTIMA OBS.").texto().trim().to_string();
        let control = valor(fila, &cabeceras, "CONTROL BITÁCORA").texto().trim().to_string();
        let columnas = esperadas.iter().map(|t| ((*t).to_string(), valor(fila, &cabeceras, t).clone())).collect();
        let mut r = Registro4 {
            lista,
            categoria: categoria(&valor(fila, &cabeceras, "DERIVACIÓN").texto()),
            tribunal: tribunal(&valor(fila, &cabeceras, "TRIBUNAL").texto()),
            columnas,
            archivo: valor(fila, &cabeceras, "ARCHIVO ORIGEN").texto(),
            lote: valor(fila, &cabeceras, "RUTA ORIGEN").texto(),
            fecha_obs,
            etapa_ultima: etapa,
            control,
            alerta: String::new(),
        };
        if lista == "Cumplimiento" && etapa_de(&r.etapa_ultima) == Some("Espera") {
            r.alerta = ALERTA.into();
        }
        registros.push(r);
    }
    Ok(registros)
}

fn lecturas(bitacoras: &Path) -> Result<Vec<Lectura>, String> {
    let libro = flow_tabular::leer(bitacoras)?;
    if libro.formato != Formato::Xlsx {
        return Err("Bitacoras_del_mes.xlsx no es un libro XLSX válido.".into());
    }
    let hoja = libro.hojas.iter().find(|h| norma(&h.nombre) == "CONTROL DE LECTURAS")
        .ok_or("Bitacoras_del_mes.xlsx no contiene Control de lecturas.")?;
    let encabezado = hoja.filas.first().ok_or("Control de lecturas está vacío.")?;
    let requeridas = ["TRIBUNAL", "RIT", "NOMBRE", "PROGRAMA", "CAUSA ID", "INGRESO ID", "CONTROL"];
    let mut indices = Vec::new();
    for titulo in requeridas {
        indices.push(encabezado.iter().position(|c| norma(&c.texto()) == norma(titulo))
            .ok_or(format!("Control de lecturas no tiene {titulo}."))?);
    }
    let mut salida = Vec::new();
    for fila in hoja.filas.iter().skip(1) {
        if fila.iter().all(Celda::es_vacia) {
            continue;
        }
        let v: Vec<String> = indices.iter().map(|i| fila.get(*i).map(Celda::texto).unwrap_or_default()).collect();
        salida.push(Lectura {
            tribunal: v[0].clone(), rit: v[1].clone(), nombre: v[2].clone(),
            programa: v[3].clone(), causa: v[4].clone(), ingreso: v[5].clone(),
            control: v[6].clone(),
        });
    }
    Ok(salida)
}

/// Carga únicamente los productos producidos por Flow y el Excel revisado; verifica la estructura.
/// La columna ALERTA no es una entrada: se recalcula a partir de la etapa revisada.
pub fn cargar(registros: &Path) -> Result<Regeneracion, String> {
    if !registros.is_file() {
        return Err("Elige el archivo Registros.xlsx revisado.".into());
    }
    let carpeta = registros.parent().ok_or("Registros.xlsx no tiene carpeta de origen.")?;
    let reporte = carpeta.join("Reporte META4.json");
    let datos = std::fs::read(&reporte).map_err(|_| "Falta Reporte META4.json junto a Registros.xlsx.".to_string())?;
    if datos.len() > 1024 * 1024 {
        return Err("Reporte META4.json excede 1 MiB.".into());
    }
    let previo: Value = serde_json::from_slice(&datos).map_err(|_| "Reporte META4.json es ilegible.".to_string())?;
    if previo["herramienta"] != "Flow · Meta 4" {
        return Err("El reporte no corresponde a Meta 4 de Flow.".into());
    }
    let corte = fecha_iso(&previo["periodo"]["hasta"], "período.hasta")?;
    let emision = fecha_iso(&previo["fecha_emision"], "fecha_emision")?;
    let tribunales = match &previo["tribunales_incluidos"] {
        Value::String(t) if t == "Todos" => None,
        Value::Array(_) => Some(normalizar_tribunales(&textos(&previo["tribunales_incluidos"], "tribunales_incluidos")?)?
            .ok_or("La lista de tribunales está vacía.")?),
        _ => return Err("El filtro de tribunales del reporte es ilegible.".into()),
    };
    let libro = flow_tabular::leer(registros)?;
    if libro.formato != Formato::Xlsx {
        return Err("Registros.xlsx debe ser un archivo XLSX válido.".into());
    }
    let mut registros_revisados = Vec::new();
    for (nombre, lista, cabeceras) in [("ESPERA", "Espera", &ESPERA[..]), ("CUMPLIMIENTO", "Cumplimiento", &CUMPLIMIENTO[..])] {
        let hojas: Vec<_> = libro.hojas.iter().filter(|h| norma(&h.nombre) == nombre).collect();
        if hojas.len() != 1 {
            return Err(format!("Registros.xlsx debe contener exactamente una hoja {nombre}."));
        }
        registros_revisados.extend(leer_hoja(hojas[0], lista, cabeceras)?);
    }
    if registros_revisados.is_empty() {
        return Err("Registros.xlsx no tiene registros para regenerar.".into());
    }
    if let Some(elegidos) = &tribunales {
        for r in &registros_revisados {
            if r.tribunal.is_none_or(|t| !elegidos.contains(&t)) {
                return Err(format!("Registros.xlsx contiene un tribunal fuera del filtro original: {}.", r.texto("TRIBUNAL")));
            }
        }
    }
    let bitacoras = carpeta.join("Bitacoras_del_mes.xlsx");
    let controles = lecturas(&bitacoras)?;
    // Los reportes nuevos registran la asociación exacta consulta/tribunal.
    // Los anteriores solo guardaban la etiqueta: no se inventa una atribución.
    let sin_bitacoras: Vec<(String, String)> = if let Some(detalle) = previo["consultas_sin_bitacoras_detalle"].as_array() {
        if detalle.len() > 10000 {
            return Err("Demasiadas consultas sin bitácoras en el reporte.".into());
        }
        let mut filas = Vec::new();
        for item in detalle {
            let etiqueta = item["consulta"].as_str().ok_or("Consulta sin bitácora inválida.")?;
            let nombre = item["tribunal"].as_str().ok_or("Tribunal de consulta sin bitácora inválido.")?;
            if tribunal(nombre) != Some(nombre) && nombre != SIN_TRIBUNAL {
                return Err("Tribunal no reconocido en las consultas sin bitácoras.".into());
            }
            filas.push((etiqueta.to_string(), nombre.to_string()));
        }
        filas
    } else {
        textos(&previo["consultas_sin_bitacoras"], "consultas_sin_bitacoras")?
            .into_iter().map(|etiqueta| (etiqueta, SIN_TRIBUNAL.to_string())).collect()
    };
    let filtros = textos(&previo["filtros_registros"], "filtros_registros")?;
    let lotes = textos(&previo["lotes"], "lotes")?;
    let mut avisos = textos(&previo["avisos"], "avisos")?;
    if previo.get("consultas_sin_bitacoras_detalle").is_none() && !sin_bitacoras.is_empty() {
        avisos.push("El reporte original no conserva el tribunal de cada consulta sin bitácoras: se muestran sin atribuir, sin inferir un tribunal.".into());
    }
    avisos.push("Regenerado desde Registros.xlsx revisado; se preservó Bitacoras_del_mes.xlsx.".into());
    let sin_clasificar = registros_revisados.iter().filter(|r| r.categoria.is_none() || r.tribunal.is_none()).count();
    if sin_clasificar > 0 {
        avisos.push(format!("{sin_clasificar} registros sin clasificación siguen fuera de la planilla."));
    }
    let meta = Meta4 {
        corte, emision, tribunales, registros: registros_revisados, observaciones: Vec::new(),
        lecturas: controles, sin_bitacoras, filtros, lotes, avisos,
    };
    Ok(Regeneracion { meta, bitacoras, huella_registros: libro.sha256 })
}

/// Regenera los cinco productos en una carpeta nueva, sin sobrescribir ni modificar los originales.
pub fn ejecutar(registros: &Path, destino: &Path) -> Result<Vec<PathBuf>, String> {
    ejecutar_con_plantilla(registros, destino, None)
}
pub fn ejecutar_con_plantilla(registros: &Path, destino: &Path, plantilla: Option<&Path>) -> Result<Vec<PathBuf>, String> {
    let estado = cargar(registros)?;
    crate::salida::publicar_regenerado_con_plantilla(
        &estado.meta, destino, &estado.bitacoras, &estado.huella_registros, plantilla
    )
}
