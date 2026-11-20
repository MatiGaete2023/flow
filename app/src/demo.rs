//! Libro de ejemplo con datos ficticios (para probar la aplicación sin datos reales).

use chrono::{Duration, NaiveDate};

fn f(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

/// Escribe un libro ficticio de la etapa pedida en la carpeta temporal y devuelve su ruta.
pub fn libro(mode: crate::motor::Mode, hoy: NaiveDate) -> Result<std::path::PathBuf, String> {
    use crate::motor::Mode;
    let d = |n: i64| f(hoy + Duration::days(n));
    let nac = |anios: i32, dias: i64| f(NaiveDate::from_ymd_opt(hoy.format("%Y").to_string().parse::<i32>().unwrap() - anios, 6, 1).unwrap() + Duration::days(dias));
    let programas = [
        ("PRM Ficticio Laja", "Juzgado de Letras y Garantía de Laja"),
        ("DCE Ficticio Biobío", "Juzgado de Letras y Garantía de Mulchén"),
        ("FAE Ficticio Costa", "Juzgado de Familia de Tomé"),
        ("RFA Ficticia Sur", "Juzgado de Letras y Garantía de Laja"),
        ("PIE Ficticio Norte", "Juzgado de Familia de Tomé"),
        ("OPD Ficticia Centro", "Juzgado de Letras y Garantía de Mulchén"),
    ];
    let nombres = [
        "Sofía (hija) Ejemplo Uno", "Matías Ejemplo Dos", "Valentina Ejemplo Tres", "Benjamín Ejemplo Cuatro",
        "Isidora Ejemplo Cinco", "Tomás Ejemplo Seis", "Agustina Ejemplo Siete", "Vicente Ejemplo Ocho",
        "Florencia Ejemplo Nueve", "Martín Ejemplo Diez", "Emilia Ejemplo Once", "Lucas Ejemplo Doce",
        "Josefa Ejemplo Trece", "Joaquín Ejemplo Catorce", "Antonia Ejemplo Quince", "Gaspar Ejemplo Dieciséis",
        "Maite Ejemplo Diecisiete", "Diego Ejemplo Dieciocho",
    ];
    let mut headers: Vec<&str> = vec!["RIT", "TRIBUNAL", "NOMBRE", "RUT", "DERIVACION", "FEC. NACIMIENTO", "CURADOR", "PROXS. AUDS.", "FEC. OIDO"];
    match mode {
        Mode::Espera => headers.extend(["T ESPERA", "FEC. RESOLUCION", "DURACION"]),
        Mode::Cumplimiento => headers.extend(["DIAS DE CUMPLIMIENTO", "DIAS PARA EGRESAR", "FEC.INGRESO EFECTIVO", "FEC.EGRESO PROYECTADO", "FEC.ACT.F.INDIVIDUAL", "FEC.ACT.F.FAE"]),
        Mode::Informes => headers.extend(["FECHA VENCIMIENTO", "FEC.INGRESO EFECTIVO"]),
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for (i, nombre) in nombres.iter().enumerate() {
        let (prog, trib) = programas[i % programas.len()];
        let i64_ = i as i64;
        let rut = (11_000_000 + i * 111_111).to_string();
        let candidato = char::from(b'0' + (i % 9) as u8);
        let dv = if crate::anon::digito_verificador(&rut) == candidato { '9' } else { candidato };
        let mut r = vec![
            format!("{}-{}-2099", if i % 3 == 0 { "P" } else { "X" }, 100 + i / 2),
            trib.to_string(),
            nombre.to_string(),
            format!("{rut}-{dv}"),
            prog.to_string(),
            if i == 4 { nac(18, -20) } else if i == 7 { nac(18, 30) } else { nac(6 + (i as i32 % 10), i64_ * 9) },
            if i % 4 == 0 { "NO POSEE".into() } else if i % 5 == 0 { "(11111111-0)CURADORA FICTICIA - Institución: CAJ#(22222222-0)OTRO FICTICIO - Institución: OPD".into() } else { "(11111111-0)CURADORA FICTICIA".into() },
            if i % 3 == 1 { format!("{}, {}", d(12 + i64_), d(40 + i64_)) } else if i % 3 == 2 { d(20 + i64_) } else { String::new() },
            if i % 6 == 2 { d(-10) } else { String::new() },
        ];
        match mode {
            Mode::Espera => {
                r.push(((i * 13) % 90).to_string());
                r.push(if i % 5 == 1 { d(-8) } else { d(-60 - i64_) });
                r.push("6 meses".into());
            }
            Mode::Cumplimiento => {
                let dc = [12, 200, 95, 400, 30, 150][i % 6];
                let dpe = [100, -5, 20, 0, 300, 60][i % 6];
                r.push(dc.to_string());
                r.push(dpe.to_string());
                r.push(d(-dc));
                r.push(d(dpe));
                r.push(if i % 2 == 0 { d(-200) } else { d(-10) });
                r.push(String::new());
            }
            Mode::Informes => {
                r.push(d([-40, 12, -5, 45, 3, -15][i % 6]));
                r.push(d(-120));
            }
        }
        rows.push(r);
    }
    let mut hojas = vec![(mode.label(), headers.iter().map(|s| s.to_string()).collect(), rows)];
    if mode == Mode::Cumplimiento {
        // Hoja de cruce: próximo informe de algunos registros.
        let h: Vec<String> = ["RIT", "RUT MENOR", "NOMBRE MENOR", "TRIBUNAL", "NOMBRE CENTRO", "FECHA VENCIMIENTO"].iter().map(|s| s.to_string()).collect();
        let mut cr = Vec::new();
        for (i, r) in hojas[0].2.iter().enumerate().filter(|(i, _)| i % 2 == 0) {
            cr.push(vec![r[0].clone(), r[3].clone(), r[2].clone(), r[1].clone(), r[4].clone(), d(15 + i as i64 * 3)]);
        }
        hojas.push(("Hoja2", h, cr));
    }
    let path = std::env::temp_dir().join(format!("Ejemplo_ficticio_{}.xlsx", mode.label()));
    let refs: Vec<(&str, Vec<String>, Vec<Vec<String>>)> = hojas.into_iter().collect();
    crate::productos::escribir_xlsx(&refs, &path)?;
    Ok(path)
}
