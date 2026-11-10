//! Compact original JSON for signed nested fields; never parse through f64.
use serde::{
    de::{Error as _, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::value::RawValue;
use std::{collections::BTreeMap, fmt};

/// Valid JSON retaining property order and numeric lexemes. Whitespace outside
/// strings is removed, as Go json.Marshal does for RawMessage. No authority,
/// canonical field order, credential validation or signature is established.
#[derive(Clone)]
pub(crate) struct CompactJson(Box<RawValue>);
impl CompactJson {
    /// Validate syntax and remove only insignificant whitespace; reject malformed
    /// UTF8/JSON without lossy replacement. No IO or floating-point conversion.
    pub fn new(raw: &str) -> Result<Self, serde_json::Error> {
        let _: Box<RawValue> = serde_json::from_str(raw)?;
        let (mut quoted, mut escaped) = (false, false);
        let mut bytes = Vec::with_capacity(raw.len());
        for byte in raw.bytes() {
            if quoted {
                bytes.push(byte);
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    quoted = false;
                }
            } else if byte == b'"' {
                quoted = true;
                bytes.push(byte);
            } else if !matches!(byte, b' ' | b'\t' | b'\r' | b'\n') {
                bytes.push(byte);
            }
        }
        let text = String::from_utf8(bytes).map_err(serde::de::Error::custom)?;
        RawValue::from_string(text).map(Self)
    }
    /// Borrow exact compact JSON; callers still owe schema and signature checks.
    pub fn raw(&self) -> &str {
        self.0.get()
    }
}
impl Serialize for CompactJson {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for CompactJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        Self::new(raw.get()).map_err(serde::de::Error::custom)
    }
}

/// Retain top-level values outside the named core without changing numeric
/// lexemes or property order. Caller owns schema validation and authorization.
pub(crate) fn extensions(
    raw: &[u8],
    known: &[&str],
) -> Result<std::collections::BTreeMap<String, CompactJson>, serde_json::Error> {
    let Fields(map) = swarm_wire_json::decode(raw)?;
    map.into_iter()
        .filter(|(name, _)| !known.contains(&name.as_str()))
        .map(|(name, v)| Ok((name, CompactJson::new(v.get())?)))
        .collect()
}

/// Raw object fields with unique names; shared by retained state and proposals.
/// Duplicate names refuse before authority selection, without rewriting values.
pub(super) struct Fields(pub BTreeMap<String, Box<RawValue>>);
impl<'de> Deserialize<'de> for Fields {
    /// Retain each field's raw representation and refuse duplicate policy keys.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Fields;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("policy object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Fields, A::Error> {
                let mut m = BTreeMap::new();
                while let Some((k, v)) = a.next_entry::<String, Box<RawValue>>()? {
                    if m.insert(k, v).is_some() {
                        return Err(A::Error::custom("duplicate policy field"));
                    }
                }
                Ok(Fields(m))
            }
        }
        d.deserialize_map(V)
    }
}
