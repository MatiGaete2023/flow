//! Línea de comandos. Código de salida: 0 bien, 1 no se pudo, 2 uso incorrecto.

use flow_dominio::{Etapa, Fecha};
use flow_motor::Config;
use flow_trabajo::{analizar_tabla, Contexto, Trabajo};
use std::path::Path;

const AYUDA: &str = "Flow — observaciones CSMP a partir de RUS

Uso:
  flow observar ARCHIVO [--etapa espera|cumplimiento|informes] [--hoja NOMBRE] [--corte AAAA-MM-DD] [--json]
  flow ayuda

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

pub fn ejecutar(args: &[String]) -> i32 {
    let resultado = match args.first().map(String::as_str) {
        None | Some("ayuda" | "--ayuda" | "-h" | "--help") => {
            println!("{AYUDA}");
            Ok(())
        }
        Some("observar") => observar(&args[1..]),
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
