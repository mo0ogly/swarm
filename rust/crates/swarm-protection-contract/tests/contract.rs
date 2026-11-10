//! PA-01 local contract regressions. No effect, launch, confinement or E2/E3 claim.
use serde_json::{json, Value};
use std::error::Error as _;
use swarm_protection_contract::{
    canonicalize, decode_context, decode_proposal, Action, ApprovalBinding, EffectReceipt, Event,
    State,
};

fn vectors() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/fixtures/paea/contract-v1.json"
    )))
    .unwrap()["vectors"]
        .as_array()
        .unwrap()
        .clone()
}
fn control(
    index: usize,
) -> (
    swarm_protection_contract::Proposal,
    swarm_protection_contract::AdmissionContext,
) {
    let v = &vectors()[index];
    (
        decode_proposal(&serde_json::to_vec(&v["proposal"]).unwrap()).unwrap(),
        decode_context(&serde_json::to_vec(&v["context"]).unwrap()).unwrap(),
    )
}

#[test]
fn both_domains_match_precomputed_python_bytes_and_digests() {
    let fixtures = vectors();
    assert_eq!(fixtures.len(), 14);
    for v in fixtures {
        let p = decode_proposal(&serde_json::to_vec(&v["proposal"]).unwrap()).unwrap();
        let c = decode_context(&serde_json::to_vec(&v["context"]).unwrap()).unwrap();
        let actual = canonicalize(&p, &c).unwrap();
        assert_eq!(
            std::str::from_utf8(actual.bytes()).unwrap(),
            v["canonical"].as_str().unwrap(),
            "{}",
            v["name"]
        );
        assert_eq!(actual.digest(), v["action_digest"].as_str().unwrap());
        assert_eq!(actual.intent_id(), v["intent_id"].as_str().unwrap());
    }
}

#[test]
fn new_transport_keys_never_change_action_or_business_identity() {
    for (a, b) in [(0, 1), (8, 9)] {
        let (pa, ca) = control(a);
        let (pb, cb) = control(b);
        assert_ne!(pa.request_id, pb.request_id);
        assert_eq!(
            canonicalize(&pa, &ca).unwrap(),
            canonicalize(&pb, &cb).unwrap()
        );
    }
}

#[test]
fn changed_content_changes_action_but_retains_business_intent() {
    for (a, b) in [(0, 2), (0, 3), (0, 4), (0, 5), (8, 10)] {
        let (pa, ca) = control(a);
        let (pb, cb) = control(b);
        let (a, b) = (
            canonicalize(&pa, &ca).unwrap(),
            canonicalize(&pb, &cb).unwrap(),
        );
        assert_ne!(a.digest(), b.digest());
        assert_eq!(a.intent_id(), b.intent_id());
    }
}

#[test]
fn different_project_or_git_base_defines_distinct_intent() {
    for (a, b) in [(0, 6), (8, 11)] {
        let (pa, ca) = control(a);
        let (pb, cb) = control(b);
        assert_ne!(
            canonicalize(&pa, &ca).unwrap().intent_id(),
            canonicalize(&pb, &cb).unwrap().intent_id()
        );
    }
}

#[test]
fn exact_approval_rejects_changed_content_and_policy() {
    let (p, c) = control(0);
    let original = canonicalize(&p, &c).unwrap();
    let approval = ApprovalBinding {
        schema_version: 1,
        action_digest: original.digest().into(),
        policy_digest: c.policy_digest.clone(),
        approver_id: "approver-1".into(),
    };
    assert!(original.matches_approval(&approval).unwrap());
    for i in [2, 3, 4, 5] {
        let (p, c) = control(i);
        assert!(!canonicalize(&p, &c)
            .unwrap()
            .matches_approval(&approval)
            .unwrap());
    }
    let mut invalid = approval.clone();
    invalid.approver_id.clear();
    assert!(original.matches_approval(&invalid).is_err());
}

#[test]
fn duplicate_keys_are_refused_at_top_level_and_nested_depths() {
    let (p, c) = control(0);
    let valid = serde_json::to_string(&p).unwrap();
    assert!(decode_proposal(valid.as_bytes()).is_ok());
    for bad in [
        valid.replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"schema_version\":1",
            1,
        ),
        valid.replacen(
            "\"currency\":\"EUR\"",
            "\"currency\":\"EUR\",\"currency\":\"USD\"",
            1,
        ),
    ] {
        let error = decode_proposal(bad.as_bytes()).unwrap_err();
        assert_eq!(error.to_string(), "protection-document-invalid");
        assert!(error.source().is_some());
    }
    let raw = serde_json::to_string(&c).unwrap();
    let bad = raw.replace("\"integrity\":0", "\"integrity\":0,\"integrity\":1");
    assert_ne!(raw, bad);
    assert!(decode_context(bad.as_bytes()).is_err());
}

#[test]
fn unknown_authority_claims_nulls_missing_and_trailing_data_refuse() {
    let v = &vectors()[0];
    for key in [
        "actor",
        "integrity",
        "policy_revision",
        "approved",
        "capability",
    ] {
        let mut p = v["proposal"].clone();
        p[key] = json!("invented");
        assert!(decode_proposal(&serde_json::to_vec(&p).unwrap()).is_err());
    }
    for bad in [
        b"{}".as_slice(),
        b"null",
        b"[]",
        b"{} {}",
        b"{\"schema_version\":1,\"request_id\":null,\"action\":{}}",
    ] {
        assert!(decode_proposal(bad).is_err());
    }
    let mut p = v["proposal"].clone();
    p["action"]["unknown"] = json!(true);
    assert!(decode_proposal(&serde_json::to_vec(&p).unwrap()).is_err());
}

#[test]
fn raw_numbers_unicode_and_json_encodings_do_not_alias_silently() {
    let raw = serde_json::to_string(&vectors()[0]["proposal"]).unwrap();
    for bad in [
        raw.replacen("\"schema_version\":1", "\"schema_version\":1.0", 1),
        raw.replacen("\"schema_version\":1", "\"schema_version\":1e0", 1),
        raw.replace("invoice-17", "invoice-\\ud800"),
        format!("{raw} false"),
    ] {
        assert!(decode_proposal(bad.as_bytes()).is_err());
    }
    assert!(decode_proposal(b"\xff").is_err());
    let escaped = raw.replace("invoice-17", "invoice-\\u0031\\u0037");
    assert_eq!(
        decode_proposal(raw.as_bytes()).unwrap(),
        decode_proposal(escaped.as_bytes()).unwrap()
    );
}

#[test]
fn money_is_exact_and_ambiguous_or_out_of_range_amounts_refuse() {
    let mut p = vectors()[0]["proposal"].clone();
    for amount in [
        "0",
        "-1",
        "+1",
        "01",
        "1.0",
        "1e3",
        "1,000",
        " 1",
        "1 ",
        "9223372036854775808",
        "١",
    ] {
        p["action"]["amount_minor"] = json!(amount);
        assert!(
            decode_proposal(&serde_json::to_vec(&p).unwrap()).is_err(),
            "{amount}"
        );
    }
    p["action"]["amount_minor"] = json!(420000);
    assert!(decode_proposal(&serde_json::to_vec(&p).unwrap()).is_err());
    let (large, context) = control(7);
    assert!(
        std::str::from_utf8(canonicalize(&large, &context).unwrap().bytes())
            .unwrap()
            .contains("\"amount_minor\":\"9223372036854775807\"")
    );
}

#[test]
fn source_labels_are_sets_but_context_source_order_is_preserved() {
    let (p, mut c) = control(0);
    let original = c.clone();
    let a = canonicalize(&p, &c).unwrap();
    assert_eq!(c, original);
    c.sources[0].confidentiality_labels.reverse();
    assert_eq!(a, canonicalize(&p, &c).unwrap());
    let mut second = c.sources[0].clone();
    second.source_id = "document-2".into();
    c.sources.push(second);
    let b = canonicalize(&p, &c).unwrap();
    c.sources.reverse();
    assert_ne!(b.digest(), canonicalize(&p, &c).unwrap().digest());
}

#[test]
fn duplicate_source_keys_labels_and_invalid_claim_shapes_refuse() {
    let (_, c) = control(0);
    let mut bad = c.clone();
    bad.sources.push(bad.sources[0].clone());
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.sources[0]
        .confidentiality_labels
        .push("internal".into());
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.sources[0].integrity = 4;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.policy_revision = 0;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.policy_digest = "A".repeat(64);
    assert!(bad.validate().is_err());
    let mut bad = c;
    bad.project_id = "../other".into();
    assert!(bad.validate().is_err());
}

#[test]
fn git_ref_revision_expressions_aliases_mixed_oids_and_noops_refuse() {
    let mut p = vectors()[8]["proposal"].clone();
    for target in [
        "HEAD",
        "refs/tags/v1",
        "refs/heads/../main",
        "refs/heads/.hidden",
        "refs/heads/a.lock",
        "refs/heads/a//b",
        "refs/heads/a@{1}",
        "refs/heads/a b",
        "refs/heads/main.",
    ] {
        p["action"]["target_ref"] = json!(target);
        assert!(
            decode_proposal(&serde_json::to_vec(&p).unwrap()).is_err(),
            "{target}"
        );
    }
    p["action"]["target_ref"] = json!("refs/heads/candidate");
    for oid in [
        "HEAD~1".into(),
        "0".repeat(40),
        "A".repeat(40),
        "3".repeat(64),
        "1".repeat(40),
    ] {
        p["action"]["candidate_oid"] = json!(oid);
        assert!(decode_proposal(&serde_json::to_vec(&p).unwrap()).is_err());
    }
}

#[test]
fn allowed_lifecycle_and_both_reconciliation_observations() {
    let mut s = State::Proposed;
    for event in [
        Event::Admit,
        Event::RequireApproval,
        Event::Authorize,
        Event::RequestCommit,
    ] {
        s = s.transition(event).unwrap();
    }
    assert_eq!(s, State::CommitRequested);
    assert_eq!(
        s.transition(Event::ObserveCommitted).unwrap(),
        State::Committed
    );
    let unknown = s.transition(Event::ObserveUnknown).unwrap();
    assert_eq!(unknown, State::Unknown);
    assert_eq!(
        unknown.transition(Event::ReconcileCommitted).unwrap(),
        State::ReconciledCommitted
    );
    assert_eq!(
        unknown.transition(Event::ReconcileNotCommitted).unwrap(),
        State::ReconciledNotCommitted
    );
}

#[test]
fn unknown_never_becomes_denial_retry_or_unobserved_success() {
    for e in [
        Event::Admit,
        Event::RequireApproval,
        Event::Authorize,
        Event::RequestCommit,
        Event::ObserveCommitted,
        Event::ObserveUnknown,
        Event::Deny,
    ] {
        assert!(State::Unknown.transition(e).is_err());
    }
    assert!(State::Authorized
        .transition(Event::ObserveCommitted)
        .is_err());
    assert!(State::ApprovalPending
        .transition(Event::RequestCommit)
        .is_err());
    assert!(State::Proposed.transition(Event::Authorize).is_err());
}

#[test]
fn terminal_states_have_no_mutating_transition() {
    for state in [
        State::Committed,
        State::Denied,
        State::ReconciledCommitted,
        State::ReconciledNotCommitted,
    ] {
        for e in [
            Event::Admit,
            Event::RequireApproval,
            Event::Authorize,
            Event::RequestCommit,
            Event::ObserveCommitted,
            Event::ObserveUnknown,
            Event::Deny,
            Event::ReconcileCommitted,
            Event::ReconcileNotCommitted,
        ] {
            assert!(state.transition(e).is_err());
        }
    }
}

#[test]
fn receipt_shape_does_not_accept_approval_as_an_effect() {
    let mut receipt = EffectReceipt {
        schema_version: 1,
        action_digest: "a".repeat(64),
        intent_id: "b".repeat(64),
        sink_id: "fixture-sink".into(),
        outcome: State::Unknown,
        observation_digest: "c".repeat(64),
    };
    assert!(receipt.validate().is_ok());
    receipt.outcome = State::Authorized;
    assert!(receipt.validate().is_err());
    receipt.outcome = State::Denied;
    assert!(receipt.validate().is_ok());
    receipt.observation_digest.clear();
    assert!(receipt.validate().is_err());
}

#[test]
fn direct_struct_construction_cannot_skip_validation() {
    let (mut p, c) = control(0);
    if let Action::Payment { amount_minor, .. } = &mut p.action {
        *amount_minor = "-4".into();
    }
    assert!(canonicalize(&p, &c).is_err());
}
