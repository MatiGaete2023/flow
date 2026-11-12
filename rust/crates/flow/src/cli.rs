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
             [--tribunales \"Laja,Tomé\"]
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
    if flow_trabajo::paquete::es_paquete(&datos) {
        let mut trabajos = flow_trabajo::paquete::importar(nombre, &datos, corte, config)?.trabajos;
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
    let o = opciones(args, &["etapa", "hoja", "corte"]).map_err(|e| (2, e))?;
    let [archivo] = o.posicionales.as_slice() else { return Err((2, "Indica un archivo.".into())) };
    let (etapa, corte) = comunes(&o)?;
    let trabajos = cargar(Path::new(archivo), etapa, o.valor("hoja"), corte, &Config::default()).map_err(|e| (1, e))?;
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
    use flow_productos::{correos, excel, word};
    let nombre = format!("Flow {} {}", trabajo.nombre(), trabajo.corte.format("%Y-%m-%d"));
    let carpeta = excel::carpeta_nueva(salida, &nombre)?;
    let mut p = Producido { carpeta: carpeta.clone(), ..Producido::default() };
    let libro = excel::ruta_nueva(&carpeta, &format!("Observaciones {}", trabajo.nombre()), "xlsx");
    excel::observaciones(trabajo, &libro)?;
    p.creados.push(format!("Excel de observaciones: {}", libro.display()));
    let opciones = correos::Opciones { modalidades: modalidades.to_string(), hoy: None };
    for tipo in correos::Tipo::TODOS {
        if tipo.usa_modalidades(trabajo.etapa) && modalidades.trim().is_empty() {
            if tipo == correos::Tipo::Informativo || trabajo.filas.iter().any(|f| tipo.accion().is_some_and(|a| f.tiene_accion(a))) {
                p.pendientes.push(format!("{}: falta indicar las modalidades.", tipo.titulo(trabajo.etapa)));
            }
            continue;
        }
        for borrador in correos::preparar(trabajo, tipo, &opciones, contactos, &carpeta.join("correos"))? {
            let quien = if borrador.programa.is_empty() { borrador.tribunal.clone() } else { borrador.programa.clone() };
            let faltan = borrador.faltantes();
            if en_outlook && faltan.is_empty() {
                match flow_outlook::guardar_borrador(&borrador)? {
                    flow_outlook::Resultado::Guardado => p.creados.push(format!("Borrador en Outlook: {} → {quien}", borrador.asunto)),
                    flow_outlook::Resultado::Incierto(m) => {
                        p.pendientes.push(format!("Borrador para {quien}: {m} Revisa Borradores antes de reintentar."))
                    }
                }
            } else {
                let ruta = correos::guardar_eml(&borrador, &carpeta.join("correos"))?;
                p.creados.push(format!("Borrador .eml: {}", ruta.display()));
                if !faltan.is_empty() {
                    p.pendientes.push(format!("Borrador para {quien}: falta {}.", faltan.join(", ")));
                }
            }
        }
    }
    let proyectos = word::proyectos(trabajo, trabajo.corte, plantillas_usuario);
    for pr in proyectos.iter().filter(|x| !x.listo()) {
        let motivo = if pr.sin_plantilla {
            format!("no hay plantilla {} para {}", pr.tipo, pr.tribunal.titulo())
        } else {
            format!("falta {}", pr.faltantes.join(", "))
        };
        p.pendientes.push(format!("Proyecto {} {}: {motivo}.", pr.tipo, pr.rit));
    }
    if proyectos.iter().any(|x| x.listo()) {
        for ruta in word::generar(&proyectos, &carpeta.join("proyectos"), plantillas_usuario)? {
            p.creados.push(format!("Proyectos Word: {}", ruta.display()));
        }
    }
    Ok(p)
}

fn producir(args: &[String]) -> Result<(), (i32, String)> {
    let o = opciones(args, &["etapa", "hoja", "corte", "salida", "modalidades", "contactos"]).map_err(|e| (2, e))?;
    let [archivo] = o.posicionales.as_slice() else { return Err((2, "Indica un archivo.".into())) };
    let salida = o.valor("salida").ok_or((2, "Indica la carpeta de --salida.".to_string()))?;
    let (etapa, corte) = comunes(&o)?;
    let contactos = o.valor("contactos").map(|r| flow_productos::correos::Contactos::cargar(Path::new(r))).unwrap_or_default();
    for trabajo in cargar(Path::new(archivo), etapa, o.valor("hoja"), corte, &Config::default()).map_err(|e| (1, e))? {
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
    let o = opciones(args, &["salida", "corte", "emision", "tribunales"]).map_err(|e| (2, e))?;
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
    let rutas = flow_meta4::salida::publicar(&m, Path::new(salida)).map_err(|e| (1, e))?;
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
