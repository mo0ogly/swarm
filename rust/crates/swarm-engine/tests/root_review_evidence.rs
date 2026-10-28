#![cfg(unix)]
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};
use swarm_engine::{
    encode_model, register_identity_plan_projection, review_contract, Attempt, IndependentReview,
    PlanningState, ReviewerConfig, RootStore, SystemEnvironment, Work,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("root-review-").unwrap());
        fs::create_dir(&p).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
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
fn request(event: &str, rev: i64, fields: serde_json::Value) -> Vec<u8> {
    let mut v = fields;
    v["schema_version"] = 1.into();
    v["event_id"] = event.into();
    v["expected_revision"] = rev.into();
    serde_json::to_vec(&v).unwrap()
}
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn setup(f: &Fixture) -> Work {
    let mut s = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let w=s.execute_external("","work.create",&request("create",0,serde_json::json!({"title":"Review fixture","objective":"Data consistency only","scope":"Owned fixture","criteria":["Proof"]})),&mut SystemEnvironment).unwrap().result.work;
    let w=s.execute_external(&w.id,"task.add",&request("task",1,serde_json::json!({"id":"task","title":"Task","deliverable":"delivery.md","criteria":["Proof"]})),&mut SystemEnvironment).unwrap().result.work;
    s.close().unwrap();
    fs::write(f.0.join("report.md"), b"Review report").unwrap();
    fs::write(f.0.join("delivery.md"), b"Delivery").unwrap();
    w
}
// Explicit owned fixture injection. Real schema triggers/UDF stay enabled.
fn seed(f: &Fixture, w: &Work) {
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.busy_timeout(std::time::Duration::ZERO).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    register_identity_plan_projection(&db).unwrap();
    assert_eq!(
        db.execute(
            "UPDATE works SET body=?1 WHERE id=?2 AND revision=?3",
            rusqlite::params![
                String::from_utf8(encode_model(w).unwrap()).unwrap(),
                w.id,
                w.revision
            ]
        )
        .unwrap(),
        1
    );
    db.close().unwrap();
}
fn reviewed(mut w: Work) -> Work {
    w.planning = Some(Box::new(PlanningState {
        reviewer: Some(Box::new(ReviewerConfig::default())),
        ..Default::default()
    }));
    let t = &mut w.tasks.as_mut().unwrap()[0];
    t.status = "submitted".into();
    t.attempts = Some(vec![Attempt {
        id: "fixture-attempt".into(),
        status: "completed".into(),
        ..Default::default()
    }]);
    let contract = review_contract(t).unwrap();
    t.independent_review = Some(Box::new(IndependentReview {
        id: "fixture-review".into(),
        attempt: "fixture-attempt".into(),
        contract,
        state: "passed".into(),
        report: "report.md".into(),
        sha256: hash(b"Review report"),
        report_artifacts: Some(BTreeMap::from([("delivery.md".into(), hash(b"Delivery"))])),
        ..Default::default()
    }));
    w
}
/// Require the exact current refusal for this deliberately minimal read fixture.
/// Its planning policy is incomplete, so validation must refuse before native
/// acceptance. This proves no valid-planning acceptance contract. Traverse typed
/// causes including cleanup wrappers; never replace this with any format error.
fn assert_fixture_acceptance_refused(error: &swarm_engine::RootStoreError) {
    let mut pending: Vec<&(dyn std::error::Error + 'static)> = vec![error];
    let mut found = false;
    while let Some(cause) = pending.pop() {
        found |= matches!(
            cause.downcast_ref::<swarm_engine::PlanningStateError>(),
            Some(swarm_engine::PlanningStateError::Refused(
                swarm_engine::PlanningRefusal::Policy
            ))
        );
        pending.extend(
            swarm_engine::native_error_links(cause)
                .children
                .into_iter()
                .map(|(_, child)| child),
        );
    }
    assert!(
        found,
        "expected typed planning-policy-invalid fixture refusal"
    );
}
/// Actual ordinary review observation remains data only. This minimal planning
/// fixture refuses mutation at policy validation, not at native review authority.
/// Preserve state on refusal and recover observation after artifact restoration.
#[test]
fn root_review_observation_reads_actual_rows_and_files_without_mutation_or_acceptance() {
    let f = Fixture::new();
    let w = setup(&f);
    let s = RootStore::open(&f.0, false).unwrap();
    let none = s.inspect_review_evidence(&w.id, "task").unwrap();
    assert!(none.review.is_none());
    assert_eq!(none.scope, "data-consistency-only");
    s.close().unwrap();
    let w = reviewed(w);
    seed(&f, &w);
    let mut s = RootStore::open(&f.0, true).unwrap();
    let before = encode_model(&s.read_work(&w.id).unwrap()).unwrap();
    let events = s.events(&w.id).unwrap().len();
    let observation = s.inspect_review_evidence(&w.id, "task").unwrap();
    assert_eq!(observation.work_revision, w.revision);
    assert_eq!(observation.review.unwrap().review_id, "fixture-review");
    assert_eq!(encode_model(&s.read_work(&w.id).unwrap()).unwrap(), before);
    assert_eq!(s.events(&w.id).unwrap().len(), events);
    let denied = s
        .execute_external(
            &w.id,
            "task.update",
            &request(
                "still-unported",
                w.revision,
                serde_json::json!({"id":"task","status":"accepted"}),
            ),
            &mut SystemEnvironment,
        )
        .map(|_| ())
        .unwrap_err();
    assert_fixture_acceptance_refused(&denied);
    assert_eq!(encode_model(&s.read_work(&w.id).unwrap()).unwrap(), before);
    assert_eq!(s.events(&w.id).unwrap().len(), events);
    fs::write(f.0.join("report.md"), b"Changed").unwrap();
    assert!(s.inspect_review_evidence(&w.id, "task").is_err());
    fs::write(f.0.join("report.md"), b"Review report").unwrap();
    s.inspect_review_evidence(&w.id, "task").unwrap();
    s.close().unwrap();
    let mut changed = w.clone();
    changed.tasks.as_mut().unwrap()[0].criteria = Some(vec!["Changed".into()]);
    seed(&f, &changed);
    let s = RootStore::open(&f.0, false).unwrap();
    assert!(s.inspect_review_evidence(&w.id, "task").is_err());
    s.close().unwrap();
    seed(&f, &w);
    let s = RootStore::open(&f.0, false).unwrap();
    s.inspect_review_evidence(&w.id, "task").unwrap();
    assert!(s.inspect_review_evidence(&w.id, "missing").is_err());
    s.close().unwrap();
}

#[test]
fn root_review_observation_uses_current_policy_and_marks_its_revision() {
    use swarm_engine::{AgentWorkflow, EngineSettingsChange, SettingCap, SettingCount};
    let f = Fixture::new();
    let mut w = reviewed(setup(&f));
    fs::write(f.0.join("instructions.md"), b"Review context").unwrap();
    fs::write(f.0.join("swarm.project.json"),br#"{"version":1,"name":"Review","instructions":[{"path":"instructions.md","roles":["reviewer"]}]}"#).unwrap();
    let s = RootStore::open(&f.0, false).unwrap();
    let context = s.project_context("reviewer").unwrap().unwrap().context;
    s.close().unwrap();
    w.tasks.as_mut().unwrap()[0]
        .independent_review
        .as_mut()
        .unwrap()
        .workflow = Some(Box::new(AgentWorkflow {
        project: Some(Box::new(context)),
        ..Default::default()
    }));
    seed(&f, &w);
    let s = RootStore::open(&f.0, true).unwrap();
    let before = encode_model(&s.read_work(&w.id).unwrap()).unwrap();
    let events = s.events(&w.id).unwrap().len();
    for (event, cap, allowed) in [
        ("small", Some(SettingCount(1)), false),
        ("zero", Some(SettingCount(0)), false),
        ("unlimited", None, true),
    ] {
        for (suffix, value) in [("control", None), ("value", cap)] {
            let doc = s.settings().unwrap();
            let mut values = doc.values.clone();
            values.project_context_max_bytes = Some(SettingCap(value));
            let mut change = EngineSettingsChange {
                schema_version: doc.schema_version,
                event_id: format!("{event}-{suffix}"),
                expected_revision: doc.revision,
                values,
                reason: "Actual reviewer context policy".into(),
                preview_token: String::new(),
            };
            change.preview_token = s.preview_settings(&change).unwrap().preview_token;
            s.apply_settings(&change, &mut SystemEnvironment).unwrap();
            let current = s.settings().unwrap();
            let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
            let observed = s.inspect_review_evidence(&w.id, "task");
            if suffix == "control" || allowed {
                let observed = observed.unwrap();
                assert_eq!(observed.settings_revision, current.revision);
                assert!(observed.review.is_some());
            } else {
                let error = observed.unwrap_err();
                assert_eq!(error.code(), "review-evidence-invalid");
                assert!(error.to_string().contains("configured-cap"));
            }
            assert_eq!(
                fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
                settings
            );
            assert_eq!(encode_model(&s.read_work(&w.id).unwrap()).unwrap(), before);
            assert_eq!(s.events(&w.id).unwrap().len(), events);
        }
    }
    s.close().unwrap();
    let s = RootStore::open(&f.0, false).unwrap();
    s.inspect_review_evidence(&w.id, "task").unwrap();
    s.close().unwrap();
}
/// Verify the actual Root row, rereads and explicit refusal of managed acceptance.
/// Managed anchor observation supplies candidate/file consistency only, not a
/// native receipt or acceptance. Its incomplete planning policy refuses mutation
/// before native authorization. Refusal preserves canonical rows and events;
/// changed/restored artifacts verify the independent observation/recovery path.
#[test]
fn root_managed_anchor_reads_actual_row_but_never_enables_acceptance() {
    let f = Fixture::new();
    let mut w = reviewed(setup(&f));
    w.planning.as_mut().unwrap().repository = Some(Box::new(swarm_engine::ManagedRepository {
        candidate_commit: "fixture-candidate".into(),
        ..Default::default()
    }));
    let t = &mut w.tasks.as_mut().unwrap()[0];
    t.automatic_validation = Some(Box::new(swarm_engine::AutomaticValidation {
        candidate_sha: "fixture-candidate".into(),
        ..Default::default()
    }));
    let r = t.independent_review.as_mut().unwrap();
    r.candidate_commit = "fixture-candidate".into();
    r.receipt = "receipt.json".into();
    r.receipt_sha256 = hash(b"{}");
    r.context = "context.json".into();
    r.context_sha256 = hash(b"{}");
    fs::write(f.0.join("receipt.json"), b"{}").unwrap();
    fs::write(f.0.join("context.json"), b"{}").unwrap();
    seed(&f, &w);
    let mut s = RootStore::open(&f.0, true).unwrap();
    let before = encode_model(&s.read_work(&w.id).unwrap()).unwrap();
    let events = s.events(&w.id).unwrap().len();
    let observed = s.inspect_managed_review_anchor(&w.id, "task").unwrap();
    assert_eq!(observed.work_revision, w.revision);
    assert_eq!(observed.settings_revision, s.settings().unwrap().revision);
    assert_eq!(observed.anchor.scope, "candidate-and-file-bindings-only");
    assert!(
        !observed.anchor.git_report_verified
            && !observed.anchor.protocol_verified
            && !observed.anchor.authority_verified
    );
    assert_eq!(
        s.inspect_review_evidence(&w.id, "task").unwrap_err().code(),
        "review-evidence-invalid"
    );
    let refused = s
        .execute_external(
            &w.id,
            "task.update",
            &request(
                "accept-managed",
                w.revision,
                serde_json::json!({"id":"task","status":"accepted"}),
            ),
            &mut SystemEnvironment,
        )
        .map(|_| ())
        .unwrap_err();
    assert_fixture_acceptance_refused(&refused);
    assert_eq!(encode_model(&s.read_work(&w.id).unwrap()).unwrap(), before);
    assert_eq!(s.events(&w.id).unwrap().len(), events);
    fs::write(f.0.join("receipt.json"), b"changed").unwrap();
    assert!(s.inspect_managed_review_anchor(&w.id, "task").is_err());
    fs::write(f.0.join("receipt.json"), b"{}").unwrap();
    s.inspect_managed_review_anchor(&w.id, "task").unwrap();
    s.close().unwrap();
    // A later row must be reread; a successful earlier observation is no lease.
    w.tasks.as_mut().unwrap()[0]
        .automatic_validation
        .as_mut()
        .unwrap()
        .candidate_sha = "other".into();
    seed(&f, &w);
    let s = RootStore::open(&f.0, false).unwrap();
    assert!(s.inspect_managed_review_anchor(&w.id, "task").is_err());
    s.close().unwrap();
    w.tasks.as_mut().unwrap()[0]
        .automatic_validation
        .as_mut()
        .unwrap()
        .candidate_sha = "fixture-candidate".into();
    seed(&f, &w);
    let s = RootStore::open(&f.0, false).unwrap();
    s.inspect_managed_review_anchor(&w.id, "task").unwrap();
    assert_eq!(s.events(&w.id).unwrap().len(), events);
    s.close().unwrap();
}

/// Create immutable local Git objects with deterministic fixture identity. No
/// shell, provider, hook-running commit command or verification watchdog is used.
#[cfg(target_os = "linux")]
fn managed_fixture_git(f: &Fixture, args: &[&str], raw: &[u8]) -> String {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let mut child = Command::new("git")
        .args(args)
        .current_dir(&f.0)
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@localhost")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@localhost")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(raw).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
/// Produce a closed private Work with a simple managed review and a real report
/// commit. This seeds data only; no agent, independent verdict or acceptance runs.
#[cfg(target_os = "linux")]
fn managed_reviewed(f: &Fixture) -> Work {
    let mut w = reviewed(setup(f));
    managed_fixture_git(f, &["init", "--bare", "repository.git"], b"");
    let blob = managed_fixture_git(
        f,
        &["--git-dir=repository.git", "hash-object", "-w", "--stdin"],
        b"Review report",
    );
    let tree = managed_fixture_git(
        f,
        &["--git-dir=repository.git", "mktree"],
        format!("100644 blob {blob}\treport.md\n").as_bytes(),
    );
    let commit = managed_fixture_git(
        f,
        &["--git-dir=repository.git", "commit-tree", &tree],
        b"Fixture report",
    );
    let context=serde_json::to_vec(&serde_json::json!({"candidate_commit":commit,"previous_candidate":"","receipt":null,"tasks":[],"citation_contract_version":2})).unwrap();
    fs::write(f.0.join("managed.receipt"), b"Receipt").unwrap();
    fs::write(f.0.join("managed.context"), &context).unwrap();
    w.planning.as_mut().unwrap().repository = Some(Box::new(swarm_engine::ManagedRepository {
        storage: f.0.to_str().unwrap().into(),
        candidate_commit: commit.clone(),
        ..Default::default()
    }));
    let t = &mut w.tasks.as_mut().unwrap()[0];
    t.automatic_validation = Some(Box::new(swarm_engine::AutomaticValidation {
        candidate_sha: commit.clone(),
        ..Default::default()
    }));
    let r = t.independent_review.as_mut().unwrap();
    r.candidate_commit = commit;
    r.receipt = "managed.receipt".into();
    r.receipt_sha256 = hash(b"Receipt");
    r.context = "managed.context".into();
    r.context_sha256 = hash(&context);
    r.git_report = "report.md".into();
    w
}

/// Reuse real stored Work/settings and an actual committed report. The existing
/// managed-Git inspector is a valid witness; common review inspection must also
/// cover context, attempt and contract, without granting acceptance or mutation.
#[cfg(target_os = "linux")]
#[test]
fn root_current_review_composes_common_and_managed_git_guards_without_mutation() {
    let f = Fixture::new();
    let w = managed_reviewed(&f);
    seed(&f, &w);
    let store = RootStore::open(&f.0, false).unwrap();
    let witness = store.inspect_managed_review_git(&w.id, "task").unwrap();
    assert!(!witness.observation.authority_verified);
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let checked = store.inspect_review_evidence(&w.id, "task").unwrap();
    let review = checked.review.unwrap();
    assert_eq!(review.report_sha256, hash(b"Review report"));
    assert_eq!(
        review.contract,
        review_contract(&w.tasks.as_ref().unwrap()[0]).unwrap()
    );
    assert_eq!(before, fs::read(f.0.join(".swarm/state.db")).unwrap());
    assert_eq!(
        settings,
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap()
    );
    store.close().unwrap();
}

/// Every fault follows a current Root witness under the same real Git/file
/// preconditions. Refusals preserve DB/settings; restoration recovers. Unsupported
/// fragments/batches refuse, and missing-file IO retains a typed cause.
#[cfg(target_os = "linux")]
#[test]
fn root_current_managed_review_refuses_stale_bindings_and_recovers_without_mutation() {
    use std::error::Error as _;
    let f = Fixture::new();
    let w = managed_reviewed(&f);
    for fault in [
        "reviewer",
        "state",
        "attempt",
        "contract",
        "candidate",
        "automatic",
        "receipt",
        "context",
        "report",
        "git-report",
        "batches",
        "fragments",
    ] {
        seed(&f, &w);
        let store = RootStore::open(&f.0, false).unwrap();
        store.inspect_review_evidence(&w.id, "task").unwrap();
        store.close().unwrap();
        let mut bad = w.clone();
        let t = &mut bad.tasks.as_mut().unwrap()[0];
        let r = t.independent_review.as_mut().unwrap();
        match fault {
            "reviewer" => bad.planning.as_mut().unwrap().reviewer = None,
            "state" => r.state = "failed".into(),
            "attempt" => r.attempt = "PRIVATE_attempt".into(),
            "contract" => r.contract = "PRIVATE_contract".into(),
            "candidate" => r.candidate_commit = "PRIVATE_candidate".into(),
            "automatic" => {
                t.automatic_validation.as_mut().unwrap().candidate_sha = "PRIVATE_candidate".into()
            }
            "receipt" => r.receipt_sha256 = "PRIVATE_digest".into(),
            "context" => r.context_sha256 = "PRIVATE_digest".into(),
            "report" => r.sha256 = "PRIVATE_digest".into(),
            "git-report" => r.git_report = "PRIVATE_missing.md".into(),
            "batches" => r.batches = Some(vec![Default::default()]),
            "fragments" => r.fragment_journal = Some(Box::default()),
            _ => unreachable!(),
        }
        seed(&f, &bad);
        let store = RootStore::open(&f.0, false).unwrap();
        let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
        let policy = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        let e = store.inspect_review_evidence(&w.id, "task").unwrap_err();
        assert!(!format!("{e:?}").contains("PRIVATE"), "{fault}");
        assert_eq!(
            before,
            fs::read(f.0.join(".swarm/state.db")).unwrap(),
            "{fault}"
        );
        assert_eq!(
            policy,
            fs::read(f.0.join(".swarm/engine-settings.json")).unwrap()
        );
        store.close().unwrap();
        seed(&f, &w);
        let store = RootStore::open(&f.0, false).unwrap();
        store.inspect_review_evidence(&w.id, "task").unwrap();
        store.close().unwrap();
    }
    for name in ["managed.receipt", "managed.context", "report.md"] {
        let store = RootStore::open(&f.0, false).unwrap();
        store.inspect_review_evidence(&w.id, "task").unwrap();
        let bytes = fs::read(f.0.join(name)).unwrap();
        fs::remove_file(f.0.join(name)).unwrap();
        let e = store.inspect_review_evidence(&w.id, "task").unwrap_err();
        assert!(e.source().is_some());
        fs::write(f.0.join(name), bytes).unwrap();
        store.inspect_review_evidence(&w.id, "task").unwrap();
        store.close().unwrap();
    }
}

/// Hierarchical waiver data still requires a current configured review. Root
/// projections must read actual files/settings, preserve state and distinguish
/// a missing pending review from a passed historical record.
#[test]
fn root_ordinary_hierarchical_projection_requires_current_review_even_for_waivers() {
    let f = Fixture::new();
    let mut w = reviewed(setup(&f));
    let t = &mut w.tasks.as_mut().unwrap()[0];
    t.status = "waived".into();
    t.manual_override = Some(Box::default());
    t.manual_override.as_mut().unwrap().reason = "Declared fixture waiver".into();
    seed(&f, &w);
    let store = RootStore::open(&f.0, false).unwrap();
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let current = store
        .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
        .unwrap()
        .validation;
    assert_eq!(current.state, "waived");
    assert!(current.tasks["task"].fresh);
    assert_eq!(current.validated, 0);
    assert_eq!(before, fs::read(f.0.join(".swarm/state.db")).unwrap());
    store.close().unwrap();
    for fault in ["review", "reviewer", "state", "contract", "attempt"] {
        seed(&f, &w);
        let store = RootStore::open(&f.0, false).unwrap();
        assert!(
            store
                .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
                .unwrap()
                .validation
                .tasks["task"]
                .fresh
        );
        store.close().unwrap();
        let mut bad = w.clone();
        let t = &mut bad.tasks.as_mut().unwrap()[0];
        match fault {
            "review" => t.independent_review = None,
            "reviewer" => bad.planning.as_mut().unwrap().reviewer = None,
            "state" => t.independent_review.as_mut().unwrap().state = "failed".into(),
            "contract" => t.independent_review.as_mut().unwrap().contract = "stale".into(),
            "attempt" => t.attempts.as_mut().unwrap().push(Attempt {
                id: "new".into(),
                ..Default::default()
            }),
            _ => unreachable!(),
        }
        seed(&f, &bad);
        let store = RootStore::open(&f.0, false).unwrap();
        let v = store
            .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::English)
            .unwrap()
            .validation;
        assert_eq!(v.state, "blocked", "{fault}");
        assert!(!v.tasks["task"].fresh);
        assert_eq!(v.stale, 1);
        if fault == "review" {
            assert_eq!(v.tasks["task"].evidence.report_review.state, "pending");
        }
        store.close().unwrap();
    }
    seed(&f, &w);
    let store = RootStore::open(&f.0, false).unwrap();
    assert!(
        store
            .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["task"]
            .fresh
    );
    store.close().unwrap();
}

/// Actual Root projection propagates required-review freshness through dependency
/// order, and does not discard current-context or report-read failures.
#[test]
fn root_ordinary_projection_tracks_review_dependencies_context_and_read_errors() {
    use std::error::Error as _;
    let f = Fixture::new();
    let mut w = reviewed(setup(&f));
    let t = &mut w.tasks.as_mut().unwrap()[0];
    t.status = "waived".into();
    t.manual_override = Some(Box::default());
    t.manual_override.as_mut().unwrap().reason = "Fixture waiver".into();
    let mut dependent = w.tasks.as_ref().unwrap()[0].clone();
    dependent.id = "dependent".into();
    dependent.depends = Some(vec!["task".into()]);
    dependent.attempts.as_mut().unwrap()[0].id = "dependent-attempt".into();
    let contract = review_contract(&dependent).unwrap();
    let r = dependent.independent_review.as_mut().unwrap();
    r.id = "dependent-review".into();
    r.attempt = "dependent-attempt".into();
    r.contract = contract;
    w.tasks.as_mut().unwrap().insert(0, dependent);
    seed(&f, &w);
    let s = RootStore::open(&f.0, false).unwrap();
    let valid = s
        .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
        .unwrap()
        .validation;
    assert_eq!(valid.state, "waived");
    assert!(valid.tasks.values().all(|t| t.fresh));
    s.close().unwrap();
    let mut failed = w.clone();
    failed.tasks.as_mut().unwrap()[1]
        .independent_review
        .as_mut()
        .unwrap()
        .state = "failed".into();
    seed(&f, &failed);
    let s = RootStore::open(&f.0, false).unwrap();
    let v = s
        .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
        .unwrap()
        .validation;
    assert_eq!(v.stale, 2);
    assert!(!v.tasks["dependent"].fresh);
    s.close().unwrap();
    seed(&f, &w);
    let s = RootStore::open(&f.0, false).unwrap();
    assert!(
        s.inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["dependent"]
            .fresh
    );
    s.close().unwrap();
    fs::write(f.0.join("instructions.md"), b"Stable review instructions").unwrap();
    fs::write(f.0.join("swarm.project.json"),br#"{"version":1,"name":"Review context","instructions":[{"path":"instructions.md","roles":["reviewer"]}]}"#).unwrap();
    let settings = RootStore::open(&f.0, false).unwrap();
    let limits = settings
        .settings()
        .unwrap()
        .values
        .context_policy_limits()
        .unwrap();
    settings.close().unwrap();
    let context = swarm_engine::read_project_context(&f.0, "reviewer", &limits)
        .unwrap()
        .unwrap()
        .context;
    for t in w.tasks.as_mut().unwrap() {
        t.independent_review.as_mut().unwrap().workflow =
            Some(Box::new(swarm_engine::AgentWorkflow {
                project: Some(Box::new(context.clone())),
                ..Default::default()
            }));
    }
    seed(&f, &w);
    let s = RootStore::open(&f.0, false).unwrap();
    assert!(
        s.inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["dependent"]
            .fresh
    );
    fs::write(f.0.join("instructions.md"), b"Changed review instructions").unwrap();
    let stale = s
        .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
        .unwrap()
        .validation;
    assert_eq!(stale.stale, 2);
    fs::write(f.0.join("instructions.md"), b"Stable review instructions").unwrap();
    assert!(
        s.inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["dependent"]
            .fresh
    );
    fs::write(f.0.join("swarm.project.json"), b"PRIVATE invalid profile").unwrap();
    let error = s
        .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
        .map(|_| ())
        .unwrap_err();
    assert!(error.source().is_some());
    assert!(!format!("{error:?}").contains("PRIVATE"));
    s.close().unwrap();
}

/// An actual simple managed review/Git witness precedes the desired read model.
/// A waiver remains subject to current review; this fixture grants no acceptance.
#[cfg(target_os = "linux")]
#[test]
fn root_simple_managed_projection_composes_real_review_without_mutation() {
    let f = Fixture::new();
    let mut w = managed_reviewed(&f);
    let task = &mut w.tasks.as_mut().unwrap()[0];
    task.status = "waived".into();
    task.manual_override = Some(Box::new(swarm_engine::ManualOverride {
        reason: "Declared fixture waiver".into(),
        ..Default::default()
    }));
    let contract = review_contract(task).unwrap();
    task.independent_review.as_mut().unwrap().contract = contract;
    seed(&f, &w);
    let store = RootStore::open(&f.0, false).unwrap();
    assert!(store
        .inspect_review_evidence(&w.id, "task")
        .unwrap()
        .review
        .is_some());
    let db = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let observation = store
        .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
        .unwrap();
    assert_eq!(observation.validation.state, "waived");
    assert!(observation.validation.tasks["task"].fresh);
    assert_eq!(
        observation.validation.tasks["task"]
            .evidence
            .report_review
            .state,
        "passed"
    );
    assert!(observation.validation.tasks["task"]
        .evidence
        .report_review
        .limits[0]
        .contains("candidat"));
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), db);
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        settings
    );
    store.close().unwrap();
}

/// Declare a waiver over a real simple-review/Git fixture; this is supplied data,
/// not an independent verdict or supported acceptance operation.
#[cfg(target_os = "linux")]
fn managed_waiver(f: &Fixture) -> Work {
    let mut w = managed_reviewed(f);
    let task = &mut w.tasks.as_mut().unwrap()[0];
    task.status = "waived".into();
    task.manual_override = Some(Box::new(swarm_engine::ManualOverride {
        reason: "Fixture waiver".into(),
        ..Default::default()
    }));
    let contract = review_contract(task).unwrap();
    task.independent_review.as_mut().unwrap().contract = contract;
    w
}
/// A real receipt fallback displays declared controls while the missing original
/// automatic record still invalidates review freshness. Saved byte policy must
/// refuse the whole read and recover after explicit preview/apply, without writes
/// by observation. No command or identity is established by the fixture receipt.
#[cfg(target_os = "linux")]
#[test]
fn root_managed_receipt_fallback_never_repairs_freshness_and_uses_saved_cap() {
    use swarm_engine::{EngineSettingsChange, SettingCap, SettingCount};
    let f = Fixture::new();
    let mut w = managed_waiver(&f);
    seed(&f, &w);
    let mut store = RootStore::open(&f.0, true).unwrap();
    assert!(
        store
            .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["task"]
            .fresh
    );
    store.close().unwrap();
    let task = &mut w.tasks.as_mut().unwrap()[0];
    task.automatic_validation = None;
    let review = task.independent_review.as_mut().unwrap();
    review.producer = "fixture-producer".into();
    let raw=serde_json::to_vec(&serde_json::json!({"candidate_commit":review.candidate_commit,"attempt_id":review.attempt,"agent_id":review.producer,"controls":{"task":[{"id":"fixture-check","executed":true,"passed":true,"command":["fixture-not-executed"],"wall_duration_ms":17}]}})).unwrap();
    fs::write(f.0.join("managed.receipt"), &raw).unwrap();
    review.receipt_sha256 = hash(&raw);
    seed(&f, &w);
    store = RootStore::open(&f.0, true).unwrap();
    let db = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let observation = store
        .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::English)
        .unwrap();
    assert_eq!(observation.validation.state, "blocked");
    let task = &observation.validation.tasks["task"];
    assert!(!task.fresh);
    assert_eq!(task.state, "stale");
    assert_eq!(task.evidence.controls.state, "passed");
    assert_eq!(task.evidence.controls.items[0].id, "fixture-check");
    assert_eq!(task.evidence.controls.items[0].freshness, "stale");
    assert_eq!(task.evidence.controls.items[0].wall_duration_ms, Some(17));
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), db);
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    for (event, cap, refusal) in [
        ("receipt-zero", Some(SettingCount(0)), true),
        ("receipt-unlimited", None, false),
    ] {
        let doc = store.settings().unwrap();
        let mut values = doc.values;
        values.managed_receipt_max_bytes = Some(SettingCap(cap));
        let mut change = EngineSettingsChange {
            schema_version: values.schema_version(),
            event_id: event.into(),
            expected_revision: doc.revision,
            values,
            reason: "Observe saved receipt policy".into(),
            preview_token: String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store
            .apply_settings(&change, &mut SystemEnvironment)
            .unwrap();
        let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        let result = store.inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French);
        if refusal {
            assert!(result
                .map(|_| ())
                .unwrap_err()
                .to_string()
                .contains("artifact-byte-limit-exceeded"));
        } else {
            assert_eq!(
                result.unwrap().validation.tasks["task"]
                    .evidence
                    .controls
                    .items[0]
                    .id,
                "fixture-check"
            );
        }
        assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), db);
        assert_eq!(
            fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
            before
        );
    }
    store.close().unwrap();
}
/// Nonported protocols refuse before deriving stale/pending in every task state.
/// Each fault has a current simple-protocol witness and restoration under the same
/// stored settings; no public mutation or acceptance is performed.
#[cfg(target_os = "linux")]
#[test]
fn root_managed_projection_refuses_batch_fragment_in_every_state_and_recovers() {
    let f = Fixture::new();
    let w = managed_waiver(&f);
    for status in ["todo", "submitted", "accepted", "waived"] {
        for protocol in ["batch", "fragment"] {
            seed(&f, &w);
            let store = RootStore::open(&f.0, false).unwrap();
            assert!(
                store
                    .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
                    .unwrap()
                    .validation
                    .tasks["task"]
                    .fresh
            );
            store.close().unwrap();
            let mut bad = swarm_engine::decode_work(&encode_model(&w).unwrap()).unwrap();
            let task = &mut bad.tasks.as_mut().unwrap()[0];
            task.status = status.into();
            let review = task.independent_review.as_mut().unwrap();
            if protocol == "batch" {
                review.batches = Some(vec![Default::default()]);
            } else {
                review.fragment_journal = Some(Default::default());
            }
            seed(&f, &bad);
            let store = RootStore::open(&f.0, false).unwrap();
            let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
            let error = store
                .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::English)
                .map(|_| ())
                .unwrap_err();
            assert_eq!(error.code(), "command_not_ported");
            assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
            store.close().unwrap();
            seed(&f, &w);
            let store = RootStore::open(&f.0, false).unwrap();
            assert!(
                store
                    .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::English)
                    .unwrap()
                    .validation
                    .tasks["task"]
                    .fresh
            );
            store.close().unwrap();
        }
    }
}
/// Missing/mismatching business review bindings derive stale; unexpected artifact
/// IO retains its cause and refuses the whole observation. Restoration recovers
/// the real Git-backed witness, with original stored data/settings unchanged.
#[cfg(target_os = "linux")]
#[test]
fn root_managed_projection_tracks_current_review_and_preserves_io_refusal() {
    use std::error::Error;
    let f = Fixture::new();
    let w = managed_waiver(&f);
    for kind in ["missing", "state", "attempt", "contract", "candidate"] {
        seed(&f, &w);
        let store = RootStore::open(&f.0, false).unwrap();
        assert!(
            store
                .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
                .unwrap()
                .validation
                .tasks["task"]
                .fresh
        );
        store.close().unwrap();
        let mut bad = swarm_engine::decode_work(&encode_model(&w).unwrap()).unwrap();
        let task = &mut bad.tasks.as_mut().unwrap()[0];
        if kind == "missing" {
            task.independent_review = None;
        } else {
            let review = task.independent_review.as_mut().unwrap();
            match kind {
                "state" => review.state = "failed".into(),
                "attempt" => review.attempt = "other".into(),
                "contract" => review.contract = "other".into(),
                "candidate" => review.candidate_commit = "other".into(),
                _ => unreachable!(),
            }
        }
        seed(&f, &bad);
        let store = RootStore::open(&f.0, false).unwrap();
        let observation = store
            .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
            .unwrap();
        assert_eq!(observation.validation.state, "blocked");
        assert!(!observation.validation.tasks["task"].fresh);
        if kind == "missing" {
            assert_eq!(
                observation.validation.tasks["task"]
                    .evidence
                    .report_review
                    .state,
                "pending"
            );
        }
        store.close().unwrap();
    }
    seed(&f, &w);
    let store = RootStore::open(&f.0, false).unwrap();
    assert!(
        store
            .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["task"]
            .fresh
    );
    fs::remove_file(f.0.join("report.md")).unwrap();
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let error = store
        .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
        .map(|_| ())
        .unwrap_err();
    assert!(error.source().is_some());
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    fs::write(f.0.join("report.md"), b"Review report").unwrap();
    assert!(
        store
            .inspect_work_validation(&w.id, swarm_engine::WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["task"]
            .fresh
    );
    store.close().unwrap();
}
