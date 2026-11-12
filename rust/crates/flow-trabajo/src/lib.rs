//! El trabajo del día: una tabla de RUS analizada, con la observación propuesta y la que el usuario deja.

pub mod almacen;
pub mod captura;
pub mod paquete;

use flow_dominio::{Celda, Etapa, Fecha, Historia, Observacion, Registro};
use flow_motor::{analizar, Config, Entrada, Resultado};
use flow_tabular::{Libro, Tabla};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilaTrabajo {
    pub id: String,
    /// Número de fila en el libro de origen.
    pub numero: usize,
    pub celdas: Vec<Celda>,
    pub registro: Registro,
    /// Lo que propone el motor. No se edita.
    pub propuesta: Resultado,
    /// Observación que queda en los productos. Parte igual a la propuesta; el usuario puede cambiarla.
    pub observacion: String,
    #[serde(default)]
    pub revisada: bool,
    /// Próximo informe según el cruce (solo Cumplimiento).
    #[serde(default)]
    pub proximo_informe: Option<Fecha>,
}

impl FilaTrabajo {
    pub fn editada(&self) -> bool {
        self.observacion != self.propuesta.observacion
    }
    pub fn tiene_accion(&self, accion: &str) -> bool {
        self.propuesta.acciones.iter().any(|a| a == accion)
    }
    /// Proyecto de resolución que corresponde (PC_IE, PC_INFO o NOMENCL), si hay.
    pub fn proyecto(&self) -> Option<&str> {
        ["PC_IE", "PC_INFO", "NOMENCL"].into_iter().find(|p| self.tiene_accion(p))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trabajo {
    pub etapa: Etapa,
    pub corte: Fecha,
    pub archivo: String,
    pub sha256: String,
    pub hoja: String,
    pub fila_cabecera: usize,
    pub cabeceras: Vec<String>,
    pub mapeo: std::collections::BTreeMap<String, usize>,
    pub filas: Vec<FilaTrabajo>,
    pub avisos: Vec<String>,
}

/// Historia y bitácora por registro, cuando vienen de una captura de SITFA.
#[derive(Debug, Default, Clone)]
pub struct Contexto {
    pub historias: HashMap<String, Historia>,
    pub bitacoras: HashMap<String, Vec<Observacion>>,
    /// Próximo informe por registro (calendario verificado); reemplaza a la hoja de cruce.
    pub proximos: Option<HashMap<String, Fecha>>,
}

/// Analiza toda la tabla: pre-pase por causa y luego fila por fila.
pub fn analizar_tabla(libro: &Libro, tabla: &Tabla, corte: Fecha, config: &Config, contexto: &Contexto) -> Trabajo {
    analizar_filas(&libro.nombre, &libro.sha256, tabla, corte, config, contexto)
}

/// Como `analizar_tabla`, para tablas que no vienen de un libro (paquete de captura).
pub fn analizar_filas(archivo: &str, sha256: &str, tabla: &Tabla, corte: Fecha, config: &Config, contexto: &Contexto) -> Trabajo {
    let registros: Vec<Registro> = tabla.filas.iter().map(|f| f.registro.clone()).collect();
    let (duplicidad, acumulado) = match tabla.etapa {
        Etapa::Informes => (flow_motor::causas_con_varios(&registros), flow_motor::acumulados(&registros, corte)),
        _ => (flow_motor::duplicidades(&registros), vec![None; registros.len()]),
    };
    let mut avisos = tabla.avisos.clone();
    let filas = tabla
        .filas
        .iter()
        .enumerate()
        .map(|(i, fila)| {
            let proximo = match (&contexto.proximos, tabla.etapa) {
                (_, Etapa::Espera | Etapa::Informes) => None,
                (Some(mapa), _) => mapa.get(&fila.id).copied(),
                (None, _) => tabla.proximo_informe(&fila.registro, corte),
            };
            let propuesta = analizar(&Entrada {
                etapa: tabla.etapa,
                registro: &fila.registro,
                corte,
                config,
                historia: contexto.historias.get(&fila.id),
                bitacora: contexto.bitacoras.get(&fila.id).map(Vec::as_slice).unwrap_or(&[]),
                duplicidad: duplicidad[i],
                acumulado: acumulado[i],
                fecha_hoja2: proximo,
            });
            FilaTrabajo {
                id: fila.id.clone(),
                numero: fila.numero,
                celdas: fila.celdas.clone(),
                registro: fila.registro.clone(),
                observacion: propuesta.observacion.clone(),
                propuesta,
                revisada: false,
                proximo_informe: proximo,
            }
        })
        .collect::<Vec<_>>();
    if filas.is_empty() {
        avisos.push("La tabla no tiene registros.".to_string());
    }
    Trabajo {
        etapa: tabla.etapa,
        corte,
        archivo: archivo.to_string(),
        sha256: sha256.to_string(),
        hoja: tabla.hoja.clone(),
        fila_cabecera: tabla.fila_cabecera,
        cabeceras: tabla.cabeceras.clone(),
        mapeo: tabla.mapeo.clone(),
        filas,
        avisos,
    }
}

impl Trabajo {
    pub fn con_observacion(&self) -> usize {
        self.filas.iter().filter(|f| !f.observacion.is_empty()).count()
    }
    pub fn por_revisar(&self) -> usize {
        self.filas.iter().filter(|f| !f.revisada && (!f.propuesta.avisos.is_empty() || !f.propuesta.marcas.is_empty())).count()
    }
}
