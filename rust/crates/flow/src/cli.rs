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
  flow ayuda

producir crea, en una carpeta nueva dentro de --salida: el Excel de observaciones, las nóminas, los borradores
(.eml, o en Outlook con --outlook) y los proyectos Word. Los borradores siempre llevan CC a ucc_concepcion@pjud.cl.

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

/// Lee y analiza un archivo de RUS.
pub fn cargar(ruta: &Path, etapa: Option<Etapa>, hoja: Option<&str>, corte: Fecha, config: &Config) -> Result<Trabajo, String> {
    let libro = flow_tabular::leer(ruta)?;
    let tabla = flow_tabular::leer_tabla(&libro, etapa, hoja)?;
    Ok(analizar_tabla(&libro, &tabla, corte, config, &Contexto::default()))
}

fn observar(args: &[String]) -> Result<(), (i32, String)> {
    let o = opciones(args, &["etapa", "hoja", "corte"]).map_err(|e| (2, e))?;
    let [archivo] = o.posicionales.as_slice() else { return Err((2, "Indica un archivo.".into())) };
    let etapa = match o.valor("etapa") {
        Some(t) => Some(Etapa::desde(t).ok_or((2, format!("Etapa desconocida: {t}.")))?),
        None => None,
    };
    let corte = match o.valor("corte") {
        Some(t) => t.parse().map_err(|_| (2, format!("Fecha de corte inválida: {t}. Usa AAAA-MM-DD.")))?,
        None => flow_tabular::libro::hoy(),
    };
    let trabajo = cargar(Path::new(archivo), etapa, o.valor("hoja"), corte, &Config::default()).map_err(|e| (1, e))?;
    if o.bandera("json") {
        println!("{}", serde_json::to_string_pretty(&trabajo).map_err(|e| (1, e.to_string()))?);
        return Ok(());
    }
    println!(
        "{} · hoja «{}» · corte {} · {} registros, {} con observación",
        trabajo.etapa.nombre(),
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
    let nombre = format!("Flow {} {}", trabajo.etapa.nombre(), trabajo.corte.format("%Y-%m-%d"));
    let carpeta = excel::carpeta_nueva(salida, &nombre)?;
    let mut p = Producido { carpeta: carpeta.clone(), ..Producido::default() };
    let libro = excel::ruta_nueva(&carpeta, &format!("Observaciones {}", trabajo.etapa.nombre()), "xlsx");
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
                    flow_outlook::Resultado::Incierto(m) => p.pendientes.push(format!("Borrador para {quien}: {m} Revisa Borradores antes de reintentar.")),
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
    let trabajo = cargar(Path::new(archivo), etapa, o.valor("hoja"), corte, &Config::default()).map_err(|e| (1, e))?;
    let p = producir_todo(&trabajo, Path::new(salida), o.valor("modalidades").unwrap_or(""), &contactos, o.bandera("outlook"), None)
        .map_err(|e| (1, e))?;
    println!("Carpeta: {}", p.carpeta.display());
    for linea in &p.creados {
        println!("  creado · {linea}");
    }
    for linea in &p.pendientes {
        println!("  pendiente · {linea}");
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
