//! Productos: Excel de observaciones, nóminas, borradores de correo (.eml sin enviar) y proyectos Word.
//! Nunca sobrescribe un archivo existente, nunca envía correos y nunca escribe en RUS.

use crate::config::{self, Config};
use crate::trabajo::{Registro, Trabajo};
use crate::util::*;
use base64::Engine;
use chrono::NaiveDate;
use regex::Regex;
use rust_xlsxwriter::{Format, Workbook};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const MODALIDADES: [(&str, &str); 4] = [
    ("RES", "Cuidado alternativo residencial"),
    ("AMB", "Intervención ambulatoria de reparación"),
    ("FAE", "Cuidado alternativo familia de acogida"),
    ("DCE", "Diagnóstico Clínico Especializado (DCE)"),
];

/// Modalidad de un programa por su sigla inicial.
pub fn modalidad(programa: &str) -> &'static str {
    let name = normalizar(programa).to_uppercase();
    for (k, label) in MODALIDADES {
        if name == k || name == normalizar(label).to_uppercase() {
            return k;
        }
    }
    let first = name.split([' ', '-']).next().unwrap_or("");
    if first == "DCE" {
        return "DCE";
    }
    if first == "FAE" || first == "FAS" {
        return "FAE";
    }
    if ["REM", "RLP", "RMA", "RFA", "RDS", "RSP", "RPE"].iter().any(|p| first.starts_with(p))
        || ["RTA", "RTT", "RVA", "RES", "RESIDENCIA", "RESIDENCIAL"].contains(&first)
    {
        return "RES";
    }
    if ["AFT", "PF", "PRM", "PPF", "PIE", "PDE", "PEE", "PAS", "DAM", "OPD", "PDC"].contains(&first) {
        return "AMB";
    }
    ""
}

pub fn alcance_modalidades(keys: &[&str]) -> String {
    let sel: Vec<String> = MODALIDADES.iter().filter(|(k, _)| keys.contains(k)).map(|(_, l)| l.to_string()).collect();
    match sel.len() {
        0 => String::new(),
        1 => format!("la modalidad {}", sel[0]),
        4 => "todas las modalidades".into(),
        _ => format!("las modalidades {}", lista_y(&sel)),
    }
}

/// Carpeta temporal propia de una operación (se borra al terminar).
pub(crate) fn nueva_ruta_temporal(prefijo: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefijo}_{}_{}", std::process::id(), chrono::Local::now().timestamp_millis()))
}

/// Ruta nueva: añade « (2)», « (3)»… si el nombre ya existe.
pub fn nueva_ruta(dir: &Path, nombre: &str, ext: &str) -> PathBuf {
    let base = safe_filename(nombre);
    let mut p = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{base} ({n}).{ext}"));
        n += 1;
    }
    p
}

pub(crate) fn xlsx_libro(hojas: &[(&str, Vec<String>, Vec<Vec<String>>)], path: &Path) -> Result<(), String> {
    let mut wb = Workbook::new();
    let head = Format::new().set_bold().set_background_color(rust_xlsxwriter::Color::RGB(0xDDE7F5)).set_border_bottom(rust_xlsxwriter::FormatBorder::Thin);
    let wrap = Format::new().set_text_wrap().set_align(rust_xlsxwriter::FormatAlign::Top);
    for (name, headers, rows) in hojas {
        let ws = wb.add_worksheet();
        ws.set_name(*name).map_err(|e| e.to_string())?;
        for (c, h) in headers.iter().enumerate() {
            ws.write_string_with_format(0, c as u16, h, &head).map_err(|e| e.to_string())?;
            let width = rows.iter().map(|r| r.get(c).map(|s| s.chars().count()).unwrap_or(0)).max().unwrap_or(0).max(h.chars().count());
            ws.set_column_width(c as u16, (width as f64 + 2.0).min(70.0)).map_err(|e| e.to_string())?;
        }
        for (r, row) in rows.iter().enumerate() {
            for (c, v) in row.iter().enumerate() {
                // Siempre texto: nada se interpreta como fórmula.
                let v: String = v.chars().take(32767).collect();
                if v.chars().count() > 60 {
                    ws.write_string_with_format(r as u32 + 1, c as u16, &v, &wrap).map_err(|e| e.to_string())?;
                } else {
                    ws.write_string(r as u32 + 1, c as u16, &v).map_err(|e| e.to_string())?;
                }
            }
        }
        if !headers.is_empty() {
            ws.autofilter(0, 0, rows.len() as u32, headers.len() as u16 - 1).map_err(|e| e.to_string())?;
            ws.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
        }
    }
    wb.save(path).map_err(|e| format!("No se pudo guardar {}: {e}", path.display()))
}

fn estado_export(r: &Registro) -> &'static str {
    if r.excluido {
        "EXCLUIDO"
    } else if r.revisada {
        "REVISADO"
    } else if r.observacion().is_empty() {
        "SIN_OBSERVACION"
    } else {
        "PROPUESTA"
    }
}

/// Columnas de Trabajo y resultados vigentes del motor para las filas que se exportan.
pub fn tabla_observaciones<'a>(
    w: &Trabajo,
    registros: impl IntoIterator<Item = &'a Registro>,
) -> (Vec<String>, Vec<Vec<String>>) {
    let mut headers = w.headers.clone();
    let extra = ["OBSERVACION", "ESTADO_REVISION", "REGLAS", "AVISOS", "RES", "FILA_ORIGEN"];
    let pos: Vec<usize> = extra
        .iter()
        .map(|t| match headers.iter().position(|h| normalizar(h) == normalizar(t)) {
            Some(p) => p,
            None => {
                headers.push(t.to_string());
                headers.len() - 1
            }
        })
        .collect();
    let mut rows = Vec::new();
    for r in registros {
        let mut row: Vec<String> = r.values.iter().map(|c| c.display()).collect();
        row.resize(headers.len(), String::new());
        let marcas: Vec<String> = r.marcas.iter().map(|m| format!("N{} {}", m.nivel, m.texto)).collect();
        let vals = [
            r.observacion().to_string(),
            estado_export(r).to_string(),
            r.eventos.join(", "),
            r.avisos.iter().cloned().chain(marcas).collect::<Vec<_>>().join(" | "),
            r.res.clone(),
            r.fila_excel.to_string(),
        ];
        for (p, v) in pos.iter().zip(vals) {
            row[*p] = v;
        }
        rows.push(row);
    }
    (headers, rows)
}

/// Producto Excel con la estructura de Flow Python (hojas de servicio, todas las columnas y las de revisión).
pub fn excel_observaciones(w: &Trabajo, dir: &Path) -> Result<PathBuf, String> {
    crate::producto_excel::exportar(w, dir, None)
}

// ── correos ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Nomina {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub incluir: Vec<bool>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Borrador {
    pub kind: String,
    pub titulo: String,
    pub para: String,
    pub cc: String,
    pub asunto: String,
    pub cuerpo: String,
    pub programa: String,
    pub tribunal: String,
    pub nomina: Option<Nomina>,
    pub adjunto_obligatorio: bool,
    pub registros: Vec<usize>,
    pub guardado: Option<PathBuf>,
    pub incluir: bool,
    /// Huella de lo que el borrador lee (`product_state.fingerprint`); vacía si aún no se selló.
    #[serde(default)]
    pub dependencia: String,
    /// Archivos adjuntos (ruta y huella de lo que se revisó), además de la nómina.
    #[serde(default)]
    pub adjuntos: Vec<(String, String)>,
}

fn render_vars(t: &str, vars: &BTreeMap<&str, String>) -> String {
    let mut s = t.to_string();
    for (k, v) in vars {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

fn nomina_para(w: &Trabajo, kind: &str, regs: &[&Registro]) -> Nomina {
    let (headers, keys): (Vec<&str>, Vec<&str>) = match kind {
        "programa_espera" => (vec!["RIT", "TRIBUNAL", "NOMBRE", "DERIVACION", "T ESPERA"], vec!["rit", "tribunal", "nombre", "programa", "espera"]),
        "programa_vencido" | "programa_por_vencer" => (vec!["RIT", "TRIBUNAL", "NOMBRE", "DERIVACION", "F. VENCIMIENTO"], vec!["rit", "tribunal", "nombre", "programa", "vencimiento"]),
        _ => (vec!["RIT", "TRIBUNAL", "RUT", "NOMBRE", "PROGRAMA", "F. EGRESO PROYECTADO", "OBSERVACION"], vec!["rit", "tribunal", "rut", "nombre", "programa", "egreso_proy", "obs"]),
    };
    let rows: Vec<Vec<String>> = regs
        .iter()
        .map(|r| {
            keys.iter()
                .map(|k| match *k {
                    "obs" => r.observacion().to_string(),
                    "vencimiento" if kind == "programa_por_vencer" && w.mode() == crate::motor::Mode::Cumplimiento => {
                        if let Some(v) = r.anulaciones.get("vencimiento") {
                            let v = v.trim();
                            if v.is_empty() {
                                "Sin dato".into()
                            } else if let Some(d) = get_date(&Cell::Text(v.to_string())) {
                                d.format("%d/%m/%Y").to_string()
                            } else {
                                v.to_string()
                            }
                        } else {
                            r.fecha_hoja2.map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_else(|| "Sin dato".into())
                        }
                    }
                    "espera" | "vencimiento" | "egreso_proy" => {
                        let v = w.value(r, k);
                        if v.is_empty() { "Sin dato".into() } else if let Some(d) = w.date(r, k) { d.format("%d/%m/%Y").to_string() } else { v }
                    }
                    other => w.value(r, other),
                })
                .collect()
        })
        .collect();
    let n = rows.len();
    Nomina { headers: headers.into_iter().map(String::from).collect(), rows, incluir: vec![true; n] }
}

pub struct OpcionesCorreo<'a> {
    pub modalidades: &'a [&'a str],
    pub tribunales: &'a [String],
    pub solo_seleccionadas: bool,
}

fn fila_en_alcance(w: &Trabajo, r: &Registro, o: &OpcionesCorreo) -> bool {
    if o.solo_seleccionadas && !r.seleccionada {
        return false;
    }
    if !o.tribunales.is_empty() && !o.tribunales.contains(&w.tribunal_clave(r)) {
        return false;
    }
    if o.modalidades.len() < 4 {
        let m = modalidad(&w.value(r, "programa"));
        if !o.modalidades.contains(&m) {
            return false;
        }
    }
    true
}

/// La revisión humana puede retirar un correo automático: si el texto final ya no menciona el correo, no va.
fn permite_correo(r: &Registro) -> bool {
    let Some(final_) = &r.editada else { return true };
    let t = normalizar(final_);
    let negativos = ["no hay correo", "sin correo", "no corresponde correo", "no se remite correo", "no se envia correo", "no requiere correo"];
    !t.is_empty() && !negativos.iter().any(|n| t.contains(n)) && t.contains("correo")
}

/// Prepara los borradores automáticos (programas y medidas) y el informativo de la etapa a cada tribunal.
pub fn preparar_borradores(w: &Trabajo, cfg: &Config, o: &OpcionesCorreo) -> Vec<Borrador> {
    let fecha = crate::util::hoy().format("%d/%m/%Y").to_string();
    let alcance = alcance_modalidades(o.modalidades);
    let mut out = Vec::new();
    let regs: Vec<(usize, &Registro)> = w.registros.iter().enumerate().filter(|(_, r)| !r.excluido && fila_en_alcance(w, r, o)).collect();
    for kind in ["programa_espera", "programa_vencido", "programa_por_vencer", "medidas"] {
        let mut grupos: BTreeMap<(String, String), Vec<(usize, &Registro)>> = BTreeMap::new();
        for (i, r) in &regs {
            // Gestión por fila (`record_edits.mail_decision`): `omit` la saca; `include` la agrega aunque la regla no la pida.
            match r.gestiones_correo.get(kind).map(String::as_str) {
                Some("omit") => continue,
                Some("include") => {}
                _ if !r.acciones.contains(&kind) || (kind != "medidas" && !permite_correo(r)) => continue,
                _ => {}
            }
            let prog = if kind == "medidas" { String::new() } else { w.value(r, "programa") };
            grupos.entry((w.tribunal_clave(r), clave(&prog))).or_default().push((*i, r));
        }
        let Some(tpl) = cfg.plantillas.get(kind).filter(|t| !t.archivada) else { continue };
        for ((trib, _), items) in grupos {
            let first = items[0].1;
            let programa = if kind == "medidas" { String::new() } else { w.value(first, "programa") };
            let trib_nombre = cfg.tribunal_nombre(&trib, &w.value(first, "tribunal"));
            let mut vars = BTreeMap::new();
            vars.insert("TRIBUNAL", trib_nombre.clone());
            vars.insert("PROGRAMA", programa.clone());
            vars.insert("FECHA", fecha.clone());
            vars.insert("ALCANCE_MODALIDADES", if alcance.is_empty() { "todas las modalidades".into() } else { alcance.clone() });
            vars.insert("PERIODO", cfg.periodo.clone());
            let para = if tpl.destino == "programa" { cfg.contacto_programa(&programa) } else { cfg.tribunales.get(&trib).map(|t| t.para.clone()).unwrap_or_default() };
            let refs: Vec<&Registro> = items.iter().map(|(_, r)| *r).collect();
            out.push(Borrador {
                kind: kind.into(),
                titulo: if programa.is_empty() { format!("{} · {}", tpl.nombre, trib_nombre) } else { format!("{} · {}", programa, trib_nombre) },
                para,
                cc: cfg.cc(),
                asunto: render_vars(&tpl.asunto, &vars),
                cuerpo: con_firma(&render_vars(&tpl.cuerpo, &vars), cfg),
                programa,
                tribunal: trib_nombre,
                nomina: Some(nomina_para(w, kind, &refs)),
                adjunto_obligatorio: tpl.adjunto,
                registros: items.iter().map(|(i, _)| *i).collect(),
                guardado: None,
                incluir: true,
                dependencia: String::new(),
                adjuntos: Vec::new(),
            });
        }
    }
    // Informativo de la etapa a cada tribunal con registros revisados en el alcance.
    if let Some(tpl) = cfg.plantillas.get(w.mode().key()).filter(|t| !t.archivada) {
        let mut por_trib: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, r) in &regs {
            por_trib.entry(w.tribunal_clave(r)).or_default().push(*i);
        }
        for (trib, ids) in por_trib {
            if crate::util::tribunal(&trib).is_none() {
                continue;
            }
            let trib_nombre = cfg.tribunal_nombre(&trib, &trib);
            let mut vars = BTreeMap::new();
            vars.insert("TRIBUNAL", trib_nombre.clone());
            vars.insert("FECHA", fecha.clone());
            vars.insert("ALCANCE_MODALIDADES", if alcance.is_empty() { "todas las modalidades".into() } else { alcance.clone() });
            vars.insert("PERIODO", cfg.periodo.clone());
            out.push(Borrador {
                kind: w.mode().key().into(),
                titulo: format!("{} · {}", tpl.nombre, trib_nombre),
                para: cfg.tribunales.get(&trib).map(|t| t.para.clone()).unwrap_or_default(),
                cc: cfg.cc(),
                asunto: render_vars(&tpl.asunto, &vars),
                cuerpo: con_firma(&render_vars(&tpl.cuerpo, &vars), cfg),
                programa: String::new(),
                tribunal: trib_nombre,
                nomina: None,
                adjunto_obligatorio: false,
                registros: ids,
                guardado: None,
                incluir: true,
                dependencia: String::new(),
                adjuntos: Vec::new(),
            });
        }
    }
    out
}

/// Borrador libre (gestión particular u otra plantilla) para las filas dadas.
pub fn borrador_plantilla(w: Option<&Trabajo>, cfg: &Config, kind: &str, filas: &[usize]) -> Option<Borrador> {
    let tpl = cfg.plantillas.get(kind).filter(|t| !t.archivada)?;
    let fecha = crate::util::hoy().format("%d/%m/%Y").to_string();
    let (trib, trib_nombre) = match (w, filas.first()) {
        (Some(w), Some(i)) => {
            let r = &w.registros[*i];
            let k = w.tribunal_clave(r);
            let n = cfg.tribunal_nombre(&k, &w.value(r, "tribunal"));
            (k, n)
        }
        _ => (String::new(), "[TRIBUNAL]".to_string()),
    };
    let mut vars = BTreeMap::new();
    vars.insert("TRIBUNAL", trib_nombre.clone());
    vars.insert("FECHA", fecha);
    vars.insert("PERIODO", cfg.periodo.clone());
    vars.insert("ALCANCE_MODALIDADES", "todas las modalidades".to_string());
    vars.insert("PROGRAMA", String::new());
    let nomina = w.filter(|_| !filas.is_empty()).map(|w| {
        let refs: Vec<&Registro> = filas.iter().map(|i| &w.registros[*i]).collect();
        nomina_para(w, "medidas", &refs)
    });
    Some(Borrador {
        kind: kind.into(),
        titulo: format!("{} · {}", tpl.nombre, trib_nombre),
        para: cfg.tribunales.get(&trib).map(|t| t.para.clone()).unwrap_or_default(),
        cc: cfg.cc(),
        asunto: render_vars(&tpl.asunto, &vars),
        cuerpo: con_firma(&render_vars(&tpl.cuerpo, &vars), cfg),
        programa: String::new(),
        tribunal: trib_nombre,
        nomina,
        adjunto_obligatorio: tpl.adjunto,
        registros: filas.to_vec(),
        guardado: None,
        incluir: true,
        dependencia: String::new(),
        adjuntos: Vec::new(),
    })
}

fn con_firma(cuerpo: &str, cfg: &Config) -> String {
    if cfg.firma.trim().is_empty() { cuerpo.to_string() } else { format!("{cuerpo}\n\n{}", cfg.firma.trim()) }
}

fn enc_word(s: &str) -> String {
    if s.is_ascii() { s.to_string() } else { format!("=?utf-8?B?{}?=", base64::engine::general_purpose::STANDARD.encode(s.as_bytes())) }
}

fn b64_lines(data: &[u8]) -> String {
    let s = base64::engine::general_purpose::STANDARD.encode(data);
    s.as_bytes().chunks(76).map(|c| String::from_utf8_lossy(c).to_string()).collect::<Vec<_>>().join("\r\n")
}

/// Guarda el borrador como .eml sin enviar (Outlook lo abre como correo nuevo, listo para revisar).
/// El CC obligatorio siempre va. Devuelve la ruta del .eml.
pub fn guardar_eml(b: &Borrador, dir: &Path) -> Result<PathBuf, String> {
    if b.adjunto_obligatorio && b.nomina.as_ref().map_or(true, |n| !n.incluir.iter().any(|x| *x)) {
        return Err(format!("«{}» necesita una nómina con al menos una fila.", b.titulo));
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut cc = b.cc.clone();
    if !cc.to_lowercase().contains(config::CC_OBLIGATORIO) {
        cc = if cc.trim().is_empty() { config::CC_OBLIGATORIO.into() } else { format!("{}; {cc}", config::CC_OBLIGATORIO) };
    }
    let boundary = format!("flow-{}", chrono::Local::now().timestamp_nanos_opt().unwrap_or(0));
    let mut out = String::new();
    out.push_str("X-Unsent: 1\r\n");
    out.push_str(&format!("To: {}\r\n", b.para.replace('\n', " ")));
    out.push_str(&format!("Cc: {}\r\n", cc.replace('\n', " ")));
    out.push_str(&format!("Subject: {}\r\n", enc_word(&b.asunto)));
    out.push_str("MIME-Version: 1.0\r\n");
    out.push_str(&format!("Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n\r\n"));
    out.push_str(&format!("--{boundary}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n"));
    out.push_str(&b64_lines(b.cuerpo.replace("\r\n", "\n").replace('\n', "\r\n").as_bytes()));
    out.push_str("\r\n");
    if let Some(n) = &b.nomina {
        let rows: Vec<Vec<String>> = n.rows.iter().zip(&n.incluir).filter(|(_, i)| **i).map(|(r, _)| r.clone()).collect();
        if !rows.is_empty() {
            let nombre_adj = format!("{}.xlsx", safe_filename(if b.programa.is_empty() { &b.titulo } else { &b.programa }));
            let tmp = std::env::temp_dir().join(format!("flow_nomina_{}.xlsx", std::process::id()));
            xlsx_libro(&[("Nómina", n.headers.clone(), rows)], &tmp)?;
            let data = std::fs::read(&tmp).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&tmp);
            out.push_str(&format!(
                "--{boundary}\r\nContent-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet; name=\"{0}\"\r\nContent-Disposition: attachment; filename=\"{0}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n",
                enc_word(&nombre_adj)
            ));
            out.push_str(&b64_lines(&data));
            out.push_str("\r\n");
        }
    }
    crate::manual::verificar_adjuntos(b)?;
    for (ruta, _) in &b.adjuntos {
        let nombre = std::path::Path::new(ruta).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let datos = std::fs::read(ruta).map_err(|e| format!("No se pudo leer el adjunto {ruta}: {e}"))?;
        out.push_str(&format!(
            "--{boundary}\r\nContent-Type: application/octet-stream; name=\"{0}\"\r\nContent-Disposition: attachment; filename=\"{0}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n",
            enc_word(&nombre)
        ));
        out.push_str(&b64_lines(&datos));
        out.push_str("\r\n");
    }
    out.push_str(&format!("--{boundary}--\r\n"));
    let path = nueva_ruta(dir, &b.asunto, "eml");
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Guarda solo la nómina como Excel (para revisarla o enviarla por otra vía).
pub fn guardar_nomina(b: &Borrador, dir: &Path) -> Result<Option<PathBuf>, String> {
    let Some(n) = &b.nomina else { return Ok(None) };
    let rows: Vec<Vec<String>> = n.rows.iter().zip(&n.incluir).filter(|(_, i)| **i).map(|(r, _)| r.clone()).collect();
    if rows.is_empty() {
        return Ok(None);
    }
    let path = nueva_ruta(dir, &format!("Nomina {}", if b.programa.is_empty() { &b.titulo } else { &b.programa }), "xlsx");
    xlsx_libro(&[("Nómina", n.headers.clone(), rows)], &path)?;
    Ok(Some(path))
}

// ── resoluciones ─────────────────────────────────────────────────────────────

pub const TIPOS: [(&str, &str); 3] = [
    ("PC_IE", "PC_IE — Pide cuenta ingreso efectivo"),
    ("PC_INFO", "PC_INFO — Pide cuenta informe"),
    ("NOMENCL", "NOMENCL — Nomenclatura"),
];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Proyecto {
    pub tribunal: String,
    pub tribunal_clave: String,
    pub rit: String,
    pub tipo: String,
    pub vars: BTreeMap<String, String>,
    pub registros: Vec<usize>,
    pub incluir: bool,
    /// Huella de lo que el proyecto lee; vacía si aún no se selló.
    #[serde(default)]
    pub dependencia: String,
}

pub fn res_tipo(txt: &str) -> Option<&'static str> {
    let t = normalizar(txt).replace('_', " ");
    let t = t.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
    match t.as_str() {
        x if x.starts_with("pc ie") => Some("PC_IE"),
        x if x.starts_with("pc info") => Some("PC_INFO"),
        x if x.starts_with("nomencl") => Some("NOMENCL"),
        _ => None,
    }
}

fn nomenclatura(regs: &[&Registro]) -> String {
    const INFORME: &str = "Ingreso Informe de Cumplimiento X";
    const EGRESO: &str = "Egreso de servicio protección especializada y otras redes";
    static ENTRE_COMILLAS: OnceLock<Regex> = OnceLock::new();
    let entre_comillas = ENTRE_COMILLAS.get_or_init(|| {
        Regex::new(r#"(?i)nomenclatura\s*[«"“]\s*([^»"”\n]+?)\s*[»"”]"#).unwrap()
    });
    let mut ordenadas = regs.to_vec();
    ordenadas.sort_by_key(|r| {
        if res_tipo(&r.res) == Some("NOMENCL") {
            0
        } else if r.acciones.contains(&"NOMENCL") {
            1
        } else {
            2
        }
    });
    for r in ordenadas {
        if let Some(texto) = entre_comillas.captures(r.observacion()) {
            return texto[1].split_whitespace().collect::<Vec<_>>().join(" ");
        }
        let texto = normalizar(r.observacion());
        let egreso = texto.contains("egreso");
        let informe = ["ingreso informe", "informe de cumplimiento", "informe de avance"]
            .iter().any(|patron| texto.contains(patron));
        match (egreso, informe) {
            (true, true) => continue,
            (true, false) => return EGRESO.into(),
            (false, true) => return INFORME.into(),
            (false, false) if r.eventos.iter().any(|e| e == "HISTORIA.C10_ESCRITO") => {
                return INFORME.into();
            }
            _ => {}
        }
    }
    String::new()
}

/// Un proyecto por tribunal, RIT y tipo con todas las personas de la causa. Si el Excel trae RES, solo esas causas.
pub fn preparar_proyectos(w: &Trabajo, cfg: &Config, solo_seleccionadas: bool) -> Vec<Proyecto> {
    let con_res = w.registros.iter().any(|r| res_tipo(&r.res).is_some());
    let mut grupos: BTreeMap<(String, String, &'static str), Vec<usize>> = BTreeMap::new();
    for (i, r) in w.registros.iter().enumerate() {
        if r.excluido || (solo_seleccionadas && !r.seleccionada) {
            continue;
        }
        let tipos: Vec<&'static str> = if con_res {
            res_tipo(&r.res).into_iter().collect()
        } else {
            r.acciones.iter().copied().filter(|a| ["PC_IE", "PC_INFO", "NOMENCL"].contains(a)).collect()
        };
        for t in tipos {
            grupos.entry((w.tribunal_clave(r), clave(&w.value(r, "rit")), t)).or_default().push(i);
        }
    }
    grupos
        .into_iter()
        .map(|((trib, _, tipo), ids)| {
            let regs: Vec<&Registro> = ids.iter().map(|i| &w.registros[*i]).collect();
            let junta = |k: &str| {
                let mut v: Vec<String> = Vec::new();
                for r in &regs {
                    let x = if k == "nombre" { limpiar_nombre(&w.value(r, k)) } else { w.value(r, k) };
                    if !x.is_empty() && !v.contains(&x) {
                        v.push(x);
                    }
                }
                lista_y(&v)
            };
            let mut vars = BTreeMap::new();
            vars.insert("RIT".into(), w.value(regs[0], "rit"));
            vars.insert("NOMBRE".into(), junta("nombre"));
            vars.insert("RUT".into(), junta("rut"));
            vars.insert("PROGRAMA".into(), {
                let p: Vec<String> = regs.iter().map(|r| titulo_programa(&w.value(r, "programa"))).fold(vec![], |mut a, x| {
                    if !a.contains(&x) {
                        a.push(x)
                    }
                    a
                });
                lista_y(&p)
            });
            vars.insert("FECHA".into(), fecha_en_palabras(crate::util::hoy()));
            let mut fechas: Vec<String> = Vec::new();
            for f in regs.iter().filter_map(|r| w.date(r, "resolucion")).map(fecha_en_palabras) {
                if !fechas.contains(&f) {
                    fechas.push(f);
                }
            }
            vars.insert("FECHA_RESOLUCION".into(), lista_y(&fechas));
            vars.insert("DURACION".into(), junta("duracion"));
            let nombre_nomenclatura = if tipo == "NOMENCL" { nomenclatura(&regs) } else { String::new() };
            vars.insert("NOMENCLATURA".into(), nombre_nomenclatura);
            // `grouped_values`: personas distintas por nombre y RUT, con sus programas.
            let mut personas: Vec<(String, String, Vec<String>)> = Vec::new();
            for r in &regs {
                let (nombre, rut, programa) = (limpiar_nombre(&w.value(r, "nombre")), w.value(r, "rut"), w.value(r, "programa"));
                match personas.iter_mut().find(|(n, u, _)| clave(n) == clave(&nombre) && clave(u) == clave(&rut)) {
                    Some(p) => p.2.push(programa),
                    None => personas.push((nombre, rut, vec![programa])),
                }
            }
            let cedula = |n: &str, u: &str| format!("{n}, cédula de identidad N° {}", if u.is_empty() { "[COMPLETAR RUT]" } else { u });
            vars.insert("PERSONAS".into(), lista_y(&personas.iter().map(|(n, u, _)| cedula(n, u)).collect::<Vec<_>>()));
            vars.insert(
                "DETALLE_PERSONAS".into(),
                personas.iter().map(|(n, u, p)| format!("{}, programa {}", cedula(n, u), lista_y(p))).collect::<Vec<_>>().join("; "),
            );
            let programas: std::collections::BTreeSet<String> = regs.iter().map(|r| clave(&w.value(r, "programa"))).collect();
            let marca = |b: bool| if b { "1".to_string() } else { String::new() };
            vars.insert("_PLURAL".into(), marca(personas.len() > 1));
            vars.insert("_VARIOS_PROGRAMAS".into(), marca(programas.len() > 1));
            vars.insert("_VARIAS_FECHAS".into(), marca(fechas.len() > 1));
            Proyecto {
                tribunal: cfg.tribunal_nombre(&trib, &w.value(regs[0], "tribunal")),
                tribunal_clave: trib,
                rit: w.value(regs[0], "rit"),
                tipo: tipo.into(),
                vars,
                registros: ids,
                incluir: true,
                dependencia: String::new(),
            }
        })
        .collect()
}

fn ciudad(trib: &str) -> &'static str {
    match trib {
        "LAJA" => "Laja",
        "MULCHEN" => "Mulchén",
        "TOME" => "Tomé",
        _ => "[CIUDAD]",
    }
}

/// Texto de la resolución (párrafos). Laja y Mulchén según sus plantillas; Tomé con el formato de Mulchén.
fn plantilla(trib: &str, tipo: &str) -> Vec<String> {
    let c = ciudad(trib);
    let laja = trib == "LAJA";
    let mut p: Vec<String> = Vec::new();
    if laja {
        p.push("Laja, la fecha de la resolución es la que se indica en el pie de firma.".into());
    } else {
        p.push(format!("{c}, {{{{FECHA}}}}."));
    }
    match (tipo, laja) {
        ("PC_IE", true) => {
            p.push("VISTO:".into());
            p.push("Atendido al mérito de los antecedentes que obran en autos y de conformidad a lo dispuesto en los artículos 13 y 16 de la Ley N° 19.968 que crea Los Tribunales de Familia, se resuelve:".into());
            p.push("Ofíciese a programa {{PROGRAMA}} a fin de pedir cuenta del ingreso efectivo de {{NOMBRE}}, cédula de identidad N° {{RUT}}.".into());
            p.push("-Notifíquese vía correo al programa interventor y curador.".into());
            p.push("-Notifíquese a los restantes intervinientes por el estado diario.".into());
            p.push("Sirva la presente resolución de suficiente y atento oficio remisor.".into());
        }
        ("PC_INFO", true) => {
            p.push("VISTO:".into());
            p.push("Atendido al mérito de los antecedentes que obran en autos y de conformidad a lo dispuesto en los artículos 13 y 16 de la Ley N° 19.968 que crea Los Tribunales de Familia, se resuelve:".into());
            p.push("Ofíciese a programa {{PROGRAMA}} a fin de pedir cuenta del informe de cumplimiento pendiente de recepción de {{NOMBRE}}, cédula de identidad N° {{RUT}}.".into());
            p.push("-Notifíquese vía correo al programa interventor y curador.".into());
            // Texto literal de la plantilla original de Laja (no se corrige sin decisión del usuario).
            p.push("-Notifíquese a los restantes intervienes por el estado diario.".into());
            p.push("Sirva la presente resolución de suficiente y atento oficio remisor.".into());
        }
        ("NOMENCL", true) => {
            p.push("Advirtiendo el Tribunal que se omitió la incorporación de la nomenclatura \"{{NOMENCLATURA}}\", aplíquese a la presente resolución, para efectos de regularizar el correcto registro en el sistema de seguimiento de tramitación de causas.".into());
        }
        ("PC_IE", false) => {
            p.push("Advirtiendo el Tribunal que con fecha {{FECHA_RESOLUCION}} se ordenó el ingreso de {{NOMBRE}}, cédula de identidad N° {{RUT}} a {{PROGRAMA}} por el plazo de {{DURACION}}, no constando en la carpeta digital el ingreso efectivo de la persona aludida a la institución, pídase cuenta al programa para que informe con carácter de urgente, en el plazo de 5 días, bajo apercibimiento del artículo 238 del Código de Procedimiento Civil, si se efectuó el ingreso del niño sujeto de protección y, en la afirmativa, su fecha.".into());
            p.push("Notifíquese al programa interventor por correo electrónico.".into());
            p.push("Sirva la presente resolución como suficiente y atento oficio remisor.".into());
        }
        ("PC_INFO", false) => {
            p.push("Atendido que el plazo para la remisión del informe de avance pertinente se encuentra vencido y teniendo presente las facultades oficiosas del Tribunal contempladas en el artículo 13 de la Ley 19.968, se resuelve:".into());
            p.push("Pídase cuenta al organismo interventor, {{PROGRAMA}}, a fin de dar estricto cumplimiento a lo ordenado en la presente causa, en cuanto a la remisión del informe de cumplimiento, correspondiente a {{NOMBRE}}, cédula de identidad N° {{RUT}}.".into());
            p.push("Profesionales de dicho programa, deberán remitir dicho informe y dar cumplimiento en el término de 5 días desde su notificación o solicitar una prórroga para su entrega en caso de ser necesario.".into());
            p.push("Notifíquese la presente resolución por correo electrónico al programa interventor.".into());
            p.push("Sirva la presente resolución de suficiente y atento oficio remisor.".into());
        }
        _ => {
            p.push("Advirtiendo el Tribunal que se omitió la incorporación de la nomenclatura \"{{NOMENCLATURA}}\", aplíquese a la presente resolución para efectos de regularizar el correcto registro en el sistema de tramitación de causas.".into());
        }
    }
    p.push("RIT: {{RIT}}".into());
    if laja {
        p.push("Proveyó, Juez del juzgado de Letras y Garantía de Laja, quien suscribe con firma electrónica avanzada.".into());
        p.push("En Laja, con esta fecha, notifique por el estado diario la resolución que antecede.".into());
    } else {
        p.push("Se proveyó y firmó mediante firma electrónica avanzada, según lo dispuesto en la ley 20.886 y acta 71-2016 de la Excelentísima Corte Suprema.".into());
        p.push(format!("En {c}, {{{{FECHA}}}}, notifiqué por el estado diario la resolución precedente."));
    }
    p
}

fn llenar(t: &str, vars: &BTreeMap<String, String>) -> String {
    let mut s = t.to_string();
    for (k, v) in vars {
        let v = if v.trim().is_empty() { format!("[COMPLETAR {k}]") } else { v.clone() };
        s = s.replace(&format!("{{{{{k}}}}}"), &v);
    }
    s
}

fn xml_esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn parrafo(t: &str, bold: bool, center: bool) -> String {
    let jc = if center { "center" } else { "both" };
    let b = if bold { "<w:b/>" } else { "" };
    format!(
        "<w:p><w:pPr><w:spacing w:after=\"200\" w:line=\"360\" w:lineRule=\"auto\"/><w:jc w:val=\"{jc}\"/></w:pPr><w:r><w:rPr>{b}<w:rFonts w:ascii=\"Times New Roman\" w:hAnsi=\"Times New Roman\"/><w:sz w:val=\"24\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        xml_esc(t)
    )
}





/// Matrices institucionales de Flow Python (`plantillas_word`), incluidas en el ejecutable. Tomé usa las de Mulchén.
macro_rules! matriz {
    ($t:literal, $k:literal) => {
        ($t, $k, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/nurus/personal/plantillas_word/", $t, "/", $k, ".docx")).as_slice())
    };
}
pub(crate) const MATRICES: [(&str, &str, &[u8]); 6] = [
    matriz!("LAJA", "PC_IE"),
    matriz!("LAJA", "PC_INFO"),
    matriz!("LAJA", "NOMENCL"),
    matriz!("MULCHEN", "PC_IE"),
    matriz!("MULCHEN", "PC_INFO"),
    matriz!("MULCHEN", "NOMENCL"),
];

/// Paquete .docx del proyecto: plantilla propia del usuario si existe; si no, la matriz institucional.
fn paquete_plantilla(trib: &str, tipo: &str) -> Option<Vec<u8>> {
    let propia = config::data_dir().join("plantillas").join(format!("{trib}_{tipo}.docx"));
    if let Ok(datos) = std::fs::read(propia) {
        return Some(datos);
    }
    let familia = match trib {
        "LAJA" => "LAJA",
        "MULCHEN" | "TOME" => "MULCHEN",
        _ => return None,
    };
    MATRICES.iter().find(|(t, k, _)| *t == familia && *k == tipo).map(|(_, _, d)| d.to_vec())
}

fn texto_xml(p: &str) -> String {
    let rt = regex::Regex::new(r"(?s)<w:t(?:\s[^>]*)?>(.*?)</w:t>").unwrap();
    rt.captures_iter(p)
        .map(|t| t[1].to_string())
        .collect::<String>()
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Aplica `cambiar` al texto de cada párrafo; el que cambia se reescribe en un solo tramo con su formato original.
fn transformar_parrafos(xml: &str, cambiar: &dyn Fn(&str) -> String) -> String {
    let rp = regex::Regex::new(r"(?s)<w:p[ >].*?</w:p>").unwrap();
    let rppr = regex::Regex::new(r"(?s)<w:pPr>.*?</w:pPr>").unwrap();
    let rrpr = regex::Regex::new(r"(?s)<w:rPr>.*?</w:rPr>").unwrap();
    rp.replace_all(xml, |c: &regex::Captures| {
        let p = &c[0];
        let texto = texto_xml(p);
        let nuevo = cambiar(&texto);
        if nuevo == texto {
            return p.to_string();
        }
        let ppr = rppr.find(p).map(|m| m.as_str().to_string()).unwrap_or_default();
        let rpr = rrpr.find(&p.replace(&ppr, "")).map(|m| m.as_str().to_string()).unwrap_or_default();
        format!("<w:p>{ppr}<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>", xml_esc(&nuevo))
    })
    .to_string()
}

/// `render_project`: variables y, con varias personas, programas o fechas, la redacción plural de Flow Python.
fn redactar(texto: &str, vars: &BTreeMap<String, String>) -> String {
    let mut t = llenar(texto, vars);
    let activo = |k: &str| vars.get(k).is_some_and(|v| !v.is_empty());
    if activo("_PLURAL") {
        for marca in ["cédula de identidad N°", "cédula de identidad N.º"] {
            let par = format!("{}, {marca} {}", vars["NOMBRE"], vars["RUT"]);
            if t.contains(&par) {
                t = t.replace(&par, &vars["PERSONAS"].replace("cédula de identidad N°", marca));
            }
        }
        let mut cambios = vec![
            ("la persona aludida", "las personas individualizadas"),
            ("del niño sujeto de protección", "de los niños, niñas o adolescentes individualizados"),
            ("del niño, niña o adolescente", "de los niños, niñas o adolescentes"),
        ];
        if activo("_VARIOS_PROGRAMAS") {
            cambios.extend([
                ("al organismo interventor", "a los organismos interventores"),
                ("al programa interventor", "a los programas intervinientes"),
                ("a la institución", "a las instituciones correspondientes"),
                ("de dicho programa", "de dichos programas"),
            ]);
        }
        if activo("_VARIAS_FECHAS") {
            cambios.push(("con fecha ", "con fechas "));
        }
        for (viejo, nuevo) in cambios {
            t = t.replace(viejo, nuevo);
        }
    }
    t
}

/// `document.xml` del proyecto: variables, plural, detalle de personas con varios programas y sin cursivas.
fn documento_xml(xml: &str, p: &Proyecto) -> String {
    let mut doc = transformar_parrafos(xml, &|t| redactar(t, &p.vars));
    if p.vars.get("_PLURAL").is_some_and(|v| !v.is_empty()) && p.vars.get("_VARIOS_PROGRAMAS").is_some_and(|v| !v.is_empty()) {
        let detalle = format!(
            "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            xml_esc(&format!("Personas comprendidas: {}.", p.vars["DETALLE_PERSONAS"]))
        );
        let rp = regex::Regex::new(r"(?s)<w:p[ >].*?</w:p>").unwrap();
        let ancla = rp.find_iter(&doc).find(|m| texto_xml(m.as_str()).trim_start().starts_with("RIT")).map(|m| m.start());
        match ancla.or_else(|| doc.rfind("<w:sectPr")).or_else(|| doc.rfind("</w:body>")) {
            Some(pos) => doc.insert_str(pos, &detalle),
            None => doc.push_str(&detalle),
        }
    }
    // Decisión del 16-nov-2026 en Flow Python: ningún texto de la resolución sale en cursiva.
    regex::Regex::new(r#"<w:i(?:Cs)?(?:\s+w:val="(?:1|true|on)")?\s*/>"#).unwrap().replace_all(&doc, "").to_string()
}

/// Copia el paquete de la plantilla con su `document.xml` completado.
fn docx_desde_plantilla(paquete: &[u8], p: &Proyecto) -> Result<Vec<u8>, String> {
    let mut entrada = zip::ZipArchive::new(std::io::Cursor::new(paquete)).map_err(|_| "La plantilla Word no es un .docx válido.".to_string())?;
    let mut salida = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for i in 0..entrada.len() {
        let mut archivo = entrada.by_index(i).map_err(|_| "La plantilla Word está dañada.".to_string())?;
        let nombre = archivo.name().to_string();
        let mut datos = Vec::new();
        std::io::Read::read_to_end(&mut archivo, &mut datos).map_err(|_| "La plantilla Word está dañada.".to_string())?;
        if nombre == "word/document.xml" {
            let xml = String::from_utf8(datos).map_err(|_| "La plantilla Word no está en UTF-8.".to_string())?;
            datos = documento_xml(&xml, p).into_bytes();
        }
        salida.start_file(nombre, o).map_err(|e| e.to_string())?;
        salida.write_all(&datos).map_err(|e| e.to_string())?;
    }
    Ok(salida.finish().map_err(|e| e.to_string())?.into_inner())
}

/// Párrafos del proyecto ya completados, desde la misma plantilla que usará el Word.
pub fn texto_proyecto(p: &Proyecto) -> Vec<String> {
    if let Some(xml) = paquete_plantilla(&p.tribunal_clave, &p.tipo).and_then(|d| {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(d)).ok()?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut z.by_name("word/document.xml").ok()?, &mut s).ok()?;
        Some(s)
    }) {
        let rp = regex::Regex::new(r"(?s)<w:p[ >].*?</w:p>").unwrap();
        return rp.find_iter(&documento_xml(&xml, p)).map(|m| texto_xml(m.as_str())).collect();
    }
    plantilla(&p.tribunal_clave, &p.tipo).iter().map(|t| redactar(t, &p.vars)).collect()
}

/// Documento mínimo para tribunales sin matriz institucional (formato anterior de Flow Rust).
fn docx_generico(p: &Proyecto) -> Result<Vec<u8>, String> {
    let mut body = String::new();
    for t in plantilla(&p.tribunal_clave, &p.tipo) {
        let txt = redactar(&t, &p.vars);
        body.push_str(&parrafo(&txt, txt == "VISTO:" || txt.starts_with("RIT"), false));
    }
    let doc = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>{body}<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"18720\"/><w:pgMar w:top=\"1701\" w:right=\"1701\" w:bottom=\"1417\" w:left=\"1701\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>"
    );
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let ct = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
    for (name, data) in [("[Content_Types].xml", ct), ("_rels/.rels", rels), ("word/document.xml", doc.as_str())] {
        z.start_file(name, o).map_err(|e| e.to_string())?;
        z.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(z.finish().map_err(|e| e.to_string())?.into_inner())
}

/// Un .docx por proyecto, con la matriz institucional exacta de su tribunal y tipo, en una carpeta nueva.
pub fn generar_word(proyectos: &[Proyecto], dir: &Path, fecha: NaiveDate) -> Result<PathBuf, String> {
    let sel: Vec<&Proyecto> = proyectos.iter().filter(|p| p.incluir).collect();
    if sel.is_empty() {
        return Err("No hay proyectos marcados para generar.".into());
    }
    let mut carpeta = dir.join(format!("Resoluciones_{}", fecha.format("%Y-%m-%d")));
    let mut n = 2;
    while carpeta.exists() {
        carpeta = dir.join(format!("Resoluciones_{} ({n})", fecha.format("%Y-%m-%d")));
        n += 1;
    }
    std::fs::create_dir_all(&carpeta).map_err(|e| format!("No se pudo crear la carpeta de resoluciones: {e}"))?;
    for p in sel {
        let datos = match paquete_plantilla(&p.tribunal_clave, &p.tipo) {
            Some(paquete) => docx_desde_plantilla(&paquete, p)?,
            None => docx_generico(p)?,
        };
        let base = safe_filename(&format!("{} {} {}", p.tipo, p.rit, p.tribunal_clave));
        let ruta = nueva_ruta(&carpeta, &base, "docx");
        std::fs::write(&ruta, datos).map_err(|e| format!("No se pudo guardar {}: {e}", ruta.display()))?;
    }
    Ok(carpeta)
}

fn etiqueta_tipo(tipo: &str) -> &str {
    TIPOS.iter().find(|(clave, _)| *clave == tipo).map(|(_, etiqueta)| *etiqueta).unwrap_or(tipo)
}

fn cuerpo_documento(xml: &str) -> Result<(String, String), String> {
    let cuerpo = regex::Regex::new(r"(?s)<w:body>(.*?)</w:body>").unwrap();
    let capturado = cuerpo.captures(xml).ok_or("La matriz Word no contiene cuerpo.")?;
    let contenido = capturado[1].to_string();
    let seccion = regex::Regex::new(r"(?s)<w:sectPr(?:\s[^>]*)?>.*?</w:sectPr>").unwrap();
    let (contenido, pie) = if let Some(m) = seccion.find(&contenido) {
        (format!("{}{}", &contenido[..m.start()], &contenido[m.end()..]), m.as_str().to_string())
    } else { (contenido, String::new()) };
    Ok((contenido, pie))
}

fn combinar_estilos(estilos_base: &mut String, cuerpo: &mut String, estilos_origen: &str, sufijo: &str) {
    let rx_estilo = regex::Regex::new(r"(?s)<w:style\b.*?</w:style>").unwrap();
    let rx_id = regex::Regex::new(r#"w:styleId="([^"]+)""#).unwrap();
    let rx_ref = regex::Regex::new(r#"<w:(?:pStyle|rStyle|tblStyle)\b[^>]*w:val="([^"]+)"[^>]*/>"#).unwrap();
    let origen: BTreeMap<String, String> = rx_estilo.captures_iter(estilos_origen).filter_map(|c| {
        let id = rx_id.captures(&c[0])?.get(1)?.as_str().to_string();
        Some((id, c[0].to_string()))
    }).collect();
    let mut base: BTreeMap<String, String> = rx_estilo.captures_iter(estilos_base).filter_map(|c| {
        let id = rx_id.captures(&c[0])?.get(1)?.as_str().to_string();
        Some((id, c[0].to_string()))
    }).collect();
    let mut ids: Vec<String> = rx_ref.captures_iter(cuerpo).map(|c| c[1].to_string()).collect();
    let mut mapa = BTreeMap::new();
    let mut copiados = Vec::new();
    while let Some(id) = ids.pop() {
        if mapa.contains_key(&id) { continue; }
        let Some(style) = origen.get(&id) else { continue };
        let nuevo = match base.get(&id) {
            Some(actual) if actual != style => format!("{id}Flow{sufijo}"),
            _ => id.clone(),
        };
        mapa.insert(id.clone(), nuevo.clone());
        if !base.contains_key(&nuevo) {
            let style_nuevo = rx_id.replace(style, format!("w:styleId=\"{nuevo}\"")).into_owned();
            let rx_based = regex::Regex::new(r#"<w:(?:basedOn|next|link)\b[^>]*w:val="([^"]+)"[^>]*/>"#).unwrap();
            for c in rx_based.captures_iter(style) { ids.push(c[1].to_string()); }
            copiados.push(style_nuevo.clone());
            base.insert(nuevo, style_nuevo);
        }
    }
    for (id, nuevo) in &mapa {
        let rx = regex::Regex::new(&format!(r#"(w:(?:pStyle|rStyle|tblStyle)\b[^>]*w:val="){id}(\")"#)).unwrap();
        *cuerpo = rx.replace_all(cuerpo, format!("${{1}}{nuevo}${{2}}")).into_owned();
    }
    let rx_based = regex::Regex::new(r#"(<w:(?:basedOn|next|link)\b[^>]*w:val=")([^"]+)(\"[^>]*/>)"#).unwrap();
    for style in copiados {
        let style = rx_based.replace_all(&style, |c: &regex::Captures| {
            let id = &c[2];
            mapa.get(id).map(|mapped| format!("{}{mapped}{}", &c[1], &c[3])).unwrap_or_else(|| c[0].to_string())
        }).into_owned();
        if let Some(pos) = estilos_base.rfind("</w:styles>") { estilos_base.insert_str(pos, &style); }
    }
}

/// Un .docx por tribunal: índice fijo y proyectos completos separados por salto de página.
pub fn generar_word_tribunal(proyectos: &[Proyecto], dir: &Path, fecha: NaiveDate) -> Result<(PathBuf, Vec<(String, PathBuf)>), String> {
    let sel: Vec<&Proyecto> = proyectos.iter().filter(|p| p.incluir).collect();
    if sel.is_empty() { return Err("No hay proyectos marcados para generar.".into()); }
    let mut carpeta = dir.join(format!("Resoluciones_{}", fecha.format("%Y-%m-%d")));
    let mut n = 2;
    while carpeta.exists() { carpeta = dir.join(format!("Resoluciones_{} ({n})", fecha.format("%Y-%m-%d"))); n += 1; }
    std::fs::create_dir_all(&carpeta).map_err(|e| format!("No se pudo crear la carpeta de resoluciones: {e}"))?;
    let mut grupos: Vec<(&str, Vec<&Proyecto>)> = Vec::new();
    for p in sel {
        if let Some((_, grupo)) = grupos.iter_mut().find(|(trib, _)| *trib == p.tribunal_clave) { grupo.push(p); }
        else { grupos.push((&p.tribunal_clave, vec![p])); }
    }
    let mut rutas = Vec::new();
    for (tribunal, grupo) in grupos {
        let primero = grupo[0];
        let paquete = paquete_plantilla(tribunal, &primero.tipo);
        let (base, mut contenido, sect) = if let Some(bytes) = paquete {
            let rendered = docx_desde_plantilla(&bytes, primero)?;
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(rendered)).map_err(|e| e.to_string())?;
            let mut xml = String::new();
            std::io::Read::read_to_string(&mut zip.by_name("word/document.xml").map_err(|e| e.to_string())?, &mut xml).map_err(|e| e.to_string())?;
            let (body, sect) = cuerpo_documento(&xml)?;
            (bytes, body, sect)
        } else {
            let bytes = docx_generico(primero)?;
            let rendered = docx_desde_plantilla(&bytes, primero)?;
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(rendered)).map_err(|e| e.to_string())?;
            let mut xml = String::new();
            std::io::Read::read_to_string(&mut zip.by_name("word/document.xml").map_err(|e| e.to_string())?, &mut xml).map_err(|e| e.to_string())?;
            let (body, sect) = cuerpo_documento(&xml)?;
            (bytes, body, sect)
        };
        let mut indice = parrafo("Índice", true, true);
        for (i, p) in grupo.iter().enumerate() {
            indice.push_str(&parrafo(&format!("{}. {} — {}", i + 1, p.rit, etiqueta_tipo(&p.tipo)), false, false));
        }
        let mut estilos_xml = {
            let mut z = zip::ZipArchive::new(std::io::Cursor::new(&base)).map_err(|e| e.to_string())?;
            let mut styles = String::new();
            if let Ok(mut f) = z.by_name("word/styles.xml") { std::io::Read::read_to_string(&mut f, &mut styles).map_err(|e| e.to_string())?; }
            styles
        };
        // El índice precede el primer proyecto; el primer cuerpo ya viene renderizado.
        contenido = format!("{indice}<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>{contenido}");
        for p in grupo.iter().skip(1) {
            contenido.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
            let paquete = paquete_plantilla(tribunal, &p.tipo);
            let datos = match paquete { Some(ref b) => docx_desde_plantilla(b, p)?, None => docx_generico(p)? };
            let mut z = zip::ZipArchive::new(std::io::Cursor::new(datos)).map_err(|e| e.to_string())?;
            let mut xml = String::new();
            std::io::Read::read_to_string(&mut z.by_name("word/document.xml").map_err(|e| e.to_string())?, &mut xml).map_err(|e| e.to_string())?;
            let mut fragmento = cuerpo_documento(&xml)?.0;
            if let Ok(mut src_styles) = z.by_name("word/styles.xml") {
                let mut styles = String::new();
                std::io::Read::read_to_string(&mut src_styles, &mut styles).map_err(|e| e.to_string())?;
                combinar_estilos(&mut estilos_xml, &mut fragmento, &styles, &format!("{}", grupo.iter().position(|q| std::ptr::eq(*q, *p)).unwrap_or(0)));
            }
            contenido.push_str(&fragmento);
        }
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(base)).map_err(|e| e.to_string())?;
        let mut entradas = Vec::new();
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(|e| e.to_string())?;
            let nombre = f.name().to_string();
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut f, &mut bytes).map_err(|e| e.to_string())?;
            if nombre == "word/document.xml" {
                let xml = String::from_utf8(bytes).map_err(|e| e.to_string())?;
                let cuerpo = regex::Regex::new(r"(?s)<w:body>.*?</w:body>").unwrap();
                let replacement = format!("<w:body>{contenido}{sect}</w:body>");
                bytes = cuerpo.replace(&xml, replacement.as_str()).into_owned().into_bytes();
                let mut doc = String::from_utf8(bytes).map_err(|e| e.to_string())?;
                doc = regex::Regex::new(r#"<w:i(?:Cs)?(?:\s+w:val="(?:1|true|on)")?\s*/>"#).unwrap().replace_all(&doc, "").to_string();
                bytes = doc.into_bytes();
            } else if nombre == "word/styles.xml" && !estilos_xml.is_empty() {
                bytes = estilos_xml.as_bytes().to_vec();
            }
            entradas.push((nombre, bytes));
        }
        let mut salida = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (nombre, bytes) in entradas { salida.start_file(nombre, opts).map_err(|e| e.to_string())?; salida.write_all(&bytes).map_err(|e| e.to_string())?; }
        let ruta = nueva_ruta(&carpeta, &safe_filename(&format!("Resoluciones {tribunal}")), "docx");
        std::fs::write(&ruta, salida.finish().map_err(|e| e.to_string())?.into_inner()).map_err(|e| format!("No se pudo guardar {}: {e}", ruta.display()))?;
        rutas.push((tribunal.to_string(), ruta));
    }
    Ok((carpeta, rutas))
}

/// Reporte de proyectos (planilla para el correo «Reporte de proyectos»).
pub fn reporte_proyectos(proyectos: &[Proyecto], dir: &Path, periodo: &str) -> Result<PathBuf, String> {
    let headers = vec!["TRIBUNAL".into(), "RIT".into(), "TIPO".into(), "NOMBRE".into(), "PROGRAMA".into(), "PERIODO".into()];
    let rows = proyectos
        .iter()
        .filter(|p| p.incluir)
        .map(|p| vec![p.tribunal.clone(), p.rit.clone(), p.tipo.clone(), p.vars["NOMBRE"].clone(), p.vars["PROGRAMA"].clone(), periodo.to_string()])
        .collect();
    let path = nueva_ruta(dir, "Reporte de proyectos", "xlsx");
    xlsx_libro(&[("Proyectos", headers, rows)], &path)?;
    Ok(path)
}

pub fn escribir_xlsx(hojas: &[(&str, Vec<String>, Vec<Vec<String>>)], path: &Path) -> Result<(), String> {
    xlsx_libro(hojas, path)
}

#[cfg(test)]
mod pruebas_word {
    use super::*;

    fn proyecto(trib: &str, tipo: &str, plural: bool) -> Proyecto {
        let mut vars = BTreeMap::new();
        for (k, v) in [
            ("RIT", "P-1-2099"),
            ("NOMBRE", if plural { "Persona Ficticia Uno y Persona Ficticia Dos" } else { "Persona Ficticia Uno" }),
            ("RUT", if plural { "ID-FICTICIO-1 y ID-FICTICIO-2" } else { "ID-FICTICIO-1" }),
            ("PROGRAMA", if plural { "PRM Ficticio y PIE Ficticio" } else { "PRM Ficticio" }),
            ("FECHA", "veinticinco de noviembre de dos mil veintiséis"),
            ("FECHA_RESOLUCION", "catorce de septiembre de dos mil veintiséis"),
            ("DURACION", "12 meses"),
            ("NOMENCLATURA", "Nomenclatura ficticia"),
            ("PERSONAS", "Persona Ficticia Uno, cédula de identidad N° ID-FICTICIO-1 y Persona Ficticia Dos, cédula de identidad N° ID-FICTICIO-2"),
            ("DETALLE_PERSONAS", "Persona Ficticia Uno, cédula de identidad N° ID-FICTICIO-1, programa PRM Ficticio; Persona Ficticia Dos, cédula de identidad N° ID-FICTICIO-2, programa PIE Ficticio"),
            ("_PLURAL", if plural { "1" } else { "" }),
            ("_VARIOS_PROGRAMAS", if plural { "1" } else { "" }),
            ("_VARIAS_FECHAS", ""),
        ] {
            vars.insert(k.to_string(), v.to_string());
        }
        Proyecto { tribunal: trib.into(), tribunal_clave: trib.into(), rit: "P-1-2099".into(), tipo: tipo.into(), vars, registros: vec![0], incluir: true, dependencia: String::new() }
    }

    fn documento(datos: &[u8]) -> String {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(datos)).unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut z.by_name("word/document.xml").unwrap(), &mut s).unwrap();
        assert!(z.by_name("word/styles.xml").is_ok(), "la matriz conserva sus estilos");
        s
    }

    #[test]
    fn usa_la_matriz_institucional_con_fecha_en_palabras_y_sin_variables_pendientes() {
        for (trib, tipo) in [("LAJA", "PC_IE"), ("LAJA", "PC_INFO"), ("LAJA", "NOMENCL"), ("MULCHEN", "PC_IE"), ("MULCHEN", "PC_INFO"), ("TOME", "NOMENCL")] {
            let p = proyecto(trib, tipo, false);
            let xml = documento(&docx_desde_plantilla(&paquete_plantilla(trib, tipo).unwrap(), &p).unwrap());
            assert!(!xml.contains("{{"), "{trib} {tipo}");
            assert!(xml.contains("P-1-2099"), "{trib} {tipo}");
            assert!(!regex::Regex::new(r"<w:i\s*/>").unwrap().is_match(&xml));
        }
        let texto = texto_proyecto(&proyecto("MULCHEN", "PC_IE", false)).join("\n");
        assert!(texto.contains("catorce de septiembre de dos mil veintiséis"));
        assert!(paquete_plantilla("OTRO", "PC_IE").is_none());
    }

    #[test]
    fn redacta_en_plural_con_varias_personas_y_programas() {
        let texto = texto_proyecto(&proyecto("MULCHEN", "PC_IE", true)).join("\n");
        assert!(texto.contains("Persona Ficticia Dos, cédula de identidad N° ID-FICTICIO-2"));
        assert!(!texto.contains("la persona aludida"));
        assert!(texto.contains("Personas comprendidas: "));
    }

    fn comparar_word(expected: &serde_json::Value, xml: &str) -> Vec<String> {
        let mut fallas = Vec::new();
        let rp = regex::Regex::new(r"(?s)<w:p[ >].*?</w:p>").unwrap();
        let mut actual = Vec::new();
        let mut salto_pendiente = false;
        for m in rp.find_iter(xml) {
            let p = m.as_str();
            let texto = texto_xml(p);
            let salto = p.contains("<w:br") && p.contains("w:type=\"page\"");
            if texto.trim().is_empty() { salto_pendiente |= salto; }
            else { actual.push((texto, salto_pendiente)); salto_pendiente = false; }
        }
        let indice = expected["indice"].as_array().unwrap();
        let indice_real: Vec<String> = actual.iter().take(indice.len()).map(|(t, _)| t.clone()).collect();
        let indice_esperado: Vec<String> = indice.iter().map(|v| v.as_str().unwrap().to_string()).collect();
        if indice_real != indice_esperado { fallas.push(format!("índice esperado {indice_esperado:?}, recibido {indice_real:?}")); }
        let mut body = actual[indice.len().min(actual.len())..].to_vec();
        if let Some(first) = body.first_mut() {
            if first.1 != expected["salto_tras_indice"].as_bool().unwrap_or(false) { fallas.push("el primer proyecto no parte en página nueva después del índice".into()); }
            first.1 = false;
        }
        let previsto = expected["parrafos"].as_array().unwrap();
        let observado: Vec<serde_json::Value> = body.iter().map(|(texto, salto)| serde_json::json!({"texto":texto,"salto_previo":salto})).collect();
        if observado != *previsto { fallas.push(format!("párrafos o saltos difieren: esperado {previsto:?}, recibido {observado:?}")); }
        let cursiva = regex::Regex::new(r#"<w:i(?:Cs)?(?:\s+w:val="(?:1|true|on)")?\s*/>"#).unwrap().is_match(xml);
        if cursiva != expected["quedan_cursivas"].as_bool().unwrap() { fallas.push("estado de cursivas difiere".into()); }
        fallas
    }

    fn proyectos_caso(caso: &serde_json::Value) -> Vec<Proyecto> {
        caso["proyectos"].as_array().unwrap().iter().map(|s| {
            let trib = s["tribunal"].as_str().unwrap();
            let tipo = s["tipo"].as_str().unwrap();
            let mut p = proyecto(trib, tipo, false);
            p.rit = s["rit"].as_str().unwrap().to_string();
            p.vars.insert("RIT".into(), p.rit.clone());
            if s["editado"].as_bool().unwrap_or(false) { p.vars.insert("PROGRAMA".into(), "PRM revisado ficticio".into()); }
            p
        }).collect()
    }

    #[test]
    fn word_por_tribunal_igual_a_python() {
        let datos: serde_json::Value = serde_json::from_str(include_str!("../tests/datos/word_tribunal.json")).unwrap();
        let dir = std::env::temp_dir().join(format!("flow-word-tribunal-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // Simula una matriz propia de TOME; se restaura cualquier archivo previo al salir.
        let propia = crate::config::data_dir().join("plantillas/TOME_PC_INFO.docx");
        std::fs::create_dir_all(propia.parent().unwrap()).unwrap();
        let respaldo = std::fs::read(&propia).ok();
        let fuente_propia = MATRICES.iter().find(|(t, k, _)| *t == "MULCHEN" && *k == "PC_INFO").unwrap().2;
        std::fs::write(&propia, fuente_propia).unwrap();
        let mut total_archivos = 0;
        for caso in datos["casos"].as_array().unwrap() {
            let proyectos = proyectos_caso(caso);
            let (carpeta, rutas) = generar_word_tribunal(&proyectos, &dir, NaiveDate::from_ymd_opt(2026, 11, 25).unwrap()).unwrap();
            total_archivos += rutas.len();
            assert_eq!(rutas.len(), caso["archivos"].as_object().unwrap().len());
            for (trib, ruta) in rutas {
                let mut z = zip::ZipArchive::new(std::io::Cursor::new(std::fs::read(&ruta).unwrap())).unwrap();
                let mut xml = String::new();
                std::io::Read::read_to_string(&mut z.by_name("word/document.xml").unwrap(), &mut xml).unwrap();
                let mut estilos = String::new();
                if let Ok(mut f) = z.by_name("word/styles.xml") { std::io::Read::read_to_string(&mut f, &mut estilos).unwrap(); }
                let ids: std::collections::BTreeSet<String> = regex::Regex::new(r#"w:styleId="([^"]+)""#).unwrap().captures_iter(&estilos).map(|c| c[1].to_string()).collect();
                for c in regex::Regex::new(r#"<w:(?:pStyle|rStyle|tblStyle)\b[^>]*w:val="([^"]+)"[^>]*/>"#).unwrap().captures_iter(&xml) {
                    assert!(ids.contains(&c[1]), "referencia de estilo {} sin definición en {trib}", &c[1]);
                }
                let diferencias = comparar_word(&caso["archivos"][&trib], &xml);
                assert!(diferencias.is_empty(), "{} / {trib}: {diferencias:?}", caso["nombre"]);
            }
            std::fs::remove_dir_all(carpeta).unwrap();
        }
        assert_eq!(datos["casos"].as_array().unwrap().len(), 6);
        assert_eq!(total_archivos, 7);
        if let Some(bytes) = respaldo { std::fs::write(&propia, bytes).unwrap(); } else { let _ = std::fs::remove_file(&propia); }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn oraculo_de_word_tribunal_detecta_alteraciones() {
        let datos: serde_json::Value = serde_json::from_str(include_str!("../tests/datos/word_tribunal.json")).unwrap();
        let esperado = &datos["casos"][1]["archivos"]["LAJA"];
        let proyectos = proyectos_caso(&datos["casos"][1]);
        let dir = std::env::temp_dir().join(format!("flow-word-mutacion-{}", std::process::id()));
        let (carpeta, rutas) = generar_word_tribunal(&proyectos, &dir, NaiveDate::from_ymd_opt(2026, 11, 25).unwrap()).unwrap();
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(std::fs::read(&rutas[0].1).unwrap())).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut z.by_name("word/document.xml").unwrap(), &mut xml).unwrap();
        assert!(comparar_word(esperado, &xml).is_empty());
        for mutate in ["parrafo", "salto", "indice"] {
            let mut alterado = esperado.clone();
            match mutate {
                "parrafo" => alterado["parrafos"][0]["texto"] = serde_json::json!("texto alterado"),
                "salto" => alterado["parrafos"][1]["salto_previo"] = serde_json::json!(true),
                _ => alterado["indice"][1] = serde_json::json!("línea alterada"),
            }
            assert!(!comparar_word(&alterado, &xml).is_empty(), "no detectó alteración de {mutate}");
        }
        std::fs::remove_dir_all(carpeta).unwrap();
    }
}

/// E-A5 (F12): la preparación de correos de Rust contra `outputs.prepare_drafts` de Flow Python.
#[cfg(test)]
mod cotejo_correos {
    use super::*;
    use crate::motor::Mode;
    use serde_json::Value;

    fn texto(v: &Value) -> String {
        v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())
    }

    fn trabajo(datos: &Value, modo: &str, cfg: &Config) -> Trabajo {
        let t = &datos["trabajos"][modo];
        let mut libro = Workbook::new();
        let hoja = libro.add_worksheet();
        hoja.set_name("informe_1").unwrap();
        hoja.write_string(0, 0, "Reporte").unwrap();
        for (c, h) in t["encabezados"].as_array().unwrap().iter().enumerate() {
            hoja.write_string(2, c as u16, h.as_str().unwrap()).unwrap();
        }
        for (i, fila) in t["filas"].as_array().unwrap().iter().enumerate() {
            for (c, v) in fila.as_array().unwrap().iter().enumerate() {
                match v.as_f64() {
                    Some(n) => hoja.write_number(3 + i as u32, c as u16, n).unwrap(),
                    None => hoja.write_string(3 + i as u32, c as u16, v.as_str().unwrap()).unwrap(),
                };
            }
        }
        let modo = match modo {
            "ESPERA" => Mode::Espera,
            "CUMPLIMIENTO" => Mode::Cumplimiento,
            _ => Mode::Informes,
        };
        let corte = NaiveDate::parse_from_str(datos["corte"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        Trabajo::abrir(PathBuf::from("origen.xlsx"), libro.save_to_buffer().unwrap(), Some(modo), corte, cfg).unwrap()
    }

    fn configuracion(datos: &Value, caso: &Value) -> Config {
        let mut cfg = Config::default();
        cfg.contactos = datos["contactos"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
        for (k, v) in datos["tribunales"].as_object().unwrap() {
            cfg.tribunales.get_mut(k).unwrap().para = texto(v);
        }
        cfg.firma = if caso["firma"].as_bool().unwrap() { texto(&datos["firma"]) } else { String::new() };
        cfg.cc_adicional = if caso["cc_adicional"].as_bool().unwrap() { texto(&datos["cc_adicional"]) } else { String::new() };
        cfg
    }

    fn comparar(datos: &Value, caso: &Value) -> Vec<String> {
        let cfg = configuracion(datos, caso);
        let mut w = trabajo(datos, caso["modo"].as_str().unwrap(), &cfg);
        if let Some(rits) = caso["rits"].as_array() {
            for r in w.registros.iter_mut() {
                r.seleccionada = rits.iter().any(|x| x == &Value::String(w_rit(&r.values, &caso["modo"], datos)));
            }
        }
        for d in caso["decisiones"].as_array().into_iter().flatten() {
            let (rit, tipo, decision) = (texto(&d[0]), texto(&d[1]), texto(&d[2]));
            let r = w.registros.iter_mut().find(|r| w_rit(&r.values, &caso["modo"], datos) == rit).unwrap();
            r.gestiones_correo.insert(tipo, decision);
        }
        let todas = ["RES", "AMB", "FAE", "DCE"];
        let claves: Vec<String> = caso["modalidades"].as_array().map(|a| a.iter().map(texto).collect()).unwrap_or_default();
        let modalidades: Vec<&str> = if claves.is_empty() { todas.to_vec() } else { todas.iter().copied().filter(|k| claves.contains(&k.to_string())).collect() };
        let tribunales: Vec<String> = caso["tribunales"].as_array().map(|a| a.iter().map(texto).collect()).unwrap_or_default();
        let o = OpcionesCorreo { modalidades: &modalidades, tribunales: &tribunales, solo_seleccionadas: caso["rits"].is_array() };
        let obtenidos = preparar_borradores(&w, &cfg, &o);
        let rit = |b: &Borrador| {
            let mut v: Vec<String> = b.registros.iter().map(|i| w.value(&w.registros[*i], "rit")).collect();
            v.sort();
            v
        };
        let mut rust: Vec<Value> = obtenidos
            .iter()
            .map(|b| serde_json::json!({"tipo": b.kind, "asunto": b.asunto, "para": b.para, "cc": b.cc, "cuerpo": b.cuerpo, "rits": rit(b), "obligatorio": b.adjunto_obligatorio}))
            .collect();
        rust.sort_by_key(|b| (texto(&b["tipo"]), texto(&b["asunto"])));
        // Diferencia declarada (ESTADO.md): la plantilla «Cumplimiento» de Python dice «modulo» sin tilde; Flow Rust dice «módulo».
        let python: Vec<Value> = caso["borradores"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| {
                let cuerpo = texto(&b["cuerpo"]);
                let cuerpo = if b["tipo"] == "cumplimiento" { cuerpo.replace("del modulo RUS", "del módulo RUS") } else { cuerpo };
                serde_json::json!({"tipo": b["tipo"], "asunto": b["asunto"], "para": b["para"], "cc": b["cc"], "cuerpo": cuerpo, "rits": b["rits"], "obligatorio": b["obligatorio"]})
            })
            .collect();
        let mut d = Vec::new();
        if rust.len() != python.len() {
            let cab = |v: &[Value]| v.iter().map(|b| format!("{}|{}|{}", texto(&b["tipo"]), texto(&b["asunto"]), b["rits"])).collect::<Vec<_>>();
            d.push(format!("cantidad Rust {} · Python {}\n   Rust {:?}\n   Python {:?}", rust.len(), python.len(), cab(&rust), cab(&python)));
        }
        for (r, p) in rust.iter().zip(&python) {
            for k in ["tipo", "asunto", "para", "cc", "cuerpo", "rits", "obligatorio"] {
                if r[k] != p[k] {
                    d.push(format!("{} · {k}: Rust {} · Python {}", texto(&p["asunto"]), r[k], p[k]));
                }
            }
        }
        d
    }

    fn w_rit(valores: &[Cell], modo: &Value, datos: &Value) -> String {
        let _ = (modo, datos);
        valores.first().map(Cell::display).unwrap_or_default()
    }

    #[test]
    fn correos_iguales_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/correos.json")).unwrap();
        // El generador fija `date.today()` de Python en esa fecha: el oráculo no depende del día en que se corre.
        let hoy: NaiveDate = datos["hoy"].as_str().unwrap().parse().unwrap();
        crate::util::HOY_FIJO.with(|c| c.set(Some(hoy)));
        let (mut iguales, mut fallos) = (0, Vec::new());
        let casos = datos["casos"].as_array().unwrap();
        for caso in casos {
            let d = comparar(&datos, caso);
            if d.is_empty() {
                iguales += 1;
            } else {
                fallos.push(format!("{}:\n  {}", caso["nombre"], d.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("CORREOS: {iguales}/{}, fallos {}", casos.len(), fallos.len());
        let sin_tilde = casos.iter().flat_map(|c| c["borradores"].as_array().unwrap()).filter(|b| b["tipo"] == "cumplimiento" && texto(&b["cuerpo"]).contains("del modulo RUS")).count();
        println!("DIFERENCIA_DECLARADA modulo/módulo: {sin_tilde} correos de Cumplimiento");
        assert!(sin_tilde > 0, "la diferencia declarada debe estar en el oráculo");
        assert_eq!(casos.len(), 24, "el oráculo debe traer 24 casos");
        assert!(fallos.is_empty());
    }

    #[test]
    fn oraculo_de_correos_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/correos.json")).unwrap();
        let mut caso = datos["casos"][0].clone();
        let antes = comparar(&datos, &caso);
        assert!(antes.is_empty(), "{antes:?}");
        caso["borradores"][0]["para"] = Value::String("otro@ejemplo.invalid".into());
        assert!(!comparar(&datos, &caso).is_empty(), "un destinatario esperado alterado no se detecta");
        // F08: la fila que Python sacó del correo con `omit` debe seguir fuera.
        let mut gestion = datos["casos"].as_array().unwrap().iter().find(|c| c["nombre"] == "espera_omite_un_programa").unwrap().clone();
        assert!(comparar(&datos, &gestion).is_empty());
        let b = gestion["borradores"].as_array_mut().unwrap().iter_mut().find(|b| b["tipo"] == "programa_espera").unwrap();
        b["rits"] = serde_json::json!(["X-1-2026", "X-2-2026"]);
        assert!(!comparar(&datos, &gestion).is_empty(), "una gestión de correo esperada alterada no se detecta");
    }
}
