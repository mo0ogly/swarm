#![cfg(target_os = "linux")]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_enrollment_authority::{
    self as a,
    authenticated_control::Context as ExecuteContext,
    authenticated_startup,
    control::{self, Request, Response},
    personal as p,
};
use swarm_enrollment_bootstrap::{
    self as b, AuthenticatedDocument, ControlDocument, Document, PersonalDeclaration,
};
use swarm_enrollment_config::{
    store::documents::Documents, Change, Head as ConfigHead, Pins, Values,
};
use swarm_enrollment_state::{sign_change, Context, Head, Journal};
use swarm_identity_contract::{Principal, Scope};
use time::{Duration, OffsetDateTime};
/// Complete isolated two-journal installation, explicit keys/costs, no live data.
pub(crate) struct Fixture {
    _root: tempfile::TempDir,
    pub(crate) path: PathBuf,
    pub(crate) doc: AuthenticatedDocument,
    pub(crate) pin: String,
    pub(crate) business: Journal,
    pub(crate) personal: Journal,
}
/// Set permissions on an existing fixture file/directory; creates nothing.
/// Panics if the fixture permission syscall fails, before dependent checks.
pub(crate) fn mode(path: &std::path::Path, bits: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(bits)).unwrap()
}
/// Pure existing config operation used after actual account and grant checks.
pub(crate) fn change() -> Change {
    Change {
        schema_version: 1,
        event_id: "from-startup".into(),
        expected_head: ConfigHead::default(),
        values: Values {
            mode: "local".into(),
            socket_path: "/run/fixture/control.sock".into(),
        },
        reason: "explicit startup fixture".into(),
    }
}
impl Fixture {
    /// Build six private leaf roots, real signed current grant/account, a separate
    /// protected checkpoint and pinned v3declaration. This proves no human presence.
    pub(crate) fn new() -> Self {
        Self::build(true)
    }
    /// Cold signed bootstrap only, without checkpoint/account/lock initialization.
    pub(crate) fn cold() -> Self {
        Self::build(false)
    }
    /// Construct isolated explicit fixture inputs, optionally pre-enrolled for old tests.
    pub(crate) fn build(enrolled: bool) -> Self {
        Self::build_with_action(enrolled, None)
    }
    /// Add an explicit fixture action permission before signing, not at use.
    pub(crate) fn build_with_action(
        enrolled: bool,
        action: Option<swarm_identity_contract::Permission>,
    ) -> Self {
        Self::build_bound(enrolled, action, None)
    }
    /// Bind the same real producers to a consumer-owned current plan/revision.
    pub(crate) fn build_for_trust(
        action: swarm_identity_contract::Permission,
        scope: Scope,
        revision: i64,
    ) -> Self {
        Self::build_bound(true, Some(action), Some((scope, revision)))
    }
    fn build_bound(
        enrolled: bool,
        action: Option<swarm_identity_contract::Permission>,
        binding: Option<(Scope, i64)>,
    ) -> Self {
        let root = tempfile::tempdir().unwrap();
        mode(root.path(), 0o700);
        for name in [
            "bootstrap",
            "configuration",
            "witness",
            "business",
            "personal",
            "personal-witness",
        ] {
            let d = root.path().join(name);
            fs::create_dir(&d).unwrap();
            mode(&d, 0o700)
        }
        let pathof = |name: &str| root.path().join(name).to_str().unwrap().to_owned();
        let key = SigningKey::from_bytes(&[7; 32]);
        let personkey = SigningKey::from_bytes(&[41; 32]);
        let principal = Principal {
            id: "operator".into(),
            kind: "human".into(),
            issuer: "https://fixture.invalid".into(),
            subject: "stable-person".into(),
        };
        let installation = "ab".repeat(32);
        let pins = Pins {
            installation_digest: installation.clone(),
            public_key: URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes()),
            principal: principal.clone(),
        };
        let scope = Scope {
            work_id: "work".into(),
            plan_version: 3,
            plan_digest: "de".repeat(32),
        };
        let (scope, trust_revision) = binding.unwrap_or((scope, 2));
        let business = Journal::open(
            &Context::new(),
            pathof("business/journal.sqlite"),
            &key.verifying_key().to_bytes(),
            [11; 32],
        )
        .unwrap();
        let now = OffsetDateTime::now_utc();
        let grant = a::Document {
            schema_version: 1,
            enrollment_id: "business-grant".into(),
            generation: 1,
            project_digest: "0b".repeat(32),
            principal: principal.clone(),
            scope: scope.clone(),
            trust_revision,
            permissions: [Request::Read, Request::Create(change())]
                .iter()
                .map(|r| control::permission(&installation, r).unwrap())
                .chain(action)
                .collect(),
            delegable: vec![],
            not_before: now - Duration::hours(1),
            expires_at: now + Duration::hours(1),
        };
        let raw = a::sign_enrollment(&key, [11; 32], &Head::default(), &grant).unwrap();
        let businesshead = business
            .apply(&Context::new(), &Head::default(), &raw)
            .unwrap();
        let personal = Journal::open(
            &Context::new(),
            pathof("personal/journal.sqlite"),
            &personkey.verifying_key().to_bytes(),
            [51; 32],
        )
        .unwrap();
        let bootstrap = p::Bootstrap {
            schema_version: 1,
            installation_digest: installation.clone(),
            principal: principal.clone(),
            generation: 7,
            parameters: a::credentials::Parameters {
                memory_kib: 32,
                passes: 1,
                parallelism: 1,
            },
            proof_commitment: p::commitment(&installation, &[42; 32]).unwrap(),
        };
        let raw = sign_change(
            &personkey,
            [51; 32],
            &Head::default(),
            "enroll",
            p::BOOTSTRAP_ID,
            &p::encode_bootstrap(&bootstrap).unwrap(),
        )
        .unwrap();
        let first = personal
            .apply(&Context::new(), &Head::default(), &raw)
            .unwrap();
        if enrolled {
            let source = p::Source::open(
                &Context::new(),
                pathof("personal/journal.sqlite"),
                &personkey.verifying_key().to_bytes(),
                [51; 32],
                p::Expected {
                    installation_digest: installation,
                    principal,
                    generation: 7,
                },
            )
            .unwrap();
            let docs = Documents::open(
                root.path().join("personal-witness").as_path(),
                rustix::process::geteuid().as_raw(),
            )
            .unwrap();
            let co = p::coordinator::Coordinator::new(source, docs);
            co.initialize(&p::coordinator::Context::new(), &first)
                .unwrap();
            co.enroll(
                &p::coordinator::Context::new(),
                &personkey,
                &[42; 32],
                b"STARTUP_FIXTURE_PASSWORD",
            )
            .unwrap();
            co.close().unwrap();
        }
        for name in ["business/journal.sqlite", "personal/journal.sqlite"] {
            mode(&root.path().join(name), 0o600)
        }
        let uid = rustix::process::geteuid().as_raw();
        let doc = AuthenticatedDocument {
            schema_version: 3,
            control: ControlDocument {
                schema_version: 2,
                installation: Document {
                    schema_version: 1,
                    pins,
                    service_uid: uid,
                    controller_uid: if uid == 0 { 1 } else { !uid },
                    configuration_directory: pathof("configuration"),
                    witness_directory: pathof("witness"),
                },
                journal_path: pathof("business/journal.sqlite"),
                project_context_digest: "0b".repeat(32),
                authority_head: businesshead,
                enrollment_id: grant.enrollment_id,
                generation: 1,
                scope,
                trust_configuration_revision: trust_revision,
            },
            personal: PersonalDeclaration {
                authority_public_key: URL_SAFE_NO_PAD.encode(personkey.verifying_key().to_bytes()),
                project_context_digest: "33".repeat(32),
                journal_path: pathof("personal/journal.sqlite"),
                checkpoint_directory: pathof("personal-witness"),
                generation: 7,
                bootstrap_head: first,
            },
        };
        let path = root.path().join("bootstrap/authenticated.json");
        let mut f = Self {
            _root: root,
            path,
            doc,
            pin: String::new(),
            business,
            personal,
        };
        f.save();
        f
    }
    /// Update only the fixture's independent declaration/pin, never a live policy.
    pub(crate) fn save(&mut self) {
        let raw = b::encode_authenticated(&self.doc).unwrap();
        fs::write(&self.path, &raw).unwrap();
        mode(&self.path, 0o600);
        self.pin = format!("{:x}", Sha256::digest(raw));
    }
    /// Explicitly close original fixture connections after tested owners closed.
    pub(crate) fn close(&self) {
        self.business.close().unwrap();
        self.personal.close().unwrap();
    }
}
