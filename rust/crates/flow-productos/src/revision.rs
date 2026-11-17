//! Exportación local de la revisión. Todos los textos se escriben como texto, nunca como fórmulas.
use flow_trabajo::{revision_bitacoras::Clasificacion as C, revision_informe::Informe};
use rust_xlsxwriter::{Format, Workbook};
use serde_json::json;
use std::{collections::BTreeSet, path::{Path, PathBuf}, sync::atomic::{AtomicBool, Ordering}};

#[derive(Debug)]
pub struct Producto { pub carpeta: PathBuf, pub excel: PathBuf, pub json: PathBuf }

pub fn etiqueta(c: C) -> &'static str {
    match c { C::SinObservacion => "Sin observación", C::ConObservacion => "Con observación",
        C::NoVerificable => "No verificables", C::FueraDeAlcance => "Fuera de alcance" }
}

fn habitual(f: &flow_trabajo::revision_informe::FilaRevision, aliases: &[&str]) -> String {
    let normalizar = flow_dominio::texto::normalizar_columna;
    f.valores.as_object().into_iter().flat_map(|m| m.iter()).find_map(|(k,v)| {
        aliases.iter().any(|alias| normalizar(alias) == normalizar(k)).then(|| v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string()))
    }).unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn publicar(origen: &Path, destino: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    use std::ffi::CString;
    extern "C" { fn renameat2(olddirfd: i32, oldpath: *const std::ffi::c_char, newdirfd: i32, newpath: *const std::ffi::c_char, flags: u32) -> i32; }
    let origen = CString::new(origen.as_os_str().as_bytes()).map_err(|_|std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let destino = CString::new(destino.as_os_str().as_bytes()).map_err(|_|std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    // RENAME_NOREPLACE: la comprobación y publicación son una sola operación del sistema.
    let resultado = unsafe { renameat2(-100,origen.as_ptr(),-100,destino.as_ptr(),1) };
    if resultado == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}
#[cfg(windows)]
fn publicar(origen: &Path, destino: &Path) -> std::io::Result<()> { std::fs::rename(origen,destino) }
#[cfg(not(any(windows,target_os = "linux")))]
fn publicar(_origen: &Path, _destino: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"La publicación segura de carpetas requiere Windows o Linux."))
}

pub fn exportar(informe: &Informe, seleccion: Option<&[usize]>, destino: &Path, cancelar: &AtomicBool) -> Result<Producto, String> {
    exportar_con(informe,seleccion,destino,cancelar,|_| Ok(()))
}

fn exportar_con(informe: &Informe, seleccion: Option<&[usize]>, destino: &Path, cancelar: &AtomicBool,
    despues_excel: impl FnOnce(&Path) -> Result<(), String>) -> Result<Producto, String> {
    if !destino.is_dir() { return Err("Elige una carpeta de destino existente.".into()); }
    let indices = match seleccion {
        None => (0..informe.filas.len()).collect::<BTreeSet<_>>(),
        Some(xs) => xs.iter().copied().collect::<BTreeSet<_>>(),
    };
    if indices.is_empty() { return Err("La selección de exportación está vacía.".into()); }
    if indices.iter().any(|i| *i >= informe.filas.len()) { return Err("La selección contiene un registro inexistente.".into()); }
    let verificar = || if cancelar.load(Ordering::Relaxed) { Err("Exportación cancelada; no se publicó ningún resultado.".to_string()) } else { Ok(()) };
    verificar()?;
    let temporal = tempfile::Builder::new().prefix(".revision-incompleta-").tempdir_in(destino).map_err(|e| e.to_string())?;
    let nombre = format!("Revision_bitacoras_{}", informe.corte);
    let excel_nombre = format!("{nombre}.xlsx");
    let json_nombre = format!("{nombre}.json");
    let mut libro = Workbook::new();
    let cabecera = Format::new().set_bold().set_font_color(rust_xlsxwriter::Color::White).set_background_color(rust_xlsxwriter::Color::RGB(0x2A3048)).set_text_wrap();
    let texto = Format::new().set_num_format("@").set_text_wrap();
    let mut exportados = [0usize;4];
    let cabezas = ["Sistema","Tribunal","Causa","Ingreso","Etapa","Desde","Hasta","Corte","Control","Motivo",
        "Eventos del centro","Última fecha","Cobertura","Propuesta motor","Edición revisada","Reglas","Avisos","Excluido por motor",
        "Revisada","IDs origen","Consulta","Archivo","Tabla","Fila","Capturada","Versión reglas","RIT","Nombre","RUT","Programa"];
    for (clase, posicion) in [(C::SinObservacion,0),(C::NoVerificable,2),(C::ConObservacion,1),(C::FueraDeAlcance,3)] {
        let hoja = libro.add_worksheet();
        hoja.set_name(etiqueta(clase)).map_err(|e| e.to_string())?;
        hoja.set_freeze_panes(1,0).map_err(|e| e.to_string())?;
        for (col, nombre) in cabezas.iter().enumerate() {
            hoja.write_string_with_format(0,col as u16,*nombre,&cabecera).map_err(|e| e.to_string())?;
            hoja.set_column_width(col as u16, if [9,13,14,15,16].contains(&col) { 65.0 } else { 22.0 }).map_err(|e| e.to_string())?;
        }
        let mut n = 1u32;
        for &i in &indices {
            verificar()?;
            let f = &informe.filas[i]; if f.revision.clasificacion != clase { continue; }
            let propuesta = f.propuesta.as_ref();
            let join = |campo: fn(&flow_trabajo::revision_informe::Procedencia)->&str| f.procedencia.iter().map(campo).collect::<Vec<_>>().join(" | ");
            let valores = vec![f.identidad.sistema.clone(),f.identidad.tribunal.clone(),f.identidad.causa.clone(),f.identidad.ingreso.clone(),f.tipo.clone(),
                informe.desde.to_string(),informe.hasta.to_string(),informe.corte.to_string(),etiqueta(clase).into(),f.revision.motivo.clone(),
                f.revision.eventos_elegibles.to_string(),f.revision.ultima_fecha.map(|d|d.to_string()).unwrap_or_default(),
                if f.revision.cobertura_parcial {"Parcial o desconocida"}else{"Completa para el período"}.into(),
                propuesta.map(|p|p.observacion.clone()).unwrap_or_default(),f.edicion.clone().unwrap_or_default(),
                propuesta.map(|p|p.reglas.join(" | ")).unwrap_or_default(),propuesta.map(|p|p.avisos.join(" | ")).unwrap_or_default(),
                propuesta.map(|p|if p.excluido {"Sí"}else{"No"}).unwrap_or("Sin aplicar").into(),
                if f.revisada {"Sí"}else{"No"}.into(),join(|p|&p.registro_id),join(|p|&p.consulta_id),join(|p|&p.archivo),
                join(|p|&p.tabla),join(|p|&p.fila),join(|p|&p.capturada),informe.version_reglas.clone().unwrap_or_default(),
                habitual(f,&["RIT","CAUSA"]),habitual(f,&["NOMBRE","NOMBRE NNA","NOMBRE MENOR"]),
                habitual(f,&["RUT","RUT MENOR","RUT NNA"]),habitual(f,&["PROGRAMA","NOMBRE PROGRAMA"])];
            for (col, valor) in valores.iter().enumerate() { hoja.write_string_with_format(n,col as u16,valor,&texto).map_err(|e|e.to_string())?; }
            n += 1; exportados[posicion] += 1;
        }
        hoja.autofilter(0,0,n.saturating_sub(1),cabezas.len() as u16-1).map_err(|e|e.to_string())?;
    }
    let alcance = if seleccion.is_some() {"Seleccionados explícitamente"}else{"Todos los clasificados"};
    let resumen = libro.add_worksheet();
    resumen.set_name("Resumen").map_err(|e|e.to_string())?;
    let campos = [("Huella del paquete",informe.huella_paquete.clone()),("Contrato",informe.contrato.clone()),
        ("Alcance",alcance.into()),("Desde",informe.desde.to_string()),("Hasta",informe.hasta.to_string()),
        ("Corte",informe.corte.to_string()),("Filas fuente",informe.filas_fuente.to_string()),
        ("Registros lógicos totales",informe.filas.len().to_string()),("Registros exportados",indices.len().to_string()),
        ("Duplicados de registros",informe.duplicados_registros.to_string()),("Motor aplicado",informe.motor_aplicado.to_string()),
        ("Versión de reglas",informe.version_reglas.clone().unwrap_or_default())];
    for (fila,(nombre,valor)) in campos.iter().enumerate() {
        resumen.write_string_with_format(fila as u32,0,*nombre,&cabecera).map_err(|e|e.to_string())?;
        resumen.write_string_with_format(fila as u32,1,valor,&texto).map_err(|e|e.to_string())?;
    }
    let totales = informe.conteos();
    for (i,nombre) in ["Sin observación","Con observación","No verificables","Fuera de alcance"].iter().enumerate() {
        resumen.write_string(14+i as u32,0,*nombre).map_err(|e|e.to_string())?;
        resumen.write_number(14+i as u32,1,totales[i] as f64).map_err(|e|e.to_string())?;
        resumen.write_number(14+i as u32,2,exportados[i] as f64).map_err(|e|e.to_string())?;
    }
    resumen.write_string(13,1,"Total clasificado").map_err(|e|e.to_string())?;
    resumen.write_string(13,2,"Exportado").map_err(|e|e.to_string())?;
    resumen.set_column_width(0,32).map_err(|e|e.to_string())?;
    resumen.set_column_width(1,75).map_err(|e|e.to_string())?;
    verificar()?;
    libro.save(temporal.path().join(&excel_nombre)).map_err(|e|e.to_string())?;
    despues_excel(temporal.path())?;
    verificar()?;
    let filas = indices.iter().map(|&i| &informe.filas[i]).collect::<Vec<_>>();
    let reporte = json!({"version":1,"alcance":alcance,"huella_paquete":informe.huella_paquete,"contrato":informe.contrato,
        "desde":informe.desde,"hasta":informe.hasta,"corte":informe.corte,"filas_fuente":informe.filas_fuente,
        "total_registros":informe.filas.len(),"conteos_totales":totales,"conteos_exportados":exportados,
        "duplicados_registros":informe.duplicados_registros,"version_reglas":informe.version_reglas,
        "motor_aplicado":informe.motor_aplicado,"filas":filas});
    std::fs::write(temporal.path().join(&json_nombre),serde_json::to_vec_pretty(&reporte).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    verificar()?;
    // El sufijo aleatorio de tempfile evita colisiones entre exportaciones concurrentes.
    let sufijo = temporal.path().file_name().unwrap_or_default().to_string_lossy();
    let carpeta = destino.join(format!("{nombre}_{}",sufijo.trim_start_matches(".revision-incompleta-")));
    if carpeta.exists() { return Err("La carpeta final ya existe; vuelva a exportar.".into()); }
    publicar(temporal.path(),&carpeta).map_err(|e|e.to_string())?;
    Ok(Producto { excel:carpeta.join(excel_nombre),json:carpeta.join(json_nombre),carpeta })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_trabajo::{revision_bitacoras::{clasificar,Evidencia},revision_informe::{FilaRevision,Identidad}};
    use std::io::Read;
    fn informe() -> Informe {
        let d = "2026-11-17".parse().unwrap();
        let evidencia = Evidencia { admitido:true,identidad_verificada:true,contradictoria:false,cobertura_completa:Some((d,d)),eventos:Some(vec![]) };
        let revision = clasificar(&evidencia,d,d).unwrap();
        Informe { version:1,contrato:"1.0.0".into(),huella_paquete:"a".repeat(64),desde:d,hasta:d,corte:d,filas_fuente:1,duplicados_registros:0,
            filas:vec![FilaRevision { identidad:Identidad { sistema:"RUS".into(),tribunal:"901".into(),causa:"100".into(),ingreso:"201".into() },
                tipo:"ESPERA".into(),valores:json!({"RIT":"X-1-2099","RUT":"11.111.111-1","NOMBRE":"Persona Ficticia","PROGRAMA":"Programa Ficticio"}),procedencia:vec![],lecturas:vec![],evidencia,revision,propuesta:None,
                edicion:Some("=HYPERLINK(\"https://ficticio.invalid\")".into()),revisada:true }],motor_aplicado:false,version_reglas:None }
    }
    #[test]
    fn productos_conciliados_y_texto_seguro_sin_sobrescribir() {
        let base = tempfile::tempdir().unwrap();
        let reporte = informe();
        let stop = AtomicBool::new(false);
        let a = exportar(&reporte,None,base.path(),&stop).unwrap();
        let b = exportar(&reporte,Some(&[0]),base.path(),&stop).unwrap();
        assert_ne!(a.carpeta,b.carpeta);
        let data: serde_json::Value = serde_json::from_slice(&std::fs::read(&a.json).unwrap()).unwrap();
        assert_eq!(data["conteos_totales"],json!([1,0,0,0]));
        assert_eq!(data["conteos_exportados"],json!([1,0,0,0]));
        assert_eq!(data["filas"][0]["edicion"],reporte.filas[0].edicion.as_deref().unwrap());
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&a.excel).unwrap()).unwrap();
        let mut strings = String::new(); zip.by_name("xl/sharedStrings.xml").unwrap().read_to_string(&mut strings).unwrap();
        assert!(strings.contains("HYPERLINK"));
        for valor in ["X-1-2099","Persona Ficticia","11.111.111-1","Programa Ficticio"] { assert!(strings.contains(valor)); }
        let mut sheet = String::new(); zip.by_name("xl/worksheets/sheet1.xml").unwrap().read_to_string(&mut sheet).unwrap();
        assert!(!sheet.contains("<f>"));
        let mut book = String::new(); zip.by_name("xl/workbook.xml").unwrap().read_to_string(&mut book).unwrap();
        for nombre in ["Sin observación","Con observación","No verificables","Fuera de alcance","Resumen"] { assert!(book.contains(nombre)); }
    }
    #[test]
    fn cancelacion_y_seleccion_invalida_no_publican_nada() {
        let base = tempfile::tempdir().unwrap(); let reporte = informe();
        assert!(exportar(&reporte,None,base.path(),&AtomicBool::new(true)).is_err());
        assert!(exportar(&reporte,Some(&[]),base.path(),&AtomicBool::new(false)).is_err());
        assert!(exportar(&reporte,Some(&[20]),base.path(),&AtomicBool::new(false)).is_err());
        assert_eq!(std::fs::read_dir(base.path()).unwrap().count(),0);
    }
    #[test]
    fn cancelacion_despues_del_excel_y_fallo_json_limpian_temporales() {
        let base = tempfile::tempdir().unwrap(); let reporte = informe();
        let stop = AtomicBool::new(false);
        let anterior = exportar(&reporte,None,base.path(),&stop).unwrap();
        let huella = std::fs::read(&anterior.excel).unwrap();
        let resultado = exportar_con(&reporte,None,base.path(),&stop,|p| {
            assert!(p.join("Revision_bitacoras_2026-11-17.xlsx").is_file());
            stop.store(true,Ordering::Relaxed); Ok(())
        });
        assert!(resultado.is_err());
        stop.store(false,Ordering::Relaxed);
        let resultado = exportar_con(&reporte,None,base.path(),&stop,|p| {
            // Un directorio con el nombre JSON fuerza un fallo real de escritura.
            std::fs::create_dir(p.join("Revision_bitacoras_2026-11-17.json")).map_err(|e|e.to_string())?;
            Ok(())
        });
        assert!(resultado.is_err());
        assert_eq!(std::fs::read_dir(base.path()).unwrap().count(),1);
        assert_eq!(std::fs::read(&anterior.excel).unwrap(),huella);
        assert!(anterior.json.is_file());
    }
    #[test]
    fn publicacion_atomica_no_reemplaza_un_destino_que_aparece() {
        let base = tempfile::tempdir().unwrap();
        let origen = base.path().join("temporal"); let destino = base.path().join("final");
        std::fs::create_dir(&origen).unwrap();
        std::fs::write(origen.join("producto"),b"nuevo").unwrap();
        std::fs::create_dir(&destino).unwrap();
        assert!(publicar(&origen,&destino).is_err());
        assert!(origen.join("producto").is_file());
        assert_eq!(std::fs::read_dir(&destino).unwrap().count(),0);
        std::fs::write(destino.join("producto"),b"anterior").unwrap();
        assert!(publicar(&origen,&destino).is_err());
        assert_eq!(std::fs::read(destino.join("producto")).unwrap(),b"anterior");
    }
}

