//! Receipt correspondence regressions, written but pending authorized compilation.
use serde_json::{json, Value};
use swarm_protection_contract::{
    canonicalize, check_receipt_transition, decode_document, decode_receipt, CanonicalAction,
    EffectReceipt, State, SCHEMA_VERSION,
};

/// Recompute the canonical action from a frozen document, not its claimed digest.
fn action(index: usize) -> CanonicalAction {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea/contract-v1.json"
    ))
    .expect("fixture JSON");
    let vector = &fixture["vectors"][index];
    let raw =
        serde_json::to_vec(&json!({"proposal":vector["proposal"],"context":vector["context"]}))
            .expect("fixture serialization");
    let document = decode_document(&raw).expect("valid fixture document");
    canonicalize(&document.proposal, &document.context).expect("canonical fixture")
}

/// Build explicitly unauthenticated test data; no service or signature is involved.
fn receipt(action: &CanonicalAction, outcome: State) -> EffectReceipt {
    EffectReceipt {
        schema_version: SCHEMA_VERSION,
        action_digest: action.digest().to_owned(),
        intent_id: action.intent_id().to_owned(),
        sink_id: "sink-1".into(),
        outcome,
        observation_digest: "d".repeat(64),
    }
}

#[test]
fn exact_receipt_correspondence_in_both_domains_establishes_no_authenticity() {
    for index in [0, 8] {
        let action = action(index);
        for (previous, outcome) in [
            (State::CommitRequested, State::Committed),
            (State::CommitRequested, State::Unknown),
            (State::Proposed, State::Denied),
            (State::Admitted, State::Denied),
            (State::ApprovalPending, State::Denied),
            (State::Authorized, State::Denied),
            (State::CommitRequested, State::Denied),
            (State::Unknown, State::ReconciledCommitted),
            (State::Unknown, State::ReconciledNotCommitted),
        ] {
            // Even this forged matching fixture passes correspondence only.
            let original = receipt(&action, outcome);
            let encoded = serde_json::to_vec(&original).unwrap();
            let decoded = decode_receipt(&encoded).unwrap();
            assert_eq!(
                check_receipt_transition(&action, &decoded, "sink-1", previous).unwrap(),
                outcome
            );
            assert_eq!(decoded, original);
        }
        let mut forged = receipt(&action, State::Committed);
        forged.observation_digest = "0".repeat(64);
        // Representation-valid observation bytes are not evidence of an effect.
        assert!(
            check_receipt_transition(&action, &forged, "sink-1", State::CommitRequested).is_ok()
        );
        let alternative_transport = action_for_new_transport(index);
        assert_eq!(action.digest(), alternative_transport.digest());
        assert!(check_receipt_transition(
            &alternative_transport,
            &forged,
            "sink-1",
            State::CommitRequested
        )
        .is_ok());
    }
}

#[test]
fn altered_action_intent_or_sink_refuses_with_control_and_recovery() {
    for index in [0, 8] {
        let action = action(index);
        let original = receipt(&action, State::Committed);
        let mut bad_action = original.clone();
        bad_action.action_digest = "e".repeat(64);
        let mut bad_intent = original.clone();
        bad_intent.intent_id = "f".repeat(64);
        let mut bad_sink = original.clone();
        bad_sink.sink_id = "sink-private-CANARY".into();
        for bad in [bad_action, bad_intent, bad_sink] {
            assert!(
                check_receipt_transition(&action, &original, "sink-1", State::CommitRequested)
                    .is_ok()
            );
            let before = bad.clone();
            let error = check_receipt_transition(&action, &bad, "sink-1", State::CommitRequested)
                .unwrap_err();
            assert_eq!(error.code(), "protection-receipt-binding");
            assert!(!error.to_string().contains("CANARY"));
            assert_eq!(bad, before);
            assert!(
                check_receipt_transition(&action, &original, "sink-1", State::CommitRequested)
                    .is_ok()
            );
        }
        let changed = action_for_same_intent(index);
        assert_eq!(action.intent_id(), changed.intent_id());
        assert!(
            check_receipt_transition(&action, &original, "sink-1", State::CommitRequested).is_ok()
        );
        assert_eq!(
            check_receipt_transition(&changed, &original, "sink-1", State::CommitRequested)
                .unwrap_err()
                .code(),
            "protection-receipt-binding"
        );
        assert!(
            check_receipt_transition(&action, &original, "sink-1", State::CommitRequested).is_ok()
        );
    }
}

/// Select a different amount or Git candidate sharing the same business intent.
fn action_for_same_intent(index: usize) -> CanonicalAction {
    action(if index == 0 { 2 } else { 10 })
}

/// The transport key is excluded from canonical identity, by the existing contract.
fn action_for_new_transport(index: usize) -> CanonicalAction {
    action(if index == 0 { 1 } else { 9 })
}

#[test]
fn unknown_and_terminal_states_cannot_be_relabelled_or_replayed() {
    let action = action(0);
    let reconciliation = receipt(&action, State::ReconciledCommitted);
    for outcome in [State::Denied, State::Committed, State::Unknown] {
        assert!(
            check_receipt_transition(&action, &reconciliation, "sink-1", State::Unknown).is_ok()
        );
        let bad = receipt(&action, outcome);
        assert_eq!(
            check_receipt_transition(&action, &bad, "sink-1", State::Unknown)
                .unwrap_err()
                .code(),
            "protection-transition-invalid"
        );
        assert!(
            check_receipt_transition(&action, &reconciliation, "sink-1", State::Unknown).is_ok()
        );
    }
    for terminal in [
        State::Committed,
        State::Denied,
        State::ReconciledCommitted,
        State::ReconciledNotCommitted,
    ] {
        let original = receipt(&action, terminal);
        let prior = match terminal {
            State::Committed => State::CommitRequested,
            State::Denied => State::Authorized,
            _ => State::Unknown,
        };
        assert!(check_receipt_transition(&action, &original, "sink-1", prior).is_ok());
        // Valid receipt representation and binding are not permission to replay.
        assert!(original.validate().is_ok());
        assert_eq!(
            check_receipt_transition(&action, &original, "sink-1", terminal)
                .unwrap_err()
                .code(),
            "protection-transition-invalid"
        );
        assert!(check_receipt_transition(&action, &original, "sink-1", prior).is_ok());
    }
}

#[test]
fn invalid_representation_or_expected_sink_never_yields_a_state() {
    let action = action(8);
    let original = receipt(&action, State::Committed);
    let mut invalid = original.clone();
    invalid.outcome = State::Authorized;
    assert!(check_receipt_transition(&action, &original, "sink-1", State::CommitRequested).is_ok());
    assert_eq!(
        check_receipt_transition(&action, &invalid, "sink-1", State::CommitRequested)
            .unwrap_err()
            .code(),
        "protection-effect-receipt"
    );
    assert!(check_receipt_transition(&action, &original, "sink-1", State::CommitRequested).is_ok());
    assert_eq!(
        check_receipt_transition(&action, &original, "", State::CommitRequested)
            .unwrap_err()
            .code(),
        "protection-receipt-expected-sink"
    );
    assert!(check_receipt_transition(&action, &original, "sink-1", State::CommitRequested).is_ok());
    for expected_sink in ["sink-2", "SINK-1"] {
        assert!(
            check_receipt_transition(&action, &original, "sink-1", State::CommitRequested).is_ok()
        );
        assert_eq!(
            check_receipt_transition(&action, &original, expected_sink, State::CommitRequested)
                .unwrap_err()
                .code(),
            "protection-receipt-binding"
        );
        assert!(
            check_receipt_transition(&action, &original, "sink-1", State::CommitRequested).is_ok()
        );
    }
}
