//! Actual owned failed-call finalization tests; no positive receipt is fabricated.
use super::{
    failure, journal, publication,
    store::{self, Job, Phase},
};
use crate::root_store::{RootStore, RootStoreError as E};
use std::{
    fs,
    process::{Command, Stdio},
};
/// Finalize one actual waited failed reviewer, proving refusal without mutation,
/// SQL rollback/recovery, exact replay and normal incident context. Each fault
/// follows a healthy witness and restores original bytes. Root/token/process
/// provenance is already verified by the parent SDK. No model is invoked, task
/// accepted, estimate settled/refunded or foreign process signalled here.
pub(super) fn verify(root: &RootStore, job: &Job) {
    let before = root.read_work(&job.work_id).unwrap();
    let before_bytes = crate::encode_model(&before).unwrap();
    let invoked = fs::read(root.root.join("reviewer-invoked.json")).unwrap();
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
    let estimated = finance();
    assert_eq!(estimated.1, "estimated");
    let unchanged = || {
        assert_eq!(
            crate::encode_model(&root.read_work(&job.work_id).unwrap()).unwrap(),
            before_bytes
        );
        assert!(store::job(&root.db, &job.id).unwrap().as_ref() == Some(job));
        assert_eq!(finance(), estimated);
    };
    let inspect = || {
        let tx = root.db.unchecked_transaction().unwrap();
        let v = failure::prepare(
            root,
            &tx,
            &job.id,
            publication::OwnerBoundary::CurrentOriginal,
        )
        .map(|_| ());
        tx.rollback().unwrap();
        v
    };
    inspect().unwrap();
    assert!(matches!(
        publication::reconcile_departed(root, &job.id),
        Err(E::Refused("review-guard-owner-still-present"))
    ));
    unchanged();
    let readonly = RootStore::open(&root.root, false).unwrap();
    assert!(matches!(
        publication::reconcile_failed(&readonly, &job.id),
        Err(E::Refused("root-read-only"))
    ));
    readonly.close().unwrap();
    unchanged();
    let private = root.root.join(".swarm").join(journal::name(&job.id));
    let original = fs::read(&private).unwrap();
    let (receipt, hash) = journal::read(root, &job.id).unwrap();
    assert!(receipt.failed && receipt.waited);
    let mut legacy = serde_json::to_value(&receipt).unwrap();
    legacy["schema_version"] = 1.into();
    legacy.as_object_mut().unwrap().remove("raw_capture");
    fs::write(
        &private,
        crate::native_receipt::encode(&serde_json::to_vec(&legacy).unwrap()).unwrap(),
    )
    .unwrap();
    assert!(publication::reconcile_failed(root, &job.id).is_err());
    unchanged();
    fs::write(&private, &original).unwrap();
    inspect().unwrap();
    let mut invalid = serde_json::to_value(&receipt).unwrap();
    invalid["waited"] = false.into();
    invalid["exit_code"] = serde_json::Value::Null;
    invalid["signal"] = serde_json::Value::Null;
    invalid["unresolved_pid"] = receipt.job.child_pid.unwrap().into();
    fs::write(
        &private,
        crate::native_receipt::encode(&serde_json::to_vec(&invalid).unwrap()).unwrap(),
    )
    .unwrap();
    journal::read(root, &job.id).expect("structurally valid unknown-wait original");
    assert!(matches!(
        publication::reconcile_failed(root, &job.id),
        Err(E::Refused("review-failure-original-not-applicable"))
    ));
    unchanged();
    fs::write(&private, &original).unwrap();
    inspect().unwrap();
    let module = module_path!().split_once("::").unwrap().1;
    let child = Command::new("/proc/self/exe")
        .args([
            "--exact",
            &format!("{module}::foreign_owner"),
            "--ignored",
            "--nocapture",
        ])
        .env("SWARM_OWNED_REVIEW_FAILURE_JOB", &job.id)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let foreign_pid = child.id();
    fs::write(
        root.root.join("owned-review-foreign-owner.json"),
        serde_json::to_vec(
            &serde_json::json!({"pid":foreign_pid,"waited":false,"provider_started":false}),
        )
        .unwrap(),
    )
    .unwrap();
    let result = child.wait_with_output().unwrap();
    fs::write(
        root.root.join("owned-review-foreign-owner.stdout"),
        &result.stdout,
    )
    .unwrap();
    fs::write(
        root.root.join("owned-review-foreign-owner.stderr"),
        &result.stderr,
    )
    .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("FOREIGN_REVIEW_OWNER_REFUSED")
            && String::from_utf8_lossy(&result.stdout).contains("1 passed; 0 failed")
    );
    assert_ne!(i64::from(foreign_pid), job.owner_pid);
    fs::write(root.root.join("owned-review-foreign-owner.json"),serde_json::to_vec(&serde_json::json!({"pid":foreign_pid,"waited":true,"foreign_owner_refused":true,"provider_started":false})).unwrap()).unwrap();
    unchanged();
    inspect().unwrap();
    root.db.execute_batch("CREATE TEMP TRIGGER owned_review_failed_commit BEFORE UPDATE ON native_review_jobs BEGIN SELECT RAISE(ABORT,'owned-failure-publication-rollback'); END;").unwrap();
    assert!(publication::reconcile_failed(root, &job.id).is_err());
    unchanged();
    root.db
        .execute_batch("DROP TRIGGER owned_review_failed_commit")
        .unwrap();
    inspect().unwrap();
    let public = super::certificate_sdk::one_candidate(root, &job.id);
    let certificate = fs::read(&public).unwrap();
    assert!(!String::from_utf8_lossy(&certificate).contains("PRIVATE"));
    let mut bad = certificate.clone();
    *bad.last_mut().unwrap() ^= 1;
    fs::write(&public, bad).unwrap();
    assert!(publication::reconcile_failed(root, &job.id).is_err());
    unchanged();
    fs::write(&public, &certificate).unwrap();
    inspect().unwrap();
    let done = publication::reconcile_failed(root, &job.id).unwrap();
    assert!(
        done.review_recorded && !done.verdict_applicable && !done.task_accepted && !done.replayed
    );
    assert_eq!(done.work.revision, before.revision + 1);
    let task = done
        .work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == job.task_id)
        .unwrap();
    let old = before
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == job.task_id)
        .unwrap();
    assert_eq!(task.status, old.status);
    let review = task.independent_review.as_ref().unwrap();
    assert_eq!(review.state, "error");
    assert!(review.criteria.as_ref().is_none_or(|c| c.is_empty()) && review.usage.is_none());
    assert_eq!(review.finished, receipt.finished_at);
    let terminal = store::job(&root.db, &job.id).unwrap().unwrap();
    assert!(terminal.phase == Phase::Refused);
    assert_eq!(terminal.receipt_sha256.as_deref(), Some(hash.as_str()));
    assert_eq!(terminal.published_revision, Some(done.work.revision));
    assert!(store::active(&root.db, &job.work_id, &job.task_id)
        .unwrap()
        .is_none());
    let scope = done
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
            &done.work,
            scope,
            &root.read_settings().unwrap(),
        )
    };
    let healthy = observe().unwrap();
    let context: serde_json::Value = serde_json::from_slice(&healthy.bytes).unwrap();
    assert!(context["reviews"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == job.id && r["state"] == "error"));
    assert_eq!(
        context["unavailable_reports"][&job.id],
        "native-review-failed-no-applicable-verdict"
    );
    assert!(!context["reports"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["event_id"] == job.id));
    assert!(healthy
        .delivery
        .unavailable_reports
        .as_ref()
        .unwrap()
        .contains_key(&job.id));
    let report = root.root.join(&review.report);
    let source = fs::read(&report).unwrap();
    fs::write(
        &report,
        b"Owned inapplicable changed report: not used as a failed-call verdict",
    )
    .unwrap();
    assert_eq!(observe().unwrap().bytes, healthy.bytes);
    fs::write(&report, source).unwrap();
    observe().unwrap();
    let saved = crate::encode_model(&done.work).unwrap();
    let replay = publication::reconcile_failed(root, &job.id).unwrap();
    assert!(replay.replayed && !replay.verdict_applicable && !replay.task_accepted);
    assert_eq!(crate::encode_model(&replay.work).unwrap(), saved);
    let mut oversized = certificate.clone();
    oversized.extend_from_slice(b" owned extra bytes");
    fs::write(&public, oversized).unwrap();
    assert!(observe().is_err() && publication::reconcile_failed(root, &job.id).is_err());
    fs::write(&public, &certificate).unwrap();
    observe().unwrap();
    assert!(
        publication::reconcile_failed(root, &job.id)
            .unwrap()
            .replayed
    );
    let mut corrupt = original.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    fs::write(&private, corrupt).unwrap();
    assert!(observe().is_err() && publication::reconcile_failed(root, &job.id).is_err());
    fs::write(&private, &original).unwrap();
    observe().unwrap();
    assert!(
        publication::reconcile_failed(root, &job.id)
            .unwrap()
            .replayed
    );
    let mut forged = done.work.clone();
    forged
        .planning
        .as_mut()
        .unwrap()
        .inbox
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e.id == job.id)
        .unwrap()
        .message
        .push_str(" forged");
    let scope = forged
        .planning
        .as_ref()
        .unwrap()
        .scopes
        .iter()
        .flatten()
        .find(|s| s.id == task.scope_id)
        .unwrap();
    assert!(crate::planning_scope_context::observe(
        &root.root,
        &forged,
        scope,
        &root.read_settings().unwrap()
    )
    .is_err());
    assert_eq!(
        crate::encode_model(&root.read_work(&job.work_id).unwrap()).unwrap(),
        saved
    );
    assert_eq!(finance(), estimated);
    assert_eq!(
        fs::read(root.root.join("reviewer-invoked.json")).unwrap(),
        invoked
    );
    // Actual negative fixture mutation only: a malformed event field must not
    // become an empty digest. Restore the byte-identical immutable request.
    let original_request: Vec<u8> = root
        .db
        .query_row(
            "SELECT request FROM events WHERE id=?",
            [&review.result_event],
            |r| r.get(0),
        )
        .unwrap();
    let mut malformed: serde_json::Value = serde_json::from_slice(&original_request).unwrap();
    malformed["reply_sha256"] = true.into();
    root.db
        .execute(
            "UPDATE events SET request=? WHERE id=?",
            rusqlite::params![serde_json::to_vec(&malformed).unwrap(), review.result_event],
        )
        .unwrap();
    assert!(observe().is_err());
    assert!(matches!(
        publication::reconcile_failed(root, &job.id),
        Err(E::Refused("review-publication-recorded-reply-digest-shape"))
    ));
    root.db
        .execute(
            "UPDATE events SET request=? WHERE id=?",
            rusqlite::params![original_request, review.result_event],
        )
        .unwrap();
    observe().unwrap();
    assert!(
        publication::reconcile_failed(root, &job.id)
            .unwrap()
            .replayed
    );
    let count: i64 = root
        .db
        .query_row(
            "SELECT count(*) FROM events WHERE id=?",
            [&review.result_event],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    let inbox: Vec<_> = done
        .work
        .planning
        .as_ref()
        .unwrap()
        .inbox
        .iter()
        .flatten()
        .filter(|e| e.id == job.id)
        .collect();
    assert_eq!(inbox.len(), 1);
    assert!(inbox[0].decision.is_empty() && inbox[0].artifacts.is_none());
    fs::write(
        root.root.join("owned-review-public-certificate.json"),
        &certificate,
    )
    .unwrap();
    fs::write(
        root.root.join("owned-review-failure-context.json"),
        &healthy.bytes,
    )
    .unwrap();
    fs::write(root.root.join("owned-review-failure-reconciliation.json"),serde_json::to_vec(&serde_json::json!({"failure_recorded":true,"verdict_applicable":false,"task_accepted":false,"review_state":"error","task_status":task.status,"malformed_digest_refused":true,"legacy_refused":true,"unknown_wait_refused":true,"foreign_owner_refused":true,"foreign_owner_child_waited":true,"sql_rollback_verified":true,"changed_public_refused":true,"private_changed_refused":true,"forged_inbox_refused":true,"normal_context_prepared":true,"inapplicable_report_not_loaded":true,"replay_without_exec":true,"finance_estimate_held":true,"inbox_events":1,"model_received":false,"decision_acknowledged":false})).unwrap()).unwrap();
}

/// An actual second OS process may read the original but cannot finalize while
/// its original owner is alive. This ignored helper is explicitly dispatched by
/// the owned parent above; it performs no provider exec, signal or mutation.
#[test]
#[ignore = "owned failed-review parent dispatch only"]
fn foreign_owner() {
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(std::env::var_os("SWARM_OWNED_REVIEW_CLAIM_FIXTURE").unwrap()).unwrap(),
    )
    .unwrap();
    let path = std::path::Path::new(manifest["root"].as_str().unwrap());
    assert_eq!(
        fs::read_to_string(path.join(".swarm/owned-review-claim-fixture")).unwrap(),
        manifest["token"].as_str().unwrap()
    );
    let id = std::env::var("SWARM_OWNED_REVIEW_FAILURE_JOB").unwrap();
    let root = RootStore::open(path, true).unwrap();
    let work = root.read_work(manifest["work"].as_str().unwrap()).unwrap();
    assert!(matches!(
        publication::reconcile_failed(&root, &id),
        Err(E::Refused("review-guard-owner-changed"))
    ));
    assert!(matches!(
        publication::reconcile_departed(&root, &id),
        Err(E::Refused("review-guard-owner-still-present"))
    ));
    assert_eq!(root.read_work(&work.id).unwrap(), work);
    root.close().unwrap();
    println!("FOREIGN_REVIEW_OWNER_REFUSED");
}
