//! Compact original JSON for signed nested fields; never parse through f64.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

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
    let map: std::collections::BTreeMap<String, Box<RawValue>> = swarm_wire_json::decode(raw)?;
    map.into_iter()
        .filter(|(name, _)| !known.contains(&name.as_str()))
        .map(|(name, v)| Ok((name, CompactJson::new(v.get())?)))
        .collect()
}
