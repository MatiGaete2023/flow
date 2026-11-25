//! Productos preparados (correos y proyectos): huella, vigencia y recibos (`product_state.py`, `activity.py` y
//! `outputs.draft_fingerprint` de Flow Python). Un producto sabe de qué registros y configuración depende; si eso cambia
//! queda desactualizado y no se guarda. Un borrador con el mismo contenido que otro ya guardado no se guarda dos veces.

use crate::config::{Config, CC_OBLIGATORIO};
use crate::productos::{Borrador, Proyecto};
use crate::trabajo::Trabajo;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const OBSOLETO: &str =
    "Este producto necesita actualizarse porque cambiaron sus registros o configuración. Prepáralo de nuevo y revisa los cambios antes de guardar.";

/// `activity.receipt`: lo que se creó, cuándo y a partir de qué registros; nunca infiere envíos.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recibo {
    pub tipo: String,
    pub estado: String,
    pub creado: String,
    pub registros: Vec<String>,
    pub huella: String,
    pub ruta: String,
}

/// Lo que Trabajo recuerda de los productos para recuperarlo al reabrir la sesión.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Preparados {
    pub borradores: Vec<Borrador>,
    pub proyectos: Vec<Proyecto>,
    pub recibos: BTreeMap<String, Recibo>,
}

fn sha(v: &serde_json::Value) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_string(v).unwrap_or_default().as_bytes()))
}

/// `draft_fingerprint`: identidad del borrador realmente revisado (destinatarios, texto y adjunto), sin rutas temporales.
pub fn huella_borrador(b: &Borrador) -> String {
    let cc = {
        let c = b.cc.to_lowercase();
        if c.contains(CC_OBLIGATORIO) { b.cc.clone() } else { format!("{CC_OBLIGATORIO}; {}", b.cc) }
    };
    let adjunto = b.nomina.as_ref().map(|n| {
        let filas: Vec<&Vec<String>> = n.rows.iter().zip(&n.incluir).filter(|(_, i)| **i).map(|(r, _)| r).collect();
        json!({"encabezados": n.headers, "filas": filas})
    });
    sha(&json!({
        "para": b.para.split(';').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>(),
        "cc": cc.split(';').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect::<std::collections::BTreeSet<_>>(),
        "asunto": b.asunto,
        "cuerpo": b.cuerpo,
        "adjunto": adjunto,
        "obligatorio": b.adjunto_obligatorio,
        "archivos": b.adjuntos.iter().map(|(r, h)| (std::path::Path::new(r).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), h.clone())).collect::<std::collections::BTreeSet<_>>(),
    }))
}

/// Huella de lo que el producto lee: sus registros (texto vigente, RES, exclusión, campos y valores de origen), la
/// configuración que alimenta correos y proyectos y la captura. No incluye el texto editable del producto.
pub fn dependencia(w: &Trabajo, cfg: &Config, registros: &[usize], tipo: &str) -> String {
    let filas: Vec<serde_json::Value> = registros
        .iter()
        .map(|&i| match w.registros.get(i) {
            Some(r) => json!({
                "fila": r.fila_excel,
                "observacion": r.observacion(),
                "res": r.res,
                "excluido": r.excluido,
                "campos": r.campos,
                "anulaciones": r.anulaciones,
                "valores": r.values.iter().map(|c| c.display()).collect::<Vec<_>>(),
            }),
            None => json!({"fila": i, "ausente": true}),
        })
        .collect();
    sha(&json!({
        "tipo": tipo,
        "registros": filas,
        "config": {
            "plantillas": cfg.plantillas,
            "contactos": cfg.contactos,
            "tribunales": cfg.tribunales,
            "firma": cfg.firma,
            "cc_adicional": cfg.cc_adicional,
            "periodo": cfg.periodo,
        },
        "captura": w.hash,
        "corte": w.corte.map(|c| c.to_string()),
    }))
}

pub fn sellar_borrador(w: &Trabajo, cfg: &Config, b: &mut Borrador) {
    b.dependencia = dependencia(w, cfg, &b.registros, &b.kind);
}

pub fn sellar_proyecto(w: &Trabajo, cfg: &Config, p: &mut Proyecto) {
    p.dependencia = dependencia(w, cfg, &p.registros, &p.tipo);
}

/// `stale`: sellado y con otra huella ahora. Un producto sin sello (de una sesión antigua) no se declara obsoleto.
pub fn borrador_obsoleto(w: &Trabajo, cfg: &Config, b: &Borrador) -> bool {
    !b.dependencia.is_empty() && b.dependencia != dependencia(w, cfg, &b.registros, &b.kind)
}

pub fn proyecto_obsoleto(w: &Trabajo, cfg: &Config, p: &Proyecto) -> bool {
    !p.dependencia.is_empty() && p.dependencia != dependencia(w, cfg, &p.registros, &p.tipo)
}

/// Identificadores de captura de los registros de un producto (o la fila de Excel si no hay paquete).
pub fn ids_registros(w: &Trabajo, registros: &[usize]) -> Vec<String> {
    registros
        .iter()
        .filter_map(|&i| w.registros.get(i))
        .map(|r| w.id_registro(r).unwrap_or_else(|| r.fila_excel.to_string()))
        .collect()
}

impl Preparados {
    /// Recibo creado de un borrador con esta huella, si lo hay.
    pub fn guardado(&self, huella: &str) -> Option<&Recibo> {
        self.recibos.values().find(|r| r.tipo == "borrador" && r.estado == "creado" && r.huella == huella)
    }

    /// Recibo incierto de un borrador con esta huella: Outlook pudo haberlo guardado sin confirmarlo.
    pub fn incierto(&self, huella: &str) -> bool {
        self.recibos.values().any(|r| r.tipo == "borrador" && r.estado == "incierto" && r.huella == huella)
    }

    /// La persona revisó Borradores de Outlook: el borrador incierto deja de bloquear un nuevo guardado.
    pub fn conciliar_incierto(&mut self, huella: &str) -> usize {
        let antes = self.recibos.len();
        self.recibos.retain(|_, r| !(r.tipo == "borrador" && r.estado == "incierto" && r.huella == huella));
        antes - self.recibos.len()
    }

    pub fn anotar(&mut self, tipo: &str, estado: &str, registros: Vec<String>, huella: &str, ruta: &str) {
        let clave = format!("{:x}", Sha256::digest(format!("{tipo}|{huella}|{ruta}|{}", self.recibos.len()).as_bytes()));
        self.recibos.insert(
            clave,
            Recibo {
                tipo: tipo.into(),
                estado: estado.into(),
                creado: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
                registros,
                huella: huella.into(),
                ruta: ruta.into(),
            },
        );
    }
}
