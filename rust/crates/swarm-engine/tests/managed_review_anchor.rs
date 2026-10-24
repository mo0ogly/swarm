use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::PathBuf};
use swarm_engine::{
    inspect_managed_review_anchor, AutomaticValidation, IndependentReview, ManagedRepository,
    PlanningState, Task, Work,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("managed-anchor-").unwrap());
        fs::create_dir(&path).unwrap();
        for (name, data) in files() {
            fs::write(path.join(name), data).unwrap();
        }
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
fn files() -> [(&'static str, &'static [u8]); 3] {
    [
        ("receipt.json", b"{\"candidate_commit\":\"candidate\"}"),
        ("context.json", b"{\"candidate_commit\":\"candidate\"}"),
        ("report.md", b"Review report\n"),
    ]
}
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn work() -> Work {
    Work {
        planning: Some(Box::new(PlanningState {
            repository: Some(Box::new(ManagedRepository {
                candidate_commit: "candidate".into(),
                ..Default::default()
            })),
            ..Default::default()
        })),
        tasks: Some(vec![Task {
            id: "task".into(),
            automatic_validation: Some(Box::new(AutomaticValidation {
                candidate_sha: "candidate".into(),
                ..Default::default()
            })),
            independent_review: Some(Box::new(IndependentReview {
                id: "review".into(),
                candidate_commit: "candidate".into(),
                receipt: "receipt.json".into(),
                receipt_sha256: hash(files()[0].1),
                context: "context.json".into(),
                context_sha256: hash(files()[1].1),
                report: "report.md".into(),
                sha256: hash(files()[2].1),
                ..Default::default()
            })),
            ..Default::default()
        }]),
        ..Default::default()
    }
}
/// Verify file/candidate consistency, refusal without writes and recovery.
#[test]
fn managed_anchor_checks_candidate_and_all_three_files_with_recovery() {
    let f = Fixture::new();
    let w = work();
    let observed = inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
    assert_eq!(observed.scope, "candidate-and-file-bindings-only");
    assert!(
        !observed.git_report_verified
            && !observed.protocol_verified
            && !observed.authority_verified
    );
    assert_eq!(observed.candidate_commit, "candidate");
    for (name, data) in files() {
        inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
        fs::write(f.0.join(name), b"Changed").unwrap();
        assert_eq!(
            inspect_managed_review_anchor(&f.0, &w, "task")
                .unwrap_err()
                .reason(),
            "managed-review-file-stale"
        );
        assert_eq!(fs::read(f.0.join(name)).unwrap(), b"Changed");
        fs::write(f.0.join(name), data).unwrap();
        inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
        fs::remove_file(f.0.join(name)).unwrap();
        assert!(inspect_managed_review_anchor(&f.0, &w, "task")
            .unwrap_err()
            .source()
            .is_some());
        fs::write(f.0.join(name), data).unwrap();
        inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
    }
}
/// Verify supplied bindings cannot omit or ambiguously select a task.
#[test]
fn managed_anchor_rejects_missing_stale_and_ambiguous_bindings() {
    let f = Fixture::new();
    let w = work();
    for fault in [
        "planning",
        "repository",
        "review",
        "candidate",
        "repository-candidate",
        "automatic",
        "automatic-candidate",
        "receipt-digest",
        "context-digest",
        "report-digest",
        "ambiguous",
    ] {
        inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
        let mut changed = w.clone();
        let t = &mut changed.tasks.as_mut().unwrap()[0];
        match fault {
            "planning" => changed.planning = None,
            "repository" => changed.planning.as_mut().unwrap().repository = None,
            "review" => t.independent_review = None,
            "candidate" => t
                .independent_review
                .as_mut()
                .unwrap()
                .candidate_commit
                .clear(),
            "repository-candidate" => {
                changed
                    .planning
                    .as_mut()
                    .unwrap()
                    .repository
                    .as_mut()
                    .unwrap()
                    .candidate_commit = "other".into()
            }
            "automatic" => t.automatic_validation = None,
            "automatic-candidate" => {
                t.automatic_validation.as_mut().unwrap().candidate_sha = "other".into()
            }
            "receipt-digest" => t
                .independent_review
                .as_mut()
                .unwrap()
                .receipt_sha256
                .clear(),
            "context-digest" => t
                .independent_review
                .as_mut()
                .unwrap()
                .context_sha256
                .clear(),
            "report-digest" => t.independent_review.as_mut().unwrap().sha256.clear(),
            "ambiguous" => {
                let duplicate = t.clone();
                changed.tasks.as_mut().unwrap().push(duplicate);
            }
            _ => unreachable!(),
        }
        assert!(
            inspect_managed_review_anchor(&f.0, &changed, "task").is_err(),
            "{fault}"
        );
        inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
    }
    assert!(inspect_managed_review_anchor(&f.0, &w, "missing").is_err());
}
#[cfg(unix)]
/// Verify containment independently of content hashes and refuse FIFO/directories.
#[test]
fn managed_anchor_rejects_matching_hash_outside_root_and_nonregular_files() {
    let f = Fixture::new();
    let w = work();
    let outside = Fixture::new();
    inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
    std::os::unix::fs::symlink(outside.0.join("report.md"), f.0.join("outside.md")).unwrap();
    let mut changed = w.clone();
    changed.tasks.as_mut().unwrap()[0]
        .independent_review
        .as_mut()
        .unwrap()
        .report = "outside.md".into();
    assert!(inspect_managed_review_anchor(&f.0, &changed, "task").is_err());
    inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
    fs::remove_file(f.0.join("report.md")).unwrap();
    rustix::fs::mknodat(
        rustix::fs::CWD,
        f.0.join("report.md"),
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        0,
    )
    .unwrap();
    assert!(inspect_managed_review_anchor(&f.0, &w, "task").is_err());
    fs::remove_file(f.0.join("report.md")).unwrap();
    fs::create_dir(f.0.join("report.md")).unwrap();
    assert!(inspect_managed_review_anchor(&f.0, &w, "task").is_err());
    fs::remove_dir(f.0.join("report.md")).unwrap();
    fs::write(f.0.join("report.md"), files()[2].1).unwrap();
    inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
}
/// Verify every coincident-path digest obligation against a valid shared control.
#[test]
fn managed_anchor_never_collapses_aliased_file_bindings() {
    let f = Fixture::new();
    let mut w = work();
    let r = w.tasks.as_mut().unwrap()[0]
        .independent_review
        .as_mut()
        .unwrap();
    r.context = r.receipt.clone();
    r.report = r.receipt.clone();
    r.context_sha256 = r.receipt_sha256.clone();
    r.sha256 = r.receipt_sha256.clone();
    for field in ["receipt", "context", "report"] {
        for bad in [String::new(), hash(b"Different")] {
            inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
            let mut changed = w.clone();
            let r = changed.tasks.as_mut().unwrap()[0]
                .independent_review
                .as_mut()
                .unwrap();
            match field {
                "receipt" => r.receipt_sha256 = bad,
                "context" => r.context_sha256 = bad,
                "report" => r.sha256 = bad,
                _ => unreachable!(),
            }
            assert_eq!(
                inspect_managed_review_anchor(&f.0, &changed, "task")
                    .unwrap_err()
                    .reason(),
                "managed-review-file-stale"
            );
            inspect_managed_review_anchor(&f.0, &w, "task").unwrap();
        }
    }
}
