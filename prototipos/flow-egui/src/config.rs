//! Configuración del usuario: umbrales, textos de observaciones, contactos y plantillas de correo.
//! Se guarda solo en este equipo (%APPDATA%\FlowRust).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub const CC_OBLIGATORIO: &str = "ucc_concepcion@pjud.cl";
pub const RETENCION_DIAS: i64 = 60;
pub const SESIONES_ANTERIORES: usize = 10;

/// (clave, valor por defecto, etiqueta)
pub const UMBRALES: [(&str, i64, &str); 14] = [
    ("espera_dce", 0, "Espera DCE: consultar ingreso desde"),
    ("espera_laja", 30, "Espera Laja: proyecto y correo desde"),
    ("espera_mulchen", 30, "Espera Mulchén: proyecto y correo desde"),
    ("espera_tome", 60, "Espera Tomé: correo desde"),
    ("proyecto_tome", 60, "Espera Tomé: proyecto desde"),
    ("mayoria", 60, "Avisar mayoría de edad dentro de"),
    ("oido", 45, "Oído: considerar reciente hasta"),
    ("resolucion_reciente", 30, "Resolución reciente: menos de"),
    ("ingreso_reciente", 30, "Ingreso reciente: hasta"),
    ("medida", 45, "Avisar medida por vencer dentro de"),
    ("informe", 30, "Avisar informe por vencer dentro de"),
    ("ficha_antigua", 180, "Ficha individual antigua: más de"),
    ("ficha_reciente", 30, "Ficha individual reciente: hasta"),
    ("ficha_fae", 120, "Ficha FAE ausente: ingreso hace más de"),
];

/// Textos aprobados de las observaciones (clave «MODO.ID» → texto con variables {VAR}).
pub const TEXTOS: [(&str, &str); 37] = [
    ("COMUN.NO_SEGUIMIENTO", "La derivación {PROGRAMA} no se encuentra sujeta a seguimiento por este Centro."),
    ("COMUN.MAYORIA_EDAD", "{PNOMBRE} alcanzó la mayoría de edad el {FECHA_MAYORIA}. Se sugiere egresar la medida."),
    ("COMUN.PROXIMA_MAYORIA", "{PNOMBRE} alcanzará la mayoría de edad el {FECHA_MAYORIA}."),
    ("COMUN.CURADOR", "No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente."),
    ("COMUN.CURADOR_SIN_DESIGNACION", "No registra curador ad litem en RUS."),
    ("COMUN.OIDO", "Oído el {FECHA_OIDO}."),
    ("COMUN.PROX_AUDIENCIA", "Audiencia decretada para el {FECHA_AUDIENCIA}."),
    ("COMUN.PROX_AUDIENCIAS", "Audiencias decretadas para {FECHAS_AUDIENCIA}."),
    ("COMUN.REITERA_OBSERVACION", "Se reitera observación anterior del {FECHA_OBSERVACION_ANTERIOR}."),
    ("ESPERA.E04_RESOLUCION_RECIENTE", "Medida revisada, a la espera de ingreso efectivo. Ingreso al programa {PROGRAMA} ordenado el {FECHA_RESOLUCION}. (Observación administrativa; no requiere gestión del Tribunal)."),
    ("ESPERA.E05_SOLO_CORREO", "Medida revisada, a la espera de ingreso efectivo. Se remite correo al programa consultando fecha estimada de ingreso."),
    ("ESPERA.E05_PROYECTO_Y_CORREO", "Medida revisada, a la espera de ingreso efectivo. Se remite proyecto de resolución pidiendo cuenta del ingreso efectivo y correo al programa consultando fecha estimada de ingreso."),
    ("ESPERA.E06_SIN_RESOLUCION", "Medida revisada, a la espera de ingreso efectivo al programa {PROGRAMA}."),
    ("CUMPLIMIENTO.C03_INGRESO_RECIENTE", "Ingreso efectivo al programa {PROGRAMA} registrado el {FECHA_INGRESO}."),
    ("CUMPLIMIENTO.C04_VENCIDA", "Medida vencida en RUS desde el {FECHA_EGRESO_PROYECTADO}."),
    ("CUMPLIMIENTO.C05_VENCE_HOY", "Medida con vencimiento en RUS hoy, {FECHA_EGRESO_PROYECTADO}."),
    ("CUMPLIMIENTO.C05_POR_VENCER", "Medida próxima a vencer en RUS el {FECHA_EGRESO_PROYECTADO}."),
    ("CUMPLIMIENTO.C07_SIN_FICHA", "No registra ficha individual en RUS. Se sugiere confeccionarla."),
    ("CUMPLIMIENTO.C07_FICHA_ANTIGUA", "Ficha individual actualizada por última vez el {FECHA_FICHA_INDIVIDUAL}. Se sugiere actualizarla."),
    ("CUMPLIMIENTO.C07_FICHA_RECIENTE", "Ficha individual actualizada el {FECHA_FICHA_INDIVIDUAL}."),
    ("CUMPLIMIENTO.C08_FICHA_FAE", "{PNOMBRE} no registra ficha FAE en RUS. Se sugiere confeccionarla."),
    ("CUMPLIMIENTO.C09_SIN_OBSERVACIONES", "Medida revisada, sin observaciones."),
    ("CUMPLIMIENTO.C10_HOJA2", "Próximo informe de avance de {PROGRAMA} vence el {FECHA_VENCIMIENTO}."),
    ("INFORMES.I01_VENCIDO_DCE", "Informe diagnóstico vencido desde el {FECHA_VENCIMIENTO}. Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe y, de igual manera, correo al programa {PROGRAMA} requiriendo su envío."),
    ("INFORMES.I01_VENCIDO_GENERAL", "Informe de avance vencido desde el {FECHA_VENCIMIENTO}. Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe y, de igual manera, correo al programa {PROGRAMA} requiriendo su envío."),
    ("INFORMES.I02_POR_VENCER_DCE", "Se remite correo al programa {PROGRAMA} informando que el informe diagnóstico debe ser remitido a más tardar el {FECHA_VENCIMIENTO}."),
    ("INFORMES.I02_POR_VENCER_GENERAL", "Se remite correo al programa {PROGRAMA} informando que el próximo informe de avance vence el {FECHA_VENCIMIENTO}."),
    ("INFORMES.I02_MARGEN_DCE", "Informe diagnóstico vencido en RUS desde el {FECHA_VENCIMIENTO}. Se remite correo al programa {PROGRAMA} requiriendo su envío a más tardar el {FECHA_LIMITE}."),
    ("HISTORIA.MEDIDA_SIN_VIGENCIA", "Medida sin vigencia en RUS desde el {FECHA_EGRESO_PROYECTADO}."),
    ("HISTORIA.EGRESO_PROYECTADO", "RUS registra como fecha de egreso proyectado el {FECHA_EGRESO_PROYECTADO}."),
    ("HISTORIA.EGRESO_POSTERIOR_MAYORIA", "El egreso proyectado es posterior a la mayoría de edad."),
    ("HISTORIA.EGRESO_POSTERIOR_MAYORIA_FECHA", "El egreso proyectado es posterior a la mayoría de edad, que se alcanza el {FECHA_MAYORIA}."),
    ("HISTORIA.T5_INSTITUCIONES", "Atendido que en sistema de seguimiento RUS se visualizan {N} instituciones distintas como curadores ad litem de {PNOMBRE}, se sugiere eliminar aquellas designaciones que no se encuentren vigentes."),
    ("HISTORIA.J2_ACUMULADOS", "Registra {N} informes vencidos en RUS; el más antiguo venció el {FECHA}."),
    ("HISTORIA.MEDIDA_REVISADA", "Medida revisada."),
    ("CUMPLIMIENTO.C09_BASE_BREVE", "Medida revisada."),
    ("ESPERA.E07_PROYECCION", "Medida revisada, a la espera de ingreso efectivo. {PROYECCION}"),
];

pub fn texto_base(key: &str) -> &'static str {
    TEXTOS.iter().find(|(k, _)| *k == key).map(|(_, t)| *t).unwrap_or("")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlantillaCorreo {
    pub nombre: String,
    pub asunto: String,
    pub cuerpo: String,
    /// Destinatario: "programa" o "tribunal".
    pub destino: String,
    pub adjunto: bool,
    pub usa_modalidades: bool,
}

pub fn plantillas_base() -> BTreeMap<String, PlantillaCorreo> {
    let mut m = BTreeMap::new();
    let mut add = |k: &str, nombre: &str, asunto: &str, cuerpo: &str, destino: &str, adjunto: bool, modal: bool| {
        m.insert(
            k.to_string(),
            PlantillaCorreo {
                nombre: nombre.into(),
                asunto: asunto.into(),
                cuerpo: cuerpo.into(),
                destino: destino.into(),
                adjunto,
                usa_modalidades: modal,
            },
        );
    };
    for (k, etapa, low) in [("espera", "Espera", "espera"), ("cumplimiento", "Cumplimiento", "cumplimiento"), ("informes", "Informes", "informes")] {
        add(
            k,
            &format!("{etapa} — informativo al tribunal"),
            &format!("Revisión pestaña \"{etapa}\" del módulo RUS — {{TRIBUNAL}}"),
            &format!("Buen día:\n\nJunto con saludar, se informa que pestaña {low} del módulo RUS en SITFA, se revisaron todos los registros pertenecientes a {{ALCANCE_MODALIDADES}} y se registraron observaciones en bitácora.\n\nAtte. a Ud.,"),
            "tribunal",
            false,
            true,
        );
    }
    add(
        "medidas",
        "Medidas sin vigencia y por vencer",
        "Medidas sin vigencia y por vencer — {TRIBUNAL} — {FECHA}",
        "Buen día:\n\nJunto con saludar, se remite archivo Excel adjunto con nomina de medidas sin vigencia y por vencer al día {FECHA} en {ALCANCE_MODALIDADES}.\n\nAtte. a Ud.,",
        "tribunal",
        true,
        true,
    );
    add(
        "proyectos",
        "Reporte de proyectos de resolución",
        "Reporte de proyectos de resolución — {TRIBUNAL} — {PERIODO}",
        "Buen día:\n\nJunto con saludar, se remite reporte de proyectos de resolución elaborados durante {PERIODO}, para su conocimiento y gestión.\n\nAtte. a Ud.,",
        "tribunal",
        true,
        false,
    );
    add(
        "especial",
        "Gestión particular / situación especial",
        "Gestión particular RUS — {TRIBUNAL} — {FECHA}",
        "Buen día:\n\nJunto con saludar, se remiten antecedentes para revisión y gestión particular del Tribunal.\n\nAtte. a Ud.,",
        "tribunal",
        false,
        false,
    );
    add(
        "programa_espera",
        "Lista de espera · programa",
        "Lista de espera — {PROGRAMA} — {TRIBUNAL}",
        "Buen día:\n\nJunto con saludar, se acompaña nómina de registros en lista de espera de {PROGRAMA}, correspondientes a {TRIBUNAL}. Se solicita informar la fecha estimada de ingreso efectivo y el estado de las gestiones realizadas.\n\nAtentamente,",
        "programa",
        true,
        false,
    );
    add(
        "programa_vencido",
        "Informes pendientes · programa",
        "Informes pendientes — {PROGRAMA} — {TRIBUNAL}",
        "Buen día:\n\nJunto con saludar, se acompaña nómina de informes que figuran pendientes de entrega en RUS, correspondientes a {PROGRAMA} y {TRIBUNAL}. Se solicita informar su estado y remitir los antecedentes que correspondan, o precisar si ya fueron presentados.\n\nAtentamente,",
        "programa",
        true,
        false,
    );
    add(
        "programa_por_vencer",
        "Informes por vencer · programa",
        "Informes por vencer — {PROGRAMA} — {TRIBUNAL}",
        "Buen día:\n\nJunto con saludar, se envía planilla con RIT de causas en las cuales, informes de avances se encuentran pronto a vencer. Se ruega acusar recibo de la información.\n\nAtte. a Ud.,",
        "programa",
        true,
        false,
    );
    m
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Tribunal {
    pub nombre: String,
    pub para: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub umbrales: BTreeMap<String, i64>,
    /// Textos editados por el usuario (solo los que difieren del texto base).
    pub textos: BTreeMap<String, String>,
    pub desactivadas: BTreeSet<String>,
    /// Programa → correos (separados por «;»).
    pub contactos: BTreeMap<String, String>,
    pub tribunales: BTreeMap<String, Tribunal>,
    pub cc_adicional: String,
    pub firma: String,
    pub plantillas: BTreeMap<String, PlantillaCorreo>,
    pub carpeta_productos: String,
    pub tema_oscuro: bool,
    pub periodo: String,
    /// true: guardar en Borradores de Outlook; false: archivos .eml.
    pub destino_outlook: bool,
}

impl Default for Config {
    fn default() -> Self {
        let mut tribunales = BTreeMap::new();
        tribunales.insert("LAJA".into(), Tribunal { nombre: "Jgdo. L. y G. de Laja".into(), para: String::new() });
        tribunales.insert("MULCHEN".into(), Tribunal { nombre: "Jgdo. L. y G. de Mulchén".into(), para: String::new() });
        tribunales.insert("TOME".into(), Tribunal { nombre: "Juzgado de Familia de Tomé".into(), para: String::new() });
        Config {
            umbrales: UMBRALES.iter().map(|(k, v, _)| (k.to_string(), *v)).collect(),
            textos: BTreeMap::new(),
            desactivadas: BTreeSet::new(),
            contactos: BTreeMap::new(),
            tribunales,
            cc_adicional: String::new(),
            firma: String::new(),
            plantillas: plantillas_base(),
            carpeta_productos: String::new(),
            tema_oscuro: false,
            periodo: "el período revisado".into(),
            destino_outlook: false,
        }
    }
}

impl Config {
    pub fn umbral(&self, k: &str) -> i64 {
        self.umbrales
            .get(k)
            .copied()
            .unwrap_or_else(|| UMBRALES.iter().find(|(n, _, _)| *n == k).map(|(_, v, _)| *v).unwrap_or(30))
    }

    pub fn texto(&self, k: &str) -> String {
        self.textos.get(k).cloned().unwrap_or_else(|| texto_base(k).to_string())
    }

    /// Completa lo que una configuración anterior no tiene, sin cambiar lo existente.
    pub fn completar(&mut self) {
        for (k, v, _) in UMBRALES {
            self.umbrales.entry(k.to_string()).or_insert(v);
        }
        for (k, p) in plantillas_base() {
            self.plantillas.entry(k).or_insert(p);
        }
        let base = Config::default();
        for (k, t) in base.tribunales {
            self.tribunales.entry(k).or_insert(t);
        }
    }

    pub fn tribunal_nombre(&self, clave: &str, original: &str) -> String {
        self.tribunales.get(clave).map(|t| t.nombre.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| original.to_string())
    }

    /// Correo de un programa (comparación sin tildes ni mayúsculas).
    pub fn contacto_programa(&self, programa: &str) -> String {
        let k = crate::util::clave(programa);
        self.contactos
            .iter()
            .find(|(n, _)| crate::util::clave(n) == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    pub fn cc(&self) -> String {
        let mut v = vec![CC_OBLIGATORIO.to_string()];
        for e in emails(&self.cc_adicional) {
            if !v.iter().any(|x| x.eq_ignore_ascii_case(&e)) {
                v.push(e);
            }
        }
        v.join("; ")
    }
}

pub fn emails(txt: &str) -> Vec<String> {
    txt.split([';', ',', ' ', '\n'])
        .map(|s| s.trim().to_string())
        .filter(|s| s.contains('@'))
        .collect()
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("FlowRust")
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

pub fn load() -> (Config, Option<String>) {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(t) => match serde_json::from_str::<Config>(&t) {
            Ok(mut c) => {
                c.completar();
                (c, None)
            }
            Err(e) => (
                Config::default(),
                Some(format!("No se pudo leer la configuración guardada ({e}); se conserva el archivo y se usan los valores base.")),
            ),
        },
        Err(_) => (Config::default(), None),
    }
}

/// Guardado atómico: escribe un temporal y lo renombra.
pub fn atomic_write(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

pub fn save(c: &Config) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(c).map_err(|e| e.to_string())?;
    atomic_write(&config_path(), &data).map_err(|e| e.to_string())
}
