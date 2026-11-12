use crate::{Modo, Opciones, OMITIDO};
use chrono::{Datelike, Duration, NaiveDate};
use flow_dominio::texto::ascii;
use hmac::{Hmac, Mac};
use regex::{Captures, Regex};
use serde_json::Value;
use sha2::Sha256;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};

static VOCAB: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(include_str!("../../../datos/anonimo_vocab.json")).unwrap());
static RUT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(\d{1,2}(?:[. ]\d{3}){2}|\d{6,8})(\s*[-‐‑‒–—−]\s*|\s+)([\dkK])\b").unwrap());
static RIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b([A-Za-z])[-‐‑‒–—−](\d{1,6})[-‐‑‒–—−/]((?:19|20)\d{2})\b").unwrap());
static RIT_LIBRE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:([A-Za-z])[\s\-‐‑‒–—−]+)?(\d{1,6})[\s\-‐‑‒–—−/]+((?:19|20)\d{2})\b").unwrap());
static MAIL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+").unwrap());
static TELEFONO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:\+?56[ -]?)?\b(?:9[ -]?\d{4}[ -]?\d{4}|\d{3}[ -]\d{3}[ -]\d{3}|\d{2}[ -]?\d{3}[ -]?\d{4})\b").unwrap()
});
static URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\b(?:https?://|ftp://|www\.)[^\s<>"')\]]+"#).unwrap());
static RUTA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:[A-Z]:[\\/]+(?:Users|Usuarios|Documents and Settings)[\\/]+|/(?:home|Users)/)[^\\/:*?"<>|\r\n]+|\\\\[^\\/\s]+"#)
        .unwrap()
});
static DIRECCION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:avenida|av|calle|pasaje|pje|villa|poblacion|población|block|depto|camino)\.?\s+[\w .'-]{2,40}?\d{1,5}").unwrap()
});
static DIGITOS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d{7,12}\b|\b\d{1,2}(?:\.\d{3}){2}\b").unwrap());
static PALABRAS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\pL\pN]+").unwrap());
static FECHA_ISO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap());
static FECHA_DMY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{1,2})([/.-])(\d{1,2})([/.-])(\d{4})").unwrap());
static ESTRUCTURA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[+-]?\d{1,6}(?:[.,]\d+)?|\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?)?|\d{1,2}[/.-]\d{1,2}[/.-]\d{4}(?:[ T]\d{1,2}:\d{2}(?::\d{2})?)?|(?i:true|false|si|sí|no))$").unwrap()
});
static CANDIDATO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:\p{Lu}\p{Ll}{2,}|\p{Lu}{3,})(?:\s+(?:(?:de|del|la|las|los|san|santa|y)\s+)?(?:\p{Lu}\p{Ll}{2,}|\p{Lu}{3,})){1,3}\b")
        .unwrap()
});
static ROL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"((?i:\b(?:sr|sra|srta|don|doña|dona|dr|dra|nna|madre|padre|abuela|abuelo|tia|tía|tio|tío|hermano|hermana|hijo|hija|curador|curadora|abogado|abogada|psicólogo|psicóloga|apoderado|tutor|menor|joven|niño|niña|contactar a|hablar con|llamar a)\.?\s+))((?:\p{Lu}\p{Ll}{2,}|\p{Lu}{3,})(?:\s+(?:(?:de|del|la|las|los|san|santa|y)\s+)?(?:\p{Lu}\p{Ll}{2,}|\p{Lu}{3,})){0,3})").unwrap()
});

pub fn normal(s: &str) -> String {
    ascii(s)
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
static LISTAS: LazyLock<BTreeMap<String, BTreeSet<String>>> = LazyLock::new(|| {
    VOCAB
        .as_object()
        .unwrap()
        .iter()
        .filter_map(|(k, v)| v.as_array().map(|vs| (k.clone(), vs.iter().filter_map(Value::as_str).map(normal).collect())))
        .collect()
});
fn en(lista: &str, s: &str) -> bool {
    LISTAS.get(lista).is_some_and(|l| l.contains(s))
}
fn prefijos(lista: &str, s: &str) -> bool {
    LISTAS.get(lista).is_some_and(|l| l.iter().any(|v| s.starts_with(v)))
}
pub fn parece_nombre(s: &str) -> bool {
    CANDIDATO.captures_iter(s).any(|c| !institucional(&c[0]))
}
pub fn titulo_seguro(s: &str) -> bool {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("^(?:{})$", VOCAB["SAFE_TITLE"].as_str().unwrap())).unwrap());
    RE.is_match(&normal(s))
}
pub fn credencial(s: &str) -> bool {
    [
        "password",
        "passwd",
        "contrasena",
        "authorization",
        "cookie",
        "set cookie",
        "token",
        "secret",
        "api key",
        "access token",
        "refresh token",
        "csrf",
        "nonce",
    ]
    .contains(&normal(s).as_str())
}
pub fn protegido(s: &str) -> bool {
    let n = normal(s);
    en("ENGINE_HEADERS", &n) || ["tribunal", "fecha", "fec ", "dias ", "t espera"].iter().any(|p| n.starts_with(p))
}
pub fn estructural(s: &str) -> bool {
    let s = s.trim();
    s.is_empty() || ESTRUCTURA.is_match(s) || en("STATE_WORDS", &normal(s))
}
fn institucional(s: &str) -> bool {
    let n = normal(s);
    let palabras = n.split_whitespace().filter(|p| !en("STOPWORDS", p)).collect::<Vec<_>>();
    !palabras.is_empty() && palabras.iter().all(|p| en("INSTITUTIONAL", p))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clase {
    Persona,
    Curador,
    Rut,
    Rit,
    Correo,
    Telefono,
    Direccion,
    Identificador,
    Huella,
    Json,
    Texto,
    Tecnico,
}
pub fn clase(s: &str) -> Option<Clase> {
    static CACHE: LazyLock<std::sync::Mutex<BTreeMap<String, Option<Clase>>>> = LazyLock::new(|| std::sync::Mutex::new(BTreeMap::new()));
    if s.len() > 256 {
        return clasificar(s);
    }
    if let Some(v) = CACHE.lock().ok().and_then(|c| c.get(s).copied()) {
        return v;
    }
    let resultado = clasificar(s);
    if let Ok(mut cache) = CACHE.lock() {
        if cache.len() < 16384 {
            cache.insert(s.into(), resultado);
        }
    }
    resultado
}
fn clasificar(s: &str) -> Option<Clase> {
    let n = normal(s);
    let palabras = n.split_whitespace().collect::<Vec<_>>();
    if VOCAB["TECHNICAL"].get(&n).is_some() {
        return Some(Clase::Tecnico);
    }
    if ["sha256", "sha 256", "huella", "hash", "checksum", "md5"].iter().any(|k| n.contains(k)) {
        return Some(Clase::Huella);
    }
    if prefijos("KEEP", &n) {
        return None;
    }
    if palabras.iter().any(|p| *p == "rit" || p.strip_prefix("rit").is_some_and(|s| s.starts_with(|c: char| c.is_ascii_digit()))) {
        return Some(Clase::Rit);
    }
    if n.contains("curador") || palabras.contains(&"cur") {
        return Some(Clase::Curador);
    }
    if palabras.iter().any(|p| {
        matches!(*p, "rut" | "run")
            || ["rut", "run"].iter().any(|pre| p.strip_prefix(pre).is_some_and(|s| s.starts_with(|c: char| c.is_ascii_digit())))
    }) {
        return Some(Clase::Rut);
    }
    if ["correo", "email", "mail"].iter().any(|k| n.contains(k)) {
        return Some(Clase::Correo);
    }
    if ["telefono", "fono", "celular"].iter().any(|k| n.contains(k)) || palabras.contains(&"tel") {
        return Some(Clase::Telefono);
    }
    if n.contains("direccion") || n.contains("domicilio") {
        return Some(Clase::Direccion);
    }
    if protegido(s) {
        return None;
    }
    if n.contains("json") {
        return Some(Clase::Json);
    }
    if palabras.windows(2).any(|w| {
        ["causa", "ingreso", "persona", "centro", "programa", "entrada", "registro", "consulta", "evento", "documento"].contains(&w[0])
            && ["rus", "sitfa", "id"].contains(&w[1])
    }) || s.to_lowercase().ends_with("_id")
        || s.to_lowercase().starts_with("id_")
        || palabras.iter().any(|p| matches!(*p, "id" | "identificador" | "folio" | "correlativo"))
    {
        return Some(Clase::Identificador);
    }
    if VOCAB["SENSITIVE"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str().is_some_and(|v| format!(" {n}").contains(&format!(" {}", normal(v)))))
    {
        return Some(Clase::Persona);
    }
    if prefijos("FREE_TEXT", &n) {
        return Some(Clase::Texto);
    }
    None
}
pub fn dv(n: &str) -> char {
    let suma = n.chars().rev().zip((2..=7).cycle()).map(|(c, f)| c.to_digit(10).unwrap_or(0) * f).sum::<u32>();
    match 11 - suma % 11 {
        11 => '0',
        10 => 'K',
        v => char::from_digit(v, 10).unwrap(),
    }
}

#[derive(Default)]
struct Nodo {
    hijos: BTreeMap<String, Nodo>,
    valor: Option<String>,
}
impl Nodo {
    fn insertar(&mut self, clave: &str, valor: &str) {
        let mut nodo = self;
        for palabra in clave.split_whitespace() {
            nodo = nodo.hijos.entry(palabra.into()).or_default();
        }
        nodo.valor = Some(valor.into());
    }
}
pub struct Sesion {
    pub opciones: Opciones,
    secreto: Vec<u8>,
    valores: BTreeMap<(String, String), String>,
    contadores: BTreeMap<String, u64>,
    usados: BTreeSet<(String, u64)>,
    nombres: BTreeMap<String, String>,
    partes: BTreeMap<String, String>,
    reservados: BTreeSet<String>,
    rits_usados: BTreeSet<String>,
    indices: Option<(Nodo, Nodo)>,
    pub avisos: BTreeSet<String>,
    pub(crate) rutas: BTreeMap<String, String>,
    fallo: bool,
}
impl Sesion {
    pub fn nueva(opciones: Opciones) -> Result<Self, String> {
        if opciones.semilla.len() > 4096 {
            return Err("La semilla supera el tamaño admitido.".into());
        }
        if opciones.desplazar_dias.unsigned_abs() > 36500 {
            return Err("El desplazamiento excede cien años.".into());
        }
        let mut secreto = vec![0u8; 32];
        getrandom::fill(&mut secreto).map_err(|_| "No se pudo iniciar la anonimización local.")?;
        if !opciones.semilla.is_empty() {
            secreto = opciones.semilla.as_bytes().to_vec();
        }
        let mut avisos = BTreeSet::new();
        if opciones.modo == Modo::Conservador {
            avisos.insert("Modo Conservador: revisa todo texto conservado antes de compartir; puede contener datos indirectos o nombres no reconocidos.".into());
        }
        Ok(Self {
            opciones,
            secreto,
            valores: BTreeMap::new(),
            contadores: BTreeMap::new(),
            usados: BTreeSet::new(),
            nombres: BTreeMap::new(),
            partes: BTreeMap::new(),
            reservados: BTreeSet::new(),
            rits_usados: BTreeSet::new(),
            avisos,
            fallo: false,
            indices: None,
            rutas: BTreeMap::new(),
        })
    }
    fn numero(&mut self, tipo: &str, key: &str, modulo: u64) -> u64 {
        let mut numero = if self.opciones.semilla.is_empty() {
            let n = self.contadores.entry(tipo.into()).or_default();
            *n += 1;
            *n
        } else {
            let mut mac = Hmac::<Sha256>::new_from_slice(&self.secreto).unwrap();
            mac.update(format!("{tipo}|{key}").as_bytes());
            mac.finalize().into_bytes()[..6].iter().fold(0u64, |a, b| a * 256 + *b as u64) % modulo + 1
        };
        while self.usados.contains(&(tipo.into(), numero)) {
            numero = numero % modulo + 1;
        }
        self.usados.insert((tipo.into(), numero));
        numero
    }
    pub fn pseudo(&mut self, tipo: &str, original: &str) -> String {
        if self.valores.len() > 200_000 {
            self.fallo = true;
            return "[identidad omitida]".into();
        }
        let canon = match tipo {
            "persona" | "direccion" => normal(original),
            "rit" => RIT_LIBRE
                .captures(original)
                .map(|c| {
                    format!("{}-{}-{}", c.get(1).map(|m| m.as_str()).unwrap_or("").to_uppercase(), c[2].parse::<u64>().unwrap_or(0), &c[3])
                })
                .unwrap_or_else(|| normal(original)),
            "telefono" => {
                let n = original.chars().filter(char::is_ascii_digit).collect::<String>();
                if n.len() == 11 && n.starts_with("56") {
                    n[2..].into()
                } else {
                    n
                }
            }
            "rut" => original.chars().filter(|c| c.is_ascii_digit() || matches!(c, 'k' | 'K')).collect::<String>().to_uppercase(),
            _ => original.trim().to_uppercase(),
        };
        let key = (tipo.to_string(), canon.clone());
        if let Some(v) = self.valores.get(&key) {
            return v.clone();
        }
        let value = match tipo {
            "rut" => {
                let base = (90_000_000 + self.numero(tipo, &canon, 9_000_000)).to_string();
                let dig = if dv(&base) == '1' { '2' } else { '1' };
                format!("{base}-{dig}")
            }
            "rit" => {
                let letra =
                    RIT_LIBRE.captures(original).and_then(|c| c.get(1).map(|m| m.as_str().to_uppercase())).unwrap_or_else(|| "X".into());
                let mut n = self.numero(tipo, &canon, 999_999);
                let mut salida = format!("{letra}-{n}-2099");
                while self.reservados.contains(&salida) || self.rits_usados.contains(&salida) {
                    n = n % 999_999 + 1;
                    salida = format!("{letra}-{n}-2099");
                }
                self.rits_usados.insert(salida.clone());
                if self.opciones.semilla.is_empty() {
                    self.contadores.insert("rit".into(), n);
                }
                salida
            }
            "mail" => format!("contacto{}@example.invalid", self.numero(tipo, &canon, 1_000_000)),
            "telefono" => format!("9{:08}", self.numero(tipo, &canon, 99_999_999)),
            "direccion" => format!("Calle Ficticia {}", self.numero(tipo, &canon, 1_000_000)),
            "numero" => format!("ID-{}", self.numero(tipo, &canon, 100_000_000)),
            "hash" => {
                let mut mac = Hmac::<Sha256>::new_from_slice(&self.secreto).unwrap();
                mac.update(canon.as_bytes());
                mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
            }
            t if t.starts_with("tok:") => {
                format!("{}_anon_{}", t[4..].chars().take(3).collect::<String>(), self.numero(tipo, &canon, 100_000_000))
            }
            _ => format!("Persona Ficticia {}", self.numero("persona", &canon, 1_000_000)),
        };
        self.valores.insert(key, value.clone());
        value
    }
    pub fn reservar(&mut self, s: &str) {
        for c in RIT.captures_iter(s) {
            if self.reservados.len() > 100_000 {
                self.fallo = true;
                break;
            }
            self.reservados.insert(format!("{}-{}-{}", c[1].to_uppercase(), c[2].parse::<u64>().unwrap_or(0), &c[3]));
        }
    }
    pub fn persona(&mut self, s: &str) {
        if s.len() > 512 || s.split_whitespace().count() > 64 {
            self.fallo = true;
            return;
        }
        let n = normal(s);
        if s.chars().any(|c| c.is_alphabetic() && !ascii(&c.to_string()).chars().any(|a| a.is_ascii_alphabetic())) {
            self.fallo = true;
            return;
        }
        if en("PLACEHOLDERS", &n) {
            return;
        }
        if n.chars().filter(|c| c.is_ascii_alphabetic()).count() < 3 && s.chars().any(char::is_alphabetic) {
            self.fallo = true;
            return;
        }
        if n.len() < 3 || !n.chars().any(|c| c.is_ascii_alphabetic()) {
            return;
        }
        if self.nombres.contains_key(&n) || self.fallo {
            return;
        }
        self.indices = None;
        let fake = self.pseudo("persona", s);
        let numero = fake.rsplit(' ').next().unwrap().to_string();
        self.nombres.entry(n.clone()).or_insert(fake);
        for (i, p) in n.split_whitespace().enumerate().filter(|(_, p)| p.len() >= 3 && !en("STOPWORDS", p)) {
            self.partes.entry(p.into()).or_insert_with(|| format!("{}{numero}", if i == 0 { "Nombre" } else { "Apellido" }));
        }
    }
    fn nombres_en(&mut self, s: &str, solo_completos: bool) -> String {
        if self.indices.is_none() {
            let (mut completos, mut todos) = (Nodo::default(), Nodo::default());
            for (k, v) in &self.partes {
                todos.insertar(k, v);
            }
            for (k, v) in &self.nombres {
                completos.insertar(k, v);
                todos.insertar(k, v);
            }
            self.indices = Some((completos, todos));
        }
        let indice = if solo_completos { &self.indices.as_ref().unwrap().0 } else { &self.indices.as_ref().unwrap().1 };
        let tokens = PALABRAS.find_iter(s).collect::<Vec<_>>();
        let normalizados = tokens.iter().map(|m| normal(m.as_str())).collect::<Vec<_>>();
        let (mut out, mut pos, mut i) = (String::new(), 0, 0);
        while i < tokens.len() {
            let mut nodo = indice;
            let mut mejor = None;
            for j in i..tokens.len() {
                if j > i && !s[tokens[j - 1].end()..tokens[j].start()].chars().all(|c| c.is_whitespace() || "-.,".contains(c)) {
                    break;
                }
                let Some(siguiente) = nodo.hijos.get(&normalizados[j]) else {
                    break;
                };
                nodo = siguiente;
                if let Some(v) = &nodo.valor {
                    mejor = Some((j, v));
                }
            }
            if let Some((j, fake)) = mejor {
                out.push_str(&s[pos..tokens[i].start()]);
                out.push_str(fake);
                pos = tokens[j].end();
                i = j + 1;
            } else {
                i += 1;
            }
        }
        out.push_str(&s[pos..]);
        out
    }
    pub fn desplazar(&self, s: &str, nacimiento: bool) -> String {
        let mover = |d: NaiveDate| {
            if nacimiento && self.opciones.generalizar_nacimiento {
                NaiveDate::from_ymd_opt(d.year(), 1, 1)
            } else {
                d.checked_add_signed(Duration::days(self.opciones.desplazar_dias))
            }
        };
        let s = FECHA_ISO.replace_all(s, |c: &Captures| {
            let m = c.get(0).unwrap();
            if s[..m.start()].chars().next_back().is_some_and(|c| c.is_ascii_digit())
                || s[m.end()..].chars().next().is_some_and(|c| c.is_ascii_digit())
            {
                return c[0].to_string();
            }
            NaiveDate::parse_from_str(&c[0], "%Y-%m-%d").ok().and_then(mover).map(|d| d.to_string()).unwrap_or(c[0].into())
        });
        FECHA_DMY
            .replace_all(&s, |c: &Captures| {
                let m = c.get(0).unwrap();
                if s[..m.start()].chars().next_back().is_some_and(|c| c.is_ascii_digit())
                    || s[m.end()..].chars().next().is_some_and(|c| c.is_ascii_digit())
                    || c[2] != c[4]
                {
                    return c[0].to_string();
                }
                NaiveDate::from_ymd_opt(c[5].parse().unwrap_or(0), c[3].parse().unwrap_or(0), c[1].parse().unwrap_or(0))
                    .and_then(mover)
                    .map(|d| format!("{:02}{}{:02}{}{}", d.day(), &c[2], d.month(), &c[2], d.year()))
                    .unwrap_or(c[0].into())
            })
            .into_owned()
    }
    pub fn texto(&mut self, s: &str, libre: bool, solo_completos: bool) -> Result<String, String> {
        if s.contains(['\u{e000}', '\u{e001}']) {
            return Err("El texto contiene marcadores internos no admitidos.".into());
        }
        if libre && self.opciones.modo == Modo::Estricto && !s.trim().is_empty() {
            return Ok(OMITIDO.into());
        }
        let mut stash = vec![];
        let mut put = |v: String| {
            let indice = stash.len();
            stash.push(v);
            format!("\u{e000}z{indice}\u{e001}")
        };
        let mut t = MAIL.replace_all(s, |c: &Captures| put(self.pseudo("mail", &c[0]))).into_owned();
        t = URL.replace_all(&t, |_: &Captures| put("[enlace omitido]".into())).into_owned();
        t = RUTA.replace_all(&t, |_: &Captures| put("[ruta omitida]".into())).into_owned();
        t = RUT.replace_all(&t, |c: &Captures| put(self.pseudo("rut", &c[0]))).into_owned();
        t = RIT.replace_all(&t, |c: &Captures| put(self.pseudo("rit", &c[0]))).into_owned();
        if libre {
            t = RIT_LIBRE
                .replace_all(&t, |c: &Captures| {
                    let letra = c.get(1).map(|m| m.as_str()).unwrap_or("");
                    if letra.is_empty() && c[2].parse::<u64>().unwrap_or(0) <= 12
                        || letra.chars().next().is_some_and(char::is_lowercase) && c[0].chars().nth(1).is_some_and(char::is_whitespace)
                    {
                        return c[0].to_string();
                    }
                    put(self.pseudo("rit", &c[0]))
                })
                .into_owned();
        }
        t = TELEFONO.replace_all(&t, |c: &Captures| put(self.pseudo("telefono", &c[0]))).into_owned();
        t = DIRECCION.replace_all(&t, |c: &Captures| put(self.pseudo("direccion", &c[0]))).into_owned();
        t = DIGITOS.replace_all(&t, |c: &Captures| put(self.pseudo("numero", &c[0]))).into_owned();
        t = self.nombres_en(&t, solo_completos);
        if libre {
            t = ROL
                .replace_all(&t, |c: &Captures| format!("{}{}", &c[1], if institucional(&c[2]) { &c[2] } else { "[identidad omitida]" }))
                .into_owned();
            t = CANDIDATO
                .replace_all(&t, |c: &Captures| if institucional(&c[0]) { c[0].to_string() } else { "[identidad omitida]".into() })
                .into_owned();
        }
        for (i, valor) in stash.iter().enumerate() {
            t = t.replace(&format!("\u{e000}z{i}\u{e001}"), valor);
        }
        Ok(t)
    }
    pub fn token(&mut self, clave: &str, s: &str) -> String {
        if s.is_empty() {
            return String::new();
        }
        let key = if clave == "parent_event_id" { "event_id" } else { clave };
        if matches!(key, "sha256" | "source_sha256") {
            return self.pseudo("hash", s);
        }
        if matches!(key, "original_file" | "local_file") {
            if let Some(n) = self.rutas.get(s) {
                return n.clone();
            }
            let base = s.rsplit(['/', '\\']).next().unwrap_or(s);
            if let Some(n) = self.rutas.get(base) {
                return n.clone();
            }
        }
        let token = self.pseudo(&format!("tok:{key}"), s);
        if key == "original_sheet" {
            return format!("Hoja{}", token.rsplit('_').next().unwrap_or("1"));
        }
        if key == "original_file" {
            let ext = std::path::Path::new(s)
                .extension()
                .and_then(|s| s.to_str())
                .filter(|s| ["xls", "xlsx", "html", "csv", "txt", "json", "docx", "ics"].contains(s))
                .unwrap_or("txt");
            return format!("fuente_{}.{ext}", token.rsplit('_').next().unwrap_or("1"));
        }
        token
    }
    pub fn valor(&mut self, clave: &str, s: &str) -> Result<String, String> {
        if s.is_empty() {
            return Ok(s.into());
        }
        if credencial(clave) {
            return Ok("[credencial omitida]".into());
        }
        if [
            "path",
            "filename",
            "archivo",
            "file",
            "local_file",
            "original_file",
            "source",
            "origen",
            "ruta",
            "folder",
            "carpeta",
            "directorio",
            "directory",
            "destino",
            "target",
        ]
        .contains(&clave.to_lowercase().as_str())
            && (s.contains(['/', '\\']) || s.contains('.') || self.rutas.contains_key(s))
        {
            return Ok(self.token("original_file", s));
        }
        let n = normal(s);
        if en("PLACEHOLDERS", &n) && (!n.is_empty() || !s.chars().any(char::is_alphabetic)) {
            return Ok(s.into());
        }
        let tipo = clase(clave);
        let resultado = match tipo {
            Some(Clase::Rit) => self.pseudo("rit", s),
            Some(Clase::Rut) => self.pseudo("rut", s),
            Some(Clase::Correo) => self.pseudo("mail", s),
            Some(Clase::Telefono) => self.pseudo("telefono", s),
            Some(Clase::Direccion) => self.pseudo("direccion", s),
            Some(Clase::Identificador) => self.token(&clave.to_lowercase(), s),
            Some(Clase::Huella) => self.pseudo("hash", s),
            Some(Clase::Persona | Clase::Curador) => {
                let nuevo = self.texto(s, false, false)?;
                if nuevo == s && !institucional(s) {
                    self.pseudo("persona", s)
                } else {
                    nuevo
                }
            }
            Some(Clase::Texto) => self.texto(s, true, false)?,
            Some(Clase::Tecnico) => self.token(VOCAB["TECHNICAL"][normal(clave)].as_str().unwrap_or("id"), s),
            _ if protegido(clave) || en("SAFE_EXTRA_KEYS", &normal(clave)) => self.texto(s, false, true)?,
            _ if estructural(s) => s.to_string(),
            _ => self.texto(s, true, false)?,
        };
        Ok(self.desplazar(&resultado, normal(clave).contains("nacimiento")))
    }
    pub fn verificar(&mut self, textos: &[(String, bool)]) -> Result<(), String> {
        if self.fallo {
            return Err("Identidades fuera de los límites admitidos; no se publica la copia.".into());
        }
        let fakes = |tipo: &str| self.valores.iter().filter(|((k, _), _)| k == tipo).map(|(_, v)| v.clone()).collect::<BTreeSet<_>>();
        let (ruts, rits, telefonos) = (fakes("rut"), fakes("rit"), fakes("telefono"));
        static FALSOS: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"Persona Ficticia \d+|(?:Nombre|Apellido)\d+|\b[0-9a-f]{24,64}\b").unwrap());
        for (s, protegido) in textos {
            let t = FALSOS.replace_all(s, "").into_owned();
            if RIT.find_iter(&t).any(|m| !rits.contains(m.as_str())) {
                return Err("La revisión final detectó un RIT; no se publica la copia.".into());
            }
            let t = RIT.replace_all(&t, " ").into_owned();
            if self.nombres_en(&t, *protegido) != t {
                return Err("La revisión final detectó un nombre; no se publica la copia.".into());
            }
            if RUT.captures_iter(&t).any(|c| {
                let n = c[1].chars().filter(char::is_ascii_digit).collect::<String>();
                dv(&n) == c[3].chars().next().unwrap().to_ascii_uppercase() && !ruts.contains(&format!("{n}-{}", c[3].to_uppercase()))
            }) {
                return Err("La revisión final detectó un RUT válido; no se publica la copia.".into());
            }
            if MAIL.find_iter(&t).any(|m| !m.as_str().ends_with("@example.invalid"))
                || TELEFONO.find_iter(&t).any(|m| !telefonos.contains(m.as_str()))
                || RUTA.is_match(&t)
            {
                return Err("La revisión final detectó un identificador; no se publica la copia.".into());
            }
        }
        Ok(())
    }
}
