#![cfg(unix)]
use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::PathBuf};
use swarm_engine::{
    decode_work, read_managed_receipt, EngineSettingsDocument, EngineSettingsValues, SettingCap,
    SettingCount, Work,
};
struct Fixture(PathBuf);
impl Fixture {
    /// Own an isolated private artifact root; never open a mission database.
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(swarm_engine::generate_identifier("receipt59-").unwrap());
        fs::create_dir(&path).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Create declared simple-review data and its actual receipt; no actor proof.
fn work(f: &Fixture, raw: &[u8]) -> Work {
    fs::write(f.0.join("receipt.json"), raw).unwrap();
    decode_work(&serde_json::to_vec(&serde_json::json!({"schema_version":1,"id":"work","revision":1,"planning":{"repository":{}},"tasks":[{"id":"task","independent_review":{"id":"review","attempt":"attempt","producer":"producer","candidate_commit":"candidate","receipt":"receipt.json","receipt_sha256":format!("{:x}",Sha256::digest(raw)),"started":"date"}}]})).unwrap()).unwrap()
}
/// Explicit finite/null cap; initial settings are only this owned data fixture.
fn settings(cap: Option<usize>) -> EngineSettingsDocument {
    let mut values = EngineSettingsValues::installation_defaults().unwrap();
    values.managed_receipt_max_bytes =
        Some(SettingCap(cap.map(|n| SettingCount(n.try_into().unwrap()))));
    EngineSettingsDocument::initial(values, "fixture".into(), "2026-10-19T00:00:00Z".into())
        .unwrap()
}
const VALID: &[u8] = br#"{"candidate_commit":"candidate","attempt_id":"attempt","agent_id":"producer","controls":{"task":[{"id":"check","executed":true,"passed":true,"exit_code":0,"output_sha256":"hash","summary":"Declared"}]}}"#;
/// Every fault is preceded and followed by a complete valid witness; None means
/// no declaration, never a successful empty result for corrupt/denied evidence.
#[test]
fn receipt_binding_faults_preserve_witness_recovery_and_safe_causes() {
    let f = Fixture::new();
    let original = work(&f, VALID);
    let settings = settings(None);
    for kind in [
        "digest",
        "candidate",
        "attempt",
        "producer",
        "path",
        "partial",
        "batch",
        "fragment",
        "missing-task",
        "duplicate-task",
    ] {
        let valid = read_managed_receipt(&f.0, &original, "task", &settings)
            .unwrap()
            .unwrap();
        assert_eq!(valid.controls.unwrap()[0].id, "check");
        let mut bad = decode_work(&swarm_engine::encode_model(&original).unwrap()).unwrap();
        let task = &mut bad.tasks.as_mut().unwrap()[0];
        let review = task.independent_review.as_mut().unwrap();
        match kind {
            "digest" => review.receipt_sha256 = "wrong".into(),
            "candidate" => review.candidate_commit = "other".into(),
            "attempt" => review.attempt = "other".into(),
            "producer" => review.producer = "other".into(),
            "path" => review.receipt = "../PRIVATE".into(),
            "partial" => review.receipt.clear(),
            "batch" => review.batches = Some(vec![Default::default()]),
            "fragment" => review.fragment_journal = Some(Default::default()),
            "missing-task" => task.id = "other".into(),
            "duplicate-task" => {
                let copy = decode_work(&swarm_engine::encode_model(&original).unwrap())
                    .unwrap()
                    .tasks
                    .unwrap()
                    .remove(0);
                bad.tasks.as_mut().unwrap().push(copy);
            }
            _ => unreachable!(),
        }
        let error = read_managed_receipt(&f.0, &bad, "task", &settings).unwrap_err();
        assert!(!format!("{error:?}").contains("PRIVATE"), "{kind}");
        assert!(read_managed_receipt(&f.0, &original, "task", &settings)
            .unwrap()
            .is_some());
        assert_eq!(fs::read(f.0.join("receipt.json")).unwrap(), VALID);
    }
    let mut absent = work(&f, VALID);
    absent.tasks.as_mut().unwrap()[0].independent_review = None;
    assert!(read_managed_receipt(&f.0, &absent, "task", &settings)
        .unwrap()
        .is_none());
    absent = work(&f, VALID);
    let review = absent.tasks.as_mut().unwrap()[0]
        .independent_review
        .as_mut()
        .unwrap();
    review.receipt.clear();
    review.receipt_sha256.clear();
    assert!(read_managed_receipt(&f.0, &absent, "task", &settings)
        .unwrap()
        .is_none());
}
/// Exact boundary, zero, explicit unlimited and >old1MiB prove there is no hidden
/// second ceiling. Oversize refuses before JSON parsing, never truncates a proof.
#[test]
fn receipt_byte_policy_exact_zero_unlimited_and_large_input() {
    let f = Fixture::new();
    let w = work(&f, VALID);
    assert!(
        read_managed_receipt(&f.0, &w, "task", &settings(Some(VALID.len())))
            .unwrap()
            .is_some()
    );
    for cap in [0, VALID.len() - 1] {
        let error = read_managed_receipt(&f.0, &w, "task", &settings(Some(cap))).unwrap_err();
        assert_eq!(error.reason(), "artifact-byte-limit-exceeded");
        assert!(
            read_managed_receipt(&f.0, &w, "task", &settings(Some(VALID.len())))
                .unwrap()
                .is_some()
        );
    }
    let mut large = VALID[..VALID.len() - 1].to_vec();
    large.extend_from_slice(b",\"unknown\":\"");
    large.extend(std::iter::repeat_n(b'x', 1048577));
    large.extend_from_slice(b"\"}");
    let w = work(&f, &large);
    assert!(read_managed_receipt(&f.0, &w, "task", &settings(None))
        .unwrap()
        .is_some());
    assert!(
        read_managed_receipt(&f.0, &w, "task", &settings(Some(large.len())))
            .unwrap()
            .is_some()
    );
    let old =
        EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v9-document.json")).unwrap();
    assert_eq!(
        read_managed_receipt(&f.0, &w, "task", &old)
            .unwrap_err()
            .reason(),
        "settings-managed-receipt-policy-not-configured"
    );
}
/// Digest-bound malformed data retains a safe codec cause. Missing artifacts
/// and nonregular paths refuse; restoration recovers without mutating settings.
#[test]
fn receipt_io_codec_and_controls_failures_never_become_none() {
    let f = Fixture::new();
    let saved = settings(None);
    for raw in [b"PRIVATE invalid".as_slice(),b"{}",b"{\"candidate_commit\":\"candidate\",\"attempt_id\":\"attempt\",\"agent_id\":\"producer\",\"controls\":{\"task\":[]}}"] {
        let witness=work(&f,VALID);assert!(read_managed_receipt(&f.0,&witness,"task",&saved).unwrap().is_some());
        let w=work(&f,raw);let error=read_managed_receipt(&f.0,&w,"task",&saved).unwrap_err();
        assert!(!format!("{error:?}").contains("PRIVATE"));if raw.starts_with(b"PRIVATE") { assert!(error.source().is_some()); }
        let w=work(&f,VALID);assert!(read_managed_receipt(&f.0,&w,"task",&saved).unwrap().is_some());
    }
    let w = work(&f, VALID);
    fs::remove_file(f.0.join("receipt.json")).unwrap();
    assert!(read_managed_receipt(&f.0, &w, "task", &saved)
        .unwrap_err()
        .source()
        .is_some());
    fs::create_dir(f.0.join("receipt.json")).unwrap();
    assert!(read_managed_receipt(&f.0, &w, "task", &saved).is_err());
    fs::remove_dir(f.0.join("receipt.json")).unwrap();
    let w = work(&f, VALID);
    assert!(read_managed_receipt(&f.0, &w, "task", &saved)
        .unwrap()
        .is_some());
}
/// Compatible Go DTO rules include casefold/unknown fields and last duplicate
/// values, while null/empty task controls are rejected after successful decoding.
#[test]
fn receipt_compatible_projection_matches_go_field_semantics() {
    let f = Fixture::new();
    let raw=br#"{"candidate_commit":"wrong","CANDIDATE_COMMIT":"candidate","ATTEMPT_ID":"attempt","agent_id":"producer","unknown":true,"controls":{"task":[{"ID":"check","passed":true}]}}"#;
    let w = work(&f, raw);
    let receipt = read_managed_receipt(&f.0, &w, "task", &settings(None))
        .unwrap()
        .unwrap();
    assert_eq!(receipt.controls.unwrap()[0].id, "check");
    assert_eq!(receipt.producer_agent_id, "producer");
}
