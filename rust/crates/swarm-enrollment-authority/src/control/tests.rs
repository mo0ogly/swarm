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

/// A caller may retain the exact private response after authority failure, but
/// ordinary diagnostics must never publish that response's configuration fields.
#[test]
fn error_debug_keeps_private_after_storage_response_structured_and_redacts_text() {
    let f = Fixture::new();
    let c = f.control();
    let mut private = change("private", config::Head::default());
    private.reason = "PRIVATE_RESPONSE_REASON".into();
    let response = f.call(&c, Request::Create(private)).unwrap();
    let e = Error::AfterStorage {
        cause: state::Error::Canceled,
        storage: Box::new(Ok(response)),
        effect_unknown: true,
    };
    assert!(
        !format!("{e:?}").contains("PRIVATE_RESPONSE_REASON"),
        "ordinary Debug leaked private response"
    );
    match e {
        Error::AfterStorage { storage, .. } => assert_eq!(
            view(storage.unwrap()).snapshot.history[0].change.reason,
            "PRIVATE_RESPONSE_REASON"
        ),
        _ => unreachable!(),
    }
    c.close().unwrap();
    f.journal.close().unwrap();
}

/// Personal fixture on separate real journal/witness roots. Explicit keys/costs
/// and human labels are synthetic, never a live account or authentication claim.
struct PersonalFixture {
    _journal_root: tempfile::TempDir,
    _witness_root: tempfile::TempDir,
    co: crate::personal::coordinator::Coordinator,
    documents: config::store::documents::Documents,
    journal: Journal,
    key: SigningKey,
    head: Head,
}
impl PersonalFixture {
    /// Initialize a signed bootstrap and actual password record with generation7;
    /// business generation1 is deliberately different, not an identity mismatch.
    fn new(installation: &str, principal: &Principal) -> Self {
        use crate::{credentials::Parameters, personal as p};
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let witness = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(witness.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = root.path().join("personal.sqlite");
        let key = SigningKey::from_bytes(&[41; 32]);
        let bootstrap = p::Bootstrap {
            schema_version: 1,
            installation_digest: installation.into(),
            principal: principal.clone(),
            generation: 7,
            parameters: Parameters {
                memory_kib: 32,
                passes: 1,
                parallelism: 1,
            },
            proof_commitment: p::commitment(installation, &[42; 32]).unwrap(),
        };
        let journal = Journal::open(
            &Context::new(),
            &path,
            &key.verifying_key().to_bytes(),
            [51; 32],
        )
        .unwrap();
        let raw = state::sign_change(
            &key,
            [51; 32],
            &Head::default(),
            "enroll",
            p::BOOTSTRAP_ID,
            &p::encode_bootstrap(&bootstrap).unwrap(),
        )
        .unwrap();
        let head = journal
            .apply(&Context::new(), &Head::default(), &raw)
            .unwrap();
        let source = p::Source::open(
            &Context::new(),
            &path,
            &key.verifying_key().to_bytes(),
            [51; 32],
            p::Expected {
                installation_digest: installation.into(),
                principal: principal.clone(),
                generation: 7,
            },
        )
        .unwrap();
        let docs = config::store::documents::Documents::open(
            witness.path(),
            rustix::process::geteuid().as_raw(),
        )
        .unwrap();
        let co = p::coordinator::Coordinator::new(source, docs.clone());
        co.initialize(&p::coordinator::Context::new(), &head)
            .unwrap();
        let account = co
            .enroll(
                &p::coordinator::Context::new(),
                &key,
                &[42; 32],
                b"PERSONAL_FIXTURE_PASSWORD",
            )
            .unwrap();
        Self {
            _journal_root: root,
            _witness_root: witness,
            co,
            documents: docs,
            journal,
            key,
            head: account.head().clone(),
        }
    }
    /// Actual current signed revocation, no caller actor/grant override.
    fn revoke(&self) -> Head {
        let raw = state::sign_change(
            &self.key,
            [51; 32],
            &self.head,
            "revoke",
            crate::personal::ACCOUNT_ID,
            b"",
        )
        .unwrap();
        self.journal
            .apply(&Context::new(), &self.head, &raw)
            .unwrap()
    }
}
#[test]
fn authenticated_composition_real_account_and_grant_apply_once_without_generation_alias() {
    use crate::authenticated_control::{AuthenticatedControl, Context as ExecuteContext};
    let f = Fixture::new();
    let h = PersonalFixture::new(&f.pins.installation_digest, &f.pins.principal);
    let c = f.control();
    h.co.with_authenticated(
        &crate::personal::coordinator::Context::new(),
        b"PERSONAL_FIXTURE_PASSWORD",
        |proof| {
            assert_eq!(proof.installation_digest(), f.pins.installation_digest);
            assert_eq!(proof.generation(), 7);
            Ok::<_, ()>(())
        },
    )
    .unwrap()
    .unwrap();
    let app = AuthenticatedControl::new(h.co, c);
    let ctx = ExecuteContext::new();
    let change = change("authenticated", config::Head::default());
    assert!(app
        .execute(&ctx, &f.head, b"wrong", Request::Create(change.clone()))
        .is_err());
    assert!(!f.config.join("config.json").exists());
    let first = view(
        app.execute(
            &ctx,
            &f.head,
            b"PERSONAL_FIXTURE_PASSWORD",
            Request::Create(change.clone()),
        )
        .unwrap()
        .unwrap(),
    );
    let replay = view(
        app.execute(
            &ctx,
            &f.head,
            b"PERSONAL_FIXTURE_PASSWORD",
            Request::Create(change),
        )
        .unwrap()
        .unwrap(),
    );
    assert_eq!(first, replay);
    let read = view(
        app.execute(&ctx, &f.head, b"PERSONAL_FIXTURE_PASSWORD", Request::Read)
            .unwrap()
            .unwrap(),
    );
    assert_eq!(read.snapshot.history.len(), 1);
    assert_eq!(read.snapshot.history[0].actor, f.pins.principal);
    app.close().unwrap();
    h.journal.close().unwrap();
    f.journal.close().unwrap();
}
#[test]
fn authenticated_composition_wrong_person_or_installation_refuses_before_business_storage() {
    use crate::authenticated_control::{AuthenticatedControl, Context as ExecuteContext};
    for wrong_install in [false, true] {
        let f = Fixture::new();
        let mut principal = f.pins.principal.clone();
        let installation = if wrong_install {
            "f".repeat(64)
        } else {
            principal.subject = "other-human".into();
            f.pins.installation_digest.clone()
        };
        let h = PersonalFixture::new(&installation, &principal);
        let app = AuthenticatedControl::new(h.co, f.control());
        let e = app
            .execute(
                &ExecuteContext::new(),
                &f.head,
                b"PERSONAL_FIXTURE_PASSWORD",
                Request::Create(change("refuse", config::Head::default())),
            )
            .unwrap()
            .unwrap_err();
        assert_eq!(e.code(), "personal-binding");
        assert!(!f.config.join("config.json").exists());
        assert!(!f.witness.join("witness.json").exists());
        app.close().unwrap();
        h.journal.close().unwrap();
        f.journal.close().unwrap();
    }
}
#[test]
fn authenticated_composition_current_revoked_grant_and_account_are_both_required() {
    use crate::authenticated_control::{AuthenticatedControl, Context as ExecuteContext};
    let f = Fixture::new();
    let h = PersonalFixture::new(&f.pins.installation_digest, &f.pins.principal);
    let revoked = f.revoke().unwrap();
    let app = AuthenticatedControl::new(h.co, f.control());
    let e = app
        .execute(
            &ExecuteContext::new(),
            &revoked,
            b"PERSONAL_FIXTURE_PASSWORD",
            Request::Create(change("refuse", config::Head::default())),
        )
        .unwrap()
        .unwrap_err();
    assert_eq!(e.code(), "revoked");
    assert!(!f.config.join("config.json").exists());
    app.close().unwrap();
    h.journal.close().unwrap();
    f.journal.close().unwrap();
    let f = Fixture::new();
    let h = PersonalFixture::new(&f.pins.installation_digest, &f.pins.principal);
    h.revoke();
    let app = AuthenticatedControl::new(h.co, f.control());
    assert!(app
        .execute(
            &ExecuteContext::new(),
            &f.head,
            b"PERSONAL_FIXTURE_PASSWORD",
            Request::Create(change("refuse", config::Head::default()))
        )
        .is_err());
    assert!(!f.config.join("config.json").exists());
    app.close().unwrap();
    h.journal.close().unwrap();
    f.journal.close().unwrap();
}
#[test]
fn authenticated_composition_posteffect_errors_keep_inner_storage_and_outer_outcome() {
    use crate::authenticated_control::{AuthenticatedControl, Context as ExecuteContext};
    use crate::personal::coordinator::UseError;
    let f = Fixture::new();
    let h = PersonalFixture::new(&f.pins.installation_digest, &f.pins.principal);
    let c = f.control();
    let ctx = ExecuteContext::new();
    let cancel = ctx.clone();
    c.coordinator
        .fixture_hook(Some(Arc::new(move |phase, _, _| {
            if phase == store::FixturePhase::Confirming {
                cancel.cancel()
            }
        })));
    let app = AuthenticatedControl::new(h.co, c);
    let result = app.execute(
        &ctx,
        &f.head,
        b"PERSONAL_FIXTURE_PASSWORD",
        Request::Create(change("effect", config::Head::default())),
    );
    match result {
        Err(UseError::AfterUse {
            outcome: Err(e), ..
        }) => assert!(e.effect_unknown()),
        other => panic!("effect outcome lost: {other:?}"),
    }
    assert!(f.config.join("config.json").exists());
    let recovered = view(
        app.execute(
            &ExecuteContext::new(),
            &f.head,
            b"PERSONAL_FIXTURE_PASSWORD",
            Request::Create(change("effect", config::Head::default())),
        )
        .unwrap()
        .unwrap(),
    );
    assert_eq!(recovered.snapshot.history.len(), 1);
    app.close().unwrap();
    h.journal.close().unwrap();
    f.journal.close().unwrap();
}

/// Both real authority journals contend while one real configuration publication
/// is held. Release/join before assertions so a failure cannot strand the worker.
#[test]
fn authenticated_composition_holds_both_real_authorities_through_one_effect() {
    use crate::authenticated_control::{AuthenticatedControl, Context as ExecuteContext};
    use std::sync::atomic::{AtomicBool, Ordering};
    let f = Fixture::new();
    let h = PersonalFixture::new(&f.pins.installation_digest, &f.pins.principal);
    let c = f.control();
    let (ready, wait_ready) = mpsc::channel();
    let done = ready.clone();
    let (release, wait_release) = mpsc::channel();
    let wait_release = Mutex::new(wait_release);
    let armed = Arc::new(AtomicBool::new(false));
    let hook_armed = armed.clone();
    c.coordinator
        .fixture_hook(Some(Arc::new(move |phase, _, _| {
            if phase == store::FixturePhase::Confirming && hook_armed.load(Ordering::SeqCst) {
                ready.send("held").unwrap();
                wait_release.lock().unwrap().recv().unwrap()
            }
        })));
    let app = Arc::new(AuthenticatedControl::new(h.co, c));
    let first = view(
        app.execute(
            &ExecuteContext::new(),
            &f.head,
            b"PERSONAL_FIXTURE_PASSWORD",
            Request::Create(change("one", config::Head::default())),
        )
        .unwrap()
        .unwrap(),
    );
    armed.store(true, Ordering::SeqCst);
    let active = app.clone();
    let head = f.head.clone();
    let two = change("two", first.head);
    let worker = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            active.execute(
                &ExecuteContext::new(),
                &head,
                b"PERSONAL_FIXTURE_PASSWORD",
                Request::Apply(two),
            )
        }));
        done.send("completed").unwrap();
        result
    });
    if wait_ready.recv().unwrap() != "held" {
        let early = worker.join().unwrap();
        panic!("operation finished before held boundary: {early:?}");
    }
    let personal_raw = state::sign_change(
        &h.key,
        [51; 32],
        &h.head,
        "revoke",
        crate::personal::ACCOUNT_ID,
        b"",
    )
    .unwrap();
    let account_revoke = h.journal.apply(&Context::new(), &h.head, &personal_raw);
    let grant_revoke = f.revoke();
    release.send(()).unwrap();
    let completed = worker.join().unwrap().unwrap();
    assert!(account_revoke.unwrap_err().is_busy());
    assert!(grant_revoke.unwrap_err().is_busy());
    let observed = view(completed.unwrap().unwrap());
    assert_eq!(observed.snapshot.history.len(), 2);
    armed.store(false, Ordering::SeqCst);
    h.journal
        .apply(&Context::new(), &h.head, &personal_raw)
        .unwrap();
    f.revoke().unwrap();
    assert!(app
        .execute(
            &ExecuteContext::new(),
            &f.head,
            b"PERSONAL_FIXTURE_PASSWORD",
            Request::Read
        )
        .is_err());
    app.close().unwrap();
    h.journal.close().unwrap();
    f.journal.close().unwrap();
}

/// Both actual close paths run, all independent errors stay visible, and a
/// reported close error never silently permits a new operation or state reset.
#[test]
fn authenticated_composition_close_keeps_both_independent_failures_and_recovers_idempotently() {
    use crate::authenticated_control::{AuthenticatedControl, Context as ExecuteContext};
    let f = Fixture::new();
    let h = PersonalFixture::new(&f.pins.installation_digest, &f.pins.principal);
    let c = f.control();
    let response = c
        .execute(
            &Context::new(),
            &store::Context::new(),
            &f.head,
            Request::Preview(change("witness", config::Head::default())),
        )
        .unwrap();
    assert!(matches!(response, Response::Preview(_)));
    h.documents
        .fixture_faults(&[store::FixtureFault::StoreClose]);
    c.coordinator.fixture_store_faults(
        &[store::FixtureFault::StoreClose],
        &[store::FixtureFault::StoreClose],
    );
    let app = AuthenticatedControl::new(h.co, c);
    let e = app.close().unwrap_err();
    assert!(e.personal.is_some() && e.control.is_some());
    assert!(std::error::Error::source(&e).is_some());
    assert!(!format!("{e:?}").contains("PERSONAL_FIXTURE_PASSWORD"));
    assert!(app
        .execute(
            &ExecuteContext::new(),
            &f.head,
            b"PERSONAL_FIXTURE_PASSWORD",
            Request::Read
        )
        .is_err());
    h.documents.fixture_faults(&[]);
    app.close().unwrap();
    assert!(!f.config.join("config.json").exists());
    h.journal.close().unwrap();
    f.journal.close().unwrap();
}
