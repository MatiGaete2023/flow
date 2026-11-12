//! Pasa un lote local verificado a Trabajo. Reúne las páginas antes del pre-pase y nunca cruza solo por nombre.

use crate::{analizar_filas, Contexto, Trabajo};
use flow_captura::{
    lotes::{huella, ruta_archivo, Resumen},
    recoleccion,
    vinculos::{self, Vinculo},
};
use flow_dominio::{Celda, Etapa, Fecha, Historia, Observacion, Tramite};
use flow_motor::Config;
use flow_tabular::Tabla;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
};

fn fecha(v: &Value) -> Option<Fecha> {
    v.as_str().and_then(|s| s.get(..10)).and_then(|s| s.parse().ok())
}

fn historia(valor: &Value, corte: Fecha) -> Historia {
    let cobertura = if valor["advertencias"].as_array().is_some_and(|xs| {
        xs.iter().any(|x| matches!(x["codigo"].as_str(), Some("HISTORIA_POSIBLE_PAGINACION" | "HISTORIA_FECHA_ILEGIBLE")))
    }) {
        "PARCIAL"
    } else {
        "COMPLETA"
    };
    let tramites = valor["filas"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| {
            Some(Tramite::nuevo(f["descripcion"].as_str().unwrap_or(""), fecha(&f["fecha_iso"])?, f["folio"].as_str().unwrap_or("")))
        })
        .collect();
    Historia::nueva(cobertura, fecha(&valor["desde"]), fecha(&valor["hasta"]), tramites).al_corte(corte)
}

pub fn importar(carpeta: &Path, resumen: &Resumen, corte: Fecha, config: &Config) -> Result<Vec<Trabajo>, String> {
    let mut lecturas = BTreeMap::new();
    for r in &resumen.lecturas {
        lecturas.insert(r.clave.clone(), recoleccion::leer(carpeta, r)?);
    }
    let mut vinculos = resumen.paginas_lectura.iter().flat_map(|p| p.vinculos.clone()).collect::<Vec<_>>();
    vinculos::descartar_ambiguos(&mut vinculos);
    let mut indice: HashMap<(usize, [String; 4]), Vec<&Vinculo>> = HashMap::new();
    let mut vinculados = vinculos.iter();
    for pagina in &resumen.paginas_lectura {
        for _ in &pagina.vinculos {
            let v = vinculados.next().unwrap();
            indice.entry((pagina.consulta, v.clave())).or_default().push(v);
        }
    }
    let mut tablas: Vec<(Tabla, Vec<String>)> = vec![];
    let mut contexto = Contexto { proximos: resumen.lote.lecturas.calendario.then(HashMap::new), ..Default::default() };
    let mut avisos = resumen.avisos.clone();
    for archivo in &resumen.archivos {
        let ruta = ruta_archivo(carpeta, &archivo.nombre)?;
        if std::fs::metadata(&ruta).map_err(|_| "Falta una descarga del lote.")?.len() > 80 * 1024 * 1024 {
            return Err("Una descarga excede el tamaño permitido.".into());
        }
        let datos = std::fs::read(ruta).map_err(|_| "No se pudo leer una descarga del lote.")?;
        if huella(&datos) != archivo.sha256 {
            return Err("Cambió una descarga desde su verificación; no se importó el lote.".into());
        }
        let seleccion = resumen.lote.consultas.get(archivo.consulta).ok_or("La descarga no identifica su consulta.")?;
        if !archivo.nombre.ends_with(".xls") {
            continue;
        }
        let Some(etapa) = Etapa::desde(&seleccion.tab) else {
            continue;
        };
        let libro = flow_tabular::leer_bytes(&archivo.nombre, datos)?;
        let mut tabla = flow_tabular::leer_tabla(&libro, Some(etapa), None)?;
        for fila in &mut tabla.filas {
            fila.id = format!("captura-{}-{}-{}", archivo.consulta, archivo.pagina, fila.id);
            let identidad = vinculos::identidad(
                &fila.registro.texto("rit"),
                &fila.registro.texto("rut"),
                &fila.registro.texto("nombre"),
                &fila.registro.texto("programa"),
            );
            let candidatos = indice.get(&(archivo.consulta, identidad)).map(Vec::as_slice).unwrap_or(&[]);
            let remotos = candidatos
                .iter()
                .filter(|v| v.tribunal == seleccion.tribunal && v.bitacora_verificada)
                .map(|v| (&v.causa, &v.ingreso, &v.etapa))
                .collect::<BTreeSet<_>>();
            let historia_causas = candidatos
                .iter()
                .filter(|v| v.tribunal == seleccion.tribunal && v.historia_verificada)
                .map(|v| &v.causa)
                .collect::<BTreeSet<_>>();
            let remoto = if remotos.len() == 1 {
                candidatos.iter().find(|v| v.tribunal == seleccion.tribunal && v.bitacora_verificada).copied()
            } else {
                None
            };
            if let Some(v) = remoto {
                let snapshot = lecturas.get(&recoleccion::clave("bitacora", v));
                let eventos = snapshot.map(|l| crate::cronologia::desde_lectura(&l["datos"])).unwrap_or_default();
                contexto.cronologias.insert(
                    fila.id.clone(),
                    crate::cronologia::mostrar(&eventos, corte, resumen.lote.lecturas.bitacoras, snapshot.is_some()),
                );
                if let Some(lectura) = lecturas.get(&recoleccion::clave("bitacora", v)) {
                    let eventos = lectura["datos"]["entries"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|e| Some(Observacion { fecha: fecha(&e["fecha"])?, texto: e["texto"].as_str()?.to_string() }))
                        .filter(|o| o.fecha <= corte)
                        .collect::<Vec<_>>();
                    contexto.bitacoras.insert(fila.id.clone(), eventos);
                }
                if let Some(lectura) = lecturas.get(&recoleccion::clave("calendario", v)) {
                    let proxima = lectura["datos"]["proyecciones"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|p| p["estado"] == "PENDIENTE")
                        .filter_map(|p| fecha(&p["fecha_proyeccion"]))
                        .filter(|f| *f > corte && (*f - corte).num_days() <= 60)
                        .min();
                    if let Some(proxima) = proxima {
                        contexto.proximos.get_or_insert_with(HashMap::new).insert(fila.id.clone(), proxima);
                    }
                }
            } else if resumen.lote.lecturas.bitacoras || resumen.lote.lecturas.calendario {
                avisos.push(format!(
                    "Consulta {}, fila {}: contexto por ingreso no evaluable; falta un vínculo único.",
                    archivo.consulta + 1,
                    fila.numero
                ));
            }
            if historia_causas.len() == 1 {
                if let Some(v) = candidatos.iter().find(|v| v.tribunal == seleccion.tribunal && v.historia_verificada) {
                    if let Some(lectura) = lecturas.get(&recoleccion::clave("historia", v)) {
                        contexto.historias.insert(fila.id.clone(), historia(&lectura["datos"], corte));
                    }
                }
            }
            // La modalidad del listado sigue disponible aunque el Excel no traiga una columna con ese dato.
            if !fila.registro.tiene("modalidad") {
                fila.registro.poner("modalidad", Celda::Texto(seleccion.modality.clone()));
            }
        }
        if let Some((actual, huellas)) = tablas.iter_mut().find(|(t, _)| t.etapa == tabla.etapa && t.cabeceras == tabla.cabeceras) {
            actual.filas.extend(tabla.filas);
            actual.avisos.extend(tabla.avisos);
            actual.cruce.extend(tabla.cruce);
            huellas.push(libro.sha256);
        } else {
            tablas.push((tabla, vec![libro.sha256]));
        }
    }
    avisos.sort();
    avisos.dedup();
    let nombre = format!("Lote {}", carpeta.file_name().unwrap_or_default().to_string_lossy());
    Ok(tablas
        .into_iter()
        .map(|(mut tabla, huellas)| {
            tabla.avisos.extend(avisos.clone());
            analizar_filas(&nombre, &huella(huellas.join("|").as_bytes()), &tabla, corte, config, &contexto)
        })
        .collect())
}
