use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use swarm_engine::{
    evaluate_gate_method2, evaluate_gate_with_policy, EngineSettingsChange, EngineSettingsDocument,
    EngineSettingsValues, SettingCount,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(swarm_engine::generate_identifier("gate-eval-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::write(p.join("proof.txt"), b"proof").unwrap();
        Self(p)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap()
    }
}
fn document() -> serde_json::Value {
    serde_json::json!({"method_version":"2","scope_id":"task","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":[{"id":"score","domain":"quality","mandatory":true,"gate":"delivery","severity":"minor","penalty":20,"max_penalty":100},{"id":"critical","mandatory":false,"gate":"delivery","severity":"critical","penalty":1,"max_penalty":10}],"results":[{"id":"score","status":"PASS","count":0,"evidence":["proof.txt"]},{"id":"critical","status":"PASS","count":0,"evidence":["proof.txt"]}]})
}
#[test]
fn method2_computes_real_artifacts_and_refuses_status_compositions_stale_paths_and_numeric_overflow(
) {
    let f = Fixture::new();
    let good = document();
    let bytes = serde_json::to_vec(&good).unwrap();
    let v = evaluate_gate_method2(&bytes, &f.0, "delivery").unwrap();
    assert!(v.ship_allowed);
    assert_eq!(v.quality.unwrap().0, 100.0);
    for status in ["FAIL|PARTIEL", "PASS|FAIL", "PASS ", "UNKNOWN"] {
        assert!(
            evaluate_gate_method2(&bytes, &f.0, "delivery")
                .unwrap()
                .ship_allowed
        );
        let mut bad = good.clone();
        bad["results"][1]["status"] = status.into();
        bad["results"][1]["count"] = 1.into();
        assert!(
            evaluate_gate_method2(&serde_json::to_vec(&bad).unwrap(), &f.0, "delivery").is_err()
        );
        assert_eq!(fs::read(f.0.join("proof.txt")).unwrap(), b"proof");
    }
    let mut fail = good.clone();
    fail["results"][1]["status"] = "FAIL".into();
    fail["results"][1]["count"] = 1.into();
    let v = evaluate_gate_method2(&serde_json::to_vec(&fail).unwrap(), &f.0, "delivery").unwrap();
    assert!(!v.ship_allowed);
    assert_eq!(v.max_severity, "critical");
    let mut big = good.clone();
    big["domains"]["quality"] = serde_json::json!(1e308);
    assert!(evaluate_gate_method2(&serde_json::to_vec(&big).unwrap(), &f.0, "delivery").is_err());
    fs::write(f.0.join("proof.txt"), b"changed").unwrap();
    assert!(evaluate_gate_method2(&bytes, &f.0, "delivery").is_err());
    fs::write(f.0.join("proof.txt"), b"proof").unwrap();
    assert!(
        evaluate_gate_method2(&bytes, &f.0, "delivery")
            .unwrap()
            .ship_allowed
    );
    let mut outside = good.clone();
    outside["results"][1]["evidence"] = serde_json::json!(["../outside"]);
    assert!(
        evaluate_gate_method2(&serde_json::to_vec(&outside).unwrap(), &f.0, "delivery").is_err()
    );
}
#[test]
fn current_policy_changes_quality_decision_and_digest_without_mutating_the_journal() {
    let f = Fixture::new();
    let doc = EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "operator".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap();
    let first = doc.quality_policy_snapshot().unwrap();
    let mut raw = document();
    raw["method_version"] = "3".into();
    raw["checks"][1]["severity"] = "minor".into();
    raw["checks"][0]["mandatory"] = false.into();
    raw["results"][0]["status"] = "FAIL".into();
    raw["results"][0]["count"] = 1.into();
    let bytes = serde_json::to_vec(&raw).unwrap();
    let initial = evaluate_gate_with_policy(&bytes, &f.0, "delivery", &first).unwrap();
    assert!(initial.evaluation.ship_allowed);
    assert_eq!(initial.evaluation.quality.as_ref().unwrap().0, 80.0);
    let before = swarm_wire_json::encode(&doc).unwrap();
    let mut values = doc.values.clone();
    values.quality_delivery_min_percent = Some(SettingCount(90));
    let mut change = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "raise-quality".into(),
        expected_revision: doc.revision,
        values,
        reason: "Higher delivery bar".into(),
        preview_token: String::new(),
    };
    change.preview_token = doc.preview(&change).unwrap().preview_token;
    let next = doc
        .apply(&change, "operator".into(), "2026-10-18T00:00:01Z".into())
        .unwrap()
        .0;
    let result = evaluate_gate_with_policy(
        &bytes,
        &f.0,
        "delivery",
        &next.quality_policy_snapshot().unwrap(),
    )
    .unwrap();
    assert!(!result.evaluation.ship_allowed);
    assert_ne!(
        result.evaluation.config_digest,
        initial.evaluation.config_digest
    );
    assert_eq!(swarm_wire_json::encode(&doc).unwrap(), before);
    let mut tampered = first.clone();
    tampered.values.quality_delivery_min_percent = SettingCount(0);
    assert!(evaluate_gate_with_policy(&bytes, &f.0, "delivery", &tampered).is_err());
}

#[test]
fn every_admin_quality_threshold_changes_the_calculated_contract() {
    let f = Fixture::new();
    let mut raw = document();
    raw["method_version"] = "3".into();
    raw["checks"][0]["mandatory"] = false.into();
    raw["results"][0]["status"] = "FAIL".into();
    raw["results"][0]["count"] = 1.into();
    let bytes = serde_json::to_vec(&raw).unwrap();
    let mut previous = std::collections::BTreeSet::new();
    for (global, domain, ok, warn, blocked, band, ship) in [
        (70, 50, 80, 70, 50, "OK", true),
        (90, 50, 80, 70, 50, "OK", false),
        (70, 90, 80, 70, 50, "OK", false),
        (70, 50, 90, 70, 50, "WARN", true),
        (70, 50, 90, 85, 50, "BLOQUÉ", true),
        (70, 50, 95, 90, 85, "STOP", true),
    ] {
        let mut values = EngineSettingsValues::installation_defaults().unwrap();
        values.quality_delivery_min_percent = Some(SettingCount(global));
        values.quality_domain_min_percent = Some(SettingCount(domain));
        values.quality_band_ok_min_percent = Some(SettingCount(ok));
        values.quality_band_warn_min_percent = Some(SettingCount(warn));
        values.quality_band_blocked_min_percent = Some(SettingCount(blocked));
        let doc = EngineSettingsDocument::initial(
            values,
            "operator".into(),
            "2026-10-18T00:00:00Z".into(),
        )
        .unwrap();
        let policy = doc.quality_policy_snapshot().unwrap();
        let result = evaluate_gate_with_policy(&bytes, &f.0, "delivery", &policy).unwrap();
        assert_eq!(result.evaluation.quality_band, band);
        assert_eq!(result.evaluation.ship_allowed, ship);
        assert!(previous.insert(result.evaluation.config_digest));
        assert_eq!(result.evaluation.quality.unwrap().0, 80.0);
    }
}
