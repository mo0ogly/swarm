#![cfg(target_os = "linux")]
use ed25519_dalek::SigningKey;
use swarm_enrollment_authority::{
    self as a, authenticated_control::Context as ExecuteContext, authenticated_startup, control,
    personal as p,
};
use swarm_enrollment_state::{sign_change, Context};
use time::OffsetDateTime;
#[path = "common/authenticated_fixture.rs"]
mod fixture;
use a::paea_action::{Operation as POperation, Request as PRequest, Service};
use fixture::Fixture;

/// Pure protocol test: exact domain and digest representation, no issued grant.
#[test]
fn paea_permission_is_separate_exact_and_rejects_invalid_digests() {
    let root = "ab".repeat(32);
    let action = "cd".repeat(32);
    let r = PRequest::new(
        &root,
        Service::FictitiousPayment,
        POperation::Commit,
        &action,
    )
    .unwrap();
    assert_eq!(r.permission().operation, "effect.execute");
    assert_eq!(
        r.permission().resource,
        format!("paea-action-v1/{root}/payment.settle/commit/{action}")
    );
    assert_ne!(
        r.permission(),
        a::trust_action::Request::new(&root, a::trust_action::Operation::Commit, &action)
            .unwrap()
            .permission()
    );
    for invalid in ["", "abcdef", &"AB".repeat(32), &"g0".repeat(32)] {
        assert!(PRequest::new(
            invalid,
            Service::FictitiousPayment,
            POperation::Commit,
            &action
        )
        .is_err());
        assert!(PRequest::new(
            &root,
            Service::FictitiousPayment,
            POperation::Commit,
            invalid
        )
        .is_err());
    }
}

/// Real signed fixture grant and password before neighboring service/action refusals.
#[test]
fn paea_exact_current_grant_refuses_neighbors_then_recovers() {
    let root = "ab".repeat(32);
    let action = "cd".repeat(32);
    let r = PRequest::new(
        &root,
        Service::FictitiousPayment,
        POperation::Commit,
        &action,
    )
    .unwrap();
    let f = Fixture::build_with_action(true, Some(r.permission()));
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let check = || {
        app.with_paea_action(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            &r,
            |id| {
                assert_eq!(id.principal().id, "operator");
                assert_eq!(id.principal().kind, "human");
                assert_eq!(id.enrollment_id(), "business-grant");
                assert_eq!(id.authority_head(), &head);
                assert_eq!(id.trust_revision(), 2);
                assert_eq!(id.request().root_digest(), root);
                assert_eq!(id.request().action_digest(), action);
                assert_eq!(id.request().service(), Service::FictitiousPayment);
                assert_eq!(id.request().operation(), POperation::Commit);
                assert!(id.expires_at() > OffsetDateTime::now_utc());
                Ok::<_, ()>("known-action")
            },
        )
        .unwrap()
    };
    let good = check();
    assert!(good.authority.is_ok());
    assert_eq!(good.action, Some(Ok("known-action")));
    for other in [
        PRequest::new(
            &"ef".repeat(32),
            Service::FictitiousPayment,
            POperation::Commit,
            &action,
        )
        .unwrap(),
        PRequest::new(
            &root,
            Service::FictitiousPayment,
            POperation::Commit,
            &"ef".repeat(32),
        )
        .unwrap(),
        PRequest::new(&root, Service::GitPromotion, POperation::Commit, &action).unwrap(),
        PRequest::new(
            &root,
            Service::FictitiousPayment,
            POperation::Reconcile,
            &action,
        )
        .unwrap(),
    ] {
        let refused = app
            .with_paea_action(
                &ExecuteContext::new(),
                &head,
                b"STARTUP_FIXTURE_PASSWORD",
                &other,
                |_| -> Result<(), ()> { panic!("neighbor capability used") },
            )
            .unwrap();
        assert!(matches!(refused.authority, Err(control::Error::Permission)));
        assert!(refused.action.is_none());
    }
    let good = check();
    assert!(good.authority.is_ok());
    assert_eq!(good.action, Some(Ok("known-action")));
    app.close().unwrap();
    f.close();
}

/// Signed permissions cannot cross namespaces even with identical digest inputs.
#[test]
fn paea_and_trust_permissions_never_substitute_each_other() {
    let root = "ab".repeat(32);
    let action = "cd".repeat(32);
    let paea = PRequest::new(
        &root,
        Service::FictitiousPayment,
        POperation::Commit,
        &action,
    )
    .unwrap();
    let trust =
        a::trust_action::Request::new(&root, a::trust_action::Operation::Commit, &action).unwrap();
    for is_paea in [true, false] {
        let f = Fixture::build_with_action(
            true,
            Some(if is_paea {
                paea.permission()
            } else {
                trust.permission()
            }),
        );
        let (app, head) =
            authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
        if is_paea {
            let good = app
                .with_paea_action(
                    &ExecuteContext::new(),
                    &head,
                    b"STARTUP_FIXTURE_PASSWORD",
                    &paea,
                    |_| Ok::<_, ()>(7),
                )
                .unwrap();
            assert!(good.authority.is_ok());
            assert_eq!(good.action, Some(Ok(7)));
            let bad = app
                .with_trust_action(
                    &ExecuteContext::new(),
                    &head,
                    b"STARTUP_FIXTURE_PASSWORD",
                    &trust,
                    |_| -> Result<(), ()> { panic!("PAEA granted Trust") },
                )
                .unwrap();
            assert!(matches!(bad.authority, Err(control::Error::Permission)));
            assert!(bad.action.is_none());
        } else {
            let good = app
                .with_trust_action(
                    &ExecuteContext::new(),
                    &head,
                    b"STARTUP_FIXTURE_PASSWORD",
                    &trust,
                    |_| Ok::<_, ()>(7),
                )
                .unwrap();
            assert!(good.authority.is_ok());
            assert_eq!(good.action, Some(Ok(7)));
            let bad = app
                .with_paea_action(
                    &ExecuteContext::new(),
                    &head,
                    b"STARTUP_FIXTURE_PASSWORD",
                    &paea,
                    |_| -> Result<(), ()> { panic!("Trust granted PAEA") },
                )
                .unwrap();
            assert!(matches!(bad.authority, Err(control::Error::Permission)));
            assert!(bad.action.is_none());
        }
        app.close().unwrap();
        f.close();
    }
}

/// A consumer refusal and a password refusal remain different from a known effect.
#[test]
fn paea_keeps_callback_failure_and_password_refusal() {
    let r = PRequest::new(
        &"ab".repeat(32),
        Service::GitPromotion,
        POperation::Commit,
        &"cd".repeat(32),
    )
    .unwrap();
    let f = Fixture::build_with_action(true, Some(r.permission()));
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let good = app
        .with_paea_action(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            &r,
            |_| Err::<(), _>("consumer-refused"),
        )
        .unwrap();
    assert!(good.authority.is_ok());
    assert_eq!(good.action, Some(Err("consumer-refused")));
    assert!(app
        .with_paea_action(
            &ExecuteContext::new(),
            &head,
            b"WRONG_PASSWORD",
            &r,
            |_| -> Result<(), ()> { panic!("wrong password used") }
        )
        .is_err());
    let good = app
        .with_paea_action(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            &r,
            |_| Ok::<_, ()>(7),
        )
        .unwrap();
    assert!(good.authority.is_ok());
    assert_eq!(good.action, Some(Ok(7)));
    app.close().unwrap();
    f.close();
}

/// Completion failures after use retain both the callback and authority outcome.
#[test]
fn paea_after_use_cancellation_retains_known_action() {
    let r = PRequest::new(
        &"ab".repeat(32),
        Service::GitPromotion,
        POperation::Reconcile,
        &"cd".repeat(32),
    )
    .unwrap();
    let f = Fixture::build_with_action(true, Some(r.permission()));
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let ctx = ExecuteContext::new();
    let control = app
        .with_paea_action(&ctx, &head, b"STARTUP_FIXTURE_PASSWORD", &r, |_| {
            Ok::<_, ()>("valid-control")
        })
        .unwrap();
    assert!(control.authority.is_ok());
    assert_eq!(control.action, Some(Ok("valid-control")));
    let result = app.with_paea_action(&ctx, &head, b"STARTUP_FIXTURE_PASSWORD", &r, |_| {
        ctx.cancel();
        Ok::<_, ()>("observed-effect")
    });
    match result {
        Err(p::coordinator::UseError::AfterUse {
            outcome: Ok(report),
            ..
        }) => {
            assert_eq!(report.action, Some(Ok("observed-effect")));
            assert!(report.authority.is_err());
        }
        _ => panic!("after-use action or failure lost"),
    }
    app.close().unwrap();
    f.close();
}

/// Completion failures after use retain both the callback and authority outcome.
#[test]
fn paea_after_use_cancellation_retains_callback_failure() {
    let r = PRequest::new(
        &"ab".repeat(32),
        Service::GitPromotion,
        POperation::Reconcile,
        &"cd".repeat(32),
    )
    .unwrap();
    let f = Fixture::build_with_action(true, Some(r.permission()));
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let ctx = ExecuteContext::new();
    let control = app
        .with_paea_action(&ctx, &head, b"STARTUP_FIXTURE_PASSWORD", &r, |_| {
            Ok::<_, ()>("valid-control")
        })
        .unwrap();
    assert!(control.authority.is_ok());
    assert_eq!(control.action, Some(Ok("valid-control")));
    let result = app.with_paea_action(&ctx, &head, b"STARTUP_FIXTURE_PASSWORD", &r, |_| {
        ctx.cancel();
        Err::<(), _>("effect-outcome-uncertain")
    });
    match result {
        Err(p::coordinator::UseError::AfterUse {
            outcome: Ok(report),
            ..
        }) => {
            assert_eq!(report.action, Some(Err("effect-outcome-uncertain")));
            assert!(report.authority.is_err());
        }
        _ => panic!("after-use action or failure lost"),
    }
    app.close().unwrap();
    f.close();
}

/// Current business revocation is reserved during use, effective on the next use.
#[test]
fn paea_holds_authority_then_refuses_revoked_use() {
    let r = PRequest::new(
        &"ab".repeat(32),
        Service::FictitiousPayment,
        POperation::Commit,
        &"cd".repeat(32),
    )
    .unwrap();
    let f = Fixture::build_with_action(true, Some(r.permission()));
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
    let good = app
        .with_paea_action(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            &r,
            |_| {
                assert!(f
                    .business
                    .apply(&Context::new(), &head, &raw)
                    .unwrap_err()
                    .is_busy());
                Ok::<_, ()>("control-used")
            },
        )
        .unwrap();
    assert!(good.authority.is_ok());
    assert_eq!(good.action, Some(Ok("control-used")));
    let revoked = f.business.apply(&Context::new(), &head, &raw).unwrap();
    let refused = app
        .with_paea_action(
            &ExecuteContext::new(),
            &revoked,
            b"STARTUP_FIXTURE_PASSWORD",
            &r,
            |_| -> Result<(), ()> { panic!("revoked permission consumed") },
        )
        .unwrap();
    assert!(refused.authority.is_err());
    assert!(refused.action.is_none());
    app.close().unwrap();
    f.close();
}

/// Stale CAS head cannot invoke a callback; the unchanged current head recovers.
#[test]
fn paea_stale_head_refuses_and_current_head_recovers() {
    let r = PRequest::new(
        &"ab".repeat(32),
        Service::FictitiousPayment,
        POperation::Commit,
        &"cd".repeat(32),
    )
    .unwrap();
    let f = Fixture::build_with_action(true, Some(r.permission()));
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let good = app
        .with_paea_action(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            &r,
            |_| Ok::<_, ()>(1),
        )
        .unwrap();
    assert!(good.authority.is_ok());
    assert_eq!(good.action, Some(Ok(1)));
    let mut stale = head.clone();
    stale.sequence += 1;
    let bad = app
        .with_paea_action(
            &ExecuteContext::new(),
            &stale,
            b"STARTUP_FIXTURE_PASSWORD",
            &r,
            |_| -> Result<(), ()> { panic!("stale head used") },
        )
        .unwrap();
    assert!(bad.authority.is_err());
    assert!(bad.action.is_none());
    let good = app
        .with_paea_action(
            &ExecuteContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
            &r,
            |_| Ok::<_, ()>(2),
        )
        .unwrap();
    assert!(good.authority.is_ok());
    assert_eq!(good.action, Some(Ok(2)));
    app.close().unwrap();
    f.close();
}
