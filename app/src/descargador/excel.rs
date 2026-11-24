//! E-Q5d · Excel del lote de bitácoras con la salida de Flow Python: `nurus.capture.diaries.export_diaries`
//! (`Bitacoras_RUS.xlsx`) y `nurus.capture.historias.export_historias` (`Historia_SITFA.xlsx`), más el escritor XLSX y
//! la presentación compartida (`nurus.personal.excel_presentation`) que usan también los productos de Resultados.

use super::paquete::{self, json_python};
use super::xls::Celda;
use rust_xlsxwriter::{Color, ExcelDateTime, Format, FormatAlign, FormatBorder, FormatPattern, Workbook};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `LIMITE_TEXTO` / `text_parts(limit=30000)`: Excel admite 32.767 caracteres por celda.
pub const LIMITE_TEXTO: usize = 30_000;

/// Filtro automático de una hoja: ninguno, `sheet.dimensions` o un rango explícito.
#[derive(Clone, Debug)]
pub enum Filtro {
    Ninguno,
    Dimensiones,
    Rango(u32, u16),
}

/// Hoja a escribir. Cada fila es la lista que Python pasa a `append` (su largo cuenta en `sheet.dimensions`).
#[derive(Clone, Debug)]
pub struct Hoja {
    pub nombre: String,
    pub filas: Vec<Vec<Celda>>,
    /// `configure_sheet` + `format_cell` (presentación de productos nuevos) con estos encabezados; la primera fila es
    /// el encabezado con su estilo.
    pub presentacion: Option<Vec<String>>,
    pub congelar: bool,
    pub filtro: Filtro,
}

impl Hoja {
    pub fn simple(nombre: &str, filas: Vec<Vec<Celda>>) -> Self {
        Self { nombre: nombre.into(), filas, presentacion: None, congelar: true, filtro: Filtro::Dimensiones }
    }
}

/// `get_column_letter`.
pub fn letra(columna: u16) -> String {
    let mut n = columna as u32 + 1;
    let mut s = Vec::new();
    while n > 0 {
        let r = (n - 1) % 26;
        s.push((b'A' + r as u8) as char);
        n = (n - 1) / 26;
    }
    s.iter().rev().collect()
}

/// `excel_presentation.column_width`.
pub fn ancho_columna(titulo: &str) -> f64 {
    let t = titulo.to_uppercase();
    let alguno = |xs: &[&str]| xs.iter().any(|x| t.contains(x));
    if alguno(&["OBSERVACION", "OBSERVACIÓN", "DETALLE", "TEXTO", "TRÁMITE", "TRAMITE"]) {
        65.0
    } else if alguno(&["NOMBRE", "PERSONA"]) {
        35.0
    } else if alguno(&["PROGRAMA", "DERIVACION"]) {
        30.0
    } else if t.contains("TRIBUNAL") {
        28.0
    } else if t == "RIT" {
        17.0
    } else if t.contains("RUT") {
        16.0
    } else if alguno(&["FECHA", "F.", "VENCIMIENTO", "EGRESO"]) {
        19.0
    } else if alguno(&["ESPERA", "DÍAS", "DIAS"]) {
        14.0
    } else {
        24.0
    }
}

/// Valor JSON del control como celda de `append` (dict/list → `json.dumps`).
pub fn celda_json(v: &Value) -> Celda {
    match v {
        Value::Null => Celda::Vacia,
        Value::Bool(b) => Celda::Logico(*b),
        Value::Number(n) => n.as_i64().map(Celda::Entero).unwrap_or_else(|| Celda::Real(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => Celda::Texto(s.clone()),
        otro => Celda::Texto(json_python(otro, false)),
    }
}

fn texto(s: impl Into<String>) -> Celda {
    Celda::Texto(s.into())
}

/// `text_parts`: trozos de hasta `limite` unidades UTF-16, sin partir un carácter.
pub fn partes_texto(valor: &str, limite: usize) -> Vec<String> {
    let (mut partes, mut pieza, mut unidades) = (Vec::new(), String::new(), 0usize);
    for c in valor.chars() {
        let tam = c.len_utf16();
        if unidades + tam > limite {
            partes.push(std::mem::take(&mut pieza));
            unidades = 0;
        }
        pieza.push(c);
        unidades += tam;
    }
    if !pieza.is_empty() {
        partes.push(pieza);
    }
    if partes.is_empty() {
        partes.push(String::new());
    }
    partes
}

fn fecha_excel(d: &chrono::NaiveDateTime) -> Result<ExcelDateTime, String> {
    use chrono::{Datelike, Timelike};
    ExcelDateTime::from_ymd(d.year() as u16, d.month() as u8, d.day() as u8)
        .and_then(|f| f.and_hms(d.hour() as u16, d.minute() as u8, d.second() as f64))
        .map_err(|e| e.to_string())
}

/// El libro en memoria. Sin presentación, como `Workbook()` de openpyxl con `freeze_panes` y `auto_filter`; con ella,
/// `configure_sheet` y `format_cell`.
pub fn libro(hojas: &[Hoja]) -> Result<Vec<u8>, String> {
    let e = |x: rust_xlsxwriter::XlsxError| x.to_string();
    let mut libro = Workbook::new();
    let base = Format::new()
        .set_font_name("Calibri")
        .set_font_size(11)
        .set_text_wrap()
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::Black);
    let encabezado = base
        .clone()
        .set_font_color(Color::White)
        .set_bold()
        .set_pattern(FormatPattern::Solid)
        .set_background_color(Color::RGB(0x195F7B));
    let cuerpo_texto = base.clone().set_num_format("@");
    let cuerpo_fecha = base.clone().set_num_format("dd/mm/yyyy");
    let fecha_simple = Format::new().set_num_format("yyyy-mm-dd h:mm:ss");
    for hoja in hojas {
        let ws = libro.add_worksheet();
        ws.set_name(&hoja.nombre).map_err(e)?;
        if let Some(cabeceras) = &hoja.presentacion {
            ws.set_repeat_rows(0, 0).map_err(e)?;
            ws.set_print_fit_to_pages(1, 0);
            ws.set_landscape();
            ws.set_paper_size(9);
            ws.set_margins(0.3, 0.3, 0.45, 0.45, 0.2, 0.2);
            ws.set_footer("&CPágina &P de &N");
            ws.set_row_height(0, 32).map_err(e)?;
            for (i, t) in cabeceras.iter().enumerate() {
                ws.set_column_width(i as u16, ancho_columna(t)).map_err(e)?;
            }
        }
        if hoja.congelar {
            ws.set_freeze_panes(1, 0).map_err(e)?;
        }
        for (r, fila) in hoja.filas.iter().enumerate() {
            let r = r as u32;
            for (c, v) in fila.iter().enumerate() {
                let c = c as u16;
                match (&hoja.presentacion, v) {
                    (None, Celda::Vacia) => {}
                    (None, Celda::Texto(s)) if s.is_empty() => {}
                    (None, Celda::Texto(s)) => {
                        ws.write_string(r, c, s).map_err(e)?;
                    }
                    (None, Celda::Entero(n)) => {
                        ws.write_number(r, c, *n as f64).map_err(e)?;
                    }
                    (None, Celda::Real(x)) => {
                        ws.write_number(r, c, *x).map_err(e)?;
                    }
                    (None, Celda::Logico(b)) => {
                        ws.write_boolean(r, c, *b).map_err(e)?;
                    }
                    (None, Celda::Fecha(d)) => {
                        ws.write_datetime_with_format(r, c, fecha_excel(d)?, &fecha_simple).map_err(e)?;
                    }
                    (Some(_), v) => {
                        let es_encabezado = r == 0;
                        let formato = if es_encabezado { &encabezado } else { &base };
                        match v {
                            Celda::Vacia => {
                                ws.write_blank(r, c, formato).map_err(e)?;
                            }
                            Celda::Texto(s) if s.is_empty() => {
                                ws.write_blank(r, c, if es_encabezado { &encabezado } else { &cuerpo_texto }).map_err(e)?;
                            }
                            Celda::Texto(s) => {
                                ws.write_string_with_format(r, c, s, if es_encabezado { &encabezado } else { &cuerpo_texto })
                                    .map_err(e)?;
                            }
                            Celda::Entero(n) => {
                                ws.write_number_with_format(r, c, *n as f64, formato).map_err(e)?;
                            }
                            Celda::Real(x) => {
                                ws.write_number_with_format(r, c, *x, formato).map_err(e)?;
                            }
                            Celda::Logico(b) => {
                                ws.write_boolean_with_format(r, c, *b, formato).map_err(e)?;
                            }
                            Celda::Fecha(d) => {
                                let f = if es_encabezado { &encabezado } else { &cuerpo_fecha };
                                ws.write_datetime_with_format(r, c, fecha_excel(d)?, f).map_err(e)?;
                            }
                        }
                    }
                }
            }
        }
        let ancho = hoja.filas.iter().map(Vec::len).max().unwrap_or(1).max(1) as u16;
        let alto = hoja.filas.len().max(1) as u32;
        match hoja.filtro {
            Filtro::Ninguno => {}
            Filtro::Dimensiones => {
                ws.autofilter(0, 0, alto - 1, ancho - 1).map_err(e)?;
            }
            Filtro::Rango(filas, columnas) => {
                ws.autofilter(0, 0, filas.max(1) - 1, columnas.max(1) - 1).map_err(e)?;
            }
        }
    }
    libro.save_to_buffer().map_err(e)
}

/// Guarda un Excel nuevo: falla si el destino existe, nunca sobrescribe.
pub fn guardar_nuevo(destino: &Path, hojas: &[Hoja]) -> Result<PathBuf, String> {
    if destino.exists() {
        return Err("El destino ya existe; elige otra copia.".into());
    }
    let bytes = libro(hojas)?;
    if let Some(padre) = destino.parent() {
        std::fs::create_dir_all(padre).map_err(|e| format!("No se pudo crear la carpeta del Excel: {e}"))?;
    }
    super::lote::guardar_exclusivo(destino, &bytes)?;
    Ok(destino.to_path_buf())
}

// ---------------------------------------------------------------- `export_diaries`

const COLUMNAS_COPIA: [&str; 13] = [
    "tribunal_codigo", "causa_id", "ingreso_id", "entry_id", "fecha", "autor", "tipo", "etapa", "texto", "fecha_respuesta",
    "respuesta", "fuente", "fila_fuente",
];

/// Las hojas de `export_diaries`: copia íntegra de todas las entradas (sin ventana de meses ni truncamiento), textos
/// íntegros partidos, fuentes, ingresos y las hojas de control de cada lote.
pub fn hojas_bitacoras(manifiestos: &[PathBuf]) -> Result<Vec<Hoja>, String> {
    let captura = paquete::importar_lotes(manifiestos, None)?;
    let mut copia = vec![COLUMNAS_COPIA.iter().map(|c| texto(*c)).collect::<Vec<_>>()];
    let mut partes = vec![["Clave", "Campo", "Parte", "Total", "SHA256 texto", "Texto"].map(texto).to_vec()];
    let mut fuentes = vec![["Archivo", "SHA256", "Peticiones", "Aperturas", "Estado"].map(texto).to_vec()];
    for f in &captura.fuentes {
        fuentes.push(["file", "sha256", "requests", "openings", "detail"].iter().map(|k| celda_json(&f[*k])).collect());
    }
    let mut ingresos = vec![["Tribunal", "Causa", "Ingreso", "Alcance", "Entradas", "Advertencias"].map(texto).to_vec()];
    for q in &captura.consultas {
        let entradas = q["entries"].as_array().cloned().unwrap_or_default();
        let avisos: Vec<String> = q["warnings"].as_array().into_iter().flatten().map(paquete::texto_de).collect();
        ingresos.push(vec![
            celda_json(&q["tribunal_codigo"]),
            celda_json(&q["causa_id"]),
            celda_json(&q["ingreso_id"]),
            celda_json(&q["coverage"]),
            Celda::Entero(entradas.len() as i64),
            texto(avisos.join("\n")),
        ]);
        for e in &entradas {
            let clave = ["tribunal_codigo", "causa_id", "ingreso_id", "entry_id"].map(|k| paquete::texto_de(&e[k])).join("|");
            let mut valores = Vec::new();
            for campo in COLUMNAS_COPIA {
                let valor = e.get(campo).cloned().unwrap_or(json!(""));
                let crudo = paquete::texto_de(&valor);
                let trozos = partes_texto(&crudo, LIMITE_TEXTO);
                if trozos.len() > 1 {
                    let huella = paquete::sha256_hex(crudo.as_bytes());
                    for (n, t) in trozos.iter().enumerate() {
                        partes.push(vec![texto(&clave), texto(campo), Celda::Entero(n as i64 + 1), Celda::Entero(trozos.len() as i64),
                            texto(&huella), texto(t)]);
                    }
                    valores.push(texto(format!("Texto íntegro en hoja Textos íntegros; clave {clave}; campo {campo}; SHA256 {huella}")));
                } else {
                    valores.push(celda_json(&valor));
                }
            }
            copia.push(valores);
        }
    }
    let mut hojas = vec![
        Hoja::simple("Copia íntegra", copia),
        Hoja::simple("Textos íntegros", partes),
        Hoja::simple("Fuentes", fuentes),
        Hoja::simple("Ingresos", ingresos),
    ];
    for (titulo, cabecera, filas) in captura.extras {
        let mut h = vec![cabecera.iter().map(|c| texto(*c)).collect::<Vec<_>>()];
        h.extend(filas.iter().map(|f| f.iter().map(celda_json).collect()));
        hojas.push(Hoja::simple(&titulo, h));
    }
    Ok(hojas)
}

/// `export_diaries(manifest, destination)`.
pub fn exportar_bitacoras(manifiestos: &[PathBuf], destino: &Path) -> Result<PathBuf, String> {
    if destino.exists() {
        return Err("El destino ya existe; elige otra copia.".into());
    }
    guardar_nuevo(destino, &hojas_bitacoras(manifiestos)?)
}

// ---------------------------------------------------------------- `export_historias`

const HOJAS_HISTORIA: [(&str, &[&str]); 3] = [
    ("Trámites", &["Tribunal", "RIT", "Folio", "Fecha Tram", "Etapa", "Trámite", "Desc. Trámite", "Referencia", "Anexo visible",
        "Causa ID", "Archivo fuente", "Fila fuente", "SHA-256 fuente", "Leída el"]),
    ("Causas", &["Tribunal", "RIT", "Estado", "Trámites en ventana", "Trámites anteriores", "Filas incompletas", "Desde", "Hasta",
        "Advertencias / error", "RIT visible", "RUC", "Caratulado", "F. Ingreso", "Proc.", "Forma Inicio", "Est. Adm.", "Etapa",
        "Estado Proc.", "Tribunal visible", "Reservada", "Causa ID", "Archivo fuente", "SHA-256"]),
    ("Vínculos", &["Tribunal", "RIT", "Nombre", "RUT", "Programa", "Pestaña", "Modalidad", "Ingreso ID", "Causa ID",
        "Estado de la historia"]),
];
const LEYENDAS: [&str; 3] = [
    "Sin PDF: no se abrieron documentos.",
    "La historia es de la causa, no de un ingreso.",
    "Solo trámites entre DESDE y HASTA; no acredita ausencia de trámites fuera de ese rango.",
];
const CABECERA_HISTORIA: [&str; 10] =
    ["RIT", "RUC", "Caratulado", "F. Ingreso", "Proc.", "Forma Inicio", "Est. Adm.", "Etapa", "Estado Proc.", "Tribunal"];

/// `agregar` de `export_historias`: los textos largos siguen en columnas de continuación «… (parte n)».
struct HojaHistoria {
    filas: Vec<Vec<Celda>>,
    continuaciones: BTreeMap<(usize, usize), usize>,
}

impl HojaHistoria {
    fn max_columna(&self) -> usize {
        self.filas.iter().map(Vec::len).max().unwrap_or(0)
    }

    fn agregar(&mut self, valores: Vec<Celda>, divididos: &mut usize) {
        let mut valores = valores;
        let mut extra: Vec<(usize, String)> = Vec::new();
        for i in 0..valores.len() {
            let Celda::Texto(v) = &valores[i] else { continue };
            let trozos = partes_texto(v, LIMITE_TEXTO);
            valores[i] = Celda::Texto(trozos[0].clone());
            if trozos.len() > 1 {
                *divididos += 1;
            }
            for (n, parte) in trozos.iter().enumerate().skip(1) {
                let numero = n + 1;
                let columna = match self.continuaciones.get(&(i, numero)) {
                    Some(c) => *c,
                    None => {
                        let c = valores.len().max(self.max_columna()) + 1;
                        self.continuaciones.insert((i, numero), c);
                        let titulo = match self.filas.first().and_then(|f| f.get(i)) {
                            Some(Celda::Texto(t)) => t.clone(),
                            Some(otro) => otro.texto_python(),
                            None => "None".into(),
                        };
                        let cab = &mut self.filas[0];
                        if cab.len() < c {
                            cab.resize(c, Celda::Vacia);
                        }
                        cab[c - 1] = Celda::Texto(format!("{titulo} (parte {numero})"));
                        c
                    }
                };
                extra.push((columna, parte.clone()));
            }
        }
        for (c, p) in extra {
            if valores.len() < c {
                valores.resize(c, Celda::Vacia);
            }
            valores[c - 1] = Celda::Texto(p);
        }
        self.filas.push(valores);
    }
}

fn si_no(v: bool) -> Celda {
    texto(if v { "Sí" } else { "No" })
}

fn literal(v: &Value) -> Celda {
    texto(paquete::texto_de(v))
}

fn folio_clave(folio: &str) -> (u8, i64) {
    if !folio.is_empty() && folio.chars().all(|c| c.is_ascii_digit()) { (0, -folio.parse::<i64>().unwrap_or(0)) } else { (1, 0) }
}

/// Las hojas de `export_historias`: trámites de la ventana, causas, vínculos y control.
pub fn hojas_historias(manifiesto: &Path) -> Result<Vec<Hoja>, String> {
    let (estado, causas) = paquete::cargar_historias(manifiesto)?;
    let nombres: BTreeMap<String, String> = causas
        .iter()
        .map(|c| {
            let n = paquete::texto_de(&c.causa["tribunal_nombre"]);
            (c.clave.clone(), if n.is_empty() { paquete::texto_de(&c.causa["tribunal"]) } else { n })
        })
        .collect();
    let mut divididos = 0usize;
    let mut hojas: Vec<HojaHistoria> = HOJAS_HISTORIA
        .iter()
        .map(|(_, cols)| HojaHistoria { filas: vec![cols.iter().map(|c| texto(*c)).collect()], continuaciones: BTreeMap::new() })
        .collect();
    let mut control = HojaHistoria { filas: Vec::new(), continuaciones: BTreeMap::new() };
    let mut filas: Vec<(&paquete::Causa, &Value)> = causas.iter().flat_map(|c| c.ventana.iter().map(move |f| (c, f))).collect();
    filas.sort_by_key(|(_, f)| folio_clave(&paquete::texto_de(&f["folio"])));
    filas.sort_by(|a, b| paquete::texto_de(&b.1["fecha_iso"]).cmp(&paquete::texto_de(&a.1["fecha_iso"])));
    filas.sort_by(|a, b| {
        (&nombres[&a.0.clave], paquete::texto_de(&a.0.causa["rit"])).cmp(&(&nombres[&b.0.clave], paquete::texto_de(&b.0.causa["rit"])))
    });
    for (item, f) in &filas {
        let c = &item.causa;
        hojas[0].agregar(
            vec![
                texto(&nombres[&item.clave]),
                celda_json(&c["rit"]),
                celda_json(&f["folio"]),
                celda_json(&f["fecha_texto"]),
                literal(&f["etapa"]),
                literal(&f["tramite"]),
                literal(&f["descripcion"]),
                literal(&f["referencia"]),
                si_no(f["anexo"].as_bool().unwrap_or(false)),
                celda_json(&c["causa_id"]),
                celda_json(&c["archivo"]),
                celda_json(&f["fila"]),
                celda_json(&c["sha256"]),
                celda_json(&c["leida"]),
            ],
            &mut divididos,
        );
    }
    let mut ordenadas: Vec<&paquete::Causa> = causas.iter().collect();
    ordenadas.sort_by(|a, b| {
        (&nombres[&a.clave], paquete::texto_de(&a.causa["rit"])).cmp(&(&nombres[&b.clave], paquete::texto_de(&b.causa["rit"])))
    });
    let o = |c: &Value, k: &str| c.get(k).map(celda_json).unwrap_or_else(|| texto(""));
    for item in ordenadas {
        let c = &item.causa;
        let error = paquete::texto_de(&c["error"]);
        let avisos = if error.is_empty() {
            c["advertencias"].as_array().into_iter().flatten().map(paquete::texto_de).collect::<Vec<_>>().join("; ")
        } else {
            error
        };
        let mut fila = vec![
            texto(&nombres[&item.clave]),
            celda_json(&c["rit"]),
            celda_json(&c["estado"]),
            o(c, "en_ventana"),
            o(c, "anteriores"),
            o(c, "incompletas"),
            o(c, "desde"),
            o(c, "hasta"),
            texto(avisos),
        ];
        let cabecera = item.lectura.as_ref().map(|l| l["encabezado"].clone()).unwrap_or(json!({}));
        fila.extend(CABECERA_HISTORIA.iter().map(|k| literal(&cabecera.get(*k).cloned().unwrap_or(json!("")))));
        fila.push(match &item.lectura {
            Some(l) => si_no(l["reservada"].as_bool().unwrap_or(false)),
            None => texto(""),
        });
        fila.extend([celda_json(&c["causa_id"]), o(c, "archivo"), o(c, "sha256")]);
        hojas[1].agregar(fila, &mut divididos);
    }
    for v in estado["vinculos"].as_array().into_iter().flatten() {
        let clave = paquete::texto_de(&v["clave"]);
        let tribunal = nombres.get(&clave).cloned().unwrap_or_else(|| paquete::texto_de(&v["tribunal_codigo"]));
        let mut fila = vec![texto(tribunal)];
        fila.extend(["rit", "nombre", "rut", "programa", "pestana", "modalidad", "ingreso_id", "causa_id"].iter().map(|k| celda_json(&v[*k])));
        fila.push(celda_json(&estado["causas"][&clave]["estado"]));
        hojas[2].agregar(fila, &mut divididos);
    }
    let mut conteo: BTreeMap<String, usize> = BTreeMap::new();
    for c in &causas {
        *conteo.entry(paquete::texto_de(&c.causa["estado"])).or_default() += 1;
    }
    let ordenados = |k: &str, n: usize| {
        let mut v: Vec<String> = causas
            .iter()
            .map(|c| paquete::texto_de(&c.causa[k]))
            .filter(|s| !s.is_empty())
            .map(|s| s.chars().take(n).collect())
            .collect();
        v.sort();
        v
    };
    let (leidas, desdes, hastas) = (ordenados("leida", 10), ordenados("desde", usize::MAX), ordenados("hasta", usize::MAX));
    let fecha = match (leidas.first(), leidas.last()) {
        (Some(a), Some(b)) if a == b => a.clone(),
        (Some(a), Some(b)) => format!("{a} a {b}"),
        _ => String::new(),
    };
    let por_estado = if conteo.is_empty() {
        "ninguna".to_string()
    } else {
        conteo.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("; ")
    };
    let mut filas_control: Vec<Vec<Celda>> = vec![
        vec![texto("Versión Descargador"), celda_json(&estado["descargador"].get("version").cloned().unwrap_or(json!("")))],
        vec![texto("Versión extensión"), celda_json(&estado.get("extension_version").cloned().unwrap_or(json!("")))],
        vec![texto("Commit"), celda_json(&estado["descargador"].get("commit").cloned().unwrap_or(json!("")))],
        vec![texto("Fecha de lectura"), texto(fecha)],
        vec![texto("Criterio"), celda_json(&estado.get("criterio_temporal").cloned().unwrap_or(json!("ventana_4_meses")))],
        vec![texto("Desde"), texto(desdes.first().cloned().unwrap_or_default())],
        vec![texto("Hasta"), texto(hastas.last().cloned().unwrap_or_default())],
        vec![texto("Causas por estado"), texto(por_estado)],
        vec![texto("Tope alcanzado"), si_no(estado["tope_alcanzado"].as_bool().unwrap_or(false))],
    ];
    control.agregar(vec![texto("Campo"), texto("Valor")], &mut divididos);
    let antes = divididos;
    filas_control.push(vec![texto("Textos divididos en columnas de continuación"), Celda::Entero(antes as i64)]);
    filas_control.extend(LEYENDAS.iter().map(|t| vec![texto("Leyenda"), texto(*t)]));
    for f in filas_control {
        control.agregar(f, &mut divididos);
    }
    let mut salida: Vec<Hoja> =
        HOJAS_HISTORIA.iter().zip(hojas).map(|((n, _), h)| Hoja::simple(n, h.filas)).collect();
    salida.push(Hoja::simple("Control", control.filas));
    Ok(salida)
}

/// `export_historias(manifest_path, destination)`.
pub fn exportar_historias(manifiesto: &Path, destino: &Path) -> Result<PathBuf, String> {
    if destino.exists() {
        return Err("El destino ya existe; elige otra copia.".into());
    }
    guardar_nuevo(destino, &hojas_historias(manifiesto)?)
}

/// Nombre libre en la carpeta del lote: `base.xlsx` o, si existe, `base_<8 hex>.xlsx` (`uuid4().hex[:8]`).
pub fn nombre_libre(carpeta: &Path, base: &str) -> PathBuf {
    let destino = carpeta.join(format!("{base}.xlsx"));
    if !destino.exists() {
        return destino;
    }
    let mut azar = [0u8; 4];
    let _ = getrandom::fill(&mut azar);
    carpeta.join(format!("{base}_{}.xlsx", azar.iter().map(|b| format!("{b:02x}")).collect::<String>()))
}

/// Volcado de un XLSX con la forma de `app/qa/generar_excel.py::volcar` (para comparar con openpyxl).
#[cfg(test)]
pub(crate) mod volcado {
    use calamine::{Data, Reader};
    use regex::Regex;
    use serde_json::{json, Value};
    use std::io::Read;

    fn xml(zip: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>, nombre: &str) -> String {
        let mut s = String::new();
        zip.by_name(nombre).unwrap().read_to_string(&mut s).unwrap();
        s
    }

    fn letra_a_indice(l: &str) -> usize {
        l.bytes().fold(0, |n, b| n * 26 + (b - b'A' + 1) as usize) - 1
    }

    /// Celdas comparables: números como reales (openpyxl distingue int de float), texto vacío como `None`.
    pub(crate) fn normalizar(v: &Value) -> Value {
        match v {
            Value::Array(a) if a.len() == 2 && a[0] == "n" => json!(["n", a[1].as_f64().unwrap()]),
            Value::Array(a) if a.len() == 2 && a[0] == "s" && a[1] == "" => Value::Null,
            Value::Array(a) => Value::Array(a.iter().map(normalizar).collect()),
            Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), normalizar(v))).collect()),
            otro => otro.clone(),
        }
    }

    /// `generar_excel.sin_base`: la carpeta temporal como `<BASE>` y sus separadores como `/`.
    pub(crate) fn sin_base(texto: &str, base: &str) -> String {
        match texto.split_once(base) {
            Some((cabeza, cola)) => format!("{cabeza}<BASE>{}", cola.replace('\\', "/")),
            None => texto.to_string(),
        }
    }

    #[allow(clippy::regex_creation_in_loops)]
    pub(crate) fn volcar(bytes: &[u8], base: &str) -> Value {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).unwrap();
        let libro = xml(&mut zip, "xl/workbook.xml");
        let rels = xml(&mut zip, "xl/_rels/workbook.xml.rels");
        let hoja_re = Regex::new(r#"<sheet name="([^"]*)" sheetId="\d+" r:id="(rId\d+)""#).unwrap();
        let mut calam = calamine::Xlsx::new(std::io::Cursor::new(bytes.to_vec())).unwrap();
        let mut hojas = Vec::new();
        for c in hoja_re.captures_iter(&libro) {
            let nombre = c[1].replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"");
            let destino = Regex::new(&format!(r#"Id="{}"[^>]*Target="([^"]*)""#, &c[2])).unwrap().captures(&rels)
                .or_else(|| Regex::new(&format!(r#"Target="([^"]*)"[^>]*Id="{}""#, &c[2])).unwrap().captures(&rels))
                .unwrap()[1].trim_start_matches('/').trim_start_matches("xl/").to_string();
            let hoja = xml(&mut zip, &format!("xl/{destino}"));
            let congelar = Regex::new(r#"<pane [^>]*topLeftCell="([A-Z]+\d+)""#).unwrap().captures(&hoja).map(|m| m[1].to_string());
            let filtro = Regex::new(r#"<autoFilter ref="([^"]+)""#).unwrap().captures(&hoja).map(|m| m[1].to_string());
            let mut anchos = serde_json::Map::new();
            for col in Regex::new(r#"<col ([^>]*)/>"#).unwrap().captures_iter(&hoja) {
                let a = &col[1];
                let campo = |k: &str| Regex::new(&format!(r#"{k}="([^"]*)""#)).unwrap().captures(a).map(|m| m[1].to_string());
                if campo("customWidth").as_deref() != Some("1") {
                    continue;
                }
                let ancho: f64 = campo("width").unwrap().parse().unwrap();
                // rust_xlsxwriter guarda el ancho en caracteres más el relleno de 5 px (0,7109375).
                let ancho = ((ancho - 0.7109375) * 100.0).round() / 100.0;
                for i in campo("min").unwrap().parse::<u16>().unwrap()..=campo("max").unwrap().parse::<u16>().unwrap() {
                    anchos.insert(super::letra(i - 1), json!(ancho));
                }
            }
            let rango = calam.worksheet_range(&nombre).unwrap();
            let (f0, c0) = rango.start().map(|(f, c)| (f as usize, c as usize)).unwrap_or((0, 0));
            let mut filas: Vec<Vec<Value>> = vec![Vec::new(); f0];
            for fila in rango.rows() {
                let mut f: Vec<Value> = vec![Value::Null; c0];
                f.extend(fila.iter().map(|d| match d {
                    Data::Empty => Value::Null,
                    Data::String(s) => json!(["s", sin_base(s, base)]),
                    Data::Float(x) => json!(["n", x]),
                    Data::Int(n) => json!(["n", *n as f64]),
                    Data::Bool(b) => json!(["b", b]),
                    Data::DateTime(t) => {
                        let s = t.as_f64();
                        let d = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap().and_hms_opt(0, 0, 0).unwrap()
                            + chrono::Duration::seconds((s * 86_400.0).round() as i64);
                        json!(["d", d.format("%Y-%m-%dT%H:%M:%S").to_string()])
                    }
                    otro => json!(["s", otro.to_string()]),
                }));
                while f.last().is_some_and(Value::is_null) {
                    f.pop();
                }
                filas.push(f);
            }
            while filas.last().is_some_and(Vec::is_empty) {
                filas.pop();
            }
            let _ = letra_a_indice;
            hojas.push(json!({"titulo": nombre, "congelar": congelar, "filtro": filtro, "anchos": anchos, "celdas": filas}));
        }
        json!({"hojas": hojas})
    }

    /// Diferencias legibles entre dos volcados (hoja, fila y celda).
    pub(crate) fn diferencias(rust: &Value, python: &Value) -> Vec<String> {
        let (r, p) = (normalizar(rust), normalizar(python));
        let mut d = Vec::new();
        let hr = r["hojas"].as_array().unwrap();
        let hp = p["hojas"].as_array().unwrap();
        let titulos = |h: &Vec<Value>| h.iter().map(|x| x["titulo"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        if titulos(hr) != titulos(hp) {
            d.push(format!("hojas: Rust {:?} · Python {:?}", titulos(hr), titulos(hp)));
            return d;
        }
        for (a, b) in hr.iter().zip(hp) {
            let t = a["titulo"].as_str().unwrap();
            for k in ["congelar", "filtro", "anchos"] {
                if a[k] != b[k] {
                    d.push(format!("{t}.{k}: Rust {} · Python {}", a[k], b[k]));
                }
            }
            let (fa, fb) = (a["celdas"].as_array().unwrap(), b["celdas"].as_array().unwrap());
            if fa.len() != fb.len() {
                d.push(format!("{t}: filas Rust {} · Python {}", fa.len(), fb.len()));
            }
            for (i, (x, y)) in fa.iter().zip(fb).enumerate() {
                if x != y {
                    let corta = |v: &Value| v.to_string().chars().take(400).collect::<String>();
                    d.push(format!("{t} fila {}:\n      Rust   {}\n      Python {}", i + 1, corta(x), corta(y)));
                }
            }
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn reconstruir(destino: &Path, archivos: &Value) {
        for (ruta, b64) in archivos.as_object().unwrap() {
            let ruta = destino.join(ruta);
            std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            std::fs::write(ruta, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
    }

    /// E-Q5d: `Bitacoras_RUS.xlsx` e `Historia_SITFA.xlsx` de Rust contra `export_diaries` y `export_historias` de Flow
    /// Python (`app/qa/generar_excel.py`): hojas, panel congelado, filtro y cada celda con su tipo.
    #[test]
    fn excel_de_bitacoras_igual_a_python() {
        let paquete: Value = serde_json::from_str(include_str!("../../tests/datos/paquete.json")).unwrap();
        let excel: Value = serde_json::from_str(include_str!("../../tests/datos/excel.json")).unwrap();
        let contexto = paquete["escenarios"].as_array().unwrap().iter().find(|e| e["escenario"] == "contexto").unwrap();
        let base = std::env::temp_dir().join(format!("flow-eq5d-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let carpeta = base.join("bitacoras").join(contexto["bitacoras"]["carpeta"].as_str().unwrap());
        reconstruir(&carpeta, &contexto["bitacoras"]["archivos"]);
        let (mut iguales, mut fallos) = (0, Vec::new());
        for e in excel["bitacoras"].as_array().unwrap() {
            let nombre = e["escenario"].as_str().unwrap();
            let manifiesto = if nombre == "contexto" {
                carpeta.join("bitacoras.json")
            } else {
                let c = base.join(nombre);
                reconstruir(&c, &e["archivos"]);
                c.join("bitacoras.json")
            };
            let destino = base.join(format!("{nombre}-bitacoras.xlsx"));
            let d = match exportar_bitacoras(&[manifiesto], &destino) {
                Ok(r) => volcado::diferencias(&volcado::volcar(&std::fs::read(r).unwrap(), &base.display().to_string()), &e["excel"]),
                Err(err) => vec![format!("exportar: {err}")],
            };
            if d.is_empty() { iguales += 1 } else { fallos.push(format!("bitacoras {nombre}:\n  {}", d.join("\n  "))) }
        }
        for e in excel["historias"].as_array().unwrap() {
            let destino = base.join("historias.xlsx");
            let d = match exportar_historias(&carpeta.join("historia").join("historias.json"), &destino) {
                Ok(r) => volcado::diferencias(&volcado::volcar(&std::fs::read(r).unwrap(), &base.display().to_string()), &e["excel"]),
                Err(err) => vec![format!("exportar: {err}")],
            };
            if d.is_empty() { iguales += 1 } else { fallos.push(format!("historias:\n  {}", d.join("\n  "))) }
        }
        // Nunca se sobrescribe un Excel.
        assert!(exportar_historias(&carpeta.join("historia").join("historias.json"), &base.join("historias.xlsx")).is_err());
        let _ = std::fs::remove_dir_all(&base);
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("EXCEL_BITACORAS: {iguales}/3, fallos {}", fallos.len());
        assert!(fallos.is_empty());
    }
}
