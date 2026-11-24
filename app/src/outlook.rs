//! Borradores en Outlook (`nurus.adapters.outlook.save_draft` de Flow Python): solo `Save()`, nunca se envía; el CC fijo
//! siempre va; se puede elegir la cuenta por su dirección o nombre exactos. Si Outlook falla antes de guardar, no se creó
//! nada; si falla después de intentar guardar, el borrador pudo existir y no se reintenta hasta que la persona lo revise.

use crate::config::CC_OBLIGATORIO;
use crate::productos::Borrador;
#[cfg(target_os = "windows")]
use crate::productos::{nueva_ruta_temporal, xlsx_libro};
#[cfg(target_os = "windows")]
use crate::util::safe_filename;
use serde_json::json;

/// Identidad del borrador que Outlook confirmó.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReciboOutlook {
    pub entry_id: String,
    pub store_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorOutlook {
    /// Falló antes de guardar: no existe ningún borrador nuevo y se puede reintentar.
    NoGuardado(String),
    /// Pudo haberse guardado sin confirmación suficiente: revisar Borradores antes de repetir (`DraftSaveUncertain`).
    Incierto(String),
}

impl ErrorOutlook {
    pub fn mensaje(&self) -> &str {
        match self {
            ErrorOutlook::NoGuardado(m) | ErrorOutlook::Incierto(m) => m,
        }
    }
}

/// Script de Outlook clásico. Cada paso informa su avance por la salida estándar: `PREPARADO` justo antes de `Save()` y
/// `GUARDADO|<EntryID>|<StoreID>` solo si Outlook devolvió la identidad completa.
pub const SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
$d = Get-Content -Raw -Encoding UTF8 -LiteralPath $env:FLOW_BORRADOR | ConvertFrom-Json
$o = New-Object -ComObject Outlook.Application
$s = $o.Session
$cuenta = ([string]$d.cuenta).Trim().ToLower()
$acc = $null
if ($cuenta) {
  $coinc = @()
  foreach ($a in $s.Accounts) {
    foreach ($v in @($a.SmtpAddress, $a.DisplayName)) {
      if ($v -and ([string]$v).Trim().ToLower() -eq $cuenta) { $coinc += $a; break }
    }
  }
  if ($coinc.Count -ne 1) { Write-Output 'NOCUENTA'; exit 3 }
  $acc = $coinc[0]
  $carpeta = $acc.DeliveryStore.GetDefaultFolder(16)
} else {
  $carpeta = $s.GetDefaultFolder(16)
}
$m = $carpeta.Items.Add('IPM.Note')
if ($acc) { $m.SendUsingAccount = $acc }
$m.To = [string]$d.to
$m.CC = [string]$d.cc
$m.Subject = [string]$d.subject
$m.Body = [string]$d.body
foreach ($a in $d.attachments) { [void]$m.Attachments.Add([string]$a) }
Write-Output 'PREPARADO'
$m.Save()
$id = [string]$m.EntryID
$st = [string]$carpeta.Store.StoreID
if (-not $id.Trim() -or -not $st.Trim()) { Write-Output 'SIN_IDENTIDAD'; exit 4 }
Write-Output "GUARDADO|$id|$st"
"#;

/// Lo que se le entrega al script: destinatarios, CC con el fijo, texto y la nómina ya escrita como Excel.
pub fn datos(b: &Borrador, cuenta: &str, adjuntos: &[String]) -> serde_json::Value {
    let mut cc = b.cc.clone();
    if !cc.to_lowercase().contains(CC_OBLIGATORIO) {
        cc = if cc.trim().is_empty() { CC_OBLIGATORIO.into() } else { format!("{CC_OBLIGATORIO}; {cc}") };
    }
    json!({"cuenta": cuenta.trim(), "to": b.para, "cc": cc, "subject": b.asunto, "body": b.cuerpo, "attachments": adjuntos})
}

/// Traduce lo que dijo PowerShell en un recibo o en el tipo de fallo que corresponde.
pub fn interpretar(exito: bool, salida: &str, error: &str, titulo: &str) -> Result<ReciboOutlook, ErrorOutlook> {
    let detalle = error.lines().next().unwrap_or("").trim();
    let lineas: Vec<&str> = salida.lines().map(str::trim).collect();
    if lineas.contains(&"NOCUENTA") {
        return Err(ErrorOutlook::NoGuardado("No existe una cuenta Outlook única que coincida exactamente con la cuenta elegida en Ajustes.".into()));
    }
    let guardado = lineas.iter().find_map(|l| l.strip_prefix("GUARDADO|"));
    if let (true, Some(ids)) = (exito, guardado) {
        if let Some((entry, store)) = ids.split_once('|') {
            if !entry.trim().is_empty() && !store.trim().is_empty() {
                return Ok(ReciboOutlook { entry_id: entry.trim().into(), store_id: store.trim().into() });
            }
        }
    }
    if lineas.contains(&"PREPARADO") || lineas.contains(&"SIN_IDENTIDAD") {
        return Err(ErrorOutlook::Incierto(format!(
            "Outlook pudo haber guardado «{titulo}» sin confirmarlo; revisa Borradores de Outlook antes de repetir. {detalle}"
        )));
    }
    Err(ErrorOutlook::NoGuardado(format!("No se pudo preparar el borrador «{titulo}» en Outlook; no se guardó nada. {detalle}")))
}

/// Guarda el borrador en la carpeta Borradores de Outlook (Windows, Outlook clásico). Solo llama a `Save()`.
#[cfg(target_os = "windows")]
pub fn guardar_outlook(b: &Borrador, cuenta: &str) -> Result<ReciboOutlook, ErrorOutlook> {
    use base64::Engine;
    use std::os::windows::process::CommandExt;
    let no = |m: String| ErrorOutlook::NoGuardado(m);
    if b.adjunto_obligatorio && b.nomina.as_ref().is_none_or(|n| !n.incluir.iter().any(|x| *x)) {
        return Err(no(format!("«{}» necesita una nómina con al menos una fila.", b.titulo)));
    }
    let tmp = nueva_ruta_temporal("flow_outlook");
    std::fs::create_dir_all(&tmp).map_err(|e| no(e.to_string()))?;
    let resultado = (|| {
        let mut adjuntos = Vec::new();
        if let Some(n) = &b.nomina {
            let filas: Vec<Vec<String>> = n.rows.iter().zip(&n.incluir).filter(|(_, i)| **i).map(|(r, _)| r.clone()).collect();
            if !filas.is_empty() {
                let p = tmp.join(format!("{}.xlsx", safe_filename(if b.programa.is_empty() { &b.titulo } else { &b.programa })));
                xlsx_libro(&[("Nómina", n.headers.clone(), filas)], &p).map_err(no)?;
                adjuntos.push(p.display().to_string());
            }
        }
        // Outlook recibe copias de los mismos bytes que se revisaron.
        crate::manual::verificar_adjuntos(b).map_err(no)?;
        for (i, (ruta, _)) in b.adjuntos.iter().enumerate() {
            let origen = std::path::Path::new(ruta);
            let copia = tmp.join(format!("adjunto{i}")).join(origen.file_name().unwrap_or_default());
            std::fs::create_dir_all(copia.parent().unwrap()).map_err(|e| no(e.to_string()))?;
            std::fs::copy(origen, &copia).map_err(|e| no(format!("No se pudo copiar el adjunto {ruta}: {e}")))?;
            adjuntos.push(copia.display().to_string());
        }
        let json = tmp.join("borrador.json");
        std::fs::write(&json, serde_json::to_vec(&datos(b, cuenta, &adjuntos)).unwrap_or_default()).map_err(|e| no(e.to_string()))?;
        let utf16: Vec<u8> = SCRIPT.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let enc = base64::engine::general_purpose::STANDARD.encode(utf16);
        let salida = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &enc])
            .env("FLOW_BORRADOR", &json)
            .creation_flags(0x0800_0000)
            .output()
            .map_err(|e| no(format!("No se pudo iniciar PowerShell: {e}")))?;
        interpretar(salida.status.success(), &String::from_utf8_lossy(&salida.stdout), &String::from_utf8_lossy(&salida.stderr), &b.titulo)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    resultado
}

#[cfg(not(target_os = "windows"))]
pub fn guardar_outlook(_b: &Borrador, _cuenta: &str) -> Result<ReciboOutlook, ErrorOutlook> {
    Err(ErrorOutlook::NoGuardado("Guardar en Outlook solo está disponible en Windows; usa archivos .eml.".into()))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn borrador() -> Borrador {
        Borrador { titulo: "Prueba".into(), para: "a@ejemplo.invalid".into(), cc: "otro@ejemplo.invalid".into(), asunto: "Asunto".into(), cuerpo: "Cuerpo".into(), ..Default::default() }
    }

    #[test]
    fn el_script_solo_guarda_y_nunca_envia() {
        assert!(SCRIPT.contains(".Save()"));
        assert!(!SCRIPT.to_lowercase().contains(".send("), "el script no puede enviar correos");
        assert!(SCRIPT.contains("PREPARADO") && SCRIPT.contains("GUARDADO|"));
        // `Save()` va después de `PREPARADO`: un fallo anterior no deja un borrador, uno posterior sí puede.
        assert!(SCRIPT.find("PREPARADO").unwrap() < SCRIPT.find(".Save()").unwrap());
    }

    #[test]
    fn el_cc_fijo_y_la_cuenta_viajan_al_script() {
        let d = datos(&borrador(), "  Cuenta@Ejemplo.invalid ", &[]);
        assert_eq!(d["cuenta"], "Cuenta@Ejemplo.invalid");
        assert!(d["cc"].as_str().unwrap().starts_with(CC_OBLIGATORIO), "{}", d["cc"]);
        assert!(d["cc"].as_str().unwrap().contains("otro@ejemplo.invalid"));
        let mut con_cc = borrador();
        con_cc.cc = format!("x@ejemplo.invalid; {CC_OBLIGATORIO}");
        assert_eq!(datos(&con_cc, "", &[])["cc"], con_cc.cc, "el CC fijo no se repite");
    }

    /// Fallos simulados antes y después de guardar (E-T1).
    #[test]
    fn un_fallo_antes_de_guardar_se_distingue_de_uno_despues() {
        // Todo bien: Outlook devuelve la identidad completa.
        let ok = interpretar(true, "PREPARADO\nGUARDADO|ENTRADA|ALMACEN\n", "", "T").unwrap();
        assert_eq!((ok.entry_id.as_str(), ok.store_id.as_str()), ("ENTRADA", "ALMACEN"));
        // Antes de guardar: ni cuenta ni inicio de Outlook; no hay borrador y se puede reintentar.
        assert!(matches!(interpretar(false, "NOCUENTA\n", "", "T"), Err(ErrorOutlook::NoGuardado(m)) if m.contains("cuenta Outlook única")));
        assert!(matches!(interpretar(false, "", "Outlook no está instalado", "T"), Err(ErrorOutlook::NoGuardado(m)) if m.contains("no se guardó nada")));
        // Después de intentar guardar: el borrador pudo existir; no se reintenta sin revisar.
        assert!(matches!(interpretar(false, "PREPARADO\n", "Fallo al guardar", "T"), Err(ErrorOutlook::Incierto(m)) if m.contains("antes de repetir")));
        assert!(matches!(interpretar(false, "PREPARADO\nSIN_IDENTIDAD\n", "", "T"), Err(ErrorOutlook::Incierto(_))));
        // Salida de éxito sin identidad completa: también incierto.
        assert!(matches!(interpretar(true, "PREPARADO\nGUARDADO||ALMACEN\n", "", "T"), Err(ErrorOutlook::Incierto(_))));
        assert!(matches!(interpretar(true, "PREPARADO\nGUARDADO|ENTRADA|\n", "", "T"), Err(ErrorOutlook::Incierto(_))));
    }
}
