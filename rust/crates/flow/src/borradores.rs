//! Recibos locales de Outlook: persistir antes y después del efecto y no repetir un resultado incierto.
use flow_productos::correos::{self, Borrador};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Estado {
    Preparando,
    Guardado,
    Incierto,
    NoGuardado,
}
#[derive(Serialize, Deserialize)]
struct Recibo {
    version: u32,
    id: String,
    estado: Estado,
    fecha: String,
}
pub enum Resultado {
    Guardado,
    YaGuardado,
    Incierto(String),
    Fallido(String),
    Bloqueado(String),
}
fn agregar(h: &mut Sha256, raw: &[u8]) {
    h.update((raw.len() as u64).to_le_bytes());
    h.update(raw);
}
fn identidad(b: &Borrador) -> Result<String, String> {
    let mut h = Sha256::new();
    let mut para = correos::direcciones(&b.para).into_iter().map(|s| s.to_lowercase()).collect::<Vec<_>>();
    para.sort();
    let mut cc = correos::direcciones(&b.cc).into_iter().map(|s| s.to_lowercase()).collect::<Vec<_>>();
    cc.sort();
    let mut filas = b.filas.clone();
    filas.sort();
    let payload = serde_json::to_vec(&(b.tipo, &b.tribunal, &b.programa, para, cc, b.asunto.trim(), b.cuerpo.replace("\r\n", "\n"), filas))
        .map_err(|_| "No se pudo identificar el borrador.")?;
    agregar(&mut h, &payload);
    for p in &b.adjuntos {
        let f = File::open(p).map_err(|_| "No se pudo verificar un adjunto del borrador.")?;
        if f.metadata().map_err(|_| "No se pudo verificar un adjunto.")?.len() > 80 * 1024 * 1024 {
            return Err("Un adjunto supera el tamaño permitido.".into());
        }
        let mut raw = vec![];
        f.take(80 * 1024 * 1024 + 1).read_to_end(&mut raw).map_err(|_| "No se pudo verificar un adjunto.")?;
        if p.extension().is_some_and(|s| s.eq_ignore_ascii_case("xlsx")) {
            // El XLSX incluye fecha de creación y envoltura ZIP variables. La identidad usa sus partes de contenido,
            // sin rutas locales ni propiedades del autor, para reconocer el mismo lote al regenerar las nóminas.
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(raw)).map_err(|_| "Un adjunto Excel no es válido.")?;
            if zip.len() > 4000 {
                return Err("Un adjunto Excel contiene demasiadas partes.".into());
            }
            let mut miembros = std::collections::BTreeMap::new();
            let mut total = 0u64;
            for i in 0..zip.len() {
                let mut m = zip.by_index(i).map_err(|_| "No se pudo verificar un adjunto Excel.")?;
                if m.is_dir() || m.name().starts_with("docProps/") {
                    continue;
                }
                total = total.saturating_add(m.size());
                if total > 200 * 1024 * 1024 {
                    return Err("Un adjunto Excel expandido supera el tamaño permitido.".into());
                }
                let mut raw = vec![];
                m.read_to_end(&mut raw).map_err(|_| "No se pudo verificar una parte del adjunto.")?;
                if miembros.insert(m.name().to_string(), raw).is_some() {
                    return Err("El adjunto Excel tiene partes repetidas.".into());
                }
            }
            for (n, r) in miembros {
                agregar(&mut h, n.as_bytes());
                agregar(&mut h, &r);
            }
        } else {
            agregar(&mut h, &raw);
        }
    }
    Ok(format!("{:x}", h.finalize()))
}
fn guardar(ruta: &Path, recibo: &Recibo) -> Result<(), String> {
    let raw = serde_json::to_vec(recibo).map_err(|_| "No se pudo preparar el recibo local.")?;
    let mut temp = tempfile::NamedTempFile::new_in(ruta.parent().unwrap())
        .map_err(|_| "No se pudo preparar el recibo; no se intentará guardar en Outlook.")?;
    temp.write_all(&raw).and_then(|_| temp.as_file().sync_all()).map_err(|_| "No se pudo persistir el recibo del borrador.")?;
    temp.persist(ruta).map_err(|_| "No se pudo publicar el recibo local del borrador.")?;
    Ok(())
}
fn leer(ruta: &Path, id: &str) -> Result<Option<Recibo>, String> {
    let f = match File::open(ruta) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("No se pudo leer el recibo anterior; no se repetirá el guardado en Outlook.".into()),
    };
    let mut raw = vec![];
    f.take(16385).read_to_end(&mut raw).map_err(|_| "No se pudo leer el recibo anterior.")?;
    if raw.len() > 16384 {
        return Err("El recibo anterior no es válido; no se repetirá Outlook.".into());
    }
    let r: Recibo = serde_json::from_slice(&raw).map_err(|_| "El recibo anterior no es válido; revise Outlook antes de recuperarlo.")?;
    if r.version != 1 || r.id != id {
        return Err("El recibo anterior no corresponde al borrador; no se repetirá Outlook.".into());
    }
    Ok(Some(r))
}
fn bloqueo(carpeta: &Path, id: &str) -> Result<File, String> {
    std::fs::create_dir_all(carpeta).map_err(|_| "No se pudo preparar el diario de borradores.")?;
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(carpeta.join(format!("{id}.lock")))
        .map_err(|_| "No se pudo abrir el control de guardado del borrador.")?;
    f.try_lock().map_err(|_| "Otro proceso está trabajando con este borrador; espere antes de reintentar.")?;
    Ok(f)
}
pub fn estado(datos: &Path, b: &Borrador) -> Result<Option<Estado>, String> {
    let id = identidad(b)?;
    Ok(leer(&datos.join("recibos_outlook").join(format!("{id}.json")), &id)?.map(|r| r.estado))
}
pub fn resolver(datos: &Path, b: &Borrador, existe: bool) -> Result<(), String> {
    let id = identidad(b)?;
    let carpeta = datos.join("recibos_outlook");
    let _lock = bloqueo(&carpeta, &id)?;
    let ruta = carpeta.join(format!("{id}.json"));
    let Some(mut r) = leer(&ruta, &id)? else {
        return Err("No hay un guardado incierto para este borrador.".into());
    };
    if !matches!(r.estado, Estado::Preparando | Estado::Incierto) {
        return Err("El borrador no está pendiente de comprobación en Outlook.".into());
    }
    r.estado = if existe { Estado::Guardado } else { Estado::NoGuardado };
    r.fecha = chrono::Utc::now().to_rfc3339();
    guardar(&ruta, &r)
}
pub fn ejecutar(datos: &Path, b: &Borrador, operacion: impl FnOnce(&Borrador) -> Result<flow_outlook::Resultado, String>) -> Resultado {
    let intento = (|| {
        flow_outlook::validar(b)?;
        let id = identidad(b)?;
        let carpeta = datos.join("recibos_outlook");
        let _lock = bloqueo(&carpeta, &id)?;
        let ruta = carpeta.join(format!("{id}.json"));
        if let Some(r) = leer(&ruta, &id)? {
            match r.estado {
                Estado::Guardado => return Ok(Resultado::YaGuardado),
                Estado::Preparando | Estado::Incierto => {
                    return Ok(Resultado::Incierto(
                        "Un intento anterior puede existir en Outlook. Revise Borradores y resuelva su estado antes de repetirlo.".into(),
                    ))
                }
                Estado::NoGuardado => {}
            }
        }
        let mut recibo = Recibo { version: 1, id, estado: Estado::Preparando, fecha: chrono::Utc::now().to_rfc3339() };
        guardar(&ruta, &recibo)?;
        let (resultado, estado) = match operacion(b) {
            Ok(flow_outlook::Resultado::Guardado) => (Resultado::Guardado, Estado::Guardado),
            Ok(flow_outlook::Resultado::Incierto(e)) => (Resultado::Incierto(e), Estado::Incierto),
            Err(e) => (Resultado::Fallido(e), Estado::NoGuardado),
        };
        recibo.estado = estado;
        recibo.fecha = chrono::Utc::now().to_rfc3339();
        if guardar(&ruta, &recibo).is_err() {
            return Ok(Resultado::Incierto(
                "No se pudo persistir el resultado del intento. Revise Borradores en Outlook; no repita automáticamente.".into(),
            ));
        }
        Ok(resultado)
    })();
    intento.unwrap_or_else(Resultado::Bloqueado)
}
#[cfg(test)]
mod pruebas {
    use super::*;
    use flow_dominio::CC_FIJO;
    use flow_productos::correos::Tipo;
    fn b() -> Borrador {
        Borrador {
            tipo: Tipo::Informativo,
            tribunal: "Laja".into(),
            programa: "Programa ficticio".into(),
            para: "ficticio@example.invalid".into(),
            cc: CC_FIJO.into(),
            asunto: "Asunto ficticio".into(),
            cuerpo: "Texto ficticio".into(),
            adjuntos: vec![],
            filas: vec!["fila-ficticia".into()],
            omitido: false,
        }
    }
    #[test]
    fn no_repite_guardado_ni_incierto_al_volver_a_guardar() {
        let p = tempfile::tempdir().unwrap();
        let b = b();
        let n = std::cell::Cell::new(0);
        let guardar = |_: &Borrador| {
            n.set(n.get() + 1);
            Ok(flow_outlook::Resultado::Guardado)
        };
        assert!(matches!(ejecutar(p.path(), &b, guardar), Resultado::Guardado));
        assert!(matches!(ejecutar(p.path(), &b, guardar), Resultado::YaGuardado));
        assert_eq!(n.get(), 1);
        let mut otro = b.clone();
        otro.asunto.push_str(" incierto");
        assert!(matches!(
            ejecutar(p.path(), &otro, |_| {
                n.set(n.get() + 1);
                Ok(flow_outlook::Resultado::Incierto("Fallo ficticio de confirmación".into()))
            }),
            Resultado::Incierto(_)
        ));
        assert!(matches!(ejecutar(p.path(), &otro, guardar), Resultado::Incierto(_)));
        assert_eq!(n.get(), 2);
        resolver(p.path(), &otro, false).unwrap();
        assert!(matches!(ejecutar(p.path(), &otro, guardar), Resultado::Guardado));
        assert_eq!(n.get(), 3);
    }
    #[test]
    fn fallo_previo_al_guardado_permite_reintentar_y_un_recibo_roto_bloquea() {
        let p = tempfile::tempdir().unwrap();
        let b = b();
        assert!(matches!(ejecutar(p.path(), &b, |_| Err("Fallo ficticio al adjuntar".into())), Resultado::Fallido(_)));
        assert!(matches!(ejecutar(p.path(), &b, |_| Ok(flow_outlook::Resultado::Guardado)), Resultado::Guardado));
        let ruta = p.path().join("recibos_outlook").join(format!("{}.json", identidad(&b).unwrap()));
        std::fs::write(&ruta, b"incompleto").unwrap();
        assert!(matches!(ejecutar(p.path(), &b, |_| panic!("No debe crear otro borrador")), Resultado::Bloqueado(_)));
        assert_eq!(std::fs::read(ruta).unwrap(), b"incompleto");
    }
    #[test]
    fn dos_procesos_no_pueden_guardar_el_mismo_borrador_a_la_vez() {
        let p = tempfile::tempdir().unwrap();
        let b = b();
        let id = identidad(&b).unwrap();
        let _lock = bloqueo(&p.path().join("recibos_outlook"), &id).unwrap();
        assert!(matches!(ejecutar(p.path(), &b, |_| panic!("Guardado concurrente")), Resultado::Bloqueado(_)));
    }
    #[test]
    fn un_intento_interrumpido_exige_revision_manual() {
        let p = tempfile::tempdir().unwrap();
        let b = b();
        let id = identidad(&b).unwrap();
        let carpeta = p.path().join("recibos_outlook");
        std::fs::create_dir(&carpeta).unwrap();
        guardar(&carpeta.join(format!("{id}.json")), &Recibo { version: 1, id, estado: Estado::Preparando, fecha: String::new() }).unwrap();
        assert!(matches!(ejecutar(p.path(), &b, |_| panic!("Repetición tras interrupción")), Resultado::Incierto(_)));
        resolver(p.path(), &b, true).unwrap();
        assert!(matches!(ejecutar(p.path(), &b, |_| panic!("Ya comprobado")), Resultado::YaGuardado));
    }
}
