//! Vigencia de los productos preparados (`personal/product_state.py`): cada borrador y proyecto guarda la huella de
//! los registros de los que depende. Si esos registros, la configuración o las reglas cambian, queda obsoleto y hay que
//! prepararlo de nuevo antes de guardarlo o generarlo.

use crate::correos::{Borrador, Contactos};
use crate::excel::valor;
use crate::word::Proyecto;
use flow_dominio::texto::normalizar_columna;
use flow_dominio::Tribunal;
use flow_trabajo::{FilaTrabajo, Trabajo};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};

/// Mensaje de `require_fresh`.
pub const OBSOLETO: &str = "Este producto necesita actualizarse porque cambiaron sus registros o plantilla. \
Prepara nuevamente y revisa los cambios antes de guardar.";

fn hex(d: impl AsRef<[u8]>) -> String {
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Huellas de las filas de un trabajo, calculadas una vez para revisar muchos productos (`dependency_scope`).
pub struct Huellas<'a> {
    trabajo: &'a Trabajo,
    filas: HashMap<&'a str, String>,
    /// Reglas vigentes en el programa (`active_ruleset_version`).
    reglas: String,
}

/// Lo que de una fila influye en sus productos: datos de origen, observación, propuesta y exclusión.
fn huella_fila(f: &FilaTrabajo) -> String {
    let datos = serde_json::to_vec(&(
        &f.id,
        &f.observacion,
        &f.propuesta.observacion,
        &f.propuesta.acciones,
        f.propuesta.excluido,
        &f.registro,
        &f.celdas,
        f.proximo_informe,
    ))
    .unwrap_or_default();
    hex(Sha256::digest(datos))
}

fn tribunal(f: &FilaTrabajo) -> String {
    let crudo = valor(f, "tribunal");
    Tribunal::desde(&crudo).map(|t| t.nombre().to_string()).unwrap_or_else(|| normalizar_columna(&crudo))
}

impl<'a> Huellas<'a> {
    pub fn nuevas(trabajo: &'a Trabajo, reglas: &str) -> Self {
        Huellas { trabajo, filas: trabajo.filas.iter().map(|f| (f.id.as_str(), huella_fila(f))).collect(), reglas: reglas.to_string() }
    }

    /// `product_state._fingerprint`: filas de los grupos del producto (también las que se sumaron o se excluyeron
    /// después), configuración, tipo e identificadores.
    fn huella(&self, ids: &[String], grupo: &dyn Fn(&FilaTrabajo) -> String, tipo: &str, config: &str) -> String {
        let propios: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
        let grupos: BTreeSet<String> = self.trabajo.filas.iter().filter(|f| propios.contains(f.id.as_str())).map(grupo).collect();
        let mut registros: Vec<(&str, &str)> = self
            .trabajo
            .filas
            .iter()
            .filter(|f| propios.contains(f.id.as_str()) || grupos.contains(&grupo(f)))
            .map(|f| (f.id.as_str(), self.filas.get(f.id.as_str()).map(String::as_str).unwrap_or("")))
            .collect();
        registros.sort();
        let carga = serde_json::to_vec(&(registros, propios, tipo, config, &self.trabajo.version_reglas, &self.reglas)).unwrap_or_default();
        hex(Sha256::digest(carga))
    }

    pub fn de_borrador(&self, b: &Borrador, contactos: &Contactos) -> String {
        let a_programa = b.tipo.a_programa();
        let grupo = move |f: &FilaTrabajo| {
            if a_programa {
                format!("{}|{}", tribunal(f), normalizar_columna(&valor(f, "programa")))
            } else {
                tribunal(f)
            }
        };
        let config = serde_json::to_string(contactos).unwrap_or_default();
        self.huella(&b.filas, &grupo, &format!("{:?}", b.tipo), &config)
    }

    /// Incluye la plantilla (`template_hash`).
    pub fn de_proyecto(&self, p: &Proyecto, plantilla: Option<&[u8]>) -> String {
        let grupo = |f: &FilaTrabajo| format!("{}|{}", tribunal(f), normalizar_columna(&valor(f, "rit")));
        let config = plantilla.map(|d| hex(Sha256::digest(d))).unwrap_or_default();
        self.huella(&p.filas, &grupo, &p.tipo, &config)
    }

    /// `product_state.stale`: tiene huella y ya no coincide.
    pub fn borrador_obsoleto(&self, b: &Borrador, contactos: &Contactos) -> bool {
        !b.huella.is_empty() && b.huella != self.de_borrador(b, contactos)
    }

    pub fn proyecto_obsoleto(&self, p: &Proyecto, plantilla: Option<&[u8]>) -> bool {
        !p.huella.is_empty() && p.huella != self.de_proyecto(p, plantilla)
    }
}
