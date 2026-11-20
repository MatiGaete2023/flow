//! Pantallazo PNG de una consulta sin registros, convertido localmente a PDF (`evidencias.screenshot_pdf`).

use base64::{engine::general_purpose::STANDARD, Engine};
use flate2::{write::ZlibEncoder, Compression};
use image::{ImageFormat, ImageReader};
use serde_json::Value;
use std::io::{Cursor, Write};

const ERROR: &str =
    "Chrome no entregó una captura PNG válida; no se guardó un PDF de esta consulta.";

fn texto_pdf(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii() && !c.is_ascii_control())
        .flat_map(|c| match c {
            '(' | ')' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect()
}

/// PDF de una página con el tamaño de la captura a 144 ppp; no recorta ni reduce a una hoja A4.
pub fn png_a_pdf(respuesta: &Value, huella_html: &str, titulo: &str) -> Result<Vec<u8>, String> {
    let convertir = || -> Option<Vec<u8>> {
        if respuesta["digest"].as_str()? != huella_html {
            return None;
        }
        let texto = respuesta["png"].as_str()?;
        if texto.len() > 40 * 1024 * 1024 {
            return None;
        }
        let png = STANDARD.decode(texto).ok()?;
        if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
            return None;
        }
        let (w, h) = ImageReader::with_format(Cursor::new(&png), ImageFormat::Png)
            .into_dimensions()
            .ok()?;
        if w < 400 || h < 200 || u64::from(w) * u64::from(h) > 30_000_000 {
            return None;
        }
        let rgb = image::load_from_memory_with_format(&png, ImageFormat::Png)
            .ok()?
            .to_rgb8();
        let mut zlib = ZlibEncoder::new(Vec::new(), Compression::default());
        zlib.write_all(rgb.as_raw()).ok()?;
        let imagen = zlib.finish().ok()?;
        let (ancho, alto) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
        let contenido = format!("q {ancho} 0 0 {alto} 0 0 cm /Im0 Do Q\n");
        let objetos = [
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {ancho} {alto}] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>").into_bytes(),
            [
                format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n", imagen.len()).into_bytes(),
                imagen,
                b"\nendstream".to_vec(),
            ]
            .concat(),
            format!("<< /Length {} >>\nstream\n{contenido}endstream", contenido.len()).into_bytes(),
            format!("<< /Title ({}) /Producer (Flow Rust) >>", texto_pdf(titulo)).into_bytes(),
        ];
        let mut pdf = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let mut offsets = vec![0];
        for (i, objeto) in objetos.iter().enumerate() {
            offsets.push(pdf.len());
            writeln!(pdf, "{} 0 obj", i + 1).ok()?;
            pdf.extend(objeto);
            pdf.extend(b"\nendobj\n");
        }
        let inicio = pdf.len();
        write!(pdf, "xref\n0 {}\n0000000000 65535 f \n", offsets.len()).ok()?;
        for offset in offsets.iter().skip(1) {
            writeln!(pdf, "{offset:010} 00000 n ").ok()?;
        }
        write!(
            pdf,
            "trailer\n<< /Size {} /Root 1 0 R /Info 6 0 R >>\nstartxref\n{inicio}\n%%EOF\n",
            offsets.len()
        )
        .ok()?;
        Some(pdf)
    };
    convertir().ok_or_else(|| ERROR.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> String {
        let mut datos = Vec::new();
        image::RgbImage::from_pixel(w, h, image::Rgb([255, 255, 255]))
            .write_to(&mut Cursor::new(&mut datos), ImageFormat::Png)
            .unwrap();
        STANDARD.encode(datos)
    }

    #[test]
    fn convierte_una_captura_valida_y_rechaza_huella_o_tamano_ajenos() {
        let respuesta = serde_json::json!({"png": png(800, 400), "digest": "abc"});
        let pdf = png_a_pdf(&respuesta, "abc", "Consulta (ficticia)").unwrap();
        assert!(pdf.starts_with(b"%PDF-") && pdf.ends_with(b"%%EOF\n"));
        assert!(png_a_pdf(&respuesta, "otra", "x").is_err());
        let chica = serde_json::json!({"png": png(100, 100), "digest": "abc"});
        assert!(png_a_pdf(&chica, "abc", "x").is_err());
    }
}
