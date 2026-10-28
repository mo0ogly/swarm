//! Complete planning report observations. Declared event fields are data, never
//! authenticated authorship, acceptance or authorization to act on the report.
use crate::{gate_evaluation::artifact_bytes_limited, ExchangeArtifact, PlanningEvent};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{error::Error, fmt, path::Path};

/// Exact report bytes and declared provenance, only after full digest validation.
/// This observation grants no freshness lease or authority to its supplied labels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlanningReportContent {
    pub event_id: String,
    pub scope: String,
    pub task: String,
    pub attempt: String,
    pub path: String,
    pub sha256: String,
    pub text: String,
}
/// Safe failure family. The original IO/path/UTF8 cause remains available through
/// Error::source without exposing private paths or report bytes in diagnostics.
pub enum PlanningReportError {
    Decode(crate::WorkError),
    Artifact(crate::GateEvaluationError),
    Utf8(std::string::FromUtf8Error),
    Refused(&'static str),
}
impl PlanningReportError {
    /// Stable public error family; no supplied event or filesystem path is rendered.
    pub fn code(&self) -> &'static str {
        "planning-report-invalid"
    }
}
impl fmt::Display for PlanningReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::Decode(_) => "event-json",
            Self::Artifact(e) => e.reason(),
            Self::Utf8(_) => "report-utf8",
            Self::Refused(reason) => reason,
        };
        write!(f, "{}: {}", self.code(), reason)
    }
}
impl fmt::Debug for PlanningReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PlanningReportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(e) => Some(e),
            Self::Artifact(e) => Some(e),
            Self::Utf8(e) => Some(e),
            Self::Refused(_) => None,
        }
    }
}
/// Select the explicit handoff first. Only a historical handoff event with exactly
/// one lowercase .md artifact gets the legacy fallback. No IO or digest is invented.
fn reference(event: &PlanningEvent) -> Option<&ExchangeArtifact> {
    if let Some(value) = event.handoff.as_deref() {
        return Some(value);
    }
    match event.artifacts.as_deref() {
        Some([value]) if event.kind == "handoff" && value.path.ends_with(".md") => Some(value),
        _ => None,
    }
}
/// Observe a complete contained regular report under the caller's explicit byte
/// cap (zero refuses every nonempty report; None removes that cap). No reference
/// legitimately returns None. Selected but missing, empty, oversize, invalid UTF8,
/// NUL, altered, path/read/allocation/close failures return Err, never empty success.
/// The same complete buffer is hashed and returned; no truncation or ambient cap.
/// Canonical Root callers must supply persisted policy. Paths are observations,
/// not OS confinement; event labels/digests alone prove no author, rights, receipt,
/// current candidate, historical-import activation or future-action freshness.
pub fn read_planning_report(
    root: &Path,
    event: &PlanningEvent,
    limit: Option<usize>,
) -> Result<Option<PlanningReportContent>, PlanningReportError> {
    let Some(reference) = reference(event) else {
        return Ok(None);
    };
    read_reference(root, event, reference, limit).map(Some)
}
/// Read a complete explicit reference after the caller establishes its event
/// provenance. Same containment/digest/text/cap semantics as handoff reads, without
/// forging a handoff event. Labels remain data and establish no permission.
/// Empty/oversize/changed/invalid UTF8/NUL/IO/close failures preserve their cause.
pub(crate) fn read_reference(
    root: &Path,
    event: &PlanningEvent,
    reference: &ExchangeArtifact,
    limit: Option<usize>,
) -> Result<PlanningReportContent, PlanningReportError> {
    let (_, raw) = artifact_bytes_limited(root, &reference.path, limit)
        .map_err(PlanningReportError::Artifact)?;
    if raw.is_empty() || raw.contains(&0) {
        return Err(PlanningReportError::Refused("report-empty-or-nul"));
    }
    let text = String::from_utf8(raw).map_err(PlanningReportError::Utf8)?;
    if format!("{:x}", Sha256::digest(text.as_bytes())) != reference.sha256 {
        return Err(PlanningReportError::Refused("report-digest-changed"));
    }
    Ok(PlanningReportContent {
        event_id: event.id.clone(),
        scope: event.scope.clone(),
        task: event.task.clone(),
        attempt: event.attempt.clone(),
        path: reference.path.clone(),
        sha256: reference.sha256.clone(),
        text,
    })
}

/// Decode the complete known Go PlanningEvent shape with strict unknown fields,
/// typed values and actual duplicate/null/casefold semantics. This is untrusted
/// data only; errors preserve cause without rendering supplied bytes. Input
/// allocation scales with bytes; external callers own their transport boundary.
/// No schema, authorship, state, acknowledgement or permission is validated here.
pub fn decode_planning_report_event(raw: &[u8]) -> Result<PlanningEvent, PlanningReportError> {
    crate::work::decode_mode(raw, crate::models::shape_planning_event(), true)
        .map_err(PlanningReportError::Decode)
}
