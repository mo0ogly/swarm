#![cfg(target_os = "linux")]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_enrollment_authority::{
    self as a,
    authenticated_control::Context as ExecuteContext,
    authenticated_startup,
    control::{self, Request, Response},
    personal as p,
};
use swarm_enrollment_bootstrap::{
    self as b, AuthenticatedDocument, ControlDocument, Document, PersonalDeclaration,
};
use swarm_enrollment_config::{
    store::documents::Documents, Change, Head as ConfigHead, Pins, Values,
};
use swarm_enrollment_state::{sign_change, Context, Head, Journal};
use swarm_identity_contract::{Principal, Scope};
use time::{Duration, OffsetDateTime};
#[path = "common/authenticated_fixture.rs"]
mod fixture;
use fixture::{change, mode, Fixture};

#[test]
fn authenticated_startup_pinned_real_sources_execute_once_and_reopen() {
    let f = Fixture::new();
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    assert_eq!(head, f.doc.control.authority_head);
    let first = app
        .execute(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            Request::Create(change()),
        )
        .unwrap()
        .unwrap();
    assert!(matches!(first, Response::View(_)));
    app.close().unwrap();
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    match app
        .execute(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            Request::Create(change()),
        )
        .unwrap()
        .unwrap()
    {
        Response::View(v) => assert_eq!(v.snapshot.history.len(), 1),
        _ => panic!("wrong response"),
    };
    app.close().unwrap();
    f.close();
}
#[test]
fn authenticated_startup_wrong_pin_and_personal_domain_refuse_then_recover_without_configuration() {
    let mut f = Fixture::new();
    assert!(authenticated_startup::open(
        &Context::new(),
        f.path.to_str().unwrap(),
        &"ef".repeat(32)
    )
    .is_err());
    let original = f.doc.clone();
    f.doc.personal.project_context_digest = "44".repeat(32);
    f.save();
    assert!(
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).is_err()
    );
    assert!(
        !std::path::Path::new(&f.doc.control.installation.configuration_directory)
            .join("config.json")
            .exists()
    );
    f.doc = original;
    f.save();
    let (app, _) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    app.close().unwrap();
    f.close();
}
#[test]
fn authenticated_startup_partial_business_open_failure_releases_personal_owner_for_recovery() {
    let mut f = Fixture::new();
    let original = f.doc.clone();
    f.doc.control.project_context_digest = "22".repeat(32);
    f.save();
    assert!(
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).is_err()
    );
    f.doc = original;
    f.save();
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    assert!(app
        .execute(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            Request::Create(change())
        )
        .unwrap()
        .is_ok());
    app.close().unwrap();
    f.close();
}

#[cfg(feature = "test-fixtures")]
#[test]
fn authenticated_startup_primary_business_refusal_and_actual_personal_close_fault_both_survive() {
    let mut f = Fixture::new();
    let original = f.doc.clone();
    let (app, _) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    app.close().unwrap();
    f.doc.control.project_context_digest = "22".repeat(32);
    f.save();
    let control =
        match authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin) {
            Err(e) => e,
            Ok((app, _)) => {
                app.close().unwrap();
                panic!("expected business context refusal")
            }
        };
    assert_eq!(control.code(), "context");
    assert!(control.coordinator_cleanup().is_none());
    let e = match authenticated_startup::open_with_document_close_fault(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
    ) {
        Err(e) => e,
        Ok((app, _)) => {
            app.close().unwrap();
            panic!("expected business refusal")
        }
    };
    assert_eq!(e.code(), "context");
    assert!(std::error::Error::source(&e).is_some());
    assert!(e.coordinator_cleanup().is_some());
    assert!(!format!("{e:?}").contains("STARTUP_FIXTURE_PASSWORD"));
    assert!(
        !std::path::Path::new(&f.doc.control.installation.configuration_directory)
            .join("config.json")
            .exists()
    );
    f.doc = original;
    f.save();
    let (app, _) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    app.close().unwrap();
    f.close();
}

/// Write one known disposable fixture seed, never on reopen or from a request.
fn fixture_signer(f: &Fixture) -> PathBuf {
    let p = f.path.with_file_name("personal.seed");
    assert!(!p.exists());
    fs::write(&p, [41; 32]).unwrap();
    mode(&p, 0o600);
    p
}
/// Assert actual bootstrap still active at the independently known initial head.
fn unchanged_bootstrap(f: &Fixture) {
    f.personal
        .with_active(
            &Context::new(),
            &f.doc.personal.bootstrap_head,
            p::BOOTSTRAP_ID,
            |_| Ok(()),
        )
        .unwrap();
    assert!(
        !PathBuf::from(&f.doc.control.installation.configuration_directory)
            .join("config.json")
            .exists()
    );
}
#[test]
fn personal_lifecycle_cold_enrollment_exactly_once_and_reopen_execute() {
    let f = Fixture::cold();
    let seed = fixture_signer(&f);
    let ctx = ExecuteContext::new();
    let app = authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    unchanged_bootstrap(&f);
    assert!(app.recover(&ctx).is_err());
    app.initialize(&ctx).unwrap();
    let checkpoint =
        PathBuf::from(&f.doc.personal.checkpoint_directory).join("personal-checkpoint.json");
    let initial = fs::read(&checkpoint).unwrap();
    let bad = app
        .enroll(&ctx, &[17; 32], b"NEW_FIXTURE_PASSWORD")
        .unwrap_err();
    assert_eq!(bad.cause.code(), "installation-proof");
    assert_eq!(fs::read(&checkpoint).unwrap(), initial);
    unchanged_bootstrap(&f);
    let a = app
        .enroll(&ctx, &[42; 32], b"NEW_FIXTURE_PASSWORD")
        .unwrap();
    assert_eq!(a.head().sequence, 3);
    let enrolled = fs::read(&checkpoint).unwrap();
    assert!(app.enroll(&ctx, &[42; 32], b"OTHER").is_err());
    assert_eq!(fs::read(&checkpoint).unwrap(), enrolled);
    assert!(app.execute(&ctx, b"WRONG", Request::Read).is_err());
    match app
        .execute(&ctx, b"NEW_FIXTURE_PASSWORD", Request::Create(change()))
        .unwrap()
        .unwrap()
    {
        Response::View(v) => assert_eq!(v.snapshot.history.len(), 1),
        _ => panic!("missing actual configuration"),
    }
    app.close().unwrap();
    let app = authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    assert_eq!(app.recover(&ctx).unwrap().head(), a.head());
    assert!(app.initialize(&ctx).is_err());
    match app
        .execute(&ctx, b"NEW_FIXTURE_PASSWORD", Request::Create(change()))
        .unwrap()
        .unwrap()
    {
        Response::View(v) => assert_eq!(v.snapshot.history.len(), 1),
        _ => panic!("replay duplicated effect"),
    }
    app.close().unwrap();
    f.close();
}
#[test]
fn personal_lifecycle_private_key_refusal_precedes_source_locks_and_recovers() {
    let control = Fixture::cold();
    let controlseed = fixture_signer(&control);
    let valid = authenticated_startup::open_lifecycle(
        &Context::new(),
        control.path.to_str().unwrap(),
        &control.pin,
        &controlseed,
    )
    .unwrap();
    valid.close().unwrap();
    control.close();
    let f = Fixture::cold();
    let seed = fixture_signer(&f);
    for path in [
        PathBuf::from(&f.doc.personal.checkpoint_directory).join("private-documents.lock"),
        PathBuf::from(&f.doc.control.installation.configuration_directory).join("config.lock"),
    ] {
        assert!(!path.exists());
    }
    fs::write(&seed, [19; 32]).unwrap();
    let e = match authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    ) {
        Err(e) => e,
        Ok(app) => {
            app.close().unwrap();
            panic!("wrong key accepted")
        }
    };
    assert!(matches!(e, authenticated_startup::Error::Signing(_)));
    assert!(std::error::Error::source(&e).is_some());
    assert!(!format!("{e:?} {e}").contains(seed.to_str().unwrap()));
    assert!(!PathBuf::from(&f.doc.personal.checkpoint_directory)
        .join("private-documents.lock")
        .exists());
    unchanged_bootstrap(&f);
    fs::write(&seed, [41; 32]).unwrap();
    let app = authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    app.initialize(&ExecuteContext::new()).unwrap();
    app.close().unwrap();
    f.close();
}
#[test]
fn personal_lifecycle_cancellation_refuses_without_initializing_and_fresh_context_recovers() {
    let f = Fixture::cold();
    let seed = fixture_signer(&f);
    let app = authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    let ctx = ExecuteContext::new();
    ctx.cancel();
    assert!(app.initialize(&ctx).is_err());
    assert!(app.recover(&ctx).is_err());
    assert!(!PathBuf::from(&f.doc.personal.checkpoint_directory)
        .join("personal-checkpoint.json")
        .exists());
    unchanged_bootstrap(&f);
    let fresh = ExecuteContext::new();
    app.initialize(&fresh).unwrap();
    let before = fs::read(
        PathBuf::from(&f.doc.personal.checkpoint_directory).join("personal-checkpoint.json"),
    )
    .unwrap();
    assert!(app
        .enroll(&ctx, &[42; 32], b"NEW_FIXTURE_PASSWORD")
        .is_err());
    unchanged_bootstrap(&f);
    assert_eq!(
        fs::read(
            PathBuf::from(&f.doc.personal.checkpoint_directory).join("personal-checkpoint.json")
        )
        .unwrap(),
        before
    );
    assert_eq!(
        app.enroll(&fresh, &[42; 32], b"NEW_FIXTURE_PASSWORD")
            .unwrap()
            .head()
            .sequence,
        3
    );
    app.close().unwrap();
    f.close();
}
#[test]
fn personal_lifecycle_stale_initial_head_never_adopts_current_journal() {
    let mut f = Fixture::cold();
    let seed = fixture_signer(&f);
    let original = f.doc.clone();
    let app = authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    app.close().unwrap();
    f.doc.personal.bootstrap_head.digest = "aa".repeat(32);
    f.save();
    let app = authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    assert!(app.initialize(&ExecuteContext::new()).is_err());
    assert!(!PathBuf::from(&f.doc.personal.checkpoint_directory)
        .join("personal-checkpoint.json")
        .exists());
    app.close().unwrap();
    f.doc = original;
    f.save();
    unchanged_bootstrap(&f);
    let app = authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    app.initialize(&ExecuteContext::new()).unwrap();
    app.close().unwrap();
    f.close();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn personal_lifecycle_exact_pending_recovers_after_each_durable_phase() {
    use p::coordinator::{Error as CoordinatorError, FixturePhase};
    use std::sync::Arc;
    for phase in [
        FixturePhase::IntentDurable,
        FixturePhase::JournalApplied,
        FixturePhase::CheckpointDurable,
    ] {
        // Same preconditions before each fault, valid full lifecycle control.
        let control = Fixture::cold();
        let seed = fixture_signer(&control);
        let app = authenticated_startup::open_lifecycle(
            &Context::new(),
            control.path.to_str().unwrap(),
            &control.pin,
            &seed,
        )
        .unwrap();
        let ctx = ExecuteContext::new();
        app.initialize(&ctx).unwrap();
        assert_eq!(
            app.enroll(&ctx, &[42; 32], b"NEW_FIXTURE_PASSWORD")
                .unwrap()
                .head()
                .sequence,
            3
        );
        app.close().unwrap();
        control.close();
        let f = Fixture::cold();
        let seed = fixture_signer(&f);
        let app = authenticated_startup::open_lifecycle(
            &Context::new(),
            f.path.to_str().unwrap(),
            &f.pin,
            &seed,
        )
        .unwrap();
        app.initialize(&ctx).unwrap();
        app.fixture_hook(Some(Arc::new(move |p| {
            if p == phase {
                Err(CoordinatorError::Internal)
            } else {
                Ok(())
            }
        })));
        let e = app
            .enroll(&ctx, &[42; 32], b"NEW_FIXTURE_PASSWORD")
            .unwrap_err();
        assert_eq!(e.cause.code(), "personal-coordinator-state");
        app.fixture_hook(None);
        app.close().unwrap();
        let app = authenticated_startup::open_lifecycle(
            &Context::new(),
            f.path.to_str().unwrap(),
            &f.pin,
            &seed,
        )
        .unwrap();
        let recovered = app.recover(&ctx).unwrap();
        assert_eq!(recovered.head().sequence, 3);
        assert_eq!(app.recover(&ctx).unwrap().head(), recovered.head());
        f.personal
            .with_active(&Context::new(), recovered.head(), p::ACCOUNT_ID, |_| Ok(()))
            .unwrap();
        assert!(app.enroll(&ctx, &[42; 32], b"OTHER").is_err());
        app.execute(&ctx, b"NEW_FIXTURE_PASSWORD", Request::Create(change()))
            .unwrap()
            .unwrap();
        app.close().unwrap();
        f.close();
    }
}

#[cfg(feature = "test-fixtures")]
#[test]
fn personal_lifecycle_consuming_close_keeps_actual_error_and_known_effect_on_reopen() {
    let f = Fixture::cold();
    let seed = fixture_signer(&f);
    let ctx = ExecuteContext::new();
    let baseline = authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    baseline.close().unwrap();
    let app = authenticated_startup::open_lifecycle_with_document_close_fault(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    app.initialize(&ctx).unwrap();
    app.enroll(&ctx, &[42; 32], b"NEW_FIXTURE_PASSWORD")
        .unwrap();
    app.execute(&ctx, b"NEW_FIXTURE_PASSWORD", Request::Create(change()))
        .unwrap()
        .unwrap();
    let e = app.close().unwrap_err();
    assert!(e.personal.is_some());
    assert!(e.control.is_none());
    assert!(std::error::Error::source(&e).is_some());
    assert!(!format!("{e:?} {e}").contains("NEW_FIXTURE_PASSWORD"));
    let app = authenticated_startup::open_lifecycle(
        &Context::new(),
        f.path.to_str().unwrap(),
        &f.pin,
        &seed,
    )
    .unwrap();
    assert_eq!(app.recover(&ctx).unwrap().head().sequence, 3);
    match app
        .execute(&ctx, b"NEW_FIXTURE_PASSWORD", Request::Read)
        .unwrap()
        .unwrap()
    {
        Response::View(v) => assert_eq!(v.snapshot.history.len(), 1),
        _ => panic!("known configuration lost"),
    };
    app.close().unwrap();
    f.close();
}

/// A real current account and signed exact action grant precede refusal cases.
#[test]
fn authenticated_trust_action_uses_current_principal_and_exact_grant() {
    use a::trust_action::{Operation, Request as ActionRequest};
    let request =
        ActionRequest::new(&"ab".repeat(32), Operation::Approve, &"cd".repeat(32)).unwrap();
    let f = Fixture::build_with_action(true, Some(request.permission()));
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let result = app
        .with_trust_action(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            &request,
            |identity| {
                assert_eq!(identity.principal().id, "operator");
                assert_eq!(identity.principal().kind, "human");
                assert_eq!(identity.trust_revision(), 2);
                assert_eq!(identity.authority_head(), &head);
                assert_eq!(identity.enrollment_id(), "business-grant");
                Ok::<_, ()>("recorded-action")
            },
        )
        .unwrap();
    assert!(result.authority.is_ok());
    assert_eq!(result.action, Some(Ok("recorded-action")));
    for other in [
        ActionRequest::new(&"ab".repeat(32), Operation::Approve, &"ef".repeat(32)).unwrap(),
        ActionRequest::new(&"ef".repeat(32), Operation::Approve, &"cd".repeat(32)).unwrap(),
        ActionRequest::new(&"ab".repeat(32), Operation::Commit, &"cd".repeat(32)).unwrap(),
    ] {
        let refused = app
            .with_trust_action(
                &ExecuteContext::new(),
                &head,
                b"STARTUP_FIXTURE_PASSWORD",
                &other,
                |_| -> Result<(), ()> { panic!("wrong action invoked") },
            )
            .unwrap();
        assert!(refused.authority.is_err());
        assert!(refused.action.is_none());
    }
    let refused_business = app
        .with_trust_action(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            &request,
            |_| Err::<(), _>("business-refused"),
        )
        .unwrap();
    assert!(refused_business.authority.is_ok());
    assert_eq!(refused_business.action, Some(Err("business-refused")));
    assert!(app
        .with_trust_action(
            &ExecuteContext::new(),
            &head,
            b"WRONG_PASSWORD",
            &request,
            |_| -> Result<(), ()> { panic!("wrong account invoked") }
        )
        .is_err());
    app.close().unwrap();
    f.close();
}

/// Cancellation after actual callback use must preserve that complete outcome.
#[test]
fn authenticated_trust_action_retains_outcome_after_cancellation() {
    use a::trust_action::{Operation, Request as ActionRequest};
    let request =
        ActionRequest::new(&"ab".repeat(32), Operation::Approve, &"cd".repeat(32)).unwrap();
    let f = Fixture::build_with_action(true, Some(request.permission()));
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let ctx = ExecuteContext::new();
    let result = app.with_trust_action(&ctx, &head, b"STARTUP_FIXTURE_PASSWORD", &request, |_| {
        ctx.cancel();
        Ok::<_, ()>(42)
    });
    match result {
        Err(p::coordinator::UseError::AfterUse {
            outcome: Ok(report),
            ..
        }) => {
            assert_eq!(report.action, Some(Ok(42)));
            assert!(report.authority.is_err());
        }
        _ => panic!("lost after-use outcome"),
    }
    app.close().unwrap();
    f.close();
}

/// The identical signed revocation is Busy during use, then succeeds afterwards;
/// a new operation must observe revocation and never invoke the callback.
#[test]
fn authenticated_trust_action_holds_and_rechecks_revocation() {
    use a::trust_action::{Operation, Request as ActionRequest};
    let request =
        ActionRequest::new(&"ab".repeat(32), Operation::Approve, &"cd".repeat(32)).unwrap();
    let f = Fixture::build_with_action(true, Some(request.permission()));
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let raw = sign_change(
        &SigningKey::from_bytes(&[7; 32]),
        [11; 32],
        &head,
        "revoke",
        "business-grant",
        &[],
    )
    .unwrap();
    let result = app
        .with_trust_action(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            &request,
            |_| {
                assert!(f.business.apply(&Context::new(), &head, &raw).is_err());
                Ok::<_, ()>("used")
            },
        )
        .unwrap();
    assert!(result.authority.is_ok());
    assert_eq!(result.action, Some(Ok("used")));
    let revoked = f.business.apply(&Context::new(), &head, &raw).unwrap();
    let result = app
        .with_trust_action(
            &ExecuteContext::new(),
            &revoked,
            b"STARTUP_FIXTURE_PASSWORD",
            &request,
            |_| -> Result<(), ()> { panic!("revoked callback") },
        )
        .unwrap();
    assert!(result.authority.is_err());
    assert!(result.action.is_none());
    app.close().unwrap();
    f.close();
}
