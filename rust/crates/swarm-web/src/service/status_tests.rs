//! HTTP status of engine refusals that are declared port boundaries.
use super::*;

#[test]
fn planning_status_port_boundary_is_not_implemented_not_a_server_error() {
    let unported = root("read", RootStoreError::Refused("mission-status-planning-unported"));
    assert_eq!(unported.status(), StatusCode::NOT_IMPLEMENTED);
    // Witness: other read refusals keep their previous classification.
    let other = root("read", RootStoreError::Refused("mission-root-unavailable"));
    assert_ne!(other.status(), StatusCode::NOT_IMPLEMENTED);
    // The same code under another operation is not reclassified.
    assert_ne!(root("mutate", RootStoreError::Refused("mission-status-planning-unported")).status(), StatusCode::NOT_IMPLEMENTED);
}

/// Public HTTP classification of the BF-16a dialogue reader refusals (lacune C7)
/// through the real adapter the served dialogue route uses: the engine refusal
/// arrives Root-wrapped under `preparation-read` (service/preparations.rs
/// `encode` -> `root`), so the exercised contract is the Root branch of
/// `HttpServiceError::status` on `root("preparation-read",
/// RootStoreError::Refused(code))`. Input: the four stable guard codes of
/// rust/crates/swarm-engine/src/root_store/preparation_dialogue.rs, the
/// cleanup-wrapped variant of the same refusal, the same code under an
/// unrelated operation, and a reconcile-family neighbor code under the dialogue
/// operation. Behavior: a refused classified projection is a caller
/// authorization state — 403 with the stable code preserved by
/// `HttpServiceError::code`, never a server error and never a silent success —
/// while a close/cleanup failure carrying that refused primary stays 500 with
/// its own safe `root-cleanup` family and both typed causes retained through
/// the Root wrapper, so an internal fault is never disguised as a caller
/// refusal; the 500 controls witness that the operation gate and the exact
/// four-code list do not leak the dialogue contract elsewhere. Limits: pure
/// conversion regression — no Root/DB/process/provider, no real GET and no
/// end-to-end producer coverage (the guard's own refusal decisions are covered
/// by the engine dialogue tests); the guard constants are engine-crate-private,
/// so the stable public strings are asserted literally.
#[test]
fn blp_dialogue_reader_refusals_are_forbidden_and_cleanup_stays_server_error() {
    // Internal-error control first: a cleanup failure carrying the same refused
    // primary keeps its own family and status, on the real operation, and both
    // typed causes stay retained for the owning transport.
    let wrapped = root("preparation-read", RootStoreError::Cleanup {
        primary: Some(Box::new(RootStoreError::Refused("blp-dialogue-access-denied"))),
        cleanup: Box::new(RootStoreError::Refused("fixture-close-failed")),
    });
    assert_eq!(wrapped.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(wrapped.code(), "root-cleanup");
    match wrapped.source().and_then(|cause| cause.downcast_ref::<RootStoreError>()) {
        Some(RootStoreError::Cleanup { primary, cleanup }) => {
            assert_eq!(primary.as_deref().map(RootStoreError::code), Some("blp-dialogue-access-denied"));
            assert_eq!(cleanup.code(), "fixture-close-failed");
        }
        other => panic!("cleanup wrapper must retain both typed causes, got {other:?}"),
    }
    for code in [
        "blp-dialogue-authentication-required",
        "blp-dialogue-policy-required",
        "blp-dialogue-policy-stale",
        "blp-dialogue-access-denied",
    ] {
        let error = root("preparation-read", RootStoreError::Refused(code));
        assert_eq!(error.status(), StatusCode::FORBIDDEN, "{code}");
        assert_eq!(error.code(), code);
    }
    // The operation gate and the exact four-code list are real: the same
    // refusal code under an unrelated operation, and the reconcile-family
    // neighbor code under the dialogue operation, are not reclassified — they
    // keep the server-error fallback instead of borrowing the dialogue
    // contract.
    assert_eq!(root("read", RootStoreError::Refused("blp-dialogue-access-denied")).status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(root("preparation-read", RootStoreError::Refused("paea-reader-access-denied")).status(), StatusCode::INTERNAL_SERVER_ERROR);
}
