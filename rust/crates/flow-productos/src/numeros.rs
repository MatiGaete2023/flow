//! Números y fechas en palabras para los proyectos de resolución (`outputs.py::number_in_words`).

use chrono::{Datelike, NaiveDate};

const UNIDADES: [&str; 10] = ["cero", "uno", "dos", "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve"];
const ESPECIALES: [&str; 20] = [
    "diez", "once", "doce", "trece", "catorce", "quince", "dieciséis", "diecisiete", "dieciocho", "diecinueve",
    "veinte", "veintiuno", "veintidós", "veintitrés", "veinticuatro", "veinticinco", "veintiséis", "veintisiete",
    "veintiocho", "veintinueve",
];
const DECENAS: [&str; 7] = ["treinta", "cuarenta", "cincuenta", "sesenta", "setenta", "ochenta", "noventa"];
const CENTENAS: [&str; 9] = [
    "cien", "doscientos", "trescientos", "cuatrocientos", "quinientos", "seiscientos", "setecientos", "ochocientos",
    "novecientos",
];
const MESES: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre",
    "diciembre",
];

/// Cardinal en español de 0 a 9999.
pub fn en_palabras(n: u32) -> String {
    match n {
        0..=9 => UNIDADES[n as usize].to_string(),
        10..=29 => ESPECIALES[(n - 10) as usize].to_string(),
        30..=99 => {
            let decena = DECENAS[(n / 10 - 3) as usize];
            if n % 10 == 0 {
                decena.to_string()
            } else {
                format!("{decena} y {}", UNIDADES[(n % 10) as usize])
            }
        }
        100..=999 => {
            let resto = n % 100;
            let cabeza = if n / 100 == 1 && resto != 0 { "ciento" } else { CENTENAS[(n / 100 - 1) as usize] };
            if resto == 0 {
                cabeza.to_string()
            } else {
                format!("{cabeza} {}", en_palabras(resto))
            }
        }
        _ => {
            let miles = n / 1000;
            let resto = n % 1000;
            let cabeza = if miles == 1 { "mil".to_string() } else { format!("{} mil", en_palabras(miles)) };
            if resto == 0 {
                cabeza
            } else {
                format!("{cabeza} {}", en_palabras(resto))
            }
        }
    }
}

/// «catorce de septiembre de dos mil veintiséis».
pub fn fecha_en_palabras(fecha: NaiveDate) -> String {
    format!("{} de {} de {}", en_palabras(fecha.day()), MESES[fecha.month0() as usize], en_palabras(fecha.year() as u32))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn cardinales() {
        assert_eq!(en_palabras(21), "veintiuno");
        assert_eq!(en_palabras(31), "treinta y uno");
        assert_eq!(en_palabras(100), "cien");
        assert_eq!(en_palabras(101), "ciento uno");
        assert_eq!(en_palabras(2026), "dos mil veintiséis");
        assert_eq!(
            fecha_en_palabras(NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()),
            "catorce de septiembre de dos mil veintiséis"
        );
    }
}
