#![cfg(target_os = "linux")]
//! Typed outcome/observer matrix. Receipts here are explicit fixture inputs,
//! not actual SQL commits; public CLI/HTTP tests prove those separately.
use std::io;
use swarm_engine::{AuxiliaryEffect, AuxiliaryEffectState, SessionVisit};
use swarm_web::{HttpServiceError, NativeJobOutcome, NativeServerError};
/// Construct one labelled fixture receipt, with no production identity claim.
fn effect(state: AuxiliaryEffectState) -> AuxiliaryEffect {
    AuxiliaryEffect::SessionVisit {
        state,
        work_id: "fixture-work".into(),
        visit: SessionVisit {
            revision: 7,
            at: "fixture-time".into(),
            operator: "fixture-operator".into(),
        },
    }
}
/// A retained receipt must never manufacture Work/config/agent approval.
#[test]
fn known_and_uncertain_receipts_survive_real_observer_io_refusal() {
    for state in [
        AuxiliaryEffectState::Recorded,
        AuxiliaryEffectState::Uncertain,
    ] {
        let receipt = effect(state);
        let failure = HttpServiceError::AuxiliaryState {
            primary: Box::new(HttpServiceError::Refused("fixture-primary")),
            effect: receipt.clone(),
        };
        assert_eq!(failure.auxiliary_effects(), vec![receipt.clone()]);
        let outcome = NativeJobOutcome::from_error(failure);
        let record = serde_json::to_value(outcome.record()).unwrap();
        assert_eq!(record["auxiliary_effects"][0]["kind"], "session_visit");
        assert!(record["recorded_work"].is_null());
        assert!(record["recorded_preparation"].is_null());
        assert_eq!(record["configuration_may_be_recorded"], false);
        let mut calls = 0;
        let refused = outcome
            .observe(&mut |_| {
                calls += 1;
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "owned observer refused",
                ))
            })
            .unwrap_err();
        assert_eq!(calls, 1);
        assert_eq!(refused.pending.len(), 1);
        assert_eq!(refused.auxiliary_effects(), vec![receipt.clone()]);
        let server = NativeServerError::SessionState {
            primary: Box::new(NativeServerError::Cleanup {
                primary: Box::new(NativeServerError::Observation(Box::new(refused))),
                cleanup: Box::new(NativeServerError::Refused("fixture-cleanup")),
            }),
        };
        assert_eq!(server.auxiliary_effects(), vec![receipt]);
    }
}
#[test]
fn failed_outcome_encoding_retains_original_auxiliary_receipt() {
    let receipt = effect(AuxiliaryEffectState::Recorded);
    let outcome = NativeJobOutcome::from_error(HttpServiceError::AuxiliaryState {
        primary: Box::new(HttpServiceError::Refused("fixture-primary")),
        effect: receipt.clone(),
    });
    // Actual serde error value is injected as a transport encoding cause. This
    // tests retention, not a claim that serializing this DTO can itself fail.
    let cause = serde_json::from_slice::<serde_json::Value>(b"{").unwrap_err();
    let server = NativeServerError::OutcomeEncoding {
        cause,
        outcome: Box::new(outcome),
    };
    assert_eq!(server.auxiliary_effects(), vec![receipt]);
}

/// Same original serde error type has distinct ownership: invalid received
/// document versus failed server response. Neither creates an auxiliary effect.
#[test]
fn received_json_stays_client_error_and_server_json_stays_server_error() {
    let input = NativeJobOutcome::from_error(HttpServiceError::Json(
        serde_json::from_slice::<serde_json::Value>(b"{").unwrap_err(),
    ));
    assert_eq!(input.status, http::StatusCode::BAD_REQUEST);
    assert!(input.auxiliary_effects.is_empty());
    let server = NativeJobOutcome::from_error(HttpServiceError::ResponseJson(
        serde_json::from_slice::<serde_json::Value>(b"{").unwrap_err(),
    ));
    assert_eq!(server.status, http::StatusCode::INTERNAL_SERVER_ERROR);
    assert!(server.auxiliary_effects.is_empty());
    assert_eq!(server.failure.unwrap().code(), "web-response-encoding");
}
