//! Anonimizador local: copias con identidades ficticias. Nunca escribe sobre el original ni usa la red.
//! Tablas (.xlsx, .xls, HTML/XML de SITFA, .csv) y texto (.txt, .md, .log, .json).

use crate::lectura::{self, Sheet};
use crate::util::*;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modo {
    Estricto,
    Conservador,
}

#[derive(Clone, Debug)]
pub struct Opciones {
    pub modo: Modo,
    pub semilla: String,
    pub desplazar_dias: i64,
    pub generalizar_nacimiento: bool,
}

impl Default for Opciones {
    fn default() -> Self {
        Opciones { modo: Modo::Estricto, semilla: String::new(), desplazar_dias: 0, generalizar_nacimiento: false }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Informe {
    pub creados: Vec<PathBuf>,
    pub errores: Vec<String>,
    pub advertencias: Vec<String>,
    pub reemplazos: BTreeMap<&'static str, usize>,
}

const SENSIBLES: &[&str] = &[
    "nombre", "apellido", "nna", "menor", "nino", "nina", "joven", "usuario", "beneficiario", "victima", "madre", "padre",
    "litigante", "abogado", "apoderado", "tutor", "hermano", "familiar", "responsable", "profesional", "dupla",
    "contacto", "referente", "cuidador", "adulto", "educador", "psicolog", "trabajador", "asistente", "denunciante",
    "imputado", "demandante", "demandado", "solicitante", "interviniente", "defensor", "fiscal", "juez", "magistrado",
    "consejero", "funcionario", "digitador", "encargado", "supervisor", "coordinador", "autor", "paciente", "hijo",
    "hija", "adolescente", "conyuge", "pareja", "acusado", "querellante", "sujeto", "titular", "representante",
    "abuelo", "abuela", "padrastro", "madrastra", "pariente", "testigo", "perito", "terapeuta", "interventor",
    "delegado", "gestor", "tramitador", "firmante", "remitente", "destinatario", "creado por", "modificado por",
    "ingresado por", "curador",
];
const CONSERVAR: &[&str] = &["nombre centro", "nombre programa", "nombre tribunal", "nombre proyecto", "nombre institucion"];
const TEXTO_LIBRE: &[&str] = &[
    "observacion", "obs", "glosa", "detalle", "texto", "comentario", "nota", "motivo", "respuesta", "descripcion",
    "mensaje", "referencia", "error", "antecedente", "resumen", "fundamento",
];
/// Columnas que lee el motor de reglas y no son identidad: se conservan.
const MOTOR: &[&str] = &[
    "derivacion", "programa", "tribunal", "fec nacimiento", "fecha nacimiento", "fec oido", "fecoido", "fec resolucion",
    "fecha resolucion", "proxs auds", "prox aud", "proxima audiencia", "t espera", "t_espera", "dias de espera",
    "dias de cumplimiento", "dias para egresar", "fec ingreso efectivo", "fec egreso proyectado", "fec act f fae",
    "fec act f individual", "fecha vencimiento", "fec vencimiento", "f vencimiento", "duracion", "etapa", "modalidad",
    "estado", "tipo", "res", "tt", "cc", "fecha", "fecha obs",
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Clase {
    Rut,
    Rit,
    Persona,
    Curador,
    Libre,
    Motor,
    Desconocida,
}

fn clasificar(h: &str) -> Clase {
    let n = normalizar(h).replace(['/', '_'], " ");
    let n = n.split_whitespace().collect::<Vec<_>>().join(" ");
    if n.is_empty() {
        return Clase::Desconocida;
    }
    if n.contains("curador") {
        return Clase::Curador;
    }
    if n == "rut" || n.starts_with("rut ") || n.ends_with(" rut") || n.contains("run ") || n == "run" {
        return Clase::Rut;
    }
    if n == "rit" || n.starts_with("rit ") || n == "ruc" || n.contains("causa") && !n.contains("estado") {
        return Clase::Rit;
    }
    if CONSERVAR.iter().any(|k| n.contains(k)) || MOTOR.iter().any(|k| n == *k || n.starts_with(k)) {
        return Clase::Motor;
    }
    if SENSIBLES.iter().any(|k| n.contains(k)) {
        return Clase::Persona;
    }
    if TEXTO_LIBRE.iter().any(|k| n.split(' ').any(|w| w.starts_with(k))) {
        return Clase::Libre;
    }
    Clase::Desconocida
}

fn re(c: &'static OnceLock<Regex>, p: &str) -> &'static Regex {
    c.get_or_init(|| Regex::new(p).unwrap())
}
fn rut_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"\b(\d{1,2}(?:\.\d{3}){2}|\d{6,8})\s*[-‐–]\s*([\dkK])\b")
}
fn rit_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"\b([A-Za-z])\s*[-‐–]\s*(\d{1,6})\s*[-‐–/]\s*((?:19|20)\d{2})\b")
}
fn mail_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+")
}
fn phone_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"(?:\+?56\s?)?(?:9\s?\d{4}\s?\d{4}|\(?\d{2}\)?\s?\d{3}\s?\d{4})\b")
}
fn path_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"(?i)([A-Z]:\\Users\\|/home/|/Users/)([^\\/\s]+)")
}
fn long_digits_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"\b\d{7,12}\b")
}

pub fn digito_verificador(numero: &str) -> char {
    let mut s = 0u32;
    let mut m = 2;
    for c in numero.chars().rev().filter(|c| c.is_ascii_digit()) {
        s += c.to_digit(10).unwrap() * m;
        m = if m == 7 { 2 } else { m + 1 };
    }
    match 11 - (s % 11) {
        11 => '0',
        10 => 'K',
        d => char::from_digit(d, 10).unwrap(),
    }
}

/// Seudónimos estables dentro de una ejecución (o entre ejecuciones con la misma semilla).
pub struct Sesion {
    semilla: String,
    mapas: HashMap<(&'static str, String), String>,
    contadores: HashMap<&'static str, usize>,
    nombres: Vec<(String, String)>,
    pub cuenta: BTreeMap<&'static str, usize>,
}

impl Sesion {
    pub fn new(semilla: &str) -> Self {
        Sesion { semilla: semilla.into(), mapas: HashMap::new(), contadores: HashMap::new(), nombres: vec![], cuenta: BTreeMap::new() }
    }

    fn numero(&mut self, kind: &'static str, original: &str, rango: u64) -> u64 {
        if self.semilla.is_empty() {
            let c = self.contadores.entry(kind).or_insert(0);
            *c += 1;
            *c as u64
        } else {
            let h = Sha256::digest(format!("{}|{kind}|{original}", self.semilla).as_bytes());
            u64::from_be_bytes(h[..8].try_into().unwrap()) % rango + 1
        }
    }

    fn get(&mut self, kind: &'static str, original: &str) -> String {
        let key = (kind, clave(original));
        if let Some(v) = self.mapas.get(&key) {
            return v.clone();
        }
        let v = match kind {
            "persona" => format!("Persona Ficticia {}", self.numero(kind, &key.1, 9999)),
            "rut" => {
                let n = 30_000_000 + self.numero(kind, &key.1, 9_999_999);
                let n = n.to_string();
                let valido = digito_verificador(&n);
                // Dígito verificador deliberadamente inválido: nunca coincide con una persona real.
                let dv = if valido == '0' { '1' } else { '0' };
                format!("{n}-{dv}")
            }
            "rit" => {
                let n = 90_000 + self.numero(kind, &key.1, 9_999);
                let partes: Vec<&str> = original.split(['-', '/']).collect();
                let letra = partes.first().map(|s| s.trim().to_uppercase()).filter(|s| s.len() == 1).unwrap_or_else(|| "X".into());
                let anio = partes.last().map(|s| s.trim().to_string()).filter(|s| s.len() == 4).unwrap_or_else(|| "2099".into());
                format!("{letra}-{n}-{anio}")
            }
            "correo" => format!("persona{}@ejemplo.invalid", self.numero(kind, &key.1, 99999)),
            "telefono" => format!("9{:08}", self.numero(kind, &key.1, 99_999_999)),
            _ => format!("ID-{}", self.numero(kind, &key.1, 999_999)),
        };
        *self.cuenta.entry(kind).or_insert(0) += 1;
        self.mapas.insert(key, v.clone());
        v
    }

    /// Registra un nombre (y cada palabra de más de 2 letras) para reemplazarlo en todo el lote.
    fn registrar_nombre(&mut self, nombre: &str) {
        let limpio = sin_parentesis(nombre);
        if limpio.chars().filter(|c| c.is_alphabetic()).count() < 3 {
            return;
        }
        let fake = self.get("persona", &limpio);
        self.nombres.push((limpio.clone(), fake.clone()));
        for w in limpio.split_whitespace() {
            if w.chars().count() > 2 && w.chars().all(|c| c.is_alphabetic()) {
                self.nombres.push((w.to_string(), "[identidad omitida]".into()));
            }
        }
    }

    fn preparar(&mut self) {
        self.nombres.sort_by_key(|(n, _)| std::cmp::Reverse(n.chars().count()));
        self.nombres.dedup_by(|a, b| clave(&a.0) == clave(&b.0));
    }

    /// Reemplaza RUT, RIT, correos, teléfonos, rutas de usuario, números largos y nombres conocidos.
    pub fn texto(&mut self, s: &str) -> String {
        let mut t = s.to_string();
        t = rut_re().replace_all(&t, |c: &regex::Captures| self.get("rut", &format!("{}-{}", c[1].replace('.', ""), &c[2]))).to_string();
        t = rit_re().replace_all(&t, |c: &regex::Captures| self.get("rit", &format!("{}-{}-{}", &c[1], &c[2], &c[3]))).to_string();
        t = mail_re().replace_all(&t, |c: &regex::Captures| self.get("correo", &c[0])).to_string();
        t = phone_re().replace_all(&t, |c: &regex::Captures| self.get("telefono", &c[0])).to_string();
        t = path_re().replace_all(&t, "${1}usuario").to_string();
        t = long_digits_re().replace_all(&t, |c: &regex::Captures| self.get("id", &c[0])).to_string();
        let nombres = self.nombres.clone();
        for (n, fake) in nombres {
            let pat = format!(r"(?i)\b{}\b", regex::escape(&n));
            if let Ok(r) = Regex::new(&pat) {
                if r.is_match(&t) {
                    *self.cuenta.entry("nombre en texto").or_insert(0) += 1;
                    t = r.replace_all(&t, fake.as_str()).to_string();
                }
            }
        }
        t
    }

    fn curador(&mut self, s: &str) -> String {
        // «(RUT)NOMBRE - Institución: X#…»: RUT ficticio, nombre ficticio, institución conservada.
        static R: OnceLock<Regex> = OnceLock::new();
        let r = re(&R, r"\(([^)]*)\)\s*([^#]*?)(\s*-\s*Instituci[oó]n:[^#]*)?(#|$)");
        if !r.is_match(s) {
            return self.texto(s);
        }
        r.replace_all(s, |c: &regex::Captures| {
            let rut = c[1].to_string();
            let rut_f = if rut_re().is_match(&rut) { self.texto(&rut) } else { rut };
            let nombre = c[2].trim().to_string();
            let nombre_f = if nombre.is_empty() { nombre } else { self.get("persona", &nombre) };
            format!("({rut_f}){nombre_f}{}{}", c.get(3).map(|m| m.as_str()).unwrap_or(""), &c[4])
        })
        .to_string()
    }
}

fn header_row(s: &Sheet) -> usize {
    s.rows
        .iter()
        .take(30)
        .position(|r| r.iter().filter(|c| matches!(c, Cell::Text(t) if !t.trim().is_empty())).count() >= 2)
        .unwrap_or(0)
}

fn es_fecha_nac(h: &str) -> bool {
    normalizar(h).contains("nacimiento")
}

fn tratar_tablas(sheets: &[Sheet], ses: &mut Sesion, o: &Opciones) -> Vec<(String, Vec<String>, Vec<Vec<String>>)> {
    // Primera pasada: nombres de todas las hojas, para reemplazarlos también en textos.
    for s in sheets {
        let hr = header_row(s);
        let heads = s.rows.get(hr).map(|r| lectura::headers_of(r)).unwrap_or_default();
        for row in s.rows.iter().skip(hr + 1) {
            for (i, c) in row.iter().enumerate() {
                if heads.get(i).map(|h| clasificar(h)) == Some(Clase::Persona) {
                    let v = c.display();
                    if !v.trim().is_empty() && !rut_re().is_match(&v) {
                        ses.registrar_nombre(&v);
                    }
                }
            }
        }
    }
    ses.preparar();
    let mut out = Vec::new();
    for s in sheets {
        let hr = header_row(s);
        let heads = s.rows.get(hr).map(|r| lectura::headers_of(r)).unwrap_or_default();
        let clases: Vec<Clase> = heads.iter().map(|h| clasificar(h)).collect();
        let mut rows = Vec::new();
        for row in s.rows.iter().skip(hr + 1) {
            if row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let mut out_row = Vec::new();
            for (i, c) in row.iter().enumerate() {
                let clase = clases.get(i).copied().unwrap_or(Clase::Desconocida);
                let v = c.display();
                let nuevo = if c.is_empty() {
                    v
                } else {
                    match clase {
                        Clase::Rut => ses.get("rut", &v),
                        Clase::Rit => ses.get("rit", &v),
                        Clase::Persona => {
                            if rut_re().is_match(&v) { ses.texto(&v) } else { ses.get("persona", &sin_parentesis(&v)) }
                        }
                        Clase::Curador => ses.curador(&v),
                        Clase::Motor => match (c, o.desplazar_dias) {
                            (_, 0) if !(o.generalizar_nacimiento && heads.get(i).is_some_and(|h| es_fecha_nac(h))) => v,
                            _ => desplazar(c, o, heads.get(i).is_some_and(|h| es_fecha_nac(h))),
                        },
                        Clase::Libre | Clase::Desconocida => match c {
                            Cell::Number(_) | Cell::Date(_) | Cell::Bool(_) => {
                                if o.desplazar_dias != 0 && matches!(c, Cell::Date(_)) { desplazar(c, o, false) } else { v }
                            }
                            _ if parse_date_text(&v).is_some() && clase == Clase::Desconocida => desplazar(c, o, false),
                            _ if o.modo == Modo::Estricto && (clase == Clase::Libre || v.chars().any(|ch| ch.is_alphabetic())) => {
                                *ses.cuenta.entry("texto omitido").or_insert(0) += 1;
                                "[texto omitido]".into()
                            }
                            _ => ses.texto(&v),
                        },
                    }
                };
                out_row.push(nuevo);
            }
            rows.push(out_row);
        }
        out.push((s.name.clone(), heads, rows));
    }
    out
}

fn desplazar(c: &Cell, o: &Opciones, nacimiento: bool) -> String {
    let Some(d) = get_date(c) else { return c.display() };
    let d = if nacimiento && o.generalizar_nacimiento {
        chrono::NaiveDate::from_ymd_opt(chrono::Datelike::year(&d), 1, 1).unwrap_or(d)
    } else {
        d + chrono::Duration::days(o.desplazar_dias)
    };
    d.format("%d/%m/%Y").to_string()
}

fn salida(dir: &Path, src: &Path, ext: &str) -> PathBuf {
    let stem = src.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "archivo".into());
    crate::productos::nueva_ruta(dir, &format!("{stem}_anonimizado"), ext)
}

pub fn ejecutar(entradas: &[PathBuf], dir: &Path, o: &Opciones) -> Informe {
    let mut inf = Informe::default();
    let mut ses = Sesion::new(&o.semilla);
    if let Err(e) = std::fs::create_dir_all(dir) {
        inf.errores.push(format!("No se pudo crear la carpeta de salida: {e}"));
        return inf;
    }
    for p in entradas {
        let nombre = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let bytes = match std::fs::read(p) {
            Ok(b) => b,
            Err(e) => {
                inf.errores.push(format!("{nombre}: no se pudo leer ({e})."));
                continue;
            }
        };
        if ["pdf", "png", "jpg", "jpeg", "gif", "exe", "dll", "msg"].contains(&ext.as_str()) {
            inf.errores.push(format!("{nombre}: formato no admitido (PDF, imágenes y ejecutables se rechazan)."));
            continue;
        }
        let res = if ["txt", "md", "log", "json", "jsonl", "ics"].contains(&ext.as_str()) {
            let txt = lectura::decode_text(&bytes);
            // Pares «Nombre: valor» primero, para que su valor también se reemplace en el resto del texto.
            static PAR: OnceLock<Regex> = OnceLock::new();
            let par = re(&PAR, r#"(?im)\b(nombre|nna|madre|padre|curador[a]?|abogad[oa]|litigante|usuario)\s*"?\s*[:=]\s*"?([^\n",;]+)"#);
            for c in par.captures_iter(&txt) {
                ses.registrar_nombre(c[2].trim());
            }
            ses.preparar();
            let out = ses.texto(&txt);
            let dest = salida(dir, p, &ext);
            std::fs::write(&dest, out).map(|_| dest).map_err(|e| e.to_string())
        } else {
            match lectura::read_sheets(&bytes) {
                Ok(sheets) => {
                    let tablas = tratar_tablas(&sheets, &mut ses, o);
                    if ext == "csv" || ext == "tsv" {
                        let sep = if ext == "tsv" { "\t" } else { ";" };
                        let (_, h, rows) = &tablas[0];
                        let mut s = h.join(sep) + "\n";
                        for r in rows {
                            s += &(r.iter().map(|v| v.replace(sep, " ")).collect::<Vec<_>>().join(sep) + "\n");
                        }
                        let dest = salida(dir, p, &ext);
                        std::fs::write(&dest, s).map(|_| dest).map_err(|e| e.to_string())
                    } else {
                        let dest = salida(dir, p, "xlsx");
                        let hojas: Vec<(&str, Vec<String>, Vec<Vec<String>>)> = tablas
                            .iter()
                            .enumerate()
                            .map(|(_i, (n, h, r))| (if n.is_empty() { "Hoja" } else { n.as_str() }, h.clone(), r.clone()))
                            .map(|(n, h, r)| (n, h, r))
                            .collect();
                        let mut nombres_unicos: Vec<String> = Vec::new();
                        let hojas: Vec<(String, Vec<String>, Vec<Vec<String>>)> = hojas
                            .into_iter()
                            .map(|(n, h, r)| {
                                let mut nm: String = safe_filename(n).chars().take(28).collect();
                                while nombres_unicos.contains(&nm) {
                                    nm.push('_');
                                }
                                nombres_unicos.push(nm.clone());
                                (nm, h, r)
                            })
                            .collect();
                        let refs: Vec<(&str, Vec<String>, Vec<Vec<String>>)> = hojas.iter().map(|(n, h, r)| (n.as_str(), h.clone(), r.clone())).collect();
                        crate::productos::escribir_xlsx(&refs, &dest).map(|_| dest)
                    }
                }
                Err(e) => Err(e),
            }
        };
        match res {
            Ok(d) => inf.creados.push(d),
            Err(e) => inf.errores.push(format!("{nombre}: {e}")),
        }
    }
    // Comprobación final: un RUT con dígito válido que quedara sería una persona real.
    for d in &inf.creados {
        if let Ok(b) = std::fs::read(d) {
            if d.extension().is_some_and(|e| e == "xlsx") {
                continue;
            }
            let t = String::from_utf8_lossy(&b);
            for c in rut_re().captures_iter(&t) {
                let num = c[1].replace('.', "");
                if digito_verificador(&num).to_string() == c[2].to_uppercase() {
                    inf.advertencias.push(format!("{}: quedó un RUT con dígito válido; revísalo.", d.display()));
                    break;
                }
            }
        }
    }
    if o.modo == Modo::Conservador {
        inf.advertencias.push("Modo conservador: revisa cada texto conservado antes de compartir la copia.".into());
    }
    inf.reemplazos = ses.cuenta.clone();
    inf
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rut_invalido_y_estable() {
        let mut s = Sesion::new("");
        let a = s.get("rut", "11111111-0");
        let b = s.get("rut", "11111111-0");
        assert_eq!(a, b);
        let (n, dv) = a.split_once('-').unwrap();
        assert_ne!(digito_verificador(n).to_string(), dv);
        let t = s.texto("RUT 12.345.678-0, RIT X-123-2099, correo prueba@example.invalid");
        assert!(!t.contains("12.345.678"), "{t}");
        assert!(!t.contains("C-123-2025"), "{t}");
        assert!(!t.contains("prueba@example.invalid"), "{t}");
    }
}
