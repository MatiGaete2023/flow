use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{Map, Value};
use std::fmt;

struct SinDuplicados(Value);

impl<'de> Deserialize<'de> for SinDuplicados {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(Json)
    }
}

struct Json;

impl<'de> Visitor<'de> for Json {
    type Value = SinDuplicados;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("un valor JSON")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(SinDuplicados(v.into()))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(SinDuplicados(v.into()))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(SinDuplicados(v.into()))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(SinDuplicados(v.into()))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(SinDuplicados(v.into()))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(SinDuplicados(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(SinDuplicados(v)) = seq.next_element()? {
            values.push(v);
        }
        Ok(SinDuplicados(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some((key, SinDuplicados(value))) = map.next_entry()? {
            if values.insert(key, value).is_some() {
                return Err(de::Error::custom("clave JSON duplicada"));
            }
        }
        Ok(SinDuplicados(Value::Object(values)))
    }
}

pub(super) fn objeto(texto: &str) -> Result<Map<String, Value>, &'static str> {
    match serde_json::from_str::<SinDuplicados>(texto) {
        Ok(SinDuplicados(Value::Object(obj))) => Ok(obj),
        _ => Err("Se esperaba un objeto JSON válido, con claves únicas."),
    }
}
