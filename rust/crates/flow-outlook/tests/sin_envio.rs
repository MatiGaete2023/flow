//! Regla que no se negocia: Flow no despacha correos. El código de este crate no puede nombrar la operación de
//! Outlook que lo hace, ni siquiera en un comentario.

use flow_dominio::CC_FIJO;
use flow_productos::correos::{Borrador, Tipo};

#[test]
fn el_codigo_no_nombra_el_despacho() {
    let prohibida = ["Se", "nd"].concat();
    let fuente = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut revisados = 0;
    for entrada in std::fs::read_dir(fuente).unwrap() {
        let ruta = entrada.unwrap().path();
        let texto = std::fs::read_to_string(&ruta).unwrap();
        assert!(
            !texto.to_lowercase().contains(&prohibida.to_lowercase()),
            "{} nombra la operación prohibida",
            ruta.display()
        );
        revisados += 1;
    }
    assert!(revisados >= 2);
}

fn borrador(cc: &str) -> Borrador {
    Borrador {
        tipo: Tipo::ProgramaEspera,
        tribunal: "Jgdo. L. y G. de Laja".into(),
        programa: "PRM Ejemplo".into(),
        para: "programa@example.invalid".into(),
        cc: cc.into(),
        asunto: "Lista de espera".into(),
        cuerpo: "Buen día".into(),
        adjuntos: Vec::new(),
        filas: Vec::new(),
        omitido: false,
    }
}

#[test]
fn sin_cc_fijo_no_se_guarda() {
    assert!(flow_outlook::validar(&borrador(CC_FIJO)).is_ok());
    assert!(flow_outlook::validar(&borrador("otra@example.invalid")).is_err());
    assert!(flow_outlook::guardar_borrador(&borrador("")).is_err());
}
