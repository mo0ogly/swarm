#![cfg(target_os = "linux")]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
};
use swarm_enrollment_authority::{sign_enrollment, Document as Grant};
use swarm_enrollment_bootstrap::{ControlDocument, Document};
use swarm_enrollment_config::Pins;
use swarm_enrollment_state::{sign_change, Context, Head, Journal};
use swarm_identity_contract::{Permission, Principal, Scope};
use time::{Duration, OffsetDateTime};
struct Fixture {
    root: tempfile::TempDir,
    path: PathBuf,
    declaration: ControlDocument,
    key: SigningKey,
    journal: Journal,
}
impl Fixture {
    /// Create disposable protected storage and a synthetic signed grant. No real
    /// human/provider is authenticated; editing the grant happens before enrollment.
    fn new(edit: impl FnOnce(&mut Grant)) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        for name in ["bootstrap", "configuration", "witness", "authority"] {
            let p = root.path().join(name);
            fs::create_dir(&p).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let key = SigningKey::from_bytes(&[7; 32]);
        let principal = Principal {
            id: "alice".into(),
            kind: "human".into(),
            issuer: "https://fixture.invalid".into(),
            subject: "employee[eu]".into(),
        };
        let scope = Scope {
            work_id: "work".into(),
            plan_version: 3,
            plan_digest: "cd".repeat(32),
        };
        let now = OffsetDateTime::now_utc();
        let mut grant = Grant {
            schema_version: 1,
            enrollment_id: "initial-human".into(),
            generation: 1,
            project_digest: "0b".repeat(32),
            principal: principal.clone(),
            scope: scope.clone(),
            trust_revision: 2,
            permissions: ["read", "preview", "create", "apply"]
                .into_iter()
                .map(|op| Permission {
                    operation: "effect.execute".into(),
                    resource: format!("enrollment-config/{}/{op}", "ab".repeat(32)),
                })
                .collect(),
            delegable: vec![],
            not_before: now - Duration::hours(1),
            expires_at: now + Duration::hours(1),
        };
        edit(&mut grant);
        let journal_path = root.path().join("authority/journal.sqlite");
        let journal = Journal::open(
            &Context::new(),
            &journal_path,
            &key.verifying_key().to_bytes(),
            [11; 32],
        )
        .unwrap();
        let raw = sign_enrollment(&key, [11; 32], &Head::default(), &grant).unwrap();
        let head = journal
            .apply(&Context::new(), &Head::default(), &raw)
            .unwrap();
        fs::set_permissions(&journal_path, fs::Permissions::from_mode(0o600)).unwrap();
        let declaration = ControlDocument {
            schema_version: 2,
            installation: Document {
                schema_version: 1,
                pins: Pins {
                    installation_digest: "ab".repeat(32),
                    public_key: URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes()),
                    principal,
                },
                service_uid: rustix::process::geteuid().as_raw(),
                controller_uid: match rustix::process::geteuid().as_raw() {
                    0 => 1,
                    uid => !uid,
                },
                configuration_directory: root.path().join("configuration").to_str().unwrap().into(),
                witness_directory: root.path().join("witness").to_str().unwrap().into(),
            },
            journal_path: journal_path.to_str().unwrap().into(),
            project_context_digest: "0b".repeat(32),
            authority_head: head,
            enrollment_id: "initial-human".into(),
            generation: 1,
            scope,
            trust_configuration_revision: 2,
        };
        let path = root.path().join("bootstrap/control.json");
        let f = Self {
            root,
            path,
            declaration,
            key,
            journal,
        };
        f.save();
        f
    }
    /// Write only this disposable declaration; return the explicitly pinned digest.
    fn save(&self) -> String {
        let raw = swarm_enrollment_bootstrap::encode_control(&self.declaration).unwrap();
        fs::write(&self.path, &raw).unwrap();
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600)).unwrap();
        format!("{:x}", Sha256::digest(raw))
    }
    /// Actual native process; an explicit verified installed binary may replace
    /// the test-built CLI for delivery replay. This is a test mechanism only.
    fn call(&self, lang: &str, extra: &[&str]) -> Output {
        let pin = self.save();
        let binary = std::env::var_os("SWARM_PAEA_CONTROL_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_swarm-rust")));
        Command::new(binary)
            .args([
                "identity",
                "control-check",
                &pin,
                "--input",
                self.path.to_str().unwrap(),
                "--json",
                "--lang",
                lang,
            ])
            .args(extra)
            .current_dir(self.root.path())
            .output()
            .unwrap()
    }
    /// Check no project, Coordinator lock or configuration/witness data was created.
    fn unchanged(&self) {
        assert!(!self.root.path().join(".swarm").exists());
        for p in [
            &self.declaration.installation.configuration_directory,
            &self.declaration.installation.witness_directory,
        ] {
            assert_eq!(fs::read_dir(p).unwrap().count(), 0);
        }
    }
}
/// Require a meaningful public failure, never an empty successful response.
fn refused(out: Output, code: &str, lang: &str) {
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    let v: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(v["failure"]["code"], code);
    assert_eq!(v["failure"]["retryable"], false);
    assert!(v["error"].as_str().unwrap().contains(if lang == "en" {
        "no request authorized"
    } else {
        "aucune requête autorisée"
    }));
}
#[test]
fn current_control_cli_observes_real_signed_enrollment_fr_en_without_authorizing_or_creating_project(
) {
    let f = Fixture::new(|_| {});
    let before = fs::read(&f.declaration.journal_path).unwrap();
    for lang in ["fr", "en"] {
        let o = f.call(lang, &[]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        assert!(o.stderr.is_empty());
        let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["current_enrollment_verified"], true);
        for key in [
            "human_authentication_verified",
            "confinement_verified",
            "request_authorized",
            "service_started",
        ] {
            assert_eq!(v[key], false, "{key}");
        }
        assert_eq!(v["declaration"]["scope"]["work_id"], "work");
        assert_eq!(v["declaration"]["generation"], 1);
        assert_eq!(
            v["declaration"]["authority_head"]["digest"],
            f.declaration.authority_head.digest
        );
        assert!(v["checked_at"].as_str().unwrap().ends_with('Z'));
        f.unchanged();
    }
    assert_eq!(fs::read(&f.declaration.journal_path).unwrap(), before);
}
#[test]
fn control_cli_wrong_context_checkpoint_principal_and_expiry_refuse_then_recover() {
    for variant in [
        "scope",
        "revision",
        "subject",
        "missing",
        "project",
        "checkpoint",
    ] {
        let mut f = Fixture::new(|_| {});
        assert!(f.call("fr", &[]).status.success());
        let original = f.declaration.clone();
        match variant {
            "scope" => f.declaration.scope.plan_version += 1,
            "revision" => f.declaration.trust_configuration_revision += 1,
            "subject" => f.declaration.installation.pins.principal.subject = "other".into(),
            "missing" => f.declaration.enrollment_id = "missing".into(),
            "project" => f.declaration.project_context_digest = "0c".repeat(32),
            _ => f.declaration.authority_head.digest = "ef".repeat(32),
        };
        refused(
            f.call("en", &[]),
            match variant {
                "missing" => "missing",
                "project" => "context",
                "checkpoint" => "head",
                _ => "binding",
            },
            "en",
        );
        f.unchanged();
        f.declaration = original;
        assert!(f.call("fr", &[]).status.success());
    }
    for future in [false, true] {
        let f = Fixture::new(|g| {
            let now = OffsetDateTime::now_utc();
            if future {
                g.not_before = now + Duration::hours(1);
                g.expires_at = now + Duration::hours(2);
            } else {
                g.not_before = now - Duration::hours(2);
                g.expires_at = now - Duration::hours(1);
            }
        });
        refused(f.call("fr", &[]), "inactive", "fr");
        f.unchanged();
    }
}
#[test]
fn control_cli_revocation_and_real_sqlite_contention_refuse_and_explicit_recovery_works() {
    let mut f = Fixture::new(|_| {});
    assert!(f.call("fr", &[]).status.success());
    let sql = rusqlite::Connection::open(&f.declaration.journal_path).unwrap();
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    let result = f.call("en", &[]);
    sql.execute_batch("ROLLBACK").unwrap();
    refused(result, "sqlite", "en");
    assert!(f.call("en", &[]).status.success());
    let raw = sign_change(
        &f.key,
        [11; 32],
        &f.declaration.authority_head,
        "revoke",
        &f.declaration.enrollment_id,
        b"",
    )
    .unwrap();
    let revoked = f
        .journal
        .apply(&Context::new(), &f.declaration.authority_head, &raw)
        .unwrap();
    refused(f.call("fr", &[]), "head", "fr");
    f.declaration.authority_head = revoked;
    refused(f.call("en", &[]), "revoked", "en");
    f.unchanged();
}
#[test]
fn control_cli_rejects_extra_runtime_options_and_output_failures_then_recovers() {
    let f = Fixture::new(|_| {});
    assert!(f.call("en", &[]).status.success());
    for args in [
        vec!["--root", "/unopened"],
        vec!["--output", "/unopened"],
        vec!["--task", "x"],
        vec!["--phase", "entry"],
        vec!["extra"],
    ] {
        let out = f.call("en", &args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        f.unchanged();
    }
    struct Broken(bool);
    impl Write for Broken {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self.0 {
                Ok(b.len())
            } else {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE_OUTPUT"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE_FLUSH"))
        }
    }
    let pin = f.save();
    let args = [
        "identity",
        "control-check",
        &pin,
        "--input",
        f.path.to_str().unwrap(),
        "--json",
    ]
    .map(String::from);
    for flush in [false, true] {
        let mut error = Vec::new();
        let result = swarm_cli::run_detailed(&args, "en", &mut Broken(flush), &mut error);
        assert_ne!(result.code, 0);
        assert!(result.error.is_some());
        let text = String::from_utf8(error).unwrap();
        assert!(!text.contains("PRIVATE_OUTPUT") && !text.contains("PRIVATE_FLUSH"));
        f.unchanged();
    }
    assert!(f.call("fr", &[]).status.success());
}
#[test]
fn pinned_startup_factory_feeds_real_core_without_implicit_activation() {
    use swarm_enrollment_authority::{
        control::{Request, Response},
        startup,
    };
    let f = Fixture::new(|_| {});
    let pin = f.save();
    let (control, head) = startup::open(&Context::new(), f.path.to_str().unwrap(), &pin).unwrap();
    let proposed = swarm_enrollment_config::Change {
        schema_version: 1,
        event_id: "explicit-fixture-event".into(),
        expected_head: swarm_enrollment_config::Head::default(),
        values: swarm_enrollment_config::Values {
            mode: "local".into(),
            socket_path: "/run/swarm-fixture/authority.sock".into(),
        },
        reason: "explicit fixture preview".into(),
    };
    let result = control
        .execute(
            &Context::new(),
            &swarm_enrollment_config::store::Context::new(),
            &head,
            Request::Preview(proposed),
        )
        .unwrap();
    assert!(matches!(result, Response::Preview(_)));
    control.close().unwrap();
    assert!(
        !PathBuf::from(&f.declaration.installation.configuration_directory)
            .join("config.json")
            .exists()
    );
    assert!(
        !PathBuf::from(&f.declaration.installation.witness_directory)
            .join("witness.json")
            .exists()
    );
}

#[test]
fn control_cli_bad_pin_unsafe_file_and_private_invalid_json_refuse_before_journal_access() {
    let f = Fixture::new(|_| {});
    assert!(f.call("en", &[]).status.success());
    let binary = std::env::var_os("SWARM_PAEA_CONTROL_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_swarm-rust")));
    let invoke = |pin: &str| {
        Command::new(&binary)
            .args([
                "identity",
                "control-check",
                pin,
                "--input",
                f.path.to_str().unwrap(),
                "--json",
                "--lang",
                "en",
            ])
            .current_dir(f.root.path())
            .output()
            .unwrap()
    };
    let sql = rusqlite::Connection::open(&f.declaration.journal_path).unwrap();
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    // A journal writer would cause sqlite, so pin/path/document show the prior
    // boundary refused before opening authority. No timing threshold is needed.
    refused(invoke(&"ef".repeat(32)), "pin", "en");
    let pin = f.save();
    fs::set_permissions(&f.path, fs::Permissions::from_mode(0o640)).unwrap();
    refused(invoke(&pin), "path", "en");
    fs::set_permissions(&f.path, fs::Permissions::from_mode(0o600)).unwrap();
    let raw = br#"{"schema_version":"PRIVATE_DECLARATION_VALUE"}"#;
    fs::write(&f.path, raw).unwrap();
    let pin = format!("{:x}", Sha256::digest(raw));
    let failed = invoke(&pin);
    assert!(!String::from_utf8_lossy(&failed.stderr).contains("PRIVATE_DECLARATION_VALUE"));
    refused(failed, "document", "en");
    sql.execute_batch("ROLLBACK").unwrap();
    assert!(f.call("fr", &[]).status.success());
    f.unchanged();
}

/// Closed identity commands must not silently accept explicit project/default
/// options or ambiguous duplicate input flags. Valid same-fixture control first.
#[test]
fn identity_cli_closed_options_refuse_default_value_aliases_before_io() {
    let f = Fixture::new(|_| {});
    let valid = f.call("en", &[]);
    assert!(valid.status.success());
    f.unchanged();
    let mut observations = vec![];
    for extra in [
        vec!["--root", "."],
        vec!["--phase", "delivery"],
        vec!["--task", ""],
        vec!["--output", ""],
        vec!["--json"],
        vec!["--lang", "en"],
        vec!["--lang=en"],
        vec!["--unknown-identity-option"],
        vec!["--input", f.path.to_str().unwrap()],
    ] {
        let o = f.call("en", &extra);
        f.unchanged();
        observations.push((extra.into_iter().map(str::to_owned).collect::<Vec<_>>(), o));
    }
    f.journal.close().unwrap();
    for (extra, o) in observations {
        assert!(
            !o.status.success(),
            "forbidden default alias accepted: {extra:?}"
        );
        assert!(o.stdout.is_empty());
        // Parser errors precede a parsed --json mode and stay plain by contract.
        let diagnostic = String::from_utf8_lossy(&o.stderr);
        assert!(!diagnostic.trim_start().starts_with('{'));
        assert!(diagnostic.contains(&swarm_cli::CliError::Usage.to_string()));
    }
}
