//! Informe trazable e integración del motor sin reducir el conjunto que calcula sus agregados.
use crate::revision_bitacoras::{clasificar, Clasificacion, Evidencia, Revision};
use flow_dominio::Fecha;
use flow_motor::{Config, Resultado};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identidad {
    pub sistema: String,
    pub tribunal: String,
    pub causa: String,
    pub ingreso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Procedencia {
    pub registro_id: String,
    pub consulta_id: String,
    pub archivo: String,
    pub tabla: String,
    pub fila: String,
    pub capturada: String,
    pub cobertura: String,
    pub asociacion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilaRevision {
    pub identidad: Identidad,
    pub tipo: String,
    pub valores: Value,
    pub procedencia: Vec<Procedencia>,
    pub lecturas: Vec<Value>,
    pub evidencia: Evidencia,
    pub revision: Revision,
    pub propuesta: Option<Resultado>,
    /// Separada de la propuesta y de las notas de origen.
    pub edicion: Option<String>,
    pub revisada: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Informe {
    pub version: u32,
    pub contrato: String,
    pub huella_paquete: String,
    pub desde: Fecha,
    pub hasta: Fecha,
    pub corte: Fecha,
    pub filas_fuente: usize,
    pub duplicados_registros: usize,
    pub filas: Vec<FilaRevision>,
    pub motor_aplicado: bool,
    pub version_reglas: Option<String>,
}

pub fn huella(datos: &[u8]) -> String {
    Sha256::digest(datos).iter().map(|b| format!("{b:02x}")).collect()
}

impl Informe {
    pub fn conteos(&self) -> [usize; 4] {
        let mut total = [0; 4];
        for f in &self.filas {
            total[match f.revision.clasificacion {
                Clasificacion::SinObservacion => 0, Clasificacion::ConObservacion => 1,
                Clasificacion::NoVerificable => 2, Clasificacion::FueraDeAlcance => 3,
            }] += 1;
        }
        total
    }

    /// Cambiar criterio invalida propuestas y ediciones derivadas: la UI pide confirmación antes.
    pub fn cambiar_periodo(&mut self, desde: Fecha, hasta: Fecha, corte: Fecha) -> Result<(), String> {
        if desde > hasta || hasta > corte { return Err("El período debe ser válido y terminar como máximo en el corte.".into()); }
        if self.desde == desde && self.hasta == hasta && self.corte == corte { return Ok(()); }
        let revisiones = self.filas.iter().map(|f| clasificar(&f.evidencia, desde, hasta)).collect::<Result<Vec<_>, _>>()?;
        self.desde = desde; self.hasta = hasta; self.corte = corte;
        self.motor_aplicado = false; self.version_reglas = None;
        for (f, r) in self.filas.iter_mut().zip(revisiones) {
            f.revision = r; f.propuesta = None; f.edicion = None; f.revisada = false;
        }
        Ok(())
    }
}

/// Se analiza el paquete completo mediante la misma ruta de Trabajo; solo después se eligen resultados.
/// No sustituye ediciones previas. Un error no deja propuestas parcialmente instaladas.
pub fn aplicar_motor(informe: &mut Informe, nombre: &str, datos: &[u8], config: &Config) -> Result<(), String> {
    if huella(datos) != informe.huella_paquete { return Err("El paquete cambió desde su clasificación.".into()); }
    if informe.motor_aplicado { return Err("El motor ya se aplicó; conserva las ediciones revisadas.".into()); }
    if informe.conteos()[0] == 0 { return Err("No hay registros con ausencia verificada para aplicar el motor.".into()); }
    let paquete = crate::paquete::importar(nombre, datos, informe.corte, config)?;
    let version = crate::version_reglas(config);
    let mut propuestas = Vec::with_capacity(informe.filas.len());
    for f in &informe.filas {
        if f.revision.clasificacion != Clasificacion::SinObservacion { propuestas.push(None); continue; }
        let mut encontrada: Option<Resultado> = None;
        for origen in &f.procedencia {
            let candidatas = paquete.trabajos.iter().flat_map(|t| &t.filas).filter(|r| {
                r.id == origen.registro_id && r.origen.as_ref().is_some_and(|o|
                    o.sistema == f.identidad.sistema && o.tribunal == f.identidad.tribunal &&
                    o.causa == f.identidad.causa && o.ingreso == f.identidad.ingreso && o.tipo == f.tipo)
            }).collect::<Vec<_>>();
            if candidatas.is_empty() { return Err("La propuesta no tiene una asociación con su registro de origen.".into()); }
            for candidata in candidatas {
                let propuesta = &candidata.propuesta;
                if let Some(previa) = &encontrada {
                    if serde_json::to_value(previa).map_err(|e| e.to_string())? != serde_json::to_value(propuesta).map_err(|e| e.to_string())? {
                        return Err("Los duplicados producen propuestas distintas; revise el paquete.".into());
                    }
                } else { encontrada = Some(propuesta.clone()); }
            }
        }
        propuestas.push(encontrada);
    }
    for (fila, propuesta) in informe.filas.iter_mut().zip(propuestas) { fila.propuesta = propuesta; }
    informe.motor_aplicado = true;
    informe.version_reglas = Some(version);
    Ok(())
}

