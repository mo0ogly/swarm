use sha2::{Digest, Sha256};
use std::{error::Error, fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{check_validation_evidence, validation_policy_digest, Task};

struct Fixture(PathBuf);
impl Fixture {
    /// Private ordinary files only; this fixture declares data, never real control execution.
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("validation-evidence-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(p.join("receipt.json"), b"original receipt bytes").unwrap();
        Self(p)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

/// Supply a structurally complete human receipt; digest is computed from its exact policy.
fn task() -> Task {
    let mut t = swarm_engine::decode_task(br#"{"id":"task","status":"submitted","attempts":[{"id":"attempt"}],"validation_policy":{"mode":"human","controls":[{"id":"control","command":["cargo","test"],"review_output":true}]},"automatic_validation":{"attempt_id":"attempt","controller":"controller://swarm-validation","receipt":"receipt.json","state":"pending_human","controls":[{"id":"control","command":["cargo","test"],"executed":true,"passed":true,"exit_code":0,"started_at":"start","finished_at":"finish","review_output":"proof","output_bytes":5}]}}"#).unwrap();
    let digest = validation_policy_digest(t.validation_policy.as_deref().unwrap()).unwrap();
    let a = t.automatic_validation.as_deref_mut().unwrap();
    a.policy_digest = digest;
    a.artifacts = Some(std::collections::BTreeMap::from([(
        "receipt.json".into(),
        format!("{:x}", Sha256::digest(b"original receipt bytes")),
    )]));
    t
}

/// Attribution is excluded from the digest, ordered command/policy content is not.
#[test]
fn policy_digest_ignores_attribution_and_binds_controls_nil_and_empty() {
    let mut p = task().validation_policy.unwrap();
    let initial = validation_policy_digest(&p).unwrap();
    p.actor = "human <&>".into();
    p.authorized_at = "date".into();
    assert_eq!(validation_policy_digest(&p).unwrap(), initial);
    p.controls.as_mut().unwrap()[0].command = None;
    let nil = validation_policy_digest(&p).unwrap();
    p.controls.as_mut().unwrap()[0].command = Some(vec![]);
    assert_ne!(validation_policy_digest(&p).unwrap(), nil);
    assert_ne!(nil, initial);
}

/// Every malformed claim is surrounded by a valid control and recovery; inputs stay intact.
#[test]
fn current_receipt_guards_attempt_policy_results_sharing_and_utf8_byte_limits() {
    let f = Fixture::new();
    let good = task();
    let original = swarm_engine::encode_model(&good).unwrap();
    for fault in 0..17 {
        assert_eq!(
            check_validation_evidence(&f.0, &good, None, Some(5))
                .unwrap()
                .unwrap()
                .observations,
            ["proof"]
        );
        let mut bad = good.clone();
        let a = bad.automatic_validation.as_deref_mut().unwrap();
        match fault {
            0 => a.attempt_id = "other".into(),
            1 => a.controller = "other".into(),
            2 => a.policy_digest = "wrong".into(),
            3 => a.state = "failed".into(),
            4 => a.receipt.clear(),
            5 => a.artifacts = None,
            6 => a.controls = None,
            7 => a.controls.as_mut().unwrap()[0].id = "wrong".into(),
            8 => a.controls.as_mut().unwrap()[0].command = None,
            9 => a.controls.as_mut().unwrap()[0].executed = false,
            10 => a.controls.as_mut().unwrap()[0].passed = false,
            11 => a.controls.as_mut().unwrap()[0].exit_code = 1,
            12 => a.controls.as_mut().unwrap()[0].started_at.clear(),
            13 => a.controls.as_mut().unwrap()[0].finished_at.clear(),
            14 => a.controls.as_mut().unwrap()[0].output_bytes = -1,
            15 => a.controls.as_mut().unwrap()[0].review_output_truncated = true,
            16 => {
                let r = &mut a.controls.as_mut().unwrap()[0];
                r.review_output = "ééé".into();
                r.output_bytes = 6;
            }
            _ => unreachable!(),
        }
        let before = swarm_engine::encode_model(&bad).unwrap();
        assert!(
            check_validation_evidence(&f.0, &bad, None, Some(5)).is_err(),
            "fault {fault}"
        );
        assert_eq!(swarm_engine::encode_model(&bad).unwrap(), before);
        check_validation_evidence(&f.0, &good, None, Some(5)).unwrap();
    }
    assert_eq!(swarm_engine::encode_model(&good).unwrap(), original);
    let mut t = good.clone();
    let p = t.validation_policy.as_deref_mut().unwrap();
    p.controls.as_mut().unwrap()[0].review_output = false;
    let digest = validation_policy_digest(p).unwrap();
    t.automatic_validation.as_deref_mut().unwrap().policy_digest = digest;
    assert_eq!(
        check_validation_evidence(&f.0, &t, None, None)
            .unwrap_err()
            .reason(),
        "control-output-not-authorized"
    );
    let r = &mut t
        .automatic_validation
        .as_deref_mut()
        .unwrap()
        .controls
        .as_mut()
        .unwrap()[0];
    r.review_output.clear();
    r.output_bytes = 0;
    assert!(check_validation_evidence(&f.0, &t, None, Some(0))
        .unwrap()
        .unwrap()
        .observations
        .is_empty());
    assert!(
        check_validation_evidence(&f.0, &Task::default(), None, None)
            .unwrap()
            .is_none()
    );
}

/// Consistent prefixes stay non-quotable; a null cap is explicit unlimited, not a fallback.
#[test]
fn complete_output_is_quotable_but_consistent_truncation_and_empty_output_are_not() {
    let f = Fixture::new();
    let good = task();
    check_validation_evidence(&f.0, &good, None, Some(5)).unwrap();
    assert!(check_validation_evidence(&f.0, &good, None, Some(4)).is_err());
    assert_eq!(
        check_validation_evidence(&f.0, &good, None, None)
            .unwrap()
            .unwrap()
            .observations,
        ["proof"]
    );
    let mut t = good.clone();
    let r = &mut t
        .automatic_validation
        .as_deref_mut()
        .unwrap()
        .controls
        .as_mut()
        .unwrap()[0];
    r.output_bytes = 9;
    r.review_output_truncated = true;
    assert!(check_validation_evidence(&f.0, &t, None, Some(5))
        .unwrap()
        .unwrap()
        .observations
        .is_empty());
    let r = &mut t
        .automatic_validation
        .as_deref_mut()
        .unwrap()
        .controls
        .as_mut()
        .unwrap()[0];
    r.output_bytes = 0;
    r.review_output.clear();
    r.review_output_truncated = false;
    assert!(check_validation_evidence(&f.0, &t, None, Some(0))
        .unwrap()
        .unwrap()
        .observations
        .is_empty());
    assert!(!format!(
        "{:?}",
        check_validation_evidence(&f.0, &good, None, None).unwrap()
    )
    .contains("proof"));
}

/// Changed, absent and escaped artifacts refuse, preserve causes and recover without writes.
#[test]
fn artifact_refusal_preserves_io_cause_and_recovery_and_automatic_mode_needs_actual_gate() {
    let f = Fixture::new();
    let good = task();
    check_validation_evidence(&f.0, &good, None, None).unwrap();
    fs::write(f.0.join("receipt.json"), b"changed").unwrap();
    assert_eq!(
        check_validation_evidence(&f.0, &good, None, None)
            .unwrap_err()
            .reason(),
        "validation-artifact-stale"
    );
    assert_eq!(fs::read(f.0.join("receipt.json")).unwrap(), b"changed");
    fs::write(f.0.join("receipt.json"), b"original receipt bytes").unwrap();
    check_validation_evidence(&f.0, &good, None, None).unwrap();
    fs::rename(f.0.join("receipt.json"), f.0.join("retained.json")).unwrap();
    let error = check_validation_evidence(&f.0, &good, None, None).unwrap_err();
    assert!(error.source().is_some());
    assert!(!format!("{error:?}").contains("receipt.json"));
    fs::rename(f.0.join("retained.json"), f.0.join("receipt.json")).unwrap();
    check_validation_evidence(&f.0, &good, None, None).unwrap();
    let mut escaped = good.clone();
    escaped
        .automatic_validation
        .as_deref_mut()
        .unwrap()
        .artifacts
        .as_mut()
        .unwrap()
        .insert("../private".into(), "hash".into());
    assert!(check_validation_evidence(&f.0, &escaped, None, None)
        .unwrap_err()
        .source()
        .is_some());
    let mut automatic = good.clone();
    automatic.validation_policy.as_deref_mut().unwrap().mode = "automatic".into();
    automatic
        .automatic_validation
        .as_deref_mut()
        .unwrap()
        .policy_digest =
        validation_policy_digest(automatic.validation_policy.as_deref().unwrap()).unwrap();
    assert_eq!(
        check_validation_evidence(&f.0, &automatic, None, None)
            .unwrap_err()
            .reason(),
        "quality-policy-required"
    );
    let settings = swarm_engine::EngineSettingsDocument::initial(
        swarm_engine::EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap();
    let profile = settings.quality_policy_snapshot().unwrap();
    assert!(check_validation_evidence(&f.0, &automatic, Some(&profile), None).is_err());
    let raw=serde_json::to_vec(&serde_json::json!({"method_version":"3","scope_id":"task",
        "artifacts":{"receipt.json":format!("{:x}",Sha256::digest(b"original receipt bytes"))},
        "domains":{"quality":1},"checks":[{"id":"control","domain":"quality","mandatory":true,"gate":"delivery","severity":"minor","penalty":1,"max_penalty":100}],
        "results":[{"id":"control","status":"PASS","count":0,"evidence":["receipt.json"]}]})).unwrap();
    automatic = swarm_engine::prepare_gate_record(
        &automatic,
        &swarm_engine::GateSubmission {
            name: "Fixture",
            phase: "delivery",
            document: &raw,
            at: "date",
        },
        &f.0,
        &profile,
    )
    .unwrap();
    check_validation_evidence(&f.0, &automatic, Some(&profile), None).unwrap();
    fs::write(f.0.join("receipt.json"), b"changed").unwrap();
    assert!(check_validation_evidence(&f.0, &automatic, Some(&profile), None).is_err());
    fs::write(f.0.join("receipt.json"), b"original receipt bytes").unwrap();
    check_validation_evidence(&f.0, &automatic, Some(&profile), None).unwrap();
    check_validation_evidence(&f.0, &good, None, None).unwrap();
}

/// Actual stored Work/task/attempt and persisted settings feed the Root consumer.
/// SQL adds declared receipt data on a private fixture with the real identity
/// projection; it does not fabricate an engine execution or acceptance proof.
#[test]
fn root_consumer_reloads_stored_task_and_saved_output_policy_and_recovers_after_restart() {
    use swarm_engine::{RootStore, SettingCap, SettingCount, SystemEnvironment};
    let f = Fixture::new();
    let mut store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let w=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Validation","objective":"Data","scope":"Fixture","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap().result.work;
    let request = |event: &str, revision: i64, mut fields: serde_json::Value| {
        fields["schema_version"] = 1.into();
        fields["event_id"] = event.into();
        fields["expected_revision"] = revision.into();
        serde_json::to_vec(&fields).unwrap()
    };
    let w=store.execute_external(&w.id,"task.add",&request("add",w.revision,serde_json::json!({"id":"task","title":"Task","deliverable":"report.md","criteria":["Proof"]})),&mut SystemEnvironment).unwrap().result.work;
    assert!(store
        .inspect_validation_evidence(&w.id, "task")
        .unwrap()
        .evidence
        .is_none());
    let mut w = store
        .execute_external(
            &w.id,
            "task.update",
            &request(
                "start",
                w.revision,
                serde_json::json!({"id":"task","status":"running"}),
            ),
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work;
    let actual = &mut w.tasks.as_mut().unwrap()[0];
    let supplied = task();
    actual.validation_policy = supplied.validation_policy;
    actual.automatic_validation = supplied.automatic_validation;
    actual
        .automatic_validation
        .as_deref_mut()
        .unwrap()
        .attempt_id = actual.attempts.as_ref().unwrap().last().unwrap().id.clone();
    w.revision += 1;
    store.close().unwrap();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    db.execute(
        "UPDATE works SET revision=?1,body=?2 WHERE id=?3",
        rusqlite::params![
            w.revision,
            String::from_utf8(swarm_engine::encode_model(&w).unwrap()).unwrap(),
            w.id
        ],
    )
    .unwrap();
    db.execute(
        "INSERT INTO events VALUES('fixture-validation',?1,?2,'task.auto-validation','date',?3,?3)",
        rusqlite::params![
            w.id,
            w.revision,
            swarm_engine::encode_model(
                w.tasks.as_ref().unwrap()[0]
                    .automatic_validation
                    .as_ref()
                    .unwrap()
            )
            .unwrap()
        ],
    )
    .unwrap();
    db.close().unwrap();
    let store = RootStore::open(&f.0, true).unwrap();
    let observed = store.inspect_validation_evidence(&w.id, "task").unwrap();
    assert_eq!(observed.evidence.unwrap().observations, ["proof"]);
    let before = swarm_engine::encode_model(&store.read_work(&w.id).unwrap()).unwrap();
    let change = |store: &RootStore, cap: Option<i64>, event: &str| {
        let document = store.settings().unwrap();
        let mut values = document.values;
        values.validation_max_output_bytes = Some(SettingCap(cap.map(SettingCount)));
        let mut request = swarm_engine::EngineSettingsChange {
            schema_version: document.schema_version,
            event_id: event.into(),
            expected_revision: document.revision,
            values,
            reason: "Actual consumer output policy".into(),
            preview_token: String::new(),
        };
        request.preview_token = store.preview_settings(&request).unwrap().preview_token;
        store
            .apply_settings(&request, &mut SystemEnvironment)
            .unwrap();
    };
    change(&store, Some(4), "lower");
    store.close().unwrap();
    let store = RootStore::open(&f.0, true).unwrap();
    let saved = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    assert_eq!(
        store
            .inspect_validation_evidence(&w.id, "task")
            .err()
            .unwrap()
            .code(),
        "validation-evidence-invalid"
    );
    assert_eq!(
        swarm_engine::encode_model(&store.read_work(&w.id).unwrap()).unwrap(),
        before
    );
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        saved
    );
    change(&store, None, "unlimited");
    store.close().unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    assert_eq!(
        store
            .inspect_validation_evidence(&w.id, "task")
            .unwrap()
            .evidence
            .unwrap()
            .observations,
        ["proof"]
    );
    assert_eq!(
        swarm_engine::encode_model(&store.read_work(&w.id).unwrap()).unwrap(),
        before
    );
    store.close().unwrap();
}
