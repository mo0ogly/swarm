#![cfg(target_os = "linux")]
mod common;
use common::*;
use std::{fs, os::unix::fs::PermissionsExt, sync::Arc};
use swarm_enrollment_config::{
    store::{Authorizer, Context, Coordinator},
    Head,
};
use tempfile::TempDir;
fn authorizer() -> Authorizer {
    Arc::new(|_, _, _| Ok(pins().principal))
}
fn fixture() -> (TempDir, TempDir, Coordinator) {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    for p in [a.path(), b.path()] {
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let c = Coordinator::open(
        a.path(),
        b.path(),
        rustix::process::geteuid().as_raw(),
        pins(),
        Some(authorizer()),
    )
    .unwrap();
    (a, b, c)
}
#[test]
fn coordinator_preview_commit_replay_and_reopen() {
    let (a, b, c) = fixture();
    let ctx = Context::new();
    let t = snapshot().history[0].at;
    let initial = change("initial", Head::default());
    let preview = c.preview(&ctx, &initial, t).unwrap();
    assert!(!a.path().join("config.json").exists());
    assert!(!b.path().join("witness.json").exists());
    let first = c.create(&ctx, &initial, t).unwrap();
    assert_eq!(preview.proposed, first);
    assert_eq!(c.read(&ctx).unwrap(), first);
    let next = change("next", first.head);
    let second = c.apply(&ctx, &next, t).unwrap();
    let raw = fs::read(a.path().join("config.json")).unwrap();
    assert_eq!(
        c.apply(&ctx, &next, t + time::Duration::hours(1)).unwrap(),
        second
    );
    assert_eq!(fs::read(a.path().join("config.json")).unwrap(), raw);
    c.close().unwrap();
    let other = Coordinator::open(
        a.path(),
        b.path(),
        rustix::process::geteuid().as_raw(),
        pins(),
        Some(authorizer()),
    )
    .unwrap();
    assert_eq!(other.recover(&ctx).unwrap(), second);
    other.close().unwrap();
}
#[test]
fn coordinator_single_store_rollback_missing_witness_and_raw_bypass_are_refused() {
    let (a, b, c) = fixture();
    let ctx = Context::new();
    let t = snapshot().history[0].at;
    let first = c
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let cp = a.path().join("config.json");
    let wp = b.path().join("witness.json");
    let oldc = fs::read(&cp).unwrap();
    let oldw = fs::read(&wp).unwrap();
    let second = c
        .apply(&ctx, &change("next", first.head.clone()), t)
        .unwrap();
    let newc = fs::read(&cp).unwrap();
    let neww = fs::read(&wp).unwrap();
    fs::write(&wp, &oldw).unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "head");
    fs::write(&wp, &neww).unwrap();
    fs::write(&cp, &oldc).unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "head");
    assert_eq!(
        c.preview(&ctx, &change("rollback", first.head), t)
            .unwrap_err()
            .code(),
        "head"
    );
    // Explicit boundary: restoring both stores is undetectable locally.
    fs::write(&wp, &oldw).unwrap();
    assert_eq!(c.read(&ctx).unwrap().head.revision, 1);
    fs::remove_file(&wp).unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "witness_missing");
    assert_eq!(
        c.create(&ctx, &change("new-root", Head::default()), t)
            .unwrap_err()
            .code(),
        "witness_missing"
    );
    assert!(!wp.exists());
    assert_eq!(fs::read(&cp).unwrap(), oldc);
    fs::write(&wp, &neww).unwrap();
    fs::set_permissions(&wp, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&cp, &newc).unwrap();
    let raw = swarm_enrollment_config::store::Store::open(
        a.path(),
        rustix::process::geteuid().as_raw(),
        pins(),
        Some(authorizer()),
    )
    .unwrap();
    raw.apply(&ctx, &change("raw-bypass", second.head), t)
        .unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "head");
    assert_eq!(c.recover(&ctx).unwrap_err().code(), "head");
    raw.close().unwrap();
    c.close().unwrap();
}
#[test]
fn coordinator_authorization_cancellation_paths_and_witness_tampering() {
    use std::os::unix::fs::symlink;
    use swarm_enrollment_config::Error;
    let (a, b, c) = fixture();
    let ctx = Context::new();
    let t = snapshot().history[0].at;
    let first = c
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let wp = b.path().join("witness.json");
    let raw = fs::read(&wp).unwrap();
    let next = change("next", first.head);
    let denied = Coordinator::open(
        a.path(),
        b.path(),
        rustix::process::geteuid().as_raw(),
        pins(),
        Some(Arc::new(|_, _, _| Err(Error::Format))),
    )
    .unwrap();
    assert_eq!(
        denied.apply(&ctx, &next, t).unwrap_err().code(),
        "unauthorized"
    );
    assert_eq!(fs::read(&wp).unwrap(), raw);
    denied.close().unwrap();
    let canceled = Context::new();
    canceled.cancel();
    assert_eq!(c.read(&canceled).unwrap_err().code(), "canceled");
    for path in [a.path().to_path_buf(), a.path().join("nested")] {
        assert_eq!(
            Coordinator::open(
                a.path(),
                &path,
                rustix::process::geteuid().as_raw(),
                pins(),
                Some(authorizer())
            )
            .unwrap_err()
            .code(),
            "path"
        );
    }
    let mut otherpins = pins();
    otherpins.installation_digest = "b".repeat(64);
    let wrong = Coordinator::open(
        a.path(),
        b.path(),
        rustix::process::geteuid().as_raw(),
        otherpins,
        Some(authorizer()),
    )
    .unwrap();
    // Config pins fail first; a valid but different witness is checked below.
    assert_eq!(wrong.read(&ctx).unwrap_err().code(), "pins");
    wrong.close().unwrap();
    // Preserve typed canonical key order by substituting only the same-length digest.
    let altered =
        String::from_utf8(raw.clone())
            .unwrap()
            .replacen(&"a".repeat(64), &"b".repeat(64), 1);
    fs::write(&wp, altered).unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "pins");
    fs::write(&wp, &raw).unwrap();
    fs::set_permissions(&wp, fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "path");
    fs::set_permissions(&wp, fs::Permissions::from_mode(0o600)).unwrap();
    fs::hard_link(&wp, b.path().join("alias")).unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "path");
    fs::remove_file(b.path().join("alias")).unwrap();
    let external = b.path().join("external");
    fs::rename(&wp, &external).unwrap();
    symlink(&external, &wp).unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "path");
    fs::remove_file(&wp).unwrap();
    fs::rename(&external, &wp).unwrap();
    fs::write(&wp, b"{}").unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "version");
    fs::write(&wp, b"{\"schema_version\":\"private-secret\"}").unwrap();
    let e = c.read(&ctx).unwrap_err();
    assert_eq!(e.code(), "witness");
    assert!(!format!("{e:?} {e}").contains("private-secret"));
    assert!(std::error::Error::source(&e).is_some());
    fs::write(&wp, &raw).unwrap();
    assert_eq!(c.read(&ctx).unwrap().head.revision, 1);
    c.close().unwrap();
}
#[test]
fn coordinator_reserves_both_handles_and_releases_first_when_second_busy() {
    use std::sync::mpsc;
    use swarm_enrollment_config::store::Store;
    let (a, b, c) = fixture();
    let ctx = Context::new();
    let t = snapshot().history[0].at;
    let first = c
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let (entered_rx_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    let auth: Authorizer = Arc::new(move |_, _, _| {
        entered_rx_tx.send(()).unwrap();
        release_rx.lock().unwrap().recv().unwrap();
        Ok(pins().principal)
    });
    let w = Store::open(
        b.path(),
        rustix::process::geteuid().as_raw(),
        pins(),
        Some(auth),
    )
    .unwrap();
    let join = std::thread::spawn(move || {
        let r = w.preview(&Context::new(), &change("held", Head::default()), t);
        w.close().unwrap();
        r.unwrap();
    });
    entered.recv().unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "busy");
    let independent = Store::open(
        a.path(),
        rustix::process::geteuid().as_raw(),
        pins(),
        Some(authorizer()),
    )
    .unwrap();
    // First lock must have been released after failing to reserve the witness.
    assert_eq!(independent.read(&ctx, &first.head).unwrap(), first);
    independent.close().unwrap();
    release.send(()).unwrap();
    join.join().unwrap();
    assert_eq!(c.read(&ctx).unwrap(), first);
    c.close().unwrap();
}
#[test]
fn coordinator_preflights_both_lexical_paths_before_creating_locks() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    for p in [a.path(), b.path()] {
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let owner = rustix::process::geteuid().as_raw();
    let valid = Coordinator::open(a.path(), b.path(), owner, pins(), Some(authorizer())).unwrap();
    valid.close().unwrap();
    for p in [a.path(), b.path()] {
        fs::remove_file(p.join("config.lock")).unwrap();
    }
    let invalid = b.path().join(".");
    assert_eq!(
        Coordinator::open(a.path(), &invalid, owner, pins(), Some(authorizer()))
            .unwrap_err()
            .code(),
        "path"
    );
    assert!(!a.path().join("config.lock").exists());
    assert!(!b.path().join("config.lock").exists());
}
#[cfg(feature = "test-fixtures")]
#[test]
fn coordinator_each_interrupted_phase_recovers_once_with_original_time() {
    use swarm_enrollment_config::store::{FixtureFault as F, FixturePhase};
    let t = snapshot().history[0].at;
    for phase in [
        "intent",
        "config-before",
        "config-after",
        "confirm-before",
        "confirm-after",
    ] {
        let (a, b, c) = fixture();
        let ctx = Context::new();
        c.create(&ctx, &change("initial", Head::default()), t)
            .unwrap(); // valid control first
        let base = c.read(&ctx).unwrap();
        let next = change("next", base.head);
        match phase {
            "intent" => c.fixture_store_faults(&[], &[F::DirectorySync]),
            "config-before" => c.fixture_store_faults(&[F::PartialWrite], &[]),
            "config-after" => c.fixture_store_faults(&[F::DirectorySync], &[]),
            "confirm-before" | "confirm-after" => c.fixture_hook(Some(Arc::new(move |p, _, w| {
                if p == FixturePhase::Confirming {
                    w.fixture_faults(&[if phase == "confirm-before" {
                        F::PartialWrite
                    } else {
                        F::DirectorySync
                    }]);
                }
            }))),
            _ => unreachable!(),
        }
        let e = c
            .apply(&ctx, &next, t + time::Duration::seconds(1))
            .unwrap_err();
        assert_eq!(
            e.code(),
            if phase.ends_with("before") {
                "storage"
            } else {
                "indeterminate"
            }
        );
        assert!(std::error::Error::source(&e).is_some());
        if phase == "intent" {
            assert!(e.to_string().contains("prepare witness"));
        }
        if phase == "confirm-after" {
            assert_eq!(c.read(&ctx).unwrap_err().code(), "indeterminate");
        } else {
            assert_eq!(c.read(&ctx).unwrap_err().code(), "pending");
        }
        let cp = fs::read(a.path().join("config.json")).unwrap();
        let wp = fs::read(b.path().join("witness.json")).unwrap();
        c.fixture_hook(None);
        c.fixture_store_faults(&[], &[]);
        c.close().unwrap();
        if phase != "confirm-after" {
            let denied = Coordinator::open(
                a.path(),
                b.path(),
                rustix::process::geteuid().as_raw(),
                pins(),
                Some(Arc::new(|_, _, _| {
                    Err(swarm_enrollment_config::Error::Format)
                })),
            )
            .unwrap();
            assert_eq!(denied.recover(&ctx).unwrap_err().code(), "unauthorized");
            assert_eq!(fs::read(a.path().join("config.json")).unwrap(), cp);
            assert_eq!(fs::read(b.path().join("witness.json")).unwrap(), wp);
            denied.close().unwrap();
        }
        let fresh = Coordinator::open(
            a.path(),
            b.path(),
            rustix::process::geteuid().as_raw(),
            pins(),
            Some(authorizer()),
        )
        .unwrap();
        if phase != "confirm-after" {
            assert_eq!(fresh.preview(&ctx, &next, t).unwrap_err().code(), "pending");
            assert_eq!(
                fresh
                    .apply(&ctx, &change("unrelated", Head::default()), t)
                    .unwrap_err()
                    .code(),
                "pending"
            );
        }
        let v = fresh
            .apply(&ctx, &next, t + time::Duration::hours(1))
            .unwrap();
        assert_eq!(v.head.revision, 2);
        assert_eq!(v.snapshot.history.len(), 2);
        assert_eq!(v.snapshot.history[1].at, t + time::Duration::seconds(1));
        assert_eq!(fresh.recover(&ctx).unwrap(), v);
        fresh.close().unwrap();
    }
}
#[cfg(feature = "test-fixtures")]
#[test]
fn coordinator_canceled_after_intent_requires_fresh_authorized_recovery() {
    use swarm_enrollment_config::store::FixturePhase;
    let (a, _b, c) = fixture();
    let ctx = Context::new();
    let t = snapshot().history[0].at;
    let cancel = ctx.clone();
    c.fixture_hook(Some(Arc::new(move |p, _, _| {
        if p == FixturePhase::Prepared {
            cancel.cancel()
        }
    })));
    let e = c
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap_err();
    assert!(e.contains_code("pending") && e.contains_code("canceled"));
    assert!(
        e.cleanup_error().is_none(),
        "cancellation is a pending cause, not cleanup"
    );
    assert!(!a.path().join("config.json").exists());
    c.fixture_hook(None);
    let fresh = Context::new();
    assert_eq!(c.read(&fresh).unwrap_err().code(), "pending");
    assert_eq!(c.recover(&fresh).unwrap().head.revision, 1);
    c.close().unwrap();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn coordinator_recovery_rejects_unrelated_history_and_missing_committed_config() {
    use swarm_enrollment_config::{decode, encode, store::FixtureFault};
    let (a, _b, c) = fixture();
    let ctx = Context::new();
    let t = snapshot().history[0].at;
    let first = c
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    c.fixture_store_faults(&[FixtureFault::DirectorySync], &[]);
    assert_eq!(
        c.apply(&ctx, &change("next", first.head), t)
            .unwrap_err()
            .code(),
        "indeterminate"
    );
    c.fixture_store_faults(&[], &[]);
    let cp = a.path().join("config.json");
    let raw = fs::read(&cp).unwrap();
    let mut altered = decode(&raw).unwrap();
    altered.history[1].change.reason = "different coherent history".into();
    fs::write(&cp, encode(&altered).unwrap()).unwrap();
    assert_eq!(c.recover(&ctx).unwrap_err().code(), "head");
    fs::remove_file(&cp).unwrap();
    assert_eq!(c.recover(&ctx).unwrap_err().code(), "not_configured");
    fs::write(&cp, raw).unwrap();
    fs::set_permissions(&cp, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(c.recover(&ctx).unwrap().head.revision, 2);
    c.close().unwrap();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn coordinator_unlock_and_close_failures_preserve_both_causes_after_commit() {
    use swarm_enrollment_config::store::FixtureFault as F;
    let (_a, _b, c) = fixture();
    let ctx = Context::new();
    let t = snapshot().history[0].at;
    let initial = change("initial", Head::default());
    c.fixture_store_faults(&[F::Unlock], &[F::Unlock]);
    let e = c.create(&ctx, &initial, t).unwrap_err();
    assert_eq!(e.code(), "storage");
    assert!(e.cleanup_error().is_some());
    c.fixture_store_faults(&[], &[]);
    assert_eq!(c.apply(&ctx, &initial, t).unwrap().head.revision, 1);
    c.fixture_store_faults(&[F::StoreClose], &[F::StoreClose]);
    let e = c.close().unwrap_err();
    assert!(e.cleanup_error().is_some());
    c.close().unwrap();
    assert_eq!(c.read(&ctx).unwrap_err().code(), "closed");
}
