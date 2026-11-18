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

/// B3: como `nurus.bitacora_html`, el destino del formulario se compara por ruta y origen; la consulta no importa.
#[test]
fn destino_de_bitacora_admite_consulta_pero_no_otro_origen() {
    let c = casos().into_iter().find(|c| c["tipo"] == "bitacora").unwrap();
    let html = c["html"].as_str().unwrap();
    for bueno in ["/SITFAWEB/InformesDAction.do?accion=ver", "https://familia.pjud.cl/SITFAWEB/InformesDAction.do?x=1#y"] {
        let v = html.replace("action=\"/SITFAWEB/InformesDAction.do\"", &format!("action=\"{bueno}\""));
        let mut leida = leer(&c, v.as_bytes()).unwrap();
        let mut esperado = c["esperado"].clone();
        leida.as_object_mut().unwrap().remove("html_sha256");
        esperado.as_object_mut().unwrap().remove("html_sha256");
        assert_eq!(leida, esperado, "{bueno}");
    }
    for malo in [
        "https://otro.invalid/SITFAWEB/InformesDAction.do",
        "//familia.pjud.cl/SITFAWEB/InformesDAction.do",
        "/SITFAWEB/Otra.do?/SITFAWEB/InformesDAction.do",
        "http://familia.pjud.cl/SITFAWEB/InformesDAction.do",
    ] {
        let v = html.replace("action=\"/SITFAWEB/InformesDAction.do\"", &format!("action=\"{malo}\""));
        assert!(leer(&c, v.as_bytes()).is_err(), "{malo}");
    }
}

/// Plan 18-nov-2026: Escritos en la misma respuesta de Historia (`nurus/escritos_html.py`). Valores de la misma fila,
/// ubicados por encabezado; vacía comprobada, ilegible y ausente son estados distintos.
mod escritos {
    use super::*;
    use serde_json::json;

    fn historia_con(bloque: &str) -> Value {
        let c = casos().into_iter().find(|c| c["tipo"] == "historia").unwrap();
        let html = c["html"].as_str().unwrap().replace("</body>", &format!("{bloque}</body>"));
        leer_historia(html.as_bytes(), "ficticio.html", "2026-11-12T12:00:00Z").unwrap()
    }

    #[test]
    fn con_filas_en_cualquier_orden_de_columnas() {
        let h = historia_con(
            "<table><thead><tr><th>Estado</th><th>Solicitante</th><th>Doc.</th><th>Tipo Escrito</th></tr></thead><tbody>\
             <tr><td>Pendiente</td><td>Programa X</td><td>D-1</td><td>Inf. Cump FAE/FAS</td></tr>\
             <tr><td>Resuelto</td><td>Programa X</td><td>D-2</td><td>Otro escrito</td></tr></tbody></table>",
        );
        let e = &h["escritos"];
        assert_eq!(e["vacio_explicito"], false);
        let lista: Vec<(&str, &str, &str)> = e["filas"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (f["doc"].as_str().unwrap(), f["tipo"].as_str().unwrap(), f["estado"].as_str().unwrap()))
            .collect();
        assert_eq!(lista, [("D-1", "Inf. Cump FAE/FAS", "Pendiente"), ("D-2", "Otro escrito", "Resuelto")]);
        assert_eq!(e["filas"][0]["literal"]["Estado"], "Pendiente");
        // La historia de la misma respuesta no cambia.
        assert!(h["filas"].as_array().unwrap().len() >= 1);
    }

    #[test]
    fn cabecera_y_cero_filas_es_vacia_comprobada() {
        let h = historia_con("<table><thead><tr><th>Doc.</th><th>Tipo Escrito</th><th>Solicitante</th><th>Estado</th></tr></thead><tbody></tbody></table>");
        assert_eq!((h["escritos"]["vacio_explicito"].clone(), h["escritos"]["filas"].clone()), (json!(true), json!([])));
    }

    #[test]
    fn pestana_ilegible_es_error_y_ausente_no_deja_rastro() {
        let mala = historia_con("<table><thead><tr><th>Tipo Escrito</th><th>Estado</th></tr></thead><tbody><tr><td>x</td></tr></tbody></table>");
        assert_eq!(mala["escritos"]["error"], "ESCRITOS_NO_VERIFICADA");
        assert!(historia_con("")["escritos"].is_null(), "sin la pestaña no hay clave: fuente no solicitada");
    }
}

/// Estructura real observada en SITFA (capturas del usuario, 18-nov-2026) con valores ficticios, compartida con Python
/// (`tests/fixtures_integracion/`): programa = «Nombre Proyecto», banda de título, tabla LRPA vecina y fila de relleno.
mod historico_real {
    use flow_captura::lecturas::{leer_escritos_html, leer_historico};

    const HISTORIAL: &str = include_str!("../../../../tests/fixtures_integracion/historial_menores_ficticio.html");
    const ESCRITOS: &str = include_str!("../../../../tests/fixtures_integracion/escritos_ficticio.html");

    #[test]
    fn nombre_proyecto_titulo_y_lrpa() {
        let l = leer_historico(HISTORIAL.as_bytes(), "", "2026-11-18T12:00:00Z", &["11.111.111-1"]).unwrap();
        assert_eq!(l["ownership"], "VERIFICADA");
        let filas: Vec<(&str, &str, &str)> = l["filas"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (f["programa"].as_str().unwrap(), f["fecha_egreso"].as_str().unwrap(), f["causal_egreso"].as_str().unwrap()))
            .collect();
        assert_eq!(
            filas,
            [("PRM - PROGRAMA FICTICIO UNO", "--", "SIN INFORMACION"), ("DCE - PROGRAMA FICTICIO DOS", "06-02-2026", "EGRESO POR ENVÍO DE INFORME")]
        );
        assert_eq!(l["vacio_explicito"], false);
        let corte = HISTORIAL.find("<table><tr><td colspan=\"19\">").unwrap();
        let solo_lrpa = format!("{}</body></html>", &HISTORIAL[..corte]);
        assert_eq!(leer_historico(solo_lrpa.as_bytes(), "", "x", &[]).unwrap_err(), "HISTORICO_NO_VERIFICADA");
    }

    #[test]
    fn escritos_en_minuscula_y_doc_solo_con_icono() {
        let l = leer_escritos_html(ESCRITOS.as_bytes()).unwrap().unwrap();
        let f = &l["filas"][0];
        assert_eq!((f["doc"].as_str(), f["tipo"].as_str(), f["estado"].as_str()), (Some(""), Some("inf. cump programa ambulatorio"), Some("pendiente")));
    }
}
