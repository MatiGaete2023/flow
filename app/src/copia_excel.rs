//! Copia preservada del Excel de trabajo (`Work.export` y `exports._native_preserved` de Flow Python).
//! Excel de escritorio abre una copia del original (o del consolidado del paquete), agrega las columnas de revisión
//! en la hoja procesada y guarda con la misma extensión. El original nunca se modifica.

use crate::lectura;
use crate::trabajo::{hex, Registro, Trabajo};
use crate::util::{historical_match, normalizar, Cell};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const FIDELIDAD_REDUCIDA: &str = "Copia de fidelidad reducida: el XLSX consolidado del paquete no sirve como base (por ejemplo, trae una hoja por modalidad o un registro repetido). El consolidado se conserva intacto dentro del paquete.";
const HOJAS_SERVICIO: [&str; 3] = ["Resumen", "SITFA_ARCHIVOS", "Vencimientos"];
const PROCEDENCIA: [&str; 3] = ["SITFA_SHA256", "SITFA_ARCHIVO", "SITFA_FILA"];
pub(crate) const TIPOS_RES: [&str; 3] = ["PC_IE", "PC_INFO", "NOMENCL"];
pub const COLUMNAS_BITACORA: [&str; 2] = ["CLASIFICACION_BITACORA", "MOTIVO_BITACORA"];

pub struct FilaBitacora<'a> {
    pub indice: usize,
    pub clasificacion: &'a str,
    pub motivo: &'a str,
}

/// Libro de partida y ubicación de cada registro en su hoja.
pub struct Base {
    pub bytes: Vec<u8>,
    pub extension: String,
    pub hoja: String,
    /// Fila de encabezado, contada desde 1.
    pub encabezado: usize,
    pub filas: Vec<usize>,
}

fn texto_clave(c: &Cell) -> String {
    let t = c.display();
    let t = t.trim();
    t.strip_suffix(".0").unwrap_or(t).to_string()
}

/// Por qué el paquete no trae `consolidado/consolidado.xlsx`: el motivo que dejó el Descargador o, si no hay, que el
/// Descargador permite crear el paquete sin él.
fn motivo_sin_consolidado(zip: &[u8]) -> String {
    let incidencia = flow_rust::captura::leer_captura(zip).ok().and_then(|c| {
        c.filas("incidencias").find(|i| i.get("code") == "CONSOLIDADO_OMITIDO").map(|i| i.get("message").to_string())
    });
    incidencia.unwrap_or_else(|| "El paquete no incluye el XLSX consolidado (el Descargador permite crearlo sin él).".into())
}

fn consolidado(zip: &[u8]) -> Result<Vec<u8>, String> {
    let mut archivo = zip::ZipArchive::new(std::io::Cursor::new(zip))
        .map_err(|_| "No se pudo abrir el paquete para la copia preservada.".to_string())?;
    let entrada = archivo
        .by_name("consolidado/consolidado.xlsx")
        .map_err(|_| motivo_sin_consolidado(zip))?;
    let mut datos = Vec::new();
    entrada
        .take(200 * 1024 * 1024)
        .read_to_end(&mut datos)
        .map_err(|_| "No se pudo leer el libro consolidado del paquete.".to_string())?;
    Ok(datos)
}

/// Huella, nombre del archivo SITFA (sin la carpeta `originales/lN/` que agrega el paquete) y fila.
fn clave_origen(huella: &Cell, archivo: &Cell, fila: &Cell) -> Vec<String> {
    let nombre = texto_clave(archivo);
    let nombre = nombre.rsplit(['/', '\\']).next().unwrap_or("").to_string();
    vec![texto_clave(huella).to_ascii_lowercase(), nombre, texto_clave(fila)]
}

/// `_map_consolidated`: cada registro aparece una sola vez, por huella, archivo y fila de SITFA, en una misma hoja.
/// `Err` explica por qué el consolidado no sirve de base.
fn base_paquete(w: &Trabajo) -> Result<Base, String> {
    let bytes = consolidado(&w.bytes)?;
    let hojas = lectura::read_sheets(&bytes).map_err(|_| "No se pudo leer el libro consolidado del paquete.".to_string())?;
    let posicion = |titulo: &str| w.headers.iter().position(|h| h == titulo);
    let propias: Vec<usize> = PROCEDENCIA
        .iter()
        .map(|t| posicion(t))
        .collect::<Option<_>>()
        .ok_or("Trabajo no conserva la procedencia SITFA de las filas.")?;
    let mut indice: HashMap<Vec<String>, (String, usize)> = HashMap::new();
    let mut repetidas = HashSet::new();
    let vacia = Cell::Empty;
    for hoja in hojas.iter().filter(|h| !HOJAS_SERVICIO.contains(&h.name.as_str())) {
        let Some(cabecera) = hoja.rows.first() else { continue };
        let titulos: Vec<String> = cabecera.iter().map(|c| c.display().trim().to_string()).collect();
        let Some(cols) = PROCEDENCIA
            .iter()
            .map(|t| titulos.iter().position(|h| h == t))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let unicos: HashSet<_> = titulos.iter().filter(|t| !t.is_empty()).map(|t| normalizar(t)).collect();
        if unicos.len() != titulos.iter().filter(|t| !t.is_empty()).count() {
            continue;
        }
        for (i, fila) in hoja.rows.iter().enumerate().skip(1) {
            let celda = |k: usize| fila.get(cols[k]).unwrap_or(&vacia);
            let clave = clave_origen(celda(0), celda(1), celda(2));
            if clave.iter().all(String::is_empty) {
                continue;
            }
            if indice.insert(clave.clone(), (hoja.name.clone(), i + 1)).is_some() {
                repetidas.insert(clave);
            }
        }
    }
    if indice.is_empty() {
        return Err("El consolidado del paquete no tiene hojas con la procedencia SITFA.".into());
    }
    let mut hojas_usadas = HashSet::new();
    let mut filas = Vec::with_capacity(w.registros.len());
    let (mut ausentes, mut dobles) = (0usize, 0usize);
    for r in &w.registros {
        let celda = |k: usize| r.values.get(propias[k]).unwrap_or(&vacia);
        let clave = clave_origen(celda(0), celda(1), celda(2));
        if repetidas.contains(&clave) {
            dobles += 1;
            continue;
        }
        match indice.get(&clave) {
            Some((hoja, fila)) => {
                hojas_usadas.insert(hoja.clone());
                filas.push(*fila);
            }
            None => ausentes += 1,
        }
    }
    if ausentes > 0 || dobles > 0 {
        return Err(format!(
            "{FIDELIDAD_REDUCIDA} Motivo: {ausentes} fila(s) sin correspondencia y {dobles} repetida(s) en el consolidado."
        ));
    }
    if hojas_usadas.len() != 1 {
        return Err(format!(
            "{FIDELIDAD_REDUCIDA} Motivo: las filas de la etapa están en {} hojas del consolidado.",
            hojas_usadas.len()
        ));
    }
    Ok(Base {
        bytes,
        extension: "xlsx".into(),
        hoja: hojas_usadas.into_iter().next().unwrap_or_default(),
        encabezado: 1,
        filas,
    })
}

/// Base de la copia. `Err` con el aviso de fidelidad reducida cuando no hay un libro que preservar.
pub fn base(w: &Trabajo) -> Result<Base, String> {
    if w.paquete.is_some() {
        return base_paquete(w);
    }
    let extension = w
        .path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !matches!(extension.as_str(), "xls" | "xlsx" | "xlsm") {
        return Err("El origen no es un formato Excel admitido para la copia preservada; se guarda una tabla nueva de fidelidad reducida.".into());
    }
    Ok(Base {
        bytes: w.bytes.clone(),
        extension,
        hoja: w.sheets.get(w.sheet).map(|s| s.name.clone()).unwrap_or_default(),
        encabezado: w.header_row + 1,
        filas: w.registros.iter().map(|r| r.fila_excel).collect(),
    })
}

/// `Work.export`: hay etapa revisada si alguna fila tiene una edición humana.
pub(crate) fn revisada(r: &Registro) -> bool {
    r.revisada || r.editada.as_ref().is_some_and(|e| *e != r.propuesta)
}

/// Identificador estable de la fila (`Work.external`): hoja, identidad histórica y ordinal.
pub(crate) fn identificadores(w: &Trabajo, hoja: &str) -> Vec<String> {
    let captura = w.headers.iter().position(|h| h == "ID_REGISTRO_CAPTURA");
    let mut vistos: HashMap<String, usize> = HashMap::new();
    w.registros
        .iter()
        .map(|r| {
            if let Some(id) = captura.and_then(|c| r.values.get(c)).map(|c| c.display()).filter(|s| !s.trim().is_empty()) {
                return id;
            }
            let identidad = ["rit", "rut", "nombre", "tribunal", "programa"]
                .iter()
                .map(|k| {
                    w.cols
                        .get(k)
                        .and_then(|c| r.values.get(*c))
                        .map(|v| historical_match(&v.display()))
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join("|");
            let n = vistos.entry(identidad.clone()).or_default();
            let ordinal = *n;
            *n += 1;
            hex(&Sha256::digest(format!("{hoja}|{identidad}|{ordinal}").as_bytes()))
        })
        .collect()
}

/// `record_edits.encode` (versión 1): decisiones legibles por Flow Python al reimportar la copia.
pub(crate) fn decisiones(r: &Registro) -> String {
    let resolucion = if TIPOS_RES.contains(&r.res.as_str()) { r.res.as_str() } else { "auto" };
    let mut decisions = json!({"resolution": resolucion});
    if !r.gestiones_correo.is_empty() {
        decisions["mail"] = json!(r.gestiones_correo);
    }
    json!({
        "version": 1,
        "decisions": decisions,
        "overrides": r.anulaciones,
        "word_overrides": {},
        "excluded": r.excluido,
        "resolution_cell": r.res,
        "proposal": r.propuesta,
    })
    .to_string()
}

/// `_safe_cell`: un texto que Excel leería como fórmula se escribe literal.
pub(crate) fn literal(s: &str) -> String {
    if s.trim_start().starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.to_string()
    }
}

/// Instrucciones para Excel: columnas, celdas por columna, filas excluidas y trazabilidad (`_trace_rows`).
pub fn instrucciones(w: &Trabajo, base: &Base, origen: &Path, destino: &Path) -> Value {
    plan_excel(w, base, origen, destino, &[])
}

fn plan_excel(
    w: &Trabajo,
    base: &Base,
    origen: &Path,
    destino: &Path,
    bitacoras: &[(usize, &FilaBitacora<'_>)],
) -> Value {
    let etapa_revisada = w.registros.iter().any(revisada);
    // Orden del registro mensual del usuario (decisión del 20-nov-2026), igual en propuesta y revisión.
    let mut titulos: Vec<&str> = vec![
        "NURUS_ID_REGISTRO", "NURUS_REGLAS", "NURUS_PROPUESTA", "FECHA_OBS", "OBSERVACION", "TT", "CC", "RES",
        "NURUS_ESTADO_REVISION", "NURUS_DECISIONES",
    ];
    let ids = identificadores(w, &base.hoja);
    let mut celdas: HashMap<&str, Vec<Value>> = HashMap::new();
    let mut res_sugerida = Vec::new();
    let mut excluidas = Vec::new();
    let mut traza = vec![
        json!(["CAMPO", "VALOR"]),
        json!(["ESTADO_DOCUMENTO", if etapa_revisada { "CONSTANCIA_REVISADA" } else { "PROPUESTA_NO_REVISADA" }]),
        json!(["FUENTE_SHA256", hex(&Sha256::digest(&base.bytes))]),
        json!(["MODO", w.mode().label().to_uppercase()]),
        json!(["HOJA_PROCESADA", base.hoja]),
        json!([]),
        json!(["HOJA", "FILA", "ESTADO", "MOTIVO", "OBSERVACION", "REGLAS", "HASH_ORIGEN"]),
    ];
    let huella = hex(&Sha256::digest(&base.bytes));
    for ((r, fila), id) in w.registros.iter().zip(&base.filas).zip(&ids) {
        let aprobada = revisada(r) && !r.excluido;
        let estado = if r.excluido {
            "EXCLUIDO"
        } else if !etapa_revisada && !r.avisos.is_empty() {
            "REQUIERE_REVISION"
        } else if !etapa_revisada {
            "PROPUESTA"
        } else if aprobada {
            "REVISADO"
        } else {
            "PENDIENTE"
        };
        let reglas = serde_json::to_string(&r.eventos).unwrap_or_else(|_| "[]".into());
        let obs = r.observacion().to_string();
        let mut poner = |titulo: &'static str, valor: String| {
            celdas.entry(titulo).or_default().push(json!({"f": fila, "v": valor}));
        };
        poner("NURUS_ID_REGISTRO", id.clone());
        poner("NURUS_REGLAS", reglas.clone());
        poner("NURUS_DECISIONES", decisiones(r));
        poner("NURUS_PROPUESTA", literal(&r.propuesta));
        poner("NURUS_ESTADO_REVISION", estado.into());
        if !etapa_revisada {
            poner("OBSERVACION", literal(&obs));
        } else if aprobada {
            poner("OBSERVACION", literal(&obs));
            if !r.res.is_empty() {
                poner("RES", literal(&r.res));
            }
        }
        if !r.res.is_empty() && !(etapa_revisada && aprobada) && TIPOS_RES.contains(&r.res.as_str()) {
            res_sugerida.push(json!({"f": fila, "v": r.res}));
        }
        if r.excluido {
            excluidas.push(json!(fila));
        }
        let decision = if r.excluido { "excluded" } else if aprobada { "approved" } else { "pending" };
        traza.push(json!([base.hoja, fila, decision, r.avisos.join("; "), literal(&obs), reglas, huella]));
    }
    if !bitacoras.is_empty() {
        titulos.extend(COLUMNAS_BITACORA);
        for (fila, lectura) in bitacoras {
            for (titulo, texto) in COLUMNAS_BITACORA.into_iter().zip([lectura.clasificacion, lectura.motivo]) {
                celdas.entry(titulo).or_default().push(json!({"f": fila, "v": literal(texto)}));
            }
        }
    }
    let escrituras: Vec<Value> = titulos
        .iter()
        .map(|t| {
            let mut lista = celdas.remove(t).unwrap_or_default();
            lista.sort_by_key(|c| c["f"].as_u64());
            json!({
                "titulo": t,
                "conservar": !etapa_revisada && *t == "OBSERVACION",
                "celdas": lista,
            })
        })
        .collect();
    res_sugerida.sort_by_key(|c| c["f"].as_u64());
    json!({
        "origen": origen, "destino": destino, "hoja": base.hoja, "encabezado": base.encabezado,
        "titulos": titulos, "escrituras": escrituras, "res_sugerida": res_sugerida,
        "ocultas": ["NURUS_REGLAS", "NURUS_DECISIONES"], "excluidas": excluidas, "traza": traza, "tipos_res": TIPOS_RES,
        "bitacoras": if bitacoras.is_empty() {
            Value::Null
        } else {
            json!({
                "columnas": COLUMNAS_BITACORA,
                "filas": bitacoras.iter().map(|(fila, _)| fila).collect::<Vec<_>>(),
            })
        },
    })
}

fn filas_bitacora<'a>(
    base: &Base,
    filas: &'a [FilaBitacora<'a>],
) -> Result<Vec<(usize, &'a FilaBitacora<'a>)>, String> {
    if filas.is_empty() {
        return Err("Selecciona al menos una fila de bitácoras.".into());
    }
    let mut elegidas = filas.iter().map(|lectura| {
        base.filas.get(lectura.indice).copied().map(|fila| (fila, lectura))
            .ok_or_else(|| "La fila de bitácora no está asociada a la hoja de origen.".to_string())
    }).collect::<Result<Vec<_>, _>>()?;
    elegidas.sort_unstable_by_key(|(fila, _)| *fila);
    if elegidas.windows(2).any(|par| par[0].0 == par[1].0) {
        return Err("Dos resultados de bitácora apuntan a la misma fila de Excel.".into());
    }
    Ok(elegidas)
}

/// Script de Excel de escritorio. Solo abre la copia temporal y guarda otra copia; nunca toca el original.
pub const SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
$d = Get-Content -Raw -Encoding UTF8 -LiteralPath $env:FLOW_COPIA | ConvertFrom-Json
function Norm($v) { if ($null -eq $v) { return '' }; return ((([string]$v).Trim().ToUpper().Replace('_', ' ')) -split '\s+' | Where-Object { $_ }) -join ' ' }
function Vacia($v) { return ($null -eq $v) -or (([string]$v).Trim() -eq '') }
function Escribir($ws, $col, $celdas, $conservar) {
  $items = @($celdas)
  $i = 0
  while ($i -lt $items.Count) {
    $j = $i
    while (($j + 1) -lt $items.Count -and [int]$items[$j + 1].f -eq ([int]$items[$j].f + 1) -and ($j + 1 - $i) -lt 500) { $j++ }
    if ($conservar) {
      for ($k = $i; $k -le $j; $k++) {
        $c = $ws.Cells.Item([int]$items[$k].f, $col)
        if (Vacia $c.Value2) { $c.NumberFormat = '@'; $c.Value2 = [string]$items[$k].v }
      }
    } else {
      $n = $j - $i + 1
      $a = New-Object 'object[,]' $n, 1
      for ($k = 0; $k -lt $n; $k++) { $a[$k, 0] = [string]$items[$i + $k].v }
      $r = $ws.Range($ws.Cells.Item([int]$items[$i].f, $col), $ws.Cells.Item([int]$items[$j].f, $col))
      $r.NumberFormat = '@'
      $r.Value2 = $a
    }
    $i = $j + 1
  }
}
$xl = $null; $wb = $null; $etapa = 'iniciar Excel'
try {
  $xl = New-Object -ComObject Excel.Application
  $xl.Visible = $false; $xl.DisplayAlerts = $false; $xl.EnableEvents = $false; $xl.AskToUpdateLinks = $false
  $xl.AutomationSecurity = 3
  $etapa = 'abrir el libro de trabajo'
  $wb = $xl.Workbooks.Open([string]$d.origen, 0, $false)
  $etapa = 'preparar la hoja procesada'
  $ws = $wb.Worksheets.Item([string]$d.hoja)
  $h = [int]$d.encabezado
  $ur = $ws.UsedRange
  $ultimaCol = $ur.Column + $ur.Columns.Count - 1
  $ultimaFila = $ur.Row + $ur.Rows.Count - 1
  $pos = @{}
  for ($c = 1; $c -le $ultimaCol; $c++) {
    $k = Norm $ws.Cells.Item($h, $c).Value2
    if ($k -ne '') { if (-not $pos.ContainsKey($k)) { $pos[$k] = @() }; $pos[$k] += $c }
  }
  $cols = @{}; $ultima = $ultimaCol
  foreach ($t in $d.titulos) {
    $k = Norm $t
    if ($pos.ContainsKey($k) -and $pos[$k].Count -gt 1) { throw "Hay más de una columna $k; no se puede anotar sin ambigüedad." }
    if ($pos.ContainsKey($k)) { $c = $pos[$k][0] } else { $c = $ultima + 1 }
    $cols[$t] = $c; if ($c -gt $ultima) { $ultima = $c }
    if (Vacia $ws.Cells.Item($h, $c).Value2) { $ws.Cells.Item($h, $c).Value2 = [string]$t }
  }
  $etapa = 'escribir observaciones y estados'
  if ($d.bitacoras) {
    foreach ($t in $d.bitacoras.columnas) {
      $c = $cols[[string]$t]
      [void]$ws.Range($ws.Cells.Item($h + 1, $c), $ws.Cells.Item($ultimaFila, $c)).ClearContents()
    }
  }
  foreach ($e in $d.escrituras) {
    foreach ($p in @($e.celdas)) { if ([int]$p.f -le $h -or [int]$p.f -gt $ultimaFila) { throw "La fila $($p.f) no existe en la hoja procesada." } }
    if (@($e.celdas).Count -gt 0) { Escribir $ws $cols[[string]$e.titulo] @($e.celdas) ([bool]$e.conservar) }
  }
  if (@($d.res_sugerida).Count -gt 0) { Escribir $ws $cols['RES'] @($d.res_sugerida) $true }
  foreach ($t in $d.ocultas) { $ws.Columns.Item($cols[[string]$t]).Hidden = $true }
  $etapa = 'configurar selector RES'
  if ($ultimaFila -gt $h) {
    try { $lista = $wb.Worksheets.Item('NURUS_LISTAS') } catch { $lista = $wb.Worksheets.Add(); $lista.Name = 'NURUS_LISTAS' }
    $lista.Visible = 0
    $lista.Cells.Item(1, 1).Value2 = 'TIPOS_RES'
    for ($i = 0; $i -lt $d.tipos_res.Count; $i++) { $lista.Cells.Item($i + 2, 1).Value2 = [string]$d.tipos_res[$i] }
    try { $wb.Names.Item('NURUS_RES_TIPOS').Delete() } catch {}
    [void]$wb.Names.Add('NURUS_RES_TIPOS', "='NURUS_LISTAS'!`$A`$2:`$A`$4")
    $rc = $cols['RES']
    $rango = $ws.Range($ws.Cells.Item($h + 1, $rc), $ws.Cells.Item($ultimaFila, $rc))
    try { $rango.Validation.Delete() } catch {}
    [void]$rango.Validation.Add(3, 1, 1, '=NURUS_RES_TIPOS')
    $rango.Validation.IgnoreBlank = $true; $rango.Validation.InCellDropdown = $true; $rango.Validation.ShowError = $true
    $rango.Validation.ErrorTitle = 'Tipo RES no válido'
    $rango.Validation.ErrorMessage = 'Usa PC_IE, PC_INFO o NOMENCL, o deja la celda vacía.'
    $rango.Validation.ShowInput = $true
    $rango.Validation.InputTitle = 'Tipo de resolución'
    $rango.Validation.InputMessage = 'Selecciona PC_IE, PC_INFO o NOMENCL. Vacío: sin proyecto.'
    $ancho = [double]$ws.Columns.Item($rc).ColumnWidth; if ($ancho -lt 14) { $ws.Columns.Item($rc).ColumnWidth = 14 }
  }
  $etapa = 'marcar filas excluidas'
  $color = 255 + 242 * 256 + 204 * 65536
  foreach ($f in @($d.excluidas)) { $ws.Range($ws.Cells.Item([int]$f, 1), $ws.Cells.Item([int]$f, $ultima)).Interior.Color = $color }
  if ($d.bitacoras) {
    $etapa = 'aplicar la selección de bitácoras'
    if ($ws.FilterMode) { $ws.ShowAllData() }
    # Oculta intervalos del cuerpo sin mover filas ni cambiar referencias de fórmulas.
    $primera = $h + 1
    foreach ($f in $d.bitacoras.filas) {
      $f = [int]$f
      if ($f -gt $primera) { $ws.Range($ws.Cells.Item($primera, 1), $ws.Cells.Item($f - 1, 1)).EntireRow.Hidden = $true }
      $ws.Cells.Item($f, 1).EntireRow.Hidden = $false
      $primera = $f + 1
    }
    if ($primera -le $ultimaFila) {
      $ws.Range($ws.Cells.Item($primera, 1), $ws.Cells.Item($ultimaFila, 1)).EntireRow.Hidden = $true
    }
  }
  $etapa = 'crear la trazabilidad'
  $nombres = @{}; for ($i = 1; $i -le $wb.Worksheets.Count; $i++) { $nombres[$wb.Worksheets.Item($i).Name] = $true }
  $nombre = 'NURUS_TRAZABILIDAD'; $n = 2; while ($nombres.ContainsKey($nombre)) { $nombre = "NURUS_TRAZABILIDAD_$n"; $n++ }
  $traza = $wb.Worksheets.Add(); $traza.Name = $nombre; $traza.Visible = 0
  $filas = @($d.traza); $ancho = 1; foreach ($f in $filas) { if (@($f).Count -gt $ancho) { $ancho = @($f).Count } }
  $a = New-Object 'object[,]' $filas.Count, $ancho
  for ($i = 0; $i -lt $filas.Count; $i++) { $f = @($filas[$i]); for ($j = 0; $j -lt $ancho; $j++) { if ($j -lt $f.Count) { $a[$i, $j] = [string]$f[$j] } else { $a[$i, $j] = '' } } }
  $rt = $traza.Range($traza.Cells.Item(1, 1), $traza.Cells.Item($filas.Count, $ancho)); $rt.NumberFormat = '@'; $rt.Value2 = $a
  $ws.Activate()
  $etapa = 'guardar la copia'
  $wb.SaveCopyAs([string]$d.destino)
  Write-Output 'GUARDADO'
} catch {
  [Console]::Error.WriteLine("Excel no pudo completar la copia durante «$etapa». El original no fue modificado. Detalle: $($_.Exception.Message)")
  exit 1
} finally {
  if ($wb) { try { $wb.Close($false) } catch {} ; [void][Runtime.InteropServices.Marshal]::ReleaseComObject($wb) }
  if ($xl) { try { $xl.Quit() } catch {} ; [void][Runtime.InteropServices.Marshal]::ReleaseComObject($xl) }
}
"#;

/// Nombre de Flow Python: «<origen> - revisable AAAAMMDD_HHMMSS_ffffff.<ext>».
pub fn nombre_salida(w: &Trabajo, base: &Base) -> String {
    let raiz = w.path.file_stem().and_then(|s| s.to_str()).unwrap_or("Trabajo");
    format!(
        "{raiz} - revisable {}.{}",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%6f"),
        base.extension
    )
}

#[cfg(target_os = "windows")]
pub fn exportar(w: &Trabajo, dir: &Path) -> Result<PathBuf, String> {
    let base = base(w)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("No se pudo crear la carpeta de productos: {e}"))?;
    let destino = dir.join(nombre_salida(w, &base));
    guardar_copia(&base, &destino, |o, d| instrucciones(w, &base, o, d))
}

pub fn exportar_bitacoras(
    w: &Trabajo,
    base: &Base,
    destino: &Path,
    filas: &[FilaBitacora<'_>],
) -> Result<PathBuf, String> {
    let elegidas = filas_bitacora(base, filas)?;
    guardar_copia(base, destino, |o, d| plan_excel(w, base, o, d, &elegidas))
}

#[cfg(target_os = "windows")]
fn guardar_copia(
    base: &Base,
    destino: &Path,
    instrucciones: impl FnOnce(&Path, &Path) -> Value,
) -> Result<PathBuf, String> {
    use base64::Engine;
    use std::os::windows::process::CommandExt;
    let dir = destino.parent().ok_or("Falta la carpeta de la copia.")?;
    if destino.exists() {
        return Err("El archivo de destino ya existe; vuelve a guardar.".into());
    }
    let marca = format!("{}_{}", std::process::id(), chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default());
    let origen = dir.join(format!("~flow-origen-{marca}.{}", base.extension));
    let temporal = dir.join(format!("~flow-salida-{marca}.{}", base.extension));
    let datos = std::env::temp_dir().join(format!("flow_copia_{marca}.json"));
    let limpiar = || {
        let _ = std::fs::remove_file(&origen);
        let _ = std::fs::remove_file(&temporal);
        let _ = std::fs::remove_file(&datos);
    };
    std::fs::write(&origen, &base.bytes).map_err(|e| format!("No se pudo preparar la copia: {e}"))?;
    let plan = instrucciones(&origen, &temporal);
    if let Err(e) = std::fs::write(&datos, serde_json::to_vec(&plan).unwrap_or_default()) {
        limpiar();
        return Err(format!("No se pudo preparar la copia: {e}"));
    }
    let utf16: Vec<u8> = SCRIPT.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let enc = base64::engine::general_purpose::STANDARD.encode(utf16);
    let salida = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &enc])
        .env("FLOW_COPIA", &datos)
        .creation_flags(0x0800_0000)
        .output();
    let resultado = (|| {
        let out = salida.map_err(|e| format!("No se pudo iniciar PowerShell: {e}"))?;
        let texto = String::from_utf8_lossy(&out.stdout);
        let creada = std::fs::metadata(&temporal).map(|m| m.len() > 0).unwrap_or(false);
        if !(out.status.success() && texto.contains("GUARDADO") && creada) {
            let err = String::from_utf8_lossy(&out.stderr);
            let detalle = err.lines().find(|l| l.starts_with("Excel no pudo completar")).unwrap_or("");
            return Err(format!(
                "La copia preservada requiere Excel de escritorio. {}",
                if detalle.is_empty() { "Excel no produjo un archivo verificable." } else { detalle }
            ));
        }
        std::fs::rename(&temporal, destino).map_err(|e| format!("No se pudo dejar la copia en la carpeta: {e}"))?;
        Ok(destino.to_owned())
    })();
    limpiar();
    resultado
}

#[cfg(not(target_os = "windows"))]
fn guardar_copia(_: &Base, _: &Path, _: impl FnOnce(&Path, &Path) -> Value) -> Result<PathBuf, String> {
    Err("La copia preservada requiere Windows y Excel de escritorio.".into())
}

#[cfg(not(target_os = "windows"))]
pub fn exportar(w: &Trabajo, _dir: &Path) -> Result<PathBuf, String> {
    base(w)?;
    Err("La copia preservada requiere Windows y Excel de escritorio.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motor::Mode;

    fn trabajo() -> Trabajo {
        let mut w = Trabajo {
            path: PathBuf::from("Espera ficticia.xls"),
            headers: vec!["RIT".into(), "NOMBRE".into(), "OBSERVACION".into()],
            mode: Some(Mode::Espera),
            header_row: 2,
            ..Default::default()
        };
        w.sheets.push(lectura::Sheet { name: "Hoja1".into(), rows: vec![] });
        w.cols.insert("rit", 0);
        w.cols.insert("nombre", 1);
        w.registros = vec![
            Registro {
                fila_excel: 4,
                values: vec![Cell::Text("C-1-2026".into()), Cell::Text("Persona Ficticia".into())],
                propuesta: "=propuesta ficticia".into(),
                eventos: vec!["ESPERA.X".into()],
                res: "PC_IE".into(),
                ..Default::default()
            },
            Registro { fila_excel: 5, excluido: true, ..Default::default() },
        ];
        w
    }

    #[test]
    fn la_copia_parte_del_original_y_agrega_las_columnas_de_flow_python() {
        let w = trabajo();
        let b = base(&w).unwrap();
        assert_eq!((b.extension.as_str(), b.hoja.as_str(), b.encabezado, b.filas.clone()), ("xls", "Hoja1", 3, vec![4, 5]));
        let plan = instrucciones(&w, &b, Path::new("o.xls"), Path::new("d.xls"));
        assert_eq!(plan["titulos"][2], "NURUS_PROPUESTA");
        let orden: Vec<&str> = plan["titulos"].as_array().unwrap().iter().map(|t| t.as_str().unwrap()).collect();
        assert_eq!(
            orden,
            ["NURUS_ID_REGISTRO", "NURUS_REGLAS", "NURUS_PROPUESTA", "FECHA_OBS", "OBSERVACION", "TT", "CC", "RES", "NURUS_ESTADO_REVISION", "NURUS_DECISIONES"]
        );
        assert_eq!(plan["ocultas"], json!(["NURUS_REGLAS", "NURUS_DECISIONES"]));
        let dec = plan["escrituras"].as_array().unwrap().iter().find(|e| e["titulo"] == "NURUS_DECISIONES").unwrap();
        let primera: Value = serde_json::from_str(dec["celdas"][0]["v"].as_str().unwrap()).unwrap();
        assert_eq!((primera["version"].clone(), primera["decisions"]["resolution"].clone()), (json!(1), json!("PC_IE")));
        let obs = plan["escrituras"].as_array().unwrap().iter().find(|e| e["titulo"] == "OBSERVACION").unwrap();
        assert_eq!(obs["conservar"], true);
        assert_eq!(obs["celdas"][0], json!({"f": 4, "v": "'=propuesta ficticia"}));
        let estados = plan["escrituras"].as_array().unwrap().iter().find(|e| e["titulo"] == "NURUS_ESTADO_REVISION").unwrap();
        assert_eq!(estados["celdas"], json!([{"f": 4, "v": "PROPUESTA"}, {"f": 5, "v": "EXCLUIDO"}]));
        assert_eq!(plan["res_sugerida"], json!([{"f": 4, "v": "PC_IE"}]));
        assert_eq!(plan["excluidas"], json!([5]));
        assert!(nombre_salida(&w, &b).starts_with("Espera ficticia - revisable "));
        let mut revisado = w;
        revisado.registros[0].revisada = true;
        revisado.registros[0].editada = Some("Edición ficticia".into());
        let plan = instrucciones(&revisado, &b, Path::new("o.xls"), Path::new("d.xls"));
        assert_eq!(plan["titulos"][2], "NURUS_PROPUESTA");
        let escritura = |titulo: &str| {
            plan["escrituras"].as_array().unwrap().iter()
                .find(|e| e["titulo"] == titulo).unwrap()
        };
        assert_eq!(escritura("NURUS_PROPUESTA")["celdas"][0], json!({"f": 4, "v": "'=propuesta ficticia"}));
        assert_eq!(escritura("OBSERVACION")["celdas"], json!([{"f": 4, "v": "Edición ficticia"}]));
        let estados = plan["escrituras"].as_array().unwrap().iter().find(|e| e["titulo"] == "NURUS_ESTADO_REVISION").unwrap();
        assert_eq!(estados["celdas"], json!([{"f": 4, "v": "REVISADO"}, {"f": 5, "v": "EXCLUIDO"}]));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn el_script_de_excel_es_powershell_valido() {
        let ruta = std::env::temp_dir().join(format!("flow_copia_script_{}.ps1", std::process::id()));
        std::fs::write(&ruta, SCRIPT).unwrap();
        let comprobar = "$e = $null; [void][System.Management.Automation.Language.Parser]::ParseFile($env:FLOW_SCRIPT, [ref]$null, [ref]$e); if ($e.Count -gt 0) { $e | ForEach-Object { $_.Message }; exit 1 }; 'SINTAXIS_OK'";
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", comprobar])
            .env("FLOW_SCRIPT", &ruta)
            .output()
            .unwrap();
        let _ = std::fs::remove_file(&ruta);
        let texto = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success() && texto.contains("SINTAXIS_OK"), "{texto}");
    }

    fn paquete_con_consolidado(filas: &[(&str, f64)]) -> Vec<u8> {
        let mut libro = rust_xlsxwriter::Workbook::new();
        let resumen = libro.add_worksheet();
        resumen.set_name("Resumen").unwrap();
        resumen.write_string(0, 0, "ESTADO").unwrap();
        let hoja = libro.add_worksheet();
        hoja.set_name("ESPERA").unwrap();
        for (c, h) in ["RIT", "NOMBRE", "SITFA_ARCHIVO", "SITFA_PAGINA", "SITFA_FILA", "SITFA_SHA256"].iter().enumerate() {
            hoja.write_string(0, c as u16, *h).unwrap();
        }
        for (i, (archivo, fila)) in filas.iter().enumerate() {
            let r = i as u32 + 1;
            hoja.write_string(r, 0, "C-1-2099").unwrap();
            hoja.write_string(r, 1, "Persona Ficticia").unwrap();
            hoja.write_string(r, 2, *archivo).unwrap();
            hoja.write_number(r, 3, 1.0).unwrap();
            hoja.write_number(r, 4, *fila).unwrap();
            hoja.write_string(r, 5, "ab".repeat(32)).unwrap();
        }
        let xlsx = libro.save_to_buffer().unwrap();
        let mut zip = Vec::new();
        {
            let mut escritor = zip::ZipWriter::new(std::io::Cursor::new(&mut zip));
            escritor.start_file("consolidado/consolidado.xlsx", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut escritor, &xlsx).unwrap();
            escritor.finish().unwrap();
        }
        zip
    }

    fn trabajo_paquete(zip: Vec<u8>) -> Trabajo {
        let mut w = Trabajo {
            path: PathBuf::from("paquete.zip"),
            headers: vec!["RIT".into(), "SITFA_ARCHIVO".into(), "SITFA_FILA".into(), "SITFA_SHA256".into()],
            mode: Some(Mode::Espera),
            bytes: zip,
            ..Default::default()
        };
        for fila in ["7", "9"] {
            w.registros.push(Registro {
                values: vec![
                    Cell::Text("C-1-2099".into()),
                    Cell::Text("originales/l0/Seguimiento ficticio.xls".into()),
                    Cell::Text(fila.into()),
                    Cell::Text("AB".repeat(32)),
                ],
                ..Default::default()
            });
        }
        w
    }

    #[test]
    fn el_consolidado_se_mapea_aunque_el_paquete_guarde_originales_en_carpeta() {
        let zip = paquete_con_consolidado(&[("Seguimiento ficticio.xls", 9.0), ("Seguimiento ficticio.xls", 7.0)]);
        let base = base_paquete(&trabajo_paquete(zip)).unwrap();
        assert_eq!((base.hoja.as_str(), base.extension.as_str(), base.encabezado), ("ESPERA", "xlsx", 1));
        assert_eq!(base.filas, vec![3, 2]);
        let incompleto = paquete_con_consolidado(&[("Seguimiento ficticio.xls", 9.0)]);
        let error = base_paquete(&trabajo_paquete(incompleto)).err().unwrap();
        assert!(error.contains("1 fila(s) sin correspondencia"), "{error}");
        let doble = paquete_con_consolidado(&[("Seguimiento ficticio.xls", 9.0), ("Seguimiento ficticio.xls", 9.0), ("Seguimiento ficticio.xls", 7.0)]);
        assert!(base_paquete(&trabajo_paquete(doble)).err().unwrap().contains("repetida"));
    }

    #[test]
    fn bitacoras_comparte_la_copia_revisada_y_anota_solo_la_fila_elegida() {
        let zip = paquete_con_consolidado(&[("Seguimiento ficticio.xls", 9.0), ("Seguimiento ficticio.xls", 7.0)]);
        let mut w = trabajo_paquete(zip);
        w.registros[0].editada = Some("Edición ficticia".into());
        w.registros[0].propuesta = "Propuesta ficticia".into();
        let b = base_paquete(&w).unwrap();
        let origen = Path::new("origen.xlsx");
        let destino = Path::new("revision.xlsx");
        let general = instrucciones(&w, &b, origen, destino);
        let seleccion = [FilaBitacora { indice: 0, clasificacion: "Con observación", motivo: "Motivo ficticio" }];
        let filas = filas_bitacora(&b, &seleccion).unwrap();
        let revision = plan_excel(&w, &b, origen, destino, &filas);
        let comunes = general["escrituras"].as_array().unwrap();
        let todas = revision["escrituras"].as_array().unwrap();
        assert_eq!(&todas[..comunes.len()], comunes.as_slice());
        assert_eq!(revision["bitacoras"]["filas"], json!([3]));
        assert_eq!(todas[comunes.len()]["celdas"], json!([{"f": 3, "v": "Con observación"}]));
        assert_eq!(todas[comunes.len() + 1]["celdas"], json!([{"f": 3, "v": "Motivo ficticio"}]));
        let fuera = [FilaBitacora { indice: 2, clasificacion: "Con observación", motivo: "Ficticio" }];
        assert!(filas_bitacora(&b, &fuera).is_err());
    }

    #[test]
    fn csv_y_paquete_sin_consolidado_quedan_como_fidelidad_reducida() {
        let mut w = trabajo();
        w.path = PathBuf::from("datos.csv");
        assert!(base(&w).is_err());
        w.path = PathBuf::from("paquete.zip");
        w.paquete = None;
        let mut zip = Vec::new();
        {
            let mut escritor = zip::ZipWriter::new(std::io::Cursor::new(&mut zip));
            escritor.start_file("manifest.json", zip::write::SimpleFileOptions::default()).unwrap();
            escritor.finish().unwrap();
        }
        w.bytes = zip;
        assert!(base_paquete(&w).err().unwrap().contains("consolidado"));
    }
}
