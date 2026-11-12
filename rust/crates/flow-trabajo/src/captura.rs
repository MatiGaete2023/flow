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
    let mut contexto = Contexto { proximos: Some(HashMap::new()), ..Default::default() };
    let mut avisos = resumen.avisos.clone();
    let mut libros = HashMap::new();
    for archivo in &resumen.archivos {
        let ruta = ruta_archivo(carpeta, &archivo.nombre)?;
        if std::fs::metadata(&ruta).map_err(|_| "Falta una descarga del lote.")?.len() > 80 * 1024 * 1024 {
            return Err("Una descarga excede el tamaño permitido.".into());
        }
        let datos = std::fs::read(ruta).map_err(|_| "No se pudo leer una descarga del lote.")?;
        if huella(&datos) != archivo.sha256 {
            return Err("Cambió una descarga desde su verificación; no se importó el lote.".into());
        }
        if archivo.nombre.ends_with(".xls") {
            libros.insert(archivo.nombre.clone(), flow_tabular::leer_bytes(&archivo.nombre, datos)?);
        }
    }
    let mut calendario91: HashMap<(String, [String; 4]), BTreeSet<Fecha>> = HashMap::new();
    for archivo in &resumen.archivos {
        let seleccion = resumen.lote.consultas.get(archivo.consulta).ok_or("La descarga no identifica su consulta.")?;
        if seleccion.screen != "calendario_informes" {
            continue;
        }
        let Some(libro) = libros.get(&archivo.nombre) else {
            continue;
        };
        match flow_tabular::leer_tabla(libro, Some(Etapa::Informes), None) {
            Ok(tabla) => {
                for f in &tabla.filas {
                    if let Some(d) = f.registro.fecha("vencimiento").filter(|d| *d > corte && (*d - corte).num_days() <= 60) {
                        let key = vinculos::identidad(
                            &f.registro.texto("rit"),
                            &f.registro.texto("rut"),
                            &f.registro.texto("nombre"),
                            &f.registro.texto("programa"),
                        );
                        calendario91.entry((seleccion.tribunal.clone(), key)).or_default().insert(d);
                    }
                }
            }
            Err(_) => avisos.push(format!(
                "Consulta {}: calendario 91 sin columnas suficientes para el cruce; conserva el Excel para revisión.",
                archivo.consulta + 1
            )),
        }
    }
    for archivo in &resumen.archivos {
        let seleccion = resumen.lote.consultas.get(archivo.consulta).ok_or("La descarga no identifica su consulta.")?;
        let Some(libro) = libros.get(&archivo.nombre) else {
            continue;
        };
        let (mut tabla, tipo) = if seleccion.screen == "litigantes" {
            (flow_tabular::leer_consulta(libro, "BUSQUEDA_LITIGANTES")?, "OTRO".to_string())
        } else if seleccion.tab == "Egresados" {
            (flow_tabular::leer_consulta(libro, "EGRESADOS")?, "EGRESADOS".to_string())
        } else if let Some(etapa) = Etapa::desde(&seleccion.tab) {
            (flow_tabular::leer_tabla(libro, Some(etapa), None)?, etapa.nombre().to_string())
        } else {
            continue;
        };
        let cruces = tabla
            .filas
            .iter()
            .filter_map(|f| tabla.proximo_informe(&f.registro, corte).map(|d| (f.id.clone(), d)))
            .collect::<HashMap<_, _>>();
        for fila in &mut tabla.filas {
            let mut hoja2 = cruces.get(&fila.id).copied();
            fila.id = format!("captura-{}-{}-{}", archivo.consulta, archivo.pagina, fila.id);
            if let Some(d) = hoja2 {
                contexto.proximos.as_mut().unwrap().insert(fila.id.clone(), d);
            }
            let mut origen = crate::controles::Origen {
                sistema: "RUS".into(),
                tribunal: seleccion.tribunal.clone(),
                causa: String::new(),
                ingreso: String::new(),
                tipo: tipo.clone(),
                pantalla: seleccion.screen.clone(),
                verificada: false,
                valores: Default::default(),
            };
            if !origen.tradicional() {
                origen.valores = tabla.cabeceras.iter().cloned().zip(fila.celdas.iter().cloned()).collect();
            }

            let identidad = vinculos::identidad(
                &fila.registro.texto("rit"),
                &fila.registro.texto("rut"),
                &fila.registro.texto("nombre"),
                &fila.registro.texto("programa"),
            );
            if let Some(fechas) = calendario91.get(&(seleccion.tribunal.clone(), identidad.clone())).filter(|fs| fs.len() == 1) {
                hoja2 = fechas.first().copied();
            }
            if let Some(d) = hoja2 {
                contexto.proximos.as_mut().unwrap().insert(fila.id.clone(), d);
            }
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
                origen.causa = v.causa.clone();
                origen.ingreso = v.ingreso.clone();
                origen.verificada = true;
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
                    let mut items = lectura["datos"]["proyecciones"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|p| {
                            Some(crate::controles::Proyeccion {
                                fecha: fecha(&p["fecha_proyeccion"])?,
                                estado: p["estado"].as_str()?.into(),
                            })
                        })
                        .collect::<Vec<_>>();
                    items.sort();
                    items.dedup();
                    if matches!(tabla.etapa, Etapa::Cumplimiento | Etapa::Informes) {
                        let vencimiento = if tabla.etapa == Etapa::Informes { fila.registro.fecha("vencimiento") } else { None };
                        contexto.avisos.insert(fila.id.clone(), crate::controles::cotejar(&items, corte, vencimiento, hoja2));
                        if hoja2.is_none() {
                            if let Some(d) = crate::controles::proxima(&items, corte) {
                                contexto.proximos.as_mut().unwrap().insert(fila.id.clone(), d);
                            }
                        }
                    }
                    contexto.calendarios.insert(fila.id.clone(), items);
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
            contexto.origenes.insert(fila.id.clone(), origen);
            // La modalidad del listado sigue disponible aunque el Excel no traiga una columna con ese dato.
            if !fila.registro.tiene("modalidad") {
                fila.registro.poner("modalidad", Celda::Texto(seleccion.modality.clone()));
            }
        }
        if let Some((actual, huellas)) =
            tablas.iter_mut().find(|(t, _)| t.etapa == tabla.etapa && t.hoja == tabla.hoja && t.cabeceras == tabla.cabeceras)
        {
            actual.filas.extend(tabla.filas);
            actual.avisos.extend(tabla.avisos);
            actual.cruce.extend(tabla.cruce);
            huellas.push(libro.sha256.clone());
        } else {
            tablas.push((tabla, vec![libro.sha256.clone()]));
        }
    }
    avisos.sort();
    avisos.dedup();
    let nombre = format!("Lote {}", carpeta.file_name().unwrap_or_default().to_string_lossy());
    let mut trabajos = tablas
        .into_iter()
        .map(|(mut tabla, huellas)| {
            tabla.avisos.extend(avisos.clone());
            analizar_filas(&nombre, &huella(huellas.join("|").as_bytes()), &tabla, corte, config, &contexto)
        })
        .collect::<Vec<_>>();
    crate::controles::paquete(&mut trabajos);
    Ok(trabajos)
}
