use csv::StringRecord;
use std::collections::HashMap;

mod columnas;

const MAX_CAMPO: usize = 8 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct Tabla {
    columnas: HashMap<String, usize>,
    filas: Vec<StringRecord>,
}

#[derive(Clone, Copy)]
pub struct Fila<'a> {
    columnas: &'a HashMap<String, usize>,
    valores: &'a StringRecord,
}

impl<'a> Fila<'a> {
    pub fn nueva(columnas: &'a HashMap<String, usize>, valores: &'a StringRecord) -> Self {
        Self { columnas, valores }
    }

    pub fn get(self, campo: &str) -> &'a str {
        self.campo(campo).unwrap_or("")
    }

    pub(super) fn campo(self, campo: &str) -> Option<&'a str> {
        self.columnas.get(campo).and_then(|i| self.valores.get(*i))
    }

    pub(super) fn pares(self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.columnas
            .iter()
            .map(|(k, &i)| (k.as_str(), &self.valores[i]))
    }
}

impl Tabla {
    pub(super) fn leer(nombre: &str, bytes: &[u8], esperadas: usize) -> Result<Self, String> {
        let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
        if bytes.contains(&0) || !comillas_validas(bytes) {
            return Err(format!("CSV irregular en {nombre}."));
        }
        let mut lector = csv::Reader::from_reader(bytes);
        let campos = lector
            .headers()
            .map_err(|_| format!("Encabezado CSV inválido en {nombre}."))?;
        let prefijo = columnas::requeridas(nombre);
        if campos.len() < prefijo.len()
            || !campos
                .iter()
                .take(prefijo.len())
                .eq(prefijo.iter().copied())
        {
            return Err(format!("Encabezado de tabla incompatible en {nombre}."));
        }
        let mut columnas = HashMap::new();
        for (i, campo) in campos.iter().enumerate() {
            if campo.is_empty()
                || campo.len() > MAX_CAMPO
                || columnas.insert(campo.to_owned(), i).is_some()
            {
                return Err(format!(
                    "Encabezado vacío, duplicado o demasiado largo en {nombre}."
                ));
            }
        }
        let mut filas = Vec::new();
        for fila in lector.records() {
            let fila = fila.map_err(|_| format!("Fila CSV irregular en {nombre}."))?;
            if filas.len() == esperadas || fila.iter().any(|v| v.len() > MAX_CAMPO) {
                return Err(format!(
                    "La tabla {nombre} excede sus filas declaradas o el límite de campo."
                ));
            }
            filas.push(fila);
        }
        if filas.len() != esperadas {
            return Err(format!("Cantidad de filas incorrecta en {nombre}."));
        }
        Ok(Self { columnas, filas })
    }

    pub(super) fn filas(&self) -> impl Iterator<Item = Fila<'_>> {
        self.filas.iter().map(|valores| Fila {
            columnas: &self.columnas,
            valores,
        })
    }
}

// csv::Reader es deliberadamente tolerante. El contrato Python usa strict=True:
// exige el cierre de cada campo citado y un separador después de su comilla final.
fn comillas_validas(bytes: &[u8]) -> bool {
    enum Campo {
        Inicio,
        Simple,
        Citado,
        Cerrado,
    }
    use Campo::*;
    let mut estado = Inicio;
    for &b in bytes {
        estado = match (estado, b) {
            (Inicio, b'"') | (Cerrado, b'"') => Citado,
            (Citado, b'"') => Cerrado,
            (Citado, _) => Citado,
            (_, b',' | b'\r' | b'\n') => Inicio,
            (Cerrado, _) => return false,
            (_, _) => Simple,
        }
    }
    !matches!(estado, Citado)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_encabezados_y_conserva_csv_citado() {
        let encabezados = columnas::requeridas("incidencias").join(",");
        let entrada =
            format!("{encabezados},extra\r\ncap,q,r,i,p,c,s,\"uno,\"\"dos\"\"\r\ntres\",true,x");
        let tabla = Tabla::leer("incidencias", entrada.as_bytes(), 1).unwrap();
        assert_eq!(
            tabla.filas().next().unwrap().get("message"),
            "uno,\"dos\"\r\ntres"
        );
        for invalida in [
            format!("{encabezados},message\n"),
            format!("{encabezados}\ncap,q,r,i,p,c,s,\"uno"),
            format!("{encabezados}\ncap,q,r,i,p,c,s,\"uno\"x,true"),
            format!("{encabezados}\ncap,q,r,i,p,c,s,m"),
        ] {
            assert!(Tabla::leer("incidencias", invalida.as_bytes(), 1).is_err());
        }
        assert!(Tabla::leer("incidencias", encabezados.as_bytes(), 1).is_err());
    }
}
