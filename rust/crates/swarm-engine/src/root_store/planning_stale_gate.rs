//! Live-only classification of a durably settled pre-exec scope change.
//! This preserves the refusal; neither decoding a receipt nor replay creates it.
use super::{feedback_error, Receipt};
use crate::{RootStoreError as E, RuntimeError, RuntimeOperation, Work};
use std::error::Error;

/// Private runtime provenance; only the live settlement path constructs it.
/// Original failure remains traversable. Formatting excludes private receipt data.
struct SettledScopeChange {
    original: E,
    claim: String,
    scope: String,
}
impl std::fmt::Debug for SettledScopeChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("planning-scope-observation-stale")
    }
}
impl std::fmt::Display for SettledScopeChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
impl Error for SettledScopeChange {
    fn source(&self) -> Option<&(dyn Error + 'static)> { Some(&self.original) }
}

/// Recognize only the protected engine helper's exact pre-exec scope refusal.
/// Independent process/stop/close errors cannot enter as a plain gate refusal.
/// No provider reply, usage, proposal, missing cause, errno or unrelated gate is
/// accepted. A registered helper/grant attempt is not provider exec. These fixed
/// values are native wire vocabulary, not policy, attempt or time ceilings.
fn eligible(error: &E, receipt: &Receipt) -> bool {
    const CODE: &str = "planning-call-scope-contract-changed";
    let Some(gate) = receipt.gate.as_ref() else { return false; };
    matches!(error, E::Refused("planning-provider-gate-refused"))
        && receipt.schema_version == 1 && !receipt.output_complete
        && receipt.failure.as_deref() == Some("planning-provider-gate-refused")
        && receipt.exit_code == Some(2) && receipt.signal.is_none()
        && receipt.proposal.is_none() && receipt.grant_attempted == Some(true)
        && receipt.run.permission == 1 && receipt.run.phase == super::Phase::Permitted
        && receipt.usage.input_tokens.is_none() && receipt.usage.output_tokens.is_none()
        && receipt.usage.provider_reported_cost_usd.is_none()
        && gate.schema_version == 1 && gate.kind == "pre_exec_failure" && gate.code == CODE
        && gate.io_errno.is_none() && gate.sqlite_code.is_none()
        && gate.causes_scope.as_deref() == Some("known-native-compound-v1")
        && !gate.causes.is_empty()
        && gate.causes.iter().all(|c| c.valid() && c.io_errno.is_none()
            && c.sqlite_code.is_none() && c.labels == [CODE])
}

/// Preserve the normal Err result, adding live provenance only after receipt
/// publication and successful failure-feedback/usage/run commit plus cleanup.
/// Superseded feedback, unknown effects and every independent failure keep the
/// original compound error. Counts/reservations are never reset. A conductor may
/// reobserve; a new claim still needs all current guards, consent and budgets.
pub(super) fn settled(error: E, receipt: &Receipt, result: Result<Option<Work>, E>) -> E {
    if eligible(&error, receipt) {
        if let Ok(Some(work)) = &result {
            if work.id == receipt.run.work_id && work.revision > receipt.run.prepared_revision {
                return E::WorkRecorded {
                    work: work.id.clone(), revision: work.revision,
                    primary: Box::new(E::Runtime(RuntimeError::new(RuntimeOperation::Platform,
                        SettledScopeChange { original: error,
                            claim: receipt.run.claim_id.clone(), scope: receipt.run.scope_id.clone() }))),
                };
            }
        }
    }
    feedback_error(error, result)
}

/// Inspect clean WorkRecorded/runtime layers only; no arbitrary source walk.
/// The deepest live marker carries the actual settled revision, not an older
/// outer claim revision. Cleanup/storage/output wrappers deliberately refuse.
pub(in crate::root_store) fn metadata(error: &E) -> Option<(&str, &str, &str, i64)> {
    let E::WorkRecorded { primary, work, revision } = error else { return None; };
    match primary.as_ref() {
        E::Runtime(runtime) => Error::source(runtime)
            .and_then(|cause| cause.downcast_ref::<SettledScopeChange>())
            .map(|marker| (work.as_str(), marker.claim.as_str(), marker.scope.as_str(), *revision)),
        E::WorkRecorded { .. } => metadata(primary).filter(|(inner, _, _, _)| *inner == work),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Synthetic projection data only; no grant, receipt publication or SQL.
    fn receipt() -> Receipt {
        serde_json::from_value(json!({
            "schema_version":1,"run":{
                "claim_id":"claim","work_id":"work","scope_id":"child","holder":"fixture",
                "generation":1,"scope_revision":1,"prepared_revision":5,"settings_revision":1,
                "owner_pid":1,"owner_stamp":"fixture","owner_host":"fixture",
                "child_pid":2,"parent_pid":1,"group_id":2,"child_stamp":"fixture",
                "phase":"permitted","permission":1,"host_scope":"fixture","preflight_until":"fixture",
                "context_sha256":"fixture","workflow_prompt_sha256":"fixture","prompt_sha256":"fixture",
                "provider_digest":"fixture","directory_device":"fixture","directory_inode":"fixture",
                "decision_event":"fixture","prepared_at":"fixture","receipt_sha256":null,
                "applied_revision":null,"refusal_code":null
            },"finished_at":"fixture","exit_code":2,"signal":null,"output_complete":false,
            "proposal":null,"usage":{
                "claim_id":"claim","quality":"incomplete","input_tokens":null,"output_tokens":null,
                "cache_read_input_tokens":null,"cache_creation_input_tokens":null,"cached_input_tokens":null,
                "provider_reported_cost_usd":null,"source":null,"at":null,"reported_model":null,
                "model_source":null,"model_at":null
            },"facts":{},"failure":"planning-provider-gate-refused","grant_attempted":true,
            "failure_causes":null,"gate":{
                "schema_version":1,"kind":"pre_exec_failure","code":"planning-call-scope-contract-changed",
                "io_errno":null,"sqlite_code":null,"causes_scope":"known-native-compound-v1",
                "causes":[{"path":["root"],"labels":["planning-call-scope-contract-changed"],"io_errno":null,"sqlite_code":null}]
            },"stderr":[],"stderr_truncated":false
        })).unwrap()
    }
    /// Exact positive witness precedes unknown/partial/compound refusal variants.
    #[test]
    fn scope_gate_marker_requires_exact_pre_exec_and_successful_settlement() {
        let r=receipt();
        let failure=|| E::Refused("planning-provider-gate-refused");
        assert!(eligible(&failure(), &r));
        for change in [
            ("/exit_code",json!(0)),("/signal",json!(15)),("/output_complete",json!(true)),
            ("/proposal",json!("{}")),("/grant_attempted",json!(null)),("/run/permission",json!(0)),
            ("/usage/input_tokens",json!(7)),("/gate/code",json!("other-refusal")),
            ("/gate/causes",json!([])),("/gate/io_errno",json!(5)),
            ("/gate/causes/0/labels",json!(["other-refusal"])),
        ] {
            let mut v=serde_json::to_value(&r).unwrap();
            *v.pointer_mut(change.0).unwrap()=change.1;
            let changed=serde_json::from_value(v).unwrap();
            assert!(!eligible(&failure(),&changed),"{}",change.0);
        }
        let known=|| Work{id:"work".into(),revision:7,..Default::default()};
        let result=settled(failure(),&r,Ok(Some(known())));
        assert_eq!(metadata(&result),Some(("work","claim","child",7)));
        let causes=crate::native_diagnostics::project(&result);
        assert!(causes.iter().any(|c|c.labels.iter().any(|s|s=="planning-provider-gate-refused")));
        for result in [
            settled(failure(),&r,Ok(None)),
            settled(failure(),&r,Err(E::Refused("fixture-write-failed"))),
            settled(failure(),&r,Ok(Some(Work{id:"other".into(),revision:7,..Default::default()}))),
            settled(failure(),&r,Ok(Some(Work{id:"work".into(),revision:5,..Default::default()}))),
            settled(E::Cleanup{primary:Some(Box::new(failure())),cleanup:Box::new(E::Refused("fixture-close"))},&r,Ok(Some(known()))),
            E::Cleanup{primary:Some(Box::new(result)),cleanup:Box::new(E::Refused("fixture-output"))},
        ] { assert!(metadata(&result).is_none()); }
    }
}
