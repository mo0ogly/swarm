//! Protocol shapes over the shared Go projection. Canonical comparison remains
//! authoritative here; field projection alone grants no validity or authority.
use crate::{Change, Error, Snapshot};
use serde_json::Value;
use swarm_wire_json::{
    self as wire,
    projection::{self as p, Kind},
};
fn zero_revision() -> Value {
    Value::String("0".into())
}
fn zero_clock() -> Value {
    Value::String("0001-01-01T00:00:00Z".into())
}
fn revision(raw: &[u8]) -> Result<Option<Value>, p::Error> {
    let s: String = wire::decode(raw)?;
    if s == "null" {
        return Ok(None);
    }
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(p::Error::Invalid("unsigned decimal string required"));
    }
    let n: u64 = s.parse().map_err(p::Error::Integer)?;
    Ok(Some(Value::String(n.to_string())))
}
fn clock(raw: &[u8]) -> Result<Option<Value>, p::Error> {
    let s: String = wire::decode(raw)?;
    if raw.contains(&b'\\') {
        return Err(p::Error::Invalid("unescaped Go timestamp required"));
    }
    crate::go_clock::parse(&s).map_err(|_| p::Error::Invalid("Go timestamp required"))?;
    Ok(Some(Value::String(s)))
}
const PRINCIPAL: p::Fields = &[
    ("principal_id", Kind::String),
    ("principal_kind", Kind::String),
    ("issuer", Kind::String),
    ("subject", Kind::String),
];
const PINS: p::Fields = &[
    ("installation_digest", Kind::String),
    ("authority_public_key", Kind::String),
    ("bootstrap_principal", Kind::Object(PRINCIPAL)),
];
const HEAD: p::Fields = &[
    (
        "revision",
        Kind::Custom {
            default: zero_revision,
            project: revision,
        },
    ),
    ("digest", Kind::String),
];
const VALUES: p::Fields = &[("mode", Kind::String), ("local_socket_path", Kind::String)];
const CHANGE: p::Fields = &[
    ("schema_version", Kind::Int64),
    ("event_id", Kind::String),
    ("expected_head", Kind::Object(HEAD)),
    ("values", Kind::Object(VALUES)),
    ("reason", Kind::String),
];
const ENTRY: p::Fields = &[
    ("change", Kind::Object(CHANGE)),
    ("actor", Kind::Object(PRINCIPAL)),
    (
        "at",
        Kind::Custom {
            default: zero_clock,
            project: clock,
        },
    ),
];
const SNAPSHOT: p::Fields = &[
    ("schema_version", Kind::Int64),
    ("pins", Kind::Object(PINS)),
    (
        "history",
        Kind::Array {
            element: &Kind::Object(ENTRY),
            nil: false,
        },
    ),
];
const INTENT: p::Fields = &[
    ("entry", Kind::Object(ENTRY)),
    ("target", Kind::Object(HEAD)),
];
const WITNESS: p::Fields = &[
    ("schema_version", Kind::Int64),
    ("pins", Kind::Object(PINS)),
    ("committed", Kind::Object(HEAD)),
    ("pending", Kind::OptionalObject(INTENT)),
];
fn decode<T: serde::de::DeserializeOwned>(raw: &[u8], shape: p::Fields) -> Result<T, Error> {
    let value = p::project(raw, shape, false).map_err(|e| match e {
        p::Error::Json(e) => Error::Frame(e),
        e => Error::Projection(e),
    })?;
    serde_json::from_value(value).map_err(Error::Frame)
}
pub(super) fn change(raw: &[u8]) -> Result<Change, Error> {
    decode(raw, CHANGE)
}
pub(super) fn snapshot(raw: &[u8]) -> Result<Snapshot, Error> {
    decode(raw, SNAPSHOT)
}
pub(crate) fn witness<T: serde::de::DeserializeOwned>(raw: &[u8]) -> Result<T, Error> {
    decode(raw, WITNESS).map_err(|e| Error::WitnessFrame(Box::new(e)))
}
