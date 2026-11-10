//! Go struct decoding before a protocol's own validation/canonical comparison.
//! Callers supply trusted field shapes; this grants no business authorization.
use serde::{
    de::{Error as _, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{value::RawValue, Value};
use std::fmt;
pub type Fields = &'static [(&'static str, Kind)];
pub type CustomProject = fn(&[u8]) -> Result<Option<Value>, Error>;
#[derive(Clone, Copy)]
pub enum Kind {
    String,
    Int64,
    Bool,
    Object(Fields),
    OptionalObject(Fields),
    Array {
        element: &'static Kind,
        nil: bool,
    },
    Map(&'static Kind),
    Custom {
        default: fn() -> Value,
        project: CustomProject,
    },
}
pub enum Error {
    Json(serde_json::Error),
    Integer(std::num::ParseIntError),
    Invalid(&'static str),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(e) => write!(
                f,
                "JSON projection ({:?}, line {}, column {})",
                e.classify(),
                e.line(),
                e.column()
            ),
            Self::Integer(_) => write!(f, "JSON projection: signed 64-bit integer required"),
            Self::Invalid(reason) => write!(f, "JSON projection: {reason}"),
        }
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProjectionError")
            .field("message", &self.to_string())
            .finish()
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
/// Parse an integer token, not a floating-point value. Go accepts -0 for signed
/// integers but rejects -0.0 and -0e0; Serde's ordinary i64 path conflates these.
pub fn signed_integer(raw: &[u8]) -> Result<i64, Error> {
    let raw: Box<RawValue> = crate::decode(raw)?;
    if raw.get() == "-0" {
        return Ok(0);
    }
    // Preserve Serde's original type/range cause, including safe access by an
    // authorized diagnostic caller. Never rewrite opaque RawValue bytes.
    crate::decode(raw.get().as_bytes()).map_err(Error::Json)
}

// Values retain private slice backing state until the entire object is decoded.
// Go can reuse truncated elements on a later duplicate field; JSON output alone
// cannot represent that state. Empty arrays and null explicitly discard it.
enum Node {
    Scalar(Value),
    Object(std::collections::BTreeMap<String, Node>),
    Pointer(Option<Box<Node>>),
    Array {
        emit_nil: bool,
        is_nil: bool,
        backing: Vec<Node>,
        len: usize,
    },
    Map(Option<std::collections::BTreeMap<String, Node>>),
}
impl Node {
    fn new(kind: Kind) -> Self {
        match kind {
            Kind::String => Self::Scalar(Value::String(String::new())),
            Kind::Int64 => Self::Scalar(Value::from(0)),
            Kind::Bool => Self::Scalar(Value::Bool(false)),
            Kind::Object(shape) => Self::object(shape),
            Kind::OptionalObject(_) => Self::Pointer(None),
            Kind::Map(_) => Self::Map(None),
            Kind::Array { nil, .. } => Self::Array {
                emit_nil: nil,
                is_nil: true,
                backing: Vec::new(),
                len: 0,
            },
            Kind::Custom { default, .. } => Self::Scalar(default()),
        }
    }
    fn object(shape: Fields) -> Self {
        Self::Object(
            shape
                .iter()
                .map(|(name, kind)| ((*name).into(), Self::new(*kind)))
                .collect(),
        )
    }
    fn value(self) -> Value {
        match self {
            Self::Scalar(v) => v,
            Self::Object(fields) => {
                Value::Object(fields.into_iter().map(|(k, v)| (k, v.value())).collect())
            }
            Self::Pointer(v) => v.map_or(Value::Null, |v| v.value()),
            Self::Map(v) => v.map_or(Value::Null, |v| {
                Value::Object(v.into_iter().map(|(k, v)| (k, v.value())).collect())
            }),
            Self::Array {
                emit_nil: true,
                is_nil: true,
                ..
            } => Value::Null,
            Self::Array { backing, len, .. } => {
                Value::Array(backing.into_iter().take(len).map(Self::value).collect())
            }
        }
    }
}
struct Object(Vec<(String, Box<RawValue>)>);
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Object;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "Go object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Object, A::Error> {
                let mut fields = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    fields.push(pair)
                }
                Ok(Object(fields))
            }
        }
        d.deserialize_map(V)
    }
}
fn fold(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'ſ' => 'S',
            'K' => 'K',
            c if c.is_ascii_lowercase() => c.to_ascii_uppercase(),
            c => c,
        })
        .collect()
}
fn object(raw: &[u8], shape: Fields, current: &mut Node, strict: bool) -> Result<(), Error> {
    if raw == b"null" {
        return Ok(());
    }
    let fields: Object = crate::decode(raw)?;
    let Node::Object(current) = current else {
        return Err(Error::Invalid("object destination required"));
    };
    for (key, raw) in fields.0 {
        let field = shape
            .iter()
            .find(|(k, _)| *k == key)
            .or_else(|| shape.iter().find(|(k, _)| fold(k) == fold(&key)));
        let Some((name, kind)) = field else {
            if strict {
                return Err(Error::Json(serde_json::Error::custom(format!(
                    "unknown field {key:?}"
                ))));
            }
            continue;
        };
        merge(
            raw.get().as_bytes(),
            *kind,
            current
                .get_mut(*name)
                .ok_or(Error::Invalid("field destination required"))?,
            strict,
        )?;
    }
    Ok(())
}
fn merge(raw: &[u8], kind: Kind, current: &mut Node, strict: bool) -> Result<(), Error> {
    if raw == b"null" {
        if matches!(
            kind,
            Kind::OptionalObject(_) | Kind::Array { .. } | Kind::Map(_)
        ) {
            *current = Node::new(kind);
        }
        return Ok(());
    }
    match kind {
        Kind::String => *current = Node::Scalar(Value::String(crate::decode(raw)?)),
        Kind::Int64 => *current = Node::Scalar(Value::from(signed_integer(raw)?)),
        Kind::Bool => *current = Node::Scalar(Value::Bool(crate::decode(raw)?)),
        Kind::Object(shape) => object(raw, shape, current, strict)?,
        Kind::OptionalObject(shape) => {
            let Node::Pointer(pointer) = current else {
                return Err(Error::Invalid("pointer destination required"));
            };
            if pointer.is_none() {
                *pointer = Some(Box::new(Node::object(shape)));
            }
            object(
                raw,
                shape,
                pointer
                    .as_mut()
                    .ok_or(Error::Invalid("pointer destination required"))?,
                strict,
            )?;
        }
        Kind::Array { element, .. } => {
            let values: Vec<Box<RawValue>> = crate::decode(raw)?;
            let Node::Array {
                is_nil,
                backing,
                len,
                ..
            } = current
            else {
                return Err(Error::Invalid("slice destination required"));
            };
            if values.is_empty() {
                backing.clear();
            }
            for (index, raw) in values.iter().enumerate() {
                if index >= backing.len() {
                    backing.push(Node::new(*element));
                }
                merge(raw.get().as_bytes(), *element, &mut backing[index], strict)?;
            }
            *len = values.len();
            *is_nil = false;
        }
        Kind::Map(element) => {
            let values: Object = crate::decode(raw)?;
            let Node::Map(map) = current else {
                return Err(Error::Invalid("map destination required"));
            };
            let map = map.get_or_insert_with(Default::default);
            for (key, raw) in values.0 {
                let mut v = Node::new(*element);
                merge(raw.get().as_bytes(), *element, &mut v, strict)?;
                map.insert(key, v);
            }
        }
        Kind::Custom { project, .. } => {
            if let Some(v) = project(raw)? {
                *current = Node::Scalar(v);
            }
        }
    }
    Ok(())
}
pub fn project(raw: &[u8], shape: Fields, strict: bool) -> Result<Value, Error> {
    let raw: Box<RawValue> = crate::decode(raw)?;
    let mut current = Node::object(shape);
    object(raw.get().as_bytes(), shape, &mut current, strict)?;
    Ok(current.value())
}
