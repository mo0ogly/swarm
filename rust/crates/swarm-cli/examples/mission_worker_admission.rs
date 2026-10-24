//! Verification-only native automatic-worker caller over an owned marked fixture.
use rusqlite::{types::Value, Connection};
use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::Path};
use swarm_engine::{MissionLease, MissionWorkerRequest, RootStore, SystemEnvironment, Work};
type Rows = Vec<Vec<Value>>;
type Snapshot = (Work, Vec<(&'static str, Option<Rows>)>);

/// Observe data and optional native-schema presence without manufacturing rows.
fn snapshot(store: &RootStore, db: &Connection, work: &str) -> Result<Snapshot, Box<dyn Error>> {
    let mut all = Vec::new();
    for table in [
        "workspace_turns",
        "workspace_turn_events",
        "agents",
        "reservations",
        "events",
        "cockpit_controls",
        "cockpit_events",
        "mission_supervision",
        "native_mission_work_receipts",
        "native_mission_claim_receipts",
    ] {
        let exists: i64 = db.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name=?",
            [table],
            |r| r.get(0),
        )?;
        if exists == 0 {
            all.push((table, None));
            continue;
        }
        // Table names above are fixed fixture vocabulary, never external input.
        let mut statement = db.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))?;
        let count = statement.column_count();
        let mut values = Vec::new();
        {
            let mut rows = statement.query([])?;
            while let Some(row) = rows.next()? {
                values.push(
                    (0..count)
                        .map(|i| row.get(i))
                        .collect::<Result<Vec<Value>, _>>()?,
                );
            }
        }
        statement.finalize()?;
        all.push((table, Some(values)));
    }
    Ok((store.read_work(work)?, all))
}
/// Apply a fresh audited operator control using its actually observed revision.
fn control(
    store: &mut RootStore,
    work: &str,
    kind: &str,
    id: &str,
    profile: Option<Json>,
    slots: Option<i64>,
) -> Result<Work, Box<dyn Error>> {
    let mut r = json!({"schema_version":1,"event_id":id,"expected_revision":store.read_work(work)?.revision,"reason":"Owned automatic worker admission verification"});
    if let Some(p) = profile {
        r["profile"] = p;
    }
    if let Some(n) = slots {
        r["slots"] = json!(n);
    }
    Ok(store
        .execute_external(work, kind, &serde_json::to_vec(&r)?, &mut SystemEnvironment)?
        .result
        .work)
}
/// Resolve fresh task/revision input; preview creates no actual worker or grant.
fn intention(
    store: &RootStore,
    work: &str,
    task: &str,
    event: &str,
) -> Result<MissionWorkerRequest, Box<dyn Error>> {
    Ok(MissionWorkerRequest {
        schema_version: 1,
        event_id: event.into(),
        expected_revision: store.read_work(work)?.revision,
        task_id: task.into(),
    })
}
/// Synchronize ready contenders through the public lease API before the positive
/// preview/denial witness. Synchronization is explicit setup, never part of the
/// preview no-mutation claim; queue/history are included in subsequent snapshots.
fn healthy(
    store: &mut RootStore,
    db: &Connection,
    lease: &MissionLease,
    work: &str,
    task: &str,
) -> Result<(), Box<dyn Error>> {
    store.update_mission_supervision(lease, &swarm_engine::system_timestamp()?, false)?;
    let registration = store.sync_mission_workspace_queue(lease)?;
    if let Some(error) = registration.permission_refusal {
        return Err(Box::new(error));
    }
    if !registration.ready.iter().any(|id| id == task) {
        return Err("expected task absent from explicitly synchronized ready queue".into());
    }
    let r = intention(store, work, task, "preview-worker")?;
    let before = snapshot(store, db, work)?;
    let result = store.prepare_mission_worker(lease, &r, true, &mut SystemEnvironment)?;
    assert!(
        result.preview_only
            && !result.created
            && result.agent.origin == "conducteur"
            && result.agent.native_mission.is_some()
    );
    assert_eq!(snapshot(store, db, work)?, before);
    Ok(())
}
/// Require the expected typed cause without state mutation, retaining safe labels.
fn denied(
    store: &mut RootStore,
    db: &Connection,
    lease: &MissionLease,
    work: &str,
    task: &str,
    expected: &str,
) -> Result<(), Box<dyn Error>> {
    let r = intention(store, work, task, "denied-worker")?;
    let before = snapshot(store, db, work)?;
    let error = store
        .prepare_mission_worker(lease, &r, true, &mut SystemEnvironment)
        .err()
        .ok_or("expected denial")?;
    let mut cause: Option<&(dyn Error + 'static)> = Some(&error);
    let mut found = false;
    while let Some(e) = cause {
        found |= e.to_string() == expected;
        cause = e.source();
    }
    assert!(found, "wrong refusal boundary: {error:?}");
    assert_eq!(snapshot(store, db, work)?, before);
    Ok(())
}

/// Save an explicit fixture policy through real preview/CAS persistence.
/// No consumption reset, budget edit, SQL authority or automatic permission.
fn save_cost_policy(store: &RootStore, id: &str, cap: Option<i64>) -> Result<(), Box<dyn Error>> {
    let current = store.settings()?;
    let mut values = current.values.clone();
    values.mission_dispatch_cost_reserve_percent = Some(swarm_engine::SettingCap(
        cap.map(swarm_engine::SettingCount),
    ));
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: id.into(),
        expected_revision: current.revision,
        values,
        reason: "Inert cost admission verification".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change)?.preview_token;
    store.apply_settings(&change, &mut SystemEnvironment)?;
    Ok(())
}

/// Exercise genuine start/lease/factory/private-gate/worker/completion linkage.
/// Owned control-mode revocation and retained historical cost data use SQL;
/// valid
/// permission always comes from public start and lease operations. Old queued
/// intentions cannot regain permission after consent/profile/epoch replacement.
/// Historical provenance permits cancellation after revocation. No paid provider
/// or acceptance occurs, and released workers retain independent supervision.
/// The actual worker runs inside the descriptor-bound heartbeat cycle.
/// Its retained cost summary must count missing declarations, without inferring
/// free execution or including planner calls. The 25 USD retained-data fixture
/// plus the actual inert worker report are simulations, never provider prices.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "_agent_gate") {
        let code = swarm_cli::run(
            &args[1..],
            "en",
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        );
        std::process::exit(i32::from(code));
    }
    assert_eq!(args.len(), 2);
    let input: Json = serde_json::from_slice(&fs::read(&args[1])?)?;
    let root = Path::new(input["root"].as_str().ok_or("missing fixture root")?).canonicalize()?;
    let work = input["work"].as_str().ok_or("missing fixture work")?;
    let marker: Json = serde_json::from_slice(&fs::read(
        root.join(".swarm/verification-mission-worker.json"),
    )?)?;
    assert_eq!(marker["root"].as_str(), root.to_str());
    assert_eq!(marker["work"].as_str(), Some(work));
    fs::create_dir(root.join("first"))?;
    fs::create_dir(root.join("second"))?;
    let mut store = RootStore::open(&root, true)?;
    let db = Connection::open(root.join(".swarm/state.db"))?;
    let profile = json!({"provider":"fixture","role":"worker","workspace":"first","level":"standard","capture_output":true});
    control(
        &mut store,
        work,
        "mission.start",
        "start-original",
        Some(profile.clone()),
        Some(1),
    )?;
    let lease = store.claim_mission_supervision(
        work,
        "owned-worker-verification",
        &swarm_engine::system_timestamp()?,
    )?;
    healthy(&mut store, &db, &lease, work, "task")?;

    // Retained historical cost data, not a forged execution receipt or permission.
    // The positive permission witness above uses only actual mission operations.
    store.execute_external(work, "budget.configure", &serde_json::to_vec(&json!({
        "schema_version":1,"event_id":"cost-budget","expected_revision":store.read_work(work)?.revision,
        "budget":{"limit_usd":1000.0,"reserve_per_launch_usd":1.0,"estimate_source":"inert fixture","reference_date":"2026-10-22"}
    }))?, &mut SystemEnvironment)?;
    let mut historical = swarm_engine::Agent::default();
    historical.id = "historical-cost".into();
    historical.work_id = work.into();
    historical.task_id = "task".into();
    historical.provider = "fixture".into();
    historical.workspace = root.join("first").to_str().ok_or("fixture path")?.into();
    historical.status = "completed".into();
    historical.usage = Some(Box::new(swarm_engine::Usage {
        provider_reported_cost_usd: Some(Box::new(swarm_engine::GoFloat(25.0))),
        ..Default::default()
    }));
    db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,'completed',?,?)",
        rusqlite::params![historical.id,work,"task",historical.workspace,swarm_engine::encode_model(&historical)?,b"{}".as_slice()])?;
    let cost_request = MissionWorkerRequest {
        schema_version: 1,
        event_id: "cost-refused".into(),
        expected_revision: store.read_work(work)?.revision,
        task_id: "task".into(),
    };
    let before_cost = snapshot(&store, &db, work)?;
    let refused =
        store.prepare_mission_worker(&lease, &cost_request, false, &mut SystemEnvironment);
    assert!(refused.is_err(), "reported task threshold must refuse");
    denied(
        &mut store,
        &db,
        &lease,
        work,
        "task",
        "mission-task-reported-cost-threshold",
    )?;
    assert_eq!(
        snapshot(&store, &db, work)?,
        before_cost,
        "refused cost admission mutated SQL"
    );
    for (id, cap, allowed) in [
        ("cost-equality", Some(2500), true),
        ("cost-below", Some(2499), false),
        ("cost-null", None, true),
        ("cost-zero", Some(0), false),
        ("cost-restore-null", None, true),
    ] {
        save_cost_policy(&store, id, cap)?;
        let before = snapshot(&store, &db, work)?;
        let result =
            store.prepare_mission_worker(&lease, &cost_request, true, &mut SystemEnvironment);
        assert_eq!(result.is_ok(), allowed, "cost policy {id}");
        assert_eq!(
            snapshot(&store, &db, work)?,
            before,
            "cost preview mutated SQL"
        );
    }
    let original = store.read_work(work)?.launch_profile;
    let valid = store.prepare_mission_worker(
        &lease,
        &intention(&store, work, "task", "raw-proof")?,
        true,
        &mut SystemEnvironment,
    )?;
    let binding: Json = serde_json::from_str(valid.agent.native_mission.as_ref().unwrap().0.get())?;
    let raw = swarm_engine::Launch {
        schema_version: 1,
        event_id: "forged-origin".into(),
        expected_revision: store.read_work(work)?.revision,
        task_id: "task".into(),
        provider: "fixture".into(),
        role: "worker".into(),
        origin: "conducteur".into(),
        conductor_id: binding["conductor"].as_str().unwrap().into(),
        ..Default::default()
    };
    assert!(store
        .prepare_agent_launch(work, &raw, true, &mut SystemEnvironment)
        .is_err());
    control(
        &mut store,
        work,
        "mission.pause",
        "pause-before-worker",
        None,
        None,
    )?;
    denied(
        &mut store,
        &db,
        &lease,
        work,
        "task",
        "planning-mission-paused",
    )?;
    control(
        &mut store,
        work,
        "mission.unpause",
        "unpause-before-worker",
        None,
        None,
    )?;
    healthy(&mut store, &db, &lease, work, "task")?;
    // Negative-only revocation; recovery uses the public operator command.
    db.execute(
        "UPDATE cockpit_controls SET autonomy='manuel' WHERE work_id=?",
        [work],
    )?;
    denied(
        &mut store,
        &db,
        &lease,
        work,
        "task",
        "mission-autonomy-required",
    )?;
    control(
        &mut store,
        work,
        "mission.start",
        "recover-mode",
        Some(profile.clone()),
        Some(1),
    )?;
    healthy(&mut store, &db, &lease, work, "task")?;
    control(
        &mut store,
        work,
        "mission.stop",
        "stop-before-worker",
        None,
        None,
    )?;
    control(
        &mut store,
        work,
        "mission.unpause",
        "unpause-without-consent",
        None,
        None,
    )?;
    denied(
        &mut store,
        &db,
        &lease,
        work,
        "task",
        "mission-consent-required",
    )?;
    control(
        &mut store,
        work,
        "mission.start",
        "recover-consent",
        Some(profile.clone()),
        Some(1),
    )?;
    healthy(&mut store, &db, &lease, work, "task")?;
    let saved = store.read_work(work)?.launch_profile;
    let first = store.prepare_mission_worker(
        &lease,
        &intention(&store, work, "task", "auto-old")?,
        false,
        &mut SystemEnvironment,
    )?;
    assert!(first.created && !first.preview_only);
    // A queued intention does not freeze permission: a new saved threshold
    // must refuse the actual execution gate before any process/provider effect.
    save_cost_policy(&store, "cost-before-grant", Some(200))?;
    let before_gate = snapshot(&store, &db, work)?;
    let gate_error = store
        .run_agent(&first.agent.id)
        .err()
        .ok_or("cost gate unexpectedly released")?;
    let mut cause: Option<&(dyn Error + 'static)> = Some(&gate_error);
    let mut found = false;
    while let Some(e) = cause {
        found |= e.to_string() == "mission-task-reported-cost-threshold";
        cause = e.source();
    }
    assert!(found, "wrong gate refusal: {gate_error:?}");
    assert_eq!(snapshot(&store, &db, work)?, before_gate);
    assert!(!root.join("first/model-received.json").exists());
    save_cost_policy(&store, "cost-before-grant-recovery", None)?;

    assert_eq!(store.read_work(work)?.launch_profile, saved);
    let mut second = profile.clone();
    second["workspace"] = json!("second");
    control(
        &mut store,
        work,
        "mission.start",
        "second-workspace-two-slots",
        Some(second.clone()),
        Some(2),
    )?;
    healthy(&mut store, &db, &lease, work, "task-2")?;
    control(
        &mut store,
        work,
        "mission.start",
        "second-workspace-one-slot",
        Some(second.clone()),
        Some(1),
    )?;
    denied(
        &mut store,
        &db,
        &lease,
        work,
        "task-2",
        "mission-agent-slots-occupied",
    )?;
    control(
        &mut store,
        work,
        "mission.stop",
        "revoke-old-intent",
        None,
        None,
    )?;
    control(
        &mut store,
        work,
        "mission.unpause",
        "show-revoked-intent",
        None,
        None,
    )?;
    assert!(store.run_agent(&first.agent.id).is_err());
    assert_eq!(store.read_agent(&first.agent.id)?.status, "queued");
    control(
        &mut store,
        work,
        "mission.start",
        "new-consent-not-old-permission",
        Some(second),
        Some(1),
    )?;
    assert!(store.run_agent(&first.agent.id).is_err());
    assert_eq!(store.read_agent(&first.agent.id)?.status, "queued");
    store.update_mission_supervision(&lease, &swarm_engine::system_timestamp()?, true)?;
    let replacement = store.claim_mission_supervision(
        work,
        "owned-new-generation",
        &swarm_engine::system_timestamp()?,
    )?;
    denied(
        &mut store,
        &db,
        &lease,
        work,
        "task-2",
        "mission-lease-lost",
    )?;
    assert!(store.run_agent(&first.agent.id).is_err());
    store.stop_agent(&first.agent.id, &mut SystemEnvironment)?;
    let cancelled = store.cancel_queued_agent(&first.agent.id, &mut SystemEnvironment)?;
    assert_eq!(cancelled.status, "interrupted");
    healthy(&mut store, &db, &replacement, work, "task-2")?;
    let saved = store.read_work(work)?.launch_profile;
    let fresh = store.prepare_mission_worker(
        &replacement,
        &intention(&store, work, "task-2", "auto-current")?,
        false,
        &mut SystemEnvironment,
    )?;
    assert_eq!(store.read_work(work)?.launch_profile, saved);
    let actual = store.run_mission_cycle(&replacement, |store, _, monitor| {
        assert!(monitor.is_running());
        store.run_agent(&fresh.agent.id)
    })?;
    assert_eq!(actual.agent.status, "completed");
    assert_eq!(actual.agent.origin, "conducteur");
    assert!(!actual.task_accepted);
    assert!(
        actual.agent.native_execution_result.is_some()
            && root.join("second/model-received.json").is_file()
    );
    let report = fs::read(root.join("second/handoff.md"))?;
    let hash = format!("{:x}", Sha256::digest(&report));
    let r = json!({"schema_version":1,"event_id":"auto-handoff","expected_revision":store.read_work(work)?.revision,"id":"task-2","agent_id":actual.agent.id,"report_path":"second/handoff.md","report_sha256":hash});
    let submitted = store
        .execute_mission_external(
            &replacement,
            "task.submit",
            &serde_json::to_vec(&r)?,
            &mut SystemEnvironment,
        )?
        .result
        .work;
    assert_eq!(
        submitted
            .tasks
            .as_ref()
            .unwrap()
            .iter()
            .find(|t| t.id == "task-2")
            .unwrap()
            .status,
        "submitted"
    );
    let final_profile = store.read_work(work)?.launch_profile;
    assert_eq!(final_profile, saved);
    assert!(original.is_some());
    // Observe real public lifecycle rows. The inherited inert program declares
    // 25 simulated USD plus a 25 USD retained-data fixture above; cancelled
    // intentions report nothing. This is fixture
    // data, not an invoice, price policy or actual provider spend.
    let costs = store.reported_costs(work)?;
    let count = i64::try_from(store.list_agents(work)?.len())?;
    assert!(count > 0);
    assert_eq!(costs.total.reported_usd.0, 50.0);
    assert_eq!(costs.total.attempts_with_cost, 2);
    assert_eq!(costs.total.attempts_without_cost, count - 2);
    assert!(costs.total.attempts_without_cost > 0);
    assert_eq!(costs.by_task["task-2"].reported_usd.0, 25.0);
    assert!(costs.by_task.contains_key("task-2"));
    let lease_generation = replacement.generation();
    replacement.close()?;
    lease.close()?;
    db.close().map_err(|(_, e)| e)?;
    store.close()?;
    println!(
        "{}",
        json!({"status":"PASS","work":work,"generation":lease_generation,"raw_conductor_fields_refused":true,"paused_manual_and_revoked_consent_refused":true,"saved_capacity_enforced":true,"old_intent_not_rearmed":true,"stale_lease_refused":true,"queued_old_intent_cancelled":true,"actual_worker_and_private_gate":true,"reported_costs_unknown_retained":true,"reported_cost_policy_refusal_recovery":true,"reported_cost_pregrant_rechecked":true,"heartbeat_cycle_used":true,"automatic_origin_retained":true,"handoff_submitted":true,"profile_preserved":true,"task_accepted":false,"agent":actual.agent.id,"old_agent":first.agent.id})
    );
    Ok(())
}
