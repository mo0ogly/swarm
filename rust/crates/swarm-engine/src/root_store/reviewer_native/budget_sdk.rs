//! Public budget change after one original, never repricing history or a new grant.
use super::{publication, store};
use crate::root_store::RootStore;
use std::{fs, path::Path};
/// One actual successful original precedes public budget adoption. Its old
/// estimate remains exact while future-call reserve changes. The old start guard
/// must still refuse, but result publication/non-verdict must not need a new
/// starting budget. No paid model, owner adoption, acceptance or settlement.
#[test]
#[ignore = "owned completed original fixture only"]
fn reviewer_budget_result_owned_fixture() {
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
    let work = input["work"].as_str().unwrap();
    let before = root.read_work(work).unwrap();
    let task = before
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == input["request"]["task_id"].as_str().unwrap())
        .unwrap()
        .clone();
    let review = task.independent_review.as_ref().unwrap();
    let job = store::job(&root.db, &review.id).unwrap().unwrap();
    let invoked = fs::read(path.join("reviewer-invoked.json")).unwrap();
    publication::inspect_departed(&root, &job.id).unwrap();
    let finance: (f64, String) = root
        .db
        .query_row(
            "SELECT estimate,state FROM planning_calls WHERE id=?",
            [&job.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(finance.1, "estimated");
    // Explicit fixture amounts, never a runtime default/ceiling or altered reservation.
    let reserve = finance.0 + 1.0;
    let limit = reserve * 100.0;
    assert!(reserve > finance.0 && limit.is_finite());
    super::stale_sdk::cli(
        &root,
        "budget-apply",
        &["budget", "apply", work],
        Some(
            serde_json::json!({"schema_version":1,"event_id":"owned-review-result-budget-change","expected_revision":before.revision,"budget":{"limit_usd":limit,"reserve_per_launch_usd":reserve,"estimate_source":"Explicit owned result/start distinction","reference_date":"2026-10-22"}}),
        ),
    );
    let before = root.read_work(work).unwrap();
    let tx = root.db.unchecked_transaction().unwrap();
    let start = crate::financial_budget::check_reserved_planning_start_budget(
        &tx, work, "reviewer", &job.id,
    );
    assert!(start.is_err());
    tx.rollback().unwrap();
    // Each corrupt-reservation witness starts from the real intact row and
    // rolls back without ever publishing a fabricated usage/receipt.
    let check = |tx: &rusqlite::Connection, w: &str, scope: &str| {
        crate::financial_budget::check_reserved_planning_result_budget(tx, w, scope, &job.id)
    };
    let tx = root.db.unchecked_transaction().unwrap();
    check(&tx, work, "reviewer").unwrap();
    assert!(check(&tx, "foreign-owned-fixture", "reviewer").is_err());
    assert!(check(&tx, work, "foreign-scope").is_err());
    tx.rollback().unwrap();
    assert!(check(&root.db, work, "reviewer").is_err());
    let tx = root.db.unchecked_transaction().unwrap();
    check(&tx, work, "reviewer").unwrap();
    tx.execute("DELETE FROM planning_calls WHERE id=?", [&job.id])
        .unwrap();
    assert!(check(&tx, work, "reviewer").is_err());
    tx.rollback().unwrap();
    let tx = root.db.unchecked_transaction().unwrap();
    check(&tx, work, "reviewer").unwrap();
    tx.execute(
        "UPDATE planning_calls SET usage=X'00' WHERE id=?",
        [&job.id],
    )
    .unwrap();
    assert!(check(&tx, work, "reviewer").is_err());
    tx.rollback().unwrap();
    let tx = root.db.unchecked_transaction().unwrap();
    check(&tx, work, "reviewer").unwrap();
    tx.rollback().unwrap();
    assert_eq!(root.read_work(work).unwrap(), before);
    let variant = std::env::var("SWARM_OWNED_REVIEW_BUDGET_RESULT").unwrap();
    let stale = variant == "stale";
    assert!(stale || variant == "verdict");
    if stale {
        let report = root.root.join(&review.report);
        let mut bytes = fs::read(&report).unwrap();
        bytes.extend_from_slice(b"\nOwned changed report after explicit budget update\n");
        fs::write(report, bytes).unwrap();
        assert!(publication::reconcile_departed(&root, &job.id).is_err());
    }
    let result = if stale {
        publication::retire_stale_departed(&root, &job.id)
    } else {
        publication::reconcile_departed(&root, &job.id)
    };
    fs::write(root.root.join("owned-review-budget-attempt.json"),serde_json::to_vec(&serde_json::json!({"error":result.as_ref().err().map(ToString::to_string),"work_unchanged":root.read_work(work).unwrap()==before,"provider_unchanged":fs::read(root.root.join("reviewer-invoked.json")).unwrap()==invoked})).unwrap()).unwrap();
    let result = result.expect("a waited original result must not need a new start budget");
    assert!(result.review_recorded && !result.task_accepted);
    assert_eq!(result.verdict_applicable, !stale);
    assert_eq!(result.work.revision, before.revision + 1);
    let current = result
        .work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == task.id)
        .unwrap();
    assert_eq!(current.status, task.status);
    let current_review = current.independent_review.as_ref().unwrap();
    assert_eq!(current_review.state, if stale { "stale" } else { "passed" });
    assert_eq!(
        current_review.finished,
        super::journal::read(&root, &job.id).unwrap().0.finished_at
    );
    let after: (f64, String) = root
        .db
        .query_row(
            "SELECT estimate,state FROM planning_calls WHERE id=?",
            [&job.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(after.0.to_bits(), finance.0.to_bits());
    assert_eq!(after.1, finance.1);
    assert_eq!(
        fs::read(root.root.join("reviewer-invoked.json")).unwrap(),
        invoked
    );
    assert_eq!(
        result
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
    assert!(
        publication::reconcile_departed(&root, &job.id)
            .unwrap()
            .replayed
    );
    let scope = result
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
        &result.work,
        scope,
        &root.read_settings().unwrap(),
    )
    .unwrap();
    fs::write(
        root.root.join("owned-review-budget-context.json"),
        context.bytes,
    )
    .unwrap();
    fs::write(root.root.join("owned-review-budget-result.json"),serde_json::to_vec(&serde_json::json!({"variant":variant,"before_publication_revision":before.revision,"state":current_review.state,"verdict_applicable":!stale,"start_still_refused":true,"historical_reservation_preserved":true,"foreign_reservation_refused":true,"missing_reservation_refused":true,"observed_or_corrupt_usage_refused":true,"transaction_required":true,"provider_invocation_unchanged":true,"replay_verified":true,"task_accepted":false,"settled":false,"model_received":false})).unwrap()).unwrap();
    fs::write(
        root.root.join("owned-review-public-certificate.json"),
        fs::read(root.root.join(&current_review.receipt)).unwrap(),
    )
    .unwrap();
    root.close().unwrap();
    println!("REVIEW_BUDGET_RESULT_CHECKED");
}
