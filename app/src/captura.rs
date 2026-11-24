//! Lectura local de CSMP_CAPTURE, sin extraer archivos.

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Cursor, Read};
use std::path::Path;
use zip::{CompressionMethod, ZipArchive};

mod contrato;
mod json;
pub mod revision;
mod tablas;
pub use tablas::Fila;
use tablas::Tabla;

const MAX_BYTES: u64 = 80 * 1024 * 1024;
const MAX_MANIFIESTO: u64 = 4 * 1024 * 1024;
const MAX_ENTRADAS: usize = 2048;
const MAX_RATIO: u64 = 200;
const TABLAS_REQUERIDAS: [&str; 3] = ["consultas", "registros", "incidencias"];
const TABLAS_BASE_OPCIONALES: [&str; 4] = [
    "bitacoras",
    "calendarios",
    "historia_tramites",
    "documentos",
];
const TABLAS_EXTENDIDAS: [&str; 2] = ["escritos", "historico_proteccion"];

#[derive(Debug, Deserialize)]
pub struct Manifiesto {
    schema_id: String,
    pub schema_version: String,
    pub capture_id: String,
    #[serde(default)]
    required_features: Vec<String>,
    originals_authorized: bool,
    files: Vec<Archivo>,
    #[serde(default)]
    captured_at: String,
    #[serde(default)]
    mapping_version: String,
    contains_sensitive_data: Option<bool>,
    #[serde(default)]
    downloader: Value,
    queries: Option<Vec<BTreeMap<String, String>>>,
    #[serde(default)]
    table_states: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct Captura {
    pub manifiesto: Manifiesto,
    tablas: HashMap<String, Tabla>,
}

impl Captura {
    pub fn filas(&self, tabla: &str) -> impl Iterator<Item = Fila<'_>> {
        self.tablas.get(tabla).into_iter().flat_map(Tabla::filas)
    }

    pub fn estado(&self, tabla: &str) -> &str {
        self.manifiesto
            .table_states
            .get(tabla)
            .map_or("NO_SOLICITADA", String::as_str)
    }
}

#[derive(Debug, Deserialize)]
struct Archivo {
    path: String,
    bytes: u64,
    sha256: String,
    #[serde(flatten)]
    tipo: Tipo,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Tipo {
    NormalizedTable { table: String, rows: usize },
    ConsolidatedWorkbook,
    Original,
}

fn ruta_valida(ruta: &str) -> bool {
    ruta.chars().count() <= 240
        && ruta.split('/').all(|parte| {
            let base = parte
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            let reservada = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || matches!(base.get(..3), Some("COM" | "LPT"))
                    && matches!(base.as_bytes().get(3..), Some([b'1'..=b'9']));
            !parte.is_empty()
                && !parte.ends_with([' ', '.'])
                && !parte.contains([':', '\\'])
                && !parte.chars().any(|c| c < '\u{20}')
                && !reservada
        })
}

// zip 2.x agrupa entradas de igual nombre. Contar las cabeceras centrales permite
// detectar esa pérdida antes de comparar el índice con el manifiesto.
fn contar_entradas(mut directorio: &[u8]) -> Result<usize, String> {
    let mut cantidad = 0;
    while directorio.starts_with(b"PK\x01\x02") {
        let cabecera = directorio.get(..46).ok_or("Directorio ZIP incompleto.")?;
        let largo = |i| u16::from_le_bytes([cabecera[i], cabecera[i + 1]]) as usize;
        directorio = directorio
            .get(46 + largo(28) + largo(30) + largo(32)..)
            .ok_or("Directorio ZIP incompleto.")?;
        cantidad += 1;
        if cantidad > MAX_ENTRADAS {
            return Err("El paquete tiene demasiadas entradas.".into());
        }
    }
    Ok(cantidad)
}

/// Comprueba la integridad del contenedor. La validación de filas, cobertura,
/// identidad y contenido del consolidado corresponde al importador.
pub fn verificar_integridad(datos: &[u8]) -> Result<Manifiesto, String> {
    leer_paquete(datos, false).map(|c| c.manifiesto)
}

pub fn leer_captura(datos: &[u8]) -> Result<Captura, String> {
    leer_paquete(datos, true)
}

fn leer_paquete(datos: &[u8], analizar_tablas: bool) -> Result<Captura, String> {
    if datos.len() as u64 > MAX_BYTES {
        return Err("El paquete supera 80 MiB.".into());
    }
    let mut zip = abrir_zip(datos, false)?;

    let manifiesto_bytes = leer(&mut zip, "manifest.json", MAX_MANIFIESTO)?;
    let texto = std::str::from_utf8(&manifiesto_bytes).map_err(|_| "Manifiesto sin UTF-8.")?;
    let manifiesto: Manifiesto =
        serde_json::from_value(json::objeto(texto.trim_start_matches('\u{feff}'))?.into())
            .map_err(|_| "Manifiesto de captura ilegible.")?;
    let mut partes = manifiesto.schema_version.split('.');
    let (Some("1"), Some(menor), Some(parche), None) =
        (partes.next(), partes.next(), partes.next(), partes.next())
    else {
        return Err("Versión de captura incompatible.".into());
    };
    let digitos = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if manifiesto.schema_id != "CSMP_CAPTURE" || !digitos(menor) || !digitos(parche) {
        return Err("Contrato de captura incompatible.".into());
    }
    if manifiesto.capture_id.trim().is_empty() || !manifiesto.required_features.is_empty() {
        return Err("El manifiesto carece de identidad o requiere capacidades adicionales.".into());
    }
    let menor = menor
        .parse::<u64>()
        .map_err(|_| "Versión de captura incompatible.")?;
    let mut declaradas = HashSet::from(["manifest.json"]);
    let mut nombres_tablas = HashSet::new();
    let mut tablas = HashMap::new();
    let mut filas = 0;
    for archivo in &manifiesto.files {
        if !declaradas.insert(&archivo.path) || !ruta_valida(&archivo.path) {
            return Err("Ruta repetida o inválida en el manifiesto.".into());
        }
        match &archivo.tipo {
            Tipo::NormalizedTable { table, rows } => {
                let tabla = table.as_str();
                let conocida = TABLAS_REQUERIDAS.contains(&tabla)
                    || TABLAS_BASE_OPCIONALES.contains(&tabla)
                    || menor >= 2 && TABLAS_EXTENDIDAS.contains(&tabla);
                if !conocida || archivo.path != format!("data/{table}.csv") {
                    return Err("Ruta de tabla incompatible.".into());
                }
                nombres_tablas.insert(tabla);
                filas = usize::checked_add(filas, *rows).ok_or("Cantidad de filas inválida.")?;
                if filas > 200_000 {
                    return Err("La captura declara más de 200.000 filas.".into());
                }
            }
            Tipo::Original
                if manifiesto.originals_authorized
                    && (archivo.path.starts_with("originales/")
                        || archivo.path.starts_with("evidencias/")) => {}
            Tipo::ConsolidatedWorkbook
                if menor >= 1 && archivo.path == "consolidado/consolidado.xlsx" => {}
            _ => return Err("El manifiesto declara un archivo incompatible.".into()),
        }
        let bytes = leer(&mut zip, &archivo.path, archivo.bytes.min(MAX_BYTES))?;
        if bytes.len() as u64 != archivo.bytes
            || format!("{:x}", Sha256::digest(&bytes)) != archivo.sha256
        {
            return Err("Una entrada tiene un tamaño o huella distinto del declarado.".into());
        }
        if analizar_tablas {
            match &archivo.tipo {
                Tipo::NormalizedTable { table, rows } => {
                    tablas.insert(table.clone(), Tabla::leer(table, &bytes, *rows)?);
                }
                Tipo::ConsolidatedWorkbook => comprobar_consolidado(&bytes)?,
                Tipo::Original => {}
            }
        }
    }
    if zip.len() != declaradas.len() {
        return Err("El ZIP contiene archivos sin declarar.".into());
    }
    if !TABLAS_REQUERIDAS
        .iter()
        .all(|tabla| nombres_tablas.contains(tabla))
    {
        return Err("Falta una tabla obligatoria en el paquete.".into());
    }
    let captura = Captura { manifiesto, tablas };
    if analizar_tablas {
        contrato::validar(&captura, menor)?;
    }
    Ok(captura)
}

fn abrir_zip(
    datos: &[u8],
    directorios_permitidos: bool,
) -> Result<ZipArchive<Cursor<&[u8]>>, String> {
    let mut zip = ZipArchive::new(Cursor::new(datos)).map_err(|_| "ZIP ilegible.")?;
    let directorio = datos
        .get(zip.central_directory_start() as usize..)
        .ok_or("Directorio ZIP inválido.")?;
    if contar_entradas(directorio)? != zip.len() {
        return Err("El ZIP contiene entradas duplicadas.".into());
    }
    let mut rutas = HashSet::new();
    let mut total = 0;
    for i in 0..zip.len() {
        let entrada = zip.by_index_raw(i).map_err(|_| "Entrada ZIP ilegible.")?;
        let directorio = directorios_permitidos && entrada.is_dir();
        let ruta = if directorio {
            entrada.name().trim_end_matches('/')
        } else {
            entrada.name()
        };
        if !ruta_valida(ruta) || !rutas.insert(ruta.to_lowercase()) {
            return Err("El ZIP contiene rutas inválidas o duplicadas.".into());
        }
        let modo = entrada.unix_mode().unwrap_or(0) & 0o170000;
        let esperado = if directorio { 0o040000 } else { 0o100000 };
        if (modo != 0 && modo != esperado)
            || entrada.encrypted()
            || !matches!(
                entrada.compression(),
                CompressionMethod::Stored | CompressionMethod::Deflated
            )
        {
            return Err("El ZIP contiene una entrada de tipo no admitido.".into());
        }
        total = entrada
            .size()
            .checked_add(total)
            .ok_or("Tamaño ZIP inválido.")?;
        if total > MAX_BYTES
            || entrada.size() > entrada.compressed_size().max(1).saturating_mul(MAX_RATIO)
        {
            return Err("La expansión del ZIP supera el límite permitido.".into());
        }
    }

    Ok(zip)
}

fn comprobar_consolidado(bytes: &[u8]) -> Result<(), String> {
    let mut zip = abrir_zip(bytes, true).map_err(|e| format!("Consolidado: {e}"))?;
    if zip.index_for_name("xl/workbook.xml").is_none() {
        return Err("El consolidado no es un libro XLSX.".into());
    }
    let tipos = String::from_utf8(leer(&mut zip, "[Content_Types].xml", MAX_BYTES)?)
        .map_err(|_| "Tipos del consolidado sin UTF-8.")?
        .to_ascii_lowercase();
    if ["macroenabled", "vbaproject", "activex"]
        .iter()
        .any(|t| tipos.contains(t))
    {
        return Err("El consolidado incluye contenido activo.".into());
    }
    for i in 0..zip.len() {
        let mut entrada = zip
            .by_index(i)
            .map_err(|_| "Entrada inválida en el consolidado.")?;
        let ruta = entrada.name().to_ascii_lowercase();
        if [
            "xl/embeddings/",
            "xl/activex/",
            "xl/externallinks/",
            "xl/querytables/",
        ]
        .iter()
        .any(|p| ruta.starts_with(p))
            || [".html", ".htm", ".pdf", ".bin", ".exe", ".dll"]
                .iter()
                .any(|e| ruta.ends_with(e))
            || ruta.contains("vbaproject")
            || ruta == "xl/connections.xml"
        {
            return Err("El consolidado incluye contenido activo o vínculos externos.".into());
        }
        let esperado = entrada.size();
        let real = std::io::copy(
            &mut entrada.by_ref().take(esperado + 1),
            &mut std::io::sink(),
        )
        .map_err(|_| "Entrada corrupta en el consolidado.")?;
        if real != esperado {
            return Err("Tamaño de entrada incorrecto en el consolidado.".into());
        }
    }
    Ok(())
}

fn leer(zip: &mut ZipArchive<Cursor<&[u8]>>, ruta: &str, limite: u64) -> Result<Vec<u8>, String> {
    let entrada = zip
        .by_name(ruta)
        .map_err(|_| "Falta una entrada declarada en el paquete.")?;
    let tamanio = entrada.size();
    if tamanio > limite {
        return Err("Una entrada supera el tamaño permitido.".into());
    }
    let mut bytes = Vec::new();
    entrada
        .take(tamanio + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Entrada ZIP dañada.")?;
    if bytes.len() as u64 != tamanio {
        return Err("El tamaño real de la entrada ZIP difiere del índice.".into());
    }
    Ok(bytes)
}

pub fn verificar_archivo(ruta: &Path) -> Result<Manifiesto, String> {
    verificar_integridad(&leer_archivo(ruta)?)
}

pub fn leer_archivo(ruta: &Path) -> Result<Vec<u8>, String> {
    let archivo =
        std::fs::File::open(ruta).map_err(|e| format!("No se pudo abrir el paquete: {e}"))?;
    let mut datos = Vec::new();
    archivo
        .take(MAX_BYTES + 1)
        .read_to_end(&mut datos)
        .map_err(|e| format!("No se pudo leer el paquete: {e}"))?;
    if datos.len() as u64 > MAX_BYTES {
        return Err("El paquete supera 80 MiB.".into());
    }
    Ok(datos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};

    fn paquete(cambiar: impl FnOnce(&mut Value), extra: Option<&str>) -> Vec<u8> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let opciones = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut archivos = Vec::new();
        for tabla in TABLAS_REQUERIDAS {
            let ruta = format!("data/{tabla}.csv");
            let bytes = b"id\r\n";
            zip.start_file(&ruta, opciones).unwrap();
            zip.write_all(bytes).unwrap();
            archivos.push(json!({
                "path": ruta, "bytes": bytes.len(), "sha256": format!("{:x}", Sha256::digest(bytes)),
                "type": "normalized_table", "table": tabla, "rows": 0
            }));
        }
        let mut manifiesto = json!({
            "schema_id": "CSMP_CAPTURE", "schema_version": "1.2.0",
            "capture_id": "ficticio", "originals_authorized": false, "files": archivos
        });
        cambiar(&mut manifiesto);
        zip.start_file("manifest.json", opciones).unwrap();
        zip.write_all(&serde_json::to_vec(&manifiesto).unwrap())
            .unwrap();
        if let Some(ruta) = extra {
            zip.start_file(ruta, opciones).unwrap();
            zip.write_all(b"ficticio").unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn verifica_la_integridad_para_versiones_compatibles() {
        for version in ["1.0.0", "1.1.0", "1.2.0", "1.3.0"] {
            let datos = paquete(|m| m["schema_version"] = json!(version), None);
            assert_eq!(
                verificar_integridad(&datos).unwrap().schema_version,
                version
            );
        }
    }

    #[test]
    fn rechaza_un_archivo_con_huella_distinta() {
        let datos = paquete(|m| m["files"][0]["sha256"] = json!("0".repeat(64)), None);
        assert!(verificar_integridad(&datos).unwrap_err().contains("huella"));
    }

    #[test]
    fn rechaza_rutas_inseguras_colisiones_y_archivos_sin_declarar() {
        for ruta in ["../fuera.csv", "DATA/CONSULTAS.CSV", "data/COM1.csv"] {
            let datos = paquete(|_| {}, Some(ruta));
            assert!(verificar_integridad(&datos).unwrap_err().contains("rutas"));
        }
        let datos = paquete(|_| {}, Some("data/extra.csv"));
        assert!(verificar_integridad(&datos)
            .unwrap_err()
            .contains("sin declarar"));
    }

    #[test]
    fn rechaza_versiones_incompatibles_y_capacidades_requeridas() {
        for version in ["2.0.0", "1.2", "1.2.a", "1..0"] {
            let datos = paquete(|m| m["schema_version"] = json!(version), None);
            assert!(verificar_integridad(&datos).is_err());
        }
        let datos = paquete(|m| m["required_features"] = json!(["futura"]), None);
        assert!(verificar_integridad(&datos)
            .unwrap_err()
            .contains("capacidades"));
    }

    #[test]
    fn contrasta_el_contrato_y_rechaza_identidad_cambiada_con_huella_correcta() {
        let raiz = crate::prueba_entorno::requerida("FLOW_REFERENCIA");
        let raiz = Path::new(&raiz);
        let fixture = raiz.join("contracts/csmp_capture_v1/fixtures/completa.zip");
        let bytes = std::fs::read(fixture).unwrap();
        let captura = leer_captura(&bytes).unwrap();
        assert_eq!(captura.filas("registros").count(), 4);
        assert_eq!(captura.estado("bitacoras"), "COMPLETA");
        let parcial =
            std::fs::read(raiz.join("contracts/csmp_capture_v1/fixtures/parcial.zip")).unwrap();
        assert_eq!(
            leer_captura(&parcial).unwrap().estado("bitacoras"),
            "NO_CONSULTADA"
        );
        let extendido = std::fs::read(
            raiz.join("rust/crates/flow-trabajo/tests/datos/paquete_integracion.zip"),
        )
        .unwrap();
        assert_eq!(
            leer_captura(&extendido).unwrap().manifiesto.schema_version,
            "1.2.0"
        );

        let mut original = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut manifest: Value =
            serde_json::from_slice(&leer_cursor(&mut original, "manifest.json")).unwrap();
        let registros = leer_cursor(&mut original, "data/registros.csv");
        let mut csv = csv::Reader::from_reader(registros.as_slice());
        let encabezados = csv.headers().unwrap().clone();
        let columna = encabezados.iter().position(|c| c == "court_code").unwrap();
        let mut nuevo = csv::Writer::from_writer(Vec::new());
        nuevo.write_record(&encabezados).unwrap();
        for (i, fila) in csv.records().enumerate() {
            let fila = fila.unwrap();
            nuevo
                .write_record(fila.iter().enumerate().map(|(j, v)| {
                    if i == 0 && j == columna {
                        "TRIBUNAL_FICTICIO_AJENO"
                    } else {
                        v
                    }
                }))
                .unwrap();
        }
        let nuevo = nuevo.into_inner().unwrap();
        let ficha = manifest["files"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["path"] == "data/registros.csv")
            .unwrap();
        ficha["bytes"] = json!(nuevo.len());
        ficha["sha256"] = json!(format!("{:x}", Sha256::digest(&nuevo)));
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for i in 0..original.len() {
            let mut entrada = original.by_index(i).unwrap();
            let mut data = Vec::new();
            match entrada.name() {
                "manifest.json" => data = serde_json::to_vec(&manifest).unwrap(),
                "data/registros.csv" => data.extend_from_slice(&nuevo),
                _ => {
                    entrada.read_to_end(&mut data).unwrap();
                }
            }
            zip.start_file(entrada.name(), SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&data).unwrap();
        }
        let mutado = zip.finish().unwrap().into_inner();
        assert!(verificar_integridad(&mutado).is_ok());
        assert!(leer_captura(&mutado)
            .unwrap_err()
            .contains("Identidad de consulta cruzada"));
    }

    fn leer_cursor(zip: &mut ZipArchive<Cursor<Vec<u8>>>, ruta: &str) -> Vec<u8> {
        let mut data = Vec::new();
        zip.by_name(ruta).unwrap().read_to_end(&mut data).unwrap();
        data
    }
}
