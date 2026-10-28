use ed25519_dalek::SigningKey;
use rusqlite::Connection;
use std::sync::{mpsc, Arc};
use swarm_enrollment_proof::Verifier;
use swarm_enrollment_state::{decode_active_reply, sign_change, Context, Error, Head, Journal};

struct Fixture {
    dir: tempfile::TempDir,
    journal: Arc<Journal>,
    key: SigningKey,
    project: [u8; 32],
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        let project = [11; 32];
        let journal = Arc::new(
            Journal::open(
                &Context::new(),
                dir.path().join("journal.sqlite"),
                &key.verifying_key().to_bytes(),
                project,
            )
            .unwrap(),
        );
        Self {
            dir,
            journal,
            key,
            project,
        }
    }
    fn change(&self, base: &Head, op: &str, id: &str, body: &[u8]) -> Vec<u8> {
        sign_change(&self.key, self.project, base, op, id, body).unwrap()
    }
    fn append(&self, base: &Head, op: &str, id: &str, body: &[u8]) -> (Head, Vec<u8>) {
        let raw = self.change(base, op, id, body);
        let head = self.journal.apply(&Context::new(), base, &raw).unwrap();
        (head, raw)
    }
    fn sql(&self) -> Connection {
        Connection::open(self.dir.path().join("journal.sqlite")).unwrap()
    }
    fn active(&self, head: &Head, id: &str) -> Result<(), Error> {
        self.journal
            .with_active::<()>(&Context::new(), head, id, |_| Ok(()))
    }
}
#[test]
fn persistence_exact_retry_revocation_and_owned_body() {
    let f = Fixture::new();
    let zero = Head::default();
    let (head, raw) = f.append(&zero, "enroll", " alice ", b"original");
    assert_eq!(f.journal.apply(&Context::new(), &zero, &raw).unwrap(), head);
    f.journal
        .with_active::<()>(&Context::new(), &head, " alice ", |mut record| {
            assert_eq!(record.body, b"original");
            record.body[0] = b'!';
            Ok(())
        })
        .unwrap();
    f.journal.close().unwrap();
    assert_eq!(f.active(&head, " alice ").unwrap_err().code(), "closed");
    let j = Journal::open(
        &Context::new(),
        f.dir.path().join("journal.sqlite"),
        &f.key.verifying_key().to_bytes(),
        f.project,
    )
    .unwrap();
    j.with_active::<()>(&Context::new(), &head, " alice ", |r| {
        assert_eq!(r.body, b"original");
        Ok(())
    })
    .unwrap();
    let revoke = f.change(&head, "revoke", " alice ", b"");
    let revoked = j.apply(&Context::new(), &head, &revoke).unwrap();
    assert_eq!(j.apply(&Context::new(), &head, &revoke).unwrap(), revoked);
    assert_eq!(
        j.with_active::<()>(&Context::new(), &revoked, " alice ", |_| panic!(
            "revoked callback"
        ))
        .unwrap_err()
        .code(),
        "revoked"
    );
    assert_eq!(
        j.with_active::<()>(&Context::new(), &head, " alice ", |_| panic!(
            "stale callback"
        ))
        .unwrap_err()
        .code(),
        "head"
    );
    assert_eq!(
        j.apply(&Context::new(), &zero, &raw).unwrap_err().code(),
        "head"
    );
    assert_eq!(
        j.apply(
            &Context::new(),
            &revoked,
            &f.change(&revoked, "enroll", " alice ", b"replacement")
        )
        .unwrap_err()
        .code(),
        "exists"
    );
}
#[test]
fn signed_active_empty_body_and_historical_signature() {
    let f = Fixture::new();
    let (head, _) = f.append(&Head::default(), "enroll", "<>&é\u{2028}", b"");
    let v = Verifier::new(&f.key.verifying_key().to_bytes(), f.project).unwrap();
    let ch = v.new_challenge([13; 32]).unwrap();
    let response = f
        .journal
        .sign_active(&Context::new(), &head, "<>&é\u{2028}", ch.request(), &f.key)
        .unwrap();
    let (revoked, _) = f.append(&head, "revoke", "<>&é\u{2028}", b"");
    let statement = v.verify(&ch, &response).unwrap();
    let reply = decode_active_reply(&statement.payload()).unwrap();
    assert_eq!(reply.head, head);
    assert_eq!(reply.record.body, Vec::<u8>::new());
    assert_eq!(reply.record.enrolled_sequence, 1);
    let new = v.new_challenge([13; 32]).unwrap();
    assert_eq!(
        f.journal
            .sign_active(
                &Context::new(),
                &revoked,
                "<>&é\u{2028}",
                new.request(),
                &f.key
            )
            .unwrap_err()
            .code(),
        "revoked"
    );
}
#[test]
fn writer_reservation_and_concurrent_identical_append() {
    let f = Fixture::new();
    let (head, _) = f.append(&Head::default(), "enroll", "alice", b"body");
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let j = Arc::clone(&f.journal);
    let h = head.clone();
    let thread = std::thread::spawn(move || {
        j.with_active::<()>(&Context::new(), &h, "alice", |_| {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(())
        })
    });
    entered_rx.recv().unwrap();
    let sql = f.sql();
    let mode: String = sql
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    let rev = f.change(&head, "revoke", "alice", b"");
    let err = f.journal.apply(&Context::new(), &head, &rev).unwrap_err();
    assert_eq!(err.code(), "sqlite");
    assert!(err.is_busy());
    release_tx.send(()).unwrap();
    thread.join().unwrap().unwrap();
    let revoked = f.journal.apply(&Context::new(), &head, &rev).unwrap();
    let raw = f.change(&revoked, "enroll", "bob", b"");
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let j = Arc::clone(&f.journal);
            let b = Arc::clone(&barrier);
            let h = revoked.clone();
            let raw = raw.clone();
            std::thread::spawn(move || {
                b.wait();
                j.apply(&Context::new(), &h, &raw)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(results.iter().any(Result::is_ok));
    for e in results.iter().filter_map(|r| r.as_ref().err()) {
        assert!(e.is_busy(), "{e}");
    }
    let count: i64 = f
        .sql()
        .query_row("SELECT count(*) FROM enrollment_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3);
}
#[test]
fn callback_failure_cancellation_and_sql_rollback_recover() {
    let f = Fixture::new();
    let (head, _) = f.append(&Head::default(), "enroll", "alice", b"body");
    f.active(&head, "alice").unwrap();
    assert_eq!(
        f.journal
            .with_active::<()>(&Context::new(), &head, "alice", |_| Err(Error::Missing))
            .unwrap_err()
            .code(),
        "missing"
    );
    let ctx = Context::new();
    f.active(&head, "alice").unwrap();
    assert_eq!(
        f.journal
            .with_active::<()>(&ctx, &head, "alice", |_| {
                ctx.cancel();
                Ok(())
            })
            .unwrap_err()
            .code(),
        "canceled"
    );
    let raw = f.change(&head, "enroll", "bob", b"body");
    f.active(&head, "alice").unwrap();
    f.sql().execute_batch("CREATE TRIGGER fixture_reject_head BEFORE UPDATE ON enrollment_meta BEGIN SELECT RAISE(ABORT,'injected head failure'); END").unwrap();
    assert_eq!(
        f.journal
            .apply(&Context::new(), &head, &raw)
            .unwrap_err()
            .code(),
        "sqlite"
    );
    let count: i64 = f
        .sql()
        .query_row("SELECT count(*) FROM enrollment_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    f.active(&head, "alice").unwrap();
    f.sql()
        .execute_batch("DROP TRIGGER fixture_reject_head")
        .unwrap();
    assert_eq!(
        f.journal
            .apply(&Context::new(), &head, &raw)
            .unwrap()
            .sequence,
        2
    );
}
#[test]
fn tamper_rollback_and_foreign_pins_never_call_consumer() {
    for fault in [
        "event",
        "missing",
        "head",
        "rollback",
        "future-row",
        "version",
        "project",
        "key",
    ] {
        let f = Fixture::new();
        let (active, raw) = f.append(&Head::default(), "enroll", "alice", b"original");
        f.active(&active, "alice").unwrap();
        let (revoked, _) = f.append(&active, "revoke", "alice", b"");
        let sql = f.sql();
        match fault {
            "event" => {
                sql.execute(
                    "UPDATE enrollment_events SET body=? WHERE sequence=1",
                    [b"invalid".as_slice()],
                )
                .unwrap();
            }
            "missing" => {
                sql.execute("DELETE FROM enrollment_events WHERE sequence=1", [])
                    .unwrap();
            }
            "head" => {
                sql.execute("UPDATE enrollment_meta SET sequence=0,digest=''", [])
                    .unwrap();
            }
            "rollback" => {
                sql.execute("DELETE FROM enrollment_events WHERE sequence=2", [])
                    .unwrap();
                sql.execute(
                    "UPDATE enrollment_meta SET sequence=1,digest=?",
                    [&active.digest],
                )
                .unwrap();
            }
            "future-row" => {
                sql.execute(
                    "INSERT INTO enrollment_events(sequence,body) VALUES(3,?)",
                    [raw],
                )
                .unwrap();
            }
            "version" => {
                sql.execute("UPDATE enrollment_meta SET schema_version=2", [])
                    .unwrap();
            }
            "project" => {
                assert_eq!(
                    Journal::open(
                        &Context::new(),
                        f.dir.path().join("journal.sqlite"),
                        &f.key.verifying_key().to_bytes(),
                        [12; 32]
                    )
                    .err()
                    .unwrap()
                    .code(),
                    "context"
                );
                continue;
            }
            "key" => {
                assert_eq!(
                    Journal::open(
                        &Context::new(),
                        f.dir.path().join("journal.sqlite"),
                        &SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes(),
                        f.project
                    )
                    .err()
                    .unwrap()
                    .code(),
                    "context"
                );
                continue;
            }
            _ => unreachable!(),
        }
        assert!(
            f.journal
                .with_active::<()>(&Context::new(), &revoked, "alice", |_| panic!(
                    "tamper invoked consumer"
                ))
                .is_err(),
            "{fault}"
        );
        if fault == "rollback" {
            f.active(&active, "alice").unwrap();
        } // A restored witness loses rollback detection.
    }
}
#[test]
fn invalid_changes_and_precanceled_calls_do_not_mutate() {
    let f = Fixture::new();
    let zero = Head::default();
    let raw = f.change(&zero, "enroll", "alice", b"");
    // Each rejection is followed by the same valid append then tested against a fresh journal.
    for bad in [
        [b" ".as_slice(), &raw].concat(),
        [raw.as_slice(), b"{}"].concat(),
        b"{}".to_vec(),
        String::from_utf8(raw.clone())
            .unwrap()
            .replace("\"body\":\"\"", "\"body\":null")
            .into_bytes(),
    ] {
        let fresh = Fixture::new();
        let control = fresh.journal.apply(&Context::new(), &zero, &raw).unwrap();
        fresh.active(&control, "alice").unwrap();
        let f2 = Fixture::new();
        assert_eq!(
            f2.journal
                .apply(&Context::new(), &zero, &bad)
                .unwrap_err()
                .code(),
            "change"
        );
        assert_eq!(
            f2.journal
                .apply(&Context::new(), &zero, &raw)
                .unwrap()
                .sequence,
            1
        );
    }
    let ctx = Context::new();
    ctx.cancel();
    assert_eq!(
        f.journal.apply(&ctx, &zero, &raw).unwrap_err().code(),
        "canceled"
    );
    let head = f.journal.apply(&Context::new(), &zero, &raw).unwrap();
    f.active(&head, "alice").unwrap();
    for (h, op, id, body) in [
        (
            Head {
                sequence: -1,
                digest: String::new(),
            },
            "enroll",
            "a",
            b"".as_slice(),
        ),
        (
            Head {
                sequence: i64::MAX,
                digest: "a".repeat(64),
            },
            "enroll",
            "a",
            b"",
        ),
        (zero.clone(), "reactivate", "a", b""),
        (zero.clone(), "enroll", "  ", b""),
        (zero.clone(), "enroll", "a\0", b""),
        (zero, "revoke", "a", b"x"),
    ] {
        assert_eq!(
            sign_change(&f.key, f.project, &h, op, id, body)
                .unwrap_err()
                .code(),
            "change"
        );
    }
}

#[test]
fn sqlite_text_event_bytes_match_go_scan_without_weakening_signature_checks() {
    let f = Fixture::new();
    let (head, raw) = f.append(&Head::default(), "enroll", "alice", b"owned");
    f.active(&head, "alice").unwrap();
    f.sql()
        .execute_batch("UPDATE enrollment_events SET body=CAST(body AS TEXT)")
        .unwrap();
    f.active(&head, "alice").unwrap();
    assert_eq!(
        f.journal
            .apply(&Context::new(), &Head::default(), &raw)
            .unwrap(),
        head
    );
    f.sql()
        .execute_batch("UPDATE enrollment_events SET body='forged'")
        .unwrap();
    assert_eq!(f.active(&head, "alice").unwrap_err().code(), "change");
}

#[test]
fn corrupt_history_keeps_the_original_rejection_cause() {
    use sha2::{Digest, Sha256};
    use std::error::Error as _;
    let f = Fixture::new();
    let (head, _) = f.append(&Head::default(), "enroll", "alice", b"owned");
    f.active(&head, "alice").unwrap();
    let raw = f.change(&head, "enroll", "alice", b"duplicate");
    let digest = format!("{:x}", Sha256::digest(&raw));
    let sql = f.sql();
    sql.execute(
        "INSERT INTO enrollment_events(sequence,body) VALUES(2,?)",
        [&raw],
    )
    .unwrap();
    sql.execute("UPDATE enrollment_meta SET sequence=2,digest=?", [&digest])
        .unwrap();
    let err = f
        .active(
            &Head {
                sequence: 2,
                digest,
            },
            "alice",
        )
        .unwrap_err();
    assert_eq!(err.code(), "change");
    assert!(err
        .source()
        .expect("history must retain the original rejection")
        .to_string()
        .contains("exists"));
}

#[test]
fn signed_negative_zero_preserves_active_reply_version_error_order() {
    let error =
        swarm_enrollment_state::decode_active_reply(br#"{"schema_version":-0}"#).unwrap_err();
    assert_eq!(error.code(), "reply-version");
}
