use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("gate-contract-").unwrap());
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap()
    }
}
use swarm_engine::{
    check_gate_evidence, prepare_gate_record, validate_required_plan_checks, EngineSettingsChange,
    EngineSettingsDocument, EngineSettingsValues, GateSubmission, SettingCount, Task,
};
#[test]
fn required_controls_use_the_same_exact_keys_as_evaluation() {
    let t = Task {
        plan_checks: Some(BTreeMap::from([("required".into(), "delivery".into())])),
        ..Default::default()
    };
    let good = br#"{"checks":[{"id":"required","mandatory":true,"gate":"delivery"}]}"#;
    for bad in [
        r#"{"checks":[{"id":"other","mandatory":true,"gate":"delivery"}],"CHECKS":[{"id":"required","mandatory":true,"gate":"delivery"}]}"#,
        r#"{"checks":[{"id":"other","ID":"required","mandatory":true,"gate":"delivery"}]}"#,
        r#"{"checks":[{"id":"required","mandatory":false,"MANDATORY":true,"gate":"delivery"}]}"#,
        r#"{"checks":[{"id":"required","mandatory":true,"gate":"entry","GATE":"delivery"}]}"#,
        r#"{"checks":[{"id":"required","mandatory":true,"gate":"delivery"}],"checks":[]}"#,
        r#"{"checks":null}"#,
    ] {
        validate_required_plan_checks(&t, good).unwrap();
        assert!(validate_required_plan_checks(&t, bad.as_bytes()).is_err());
        validate_required_plan_checks(&t, good).unwrap();
    }
    validate_required_plan_checks(&t,br#"{"checks":[{"id":"required","mandatory":true,"gate":"delivery","MANDATORY":false}],"CHECKS":[]}"#).unwrap();
}
#[test]
fn receipt_rechecks_artifacts_scope_required_checks_and_policy_generation_after_revert() {
    let root = Fixture::new();
    fs::write(root.path().join("proof.txt"), b"proof").unwrap();
    let d = EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "operator".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap();
    let profile = d.quality_policy_snapshot().unwrap();
    let raw=serde_json::to_vec(&serde_json::json!({"method_version":"3","scope_id":"task","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":[{"id":"required","domain":"quality","mandatory":true,"gate":"delivery","severity":"minor","penalty":1,"max_penalty":100}],"results":[{"id":"required","status":"PASS","count":0,"evidence":["proof.txt"]}]})).unwrap();
    let t = Task {
        id: "task".into(),
        status: "submitted".into(),
        plan_checks: Some(BTreeMap::from([("required".into(), "delivery".into())])),
        ..Default::default()
    };
    let before = swarm_engine::encode_model(&t).unwrap();
    let staged = prepare_gate_record(
        &t,
        &GateSubmission {
            name: "  Delivery proof  ",
            phase: "delivery",
            document: &raw,
            at: "2026-10-18T00:00:00Z",
        },
        root.path(),
        &profile,
    )
    .unwrap();
    assert_eq!(staged.gate.as_ref().unwrap().name, "Delivery proof");
    assert_eq!(swarm_engine::encode_model(&t).unwrap(), before);
    check_gate_evidence(&staged, root.path(), &profile).unwrap();
    let control_policy = |settings: &EngineSettingsDocument| {
        let profile = settings.quality_policy_snapshot().unwrap();
        let fresh = prepare_gate_record(
            &t,
            &GateSubmission {
                name: "New policy control",
                phase: "delivery",
                document: &raw,
                at: "2026-10-18T00:00:00Z",
            },
            root.path(),
            &profile,
        )
        .unwrap();
        check_gate_evidence(&fresh, root.path(), &profile).unwrap();
    };
    fs::write(root.path().join("proof.txt"), b"changed").unwrap();
    assert!(check_gate_evidence(&staged, root.path(), &profile).is_err());
    fs::write(root.path().join("proof.txt"), b"proof").unwrap();
    check_gate_evidence(&staged, root.path(), &profile).unwrap();
    let mut change = EngineSettingsChange {
        schema_version: d.schema_version,
        event_id: "raise".into(),
        expected_revision: d.revision,
        values: d.values.clone(),
        reason: "Higher quality bar".into(),
        preview_token: String::new(),
    };
    change.values.quality_delivery_min_percent = Some(SettingCount(90));
    change.preview_token = d.preview(&change).unwrap().preview_token;
    let raised = d
        .apply(&change, "operator".into(), "2026-10-18T00:00:01Z".into())
        .unwrap()
        .0;
    control_policy(&raised);
    assert!(check_gate_evidence(
        &staged,
        root.path(),
        &raised.quality_policy_snapshot().unwrap()
    )
    .is_err());
    change.event_id = "revert".into();
    change.expected_revision = raised.revision;
    change.values = d.values.clone();
    change.preview_token = raised.preview(&change).unwrap().preview_token;
    let reverted = raised
        .apply(&change, "operator".into(), "2026-10-18T00:00:02Z".into())
        .unwrap()
        .0;
    assert_eq!(
        reverted.quality_policy_snapshot().unwrap().values,
        profile.values
    );
    control_policy(&reverted);
    assert!(check_gate_evidence(
        &staged,
        root.path(),
        &reverted.quality_policy_snapshot().unwrap()
    )
    .is_err());
    for status in ["accepted", "waived", "abandoned"] {
        check_gate_evidence(&staged, root.path(), &profile).unwrap();
        let mut closed = t.clone();
        closed.status = status.into();
        assert!(prepare_gate_record(
            &closed,
            &GateSubmission {
                name: "Proof",
                phase: "delivery",
                document: &raw,
                at: "2026-10-18T00:00:00Z"
            },
            root.path(),
            &profile
        )
        .is_err());
    }
    for alteration in ["scope", "required", "stored-evaluation", "stale"] {
        check_gate_evidence(&staged, root.path(), &profile).unwrap();
        let mut bad = staged.clone();
        match alteration {
            "scope" => bad.id = "other".into(),
            "required" => {
                bad.plan_checks
                    .as_mut()
                    .unwrap()
                    .insert("missing".into(), "delivery".into());
            }
            "stored-evaluation" => {
                bad.gate.as_mut().unwrap().evaluation.quality_band = "STOP".into()
            }
            _ => bad.evidence_stale_reason = "Known drift".into(),
        };
        assert!(check_gate_evidence(&bad, root.path(), &profile).is_err());
    }
}

#[test]
fn historical_gate_requires_compatible_profile_and_revalidation_requires_bound_report() {
    let root = Fixture::new();
    fs::write(root.path().join("proof.txt"), b"proof").unwrap();
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    let document = EngineSettingsDocument::initial(
        defaults.clone(),
        "operator".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap();
    let policy = document.quality_policy_snapshot().unwrap();
    let mut raw = serde_json::json!({"method_version":"2","scope_id":"task","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":[{"id":"required","domain":"quality","mandatory":true,"gate":"delivery","severity":"minor","penalty":1,"max_penalty":100}],"results":[{"id":"required","status":"PASS","count":0,"evidence":["proof.txt"]}]});
    let mut task = Task {
        id: "task".into(),
        status: "submitted".into(),
        ..Default::default()
    };
    let prepare =
        |task: &Task, raw: &serde_json::Value, profile: &swarm_engine::QualityPolicySnapshot| {
            prepare_gate_record(
                task,
                &GateSubmission {
                    name: "Proof",
                    phase: "delivery",
                    document: &serde_json::to_vec(raw).unwrap(),
                    at: "2026-10-18T00:00:00Z",
                },
                root.path(),
                profile,
            )
        };
    let first = prepare(&task, &raw, &policy).unwrap();
    check_gate_evidence(&first, root.path(), &policy).unwrap();
    let mut raised = defaults;
    raised.quality_delivery_min_percent = Some(SettingCount(90));
    let raised =
        EngineSettingsDocument::initial(raised, "operator".into(), "2026-10-18T00:00:00Z".into())
            .unwrap()
            .quality_policy_snapshot()
            .unwrap();
    let mut current = raw.clone();
    current["method_version"] = "3".into();
    let current_control = prepare(&task, &current, &raised).unwrap();
    check_gate_evidence(&current_control, root.path(), &raised).unwrap();
    assert!(prepare(&task, &raw, &raised).is_err());
    assert!(check_gate_evidence(&first, root.path(), &raised).is_err());
    check_gate_evidence(&first, root.path(), &policy).unwrap();
    // A supplied snapshot may be consistent but this primitive does not assert
    // that it is still authoritative. Root/acceptance integration must provide it.
    task.revalidation = Some(Box::new(swarm_engine::Revalidation {
        config_digest: first
            .gate
            .as_ref()
            .unwrap()
            .evaluation
            .config_digest
            .clone(),
        ..Default::default()
    }));
    assert!(prepare(&task, &raw, &policy).is_err());
    fs::write(root.path().join("report.txt"), b"new control report").unwrap();
    let hash = format!("{:x}", Sha256::digest(b"new control report"));
    raw["artifacts"]["report.txt"] = hash.clone().into();
    task.revalidation.as_mut().unwrap().report = "report.txt".into();
    task.revalidation.as_mut().unwrap().report_hash = hash;
    let updated = prepare(&task, &raw, &policy).unwrap();
    check_gate_evidence(&updated, root.path(), &policy).unwrap();
    let encoded = swarm_engine::encode_model(&updated).unwrap();
    let decoded = swarm_engine::decode_task(&encoded).unwrap();
    assert_eq!(decoded, updated);
    check_gate_evidence(&decoded, root.path(), &policy).unwrap();
    for alteration in ["report-hash", "bar", "not-submitted"] {
        assert!(prepare(&task, &raw, &policy).is_ok());
        let mut bad = task.clone();
        match alteration {
            "report-hash" => bad.revalidation.as_mut().unwrap().report_hash = "stale".into(),
            "bar" => bad.revalidation.as_mut().unwrap().config_digest = "different".into(),
            _ => bad.status = "todo".into(),
        };
        let original = swarm_engine::encode_model(&bad).unwrap();
        assert!(prepare(&bad, &raw, &policy).is_err());
        assert_eq!(swarm_engine::encode_model(&bad).unwrap(), original);
    }
    let mut changed = raw.clone();
    changed["checks"][0]["max_penalty"] = 99.into();
    assert!(prepare(&first, &changed, &policy).is_err());
    for name in ["", " \t\n"] {
        assert!(prepare_gate_record(
            &task,
            &GateSubmission {
                name,
                phase: "delivery",
                document: &serde_json::to_vec(&raw).unwrap(),
                at: "2026-10-18T00:00:00Z"
            },
            root.path(),
            &policy
        )
        .is_err());
    }
}
