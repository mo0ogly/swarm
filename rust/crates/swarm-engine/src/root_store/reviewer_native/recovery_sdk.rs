//! Recover genuine originals only after their actual creator process exits.
use super::{
    guard, journal, publication,
    store::{self, Phase},
};
use crate::root_store::{RootStore, RootStoreError as E};
use std::{fs, path::Path};
/// Start from a token-bound original produced and waited by an earlier SDK.
/// Prove actual same-host owner departure, immutable owner/child/job fields,
/// known-wait/source/readonly/host refusals, SQL rollback and idempotent replay.
/// No process is adopted, signalled or started here. Context is data preparation,
/// not model reception, authority authentication, settlement or task acceptance.
#[test]
#[ignore = "owned previously completed original SDK fixture required"]
fn reviewer_recover_departed_owned_public_fixture() {
    let path = std::env::var_os("SWARM_OWNED_REVIEW_CLAIM_FIXTURE").unwrap();
    let input: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let root_path = Path::new(input["root"].as_str().unwrap());
    assert_eq!(
        fs::read_to_string(root_path.join(".swarm/owned-review-claim-fixture")).unwrap(),
        input["token"].as_str().unwrap()
    );
    let root = RootStore::open(root_path, true).unwrap();
    let before = root.read_work(input["work"].as_str().unwrap()).unwrap();
    let task = before
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == input["request"]["task_id"].as_str().unwrap())
        .unwrap();
    let review = task.independent_review.as_ref().unwrap();
    let job = store::job(&root.db, &review.id).unwrap().unwrap();
    let (original, hash) = journal::read(&root, &job.id).unwrap();
    assert!(original.waited && original.process_attempted && original.grant_attempted);
    let caller = crate::SupervisorIdentity::current().unwrap();
    assert_eq!(caller.host(), job.owner_host);
    assert_ne!(caller.pid(), job.owner_pid);
    guard::departed(&job).unwrap();
    let mut live = job.clone();
    live.owner_pid = caller.pid();
    live.owner_stamp = caller.process_stamp().into();
    assert!(matches!(
        guard::departed(&live),
        Err(E::Refused("review-guard-owner-still-present"))
    ));
    let mut foreign = job.clone();
    foreign.owner_host = "owned-foreign-host-fixture".into();
    assert!(matches!(
        guard::departed(&foreign),
        Err(E::Refused("review-guard-owner-host-changed"))
    ));
    guard::departed(&job).unwrap();
    let invocation = fs::read(root.root.join("reviewer-invoked.json")).unwrap();
    let finance = || {
        root.db
            .query_row(
                "SELECT estimate,state FROM planning_calls WHERE id=?",
                [&job.id],
                |r| {
                    Ok((
                        r.get::<_, rusqlite::types::Value>(0)?,
                        r.get::<_, String>(1)?,
                    ))
                },
            )
            .unwrap()
    };
    let reserved = finance();
    assert_eq!(reserved.1, "estimated");
    let unchanged = || {
        assert_eq!(root.read_work(&before.id).unwrap(), before);
        assert!(store::job(&root.db, &job.id).unwrap().as_ref() == Some(&job));
        assert_eq!(finance(), reserved);
    };
    publication::inspect_departed(&root, &job.id).unwrap();
    unchanged();
    let ro = RootStore::open(&root.root, false).unwrap();
    assert!(matches!(
        publication::reconcile_departed(&ro, &job.id),
        Err(E::Refused("root-read-only"))
    ));
    ro.close().unwrap();
    unchanged();
    let private = root.root.join(".swarm").join(journal::name(&job.id));
    let frame = fs::read(&private).unwrap();
    let mut unknown = serde_json::to_value(&original).unwrap();
    unknown["waited"] = false.into();
    unknown["exit_code"] = serde_json::Value::Null;
    unknown["signal"] = serde_json::Value::Null;
    unknown["unresolved_pid"] = job.child_pid.unwrap().into();
    unknown["failed"] = true.into();
    unknown["output_complete"] = false.into();
    fs::write(
        &private,
        crate::native_receipt::encode(&serde_json::to_vec(&unknown).unwrap()).unwrap(),
    )
    .unwrap();
    journal::read(&root, &job.id).unwrap();
    assert!(matches!(
        publication::reconcile_departed(&root, &job.id),
        Err(E::Refused("review-failure-original-not-applicable"))
    ));
    unchanged();
    fs::write(&private, &frame).unwrap();
    publication::inspect_departed(&root, &job.id).unwrap();
    let report = root.root.join(&review.report);
    let report_original = fs::read(&report).unwrap();
    fs::write(
        root.root.join("owned-review-recovery-source-original"),
        &report_original,
    )
    .unwrap();
    fs::write(
        &report,
        b"Owned source changed after original owner departed",
    )
    .unwrap();
    if original.failed {
        publication::inspect_departed(&root, &job.id).unwrap();
    } else {
        assert!(publication::inspect_departed(&root, &job.id).is_err());
        assert!(publication::reconcile_departed(&root, &job.id).is_err());
        unchanged();
    }
    fs::write(&report, &report_original).unwrap();
    publication::inspect_departed(&root, &job.id).unwrap();
    root.db.execute_batch("CREATE TEMP TRIGGER owned_departed_commit BEFORE UPDATE ON native_review_jobs BEGIN SELECT RAISE(ABORT,'owned-departed-review-rollback'); END;").unwrap();
    assert!(publication::reconcile_departed(&root, &job.id).is_err());
    unchanged();
    root.db
        .execute_batch("DROP TRIGGER owned_departed_commit")
        .unwrap();
    publication::inspect_departed(&root, &job.id).unwrap();
    let public = super::certificate_sdk::one_candidate(&root, &job.id);
    let certificate = fs::read(&public).unwrap();
    let mut bad = certificate.clone();
    *bad.last_mut().unwrap() ^= 1;
    fs::write(&public, bad).unwrap();
    assert!(publication::reconcile_departed(&root, &job.id).is_err());
    unchanged();
    fs::write(&public, &certificate).unwrap();
    publication::inspect_departed(&root, &job.id).unwrap();
    let output = publication::reconcile_departed(&root, &job.id).unwrap();
    assert!(output.review_recorded && !output.task_accepted && !output.replayed);
    assert_eq!(output.verdict_applicable, !original.failed);
    assert_eq!(output.work.revision, before.revision + 1);
    let current = output
        .work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == task.id)
        .unwrap();
    let result = current.independent_review.as_ref().unwrap();
    let expected = if original.failed {
        "error"
    } else if input["controller_case"] == "changes-requested" {
        "changes_requested"
    } else {
        "passed"
    };
    assert_eq!(result.state, expected);
    assert_eq!(
        current.status,
        if expected == "changes_requested" {
            "blocked"
        } else {
            task.status.as_str()
        }
    );
    assert_eq!(result.finished, original.finished_at);
    assert!(result.usage.is_none());
    let mut terminal = job.clone();
    terminal.phase = if original.failed {
        Phase::Refused
    } else {
        Phase::Recorded
    };
    terminal.receipt_sha256 = Some(hash);
    terminal.published_revision = Some(output.work.revision);
    assert!(store::job(&root.db, &job.id).unwrap().as_ref() == Some(&terminal));
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
    let observe = || {
        crate::planning_scope_context::observe(
            &root.root,
            &output.work,
            scope,
            &root.read_settings().unwrap(),
        )
    };
    let context = observe().unwrap();
    let data: serde_json::Value = serde_json::from_slice(&context.bytes).unwrap();
    assert!(data["reviews"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == job.id && r["state"] == expected));
    if original.failed {
        assert_eq!(
            data["unavailable_reports"][&job.id],
            "native-review-failed-no-applicable-verdict"
        );
    } else {
        assert!(data["reports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["event_id"] == job.id));
    }
    assert!(
        publication::reconcile_departed(&root, &job.id)
            .unwrap()
            .replayed
    );
    let mut oversized = certificate.clone();
    oversized.extend_from_slice(b" owned replay overflow");
    fs::write(&public, oversized).unwrap();
    assert!(publication::reconcile_departed(&root, &job.id).is_err() && observe().is_err());
    fs::write(&public, &certificate).unwrap();
    observe().unwrap();
    assert!(
        publication::reconcile_departed(&root, &job.id)
            .unwrap()
            .replayed
    );
    let mut corrupt = frame.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    fs::write(&private, corrupt).unwrap();
    assert!(publication::reconcile_departed(&root, &job.id).is_err() && observe().is_err());
    fs::write(&private, &frame).unwrap();
    observe().unwrap();
    assert!(
        publication::reconcile_departed(&root, &job.id)
            .unwrap()
            .replayed
    );
    assert_eq!(root.read_work(&before.id).unwrap(), output.work);
    assert_eq!(finance(), reserved);
    assert_eq!(
        fs::read(root.root.join("reviewer-invoked.json")).unwrap(),
        invocation
    );
    let count: i64 = root
        .db
        .query_row(
            "SELECT count(*) FROM events WHERE id=?",
            [&result.result_event],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    fs::write(
        root.root.join("owned-review-public-certificate.json"),
        &certificate,
    )
    .unwrap();
    fs::write(
        root.root.join("owned-review-departed-context.json"),
        &context.bytes,
    )
    .unwrap();
    fs::write(root.root.join("owned-review-departed-recovery.json"),serde_json::to_vec(&serde_json::json!({"owner_confirmed_departed":true,"original_owner_preserved":true,"known_wait_required":true,"foreign_host_refused":true,"live_tuple_refused":true,"unknown_wait_refused":true,"source_guard_for_verdict":!original.failed,"source_inapplicable_for_failed_call":original.failed,"sql_rollback_verified":true,"changed_public_refused":true,"changed_private_refused":true,"replay_without_exec":true,"provider_invocation_unchanged":true,"financial_estimate_held":true,"verdict_applicable":output.verdict_applicable,"review_state":expected,"task_status":current.status,"applying_os_pid":caller.pid(),"original_os_pid":job.owner_pid,"model_received":false,"decision_acknowledged":false,"task_accepted":false})).unwrap()).unwrap();
    root.close().unwrap();
    println!("DEPARTED_REVIEW_ORIGINAL_RECOVERED");
}
