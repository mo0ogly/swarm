#![cfg(target_os = "linux")]
mod common;
use common::*;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    sync::{mpsc, Arc, Mutex},
};
use swarm_enrollment_config::{
    store::{open_private_directory, Authorizer, Context, Store},
    Error, Head,
};
use tempfile::TempDir;
use time::Duration;
fn owner() -> u32 {
    rustix::process::geteuid().as_raw()
}
fn authorizer() -> Authorizer {
    Arc::new(|_, _, _| Ok(pins().principal))
}
fn fixture() -> (TempDir, Store) {
    let root = TempDir::new().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let s = Store::open(root.path(), owner(), pins(), Some(authorizer())).unwrap();
    (root, s)
}
fn code<T>(r: Result<T, Error>, expected: &str) {
    match r {
        Ok(_) => panic!("expected {expected}"),
        Err(e) => assert_eq!(e.code(), expected, "{e}"),
    }
}
#[test]
fn store_create_preview_cas_replay_and_reopen_preserve_owned_history() {
    let (root, s) = fixture();
    let ctx = Context::new();
    let c = change("initial", Head::default());
    let t = at("2026-10-17T12:00:00Z");
    assert!(!root.path().join("config.json").exists());
    let p = s.preview(&ctx, &c, t).unwrap();
    assert!(!root.path().join("config.json").exists());
    assert!(!p.current_event_replay);
    let first = s.create(&ctx, &c, t).unwrap();
    assert_eq!(p.proposed, first);
    assert_eq!(first.snapshot, snapshot());
    assert_eq!(s.create(&ctx, &c, t + Duration::hours(1)).unwrap(), first);
    let raw = fs::read(root.path().join("config.json")).unwrap();
    let next = change("second", first.head.clone());
    let mut preview = s.preview(&ctx, &next, t + Duration::seconds(1)).unwrap();
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), raw);
    preview.proposed.snapshot.history[0].change.reason = "mutated copy".into();
    assert_eq!(s.read(&ctx, &first.head).unwrap(), first);
    let second = s.apply(&ctx, &next, t + Duration::seconds(1)).unwrap();
    let saved = fs::read(root.path().join("config.json")).unwrap();
    assert_eq!(second.snapshot.history.len(), 2);
    assert_eq!(
        s.apply(&ctx, &next, t + Duration::hours(1)).unwrap(),
        second
    );
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), saved);
    code(
        s.apply(&ctx, &change("stale", first.head.clone()), t),
        "conflict",
    );
    code(s.apply(&ctx, &c, t), "event");
    code(s.read(&ctx, &first.head), "head");
    let clone = s.clone();
    s.close().unwrap();
    code(clone.read(&ctx, &second.head), "closed");
    s.close().unwrap();
    let reopened = Store::open(root.path(), owner(), pins(), Some(authorizer())).unwrap();
    assert_eq!(reopened.read(&ctx, &second.head).unwrap(), second);
    reopened.close().unwrap();
}
#[test]
fn rejected_authority_cancellation_and_owned_callback_cannot_mutate_storage() {
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let first = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let raw = fs::read(root.path().join("config.json")).unwrap();
    s.close().unwrap();
    let denied: Authorizer = Arc::new(|_, _, _| Err(Error::Format));
    let s = Store::open(root.path(), owner(), pins(), Some(denied)).unwrap();
    let next = change("second", first.head.clone());
    let e = s.apply(&ctx, &next, t).unwrap_err();
    assert_eq!(e.code(), "unauthorized");
    assert!(std::error::Error::source(&e).is_some());
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), raw);
    s.close().unwrap();
    let wrong: Authorizer = Arc::new(|_, _, _| {
        let mut p = pins().principal;
        p.subject = "other".into();
        Ok(p)
    });
    let s = Store::open(root.path(), owner(), pins(), Some(wrong)).unwrap();
    code(s.apply(&ctx, &next, t), "unauthorized");
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), raw);
    s.close().unwrap();
    let callback: Authorizer = Arc::new(|ctx, mut current, mut c| {
        current.as_mut().unwrap().history[0].change.reason = "own copy".into();
        c.reason = "own change".into();
        ctx.cancel();
        Ok(pins().principal)
    });
    let s = Store::open(root.path(), owner(), pins(), Some(callback)).unwrap();
    code(s.apply(&ctx, &next, t), "canceled");
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), raw);
    s.close().unwrap();
    let s = Store::open(root.path(), owner(), pins(), Some(authorizer())).unwrap();
    assert_eq!(
        s.apply(&Context::new(), &next, t)
            .unwrap()
            .snapshot
            .history
            .len(),
        2
    );
    s.close().unwrap();
}
#[test]
fn independent_handles_reject_busy_then_stale_and_recover_without_wait_policy() {
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let first = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    s.close().unwrap();
    let (tx, rx) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let wait = Mutex::new(wait);
    let a: Authorizer = Arc::new(move |_, _, _| {
        tx.send(()).unwrap();
        wait.lock().unwrap().recv().unwrap();
        Ok(pins().principal)
    });
    let winner = Store::open(root.path(), owner(), pins(), Some(a)).unwrap();
    let other = Store::open(root.path(), owner(), pins(), Some(authorizer())).unwrap();
    let c = change("winner", first.head.clone());
    let clone = winner.clone();
    let thread = std::thread::spawn(move || clone.apply(&Context::new(), &c, t));
    rx.recv().unwrap();
    code(
        other.apply(&ctx, &change("loser", first.head.clone()), t),
        "busy",
    );
    code(
        winner.preview(&ctx, &change("same-handle", first.head.clone()), t),
        "busy",
    );
    let canceled = Context::new();
    canceled.cancel();
    code(
        winner.preview(&canceled, &change("canceled", first.head.clone()), t),
        "canceled",
    );
    release.send(()).unwrap();
    let second = thread.join().unwrap().unwrap();
    code(
        other.apply(&ctx, &change("loser", first.head.clone()), t),
        "conflict",
    );
    assert_eq!(
        other
            .apply(&ctx, &change("recovered", second.head), t)
            .unwrap()
            .head
            .revision,
        3
    );
    winner.close().unwrap();
    other.close().unwrap();
}
#[test]
fn paths_and_live_permissions_links_and_file_types_are_refused() {
    let (root, s) = fixture();
    s.close().unwrap();
    let alias = root.path().with_extension("symlink");
    symlink(root.path(), &alias).unwrap();
    code(open_private_directory(&alias, owner()), "path");
    fs::remove_file(alias).unwrap();
    code(
        Store::open(root.path(), owner() + 1, pins(), Some(authorizer())),
        "path",
    );
    code(
        Store::open(root.path(), owner(), pins(), None),
        "unauthorized",
    );
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    code(
        Store::open(root.path(), owner(), pins(), Some(authorizer())),
        "path",
    );
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let s = Store::open(root.path(), owner(), pins(), Some(authorizer())).unwrap();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let v = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let path = root.path().join("config.json");
    let raw = fs::read(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    code(s.read(&ctx, &v.head), "path");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(s.read(&ctx, &v.head).unwrap(), v);
    let hard = root.path().join("hardlink");
    fs::hard_link(&path, &hard).unwrap();
    code(s.read(&ctx, &v.head), "path");
    fs::remove_file(hard).unwrap();
    assert_eq!(s.read(&ctx, &v.head).unwrap(), v);
    let outside = root.path().join("outside");
    fs::write(&outside, &raw).unwrap();
    fs::remove_file(&path).unwrap();
    symlink(&outside, &path).unwrap();
    code(s.read(&ctx, &v.head), "path");
    assert_eq!(fs::read(&outside).unwrap(), raw);
    fs::remove_file(&path).unwrap();
    fs::write(&path, &raw).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(s.read(&ctx, &v.head).unwrap(), v);
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    code(s.read(&ctx, &v.head), "path");
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(s.read(&ctx, &v.head).unwrap(), v);
    s.close().unwrap();
}
#[test]
fn missing_configuration_wrong_pins_coherent_rollback_and_invalid_priority_refuse() {
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    code(s.read(&ctx, &Head::default()), "head");
    code(
        s.apply(&ctx, &change("absent", Head::default()), t),
        "not_configured",
    );
    let first = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let old = fs::read(root.path().join("config.json")).unwrap();
    let second = s.apply(&ctx, &change("second", first.head), t).unwrap();
    let current = fs::read(root.path().join("config.json")).unwrap();
    fs::write(root.path().join("config.json"), &old).unwrap();
    code(s.read(&ctx, &second.head), "head");
    fs::write(root.path().join("config.json"), &current).unwrap();
    assert_eq!(s.read(&ctx, &second.head).unwrap(), second);
    s.close().unwrap();
    let mut bad = pins();
    bad.installation_digest = "b".repeat(64);
    let s = Store::open(root.path(), owner(), bad, Some(authorizer())).unwrap();
    code(s.read(&ctx, &second.head), "pins");
    s.close().unwrap();
    let s = Store::open(root.path(), owner(), pins(), Some(authorizer())).unwrap();
    let mut c = change("invalid", second.head);
    c.schema_version = 2;
    code(s.create(&ctx, &c, t), "conflict");
    s.close().unwrap();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn faults_preserve_original_before_replace_and_exact_replay_recovers_uncertain_sync() {
    use swarm_enrollment_config::store::FixtureFault as F;
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let first = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let raw = fs::read(root.path().join("config.json")).unwrap();
    let c = change("second", first.head.clone());
    let preview = s.preview(&ctx, &c, t).unwrap();
    for fault in [
        F::Entropy,
        F::Write,
        F::PartialWrite,
        F::FileSync,
        F::FileClose,
    ] {
        s.fixture_faults(&[]);
        assert_eq!(s.read(&ctx, &first.head).unwrap(), first);
        s.fixture_faults(&[fault]);
        let e = s.apply(&ctx, &c, t).unwrap_err();
        assert_eq!(e.code(), "storage", "{e}");
        assert!(std::error::Error::source(&e).is_some());
        assert_eq!(fs::read(root.path().join("config.json")).unwrap(), raw);
        s.fixture_faults(&[]);
        assert_eq!(s.read(&ctx, &first.head).unwrap(), first);
        assert_eq!(
            fs::read_dir(root.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|p| p.file_name().to_string_lossy().starts_with(".config-"))
                .count(),
            0
        );
    }
    s.fixture_faults(&[F::DirectorySync]);
    code(s.apply(&ctx, &c, t), "indeterminate");
    s.fixture_faults(&[]);
    let observed = s.read(&ctx, &preview.proposed.head).unwrap();
    assert_eq!(observed.head.revision, 2);
    assert_eq!(s.apply(&ctx, &c, t + Duration::days(1)).unwrap(), observed);
    assert_eq!(observed.snapshot.history.len(), 2);
    s.close().unwrap();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn cleanup_unlock_read_close_and_store_close_errors_are_not_swallowed() {
    use swarm_enrollment_config::store::FixtureFault as F;
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let first = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let raw = fs::read(root.path().join("config.json")).unwrap();
    s.fixture_faults(&[F::PartialWrite, F::Unlink]);
    let e = s
        .apply(&ctx, &change("second", first.head.clone()), t)
        .unwrap_err();
    assert!(e.contains_code("storage"));
    assert!(e.to_string().contains("cleanup"));
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), raw);
    s.fixture_faults(&[]);
    assert_eq!(s.read(&ctx, &first.head).unwrap(), first);
    for fault in [F::Unlock, F::ReadClose] {
        s.fixture_faults(&[fault]);
        code(s.read(&ctx, &first.head), "storage");
        s.fixture_faults(&[]);
        assert_eq!(s.read(&ctx, &first.head).unwrap(), first);
    }
    s.fixture_faults(&[F::StoreClose]);
    code(s.close(), "storage");
    s.fixture_faults(&[]);
    s.close().unwrap();
    code(s.read(&ctx, &first.head), "closed");
}

#[test]
fn fifo_lock_symlink_writable_ancestor_and_nonutf8_paths_keep_the_actual_contract() {
    use rustix::fs::{mknodat, FileType, Mode, CWD};
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let v = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let path = root.path().join("config.json");
    let raw = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    mknodat(CWD, &path, FileType::Fifo, Mode::from_raw_mode(0o600), 0).unwrap();
    code(s.read(&ctx, &v.head), "path");
    fs::remove_file(&path).unwrap();
    fs::write(&path, &raw).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(s.read(&ctx, &v.head).unwrap(), v);
    s.close().unwrap();
    let outside = root.path().join("outside");
    fs::write(&outside, b"untouched").unwrap();
    let lock = root.path().join("config.lock");
    fs::remove_file(&lock).unwrap();
    symlink(&outside, &lock).unwrap();
    code(
        Store::open(root.path(), owner(), pins(), Some(authorizer())),
        "path",
    );
    assert_eq!(fs::read(&outside).unwrap(), b"untouched");
    fs::remove_file(&lock).unwrap();
    let child = root.path().join("private");
    fs::create_dir(&child).unwrap();
    fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
    let fd = open_private_directory(&child, owner()).unwrap();
    nix::unistd::close(fd).unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o770)).unwrap();
    code(open_private_directory(&child, owner()), "path");
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let fd = open_private_directory(&child, owner()).unwrap();
    nix::unistd::close(fd).unwrap();
    let byte_path = root.path().join(OsString::from_vec(b"raw-\xff".to_vec()));
    fs::create_dir(&byte_path).unwrap();
    fs::set_permissions(&byte_path, fs::Permissions::from_mode(0o700)).unwrap();
    let fd = open_private_directory(&byte_path, owner()).unwrap();
    nix::unistd::close(fd).unwrap();
}
#[test]
fn invalid_clock_preview_and_apply_cannot_mutate_the_saved_document() {
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let first = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let c = change("second", first.head.clone());
    let raw = fs::read(root.path().join("config.json")).unwrap();
    let invalid = time::Date::from_calendar_date(10000, time::Month::January, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    code(s.preview(&ctx, &c, invalid), "format");
    code(s.apply(&ctx, &c, invalid), "format");
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), raw);
    assert_eq!(s.apply(&ctx, &c, t).unwrap().head.revision, 2);
    s.close().unwrap();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn unwind_rejects_poisoned_state_and_close_reports_retained_release_failure() {
    use swarm_enrollment_config::store::FixtureFault;
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let first = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    s.close().unwrap();
    let panicking: Authorizer = Arc::new(|_, _, _| panic!("isolated authorizer panic fixture"));
    let s = Store::open(root.path(), owner(), pins(), Some(panicking)).unwrap();
    s.fixture_faults(&[FixtureFault::Unlock]);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        s.apply(&ctx, &change("second", first.head.clone()), t)
    }));
    assert!(panic.is_err());
    code(s.read(&ctx, &first.head), "state");
    s.fixture_faults(&[]);
    code(s.close(), "storage");
    code(s.read(&ctx, &first.head), "closed");
    let recovered = Store::open(root.path(), owner(), pins(), Some(authorizer())).unwrap();
    assert_eq!(recovered.read(&ctx, &first.head).unwrap(), first);
    recovered.close().unwrap();
}

#[test]
fn create_never_replaces_a_file_that_appears_after_the_initial_read() {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let root = TempDir::new().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("config.json");
    let target = path.clone();
    let auth: Authorizer = Arc::new(move |_, _, _| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&target)
            .unwrap();
        file.write_all(b"independent fixture writer").unwrap();
        file.sync_all().unwrap();
        nix::unistd::close(file).unwrap();
        Ok(pins().principal)
    });
    let s = Store::open(root.path(), owner(), pins(), Some(auth)).unwrap();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    code(
        s.create(&ctx, &change("initial", Head::default()), t),
        "exists",
    );
    assert_eq!(fs::read(&path).unwrap(), b"independent fixture writer");
    s.close().unwrap();
    fs::remove_file(path).unwrap();
    let s = Store::open(root.path(), owner(), pins(), Some(authorizer())).unwrap();
    assert_eq!(
        s.create(&ctx, &change("initial", Head::default()), t)
            .unwrap()
            .head
            .revision,
        1
    );
    s.close().unwrap();
}
#[test]
fn exact_replays_require_current_authorization_before_returning_saved_data() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let root = TempDir::new().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let allowed = Arc::new(AtomicBool::new(true));
    let flag = allowed.clone();
    let auth: Authorizer = Arc::new(move |_, _, _| {
        if flag.load(Ordering::SeqCst) {
            Ok(pins().principal)
        } else {
            Err(Error::Unauthorized)
        }
    });
    let s = Store::open(root.path(), owner(), pins(), Some(auth)).unwrap();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let c = change("initial", Head::default());
    let first = s.create(&ctx, &c, t).unwrap();
    let raw = fs::read(root.path().join("config.json")).unwrap();
    allowed.store(false, Ordering::SeqCst);
    code(s.create(&ctx, &c, t), "unauthorized");
    code(s.apply(&ctx, &c, t), "unauthorized");
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), raw);
    allowed.store(true, Ordering::SeqCst);
    assert_eq!(s.apply(&ctx, &c, t + Duration::hours(1)).unwrap(), first);
    s.close().unwrap();
}

#[cfg(feature = "test-fixtures")]
#[test]
fn postcommit_release_failure_and_both_error_branches_remain_observable() {
    use swarm_enrollment_config::store::FixtureFault as F;
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let first = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    let c = change("second", first.head.clone());
    let preview = s.preview(&ctx, &c, t).unwrap();
    assert_eq!(s.read(&ctx, &first.head).unwrap(), first);
    s.fixture_faults(&[F::Unlock]);
    code(s.apply(&ctx, &c, t), "storage");
    s.fixture_faults(&[]);
    let observed = s.read(&ctx, &preview.proposed.head).unwrap();
    let bytes = fs::read(root.path().join("config.json")).unwrap();
    assert_eq!(observed.head.revision, 2);
    assert_eq!(s.apply(&ctx, &c, t + Duration::hours(1)).unwrap(), observed);
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), bytes);
    let next = change("third", observed.head.clone());
    let expected = s.preview(&ctx, &next, t).unwrap();
    assert_eq!(s.read(&ctx, &observed.head).unwrap(), observed);
    s.fixture_faults(&[F::DirectorySync, F::Unlock]);
    let error = s.apply(&ctx, &next, t).unwrap_err();
    assert!(error.contains_code("indeterminate") && error.contains_code("storage"));
    assert!(error.cleanup_error().is_some());
    s.fixture_faults(&[]);
    assert_eq!(
        s.read(&ctx, &expected.proposed.head).unwrap().head.revision,
        3
    );
    assert_eq!(s.apply(&ctx, &next, t).unwrap().head.revision, 3);
    s.close().unwrap();
}

#[test]
fn external_deadline_and_cancellation_keep_the_first_reason_without_any_timer_policy() {
    let (root, s) = fixture();
    let t = at("2026-10-17T12:00:00Z");
    let active = Context::new();
    let first = s
        .create(&active, &change("initial", Head::default()), t)
        .unwrap();
    let raw = fs::read(root.path().join("config.json")).unwrap();
    let expired = Context::new();
    expired.cancel_for_deadline();
    expired.cancel();
    code(s.read(&expired, &first.head), "deadline");
    code(
        s.apply(&expired, &change("second", first.head.clone()), t),
        "deadline",
    );
    assert_eq!(fs::read(root.path().join("config.json")).unwrap(), raw);
    let canceled = Context::new();
    canceled.cancel();
    canceled.cancel_for_deadline();
    code(s.read(&canceled, &first.head), "canceled");
    assert_eq!(s.read(&active, &first.head).unwrap(), first);
    s.close().unwrap();
}

/// Replacing a lock name must not create two independent exclusion domains for
/// an existing source. Exact original inode restoration permits explicit reuse.
#[test]
fn store_replaced_lock_refuses_old_descriptor_until_exact_inode_is_restored() {
    let (root, s) = fixture();
    let ctx = Context::new();
    let t = at("2026-10-17T12:00:00Z");
    let v = s
        .create(&ctx, &change("initial", Head::default()), t)
        .unwrap();
    assert_eq!(s.read(&ctx, &v.head).unwrap(), v);
    let lock = root.path().join("config.lock");
    let held = root.path().join("held.lock");
    fs::rename(&lock, &held).unwrap();
    fs::write(&lock, b"replacement").unwrap();
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
    code(s.read(&ctx, &v.head), "path");
    fs::remove_file(&lock).unwrap();
    fs::rename(&held, &lock).unwrap();
    assert_eq!(s.read(&ctx, &v.head).unwrap(), v);
    s.close().unwrap();
}
