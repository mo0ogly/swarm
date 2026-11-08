//! `Journal::snapshot`: verified head + sorted active ids, read-only.
//! Fixture directory under the repository fixture area (never /tmp).
use ed25519_dalek::SigningKey;
use swarm_enrollment_state::{sign_change, Context, Error, Head, Journal};

fn area() -> std::path::PathBuf {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
    std::fs::create_dir_all(&base).unwrap();
    base
}

#[test]
fn snapshot_tracks_enroll_and_revoke_without_writing() {
    let dir = tempfile::tempdir_in(area()).unwrap();
    let key = SigningKey::from_bytes(&[7; 32]);
    let project = [11; 32];
    let ctx = Context::new();
    let journal = Journal::open(&ctx, dir.path().join("journal.sqlite"), &key.verifying_key().to_bytes(), project).unwrap();
    let (empty_head, empty) = journal.snapshot(&ctx).unwrap();
    assert_eq!((empty_head, empty), (Head::default(), Vec::<String>::new()));
    let mut head = Head::default();
    for id in ["account/zoe/g1", "account/ana/g1"] {
        let raw = sign_change(&key, project, &head, "enroll", id, b"{}").unwrap();
        head = journal.apply(&ctx, &head, &raw).unwrap();
    }
    let raw = sign_change(&key, project, &head, "revoke", "account/zoe/g1", b"").unwrap();
    head = journal.apply(&ctx, &head, &raw).unwrap();
    let (observed, active) = journal.snapshot(&ctx).unwrap();
    assert_eq!(observed, head);
    assert_eq!(active, vec!["account/ana/g1".to_owned()]);
    // Read-only: a second snapshot sees the same head (nothing appended).
    assert_eq!(journal.snapshot(&ctx).unwrap().0, head);
    journal.close().unwrap();
    assert!(matches!(journal.snapshot(&ctx), Err(Error::Closed)));
}
