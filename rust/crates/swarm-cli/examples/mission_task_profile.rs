//! Verification-only automatic retry with a genuine saved per-task launch profile.
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{error::Error, fs, path::Path};
use swarm_engine::{MissionLease, MissionWorkerRequest, RootStore, SystemEnvironment};

/// Real local operator start under current Work revision, never SQL consent.
fn start(store: &mut RootStore, work: &str) -> Result<(), Box<dyn Error>> {
    let request = json!({"schema_version":1,"event_id":"profile-start",
        "expected_revision":store.read_work(work)?.revision,
        "reason":"Owned inert automatic task-profile verification","slots":1,
        "profile":{"provider":"fixture","role":"worker","workspace":"mission-parent",
            "instruction":"Retain the mission parent profile","capture_output":true}});
    store.execute_external(
        work,
        "mission.start",
        &serde_json::to_vec(&request)?,
        &mut SystemEnvironment,
    )?;
    Ok(())
}

/// Require a specific safe cause; no error text becomes an execution permission.
fn has_cause(error: &(dyn Error + 'static), expected: &str) -> bool {
    let mut item = Some(error);
    while let Some(e) = item {
        if e.to_string() == expected {
            return true;
        }
        item = e.source();
    }
    false
}

/// Restore only an expected pre-grant refusal with unchanged injected Work;
/// unexpected execution/mutation remains preserved instead of being overwritten.
/// No positive claim, consent, receipt or retry decision is written through SQL.
fn negative_profile(
    store: &mut RootStore,
    db: &Connection,
    work: &str,
    agent: &str,
    parent: bool,
) -> Result<(), Box<dyn Error>> {
    let original: Vec<u8> =
        db.query_row("SELECT body FROM works WHERE id=?", [work], |r| r.get(0))?;
    let mut changed: Value = serde_json::from_slice(&original)?;
    let target = if parent {
        &mut changed["launch_profile"]
    } else {
        &mut changed["tasks"][0]["launch_profile"]
    };
    // Audit-only corruption leaves the identity-plan projection unchanged.
    // Full profile hashing must still detect it; no identity trigger is bypassed.
    target["actor"] = json!("fixture-changed-audit");
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        params![serde_json::to_vec(&changed)?, work],
    )?;
    let actual = store.run_agent(agent);
    let expected = if parent {
        "mission-agent-profile-changed"
    } else {
        "mission-agent-task-profile-changed"
    };
    // A refusal is not itself proof of no change. Preserve unexpected effects
    // rather than restoring a Work over a process that actually ran.
    let error = actual
        .err()
        .ok_or("changed profile unexpectedly released worker; retain fixture")?;
    assert!(
        has_cause(&error, expected),
        "wrong refusal; preserve fixture: {error:?}"
    );
    let retained: Vec<u8> =
        db.query_row("SELECT body FROM works WHERE id=?", [work], |r| r.get(0))?;
    assert_eq!(
        retained,
        serde_json::to_vec(&changed)?,
        "unexpected Work effect; preserve fixture"
    );
    let restore = db.execute(
        "UPDATE works SET body=? WHERE id=?",
        params![&original, work],
    );
    assert!(
        restore.is_ok(),
        "restore failed: {restore:?}; original refusal: {error:?}"
    );
    assert_eq!(restore?, 1);
    assert_eq!(store.read_agent(agent)?.status, "queued");
    let after: Vec<u8> = db.query_row("SELECT body FROM works WHERE id=?", [work], |r| r.get(0))?;
    assert_eq!(after, original);
    Ok(())
}

/// Positive authority comes from actual public worker completion, native retry,
/// start and owning lease. SQL below is negative configuration corruption only.
/// The actual automatic worker uses its own private gate and heartbeat cycle.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "_agent_gate") {
        let status = swarm_cli::run(
            &args[1..],
            "en",
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        );
        std::process::exit(i32::from(status));
    }
    assert_eq!(args.len(), 2);
    let input: Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let root = Path::new(input["root"].as_str().ok_or("root missing")?).canonicalize()?;
    let work = input["work"].as_str().ok_or("work missing")?;
    let marker: Value = serde_json::from_slice(&fs::read(
        root.join(".swarm/verification-task-profile.json"),
    )?)?;
    assert_eq!(marker, input);
    fs::create_dir(root.join("mission-parent"))?;
    let mut store = RootStore::open(&root, true)?;
    let before = store.read_work(work)?;
    let configured = before
        .tasks
        .as_ref()
        .and_then(|t| t.first())
        .and_then(|t| t.launch_profile.clone())
        .ok_or("genuine manual profile missing")?;
    assert!(before.tasks.as_ref().unwrap()[0].planning_retry);
    assert_eq!(Path::new(&configured.workspace), root);
    assert_eq!(
        before.tasks.as_ref().unwrap()[0]
            .model_selection
            .as_ref()
            .unwrap()
            .route
            .level,
        "simple"
    );
    start(&mut store, work)?;
    let parent = store.read_work(work)?.launch_profile;
    let lease: MissionLease = store.claim_mission_supervision(
        work,
        "task-profile-fixture",
        &swarm_engine::system_timestamp()?,
    )?;
    let request = MissionWorkerRequest {
        schema_version: 1,
        event_id: "automatic-profile".into(),
        expected_revision: store.read_work(work)?.revision,
        task_id: "task".into(),
    };
    let before = store.read_work(work)?;
    let preview = store.prepare_mission_worker(&lease, &request, true, &mut SystemEnvironment)?;
    assert_eq!(
        Path::new(&preview.agent.workspace),
        root,
        "task profile must precede mission default"
    );
    assert_eq!(preview.agent.model_route.as_ref().unwrap().level, "simple");
    assert_eq!(store.read_work(work)?, before);
    let binding: Value =
        serde_json::from_str(preview.agent.native_mission.as_ref().unwrap().0.get())?;
    assert_eq!(binding["schema_version"], 2);
    assert_eq!(binding["parent_profile_sha256"].as_str().unwrap().len(), 64);
    let queued = store.prepare_mission_worker(&lease, &request, false, &mut SystemEnvironment)?;
    assert!(queued.created);
    assert_eq!(store.read_work(work)?.launch_profile, parent);
    assert_eq!(
        store.read_work(work)?.tasks.as_ref().unwrap()[0]
            .launch_profile
            .as_deref(),
        Some(configured.as_ref())
    );
    let db = Connection::open(root.join(".swarm/state.db"))?;
    swarm_engine::register_identity_plan_projection(&db)?;
    negative_profile(&mut store, &db, work, &queued.agent.id, false)?;
    negative_profile(&mut store, &db, work, &queued.agent.id, true)?;
    assert!(!root.join("model-received.json").exists());
    let result = store.run_mission_cycle(&lease, |store, _, monitor| {
        assert!(monitor.is_running());
        store.run_agent(&queued.agent.id)
    })?;
    assert_eq!(result.agent.status, "completed");
    assert_eq!(Path::new(&result.agent.workspace), root);
    assert_eq!(result.agent.model_route.as_ref().unwrap().level, "simple");
    assert!(!result.task_accepted);
    assert!(root.join("model-received.json").is_file());
    assert!(root.join("handoff.md").is_file());
    assert!(!root.join("mission-parent/model-received.json").exists());
    let after = store.read_work(work)?;
    assert_eq!(after.launch_profile, parent);
    assert_eq!(
        after.tasks.as_ref().unwrap()[0].launch_profile.as_deref(),
        Some(configured.as_ref())
    );
    assert_eq!(
        after.tasks.as_ref().unwrap()[0]
            .attempts
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    lease.close()?;
    db.close().map_err(|(_, e)| e)?;
    store.close()?;
    println!(
        "{}",
        json!({"status":"PASS","task_profile_precedence":true,"model_choice_precedence":true,
        "parent_and_task_profile_rechecked":true,"configured_profiles_preserved":true,
        "actual_automatic_retry":true,"task_accepted":false})
    );
    Ok(())
}
