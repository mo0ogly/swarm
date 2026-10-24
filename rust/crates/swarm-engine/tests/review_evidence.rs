use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fs, path::PathBuf};
use swarm_engine::{
    check_review_evidence, review_contract, Attempt, EngineSettingsValues, IndependentReview,
    ManagedRepository, PlanningState, ReviewerConfig, Task, Work,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("review-evidence-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::write(p.join("report.md"), b"Review report").unwrap();
        fs::write(p.join("delivery.md"), b"Delivery").unwrap();
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
fn sha(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn work() -> Work {
    let mut t = Task {
        id: "task".into(),
        title: "Task".into(),
        deliverable: "delivery.md".into(),
        criteria: Some(vec!["Proof".into()]),
        attempts: Some(vec![Attempt {
            id: "attempt".into(),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let contract = review_contract(&t).unwrap();
    t.independent_review = Some(Box::new(IndependentReview {
        id: "review".into(),
        attempt: "attempt".into(),
        contract,
        state: "passed".into(),
        report: "report.md".into(),
        sha256: sha(b"Review report"),
        report_artifacts: Some(BTreeMap::from([("delivery.md".into(), sha(b"Delivery"))])),
        ..Default::default()
    }));
    Work {
        planning: Some(Box::new(PlanningState {
            reviewer: Some(Box::new(ReviewerConfig::default())),
            ..Default::default()
        })),
        tasks: Some(vec![t]),
        ..Default::default()
    }
}
#[test]
fn review_evidence_rereads_report_artifacts_and_context_with_recovery() {
    let f = Fixture::new();
    let w = work();
    let limits = EngineSettingsValues::installation_defaults()
        .unwrap()
        .context_policy_limits()
        .unwrap();
    let baseline = check_review_evidence(&f.0, &w, "task", Some(&limits))
        .unwrap()
        .unwrap();
    assert_eq!(baseline.review_id, "review");
    for (file, body) in [
        ("report.md", b"Review report".as_slice()),
        ("delivery.md", b"Delivery".as_slice()),
    ] {
        check_review_evidence(&f.0, &w, "task", Some(&limits)).unwrap();
        fs::write(f.0.join(file), b"Changed").unwrap();
        assert!(check_review_evidence(&f.0, &w, "task", Some(&limits)).is_err());
        assert_eq!(fs::read(f.0.join(file)).unwrap(), b"Changed");
        fs::write(f.0.join(file), body).unwrap();
        check_review_evidence(&f.0, &w, "task", Some(&limits)).unwrap();
        fs::remove_file(f.0.join(file)).unwrap();
        let e = check_review_evidence(&f.0, &w, "task", Some(&limits)).unwrap_err();
        assert!(e.source().is_some());
        fs::write(f.0.join(file), body).unwrap();
        check_review_evidence(&f.0, &w, "task", Some(&limits)).unwrap();
    }
    fs::write(f.0.join("swarm.project.json"),br#"{"version":1,"name":"New profile","instructions":[{"path":"delivery.md","roles":["reviewer"]}]}"#).unwrap();
    assert!(check_review_evidence(&f.0, &w, "task", Some(&limits)).is_err());
    fs::remove_file(f.0.join("swarm.project.json")).unwrap();
    check_review_evidence(&f.0, &w, "task", Some(&limits)).unwrap();
}
#[test]
fn review_evidence_requires_attempt_contract_reviewer_and_complete_policy() {
    let f = Fixture::new();
    let w = work();
    let limits = EngineSettingsValues::installation_defaults()
        .unwrap()
        .context_policy_limits()
        .unwrap();
    for fault in [
        "reviewer", "review", "state", "attempt", "criteria", "brief", "policy", "managed",
    ] {
        check_review_evidence(&f.0, &w, "task", Some(&limits)).unwrap();
        let mut changed = w.clone();
        let task = &mut changed.tasks.as_mut().unwrap()[0];
        match fault {
            "reviewer" => changed.planning.as_mut().unwrap().reviewer = None,
            "review" => task.independent_review = None,
            "state" => task.independent_review.as_mut().unwrap().state = "running".into(),
            "attempt" => task.attempts = None,
            "criteria" => task.criteria = Some(vec!["Changed".into()]),
            "brief" => task.plan_brief_hash = "new".into(),
            "managed" => {
                changed.planning.as_mut().unwrap().repository =
                    Some(Box::new(ManagedRepository::default()))
            }
            _ => {}
        }
        let result = check_review_evidence(
            &f.0,
            &changed,
            "task",
            if fault == "policy" {
                None
            } else {
                Some(&limits)
            },
        );
        assert!(result.is_err(), "{fault}");
        check_review_evidence(&f.0, &w, "task", Some(&limits)).unwrap();
    }
    let mut manual = w.clone();
    manual.planning = None;
    assert!(check_review_evidence(&f.0, &manual, "task", None)
        .unwrap()
        .is_none());
    assert!(check_review_evidence(&f.0, &w, "missing", Some(&limits)).is_err());
}

/// A declared native namespace is only a protocol selector. An ordinary valid
/// fixture becomes invalid without the actual Root ledger/originals; the older
/// no-settings API must not silently treat those claims as ordinary evidence.
#[cfg(target_os = "linux")]
#[test]
fn declared_native_review_never_falls_back_to_ordinary_supplied_data() {
    let f = Fixture::new();
    let mut w = work();
    let settings = swarm_engine::EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(), "2026-10-22T00:00:00Z".into(),
    ).unwrap();
    assert!(swarm_engine::check_current_review_evidence(&f.0, &w, "task", &settings).unwrap().is_some());
    w.tasks.as_mut().unwrap()[0].independent_review.as_mut().unwrap().receipt =
        ".swarm/native-review-public-v2-declared-only.json".into();
    assert!(matches!(
        swarm_engine::check_current_review_evidence(&f.0, &w, "task", &settings),
        Err(swarm_engine::ReviewEvidenceError::Native(_))
    ));
    let limits = settings.values.context_policy_limits().unwrap();
    assert!(matches!(
        check_review_evidence(&f.0, &w, "task", Some(&limits)),
        Err(swarm_engine::ReviewEvidenceError::Refused("native-review-current-root-required"))
    ));
}
