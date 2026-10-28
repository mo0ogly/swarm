//! Root cycle decisions, separate from candidate review and scope closure.
use crate::{
    AgentLaunchError as E, PlanningDecisionPolicy as P, PlanningOperation, PlanningRequest, Work,
};

/// Structural model hints only. Caller checks the adopted operation policy;
/// decisions independently revalidate leases, current cycle, quotas and evidence.
/// A closed/missing/failed judge yields no capability, never an inferred grant.
/// No IO, mutation, authenticated identity or end-candidate proof is established.
pub(crate) fn availability(work: &Work, scope: &str) -> (bool, bool) {
    if scope != "root" {
        return (false, false);
    }
    let Some(p) = work.planning.as_deref() else {
        return (false, false);
    };
    let Some(j) = p.judge.as_deref() else {
        return (false, false);
    };
    if !p.failure.is_empty() || j.closed || j.cycle < 1 || j.max_cycles < 0 {
        return (false, false);
    }
    (
        j.verdict.is_empty(),
        j.verdict == "continue"
            && j.cycle.checked_add(1).is_some()
            && (j.max_cycles == 0 || j.cycle < j.max_cycles),
    )
}

/// Apply one root-only continue/renew operation to a private Work candidate.
/// Requires the caller's complete validated Work/lease/current adopted decision
/// policy; rechecks state/history, exclusive fields, cycle and saved cycle quota.
/// New reasons obey the existing persisted byte cap; old history stays intact.
/// Continue's optional evidence reference is supplied data, not a checked Git
/// candidate. End is refused until native managed-candidate closure is ported.
/// Renew adds only renewed_at/current cycle and clears the live verdict fields;
/// it never resets activations, decisions, funding, attempts or older verdicts.
/// Encoding/allocation/overflow/state failures retain typed causes; caller must
/// discard this private candidate on error. No process/SQL/claim/acceptance runs.
pub(crate) fn apply(
    w: &mut Work,
    op: &PlanningOperation,
    r: &PlanningRequest,
    at: &str,
    policy: &P,
) -> Result<(), E> {
    if r.scope != "root" {
        return Err(E::Refused("planning-judge-root-required"));
    }
    if r.operations.as_deref().unwrap_or_default().len() != 1 {
        return Err(E::Refused("planning-judge-exclusive-operation"));
    }
    if !op.id.is_empty()
        || !op.title.is_empty()
        || !op.next.is_empty()
        || !op.deliverable.is_empty()
        || op.requirements.as_ref().is_some_and(|v| !v.is_empty())
        || op.criteria.as_ref().is_some_and(|v| !v.is_empty())
        || op.depends.as_ref().is_some_and(|v| !v.is_empty())
        || op.max_tasks != 0
        || op.max_activations != 0
    {
        return Err(E::Refused("planning-judge-field-not-applicable"));
    }
    let kind = transition(
        w,
        &Decision {
            kind: op.kind.as_str(),
            cycle: op.judge_cycle,
            verdict: &op.judge_verdict,
            candidate: &op.expected_candidate,
            reason: &r.reason,
        },
        at,
        policy,
    )?;
    let p = w
        .planning
        .as_deref_mut()
        .ok_or(E::Refused("planning-not-enabled"))?;
    let j = p
        .judge
        .as_deref()
        .ok_or(E::Refused("planning-judge-required"))?;
    let cycle = j.cycle;
    let id = crate::planning_decision::event_id(&[&r.event_id, kind])?;
    let inbox = p.inbox.get_or_insert_default();
    if inbox.iter().any(|e| e.id == id) {
        return Err(E::Refused("planning-judge-event-conflict"));
    }
    inbox
        .try_reserve(1)
        .map_err(|e| E::caused("planning-judge-inbox-capacity", e))?;
    inbox.push(crate::PlanningEvent { id, scope:"root".into(), kind:kind.into(),
        message:format!("Décision du superviseur appliquée ; cycle {cycle}. Reconstruire le contexte depuis les exigences, résultats et incidents courants ; historique et consommation conservés."),at:at.into(),..Default::default() });
    crate::validate_judge_state(p.judge.as_deref())
        .map_err(|e| E::caused("planning-judge-result-invalid", e))?;
    Ok(())
}

/// Internal values from one accepted operation or operator request. These are
/// data, not a lease, caller-provided permission flag or proof of actor identity.
struct Decision<'a> {
    kind: &'a str,
    cycle: i64,
    verdict: &'a str,
    candidate: &'a str,
    reason: &'a str,
}

/// Shared state transition without inbox, scope revision or accounting effects.
/// Caller owns the validated boundary/private clone and discards it on failure.
/// Current policy caps only new reasons; semantic history/quotas/representation
/// remain checked. No canonical event or receipt is manufactured by this helper.
fn transition(w: &mut Work, d: &Decision<'_>, at: &str, policy: &P) -> Result<&'static str, E> {
    P::check(
        policy.max_reason_bytes,
        d.reason.len(),
        "planning-decision-reason-limit",
    )?;
    let p = w
        .planning
        .as_deref_mut()
        .ok_or(E::Refused("planning-not-enabled"))?;
    if !p.failure.is_empty() {
        return Err(E::Refused("planning-failed"));
    }
    crate::validate_judge_state(p.judge.as_deref())
        .map_err(|e| E::caused("planning-judge-state-invalid", e))?;
    let j = p
        .judge
        .as_deref_mut()
        .ok_or(E::Refused("planning-judge-required"))?;
    if j.closed {
        return Err(E::Refused("planning-judge-closed"));
    }
    if d.cycle != j.cycle {
        return Err(E::Refused("planning-judge-stale-cycle"));
    }
    let kind = match d.kind {
        "judge" => {
            if !j.verdict.is_empty() {
                return Err(E::Refused("planning-judge-already-decided"));
            }
            if !matches!(d.verdict, "continue" | "end") {
                return Err(E::Refused("planning-judge-verdict-invalid"));
            }
            if d.verdict == "end" {
                return Err(E::Refused("planning-judge-end-managed-evidence-unported"));
            }
            if !crate::text::nonempty(d.reason) {
                return Err(E::Refused("planning-judge-reason-required"));
            }
            let history = j.history.get_or_insert_default();
            history
                .try_reserve(1)
                .map_err(|e| E::caused("planning-judge-history-capacity", e))?;
            j.verdict = "continue".into();
            j.reason = d.reason.into();
            j.evidence_ref = d.candidate.into();
            j.at = at.into();
            history.push(crate::JudgeCycleRecord {
                cycle: j.cycle,
                verdict: j.verdict.clone(),
                reason: j.reason.clone(),
                evidence_ref: j.evidence_ref.clone(),
                at: j.at.clone(),
                ..Default::default()
            });
            "cycle_judged"
        }
        "renew" => {
            if !d.verdict.is_empty() || !d.candidate.is_empty() {
                return Err(E::Refused("planning-judge-renew-field-not-applicable"));
            }
            if j.verdict != "continue" {
                return Err(E::Refused("planning-judge-continue-required"));
            }
            if j.max_cycles > 0 && j.cycle >= j.max_cycles {
                return Err(E::Refused("planning-judge-cycle-budget-exhausted"));
            }
            let cycle = j
                .cycle
                .checked_add(1)
                .ok_or(E::Refused("planning-judge-cycle-representation"))?;
            let last = j
                .history
                .as_mut()
                .and_then(|h| h.last_mut())
                .ok_or(E::Refused("planning-judge-history-required"))?;
            if last.cycle != j.cycle || !last.renewed_at.is_empty() {
                return Err(E::Refused("planning-judge-history-changed"));
            }
            last.renewed_at = at.into();
            j.cycle = cycle;
            j.verdict.clear();
            j.reason.clear();
            j.evidence_ref.clear();
            j.at.clear();
            "cycle_renewed"
        }
        _ => return Err(E::Refused("planning-judge-operation-unported")),
    };
    crate::validate_judge_state(p.judge.as_deref())
        .map_err(|e| E::caused("planning-judge-result-invalid", e))?;
    Ok(kind)
}

/// Root operator cycle action, distinct from leased decide and lease renewal.
/// Requires a canonical transaction/revision/settings reservation from caller;
/// validates exact applicable fields, root scope/revision and unpaused state.
/// Nonempty inputs are rejected rather than pretended acknowledged. Any logical
/// holder, even expired, refuses: only public pause/resume may revoke it, and
/// caller separately checks native physical ownership in the same transaction.
/// Returns a private candidate with judge transition and root scope revision+1;
/// preserves inbox decisions, activations, decisions, funding and task state.
/// Typed codec/state/policy/overflow failures retain cause. No IO/model/claim,
/// synthetic received context, input acknowledgement or acceptance is produced.
pub(crate) fn operator_candidate(
    work: &Work,
    r: &PlanningRequest,
    kind: &str,
    at: &str,
    settings: &crate::EngineSettingsDocument,
) -> Result<Work, E> {
    if r.schema_version != 1 {
        return Err(E::Refused("planning-schema-required"));
    }
    if r.scope != "root" {
        return Err(E::Refused("planning-judge-root-required"));
    }
    if r.input_events.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err(E::Refused("planning-judge-unacknowledged-inputs"));
    }
    let raw = crate::encode_planning_request(r)
        .map_err(|e| E::caused("planning-judge-request-encoding", e))?;
    let fields: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&raw).map_err(|e| E::caused("planning-judge-request-format", e))?;
    for field in fields.keys() {
        let allowed = matches!(
            field.as_str(),
            "schema_version"
                | "event_id"
                | "expected_revision"
                | "scope"
                | "scope_revision"
                | "judge_cycle"
                | "reason"
        ) || kind == "judge"
            && matches!(field.as_str(), "judge_verdict" | "expected_candidate");
        if !allowed {
            return Err(E::Refused("planning-judge-operator-field-not-applicable"));
        }
    }
    settings.validate().map_err(|e| E::caused(e.reason(), e))?;
    crate::validate_planning_state(work)
        .map_err(|e| E::caused("planning-judge-state-invalid", e))?;
    let p = work
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    if p.paused {
        return Err(E::Refused("planning-paused"));
    }
    let scope = p
        .scopes
        .iter()
        .flatten()
        .find(|s| s.id == "root")
        .ok_or(E::Refused("planning-root-required"))?;
    if r.scope_revision != scope.revision {
        return Err(E::Refused("revision_conflict"));
    }
    if !scope.holder.is_empty() {
        return Err(E::Refused("planning-judge-owner-active"));
    }
    let revision = scope
        .revision
        .checked_add(1)
        .ok_or(E::Refused("planning-judge-scope-revision-overflow"))?;
    let policy = settings
        .values
        .planning_decision_policy()
        .map_err(|e| E::caused(e.reason(), e))?;
    let mut next = work.clone();
    transition(
        &mut next,
        &Decision {
            kind,
            cycle: r.judge_cycle,
            verdict: &r.judge_verdict,
            candidate: &r.expected_candidate,
            reason: &r.reason,
        },
        at,
        &policy,
    )?;
    next.planning
        .as_mut()
        .unwrap()
        .scopes
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s.id == "root")
        .unwrap()
        .revision = revision;
    crate::validate_planning_state(&next)
        .map_err(|e| E::caused("planning-judge-result-invalid", e))?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Pure enabled state only; no Root/provider/receipt or authenticated actor.
    fn fixture() -> Work {
        let settings = crate::EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture".into(),
            "2026-10-22T12:00:00Z".into(),
        )
        .unwrap();
        crate::apply_planning_control(
            &Work {
                schema_version: 1,
                id: "wire-work".into(),
                criteria: Some(vec!["Open requirement".into()]),
                ..Default::default()
            },
            crate::PlanningControlAction::Enable,
            &PlanningRequest {
                schema_version: 1,
                event_id: "wire-enable".into(),
                max_tasks: 3,
                max_decisions: 10,
                max_activations: 10,
                max_judge_cycles: 2,
                ..Default::default()
            },
            "2026-10-22T12:00:00Z",
            &settings,
        )
        .unwrap()
    }
    /// Explicit test policy, not an installation fallback or new operator cap.
    fn policy() -> P {
        P {
            max_inputs: None,
            max_operations: None,
            max_reason_bytes: Some(6000),
            max_title_characters: None,
            max_instruction_bytes: None,
            max_dependencies: None,
            max_criterion_bytes: None,
            worker_default_attempts: 1,
        }
    }
    /// Same operation/request pairing as the dispatcher; no concurrency grant.
    fn request(id: &str, op: &PlanningOperation, reason: &str) -> PlanningRequest {
        PlanningRequest {
            schema_version: 1,
            event_id: id.into(),
            scope: "root".into(),
            reason: reason.into(),
            operations: Some(vec![op.clone()]),
            ..Default::default()
        }
    }
    /// Operator boundary has no fabricated delivery/lease. Its unexecuted
    /// private candidate changes only judge and scope revision, leaving all
    /// received/pending inputs and consumption exactly unchanged.
    #[test]
    fn operator_cycle_actions_preserve_inbox_and_consumption() {
        let w = fixture();
        let mut values = crate::EngineSettingsValues::installation_defaults().unwrap();
        values.planning_decision_max_reason_bytes =
            Some(crate::SettingCap(Some(crate::SettingCount(6000))));
        let settings = crate::EngineSettingsDocument::initial(
            values,
            "fixture".into(),
            "2026-10-22T12:00:00Z".into(),
        )
        .unwrap();
        let r = PlanningRequest {
            schema_version: 1,
            event_id: "operator-judge".into(),
            scope: "root".into(),
            scope_revision: 1,
            judge_cycle: 1,
            judge_verdict: "continue".into(),
            reason: "é".repeat(2500),
            ..Default::default()
        };
        let first = operator_candidate(&w, &r, "judge", "2026-10-22T12:01:00Z", &settings).unwrap();
        let mut expected = w.clone();
        expected.planning.as_mut().unwrap().judge = first.planning.as_ref().unwrap().judge.clone();
        expected.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].revision = 2;
        assert!(first == expected);
        let renew = PlanningRequest {
            schema_version: 1,
            event_id: "operator-renew".into(),
            scope: "root".into(),
            scope_revision: 2,
            judge_cycle: 1,
            ..Default::default()
        };
        let second =
            operator_candidate(&first, &renew, "renew", "2026-10-22T12:02:00Z", &settings).unwrap();
        assert_eq!(
            second
                .planning
                .as_ref()
                .unwrap()
                .judge
                .as_ref()
                .unwrap()
                .cycle,
            2
        );
        assert_eq!(
            second.planning.as_ref().unwrap().scopes.as_ref().unwrap()[0].revision,
            3
        );
        assert!(second.planning.as_ref().unwrap().inbox == w.planning.as_ref().unwrap().inbox);
        assert_eq!(
            second.planning.as_ref().unwrap().activations,
            w.planning.as_ref().unwrap().activations
        );
        assert_eq!(
            second.planning.as_ref().unwrap().decisions,
            w.planning.as_ref().unwrap().decisions
        );
        for case in 0..6 {
            let mut bad = r.clone();
            let expected = match case {
                0 => {
                    bad.input_events = Some(vec!["original-not-acknowledged".into()]);
                    "planning-judge-unacknowledged-inputs"
                }
                1 => {
                    bad.operations = Some(vec![PlanningOperation::default()]);
                    "planning-judge-operator-field-not-applicable"
                }
                2 => {
                    bad.holder = "untrusted-lease-holder".into();
                    "planning-judge-operator-field-not-applicable"
                }
                3 => {
                    bad.scope = "child".into();
                    "planning-judge-root-required"
                }
                4 => {
                    bad.scope_revision = 2;
                    "revision_conflict"
                }
                _ => {
                    bad.schema_version = 0;
                    "planning-schema-required"
                }
            };
            assert_eq!(
                operator_candidate(&w, &bad, "judge", "2026-10-22T12:01:00Z", &settings)
                    .unwrap_err()
                    .code(),
                expected
            );
            assert!(w == fixture());
        }
    }
    /// Only judge/history and its new inbox signal change in this helper; outer
    /// leased decide owns decision/activation accounting and atomic publication.
    #[test]
    fn judge_continue_renew_preserve_history_and_other_state() {
        let mut w = fixture();
        let original = w.clone();
        let mut p = policy();
        let op = PlanningOperation {
            kind: "judge".into(),
            judge_cycle: 1,
            judge_verdict: "continue".into(),
            expected_candidate: "supplied-reference-not-verified".into(),
            ..Default::default()
        };
        let reason = "é".repeat(2500);
        assert_eq!(availability(&w, "root"), (true, false));
        assert_eq!(availability(&w, "child"), (false, false));
        apply(
            &mut w,
            &op,
            &request("first", &op, &reason),
            "2026-10-22T12:01:00Z",
            &p,
        )
        .unwrap();
        assert_eq!(availability(&w, "root"), (false, true));
        let mut allowed = original.clone();
        allowed.planning.as_mut().unwrap().judge = w.planning.as_ref().unwrap().judge.clone();
        allowed.planning.as_mut().unwrap().inbox = w.planning.as_ref().unwrap().inbox.clone();
        assert!(w == allowed);
        let first = w
            .planning
            .as_ref()
            .unwrap()
            .judge
            .as_ref()
            .unwrap()
            .history
            .as_ref()
            .unwrap()[0]
            .clone();
        let renew = PlanningOperation {
            kind: "renew".into(),
            judge_cycle: 1,
            ..Default::default()
        };
        apply(
            &mut w,
            &renew,
            &request("renew", &renew, "Next cycle"),
            "2026-10-22T12:02:00Z",
            &p,
        )
        .unwrap();
        let j = w.planning.as_ref().unwrap().judge.as_ref().unwrap();
        assert_eq!(j.cycle, 2);
        assert!(
            j.verdict.is_empty()
                && j.reason.is_empty()
                && j.at.is_empty()
                && j.evidence_ref.is_empty()
        );
        let mut historical = j.history.as_ref().unwrap()[0].clone();
        assert_eq!(historical.renewed_at, "2026-10-22T12:02:00Z");
        historical.renewed_at.clear();
        assert!(historical == first);
        let next = PlanningOperation {
            judge_cycle: 2,
            expected_candidate: String::new(),
            ..op
        };
        p.max_reason_bytes = Some(3);
        apply(
            &mut w,
            &next,
            &request("second", &next, "New"),
            "2026-10-22T12:03:00Z",
            &p,
        )
        .unwrap();
        assert_eq!(availability(&w, "root"), (false, false));
        crate::validate_planning_state(&w).unwrap();
        let reopened = crate::decode_work(&crate::encode_model(&w).unwrap()).unwrap();
        assert!(reopened.planning.as_ref().unwrap().judge == w.planning.as_ref().unwrap().judge);
        assert_eq!(
            w.planning.as_ref().unwrap().activations,
            original.planning.as_ref().unwrap().activations
        );
        assert_eq!(
            w.planning.as_ref().unwrap().decisions,
            original.planning.as_ref().unwrap().decisions
        );
    }
    /// Discriminating refusal variants follow a valid operation with the same
    /// pure fixture. No failure here may mutate that helper's input candidate.
    #[test]
    fn judge_refuses_wrong_cycle_fields_scope_quota_and_unverified_end() {
        let w = fixture();
        let p = policy();
        let good = PlanningOperation {
            kind: "judge".into(),
            judge_cycle: 1,
            judge_verdict: "continue".into(),
            ..Default::default()
        };
        let mut control = w.clone();
        apply(
            &mut control,
            &good,
            &request("control", &good, "Control"),
            "2026-10-22T12:01:00Z",
            &p,
        )
        .unwrap();
        for case in 0..9 {
            let mut candidate = w.clone();
            let mut op = good.clone();
            let mut policy = p;
            let expected = match case {
                0 => {
                    op.judge_cycle = 2;
                    "planning-judge-stale-cycle"
                }
                1 => {
                    op.title = "Unrelated plan field".into();
                    "planning-judge-field-not-applicable"
                }
                2 => {
                    op.judge_verdict = "end".into();
                    op.expected_candidate = "unattested".into();
                    "planning-judge-end-managed-evidence-unported"
                }
                3 => {
                    op.judge_verdict = "unknown".into();
                    "planning-judge-verdict-invalid"
                }
                4 => "planning-judge-root-required",
                5 => "planning-judge-exclusive-operation",
                6 => {
                    policy.max_reason_bytes = Some(1);
                    "planning-decision-reason-limit"
                }
                7 => {
                    op.kind = "renew".into();
                    op.judge_verdict.clear();
                    "planning-judge-continue-required"
                }
                _ => {
                    candidate = control.clone();
                    candidate
                        .planning
                        .as_mut()
                        .unwrap()
                        .judge
                        .as_mut()
                        .unwrap()
                        .max_cycles = 1;
                    op.kind = "renew".into();
                    op.judge_verdict.clear();
                    "planning-judge-cycle-budget-exhausted"
                }
            };
            let mut r = request("invalid", &op, "Control");
            if case == 4 {
                r.scope = "child".into();
            }
            if case == 5 {
                r.operations.as_mut().unwrap().push(good.clone());
            }
            let before = candidate.clone();
            assert_eq!(
                apply(&mut candidate, &op, &r, "2026-10-22T12:02:00Z", &policy)
                    .unwrap_err()
                    .code(),
                expected
            );
            assert!(candidate == before);
        }
    }
}
