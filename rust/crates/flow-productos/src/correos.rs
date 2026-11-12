//! Borradores de correo: a programas (con nómina) y a tribunales. Nunca se envían.
//! El CC fijo va en todos y no se puede quitar.

use crate::excel::{nomina, ruta_nueva, valor};
use flow_dominio::texto::normalizar_columna;
use flow_dominio::{Etapa, Tribunal, CC_FIJO};
use flow_trabajo::{FilaTrabajo, Trabajo};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

fn base() -> Base {
    serde_json::from_str(include_str!("../../../datos/correos.json")).expect("correos.json")
}

/// Modalidades que se ofrecen por defecto para el alcance de los correos al tribunal.
pub fn modalidades_base() -> Vec<String> {
    base().modalidades
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
}

impl Tipo {
    pub const TODOS: [Tipo; 5] =
        [Tipo::ProgramaEspera, Tipo::ProgramaVencido, Tipo::ProgramaPorVencer, Tipo::Medidas, Tipo::Informativo];

    /// Acción del motor que alimenta este correo; el informativo no depende de una acción.
    pub fn accion(self) -> Option<&'static str> {
        match self {
            Tipo::ProgramaEspera => Some("programa_espera"),
            Tipo::ProgramaVencido => Some("programa_vencido"),
            Tipo::ProgramaPorVencer => Some("programa_por_vencer"),
            Tipo::Medidas => Some("medidas"),
            Tipo::Informativo => None,
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
}

impl Contactos {
    pub fn de_programa(&self, programa: &str) -> String {
        let clave = normalizar_columna(programa.trim());
        let mut halladas: Vec<&String> = self
            .programas
            .iter()
            .filter(|(k, v)| normalizar_columna(k.trim()) == clave && !v.trim().is_empty())
            .map(|(_, v)| v)
            .collect();
        halladas.dedup();
        if halladas.len() == 1 {
            halladas[0].clone()
        } else {
            String::new()
        }
    }
    pub fn cargar(ruta: &Path) -> Contactos {
        std::fs::read_to_string(ruta).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }
    pub fn guardar(&self, ruta: &Path) -> Result<(), String> {
        if let Some(carpeta) = ruta.parent() {
            std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
        }
        let texto = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(ruta, texto).map_err(|e| format!("No se pudieron guardar los contactos: {e}"))
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
    let base = base();
    let clave_plantilla = tipo.plantilla(trabajo.etapa);
    let plantilla = &base.plantillas[clave_plantilla];
    if plantilla.usa_modalidades && opciones.modalidades.trim().is_empty() {
        return Err("Indica las modalidades efectivamente comprendidas.".into());
    }
    let hoy = opciones.hoy.unwrap_or(trabajo.corte).format("%d/%m/%Y").to_string();
    let mut grupos: BTreeMap<(Tribunal, String), Vec<&FilaTrabajo>> = BTreeMap::new();
    let mut orden: Vec<(Tribunal, String)> = Vec::new();
    for fila in &trabajo.filas {
        if fila.propuesta.excluido || tipo.accion().is_some_and(|a| !fila.tiene_accion(a)) {
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
        let contexto = [
            ("TRIBUNAL", nombre_tribunal.as_str()),
            ("PROGRAMA", programa.as_str()),
            ("FECHA", hoy.as_str()),
            ("ALCANCE_MODALIDADES", opciones.modalidades.as_str()),
        ];
        let mut cuerpo = rellenar(&plantilla.cuerpo, &contexto)?;
        if !contactos.firma.trim().is_empty() {
            cuerpo = format!("{cuerpo}\n\n{}", contactos.firma.trim());
        }
        let asunto = rellenar(&plantilla.asunto, &contexto)?;
        let mut adjuntos = Vec::new();
        if plantilla.adjunto == "obligatorio" {
            std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
            let nombre = if tipo.a_programa() {
                format!("Nómina {} - {}", programa, tribunal.titulo())
            } else {
                format!("Nómina medidas - {}", tribunal.titulo())
            };
            let ruta = ruta_nueva(carpeta, &nombre, "xlsx");
            nomina(trabajo, filas, clave_plantilla, &ruta)?;
            adjuntos.push(ruta);
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
        });
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
