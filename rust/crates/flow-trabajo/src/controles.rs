//! Avisos N-03/N-04, Q-03, OB-01..05 y CAL-03: contexto de las fuentes, sin nuevas acciones.
use crate::Trabajo;
use flow_dominio::{
    texto::{fecha_es, fecha_mayoria, normalizar_columna},
    Celda, Fecha,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Origen {
    pub sistema: String,
    pub tribunal: String,
    pub causa: String,
    pub ingreso: String,
    pub tipo: String,
    pub pantalla: String,
    pub verificada: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub valores: BTreeMap<String, Celda>,
}
impl Origen {
    pub fn clave(&self) -> Option<[String; 4]> {
        let key = [self.sistema.clone(), self.tribunal.clone(), self.causa.clone(), self.ingreso.clone()];
        (self.verificada && key.iter().all(|v| !v.is_empty())).then_some(key)
    }
    pub fn tradicional(&self) -> bool {
        matches!(self.tipo.as_str(), "ESPERA" | "CUMPLIMIENTO" | "INFORMES")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Proyeccion {
    pub fecha: Fecha,
    pub estado: String,
}
pub fn primera(items: &[Proyeccion]) -> Option<Fecha> {
    items.iter().filter(|p| p.estado == "PENDIENTE").map(|p| p.fecha).min()
}
pub fn proxima(items: &[Proyeccion], corte: Fecha) -> Option<Fecha> {
    items.iter().filter(|p| p.estado == "PENDIENTE" && p.fecha > corte).map(|p| p.fecha).min()
}
pub fn cotejar(items: &[Proyeccion], corte: Fecha, vencimiento: Option<Fecha>, hoja2: Option<Fecha>) -> Vec<String> {
    if items.is_empty() {
        return vec![];
    }
    let mut out = vec![];
    let pendiente = primera(items);
    if let Some(v) = vencimiento.filter(|v| Some(*v) != pendiente) {
        let otra = pendiente
            .map(|d| format!(" y el calendario del ingreso proyecta el informe pendiente el {}", fecha_es(d)))
            .unwrap_or(" y el calendario del ingreso no tiene informes pendientes".into());
        out.push(format!("CAL-03: la pestaña Informes indica vencimiento el {}{otra}; revisar en RUS.", fecha_es(v)));
    }
    let proxima = proxima(items, corte);
    if let Some(v) = hoja2.filter(|v| Some(*v) != proxima) {
        let otra = proxima
            .map(|d| format!(" y el calendario del ingreso proyecta el próximo informe el {}", fecha_es(d)))
            .unwrap_or(" y el calendario del ingreso no tiene un próximo informe pendiente".into());
        out.push(format!("CAL-03: Hoja2 (calendario 91) indica el {}{otra}; revisar en RUS.", fecha_es(v)));
    }
    out
}

fn clave(s: &str) -> String {
    normalizar_columna(s).to_uppercase().split_whitespace().collect::<Vec<_>>().join(" ")
}
fn vacia(c: &Celda) -> bool {
    matches!(c.texto().trim().to_lowercase().as_str(), "" | "-" | "--" | "---" | "nan" | "none")
}
pub fn orden(valores: &BTreeMap<String, Celda>, corte: Fecha) -> Vec<String> {
    let columnas = valores.iter().map(|(k, v)| (clave(k), v)).collect::<BTreeMap<_, _>>();
    let get = |k: &str| columnas.get(k).copied();
    if get("ESTADO O B").is_none_or(|c| clave(&c.texto()) != "ACTIVA") {
        return vec![];
    }
    let mut out = vec![];
    if let (Some(resultado), Some(tramite)) = (get("RESULTADO"), get("FECHA TRAMITE").and_then(Celda::fecha)) {
        let dias = (corte - tramite).num_days();
        if vacia(resultado) && dias > 60 {
            out.push(format!("OB-01: orden de búsqueda activa sin resultado desde el {} ({dias} días).", fecha_es(tramite)));
        }
    }
    if let Some(nac) = get("FEC NACIMIENTO").and_then(Celda::fecha) {
        let mayoria = fecha_mayoria(nac);
        if mayoria <= corte {
            out.push(format!("OB-02: orden de búsqueda activa y la persona cumplió 18 años el {}.", fecha_es(mayoria)));
        }
    }
    if let Some(envio) = get("ESTADO ENVIO").filter(|c| clave(&c.texto()) != "REALIZADA") {
        let estado = envio.texto();
        let estado = estado.trim();
        let estado = if estado.is_empty() { "sin estado" } else { estado };
        out.push(format!("OB-03: el envío de la orden no figura como realizado ({estado})."));
    }
    if let (Some(tipo), Some(curador)) = (get("TIPO LITIGANTE"), get("CURADOR")) {
        if matches!(clave(&tipo.texto()).as_str(), "NINO" | "NINA" | "ADOLESCENTE") && vacia(curador) {
            out.push("OB-04: orden de búsqueda activa de un niño, niña o adolescente sin curador ad litem registrado.".into());
        }
    }
    if let Some(fecha) = get("PROX AUDS").into_iter().flat_map(|c| c.fechas_varias().0).filter(|d| *d >= corte).min() {
        out.push(format!("OB-05: audiencia fijada para el {} con orden de búsqueda activa.", fecha_es(fecha)));
    }
    out
}

/// Inserta antes de la advertencia CT-07, que Python agrega al final de la importación.
pub fn agregar(avisos: &mut Vec<String>, nuevos: impl IntoIterator<Item = String>) {
    let mut posicion = avisos.iter().position(|a| a.starts_with("CT-07:")).unwrap_or(avisos.len());
    for aviso in nuevos {
        if !avisos.contains(&aviso) {
            avisos.insert(posicion, aviso);
            posicion += 1;
        }
    }
}
pub fn paquete(trabajos: &mut [Trabajo]) {
    let mut causas: BTreeMap<[String; 3], BTreeSet<String>> = BTreeMap::new();
    let mut tipos: BTreeMap<[String; 4], BTreeSet<String>> = BTreeMap::new();
    for f in trabajos.iter().flat_map(|t| &t.filas) {
        if let Some((o, k)) = f.origen.as_ref().and_then(|o| o.clave().map(|k| (o, k))) {
            causas.entry([k[0].clone(), k[1].clone(), k[2].clone()]).or_default().insert(k[3].clone());
            tipos.entry(k).or_default().insert(o.tipo.clone());
        }
    }
    for f in trabajos.iter_mut().flat_map(|t| &mut t.filas) {
        let Some((o, k)) = f.origen.as_ref().and_then(|o| o.clave().map(|k| (o, k))) else {
            continue;
        };
        let tipos = &tipos[&k];
        let mut avisos = vec![];
        let estados = [("ESPERA", "Espera"), ("CUMPLIMIENTO", "Cumplimiento"), ("EGRESADOS", "Egresados")]
            .into_iter()
            .filter(|(t, _)| tipos.contains(*t))
            .collect::<Vec<_>>();
        if estados.len() > 1 && matches!(o.tipo.as_str(), "ESPERA" | "CUMPLIMIENTO" | "EGRESADOS") {
            avisos.push(format!(
                "Q-03: el mismo ingreso figura en {} en este lote; revisar su estado en RUS.",
                estados.iter().map(|(_, n)| *n).collect::<Vec<_>>().join(" y ")
            ));
        }
        let n = causas[&[k[0].clone(), k[1].clone(), k[2].clone()]].len();
        if n > 1 && o.tradicional() {
            avisos.push(format!("N-03: la causa tiene {n} ingresos en este trabajo; considerar un solo correo o proyecto."));
        }
        if tipos.contains("EGRESADOS") && tipos.contains("INFORMES") && matches!(o.tipo.as_str(), "EGRESADOS" | "INFORMES") {
            avisos.push("N-04: medida egresada que sigue con informe pendiente en RUS; revisar.".into());
        }
        agregar(&mut f.propuesta.avisos, avisos);
        f.propuesta.avisos.sort_by_key(|a| {
            if a.starts_with("Q-03:") {
                1
            } else if a.starts_with("CAL-03:") {
                2
            } else if a.starts_with("OB-") {
                3
            } else if a.starts_with("N-03:") || a.starts_with("N-04:") {
                4
            } else if a.starts_with("CT-07:") {
                5
            } else {
                0
            }
        });
    }
}
