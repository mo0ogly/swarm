use ed25519_dalek::SigningKey;
use std::{
    fs,
    sync::{mpsc, Arc},
};
use swarm_enrollment_state::{sign_change, BoundJournal, Context, Head, Journal};
struct Fixture {
    dir: tempfile::TempDir,
    key: SigningKey,
    journal: Journal,
    head: Head,
}
impl Fixture {
    /// Create a disposable signed WAL journal; setup failures panic, no live storage is used.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        let journal = Journal::open(
            &Context::new(),
            dir.path().join("journal.sqlite"),
            &key.verifying_key().to_bytes(),
            [11; 32],
        )
        .unwrap();
        let raw = sign_change(
            &key,
            [11; 32],
            &Head::default(),
            "enroll",
            "alice",
            b"original",
        )
        .unwrap();
        let head = journal
            .apply(&Context::new(), &Head::default(), &raw)
            .unwrap();
        Self {
            dir,
            key,
            journal,
            head,
        }
    }
    /// Retain the fixture journal with its explicit public pins; panic if opening fails.
    fn bound(&self) -> BoundJournal {
        BoundJournal::open(
            &Context::new(),
            self.dir.path().join("journal.sqlite"),
            &self.key.verifying_key().to_bytes(),
            [11; 32],
        )
        .unwrap()
    }
}
#[test]
fn existing_only_no_follow_foreign_pins_and_explicit_close() {
    let f = Fixture::new();
    let bound = f.bound();
    assert_eq!(
        bound
            .with_active(&Context::new(), &f.head, "alice", |r| Ok(r.body))
            .unwrap(),
        b"original"
    );
    let absent = f.dir.path().join("absent.sqlite");
    assert!(BoundJournal::open(
        &Context::new(),
        &absent,
        &f.key.verifying_key().to_bytes(),
        [11; 32]
    )
    .is_err());
    assert!(!absent.exists());
    let link = f.dir.path().join("linked.sqlite");
    std::os::unix::fs::symlink(f.dir.path().join("journal.sqlite"), &link).unwrap();
    assert!(BoundJournal::open(
        &Context::new(),
        &link,
        &f.key.verifying_key().to_bytes(),
        [11; 32]
    )
    .is_err());
    assert!(BoundJournal::open(
        &Context::new(),
        f.dir.path().join("journal.sqlite"),
        &[8; 32],
        [11; 32]
    )
    .is_err());
    assert!(BoundJournal::open(
        &Context::new(),
        f.dir.path().join("journal.sqlite"),
        &f.key.verifying_key().to_bytes(),
        [12; 32]
    )
    .is_err());
    bound.close().unwrap();
    bound.close().unwrap();
    assert_eq!(
        bound
            .with_active(&Context::new(), &f.head, "alice", |_| Ok(()))
            .unwrap_err()
            .code(),
        "closed"
    );
}
#[test]
fn retained_connection_never_reopens_replaced_path() {
    let f = Fixture::new();
    let bound = f.bound();
    bound
        .with_active(&Context::new(), &f.head, "alice", |_| Ok(()))
        .unwrap();
    let path = f.dir.path().join("journal.sqlite");
    fs::rename(&path, f.dir.path().join("original.sqlite")).unwrap();
    fs::write(&path, b"foreign-not-a-database").unwrap();
    let before = fs::read(&path).unwrap();
    let result = bound.with_active(&Context::new(), &f.head, "alice", |r| {
        assert_eq!(r.body, b"original");
        Ok(())
    });
    // SQLite may refuse a moved WAL database. Either way it never follows or
    // recreates the replacement; that would yield NotADatabase instead.
    if let Err(e) = result {
        assert!(!e.to_string().contains("not a database"), "{e}");
    }
    assert_eq!(fs::read(&path).unwrap(), before);
    bound.close().unwrap();
}
#[test]
fn current_revocation_contention_cancellation_and_recovery() {
    let f = Fixture::new();
    let bound = Arc::new(f.bound());
    bound
        .with_active(&Context::new(), &f.head, "alice", |_| Ok(()))
        .unwrap();
    let ctx = Context::new();
    ctx.cancel();
    assert_eq!(
        bound
            .with_active::<()>(&ctx, &f.head, "alice", |_| panic!("canceled"))
            .unwrap_err()
            .code(),
        "canceled"
    );
    bound
        .with_active(&Context::new(), &f.head, "alice", |_| Ok(()))
        .unwrap();
    let (tx, rx) = mpsc::channel();
    let (release, waiting) = mpsc::channel();
    let b = bound.clone();
    let head = f.head.clone();
    let worker = std::thread::spawn(move || {
        b.with_active(&Context::new(), &head, "alice", |_| {
            tx.send(()).unwrap();
            waiting.recv().unwrap();
            Ok(())
        })
    });
    rx.recv().unwrap();
    let raw = sign_change(&f.key, [11; 32], &f.head, "revoke", "alice", b"").unwrap();
    let revoke = f.journal.apply(&Context::new(), &f.head, &raw);
    let close = bound.close();
    release.send(()).unwrap();
    worker.join().unwrap().unwrap();
    assert!(revoke.unwrap_err().is_busy());
    assert!(close.unwrap_err().is_busy());
    let revoked = f.journal.apply(&Context::new(), &f.head, &raw).unwrap();
    assert_eq!(
        bound
            .with_active(&Context::new(), &revoked, "alice", |_| Ok(()))
            .unwrap_err()
            .code(),
        "revoked"
    );
    assert_eq!(
        bound
            .with_active(&Context::new(), &f.head, "alice", |_| Ok(()))
            .unwrap_err()
            .code(),
        "head"
    );
    bound.close().unwrap();
}
