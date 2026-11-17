//! Borradores de correo: a programas (con nómina) y a tribunales. Nunca se envían.
//! El CC fijo va en todos y no se puede quitar.

use crate::excel::{nomina, ruta_nueva, valor};
use flow_dominio::texto::normalizar_columna;
use flow_dominio::{Etapa, Tribunal, CC_FIJO};
use flow_trabajo::{FilaTrabajo, Trabajo};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

#[derive(Debug, Clone, Deserialize)]
struct Plantilla {
    nombre: String,
    asunto: String,
    cuerpo: String,
    adjunto: String,
    usa_modalidades: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct Base {
    tribunales: BTreeMap<String, String>,
    modalidades: Vec<String>,
    plantillas: BTreeMap<String, Plantilla>,
}

static BASE: LazyLock<Base> = LazyLock::new(|| serde_json::from_str(include_str!("../../../datos/correos.json")).expect("correos.json"));
fn base() -> &'static Base {
    &BASE
}

/// Modalidades que se ofrecen por defecto para el alcance de los correos al tribunal.
pub fn modalidades_base() -> Vec<String> {
    base().modalidades.clone()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tipo {
    /// Al programa: registros en lista de espera.
    ProgramaEspera,
    /// Al programa: informes pendientes.
    ProgramaVencido,
    /// Al programa: informes por vencer.
    ProgramaPorVencer,
    /// Al tribunal: medidas sin vigencia y por vencer.
    Medidas,
    /// Al tribunal: se revisó la pestaña.
    Informativo,
    /// Al tribunal: reporte de proyectos de resolución (filas con Word generado). Solo a pedido.
    Proyectos,
    /// Al tribunal: gestión particular / situación especial. Solo a pedido.
    Especial,
}

impl Tipo {
    pub const TODOS: [Tipo; 7] = [
        Tipo::ProgramaEspera,
        Tipo::ProgramaVencido,
        Tipo::ProgramaPorVencer,
        Tipo::Medidas,
        Tipo::Informativo,
        Tipo::Proyectos,
        Tipo::Especial,
    ];
    /// Los que «Preparar borradores» arma de una vez (`prepare_required_drafts`): el informativo de la etapa y los
    /// que surgen de las acciones. «Reporte de proyectos» y «Gestión particular» solo se preparan a pedido.
    /// En el orden de Python: primero el informativo de la etapa, luego los de las acciones.
    pub const AUTOMATICOS: [Tipo; 5] =
        [Tipo::Informativo, Tipo::ProgramaEspera, Tipo::ProgramaVencido, Tipo::ProgramaPorVencer, Tipo::Medidas];

    /// Acción del motor que alimenta este correo; el informativo no depende de una acción.
    pub fn accion(self) -> Option<&'static str> {
        match self {
            Tipo::ProgramaEspera => Some("programa_espera"),
            Tipo::ProgramaVencido => Some("programa_vencido"),
            Tipo::ProgramaPorVencer => Some("programa_por_vencer"),
            Tipo::Medidas => Some("medidas"),
            Tipo::Informativo | Tipo::Proyectos | Tipo::Especial => None,
        }
    }
    pub fn a_programa(self) -> bool {
        matches!(self, Tipo::ProgramaEspera | Tipo::ProgramaVencido | Tipo::ProgramaPorVencer)
    }
    fn plantilla(self, etapa: Etapa) -> &'static str {
        match self {
            Tipo::Informativo => match etapa {
                Etapa::Espera => "espera",
                Etapa::Cumplimiento => "cumplimiento",
                Etapa::Informes => "informes",
            },
            Tipo::Proyectos => "proyectos",
            Tipo::Especial => "especial",
            otro => otro.accion().unwrap(),
        }
    }
    pub fn titulo(self, etapa: Etapa) -> String {
        base().plantillas[self.plantilla(etapa)].nombre.clone()
    }
    pub fn usa_modalidades(self, etapa: Etapa) -> bool {
        base().plantillas[self.plantilla(etapa)].usa_modalidades
    }
}

/// Direcciones del usuario. Viven en su equipo, nunca en el repositorio.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Contactos {
    /// Nombre del programa → correo(s).
    #[serde(default)]
    pub programas: BTreeMap<String, String>,
    /// LAJA, MULCHEN, TOME → correo(s).
    #[serde(default)]
    pub tribunales: BTreeMap<String, String>,
    #[serde(default)]
    pub cc_adicional: String,
    #[serde(default)]
    pub firma: String,
    /// Alias que agrega el usuario (alias → nombre del contacto), además de `aliases_base.json` de CSMP.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub aliases: BTreeMap<String, String>,
}

static ALIASES_BASE: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    #[derive(Deserialize)]
    struct Alias {
        alias: String,
        nombre_catastro: String,
    }
    let lista: Vec<Alias> = serde_json::from_str(include_str!("../../../datos/aliases.json")).expect("aliases.json");
    lista.into_iter().map(|a| (a.alias, a.nombre_catastro)).collect()
});

/// Tribunales con destinatarios propios (`correos.tribunales`).
pub const TRIBUNALES: [&str; 3] = ["LAJA", "MULCHEN", "TOME"];

/// `config.emails`: direcciones separadas por «;», «,» o salto de línea, sin repetir; una inválida es un error.
pub fn correos_validos(texto: &str) -> Result<Vec<String>, String> {
    static FORMA: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\A[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+\z").unwrap());
    let mut salida: Vec<String> = Vec::new();
    for parte in texto.split([';', ',', '\n']) {
        let parte = parte.trim();
        if !parte.is_empty() && !salida.iter().any(|d| d.to_lowercase() == parte.to_lowercase()) {
            if !FORMA.is_match(parte) {
                return Err(format!("Dirección inválida: {parte}"));
            }
            salida.push(parte.to_string());
        }
    }
    Ok(salida)
}

/// `config.parse_contacts`: lista JSON de {Nombre, Mail} o el formato histórico de objetos separados por comas. Se
/// omiten los registros sin nombre o con una dirección inválida y los nombres con direcciones distintas.
pub fn leer_lista_csmp(texto: &str) -> Result<BTreeMap<String, String>, String> {
    let texto = texto.trim().trim_start_matches('\u{feff}').trim();
    let lista = if texto.starts_with('[') { texto.to_string() } else { format!("[{}]", texto.trim_end_matches(',')) };
    let registros: Vec<serde_json::Value> =
        serde_json::from_str(&lista).map_err(|_| "El archivo de contactos de CSMP no contiene una lista JSON válida.".to_string())?;
    let campo = |r: &serde_json::Value, k: &str| match &r[k] {
        serde_json::Value::String(s) => s.trim().to_string(),
        serde_json::Value::Null => String::new(),
        otro => otro.to_string().trim().to_string(),
    };
    let mut contactos = BTreeMap::new();
    let mut ambiguos = BTreeSet::new();
    for r in &registros {
        let (nombre, correo) = (campo(r, "Nombre"), campo(r, "Mail"));
        if nombre.is_empty() || correos_validos(&correo).is_err() {
            continue;
        }
        match contactos.get(&nombre) {
            Some(previo) if *previo != correo => {
                ambiguos.insert(nombre);
            }
            _ => {
                contactos.insert(nombre, correo);
            }
        }
    }
    for nombre in ambiguos {
        contactos.remove(&nombre);
    }
    Ok(contactos)
}

/// Archivos del Asistente Python donde viven los contactos, en orden de preferencia: la configuración de la app
/// (`app.py`: %LOCALAPPDATA%\CSMP_Personal_Prototipo_Integral\configuracion.json, con `contactos`,
/// `correos.tribunales.*.para` y `aliases`, que es donde Python guarda lo editado) y el directorio local
/// (`config.user_directory() / contactos.json`, %LOCALAPPDATA%\CSMP_Personal).
pub fn rutas_csmp() -> Vec<PathBuf> {
    let Some(base) = std::env::var_os("LOCALAPPDATA")
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var_os("USERPROFILE").filter(|v| !v.is_empty()))
        .or_else(|| std::env::var_os("HOME").filter(|v| !v.is_empty()))
    else {
        return Vec::new();
    };
    let base = PathBuf::from(base);
    vec![base.join("CSMP_Personal_Prototipo_Integral").join("configuracion.json"), base.join("CSMP_Personal").join("contactos.json")]
}

/// `contactos_ejemplo.json` de Python: contactos ficticios que no son del usuario.
fn de_ejemplo(nombre: &str, correo: &str) -> bool {
    matches!(
        (nombre, correo),
        ("PRM EJEMPLO UNO", "prm.uno@example.invalid")
            | ("PIE EJEMPLO DOS", "pie.dos@example.invalid")
            | ("FAE EJEMPLO TRES", "fae.tres@example.invalid")
    )
}

/// Resultado de una importación, a confirmar antes de guardarse: nunca sustituye un contacto con otra dirección.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Importacion {
    pub contactos: Contactos,
    /// Contactos que se agregan (o se completan con la misma dirección).
    pub nuevos: Vec<String>,
    /// Tribunales cuyos destinatarios cambian («Tribunal: LAJA» del archivo de CSMP).
    pub tribunales: Vec<String>,
    /// Nombres con otra dirección ya guardada: se conservan sin cambios.
    pub conflictos: Vec<String>,
}

impl Contactos {
    /// `outputs.resolve_contact`: con los alias de CSMP y los del usuario; una sola dirección o ninguna.
    pub fn de_programa(&self, programa: &str) -> String {
        let alias: BTreeMap<String, String> =
            ALIASES_BASE.iter().chain(self.aliases.iter()).map(|(k, v)| (normalizar_columna(k), normalizar_columna(v))).collect();
        let clave = normalizar_columna(programa);
        let clave = alias.get(&clave).cloned().unwrap_or(clave);
        let halladas: BTreeSet<&String> =
            self.programas.iter().filter(|(k, v)| normalizar_columna(k) == clave && !v.trim().is_empty()).map(|(_, v)| v).collect();
        if halladas.len() == 1 {
            halladas.into_iter().next().unwrap().clone()
        } else {
            String::new()
        }
    }

    /// Alias (base y del usuario) que apuntan a `nombre`.
    pub fn alias_de(&self, nombre: &str) -> Vec<String> {
        let mut todos: BTreeMap<&String, &String> = ALIASES_BASE.iter().collect();
        todos.extend(self.aliases.iter());
        todos.into_iter().filter(|(_, v)| *v == nombre).map(|(k, _)| k.clone()).collect()
    }

    /// Contactos del archivo de CSMP (`contactos.json`, lista {Nombre, Mail}): «Tribunal: LAJA» va a los tribunales.
    pub fn desde_csmp(lista: &BTreeMap<String, String>) -> Contactos {
        Contactos::default().importar_csmp(lista).contactos
    }

    /// Lee los contactos del Asistente Python sin modificarlos: `configuracion.json` de la app o `contactos.json`.
    /// `None` si no existen o solo trae los contactos ficticios de ejemplo.
    pub fn leer_csmp(ruta: &Path) -> Result<Option<Contactos>, String> {
        if ruta.file_name().is_some_and(|n| n == "configuracion.json") {
            return Self::leer_configuracion_csmp(ruta);
        }
        Self::leer_lista_csmp(ruta)
    }

    fn leer_configuracion_csmp(ruta: &Path) -> Result<Option<Contactos>, String> {
        use std::io::Read;
        let archivo = match std::fs::File::open(ruta) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("No se pudo leer la configuración de CSMP.".into()),
        };
        let mut raw = vec![];
        archivo.take(16 * 1024 * 1024 + 1).read_to_end(&mut raw).map_err(|_| "No se pudo leer la configuración de CSMP.")?;
        if raw.len() > 16 * 1024 * 1024 {
            return Err("La configuración de CSMP supera 16 MiB.".into());
        }
        let v: serde_json::Value = serde_json::from_slice(&raw).map_err(|_| "La configuración de CSMP no contiene JSON válido.")?;
        let mut c = Contactos::default();
        for (nombre, correo) in v["contactos"].as_object().into_iter().flatten() {
            let (nombre, correo) = (nombre.trim(), correo.as_str().unwrap_or("").trim());
            if !nombre.is_empty() && !de_ejemplo(nombre, correo) && correos_validos(correo).is_ok() {
                c.programas.insert(nombre.to_string(), correo.to_string());
            }
        }
        for clave in TRIBUNALES {
            let para: Vec<String> = v["correos"]["tribunales"][clave]["para"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|d| d.as_str())
                .filter_map(|d| correos_validos(d).ok())
                .flatten()
                .collect();
            if !para.is_empty() {
                c.tribunales.insert(clave.to_string(), para.join("; "));
            }
        }
        for (alias, destino) in v["aliases"].as_object().into_iter().flatten() {
            if let Some(destino) = destino.as_str().filter(|d| !d.trim().is_empty()) {
                if ALIASES_BASE.get(alias).map(String::as_str) != Some(destino) {
                    c.aliases.insert(alias.clone(), destino.to_string());
                }
            }
        }
        c.cc_adicional = v["correos"]["cc_adicional"].as_str().unwrap_or("").trim().to_string();
        c.firma = v["firma"].as_str().unwrap_or("").trim().to_string();
        let tribunales = c.tribunales.values().any(|d| !d.is_empty());
        Ok((!c.programas.is_empty() || tribunales).then_some(c))
    }

    fn leer_lista_csmp(ruta: &Path) -> Result<Option<Contactos>, String> {
        use std::io::Read;
        let archivo = match std::fs::File::open(ruta) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("No se pudieron leer los contactos de CSMP.".into()),
        };
        let mut raw = vec![];
        archivo.take(2 * 1024 * 1024 + 1).read_to_end(&mut raw).map_err(|_| "No se pudieron leer los contactos de CSMP.")?;
        if raw.len() > 2 * 1024 * 1024 {
            return Err("El archivo de contactos de CSMP supera 2 MiB.".into());
        }
        let texto = String::from_utf8(raw).map_err(|_| "El archivo de contactos de CSMP no está en UTF-8.")?;
        let mut lista = leer_lista_csmp(&texto)?;
        lista.retain(|n, c| !de_ejemplo(n, c));
        Ok((!lista.is_empty()).then(|| Contactos::desde_csmp(&lista)))
    }

    /// `app_base._import_contacts` con un JSON de CSMP: los tribunales reciben sus destinatarios y un programa ya
    /// guardado con otra dirección queda en conflicto, sin sustituirse.
    pub fn importar_csmp(&self, lista: &BTreeMap<String, String>) -> Importacion {
        let mut r = Importacion { contactos: self.clone(), ..Default::default() };
        for (nombre, correo) in lista {
            if let Some(clave) = nombre.strip_prefix("Tribunal: ").map(|t| t.trim().to_uppercase()) {
                if TRIBUNALES.contains(&clave.as_str()) {
                    let direcciones = correos_validos(correo).unwrap_or_default().join("; ");
                    if r.contactos.tribunales.get(&clave) != Some(&direcciones) {
                        r.tribunales.push(clave.clone());
                    }
                    r.contactos.tribunales.insert(clave, direcciones);
                    continue;
                }
            }
            match r.contactos.programas.get(nombre) {
                Some(previo) if previo != correo => r.conflictos.push(nombre.clone()),
                Some(_) => {}
                None => {
                    r.contactos.programas.insert(nombre.clone(), correo.clone());
                    r.nuevos.push(nombre.clone());
                }
            }
        }
        r
    }

    /// `outputs.import_contacts`: catastro con columnas Nombre/Programa y Mail/Correo (primera fila de títulos). Un
    /// nombre (normalizado) ya guardado con otra dirección queda en conflicto, sin sustituirse.
    pub fn importar_catastro(&self, cabeceras: &[String], filas: &[Vec<String>]) -> Result<Importacion, String> {
        let columnas: BTreeMap<String, usize> = cabeceras.iter().enumerate().map(|(i, c)| (normalizar_columna(c), i)).collect();
        let nombre = ["nombre", "programa", "nombre programa"].iter().find_map(|k| columnas.get(*k).copied());
        let correo = ["mail", "correo", "email", "correo electronico"].iter().find_map(|k| columnas.get(*k).copied());
        let (Some(nombre), Some(correo)) = (nombre, correo) else {
            return Err("El catastro necesita columnas Nombre/Programa y Mail/Correo.".into());
        };
        let mut r = Importacion { contactos: self.clone(), ..Default::default() };
        for fila in filas {
            let celda = |i: usize| fila.get(i).map(|s| s.trim().to_string()).unwrap_or_default();
            let n = celda(nombre);
            let m = correos_validos(&celda(correo))?.join("; ");
            if n.is_empty() {
                continue;
            }
            let clave = normalizar_columna(&n);
            let previas: BTreeSet<&String> =
                r.contactos.programas.iter().filter(|(k, _)| normalizar_columna(k) == clave).map(|(_, v)| v).collect();
            if !previas.is_empty() && previas != BTreeSet::from([&m]) {
                r.conflictos.push(n);
                continue;
            }
            if r.contactos.programas.get(&n) != Some(&m) {
                r.nuevos.push(n.clone());
            }
            r.contactos.programas.insert(n, m);
        }
        Ok(r)
    }

    /// `personalization.save_contact`: guarda un programa nuevo o, con `previo`, edita (o renombra) uno existente, con
    /// sus alias. No reemplaza otro contacto ni toma alias ajenos. Los alias base de CSMP no se borran.
    pub fn guardar_programa(&mut self, previo: Option<&str>, nombre: &str, correo: &str, alias: &[String]) -> Result<(), String> {
        let nombre = nombre.trim();
        if nombre.is_empty() {
            return Err("Indica un nombre para el contacto.".into());
        }
        let correo = correos_validos(correo)?.join("; ");
        let n = normalizar_columna;
        let todos: Vec<(&String, &String)> = ALIASES_BASE.iter().chain(self.aliases.iter()).collect();
        if todos.iter().any(|(a, t)| n(a) == n(nombre) && Some(t.as_str()) != previo) {
            return Err(format!("El nombre ya es alias de otro contacto: {nombre}"));
        }
        let otros: BTreeSet<String> = self.programas.keys().filter(|k| Some(k.as_str()) != previo).map(|k| n(k)).collect();
        if otros.contains(&n(nombre)) && Some(nombre) != previo {
            return Err("Ya existe otro contacto con ese nombre. No se reemplazó.".into());
        }
        let mut limpios: Vec<String> = Vec::new();
        for a in alias.iter().map(|a| a.trim()).filter(|a| !a.is_empty()) {
            if !limpios.iter().any(|x| x == a) {
                limpios.push(a.to_string());
            }
        }
        let propios: BTreeSet<&String> = todos.iter().filter(|(_, t)| Some(t.as_str()) == previo).map(|(a, _)| *a).collect();
        for a in &limpios {
            if otros.contains(&n(a)) {
                return Err(format!("El alias coincide con otro contacto: {a}"));
            }
            if todos.iter().any(|(k, t)| n(k) == n(a) && !propios.contains(k) && n(t) != n(nombre)) {
                return Err(format!("El alias ya pertenece a otro contacto: {a}"));
            }
        }
        if let Some(previo) = previo {
            self.programas.remove(previo);
            self.aliases.retain(|_, t| t != previo);
        }
        for a in limpios {
            if ALIASES_BASE.get(&a).map(String::as_str) != Some(nombre) {
                self.aliases.insert(a, nombre.to_string());
            }
        }
        self.programas.insert(nombre.to_string(), correo);
        Ok(())
    }

    /// Quita un programa y los alias del usuario que apuntan a él.
    pub fn quitar_programa(&mut self, nombre: &str) {
        self.programas.remove(nombre);
        self.aliases.retain(|_, v| v != nombre);
    }
    pub fn cargar(ruta: &Path) -> Result<Contactos, String> {
        use std::io::Read;
        let archivo = match std::fs::File::open(ruta) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(_) => return Err("No se pudieron leer los contactos. El archivo se conserva y no se reemplazará.".into()),
        };
        let mut raw = vec![];
        archivo
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut raw)
            .map_err(|_| "No se pudieron leer los contactos; el archivo se conserva.")?;
        if raw.len() > 2 * 1024 * 1024 {
            return Err("El archivo de contactos supera 2 MiB; se conserva sin modificar.".into());
        }
        serde_json::from_slice(&raw)
            .map_err(|_| "El archivo de contactos no contiene JSON válido. Corríjalo o recupere una copia; no se reemplazará.".into())
    }
    pub fn guardar(&self, ruta: &Path) -> Result<(), String> {
        use std::io::Write;
        // Una carga fallida nunca se convierte en un archivo vacío al editar o cerrar la aplicación.
        Self::cargar(ruta)?;
        let carpeta = ruta.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        std::fs::create_dir_all(carpeta).map_err(|_| "No se pudo crear la carpeta de contactos.")?;
        let texto = serde_json::to_vec_pretty(self).map_err(|_| "No se pudieron preparar los contactos.")?;
        let mut tmp = tempfile::NamedTempFile::new_in(carpeta)
            .map_err(|_| "No se pudieron preparar los contactos; el archivo anterior se conserva.")?;
        tmp.write_all(&texto)
            .and_then(|_| tmp.as_file().sync_all())
            .map_err(|_| "No se pudieron guardar los contactos; el archivo anterior se conserva.")?;
        tmp.persist(ruta).map_err(|_| "No se pudo reemplazar el archivo de contactos; el anterior se conserva.")?;
        Ok(())
    }
}

/// Direcciones de una lista separada por «;» o «,», sin repetir.
pub fn direcciones(texto: &str) -> Vec<String> {
    let mut salida: Vec<String> = Vec::new();
    for parte in texto.split([';', ',']) {
        let limpia = parte.trim();
        if limpia.contains('@') && !salida.iter().any(|d| d.eq_ignore_ascii_case(limpia)) {
            salida.push(limpia.to_string());
        }
    }
    salida
}

/// CC de todo correo: el fijo primero y luego los adicionales. El fijo no se puede quitar.
pub fn cc(adicional: &str) -> String {
    direcciones(&format!("{CC_FIJO};{adicional}")).join("; ")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Borrador {
    pub tipo: Tipo,
    pub tribunal: String,
    pub programa: String,
    pub para: String,
    pub cc: String,
    pub asunto: String,
    pub cuerpo: String,
    pub adjuntos: Vec<PathBuf>,
    pub filas: Vec<String>,
    /// El usuario lo deja fuera del lote sin borrarlo.
    #[serde(default)]
    pub omitido: bool,
    /// Texto al prepararlo (`Draft.original`): lo que difiere es edición del usuario y se conserva al volver a preparar.
    #[serde(default)]
    pub originales: Option<Originales>,
    /// Huella de los registros de los que depende (`dependency_hash`; ver `vigencia`). Vacía: sin control.
    #[serde(default)]
    pub huella: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Originales {
    pub para: String,
    pub cc: String,
    pub asunto: String,
    pub cuerpo: String,
}

/// `session.py::merge_draft_edits`: al volver a preparar, un borrador del mismo tipo, destino, tribunal y programa
/// conserva lo que el usuario cambió (destinatario, CC, asunto y texto) y su omisión.
pub fn conservar_ediciones(previos: &[Borrador], nuevos: &mut [Borrador]) {
    for b in nuevos.iter_mut() {
        let clave = |x: &Borrador| (x.tipo, x.tipo.a_programa(), x.tribunal.clone(), x.programa.clone());
        let Some(viejo) = previos.iter().find(|v| clave(v) == clave(b)) else { continue };
        if let Some(o) = &viejo.originales {
            if viejo.para != o.para {
                b.para = viejo.para.clone();
            }
            if viejo.cc != o.cc {
                b.cc = viejo.cc.clone();
            }
            if viejo.asunto != o.asunto {
                b.asunto = viejo.asunto.clone();
            }
            if viejo.cuerpo != o.cuerpo {
                b.cuerpo = viejo.cuerpo.clone();
            }
        }
        b.omitido = viejo.omitido;
    }
}

impl Borrador {
    /// Qué falta para poder guardarlo en Outlook.
    pub fn faltantes(&self) -> Vec<&'static str> {
        let mut f = Vec::new();
        if direcciones(&self.para).is_empty() {
            f.push("destinatario");
        }
        if !direcciones(&self.cc).iter().any(|d| d.eq_ignore_ascii_case(CC_FIJO)) {
            f.push("CC fijo");
        }
        if self.asunto.trim().is_empty() {
            f.push("asunto");
        }
        f
    }
}

#[derive(Debug, Clone, Default)]
pub struct Opciones {
    /// Modalidades efectivamente comprendidas (correos al tribunal).
    pub modalidades: String,
    pub hoy: Option<flow_dominio::Fecha>,
    /// Claves de modalidad (RES, AMB, FAE, DCE) cuyas filas entran. `None`: todas (`modalities.py::selected_row`).
    pub claves_modalidad: Option<BTreeSet<&'static str>>,
    /// PERIODO de las plantillas que lo usan (reporte de proyectos). En la ventana parte en el mes actual (mm/aaaa).
    pub periodo: String,
    /// Filas con proyecto Word generado: las únicas que entran en el reporte de proyectos (recibos `word`).
    pub con_word: BTreeSet<String>,
    /// Filas elegidas en Trabajo («Solo filas seleccionadas»). `None`: todas.
    pub seleccion: Option<BTreeSet<String>>,
    /// `manual_selection` de «Preparar tipo»: las filas elegidas entran aunque el motor no proponga ese correo (y en el
    /// reporte de proyectos, aunque no tengan Word).
    pub manual: bool,
    /// Reglas vigentes en el programa, para la huella; vacío: las del trabajo.
    pub reglas: String,
}

/// `outputs.py::prepare_required_drafts` («Preparar todos»): exige modalidades y prepara el informativo de la etapa y
/// los correos que surgen de las acciones de las filas elegidas. Un tipo que no se puede preparar no anula los demás
/// (se informa en la segunda lista).
pub fn preparar_todos(
    trabajo: &Trabajo,
    opciones: &Opciones,
    contactos: &Contactos,
    carpeta: &Path,
) -> Result<(Vec<Borrador>, Vec<String>), String> {
    if trabajo.fuente.is_some() {
        return Ok((Vec::new(), Vec::new()));
    }
    if opciones.modalidades.trim().is_empty() {
        return Err("Selecciona al menos una modalidad para preparar el conjunto de correos del trabajo.".into());
    }
    let opciones = Opciones { manual: false, ..opciones.clone() };
    let (mut lista, mut fallidos) = (Vec::new(), Vec::new());
    for tipo in Tipo::AUTOMATICOS {
        match preparar(trabajo, tipo, &opciones, contactos, carpeta) {
            Ok(mut nuevos) => lista.append(&mut nuevos),
            Err(e) => fallidos.push(format!("{}: {e}", tipo.titulo(trabajo.etapa))),
        }
    }
    Ok((lista, fallidos))
}

/// Modalidades de `src/nurus/personal/modalities.py`: clave y nombre.
pub const MODALIDADES: [(&str, &str); 4] = [
    ("RES", "Cuidado alternativo residencial"),
    ("AMB", "Intervención ambulatoria de reparación"),
    ("FAE", "Cuidado alternativo familia de acogida"),
    ("DCE", "Diagnóstico Clínico Especializado (DCE)"),
];

/// Clave de modalidad de un texto (nombre de modalidad o de programa): `modalities.py::modality`. Vacío si no se sabe.
pub fn modalidad(programa: &str) -> &'static str {
    let nombre = normalizar_columna(programa).to_uppercase();
    for (clave, etiqueta) in MODALIDADES {
        if nombre == clave || nombre == normalizar_columna(etiqueta).to_uppercase() {
            return clave;
        }
    }
    let primero = nombre.split(|c: char| c.is_whitespace() || c == '-').next().unwrap_or("");
    if primero == "DCE" {
        return "DCE";
    }
    if matches!(primero, "FAE" | "FAS") {
        return "FAE";
    }
    if ["REM", "RLP", "RMA", "RFA", "RDS", "RSP", "RPE"].iter().any(|p| primero.starts_with(p))
        || matches!(primero, "RTA" | "RTT" | "RVA" | "RES" | "RESIDENCIA" | "RESIDENCIAL")
    {
        return "RES";
    }
    if matches!(primero, "AFT" | "PF" | "PRM" | "PPF" | "PIE" | "PDE" | "PEE" | "PAS" | "DAM" | "OPD" | "PDC") {
        return "AMB";
    }
    ""
}

/// Claves de una lista de nombres de modalidad. `None` (sin filtro) si la lista está vacía, si alguno no es una
/// modalidad conocida (no se puede saber qué filas comprende) o si están todas.
pub fn claves_modalidad<'a>(nombres: impl IntoIterator<Item = &'a str>) -> Option<BTreeSet<&'static str>> {
    let mut claves = BTreeSet::new();
    for nombre in nombres {
        match modalidad(nombre) {
            "" => return None,
            clave => {
                claves.insert(clave);
            }
        }
    }
    (!claves.is_empty() && claves.len() < MODALIDADES.len()).then_some(claves)
}

/// Claves desde el texto libre de modalidades («A, B y C»), para la línea de comandos.
pub fn claves_de_texto(texto: &str) -> Option<BTreeSet<&'static str>> {
    let partes: Vec<String> = texto.split([',', ';']).flat_map(|p| p.split(" y ")).map(|p| p.trim().to_string()).collect();
    claves_modalidad(partes.iter().map(String::as_str).filter(|p| !p.is_empty()))
}

/// La fila entra en las modalidades elegidas (`modalities.py::selected_row`): la modalidad explícita del registro
/// (o una columna MODALIDAD / TIPO PROGRAMA) y, si no se reconoce, el prefijo del programa.
pub fn en_modalidades(trabajo: &Trabajo, fila: &FilaTrabajo, claves: Option<&BTreeSet<&'static str>>) -> bool {
    let Some(claves) = claves else { return true };
    if claves.len() == MODALIDADES.len() {
        return true;
    }
    let mut explicita = fila.registro.texto("modalidad");
    if explicita.trim().is_empty() {
        explicita = trabajo
            .cabeceras
            .iter()
            .position(|c| matches!(normalizar_columna(c).trim(), "modalidad" | "tipo programa"))
            .and_then(|i| fila.celdas.get(i))
            .map(|c| c.texto())
            .unwrap_or_default();
    }
    let clave = match modalidad(&explicita) {
        "" => modalidad(&valor(fila, "programa")),
        clave => clave,
    };
    claves.contains(clave)
}

fn rellenar(texto: &str, contexto: &[(&str, &str)]) -> Result<String, String> {
    let mut salida = texto.to_string();
    for (clave, valor) in contexto {
        let marca = format!("{{{clave}}}");
        if salida.contains(&marca) {
            if valor.trim().is_empty() {
                return Err(format!("Completa {clave} para esta plantilla."));
            }
            salida = salida.replace(&marca, valor);
        }
    }
    Ok(salida)
}

/// Prepara los borradores de un tipo. Agrupa por tribunal (y por programa si va a programas) y crea las nóminas en
/// `carpeta`. No toca Outlook.
pub fn preparar(
    trabajo: &Trabajo,
    tipo: Tipo,
    opciones: &Opciones,
    contactos: &Contactos,
    carpeta: &Path,
) -> Result<Vec<Borrador>, String> {
    if trabajo.fuente.is_some() {
        return Ok(Vec::new());
    }
    let base = base();
    let clave_plantilla = tipo.plantilla(trabajo.etapa);
    let plantilla = &base.plantillas[clave_plantilla];
    if plantilla.usa_modalidades && opciones.modalidades.trim().is_empty() {
        return Err("Indica las modalidades efectivamente comprendidas.".into());
    }
    let hoy = opciones.hoy.unwrap_or(trabajo.corte).format("%d/%m/%Y").to_string();
    // «Preparar tipo» de Python solo filtra por modalidad las plantillas que las usan; estos dos nunca.
    let claves = if matches!(tipo, Tipo::Proyectos | Tipo::Especial) { None } else { opciones.claves_modalidad.as_ref() };
    let mut grupos: BTreeMap<(Tribunal, String), Vec<&FilaTrabajo>> = BTreeMap::new();
    let mut orden: Vec<(Tribunal, String)> = Vec::new();
    for fila in &trabajo.filas {
        if fila.propuesta.excluido
            || opciones.seleccion.as_ref().is_some_and(|s| !s.contains(&fila.id))
            || (!opciones.manual && tipo.accion().is_some_and(|a| !fila.tiene_accion(a)))
            || (!opciones.manual && tipo == Tipo::Proyectos && !opciones.con_word.contains(&fila.id))
            || !en_modalidades(trabajo, fila, claves)
        {
            continue;
        }
        let Some(tribunal) = Tribunal::desde(&valor(fila, "tribunal")) else { continue };
        let programa = if tipo.a_programa() { normalizar_columna(&valor(fila, "programa")) } else { String::new() };
        let clave = (tribunal, programa);
        if !grupos.contains_key(&clave) {
            orden.push(clave.clone());
        }
        grupos.entry(clave).or_default().push(fila);
    }
    let mut salida = Vec::new();
    for clave in orden {
        let filas = &grupos[&clave];
        let tribunal = clave.0;
        let nombre_tribunal = base.tribunales[tribunal.nombre()].clone();
        let programa = if tipo.a_programa() { valor(filas[0], "programa") } else { String::new() };
        let tabla = filas.iter().map(|f| format!("{} — {}", valor(f, "rit"), valor(f, "nombre"))).collect::<Vec<_>>().join("\n");
        let contexto = [
            ("TRIBUNAL", nombre_tribunal.as_str()),
            ("PROGRAMA", programa.as_str()),
            ("FECHA", hoy.as_str()),
            ("ALCANCE_MODALIDADES", opciones.modalidades.as_str()),
            ("PERIODO", opciones.periodo.as_str()),
            ("TABLA_REGISTROS", tabla.as_str()),
        ];
        let mut cuerpo = rellenar(&plantilla.cuerpo, &contexto)?;
        if !contactos.firma.trim().is_empty() {
            cuerpo = format!("{cuerpo}\n\n{}", contactos.firma.trim());
        }
        let asunto = rellenar(&plantilla.asunto, &contexto)?;
        let mut adjuntos = Vec::new();
        if plantilla.adjunto == "obligatorio" {
            std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
            // Un adjunto por programa dentro del mismo borrador (`outputs.py`: `by_program`, texto tal cual).
            let mut por_programa: Vec<(String, Vec<&FilaTrabajo>)> = Vec::new();
            for fila in filas {
                let p = valor(fila, "programa");
                match por_programa.iter_mut().find(|(k, _)| *k == p) {
                    Some((_, v)) => v.push(fila),
                    None => por_programa.push((p, vec![fila])),
                }
            }
            // Al tribunal, cada nómina se llama como su programa (`program_filename`), en una carpeta propia.
            let propia = if tipo.a_programa() {
                None
            } else {
                let nombre = crate::excel::nombre_archivo(&format!("{} - {}", plantilla.nombre, tribunal.titulo()));
                Some(crate::excel::carpeta_nueva(&carpeta.join("adjuntos"), &nombre)?)
            };
            for (nombre_programa, subconjunto) in por_programa {
                let ruta = match &propia {
                    None => ruta_nueva(carpeta, &format!("Nómina {} - {}", nombre_programa, tribunal.titulo()), "xlsx"),
                    Some(dir) => ruta_nueva(dir, &nombre_programa, "xlsx"),
                };
                nomina(trabajo, &subconjunto, clave_plantilla, &ruta)?;
                adjuntos.push(ruta);
            }
        }
        salida.push(Borrador {
            tipo,
            tribunal: nombre_tribunal.clone(),
            para: if tipo.a_programa() {
                contactos.de_programa(&programa)
            } else {
                direcciones(contactos.tribunales.get(tribunal.nombre()).map(String::as_str).unwrap_or("")).join("; ")
            },
            programa,
            cc: cc(&contactos.cc_adicional),
            asunto,
            cuerpo,
            adjuntos,
            filas: filas.iter().map(|f| f.id.clone()).collect(),
            omitido: false,
            originales: None,
            huella: String::new(),
        });
    }
    let reglas = if opciones.reglas.is_empty() { trabajo.version_reglas.as_str() } else { opciones.reglas.as_str() };
    let huellas = crate::vigencia::Huellas::nuevas(trabajo, reglas);
    for b in &mut salida {
        b.originales = Some(Originales { para: b.para.clone(), cc: b.cc.clone(), asunto: b.asunto.clone(), cuerpo: b.cuerpo.clone() });
        b.huella = huellas.de_borrador(b, contactos);
    }
    Ok(salida)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(datos: &[u8]) -> String {
    let mut salida = String::with_capacity(datos.len().div_ceil(3) * 4);
    for trozo in datos.chunks(3) {
        let n = (u32::from(trozo[0]) << 16) | (u32::from(*trozo.get(1).unwrap_or(&0)) << 8) | u32::from(*trozo.get(2).unwrap_or(&0));
        salida.push(B64[(n >> 18) as usize & 63] as char);
        salida.push(B64[(n >> 12) as usize & 63] as char);
        salida.push(if trozo.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        salida.push(if trozo.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    salida
}

fn en_lineas(texto: &str) -> String {
    texto.as_bytes().chunks(76).map(|l| std::str::from_utf8(l).unwrap()).collect::<Vec<_>>().join("\r\n")
}

fn cabecera(texto: &str) -> String {
    if texto.is_ascii() {
        texto.to_string()
    } else {
        // Palabras codificadas de a 45 bytes como máximo, sin partir un carácter.
        let mut partes = Vec::new();
        let mut actual = String::new();
        for c in texto.chars() {
            if actual.len() + c.len_utf8() > 45 {
                partes.push(std::mem::take(&mut actual));
            }
            actual.push(c);
        }
        partes.push(actual);
        partes.iter().map(|p| format!("=?UTF-8?B?{}?=", base64(p.as_bytes()))).collect::<Vec<_>>().join("\r\n ")
    }
}

/// Archivo `.eml` sin enviar (`X-Unsent: 1`): al abrirlo, Outlook muestra el borrador listo para revisar.
/// Es la salida cuando no hay Outlook de escritorio disponible. No envía nada.
pub fn eml(borrador: &Borrador) -> Result<Vec<u8>, String> {
    let faltan = borrador.faltantes();
    if faltan.contains(&"CC fijo") {
        return Err("El borrador no lleva el CC fijo.".into());
    }
    let limite = "----=_Flow_borrador_0001";
    let mut m = String::new();
    m.push_str("X-Unsent: 1\r\n");
    m.push_str(&format!("To: {}\r\n", direcciones(&borrador.para).join(", ")));
    m.push_str(&format!("Cc: {}\r\n", direcciones(&borrador.cc).join(", ")));
    m.push_str(&format!("Subject: {}\r\n", cabecera(&borrador.asunto)));
    m.push_str("MIME-Version: 1.0\r\n");
    m.push_str(&format!("Content-Type: multipart/mixed; boundary=\"{limite}\"\r\n\r\n"));
    m.push_str(&format!("--{limite}\r\nContent-Type: text/plain; charset=\"utf-8\"\r\nContent-Transfer-Encoding: base64\r\n\r\n"));
    m.push_str(&en_lineas(&base64(borrador.cuerpo.replace('\n', "\r\n").as_bytes())));
    m.push_str("\r\n");
    for ruta in &borrador.adjuntos {
        let datos = std::fs::read(ruta).map_err(|e| format!("No se pudo leer el adjunto {}: {e}", ruta.display()))?;
        let nombre = ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        m.push_str(&format!(
            "--{limite}\r\nContent-Type: application/octet-stream; name=\"{n}\"\r\nContent-Transfer-Encoding: base64\r\n\
             Content-Disposition: attachment; filename=\"{n}\"\r\n\r\n",
            n = cabecera(&nombre)
        ));
        m.push_str(&en_lineas(&base64(&datos)));
        m.push_str("\r\n");
    }
    m.push_str(&format!("--{limite}--\r\n"));
    Ok(m.into_bytes())
}

/// Guarda el borrador como `.eml` nuevo en la carpeta.
pub fn guardar_eml(borrador: &Borrador, carpeta: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
    let nombre = if borrador.programa.is_empty() {
        format!("Borrador {}", borrador.asunto)
    } else {
        format!("Borrador {} - {}", borrador.programa, borrador.tribunal)
    };
    let ruta = ruta_nueva(carpeta, &nombre.chars().take(110).collect::<String>(), "eml");
    std::fs::write(&ruta, eml(borrador)?).map_err(|e| format!("No se pudo guardar el borrador: {e}"))?;
    Ok(ruta)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn cc_fijo_siempre_primero() {
        assert_eq!(cc(""), CC_FIJO);
        assert_eq!(cc("otra@example.invalid; UCC_CONCEPCION@pjud.cl"), format!("{CC_FIJO}; otra@example.invalid"));
    }

    #[test]
    fn base64_estandar() {
        assert_eq!(base64(b"Flow"), "Rmxvdw==");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
    }

    #[test]
    fn todas_las_plantillas_existen() {
        for etapa in [Etapa::Espera, Etapa::Cumplimiento, Etapa::Informes] {
            for tipo in Tipo::TODOS {
                assert!(!tipo.titulo(etapa).is_empty());
            }
        }
    }
}
