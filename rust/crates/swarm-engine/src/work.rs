use super::{model_wire, models};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

pub enum WorkError {
    Identity,
    Utf8(std::str::Utf8Error),
    Json(serde_json::Error),
    Projection(model_wire::Error),
    Incoherent,
}
impl WorkError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Identity => "row-identity",
            Self::Utf8(_) => "utf8",
            Self::Incoherent => "row-incoherent",
            _ => "format",
        }
    }
}
impl fmt::Display for WorkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity => write!(f, "invalid work row identity"),
            Self::Utf8(_) => write!(f, "model document: UTF-8 required"),
            Self::Incoherent => write!(f, "work row: schema, identity or revision mismatch"),
            Self::Json(e) => write!(
                f,
                "model JSON ({:?}, line {}, column {})",
                e.classify(),
                e.line(),
                e.column()
            ),
            Self::Projection(e) => write!(f, "{e}"),
        }
    }
}
impl fmt::Debug for WorkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkError")
            .field("code", &self.code())
            .field("message", &self.to_string())
            .finish()
    }
}
impl std::error::Error for WorkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Utf8(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Projection(e) => Some(e),
            _ => None,
        }
    }
}
pub(super) fn decode<T: DeserializeOwned>(
    raw: &[u8],
    shape: model_wire::Fields,
) -> Result<T, WorkError> {
    decode_mode(raw, shape, false)
}
/// Decode with Go merge/null semantics and an explicit unknown-field policy.
/// No state is changed; codec errors preserve their cause and grant no authority.
pub(super) fn decode_mode<T: DeserializeOwned>(
    raw: &[u8],
    shape: model_wire::Fields,
    deny_unknown_fields: bool,
) -> Result<T, WorkError> {
    decode_mode_from(raw, shape, deny_unknown_fields, None)
}
/// Decode a Go DTO whose producer initialized fields before JSON merge.
/// The optional initial representation is explicit protocol data, not a policy
/// fallback. Both UTF8/JSON inputs and unknown/type/range errors preserve causes;
/// the shared serializer remains stack-safe and performs no IO or application.
pub(crate) fn decode_mode_from<T: DeserializeOwned>(
    raw: &[u8],
    shape: model_wire::Fields,
    deny_unknown_fields: bool,
    initial: Option<&[u8]>,
) -> Result<T, WorkError> {
    std::str::from_utf8(raw).map_err(WorkError::Utf8)?;
    if let Some(initial) = initial {
        std::str::from_utf8(initial).map_err(WorkError::Utf8)?;
    }
    let json = if initial.is_some() {
        model_wire::project_mode_from(raw, shape, initial, deny_unknown_fields)
    } else if deny_unknown_fields {
        model_wire::project_mode(raw, shape, true)
    } else {
        model_wire::project(raw, shape)
    }
    .map_err(WorkError::Projection)?;
    let mut deserializer = serde_json::Deserializer::from_slice(&json);
    deserializer.disable_recursion_limit();
    let value = T::deserialize(serde_stacker::Deserializer::new(&mut deserializer))
        .map_err(WorkError::Json)?;
    deserializer.end().map_err(WorkError::Json)?;
    Ok(value)
}
/// UTF-8 data codec, not a permission check or a task executor.
pub fn decode_work(raw: &[u8]) -> Result<models::Work, WorkError> {
    decode(raw, models::shape_work())
}
pub fn decode_task(raw: &[u8]) -> Result<models::Task, WorkError> {
    decode(raw, models::shape_task())
}
pub fn decode_event(raw: &[u8]) -> Result<models::Event, WorkError> {
    decode(raw, models::shape_event())
}
/// Bind the persisted row's columns and JSON envelope before returning any data.
pub fn decode_work_row(id: &str, revision: i64, raw: &[u8]) -> Result<models::Work, WorkError> {
    if !swarm_identity_contract::name(id.as_bytes()) {
        return Err(WorkError::Identity);
    }
    let work = decode_work(raw)?;
    if revision < 1 || work.schema_version != 1 || work.id != id || work.revision != revision {
        return Err(WorkError::Incoherent);
    }
    Ok(work)
}
pub fn encode_model(value: &impl Serialize) -> Result<Vec<u8>, WorkError> {
    let mut raw = Vec::new();
    let mut serializer = serde_json::Serializer::new(&mut raw);
    value
        .serialize(serde_stacker::Serializer::new(&mut serializer))
        .map_err(WorkError::Json)?;
    let raw = serde_json::value::RawValue::from_string(
        String::from_utf8(raw).expect("JSON serializer emits UTF-8"),
    )
    .map_err(WorkError::Json)?;
    swarm_wire_json::encode(&raw).map_err(WorkError::Json)
}

/// Compact public JSON for Go-compatible typed producers: HTML stays literal,
/// JavaScript separators are escaped, and opaque RawValue lexemes stay intact.
/// Uses the same growing-stack serializer as stored models. This validates no
/// actor/authority; serialization errors retain their cause and emit no output.
pub fn encode_public_model(value: &impl Serialize) -> Result<Vec<u8>, WorkError> {
    let mut raw = Vec::new();
    value
        .serialize(serde_stacker::Serializer::new(
            &mut serde_json::Serializer::new(&mut raw),
        ))
        .map_err(WorkError::Json)?;
    let text = String::from_utf8(raw).expect("JSON serializer emits UTF-8");
    Ok(text
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
        .into_bytes())
}
