//! Go data projection in an arena: no recursive tree traversal or raw JSON loss.
use serde::{Deserialize, Serialize, Serializer};
use serde_json::value::RawValue;
use std::collections::BTreeMap;

pub(crate) type Fields = &'static [(&'static str, Kind)];
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    String,
    Int64,
    Float64,
    Bool,
    Raw,
    Object(fn() -> Fields),
    Pointer(&'static Kind),
    Array(&'static Kind),
    Map(&'static Kind),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct GoFloat(pub f64);
impl Serialize for GoFloat {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let raw =
            float_text(self.0).ok_or_else(|| serde::ser::Error::custom("nonfinite Go float"))?;
        let raw = RawValue::from_string(raw).map_err(serde::ser::Error::custom)?;
        raw.serialize(s)
    }
}
fn float_text(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    let text = value.to_string();
    let absolute = value.abs();
    if absolute == 0.0 || (1e-6..1e21).contains(&absolute) {
        return Some(text);
    }
    let (sign, magnitude) = if let Some(s) = text.strip_prefix('-') {
        ("-", s)
    } else {
        ("", text.as_str())
    };
    let point = magnitude.find('.').unwrap_or(magnitude.len());
    let digits = magnitude.replace('.', "");
    let first = digits.find(|c| c != '0')?;
    let significant = digits[first..].trim_end_matches('0');
    let exponent = point as i64 - first as i64 - 1;
    let fraction = if significant.len() > 1 {
        format!(".{}", &significant[1..])
    } else {
        String::new()
    };
    Some(format!(
        "{sign}{}{fraction}e{exponent:+}",
        &significant[..1]
    ))
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct GoRaw(pub Box<RawValue>);
/// Encode an engine-produced opaque observation in its persisted wire spelling.
/// Go-compatible HTML/separator escaping and whitespace compaction must happen
/// before an exact Agent/receipt readback comparison. This preserves duplicate
/// keys, number lexemes and the existing strict GoRaw equality; it validates no
/// identity, freshness or authority. Encoding/decoding failures preserve WorkError
/// causes. No IO or mutation occurs; callers still owe their persistence guards.
pub(crate) fn persisted_raw(value: &impl Serialize) -> Result<GoRaw, crate::WorkError> {
    let bytes = crate::encode_model(value)?;
    swarm_wire_json::decode::<Box<RawValue>>(&compact(&bytes))
        .map(GoRaw)
        .map_err(crate::WorkError::Json)
}
impl PartialEq for GoRaw {
    fn eq(&self, other: &Self) -> bool {
        self.0.get() == other.0.get()
    }
}
impl Serialize for GoRaw {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        swarm_wire_json::decode::<serde::de::IgnoredAny>(self.0.get().as_bytes())
            .map_err(|_| serde::ser::Error::custom("invalid Go raw JSON"))?;
        let raw = RawValue::from_string(
            String::from_utf8(compact(self.0.get().as_bytes()))
                .map_err(serde::ser::Error::custom)?,
        )
        .map_err(serde::ser::Error::custom)?;
        raw.serialize(s)
    }
}
fn compact(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let (mut inside, mut escape) = (false, false);
    for &b in raw {
        if inside {
            out.push(b);
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                inside = false;
            }
        } else if b == b'"' {
            inside = true;
            out.push(b);
        } else if !b.is_ascii_whitespace() {
            out.push(b);
        }
    }
    out
}

pub enum Error {
    Json(serde_json::Error),
    Integer(swarm_wire_json::projection::Error),
    Invalid(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(e) => write!(
                f,
                "model JSON ({:?}, line {}, column {})",
                e.classify(),
                e.line(),
                e.column()
            ),
            Self::Integer(e) => write!(f, "{e}"),
            Self::Invalid(s) => write!(f, "model JSON: {s}"),
        }
    }
}
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            Self::Integer(e) => Some(e),
            Self::Invalid(_) => None,
        }
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

enum Node {
    Scalar(Vec<u8>),
    Object(BTreeMap<&'static str, usize>),
    Pointer(Option<usize>),
    Array {
        backing: Vec<usize>,
        len: usize,
        nil: bool,
    },
    Map(Option<BTreeMap<String, usize>>),
}
#[derive(Default)]
struct Arena(Vec<Node>);
impl Arena {
    fn allocate(&mut self, kind: Kind) -> usize {
        let value = match kind {
            Kind::String => Node::Scalar(b"\"\"".to_vec()),
            Kind::Int64 | Kind::Float64 => Node::Scalar(b"0".to_vec()),
            Kind::Bool => Node::Scalar(b"false".to_vec()),
            Kind::Raw => Node::Scalar(b"null".to_vec()),
            Kind::Object(fields) => {
                let mut out = BTreeMap::new();
                for &(key, kind) in fields() {
                    out.insert(key, self.allocate(kind));
                }
                Node::Object(out)
            }
            Kind::Pointer(_) => Node::Pointer(None),
            Kind::Map(_) => Node::Map(None),
            Kind::Array(_) => Node::Array {
                backing: Vec::new(),
                len: 0,
                nil: true,
            },
        };
        let index = self.0.len();
        self.0.push(value);
        index
    }
    fn apply(
        &mut self,
        raw: &[u8],
        shape: Fields,
        root: usize,
        boundaries: &Boundaries,
        deny_unknown_fields: bool,
    ) -> Result<(), Error> {
        let mut pending = vec![(raw, Kind::Object(|| &[]), root)];
        // Root shape is supplied separately; child object functions remain lazy,
        // allowing recursive Go pointer types without recursive constants.
        let mut first = true;
        while let Some((raw, kind, destination)) = pending.pop() {
            let root_shape = if first {
                first = false;
                Some(shape)
            } else {
                None
            };
            let raw = raw.trim_ascii();
            if raw == b"null" {
                match kind {
                    Kind::Pointer(_) => self.0[destination] = Node::Pointer(None),
                    Kind::Map(_) => self.0[destination] = Node::Map(None),
                    Kind::Array(_) => {
                        self.0[destination] = Node::Array {
                            backing: Vec::new(),
                            len: 0,
                            nil: true,
                        }
                    }
                    Kind::Raw => self.0[destination] = Node::Scalar(b"null".to_vec()),
                    _ => {}
                }
                continue;
            }
            match kind {
                Kind::String => {
                    self.0[destination] = Node::Scalar(serde_json::to_vec(
                        &swarm_wire_json::decode::<String>(raw)?,
                    )?)
                }
                Kind::Int64 => {
                    self.0[destination] = Node::Scalar(
                        swarm_wire_json::projection::signed_integer(raw)
                            .map_err(Error::Integer)?
                            .to_string()
                            .into_bytes(),
                    )
                }
                Kind::Float64 => {
                    self.0[destination] = Node::Scalar(
                        float_text(serde_json::from_slice::<f64>(raw)?)
                            .ok_or(Error::Invalid("finite float required"))?
                            .into_bytes(),
                    )
                }
                Kind::Bool => {
                    self.0[destination] =
                        Node::Scalar(serde_json::to_vec(&swarm_wire_json::decode::<bool>(raw)?)?)
                }
                Kind::Raw => self.0[destination] = Node::Scalar(compact(raw)),
                Kind::Object(fields) => {
                    let shape = root_shape.unwrap_or_else(fields);
                    let values = object(raw, boundaries)?;
                    let mut operations = Vec::new();
                    for (key, value) in values {
                        let field = shape
                            .iter()
                            .find(|(k, _)| *k == key)
                            .or_else(|| shape.iter().find(|(k, _)| fold(k) == fold(&key)));
                        let Some(&(name, kind)) = field else {
                            if deny_unknown_fields {
                                return Err(Error::Invalid("unknown structured field"));
                            }
                            continue;
                        };
                        let Node::Object(ref current) = self.0[destination] else {
                            return Err(Error::Invalid("object destination"));
                        };
                        let child = *current.get(name).ok_or(Error::Invalid("object field"))?;
                        operations.push((value, kind, child));
                    }
                    pending.extend(operations.into_iter().rev());
                }
                Kind::Pointer(element) => {
                    let Node::Pointer(mut value) = self.0[destination] else {
                        return Err(Error::Invalid("pointer destination"));
                    };
                    let child = *value.get_or_insert_with(|| self.allocate(*element));
                    self.0[destination] = Node::Pointer(value);
                    pending.push((raw, *element, child));
                }
                Kind::Array(element) => {
                    let values = array(raw, boundaries)?;
                    let mut previous =
                        std::mem::replace(&mut self.0[destination], Node::Pointer(None));
                    let Node::Array {
                        ref mut backing,
                        ref mut len,
                        ref mut nil,
                    } = previous
                    else {
                        return Err(Error::Invalid("slice destination"));
                    };
                    if values.is_empty() {
                        backing.clear();
                    }
                    let mut operations = Vec::new();
                    for (index, value) in values.iter().enumerate() {
                        if index >= backing.len() {
                            backing.push(self.allocate(*element));
                        }
                        operations.push((*value, *element, backing[index]));
                    }
                    *len = values.len();
                    *nil = false;
                    self.0[destination] = previous;
                    pending.extend(operations.into_iter().rev());
                }
                Kind::Map(element) => {
                    let values = object(raw, boundaries)?;
                    let mut previous =
                        std::mem::replace(&mut self.0[destination], Node::Pointer(None));
                    let Node::Map(ref mut map) = previous else {
                        return Err(Error::Invalid("map destination"));
                    };
                    let map = map.get_or_insert_with(Default::default);
                    let mut operations = Vec::new();
                    for (key, value) in values {
                        let child = self.allocate(*element);
                        map.insert(key, child);
                        operations.push((value, *element, child));
                    }
                    self.0[destination] = previous;
                    pending.extend(operations.into_iter().rev());
                }
            }
        }
        Ok(())
    }
    fn json(&self, root: usize) -> Result<Vec<u8>, Error> {
        enum Token {
            Node(usize),
            Bytes(Vec<u8>),
            Literal(&'static [u8]),
        }
        let mut output = Vec::new();
        let mut pending = vec![Token::Node(root)];
        while let Some(token) = pending.pop() {
            match token {
                Token::Bytes(v) => output.extend(v),
                Token::Literal(v) => output.extend_from_slice(v),
                Token::Node(index) => match &self.0[index] {
                    Node::Scalar(v) => output.extend_from_slice(v),
                    Node::Pointer(None) | Node::Map(None) | Node::Array { nil: true, .. } => {
                        output.extend_from_slice(b"null")
                    }
                    Node::Pointer(Some(v)) => pending.push(Token::Node(*v)),
                    Node::Array { backing, len, .. } => {
                        pending.push(Token::Literal(b"]"));
                        for (i, v) in backing[..*len].iter().enumerate().rev() {
                            pending.push(Token::Node(*v));
                            if i > 0 {
                                pending.push(Token::Literal(b","));
                            }
                        }
                        pending.push(Token::Literal(b"["));
                    }
                    Node::Object(fields) => {
                        pending.push(Token::Literal(b"}"));
                        for (i, (key, v)) in fields.iter().enumerate().rev() {
                            pending.push(Token::Node(*v));
                            pending.push(Token::Literal(b":"));
                            pending.push(Token::Bytes(serde_json::to_vec(key)?));
                            if i > 0 {
                                pending.push(Token::Literal(b","));
                            }
                        }
                        pending.push(Token::Literal(b"{"));
                    }
                    Node::Map(Some(fields)) => {
                        pending.push(Token::Literal(b"}"));
                        for (i, (key, v)) in fields.iter().enumerate().rev() {
                            pending.push(Token::Node(*v));
                            pending.push(Token::Literal(b":"));
                            pending.push(Token::Bytes(serde_json::to_vec(key)?));
                            if i > 0 {
                                pending.push(Token::Literal(b","));
                            }
                        }
                        pending.push(Token::Literal(b"{"));
                    }
                },
            }
        }
        Ok(output)
    }
}
pub(crate) fn fold(v: &str) -> String {
    v.chars()
        .map(|c| match c {
            'ſ' => 'S',
            'K' => 'K',
            c if c.is_ascii_lowercase() => c.to_ascii_uppercase(),
            c => c,
        })
        .collect()
}
// The whole document is syntax/depth validated before these boundary scanners.
// They preserve original raw spans rather than decoding opaque JSON into Value.
pub(super) struct Boundaries(std::collections::HashMap<usize, usize>);
impl Boundaries {
    pub(super) fn new(raw: &[u8]) -> Result<Self, Error> {
        let base = raw.as_ptr() as usize;
        let mut ends = std::collections::HashMap::new();
        let (mut quoted, mut escaped) = (None, false);
        let mut containers = Vec::new();
        for (i, &b) in raw.iter().enumerate() {
            if let Some(start) = quoted {
                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    ends.insert(base + start, base + i + 1);
                    quoted = None;
                }
            } else {
                match b {
                    b'"' => quoted = Some(i),
                    b'{' | b'[' => containers.push(i),
                    b'}' | b']' => {
                        let start = containers
                            .pop()
                            .ok_or(Error::Invalid("unpaired JSON boundary"))?;
                        ends.insert(base + start, base + i + 1);
                    }
                    _ => {}
                }
            }
        }
        if quoted.is_some() || !containers.is_empty() {
            return Err(Error::Invalid("unfinished JSON boundary"));
        }
        Ok(Self(ends))
    }
    fn end(&self, raw: &[u8], start: usize) -> Result<usize, Error> {
        let first = *raw.get(start).ok_or(Error::Invalid("missing JSON value"))?;
        if matches!(first, b'"' | b'{' | b'[') {
            let origin = raw.as_ptr() as usize;
            let end = *self
                .0
                .get(&(origin + start))
                .ok_or(Error::Invalid("missing JSON boundary"))?;
            return end
                .checked_sub(origin)
                .filter(|&n| n <= raw.len())
                .ok_or(Error::Invalid("JSON boundary outside value"));
        }
        Ok(raw
            .iter()
            .enumerate()
            .skip(start)
            .find_map(|(i, b)| {
                if matches!(b, b',' | b'}' | b']' | b':' | b' ' | b'\n' | b'\t' | b'\r') {
                    Some(i)
                } else {
                    None
                }
            })
            .unwrap_or(raw.len()))
    }
}
fn whitespace(raw: &[u8], mut position: usize) -> usize {
    while raw.get(position).is_some_and(u8::is_ascii_whitespace) {
        position += 1;
    }
    position
}
pub(super) fn object<'a>(
    raw: &'a [u8],
    boundaries: &Boundaries,
) -> Result<Vec<(String, &'a [u8])>, Error> {
    if raw.first() != Some(&b'{') {
        return Err(Error::Invalid("object required"));
    }
    let mut position = whitespace(raw, 1);
    let mut out = Vec::new();
    while raw.get(position) != Some(&b'}') {
        let key_end = boundaries.end(raw, position)?;
        let key = swarm_wire_json::decode(&raw[position..key_end])?;
        position = whitespace(raw, key_end);
        if raw.get(position) != Some(&b':') {
            return Err(Error::Invalid("object colon"));
        }
        position = whitespace(raw, position + 1);
        let value_end = boundaries.end(raw, position)?;
        out.push((key, &raw[position..value_end]));
        position = whitespace(raw, value_end);
        if raw.get(position) == Some(&b',') {
            position = whitespace(raw, position + 1);
        } else {
            break;
        }
    }
    Ok(out)
}
pub(super) fn array<'a>(raw: &'a [u8], boundaries: &Boundaries) -> Result<Vec<&'a [u8]>, Error> {
    if raw.first() != Some(&b'[') {
        return Err(Error::Invalid("array required"));
    }
    let mut position = whitespace(raw, 1);
    let mut out = Vec::new();
    while raw.get(position) != Some(&b']') {
        let value_end = boundaries.end(raw, position)?;
        out.push(&raw[position..value_end]);
        position = whitespace(raw, value_end);
        if raw.get(position) == Some(&b',') {
            position = whitespace(raw, position + 1);
        } else {
            break;
        }
    }
    Ok(out)
}
/// Project the real Go data graph, optionally rejecting every unknown structured field.
/// Opaque JSON and maps retain their existing semantics; this is schema checking,
/// not permission validation. Errors retain the original parse/type cause.
pub(crate) fn project_mode(
    raw: &[u8],
    shape: Fields,
    deny_unknown_fields: bool,
) -> Result<Vec<u8>, Error> {
    project_mode_from(raw, shape, None, deny_unknown_fields)
}
/// Project optional producer-initialized state before applying incoming Go JSON.
/// This preserves omitted/scalar-null fields, explicit slice null and duplicate
/// merges in the same arena. Initial bytes are a caller's DTO representation,
/// never operator defaults or an error fallback. Both documents are validated;
/// failures preserve their original parse/type cause without partial output.
pub(crate) fn project_mode_from(
    raw: &[u8],
    shape: Fields,
    initial: Option<&[u8]>,
    deny_unknown_fields: bool,
) -> Result<Vec<u8>, Error> {
    swarm_wire_json::decode::<serde::de::IgnoredAny>(raw)?;
    let mut arena = Arena::default();
    // Construct root fields from their real shape; nested shapes are functions.
    let mut fields = BTreeMap::new();
    for &(name, kind) in shape {
        fields.insert(name, arena.allocate(kind));
    }
    let root = arena.0.len();
    arena.0.push(Node::Object(fields));
    if let Some(initial) = initial {
        swarm_wire_json::decode::<serde::de::IgnoredAny>(initial)?;
        let boundaries = Boundaries::new(initial)?;
        arena.apply(initial, shape, root, &boundaries, deny_unknown_fields)?;
    }
    let boundaries = Boundaries::new(raw)?;
    arena.apply(raw, shape, root, &boundaries, deny_unknown_fields)?;
    arena.json(root)
}

/// Ordinary Go model decoding ignores unknown fields, preserving its historical contract.
pub(crate) fn project(raw: &[u8], shape: Fields) -> Result<Vec<u8>, Error> {
    project_mode(raw, shape, false)
}

#[cfg(test)]
mod persisted_observation_tests {
    use super::*;
    /// Persisted opaque observations retain duplicate keys/number lexemes and
    /// exact readback even when Go wire escaping changes literal source strings.
    /// This is pure codec evidence, not a process, permission or DB proof.
    #[test]
    fn persisted_observation_escaping_and_lexemes() {
        for text in ["plain", "<b>&</b>\u{2028}\u{2029}"] {
            let value = serde_json::json!({"capture":text});
            let raw = persisted_raw(&value).unwrap();
            let agent = crate::Agent { native_execution_result:Some(raw), ..Default::default() };
            let bytes = crate::encode_model(&agent).unwrap();
            let decoded = crate::decode_agent(&bytes).unwrap();
            assert!(agent == decoded);
        }
        let original = RawValue::from_string(r#"{ "n":9007199254740993,"n":1.00,"text":"<>&" }"#.into()).unwrap();
        let frozen = persisted_raw(&original).unwrap();
        assert_eq!(frozen.0.get(),r#"{"n":9007199254740993,"n":1.00,"text":"\u003c\u003e\u0026"}"#);
        assert!(persisted_raw(&frozen).unwrap() == frozen);
        // Strict equality itself remains lexical; no semantic/numeric fallback.
        let integer = GoRaw(RawValue::from_string("1".into()).unwrap());
        let decimal = GoRaw(RawValue::from_string("1.0".into()).unwrap());
        assert!(integer != decimal);
    }
}
