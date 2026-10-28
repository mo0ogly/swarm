#![cfg(target_os = "linux")]
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
};
use swarm_enrollment_config::{
    store::{
        documents::{Documents, Publication},
        Context,
    },
    Error,
};
use tempfile::TempDir;
/// Isolated existing private directory; no live service data is used.
fn fixture() -> (TempDir, Documents) {
    let root = TempDir::new().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let d = Documents::open(root.path(), rustix::process::geteuid().as_raw()).unwrap();
    (root, d)
}
/// Assert the actual error category without requiring a secret-bearing success Debug.
fn code<T>(r: Result<T, Error>, expected: &str) {
    match r {
        Ok(_) => panic!("expected {expected}"),
        Err(e) => assert_eq!(e.code(), expected, "{e}"),
    }
}
#[test]
fn private_documents_exact_create_cas_replace_sync_and_reopen_without_defaults() {
    let (root, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    code(r.read("checkpoint.json"), "not_configured");
    assert_eq!(
        r.create("checkpoint.json", b"original").unwrap(),
        Publication::Durable
    );
    assert_eq!(r.read("checkpoint.json").unwrap(), b"original");
    assert_eq!(
        r.create("checkpoint.json", b"other")
            .unwrap_err()
            .cause
            .code(),
        "exists"
    );
    let e = r
        .replace("checkpoint.json", b"stale", b"other")
        .unwrap_err();
    assert_eq!(e.cause.code(), "conflict");
    assert_eq!(e.publication, Publication::NotAttempted);
    assert_eq!(r.read("checkpoint.json").unwrap(), b"original");
    assert_eq!(
        r.replace("checkpoint.json", b"original", b"next").unwrap(),
        Publication::Durable
    );
    r.sync().unwrap();
    r.finish().unwrap();
    d.close().unwrap();
    let d = Documents::open(root.path(), rustix::process::geteuid().as_raw()).unwrap();
    let r = d.reserve(&ctx).unwrap();
    assert_eq!(r.read("checkpoint.json").unwrap(), b"next");
    r.finish().unwrap();
    d.close().unwrap();
    assert_eq!(
        fs::metadata(root.path().join("checkpoint.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
}
#[test]
fn private_documents_reject_invalid_names_before_io_and_never_reset_missing_state() {
    let (root, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("valid.json", b"witness").unwrap();
    for name in [
        "",
        ".",
        "..",
        "../escape",
        "/absolute",
        "child/file",
        "nul\0file",
        ".config-secret",
        "private-documents.lock",
    ] {
        code(r.read(name), "path");
        assert_eq!(
            r.create(name, b"bad").unwrap_err().publication,
            Publication::NotAttempted
        );
        assert_eq!(
            r.replace(name, b"witness", b"bad")
                .unwrap_err()
                .cause
                .code(),
            "path"
        );
    }
    assert_eq!(
        r.replace("absent.json", b"", b"bad")
            .unwrap_err()
            .cause
            .code(),
        "not_configured"
    );
    assert!(!root.path().join("absent.json").exists());
    assert_eq!(r.read("valid.json").unwrap(), b"witness");
    r.finish().unwrap();
    d.close().unwrap();
}
#[test]
fn private_documents_reservations_are_immediate_and_retained_directory_survives_path_replacement() {
    let (root, d) = fixture();
    let ctx = Context::new();
    let other = Documents::open(root.path(), rustix::process::geteuid().as_raw()).unwrap();
    let r = d.reserve(&ctx).unwrap();
    r.create("checkpoint", b"bound").unwrap();
    code(d.reserve(&ctx), "busy");
    code(other.reserve(&ctx), "busy");
    code(d.close(), "busy");
    r.finish().unwrap();
    let r = other.reserve(&ctx).unwrap();
    assert_eq!(r.read("checkpoint").unwrap(), b"bound");
    r.finish().unwrap();
    other.close().unwrap();
    let parent = TempDir::new().unwrap();
    let retained = parent.path().join("retained");
    fs::rename(root.path(), &retained).unwrap();
    fs::create_dir(root.path()).unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.path().join("checkpoint"), b"replacement").unwrap();
    let r = d.reserve(&ctx).unwrap();
    assert_eq!(r.read("checkpoint").unwrap(), b"bound");
    r.replace("checkpoint", b"bound", b"retained-next").unwrap();
    r.finish().unwrap();
    d.close().unwrap();
    assert_eq!(
        fs::read(retained.join("checkpoint")).unwrap(),
        b"retained-next"
    );
    assert_eq!(
        fs::read(root.path().join("checkpoint")).unwrap(),
        b"replacement"
    );
}
#[test]
fn private_documents_links_modes_types_and_lock_replacement_refuse_then_recover() {
    use rustix::fs::{mknodat, FileType, Mode, CWD};
    let (root, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("checkpoint", b"witness").unwrap();
    r.finish().unwrap();
    let file = root.path().join("checkpoint");
    let hard = root.path().join("alias");
    fs::hard_link(&file, &hard).unwrap();
    let r = d.reserve(&ctx).unwrap();
    code(r.read("checkpoint"), "path");
    assert_eq!(
        r.replace("checkpoint", b"witness", b"bad")
            .unwrap_err()
            .cause
            .code(),
        "path"
    );
    r.finish().unwrap();
    fs::remove_file(&hard).unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    let r = d.reserve(&ctx).unwrap();
    code(r.read("checkpoint"), "path");
    r.finish().unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    fs::rename(&file, &hard).unwrap();
    symlink(&hard, &file).unwrap();
    let r = d.reserve(&ctx).unwrap();
    code(r.read("checkpoint"), "path");
    r.finish().unwrap();
    assert_eq!(fs::read(&hard).unwrap(), b"witness");
    fs::remove_file(&file).unwrap();
    mknodat(CWD, &file, FileType::Fifo, Mode::from_raw_mode(0o600), 0).unwrap();
    let r = d.reserve(&ctx).unwrap();
    code(r.read("checkpoint"), "path");
    r.finish().unwrap();
    fs::remove_file(&file).unwrap();
    fs::rename(&hard, &file).unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    code(d.reserve(&ctx), "path");
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let lock = root.path().join("private-documents.lock");
    fs::rename(&lock, &hard).unwrap();
    fs::write(&lock, b"new inode").unwrap();
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
    code(d.reserve(&ctx), "path");
    fs::remove_file(&lock).unwrap();
    fs::rename(&hard, &lock).unwrap();
    let r = d.reserve(&ctx).unwrap();
    assert_eq!(r.read("checkpoint").unwrap(), b"witness");
    r.finish().unwrap();
    d.close().unwrap();
    code(
        Documents::open(root.path(), rustix::process::geteuid().as_raw() + 1),
        "path",
    );
    let alias = root.path().with_extension("alias");
    symlink(root.path(), &alias).unwrap();
    code(
        Documents::open(&alias, rustix::process::geteuid().as_raw()),
        "path",
    );
    fs::remove_file(alias).unwrap();
}
#[test]
fn private_documents_cancellation_and_close_are_explicit_without_mutation() {
    let (_root, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("checkpoint", b"witness").unwrap();
    ctx.cancel();
    code(r.read("checkpoint"), "canceled");
    let e = r.replace("checkpoint", b"witness", b"bad").unwrap_err();
    assert_eq!(e.cause.code(), "canceled");
    assert_eq!(e.publication, Publication::NotAttempted);
    r.finish().unwrap();
    code(d.reserve(&ctx), "canceled");
    let active = Context::new();
    let r = d.reserve(&active).unwrap();
    assert_eq!(r.read("checkpoint").unwrap(), b"witness");
    r.finish().unwrap();
    let clone = d.clone();
    d.close().unwrap();
    code(clone.reserve(&active), "closed");
    clone.close().unwrap();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn private_documents_partial_enospc_sync_close_failures_preserve_original_and_recover() {
    use swarm_enrollment_config::store::FixtureFault as F;
    let (root, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("checkpoint", b"witness").unwrap();
    r.finish().unwrap();
    for f in [
        F::Entropy,
        F::Write,
        F::PartialWrite,
        F::FileSync,
        F::FileClose,
    ] {
        d.fixture_faults(&[]);
        let r = d.reserve(&ctx).unwrap();
        assert_eq!(r.read("checkpoint").unwrap(), b"witness");
        r.finish().unwrap();
        d.fixture_faults(&[f]);
        let r = d.reserve(&ctx).unwrap();
        let e = r
            .replace("checkpoint", b"witness", b"new-long-value")
            .unwrap_err();
        assert_eq!(e.publication, Publication::NotAttempted);
        assert_eq!(e.cause.code(), "storage");
        assert!(std::error::Error::source(&e).is_some());
        if matches!(f, F::Write | F::PartialWrite) {
            let cause = std::error::Error::source(&e.cause)
                .unwrap()
                .downcast_ref::<std::io::Error>()
                .unwrap();
            assert_eq!(cause.raw_os_error(), Some(28));
        }
        r.finish().unwrap();
        assert_eq!(
            fs::read(root.path().join("checkpoint")).unwrap(),
            b"witness"
        );
        assert_eq!(
            fs::read_dir(root.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|x| x.file_name().to_string_lossy().starts_with(".config-"))
                .count(),
            0
        );
    }
    d.fixture_faults(&[]);
    let r = d.reserve(&ctx).unwrap();
    assert_eq!(
        r.replace("checkpoint", b"witness", b"recovered").unwrap(),
        Publication::Durable
    );
    r.finish().unwrap();
    d.close().unwrap();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn private_documents_postrename_and_secondary_cleanup_failures_keep_publication_state() {
    use swarm_enrollment_config::store::FixtureFault as F;
    let (_root, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("checkpoint", b"witness").unwrap();
    r.finish().unwrap();
    d.fixture_faults(&[F::DirectorySync]);
    let r = d.reserve(&ctx).unwrap();
    let e = r.replace("checkpoint", b"witness", b"visible").unwrap_err();
    assert_eq!(e.publication, Publication::Visible);
    assert_eq!(e.cause.code(), "indeterminate");
    r.finish().unwrap();
    d.fixture_faults(&[]);
    let r = d.reserve(&ctx).unwrap();
    assert_eq!(r.read("checkpoint").unwrap(), b"visible");
    r.sync().unwrap();
    r.finish().unwrap();
    d.fixture_faults(&[F::Unlink]);
    let r = d.reserve(&ctx).unwrap();
    let e = r.replace("checkpoint", b"visible", b"durable").unwrap_err();
    assert_eq!(e.publication, Publication::Durable);
    r.finish().unwrap();
    d.fixture_faults(&[]);
    let r = d.reserve(&ctx).unwrap();
    assert_eq!(r.read("checkpoint").unwrap(), b"durable");
    r.finish().unwrap();
    d.fixture_faults(&[F::PartialWrite, F::Unlink]);
    let r = d.reserve(&ctx).unwrap();
    let e = r.replace("checkpoint", b"durable", b"bad").unwrap_err();
    assert_eq!(e.publication, Publication::NotAttempted);
    assert!(e.cause.cleanup_error().is_some());
    r.finish().unwrap();
    d.fixture_faults(&[]);
    let witness = d.reserve(&ctx).unwrap();
    assert_eq!(witness.read("checkpoint").unwrap(), b"durable");
    witness.finish().unwrap();
    d.fixture_faults(&[F::ReadClose]);
    let r = d.reserve(&ctx).unwrap();
    code(r.read("checkpoint"), "storage");
    r.finish().unwrap();
    d.fixture_faults(&[]);
    let r = d.reserve(&ctx).unwrap();
    assert_eq!(r.read("checkpoint").unwrap(), b"durable");
    r.finish().unwrap();
    d.close().unwrap();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn private_documents_release_and_panic_cleanup_preserve_outcomes_and_refuse_poison() {
    use swarm_enrollment_config::store::FixtureFault as F;
    let (_root, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    let outcome = r.create("checkpoint", b"durable").unwrap();
    d.fixture_faults(&[F::Unlock]);
    code(r.finish(), "storage");
    assert_eq!(outcome, Publication::Durable);
    d.fixture_faults(&[]);
    let r = d.reserve(&ctx).unwrap();
    assert_eq!(r.read("checkpoint").unwrap(), b"durable");
    r.finish().unwrap();
    d.fixture_faults(&[F::Unlock]);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _r = d.reserve(&ctx).unwrap();
        panic!("isolated panic witness")
    }));
    assert!(panic.is_err());
    code(d.reserve(&ctx), "state");
    d.fixture_faults(&[F::StoreClose]);
    let e = d.close().unwrap_err();
    assert_eq!(e.code(), "storage");
    assert!(e.cleanup_error().is_some());
    d.fixture_faults(&[]);
    d.close().unwrap();
    code(d.reserve(&ctx), "closed");
}

#[cfg(feature = "test-fixtures")]
#[test]
fn private_documents_dropped_release_failure_blocks_reuse_until_close_receives_cause() {
    use swarm_enrollment_config::store::FixtureFault as F;
    let (_root, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("checkpoint", b"witness").unwrap();
    r.finish().unwrap();
    d.fixture_faults(&[F::Unlock]);
    let r = d.reserve(&ctx).unwrap();
    assert_eq!(r.read("checkpoint").unwrap(), b"witness");
    drop(r);
    d.fixture_faults(&[]);
    code(d.reserve(&ctx), "state");
    code(d.close(), "storage");
    d.close().unwrap();
}
