#![cfg(target_os = "linux")]
#[path = "../../swarm-enrollment-authority/tests/personal_common/mod.rs"]
mod personal_common;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output, Stdio},
};
use swarm_enrollment_authority::{self as a, personal as p};
use swarm_enrollment_bootstrap::{
    self as b, AuthenticatedDocument, ControlDocument, Document, PersonalDeclaration,
};
use swarm_enrollment_config::Pins;
use swarm_enrollment_state::{Context, Head, Journal};
use swarm_identity_contract::{Permission, Scope};
use time::{Duration, OffsetDateTime};
/// Existing independently signed personal fixture reused by reference, plus
/// disjoint business/bootstrap/configuration/witness roots. No live data/human.
struct Case {
    root: tempfile::TempDir,
    personal: personal_common::Fixture,
    path: PathBuf,
    seed: PathBuf,
    pin: String,
    doc: AuthenticatedDocument,
}
/// Set exact permissions on this disposable fixture only.
fn mode(p: &std::path::Path, bits: u32) {
    fs::set_permissions(p, fs::Permissions::from_mode(bits)).unwrap()
}
impl Case {
    /// Build personal generation7 at bootstraphead1 and business grant1 in separate
    /// private domains; explicit cheap test KDF costs come from shared fixture.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        mode(root.path(), 0o700);
        let personal = personal_common::Fixture::with_generation(7);
        mode(personal._dir.path(), 0o700);
        mode(&personal.path, 0o600);
        for n in [
            "bootstrap",
            "configuration",
            "witness",
            "business",
            "personal-witness",
        ] {
            let p = root.path().join(n);
            fs::create_dir(&p).unwrap();
            mode(&p, 0o700)
        }
        let pathof = |n: &str| root.path().join(n).to_str().unwrap().to_owned();
        let business_key = SigningKey::from_bytes(&[7; 32]);
        let scope = Scope {
            work_id: "cli-work".into(),
            plan_version: 3,
            plan_digest: "de".repeat(32),
        };
        let now = OffsetDateTime::now_utc();
        let grant = a::Document {
            schema_version: 1,
            enrollment_id: "cli-business".into(),
            generation: 1,
            project_digest: "0b".repeat(32),
            principal: personal.expected.principal.clone(),
            scope: scope.clone(),
            trust_revision: 2,
            permissions: vec![Permission {
                operation: "effect.execute".into(),
                resource: format!(
                    "enrollment-config/{}/read",
                    personal.expected.installation_digest
                ),
            }],
            delegable: vec![],
            not_before: now - Duration::hours(1),
            expires_at: now + Duration::hours(1),
        };
        let business_path = pathof("business/journal.sqlite");
        let j = Journal::open(
            &Context::new(),
            &business_path,
            &business_key.verifying_key().to_bytes(),
            [11; 32],
        )
        .unwrap();
        let raw = a::sign_enrollment(&business_key, [11; 32], &Head::default(), &grant).unwrap();
        let business_head = j.apply(&Context::new(), &Head::default(), &raw).unwrap();
        j.close().unwrap();
        mode(std::path::Path::new(&business_path), 0o600);
        let doc = AuthenticatedDocument {
            schema_version: 3,
            control: ControlDocument {
                schema_version: 2,
                installation: Document {
                    schema_version: 1,
                    pins: Pins {
                        installation_digest: personal.expected.installation_digest.clone(),
                        public_key: URL_SAFE_NO_PAD.encode(business_key.verifying_key().to_bytes()),
                        principal: personal.expected.principal.clone(),
                    },
                    service_uid: rustix::process::geteuid().as_raw(),
                    controller_uid: match rustix::process::geteuid().as_raw() {
                        0 => 1,
                        uid => !uid,
                    },
                    configuration_directory: pathof("configuration"),
                    witness_directory: pathof("witness"),
                },
                journal_path: business_path,
                project_context_digest: "0b".repeat(32),
                authority_head: business_head,
                enrollment_id: grant.enrollment_id,
                generation: 1,
                scope,
                trust_configuration_revision: 2,
            },
            personal: PersonalDeclaration {
                authority_public_key: URL_SAFE_NO_PAD
                    .encode(personal.key.verifying_key().to_bytes()),
                project_context_digest: "2b".repeat(32),
                journal_path: personal.path.to_str().unwrap().into(),
                checkpoint_directory: pathof("personal-witness"),
                generation: personal.expected.generation,
                bootstrap_head: personal.head.clone(),
            },
        };
        let path = root.path().join("bootstrap/personal.json");
        let raw = b::encode_authenticated(&doc).unwrap();
        fs::write(&path, &raw).unwrap();
        mode(&path, 0o600);
        let pin = format!("{:x}", Sha256::digest(&raw));
        let seed = root.path().join("bootstrap/personal.seed");
        fs::write(&seed, personal.key.to_bytes()).unwrap();
        mode(&seed, 0o600);
        Self {
            root,
            personal,
            path,
            seed,
            pin,
            doc,
        }
    }
    /// Actual native executable; a verified normal candidate may override only
    /// this test's binary, never a production setting, key or account identity.
    fn command(&self, op: &str, lang: &str) -> Command {
        let binary = std::env::var_os("SWARM_PAEA_PERSONAL_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_swarm-rust")));
        let mut c = Command::new(binary);
        c.args([
            "identity",
            op,
            &self.pin,
            self.seed.to_str().unwrap(),
            "--input",
            self.path.to_str().unwrap(),
            "--json",
            "--lang",
            lang,
        ])
        .current_dir(self.root.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
        c
    }
    /// Supply secret bytes only through a child pipe, close it for exact EOF,
    /// retain stdout/stderr/status; no watchdog, shell, provider or live Root.
    fn run(&self, mut c: Command, payload: Option<&[u8]>) -> Output {
        c.stdin(if payload.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        let mut child = c.spawn().unwrap();
        if let Some(p) = payload {
            let mut input = child.stdin.take().unwrap();
            input.write_all(p).unwrap();
            drop(input)
        }
        child.wait_with_output().unwrap()
    }
    /// Closed command wrapper, with binary/password never interpolated in argv.
    fn call(&self, op: &str, lang: &str, payload: Option<&[u8]>) -> Output {
        self.run(self.command(op, lang), payload)
    }
    /// Synthetic independent wire encoding, not copied from the private parser.
    fn frame(&self, password: &[u8]) -> Vec<u8> {
        let mut v = b"swarm-personal-enrollment-v1\n".to_vec();
        v.extend(self.personal.proof);
        v.extend(password);
        v
    }
    /// Before enrollment, actual signed bootstrap remains active at pinned head;
    /// no project or configuration was opened by a failed/identity command.
    fn bootstrap_unchanged(&self) {
        let j = self.personal.bound();
        j.with_active(
            &Context::new(),
            &self.personal.head,
            p::BOOTSTRAP_ID,
            |_| Ok(()),
        )
        .unwrap();
        j.close().unwrap();
        assert!(!self.root.path().join(".swarm").exists());
        assert!(
            !PathBuf::from(&self.doc.control.installation.configuration_directory)
                .join("config.json")
                .exists()
        );
    }
    /// Read private checkpoint only inside this owned fixture to assert actual
    /// account state. The observed head is never supplied to a CLI mutation.
    fn account(&self) -> Head {
        let raw = fs::read(
            PathBuf::from(&self.doc.personal.checkpoint_directory).join("personal-checkpoint.json"),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        let h: Head = serde_json::from_value(v["head"].clone()).unwrap();
        let j = self.personal.bound();
        j.with_active(&Context::new(), &h, p::ACCOUNT_ID, |_| Ok(()))
            .unwrap();
        assert!(j
            .with_active(&Context::new(), &h, p::BOOTSTRAP_ID, |_| Ok(()))
            .is_err());
        j.close().unwrap();
        h
    }
    /// Explicitly close the original shared fixture Source after all children exit.
    fn close(&self) {
        self.personal.source.close().unwrap()
    }
}
/// Require meaningful JSON, known close and scope, never mere exit/file presence.
fn completed(o: &Output, op: &str) -> serde_json::Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(o.stderr.is_empty());
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["protocol"], "swarm-personal-lifecycle-cli-v1");
    assert_eq!(v["operation"], op);
    assert_eq!(v["operation_result"], "completed");
    assert_eq!(v["close"], "closed");
    for k in [
        "human_presence_verified",
        "service_started",
        "confinement_verified",
    ] {
        assert_eq!(v[k], false)
    }
    v
}
/// A runtime refusal must remain nonzero, nonretryable, classified and redacted.
fn refused(o: &Output, code: &str) {
    assert!(!o.status.success());
    let v: serde_json::Value = serde_json::from_slice(&o.stderr).unwrap();
    assert_eq!(v["failure"]["code"], code);
    assert_eq!(v["failure"]["retryable"], false);
    assert!(!String::from_utf8_lossy(&o.stderr).contains("CLI_PRIVATE"));
}
#[test]
fn personal_native_cold_enrollment_exact_once_and_recovery_fr_en() {
    for lang in ["fr", "en"] {
        let f = Case::new();
        f.bootstrap_unchanged();
        let v = completed(&f.call("personal-init", lang, None), "personal-init");
        assert_eq!(v["journal"], "not_attempted");
        let frame = f.frame(b"CLI_PRIVATE\0PASSWORD\n");
        let v = completed(
            &f.call("personal-enroll", lang, Some(&frame)),
            "personal-enroll",
        );
        assert_eq!(v["journal"], "acknowledged");
        assert_eq!(v["sequence"], 3);
        assert_eq!(v["intent_publication"], "durable");
        assert_eq!(f.account().sequence, 3);
        let repeated = f.call("personal-enroll", lang, Some(&frame));
        refused(&repeated, "personal-account-exists");
        let r: serde_json::Value = serde_json::from_slice(&repeated.stdout).unwrap();
        assert_eq!(r["operation_result"], "failed");
        assert_eq!(r["journal"], "not_attempted");
        let v = completed(&f.call("personal-recover", lang, None), "personal-recover");
        assert_eq!(v["journal"], "current_verified");
        assert_eq!(v["sequence"], 3);
        assert_eq!(f.account().sequence, 3);
        assert!(!f.root.path().join(".swarm").exists());
        f.close();
    }
}
#[test]
fn personal_native_bad_frame_and_proof_refuse_without_consuming_then_recover() {
    let f = Case::new();
    completed(&f.call("personal-init", "en", None), "personal-init");
    let cp = PathBuf::from(&f.doc.personal.checkpoint_directory).join("personal-checkpoint.json");
    let before = fs::read(&cp).unwrap();
    let good = f.frame(b"CLI_PRIVATE_PASSWORD");
    for raw in [
        vec![],
        good[..10].to_vec(),
        good[..b"swarm-personal-enrollment-v1\n".len() + 32].to_vec(),
    ] {
        let o = f.call("personal-enroll", "en", Some(&raw));
        refused(&o, "personal-frame-invalid");
        assert!(o.stdout.is_empty());
        assert_eq!(fs::read(&cp).unwrap(), before);
        f.bootstrap_unchanged();
    }
    let mut wrong = good.clone();
    wrong[b"swarm-personal-enrollment-v1\n".len()] ^= 1;
    let o = f.call("personal-enroll", "en", Some(&wrong));
    refused(&o, "installation-proof");
    let r: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(r["journal"], "not_attempted");
    assert_eq!(fs::read(&cp).unwrap(), before);
    f.bootstrap_unchanged();
    completed(
        &f.call("personal-enroll", "en", Some(&good)),
        "personal-enroll",
    );
    assert_eq!(f.account().sequence, 3);
    f.close();
}
#[test]
fn personal_native_private_key_refusal_precedes_checkpoint_then_recovers() {
    let control = Case::new();
    completed(&control.call("personal-init", "en", None), "personal-init");
    control.close();
    let f = Case::new();
    let frame = f.frame(b"CLI_PRIVATE_PASSWORD");
    fs::write(&f.seed, [19; 32]).unwrap();
    let o = f.call("personal-enroll", "en", Some(&frame));
    refused(&o, "binding");
    assert!(o.stdout.is_empty());
    assert!(!PathBuf::from(&f.doc.personal.checkpoint_directory)
        .join("personal-checkpoint.json")
        .exists());
    f.bootstrap_unchanged();
    fs::write(&f.seed, f.personal.key.to_bytes()).unwrap();
    completed(&f.call("personal-init", "en", None), "personal-init");
    completed(
        &f.call("personal-enroll", "en", Some(&frame)),
        "personal-enroll",
    );
    assert_eq!(f.account().sequence, 3);
    f.close();
}
#[test]
fn personal_native_stdout_loss_keeps_actual_enrollment_and_safe_diagnostic() {
    let f = Case::new();
    completed(&f.call("personal-init", "en", None), "personal-init");
    let frame = f.frame(b"CLI_PRIVATE_PASSWORD");
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let mut c = f.command("personal-enroll", "en");
    c.stdout(Stdio::from(full));
    let lost = f.run(c, Some(&frame));
    refused(&lost, "personal-lifecycle-output");
    assert!(lost.stdout.is_empty());
    let e: serde_json::Value = serde_json::from_slice(&lost.stderr).unwrap();
    let causes = e["failure"]["causes"].as_array().unwrap();
    for label in [
        "personal-reported-result:completed",
        "personal-reported-sequence:3",
        "personal-reported-checkpoint:durable",
        "os-errno:28",
    ] {
        assert!(causes.iter().any(|v| v == label), "missing {label}: {e}");
    }
    assert_eq!(f.account().sequence, 3);
    let r = completed(&f.call("personal-recover", "en", None), "personal-recover");
    assert_eq!(r["sequence"], 3);
    refused(
        &f.call("personal-enroll", "en", Some(&frame)),
        "personal-account-exists",
    );
    assert_eq!(f.account().sequence, 3);
    f.close();
}
#[test]
fn personal_native_real_input_io_and_oversize_refuse_before_checkpoint() {
    let control = Case::new();
    completed(&control.call("personal-init", "en", None), "personal-init");
    control.close();
    let f = Case::new();
    let mut c = f.command("personal-enroll", "en");
    c.stdin(Stdio::from(fs::File::open(f.root.path()).unwrap()));
    let io = c.output().unwrap();
    refused(&io, "personal-input-io");
    assert!(io.stdout.is_empty());
    let big = vec![0; swarm_engine::COMMAND_INPUT_MAX_BYTES + 1];
    let o = f.call("personal-enroll", "en", Some(&big));
    refused(&o, "input-document-too-large");
    assert!(o.stdout.is_empty());
    assert!(!PathBuf::from(&f.doc.personal.checkpoint_directory)
        .join("personal-checkpoint.json")
        .exists());
    f.bootstrap_unchanged();
    completed(&f.call("personal-init", "en", None), "personal-init");
    f.close();
}

#[test]
fn personal_native_terminal_refuses_without_echo_or_bootstrap_use() {
    let control = Case::new();
    completed(&control.call("personal-init", "en", None), "personal-init");
    control.close();
    let f = Case::new();
    let binary = std::env::var_os("SWARM_PAEA_PERSONAL_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_swarm-rust")));
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/paea_personal_tty.py");
    let out = Command::new("python3")
        .arg(script)
        .arg(binary)
        .args([
            "identity",
            "personal-enroll",
            &f.pin,
            f.seed.to_str().unwrap(),
            "--input",
            f.path.to_str().unwrap(),
            "--json",
            "--lang",
            "en",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(meta["terminal"], true);
    assert_ne!(meta["exit"], 0);
    assert_eq!(meta["stdout"], "");
    let error: serde_json::Value = serde_json::from_str(meta["stderr"].as_str().unwrap()).unwrap();
    assert_eq!(error["failure"]["code"], "personal-input-stream-required");
    assert!(!PathBuf::from(&f.doc.personal.checkpoint_directory)
        .join("personal-checkpoint.json")
        .exists());
    f.bootstrap_unchanged();
    f.close();
}

#[test]
fn personal_native_observation_reports_match_closed_schema() {
    let f = Case::new();
    let init = f.call("personal-init", "en", None);
    completed(&init, "personal-init");
    let good = f.frame(b"CLI_PRIVATE_PASSWORD");
    let mut bad = good.clone();
    bad[b"swarm-personal-enrollment-v1\n".len()] ^= 1;
    let refused_proof = f.call("personal-enroll", "en", Some(&bad));
    refused(&refused_proof, "installation-proof");
    let enrolled = f.call("personal-enroll", "en", Some(&good));
    completed(&enrolled, "personal-enroll");
    let recovered = f.call("personal-recover", "en", None);
    completed(&recovered, "personal-recover");
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let script = base.join("tests/fixtures/paea_personal_report_schema.py");
    let schema = base.join("schemas/paea/personal-lifecycle-cli-v1.schema.json");
    for output in [&init, &refused_proof, &enrolled, &recovered] {
        let mut child = Command::new("python3")
            .arg(&script)
            .arg(&schema)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&output.stdout)
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert_eq!(f.account().sequence, 3);
    f.close();
}
