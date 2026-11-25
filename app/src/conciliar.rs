//! Conciliación entre capturas (`capture_reconciliation.reconcile` y `capture_changes.compare` de Flow Python): al abrir
//! un paquete nuevo con un trabajo de paquete abierto, las decisiones de la persona pasan a las filas que son la misma
//! por su identidad remota, sin asociar personas por nombre ni por RIT. Si los datos fuente cambiaron o la
//! correspondencia es ambigua, la persona decide fila por fila; nada se aplica hasta que todo está resuelto.

use crate::config::Config;
use crate::trabajo::{Registro, Trabajo};
use flow_rust::captura::Fila;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Respuesta {
    Conservar,
    Reiniciar,
}

/// Respuestas por identificador de conflicto: el `record_id` de la fila nueva, o `missing:` + el de la anterior.
pub type Respuestas = BTreeMap<String, Respuesta>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflicto {
    pub id: String,
    pub previos: Vec<String>,
    pub motivo: String,
    pub puede_conservar: bool,
    pub campos: Vec<String>,
    pub antes: BTreeMap<String, Value>,
    pub despues: BTreeMap<String, Value>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Conflictos(Vec<Conflicto>),
    /// Respuesta que no corresponde a ningún conflicto; el trabajo anterior queda intacto.
    Respuesta(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Nuevo,
    Fuente,
    Reglas,
    SinCambio,
}

impl Tipo {
    pub fn etiqueta(self) -> &'static str {
        match self {
            Tipo::Nuevo => "Nueva",
            Tipo::Fuente => "Fuente",
            Tipo::Reglas => "Reglas",
            Tipo::SinCambio => "",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CambioFila {
    pub tipo: Tipo,
    pub campos: Vec<String>,
    pub reglas_antes: Vec<String>,
    pub reglas_despues: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Egresado {
    pub id_anterior: String,
    pub rit: String,
    pub nombre: String,
    pub tribunal: String,
    pub programa: String,
}

/// Comparación anónima entre dos capturas de un trabajo (`capture_changes`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cambios {
    pub filas: BTreeMap<String, CambioFila>,
    pub egresados: Vec<Egresado>,
}

impl Cambios {
    pub fn resumen(&self) -> BTreeMap<&'static str, usize> {
        let mut m = BTreeMap::from([("Nuevas", 0), ("Fuente", 0), ("Reglas", 0), ("Sin cambio", 0)]);
        for c in self.filas.values() {
            *m.get_mut(match c.tipo {
                Tipo::Nuevo => "Nuevas",
                Tipo::Fuente => "Fuente",
                Tipo::Reglas => "Reglas",
                Tipo::SinCambio => "Sin cambio",
            })
            .unwrap() += 1;
        }
        m.insert("Egresados", self.egresados.len());
        m
    }
}

#[derive(Debug, Default)]
pub struct Resultado {
    pub continuada: bool,
    /// Fila nueva → fila anterior (por `record_id`).
    pub emparejadas: BTreeMap<String, String>,
}

fn normalizado(f: &Fila<'_>) -> BTreeMap<String, Value> {
    serde_json::from_str::<serde_json::Map<String, Value>>(f.get("normalized_json")).map(|m| m.into_iter().collect()).unwrap_or_default()
}

fn identidad(f: &Fila<'_>) -> [String; 4] {
    ["source_system", "court_code", "cause_id", "ingreso_id"].map(|k| f.get(k).to_string())
}

/// `_key`: la identidad remota con su alcance o, sin ella, la fila física íntegra.
fn clave(f: &Fila<'_>) -> Vec<String> {
    let id = identidad(f);
    let alcance = ["record_type", "modality_code", "tab", "person_id", "program_id"].map(|k| f.get(k).to_string());
    if f.get("identity_status") == "VERIFICADA" && id.iter().all(|s| !s.is_empty()) {
        return ["REMOTE".to_string()].into_iter().chain(id).chain(alcance).collect();
    }
    ["PHYSICAL".to_string()]
        .into_iter()
        .chain(id)
        .chain(alcance)
        .chain([f.get("source_sha256"), f.get("original_sheet"), f.get("original_row")].map(str::to_string))
        .chain([serde_json::to_string(&normalizado(f)).unwrap_or_default()])
        .collect()
}

/// `_edited`: la persona tomó alguna decisión sobre la fila.
pub(crate) fn editada(r: &Registro) -> bool {
    r.editada.is_some() || r.revisada || !r.res.is_empty() || !r.campos.is_empty() || !r.gestiones_correo.is_empty()
}

/// Filas del trabajo con su registro de paquete.
struct Fuentes<'a> {
    filas: Vec<(&'a Registro, String, Fila<'a>)>,
}

fn fuentes(w: &Trabajo) -> Result<Fuentes<'_>, String> {
    let paquete = w.paquete.as_ref().ok_or("El trabajo no viene de un paquete.")?;
    let registros: HashMap<&str, Fila<'_>> = paquete.fuente.filas("registros").map(|r| (r.get("record_id"), r)).collect();
    let mut filas = Vec::with_capacity(w.registros.len());
    for r in &w.registros {
        let id = w.id_registro(r).ok_or("Una fila del trabajo no conserva su identificador de captura.")?;
        let fuente = *registros.get(id.as_str()).ok_or("Una fila del trabajo no está en su paquete.")?;
        filas.push((r, id, fuente));
    }
    Ok(Fuentes { filas })
}

/// Combina el trabajo nuevo (`nuevo`, recién abierto) con el anterior (`previo`). Todo o nada: si hay conflictos sin
/// respuesta, `nuevo` no cambia.
pub fn conciliar(nuevo: &mut Trabajo, previo: &Trabajo, respuestas: &Respuestas, cfg: &Config) -> Result<Resultado, Error> {
    let raro = |m: String| Error::Respuesta(m);
    if previo.paquete.is_none() || nuevo.paquete.is_none() {
        return Ok(Resultado::default());
    }
    let (viejas, nuevas) = (fuentes(previo).map_err(raro)?, fuentes(nuevo).map_err(raro)?);
    let mismos_bytes = previo.hash == nuevo.hash;
    let por_id_viejo: HashMap<&str, usize> = viejas.filas.iter().enumerate().map(|(i, (_, id, _))| (id.as_str(), i)).collect();
    let mut viejas_por_clave: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (i, (_, _, f)) in viejas.filas.iter().enumerate() {
        viejas_por_clave.entry(clave(f)).or_default().push(i);
    }
    let mut nuevas_por_clave: HashMap<Vec<String>, usize> = HashMap::new();
    for (_, _, f) in &nuevas.filas {
        *nuevas_por_clave.entry(clave(f)).or_default() += 1;
    }
    let (mut emparejadas, mut conflictos): (BTreeMap<usize, usize>, Vec<Conflicto>) = (BTreeMap::new(), Vec::new());
    let (mut consumidas, mut resueltas): (BTreeSet<usize>, BTreeSet<String>) = (BTreeSet::new(), BTreeSet::new());
    let mut campos_distintos: HashMap<usize, Vec<String>> = HashMap::new();
    for (n, (_, id_nuevo, f_nuevo)) in nuevas.filas.iter().enumerate() {
        let k = clave(f_nuevo);
        let previos: Vec<usize> = match por_id_viejo.get(id_nuevo.as_str()) {
            Some(&i) if mismos_bytes => vec![i],
            _ => viejas_por_clave.get(&k).cloned().unwrap_or_default(),
        };
        if previos.is_empty() {
            continue;
        }
        let unica = previos.len() == 1 && (mismos_bytes || nuevas_por_clave.get(&k) == Some(&1));
        let (mut cambiados, mut antes, mut despues) = (Vec::new(), BTreeMap::new(), BTreeMap::new());
        if unica {
            let (v, nn) = (normalizado(&viejas.filas[previos[0]].2), normalizado(f_nuevo));
            let claves: BTreeSet<&String> = v.keys().chain(nn.keys()).collect();
            for c in claves {
                if v.get(c) != nn.get(c) {
                    cambiados.push(c.clone());
                    antes.insert(c.clone(), v.get(c).cloned().unwrap_or(Value::Null));
                    despues.insert(c.clone(), nn.get(c).cloned().unwrap_or(Value::Null));
                }
            }
        }
        campos_distintos.insert(n, cambiados.clone());
        let con_ediciones = previos.iter().any(|&i| editada(viejas.filas[i].0));
        if (!unica || !cambiados.is_empty()) && con_ediciones {
            match respuestas.get(id_nuevo) {
                Some(Respuesta::Reiniciar) => {
                    consumidas.extend(&previos);
                    resueltas.insert(id_nuevo.clone());
                    continue;
                }
                Some(Respuesta::Conservar) if unica => {
                    resueltas.insert(id_nuevo.clone());
                }
                _ => {
                    conflictos.push(Conflicto {
                        id: id_nuevo.clone(),
                        previos: previos.iter().map(|&i| viejas.filas[i].1.clone()).collect(),
                        motivo: if unica { "Datos fuente modificados." } else { "Correspondencia ambigua." }.into(),
                        puede_conservar: unica,
                        campos: cambiados.clone(),
                        antes: if unica { antes } else { BTreeMap::new() },
                        despues: if unica { despues } else { BTreeMap::new() },
                    });
                    consumidas.extend(&previos);
                    continue;
                }
            }
        }
        if unica && (cambiados.is_empty() || respuestas.get(id_nuevo) == Some(&Respuesta::Conservar)) {
            emparejadas.insert(n, previos[0]);
        }
        consumidas.extend(&previos);
    }
    for (i, (r, id, _)) in viejas.filas.iter().enumerate() {
        if !consumidas.contains(&i) && editada(r) {
            let conflicto = format!("missing:{id}");
            if respuestas.get(&conflicto) == Some(&Respuesta::Reiniciar) {
                resueltas.insert(conflicto);
            } else {
                conflictos.push(Conflicto {
                    id: conflicto,
                    previos: vec![id.clone()],
                    motivo: "Registro anterior no presente.".into(),
                    puede_conservar: false,
                    campos: Vec::new(),
                    antes: BTreeMap::new(),
                    despues: BTreeMap::new(),
                });
            }
        }
    }
    if respuestas.keys().any(|k| !resueltas.contains(k)) {
        return Err(Error::Respuesta("Respuesta de conciliación ajena o innecesaria; trabajo anterior intacto.".into()));
    }
    if !conflictos.is_empty() {
        return Err(Error::Conflictos(conflictos));
    }
    // Lo anterior ya está validado: desde aquí se modifica el trabajo nuevo.
    let ids_nuevos: Vec<String> = nuevas.filas.iter().map(|(_, id, _)| id.clone()).collect();
    let ids_viejos: Vec<String> = viejas.filas.iter().map(|(_, id, _)| id.clone()).collect();
    let mapa: BTreeMap<usize, usize> = emparejadas;
    let continuada = mismos_bytes || !mapa.is_empty();
    // Consignaciones de las filas que continúan, con el identificador nuevo.
    if continuada {
        for (&n, &i) in &mapa {
            if let Some(lista) = previo.consignaciones.get(&ids_viejos[i]) {
                nuevo.consignaciones.insert(ids_nuevos[n].clone(), lista.clone());
            }
        }
        if !nuevo.consignaciones.is_empty() {
            nuevo.analizar(cfg);
        }
    }
    for (&n, &i) in &mapa {
        let anterior = &previo.registros[i];
        let r = &mut nuevo.registros[n];
        r.editada = anterior.editada.clone();
        r.revisada = anterior.revisada;
        r.seleccionada = anterior.seleccionada;
        r.res = anterior.res.clone();
        r.campos = anterior.campos.clone();
        r.gestiones_correo = anterior.gestiones_correo.clone();
        if mismos_bytes {
            r.sync_base = anterior.sync_base.clone();
            r.sync_local = anterior.sync_local.clone();
            r.id_excel = anterior.id_excel.clone();
            r.sync_correo = anterior.sync_correo.clone();
        }
    }
    if mismos_bytes {
        nuevo.copia = previo.copia.clone();
    }
    // Correos y proyectos preparados con la captura anterior siguen disponibles, pero marcados para revisar: dependen de
    // registros que pueden haber cambiado (`REVISAR_CAPTURA_ANTERIOR`).
    if continuada {
        let a_nuevo: BTreeMap<usize, usize> = mapa.iter().map(|(&n, &i)| (i, n)).collect();
        let remapear = |registros: &[usize]| -> Option<Vec<usize>> { registros.iter().map(|i| a_nuevo.get(i).copied()).collect() };
        let sello = |dep: &str| if mismos_bytes { dep.to_string() } else { "REVISAR_CAPTURA_ANTERIOR".to_string() };
        for b in &previo.preparados.borradores {
            if let Some(registros) = remapear(&b.registros) {
                let mut b = b.clone();
                b.registros = registros;
                b.dependencia = sello(&b.dependencia);
                nuevo.preparados.borradores.push(b);
            }
        }
        for p in &previo.preparados.proyectos {
            if let Some(registros) = remapear(&p.registros) {
                let mut p = p.clone();
                p.registros = registros;
                p.dependencia = sello(&p.dependencia);
                nuevo.preparados.proyectos.push(p);
            }
        }
        let renombre: BTreeMap<&String, &String> = mapa.iter().map(|(&n, &i)| (&ids_viejos[i], &ids_nuevos[n])).collect();
        for (clave, r) in &previo.preparados.recibos {
            let mut r = r.clone();
            r.registros = r.registros.iter().map(|id| renombre.get(id).map_or(id.clone(), |n| (*n).clone())).collect();
            nuevo.preparados.recibos.insert(clave.clone(), r);
        }
    }
    // `capture_changes.compare`.
    let mut cambios = Cambios::default();
    for (n, id) in ids_nuevos.iter().enumerate() {
        let r = &nuevo.registros[n];
        let cambio = match mapa.get(&n) {
            None => CambioFila { tipo: Tipo::Nuevo, campos: Vec::new(), reglas_antes: Vec::new(), reglas_despues: Vec::new() },
            Some(&i) => {
                let anterior = &previo.registros[i];
                let reglas = anterior.eventos != r.eventos || anterior.propuesta != r.propuesta;
                let cambiados = campos_distintos.get(&n).cloned().unwrap_or_default();
                let tipo = if !cambiados.is_empty() {
                    Tipo::Fuente
                } else if reglas {
                    Tipo::Reglas
                } else {
                    Tipo::SinCambio
                };
                CambioFila {
                    tipo,
                    campos: if tipo == Tipo::Fuente { cambiados } else { Vec::new() },
                    reglas_antes: if reglas { anterior.eventos.clone() } else { Vec::new() },
                    reglas_despues: if reglas { r.eventos.clone() } else { Vec::new() },
                }
            }
        };
        cambios.filas.insert(id.clone(), cambio);
    }
    let emparejadas_viejas: BTreeSet<usize> = mapa.values().copied().collect();
    for (i, id) in ids_viejos.iter().enumerate() {
        if !emparejadas_viejas.contains(&i) {
            let r = &previo.registros[i];
            cambios.egresados.push(Egresado {
                id_anterior: id.clone(),
                rit: previo.value(r, "rit"),
                nombre: previo.value(r, "nombre"),
                tribunal: previo.value(r, "tribunal"),
                programa: previo.value(r, "programa"),
            });
        }
    }
    nuevo.cambios = Some(cambios);
    Ok(Resultado {
        continuada,
        emparejadas: mapa.iter().map(|(&n, &i)| (ids_nuevos[n].clone(), ids_viejos[i].clone())).collect(),
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::motor::Mode;
    use base64::Engine;
    use chrono::NaiveDate;
    use flow_rust::descargador::paquete::{self, Opciones};
    use std::path::{Path, PathBuf};

    fn reconstruir(destino: &Path, archivos: &Value) {
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    /// Mismo nombre de fila que usa `generar_conciliacion.py`.
    fn etiqueta(w: &Trabajo, i: usize) -> String {
        let id = w.id_registro(&w.registros[i]).unwrap();
        let f = w.paquete.as_ref().unwrap().fuente.filas("registros").find(|r| r.get("record_id") == id).unwrap();
        if f.get("identity_status") == "VERIFICADA" && ["court_code", "cause_id", "ingreso_id"].iter().all(|k| !f.get(k).is_empty()) {
            format!("{}-{}-{}|{}|{}", f.get("court_code"), f.get("cause_id"), f.get("ingreso_id"), f.get("record_type"), f.get("modality_code"))
        } else {
            format!("fisica|{}#{}|{}", f.get("original_file"), f.get("original_row"), f.get("record_type"))
        }
    }

    fn texto(v: &Value) -> String {
        v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())
    }

    struct Banco {
        zips: BTreeMap<String, PathBuf>,
        corte: NaiveDate,
    }

    fn banco(datos: &Value, raiz: &Path) -> Banco {
        let paquetes: Value = serde_json::from_str(include_str!("../tests/datos/paquete.json")).unwrap();
        let resultados: Value = serde_json::from_str(include_str!("../tests/datos/resultados.json")).unwrap();
        let escenario = |n: &str| paquetes["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == n).unwrap()["lote"].clone();
        let lotes = [
            ("listados", escenario("listados")),
            ("listados_dos_pestanas", escenario("listados_dos_pestanas")),
            ("modificado", resultados["modificado"].clone()),
            ("programa_cambiado", datos["lote_programa_cambiado"].clone()),
        ];
        let mut zips = BTreeMap::new();
        for (nombre, lote) in lotes {
            let carpeta = raiz.join(nombre).join(lote["carpeta"].as_str().unwrap());
            reconstruir(&carpeta, &lote["archivos"]);
            let zip = raiz.join(nombre).join("captura.zip");
            paquete::exportar_con(&[carpeta], &zip, &Opciones::default()).unwrap();
            zips.insert(nombre.to_string(), zip);
        }
        Banco { zips, corte: NaiveDate::parse_from_str(datos["corte"].as_str().unwrap(), "%Y-%m-%d").unwrap() }
    }

    impl Banco {
        fn abrir(&self, nombre: &str) -> Trabajo {
            Trabajo::abrir_archivo(self.zips[nombre].clone(), Some(Mode::Cumplimiento), self.corte, &Config::default()).unwrap()
        }
    }

    fn tipo(t: Tipo) -> &'static str {
        match t {
            Tipo::Nuevo => "NUEVO",
            Tipo::Fuente => "CAMBIO_FUENTE",
            Tipo::Reglas => "CAMBIO_REGLAS",
            Tipo::SinCambio => "SIN_CAMBIO",
        }
    }

    /// Repite el escenario de Python con Trabajo y devuelve las diferencias con lo que Python grabó.
    fn comparar(banco: &Banco, caso: &Value) -> Vec<String> {
        let mut previo = banco.abrir(caso["previo"].as_str().unwrap());
        let mut nuevo = banco.abrir(caso["nuevo"].as_str().unwrap());
        let indice = |w: &Trabajo, etq: &str| (0..w.registros.len()).find(|&i| etiqueta(w, i) == etq).unwrap_or_else(|| panic!("sin fila {etq}"));
        for e in caso["ediciones"].as_array().unwrap() {
            let i = indice(&previo, e[0].as_str().unwrap());
            match e[1].as_str().unwrap() {
                "OBSERVACION" => previo.registros[i].editada = Some(texto(&e[2])),
                _ => previo.registros[i].res = texto(&e[2]),
            }
        }
        let mut respuestas = Respuestas::new();
        for (clave, r) in caso["respuestas"].as_object().unwrap() {
            let respuesta = if r == "KEEP" { Respuesta::Conservar } else { Respuesta::Reiniciar };
            let id = match clave.strip_prefix("missing:") {
                Some(etq) => format!("missing:{}", previo.id_registro(&previo.registros[indice(&previo, etq)]).unwrap()),
                None => match (0..nuevo.registros.len()).find(|&i| etiqueta(&nuevo, i) == *clave) {
                    Some(i) => nuevo.id_registro(&nuevo.registros[i]).unwrap(),
                    None => clave.clone(),
                },
            };
            respuestas.insert(id, respuesta);
        }
        let resultado = conciliar(&mut nuevo, &previo, &respuestas, &Config::default());
        let mut d = Vec::new();
        let etiqueta_previa = |id: &str| {
            let i = previo.registros.iter().position(|r| previo.id_registro(r).as_deref() == Some(id)).unwrap();
            etiqueta(&previo, i)
        };
        match (caso["resultado"].as_str().unwrap(), &resultado) {
            ("error", Err(Error::Respuesta(m))) if *m == texto(&caso["error"]) => {}
            ("conflictos", Err(Error::Conflictos(c))) => {
                let mut obtenidos: Vec<Value> = c
                    .iter()
                    .map(|x| {
                        let id = match x.id.strip_prefix("missing:") {
                            Some(p) => format!("missing:{}", etiqueta_previa(p)),
                            None => etiqueta(&nuevo, (0..nuevo.registros.len()).find(|&i| nuevo.id_registro(&nuevo.registros[i]).as_deref() == Some(&x.id)).unwrap()),
                        };
                        let mut campos = x.campos.clone();
                        campos.sort();
                        serde_json::json!([id, x.motivo, x.puede_conservar, campos])
                    })
                    .collect();
                obtenidos.sort_by_key(Value::to_string);
                let mut previstos: Vec<Value> = caso["conflictos"].as_array().unwrap().clone();
                previstos.sort_by_key(Value::to_string);
                if obtenidos != previstos {
                    d.push(format!("conflictos Rust {obtenidos:?} · Python {previstos:?}"));
                }
            }
            ("ok", Ok(r)) => {
                if r.continuada != caso["continuada"].as_bool().unwrap() {
                    d.push(format!("continuada Rust {} · Python {}", r.continuada, caso["continuada"]));
                }
                let por_id = |id: &str| etiqueta(&nuevo, (0..nuevo.registros.len()).find(|&i| nuevo.id_registro(&nuevo.registros[i]).as_deref() == Some(id)).unwrap());
                let mut emparejadas: Vec<String> = r.emparejadas.keys().map(|k| por_id(k)).collect();
                emparejadas.sort();
                let previstas: Vec<String> = caso["emparejadas"].as_array().unwrap().iter().map(texto).collect();
                if emparejadas != previstas {
                    d.push(format!("emparejadas Rust {emparejadas:?} · Python {previstas:?}"));
                }
                for i in 0..nuevo.registros.len() {
                    let etq = etiqueta(&nuevo, i);
                    let rev = &caso["revision"][&etq];
                    let (obs, res) = (nuevo.registros[i].editada.clone().unwrap_or_default(), nuevo.registros[i].res.clone());
                    if (obs.as_str(), res.as_str()) != (texto(&rev["OBSERVACION"]).as_str(), texto(&rev["RES"]).as_str()) {
                        d.push(format!("{etq}: revisión Rust ({obs:?}, {res:?}) · Python {rev}"));
                    }
                    let cambio = &nuevo.cambios.as_ref().unwrap().filas[&nuevo.id_registro(&nuevo.registros[i]).unwrap()];
                    if tipo(cambio.tipo) != texto(&caso["cambios"][&etq]) {
                        d.push(format!("{etq}: cambio Rust {} · Python {}", tipo(cambio.tipo), caso["cambios"][&etq]));
                    }
                    let previstos: Vec<String> = caso["campos"].get(&etq).and_then(Value::as_array).map(|a| a.iter().map(texto).collect()).unwrap_or_default();
                    if cambio.campos != previstos {
                        d.push(format!("{etq}: campos Rust {:?} · Python {previstos:?}", cambio.campos));
                    }
                }
                // Trabajo solo ve la etapa abierta: se comparan los egresados de esa etapa.
                let mut egresados: Vec<String> = nuevo.cambios.as_ref().unwrap().egresados.iter().map(|e| etiqueta_previa(&e.id_anterior)).collect();
                egresados.sort();
                let previstos: Vec<String> = caso["egresados"].as_array().unwrap().iter().map(texto).filter(|e| e.contains("|CUMPLIMIENTO")).collect();
                if egresados != previstos {
                    d.push(format!("egresados Rust {egresados:?} · Python {previstos:?}"));
                }
            }
            (esperado, otro) => d.push(format!("resultado Rust {otro:?} · Python {esperado}")),
        }
        d
    }

    /// E-A2: la conciliación de Rust contra `capture_reconciliation.reconcile` de Flow Python
    /// (`app/qa/generar_conciliacion.py`): filas conservadas, conflictos, respuestas y comparación entre capturas.
    #[test]
    fn conciliacion_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/conciliacion.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let raiz = perfil.join("conciliacion");
        let _ = std::fs::remove_dir_all(&raiz);
        let banco = banco(&datos, &raiz);
        let casos = datos["casos"].as_array().unwrap();
        let (mut iguales, mut fallos) = (0, Vec::new());
        for caso in casos {
            let d = comparar(&banco, caso);
            if d.is_empty() {
                iguales += 1;
            } else {
                fallos.push(format!("{}:\n  {}", caso["escenario"], d.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("CONCILIACION: {iguales}/{}, fallos {}", casos.len(), fallos.len());
        let _ = std::fs::remove_dir_all(&raiz);
        assert_eq!(casos.len(), 12, "el oráculo debe traer 12 casos");
        assert!(fallos.is_empty());
    }

    /// Prueba permanente del oráculo: con otra revisión esperada, la comparación debe fallar.
    #[test]
    fn oraculo_de_conciliacion_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/conciliacion.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let raiz = perfil.join("conciliacion-mutacion");
        let _ = std::fs::remove_dir_all(&raiz);
        let banco = banco(&datos, &raiz);
        let mut caso = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "conflicto_conservar").unwrap().clone();
        assert!(comparar(&banco, &caso).is_empty());
        let etq = caso["revision"].as_object().unwrap().keys().next().unwrap().clone();
        caso["revision"][&etq]["OBSERVACION"] = Value::String("otra".into());
        assert!(!comparar(&banco, &caso).is_empty(), "una revisión esperada alterada no se detecta");
        let _ = std::fs::remove_dir_all(&raiz);
    }
}
