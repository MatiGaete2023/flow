//! Productos: Excel de observaciones, nóminas, borradores de correo (.eml sin enviar) y proyectos Word.
//! Nunca sobrescribe un archivo existente, nunca envía correos y nunca escribe en RUS.

use crate::config::{self, Config};
use crate::trabajo::{Registro, Trabajo};
use crate::util::*;
use base64::Engine;
use chrono::NaiveDate;
use regex::Regex;
use rust_xlsxwriter::{Format, Workbook};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
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
    static CONTADOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let c = CONTADOR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefijo}_{}_{}_{c}", std::process::id(), chrono::Local::now().timestamp_millis()))
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

fn paquete_plantilla_en(trib: &str, tipo: &str, dir_propia: Option<&Path>) -> Option<Vec<u8>> {
    let base = dir_propia.map(PathBuf::from).unwrap_or_else(|| config::data_dir().join("plantillas"));
    for cand in [base.join(format!("{trib}_{tipo}.docx")), base.join(trib).join(format!("{tipo}.docx"))] {
        if let Ok(datos) = std::fs::read(cand) {
            return Some(datos);
        }
    }
    let familia = match trib {
        "LAJA" => "LAJA",
        "MULCHEN" | "TOME" => "MULCHEN",
        _ => return None,
    };
    MATRICES.iter().find(|(t, k, _)| *t == familia && *k == tipo).map(|(_, _, d)| d.to_vec())
}

/// Paquete .docx del proyecto: plantilla propia del usuario si existe; si no, la matriz institucional.
fn paquete_plantilla(trib: &str, tipo: &str) -> Option<Vec<u8>> {
    paquete_plantilla_en(trib, tipo, None)
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
    quitar_cursivas_xml(&doc)
}

fn quitar_cursivas_xml(xml: &str) -> String {
    static RE_I: OnceLock<Regex> = OnceLock::new();
    RE_I.get_or_init(|| Regex::new(r#"<w:i(?:Cs)?(?:\s+w:val="(?:1|true|on)")?\s*/>"#).unwrap()).replace_all(xml, "").into_owned()
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
        } else if nombre == "word/styles.xml" || nombre.starts_with("word/header") || nombre.starts_with("word/footer") {
            if let Ok(xml) = String::from_utf8(datos.clone()) {
                datos = quitar_cursivas_xml(&xml).into_bytes();
            }
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

/// Formato al generar proyectos Word (decisión F14, E-P1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum FormatoWord {
    #[default]
    PorProyecto,
    PorTribunal,
}

/// Resultado de generar proyectos Word (carpeta creada y cada archivo con los índices de sus proyectos).
#[derive(Clone, Debug)]
pub struct ResultadoWord {
    pub carpeta: PathBuf,
    pub archivos: Vec<(PathBuf, Vec<usize>)>,
}

/// Etiqueta legible de tipo de proyecto (`kind_label` de Python).
pub fn tipo_etiqueta(tipo: &str) -> &str {
    TIPOS.iter().find(|(k, _)| *k == tipo).map(|(_, v)| *v).unwrap_or(tipo)
}

/// Líneas del índice al inicio de un documento por tribunal (título «Índice» + una línea por proyecto).
pub fn lineas_indice(grupo: &[&Proyecto]) -> Vec<String> {
    let mut out = Vec::with_capacity(1 + grupo.len());
    out.push("Índice".to_string());
    for (idx, p) in grupo.iter().enumerate() {
        out.push(format!("{}. {} — {}", idx + 1, p.rit, tipo_etiqueta(&p.tipo)));
    }
    out
}

fn docx_desde_cuerpo_generico(body: &str) -> Result<Vec<u8>, String> {
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

fn cuerpo_generico(p: &Proyecto) -> String {
    let mut body = String::new();
    for t in plantilla(&p.tribunal_clave, &p.tipo) {
        let txt = redactar(&t, &p.vars);
        body.push_str(&parrafo(&txt, txt == "VISTO:" || txt.starts_with("RIT"), false));
    }
    body
}

/// Documento mínimo para tribunales sin matriz institucional (formato anterior de Flow Rust).
fn docx_generico(p: &Proyecto) -> Result<Vec<u8>, String> {
    docx_desde_cuerpo_generico(&cuerpo_generico(p))
}

fn extraer_cuerpo_body(doc_xml: &str) -> Result<String, String> {
    let rb = regex::Regex::new(r"(?s)<w:body(?:\s[^>]*)?>(.*)</w:body>").unwrap();
    let caps = rb.captures(doc_xml).ok_or_else(|| "La plantilla Word no tiene cuerpo <w:body>.".to_string())?;
    let rs = regex::Regex::new(r"(?s)<w:sectPr(?:\s[^>]*)?(?:/>|>.*?</w:sectPr>)\s*$").unwrap();
    Ok(rs.replace(&caps[1], "").into_owned())
}

fn reemplazar_cuerpo_body(doc_xml: &str, nuevo_cuerpo: &str) -> Result<String, String> {
    let rb = regex::Regex::new(r"(?s)(<w:body(?:\s[^>]*)?>)(.*)(</w:body>)").unwrap();
    let caps = rb.captures(doc_xml).ok_or_else(|| "La plantilla Word no tiene cuerpo <w:body>.".to_string())?;
    let rs = regex::Regex::new(r"(?s)<w:sectPr(?:\s[^>]*)?(?:/>|>.*?</w:sectPr>)\s*$").unwrap();
    let sect_pr = rs.find(&caps[2]).map(|m| m.as_str()).unwrap_or("");
    let reemplazo = format!("{}{}{}{}", &caps[1], nuevo_cuerpo, sect_pr, &caps[3]);
    Ok(format!("{}{}{}", &doc_xml[..caps.get(0).unwrap().start()], reemplazo, &doc_xml[caps.get(0).unwrap().end()..]))
}

#[derive(Clone, Debug)]
struct RelOoxml {
    id: String,
    tipo: String,
    destino: String,
    externa: bool,
}

fn sha256_hex(datos: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(datos);
    format!("{:x}", hasher.finalize())
}

fn leer_zip_partes(paquete: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut entrada = zip::ZipArchive::new(std::io::Cursor::new(paquete))
        .map_err(|_| "La plantilla Word no es un .docx válido.".to_string())?;
    let mut partes = BTreeMap::new();
    for i in 0..entrada.len() {
        let mut archivo = entrada.by_index(i).map_err(|_| "La plantilla Word está dañada.".to_string())?;
        if archivo.is_dir() {
            continue;
        }
        let nombre = archivo.name().trim_start_matches('/').to_string();
        let mut datos = Vec::new();
        std::io::Read::read_to_end(&mut archivo, &mut datos).map_err(|_| "La plantilla Word está dañada.".to_string())?;
        partes.insert(nombre, datos);
    }
    Ok(partes)
}

fn extraer_attr(etiqueta: &str, attr: &str) -> Option<String> {
    let pat = format!(r#"(?:^|\s){attr}="([^"]*)""#, attr = regex::escape(attr));
    Regex::new(&pat).ok()?.captures(etiqueta).map(|c| c[1].to_string())
}

fn normalizar_ruta_zip(ruta: &str) -> String {
    let mut segs: Vec<&str> = Vec::new();
    for s in ruta.trim_start_matches('/').split('/') {
        match s {
            "" | "." => {}
            ".." => {
                segs.pop();
            }
            otro => segs.push(otro),
        }
    }
    segs.join("/")
}

fn carpeta_parte(ruta: &str) -> &str {
    ruta.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

fn resolver_target_zip(carpeta_padre: &str, target: &str) -> String {
    if target.starts_with('/') || carpeta_padre.is_empty() {
        normalizar_ruta_zip(target)
    } else {
        normalizar_ruta_zip(&format!("{carpeta_padre}/{target}"))
    }
}

fn ruta_rels_de_parte(ruta: &str) -> String {
    match ruta.rsplit_once('/') {
        Some((dir, archivo)) if !dir.is_empty() => format!("{dir}/_rels/{archivo}.rels"),
        _ => format!("_rels/{ruta}.rels"),
    }
}

fn target_relativo_desde(carpeta_padre: &str, ruta_zip: &str) -> String {
    if !carpeta_padre.is_empty() {
        if let Some(rest) = ruta_zip.strip_prefix(&format!("{carpeta_padre}/")) {
            return rest.to_string();
        }
    }
    format!("/{ruta_zip}")
}

fn parsear_rels(xml: &str) -> Vec<RelOoxml> {
    static RE_REL: OnceLock<Regex> = OnceLock::new();
    let re = RE_REL.get_or_init(|| Regex::new(r"(?s)<Relationship\b[^>]*>").unwrap());
    let mut out = Vec::new();
    for m in re.find_iter(xml) {
        let tag = m.as_str();
        let Some(id) = extraer_attr(tag, "Id") else { continue };
        let Some(tipo) = extraer_attr(tag, "Type") else { continue };
        let Some(destino) = extraer_attr(tag, "Target") else { continue };
        let externa = extraer_attr(tag, "TargetMode")
            .is_some_and(|v| v.eq_ignore_ascii_case("External"));
        out.push(RelOoxml { id, tipo, destino, externa });
    }
    out
}

fn serializar_rels(rels: &[RelOoxml]) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    for r in rels {
        xml.push_str("<Relationship Id=\"");
        xml.push_str(&xml_esc(&r.id));
        xml.push_str("\" Type=\"");
        xml.push_str(&xml_esc(&r.tipo));
        xml.push_str("\" Target=\"");
        xml.push_str(&xml_esc(&r.destino));
        xml.push('"');
        if r.externa {
            xml.push_str(" TargetMode=\"External\"");
        }
        xml.push_str("/>");
    }
    xml.push_str("</Relationships>");
    xml
}

fn siguiente_rid(rels: &[RelOoxml]) -> String {
    let max_n = rels
        .iter()
        .filter_map(|r| r.id.strip_prefix("rId")?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("rId{}", max_n + 1)
}

fn ruta_unica_parte(partes: &BTreeMap<String, Vec<u8>>, ruta_origen: &str) -> String {
    let normal = normalizar_ruta_zip(ruta_origen);
    if !partes.contains_key(&normal) {
        return normal;
    }
    let (dir, nombre) = normal.rsplit_once('/').unwrap_or(("", &normal));
    let (stem, ext) = nombre.rsplit_once('.').unwrap_or((nombre, ""));
    let base = stem.trim_end_matches(|c: char| c.is_ascii_digit());
    let base = if base.is_empty() { "parte" } else { base };
    let mut n = 1usize;
    loop {
        let cand_nombre = if ext.is_empty() {
            format!("{base}{n}")
        } else {
            format!("{base}{n}.{ext}")
        };
        let cand = if dir.is_empty() { cand_nombre } else { format!("{dir}/{cand_nombre}") };
        if !partes.contains_key(&cand) {
            return cand;
        }
        n += 1;
    }
}

fn tipo_mime_por_extension(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "rels" => "application/vnd.openxmlformats-package.relationships+xml",
        "xml" => "application/xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "emf" => "image/x-emf",
        "wmf" => "image/x-wmf",
        _ => "application/octet-stream",
    }
}

fn tipo_contenido_por_parte(ruta_zip: &str, src_ct: Option<&str>) -> Option<String> {
    let part_name = format!("/{}", normalizar_ruta_zip(ruta_zip));
    if let Some(ct) = src_ct {
        static RE_OV: OnceLock<Regex> = OnceLock::new();
        let re = RE_OV.get_or_init(|| Regex::new(r"(?s)<Override\b[^>]*>").unwrap());
        for m in re.find_iter(ct) {
            if extraer_attr(m.as_str(), "PartName").as_deref() == Some(part_name.as_str()) {
                if let Some(t) = extraer_attr(m.as_str(), "ContentType") {
                    return Some(t);
                }
            }
        }
    }
    let lower = part_name.to_ascii_lowercase();
    if lower == "/word/document.xml" {
        return Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml".into());
    }
    if lower == "/word/styles.xml" {
        return Some("application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml".into());
    }
    if lower == "/word/numbering.xml" {
        return Some("application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml".into());
    }
    if lower.starts_with("/word/header") && lower.ends_with(".xml") {
        return Some("application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml".into());
    }
    if lower.starts_with("/word/footer") && lower.ends_with(".xml") {
        return Some("application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml".into());
    }
    if lower == "/word/footnotes.xml" {
        return Some("application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml".into());
    }
    if lower == "/word/endnotes.xml" {
        return Some("application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml".into());
    }
    None
}

fn asegurar_content_type(ct_xml: &mut String, ruta_zip: &str, src_ct: Option<&str>) {
    let norm = normalizar_ruta_zip(ruta_zip);
    if norm.eq_ignore_ascii_case("[Content_Types].xml") {
        return;
    }
    let part_name = format!("/{norm}");
    let override_pat = format!(r#"(?i)PartName="{}""#, regex::escape(&part_name));
    if Regex::new(&override_pat).unwrap().is_match(ct_xml) {
        return;
    }
    let ext = norm.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if ext == "xml" {
        if let Some(ct) = tipo_contenido_por_parte(&norm, src_ct) {
            if let Some(pos) = ct_xml.rfind("</Types>") {
                ct_xml.insert_str(
                    pos,
                    &format!("<Override PartName=\"{}\" ContentType=\"{}\"/>", xml_esc(&part_name), xml_esc(&ct)),
                );
                return;
            }
        }
    }
    if !ext.is_empty() {
        let def_pat = format!(r#"(?i)<Default\b[^>]*Extension="{}""#, regex::escape(&ext));
        if !Regex::new(&def_pat).unwrap().is_match(ct_xml) {
            let mime = src_ct
                .and_then(|s| {
                    let re = Regex::new(&format!(r#"(?i)<Default\b[^>]*Extension="{}"[^>]*>"#, regex::escape(&ext))).ok()?;
                    re.find(s).and_then(|m| extraer_attr(m.as_str(), "ContentType"))
                })
                .unwrap_or_else(|| tipo_mime_por_extension(&ext).to_string());
            if let Some(pos) = ct_xml.rfind("</Types>") {
                ct_xml.insert_str(
                    pos,
                    &format!("<Default Extension=\"{}\" ContentType=\"{}\"/>", xml_esc(&ext), xml_esc(&mime)),
                );
            }
        }
    }
}

fn extraer_top_level_elementos(body: &str) -> Vec<String> {
    let mut elems = Vec::new();
    let mut pos = 0usize;
    let bytes = body.as_bytes();
    while pos < bytes.len() {
        let Some(rel_lt) = body[pos..].find('<') else { break };
        let start = pos + rel_lt;
        let rest = &body[start..];
        if rest.starts_with("<!--") {
            pos = rest.find("-->").map(|e| start + e + 3).unwrap_or(bytes.len());
            continue;
        }
        if rest.starts_with("<?") {
            pos = rest.find("?>").map(|e| start + e + 2).unwrap_or(bytes.len());
            continue;
        }
        // Conteo simple de profundidad de etiquetas hasta cerrar el elemento raíz actual.
        let mut depth = 0i32;
        let mut cur = start;
        let mut ended = bytes.len();
        while cur < bytes.len() {
            let Some(lt) = body[cur..].find('<') else { break };
            let tag_start = cur + lt;
            let Some(gt) = body[tag_start..].find('>') else { break };
            let tag_end = tag_start + gt + 1;
            let tag = &body[tag_start..tag_end];
            if tag.starts_with("<!--") {
                cur = body[tag_start..].find("-->").map(|e| tag_start + e + 3).unwrap_or(bytes.len());
                continue;
            }
            if tag.starts_with("</") {
                depth -= 1;
            } else if !tag.ends_with("/>") && !tag.starts_with("<?") {
                depth += 1;
            }
            cur = tag_end;
            if depth <= 0 {
                ended = cur;
                break;
            }
        }
        elems.push(body[start..ended].to_string());
        pos = ended;
    }
    elems
}

struct DocumentoCombinado {
    partes: BTreeMap<String, Vec<u8>>,
    doc_rels: Vec<RelOoxml>,
    media_por_sha256: BTreeMap<String, String>,
    content_types: String,
    styles_xml: String,
    numbering_xml: Option<String>,
    next_num_id: u32,
    next_anum_id: u32,
}

impl DocumentoCombinado {
    fn desde_base(base: &[u8]) -> Result<Self, String> {
        let mut partes = leer_zip_partes(base)?;
        let ct_bytes = partes.get("[Content_Types].xml").cloned().unwrap_or_else(|| {
            b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>".to_vec()
        });
        let content_types = String::from_utf8(ct_bytes).map_err(|_| "Content_Types no está en UTF-8.".to_string())?;
        let doc_rels = partes
            .get("word/_rels/document.xml.rels")
            .and_then(|d| std::str::from_utf8(d).ok())
            .map(parsear_rels)
            .unwrap_or_default();
        let mut media_por_sha256 = BTreeMap::new();
        for (ruta, datos) in &partes {
            if ruta.starts_with("word/media/") {
                media_por_sha256.entry(sha256_hex(datos)).or_insert_with(|| ruta.clone());
            }
        }
        for (ruta, datos) in partes.iter_mut() {
            if ruta.starts_with("word/header") || ruta.starts_with("word/footer") {
                if let Ok(xml) = std::str::from_utf8(datos) {
                    *datos = quitar_cursivas_xml(xml).into_bytes();
                }
            }
        }
        let styles_xml = partes
            .get("word/styles.xml")
            .and_then(|d| std::str::from_utf8(d).ok())
            .map(quitar_cursivas_xml)
            .unwrap_or_else(|| {
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"/>".into()
            });
        let numbering_xml = partes
            .get("word/numbering.xml")
            .and_then(|d| std::str::from_utf8(d).ok())
            .map(str::to_string);
        let (next_num_id, next_anum_id) = Self::calcular_siguientes_ids_numeracion(numbering_xml.as_deref());
        Ok(Self {
            partes,
            doc_rels,
            media_por_sha256,
            content_types,
            styles_xml,
            numbering_xml,
            next_num_id,
            next_anum_id,
        })
    }

    fn calcular_siguientes_ids_numeracion(num_xml: Option<&str>) -> (u32, u32) {
        let Some(xml) = num_xml else { return (1, 0) };
        static RE_NUM: OnceLock<Regex> = OnceLock::new();
        static RE_ANUM: OnceLock<Regex> = OnceLock::new();
        let re_num = RE_NUM.get_or_init(|| Regex::new(r#"<w:num\b[^>]*\bw:numId="(\d+)""#).unwrap());
        let re_anum = RE_ANUM.get_or_init(|| Regex::new(r#"<w:abstractNum\b[^>]*\bw:abstractNumId="(\d+)""#).unwrap());
        let next_num = re_num
            .captures_iter(xml)
            .filter_map(|c| c[1].parse::<u32>().ok())
            .max()
            .map_or(1, |m| m + 1);
        let next_anum = re_anum
            .captures_iter(xml)
            .filter_map(|c| c[1].parse::<u32>().ok())
            .max()
            .map_or(0, |m| m + 1);
        (next_num, next_anum)
    }

    fn asegurar_numbering_xml(&mut self) {
        if self.numbering_xml.is_some() {
            return;
        }
        self.numbering_xml = Some(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:numbering xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"></w:numbering>"
                .into(),
        );
        let tiene_rel = self.doc_rels.iter().any(|r| r.tipo.ends_with("/numbering"));
        if !tiene_rel {
            let rid = siguiente_rid(&self.doc_rels);
            self.doc_rels.push(RelOoxml {
                id: rid,
                tipo: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering".into(),
                destino: "numbering.xml".into(),
                externa: false,
            });
        }
        asegurar_content_type(&mut self.content_types, "word/numbering.xml", None);
    }

    fn copiar_parte_recursiva(
        &mut self,
        src_partes: &BTreeMap<String, Vec<u8>>,
        src_ct: Option<&str>,
        ruta_origen_zip: &str,
        rel_tipo: &str,
    ) -> String {
        let origen = normalizar_ruta_zip(ruta_origen_zip);
        if rel_tipo.ends_with("/image") {
            if let Some(bytes) = src_partes.get(&origen) {
                let hash = sha256_hex(bytes);
                if let Some(existente) = self.media_por_sha256.get(&hash) {
                    asegurar_content_type(&mut self.content_types, existente, src_ct);
                    return existente.clone();
                }
                let dest = ruta_unica_parte(&self.partes, &origen);
                self.media_por_sha256.insert(hash, dest.clone());
                self.partes.insert(dest.clone(), bytes.clone());
                asegurar_content_type(&mut self.content_types, &dest, src_ct);
                return dest;
            }
        }
        let dest = ruta_unica_parte(&self.partes, &origen);
        if let Some(raw) = src_partes.get(&origen) {
            let mut bytes = raw.clone();
            if dest.starts_with("word/header") || dest.starts_with("word/footer") {
                if let Ok(xml) = std::str::from_utf8(raw) {
                    bytes = quitar_cursivas_xml(xml).into_bytes();
                }
            }
            self.partes.insert(dest.clone(), bytes);
            asegurar_content_type(&mut self.content_types, &dest, src_ct);

            let src_rels_ruta = ruta_rels_de_parte(&origen);
            if let Some(src_rels_bytes) = src_partes.get(&src_rels_ruta) {
                if let Ok(src_rels_str) = std::str::from_utf8(src_rels_bytes) {
                    let src_dir = carpeta_parte(&origen);
                    let dst_dir = carpeta_parte(&dest);
                    let mut nuevas = Vec::new();
                    for mut r in parsear_rels(src_rels_str) {
                        if !r.externa {
                            let child_src = resolver_target_zip(src_dir, &r.destino);
                            let child_dst = self.copiar_parte_recursiva(src_partes, src_ct, &child_src, &r.tipo);
                            r.destino = target_relativo_desde(dst_dir, &child_dst);
                        }
                        nuevas.push(r);
                    }
                    let dst_rels_ruta = ruta_rels_de_parte(&dest);
                    self.partes.insert(dst_rels_ruta, serializar_rels(&nuevas).into_bytes());
                }
            }
        }
        dest
    }

    fn registrar_relacion_cuerpo(
        &mut self,
        src_partes: &BTreeMap<String, Vec<u8>>,
        src_ct: Option<&str>,
        rel: &RelOoxml,
    ) -> String {
        if rel.externa {
            if let Some(ex) = self.doc_rels.iter().find(|r| r.externa && r.tipo == rel.tipo && r.destino == rel.destino) {
                return ex.id.clone();
            }
            let rid = siguiente_rid(&self.doc_rels);
            self.doc_rels.push(RelOoxml { id: rid.clone(), tipo: rel.tipo.clone(), destino: rel.destino.clone(), externa: true });
            return rid;
        }
        let src_zip = resolver_target_zip("word", &rel.destino);
        let dst_zip = self.copiar_parte_recursiva(src_partes, src_ct, &src_zip, &rel.tipo);
        let target = target_relativo_desde("word", &dst_zip);
        if rel.tipo.ends_with("/image") {
            if let Some(ex) = self.doc_rels.iter().find(|r| !r.externa && r.tipo == rel.tipo && r.destino == target) {
                return ex.id.clone();
            }
        }
        let rid = siguiente_rid(&self.doc_rels);
        self.doc_rels.push(RelOoxml { id: rid.clone(), tipo: rel.tipo.clone(), destino: target, externa: false });
        rid
    }

    fn renumerar_relaciones_elemento(
        &mut self,
        elem: &str,
        src_partes: &BTreeMap<String, Vec<u8>>,
        src_rels: &[RelOoxml],
        src_ct: Option<&str>,
        rid_map: &mut BTreeMap<String, String>,
    ) -> String {
        static RE_ATTR_RID: OnceLock<Regex> = OnceLock::new();
        let re_rid = RE_ATTR_RID.get_or_init(|| Regex::new(r#"(r:(?:embed|id|link|dm|lo|qs|cs)=")([^"]+)(")"#).unwrap());
        let mut necesarios = Vec::new();
        for cap in re_rid.captures_iter(elem) {
            necesarios.push(cap[2].to_string());
        }
        for old_rid in necesarios {
            if rid_map.contains_key(&old_rid) {
                continue;
            }
            if let Some(rel) = src_rels.iter().find(|r| r.id == old_rid) {
                let new_rid = self.registrar_relacion_cuerpo(src_partes, src_ct, rel);
                rid_map.insert(old_rid, new_rid);
            }
        }
        if rid_map.is_empty() {
            return elem.to_string();
        }
        re_rid
            .replace_all(elem, |cap: &regex::Captures| {
                let new_id = rid_map.get(&cap[2]).map(String::as_str).unwrap_or(&cap[2]);
                format!("{}{}{}", &cap[1], new_id, &cap[3])
            })
            .into_owned()
    }

    fn insertar_abstract_num(&mut self, bloque: &str) {
        self.asegurar_numbering_xml();
        let num_xml = self.numbering_xml.as_mut().unwrap();
        if let Some(pos) = num_xml.find("<w:num ").or_else(|| num_xml.find("<w:num>")).or_else(|| num_xml.rfind("</w:numbering>")) {
            num_xml.insert_str(pos, bloque);
        }
    }

    fn insertar_num(&mut self, bloque: &str) {
        self.asegurar_numbering_xml();
        let num_xml = self.numbering_xml.as_mut().unwrap();
        if let Some(pos) = num_xml.rfind("</w:numbering>") {
            num_xml.insert_str(pos, bloque);
        }
    }

    fn buscar_num_en_xml(xml: &str, num_id: u32) -> Option<String> {
        let pat = format!(r#"(?s)<w:num\b[^>]*\bw:numId="{num_id}"[^>]*>.*?</w:num>"#);
        Regex::new(&pat).ok()?.find(xml).map(|m| m.as_str().to_string())
    }

    fn buscar_abstract_num_en_xml(xml: &str, anum_id: u32) -> Option<String> {
        let pat = format!(r#"(?s)<w:abstractNum\b[^>]*\bw:abstractNumId="{anum_id}"[^>]*>.*?</w:abstractNum>"#);
        Regex::new(&pat).ok()?.find(xml).map(|m| m.as_str().to_string())
    }

    fn formato_nivel_cero(num_xml: &str, num_id: u32) -> Option<String> {
        let num_bloque = Self::buscar_num_en_xml(num_xml, num_id)?;
        let re_aid = Regex::new(r#"<w:abstractNumId\b[^>]*\bw:val="(\d+)""#).ok()?;
        let aid = re_aid.captures(&num_bloque)?[1].parse::<u32>().ok()?;
        let anum_bloque = Self::buscar_abstract_num_en_xml(num_xml, aid)?;
        let re_lvl0 = Regex::new(r#"(?s)<w:lvl\b[^>]*\bw:ilvl="0"[^>]*>(.*?)</w:lvl>"#).ok()?;
        let lvl_body = &re_lvl0.captures(&anum_bloque)?[1];
        let re_fmt = Regex::new(r#"<w:numFmt\b[^>]*\bw:val="([^"]+)""#).ok()?;
        Some(re_fmt.captures(lvl_body)?[1].to_string())
    }

    fn mapear_num_id(
        &mut self,
        old_num_id: u32,
        src_num_xml: Option<&str>,
        proj_idx: usize,
        num_map: &mut BTreeMap<u32, u32>,
        anum_map: &mut BTreeMap<u32, u32>,
    ) {
        if old_num_id == 0 || num_map.contains_key(&old_num_id) {
            return;
        }
        let Some(src_xml) = src_num_xml else { return };
        let Some(src_num_bloque) = Self::buscar_num_en_xml(src_xml, old_num_id) else { return };
        let re_aid = Regex::new(r#"(<w:abstractNumId\b[^>]*\bw:val=")(\d+)(")"#).unwrap();
        let Some(cap_aid) = re_aid.captures(&src_num_bloque) else { return };
        let old_anum_id: u32 = cap_aid[2].parse().unwrap_or(0);

        let mapped_anum_id = if let Some(&ya) = anum_map.get(&old_anum_id) {
            ya
        } else {
            let Some(mut src_anum_bloque) = Self::buscar_abstract_num_en_xml(src_xml, old_anum_id) else {
                return;
            };
            let new_anum_id = self.next_anum_id;
            self.next_anum_id += 1;
            let re_set_aid = Regex::new(r#"(\bw:abstractNumId=")\d+(")"#).unwrap();
            src_anum_bloque = re_set_aid.replace(&src_anum_bloque, format!("${{1}}{new_anum_id}${{2}}")).into_owned();
            let nsid_hex = format!("{:08X}", 0x1000_0001u32.wrapping_add(new_anum_id * 7919 + proj_idx as u32 * 131));
            let re_nsid = Regex::new(r#"(<w:nsid\b[^>]*\bw:val=")[^"]*(")"#).unwrap();
            if re_nsid.is_match(&src_anum_bloque) {
                src_anum_bloque = re_nsid.replace(&src_anum_bloque, format!("${{1}}{nsid_hex}${{2}}")).into_owned();
            }
            self.insertar_abstract_num(&src_anum_bloque);
            anum_map.insert(old_anum_id, new_anum_id);
            new_anum_id
        };

        let new_num_id = self.next_num_id;
        self.next_num_id += 1;
        let re_set_nid = Regex::new(r#"(\bw:numId=")\d+(")"#).unwrap();
        let mut nuevo_num = re_set_nid.replace(&src_num_bloque, format!("${{1}}{new_num_id}${{2}}")).into_owned();
        nuevo_num = re_aid.replace(&nuevo_num, format!("${{1}}{mapped_anum_id}${{3}}")).into_owned();
        self.insertar_num(&nuevo_num);
        num_map.insert(old_num_id, new_num_id);
    }

    fn fusionar_numeracion_y_reinicio(
        &mut self,
        elem: &str,
        src_num_xml: Option<&str>,
        proj_idx: usize,
        num_map: &mut BTreeMap<u32, u32>,
        anum_map: &mut BTreeMap<u32, u32>,
        restarted_styles: &mut BTreeSet<String>,
    ) -> String {
        static RE_NUM_ID: OnceLock<Regex> = OnceLock::new();
        static RE_PSTYLE: OnceLock<Regex> = OnceLock::new();
        let re_nid = RE_NUM_ID.get_or_init(|| Regex::new(r#"(<w:numId\b[^>]*\bw:val=")(\d+)(")"#).unwrap());
        let encontrados: Vec<u32> = re_nid
            .captures_iter(elem)
            .filter_map(|c| c[2].parse::<u32>().ok())
            .collect();
        for old_id in encontrados {
            self.mapear_num_id(old_id, src_num_xml, proj_idx, num_map, anum_map);
        }
        let mut out = if num_map.is_empty() {
            elem.to_string()
        } else {
            re_nid
                .replace_all(elem, |cap: &regex::Captures| {
                    let old_id = cap[2].parse::<u32>().unwrap_or(0);
                    let new_id = num_map.get(&old_id).copied().unwrap_or(old_id);
                    format!("{}{new_id}{}", &cap[1], &cap[3])
                })
                .into_owned()
        };

        // `restart_first_numbering` de docxcompose: con pStyle sin outlineLvl y numFmt != "bullet",
        // crea un nuevo <w:num> con <w:startOverride w:val="1"/> en ilvl=0 y actualiza num_map.
        let re_pstyle = RE_PSTYLE.get_or_init(|| Regex::new(r#"<w:pStyle\b[^>]*\bw:val="([^"]+)""#).unwrap());
        let Some(cap_style) = re_pstyle.captures(&out) else {
            return out;
        };
        let style_id = cap_style[1].to_string();
        if restarted_styles.contains(&style_id) {
            return out;
        }
        let pat_style = format!(r#"(?s)<w:style\b[^>]*\bw:styleId="{}"[^>]*>.*?</w:style>"#, regex::escape(&style_id));
        let Ok(re_style) = Regex::new(&pat_style) else {
            return out;
        };
        let Some(m_style) = re_style.find(&self.styles_xml) else {
            return out;
        };
        let style_xml = m_style.as_str().to_string();
        if style_xml.contains("<w:outlineLvl") {
            return out;
        }
        let active_num_id = re_nid
            .captures(&out)
            .and_then(|c| c[2].parse::<u32>().ok())
            .filter(|&id| id > 0)
            .or_else(|| {
                re_nid
                    .captures(&style_xml)
                    .and_then(|c| c[2].parse::<u32>().ok())
                    .filter(|&id| id > 0)
            });
        let Some(prev_num_id) = active_num_id else {
            return out;
        };
        let Some(cur_num_xml) = self.numbering_xml.as_deref() else {
            return out;
        };
        let Some(fmt) = Self::formato_nivel_cero(cur_num_xml, prev_num_id) else {
            return out;
        };
        if fmt == "bullet" {
            return out;
        }
        let Some(orig_num_bloque) = Self::buscar_num_en_xml(cur_num_xml, prev_num_id) else {
            return out;
        };

        let restarted_id = self.next_num_id;
        self.next_num_id += 1;
        let re_set_nid = Regex::new(r#"(\bw:numId=")\d+(")"#).unwrap();
        let mut nuevo_num = re_set_nid.replace(&orig_num_bloque, format!("${{1}}{restarted_id}${{2}}")).into_owned();
        if !nuevo_num.contains("w:startOverride") {
            if let Some(pos) = nuevo_num.rfind("</w:num>") {
                nuevo_num.insert_str(
                    pos,
                    "<w:lvlOverride w:ilvl=\"0\"><w:startOverride w:val=\"1\"/></w:lvlOverride>",
                );
            }
        }
        self.insertar_num(&nuevo_num);
        for v in num_map.values_mut() {
            if *v == prev_num_id {
                *v = restarted_id;
            }
        }
        if re_nid.is_match(&out) {
            out = re_nid
                .replace(&out, format!("${{1}}{restarted_id}${{3}}"))
                .into_owned();
        } else if let Some(pos) = out.find("</w:pPr>") {
            out.insert_str(
                pos,
                &format!("<w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"{restarted_id}\"/></w:numPr>"),
            );
        }
        restarted_styles.insert(style_id);
        out
    }

    fn combinar_estilos_proyecto(
        &mut self,
        extra_styles_xml: Option<&str>,
        src_num_xml: Option<&str>,
        body_xml: &str,
        proj_idx: usize,
        num_map: &mut BTreeMap<u32, u32>,
        anum_map: &mut BTreeMap<u32, u32>,
    ) {
        let Some(extra) = extra_styles_xml else { return };
        static RE_STYLE: OnceLock<Regex> = OnceLock::new();
        static RE_NUM_ID: OnceLock<Regex> = OnceLock::new();
        static RE_USED_STYLE: OnceLock<Regex> = OnceLock::new();
        static RE_LINKED_STYLE: OnceLock<Regex> = OnceLock::new();
        let rs = RE_STYLE.get_or_init(|| Regex::new(r#"(?s)<w:style\b[^>]*w:styleId="([^"]+)"[^>]*>.*?</w:style>"#).unwrap());
        let re_nid = RE_NUM_ID.get_or_init(|| Regex::new(r#"(<w:numId\b[^>]*\bw:val=")(\d+)(")"#).unwrap());
        let re_used = RE_USED_STYLE.get_or_init(|| Regex::new(r#"<w:(?:pStyle|rStyle|tblStyle)\b[^>]*\bw:val="([^"]+)""#).unwrap());
        let re_linked = RE_LINKED_STYLE.get_or_init(|| Regex::new(r#"<w:(?:link|basedOn)\b[^>]*\bw:val="([^"]+)""#).unwrap());
        let disponibles: BTreeMap<String, String> = rs
            .captures_iter(extra)
            .map(|c| (c[1].to_string(), c[0].to_string()))
            .collect();
        let mut cola: Vec<String> = re_used.captures_iter(body_xml).map(|c| c[1].to_string()).collect();
        let mut visitados = BTreeSet::new();
        while let Some(id) = cola.pop() {
            if !visitados.insert(id.clone()) {
                continue;
            }
            let Some(raw) = disponibles.get(&id) else {
                continue;
            };
            for cap in re_linked.captures_iter(raw) {
                cola.push(cap[1].to_string());
            }
            if !self.styles_xml.contains(&format!("w:styleId=\"{id}\"")) {
                let mut bloque = quitar_cursivas_xml(raw);
                let encontrados: Vec<u32> = re_nid
                    .captures_iter(&bloque)
                    .filter_map(|c| c[2].parse::<u32>().ok())
                    .collect();
                for old_id in encontrados {
                    self.mapear_num_id(old_id, src_num_xml, proj_idx, num_map, anum_map);
                }
                if !num_map.is_empty() {
                    bloque = re_nid
                        .replace_all(&bloque, |c: &regex::Captures| {
                            let old_id = c[2].parse::<u32>().unwrap_or(0);
                            let new_id = num_map.get(&old_id).copied().unwrap_or(old_id);
                            format!("{}{new_id}{}", &c[1], &c[3])
                        })
                        .into_owned();
                }
                if let Some(pos) = self.styles_xml.rfind("</w:styles>") {
                    self.styles_xml.insert_str(pos, &bloque);
                }
            }
        }
        self.styles_xml = quitar_cursivas_xml(&self.styles_xml);
    }

    fn unir_proyecto(&mut self, paquete: &[u8], body_xml: &str, proj_idx: usize) -> Result<String, String> {
        let src_partes = leer_zip_partes(paquete)?;
        let src_ct = src_partes
            .get("[Content_Types].xml")
            .and_then(|d| std::str::from_utf8(d).ok())
            .map(str::to_string);
        let src_rels = src_partes
            .get("word/_rels/document.xml.rels")
            .and_then(|d| std::str::from_utf8(d).ok())
            .map(parsear_rels)
            .unwrap_or_default();
        let src_styles = src_partes
            .get("word/styles.xml")
            .and_then(|d| std::str::from_utf8(d).ok())
            .map(str::to_string);
        let src_num = src_partes
            .get("word/numbering.xml")
            .and_then(|d| std::str::from_utf8(d).ok())
            .map(str::to_string);

        let mut rid_map = BTreeMap::new();
        let mut num_map = BTreeMap::new();
        let mut anum_map = BTreeMap::new();
        let mut restarted_styles = BTreeSet::new();

        self.combinar_estilos_proyecto(src_styles.as_deref(), src_num.as_deref(), body_xml, proj_idx, &mut num_map, &mut anum_map);

        let mut unido = String::new();
        for elem in extraer_top_level_elementos(body_xml) {
            let elem = self.renumerar_relaciones_elemento(&elem, &src_partes, &src_rels, src_ct.as_deref(), &mut rid_map);
            let elem = self.fusionar_numeracion_y_reinicio(
                &elem,
                src_num.as_deref(),
                proj_idx,
                &mut num_map,
                &mut anum_map,
                &mut restarted_styles,
            );
            unido.push_str(&elem);
        }
        Ok(unido)
    }

    fn empaquetar(mut self, cuerpo_final: &str) -> Result<Vec<u8>, String> {
        let doc_bytes = self
            .partes
            .get("word/document.xml")
            .ok_or_else(|| "La plantilla Word no contiene word/document.xml.".to_string())?;
        let doc_xml = std::str::from_utf8(doc_bytes).map_err(|_| "La plantilla Word no está en UTF-8.".to_string())?;
        let doc_final = reemplazar_cuerpo_body(doc_xml, &quitar_cursivas_xml(cuerpo_final))?;
        self.partes.insert("word/document.xml".into(), doc_final.into_bytes());
        self.partes.insert("word/styles.xml".into(), quitar_cursivas_xml(&self.styles_xml).into_bytes());
        asegurar_content_type(&mut self.content_types, "word/styles.xml", None);

        if let Some(num_xml) = &self.numbering_xml {
            self.partes.insert("word/numbering.xml".into(), num_xml.as_bytes().to_vec());
            asegurar_content_type(&mut self.content_types, "word/numbering.xml", None);
        }
        if !self.doc_rels.is_empty() || self.partes.contains_key("word/_rels/document.xml.rels") {
            self.partes
                .insert("word/_rels/document.xml.rels".into(), serializar_rels(&self.doc_rels).into_bytes());
        }
        let rutas: Vec<String> = self.partes.keys().cloned().collect();
        for ruta in rutas {
            asegurar_content_type(&mut self.content_types, &ruta, None);
        }
        self.partes.insert("[Content_Types].xml".into(), self.content_types.into_bytes());

        let mut salida = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (nombre, datos) in self.partes {
            salida.start_file(nombre, o).map_err(|e| e.to_string())?;
            salida.write_all(&datos).map_err(|e| e.to_string())?;
        }
        Ok(salida.finish().map_err(|e| e.to_string())?.into_inner())
    }
}

fn leer_cuerpo_proyecto_paquete(paquete: &[u8], p: &Proyecto) -> Result<String, String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(paquete))
        .map_err(|_| "La plantilla Word no es un .docx válido.".to_string())?;
    let mut s = String::new();
    std::io::Read::read_to_string(
        &mut z.by_name("word/document.xml").map_err(|_| "La plantilla Word está dañada.".to_string())?,
        &mut s,
    )
    .map_err(|_| "La plantilla Word no está en UTF-8.".to_string())?;
    let doc = documento_xml(&s, p);
    extraer_cuerpo_body(&doc)
}

#[cfg(test)]
fn leer_xml_zip(paquete: &[u8], entrada: &str) -> Option<String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(paquete)).ok()?;
    let mut s = String::new();
    std::io::Read::read_to_string(&mut z.by_name(entrada).ok()?, &mut s).ok()?;
    Some(s)
}

fn indice_tribunal_xml(grupo: &[&Proyecto]) -> String {
    let mut xml = String::new();
    for (idx, linea) in lineas_indice(grupo).into_iter().enumerate() {
        xml.push_str(&parrafo(&linea, idx == 0, false));
    }
    xml
}

/// Un .docx por tribunal con índice al inicio y salto de página entre proyectos.
/// Combina relaciones, medios deduplicados, encabezados/pies, `numbering.xml` y `[Content_Types].xml`.
fn docx_por_tribunal(grupo: &[&Proyecto], dir_propia: Option<&Path>) -> Result<Vec<u8>, String> {
    const SALTO_PAGINA: &str = "<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>";
    let mut paquetes: Vec<(Vec<u8>, String)> = Vec::with_capacity(grupo.len());
    let mut todos_con_plantilla = true;
    for p in grupo {
        if let Some(paquete) = paquete_plantilla_en(&p.tribunal_clave, &p.tipo, dir_propia) {
            let cuerpo = leer_cuerpo_proyecto_paquete(&paquete, p)?;
            paquetes.push((paquete, cuerpo));
        } else {
            todos_con_plantilla = false;
            paquetes.push((docx_generico(p)?, cuerpo_generico(p)));
        }
    }
    if !todos_con_plantilla && paquetes.iter().all(|(_, _)| !grupo.iter().any(|p| paquete_plantilla_en(&p.tribunal_clave, &p.tipo, dir_propia).is_some())) {
        let mut combinado = indice_tribunal_xml(grupo);
        for (_, cuerpo) in &paquetes {
            combinado.push_str(SALTO_PAGINA);
            combinado.push_str(cuerpo);
        }
        return docx_desde_cuerpo_generico(&quitar_cursivas_xml(&combinado));
    }
    let mut combinado_doc = DocumentoCombinado::desde_base(&paquetes[0].0)?;
    let mut cuerpo_final = indice_tribunal_xml(grupo);
    cuerpo_final.push_str(SALTO_PAGINA);
    cuerpo_final.push_str(&paquetes[0].1);
    for (idx, (paq, cuerpo)) in paquetes.iter().enumerate().skip(1) {
        cuerpo_final.push_str(SALTO_PAGINA);
        cuerpo_final.push_str(&combinado_doc.unir_proyecto(paq, cuerpo, idx)?);
    }
    combinado_doc.empaquetar(&cuerpo_final)
}

fn generar_word_interno(
    proyectos: &[Proyecto],
    dir: &Path,
    fecha: NaiveDate,
    formato: FormatoWord,
    dir_propia: Option<&Path>,
) -> Result<ResultadoWord, String> {
    let sel: Vec<(usize, &Proyecto)> = proyectos.iter().enumerate().filter(|(_, p)| p.incluir).collect();
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
    let mut salidas: Vec<(String, Vec<usize>, Vec<u8>)> = Vec::new();
    match formato {
        FormatoWord::PorProyecto => {
            for (idx, p) in sel {
                let datos = match paquete_plantilla_en(&p.tribunal_clave, &p.tipo, dir_propia) {
                    Some(paquete) => docx_desde_plantilla(&paquete, p)?,
                    None => docx_generico(p)?,
                };
                salidas.push((format!("{} {} {}", p.tipo, p.rit, p.tribunal_clave), vec![idx], datos));
            }
        }
        FormatoWord::PorTribunal => {
            let mut por_tribunal: BTreeMap<&str, Vec<(usize, &Proyecto)>> = BTreeMap::new();
            for (idx, p) in sel {
                por_tribunal.entry(p.tribunal_clave.as_str()).or_default().push((idx, p));
            }
            for (trib, grupo) in por_tribunal {
                let proys: Vec<&Proyecto> = grupo.iter().map(|(_, p)| *p).collect();
                let indices: Vec<usize> = grupo.iter().map(|(i, _)| *i).collect();
                salidas.push((format!("Resoluciones {trib}"), indices, docx_por_tribunal(&proys, dir_propia)?));
            }
        }
    }
    let mut archivos = Vec::with_capacity(salidas.len());
    for (nombre, indices, datos) in salidas {
        let base = safe_filename(&nombre);
        let ruta = nueva_ruta(&carpeta, &base, "docx");
        std::fs::write(&ruta, datos).map_err(|e| format!("No se pudo guardar {}: {e}", ruta.display()))?;
        archivos.push((ruta, indices));
    }
    Ok(ResultadoWord { carpeta, archivos })
}

/// Genera un .docx por tribunal con índice al inicio y salto de página entre proyectos (E-P1).
pub fn generar_word_por_tribunal(proyectos: &[Proyecto], dir: &Path, fecha: NaiveDate) -> Result<ResultadoWord, String> {
    generar_word_interno(proyectos, dir, fecha, FormatoWord::PorTribunal, None)
}

/// Genera proyectos Word según el formato elegido: uno por proyecto o uno por tribunal.
pub fn generar_word_formato(proyectos: &[Proyecto], dir: &Path, fecha: NaiveDate, formato: FormatoWord) -> Result<ResultadoWord, String> {
    generar_word_interno(proyectos, dir, fecha, formato, None)
}

/// Un .docx por proyecto, con la matriz institucional exacta de su tribunal y tipo, en una carpeta nueva.
pub fn generar_word(proyectos: &[Proyecto], dir: &Path, fecha: NaiveDate) -> Result<PathBuf, String> {
    generar_word_interno(proyectos, dir, fecha, FormatoWord::PorProyecto, None).map(|r| r.carpeta)
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
            ("FECHA", "veinte de noviembre de dos mil veintiséis"),
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ParrafoInspeccion {
        texto: String,
        salto: bool,
        num_id: Option<String>,
        ilvl: Option<String>,
        num_fmt: Option<String>,
        reinicia: bool,
    }

    struct InspeccionDocx {
        parrafos: Vec<ParrafoInspeccion>,
        cursivas: bool,
        relaciones_validas: bool,
        medios: Vec<(String, String)>,
        partes: Vec<(String, String)>,
    }

    fn validar_relaciones_y_tipos(partes: &BTreeMap<String, Vec<u8>>) -> (Vec<(String, String)>, bool) {
        let Some(ct_bytes) = partes.get("[Content_Types].xml") else {
            return (Vec::new(), false);
        };
        let Ok(ct_xml) = std::str::from_utf8(ct_bytes) else {
            return (Vec::new(), false);
        };
        let re_ov = Regex::new(r"(?s)<Override\b[^>]*>").unwrap();
        let re_def = Regex::new(r"(?s)<Default\b[^>]*>").unwrap();
        let mut overrides = BTreeMap::new();
        for m in re_ov.find_iter(ct_xml) {
            if let (Some(pn), Some(ct)) = (extraer_attr(m.as_str(), "PartName"), extraer_attr(m.as_str(), "ContentType")) {
                overrides.insert(pn, ct);
            }
        }
        let mut defaults = BTreeMap::new();
        for m in re_def.find_iter(ct_xml) {
            if let (Some(ext), Some(ct)) = (extraer_attr(m.as_str(), "Extension"), extraer_attr(m.as_str(), "ContentType")) {
                defaults.insert(ext.to_ascii_lowercase(), ct);
            }
        }
        let mut lista_partes = Vec::new();
        let mut tipos_ok = true;
        for nombre in partes.keys() {
            if nombre == "[Content_Types].xml" {
                continue;
            }
            let pn = format!("/{nombre}");
            let ext = nombre.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
            let tipo = overrides
                .get(&pn)
                .cloned()
                .or_else(|| defaults.get(&ext).cloned())
                .unwrap_or_default();
            if tipo.is_empty() {
                tipos_ok = false;
            }
            lista_partes.push((nombre.clone(), tipo));
        }

        let mut rels_ok = partes
            .get("_rels/.rels")
            .and_then(|d| std::str::from_utf8(d).ok())
            .map(parsear_rels)
            .is_some_and(|rs| rs.iter().any(|r| !r.externa && partes.contains_key(&resolver_target_zip("", &r.destino))));
        let re_attr_rid = Regex::new(r#"\br:(?:embed|id|link|dm|lo|qs|cs)="([^"]+)""#).unwrap();
        let mut revisar: Vec<String> = vec!["word/document.xml".into()];
        for n in partes.keys() {
            if n.starts_with("word/header") || n.starts_with("word/footer") {
                revisar.push(n.clone());
            }
        }
        for parte in revisar {
            let Some(xml_bytes) = partes.get(&parte) else {
                rels_ok = false;
                break;
            };
            let Ok(xml) = std::str::from_utf8(xml_bytes) else {
                rels_ok = false;
                break;
            };
            let rels = partes
                .get(&ruta_rels_de_parte(&parte))
                .and_then(|d| std::str::from_utf8(d).ok())
                .map(parsear_rels)
                .unwrap_or_default();
            let dir_parte = carpeta_parte(&parte);
            for cap in re_attr_rid.captures_iter(xml) {
                let rid = &cap[1];
                let Some(rel) = rels.iter().find(|r| r.id == rid) else {
                    rels_ok = false;
                    break;
                };
                if !rel.externa && !partes.contains_key(&resolver_target_zip(dir_parte, &rel.destino)) {
                    rels_ok = false;
                    break;
                }
            }
            if !rels_ok {
                break;
            }
        }
        (lista_partes, tipos_ok && rels_ok)
    }

    fn extraer_secuencia_docx(datos: &[u8], exige_estilos: bool) -> InspeccionDocx {
        let partes = leer_zip_partes(datos).unwrap();
        let xml = std::str::from_utf8(partes.get("word/document.xml").unwrap()).unwrap().to_string();
        if exige_estilos {
            assert!(partes.contains_key("word/styles.xml"), "la matriz conserva sus estilos");
        }
        let re_italic = Regex::new(r#"<w:i(?:Cs)?(?:\s+w:val="(?:1|true|on)")?\s*/>"#).unwrap();
        let mut cursivas = re_italic.is_match(&xml);
        for (n, raw) in &partes {
            if n == "word/styles.xml" || n.starts_with("word/header") || n.starts_with("word/footer") {
                if let Ok(s) = std::str::from_utf8(raw) {
                    if re_italic.is_match(s) {
                        cursivas = true;
                    }
                }
            }
        }
        let num_xml = partes.get("word/numbering.xml").and_then(|d| std::str::from_utf8(d).ok());
        let (lista_partes, relaciones_validas) = validar_relaciones_y_tipos(&partes);
        let medios: Vec<(String, String)> = partes
            .iter()
            .filter(|(n, _)| n.starts_with("word/media/"))
            .map(|(n, d)| (n.clone(), sha256_hex(d)))
            .collect();

        let body = extraer_cuerpo_body(&xml).unwrap();
        let rp = Regex::new(r"(?s)<w:p[ >].*?</w:p>").unwrap();
        let re_numpr = Regex::new(r"(?s)<w:numPr\b[^>]*>(.*?)</w:numPr>").unwrap();
        let re_nid = Regex::new(r#"<w:numId\b[^>]*\bw:val="([^"]+)""#).unwrap();
        let re_ilvl = Regex::new(r#"<w:ilvl\b[^>]*\bw:val="([^"]+)""#).unwrap();
        let mut out = Vec::new();
        let mut pendiente_salto = false;
        for m in rp.find_iter(&body) {
            let p_xml = m.as_str();
            if p_xml.contains("w:type=\"page\"") || p_xml.contains("<w:pageBreakBefore") {
                pendiente_salto = true;
            }
            let txt = texto_xml(p_xml);
            if !txt.trim().is_empty() {
                let mut num_id = None;
                let mut ilvl = None;
                let mut num_fmt = None;
                let mut reinicia = false;
                if let Some(np) = re_numpr.captures(p_xml).map(|c| c[1].to_string()) {
                    num_id = re_nid.captures(&np).map(|c| c[1].to_string());
                    ilvl = re_ilvl.captures(&np).map(|c| c[1].to_string());
                    if let (Some(nx), Some(nid_str)) = (num_xml, num_id.as_deref().filter(|s| *s != "0")) {
                        if let Ok(nid_u) = nid_str.parse::<u32>() {
                            let nivel = ilvl.as_deref().unwrap_or("0");
                            if let Some(num_bloque) = DocumentoCombinado::buscar_num_en_xml(nx, nid_u) {
                                let pat_ov = format!(
                                    r#"(?s)<w:lvlOverride\b[^>]*\bw:ilvl="{}"[^>]*>.*?<w:startOverride\b[^>]*\bw:val="1""#,
                                    regex::escape(nivel)
                                );
                                reinicia = Regex::new(&pat_ov).unwrap().is_match(&num_bloque);
                                if let Some(aid) = Regex::new(r#"<w:abstractNumId\b[^>]*\bw:val="(\d+)""#)
                                    .unwrap()
                                    .captures(&num_bloque)
                                    .and_then(|c| c[1].parse::<u32>().ok())
                                {
                                    if let Some(anum) = DocumentoCombinado::buscar_abstract_num_en_xml(nx, aid) {
                                        let pat_lvl = format!(
                                            r#"(?s)<w:lvl\b[^>]*\bw:ilvl="{}"[^>]*>(.*?)</w:lvl>"#,
                                            regex::escape(nivel)
                                        );
                                        if let Some(cap_l) = Regex::new(&pat_lvl).unwrap().captures(&anum) {
                                            num_fmt = Regex::new(r#"<w:numFmt\b[^>]*\bw:val="([^"]+)""#)
                                                .unwrap()
                                                .captures(&cap_l[1])
                                                .map(|c| c[1].to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                out.push(ParrafoInspeccion {
                    texto: txt,
                    salto: pendiente_salto,
                    num_id,
                    ilvl,
                    num_fmt,
                    reinicia,
                });
                pendiente_salto = false;
            }
        }
        InspeccionDocx {
            parrafos: out,
            cursivas,
            relaciones_validas,
            medios,
            partes: lista_partes,
        }
    }

    fn comparar_word_tribunal(datos: &serde_json::Value, caso: &serde_json::Value) -> Vec<String> {
        use crate::motor::Mode;
        let mut diffs = Vec::new();
        let cfg = Config::default();
        let mut libro = Workbook::new();
        let hoja = libro.add_worksheet();
        let modo_str = caso["modo"].as_str().unwrap();
        hoja.set_name(modo_str).unwrap();
        for (c, h) in caso["encabezados"].as_array().unwrap().iter().enumerate() {
            hoja.write_string(2, c as u16, h.as_str().unwrap()).unwrap();
        }
        for (i, fila) in caso["filas"].as_array().unwrap().iter().enumerate() {
            for (c, v) in fila.as_array().unwrap().iter().enumerate() {
                match v.as_f64() {
                    Some(n) => hoja.write_number(3 + i as u32, c as u16, n).unwrap(),
                    None => hoja.write_string(3 + i as u32, c as u16, v.as_str().unwrap()).unwrap(),
                };
            }
        }
        let modo = match modo_str {
            "ESPERA" => Mode::Espera,
            "INFORMES" => Mode::Informes,
            _ => Mode::Cumplimiento,
        };
        let corte: NaiveDate = datos["corte"].as_str().unwrap().parse().unwrap();
        let mut w = Trabajo::abrir(PathBuf::from("origen.xlsx"), libro.save_to_buffer().unwrap(), Some(modo), corte, &cfg).unwrap();
        for (i, val) in caso["res"].as_array().into_iter().flatten().enumerate() {
            if let Some(s) = val.as_str().filter(|s| !s.is_empty()) {
                w.registros[i].res = s.to_string();
            }
        }
        for (i, val) in caso["editadas"].as_array().into_iter().flatten().enumerate() {
            if let Some(s) = val.as_str().filter(|s| !s.is_empty()) {
                w.registros[i].editada = Some(s.to_string());
            }
        }
        let ps = preparar_proyectos(&w, &cfg, false);

        let tmp = nueva_ruta_temporal("word_trib_oraculo");
        let dir_plantillas = tmp.join("plantillas");
        let dir_trib = tmp.join("trib");
        std::fs::create_dir_all(&dir_plantillas).unwrap();
        std::fs::create_dir_all(&dir_trib).unwrap();

        if let Some(propias) = caso["plantillas_propias"].as_object() {
            for (rel, b64) in propias {
                let bytes = base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap();
                let dest = dir_plantillas.join(rel);
                std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
                std::fs::write(&dest, bytes).unwrap();
            }
        }

        let res = generar_word_interno(&ps, &dir_trib, corte, FormatoWord::PorTribunal, Some(&dir_plantillas)).unwrap();
        let por_trib_esp = caso["por_tribunal"].as_object().unwrap();
        if res.archivos.len() != por_trib_esp.len() {
            diffs.push(format!("cantidad archivos: esp {} obt {}", por_trib_esp.len(), res.archivos.len()));
        }
        for (trib, esp_comb) in por_trib_esp {
            let ruta = res.carpeta.join(format!("{}.docx", safe_filename(&format!("Resoluciones {trib}"))));
            if !ruta.exists() {
                diffs.push(format!("falta archivo tribunal {trib}"));
                continue;
            }
            let raw_bytes = std::fs::read(&ruta).unwrap();
            let insp = extraer_secuencia_docx(&raw_bytes, true);
            if caso["nombre"] == "matriz_propia_del_usuario" {
                let styles = leer_xml_zip(&raw_bytes, "word/styles.xml").unwrap();
                assert!(styles.contains("EstiloPropio"), "la matriz propia conserva su estilo personalizado");
            }
            let cursivas_esp = esp_comb["cursivas"].as_bool().unwrap();
            if insp.cursivas != cursivas_esp {
                diffs.push(format!("{trib}: cursivas esp {cursivas_esp} obt {}", insp.cursivas));
            }
            let rels_esp = esp_comb["relaciones_validas"].as_bool().unwrap();
            if insp.relaciones_validas != rels_esp {
                diffs.push(format!("{trib}: relaciones_validas esp {rels_esp} obt {}", insp.relaciones_validas));
            }
            let medios_esp: Vec<(String, String)> = esp_comb["medios"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (v["ruta"].as_str().unwrap().to_string(), v["sha256"].as_str().unwrap().to_string()))
                .collect();
            if insp.medios != medios_esp {
                diffs.push(format!("{trib}: medios esp {medios_esp:?} obt {:?}", insp.medios));
            }
            let partes_esp: Vec<(String, String)> = esp_comb["partes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (v["ruta"].as_str().unwrap().to_string(), v["tipo"].as_str().unwrap().to_string()))
                .collect();
            if insp.partes != partes_esp {
                diffs.push(format!("{trib}: partes esp {partes_esp:?} obt {:?}", insp.partes));
            }

            let ind_esp: Vec<String> = esp_comb["indice"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
            if insp.parrafos.len() <= ind_esp.len() {
                diffs.push(format!("{trib}: faltan párrafos tras el índice ({})", insp.parrafos.len()));
                continue;
            }
            for (i, linea_esp) in ind_esp.iter().enumerate() {
                if insp.parrafos[i].texto != *linea_esp || insp.parrafos[i].salto {
                    diffs.push(format!("{trib} índice[{i}]: esp ({linea_esp:?}, false) obt {:?}", insp.parrafos[i]));
                }
            }
            let mut cuerpo = insp.parrafos[ind_esp.len()..].to_vec();
            if !cuerpo[0].salto {
                diffs.push(format!("{trib}: el primer proyecto no empieza tras salto de página"));
            }
            cuerpo[0].salto = false;
            let esp_parrafos: Vec<ParrafoInspeccion> = esp_comb["parrafos"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| ParrafoInspeccion {
                    texto: v["texto"].as_str().unwrap().to_string(),
                    salto: v["salto"].as_bool().unwrap(),
                    num_id: v["num_id"].as_str().map(str::to_string),
                    ilvl: v["ilvl"].as_str().map(str::to_string),
                    num_fmt: v["num_fmt"].as_str().map(str::to_string),
                    reinicia: v["reinicia"].as_bool().unwrap(),
                })
                .collect();
            if cuerpo != esp_parrafos {
                diffs.push(format!("{trib} cuerpo distinto a Python:\n  esp: {esp_parrafos:?}\n  obt: {cuerpo:?}"));
            }
        }
        let _ = std::fs::remove_dir_all(&tmp);
        diffs
    }

    #[test]
    fn word_por_tribunal_igual_a_python() {
        let datos: serde_json::Value = serde_json::from_str(include_str!("../tests/datos/word_tribunal.json")).unwrap();
        let hoy: NaiveDate = datos["hoy"].as_str().unwrap().parse().unwrap();
        crate::util::HOY_FIJO.with(|c| c.set(Some(hoy)));
        let casos = datos["casos"].as_array().unwrap();
        let mut total_archivos = 0;
        for caso in casos {
            let d = comparar_word_tribunal(&datos, caso);
            assert!(d.is_empty(), "{}: {:?}", caso["nombre"], d);
            total_archivos += caso["por_tribunal"].as_object().unwrap().len();
        }
        println!("WORD_TRIBUNAL: {}/{}, archivos {total_archivos}", casos.len(), casos.len());
        assert_eq!(casos.len(), 12);
        assert_eq!(total_archivos, 13);
    }

    #[test]
    fn cada_docx_generado_resuelve_relaciones_y_content_types() {
        let datos: serde_json::Value = serde_json::from_str(include_str!("../tests/datos/word_tribunal.json")).unwrap();
        let hoy: NaiveDate = datos["hoy"].as_str().unwrap().parse().unwrap();
        crate::util::HOY_FIJO.with(|c| c.set(Some(hoy)));
        let corte: NaiveDate = datos["corte"].as_str().unwrap().parse().unwrap();
        let cfg = Config::default();

        let mut total = 0usize;
        for caso in datos["casos"].as_array().unwrap() {
            let mut libro = Workbook::new();
            let hoja = libro.add_worksheet();
            let modo_str = caso["modo"].as_str().unwrap();
            hoja.set_name(modo_str).unwrap();
            for (c, h) in caso["encabezados"].as_array().unwrap().iter().enumerate() {
                hoja.write_string(2, c as u16, h.as_str().unwrap()).unwrap();
            }
            for (i, fila) in caso["filas"].as_array().unwrap().iter().enumerate() {
                for (c, v) in fila.as_array().unwrap().iter().enumerate() {
                    match v.as_f64() {
                        Some(n) => hoja.write_number(3 + i as u32, c as u16, n).unwrap(),
                        None => hoja.write_string(3 + i as u32, c as u16, v.as_str().unwrap()).unwrap(),
                    };
                }
            }
            let mut w = Trabajo::abrir(PathBuf::from("origen.xlsx"), libro.save_to_buffer().unwrap(), Some(crate::motor::Mode::Espera), corte, &cfg).unwrap();
            for (i, val) in caso["res"].as_array().into_iter().flatten().enumerate() {
                if let Some(s) = val.as_str().filter(|s| !s.is_empty()) {
                    w.registros[i].res = s.to_string();
                }
            }
            let ps = preparar_proyectos(&w, &cfg, false);
            let tmp = nueva_ruta_temporal("word_rels_check");
            let dir_p = tmp.join("p");
            let dir_o = tmp.join("o");
            std::fs::create_dir_all(&dir_p).unwrap();
            std::fs::create_dir_all(&dir_o).unwrap();
            if let Some(propias) = caso["plantillas_propias"].as_object() {
                for (rel, b64) in propias {
                    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap();
                    let dest = dir_p.join(rel);
                    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
                    std::fs::write(&dest, bytes).unwrap();
                }
            }
            let res = generar_word_interno(&ps, &dir_o, corte, FormatoWord::PorTribunal, Some(&dir_p)).unwrap();
            for (ruta, _) in &res.archivos {
                let mut partes = leer_zip_partes(&std::fs::read(ruta).unwrap()).unwrap();
                let (lista, ok) = validar_relaciones_y_tipos(&partes);
                assert!(ok, "{}: relaciones o [Content_Types].xml incompletos en {:?}", caso["nombre"], ruta);
                assert!(lista.iter().all(|(_, t)| !t.is_empty()));
                // Verificar que romper un rId en document.xml falla la comprobación.
                if let Some(doc) = partes.get_mut("word/document.xml") {
                    let xml = String::from_utf8(doc.clone()).unwrap();
                    *doc = xml.replace("<w:body>", "<w:body><w:p><w:r><a:blip r:embed=\"rId999\"/></w:r></w:p>").into_bytes();
                    let (_, ok_roto) = validar_relaciones_y_tipos(&partes);
                    assert!(!ok_roto, "debe detectar rId999 huérfano");
                }
                total += 1;
            }
            let _ = std::fs::remove_dir_all(&tmp);
        }
        assert_eq!(total, 13);
    }

    #[test]
    fn por_tribunal_soporta_tribunal_generico_y_excluidos() {
        let mut p1 = proyecto("OTRO", "PC_IE", false);
        p1.rit = "P-10-2099".into();
        let mut p2 = proyecto("OTRO", "PC_INFO", false);
        p2.rit = "P-11-2099".into();
        let mut p3 = proyecto("LAJA", "PC_IE", false);
        p3.incluir = false;
        let dir = nueva_ruta_temporal("word_gen");
        std::fs::create_dir_all(&dir).unwrap();
        let res = generar_word_por_tribunal(&[p1, p2, p3], &dir, "2026-11-20".parse().unwrap()).unwrap();
        assert_eq!(res.archivos.len(), 1);
        let insp = extraer_secuencia_docx(&std::fs::read(&res.archivos[0].0).unwrap(), false);
        assert!(!insp.cursivas);
        assert!(insp.relaciones_validas);
        assert_eq!(insp.parrafos[0].texto, "Índice");
        assert!(insp.parrafos[1].texto.contains("P-10-2099"));
        assert!(insp.parrafos[2].texto.contains("P-11-2099"));
        assert!(insp.parrafos[3].salto, "primer proyecto inicia tras salto");
        assert_eq!(insp.parrafos.iter().filter(|x| x.salto).count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn oraculo_de_word_tribunal_detecta_alteraciones() {
        let datos: serde_json::Value = serde_json::from_str(include_str!("../tests/datos/word_tribunal.json")).unwrap();
        let hoy: NaiveDate = datos["hoy"].as_str().unwrap().parse().unwrap();
        crate::util::HOY_FIJO.with(|c| c.set(Some(hoy)));
        // 1. Alterar un párrafo.
        let mut c1 = datos["casos"][1].clone();
        assert!(comparar_word_tribunal(&datos, &c1).is_empty());
        c1["por_tribunal"]["LAJA"]["parrafos"][0]["texto"] = serde_json::json!("Párrafo alterado");
        assert!(!comparar_word_tribunal(&datos, &c1).is_empty(), "un párrafo alterado no se detecta");
        // 2. Alterar un salto de página entre proyectos.
        let mut c2 = datos["casos"][1].clone();
        c2["por_tribunal"]["LAJA"]["parrafos"][10]["salto"] = serde_json::json!(false);
        assert!(!comparar_word_tribunal(&datos, &c2).is_empty(), "un salto alterado no se detecta");
        // 3. Alterar una línea del índice.
        let mut c3 = datos["casos"][1].clone();
        c3["por_tribunal"]["LAJA"]["indice"][1] = serde_json::json!("1. RIT-ALTERADO");
        assert!(!comparar_word_tribunal(&datos, &c3).is_empty(), "una línea del índice alterada no se detecta");
        // 4. Alterar validación de relación rId en plantilla con imagen.
        let idx_img = datos["casos"]
            .as_array()
            .unwrap()
            .iter()
            .position(|c| c["nombre"] == "plantilla_propia_con_imagen")
            .unwrap();
        let mut c4 = datos["casos"][idx_img].clone();
        assert!(comparar_word_tribunal(&datos, &c4).is_empty());
        c4["por_tribunal"]["LAJA"]["relaciones_validas"] = serde_json::json!(false);
        assert!(!comparar_word_tribunal(&datos, &c4).is_empty(), "una relación inválida no se detecta");
        // 5. Alterar formato / reinicio de numeración en proyectos consecutivos.
        let idx_num = datos["casos"]
            .as_array()
            .unwrap()
            .iter()
            .position(|c| c["nombre"] == "plantilla_lista_numerada_seguidos")
            .unwrap();
        let mut c5 = datos["casos"][idx_num].clone();
        assert!(comparar_word_tribunal(&datos, &c5).is_empty());
        c5["por_tribunal"]["MULCHEN"]["parrafos"][6]["num_fmt"] = serde_json::json!("bullet");
        assert!(!comparar_word_tribunal(&datos, &c5).is_empty(), "un formato de numeración alterado no se detecta");
        let mut c6 = datos["casos"][idx_num].clone();
        c6["por_tribunal"]["MULCHEN"]["parrafos"][6]["reinicia"] = serde_json::json!(false);
        assert!(!comparar_word_tribunal(&datos, &c6).is_empty(), "un reinicio de numeración alterado no se detecta");
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
