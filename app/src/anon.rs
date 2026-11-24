//! Anonimizador local: copias con identidades ficticias. Nunca escribe sobre el original ni usa la red.
//! Delega en `flow-anonimo`: tablas (.xlsx, .xls, HTML/XML de SITFA, .csv), texto, calendario, Word, ZIP y paquetes de
//! captura. Cada copia se verifica antes de publicarse.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modo {
    Estricto,
    Conservador,
}

#[derive(Clone, Debug)]
pub struct Opciones {
    pub modo: Modo,
    pub semilla: String,
    pub desplazar_dias: i64,
    pub generalizar_nacimiento: bool,
}

impl Default for Opciones {
    fn default() -> Self {
        Opciones { modo: Modo::Estricto, semilla: String::new(), desplazar_dias: 0, generalizar_nacimiento: false }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Informe {
    pub creados: Vec<PathBuf>,
    pub errores: Vec<String>,
    pub advertencias: Vec<String>,
}

pub fn digito_verificador(numero: &str) -> char {
    let mut s = 0u32;
    let mut m = 2;
    for c in numero.chars().rev().filter(|c| c.is_ascii_digit()) {
        s += c.to_digit(10).unwrap() * m;
        m = if m == 7 { 2 } else { m + 1 };
    }
    match 11 - (s % 11) {
        11 => '0',
        10 => 'K',
        d => char::from_digit(d, 10).unwrap(),
    }
}

/// Anonimiza con el motor de `flow-anonimo` (tablas, texto, ICS, Word, ZIP y paquetes de captura; cada copia se verifica
/// antes de publicarse y nada se publica si falla la revisión). Las copias quedan en una carpeta nueva dentro de `dir`.
pub fn ejecutar(entradas: &[PathBuf], dir: &Path, o: &Opciones) -> Informe {
    let mut inf = Informe::default();
    if let Err(e) = std::fs::create_dir_all(dir) {
        inf.errores.push(format!("No se pudo crear la carpeta de salida: {e}"));
        return inf;
    }
    let opciones = flow_anonimo::Opciones {
        modo: if o.modo == Modo::Estricto { flow_anonimo::Modo::Estricto } else { flow_anonimo::Modo::Conservador },
        desplazar_dias: o.desplazar_dias,
        generalizar_nacimiento: o.generalizar_nacimiento,
        semilla: o.semilla.clone(),
        rechazar_no_admitidos: false,
    };
    let cancelar = std::sync::atomic::AtomicBool::new(false);
    match flow_anonimo::lote::ejecutar(entradas, dir, opciones, &cancelar, |_| {}) {
        Ok(r) => {
            inf.creados = r.informe.archivos.iter().map(|n| r.carpeta.join(n)).collect();
            inf.errores = r.informe.rechazados.iter().map(|x| format!("{}: {}", x.entrada, x.motivo)).collect();
            inf.advertencias = r.informe.avisos.clone();
            inf.advertencias.push(r.informe.revision_manual.clone());
        }
        Err(e) => inf.errores.push(e),
    }
    inf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carpeta(nombre: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("flow-anon-app-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn digito_verificador_conocido() {
        assert_eq!(digito_verificador("11111111"), '1');
        assert_eq!(digito_verificador("12345678"), '5');
    }

    #[test]
    fn texto_zip_y_paquete_salen_sin_identidades_y_el_original_queda_intacto() {
        let d = carpeta("lote");
        let txt = d.join("nota.txt");
        std::fs::write(&txt, "Se conversa con Identcanario Apelcanario, RUT 11.111.111-1.").unwrap();
        let zip = d.join("entrada.zip");
        let bytes = flow_anonimo::documentos::zip(&[("datos.json".into(), br#"{"Nombre":"Identcanario Apelcanario"}"#.to_vec())]).unwrap();
        std::fs::write(&zip, &bytes).unwrap();
        let paquete = d.join("captura.zip");
        std::fs::write(&paquete, include_bytes!("../../rust/crates/flow-trabajo/tests/datos/paquete_cronologia.zip")).unwrap();
        let originales: Vec<Vec<u8>> = [&txt, &zip, &paquete].iter().map(|p| std::fs::read(p).unwrap()).collect();

        let o = Opciones { modo: Modo::Conservador, semilla: "FICTICIA".into(), ..Default::default() };
        let inf = ejecutar(&[txt.clone(), zip.clone(), paquete.clone()], &d.join("salida"), &o);
        assert!(inf.errores.is_empty(), "{:?}", inf.errores);
        assert_eq!(inf.creados.len(), 3, "{:?}", inf.creados);
        for c in &inf.creados {
            assert!(c.is_file(), "{c:?}");
            let crudo = std::fs::read(c).unwrap();
            let t = String::from_utf8_lossy(&crudo);
            assert!(!t.contains("Identcanario") && !t.contains("Apelcanario") && !t.contains("11.111.111-1"), "{c:?}");
        }
        for (p, antes) in [&txt, &zip, &paquete].iter().zip(originales) {
            assert_eq!(std::fs::read(p).unwrap(), antes, "el original no debe cambiar");
        }
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn captura_alterada_no_publica_nada() {
        let d = carpeta("alterada");
        let mut entradas = flow_anonimo::formatos::validar_zip(include_bytes!("../../rust/crates/flow-trabajo/tests/datos/paquete_cronologia.zip")).unwrap();
        let i = entradas.iter().position(|(n, _)| n == "data/registros.csv").unwrap();
        entradas[i].1.push(b' ');
        let mala = d.join("mala.zip");
        std::fs::write(&mala, flow_anonimo::documentos::zip(&entradas).unwrap()).unwrap();
        let inf = ejecutar(&[mala], &d.join("salida"), &Opciones::default());
        assert!(inf.creados.is_empty(), "{:?}", inf.creados);
        assert!(!inf.errores.is_empty());
        let _ = std::fs::remove_dir_all(d);
    }
}
