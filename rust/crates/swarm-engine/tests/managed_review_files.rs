use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{
    check_managed_review_files, EngineSettingsChange, EngineSettingsDocument, EngineSettingsValues,
    IndependentReview, SettingCap, SettingCount,
};
struct Fixture(PathBuf);
impl Fixture {
    /// Allocate a private isolated file root; no live mission or provider is used.
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("baseline-files-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
    /// Create literal contract files with real digests, optionally referencing prior reviews.
    fn review(
        &self,
        name: &str,
        candidate: &str,
        previous: &str,
        children: Vec<IndependentReview>,
        version: i64,
    ) -> IndependentReview {
        let mut context = serde_json::json!({"candidate_commit":candidate,"previous_candidate":previous,"receipt":null,"tasks":[]});
        if !children.is_empty() {
            context["accepted_baseline"] = serde_json::json!({"version":version,"candidate_commit":previous,"accepted_reviews":children});
        }
        let raw = serde_json::to_vec(&context).unwrap();
        let receipt = format!("{name}.receipt");
        let path = format!("{name}.context");
        let report = format!("{name}.report");
        fs::write(self.0.join(&receipt), b"Receipt").unwrap();
        fs::write(self.0.join(&path), &raw).unwrap();
        fs::write(self.0.join(&report), b"Report").unwrap();
        IndependentReview {
            id: name.into(),
            state: "passed".into(),
            candidate_commit: candidate.into(),
            receipt,
            receipt_sha256: hash(b"Receipt"),
            context: path,
            context_sha256: hash(&raw),
            report,
            sha256: hash(b"Report"),
            ..Default::default()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Build a valid explicit policy document instead of mutating a hashed settings snapshot.
fn settings(limit: Option<i64>) -> EngineSettingsDocument {
    let mut v = EngineSettingsValues::installation_defaults().unwrap();
    v.managed_review_max_baseline_depth = Some(SettingCap(limit.map(SettingCount)));
    EngineSettingsDocument::initial(v, "fixture".into(), "2026-10-18T00:00:00Z".into()).unwrap()
}

/// Historical versions, each review's bindings, unsupported protocols and recovery remain enforced.
#[test]
fn historical_file_proofs_keep_every_shared_context_obligation() {
    let f = Fixture::new();
    let d = settings(Some(128));
    let leaf = f.review("leaf", "old", "", vec![], 0);
    for version in [0, 1, 2] {
        let root = f.review(
            &format!("version-{version}"),
            "current",
            "old",
            vec![leaf.clone()],
            version,
        );
        let r = check_managed_review_files(&f.0, &root, &d, false).unwrap();
        assert_eq!(r.reviews_checked, 2);
        assert_eq!(r.max_depth, 1);
        assert_eq!(
            r.context
                .accepted_baseline
                .as_ref()
                .unwrap()
                .accepted_reviews
                .as_ref()
                .unwrap()
                .len(),
            1
        );
    }
    for fault in [
        "receipt-empty",
        "receipt-wrong",
        "report-empty",
        "report-wrong",
        "batch",
        "fragment",
        "state",
        "candidate",
    ] {
        let root = f.review(
            "control",
            "current",
            "old",
            vec![leaf.clone(), leaf.clone()],
            2,
        );
        assert_eq!(
            check_managed_review_files(&f.0, &root, &d, false)
                .unwrap()
                .reviews_checked,
            3
        );
        let mut bad = leaf.clone();
        let expected = match fault {
            "receipt-empty" => {
                bad.receipt_sha256.clear();
                "managed-review-file-stale"
            }
            "receipt-wrong" => {
                bad.receipt_sha256 = "wrong".into();
                "managed-review-file-stale"
            }
            "report-empty" => {
                bad.sha256.clear();
                "managed-review-file-stale"
            }
            "report-wrong" => {
                bad.sha256 = "wrong".into();
                "managed-review-file-stale"
            }
            "batch" => {
                bad.batches = Some(vec![Default::default()]);
                "managed-review-batch-protocol-not-ported"
            }
            "fragment" => {
                bad.fragment_journal = Some(Box::default());
                "managed-review-fragment-protocol-not-ported"
            }
            "state" => {
                bad.state = "failed".into();
                "managed-review-baseline-review-invalid"
            }
            "candidate" => {
                bad.candidate_commit = "other".into();
                "managed-review-baseline-review-invalid"
            }
            _ => unreachable!(),
        };
        let root = f.review("fault", "current", "old", vec![leaf.clone(), bad], 2);
        let before = fs::read(f.0.join(&root.context)).unwrap();
        assert_eq!(
            check_managed_review_files(&f.0, &root, &d, false)
                .unwrap_err()
                .reason(),
            expected
        );
        assert_eq!(fs::read(f.0.join(&root.context)).unwrap(), before);
        assert_eq!(
            check_managed_review_files(
                &f.0,
                &f.review(
                    "recovery",
                    "current",
                    "old",
                    vec![leaf.clone(), leaf.clone()],
                    2
                ),
                &d,
                false
            )
            .unwrap()
            .reviews_checked,
            3
        );
    }
    let unsupported = f.review("unsupported", "current", "old", vec![leaf], 3);
    assert_eq!(
        check_managed_review_files(&f.0, &unsupported, &d, false)
            .unwrap_err()
            .reason(),
        "managed-review-baseline-version-unsupported"
    );
}

/// Boundaries are persisted operator policy, not a hidden Rust recursion or test watchdog.
#[test]
fn baseline_depth_zero_null_128_129_and_shared_dag_follow_actual_path_depth() {
    let f = Fixture::new();
    let mut node = f.review("chain-0", "candidate-0", "", vec![], 2);
    assert_eq!(
        check_managed_review_files(&f.0, &node, &settings(Some(0)), false)
            .unwrap()
            .max_depth,
        0
    );
    for i in 1..=129 {
        let previous = node.candidate_commit.clone();
        node = f.review(
            &format!("chain-{i}"),
            &format!("candidate-{i}"),
            &previous,
            vec![node],
            2,
        );
        if i == 128 {
            assert_eq!(
                check_managed_review_files(&f.0, &node, &settings(Some(128)), false)
                    .unwrap()
                    .max_depth,
                128
            );
        }
    }
    assert_eq!(
        check_managed_review_files(
            &f.0,
            &f.review("witness", "valid", "", vec![], 2),
            &settings(Some(128)),
            false
        )
        .unwrap()
        .max_depth,
        0
    );
    assert_eq!(
        check_managed_review_files(&f.0, &node, &settings(Some(128)), false)
            .unwrap_err()
            .reason(),
        "managed-review-baseline-depth-exceeded"
    );
    assert_eq!(
        check_managed_review_files(&f.0, &node, &settings(None), false)
            .unwrap()
            .max_depth,
        129
    );
    let leaf = f.review("dag-leaf", "Z", "", vec![], 2);
    let shared = f.review("dag-shared", "X", "Z", vec![leaf], 2);
    let bridge = f.review("dag-bridge", "Y", "X", vec![shared.clone()], 2);
    let head = f.review("dag-head", "X", "Y", vec![bridge], 2);
    let control = f.review("dag-control", "ROOT", "X", vec![shared.clone()], 2);
    assert_eq!(
        check_managed_review_files(&f.0, &control, &settings(Some(3)), false)
            .unwrap()
            .max_depth,
        2
    );
    let dag = f.review("dag-root", "ROOT", "X", vec![shared, head], 2);
    assert_eq!(
        check_managed_review_files(&f.0, &dag, &settings(Some(3)), false)
            .unwrap_err()
            .reason(),
        "managed-review-baseline-depth-exceeded"
    );
    assert!(check_managed_review_files(&f.0, &dag, &settings(None), false).is_ok());
}

/// Real v5 bytes/replay remain exact; only explicit v6 adoption enables baseline policy.
#[test]
fn v5_journal_preserves_exact_history_and_requires_explicit_baseline_adoption() {
    let raw = include_bytes!("fixtures/legacy-v5-document.json");
    let old = EngineSettingsDocument::decode(raw).unwrap();
    assert_eq!(swarm_engine::encode_model(&old).unwrap(), raw);
    assert_eq!(old.schema_version, 5);
    let replay: EngineSettingsChange =
        serde_json::from_slice(include_bytes!("fixtures/legacy-v5-request.json")).unwrap();
    assert!(old.replay(&replay).unwrap());
    assert_eq!(
        old.values
            .managed_review_baseline_depth()
            .unwrap_err()
            .reason(),
        "settings-review-baseline-policy-not-configured"
    );
    let mut change = EngineSettingsChange {
        schema_version: 6,
        event_id: "adopt-baselines".into(),
        expected_revision: old.revision,
        values: old.values.clone(),
        reason: "Explicit baseline depth policy".into(),
        preview_token: String::new(),
    };
    assert!(old.preview(&change).is_err());
    change.values.managed_review_max_baseline_depth = Some(SettingCap(Some(SettingCount(128))));
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
    let f = Fixture::new();
    let leaf = f.review("leaf", "old", "", vec![], 2);
    assert!(check_managed_review_files(&f.0, &leaf, &old, false).is_ok());
    let root = f.review("root", "current", "old", vec![leaf], 2);
    assert_eq!(
        check_managed_review_files(&f.0, &root, &old, false)
            .unwrap_err()
            .reason(),
        "settings-review-baseline-policy-not-configured"
    );
    assert!(check_managed_review_files(&f.0, &root, &new, false).is_ok());
    assert_eq!(swarm_engine::encode_model(&old).unwrap(), raw);
}
