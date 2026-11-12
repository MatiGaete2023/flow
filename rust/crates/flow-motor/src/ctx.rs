//! Contexto de una fila, textos y composición de la observación
//! (`motor/contexto.py`, `motor/textos.py`, `motor/composicion.py`).

use chrono::NaiveDate;
use flow_dominio::{Historia, Observacion, Tramite};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// Textos, umbrales y reglas desactivadas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// «MODO.CLAVE» → plantilla.
    pub textos: BTreeMap<String, String>,
    pub umbrales: BTreeMap<String, i64>,
    #[serde(default)]
    pub desactivadas: BTreeSet<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            textos: serde_json::from_str(include_str!("../../../datos/textos.json")).expect("textos.json"),
            umbrales: serde_json::from_str(include_str!("../../../datos/umbrales.json")).expect("umbrales.json"),
            desactivadas: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Marca {
    /// 1 prioritaria, 2 revisar, 3 informativa.
    pub nivel: u8,
    pub codigo: String,
    pub texto: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incidencia {
    pub regla: String,
    pub motivo: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Duplicidad {
    MismoPrograma,
    OtroRegistro,
}

/// Fragmento de observación con su grupo: C constancia, A acción del Centro, T gestión del Tribunal, F cierre.
#[derive(Debug, Clone)]
pub struct Frag {
    pub texto: String,
    pub grupo: char,
    pub partes: Vec<(String, char)>,
    pub dato: &'static str,
}

impl Frag {
    pub fn vacio() -> Self {
        Frag { texto: String::new(), grupo: 'C', partes: Vec::new(), dato: "" }
    }
    pub fn simple(texto: String, grupo: char) -> Self {
        Frag { texto, grupo, partes: Vec::new(), dato: "" }
    }
    pub fn es_vacio(&self) -> bool {
        self.texto.is_empty()
    }
}

/// Grupo de cada oración de los textos que mezclan grupos; los demás son constancias.
fn estructura(clave: &str) -> &'static str {
    match clave {
        "ESPERA.E05_SOLO_CORREO" | "ESPERA.E05_PROYECTO_Y_CORREO" => "CA",
        "INFORMES.I01_VENCIDO_DCE" | "INFORMES.I01_VENCIDO_GENERAL" => "CA",
        "INFORMES.I02_POR_VENCER_DCE" | "INFORMES.I02_POR_VENCER_GENERAL" => "A",
        "COMUN.CURADOR" => "CT",
        "CUMPLIMIENTO.C07_SIN_FICHA" | "CUMPLIMIENTO.C07_FICHA_ANTIGUA" | "CUMPLIMIENTO.C08_FICHA_FAE" => "CT",
        "HISTORIA.C10_ESCRITO" => "CA",
        "HISTORIA.T5_INSTITUCIONES" => "T",
        "HISTORIA.R3_MULTI" => "F",
        _ => "C",
    }
}

/// Un dato se dice una sola vez: reglas que llegan a la misma información comparten etiqueta.
fn dato_de(clave: &str) -> &'static str {
    match clave {
        "CUMPLIMIENTO.C03_INGRESO_RECIENTE" | "HISTORIA.INGRESO_INFORMADO" => "ingreso_efectivo",
        "CUMPLIMIENTO.C04_VENCIDA"
        | "CUMPLIMIENTO.C05_VENCE_HOY"
        | "CUMPLIMIENTO.C05_POR_VENCER"
        | "HISTORIA.MEDIDA_SIN_VIGENCIA"
        | "HISTORIA.EGRESO_PROYECTADO" => "vencimiento",
        "HISTORIA.E05_DIAS" | "HISTORIA.E10_LISTA" => "dias_espera",
        _ => "",
    }
}

fn formatear(plantilla: &str, datos: &[(&str, String)]) -> String {
    let mut salida = plantilla.to_string();
    for (nombre, valor) in datos {
        salida = salida.replace(&format!("{{{nombre}}}"), valor);
    }
    salida
}

/// Divide en oraciones: corta en los espacios que siguen a un punto.
fn oraciones(texto: &str) -> Vec<String> {
    let mut salida = Vec::new();
    let mut actual = String::new();
    let mut previo = ' ';
    let mut en_corte = false;
    for c in texto.trim().chars() {
        if c.is_whitespace() && (previo == '.' || en_corte) {
            en_corte = true;
            continue;
        }
        if en_corte {
            salida.push(std::mem::take(&mut actual));
            en_corte = false;
        }
        actual.push(c);
        previo = c;
    }
    salida.push(actual);
    salida
}

pub struct Ctx<'a> {
    pub cfg: &'a Config,
    pub hoy: NaiveDate,
    pub eventos: Vec<String>,
    pub invalidas: Vec<&'static str>,
    pub historia: Option<Historia>,
    pub bitacora: Vec<Observacion>,
    pub marcas: Vec<Marca>,
    pub duplicidad: Option<Duplicidad>,
    pub acumulado: Option<(i64, NaiveDate)>,
    pub emitidos: HashSet<(String, NaiveDate, String)>,
    pub incidencias: Vec<Incidencia>,
}

impl<'a> Ctx<'a> {
    pub fn umbral(&self, nombre: &str) -> i64 {
        *self.cfg.umbrales.get(nombre).unwrap_or_else(|| panic!("umbral desconocido: {nombre}"))
    }

    pub fn desactivada(&self, clave: &str) -> bool {
        self.cfg.desactivadas.contains(clave)
    }

    pub fn marcar(&mut self, nivel: u8, codigo: &str, texto: &str) {
        self.marcas.push(Marca { nivel, codigo: codigo.into(), texto: texto.into() });
    }

    pub fn incidencia(&mut self, regla: &str, motivo: &str) {
        self.incidencias.push(Incidencia { regla: regla.into(), motivo: motivo.into() });
    }

    /// Historia utilizable (cobertura completa o parcial).
    pub fn h(&self) -> Option<&Historia> {
        self.historia.as_ref().filter(|h| h.con_datos())
    }

    pub fn tramites(&self, nombres: &[&str]) -> Vec<Tramite> {
        self.h().map(|h| h.buscar(nombres).into_iter().cloned().collect()).unwrap_or_default()
    }

    /// Texto de una regla. Vacío si está desactivada o invalidada para esta fila.
    pub fn render(&mut self, modo: &str, id: &str, datos: &[(&str, String)]) -> Frag {
        let clave = format!("{modo}.{id}");
        if self.desactivada(&clave) || self.invalidas.contains(&clave.as_str()) {
            return Frag::vacio();
        }
        let plantilla = self.cfg.textos.get(&clave).unwrap_or_else(|| panic!("texto desconocido: {clave}")).clone();
        let letras = estructura(&clave);
        self.eventos.push(clave.clone());
        let completo = formatear(&plantilla, datos);
        let mut frag = Frag::simple(completo, letras.chars().next().unwrap());
        if letras.len() > 1 {
            let partes = oraciones(&plantilla);
            if partes.len() == letras.len() {
                frag.partes = partes.iter().zip(letras.chars()).map(|(o, l)| (formatear(o, datos), l)).collect();
            }
        }
        let dato = dato_de(&clave);
        if !dato.is_empty() && !frag.es_vacio() {
            frag.partes.clear();
            frag.dato = dato;
        }
        frag
    }
}

fn orden_grupo(grupo: char) -> u8 {
    match grupo {
        'C' => 0,
        'A' => 1,
        'T' => 2,
        _ => 3,
    }
}

const PRESENTE: &str = "Se hace presente que ";

fn normalizar_fragmento(texto: &str) -> String {
    let t = texto.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() {
        return t;
    }
    let mut t = t.trim_end_matches(['.', ' ']).to_string() + ".";
    while t.contains("..") {
        t = t.replace("..", ".");
    }
    t
}

/// Ordena por grupo, dice cada dato una vez, enlaza las constancias y antepone el prefijo.
pub fn componer(prefijo: &str, fragmentos: &[Frag]) -> String {
    let mut piezas: Vec<(u8, usize, String)> = Vec::new();
    let mut dichos: Vec<&str> = Vec::new();
    for frag in fragmentos {
        if !frag.dato.is_empty() {
            if dichos.contains(&frag.dato) {
                continue;
            }
            dichos.push(frag.dato);
        }
        if frag.partes.is_empty() {
            piezas.push((orden_grupo(frag.grupo), piezas.len(), frag.texto.clone()));
        } else {
            for (texto, grupo) in &frag.partes {
                piezas.push((orden_grupo(*grupo), piezas.len(), texto.clone()));
            }
        }
    }
    piezas.sort_by_key(|p| (p.0, p.1));
    let mut vistas = 0;
    let mut partes = Vec::new();
    for (_, _, mut texto) in piezas {
        if texto.starts_with(PRESENTE) {
            vistas += 1;
            if vistas > 1 {
                texto = format!("Asimismo, se hace presente que {}", &texto[PRESENTE.len()..]);
            }
        }
        let limpio = normalizar_fragmento(&texto);
        if !limpio.is_empty() {
            partes.push(limpio);
        }
    }
    if partes.is_empty() {
        return String::new();
    }
    let texto = partes.join(" ").split_whitespace().collect::<Vec<_>>().join(" ").replace("..", ".");
    format!("{prefijo}{texto}")
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn corta_oraciones() {
        assert_eq!(oraciones("Uno.  Dos. Tres"), vec!["Uno.", "Dos.", "Tres"]);
        assert_eq!(oraciones("Sin punto final"), vec!["Sin punto final"]);
    }

    #[test]
    fn compone_en_orden_y_enlaza() {
        let f = vec![
            Frag::simple("Gestión del tribunal".into(), 'T'),
            Frag::simple("Se hace presente que uno.".into(), 'C'),
            Frag::simple("Se hace presente que dos".into(), 'C'),
        ];
        assert_eq!(
            componer("Ana PRM: ", &f),
            "Ana PRM: Se hace presente que uno. Asimismo, se hace presente que dos. Gestión del tribunal."
        );
    }
}
