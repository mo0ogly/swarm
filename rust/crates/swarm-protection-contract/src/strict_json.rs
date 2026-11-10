//! Duplicate-aware v1 decoding; separate from historical Go-compatible codecs.
use crate::Error;
use serde::{
    de::{self, DeserializeOwned, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{Map, Number, Value};
use std::{collections::BTreeSet, fmt};

struct Checked(Value);
struct JsonVisitor;

impl<'de> Deserialize<'de> for Checked {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        input.deserialize_any(JsonVisitor)
    }
}

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Checked;
    fn expecting(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("unambiguous PAEA JSON without floating point numbers")
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Checked, E> {
        Ok(Checked(Value::Bool(value)))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Checked, E> {
        Ok(Checked(Value::Number(Number::from(value))))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Checked, E> {
        Ok(Checked(Value::Number(Number::from(value))))
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Checked, E> {
        Err(E::custom("protection-floating-number-forbidden"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Checked, E> {
        Ok(Checked(Value::String(value.into())))
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Checked, E> {
        Ok(Checked(Value::String(value)))
    }
    fn visit_none<E: de::Error>(self) -> Result<Checked, E> {
        Ok(Checked(Value::Null))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Checked, E> {
        Ok(Checked(Value::Null))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut input: A) -> Result<Checked, A::Error> {
        let mut values = Vec::new();
        while let Some(Checked(value)) = input.next_element()? {
            values.push(value);
        }
        Ok(Checked(Value::Array(values)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut input: A) -> Result<Checked, A::Error> {
        let mut values = Map::new();
        let mut seen = BTreeSet::new();
        while let Some(key) = input.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom("protection-duplicate-field"));
            }
            let Checked(value) = input.next_value()?;
            values.insert(key, value);
        }
        Ok(Checked(Value::Object(values)))
    }
}

/// Parse a single whole document, preserving the original cause on any failure.
/// This validates duplicate keys before conversion to typed structs; it performs
/// no permission checks. serde_json's parser nesting protection stays enabled.
pub(crate) fn decode<T: DeserializeOwned>(raw: &[u8]) -> Result<T, Error> {
    let mut input = serde_json::Deserializer::from_slice(raw);
    let Checked(value) = Checked::deserialize(&mut input).map_err(Error::Json)?;
    input.end().map_err(Error::Json)?;
    serde_json::from_value(value).map_err(Error::Json)
}
