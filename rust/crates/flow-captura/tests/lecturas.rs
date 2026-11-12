use flow_captura::lecturas::{decodificar, leer_bitacora, leer_calendario, leer_historia};
use serde_json::Value;
use std::collections::BTreeMap;

fn casos() -> Vec<Value> {
    serde_json::from_str(include_str!("datos/lecturas_ficticias.json")).unwrap()
}
fn leer(c: &Value, html: &[u8]) -> Result<Value, String> {
    match c["tipo"].as_str().unwrap() {
        "bitacora" => leer_bitacora(
            html,
            &serde_json::from_value::<BTreeMap<String, String>>(c["params"].clone()).unwrap(),
            "ficticio.html",
            "2026-11-12T12:00:00Z",
        ),
        "historia" => leer_historia(html, "ficticio.html", "2026-11-12T12:00:00Z"),
        _ => leer_calendario(
            html,
            &serde_json::from_value::<Vec<String>>(c["args"].clone()).unwrap(),
            "ficticio.html",
            "2026-11-12T12:00:00Z",
        ),
    }
}

#[test]
fn ventanas_dan_lo_mismo_que_python() {
    for c in casos() {
        assert_eq!(leer(&c, c["html"].as_str().unwrap().as_bytes()).unwrap(), c["esperado"], "{}", c["tipo"]);
    }
}

#[test]
fn respuesta_cortada_login_o_estructura_ajena_nunca_es_vacia() {
    for c in casos() {
        let html = c["html"].as_str().unwrap();
        for variante in [
            html.replace("</html>", ""),
            html.replace("</body>", "<input type='PaSsWoRd'></body>"),
            html.replace("<table", "<div").replace("</table>", "</div>"),
        ] {
            assert!(leer(&c, variante.as_bytes()).is_err(), "{}", c["tipo"]);
        }
    }
}

#[test]
fn bitacora_usa_textos_completos_y_rechaza_ids_ocultos_ajenos() {
    let c = casos().remove(0);
    let html = c["html"].as_str().unwrap();
    let leida = leer(&c, html.as_bytes()).unwrap();
    assert_eq!(leida["entries"][0]["texto"], "Texto completo que la tabla oculta.");
    assert_eq!(leida["coverage"], "PARCIAL");
    for (de, a) in [
        ("name=\"ID_Ingreso\" value=\"33\"", "name=\"ID_Ingreso\" value=\"99\""),
        ("HOBS_Centro_44", "HOBS_Centro_45"),
        ("id=\"44\"", "id=\"0\""),
        (">12/08/2026 10:13<", ">ilegible<"),
    ] {
        assert!(html.contains(de));
        assert!(leer(&c, html.replace(de, a).as_bytes()).is_err());
    }
    let ajeno = html.replace("</body>", "<input type='hidden' name='HOBS_Centro_999' value='texto ficticio fuera de tabla'></body>");
    assert!(leer(&c, ajeno.as_bytes()).is_err());
}

#[test]
fn calendario_ajeno_estado_y_fecha_incoherentes_son_error() {
    let c = casos().pop().unwrap();
    let html = c["html"].as_str().unwrap();
    for (de, a) in [
        ("value=\"201\"", "value=\"202\""),
        ("PENDIENTE", "OTRO"),
        ("ENVIADO", "PENDIENTE"),
        ("28/05/2026 13:16:05", "---"),
        ("01/09/2026", "ileGible"),
    ] {
        assert!(html.contains(de));
        assert!(leer(&c, html.replace(de, a).as_bytes()).is_err());
    }
}

#[test]
fn decodifica_solo_charsets_auditados_sin_reemplazar_errores() {
    assert_eq!(decodificar(b"<meta charset=windows-1252>ni\xf1a").unwrap(), "<meta charset=windows-1252>niña");
    assert!(decodificar(b"<meta charset=utf-8>ni\xf1a").is_err());
    assert!(decodificar(b"<meta charset=utf-16>texto").is_err());
}

#[test]
fn historia_conserva_fecha_ilegible_y_advierte_paginacion_sin_leer_scripts() {
    for c in casos().into_iter().filter(|c| c["tipo"] == "historia") {
        let resultado = leer(&c, c["html"].as_str().unwrap().as_bytes()).unwrap();
        let texto = resultado.to_string();
        assert!(!texto.contains("ShowPDF"));
        assert!(!texto.contains("TOKENDOCCANARIO"));
        assert!(!texto.contains("REQCANARIO"));
    }
}

#[test]
fn ventana_de_meses_naturales_conserva_fechas_ilegibles_y_excluye_futuras() {
    let filas = vec![
        serde_json::json!({"fecha_iso":"2024-02-29"}),
        serde_json::json!({"fecha_iso":"2024-02-28"}),
        serde_json::json!({"fecha_iso":"2024-04-01"}),
        serde_json::json!({"fecha_iso":""}),
    ];
    let corte = "2024-03-31".parse().unwrap();
    let resultado = flow_captura::lecturas::separar_ventana(&filas, corte, 1).unwrap();
    assert_eq!(resultado.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1, 1, 1]);
    assert!(flow_captura::lecturas::separar_ventana(&filas, corte, 0).is_err());
    assert!(flow_captura::lecturas::separar_ventana(&filas, corte, 5).is_err());
}
