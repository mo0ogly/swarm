//! Separate strict gate DTO. Opaque document data never grants authority.
use crate::{
    gate_json::Document,
    model_wire::{self, Fields, Kind},
    GateEvaluationError, GoRaw, WorkError,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GateRequest {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub task_id: String,
    pub phase: String,
    pub name: String,
    pub document: GoRaw,
}
impl fmt::Debug for GateRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GateRequest (data redacted)")
    }
}
const FIELDS: Fields = &[
    ("schema_version", Kind::Int64),
    ("event_id", Kind::String),
    ("expected_revision", Kind::Int64),
    ("task_id", Kind::String),
    ("phase", Kind::String),
    ("name", Kind::String),
    ("document", Kind::Raw),
];
pub enum GateRequestError {
    Schema,
    TaskMissing,
    UnknownField,
    Json(serde_json::Error),
    Projection(model_wire::Error),
    Document(GateEvaluationError),
    Encode(WorkError),
}
impl GateRequestError {
    pub fn code(&self) -> &'static str {
        if matches!(self, Self::Schema) {
            "invalid_schema"
        } else {
            "format"
        }
    }
}
impl fmt::Display for GateRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for GateRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for GateRequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            Self::Projection(e) => Some(e),
            Self::Document(e) => Some(e),
            Self::Encode(e) => Some(e),
            _ => None,
        }
    }
}
pub fn decode_gate_request(raw: &[u8]) -> Result<GateRequest, GateRequestError> {
    let document = Document::parse(raw).map_err(|e| GateRequestError::Document(e.into()))?;
    if let Some(fields) = document.object(0) {
        for key in fields.keys() {
            if !FIELDS
                .iter()
                .any(|(name, _)| model_wire::fold(name) == model_wire::fold(key))
            {
                return Err(GateRequestError::UnknownField);
            }
        }
    }
    let projected = model_wire::project(raw, FIELDS).map_err(GateRequestError::Projection)?;
    let mut decoder = serde_json::Deserializer::from_slice(&projected);
    decoder.disable_recursion_limit();
    let request = GateRequest::deserialize(serde_stacker::Deserializer::new(&mut decoder))
        .map_err(GateRequestError::Json)?;
    decoder.end().map_err(GateRequestError::Json)?;
    if request.schema_version != 1 {
        return Err(GateRequestError::Schema);
    }
    if request.task_id.is_empty() {
        return Err(GateRequestError::TaskMissing);
    }
    Ok(request)
}
/// Go-compatible normalized gate request, including opaque numeric lexemes.
pub fn encode_gate_request_intent(request: &GateRequest) -> Result<Vec<u8>, GateRequestError> {
    crate::encode_model(request).map_err(GateRequestError::Encode)
}
