//! Productos: Excel de observaciones, nóminas, borradores de correo (.eml sin enviar) y proyectos Word.
//! Nunca sobrescribe un archivo existente, nunca envía correos y nunca escribe en RUS.

use crate::config::{self, Config};
use crate::trabajo::{Registro, Trabajo};
use crate::util::*;
use base64::Engine;
use chrono::NaiveDate;
use rust_xlsxwriter::{Format, Workbook};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

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

fn xlsx_libro(hojas: &[(&str, Vec<String>, Vec<Vec<String>>)], path: &Path) -> Result<(), String> {
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

/// Excel de observaciones: las columnas originales del libro + OBSERVACION, ESTADO, REGLAS, AVISOS y RES.
pub fn excel_observaciones(w: &Trabajo, dir: &Path) -> Result<PathBuf, String> {
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
    for r in &w.registros {
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
    let fecha = w.corte.map(|c| c.format("%Y-%m-%d").to_string()).unwrap_or_default();
    let path = nueva_ruta(dir, &format!("Observaciones_{}_{}", w.mode().label().to_uppercase(), fecha), "xlsx");
    xlsx_libro(&[(w.mode().label(), headers, rows)], &path)?;
    Ok(path)
}

// ── correos ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Nomina {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub incluir: Vec<bool>,
}

#[derive(Clone, Debug)]
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
                        r.fecha_hoja2.map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_else(|| "Sin dato".into())
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
    let fecha = chrono::Local::now().format("%d/%m/%Y").to_string();
    let alcance = alcance_modalidades(o.modalidades);
    let mut out = Vec::new();
    let regs: Vec<(usize, &Registro)> = w.registros.iter().enumerate().filter(|(_, r)| !r.excluido && fila_en_alcance(w, r, o)).collect();
    for kind in ["programa_espera", "programa_vencido", "programa_por_vencer", "medidas"] {
        let mut grupos: BTreeMap<(String, String), Vec<(usize, &Registro)>> = BTreeMap::new();
        for (i, r) in &regs {
            if !r.acciones.contains(&kind) || (kind != "medidas" && !permite_correo(r)) {
                continue;
            }
            let prog = if kind == "medidas" { String::new() } else { w.value(r, "programa") };
            grupos.entry((w.tribunal_clave(r), clave(&prog))).or_default().push((*i, r));
        }
        let Some(tpl) = cfg.plantillas.get(kind) else { continue };
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
            });
        }
    }
    // Informativo de la etapa a cada tribunal con registros revisados en el alcance.
    if let Some(tpl) = cfg.plantillas.get(w.mode().key()) {
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
            });
        }
    }
    out
}

/// Borrador libre (gestión particular u otra plantilla) para las filas dadas.
pub fn borrador_plantilla(w: Option<&Trabajo>, cfg: &Config, kind: &str, filas: &[usize]) -> Option<Borrador> {
    let tpl = cfg.plantillas.get(kind)?;
    let fecha = chrono::Local::now().format("%d/%m/%Y").to_string();
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
    out.push_str(&format!("--{boundary}--\r\n"));
    let path = nueva_ruta(dir, &b.asunto, "eml");
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Guarda el borrador en la carpeta Borradores de Outlook (Windows, Outlook clásico) mediante su interfaz COM.
/// Solo llama a Save(): nunca envía. El CC obligatorio siempre va.
#[cfg(target_os = "windows")]
pub fn guardar_outlook(b: &Borrador) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    if b.adjunto_obligatorio && b.nomina.as_ref().map_or(true, |n| !n.incluir.iter().any(|x| *x)) {
        return Err(format!("«{}» necesita una nómina con al menos una fila.", b.titulo));
    }
    let tmp = std::env::temp_dir().join(format!("flow_outlook_{}_{}", std::process::id(), chrono::Local::now().timestamp_millis()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut adjuntos = Vec::new();
    if let Some(n) = &b.nomina {
        let rows: Vec<Vec<String>> = n.rows.iter().zip(&n.incluir).filter(|(_, i)| **i).map(|(r, _)| r.clone()).collect();
        if !rows.is_empty() {
            let p = tmp.join(format!("{}.xlsx", safe_filename(if b.programa.is_empty() { &b.titulo } else { &b.programa })));
            xlsx_libro(&[("Nómina", n.headers.clone(), rows)], &p)?;
            adjuntos.push(p.display().to_string());
        }
    }
    let mut cc = b.cc.clone();
    if !cc.to_lowercase().contains(config::CC_OBLIGATORIO) {
        cc = if cc.trim().is_empty() { config::CC_OBLIGATORIO.into() } else { format!("{}; {cc}", config::CC_OBLIGATORIO) };
    }
    let datos = serde_json::json!({"to": b.para, "cc": cc, "subject": b.asunto, "body": b.cuerpo, "attachments": adjuntos});
    let json = tmp.join("borrador.json");
    std::fs::write(&json, serde_json::to_vec(&datos).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let script = "$ErrorActionPreference='Stop'\n\
        $d = Get-Content -Raw -Encoding UTF8 -LiteralPath $env:FLOW_BORRADOR | ConvertFrom-Json\n\
        $o = New-Object -ComObject Outlook.Application\n\
        $m = $o.CreateItem(0)\n\
        $m.To = [string]$d.to\n$m.CC = [string]$d.cc\n$m.Subject = [string]$d.subject\n$m.Body = [string]$d.body\n\
        foreach ($a in $d.attachments) { [void]$m.Attachments.Add([string]$a) }\n\
        $m.Save()\nWrite-Output 'GUARDADO'";
    let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let enc = base64::engine::general_purpose::STANDARD.encode(utf16);
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &enc])
        .env("FLOW_BORRADOR", &json)
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| format!("No se pudo iniciar PowerShell: {e}"))?;
    let _ = std::fs::remove_file(&json);
    let salida = String::from_utf8_lossy(&out.stdout);
    if out.status.success() && salida.contains("GUARDADO") {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(format!("Outlook no confirmó el guardado de «{}»; revisa Borradores antes de repetir. {}", b.titulo, err.lines().next().unwrap_or("")))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn guardar_outlook(_b: &Borrador) -> Result<(), String> {
    Err("Guardar en Outlook solo está disponible en Windows; usa archivos .eml.".into())
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

#[derive(Clone, Debug)]
pub struct Proyecto {
    pub tribunal: String,
    pub tribunal_clave: String,
    pub rit: String,
    pub tipo: String,
    pub vars: BTreeMap<String, String>,
    pub registros: Vec<usize>,
    pub incluir: bool,
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
    let fecha = w.corte.map(fecha_es).unwrap_or_default();
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
            vars.insert("FECHA".into(), fecha.clone());
            vars.insert(
                "FECHA_RESOLUCION".into(),
                regs.iter().filter_map(|r| w.date(r, "resolucion")).next().map(fecha_es).unwrap_or_default(),
            );
            vars.insert("DURACION".into(), junta("duracion"));
            vars.insert("NOMENCLATURA".into(), String::new());
            Proyecto {
                tribunal: cfg.tribunal_nombre(&trib, &w.value(regs[0], "tribunal")),
                tribunal_clave: trib,
                rit: w.value(regs[0], "rit"),
                tipo: tipo.into(),
                vars,
                registros: ids,
                incluir: true,
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

/// Plantilla .docx propia del usuario (%APPDATA%\FlowRust\plantillas\<TRIBUNAL>_<TIPO>.docx), si existe.
fn plantilla_usuario(trib: &str, tipo: &str) -> Option<String> {
    let p = config::data_dir().join("plantillas").join(format!("{trib}_{tipo}.docx"));
    let f = std::fs::File::open(p).ok()?;
    let mut z = zip::ZipArchive::new(f).ok()?;
    let mut s = String::new();
    std::io::Read::read_to_string(&mut z.by_name("word/document.xml").ok()?, &mut s).ok()?;
    let ini = s.find("<w:body>")? + 8;
    let fin = s.rfind("<w:sectPr").or_else(|| s.rfind("</w:body>"))?;
    Some(s[ini..fin].to_string())
}

/// Reemplaza {{VARIABLES}} en el XML de una plantilla propia, aunque Word las haya partido en varios trozos.
fn llenar_xml(body: &str, vars: &BTreeMap<String, String>) -> String {
    let rp = regex::Regex::new(r"(?s)<w:p[ >].*?</w:p>").unwrap();
    let rt = regex::Regex::new(r"(?s)<w:t[^>]*>(.*?)</w:t>").unwrap();
    rp.replace_all(body, |c: &regex::Captures| {
        let p = &c[0];
        let texto: String = rt.captures_iter(p).map(|t| t[1].to_string()).collect();
        if !texto.contains("{{") {
            return p.to_string();
        }
        let ppr = regex::Regex::new(r"(?s)<w:pPr>.*?</w:pPr>").unwrap().find(p).map(|m| m.as_str().to_string()).unwrap_or_default();
        let rpr = regex::Regex::new(r"(?s)<w:rPr>.*?</w:rPr>").unwrap().find(&p.replace(&ppr, "")).map(|m| m.as_str().to_string()).unwrap_or_default();
        let unesc = texto.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&");
        format!("<w:p>{ppr}<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>", xml_esc(&llenar(&unesc, vars)))
    })
    .to_string()
}

/// Párrafos del proyecto ya completados (vista previa; con plantilla propia se muestra el texto base).
pub fn texto_proyecto(p: &Proyecto) -> Vec<String> {
    plantilla(&p.tribunal_clave, &p.tipo).iter().map(|t| llenar(t, &p.vars)).collect()
}

/// Un solo documento con todos los proyectos incluidos, separados por salto de página.
pub fn generar_word(proyectos: &[Proyecto], dir: &Path, fecha: NaiveDate) -> Result<PathBuf, String> {
    let sel: Vec<&Proyecto> = proyectos.iter().filter(|p| p.incluir).collect();
    if sel.is_empty() {
        return Err("No hay proyectos marcados para generar.".into());
    }
    let mut body = String::new();
    for (i, p) in sel.iter().enumerate() {
        if i > 0 {
            body.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
        }
        if let Some(xml) = plantilla_usuario(&p.tribunal_clave, &p.tipo) {
            body.push_str(&llenar_xml(&xml, &p.vars));
        } else {
            for (j, t) in plantilla(&p.tribunal_clave, &p.tipo).iter().enumerate() {
                let txt = llenar(t, &p.vars);
                body.push_str(&parrafo(&txt, txt == "VISTO:" || txt.starts_with("RIT"), j == 0 && false));
            }
        }
    }
    let doc = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>{body}<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"18720\"/><w:pgMar w:top=\"1701\" w:right=\"1701\" w:bottom=\"1417\" w:left=\"1701\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>"
    );
    let path = nueva_ruta(dir, &format!("Resoluciones_{}", fecha.format("%Y-%m-%d")), "docx");
    let f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipWriter::new(f);
    let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let ct = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
    for (name, data) in [("[Content_Types].xml", ct), ("_rels/.rels", rels), ("word/document.xml", doc.as_str())] {
        z.start_file(name, o).map_err(|e| e.to_string())?;
        z.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    }
    z.finish().map_err(|e| e.to_string())?;
    Ok(path)
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
