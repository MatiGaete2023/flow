//! Oráculo del validador Python y regresiones con respuestas ficticias alteradas. Sin navegador ni red.

use flow_captura::sitfa::{siguiente_consulta, validar_respuesta, Pagina, Pares, Perfil, Respuesta, Seleccion, VerificadorDescargas};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Caso {
    nombre: String,
    seleccion: Seleccion,
    catalogo: Value,
    pares: Pares,
    html: String,
    registros: Vec<(String, String, usize)>,
    actual: Option<usize>,
    total: Option<usize>,
    url: Option<String>,
    excel: Option<String>,
    html2: Option<String>,
    excel2: Option<String>,
}

fn casos() -> Vec<Caso> {
    serde_json::from_str(include_str!("datos/sitfa_ficticio.json")).unwrap()
}

fn caso(nombre: &str) -> Caso {
    casos().into_iter().find(|c| c.nombre == nombre).unwrap()
}

fn perfil(c: &Caso) -> Perfil {
    Perfil::desde_catalogo(&c.seleccion, &c.catalogo).unwrap()
}

fn pagina(c: &Caso) -> Pagina {
    let Respuesta::ConRegistros(p) = validar_respuesta(c.html.as_bytes(), &c.pares, &perfil(c), None).unwrap() else {
        panic!("{} debía tener registros", c.nombre)
    };
    p
}

fn rechaza(c: &Caso, html: &str) {
    assert!(validar_respuesta(html.as_bytes(), &c.pares, &perfil(c), None).is_err(), "{}", c.nombre);
}

#[test]
fn cuatro_pantallas_y_cuatro_pestanas_coinciden_con_python() {
    for c in casos() {
        let respuesta = validar_respuesta(c.html.as_bytes(), &c.pares, &perfil(&c), None).unwrap();
        if c.registros.is_empty() {
            assert_eq!(respuesta, Respuesta::SinRegistros, "{}", c.nombre);
        } else {
            let Respuesta::ConRegistros(p) = respuesta else { panic!("{}", c.nombre) };
            assert_eq!(Some(p.actual), c.actual);
            assert_eq!(Some(p.total), c.total);
            assert_eq!(Some(&p.excel_url), c.url.as_ref());
            assert_eq!(p.registros, c.registros.iter().map(|(r, n, q)| ((r.clone(), n.clone()), *q)).collect());
            let descarga =
                VerificadorDescargas::default().verificar(c.excel.as_ref().unwrap().as_bytes().to_vec(), &p, &perfil(&c)).unwrap();
            assert_eq!(descarga.registros, p.registros);
        }
    }
}

#[test]
fn segunda_pagina_conserva_filtros_y_verifica_excel() {
    for c in casos().into_iter().filter(|c| c.html2.is_some()) {
        let perfil = perfil(&c);
        let p = pagina(&c);
        let pares = siguiente_consulta(&c.pares, &perfil, &p, 2).unwrap();
        let filtros = |ps: &Pares| ps.iter().filter(|(k, _)| !k.starts_with("NUM_")).cloned().collect::<Pares>();
        assert_eq!(filtros(&pares), filtros(&c.pares));
        let Respuesta::ConRegistros(p2) = validar_respuesta(c.html2.as_ref().unwrap().as_bytes(), &pares, &perfil, Some((2, &p))).unwrap()
        else {
            panic!()
        };
        assert_eq!(p2.actual, 2);
        let mut verificador = VerificadorDescargas::default();
        verificador.verificar(c.excel.as_ref().unwrap().as_bytes().to_vec(), &p, &perfil).unwrap();
        verificador.verificar(c.excel2.as_ref().unwrap().as_bytes().to_vec(), &p2, &perfil).unwrap();
        assert!(siguiente_consulta(&pares, &perfil, &p, 3).is_err());
    }
}

#[test]
fn rechaza_escritura_campos_desconocidos_repetidos_y_catalogo_ajeno() {
    for c in casos() {
        let p = perfil(&c);
        for campo in ["irAccion", "csrf", "cookie", "escribir_rus", "COD_NoAuditado"] {
            let mut pares = c.pares.clone();
            pares.push((campo.into(), "Guardar".into()));
            assert!(p.validar_consulta(&pares).is_err());
        }
        let mut pares = c.pares.clone();
        pares.iter_mut().find(|(k, _)| k == "irAccion").unwrap().1 = "Guardar".into();
        assert!(p.validar_consulta(&pares).is_err());
        let mut s = c.seleccion.clone();
        s.tribunal = "999".into();
        assert!(Perfil::desde_catalogo(&s, &c.catalogo).is_err());
        s.screen = "registro_rus".into();
        assert!(Perfil::desde_catalogo(&s, &c.catalogo).is_err());
    }
}

#[test]
fn rechaza_login_destino_ajeno_metodo_y_formularios_repetidos() {
    let c = caso("seguimiento/Espera");
    for alterada in [
        c.html.replace("<body>", "<body><input type='PaSsWoRd'>"),
        c.html.replace("/SITFAWEB/InformesDAction.do", "https://otro.invalid/guardar"),
        c.html.replace("method=\"post\"", "method=\"get\""),
        c.html.replace("</body>", "<form name='InformesPpalForm'></form></body>"),
        c.html.replace("InformesPpalForm", "OtroForm"),
    ] {
        rechaza(&c, &alterada);
    }
}

#[test]
fn rechaza_otro_tribunal_modalidad_pestana_o_filtro() {
    let c = caso("seguimiento/Cumplimiento");
    for (antes, despues) in [
        ("value=\"901\"", "value=\"902\""),
        ("name=\"TIP_Consulta\"><option value=\"2\"", "name=\"TIP_Consulta\"><option value=\"3\""),
        ("name=\"COD_Lengueta\" value=\"\"", "name=\"COD_Lengueta\" value=\"tdEspera\""),
        ("name=\"TIP_Informe\"><option value=\"4\"", "name=\"TIP_Informe\"><option value=\"5\""),
    ] {
        assert!(c.html.contains(antes));
        rechaza(&c, &c.html.replace(antes, despues));
    }
}

#[test]
fn rechaza_campos_sensibles_y_ambiguos_sin_exponer_valores() {
    let c = caso("seguimiento/Espera");
    for campo in ["csrfOculto", "TOKEN", "nonce", "COD_Lengueta"] {
        let alterada = c.html.replace("</form>", &format!("<input name='{campo}' value='VALOR_PRIVADO_FICTICIO'></form>"));
        let error = validar_respuesta(alterada.as_bytes(), &c.pares, &perfil(&c), None).unwrap_err();
        assert!(!error.contains("VALOR_PRIVADO_FICTICIO"));
    }
    rechaza(&c, &c.html.replace("</form>", "<input name='TIP_Consulta' type='radio' value='3'></form>"));
}

#[test]
fn radio_auditado_y_seleccion_ambigua() {
    let c = caso("seguimiento/Espera");
    let original = "<select name=\"TIP_Consulta\"><option value=\"2\" selected>2</option></select>";
    assert!(c.html.contains(original));
    let radios = "<input type='radio' name='TIP_Consulta' value='1'><input type='radio' name='TIP_Consulta' value='2' checked>";
    let html = c.html.replace(original, radios);
    assert!(validar_respuesta(html.as_bytes(), &c.pares, &perfil(&c), None).is_ok());
    rechaza(&c, &html.replace("value='1'", "value='1' checked"));
    // Igual que el Descargador: con varias opciones marcadas vale la primera.
    let dos = c.html.replace("selected>2", "selected>2</option><option value='3' selected>3");
    assert!(validar_respuesta(dos.as_bytes(), &c.pares, &perfil(&c), None).is_ok());
    rechaza(&c, &c.html.replace("<option value=\"2\" selected>2", "<option value='3' selected>3</option><option value=\"2\" selected>2"));
}

/// Estructuras que SITFA usa en listados grandes y que el Descargador acepta: título en `th`, cabecera en `thead`,
/// paginación en `tfoot` y una tabla dentro de una celda. Antes Flow detenía el lote con
/// «No se pudieron identificar todos los registros de la consulta.» o «tabla anidada no auditada».
#[test]
fn listado_con_titulo_pie_y_tabla_anidada_se_cuenta_como_el_descargador() {
    let c = caso("seguimiento/Espera");
    let esperado = pagina(&c).registros;
    let cabecera = "<table id=\"tablaEspera\"><tr><td>RIT</td><td>Nombre</td></tr>";
    assert!(c.html.contains(cabecera));
    for reemplazo in [
        "<table id=\"tablaEspera\"><tr><th colspan='2'>Listado de causas</th></tr><tr><td>RIT</td><td>Nombre</td></tr>",
        "<table id=\"tablaEspera\"><thead><tr><td colspan='2'>Página 1 de 2</td></tr><tr><td>RIT</td><td>Nombre</td></tr></thead>",
        "<table id=\"tablaEspera\"><tfoot><tr><td>Página</td><td><a>1</a> <a>2</a></td></tr></tfoot><tr><td>RIT</td><td>Nombre</td></tr>",
    ] {
        let html = c.html.replace(cabecera, reemplazo);
        let Respuesta::ConRegistros(p) = validar_respuesta(html.as_bytes(), &c.pares, &perfil(&c), None).expect(reemplazo) else {
            panic!("{reemplazo}")
        };
        assert_eq!(p.registros, esperado, "{reemplazo}");
    }
    // Tabla dentro de la celda de un registro (p. ej. íconos de acciones): su texto queda en la celda y no suma filas.
    let anidada = c.html.replacen("<td>X-10-2099</td>", "<td>X-10-2099<table><tr><td></td></tr></table></td>", 1);
    let Respuesta::ConRegistros(p) = validar_respuesta(anidada.as_bytes(), &c.pares, &perfil(&c), None).unwrap() else { panic!() };
    assert_eq!(p.registros, esperado);
    // Una fila de datos desconocida en el cuerpo sigue deteniendo la consulta: nunca se omite en silencio.
    rechaza(&c, &c.html.replace(cabecera, &format!("{cabecera}<tr><td>Fila extraña</td><td>sin RIT</td></tr>")));
}

#[test]
fn selector_dinamico_conocido_se_lee_sin_ejecutar_js() {
    let c = caso("seguimiento/Cumplimiento");
    let js = include_str!("datos/selector_vencimiento.js").replace("valorSeleccionado = '0'", "valorSeleccionado = '4'");
    let html = c
        .html
        .replace("<script>", &format!("<script>{js}\n"))
        .replace("name=\"TIP_Informe\"><option value=\"4\"", "name=\"TIP_Informe\"><option value=\"1\"");
    assert!(validar_respuesta(html.as_bytes(), &c.pares, &perfil(&c), None).is_ok());
    rechaza(&c, &html.replace("valorSeleccionado = '4'", "valorSeleccionado = '5'"));
    rechaza(&c, &html.replace("valorSeleccionado = '4'", "valorSeleccionado = obtenerValor()"));
    rechaza(&c, &html.replace("select.innerHTML = '';", "select.innerHTML = ''; cambiarFiltro();"));
    rechaza(&c, &html.replace("<script>", &format!("<script>{js}\n")));
}

#[test]
fn paginacion_fuera_de_rango_incoherente_o_cambiada() {
    let c = caso("seguimiento/Espera");
    for (campo, valor) in
        [("NUM_PaginaEspera", "0"), ("NUM_PaginaEspera", "3"), ("NUM_TotalEspera", "501"), ("NUM_TotalEspera", "no-es-numero")]
    {
        let original = format!("name=\"{campo}\" value=\"{}\"", if campo == "NUM_TotalEspera" { "2" } else { "1" });
        rechaza(&c, &c.html.replace(&original, &format!("name=\"{campo}\" value=\"{valor}\"")));
    }
    let p = pagina(&c);
    let perfil = perfil(&c);
    let pares = siguiente_consulta(&c.pares, &perfil, &p, 2).unwrap();
    for html in [
        c.html.clone(),
        c.html2.clone().unwrap().replace("NUM_TotalEspera\" value=\"2", "NUM_TotalEspera\" value=\"3"),
        c.html2.unwrap().replace("listdemo.xls", "otra.xls"),
    ] {
        assert!(validar_respuesta(html.as_bytes(), &pares, &perfil, Some((2, &p))).is_err());
    }
}

#[test]
fn no_confunde_otra_pestana_con_consulta_vacia() {
    let c = caso("seguimiento/Espera/vacia");
    let otra = c.html.replace(
        "id=\"tablaCumplimiento\"><tr><td>RIT</td><td>Nombre</td></tr>",
        "id=\"tablaCumplimiento\"><tr><td>RIT</td><td>Nombre</td></tr><tr><td>X-1-2099</td><td>Persona ficticia</td></tr>",
    );
    rechaza(&c, &otra);
    rechaza(&c, &c.html.replace("NUM_TotalEspera\" value=\"1", "NUM_TotalEspera\" value=\"2"));
    let primera = pagina(&caso("seguimiento/Espera"));
    assert!(validar_respuesta(c.html.as_bytes(), &c.pares, &perfil(&c), Some((2, &primera))).is_err());
}

/// Prueba del usuario 13-nov: «Lote detenido: No se encontró una única exportación XLS auditada.» El Descargador
/// Python toma el primer `window.open` literal de `ShowExcel`, con cualquier número de argumentos.
#[test]
fn exportacion_con_opciones_de_ventana_o_varias_aperturas_como_el_descargador() {
    let c = caso("seguimiento/Espera");
    let url = c.url.clone().unwrap();
    let original = format!("window.open(\"{url}\");");
    assert!(c.html.contains(&original));
    for nuevo in [
        format!("window.open(\"{url}\", \"Excel\", \"width=800,height=600,scrollbars=yes\");"),
        format!("window.open( '{url}' ,'_blank');"),
        format!("var w = window.open(\"{url}\",\"\",\"menubar=yes\"); if (!w) {{ window.open(\"{url}\"); }}"),
    ] {
        let html = c.html.replace(&original, &nuevo);
        let Respuesta::ConRegistros(p) = validar_respuesta(html.as_bytes(), &c.pares, &perfil(&c), None).expect(&nuevo) else { panic!() };
        assert_eq!(p.excel_url, url);
    }
}

#[test]
fn rechaza_urls_no_permitidas_y_expresiones() {
    let c = caso("seguimiento/Espera");
    for url in [
        "http://familia.pjud.cl/sitfa/reportes/a.xls",
        "https://otro.invalid/sitfa/reportes/a.xls",
        "https://familia.pjud.cl:443/sitfa/reportes/a.xls",
        "https://familia.pjud.cl/sitfa/reportes/../a.xls",
        "https://familia.pjud.cl/sitfa/reportes/a.xls?x=1",
        "https://familia.pjud.cl/sitfa/reportes/a.xls#x",
        "https://familia.pjud.cl/sitfa/reportes/a%2fb.xls",
    ] {
        rechaza(&c, &c.html.replace(c.url.as_ref().unwrap(), url));
    }
    rechaza(&c, &c.html.replace("listdemo.xls\");", "listdemo.xls\" + obtener());"));
    let calendario = caso("calendario_informes/");
    rechaza(&calendario, &calendario.html.replace("InfoPorVencer.xls", "MedPorVencer.xls"));
}

#[test]
fn registros_ambiguos_incompletos_y_tabla_duplicada() {
    let c = caso("seguimiento/Espera");
    for html in [
        c.html.replace("</form>", "<table id='tablaEspera'></table></form>"),
        c.html.replace("tablaEspera", "tablaDesconocida"),
        c.html.replace("<td>Nombre</td>", "<td>Nombre Centro</td>"),
        c.html.replace("<td>Nombre</td>", "<td>Nombre</td><td>Nombre menor</td>"),
        c.html.replacen("<td>X-10-2099</td>", "<td>ilegible</td>", 1),
    ] {
        rechaza(&c, &html);
    }
}

#[test]
fn cotejo_conserva_multiplicidad_y_no_confunde_mismo_nna_con_misma_fila() {
    let c = caso("seguimiento/Espera");
    let mut p = pagina(&c);
    let perfil = perfil(&c);
    let excel = c.excel.as_ref().unwrap();
    let mut verificador = VerificadorDescargas::default();
    verificador.verificar(excel.as_bytes().to_vec(), &p, &perfil).unwrap();
    assert!(verificador.verificar(excel.as_bytes().to_vec(), &p, &perfil).is_err());
    let equivalente = excel.replace("<td>", "<td class='otra'>");
    assert!(verificador.verificar(equivalente.into_bytes(), &p, &perfil).is_err());
    *p.registros.values_mut().next().unwrap() += 1;
    assert!(VerificadorDescargas::default().verificar(excel.as_bytes().to_vec(), &p, &perfil).is_err());
    let otra_medida =
        excel.replace("Nombre</td>", "Nombre</td><td>Medida</td>").replace("</tr></table>", "<td>Medida ficticia</td></tr></table>");
    let p = pagina(&c);
    verificador.verificar(otra_medida.into_bytes(), &p, &perfil).unwrap();
}

#[test]
fn excel_ajeno_activo_o_ilegible_no_se_admite() {
    let c = caso("seguimiento/Espera");
    for excel in [
        "".to_string(),
        "ilegible".into(),
        c.html.clone(),
        c.excel.as_ref().unwrap().replace("X-10-2099", "X-99-2099"),
        c.excel.as_ref().unwrap().replace("<table>", "<script>activo()</script><table>"),
    ] {
        assert!(VerificadorDescargas::default().verificar(excel.into_bytes(), &pagina(&c), &perfil(&c)).is_err());
    }
}

#[test]
fn calendario_exige_fecha_del_mes_y_ano_y_nunca_inventa_una_vacia() {
    for nombre in ["calendario_informes/", "calendario_medidas/"] {
        let c = caso(nombre);
        for fecha in ["15/09/2026", "15/10/2027", "", "31/02/2026"] {
            let datos = c.excel.as_ref().unwrap().replace("15/10/2026", fecha);
            assert!(VerificadorDescargas::default().verificar(datos.into_bytes(), &pagina(&c), &perfil(&c)).is_err());
        }
        let datos = c.excel.as_ref().unwrap().replace("Fecha Vencimiento", "Fecha sin acreditar");
        assert!(VerificadorDescargas::default().verificar(datos.into_bytes(), &pagina(&c), &perfil(&c)).is_err());
    }
}

#[test]
fn calendario_rechaza_literales_incompletos_dinamicos_y_tablas_ajenas() {
    let c = caso("calendario_informes/");
    for html in [
        c.html.replace("Nombresxdias[dia]", "OtroNombre[dia]"),
        c.html.replace("Ritxdias[dia] = \"C-10-2099\";", "Ritxdias[dia] = obtener();"),
        c.html.replace("TablaCalendarioDia", "OtraTabla"),
        c.html.replace("Nombresxdias[dia] =", "Nombresxdias[dia] = secreto +"),
    ] {
        rechaza(&c, &html);
    }
}

#[test]
fn datos_no_utf8_y_entidades_se_comparan_con_nombre_completo() {
    let c = caso("seguimiento/Espera");
    let html = c.html.replace("Persona ficticia", "Ni&ntilde;a Ficticia &Aacute;lvarez");
    let Respuesta::ConRegistros(p) = validar_respuesta(html.as_bytes(), &c.pares, &perfil(&c), None).unwrap() else { panic!() };
    assert!(p.registros.keys().all(|(_, nombre)| nombre.starts_with("NINA FICTICIA ALVAREZ")));
    let excel = c.excel.as_ref().unwrap().replace("Persona ficticia", "Niña Ficticia Álvarez");
    let bytes = excel.chars().map(|c| c as u8).collect();
    VerificadorDescargas::default().verificar(bytes, &p, &perfil(&c)).unwrap();
}
