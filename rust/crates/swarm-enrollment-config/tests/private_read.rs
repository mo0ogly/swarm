#![cfg(target_os = "linux")]
//! Disposable private file reads, no installed seeds, service or live datastore.
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::PathBuf,
};
use swarm_enrollment_config::{
    store::{documents::Documents, Context, FixtureFault},
    Error,
};
/// Private child under an owned temporary parent, allowing safe rename witnesses.
fn fixture() -> (tempfile::TempDir, PathBuf, Documents) {
    let root = tempfile::tempdir().unwrap();
    mode(root.path(), 0o700);
    let path = root.path().join("documents");
    fs::create_dir(&path).unwrap();
    mode(&path, 0o700);
    let d = Documents::open(&path, rustix::process::geteuid().as_raw()).unwrap();
    (root, path, d)
}
/// Fixture-only modes, never touches an installed path.
fn mode(path: &std::path::Path, bits: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(bits)).unwrap()
}
/// No success Debug is needed to assert a refusal, avoiding accidental secrets.
fn code<T>(result: Result<T, Error>, expected: &str) {
    match result {
        Ok(_) => panic!("expected {expected}"),
        Err(e) => assert_eq!(e.code(), expected),
    }
}

/// Exact owned bytes survive release/close, while ordinary Debug stays redacted.
#[test]
fn private_read_exact_empty_binary_and_old_reader_remain_distinct() {
    let (_root, _path, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    for (name, raw) in [
        ("empty", vec![]),
        ("binary", b"PRIVATE_BYTES\0\n".to_vec()),
        ("long", vec![42; 100_000]),
    ] {
        r.create(name, &raw).unwrap();
        let bytes = r.read_private(name).unwrap();
        assert_eq!(bytes.as_bytes(), raw);
        assert_eq!(format!("{bytes:?}"), "PrivateBytes { <private> }");
        assert_eq!(r.read(name).unwrap(), raw);
    }
    let held = r.read_private("binary").unwrap();
    r.finish().unwrap();
    d.close().unwrap();
    assert_eq!(held.as_bytes(), b"PRIVATE_BYTES\0\n");
}
/// A real valid witness precedes every physical refusal, followed by recovery.
#[test]
fn private_read_missing_names_modes_links_types_refuse_and_recover() {
    let (_root, path, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("secret", b"PRIVATE_BINARY").unwrap();
    for name in ["../secret", "/secret", "private-documents.lock", ""] {
        assert_eq!(
            r.read_private("secret").unwrap().as_bytes(),
            b"PRIVATE_BINARY"
        );
        code(r.read_private(name), "path");
    }
    code(r.read_private("absent"), "not_configured");
    assert_eq!(
        r.read_private("secret").unwrap().as_bytes(),
        b"PRIVATE_BINARY"
    );
    mode(&path.join("secret"), 0o644);
    code(r.read_private("secret"), "path");
    mode(&path.join("secret"), 0o600);
    assert_eq!(
        r.read_private("secret").unwrap().as_bytes(),
        b"PRIVATE_BINARY"
    );
    fs::hard_link(path.join("secret"), path.join("alias")).unwrap();
    code(r.read_private("secret"), "path");
    fs::remove_file(path.join("alias")).unwrap();
    assert_eq!(
        r.read_private("secret").unwrap().as_bytes(),
        b"PRIVATE_BINARY"
    );
    symlink(path.join("secret"), path.join("symlink")).unwrap();
    code(r.read_private("symlink"), "path");
    fs::create_dir(path.join("directory")).unwrap();
    code(r.read_private("directory"), "path");
    assert_eq!(
        r.read_private("secret").unwrap().as_bytes(),
        b"PRIVATE_BINARY"
    );
    r.finish().unwrap();
    d.close().unwrap();
}
/// Retained directory descriptors keep selecting the original object after rename.
#[test]
fn private_read_retained_directory_survives_path_replacement() {
    let (root, path, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("secret", b"original").unwrap();
    assert_eq!(r.read_private("secret").unwrap().as_bytes(), b"original");
    fs::rename(&path, root.path().join("retained")).unwrap();
    fs::create_dir(&path).unwrap();
    mode(&path, 0o700);
    fs::write(path.join("secret"), b"replacement").unwrap();
    mode(&path.join("secret"), 0o600);
    assert_eq!(r.read_private("secret").unwrap().as_bytes(), b"original");
    r.finish().unwrap();
    d.close().unwrap();
    assert_eq!(fs::read(path.join("secret")).unwrap(), b"replacement");
}
/// Cancellation never prevents explicit release/close and does not mutate data.
#[test]
fn private_read_cancellation_and_closed_owner_refuse_without_mutation() {
    let (_root, path, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("secret", b"original").unwrap();
    assert_eq!(r.read_private("secret").unwrap().as_bytes(), b"original");
    ctx.cancel();
    code(r.read_private("secret"), "canceled");
    r.finish().unwrap();
    assert_eq!(fs::read(path.join("secret")).unwrap(), b"original");
    let fresh = Context::new();
    let r = d.reserve(&fresh).unwrap();
    assert_eq!(r.read_private("secret").unwrap().as_bytes(), b"original");
    r.finish().unwrap();
    d.close().unwrap();
    code(d.reserve(&fresh), "closed");
}
/// Existing ReadClose fault follows actual close; no bytes are returned on failure.
#[test]
fn private_read_actual_close_error_refuses_bytes_then_recovery_reads_exact() {
    let (_root, path, d) = fixture();
    let ctx = Context::new();
    let r = d.reserve(&ctx).unwrap();
    r.create("secret", b"PRIVATE_BINARY").unwrap();
    assert_eq!(
        r.read_private("secret").unwrap().as_bytes(),
        b"PRIVATE_BINARY"
    );
    d.fixture_faults(&[FixtureFault::ReadClose]);
    let e = r.read_private("secret").unwrap_err();
    assert_eq!(e.code(), "storage");
    assert!(!format!("{e:?} {e}").contains("PRIVATE_BINARY"));
    d.fixture_faults(&[]);
    assert_eq!(
        r.read_private("secret").unwrap().as_bytes(),
        b"PRIVATE_BINARY"
    );
    r.finish().unwrap();
    d.close().unwrap();
    assert_eq!(fs::read(path.join("secret")).unwrap(), b"PRIVATE_BINARY");
}
