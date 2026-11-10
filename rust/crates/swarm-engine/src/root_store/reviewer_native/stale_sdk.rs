//! Genuine waited original with changed dependency; no new provider or positive receipt.
use super::{journal, publication, store};
use crate::root_store::RootStore;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};
/// Apply one supported public fixture operation, retaining exact inputs/channels.
/// The SDK owns this Root/token; no shell, live mission, provider or hidden retry.
/// CLI/JSON failure panics with original channels saved before any assertion.
pub(super) fn cli(
    root: &RootStore,
    label: &str,
    args: &[&str],
    input: Option<serde_json::Value>,
) -> serde_json::Value {
    let binary = std::env::var_os("SWARM_OWNED_REVIEW_PUBLIC_BINARY").unwrap();
    let mut command: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    command.extend([
        "--root".into(),
        root.root.to_str().unwrap().into(),
        "--json".into(),
    ]);
    if let Some(value) = input {
        let path = root
            .root
            .join(format!("owned-review-stale-{label}.input.json"));
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        command.extend(["--input".into(), path.to_str().unwrap().into()]);
    }
    fs::write(
        root.root
            .join(format!("owned-review-stale-{label}.command.json")),
        serde_json::to_vec(&command).unwrap(),
    )
    .unwrap();
    let output = Command::new(binary)
        .args(command)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    fs::write(
        root.root.join(format!("owned-review-stale-{label}.stdout")),
        &output.stdout,
    )
    .unwrap();
    fs::write(
        root.root.join(format!("owned-review-stale-{label}.stderr")),
        &output.stderr,
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
/// Valid witness before every injection: unchanged, missing or unreadable report
/// must refuse retirement. A real verdict rollback then changed report/settings/
/// pause or resumed authority must permit only explicit non-verdict output. Test
/// another SQL rollback, exact replay, changed public/private refusal and skipped
/// invalid report context; retain originals and held financial estimate. No model,
/// acceptance, owner adoption or second provider execution is established.
#[test]
#[ignore = "owned completed original fixture only"]
fn reviewer_stale_owned_fixture() {
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
    let mut before = root.read_work(input["work"].as_str().unwrap()).unwrap();
    let task = before
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == input["request"]["task_id"].as_str().unwrap())
        .unwrap()
        .clone();
    let review = task.independent_review.as_ref().unwrap().clone();
    let job = store::job(&root.db, &review.id).unwrap().unwrap();
    let invoked = fs::read(path.join("reviewer-invoked.json")).unwrap();
    let report_path = path.join(&review.report);
    let report = fs::read(&report_path).unwrap();
    publication::inspect_departed(&root, &job.id).unwrap();
    assert!(publication::retire_stale_departed(&root, &job.id).is_err());
    assert_eq!(root.read_work(&before.id).unwrap(), before);
    fs::write(&report_path, [0xff]).unwrap();
    assert!(publication::retire_stale_departed(&root, &job.id).is_err());
    assert_eq!(root.read_work(&before.id).unwrap(), before);
    fs::write(&report_path, &report).unwrap();
    publication::inspect_departed(&root, &job.id).unwrap();
    let retained = root.root.join("owned-review-stale-report-retained");
    assert!(!retained.exists());
    fs::rename(&report_path, &retained).unwrap();
    assert!(publication::retire_stale_departed(&root, &job.id).is_err());
    assert_eq!(root.read_work(&before.id).unwrap(), before);
    fs::rename(&retained, &report_path).unwrap();
    publication::inspect_departed(&root, &job.id).unwrap();
    root.db.execute_batch("CREATE TEMP TRIGGER owned_stale_rollback BEFORE UPDATE ON native_review_jobs BEGIN SELECT RAISE(ABORT,'owned-stale-rollback'); END;").unwrap();
    assert!(publication::reconcile_departed(&root, &job.id).is_err());
    assert_eq!(root.read_work(&before.id).unwrap(), before);
    root.db
        .execute_batch("DROP TRIGGER owned_stale_rollback")
        .unwrap();
    let candidate = super::certificate_sdk::one_candidate(&root, &job.id);
    let candidate_bytes = fs::read(&candidate).unwrap();
    let variant = std::env::var("SWARM_OWNED_REVIEW_STALE_VARIANT").unwrap();
    let reason = match variant.as_str() {
        "report" => {
            let mut changed = report.clone();
            changed.extend_from_slice(b"\nChanged owned review source\n");
            fs::write(&report_path, &changed).unwrap();
            "report-changed"
        }
        "settings" => {
            let old = cli(&root, "settings-show", &["engine-settings", "show"], None);
            let mut values = old["values"].clone();
            assert_ne!(values["agent_capture_max_bytes"], "0");
            values["agent_capture_max_bytes"] = "0".into();
            let mut change = serde_json::json!({"schema_version":old["schema_version"],"expected_revision":old["revision"],"event_id":"owned-review-stale-settings","values":values,"reason":"Explicit owned stale dependency witness","preview_token":""});
            let preview = cli(
                &root,
                "settings-preview",
                &["engine-settings", "preview"],
                Some(change.clone()),
            );
            change["preview_token"] = preview["preview_token"].clone();
            cli(
                &root,
                "settings-apply",
                &["engine-settings", "apply"],
                Some(change),
            );
            assert_ne!(
                root.read_settings().unwrap().revision.0,
                job.settings_revision
            );
            "settings-changed"
        }
        "paused" | "authority" => {
            let w = root.read_work(&before.id).unwrap();
            cli(
                &root,
                "pause",
                &["planning", "pause", &before.id],
                Some(
                    serde_json::json!({"schema_version":1,"event_id":"owned-review-stale-pause","expected_revision":w.revision,"reason":"Explicit owned stale authority witness"}),
                ),
            );
            if variant == "authority" {
                let w = root.read_work(&before.id).unwrap();
                cli(
                    &root,
                    "resume",
                    &["planning", "resume", &before.id],
                    Some(
                        serde_json::json!({"schema_version":1,"event_id":"owned-review-stale-resume","expected_revision":w.revision,"reason":"Explicit owned resumed authority witness"}),
                    ),
                );
                "validation-source-changed"
            } else {
                "planning-inactive"
            }
        }
        _ => panic!("unsupported owned stale variant"),
    };
    before = root.read_work(&before.id).unwrap();
    assert!(publication::reconcile_departed(&root, &job.id).is_err());
    assert_eq!(root.read_work(&before.id).unwrap(), before);
    let finance: (u64, String) = root
        .db
        .query_row(
            "SELECT estimate,state FROM planning_calls WHERE id=?",
            [&job.id],
            |r| Ok((r.get::<_, f64>(0)?.to_bits(), r.get::<_, String>(1)?)),
        )
        .unwrap();
    root.db.execute_batch("CREATE TEMP TRIGGER owned_stale_rollback BEFORE UPDATE ON native_review_jobs BEGIN SELECT RAISE(ABORT,'owned-stale-rollback'); END;").unwrap();
    assert!(publication::retire_stale_departed(&root, &job.id).is_err());
    assert_eq!(root.read_work(&before.id).unwrap(), before);
    assert!(store::job(&root.db, &job.id).unwrap().as_ref() == Some(&job));
    root.db
        .execute_batch("DROP TRIGGER owned_stale_rollback")
        .unwrap();
    let output = publication::retire_stale_departed(&root, &job.id);
    fs::write(path.join("owned-review-stale-attempt.json"),serde_json::to_vec(&serde_json::json!({"error":output.as_ref().err().map(ToString::to_string),"original_candidate_preserved":fs::read(&candidate).unwrap()==candidate_bytes,"original_provider_unchanged":fs::read(path.join("reviewer-invoked.json")).unwrap()==invoked})).unwrap()).unwrap();
    let output = output.expect(
        "known successful original with changed dependency needs explicit non-verdict exit",
    );
    assert!(output.review_recorded && !output.verdict_applicable && !output.task_accepted);
    assert_eq!(output.work.revision, before.revision + 1);
    let current = output
        .work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == task.id)
        .unwrap();
    assert_eq!(current.status, task.status);
    let review = current.independent_review.as_ref().unwrap();
    assert_eq!(review.state, "stale");
    assert_eq!(
        review.reason,
        format!("native-review-original-stale: {reason}")
    );
    assert!(review.criteria.as_ref().is_none_or(|v| v.is_empty()));
    assert_eq!(fs::read(&candidate).unwrap(), candidate_bytes);
    assert_ne!(path.join(&review.receipt), candidate);
    let public = path.join(&review.receipt);
    let public_bytes = fs::read(&public).unwrap();
    let mut bad = public_bytes.clone();
    bad[0] ^= 1;
    fs::write(&public, bad).unwrap();
    assert!(publication::retire_stale_departed(&root, &job.id).is_err());
    assert_eq!(root.read_work(&before.id).unwrap(), output.work);
    fs::write(&public, &public_bytes).unwrap();
    assert!(
        publication::retire_stale_departed(&root, &job.id)
            .unwrap()
            .replayed
    );
    let private = root.root.join(".swarm").join(journal::name(&job.id));
    let original = fs::read(&private).unwrap();
    let mut bad = original.clone();
    bad[0] ^= 1;
    fs::write(&private, bad).unwrap();
    assert!(publication::retire_stale_departed(&root, &job.id).is_err());
    fs::write(&private, &original).unwrap();
    assert!(
        publication::retire_stale_departed(&root, &job.id)
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
    fs::write(&report_path, [0xff]).unwrap();
    let context = crate::planning_scope_context::observe(
        path,
        &output.work,
        scope,
        &root.read_settings().unwrap(),
    )
    .unwrap();
    let context: serde_json::Value = serde_json::from_slice(&context.bytes).unwrap();
    assert_eq!(
        context["unavailable_reports"][&job.id],
        "native-review-stale-no-applicable-verdict"
    );
    fs::write(&report_path, &report).unwrap();
    assert!(
        publication::retire_stale_departed(&root, &job.id)
            .unwrap()
            .replayed
    );
    assert_eq!(
        fs::read(path.join("reviewer-invoked.json")).unwrap(),
        invoked
    );
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
    let after_finance: (u64, String) = root
        .db
        .query_row(
            "SELECT estimate,state FROM planning_calls WHERE id=?",
            [&job.id],
            |r| Ok((r.get::<_, f64>(0)?.to_bits(), r.get::<_, String>(1)?)),
        )
        .unwrap();
    assert_eq!(after_finance, finance);
    assert_eq!(finance.1, "estimated");
    fs::write(
        path.join("owned-review-stale-context.json"),
        serde_json::to_vec(&context).unwrap(),
    )
    .unwrap();
    fs::write(path.join("owned-review-stale-recovery.json"),serde_json::to_vec(&serde_json::json!({"variant":variant,"reason":reason,"before_publication_revision":before.revision,"state":"stale","verdict_applicable":false,"task_accepted":false,"original_candidate_preserved":true,"provider_invocation_unchanged":true,"replay_verified":true,"current_report_not_loaded":true,"unchanged_refused":true,"unreadable_refused":true,"missing_report_refused":true,"sql_rollback_verified":true,"public_changed_refused":true,"private_changed_refused":true,"finance_estimate_held":true,"model_received":false,"decision_acknowledged":false})).unwrap()).unwrap();
    fs::write(
        path.join("owned-review-public-certificate.json"),
        public_bytes,
    )
    .unwrap();
    root.close().unwrap();
    println!("STALE_REVIEW_ORIGINAL_RETIRED");
}
