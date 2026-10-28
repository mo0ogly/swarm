//! Historical Agent transcript read capability; no view publication or visit mutation.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;

/// Strict explicit Work/Agent pair. Reject unknown, duplicate, or unsafe
/// values before any store IO. Identifiers are data, never another Root or
/// path choice.
fn query(raw: Option<&str>) -> Result<(String, String), E> {
    let (mut work, mut agent) = (None, None);
    for (key, value) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        let slot = match key.as_ref() {
            "work" => &mut work,
            "agent" => &mut agent,
            _ => return Err(E::Refused("agent-transcript-query-invalid")),
        };
        if slot.replace(value.into_owned()).is_some() {
            return Err(E::Refused("agent-transcript-query-invalid"));
        }
    }
    match (work, agent) {
        (Some(work), Some(agent))
            if swarm_identity_contract::name(work.as_bytes())
                && swarm_identity_contract::name(agent.as_bytes()) =>
        {
            Ok((work, agent))
        }
        _ => Err(E::Refused("agent-transcript-query-invalid")),
    }
}

/// Typed subset of the stored raw execution receipt. Every flag is required
/// (absence or a wrong type is a JSON error; no `false` defaults), unknown
/// sibling receipt fields are ignored, `capture` is optional.
#[derive(serde::Deserialize)]
struct Receipt {
    schema_version: i64,
    capture: Option<String>,
    capture_truncated: bool,
    capture_invalid_utf8: bool,
    output_complete: bool,
}

/// Decode the stored native receipt without inventing missing completeness
/// flags. Codec/schema errors are server-side read failures with original causes;
/// no mutation, fallback, identity or liveness follows from a valid document.
fn receipt(raw: &str) -> Result<Receipt, E> {
    let value: Receipt = serde_json::from_str(raw).map_err(|cause| root("agent-transcript-result-read", swarm_engine::RootStoreError::Work(swarm_engine::RecordedWorkError::Work(swarm_engine::WorkError::Json(cause)))))?;
    if value.schema_version != 1 {
        return Err(root("agent-transcript-result-read", swarm_engine::RootStoreError::Refused("agent-transcript-result-invalid")));
    }
    Ok(value)
}

/// Native transcript DTO. Transcript entries are redacted metadata (kinds,
/// counts, byte lengths, labels); no raw capture, prompt, tool input/output
/// or assistant text is included. Recorded text is not agent authentication,
/// liveness or permission; consumers must escape it for their rendering context.
#[derive(serde::Serialize)]
struct AgentTranscript {
    schema_version: i64,
    observation: &'static str,
    work_id: String,
    agent_id: String,
    recorded_status: String,
    state: &'static str,
    reason: Option<&'static str>,
    capture_truncated: Option<bool>,
    capture_invalid_utf8: Option<bool>,
    output_complete: Option<bool>,
    transcript: Option<swarm_engine::Transcript>,
}

impl NativeHttpService {
    /// Read the typed, redacted transcript derived from a stored Agent's
    /// `native_execution_result` capture under shared HTTP admission/capacity
    /// and the opened-readonly owned Root binding.
    ///
    /// Preconditions: GET/HEAD only, empty request body only, query carrying
    /// exactly `work` and `agent`, each once, with canonical identity names.
    /// The Agent must exist and its recorded work_id must equal the requested
    /// work. If a receipt is stored, its `schema_version` must be 1.
    ///
    /// State semantics:
    /// - No receipt, or a receipt with a missing/null `capture`:
    ///   state `unavailable`, reason `receipt-absent` or `capture-not-retained`,
    ///   `transcript` and all capture flags `null`.
    /// - Otherwise: state `available`, reason `null`, `transcript` set via
    ///   `typed_transcript` and all flags from the actual receipt. Note that
    ///   the flags (`capture_truncated`, `capture_invalid_utf8`,
    ///   `output_complete`) reflect declared receipt state only; they are not
    ///   recomputed from the transcript.
    ///
    /// Errors: unknown/duplicate/unsafe query parameters, or a non-empty body
    /// are refused before store IO (`agent-transcript-query-invalid`); a
    /// stored Agent belonging to a different Work is refused
    /// (`agent-transcript-work-mismatch`); a stored receipt with a
    /// `schema_version` other than 1 is refused (`agent-transcript-result-invalid`);
    /// a receipt that fails typed decoding (wrong types, duplicate known
    /// fields, missing required flags) fails as a typed Root read error (HTTP500); encoding also remains a
    /// server error, distinct from bad query data (HTTP400). Typed read/
    /// encode/close failures propagate, never an empty successful placeholder.
    ///
    /// No mutation: no Work, Agent, log, visit/OODA/budget or view state is
    /// changed; the Root is opened readonly and everything returned is an
    /// observation only.
    pub(super) fn agent_transcript(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !request.body().is_empty() {
            return Err(E::Refused("agent-transcript-query-invalid"));
        }
        let (work, agent) = query(request.uri().query())?;
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            store
                .read_work(&work)
                .map_err(|e| root("agent-transcript-work-read", e))?;
            let stored = store
                .read_agent(&agent)
                .map_err(|e| root("agent-transcript-agent-read", e))?;
            if stored.work_id != work {
                return Err(E::Refused("agent-transcript-work-mismatch"));
            }
            let (state, reason, flags, transcript) = match stored.native_execution_result.as_ref() {
                None => ("unavailable", Some("receipt-absent"), None, None),
                Some(raw) => {
                    let receipt = receipt(raw.0.get())?;
                    match receipt.capture.as_ref() {
                        None => ("unavailable", Some("capture-not-retained"), None, None),
                        Some(capture) => (
                            "available",
                            None,
                            Some((
                                receipt.capture_truncated,
                                receipt.capture_invalid_utf8,
                                receipt.output_complete,
                            )),
                            Some(swarm_engine::typed_transcript(capture)),
                        ),
                    }
                }
            };
            let (capture_truncated, capture_invalid_utf8, output_complete) =
                flags.map(|(a,b,c)| (Some(a),Some(b),Some(c))).unwrap_or((None, None, None));
            serde_json::to_vec(&AgentTranscript {
                schema_version: 1,
                observation: "recorded",
                work_id: work,
                agent_id: agent,
                recorded_status: stored.status,
                state,
                reason,
                capture_truncated,
                capture_invalid_utf8,
                output_complete,
                transcript,
            })
            .map(RouteBody::plain)
            .map_err(|cause| E::Model(swarm_engine::WorkError::Json(cause)))
        })();
        close(result, store.close())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A valid receipt witnesses explicit partial/output flags before malformed
    /// variants. Missing/duplicate/wrong-type flags cannot become false defaults;
    /// valid empty capture remains distinct from capture absence. No storage IO.
    #[test]
    fn required_receipt_flags_and_empty_capture() {
        let valid=r#"{"schema_version":1,"capture":"","capture_truncated":true,"capture_invalid_utf8":false,"output_complete":false}"#;
        let r=receipt(valid).unwrap();
        assert_eq!(r.schema_version,1);assert_eq!(r.capture.as_deref(),Some(""));
        assert!(r.capture_truncated&&!r.capture_invalid_utf8&&!r.output_complete);
        for bad in [valid.replace(",\"output_complete\":false", ""),valid.replace("\"output_complete\":false", "\"output_complete\":0"),valid.replace("\"capture_truncated\":true", "\"capture_truncated\":true,\"capture_truncated\":false"),valid.replace("\"capture\":\"\"", "\"capture\":42")] {
            assert_eq!(receipt(&bad).err().expect("invalid stored receipt").status(), http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        let absent=receipt(&valid.replace("\"capture\":\"\"", "\"capture\":null")).unwrap();
        assert!(absent.capture.is_none());
        assert_eq!(receipt(&valid.replace("\"schema_version\":1", "\"schema_version\":2")).err().expect("unsupported schema").status(), http::StatusCode::INTERNAL_SERVER_ERROR);
    }
}
