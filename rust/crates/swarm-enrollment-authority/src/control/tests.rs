use super::*;
use crate::{sign_enrollment, Document};
use ed25519_dalek::SigningKey;
use state::{sign_change, Context, Journal};
use std::{fs, sync::mpsc};
use time::Duration;

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
    config: PathBuf,
    witness: PathBuf,
    key: SigningKey,
    journal: Journal,
    head: Head,
    doc: Document,
    pins: Pins,
}
/// Build explicit, non-authoritative local fixture data; no I/O or grant is issued.
fn change(id: &str, head: config::Head) -> Change {
    Change {
        schema_version: 1,
        event_id: id.into(),
        expected_head: head,
        values: config::Values {
            mode: "local".into(),
            socket_path: "/run/swarm-fixture/control.sock".into(),
        },
        reason: "explicit fixture change".into(),
    }
}
/// Assert the public response kind, retaining the full view; panic on a protocol mismatch.
fn view(response: Response) -> config::View {
    match response {
        Response::View(v) => v,
        other => panic!("expected view: {other:?}"),
    }
}
impl Fixture {
    /// Create disposable real storage and a synthetic one-hour signed grant.
    /// Fixture-only keys/identity prove no human authentication; setup failures panic.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.sqlite");
        let config = dir.path().join("config");
        let witness = dir.path().join("witness");
        fs::create_dir(&config).unwrap();
        fs::create_dir(&witness).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&witness, fs::Permissions::from_mode(0o700)).unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        let principal = Principal {
            id: "alice".into(),
            kind: "human".into(),
            issuer: "https://fixture.invalid".into(),
            subject: "employee[eu]".into(),
        };
        let pins = Pins {
            installation_digest: "ab".repeat(32),
            public_key: URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes()),
            principal: principal.clone(),
        };
        let c = change("one", config::Head::default());
        let now = OffsetDateTime::now_utc();
        let doc = Document {
            schema_version: 1,
            enrollment_id: "alice-control".into(),
            generation: 1,
            project_digest: "0b".repeat(32),
            principal,
            scope: Scope {
                work_id: "work".into(),
                plan_version: 3,
                plan_digest: "cd".repeat(32),
            },
            trust_revision: 2,
            permissions: [
                Request::Read,
                Request::Preview(c.clone()),
                Request::Create(c.clone()),
                Request::Apply(c),
            ]
            .iter()
            .map(|r| permission(&pins.installation_digest, r).unwrap())
            .collect(),
            delegable: vec![],
            not_before: now - Duration::hours(1),
            expires_at: now + Duration::hours(1),
        };
        let journal = Journal::open(
            &Context::new(),
            &path,
            &key.verifying_key().to_bytes(),
            [11; 32],
        )
        .unwrap();
        let raw = sign_enrollment(&key, [11; 32], &Head::default(), &doc).unwrap();
        let head = journal
            .apply(&Context::new(), &Head::default(), &raw)
            .unwrap();
        Self {
            _dir: dir,
            path,
            config,
            witness,
            key,
            journal,
            head,
            doc,
            pins,
        }
    }
    /// Copy fixture-owned pins/context for the public constructor, without authenticating them.
    fn startup(&self) -> Startup {
        Startup {
            journal: self.path.clone(),
            authority_key: self.key.verifying_key().to_bytes(),
            project: [11; 32],
            configuration_directory: self.config.clone(),
            witness_directory: self.witness.clone(),
            owner: rustix::process::geteuid().as_raw(),
            pins: self.pins.clone(),
            binding: Binding {
                enrollment_id: self.doc.enrollment_id.clone(),
                generation: self.doc.generation,
                principal: self.doc.principal.clone(),
                scope: self.doc.scope.clone(),
                trust_revision: self.doc.trust_revision,
            },
        }
    }
    /// Open the public core on this isolated fixture; panic on an invalid prerequisite.
    fn control(&self) -> Control {
        Control::open(&Context::new(), self.startup()).unwrap()
    }
    /// Submit an actual signed revocation at the fixture head; preserve Busy for explicit recovery.
    fn revoke(&self) -> Result<Head, state::Error> {
        let raw = sign_change(
            &self.key,
            [11; 32],
            &self.head,
            "revoke",
            &self.doc.enrollment_id,
            b"",
        )
        .unwrap();
        self.journal.apply(&Context::new(), &self.head, &raw)
    }
    /// Invoke the public API with fresh cancellation tokens and the fixture CAS head.
    fn call(&self, c: &Control, r: Request) -> Result<Response, Error> {
        c.execute(&Context::new(), &store::Context::new(), &self.head, r)
    }
    /// Replace only this disposable journal to construct an invalid/expired grant.
    /// Must precede opening any Control; setup/signature failures panic, never touch live state.
    fn rewrite_before_open(&mut self, edit: impl FnOnce(&mut Document)) {
        self.journal.close().unwrap();
        fs::remove_file(&self.path).unwrap();
        self.journal = Journal::open(
            &Context::new(),
            &self.path,
            &self.key.verifying_key().to_bytes(),
            [11; 32],
        )
        .unwrap();
        edit(&mut self.doc);
        let raw = sign_enrollment(&self.key, [11; 32], &Head::default(), &self.doc).unwrap();
        self.head = self
            .journal
            .apply(&Context::new(), &Head::default(), &raw)
            .unwrap();
    }
}
#[test]
fn actual_preview_create_read_apply_exact_replay_and_reopen() {
    let f = Fixture::new();
    let c = f.control();
    let one = change("one", config::Head::default());
    let before: Vec<_> = fs::read_dir(&f.config)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let p = f.call(&c, Request::Preview(one.clone())).unwrap();
    assert!(
        matches!(p, Response::Preview(ref p) if p.base.revision == 0 && p.proposed.head.revision == 1)
    );
    let after: Vec<_> = fs::read_dir(&f.config)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(before, after, "preview publishes no configuration");
    let created = view(f.call(&c, Request::Create(one.clone())).unwrap());
    assert_eq!(created.snapshot.history[0].actor, f.doc.principal);
    assert_eq!(view(f.call(&c, Request::Read).unwrap()), created);
    assert_eq!(view(f.call(&c, Request::Create(one)).unwrap()), created);
    let mut two = change("two", created.head);
    two.values.socket_path = "/run/swarm-fixture/changed.sock".into();
    let updated = view(f.call(&c, Request::Apply(two.clone())).unwrap());
    assert_eq!(updated.snapshot.history.len(), 2);
    assert_eq!(view(f.call(&c, Request::Apply(two)).unwrap()), updated);
    c.close().unwrap();
    assert_eq!(f.call(&c, Request::Read).unwrap_err().code(), "closed");
    let reopened = f.control();
    assert_eq!(view(f.call(&reopened, Request::Read).unwrap()), updated);
    reopened.close().unwrap();
}
#[test]
fn every_operation_requires_its_exact_signed_permission_including_replay_and_read() {
    let mut f = Fixture::new();
    f.rewrite_before_open(|d| d.permissions.retain(|p| p.resource.ends_with("/create")));
    let c = f.control();
    let one = change("one", config::Head::default());
    let created = view(f.call(&c, Request::Create(one.clone())).unwrap());
    for r in [
        Request::Read,
        Request::Preview(one.clone()),
        Request::Apply(one.clone()),
    ] {
        assert_eq!(f.call(&c, r).unwrap_err().code(), "permission");
    }
    assert_eq!(view(f.call(&c, Request::Create(one)).unwrap()), created);
    c.close().unwrap();
}
#[test]
fn wrong_binding_expiry_and_future_enrollment_refuse_before_storage_then_recover() {
    let f = Fixture::new();
    let control = f.control();
    assert!(matches!(
        f.call(
            &control,
            Request::Preview(change("control", config::Head::default()))
        ),
        Ok(Response::Preview(_))
    ));
    control.close().unwrap();
    for variant in 0..4 {
        let mut startup = f.startup();
        match variant {
            0 => startup.binding.generation += 1,
            1 => startup.binding.scope.plan_version += 1,
            2 => startup.binding.trust_revision += 1,
            _ => startup.binding.enrollment_id = "other".into(),
        }
        let c = Control::open(&Context::new(), startup).unwrap();
        let e = f
            .call(&c, Request::Create(change("one", config::Head::default())))
            .unwrap_err();
        assert!(!e.effect_unknown());
        assert!(matches!(e.code(), "binding" | "missing"));
        c.close().unwrap();
    }
    for future in [false, true] {
        let mut expired = Fixture::new();
        expired.rewrite_before_open(|d| {
            if future {
                d.not_before = OffsetDateTime::now_utc() + Duration::hours(1);
                d.expires_at = d.not_before + Duration::hours(1);
            } else {
                d.expires_at = OffsetDateTime::now_utc() - Duration::hours(1);
                d.not_before = d.expires_at - Duration::hours(1);
            }
        });
        let c = expired.control();
        let e = expired
            .call(&c, Request::Create(change("one", config::Head::default())))
            .unwrap_err();
        assert_eq!(e.code(), "inactive");
        assert!(!e.effect_unknown());
        assert_eq!(
            c.coordinator
                .read(&store::Context::new())
                .unwrap_err()
                .code(),
            "not_configured"
        );
        c.close().unwrap();
    }
    let c = f.control();
    assert!(f
        .call(&c, Request::Create(change("one", config::Head::default())))
        .is_ok());
    c.close().unwrap();
}
#[test]
fn revoke_before_use_refuses_old_and_current_heads_without_configuration() {
    let f = Fixture::new();
    let c = f.control();
    assert!(f
        .call(
            &c,
            Request::Preview(change("control", config::Head::default()))
        )
        .is_ok());
    let revoked = f.revoke().unwrap();
    assert_eq!(
        f.call(&c, Request::Create(change("one", config::Head::default())))
            .unwrap_err()
            .code(),
        "head"
    );
    let e = c
        .execute(
            &Context::new(),
            &store::Context::new(),
            &revoked,
            Request::Create(change("one", config::Head::default())),
        )
        .unwrap_err();
    assert_eq!(e.code(), "revoked");
    assert!(!e.effect_unknown());
    assert_eq!(
        c.coordinator
            .read(&store::Context::new())
            .unwrap_err()
            .code(),
        "not_configured"
    );
    c.close().unwrap();
}
#[test]
fn revocation_and_close_contend_during_real_storage_then_explicit_revoke_wins() {
    let f = Fixture::new();
    let c = Arc::new(f.control());
    assert!(f
        .call(
            &c,
            Request::Preview(change("control", config::Head::default()))
        )
        .is_ok());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    c.coordinator
        .fixture_hook(Some(Arc::new(move |phase, _, _| {
            if phase == store::FixturePhase::Prepared {
                entered_tx.send(()).unwrap();
                release_rx.lock().unwrap().recv().unwrap();
            }
        })));
    let worker = c.clone();
    let head = f.head.clone();
    let task = std::thread::spawn(move || {
        worker.execute(
            &Context::new(),
            &store::Context::new(),
            &head,
            Request::Create(change("one", config::Head::default())),
        )
    });
    entered_rx.recv().unwrap();
    let revoke = f.revoke();
    let read = f.call(&c, Request::Read);
    let close = c.close();
    release_tx.send(()).unwrap();
    let made = view(task.join().unwrap().unwrap());
    assert!(revoke.unwrap_err().is_busy());
    assert_eq!(read.unwrap_err().code(), "busy");
    assert_eq!(close.unwrap_err().code(), "busy");
    assert_eq!(made.snapshot.history.len(), 1);
    c.coordinator.fixture_hook(None);
    assert_eq!(view(f.call(&c, Request::Read).unwrap()), made);
    let revoked = f.revoke().unwrap();
    assert_eq!(
        c.execute(
            &Context::new(),
            &store::Context::new(),
            &revoked,
            Request::Read
        )
        .unwrap_err()
        .code(),
        "revoked"
    );
    c.close().unwrap();
}
#[test]
fn authority_failure_after_publication_is_unknown_preserves_result_and_exact_replay() {
    let f = Fixture::new();
    let c = f.control();
    let one = change("one", config::Head::default());
    assert!(f.call(&c, Request::Preview(one.clone())).is_ok());
    let ctx = Context::new();
    let canceled = ctx.clone();
    c.coordinator
        .fixture_hook(Some(Arc::new(move |phase, _, _| {
            if phase == store::FixturePhase::Confirming {
                canceled.cancel();
            }
        })));
    let error = c
        .execute(
            &ctx,
            &store::Context::new(),
            &f.head,
            Request::Create(one.clone()),
        )
        .unwrap_err();
    assert!(error.effect_unknown());
    match error {
        Error::AfterStorage { cause, storage, .. } => {
            assert_eq!(cause.code(), "canceled");
            assert!(storage.is_ok());
        }
        e => panic!("{e:?}"),
    }
    c.coordinator.fixture_hook(None);
    let observed = view(f.call(&c, Request::Read).unwrap());
    assert_eq!(observed.snapshot.history.len(), 1);
    assert_eq!(view(f.call(&c, Request::Create(one)).unwrap()), observed);
    f.revoke().unwrap();
    assert_eq!(f.call(&c, Request::Read).unwrap_err().code(), "head");
    c.close().unwrap();
}
#[test]
fn storage_failure_is_retained_cancellation_recovers_without_duplicate_or_lingering_permit() {
    let f = Fixture::new();
    let c = f.control();
    let one = change("one", config::Head::default());
    assert!(f.call(&c, Request::Preview(one.clone())).is_ok());
    let ctx = store::Context::new();
    ctx.cancel();
    let error = c
        .execute(&Context::new(), &ctx, &f.head, Request::Create(one.clone()))
        .unwrap_err();
    assert!(error.effect_unknown());
    assert!(matches!(error, Error::Storage { ref cause, .. } if cause.contains_code("canceled")));
    assert!(c.permit.lock().unwrap().is_none());
    assert_eq!(
        c.coordinator
            .read(&store::Context::new())
            .unwrap_err()
            .code(),
        "not_configured"
    );
    let made = view(f.call(&c, Request::Create(one.clone())).unwrap());
    assert_eq!(view(f.call(&c, Request::Create(one)).unwrap()), made);
    c.close().unwrap();
}
#[test]
fn startup_foreign_pins_or_key_refuse_without_creating_journal() {
    let f = Fixture::new();
    let c = f.control();
    c.close().unwrap();
    let mut s = f.startup();
    s.pins.principal.subject = "other".into();
    assert!(matches!(
        Control::open(&Context::new(), s),
        Err(Error::Binding)
    ));
    let mut s = f.startup();
    s.authority_key = SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes();
    assert!(matches!(
        Control::open(&Context::new(), s),
        Err(Error::Binding)
    ));
    let mut s = f.startup();
    s.journal = f.path.with_file_name("missing.sqlite");
    let absent = s.journal.clone();
    assert!(Control::open(&Context::new(), s).is_err());
    assert!(!absent.exists());
    let c = f.control();
    assert!(f
        .call(&c, Request::Create(change("one", config::Head::default())))
        .is_ok());
    c.close().unwrap();
}

#[test]
fn both_authority_and_storage_failures_survive_pending_exact_event_recovery() {
    let f = Fixture::new();
    let c = f.control();
    let first = view(
        f.call(
            &c,
            Request::Create(change("control", config::Head::default())),
        )
        .unwrap(),
    );
    let two = change("two", first.head);
    let ctx = Context::new();
    let canceled = ctx.clone();
    c.coordinator
        .fixture_hook(Some(Arc::new(move |phase, _, witness| {
            if phase == store::FixturePhase::Confirming {
                canceled.cancel();
                witness.fixture_faults(&[store::FixtureFault::PartialWrite]);
            }
        })));
    let error = c
        .execute(
            &ctx,
            &store::Context::new(),
            &f.head,
            Request::Apply(two.clone()),
        )
        .unwrap_err();
    assert!(error.effect_unknown());
    match error {
        Error::AfterStorage { cause, storage, .. } => {
            assert_eq!(cause.code(), "canceled");
            assert!(storage.unwrap_err().contains_code("storage"));
        }
        e => panic!("{e:?}"),
    }
    c.coordinator.fixture_hook(None);
    c.coordinator.fixture_store_faults(&[], &[]);
    assert_eq!(f.call(&c, Request::Read).unwrap_err().code(), "pending");
    let recovered = view(f.call(&c, Request::Apply(two.clone())).unwrap());
    assert_eq!(recovered.snapshot.history.len(), 2);
    assert_eq!(
        view(f.call(&c, Request::Apply(two.clone())).unwrap()),
        recovered
    );
    assert_eq!(view(f.call(&c, Request::Read).unwrap()), recovered);
    f.revoke().unwrap();
    assert_eq!(f.call(&c, Request::Apply(two)).unwrap_err().code(), "head");
    c.close().unwrap();
}
