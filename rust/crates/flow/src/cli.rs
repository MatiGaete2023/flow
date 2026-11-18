//! Línea de comandos. Código de salida: 0 bien, 1 no se pudo, 2 uso incorrecto.

use flow_dominio::{Etapa, Fecha};
use flow_motor::Config;
use flow_trabajo::{analizar_tabla, Contexto, Trabajo};
use std::path::Path;

const AYUDA: &str = "Flow — observaciones CSMP a partir de RUS

Uso:
  flow observar ARCHIVO [--etapa espera|cumplimiento|informes] [--hoja NOMBRE] [--corte AAAA-MM-DD] [--json]
  flow producir ARCHIVO --salida CARPETA [--etapa E] [--hoja H] [--corte AAAA-MM-DD]
                [--modalidades TEXTO] [--contactos contactos.json] [--outlook]
  flow anonimizar ARCHIVO_O_CARPETA... --salida CARPETA [--modo estricto|conservador]\n                  [--desplazar-dias N] [--generalizar-nacimiento] [--rechazar-no-admitidos] [--semilla-stdin]
  flow meta4 LOTE [LOTE…] --salida CARPETA_NUEVA [--corte AAAA-MM-DD] [--emision AAAA-MM-DD]
             [--tribunales \"Laja,Tomé\"] [--plantilla PLANTILLA.xlsx]
  flow meta4-regenerar REGISTROS.xlsx --salida CARPETA_NUEVA [--plantilla PLANTILLA.xlsx]
  flow consignar PAQUETE --fila ID --programa NOMBRE --archivo consignaciones.json [--corte AAAA-MM-DD]
  flow historico VISTA.html --carpeta LOTE --tribunal COD --persona ID --rut RUT [--nombre N] [--url DIRECCION]
  flow ayuda

producir crea, en una carpeta nueva dentro de --salida: el Excel de observaciones, las nóminas, los borradores
(.eml, o en Outlook con --outlook) y los proyectos Word. Los borradores siempre llevan CC a ucc_concepcion@pjud.cl.

ARCHIVO es un Excel de RUS (.xls o .xlsx) o el paquete ZIP del Descargador.
Sin --corte se usa la fecha de hoy. Sin --etapa se deduce del archivo.
No escribe en RUS ni SATURNO y no envía correos.";

pub struct Opciones {
    pub posicionales: Vec<String>,
    pub pares: Vec<(String, String)>,
    pub banderas: Vec<String>,
}

pub fn opciones(args: &[String], con_valor: &[&str]) -> Result<Opciones, String> {
    let mut o = Opciones { posicionales: Vec::new(), pares: Vec::new(), banderas: Vec::new() };
    let mut i = 0;
    while i < args.len() {
        match args[i].strip_prefix("--") {
            Some(nombre) if con_valor.contains(&nombre) => {
                let valor = args.get(i + 1).ok_or(format!("Falta el valor de --{nombre}."))?;
                o.pares.push((nombre.to_string(), valor.clone()));
                i += 1;
            }
            Some(nombre) => o.banderas.push(nombre.to_string()),
            None => o.posicionales.push(args[i].clone()),
        }
        i += 1;
    }
    Ok(o)
}

impl Opciones {
    pub fn valor(&self, nombre: &str) -> Option<&str> {
        self.pares.iter().find(|(n, _)| n == nombre).map(|(_, v)| v.as_str())
    }
    pub fn bandera(&self, nombre: &str) -> bool {
        self.banderas.iter().any(|b| b == nombre)
    }
}

/// Lee y analiza un archivo: un Excel de RUS (una o varias etapas) o un paquete ZIP del Descargador.
/// Devuelve un trabajo por etapa.
pub fn cargar(ruta: &Path, etapa: Option<Etapa>, hoja: Option<&str>, corte: Fecha, config: &Config) -> Result<Vec<Trabajo>, String> {
    let datos = std::fs::read(ruta).map_err(|e| format!("No se pudo leer {}: {e}", ruta.display()))?;
    let nombre = ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    cargar_bytes(&nombre, datos, etapa, hoja, corte, config)
}

pub fn cargar_bytes(
    nombre: &str,
    datos: Vec<u8>,
    etapa: Option<Etapa>,
    hoja: Option<&str>,
    corte: Fecha,
    config: &Config,
) -> Result<Vec<Trabajo>, String> {
    cargar_bytes_con(nombre, datos, etapa, hoja, corte, config, &Default::default())
}

/// Como `cargar`, sin repetir las constancias que el funcionario ya consignó (solo el paquete usa esas fuentes).
pub fn cargar_con(
    ruta: &Path,
    etapa: Option<Etapa>,
    hoja: Option<&str>,
    corte: Fecha,
    config: &Config,
    consignaciones: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<Vec<Trabajo>, String> {
    let datos = std::fs::read(ruta).map_err(|e| format!("No se pudo leer {}: {e}", ruta.display()))?;
    let nombre = ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    cargar_bytes_con(&nombre, datos, etapa, hoja, corte, config, consignaciones)
}

pub fn cargar_bytes_con(
    nombre: &str,
    datos: Vec<u8>,
    etapa: Option<Etapa>,
    hoja: Option<&str>,
    corte: Fecha,
    config: &Config,
    consignaciones: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<Vec<Trabajo>, String> {
    if flow_trabajo::paquete::es_paquete(&datos) {
        let mut trabajos = flow_trabajo::paquete::importar_con(nombre, &datos, corte, config, consignaciones)?.trabajos;
        if let Some(e) = etapa {
            trabajos.retain(|t| t.etapa == e);
            if trabajos.is_empty() {
                return Err(format!("El paquete no trae registros de {}.", e.nombre()));
            }
        }
        return Ok(trabajos);
    }
    let libro = flow_tabular::leer_bytes(nombre, datos)?;
    if etapa.is_none() && hoja.is_none() {
        if let Some(tipo) = flow_trabajo::consulta::detectar(&libro) {
            return Ok(vec![flow_trabajo::consulta::analizar(&libro, tipo, corte, config)?]);
        }
        let tablas = flow_tabular::leer_todas(&libro);
        if !tablas.is_empty() {
            return Ok(tablas.iter().map(|t| analizar_tabla(&libro, t, corte, config, &Contexto::default())).collect());
        }
    }
    let tabla = flow_tabular::leer_tabla(&libro, etapa, hoja)?;
    Ok(vec![analizar_tabla(&libro, &tabla, corte, config, &Contexto::default())])
}

fn imprimir(trabajo: &Trabajo) {
    println!(
        "{} · hoja «{}» · corte {} · {} registros, {} con observación",
        trabajo.nombre(),
        trabajo.hoja,
        trabajo.corte,
        trabajo.filas.len(),
        trabajo.con_observacion()
    );
    for aviso in &trabajo.avisos {
        println!("  aviso: {aviso}");
    }
    for fila in &trabajo.filas {
        println!("\nFila {} · {}", fila.numero, fila.registro.texto("rit"));
        println!("  {}", if fila.observacion.is_empty() { "(sin observación)" } else { &fila.observacion });
        if !fila.propuesta.acciones.is_empty() {
            println!("  acciones: {}", fila.propuesta.acciones.join(", "));
        }
        for marca in &fila.propuesta.marcas {
            println!("  revisar (nivel {}): {}", marca.nivel, marca.texto);
        }
        for aviso in &fila.propuesta.avisos {
            println!("  aviso: {aviso}");
        }
    }
}

fn observar(args: &[String]) -> Result<(), (i32, String)> {
    let o = opciones(args, &["etapa", "hoja", "corte", "consignaciones"]).map_err(|e| (2, e))?;
    let [archivo] = o.posicionales.as_slice() else { return Err((2, "Indica un archivo.".into())) };
    let (etapa, corte) = comunes(&o)?;
    let consignaciones = o.valor("consignaciones").map(|r| leer_consignaciones(Path::new(r))).transpose()?.unwrap_or_default();
    let trabajos = cargar_con(Path::new(archivo), etapa, o.valor("hoja"), corte, &Config::default(), &consignaciones).map_err(|e| (1, e))?;
    if o.bandera("json") {
        println!("{}", serde_json::to_string_pretty(&trabajos).map_err(|e| (1, e.to_string()))?);
        return Ok(());
    }
    for (i, trabajo) in trabajos.iter().enumerate() {
        if i > 0 {
            println!();
        }
        imprimir(trabajo);
    }
    Ok(())
}

/// Etapa, hoja y corte comunes a los comandos.
fn comunes(o: &Opciones) -> Result<(Option<Etapa>, Fecha), (i32, String)> {
    let etapa = match o.valor("etapa") {
        Some(t) => Some(Etapa::desde(t).ok_or((2, format!("Etapa desconocida: {t}.")))?),
        None => None,
    };
    let corte = match o.valor("corte") {
        Some(t) => t.parse().map_err(|_| (2, format!("Fecha de corte inválida: {t}. Usa AAAA-MM-DD.")))?,
        None => flow_tabular::libro::hoy(),
    };
    Ok((etapa, corte))
}

/// Resultado de producir todo: lo creado y lo que quedó pendiente.
#[derive(Debug, Default)]
pub struct Producido {
    pub carpeta: std::path::PathBuf,
    pub creados: Vec<String>,
    pub pendientes: Vec<String>,
}

/// Crea todos los productos del trabajo en una carpeta nueva.
pub fn producir_todo(
    trabajo: &Trabajo,
    salida: &Path,
    modalidades: &str,
    contactos: &flow_productos::correos::Contactos,
    en_outlook: bool,
    plantillas_usuario: Option<&Path>,
) -> Result<Producido, String> {
    let datos = flow_trabajo::almacen::carpeta_datos();
    producir_interno(
        trabajo,
        salida,
        modalidades,
        contactos,
        if en_outlook { Some((&datos, &flow_outlook::guardar_borrador)) } else { None },
        plantillas_usuario,
    )
}
type Guardador<'a> = dyn Fn(&flow_productos::correos::Borrador) -> Result<flow_outlook::Resultado, String> + 'a;
fn producir_interno(
    trabajo: &Trabajo,
    salida: &Path,
    modalidades: &str,
    contactos: &flow_productos::correos::Contactos,
    outlook: Option<(&Path, &Guardador<'_>)>,
    plantillas_usuario: Option<&Path>,
) -> Result<Producido, String> {
    use flow_productos::{correos, excel, word};
    let nombre = format!("Flow {} {}", trabajo.nombre(), trabajo.corte.format("%Y-%m-%d"));
    let carpeta = excel::carpeta_nueva(salida, &nombre)?;
    let mut p = Producido { carpeta: carpeta.clone(), ..Producido::default() };
    let libro = excel::ruta_nueva(&carpeta, &format!("Observaciones {}", trabajo.nombre()), "xlsx");
    excel::observaciones(trabajo, &libro)?;
    p.creados.push(format!("Excel de observaciones: {}", libro.display()));
    // Las modalidades escritas filtran las filas solo si todas se reconocen (RES, AMB, FAE, DCE).
    let opciones = correos::Opciones {
        modalidades: modalidades.to_string(),
        claves_modalidad: correos::claves_de_texto(modalidades),
        ..Default::default()
    };
    // Como «Preparar todos» de Python (`prepare_required_drafts`): exige modalidades; el reporte de proyectos y la
    // gestión particular se preparan a pedido en la ventana.
    let preparados = match correos::preparar_todos(trabajo, &opciones, contactos, &carpeta.join("correos")) {
        Ok((lista, fallidos)) => {
            p.pendientes.extend(fallidos);
            lista
        }
        Err(e) => {
            p.pendientes.push(format!("Correos: {e}"));
            Vec::new()
        }
    };
    {
        for borrador in preparados {
            let quien = if borrador.programa.is_empty() { borrador.tribunal.clone() } else { borrador.programa.clone() };
            let faltan = borrador.faltantes();
            let mut respaldo = true;
            if let Some((datos, guardar)) = outlook.filter(|_| faltan.is_empty()) {
                match crate::borradores::ejecutar(datos, &borrador, guardar) {
                    crate::borradores::Resultado::Guardado => {
                        p.creados.push(format!("Borrador en Outlook: {} → {quien}", borrador.asunto));
                        respaldo = false;
                    }
                    crate::borradores::Resultado::YaGuardado => {
                        p.creados.push(format!("Ya estaba en Outlook; no se duplicó: {} → {quien}", borrador.asunto));
                        respaldo = false;
                    }
                    crate::borradores::Resultado::Incierto(m) => p.pendientes.push(format!(
                        "Borrador para {quien}: guardado incierto. {m} La copia .eml es solo respaldo: revise Outlook antes de usarla."
                    )),
                    crate::borradores::Resultado::Bloqueado(e) => p
                        .pendientes
                        .push(format!("Borrador para {quien}: no se intentó guardar en Outlook. {e} La copia .eml es solo respaldo.")),
                    crate::borradores::Resultado::Fallido(e) => p
                        .pendientes
                        .push(format!("Borrador para {quien}: no guardado en Outlook. {e} Se intenta conservar una copia .eml.")),
                }
            }
            if respaldo {
                match correos::guardar_eml(&borrador, &carpeta.join("correos")) {
                    Ok(ruta) => p.creados.push(format!("Borrador .eml: {}", ruta.display())),
                    Err(e) => p.pendientes.push(format!("Borrador para {quien}: {e}")),
                }
            }
            if !faltan.is_empty() {
                p.pendientes.push(format!("Borrador para {quien}: falta {}.", faltan.join(", ")));
            }
        }
    }

    let proyectos = word::proyectos(trabajo, trabajo.corte, plantillas_usuario);
    for pr in &proyectos {
        let motivo = if pr.sin_plantilla {
            format!("no hay plantilla {} para {}", pr.tipo, pr.tribunal.titulo())
        } else if !pr.listo() {
            format!("falta {}", pr.vacias().join(", "))
        } else if !pr.faltantes.is_empty() {
            // Como el Asistente Python: se genera con «[COMPLETAR X]» para completarlo en el Word.
            format!("generado con [COMPLETAR …] en {}; complétalo en el Word", pr.faltantes.join(", "))
        } else {
            continue;
        };
        p.pendientes.push(format!("Proyecto {} {}: {motivo}.", pr.tipo, pr.rit));
    }
    if proyectos.iter().any(|x| x.listo()) {
        match word::generar(&proyectos, &carpeta, plantillas_usuario) {
            Ok(rutas) => {
                for ruta in rutas {
                    p.creados.push(format!("Proyectos Word: {}", ruta.display()));
                }
            }
            Err(e) => p.pendientes.push(format!("Proyectos Word: {e}")),
        }
    }
    let reporte = serde_json::json!({"creados":p.creados,"pendientes":p.pendientes});
    if let Err(e) = std::fs::write(carpeta.join("Productos.json"), serde_json::to_vec_pretty(&reporte).map_err(|e| e.to_string())?) {
        p.pendientes.push(format!("No se pudo guardar el inventario de productos: {e}"));
    }

    Ok(p)
}

fn producir(args: &[String]) -> Result<(), (i32, String)> {
    let o = opciones(args, &["etapa", "hoja", "corte", "salida", "modalidades", "contactos", "consignaciones"]).map_err(|e| (2, e))?;
    let [archivo] = o.posicionales.as_slice() else { return Err((2, "Indica un archivo.".into())) };
    let salida = o.valor("salida").ok_or((2, "Indica la carpeta de --salida.".to_string()))?;
    let (etapa, corte) = comunes(&o)?;
    let contactos = o
        .valor("contactos")
        .map(|r| flow_productos::correos::Contactos::cargar(Path::new(r)))
        .transpose()
        .map_err(|e| (1, e))?
        .unwrap_or_default();
    let consignaciones = o.valor("consignaciones").map(|r| leer_consignaciones(Path::new(r))).transpose()?.unwrap_or_default();
    for trabajo in cargar_con(Path::new(archivo), etapa, o.valor("hoja"), corte, &Config::default(), &consignaciones).map_err(|e| (1, e))? {
        let p = producir_todo(&trabajo, Path::new(salida), o.valor("modalidades").unwrap_or(""), &contactos, o.bandera("outlook"), None)
            .map_err(|e| (1, e))?;
        println!("Carpeta: {}", p.carpeta.display());
        for linea in &p.creados {
            println!("  creado · {linea}");
        }
        for linea in &p.pendientes {
            println!("  pendiente · {linea}");
        }
    }
    Ok(())
}

/// Meta 4 desde lotes de captura locales. La carpeta de salida debe ser nueva.
fn meta4(args: &[String]) -> Result<(), (i32, String)> {
    let o = opciones(args, &["salida", "corte", "emision", "tribunales", "plantilla"]).map_err(|e| (2, e))?;
    if o.posicionales.is_empty() {
        return Err((2, "Indica al menos la carpeta de un lote capturado.".into()));
    }
    let salida = o.valor("salida").ok_or((2, "Indica la carpeta nueva de --salida.".to_string()))?;
    let fecha = |nombre: &str| -> Result<Fecha, (i32, String)> {
        match o.valor(nombre) {
            Some(t) => t.parse().map_err(|_| (2, format!("Fecha inválida en --{nombre}: {t}. Usa AAAA-MM-DD."))),
            None => Ok(flow_tabular::libro::hoy()),
        }
    };
    let (corte, emision) = (fecha("corte")?, fecha("emision")?);
    if corte > flow_tabular::libro::hoy() {
        return Err((2, "La fecha de corte no puede ser futura.".into()));
    }
    let lotes: Vec<std::path::PathBuf> = o.posicionales.iter().map(std::path::PathBuf::from).collect();
    let tribunales: Vec<String> = o.valor("tribunales").unwrap_or("").split(',').map(|t| t.trim().to_string()).collect();
    let m = flow_meta4::calcular(&lotes, corte, emision, &tribunales).map_err(|e| (1, e))?;
    let rutas =
        flow_meta4::salida::publicar_con_plantilla(&m, Path::new(salida), o.valor("plantilla").map(Path::new)).map_err(|e| (1, e))?;
    println!(
        "Meta 4: {} registros, {} alertas, {} por revisar.",
        m.registros.len(),
        m.registros.iter().filter(|r| !r.alerta.is_empty()).count(),
        m.registros.iter().filter(|r| r.requiere_revision()).count()
    );
    for ruta in rutas {
        println!("  creado · {}", ruta.display());
    }
    for aviso in &m.avisos {
        println!("  aviso · {aviso}");
    }
    Ok(())
}

/// Regenera Meta 4 desde el Registros.xlsx corregido y el reporte original.
fn meta4_regenerar(args: &[String]) -> Result<(), (i32, String)> {
    let o = opciones(args, &["salida", "plantilla"]).map_err(|e| (2, e))?;
    if o.posicionales.len() != 1
        || !o.banderas.is_empty()
        || o.valor("salida").is_none()
        || o.pares.len() != (1 + usize::from(o.valor("plantilla").is_some()))
    {
        return Err((2, "Uso: flow meta4-regenerar REGISTROS.xlsx --salida CARPETA_NUEVA [--plantilla PLANTILLA.xlsx].".into()));
    }
    let destino = o.valor("salida").ok_or((2, "Falta --salida.".to_string()))?;
    let rutas = flow_meta4::regenerar::ejecutar_con_plantilla(
        Path::new(&o.posicionales[0]),
        Path::new(destino),
        o.valor("plantilla").map(Path::new),
    )
    .map_err(|e| (1, e))?;
    println!("Meta 4 regenerado desde Registros.xlsx revisado:");
    for ruta in rutas {
        println!("  creado · {}", ruta.display());
    }
    Ok(())
}

pub fn ejecutar(args: &[String]) -> i32 {
    let resultado = match args.first().map(String::as_str) {
        None | Some("ayuda" | "--ayuda" | "-h" | "--help") => {
            println!("{AYUDA}");
            Ok(())
        }
        Some("observar") => observar(&args[1..]),
        Some("producir") => producir(&args[1..]),
        Some("anonimizar") => anonimizar(&args[1..]),
        Some("meta4") => meta4(&args[1..]),
        Some("meta4-regenerar") => meta4_regenerar(&args[1..]),
        Some("consignar") => consignar(&args[1..]),
        Some("historico") => historico(&args[1..]),
        Some(otro) => Err((2, format!("Comando desconocido: {otro}.\n\n{AYUDA}"))),
    };
    match resultado {
        Ok(()) => 0,
        Err((codigo, mensaje)) => {
            eprintln!("{mensaje}");
            codigo
        }
    }
}

/// Lee el archivo de consignaciones (`{"id de fila": ["clave de programa", …]}`); ausente = ninguna.
fn leer_consignaciones(ruta: &Path) -> Result<std::collections::BTreeMap<String, Vec<String>>, (i32, String)> {
    match std::fs::read(ruta) {
        Ok(b) => serde_json::from_slice(&b).map_err(|_| (1, format!("{} no es un archivo de consignaciones legible.", ruta.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
        Err(e) => Err((1, format!("No se pudo leer {}: {e}", ruta.display()))),
    }
}

/// Marca una constancia como consignada por el funcionario (nunca al generar ni exportar). Idempotente; solo vale si el motor la
/// propone para esa fila.
fn consignar(args: &[String]) -> Result<(), (i32, String)> {
    let o = opciones(args, &["fila", "programa", "archivo", "corte", "etapa"]).map_err(|e| (2, e))?;
    let [paquete] = o.posicionales.as_slice() else { return Err((2, "Uso: flow consignar PAQUETE --fila ID --programa NOMBRE --archivo consignaciones.json".into())) };
    let (fila, programa, archivo) = (
        o.valor("fila").ok_or((2, "Falta --fila ID.".to_string()))?,
        o.valor("programa").ok_or((2, "Falta --programa NOMBRE.".to_string()))?,
        Path::new(o.valor("archivo").ok_or((2, "Falta --archivo consignaciones.json.".to_string()))?),
    );
    let (etapa, corte) = comunes(&o)?;
    let mut consignaciones = leer_consignaciones(archivo)?;
    let trabajos = cargar_con(Path::new(paquete), etapa, None, corte, &Config::default(), &consignaciones).map_err(|e| (1, e))?;
    let clave = flow_dominio::evidencia::clave_programa(programa);
    let propuesta = trabajos.iter().flat_map(|t| &t.filas).find(|f| f.id == fila).map(|f| f.propuesta.otros_programas.iter().any(|p| p.clave == clave));
    match propuesta {
        None => return Err((1, "Esa fila no existe en el paquete.".into())),
        Some(false) => return Err((1, "El motor no propone esa constancia para la fila (ya consignada o no vigente).".into())),
        Some(true) => {}
    }
    let lista = consignaciones.entry(fila.to_string()).or_default();
    if !lista.contains(&clave) {
        lista.push(clave);
    }
    std::fs::write(archivo, serde_json::to_vec_pretty(&consignaciones).map_err(|e| (1, e.to_string()))?).map_err(|e| (1, format!("No se pudo guardar: {e}")))?;
    println!("Consignada. Vuelve a ejecutar observar/producir con --consignaciones {}.", archivo.display());
    Ok(())
}

/// Captura MANUAL del Ingreso Histórico Protección desde un HTML guardado de la ventana «Historial Menores».
fn historico(args: &[String]) -> Result<(), (i32, String)> {
    use flow_trabajo::historico_manual::{capturar, Persona};
    let o = opciones(args, &["carpeta", "tribunal", "persona", "rut", "nombre", "tribunal-nombre", "url"]).map_err(|e| (2, e))?;
    let [html] = o.posicionales.as_slice() else { return Err((2, "Uso: flow historico VISTA.html --carpeta LOTE --tribunal COD --persona ID --rut RUT [--nombre N] [--url DIRECCION]".into())) };
    let pedir = |n: &str| o.valor(n).ok_or((2, format!("Falta --{n}.")));
    let datos = std::fs::read(html).map_err(|e| (1, format!("No se pudo leer {html}: {e}")))?;
    let ahora = chrono::Local::now().to_rfc3339();
    let persona = Persona {
        tribunal: pedir("tribunal")?,
        tribunal_nombre: o.valor("tribunal-nombre").unwrap_or(""),
        person_id: pedir("persona")?,
        rut: o.valor("rut").unwrap_or(""),
        nombre: o.valor("nombre").unwrap_or(""),
        url: o.valor("url").unwrap_or(""),
    };
    let e = capturar(Path::new(pedir("carpeta")?), &datos, &persona, &ahora).map_err(|e| (1, e))?;
    println!("{} · pertenencia {} · {} programas", e["estado"].as_str().unwrap_or(""), e["ownership"].as_str().unwrap_or(""), e["filas"].as_array().map_or(0, Vec::len));
    for f in e["filas"].as_array().into_iter().flatten() {
        println!("   {} | {} | {}", f["programa"].as_str().unwrap_or(""), f["fecha_egreso"].as_str().unwrap_or(""), f["causal_egreso"].as_str().unwrap_or(""));
    }
    if e["estado"] == "FALLIDA" { Err((1, "No se pudo leer la tabla del histórico.".into())) } else { Ok(()) }
}

fn anonimizar(args: &[String]) -> Result<(), (i32, String)> {
    let o = opciones(args, &["salida", "modo", "desplazar-dias"]).map_err(|e| (2, e))?;
    if !o.banderas.iter().all(|k| ["generalizar-nacimiento", "rechazar-no-admitidos", "semilla-stdin"].contains(&k.as_str()))
        || o.posicionales.is_empty()
    {
        return Err((2, "Uso: flow anonimizar ARCHIVO_O_CARPETA... --salida CARPETA".into()));
    }
    let destino = o.valor("salida").ok_or((2, "Falta --salida CARPETA.".into()))?;
    let modo = match o.valor("modo").unwrap_or("estricto") {
        "estricto" => flow_anonimo::Modo::Estricto,
        "conservador" => flow_anonimo::Modo::Conservador,
        _ => return Err((2, "Modo desconocido: use estricto o conservador.".into())),
    };
    let dias =
        o.valor("desplazar-dias").unwrap_or("0").parse().map_err(|_| (2, "El desplazamiento debe ser un número entero de días.".into()))?;
    let mut semilla = String::new();
    if o.bandera("semilla-stdin") {
        use std::io::Read;
        std::io::stdin()
            .take(4097)
            .read_to_string(&mut semilla)
            .map_err(|_| (1, "No se pudo leer la semilla de la entrada estándar.".into()))?;
        if semilla.len() > 4096 {
            return Err((2, "La semilla supera el tamaño admitido.".into()));
        }
        semilla = semilla.trim_end_matches(['\r', '\n']).into();
    }
    let opciones = flow_anonimo::Opciones {
        modo,
        desplazar_dias: dias,
        generalizar_nacimiento: o.bandera("generalizar-nacimiento"),
        semilla,
        rechazar_no_admitidos: o.bandera("rechazar-no-admitidos"),
    };
    let entradas = o.posicionales.iter().map(std::path::PathBuf::from).collect::<Vec<_>>();
    let r = flow_anonimo::lote::ejecutar(&entradas, Path::new(destino), opciones, &std::sync::atomic::AtomicBool::new(false), |_| {})
        .map_err(|e| (1, e))?;
    println!(
        "{} copias verificadas; {} entradas excluidas.\nResultados: {}\nRevise INFORME.json y las copias antes de compartirlas.",
        r.informe.archivos.len(),
        r.informe.excluidos,
        r.carpeta.display()
    );
    Ok(())
}

#[cfg(test)]
mod pruebas_producir {
    use super::*;
    #[test]
    fn fallo_outlook_conserva_eml_continua_word_y_reintento_no_duplica_guardados() {
        let temp = tempfile::tempdir().unwrap();
        let origen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/evidencia_p0/golden/Origen_ESPERA.xlsx");
        let t = cargar(&origen, Some(Etapa::Espera), None, "2026-10-08".parse().unwrap(), &Config::default()).unwrap().remove(0);
        let mut contactos = flow_productos::correos::Contactos::default();
        for f in &t.filas {
            contactos.programas.insert(f.registro.texto("programa"), "ficticio@example.invalid".into());
        }
        for t in ["LAJA", "MULCHEN", "TOME"] {
            contactos.tribunales.insert(t.into(), "ficticio@example.invalid".into());
        }
        let n = std::cell::Cell::new(0);
        let guardar = |_: &flow_productos::correos::Borrador| {
            n.set(n.get() + 1);
            if n.get() == 2 {
                Err("Fallo ficticio al adjuntar".into())
            } else {
                Ok(flow_outlook::Resultado::Guardado)
            }
        };
        let datos = temp.path().join("datos");
        let r = producir_interno(&t, temp.path(), "todas las modalidades", &contactos, Some((&datos, &guardar)), None).unwrap();
        let primero = n.get();
        assert!(primero >= 3);
        assert!(r.creados.iter().any(|s| s.contains(".eml")));
        assert!(r.creados.iter().any(|s| s.contains(".docx")));
        assert!(r.pendientes.iter().any(|s| s.contains("Fallo ficticio al adjuntar")));
        assert!(r.carpeta.join("Productos.json").is_file());
        let r2 = producir_interno(&t, temp.path(), "todas las modalidades", &contactos, Some((&datos, &guardar)), None).unwrap();
        assert_eq!(n.get(), primero + 1);
        assert!(r2.creados.iter().any(|s| s.contains("no se duplicó")));
        // Un solo Word conjunto, como `generate_projects`.
        assert_eq!(r.creados.iter().filter(|s| s.contains(".docx")).count(), 1);
        assert!(r.creados.iter().any(|s| s.contains("Resoluciones_")));
        // Sin modalidades, como «Preparar todos» de Python: no se prepara ningún correo.
        let r3 = producir_interno(&t, temp.path(), "", &contactos, None, None).unwrap();
        assert!(!r3.creados.iter().any(|s| s.contains(".eml")));
        assert!(r3.pendientes.iter().any(|s| s.contains("al menos una modalidad")));
    }
}

#[cfg(test)]
mod pruebas_integracion {
    use super::*;
    use flow_trabajo::historico_manual::rut_de_url;

    fn datos(nombre: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../flow-trabajo/tests/datos").join(nombre)
    }

    #[test]
    fn rut_de_la_direccion_de_la_ventana() {
        assert_eq!(rut_de_url("https://x.invalido/PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante=11111111&RUT_DV=k"), "11111111K");
        assert_eq!(rut_de_url("https://x.invalido/?a=1"), "");
        assert_eq!(rut_de_url("https://x.invalido/?RUT_Litigante=abc&RUT_DV=1"), "");
    }

    #[test]
    fn consignar_por_linea_de_comandos_es_idempotente_y_cambia_la_propuesta() {
        let temp = tempfile::tempdir().unwrap();
        let paquete = datos("paquete_integracion.zip");
        let corte = "2026-10-08";
        let antes = cargar(&paquete, None, None, corte.parse().unwrap(), &Config::default()).unwrap();
        let fila = antes.iter().flat_map(|t| &t.filas).find(|f| !f.propuesta.otros_programas.is_empty()).unwrap();
        let (id, programa) = (fila.id.clone(), fila.propuesta.otros_programas[0].programa.clone());
        let archivo = temp.path().join("consignaciones.json");
        let args = |extra: &[&str]| -> Vec<String> {
            ["consignar", paquete.to_str().unwrap(), "--fila", &id, "--programa", &programa, "--archivo", archivo.to_str().unwrap(), "--corte", corte]
                .iter()
                .chain(extra)
                .map(|s| s.to_string())
                .collect()
        };
        assert_eq!(ejecutar(&args(&[])), 0);
        assert_eq!(ejecutar(&args(&[])), 1, "ya consignada: el motor no la propone y el comando lo dice");
        let guardado: std::collections::BTreeMap<String, Vec<String>> = serde_json::from_slice(&std::fs::read(&archivo).unwrap()).unwrap();
        assert_eq!(guardado.len(), 1);
        let despues = cargar_con(&paquete, None, None, corte.parse().unwrap(), &Config::default(), &guardado).unwrap();
        let fila = despues.iter().flat_map(|t| &t.filas).find(|f| f.id == id).unwrap();
        assert!(fila.propuesta.otros_programas.is_empty() && !fila.observacion.contains("cuenta con ingreso efectivo"));
        let otra = ["consignar", paquete.to_str().unwrap(), "--fila", "no-existe", "--programa", "X", "--archivo", archivo.to_str().unwrap(), "--corte", corte];
        assert_eq!(ejecutar(&otra.iter().map(|s| s.to_string()).collect::<Vec<_>>()), 1);
    }

    #[test]
    fn historico_por_linea_de_comandos_con_la_estructura_real() {
        let temp = tempfile::tempdir().unwrap();
        let vista = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures_integracion/historial_menores_ficticio.html");
        let carpeta = temp.path().join("lote");
        let args: Vec<String> = ["historico", vista.to_str().unwrap(), "--carpeta", carpeta.to_str().unwrap(), "--tribunal", "113", "--persona", "p1", "--rut", "11.111.111-1"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(ejecutar(&args), 0);
        let manifiesto: serde_json::Value = serde_json::from_slice(&std::fs::read(carpeta.join("historico.json")).unwrap()).unwrap();
        let e = &manifiesto["capturas"]["113|p1"];
        assert_eq!((e["estado"].as_str(), e["ownership"].as_str()), (Some("LEIDA"), Some("VERIFICADA")));
        assert_eq!(e["filas"][0]["programa"], "PRM - PROGRAMA FICTICIO UNO");
    }
}
