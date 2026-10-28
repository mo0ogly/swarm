use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use swarm_enrollment_state::{sign_change, BoundJournal, Context, Head, Journal};
struct Fixture {
    _dir: tempfile::TempDir,
    path: std::path::PathBuf,
    key: SigningKey,
    head: Head,
    bound: BoundJournal,
}
impl Fixture {
    /// Open only a disposable initialized signed WAL store with independent fixture pins.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.sqlite");
        let key = SigningKey::from_bytes(&[21; 32]);
        let j = Journal::open(
            &Context::new(),
            &path,
            &key.verifying_key().to_bytes(),
            [22; 32],
        )
        .unwrap();
        let raw = sign_change(
            &key,
            [22; 32],
            &Head::default(),
            "enroll",
            "setup",
            b"commitment",
        )
        .unwrap();
        let head = j.apply(&Context::new(), &Head::default(), &raw).unwrap();
        j.close().unwrap();
        let bound = BoundJournal::open(
            &Context::new(),
            &path,
            &key.verifying_key().to_bytes(),
            [22; 32],
        )
        .unwrap();
        Self {
            _dir: dir,
            path,
            key,
            head,
            bound,
        }
    }
    /// Build two chained signed mutations; the second ID is an explicit test variant.
    fn changes(&self, id: &str) -> (Vec<u8>, Vec<u8>, Head) {
        let a = sign_change(&self.key, [22; 32], &self.head, "revoke", "setup", b"").unwrap();
        let h = next(&self.head, &a);
        let b = sign_change(&self.key, [22; 32], &h, "enroll", id, b"credential").unwrap();
        let end = next(&h, &b);
        (a, b, end)
    }
    /// Observe exact fixture SQL rows rather than WAL-file bytes altered by SQLite housekeeping.
    fn snapshot(&self) -> Vec<(i64, Vec<u8>)> {
        let db = rusqlite::Connection::open(&self.path).unwrap();
        let mut q = db
            .prepare("SELECT sequence,body FROM enrollment_events ORDER BY sequence")
            .unwrap();
        q.query_map([], |r| Ok((r.get(0)?, r.get_ref(1)?.as_bytes()?.to_vec())))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }
}
fn next(h: &Head, raw: &[u8]) -> Head {
    Head {
        sequence: h.sequence + 1,
        digest: format!("{:x}", Sha256::digest(raw)),
    }
}
#[test]
fn signed_batch_atomically_retires_setup_creates_account_and_exact_replay_never_duplicates() {
    let f = Fixture::new();
    let (a, b, end) = f.changes("account");
    assert_eq!(
        f.bound
            .apply_batch(&Context::new(), &f.head, &[&a, &b])
            .unwrap(),
        end
    );
    assert_eq!(
        f.bound
            .with_active(&Context::new(), &end, "setup", |_| Ok(()))
            .unwrap_err()
            .code(),
        "revoked"
    );
    assert_eq!(
        f.bound
            .with_active(&Context::new(), &end, "account", |r| Ok(r.body))
            .unwrap(),
        b"credential"
    );
    // Valid unchanged signed bytes represented as TEXT, matching Go Scan([]byte).
    rusqlite::Connection::open(&f.path)
        .unwrap()
        .execute_batch("UPDATE enrollment_events SET body=CAST(body AS TEXT)")
        .unwrap();
    let before = f.snapshot();
    assert_eq!(before.len(), 3);
    assert_eq!(
        f.bound
            .apply_batch(&Context::new(), &f.head, &[&a, &b])
            .unwrap(),
        end
    );
    assert_eq!(f.snapshot(), before);
    f.bound.close().unwrap();
    let reopened = BoundJournal::open(
        &Context::new(),
        &f.path,
        &f.key.verifying_key().to_bytes(),
        [22; 32],
    )
    .unwrap();
    assert_eq!(
        reopened
            .with_active(&Context::new(), &end, "account", |r| Ok(r.body))
            .unwrap(),
        b"credential"
    );
    reopened.close().unwrap();
}
#[test]
fn signed_batch_second_refusal_rolls_back_first_and_bad_chain_signature_context_empty_cancel_refuse(
) {
    let f = Fixture::new();
    let before = f.snapshot();
    let (a, invalid, _) = f.changes("setup");
    assert!(f
        .bound
        .apply_batch(&Context::new(), &f.head, &[&a, &invalid])
        .is_err());
    assert_eq!(f.snapshot(), before);
    assert!(f
        .bound
        .with_active(&Context::new(), &f.head, "setup", |_| Ok(()))
        .is_ok());
    let (a, b, end) = f.changes("account");
    let wrong = sign_change(
        &SigningKey::from_bytes(&[23; 32]),
        [22; 32],
        &f.head,
        "revoke",
        "setup",
        b"",
    )
    .unwrap();
    for changes in [
        vec![],
        vec![b.as_slice(), a.as_slice()],
        vec![wrong.as_slice()],
        vec![a.as_slice(), a.as_slice()],
    ] {
        assert!(f
            .bound
            .apply_batch(&Context::new(), &f.head, &changes)
            .is_err());
        assert_eq!(f.snapshot(), before);
    }
    let canceled = Context::new();
    canceled.cancel();
    assert!(f.bound.apply_batch(&canceled, &f.head, &[&a, &b]).is_err());
    assert_eq!(f.snapshot(), before);
    assert_eq!(
        f.bound
            .apply_batch(&Context::new(), &f.head, &[&a, &b])
            .unwrap(),
        end
    );
    f.bound.close().unwrap();
}
#[test]
fn signed_batch_real_writer_contention_then_recovery_and_stale_conflicting_batch_refuse() {
    let f = Fixture::new();
    let (a, b, end) = f.changes("account");
    let before = f.snapshot();
    let other = rusqlite::Connection::open(&f.path).unwrap();
    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(f
        .bound
        .apply_batch(&Context::new(), &f.head, &[&a, &b])
        .unwrap_err()
        .is_busy());
    assert_eq!(f.snapshot(), before);
    other.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        f.bound
            .apply_batch(&Context::new(), &f.head, &[&a, &b])
            .unwrap(),
        end
    );
    let (_, different, _) = f.changes("another-account");
    let before = f.snapshot();
    assert!(f
        .bound
        .apply_batch(&Context::new(), &f.head, &[&a, &different])
        .is_err());
    assert_eq!(f.snapshot(), before);
    f.bound.close().unwrap();
}

#[test]
fn single_apply_preserves_base_frame_closed_then_cancellation_refusal_order() {
    let f = Fixture::new();
    let j = Journal::open(
        &Context::new(),
        &f.path,
        &f.key.verifying_key().to_bytes(),
        [22; 32],
    )
    .unwrap();
    let canceled = Context::new();
    canceled.cancel();
    assert_eq!(
        j.apply(
            &canceled,
            &Head {
                sequence: -1,
                digest: String::new()
            },
            b"bad"
        )
        .unwrap_err()
        .code(),
        "head"
    );
    assert_eq!(
        j.apply(&canceled, &f.head, b"bad").unwrap_err().code(),
        "change"
    );
    let (raw, _, _) = f.changes("account");
    j.close().unwrap();
    assert_eq!(
        j.apply(&canceled, &f.head, &raw).unwrap_err().code(),
        "closed"
    );
    f.bound.close().unwrap();
}
