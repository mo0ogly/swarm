//! Provider declarations, never a requested route, authenticated model or invoice.
use crate::{
    model_wire::{object, Boundaries},
    AgentGuardError, GoFloat, ReportedModel, Usage,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Interoperability bound of Go providerUsage's float64→integer contract. This
/// is not a token budget, retention policy or process stop; values outside this
/// legacy wire domain stay unknown rather than becoming rounded measurements.
const GO_USAGE_MAX_INTEGER: f64 = 9_000_000_000_000_000.0;
type Fields<'a> = BTreeMap<String, &'a [u8]>;
/// Current observation quality; an earlier valid declaration is only history.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FactState {
    #[default]
    NotReported,
    Observed,
    Invalid,
    Incomplete,
}
/// One typed channel retains its first refusal and last valid declaration.
/// There is no deserialization path from Agent JSON into this observation state.
struct Channel<T> {
    state: FactState,
    last_valid: Option<T>,
    first_error: Option<AgentGuardError>,
    invalid_events: u64,
}
impl<T> Default for Channel<T> {
    /// Begin unknown, never a measured zero or an inherited stored assertion.
    fn default() -> Self {
        Self {
            state: FactState::NotReported,
            last_valid: None,
            first_error: None,
            invalid_events: 0,
        }
    }
}
impl<T> Channel<T> {
    /// Apply a fresh independent candidate. Checked diagnostic counting precedes
    /// mutation; malformed data clears canonical visibility, not prior evidence.
    /// Absence is explicit and does not manufacture a zero measurement.
    fn record(
        &mut self,
        candidate: Result<Option<T>, AgentGuardError>,
    ) -> Result<(), AgentGuardError> {
        match candidate {
            Ok(Some(value)) => {
                self.last_valid = Some(value);
                self.state = FactState::Observed;
            }
            Ok(None) => self.state = FactState::NotReported,
            Err(error) => {
                let count = self.invalid_events.checked_add(1).ok_or_else(|| {
                    AgentGuardError::refused("agent-provider-fact-counter-overflow")
                })?;
                self.state = FactState::Invalid;
                self.invalid_events = count;
                if self.first_error.is_none() {
                    self.first_error = Some(error);
                }
            }
        }
        Ok(())
    }
    /// Only a currently valid declaration is canonical. History is not fallback.
    fn current(&self) -> Option<&T> {
        if self.state == FactState::Observed {
            self.last_valid.as_ref()
        } else {
            None
        }
    }
}
/// Owned serialization projection built by this module, not public input.
/// Canonical fields are absent after a gap/refusal; last-valid fields are then
/// diagnostic history only. Numeric cost remains explicitly provider reported.
#[derive(Clone, Serialize)]
pub(crate) struct ProviderFactsSnapshot {
    schema_version: i64,
    source: &'static str,
    pub(crate) usage_state: FactState,
    pub(crate) model_state: FactState,
    pub usage: Option<Usage>,
    pub reported_model: Option<ReportedModel>,
    last_valid_usage: Option<Usage>,
    last_valid_reported_model: Option<ReportedModel>,
    first_usage_error: Option<&'static str>,
    first_model_error: Option<&'static str>,
    invalid_usage_events: u64,
    invalid_model_events: u64,
}
/// Independent channels prevent malformed billing/model metadata from hiding a
/// valid tool call or authorizing a process stop. Raw error text is never output.
#[derive(Default)]
pub(crate) struct ProviderFacts {
    usage: Channel<Usage>,
    model: Channel<ReportedModel>,
}
/// Read immediate members from already validated iterative JSON boundaries.
/// Shape errors preserve their original cause; no recursive Value tree is built.
fn fields<'a>(raw: &'a [u8], boundaries: &Boundaries) -> Result<Fields<'a>, AgentGuardError> {
    object(raw.trim_ascii(), boundaries)
        .map(|v| v.into_iter().collect())
        .map_err(|e| AgentGuardError::caused("agent-provider-fact-object", e))
}
/// Decode a required string with a safe family and original codec cause.
fn text(raw: &[u8]) -> Result<String, AgentGuardError> {
    swarm_wire_json::decode(raw)
        .map_err(|e| AgentGuardError::caused("agent-provider-fact-string", e))
}
/// Resolve actual event type. Legacy terminal results without type require all
/// success/session witnesses; a supplied wrong type never becomes that fallback.
/// Unknown nonterminal types remain for the stream guard to classify separately.
pub(crate) fn event_kind(top: &Fields<'_>) -> Result<String, AgentGuardError> {
    let kind = top
        .get("type")
        .map(|v| text(v))
        .transpose()?
        .unwrap_or_default();
    if !kind.is_empty() {
        return Ok(kind);
    }
    if top.get("subtype").map(|v| text(v)).transpose()?.as_deref() == Some("success")
        && top
            .get("terminal_reason")
            .map(|v| text(v))
            .transpose()?
            .as_deref()
            == Some("completed")
        && top
            .get("session_id")
            .map(|v| text(v))
            .transpose()?
            .is_some_and(|s| !s.is_empty())
    {
        return Ok("terminal_result".into());
    }
    Err(AgentGuardError::refused("agent-event-type-missing"))
}
/// Decode the Go float64-compatible numeric integer domain, without strings/null,
/// representable fractions, negative/nonfinite values or a cast outside its bound.
fn tokens(raw: &[u8]) -> Result<i64, AgentGuardError> {
    let value: GoFloat = swarm_wire_json::decode(raw)
        .map_err(|e| AgentGuardError::caused("agent-provider-usage-number", e))?;
    let n = value.0;
    if !n.is_finite() || !(0.0..=GO_USAGE_MAX_INTEGER).contains(&n) || n.fract() != 0.0 {
        return Err(AgentGuardError::refused(
            "agent-provider-usage-integer-domain",
        ));
    }
    Ok(n as i64)
}
/// Extract one terminal observation, never a sum across turns. Missing usage
/// stays absent; required/optional supplied fields are strict, including null.
/// Timestamp is supplied by the engine's current clock, never provider input.
fn usage(
    top: &Fields<'_>,
    kind: &str,
    b: &Boundaries,
    at: &str,
) -> Result<Option<Usage>, AgentGuardError> {
    let Some(raw) = top.get("usage") else {
        return Ok(None);
    };
    let m = fields(raw, b)?;
    let required = |name: &str| {
        tokens(
            m.get(name)
                .copied()
                .ok_or_else(|| AgentGuardError::refused("agent-provider-usage-required"))?,
        )
    };
    let optional = |name: &str| m.get(name).map(|v| tokens(v).map(Box::new)).transpose();
    let cost = top
        .get("total_cost_usd")
        .map(|v| -> Result<Box<GoFloat>, AgentGuardError> {
            let n: GoFloat = swarm_wire_json::decode(v)
                .map_err(|e| AgentGuardError::caused("agent-provider-cost-number", e))?;
            if !n.0.is_finite() || n.0 < 0.0 {
                return Err(AgentGuardError::refused("agent-provider-cost-nonnegative"));
            }
            Ok(Box::new(n))
        })
        .transpose()?;
    Ok(Some(Usage{input_tokens:required("input_tokens")?,output_tokens:required("output_tokens")?,cache_read_input_tokens:optional("cache_read_input_tokens")?,cache_creation_input_tokens:optional("cache_creation_input_tokens")?,cached_input_tokens:optional("cached_input_tokens")?,provider_reported_cost_usd:cost,source:format!("événement fournisseur {kind}"),scope:"dernier événement terminal valide, déclaration non authentifiée ; caches séparés, sans somme ni cumul de session".into(),at:at.into()}))
}
/// Extract only explicit supported model declarations. The frozen event cap is
/// the persisted size boundary; no hidden 200-byte model policy is introduced.
/// Whitespace trimming matches Go; control/format characters refuse instead of
/// silently rewriting one asserted identifier into another. No route is resolved.
fn model(
    top: &Fields<'_>,
    kind: &str,
    b: &Boundaries,
    at: &str,
) -> Result<Option<ReportedModel>, AgentGuardError> {
    let message;
    let (container, source) = if kind == "system" {
        if top.get("subtype").map(|v| text(v)).transpose()?.as_deref() != Some("init") {
            return Ok(None);
        }
        (top, "provider system/init")
    } else if kind == "assistant" {
        let Some(raw) = top.get("message") else {
            return Ok(None);
        };
        message = fields(raw, b)?;
        (&message, "provider assistant/message")
    } else {
        return Ok(None);
    };
    let Some(raw) = container.get("model") else {
        return Ok(None);
    };
    let supplied = text(raw)?;
    let value = crate::text::trim_space(&supplied);
    if value.is_empty() || crate::agent_terminal_text::terminal_text(value) != value {
        return Err(AgentGuardError::refused("agent-provider-model-invalid"));
    }
    Ok(Some(ReportedModel {
        model: value.into(),
        source: source.into(),
        at: at.into(),
    }))
}
impl ProviderFacts {
    /// Observe complete validated input using an engine-generated RFC3339 time.
    /// Metadata refusals clear only their channel and retain typed diagnostics;
    /// tool attribution proceeds independently. Only counter representation
    /// failure returns Err. No IO, financial mutation, retry or grant occurs.
    pub(crate) fn observe(
        &mut self,
        top: &Fields<'_>,
        kind: &str,
        b: &Boundaries,
        at: &str,
    ) -> Result<(), AgentGuardError> {
        if matches!(kind, "result" | "turn.completed" | "terminal_result") {
            self.usage.record(usage(top, kind, b, at))?;
        }
        match model(top, kind, b, at) {
            Ok(None) => {}
            candidate => self.model.record(candidate)?,
        }
        Ok(())
    }
    /// Mark missing/malformed/truncated output without erasing earlier evidence.
    /// A later independently valid declaration can establish a new observation;
    /// no partial line or old stored JSON becomes canonical after an output gap.
    pub(crate) fn incomplete(&mut self) {
        self.usage.state = FactState::Incomplete;
        self.model.state = FactState::Incomplete;
    }
    /// Clone a safe owned projection. Raw errors remain in memory as causes,
    /// while output exposes only families, exact counts, provenance and allowed
    /// declaration fields. Unknown channels never supply a canonical zero.
    pub(crate) fn snapshot(&self) -> ProviderFactsSnapshot {
        ProviderFactsSnapshot {
            schema_version: 1,
            source: "provider_declared_unverified",
            usage_state: self.usage.state,
            model_state: self.model.state,
            usage: self.usage.current().cloned(),
            reported_model: self.model.current().cloned(),
            last_valid_usage: if self.usage.state == FactState::Observed {
                None
            } else {
                self.usage.last_valid.clone()
            },
            last_valid_reported_model: if self.model.state == FactState::Observed {
                None
            } else {
                self.model.last_valid.clone()
            },
            first_usage_error: self.usage.first_error.as_ref().map(AgentGuardError::code),
            first_model_error: self.model.first_error.as_ref().map(AgentGuardError::code),
            invalid_usage_events: self.usage.invalid_events,
            invalid_model_events: self.model.invalid_events,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const AT: &str = "2026-10-19T12:00:00Z";
    /// Real iterative JSON validation precedes extraction in every fixture.
    fn observe(f: &mut ProviderFacts, raw: &[u8]) {
        let b = crate::agent_tool_guard::validated_json(raw).unwrap();
        let top = fields(raw, &b).unwrap();
        let kind = event_kind(&top).unwrap();
        f.observe(&top, &kind, &b, AT).unwrap();
    }
    /// Last terminal fields replace previous values; missing is not measured0,
    /// and caches are separate observations rather than a computed total.
    #[test]
    fn usage_replaces_without_summing_and_keeps_missing_distinct_from_zero() {
        let mut f = ProviderFacts::default();
        assert!(f.snapshot().usage.is_none());
        observe(&mut f,br#"{"type":"turn.completed","usage":{"input_tokens":7,"output_tokens":3,"cached_input_tokens":0},"total_cost_usd":0.01}"#);
        assert_eq!(f.snapshot().usage.unwrap().input_tokens, 7);
        observe(&mut f,br#"{"type":"result","usage":{"input_tokens":0,"output_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":2},"total_cost_usd":25}"#);
        let u = f.snapshot().usage.unwrap();
        assert_eq!((u.input_tokens, u.output_tokens), (0, 1));
        assert_eq!(u.cache_read_input_tokens.as_deref(), Some(&0));
        assert_eq!(u.cache_creation_input_tokens.as_deref(), Some(&2));
        assert!(u.cached_input_tokens.is_none());
        assert_eq!(u.provider_reported_cost_usd.unwrap().0, 25.0);
        assert_eq!(u.at, AT);
        observe(&mut f, br#"{"type":"result"}"#);
        let v = f.snapshot();
        assert!(v.usage.is_none());
        assert_eq!(v.usage_state, FactState::NotReported);
        assert_eq!(v.last_valid_usage.unwrap().output_tokens, 1);
    }
    /// Valid control precedes each invalid payload and recovery on the same
    /// channel. Wrong supplied optional fields cannot become empty success.
    #[test]
    fn usage_invalid_fields_preserve_history_and_recover() {
        for raw in [br#"{"type":"result","usage":null}"#.as_slice(),br#"{"type":"result","usage":{}}"#,br#"{"type":"result","usage":{"input_tokens":-1,"output_tokens":0}}"#,br#"{"type":"result","usage":{"input_tokens":1.5,"output_tokens":0}}"#,br#"{"type":"result","usage":{"input_tokens":9000000000000001,"output_tokens":0}}"#,br#"{"type":"result","usage":{"input_tokens":0,"output_tokens":"PRIVATE"}}"#,br#"{"type":"result","usage":{"input_tokens":0,"output_tokens":0,"cached_input_tokens":null}}"#,br#"{"type":"result","usage":{"input_tokens":0,"output_tokens":0},"total_cost_usd":-1}"#,br#"{"type":"result","usage":{"input_tokens":0,"output_tokens":0},"total_cost_usd":null}"#] {
            let mut f=ProviderFacts::default();let good=br#"{"type":"result","usage":{"input_tokens":2,"output_tokens":3}}"#;observe(&mut f,good);assert!(f.snapshot().usage.is_some());observe(&mut f,raw);let v=f.snapshot();assert!(v.usage.is_none());assert_eq!(v.usage_state,FactState::Invalid);assert_eq!(v.last_valid_usage.unwrap().input_tokens,2);assert_eq!(v.invalid_usage_events,1);assert!(v.first_usage_error.is_some());assert!(!serde_json::to_string(&f.snapshot()).unwrap().contains("PRIVATE"));observe(&mut f,good);assert_eq!(f.snapshot().usage.unwrap().input_tokens,2);assert_eq!(f.snapshot().invalid_usage_events,1);
        }
        assert_eq!(tokens(b"9e15").unwrap(), 9_000_000_000_000_000);
        assert_eq!(tokens(b"1e3").unwrap(), 1000);
        assert_eq!(tokens(b"-0").unwrap(), 0);
        assert!(tokens(b"1e400").is_err());
    }
    /// Event framing provides the explicit persisted size bound; valid IDs
    /// exceeding Go's former hidden200-byte policy remain declarations only.
    /// Cc/Cf and wrong shapes refuse instead of being silently rewritten.
    #[test]
    fn model_declarations_reject_controls_and_preserve_engine_timestamp() {
        let mut f = ProviderFacts::default();
        let name = "m".repeat(257);
        let raw=serde_json::to_vec(&serde_json::json!({"type":"system","subtype":"init","model":format!(" {name} "),"at":"1900-01-01T00:00:00Z"})).unwrap();
        observe(&mut f, &raw);
        let m = f.snapshot().reported_model.unwrap();
        assert_eq!(m.model, name);
        assert_eq!(m.at, AT);
        for bad in [
            serde_json::json!(null),
            serde_json::json!(3),
            serde_json::json!(""),
            serde_json::json!("m\u{0000}x"),
            serde_json::json!("m\u{202e}x"),
            serde_json::json!({"PRIVATE":"secret"}),
        ] {
            observe(&mut f, &raw);
            let invalid = serde_json::to_vec(
                &serde_json::json!({"type":"assistant","message":{"model":bad}}),
            )
            .unwrap();
            observe(&mut f, &invalid);
            let v = f.snapshot();
            assert!(v.reported_model.is_none());
            assert_eq!(v.model_state, FactState::Invalid);
            assert_eq!(v.last_valid_reported_model.unwrap().model, name);
            assert!(!serde_json::to_string(&f.snapshot())
                .unwrap()
                .contains("secret"));
        }
        observe(
            &mut f,
            br#"{"type":"assistant","message":{"model":"fresh"}}"#,
        );
        assert_eq!(f.snapshot().reported_model.unwrap().model, "fresh");
    }
    /// Legacy terminal inference requires the complete wire witness; a wrong
    /// supplied type/session is never silently normalized into a terminal event.
    #[test]
    fn legacy_terminal_kind_requires_complete_witness() {
        let mut f = ProviderFacts::default();
        observe(&mut f,br#"{"subtype":"success","terminal_reason":"completed","session_id":"S","usage":{"input_tokens":0,"output_tokens":0}}"#);
        assert_eq!(f.snapshot().usage.unwrap().input_tokens, 0);
        for raw in [
            br#"{}"#.as_slice(),
            br#"{"type":3,"subtype":"success","terminal_reason":"completed","session_id":"S"}"#,
            br#"{"subtype":"success","terminal_reason":"completed","session_id":""}"#,
            br#"{"subtype":"success","session_id":"S"}"#,
        ] {
            let b = crate::agent_tool_guard::validated_json(raw).unwrap();
            assert!(event_kind(&fields(raw, &b).unwrap()).is_err());
        }
    }
    /// Missing output clears current channels but keeps earlier observations as
    /// explicit history. A fresh valid usage event does not restore model proof.
    #[test]
    fn output_gap_preserves_history_without_canonical_fallback() {
        let mut f = ProviderFacts::default();
        observe(&mut f, br#"{"type":"system","subtype":"init","model":"m"}"#);
        observe(
            &mut f,
            br#"{"type":"result","usage":{"input_tokens":1,"output_tokens":2}}"#,
        );
        f.incomplete();
        let v = f.snapshot();
        assert!(v.usage.is_none() && v.reported_model.is_none());
        assert_eq!(v.usage_state, FactState::Incomplete);
        assert_eq!(v.last_valid_usage.unwrap().output_tokens, 2);
        assert_eq!(v.last_valid_reported_model.unwrap().model, "m");
        observe(
            &mut f,
            br#"{"type":"result","usage":{"input_tokens":0,"output_tokens":0}}"#,
        );
        assert_eq!(f.snapshot().usage.unwrap().input_tokens, 0);
        assert!(f.snapshot().reported_model.is_none());
    }
    /// Diagnostic overflow refuses before mutation; first typed codec causes
    /// survive while public snapshots expose only safe families and counts.
    #[test]
    fn diagnostic_counter_refuses_overflow_and_keeps_typed_cause() {
        use std::error::Error;
        let mut f = ProviderFacts::default();
        observe(
            &mut f,
            br#"{"type":"result","usage":{"input_tokens":"PRIVATE","output_tokens":0}}"#,
        );
        assert!(f.usage.first_error.as_ref().unwrap().source().is_some());
        assert!(!serde_json::to_string(&f.snapshot())
            .unwrap()
            .contains("PRIVATE"));
        f.usage.invalid_events = u64::MAX;
        let before = f.usage.state;
        let error = f
            .usage
            .record(Err(AgentGuardError::refused("fixture")))
            .unwrap_err();
        assert_eq!(error.code(), "agent-provider-fact-counter-overflow");
        assert_eq!(f.usage.invalid_events, u64::MAX);
        assert_eq!(f.usage.state, before);
    }
}
