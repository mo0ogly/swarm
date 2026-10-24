#![cfg(target_os = "linux")]
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};
use swarm_engine::{
    EngineSettingsChange, EngineSettingsDocument, EngineSettingsValues, GitReportError, RootStore,
    SettingCap, SettingCount, SystemEnvironment,
};

struct Fixture(PathBuf);
impl Fixture {
    /// Allocate a private root and bare repository; failures preserve original evidence.
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("git-report-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        let f = Self(p);
        f.git(&["init", "--bare", "repository.git"], b"");
        f
    }
    /// Run fixture setup Git without a shell or watchdog; assert success and retain stderr on failure.
    fn git(&self, args: &[&str], input: &[u8]) -> Vec<u8> {
        let mut child = Command::new("git")
            .args(args)
            .current_dir(&self.0)
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@localhost")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@localhost")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result.stdout
    }
    /// Create an actual committed report blob and return its immutable commit ID.
    fn commit(&self, raw: &[u8]) -> String {
        let blob = String::from_utf8(self.git(
            &["--git-dir=repository.git", "hash-object", "-w", "--stdin"],
            raw,
        ))
        .unwrap();
        let tree = String::from_utf8(self.git(
            &["--git-dir=repository.git", "mktree"],
            format!("100644 blob {}\treport.md\n", blob.trim()).as_bytes(),
        ))
        .unwrap();
        String::from_utf8(self.git(
            &["--git-dir=repository.git", "commit-tree", tree.trim()],
            b"Fixture report\n",
        ))
        .unwrap()
        .trim()
        .into()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Build an explicit v5 fixture journal; no settings fallback is supplied by the reader.
fn settings() -> EngineSettingsDocument {
    EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap()
}
/// Match the full blob, legacy whitespace handling, errors and a valid recovery on real Git.
#[test]
fn committed_report_exact_and_historical_bytes_refusals_and_recovery() {
    let f = Fixture::new();
    let d = settings();
    let mut raw = "\u{0085} \r\n".as_bytes().to_vec();
    raw.extend_from_slice(&vec![b'x'; 150_000]);
    raw.extend_from_slice(b"\xff interior \xf0");
    raw.extend_from_slice(" \u{3000}\n".as_bytes());
    let candidate = f.commit(&raw);
    let read = |candidate: &str, report: &str, version| {
        swarm_engine::hash_git_report(&f.0, "repository.git", candidate, report, version, &d)
    };
    let exact = read(&candidate, "report.md", 2).unwrap();
    assert_eq!(exact.sha256, format!("{:x}", Sha256::digest(&raw)));
    assert_eq!(exact.stdout_bytes, raw.len() as u64);
    assert_eq!(exact.stderr_bytes, 0);
    assert_eq!(exact.scope, "git-blob-only");
    for version in [0, 1] {
        let legacy = read(&candidate, "report.md", version).unwrap();
        let begin = "\u{0085} \r\n".len();
        let end = " \u{3000}\n".len();
        assert_eq!(
            legacy.sha256,
            format!("{:x}", Sha256::digest(&raw[begin..raw.len() - end]))
        );
        assert_ne!(legacy.sha256, exact.sha256);
    }
    for (c, p, v, reason) in [
        (&candidate[..], "absent.md", 2, "git-command-failed"),
        (
            &candidate,
            "report.md",
            3,
            "git-citation-version-unsupported",
        ),
        ("--help", "report.md", 2, "git-candidate-object-id-required"),
        (
            &candidate,
            "../report.md",
            2,
            "git-report-relative-path-required",
        ),
        (
            &candidate,
            "/report.md",
            2,
            "git-report-relative-path-required",
        ),
        (
            &candidate,
            "report.md\0extra",
            2,
            "git-report-relative-path-required",
        ),
    ] {
        assert_eq!(
            read(&candidate, "report.md", 2).unwrap().sha256,
            exact.sha256
        ); // valid control before each fault
        let error = read(c, p, v).unwrap_err();
        assert_eq!(error.reason(), reason);
        assert!(!format!("{error:?}").contains("interior"));
        assert_eq!(
            read(&candidate, "report.md", 2).unwrap().sha256,
            exact.sha256
        );
    }
    let missing = read(&"f".repeat(40), "report.md", 2).unwrap_err();
    assert_eq!(missing.reason(), "git-command-failed");
    assert!(matches!(
        missing,
        GitReportError::Command {
            timed_out: false,
            ..
        }
    ));
    if let GitReportError::Command { output, .. } = &missing {
        assert!(output.stdout().is_empty());
        assert!(!output.stderr().is_empty());
        assert!(String::from_utf8_lossy(output.stderr()).contains("fatal:"));
        assert!(!format!("{output:?}").contains("fatal:"));
    }
    let empty = f.commit(b"");
    assert_eq!(
        read(&empty, "report.md", 2).unwrap().sha256,
        format!("{:x}", Sha256::digest(b""))
    );
}
/// Old bytes and replay survive explicit v5 adoption; zero/null are distinct effective policies.
#[test]
fn v4_history_and_replay_require_explicit_git_adoption() {
    let raw = include_bytes!("fixtures/legacy-v4-document.json");
    let old = EngineSettingsDocument::decode(raw).unwrap();
    assert_eq!(swarm_engine::encode_model(&old).unwrap(), raw);
    assert_eq!(old.schema_version, 4);
    let replay: EngineSettingsChange =
        serde_json::from_slice(include_bytes!("fixtures/legacy-v4-request.json")).unwrap();
    assert!(old.replay(&replay).unwrap());
    assert_eq!(
        old.values.git_command_timeout().unwrap_err().reason(),
        "settings-git-policy-not-configured"
    );
    let mut change = EngineSettingsChange {
        schema_version: 5,
        event_id: "adopt-git".into(),
        expected_revision: old.revision,
        values: old.values.clone(),
        reason: "Explicit Git timeout adoption".into(),
        preview_token: String::new(),
    };
    assert!(old.preview(&change).is_err());
    change.values.git_command_timeout_seconds = Some(SettingCap(Some(SettingCount(90))));
    change.preview_token = old.preview(&change).unwrap().preview_token;
    let new = old
        .apply(&change, "operator".into(), "2026-10-18T00:00:00Z".into())
        .unwrap()
        .0;
    assert_eq!(&new.history[..old.history.len()], &old.history);
    assert!(new.replay(&replay).unwrap());
    assert!(new.replay(&change).unwrap());
    assert_eq!(
        new.quality_policy_snapshot().unwrap().digest,
        old.quality_policy_snapshot().unwrap().digest
    );
    let mut values = new.values.clone();
    values.git_command_timeout_seconds = Some(SettingCap(None));
    assert_eq!(values.git_command_timeout().unwrap(), None);
    values.git_command_timeout_seconds = Some(SettingCap(Some(SettingCount(0))));
    assert_eq!(
        values.git_command_timeout().unwrap(),
        Some(std::time::Duration::ZERO)
    );
    values.git_command_timeout_seconds = Some(SettingCap(Some(SettingCount(-1))));
    assert!(values.validate().is_err());
    values = new.values.clone();
    values.project_context_max_bytes = None;
    assert!(values.validate().is_err());
    assert_eq!(swarm_engine::encode_model(&old).unwrap(), raw);
}
/// A saved Root policy actually changes the Git process outcome and survives reopen.
#[test]
fn persisted_root_deadline_controls_actual_git_and_recovers_without_history_reset() {
    let f = Fixture::new();
    let candidate = f.commit(b"report\n");
    let store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let first = store
        .inspect_git_report("repository.git", &candidate, "report.md", 2)
        .unwrap();
    assert_eq!(first.settings_revision, SettingCount(1));
    store.close().unwrap();
    for (index, value) in [Some(0), None, Some(90)].into_iter().enumerate() {
        let store = RootStore::open(&f.0, true).unwrap();
        let doc = store.settings().unwrap();
        assert_eq!(
            store
                .inspect_git_report("repository.git", &candidate, "report.md", 2)
                .is_ok(),
            index != 1
        );
        let mut values = doc.values.clone();
        values.git_command_timeout_seconds = Some(SettingCap(value.map(SettingCount)));
        let mut change = EngineSettingsChange {
            schema_version: doc.schema_version,
            event_id: format!("git-policy-{index}"),
            expected_revision: doc.revision,
            values,
            reason: "Actual persisted Git deadline consumer".into(),
            preview_token: String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store
            .apply_settings(&change, &mut SystemEnvironment)
            .unwrap();
        store.close().unwrap();
        let store = RootStore::open(&f.0, false).unwrap();
        let observed = store.settings().unwrap();
        assert_eq!(observed.history.len(), doc.history.len() + 1);
        assert_eq!(&observed.history[..doc.history.len()], &doc.history);
        let saved = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        let result = store.inspect_git_report("repository.git", &candidate, "report.md", 2);
        if value == Some(0) {
            assert_eq!(
                result.unwrap_err().to_string(),
                "git-report: git-command-timeout"
            );
        } else {
            assert_eq!(result.unwrap().sha256, first.sha256);
        }
        assert_eq!(
            fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
            saved
        );
        store.close().unwrap();
    }
}

/// Inject only an owned fixture Work while retaining real SQL triggers, foreign keys and UDF.
fn seed_work(f: &Fixture, w: &swarm_engine::Work) {
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    assert_eq!(
        db.execute(
            "UPDATE works SET body=?1 WHERE id=?2 AND revision=?3",
            rusqlite::params![
                String::from_utf8(swarm_engine::encode_model(w).unwrap()).unwrap(),
                w.id,
                w.revision
            ]
        )
        .unwrap(),
        1
    );
    db.close().unwrap();
}
/// Return authoritative fixture row bytes and event count for no-mutation assertions.
fn persisted_work(f: &Fixture, id: &str) -> (String, i64) {
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    let body = db
        .query_row("SELECT body FROM works WHERE id=?1", [id], |r| r.get(0))
        .unwrap();
    let events = db
        .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
        .unwrap();
    db.close().unwrap();
    (body, events)
}
/// Observe real stored review/Git bindings, each invalid variant, no mutation and recovery.
#[test]
fn stored_review_context_and_git_are_current_data_without_acceptance_authority() {
    use swarm_engine::{AutomaticValidation, IndependentReview, ManagedRepository, PlanningState};
    let f = Fixture::new();
    let candidate = f.commit(b"Review report\n");
    let mut store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let request = |event: &str, rev: i64, fields: serde_json::Value| {
        let mut value = fields;
        value["schema_version"] = 1.into();
        value["event_id"] = event.into();
        value["expected_revision"] = rev.into();
        serde_json::to_vec(&value).unwrap()
    };
    let mut w=store.execute_external("","work.create",&request("create",0,serde_json::json!({"title":"Managed Git fixture","objective":"Current bindings","scope":"Owned fixture","criteria":["Proof"]})),&mut SystemEnvironment).unwrap().result.work;
    w=store.execute_external(&w.id,"task.add",&request("task",w.revision,serde_json::json!({"id":"task","title":"Task","deliverable":"report.md","criteria":["Proof"]})),&mut SystemEnvironment).unwrap().result.work;
    store.close().unwrap();
    fs::write(f.0.join("receipt.json"), b"Fixture receipt").unwrap();
    fs::write(f.0.join("report.md"), b"Review report\n").unwrap();
    let context=serde_json::to_vec(&serde_json::json!({"citation_contract_version":2,"candidate_commit":candidate,"tasks":[],"receipt":null})).unwrap();
    fs::write(f.0.join("context.json"), &context).unwrap();
    let hash = |raw: &[u8]| format!("{:x}", Sha256::digest(raw));
    w.planning = Some(Box::new(PlanningState {
        repository: Some(Box::new(ManagedRepository {
            candidate_commit: candidate.clone(),
            storage: ".".into(),
            ..Default::default()
        })),
        ..Default::default()
    }));
    let t = &mut w.tasks.as_mut().unwrap()[0];
    t.automatic_validation = Some(Box::new(AutomaticValidation {
        candidate_sha: candidate.clone(),
        ..Default::default()
    }));
    t.independent_review = Some(Box::new(IndependentReview {
        id: "review".into(),
        state: "passed".into(),
        candidate_commit: candidate.clone(),
        receipt: "receipt.json".into(),
        receipt_sha256: hash(b"Fixture receipt"),
        context: "context.json".into(),
        context_sha256: hash(&context),
        report: "report.md".into(),
        sha256: hash(b"Review report\n"),
        git_report: "report.md".into(),
        ..Default::default()
    }));
    seed_work(&f, &w);
    let inspect = || {
        let store = RootStore::open(&f.0, false).unwrap();
        let result = store.inspect_managed_review_git(&w.id, "task");
        store.close().unwrap();
        result
    };
    let observed = inspect().unwrap();
    assert_eq!(observed.work_revision, w.revision);
    assert_eq!(observed.settings_revision, SettingCount(1));
    assert_eq!(observed.observation.git.sha256, hash(b"Review report\n"));
    assert!(!observed.observation.protocol_verified && !observed.observation.authority_verified);
    for fault in [
        "file-tamper",
        "git-path",
        "git-content",
        "context-json",
        "context-candidate",
        "version",
        "baseline",
        "batch",
        "fragment",
        "candidate",
    ] {
        fs::write(f.0.join("context.json"), &context).unwrap();
        fs::write(f.0.join("report.md"), b"Review report\n").unwrap();
        seed_work(&f, &w);
        assert!(inspect().is_ok());
        let mut bad = swarm_engine::decode_work(&swarm_engine::encode_model(&w).unwrap()).unwrap();
        let review = bad.tasks.as_mut().unwrap()[0]
            .independent_review
            .as_mut()
            .unwrap();
        let expected = match fault {
            "file-tamper" => {
                fs::write(f.0.join("context.json"), b"{}").unwrap();
                "managed-review-file-stale"
            }
            "git-path" => {
                review.git_report = "absent.md".into();
                "git-command-failed"
            }
            "git-content" => {
                fs::write(f.0.join("report.md"), b"Changed report\n").unwrap();
                review.sha256 = hash(b"Changed report\n");
                "managed-review-git-report-stale"
            }
            "context-json" => {
                let raw = serde_json::to_vec(
                    &serde_json::json!({"candidate_commit":candidate,"unexpected":1}),
                )
                .unwrap();
                fs::write(f.0.join("context.json"), &raw).unwrap();
                review.context_sha256 = hash(&raw);
                "managed-review-context-format"
            }
            "context-candidate" => {
                let raw = br#"{"candidate_commit":"different"}"#;
                fs::write(f.0.join("context.json"), raw).unwrap();
                review.context_sha256 = hash(raw);
                "managed-review-context-candidate-stale"
            }
            "version" => {
                let raw=serde_json::to_vec(&serde_json::json!({"candidate_commit":candidate,"citation_contract_version":3})).unwrap();
                fs::write(f.0.join("context.json"), &raw).unwrap();
                review.context_sha256 = hash(&raw);
                "git-citation-version-unsupported"
            }
            "baseline" => {
                let raw=serde_json::to_vec(&serde_json::json!({"candidate_commit":candidate,"accepted_baseline":{"candidate_commit":"old","accepted_reviews":[]}})).unwrap();
                fs::write(f.0.join("context.json"), &raw).unwrap();
                review.context_sha256 = hash(&raw);
                "managed-review-baseline-incoherent"
            }
            "batch" => {
                review.batches = Some(vec![Default::default()]);
                "managed-review-batch-protocol-not-ported"
            }
            "fragment" => {
                review.fragment_journal = Some(Box::default());
                "managed-review-fragment-protocol-not-ported"
            }
            "candidate" => {
                bad.planning
                    .as_mut()
                    .unwrap()
                    .repository
                    .as_mut()
                    .unwrap()
                    .candidate_commit = "f".repeat(40);
                "managed-review-candidate-stale"
            }
            _ => unreachable!(),
        };
        seed_work(&f, &bad);
        let before = persisted_work(&f, &w.id);
        let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        let error = inspect().unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("review-evidence-invalid: {expected}"),
            "{fault}"
        );
        assert_eq!(persisted_work(&f, &w.id), before);
        assert_eq!(
            fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        fs::write(f.0.join("context.json"), &context).unwrap();
        fs::write(f.0.join("report.md"), b"Review report\n").unwrap();
        seed_work(&f, &w);
        assert!(inspect().is_ok());
    }
    let mut store = RootStore::open(&f.0, true).unwrap();
    let before = persisted_work(&f, &w.id);
    let result = store
        .execute_external(
            &w.id,
            "task.update",
            &request(
                "accept",
                w.revision,
                serde_json::json!({"id":"task","status":"accepted"}),
            ),
            &mut SystemEnvironment,
        )
        .map(|_| ());
    assert_eq!(result.unwrap_err().code(), "command_not_ported");
    assert_eq!(persisted_work(&f, &w.id), before);
    store.close().unwrap();
}
