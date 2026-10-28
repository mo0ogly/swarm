//! Initial material witnesses using only disposable private roots. No installed
//! grant, real human, provider, service account or confinement is established.
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_enrollment_authority::{self as a, credentials::Parameters, provisioning as p};
use swarm_enrollment_state::{Context, Head, Journal};
use swarm_identity_contract::{Permission, Principal, Scope};
use time::{Duration, OffsetDateTime};

/// Explicit synthetic operator decision, never a production default policy.
fn request(root: &std::path::Path) -> p::Request {
    let now = OffsetDateTime::now_utc();
    p::Request {
        schema_version: 1,
        installation_digest: "ab".repeat(32),
        root_directory: root.to_str().unwrap().into(),
        service_uid: rustix::process::geteuid().as_raw(),
        controller_uid: match rustix::process::geteuid().as_raw() {
            0 => 1,
            uid => !uid,
        },
        personal_project_context_digest: "cd".repeat(32),
        personal_generation: 7,
        parameters: Parameters {
            memory_kib: 32,
            passes: 1,
            parallelism: 1,
        },
        grant: a::Document {
            schema_version: 1,
            enrollment_id: "initial-human-grant".into(),
            generation: 1,
            project_digest: "12".repeat(32),
            principal: Principal {
                kind: "human".into(),
                id: "synthetic-operator".into(),
                issuer: "urn:swarm:provisioning:fixture".into(),
                subject: "synthetic-human".into(),
            },
            scope: Scope {
                work_id: "fixture-work".into(),
                plan_version: 3,
                plan_digest: "34".repeat(32),
            },
            trust_revision: 2,
            permissions: vec![Permission {
                operation: "effect.execute".into(),
                resource: format!("enrollment-config/{}/read", "ab".repeat(32)),
            }],
            delegable: vec![],
            not_before: now - Duration::hours(1),
            expires_at: now + Duration::hours(1),
        },
    }
}
/// Set this fixture's file/directory mode only; no live permissions are changed.
fn mode(path: &std::path::Path, bits: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(bits)).unwrap()
}

/// Real random preparation followed by exact private roundtrip; no secret in Debug.
#[test]
fn provisioning_private_roundtrip_preserves_exact_initial_proposals() {
    let d = tempfile::tempdir().unwrap();
    let r = request(d.path());
    let one = p::prepare(&r).unwrap();
    let other = p::prepare(&r).unwrap();
    assert_ne!(one.business_initial(), other.business_initial());
    assert_ne!(one.personal_initial(), other.personal_initial());
    let raw = one.encode_private().unwrap();
    let recovered = p::decode_private(&raw).unwrap();
    assert_eq!(one.business_initial(), recovered.business_initial());
    assert_eq!(one.personal_initial(), recovered.personal_initial());
    assert_eq!(one.declaration().unwrap(), recovered.declaration().unwrap());
    assert_eq!(&*raw, &*recovered.encode_private().unwrap());
    assert_eq!(
        format!("{one:?}"),
        "InitialProvisioningMaterial { <private> }"
    );
    let doc = one.declaration().unwrap();
    assert_ne!(
        doc.control.installation.pins.public_key,
        doc.personal.authority_public_key
    );
}

/// Rejection of ambiguous/corrupt framing never generates replacement keys.
#[test]
fn provisioning_private_codec_refuses_truncation_noncanonical_and_unknown_fields() {
    let d = tempfile::tempdir().unwrap();
    let r = request(d.path());
    let one = p::prepare(&r).unwrap();
    let raw = one.encode_private().unwrap();
    for length in [0, 10, raw.len() - 1] {
        assert!(p::decode_private(&raw[..length]).is_err())
    }
    let mut wrong = raw.to_vec();
    wrong[0] ^= 1;
    assert!(p::decode_private(&wrong).is_err());
    let mut space = raw.to_vec();
    space.push(b' ');
    assert!(p::decode_private(&space).is_err());
    let canonical = p::encode_request(&r).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
    value["extra"] = serde_json::json!(true);
    assert!(p::decode_request(&serde_json::to_vec(&value).unwrap()).is_err());
    value.as_object_mut().unwrap().remove("extra");
    value["parameters"] = serde_json::Value::Null;
    assert!(p::decode_request(&serde_json::to_vec(&value).unwrap()).is_err());
    assert_eq!(
        one.business_initial(),
        p::decode_private(&raw).unwrap().business_initial()
    );
}

/// Operator costs, rights, generations, distinct contexts/UIDs and clean roots
/// are explicit; invalid requests cannot become an empty/default installation.
#[test]
fn provisioning_invalid_request_refuses_then_valid_request_recovers() {
    let d = tempfile::tempdir().unwrap();
    let good = request(d.path());
    p::prepare(&good).unwrap();
    let changes: Vec<fn(&mut p::Request)> = vec![
        |r| r.schema_version = 2,
        |r| r.installation_digest.clear(),
        |r| r.root_directory = "/".into(),
        |r| r.root_directory = "/tmp/../private".into(),
        |r| r.root_directory = "relative".into(),
        |r| r.controller_uid = r.service_uid,
        |r| r.personal_generation = 0,
        |r| r.personal_project_context_digest = r.grant.project_digest.clone(),
        |r| r.grant.generation = 2,
        |r| r.grant.principal.kind = "agent".into(),
        |r| r.parameters.memory_kib = 0,
    ];
    for change in changes {
        let mut bad = good.clone();
        change(&mut bad);
        assert!(p::prepare(&bad).is_err());
        assert!(p::encode_request(&bad).is_err());
        p::prepare(&good).unwrap();
    }
}

/// End-to-end library witness from actual generated material to signed journals,
/// pinned v3 startup, unique personal account and exact recovery. Only personal
/// key is installed in fixture service storage; no business private key is used.
#[test]
fn provisioning_material_drives_actual_cold_lifecycle_without_business_key() {
    let d = tempfile::tempdir().unwrap();
    mode(d.path(), 0o700);
    let r = request(d.path());
    let prepared = p::prepare(&r).unwrap();
    let doc = prepared.declaration().unwrap();
    for n in [
        "bootstrap",
        "configuration",
        "configuration-witness",
        "business",
        "personal",
        "personal-witness",
    ] {
        let p = d.path().join(n);
        fs::create_dir(&p).unwrap();
        mode(&p, 0o700)
    }
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let business_key = URL_SAFE_NO_PAD
        .decode(&doc.control.installation.pins.public_key)
        .unwrap();
    let personal_key = URL_SAFE_NO_PAD
        .decode(&doc.personal.authority_public_key)
        .unwrap();
    let business = Journal::open(
        &Context::new(),
        &doc.control.journal_path,
        &business_key,
        [0x12; 32],
    )
    .unwrap();
    let bh = business
        .apply(
            &Context::new(),
            &Head::default(),
            prepared.business_initial(),
        )
        .unwrap();
    assert_eq!(bh, doc.control.authority_head);
    business.close().unwrap();
    let personal = Journal::open(
        &Context::new(),
        &doc.personal.journal_path,
        &personal_key,
        [0xcd; 32],
    )
    .unwrap();
    let ph = personal
        .apply(
            &Context::new(),
            &Head::default(),
            prepared.personal_initial(),
        )
        .unwrap();
    assert_eq!(ph, doc.personal.bootstrap_head);
    personal.close().unwrap();
    mode(&PathBuf::from(&doc.control.journal_path), 0o600);
    mode(&PathBuf::from(&doc.personal.journal_path), 0o600);
    let path = d.path().join("bootstrap/startup.json");
    let bytes = swarm_enrollment_bootstrap::encode_authenticated(&doc).unwrap();
    fs::write(&path, &bytes).unwrap();
    mode(&path, 0o600);
    let seed = d.path().join("bootstrap/personal.seed");
    let mut f = fs::File::create(&seed).unwrap();
    prepared.write_personal_seed(&mut f).unwrap();
    drop(f);
    mode(&seed, 0o600);
    let mut proof = zeroize::Zeroizing::new(Vec::new());
    prepared.write_installation_proof(&mut *proof).unwrap();
    let proof: &[u8; 32] = proof.as_slice().try_into().unwrap();
    use sha2::{Digest, Sha256};
    let pin = format!("{:x}", Sha256::digest(&bytes));
    // Optional exact normal executable is a verification input only. The same
    // newly generated fixture is exercised through the already public CLI.
    if let Some(binary) = std::env::var_os("SWARM_PAEA_PROVISIONING_BINARY") {
        let init = native_call(&binary, &path, &pin, &seed, "personal-init", None);
        assert!(init.status.success());
        let mut frame = zeroize::Zeroizing::new(b"swarm-personal-enrollment-v1\n".to_vec());
        frame.extend_from_slice(proof);
        frame.extend_from_slice(b"synthetic-personal-password");
        let enrolled = native_call(&binary, &path, &pin, &seed, "personal-enroll", Some(&frame));
        assert!(
            enrolled.status.success(),
            "{}",
            String::from_utf8_lossy(&enrolled.stderr)
        );
        let result: serde_json::Value = serde_json::from_slice(&enrolled.stdout).unwrap();
        assert_eq!(result["sequence"], 3);
        assert_eq!(result["human_presence_verified"], false);
        let again = native_call(&binary, &path, &pin, &seed, "personal-enroll", Some(&frame));
        assert!(!again.status.success());
        let error: serde_json::Value = serde_json::from_slice(&again.stderr).unwrap();
        assert_eq!(error["failure"]["code"], "personal-account-exists");
        let recovered = native_call(&binary, &path, &pin, &seed, "personal-recover", None);
        assert!(recovered.status.success());
        let result: serde_json::Value = serde_json::from_slice(&recovered.stdout).unwrap();
        assert_eq!(result["sequence"], 3);
        assert_eq!(result["journal"], "current_verified");
        assert!(!d.path().join("bootstrap/business.seed").exists());
        assert!(!d.path().join(".swarm").exists());
        println!("PROVISIONING_PUBLIC_CLI_LIFECYCLE");
        return;
    }
    let app = a::authenticated_startup::open_lifecycle(
        &Context::new(),
        path.to_str().unwrap(),
        &pin,
        &seed,
    )
    .unwrap();
    let ctx = a::authenticated_control::Context::new();
    app.initialize(&ctx).unwrap();
    let account = app
        .enroll(&ctx, proof, b"synthetic-personal-password")
        .unwrap();
    assert_eq!(account.head().sequence, 3);
    assert!(app
        .enroll(&ctx, proof, b"synthetic-personal-password")
        .is_err());
    assert_eq!(app.recover(&ctx).unwrap().head().sequence, 3);
    app.close().unwrap();
    assert!(!d.path().join("bootstrap/business.seed").exists());
    assert!(!d.path().join(".swarm").exists());
}

/// Run the selected native CLI on this generated fixture, with private input
/// solely in a child pipe and exact EOF. No watchdog, provider or live Root.
fn native_call(
    binary: &std::ffi::OsStr,
    path: &std::path::Path,
    pin: &str,
    seed: &std::path::Path,
    operation: &str,
    payload: Option<&[u8]>,
) -> std::process::Output {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let mut command = Command::new(binary);
    command.args([
        "identity",
        operation,
        pin,
        seed.to_str().unwrap(),
        "--input",
        path.to_str().unwrap(),
        "--json",
        "--lang",
        "en",
    ]);
    command
        .stdin(if payload.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(bytes) = payload {
        child.stdin.take().unwrap().write_all(bytes).unwrap();
    }
    child.wait_with_output().unwrap()
}

/// A failing private delivery sink retains its cause; no no-write/rollback claim.
#[test]
fn provisioning_private_sink_error_is_typed_and_redacted() {
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "PRIVATE_SINK_CAUSE",
            ))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let d = tempfile::tempdir().unwrap();
    let material = p::prepare(&request(d.path())).unwrap();
    let e = material.write_installation_proof(&mut Broken).unwrap_err();
    assert_eq!(e.code(), "provisioning-output");
    assert!(!format!("{e:?} {e}").contains("PRIVATE_"));
    assert!(std::error::Error::source(&e)
        .unwrap()
        .is::<std::io::Error>());
}
