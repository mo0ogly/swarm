#![cfg(target_os = "linux")]
mod personal_common;
use ed25519_dalek::SigningKey;
use personal_common::Fixture;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_enrollment_authority::personal::{
    coordinator::{Context, Coordinator, JournalOutcome, UseError},
    Expected, Source,
};
use swarm_enrollment_config::store::documents::{Documents, Publication};
use swarm_enrollment_state::{Context as JournalContext, Head};
/// Real disposable signed journal plus a separate private witness directory.
struct Case {
    _journal: tempfile::TempDir,
    witness: tempfile::TempDir,
    path: PathBuf,
    key: SigningKey,
    expected: Expected,
    head: Head,
    proof: [u8; 32],
    documents: Documents,
    co: Coordinator,
}
impl Case {
    /// Independent fixture pins/head/proof, never a live account or agent identity.
    fn new() -> Self {
        Self::from_fixture(Fixture::new())
    }
    /// Preserve the same real fixture when a test also needs its independent writer.
    fn from_fixture(f: Fixture) -> Self {
        let witness = tempfile::tempdir().unwrap();
        fs::set_permissions(witness.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let documents =
            Documents::open(witness.path(), rustix::process::geteuid().as_raw()).unwrap();
        let co = Coordinator::new(f.source, documents.clone());
        Self {
            _journal: f._dir,
            witness,
            path: f.path,
            key: f.key,
            expected: f.expected,
            head: f.head,
            proof: f.proof,
            documents,
            co,
        }
    }
    /// Reopen the same protected objects with independently retained root context.
    fn reopen(&mut self) {
        self.co.close().unwrap();
        let source = Source::open(
            &JournalContext::new(),
            &self.path,
            &self.key.verifying_key().to_bytes(),
            [43; 32],
            self.expected.clone(),
        )
        .unwrap();
        self.documents =
            Documents::open(self.witness.path(), rustix::process::geteuid().as_raw()).unwrap();
        self.co = Coordinator::new(source, self.documents.clone());
    }
    /// Read only this fixture's checkpoint for exact mutation/secret assertions.
    fn bytes(&self) -> Vec<u8> {
        fs::read(self.witness.path().join("personal-checkpoint.json")).unwrap()
    }
}
#[test]
fn personal_coordinator_initializes_explicitly_enrolls_once_and_authenticates_after_reopen() {
    let mut c = Case::new();
    let ctx = Context::new();
    let called = std::cell::Cell::new(false);
    assert!(c
        .co
        .with_authenticated(&ctx, b"private", |_| {
            called.set(true);
            Ok::<_, ()>(())
        })
        .is_err());
    assert!(!called.get());
    c.co.initialize(&ctx, &c.head).unwrap();
    let original = c.bytes();
    assert!(c.co.initialize(&ctx, &c.head).is_err());
    assert_eq!(c.bytes(), original);
    let account =
        c.co.enroll(&ctx, &c.key, &c.proof, b"PRIVATE_PASSWORD")
            .unwrap();
    let saved = c.bytes();
    assert!(!String::from_utf8(saved.clone())
        .unwrap()
        .contains("PRIVATE_PASSWORD"));
    assert!(c.co.enroll(&ctx, &c.key, &c.proof, b"changed").is_err());
    assert_eq!(c.bytes(), saved);
    c.reopen();
    assert_eq!(
        c.co.recover(&Context::new()).unwrap().head(),
        account.head()
    );
    assert!(c
        .co
        .with_authenticated(&Context::new(), b"wrong", |_| {
            called.set(true);
            Ok::<_, ()>(())
        })
        .is_err());
    assert!(!called.get());
    assert_eq!(
        c.co.with_authenticated(&Context::new(), b"PRIVATE_PASSWORD", |proof| {
            assert_eq!(proof.principal(), &c.expected.principal);
            Ok::<_, ()>("observed")
        })
        .unwrap()
        .unwrap(),
        "observed"
    );
    assert_eq!(
        rusqlite::Connection::open(&c.path)
            .unwrap()
            .query_row("SELECT count(*) FROM enrollment_events", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        3
    );
    c.co.close().unwrap();
}
#[test]
fn personal_coordinator_wrong_proof_cancel_missing_or_altered_checkpoint_never_adopts_state() {
    let c = Case::new();
    let ctx = Context::new();
    assert!(c.co.recover(&ctx).is_err());
    assert!(!c.witness.path().join("personal-checkpoint.json").exists());
    let mut invalid = c.head.clone();
    invalid.digest = "e".repeat(64);
    assert!(c.co.initialize(&ctx, &invalid).is_err());
    c.co.initialize(&ctx, &c.head).unwrap();
    let initial = c.bytes();
    let e =
        c.co.enroll(&ctx, &c.key, &[99; 32], b"private")
            .unwrap_err();
    assert!(matches!(e.progress.journal, JournalOutcome::NotAttempted));
    assert_eq!(c.bytes(), initial);
    let canceled = Context::new();
    canceled.cancel();
    let e =
        c.co.enroll(&canceled, &c.key, &c.proof, b"private")
            .unwrap_err();
    assert!(matches!(e.progress.journal, JournalOutcome::NotAttempted));
    assert_eq!(c.bytes(), initial);
    let mut data: serde_json::Value = serde_json::from_slice(&initial).unwrap();
    data["installation_digest"] = "f".repeat(64).into();
    fs::write(
        c.witness.path().join("personal-checkpoint.json"),
        serde_json::to_vec(&data).unwrap(),
    )
    .unwrap();
    assert!(c.co.recover(&ctx).is_err());
    assert!(c
        .co
        .with_authenticated(&ctx, b"private", |_| Ok::<_, ()>(()))
        .is_err());
    fs::write(c.witness.path().join("personal-checkpoint.json"), &initial).unwrap();
    assert!(c.co.enroll(&ctx, &c.key, &c.proof, b"private").is_ok());
    c.co.close().unwrap();
}
#[cfg(feature = "test-fixtures")]
#[test]
fn personal_coordinator_each_durable_phase_resumes_exactly_once_without_new_secrets() {
    use std::sync::Arc;
    use swarm_enrollment_authority::personal::coordinator::{Error, FixturePhase};
    for phase in [
        FixturePhase::IntentDurable,
        FixturePhase::JournalApplied,
        FixturePhase::CheckpointDurable,
    ] {
        let control = Case::new();
        control
            .co
            .initialize(&Context::new(), &control.head)
            .unwrap();
        control
            .co
            .enroll(
                &Context::new(),
                &control.key,
                &control.proof,
                b"phase-secret",
            )
            .unwrap();
        control.co.close().unwrap();
        let mut c = Case::new();
        c.co.initialize(&Context::new(), &c.head).unwrap();
        c.co.fixture_hook(Some(Arc::new(move |p| {
            if p == phase {
                Err(Error::Interrupted)
            } else {
                Ok(())
            }
        })));
        let e =
            c.co.enroll(&Context::new(), &c.key, &c.proof, b"phase-secret")
                .unwrap_err();
        assert_eq!(e.progress.intent, Publication::Durable);
        if phase == FixturePhase::IntentDurable {
            assert!(matches!(e.progress.journal, JournalOutcome::NotAttempted))
        } else {
            assert!(matches!(
                e.progress.journal,
                JournalOutcome::Acknowledged(_)
            ))
        }
        c.reopen();
        let a = c.co.recover(&Context::new()).unwrap();
        assert_eq!(c.co.recover(&Context::new()).unwrap().head(), a.head());
        assert!(c
            .co
            .with_authenticated(&Context::new(), b"phase-secret", |_| Ok::<_, ()>(()))
            .unwrap()
            .is_ok());
        assert_eq!(
            rusqlite::Connection::open(&c.path)
                .unwrap()
                .query_row("SELECT count(*) FROM enrollment_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            3
        );
        c.co.close().unwrap();
    }
}
#[cfg(feature = "test-fixtures")]
#[test]
fn personal_coordinator_file_failure_preserves_original_and_postapply_acknowledgement() {
    use std::sync::Arc;
    use swarm_enrollment_authority::personal::coordinator::FixturePhase;
    use swarm_enrollment_config::store::FixtureFault as F;
    let c = Case::new();
    let ctx = Context::new();
    c.co.initialize(&ctx, &c.head).unwrap();
    let original = c.bytes();
    c.documents.fixture_faults(&[F::PartialWrite]);
    let e = c.co.enroll(&ctx, &c.key, &c.proof, b"private").unwrap_err();
    assert_eq!(e.progress.intent, Publication::NotAttempted);
    assert!(matches!(e.progress.journal, JournalOutcome::NotAttempted));
    assert_eq!(c.bytes(), original);
    c.documents.fixture_faults(&[]);
    let d = c.documents.clone();
    c.co.fixture_hook(Some(Arc::new(move |p| {
        if p == FixturePhase::JournalApplied {
            d.fixture_faults(&[F::DirectorySync])
        }
        Ok(())
    })));
    let e = c.co.enroll(&ctx, &c.key, &c.proof, b"private").unwrap_err();
    assert!(matches!(
        e.progress.journal,
        JournalOutcome::Acknowledged(_)
    ));
    assert_eq!(e.progress.checkpoint, Publication::Visible);
    c.documents.fixture_faults(&[]);
    c.co.fixture_hook(None);
    assert!(c.co.recover(&ctx).is_ok());
    c.co.close().unwrap();
}
#[test]
fn personal_coordinator_releases_files_before_callback_and_keeps_after_use_outcomes() {
    let c = Case::new();
    let ctx = Context::new();
    c.co.initialize(&ctx, &c.head).unwrap();
    c.co.enroll(&ctx, &c.key, &c.proof, b"private").unwrap();
    let files = swarm_enrollment_config::store::Context::new();
    let observed = std::cell::Cell::new(false);
    let e =
        c.co.with_authenticated(&ctx, b"private", |_| {
            let r = c.documents.reserve(&files).unwrap();
            assert!(!r.read("personal-checkpoint.json").unwrap().is_empty());
            r.finish().unwrap();
            observed.set(true);
            ctx.cancel();
            Ok::<_, ()>("effect")
        })
        .unwrap_err();
    assert!(observed.get());
    match e {
        UseError::AfterUse {
            outcome: Ok(value), ..
        } => assert_eq!(value, "effect"),
        _ => panic!("after-use outcome lost"),
    }
    assert!(c
        .co
        .with_authenticated(&Context::new(), b"private", |_| Err::<(), _>(
            "business refusal"
        ))
        .unwrap()
        .is_err());
    let failed = Context::new();
    let e =
        c.co.with_authenticated(&failed, b"private", |_| {
            failed.cancel();
            Err::<(), _>("PRIVATE_BUSINESS_ERROR")
        })
        .unwrap_err();
    assert!(!format!("{e:?}").contains("PRIVATE_BUSINESS_ERROR"));
    match e {
        UseError::AfterUse {
            outcome: Err(value),
            ..
        } => assert_eq!(value, "PRIVATE_BUSINESS_ERROR"),
        _ => panic!("failed callback outcome lost"),
    }
    c.co.close().unwrap();
}

/// Match the producer's exact field order so context/phase faults are not hidden
/// by unrelated noncanonical JSON. This schema is test data, not another codec.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    schema_version: i64,
    installation_digest: String,
    principal: swarm_identity_contract::Principal,
    generation: i64,
    authority_public_key: String,
    project_digest: String,
    head: Head,
    credential_digest: Option<String>,
    pending: Option<String>,
}
#[test]
fn personal_coordinator_canonical_context_faults_and_coherent_witness_rollback_refuse() {
    let c = Case::new();
    let ctx = Context::new();
    c.co.initialize(&ctx, &c.head).unwrap();
    let initial = c.bytes();
    let valid: Stored = serde_json::from_slice(&initial).unwrap();
    assert_eq!(swarm_wire_json::encode(&valid).unwrap(), initial);
    for variant in 0..7 {
        assert!(matches!(
            c.co.recover(&ctx).unwrap_err().cause,
            swarm_enrollment_authority::personal::coordinator::Error::NoPending
        ));
        let mut d: Stored = serde_json::from_slice(&initial).unwrap();
        match variant {
            0 => d.schema_version = 2,
            1 => d.installation_digest = "f".repeat(64),
            2 => d.principal.subject = "different".into(),
            3 => d.generation += 1,
            4 => d.authority_public_key = "A".repeat(43),
            5 => d.project_digest = "e".repeat(64),
            _ => d.head.sequence = -1,
        }
        let changed = swarm_wire_json::encode(&d).unwrap();
        fs::write(c.witness.path().join("personal-checkpoint.json"), &changed).unwrap();
        let e = c.co.enroll(&ctx, &c.key, &c.proof, b"private").unwrap_err();
        assert_eq!(e.cause.code(), "personal-checkpoint", "variant{variant}");
        assert!(matches!(e.progress.journal, JournalOutcome::NotAttempted));
        assert_eq!(c.bytes(), changed);
        fs::write(c.witness.path().join("personal-checkpoint.json"), &initial).unwrap();
    }
    let a = c.co.enroll(&ctx, &c.key, &c.proof, b"private").unwrap();
    let committed = c.bytes();
    fs::write(c.witness.path().join("personal-checkpoint.json"), &initial).unwrap();
    assert_eq!(
        c.co.recover(&ctx).unwrap_err().cause.code(),
        "personal-no-pending"
    );
    let e = c.co.enroll(&ctx, &c.key, &c.proof, b"other").unwrap_err();
    assert_eq!(e.cause.code(), "head");
    assert_eq!(c.bytes(), initial);
    assert!(c
        .co
        .with_authenticated(&ctx, b"private", |_| Ok::<_, ()>(()))
        .is_err());
    fs::write(
        c.witness.path().join("personal-checkpoint.json"),
        &committed,
    )
    .unwrap();
    assert_eq!(c.co.recover(&ctx).unwrap().head(), a.head());
    c.co.close().unwrap();
}
#[test]
fn personal_coordinator_actual_writer_contention_recovers_and_account_is_held_until_use_finishes() {
    use std::sync::{mpsc, Arc};
    let f = Fixture::new();
    let writer = Arc::new(f.bound());
    let c = Case::from_fixture(f);
    let ctx = Context::new();
    c.co.initialize(&ctx, &c.head).unwrap();
    let initial = c.bytes();
    let (ready, wait_ready) = mpsc::channel();
    let (release, wait_release) = mpsc::channel();
    let w = writer.clone();
    let head = c.head.clone();
    let held = std::thread::spawn(move || {
        w.with_active(
            &JournalContext::new(),
            &head,
            swarm_enrollment_authority::personal::BOOTSTRAP_ID,
            |_| {
                ready.send(()).unwrap();
                wait_release.recv().unwrap();
                Ok(())
            },
        )
    });
    wait_ready.recv().unwrap();
    let attempted = c.co.enroll(&ctx, &c.key, &c.proof, b"private");
    let observed = c.bytes();
    release.send(()).unwrap();
    held.join().unwrap().unwrap();
    let e = attempted.unwrap_err();
    match &e.cause {
        swarm_enrollment_authority::personal::coordinator::Error::Personal(
            swarm_enrollment_authority::personal::Error::State(cause),
        ) => assert!(cause.is_busy()),
        other => panic!("expected original SQLite contention cause: {other}"),
    }
    assert!(matches!(e.progress.journal, JournalOutcome::NotAttempted));
    assert_eq!(observed, initial);
    let a = c.co.enroll(&ctx, &c.key, &c.proof, b"private").unwrap();
    let revoke = swarm_enrollment_state::sign_change(
        &c.key,
        [43; 32],
        a.head(),
        "revoke",
        swarm_enrollment_authority::personal::ACCOUNT_ID,
        b"",
    )
    .unwrap();
    let co = Arc::new(c.co);
    let in_use = co.clone();
    let (ready, wait_ready) = mpsc::channel();
    let (release, wait_release) = mpsc::channel();
    let held = std::thread::spawn(move || {
        in_use.with_authenticated(&Context::new(), b"private", |_| {
            ready.send(()).unwrap();
            wait_release.recv().unwrap();
            Ok::<_, ()>("completed")
        })
    });
    wait_ready.recv().unwrap();
    let attempted = writer.apply_batch(&JournalContext::new(), a.head(), &[&revoke]);
    release.send(()).unwrap();
    assert_eq!(held.join().unwrap().unwrap().unwrap(), "completed");
    assert!(attempted.unwrap_err().is_busy());
    writer
        .apply_batch(&JournalContext::new(), a.head(), &[&revoke])
        .unwrap();
    let called = std::cell::Cell::new(false);
    assert!(co
        .with_authenticated(&Context::new(), b"private", |_| {
            called.set(true);
            Ok::<_, ()>(())
        })
        .is_err());
    assert!(!called.get());
    writer.close().unwrap();
    co.close().unwrap();
}

/// Parent-selected isolated fixture roots and independent context. No key or
/// authority is learned from the database; fixed signing seed is fixture-only.
#[cfg(feature = "test-fixtures")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    journal: PathBuf,
    witness: PathBuf,
    installation: String,
    principal: swarm_identity_contract::Principal,
    generation: i64,
    phase: Option<String>,
}
#[cfg(feature = "test-fixtures")]
#[test]
fn personal_coordinator_actual_process_exit_after_each_durable_phase_recovers_once() {
    use std::sync::Arc;
    use swarm_enrollment_authority::personal::coordinator::FixturePhase;
    const PROBE: &str = "SWARM_PAEA_PERSONAL_CRASH_PROBE";
    if let Ok(raw) = std::env::var(PROBE) {
        let p: Probe = serde_json::from_str(&raw).unwrap();
        let key = SigningKey::from_bytes(&[41; 32]);
        let source = Source::open(
            &JournalContext::new(),
            &p.journal,
            &key.verifying_key().to_bytes(),
            [43; 32],
            Expected {
                installation_digest: p.installation,
                principal: p.principal,
                generation: p.generation,
            },
        )
        .unwrap();
        let docs = Documents::open(&p.witness, rustix::process::geteuid().as_raw()).unwrap();
        let co = Coordinator::new(source, docs);
        co.fixture_hook(Some(Arc::new(move |phase| {
            if p.phase
                .as_ref()
                .is_some_and(|name| *name == format!("{phase:?}"))
            {
                std::process::exit(73)
            }
            Ok(())
        })));
        co.enroll(&Context::new(), &key, &[42; 32], b"process-fixture-secret")
            .unwrap();
        co.close().unwrap();
        println!("CHILD_CONTROL_COMPLETED");
        return;
    }
    // A valid actual subprocess control precedes each real abrupt-exit variant.
    for phase in [
        FixturePhase::IntentDurable,
        FixturePhase::JournalApplied,
        FixturePhase::CheckpointDurable,
    ] {
        for crash in [false, true] {
            let mut c = Case::new();
            c.co.initialize(&Context::new(), &c.head).unwrap();
            c.co.close().unwrap();
            let input = Probe {
                journal: c.path.clone(),
                witness: c.witness.path().to_owned(),
                installation: c.expected.installation_digest.clone(),
                principal: c.expected.principal.clone(),
                generation: c.expected.generation,
                phase: crash.then(|| format!("{phase:?}")),
            };
            let out=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","personal_coordinator_actual_process_exit_after_each_durable_phase_recovers_once","--nocapture"]).env(PROBE,serde_json::to_string(&input).unwrap()).output().unwrap();
            println!(
                "PROCESS_PROBE phase={phase:?} crash={crash} exit={:?}",
                out.status.code()
            );
            print!("{}", String::from_utf8_lossy(&out.stdout));
            eprint!("{}", String::from_utf8_lossy(&out.stderr));
            assert_eq!(out.status.code(), Some(if crash { 73 } else { 0 }));
            if !crash {
                assert!(String::from_utf8_lossy(&out.stdout).contains("CHILD_CONTROL_COMPLETED"))
            }
            c.reopen();
            let a = c.co.recover(&Context::new()).unwrap();
            assert_eq!(c.co.recover(&Context::new()).unwrap().head(), a.head());
            assert!(c
                .co
                .with_authenticated(&Context::new(), b"process-fixture-secret", |_| Ok::<_, ()>(
                    ()
                ))
                .unwrap()
                .is_ok());
            assert_eq!(
                rusqlite::Connection::open(&c.path)
                    .unwrap()
                    .query_row("SELECT count(*) FROM enrollment_events", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                3
            );
            c.co.close().unwrap();
        }
    }
}

#[cfg(feature = "test-fixtures")]
#[test]
fn personal_coordinator_requires_checkpoint_sync_before_business_callback_then_recovers() {
    use swarm_enrollment_config::store::FixtureFault as F;
    let c = Case::new();
    let ctx = Context::new();
    c.co.initialize(&ctx, &c.head).unwrap();
    c.co.enroll(&ctx, &c.key, &c.proof, b"private").unwrap();
    assert!(c
        .co
        .with_authenticated(&ctx, b"private", |_| Ok::<_, ()>(()))
        .unwrap()
        .is_ok());
    let saved = c.bytes();
    let called = std::cell::Cell::new(false);
    c.documents.fixture_faults(&[F::DirectorySync]);
    let result = c.co.with_authenticated(&ctx, b"private", |_| {
        called.set(true);
        Ok::<_, ()>(())
    });
    assert!(
        result.is_err(),
        "checkpoint durability must be acknowledged before use"
    );
    assert!(!called.get());
    match result.unwrap_err() {
        UseError::Before(e) => assert_eq!(e.code(), "indeterminate"),
        other => panic!("unexpected after-use result: {other}"),
    }
    assert_eq!(c.bytes(), saved);
    c.documents.fixture_faults(&[]);
    assert!(c
        .co
        .with_authenticated(&Context::new(), b"private", |_| Ok::<_, ()>(()))
        .unwrap()
        .is_ok());
    c.documents.fixture_faults(&[F::Unlock]);
    let called = std::cell::Cell::new(false);
    let released = c.co.with_authenticated(&Context::new(), b"private", |_| {
        called.set(true);
        Ok::<_, ()>(())
    });
    assert!(released.is_err());
    assert!(!called.get());
    c.documents.fixture_faults(&[]);
    assert!(c
        .co
        .with_authenticated(&Context::new(), b"private", |_| Ok::<_, ()>(()))
        .unwrap()
        .is_ok());
    c.co.close().unwrap();
}
