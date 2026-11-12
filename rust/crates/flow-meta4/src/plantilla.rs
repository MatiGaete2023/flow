//! Adapta los estilos de una plantilla XLSX a los cuatro informes Meta 4.
//! Solo toma estilos, alturas, anchuras y tema; nunca copia fórmulas, vínculos,
//! dibujos, macros ni contenido del archivo aportado por el usuario.
//! Equivalencia con capture_templates/rebuild del generador Python.
use crate::salida::HOJAS;
use crate::tribunales::TRIBUNALES;
use flow_dominio::Celda;
use flow_tabular::Hoja;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const LIMITE_PLANTILLA: usize = 20 * 1024 * 1024;
const LIMITE_XML: u64 = 8 * 1024 * 1024;
const PAPEL: [&str; 7] = ["título", "informe", "fecha", "cabecera", "dato", "sin registros", "total"];

#[derive(Clone)]
struct Muestra {
    estilo: [[usize; 8]; 7],
    altura: [Option<String>; 7],
    columnas: String,
}

fn attr(e: &BytesStart<'_>, clave: &[u8]) -> Option<String> {
    e.attributes().flatten()
        .find(|a| a.key.as_ref() == clave)
        .map(|a| String::from_utf8_lossy(a.value.as_ref()).into_owned())
}
fn nombre(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}
fn zip_leer<R: Read + std::io::Seek>(archivo: &mut ZipArchive<R>, ruta: &str) -> Result<String, String> {
    let mut entrada = archivo.by_name(ruta).map_err(|_| format!("Falta {ruta} en la plantilla XLSX."))?;
    if entrada.size() > LIMITE_XML {
        return Err(format!("{ruta} excede el tamaño permitido."));
    }
    let mut texto = String::new();
    entrada.read_to_string(&mut texto).map_err(|_| format!("{ruta} no es XML UTF-8 válido."))?;
    Ok(texto)
}
fn sheet_names<R: Read + std::io::Seek>(archivo: &mut ZipArchive<R>) -> Result<BTreeMap<String, String>, String> {
    let libro = zip_leer(archivo, "xl/workbook.xml")?;
    let relaciones = zip_leer(archivo, "xl/_rels/workbook.xml.rels")?;
    let mut objetivos = BTreeMap::new();
    let mut lector = Reader::from_str(&relaciones);
    loop {
        match lector.read_event().map_err(|e| e.to_string())? {
            Event::Empty(e) | Event::Start(e) if nombre(&e) == "Relationship" => {
                let Some(id) = attr(&e, b"Id") else { continue };
                let Some(target) = attr(&e, b"Target") else { continue };
                if attr(&e, b"Type").is_some_and(|t| t.ends_with("/worksheet")) {
                    let archivo = if let Some(p) = target.strip_prefix("/xl/") {
                        format!("xl/{p}")
                    } else if target.starts_with("worksheets/") {
                        format!("xl/{target}")
                    } else {
                        return Err("Relación de hoja de plantilla no admitida.".into());
                    };
                    objetivos.insert(id, archivo);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    let mut hojas = BTreeMap::new();
    let mut lector = Reader::from_str(&libro);
    loop {
        match lector.read_event().map_err(|e| e.to_string())? {
            Event::Empty(e) | Event::Start(e) if nombre(&e) == "sheet" => {
                let (Some(titulo), Some(id)) = (attr(&e, b"name"), attr(&e, b"r:id")) else { continue };
                if let Some(destino) = objetivos.get(&id) {
                    if hojas.insert(titulo.trim().to_string(), destino.clone()).is_some() {
                        return Err("La plantilla tiene nombres de hojas duplicados.".into());
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(hojas)
}
fn columnas(xml: &str) -> Result<String, String> {
    let desde = xml.find("<cols>").or_else(|| xml.find("<cols "))
        .ok_or("La plantilla no define los anchos de columna.")?;
    let fin = xml[desde..].find("</cols>").ok_or("Bloque de columnas incompleto.")? + desde + "</cols>".len();
    Ok(xml[desde..fin].to_string())
}
fn columna_numero(ref_celda: &str) -> Option<usize> {
    let letras = ref_celda.bytes().take_while(|b| b.is_ascii_alphabetic());
    let mut resultado = 0usize;
    for c in letras {
        resultado = resultado.checked_mul(26)?.checked_add((c.to_ascii_uppercase() - b'A' + 1) as usize)?;
    }
    resultado.checked_sub(1)
}
fn fila_numero(ref_celda: &str) -> Option<usize> {
    ref_celda.bytes().skip_while(|b| b.is_ascii_alphabetic())
        .map(char::from).collect::<String>().parse::<usize>().ok()
}
fn analizar_celdas(xml: &str) -> Result<(BTreeMap<(usize, usize), usize>, BTreeMap<usize, String>), String> {
    let mut estilos = BTreeMap::new();
    let mut alturas = BTreeMap::new();
    let mut lector = Reader::from_str(xml);
    loop {
        match lector.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) | Event::Empty(e) => match nombre(&e).as_str() {
                "row" => {
                    if let (Some(n), Some(h)) = (attr(&e, b"r"), attr(&e, b"ht")) {
                        if let Ok(fila) = n.parse::<usize>() { alturas.insert(fila, h); }
                    }
                }
                "c" => {
                    if let Some(referencia) = attr(&e, b"r") {
                        if let (Some(fila), Some(col)) = (fila_numero(&referencia), columna_numero(&referencia)) {
                            let estilo = attr(&e, b"s").and_then(|s| s.parse().ok()).unwrap_or(0);
                            estilos.insert((fila, col), estilo);
                        }
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok((estilos, alturas))
}
fn texto_fila(hoja: &Hoja, n: usize) -> String {
    hoja.filas.get(n).and_then(|f| f.first()).map(Celda::texto).unwrap_or_default()
}
fn muestra(hoja: &Hoja, xml: &str) -> Result<Muestra, String> {
    let fila = |p: &dyn Fn(&str)->bool| hoja.filas.iter().enumerate()
        .find(|(n, _)| p(&texto_fila(hoja, *n))).map(|(i, _)| i + 1);
    let titulo = fila(&|t| TRIBUNALES.contains(&t)).ok_or(format!("La hoja {} no contiene tribunal modelo.", hoja.nombre))?;
    if !texto_fila(hoja, titulo).starts_with("Informe de Ingresos") {
        return Err(format!("La hoja {} no contiene el informe bajo el tribunal modelo.", hoja.nombre));
    }
    let cabecera = fila(&|t| t == "RIT").ok_or(format!("La hoja {} no contiene RIT.", hoja.nombre))?;
    let sin = fila(&|t| t == "SIN REGISTROS").ok_or(format!("La hoja {} no contiene SIN REGISTROS.", hoja.nombre))?;
    let total = fila(&|t| t.starts_with("TOTAL")).ok_or(format!("La hoja {} no contiene TOTAL.", hoja.nombre))?;
    let filas = [titulo, titulo + 1, titulo + 2, cabecera, cabecera + 1, sin, total];
    let (estilos, alturas) = analizar_celdas(xml)?;
    let mut e = [[0usize; 8]; 7];
    let mut h: [Option<String>; 7] = std::array::from_fn(|_| None);
    for (papel, numero) in filas.into_iter().enumerate() {
        for col in 0..8 {
            e[papel][col] = *estilos.get(&(numero, col)).unwrap_or(&0);
        }
        h[papel] = alturas.get(&numero).cloned();
    }
    Ok(Muestra { estilo: e, altura: h, columnas: columnas(xml)? })
}
fn papel_de(texto: &str) -> Option<usize> {
    if texto == "RIT" { Some(3) }
    else if texto == "SIN REGISTROS" { Some(5) }
    else if texto.starts_with("TOTAL") { Some(6) }
    else if TRIBUNALES.contains(&texto) { Some(0) }
    else if texto.starts_with("Informe de Ingresos") { Some(1) }
    else if texto.starts_with("Fecha Emisión") { Some(2) }
    else if texto.is_empty() { None }
    else { Some(4) }
}
fn celda_modificada(e: &BytesStart<'_>, nuevo_estilo: usize) -> BytesStart<'static> {
    let mut salida = BytesStart::new(String::from_utf8_lossy(e.name().as_ref()).into_owned());
    for a in e.attributes().flatten() {
        if a.key.as_ref() != b"s" {
            let k = String::from_utf8_lossy(a.key.as_ref()).into_owned();
            let v = String::from_utf8_lossy(a.value.as_ref()).into_owned();
            salida.push_attribute((k.as_str(), v.as_str()));
        }
    }
    let n = nuevo_estilo.to_string();
    salida.push_attribute(("s", n.as_str()));
    salida
}
fn fila_modificada(e: &BytesStart<'_>, altura: Option<&str>) -> BytesStart<'static> {
    let mut salida = BytesStart::new(String::from_utf8_lossy(e.name().as_ref()).into_owned());
    for a in e.attributes().flatten() {
        if a.key.as_ref() != b"ht" && a.key.as_ref() != b"customHeight" {
            let k = String::from_utf8_lossy(a.key.as_ref()).into_owned();
            let v = String::from_utf8_lossy(a.value.as_ref()).into_owned();
            salida.push_attribute((k.as_str(), v.as_str()));
        }
    }
    if let Some(altura) = altura {
        salida.push_attribute(("ht", altura));
        salida.push_attribute(("customHeight", "1"));
    }
    salida
}
fn reestilizar(xml: &str, hoja: &Hoja, muestra: Option<&Muestra>, fecha_ids: &BTreeMap<usize, usize>) -> Result<Vec<u8>, String> {
    let mut lector = Reader::from_str(xml);
    let mut escritor = Writer::new(Vec::new());
    let mut saltar_columnas = false;
    let mut fila_actual = 0usize;
    loop {
        let evento = lector.read_event().map_err(|e| e.to_string())?;
        match evento {
            Event::Start(ref e) if nombre(e) == "cols" && muestra.is_some() => {
                escritor.get_mut().extend_from_slice(muestra.unwrap().columnas.as_bytes());
                saltar_columnas = true;
            }
            Event::End(ref e) if e.local_name().as_ref() == b"cols" && saltar_columnas => {
                saltar_columnas = false;
            }
            _ if saltar_columnas => {}
            Event::Start(ref e) if nombre(e) == "row" => {
                fila_actual = attr(e, b"r").and_then(|s| s.parse().ok()).unwrap_or(0);
                let rol = papel_de(&texto_fila(hoja, fila_actual.saturating_sub(1)));
                let altura = muestra.and_then(|m| rol.and_then(|p| m.altura[p].as_deref()));
                escritor.write_event(Event::Start(fila_modificada(e, altura))).map_err(|e| e.to_string())?;
            }
            Event::Empty(ref e) | Event::Start(ref e) if nombre(e) == "c" => {
                let posicion = attr(e, b"r").unwrap_or_default();
                let col = columna_numero(&posicion).unwrap_or(usize::MAX);
                let fila = fila_numero(&posicion).unwrap_or(fila_actual);
                let rol = papel_de(&texto_fila(hoja, fila.saturating_sub(1)));
                let mut estilo = if let (Some(m), Some(p)) = (muestra, rol) {
                    if col < 8 { m.estilo[p][col] } else { 0 }
                } else { 0 };
                if muestra.is_some() && rol == Some(4) && col < 8 {
                    let es_fecha = hoja.filas.get(fila.saturating_sub(1)).and_then(|r| r.get(col))
                        .is_some_and(|c| matches!(c, Celda::Fecha(_)));
                    if es_fecha {
                        estilo = *fecha_ids.get(&estilo).ok_or("No existe estilo de fecha en la plantilla.")?;
                    }
                }
                let nueva = celda_modificada(e, estilo);
                if matches!(evento, Event::Empty(_)) {
                    escritor.write_event(Event::Empty(nueva)).map_err(|e| e.to_string())?;
                } else {
                    escritor.write_event(Event::Start(nueva)).map_err(|e| e.to_string())?;
                }
            }
            Event::Eof => break,
            otro => escritor.write_event(otro.into_owned()).map_err(|e| e.to_string())?,
        }
    }
    Ok(escritor.into_inner())
}
fn poner_attr(elemento: &str, nombre: &str, valor: &str) -> Result<String, String> {
    let inicio = format!("{nombre}=\"");
    if let Some(i) = elemento.find(&inicio) {
        let desde = i + inicio.len();
        let hasta = elemento[desde..].find('"').ok_or("Atributo XML sin cierre.")? + desde;
        let mut nuevo = elemento.to_string();
        nuevo.replace_range(desde..hasta, valor);
        Ok(nuevo)
    } else {
        let pos = elemento.find('>').ok_or("Etiqueta XML sin cierre.")?;
        let pos = if elemento[..pos].ends_with('/') { pos - 1 } else { pos };
        let mut nuevo = elemento.to_string();
        nuevo.insert_str(pos, &format!(" {nombre}=\"{valor}\""));
        Ok(nuevo)
    }
}
fn estilos_con_fecha(original: &str, indices: &BTreeSet<usize>) -> Result<(String, BTreeMap<usize, usize>), String> {
    let inicio = original.find("<cellXfs").ok_or("La plantilla carece de cellXfs.")?;
    let apertura = inicio + original[inicio..].find('>').ok_or("cellXfs XML incompleto.")? + 1;
    let cierre = apertura + original[apertura..].find("</cellXfs>").ok_or("cellXfs sin cierre.")?;
    let contenido = &original[apertura..cierre];
    let mut xfs = Vec::new();
    let mut cursor = 0usize;
    while let Some(i) = contenido[cursor..].find("<xf") {
        let desde = cursor + i;
        let fin_apertura = desde + contenido[desde..].find('>').ok_or("xf incompleto.")? + 1;
        let hasta = if contenido[desde..fin_apertura].ends_with("/>") {
            fin_apertura
        } else {
            fin_apertura + contenido[fin_apertura..].find("</xf>").ok_or("xf sin cierre.")? + "</xf>".len()
        };
        xfs.push(contenido[desde..hasta].to_string());
        cursor = hasta;
    }
    if xfs.is_empty() || indices.iter().any(|i| *i >= xfs.len()) {
        return Err("Un estilo de celda de la plantilla no existe en cellXfs.".into());
    }
    let mut mayor = 163usize;
    for parte in original.split("numFmtId=\"").skip(1) {
        if let Some(n) = parte.split('"').next().and_then(|n| n.parse::<usize>().ok()) {
            mayor = mayor.max(n);
        }
    }
    let formato_id = mayor + 1;
    let mut nuevos = String::new();
    let mut ids = BTreeMap::new();
    for (i, base) in indices {
        let estilo = xfs[*i].clone();
        let (inicio_tag, resto) = estilo.split_once('>').ok_or("xf de estilo incompleto.")?;
        let cierre_tag = format!("{inicio_tag}>");
        let primero = poner_attr(&cierre_tag, "numFmtId", &formato_id.to_string())?;
        let segundo = poner_attr(&primero, "applyNumberFormat", "1")?;
        let final_xf = format!("{segundo}{resto}");
        ids.insert(*i, xfs.len() + ids.len());
        nuevos.push_str(&final_xf);
    }
    let apertura_vieja = &original[inicio..apertura];
    let apertura_nueva = poner_attr(apertura_vieja, "count", &(xfs.len() + ids.len()).to_string())?;
    let mut nuevo = String::new();
    nuevo.push_str(&original[..inicio]);
    nuevo.push_str(&apertura_nueva);
    nuevo.push_str(contenido);
    nuevo.push_str(&nuevos);
    nuevo.push_str(&original[cierre..]);

    let fmt = format!("<numFmt numFmtId=\"{formato_id}\" formatCode=\"dd-mm-yyyy\"/>");
    if let Some(inicio) = nuevo.find("<numFmts") {
        let fin_apertura = inicio + nuevo[inicio..].find('>').ok_or("numFmts incompleto.")? + 1;
        let cierre = fin_apertura + nuevo[fin_apertura..].find("</numFmts>").ok_or("numFmts sin cierre.")?;
        let n = nuevo[inicio..fin_apertura].to_string();
        let anterior = n.split("count=\"").nth(1).and_then(|p| p.split('"').next()).and_then(|p| p.parse::<usize>().ok()).unwrap_or(0);
        let cabecera = poner_attr(&n, "count", &(anterior+1).to_string())?;
        nuevo.replace_range(inicio..fin_apertura, &cabecera);
        let cierre = cierre + (cabecera.len() - n.len());
        nuevo.insert_str(cierre, &fmt);
    } else {
        let root = nuevo.find("<styleSheet").ok_or("styles.xml no es un estilo de Excel.")?;
        let fin = root + nuevo[root..].find('>').ok_or("styleSheet incompleto.")? + 1;
        nuevo.insert_str(fin, &format!("<numFmts count=\"1\">{fmt}</numFmts>"));
    }
    Ok((nuevo, ids))
}
/// Aplica al libro generado la plantilla de estilos. No altera la plantilla ni el contenido.
pub fn aplicar(plantilla: &Path, generado: &Path) -> Result<(), String> {
    if plantilla == generado { return Err("La plantilla no puede ser el libro de salida.".into()); }
    let bytes = std::fs::read(plantilla).map_err(|e| format!("No se pudo abrir plantilla: {e}"))?;
    if bytes.len() > LIMITE_PLANTILLA { return Err("La plantilla XLSX supera 20 MiB.".into()); }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|_| "La plantilla no es un XLSX válido.".to_string())?;
    if zip.len() > 512 { return Err("La plantilla contiene demasiadas partes.".into()); }
    if (0..zip.len()).any(|i| {
        let f = zip.by_index(i).ok();
        f.is_some_and(|f| f.name().ends_with("vbaProject.bin") || f.name().starts_with("xl/externalLinks/"))
    }) {
        return Err("La plantilla contiene macros o vínculos externos, que no se admiten.".into());
    }
    let origen = flow_tabular::leer(plantilla)?;
    if origen.formato != flow_tabular::Formato::Xlsx { return Err("La plantilla debe ser XLSX.".into()); }
    let rutas = sheet_names(&mut zip)?;
    let mut muestras = BTreeMap::new();
    for (hoja, _, _, _) in HOJAS {
        let contenido = origen.hojas.iter().find(|h| h.nombre.trim() == hoja)
            .ok_or(format!("La plantilla no tiene la hoja {hoja}."))?;
        let ruta = rutas.get(hoja).ok_or(format!("No se encontró el XML de {hoja}."))?;
        let xml = zip_leer(&mut zip, ruta)?;
        if xml.contains("<mergeCells") || xml.contains("<drawing") {
            return Err(format!("La hoja {hoja} incluye celdas combinadas o dibujos; usa una plantilla con filas de estilo simples."));
        }
        muestras.insert(hoja.to_string(), muestra(contenido, &xml)?);
    }
    let mut fechas = BTreeSet::new();
    for m in muestras.values() { fechas.extend(m.estilo[4].into_iter()); }
    let (estilos, fecha_ids) = estilos_con_fecha(&zip_leer(&mut zip, "xl/styles.xml")?, &fechas)?;
    let tema = zip.by_name("xl/theme/theme1.xml").ok().map(|mut e| {
        let mut b = Vec::new();
        e.read_to_end(&mut b).map_err(|e| e.to_string())?;
        if b.len() > LIMITE_XML as usize { return Err("Tema demasiado grande.".into()); }
        Ok::<Vec<u8>, String>(b)
    }).transpose()?;

    let generado_bytes = std::fs::read(generado).map_err(|e| format!("No se pudo leer la planilla generada: {e}"))?;
    let generado_libro = flow_tabular::leer(generado)?;
    let mut fuente = ZipArchive::new(Cursor::new(generado_bytes)).map_err(|_| "La planilla generada no es XLSX.".to_string())?;
    let temporal = generado.with_extension("meta4-temporal-xlsx");
    if temporal.exists() { return Err("Ya existe un temporal de plantilla: no se sobrescribe.".into()); }
    let operacion = || -> Result<(), String> {
        let mut escritor = ZipWriter::new(std::fs::File::create(&temporal).map_err(|e| e.to_string())?);
        let opciones = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..fuente.len() {
            let mut archivo = fuente.by_index(i).map_err(|e| e.to_string())?;
            let nombre = archivo.name().to_string();
            if archivo.is_dir() { escritor.add_directory(&nombre, opciones).map_err(|e| e.to_string())?; continue; }
            if archivo.size() > 80 * 1024 * 1024 { return Err("La planilla generada supera el límite permitido.".into()); }
            let mut contenido = Vec::new();
            archivo.read_to_end(&mut contenido).map_err(|e| e.to_string())?;
            if nombre == "xl/styles.xml" { contenido = estilos.as_bytes().to_vec(); }
            else if nombre == "xl/theme/theme1.xml" {
                if let Some(ref tema) = tema { contenido = tema.clone(); }
            } else if nombre.starts_with("xl/worksheets/sheet") && nombre.ends_with(".xml") {
                let numero = nombre.strip_prefix("xl/worksheets/sheet").unwrap_or("")
                    .strip_suffix(".xml").unwrap_or("").parse::<usize>().unwrap_or(0);
                if (1..=5).contains(&numero) {
                    let hoja = generado_libro.hojas.get(numero - 1).ok_or("Hoja generada inexistente.")?;
                    let modelo = muestras.get(&hoja.nombre);
                    if numero <= 4 && modelo.is_none() { return Err("El orden de las hojas generadas no coincide con Meta 4.".into()); }
                    contenido = reestilizar(std::str::from_utf8(&contenido).map_err(|_| "XML de hoja ilegible.")?, hoja, modelo, &fecha_ids)?;
                }
            }
            escritor.start_file(nombre, opciones).map_err(|e| e.to_string())?;
            escritor.write_all(&contenido).map_err(|e| e.to_string())?;
        }
        escritor.finish().map_err(|e| e.to_string())?;
        // Control estructural y de contenido: los estilos no autorizan un cambio de datos.
        let revisado = flow_tabular::leer(&temporal)?;
        if generado_libro.hojas.len() != revisado.hojas.len() {
            return Err("La aplicación de la plantilla cambió el número de hojas.".into());
        }
        for (antes, despues) in generado_libro.hojas.iter().zip(&revisado.hojas) {
            if antes.nombre != despues.nombre || antes.filas != despues.filas {
                return Err(format!("La plantilla modificó datos de la hoja {}; se rechazó.", antes.nombre));
            }
        }
        Ok(())
    };
    if let Err(e) = operacion() { let _ = std::fs::remove_file(&temporal); return Err(e); }
    // Solo sobre un libro de salida recién creado por Flow.
    std::fs::rename(&temporal, generado).map_err(|e| {
        let _ = std::fs::remove_file(&temporal);
        format!("No se pudo aplicar la plantilla: {e}")
    })
}
