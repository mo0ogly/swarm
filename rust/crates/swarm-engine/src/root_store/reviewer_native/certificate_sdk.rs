//! Wire compatibility and coexistence on genuine owned native review originals.
use super::{publication, store};
use crate::root_store::RootStore;
use std::{
    fs,
    path::{Path, PathBuf},
};
/// Locate the single actual candidate from this single-job fixture after a
/// refused SQL commit. Every matching file is parsed; none is silently skipped.
/// This is a test-only finite fixture precondition, not a product scan or cap.
/// The file alone creates no job result, execution grant or acceptance.
pub(super) fn one_candidate(root: &RootStore, id: &str) -> PathBuf {
    let mut found = Vec::new();
    for item in fs::read_dir(root.root.join(".swarm")).unwrap() {
        let path = item.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if name.starts_with("native-review-public-") && name.ends_with(".json") {
            let data: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            if data["review"]["id"] == id {
                found.push(path);
            }
        }
    }
    assert_eq!(
        found.len(),
        1,
        "single-job fixture must retain exactly its one candidate here"
    );
    found.pop().unwrap()
}
/// Publish a genuine legacy1 result or retain its real SQL-rollback candidate,
/// then apply the default2 result without replacing the first file or rerunning
/// the provider. The original SDK has already exited and is verified departed.
/// Private row/receipt/Work provenance is real; no positive receipt is seeded.
fn verify(upgrade: bool) {
    let input: serde_json::Value = serde_json::from_slice(
        &fs::read(std::env::var_os("SWARM_OWNED_REVIEW_CLAIM_FIXTURE").unwrap()).unwrap(),
    )
    .unwrap();
    let path = Path::new(input["root"].as_str().unwrap());
    assert_eq!(
        fs::read_to_string(path.join(".swarm/owned-review-claim-fixture")).unwrap(),
        input["token"].as_str().unwrap()
    );
    let root = RootStore::open(path, true).unwrap();
    let before = root.read_work(input["work"].as_str().unwrap()).unwrap();
    let task = before
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == input["request"]["task_id"].as_str().unwrap())
        .unwrap();
    let review = task.independent_review.as_ref().unwrap();
    let job = store::job(&root.db, &review.id).unwrap().unwrap();
    let invoked = fs::read(path.join("reviewer-invoked.json")).unwrap();
    let output = if upgrade {
        root.db.execute_batch("CREATE TEMP TRIGGER owned_cert_rollback BEFORE UPDATE ON native_review_jobs BEGIN SELECT RAISE(ABORT,'owned-legacy-candidate-rollback'); END;").unwrap();
        assert!(publication::reconcile_departed_legacy_fixture(&root, &job.id).is_err());
        assert_eq!(root.read_work(&before.id).unwrap(), before);
        assert!(store::job(&root.db, &job.id).unwrap().as_ref() == Some(&job));
        root.db
            .execute_batch("DROP TRIGGER owned_cert_rollback")
            .unwrap();
        let legacy = one_candidate(&root, &job.id);
        let original = fs::read(&legacy).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&original).unwrap()["schema_version"],
            1
        );
        let out = publication::reconcile_departed(&root, &job.id).unwrap();
        assert_eq!(fs::read(&legacy).unwrap(), original);
        assert_ne!(
            out.work.tasks.as_ref().unwrap()[0]
                .independent_review
                .as_ref()
                .unwrap()
                .receipt,
            format!(".swarm/{}", legacy.file_name().unwrap().to_str().unwrap())
        );
        out
    } else {
        publication::reconcile_departed_legacy_fixture(&root, &job.id).unwrap()
    };
    assert!(output.review_recorded && output.verdict_applicable && !output.task_accepted);
    assert_eq!(output.work.revision, before.revision + 1);
    assert_eq!(
        output
            .work
            .planning
            .as_ref()
            .unwrap()
            .reviewer
            .as_ref()
            .unwrap()
            .calls,
        before
            .planning
            .as_ref()
            .unwrap()
            .reviewer
            .as_ref()
            .unwrap()
            .calls
    );
    let current = output
        .work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == task.id)
        .unwrap();
    let review = current.independent_review.as_ref().unwrap();
    let raw = fs::read(path.join(&review.receipt)).unwrap();
    let certificate: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(certificate["schema_version"], if upgrade { 2 } else { 1 });
    assert_eq!(
        crate::provider_configuration::configuration_digest(&raw),
        review.receipt_sha256
    );
    if upgrade {
        assert!(certificate["review"].get("receipt").is_none());
        assert_eq!(
            review.receipt,
            format!(
                ".swarm/native-review-public-v2-{}.json",
                review.receipt_sha256
            )
        );
    }
    assert!(
        publication::reconcile_departed(&root, &job.id)
            .unwrap()
            .replayed
    );
    let scope = output
        .work
        .planning
        .as_ref()
        .unwrap()
        .scopes
        .iter()
        .flatten()
        .find(|s| s.id == task.scope_id)
        .unwrap();
    let context = crate::planning_scope_context::observe(
        path,
        &output.work,
        scope,
        &root.read_settings().unwrap(),
    )
    .unwrap();
    assert!(String::from_utf8_lossy(&context.bytes).contains(&job.id));
    assert_eq!(
        fs::read(path.join("reviewer-invoked.json")).unwrap(),
        invoked
    );
    assert_eq!(root.read_work(&before.id).unwrap(), output.work);
    fs::write(path.join("owned-review-certificate-compat.json"),serde_json::to_vec(&serde_json::json!({"legacy_wire":!upgrade,"upgraded_after_rollback":upgrade,"schema_version":if upgrade{2}else{1},"old_candidate_preserved":upgrade,"context_prepared":true,"provider_execs":1,"recovery_execs":0,"replay_verified":true,"task_accepted":false,"model_received":false})).unwrap()).unwrap();
    fs::write(path.join("owned-review-public-certificate.json"), raw).unwrap();
    root.close().unwrap();
    println!("PUBLIC_CERTIFICATE_COMPAT_CHECKED");
}
/// Real protected publication using only the historical wire encoder.
#[test]
#[ignore = "owned completed original fixture only"]
fn public_certificate_legacy_owned_fixture() {
    verify(false)
}
/// A rolled-back legacy candidate must coexist with its committed v2 successor.
#[test]
#[ignore = "owned completed original fixture only"]
fn public_certificate_upgrade_owned_fixture() {
    verify(true)
}
