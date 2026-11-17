//! DURACION y FECHA_RESOLUCION de los proyectos de resolución por cada vía de entrada, con datos ficticios: Excel RUS
//! directo, XLS de SITFA (HTML y XML 2003), lote del Descargador («Llevar a Trabajo») y `Captura_CSMP.zip` del lote.

use flow_dominio::Etapa;
use flow_motor::Config;
use flow_productos::word;
use flow_trabajo::{analizar_tabla, captura, exportar, paquete, Contexto, Trabajo};
use serde_json::json;
use sha2::{Digest, Sha256};

const CORTE: &str = "2026-10-08";
const RESOLUCION: &str = "uno de agosto de dos mil veintiséis";

fn html(duracion: &str, valores: [&str; 3]) -> String {
    let filas: String = valores
        .iter()
        .enumerate()
        .map(|(i, d)| {
            format!(
                "<tr><td>X-{}-2099</td><td>Laja</td><td>Persona Ficticia {}</td><td>11.111.111-1</td><td>AFT Ficticio</td>\
                 <td>30</td><td>20/10/2026</td><td>01/08/2026</td><td>{d}</td></tr>",
                i + 1,
                i + 1
            )
        })
        .collect();
    format!(
        "<html><table><tr><th>RIT</th><th>Tribunal</th><th>Nombre</th><th>RUT</th><th>Derivación</th><th>T ESPERA</th>\
         <th>FECHA VENCIMIENTO</th><th>FEC. RESOLUCION</th><th>{duracion}</th></tr>{filas}</table></html>"
    )
}

fn duraciones(t: &Trabajo) -> Vec<(String, String, String)> {
    word::proyectos(t, CORTE.parse().unwrap(), None)
        .into_iter()
        .filter(|p| p.tipo == "PC_IE")
        .map(|p| (p.rit.clone(), p.valores["DURACION"].clone(), p.valores["FECHA_RESOLUCION"].clone()))
        .collect()
}

fn esperado() -> Vec<(String, String, String)> {
    [("X-1-2099", "6"), ("X-2-2099", "6 meses"), ("X-3-2099", "")].map(|(r, d)| (r.into(), d.into(), RESOLUCION.into())).to_vec()
}

fn directo(nombre: &str, datos: Vec<u8>) -> Trabajo {
    let libro = flow_tabular::leer_bytes(nombre, datos).unwrap();
    let tabla = flow_tabular::leer_tabla(&libro, Some(Etapa::Espera), None).unwrap();
    analizar_tabla(&libro, &tabla, CORTE.parse().unwrap(), &Config::default(), &Contexto::default())
}

#[test]
fn excel_rus_directo_trae_duracion() {
    let ruta = format!("{}/../../../docs/evidencia_p0/golden/Origen_ESPERA.xlsx", env!("CARGO_MANIFEST_DIR"));
    let t = directo("Origen_ESPERA.xlsx", std::fs::read(ruta).unwrap());
    let d = duraciones(&t);
    assert!(!d.is_empty());
    assert!(d.iter().all(|(_, dur, fecha)| dur == "6 meses" && fecha == RESOLUCION), "{d:?}");
}

#[test]
fn xls_de_sitfa_html_y_xml_traen_duracion() {
    for cabecera in ["DURACIÓN", "Duracion", "PLAZO", "Vigencia"] {
        assert_eq!(duraciones(&directo("x.xls", html(cabecera, ["6", "6 meses", ""]).into_bytes())), esperado(), "{cabecera}");
    }
    // Dos columnas de duración: ninguna (como `word_values`, que exige exactamente una).
    let doble = html("DURACION", ["6", "6 meses", ""])
        .replace("<th>DURACION</th>", "<th>DURACION</th><th>PLAZO</th>")
        .replace("</td></tr>", "</td><td>9</td></tr>");
    assert!(duraciones(&directo("x.xls", doble.into_bytes())).iter().all(|(_, d, _)| d.is_empty()));
    // XML 2003 (el otro formato del XLS de SITFA) con la duración como número.
    let celda = |v: &str, tipo: &str| format!("<Cell><Data ss:Type=\"{tipo}\">{v}</Data></Cell>");
    let mut xml = String::from(
        "<?xml version=\"1.0\"?><Workbook xmlns=\"urn:schemas-microsoft-com:office:spreadsheet\" \
         xmlns:ss=\"urn:schemas-microsoft-com:office:spreadsheet\"><Worksheet ss:Name=\"Hoja1\"><Table><Row>",
    );
    for c in ["RIT", "Tribunal", "Nombre", "RUT", "Derivación", "T ESPERA", "FECHA VENCIMIENTO", "FEC. RESOLUCION", "DURACIÓN"] {
        xml.push_str(&celda(c, "String"));
    }
    xml.push_str("</Row><Row>");
    for c in ["X-1-2099", "Laja", "Persona Ficticia 1", "11.111.111-1", "AFT Ficticio"] {
        xml.push_str(&celda(c, "String"));
    }
    xml.push_str(&celda("30", "Number"));
    xml.push_str(&celda("20/10/2026", "String"));
    xml.push_str(&celda("01/08/2026", "String"));
    xml.push_str(&celda("6", "Number"));
    xml.push_str("</Row></Table></Worksheet></Workbook>");
    assert_eq!(duraciones(&directo("x.xls", xml.into_bytes())), esperado()[..1]);
}

fn temporal(nombre: &str) -> std::path::PathBuf {
    let c = std::env::temp_dir().join(format!("flow-duracion-{}-{nombre}", std::process::id()));
    let _ = std::fs::remove_dir_all(&c);
    std::fs::create_dir_all(&c).unwrap();
    c
}

#[test]
fn lote_del_descargador_y_su_paquete_traen_duracion() {
    // Lote ficticio de Espera del Descargador con una página que trae la columna DURACIÓN.
    let c = temporal("lote");
    let xls = html("DURACIÓN", ["6", "6 meses", ""]);
    let nombre = "ESP Ambulatorio Jdo Familia Ficticio Norte.xls";
    std::fs::write(c.join(nombre), &xls).unwrap();
    let espera =
        json!({"tribunal":"901","screen":"seguimiento","tab":"Espera","modality":"2","report":null,"state":null,"month":null,"year":null});
    let r = serde_json::from_value(json!({"version":1,"lote":{"consultas":[espera],"fechas":null,"conservar_filtros":false,"evidencia":false,
        "pausa_ms":250},"estado":"VALIDADA","terminadas":[0],"vacias":[],"avisos":[],"lecturas":[],"paginas_lectura":[],
        "archivos":[{"nombre":nombre,"sha256":format!("{:x}", Sha256::digest(xls.as_bytes())),"consulta":0,"pagina":1,"total":1,"registros":3}],
        "inicio":"2026-10-08T11:00:00-03:00","tribunales":{"901":"Juzgado de Familia Ficticio Norte"}}))
    .unwrap();
    let config = Config::default();
    let desde_lote = captura::importar(&c, &r, CORTE.parse().unwrap(), &config).unwrap();
    assert_eq!(duraciones(&desde_lote[0]), esperado(), "«Llevar a Trabajo»");
    let producto = exportar::construir(&[exportar::Lote { carpeta: &c, resumen: &r }], &exportar::Opciones::default()).unwrap();
    let desde_zip = paquete::importar(exportar::NOMBRE, &producto.datos, CORTE.parse().unwrap(), &config).unwrap().trabajos;
    assert_eq!(duraciones(&desde_zip[0]), esperado(), "Captura_CSMP.zip");
    // La tabla del motor del paquete sigue con sus columnas canónicas (la duración no se agrega al Excel de salida).
    assert!(!desde_zip[0].cabeceras.iter().any(|k| flow_trabajo::controles::columna_duracion(k)));
    std::fs::remove_dir_all(c).unwrap();
}
