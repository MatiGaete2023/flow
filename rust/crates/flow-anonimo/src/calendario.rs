//! ICS reconstruido con propiedades cerradas. Excluye alarmas, asistentes, organizadores y parámetros libres.
use crate::{formatos, texto::Sesion};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
use regex::Regex;
use std::sync::LazyLock;

fn fecha(s: &str, dias: i64) -> Result<String, String> {
    if s.len() == 8 {
        let d = NaiveDate::parse_from_str(s, "%Y%m%d").map_err(|_| "Fecha ICS ilegible.")?;
        let d = d.checked_add_signed(Duration::days(dias)).ok_or("Fecha ICS fuera de rango.")?;
        if !(1..=9999).contains(&d.year()) {
            return Err("Fecha ICS fuera de rango.".into());
        }
        return Ok(d.format("%Y%m%d").to_string());
    }
    let (s, z) = s.strip_suffix('Z').map(|s| (s, "Z")).unwrap_or((s, ""));
    let d = NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S").map_err(|_| "Fecha y hora ICS ilegibles.")?;
    let d = d.checked_add_signed(Duration::days(dias)).ok_or("Fecha ICS fuera de rango.")?;
    if !(1..=9999).contains(&d.year()) {
        return Err("Fecha ICS fuera de rango.".into());
    }
    Ok(format!("{}{z}", d.format("%Y%m%dT%H%M%S")))
}
fn regla(s: &str, dias: i64) -> Result<String, String> {
    static NUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[+-]?\d{1,7}(?:,[+-]?\d{1,7})*$").unwrap());
    static DAY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:[+-]?\d{1,2})?(?:MO|TU|WE|TH|FR|SA|SU)(?:,(?:[+-]?\d{1,2})?(?:MO|TU|WE|TH|FR|SA|SU))*$").unwrap()
    });
    let mut out = vec![];
    for parte in s.split(';') {
        let (key, value) = parte.split_once('=').ok_or("Regla ICS no admitida.")?;
        let value = match key {
            "FREQ" if ["SECONDLY", "MINUTELY", "HOURLY", "DAILY", "WEEKLY", "MONTHLY", "YEARLY"].contains(&value) => value.into(),
            "WKST" if ["MO", "TU", "WE", "TH", "FR", "SA", "SU"].contains(&value) => value.into(),
            "BYDAY" if DAY.is_match(value) => value.into(),
            "UNTIL" => fecha(value, dias)?,
            "COUNT" | "INTERVAL" | "BYSECOND" | "BYMINUTE" | "BYHOUR" | "BYMONTHDAY" | "BYYEARDAY" | "BYWEEKNO" | "BYMONTH"
            | "BYSETPOS"
                if NUM.is_match(value) =>
            {
                value.into()
            }
            _ => return Err("Regla ICS no admitida.".into()),
        };
        out.push(format!("{key}={value}"));
    }
    Ok(out.join(";"))
}
pub fn lineas(s: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = vec![];
    for l in s.lines() {
        if l.starts_with([' ', '\t']) {
            let previo = out.last_mut().ok_or("Calendario plegado inválido.")?;
            previo.push_str(&l[1..]);
        } else {
            out.push(l.trim_end_matches('\r').to_string());
        }
    }
    if out.len() > 200_000 {
        return Err("Calendario demasiado extenso.".into());
    }
    Ok(out)
}
fn texto(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\N", "\n").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\")
}
fn escapar(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\r', "").replace('\n', "\\n").replace(',', "\\,").replace(';', "\\;")
}
pub fn semillar(sesion: &mut Sesion, lineas: &[String]) {
    for l in lineas {
        if let Some((_, value)) = l.split_once(':') {
            formatos::semillar_linea(sesion, &texto(value));
        }
    }
}
pub fn transformar(sesion: &mut Sesion, lineas: &[String], textos: &mut Vec<(String, bool)>) -> Result<Vec<u8>, String> {
    let (mut out, mut pila, mut alarmas, mut uids, mut stamps, mut serie) = (vec![], Vec::<String>::new(), 0usize, vec![], vec![], 0usize);
    let mut omitidas = false;
    for l in lineas {
        if l.trim().is_empty() {
            continue;
        }
        let (name, value) = l.split_once(':').ok_or("Propiedad ICS inválida.")?;
        let mut partes = name.split(';');
        let prop = partes.next().unwrap_or("").to_uppercase();
        if prop == "BEGIN" {
            let tipo = value.to_uppercase();
            if !["VCALENDAR", "VEVENT", "VTODO", "VTIMEZONE", "STANDARD", "DAYLIGHT", "VALARM"].contains(&tipo.as_str()) {
                return Err("Componente ICS no admitido.".into());
            }
            if tipo == "VALARM" {
                alarmas += 1;
                omitidas = true;
            }
            pila.push(tipo.clone());
            if pila.len() > 32 {
                return Err("Calendario demasiado anidado.".into());
            }
            if alarmas > 0 {
                continue;
            }
            if matches!(tipo.as_str(), "VEVENT" | "VTODO") {
                uids.push(false);
                stamps.push(false);
                serie += 1;
            }
            out.push(format!("BEGIN:{tipo}"));
            continue;
        }
        if prop == "END" {
            let tipo = pila.pop().ok_or("Estructura ICS inválida.")?;
            if tipo != value.to_uppercase() {
                return Err("Estructura ICS inválida.".into());
            }
            if tipo == "VALARM" {
                alarmas = alarmas.saturating_sub(1);
                continue;
            }
            if alarmas > 0 {
                continue;
            }
            if matches!(tipo.as_str(), "VEVENT" | "VTODO") {
                if !uids.pop().unwrap_or(false) {
                    out.push(format!("UID:{}@example.invalid", sesion.pseudo("tok:uid", &format!("sin-uid-{serie}"))));
                }
                if !stamps.pop().unwrap_or(false) {
                    out.push("DTSTAMP:19700101T000000Z".into());
                }
            }
            out.push(format!("END:{tipo}"));
            continue;
        }
        if alarmas > 0 {
            continue;
        }
        let en_zona = pila.iter().any(|s| s == "VTIMEZONE");
        let dias = if en_zona { 0 } else { sesion.opciones.desplazar_dias };
        let mut nombre = prop.clone();
        let nuevo = match prop.as_str() {
            "VERSION" => "2.0".into(),
            "PRODID" => "-//Flow//Anonimizado//ES".into(),
            "METHOD" => "PUBLISH".into(),
            "CALSCALE" => "GREGORIAN".into(),
            "DTSTART" | "DTEND" | "DTSTAMP" | "CREATED" | "LAST-MODIFIED" | "COMPLETED" | "DUE" | "RECURRENCE-ID" | "RDATE" | "EXDATE" => {
                for p in partes {
                    let (k, v) = p.split_once('=').ok_or("Parámetro de fecha ICS inválido.")?;
                    match k.to_uppercase().as_str() {
                        "TZID" => {
                            v.parse::<chrono_tz::Tz>().map_err(|_| "Zona horaria ICS no admitida.")?;
                            nombre.push_str(&format!(";TZID={v}"));
                        }
                        "VALUE" if matches!(v, "DATE" | "DATE-TIME") => nombre.push_str(&format!(";VALUE={v}")),
                        _ => return Err("Parámetro de fecha ICS no admitido.".into()),
                    }
                }
                if prop == "DTSTAMP" {
                    if let Some(s) = stamps.last_mut() {
                        *s = true;
                    }
                }
                value.split(',').map(|v| fecha(v, dias)).collect::<Result<Vec<_>, _>>()?.join(",")
            }
            "TZID" => {
                value.parse::<chrono_tz::Tz>().map_err(|_| "Zona horaria ICS no admitida.")?;
                value.into()
            }
            "TZOFFSETFROM" | "TZOFFSETTO"
                if value.len() == 5 && value.starts_with(['+', '-']) && value[1..].bytes().all(|b| b.is_ascii_digit()) =>
            {
                value.into()
            }
            "RRULE" | "EXRULE" => regla(value, dias)?,
            "UID" => {
                if let Some(u) = uids.last_mut() {
                    *u = true;
                }
                format!("{}@example.invalid", sesion.pseudo("tok:uid", value))
            }
            "SUMMARY" | "DESCRIPTION" | "LOCATION" | "COMMENT" | "X-WR-CALNAME" | "X-WR-CALDESC" | "CATEGORIES" => {
                escapar(&formatos::transformar_linea(sesion, &texto(value))?)
            }
            "SEQUENCE" if value.parse::<u32>().is_ok() => value.into(),
            "STATUS"
                if ["TENTATIVE", "CONFIRMED", "CANCELLED", "NEEDS-ACTION", "COMPLETED", "IN-PROCESS", "DRAFT", "FINAL"]
                    .contains(&value) =>
            {
                value.into()
            }
            "CLASS" if ["PUBLIC", "PRIVATE", "CONFIDENTIAL"].contains(&value) => value.into(),
            "TRANSP" if ["TRANSPARENT", "OPAQUE"].contains(&value) => value.into(),
            "DURATION" => {
                static D: LazyLock<Regex> =
                    LazyLock::new(|| Regex::new(r"^[+-]?P(?:\d+W|(?:\d+D)?(?:T(?:\d+H)?(?:\d+M)?(?:\d+S)?)?)$").unwrap());
                if !D.is_match(value) || !value.chars().any(|c| c.is_ascii_digit()) {
                    return Err("Duración ICS no admitida.".into());
                }
                value.into()
            }
            _ => {
                omitidas = true;
                continue;
            }
        };
        textos.push((nuevo.clone(), false));
        out.push(format!("{nombre}:{nuevo}"));
    }
    if !pila.is_empty()
        || out.first().map(String::as_str) != Some("BEGIN:VCALENDAR")
        || out.last().map(String::as_str) != Some("END:VCALENDAR")
    {
        return Err("Calendario incompleto.".into());
    }
    if omitidas {
        sesion.avisos.insert("El calendario excluye alarmas, asistentes, organizadores y propiedades no admitidas.".into());
    }
    // Plegado por octetos, sin cortar caracteres UTF-8, según RFC 5545.
    let mut resultado = String::new();
    for l in out {
        let mut usados = 0;
        for c in l.chars() {
            if usados + c.len_utf8() > 74 {
                resultado.push_str("\r\n ");
                usados = 1;
            }
            resultado.push(c);
            usados += c.len_utf8();
        }
        resultado.push_str("\r\n");
    }
    Ok(resultado.into_bytes())
}
