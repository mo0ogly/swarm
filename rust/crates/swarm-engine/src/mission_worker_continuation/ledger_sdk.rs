//! Explicit integration test on an owned, fresh public planning fixture. SQL is
//! read-only instrumentation on the actual Root connection, never admission.
use super::{MissionLease, MissionWorkerContinuationState, RootStore};
use crate::{AgentLaunchError, MissionWorkerDispatchOutcome, SystemEnvironment};
use rusqlite::types::ValueRef;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{error::Error, fmt, fs, io::Write, os::unix::fs::MetadataExt, path::PathBuf};
type TestResult<T> = Result<T, Box<dyn Error>>;

/// Independent typed failures remain owned, including cleanup/publication.
/// JSON/Debug use safe_diagnostics for every branch; source alone exposes only
/// the first. Diagnostics neither claim rollback nor authorize a retry.
struct Failures(Vec<(&'static str, Box<dyn Error>)>);
impl fmt::Debug for Failures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LedgerFailures")
            .field(
                "phases",
                &self.0.iter().map(|(phase, _)| phase).collect::<Vec<_>>(),
            )
            .field("diagnostics", &safe_diagnostics(self))
            .finish()
    }
}
impl fmt::Display for Failures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ledger fixture failed in {} retained phases",
            self.0.len()
        )
    }
}
impl Error for Failures {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|(_, e)| e.as_ref())
    }
}
/// Render all retained failure branches as safe native codes and numeric
/// observations, alongside fixture phases/trace. Never format private Error
/// messages; the trace keeps its existing explicit fixture-only scope.
fn failure_document(failures: &Failures, trace: &Value) -> Value {
    json!({"status":"FAIL","phases":failures.0.iter().map(|(p, _)| p).collect::<Vec<_>>(),"diagnostics":safe_diagnostics(failures),"mutation_absence_claimed":false,"partial_observation":trace})
}
/// Extend the existing native error links with fixture-owned compound wrappers.
/// All local children, including boxed wrappers, are visited; native families
/// keep their established primary/cleanup and custom-IO traversal. Paths contain
/// only closed branch names, never fixture IDs, phases or Error Display text.
/// Typed pointer identity terminates shared/cyclic graphs without a depth/count
/// policy. Unknown types keep native source traversal and its documented limits.
fn safe_diagnostics(
    error: &(dyn Error + 'static),
) -> Vec<crate::native_diagnostics::NativeGateCause> {
    let mut pending = vec![(vec!["root".into()], error)];
    let mut seen: Vec<*const dyn Error> = Vec::new();
    let mut result = Vec::new();
    while let Some((path, error)) = pending.pop() {
        let pointer = error as *const dyn Error;
        if seen.iter().any(|prior| std::ptr::eq(*prior, pointer)) {
            continue;
        }
        seen.push(pointer);
        let mut links = crate::native_error_links(error);
        if let Some(failures) = error.downcast_ref::<Failures>() {
            links.labels.push("ledger-fixture-failures".into());
            links.children = failures
                .0
                .iter()
                .enumerate()
                .map(|(index, (_, cause))| {
                    (
                        if index == 0 { "primary" } else { "cleanup" },
                        cause.as_ref(),
                    )
                })
                .collect();
        } else if let Some(dispatch) = error.downcast_ref::<UnexpectedDispatch>() {
            links
                .labels
                .push("ledger-fixture-unexpected-dispatch".into());
            links.children = dispatch
                .causes()
                .into_iter()
                .map(|cause| ("source", cause))
                .collect();
        } else if let Some(boxed) = error.downcast_ref::<Box<Failures>>() {
            links.children = vec![("source", boxed.as_ref())];
        } else if let Some(boxed) = error.downcast_ref::<Box<UnexpectedDispatch>>() {
            links.children = vec![("source", boxed.as_ref())];
        }
        result.push(crate::native_diagnostics::NativeGateCause {
            path: path.clone(),
            labels: links.labels,
            io_errno: links.io_errno,
            sqlite_code: links.sqlite_code,
        });
        for (kind, child) in links.children.into_iter().rev() {
            let mut child_path = path.clone();
            child_path.push(kind.into());
            pending.push((child_path, child));
        }
    }
    result
}
/// Reject a fixture assertion without printing private Root/Agent data.
fn require(valid: bool, code: &'static str) -> TestResult<()> {
    if valid {
        Ok(())
    } else {
        Err(code.into())
    }
}

/// No field conveys product authority. Exact marker correlation and local
/// ownership precede public operations; the harness supplies a READY Work.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: i64,
    kind: String,
    root: PathBuf,
    work: String,
    result: PathBuf,
}
impl Input {
    /// Require explicit JSON, a same-object marker, owned private Root and fresh
    /// owned output directory. Missing/malformed input is a real explicit failure.
    fn read() -> TestResult<Self> {
        let path = std::env::var_os("SWARM_CONTINUATION_LEDGER_FIXTURE")
            .ok_or("ledger-fixture-env-required")?;
        let raw: Value = serde_json::from_slice(&fs::read(PathBuf::from(path))?)?;
        let input: Self = serde_json::from_value(raw.clone())?;
        require(
            input.schema_version == 1 && input.kind == "owned-inert-continuation-ledger-fixture",
            "ledger-fixture-kind",
        )?;
        require(
            input.root.is_absolute() && input.root.canonicalize()? == input.root,
            "ledger-root-canonical-required",
        )?;
        let meta = fs::symlink_metadata(&input.root)?;
        require(
            meta.is_dir()
                && meta.uid() == nix::unistd::geteuid().as_raw()
                && meta.mode() & 0o077 == 0,
            "ledger-root-private-owned-required",
        )?;
        let marker: Value = serde_json::from_slice(&fs::read(
            input
                .root
                .join(".swarm/verification-continuation-ledger.json"),
        )?)?;
        require(marker == raw, "ledger-marker-mismatch")?;
        require(
            !input.work.is_empty() && input.result.is_absolute() && !input.result.exists(),
            "ledger-output-fresh-required",
        )?;
        let parent = input.result.parent().ok_or("ledger-output-parent")?;
        let meta = fs::symlink_metadata(parent)?;
        require(
            parent.canonicalize()? == parent
                && meta.is_dir()
                && meta.uid() == nix::unistd::geteuid().as_raw(),
            "ledger-output-owned-parent-required",
        )?;
        require(
            !input.root.join("model-received.json").exists(),
            "ledger-fixture-worker-already-entered",
        )?;
        Ok(input)
    }
}

/// SQLite representation is part of equality: REAL uses exact IEEE bits,
/// INTEGER stays distinct, text/blob bytes and duplicate rows are not collapsed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum Cell {
    Null,
    Integer(i64),
    Real(u64),
    Text(Vec<u8>),
    Blob(Vec<u8>),
}
/// Preserve every SQLite value without coercion; NaN/Infinity refuse the fixture.
fn cell(value: ValueRef<'_>) -> Result<Cell, AgentLaunchError> {
    Ok(match value {
        ValueRef::Null => Cell::Null,
        ValueRef::Integer(n) => Cell::Integer(n),
        ValueRef::Real(n) if n.is_finite() => Cell::Real(n.to_bits()),
        ValueRef::Real(_) => return Err(AgentLaunchError::Refused("ledger-nonfinite-amount")),
        ValueRef::Text(s) => Cell::Text(s.to_vec()),
        ValueRef::Blob(s) => Cell::Blob(s.to_vec()),
    })
}
type Ledger = Vec<[Cell; 4]>;
/// Read ALL reservation rows on this exact connection, preserving types and
/// multiplicity. Shared query closes its statement and retains primary/finalize
/// failures. No transaction wraps the observer or remains open after this read.
fn ledger(store: &RootStore) -> TestResult<Ledger> {
    require(
        store.db.is_autocommit(),
        "ledger-read-inside-external-transaction",
    )?;
    Ok(crate::agent_launch::query(
        &store.db,
        "SELECT agent_id,work_id,amount,state FROM main.reservations",
        [],
        |rows| {
            let mut result = Vec::new();
            while let Some(row) = rows.next()? {
                let cells = [
                    cell(row.get_ref(0)?)?,
                    cell(row.get_ref(1)?)?,
                    cell(row.get_ref(2)?)?,
                    cell(row.get_ref(3)?)?,
                ];
                if !matches!(&cells[0], Cell::Text(s) if !s.is_empty())
                    || !matches!(&cells[1], Cell::Text(s) if !s.is_empty())
                    || !matches!(&cells[2], Cell::Real(_) | Cell::Integer(_))
                    || !matches!(&cells[3], Cell::Text(s) if !s.is_empty())
                {
                    return Err(AgentLaunchError::Refused("ledger-column-type-invalid"));
                }
                result.push(cells);
            }
            result.sort();
            Ok(result)
        },
    )?)
}
/// Adjacent sequential projections, not a distributed transaction or a promise
/// against hostile writers. Raw Agent/Work bytes are compared, never published.
#[derive(PartialEq, Eq)]
struct Snapshot {
    ledger: Ledger,
    agent: Vec<u8>,
    work: Vec<u8>,
    receipts: Vec<crate::MissionWorkReceipt>,
    queue: crate::MissionWorkspaceQueue,
    total_changes: u64,
}
impl Snapshot {
    /// Compare the same Root's complete ledger and existing public projections;
    /// every reader/finalizer error fails the test with its original cause.
    fn read(store: &RootStore, work: &str, agent: &str) -> TestResult<Self> {
        Ok(Self {
            ledger: ledger(store)?,
            agent: serde_json::to_vec(&store.read_agent(agent)?)?,
            work: serde_json::to_vec(&store.read_work(work)?)?,
            receipts: store.read_mission_work_receipts(work)?,
            queue: store.mission_workspace_queue(work)?,
            total_changes: store.db.total_changes(),
        })
    }
}
/// Preserve unexpected dispatch including every queue/resource cause; its
/// presence never becomes successful empty admission or starts a provider.
struct UnexpectedDispatch(MissionWorkerDispatchOutcome);
impl fmt::Debug for UnexpectedDispatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnexpectedLedgerDispatch")
            .field("diagnostics", &safe_diagnostics(self))
            .finish()
    }
}
impl fmt::Display for UnexpectedDispatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
impl UnexpectedDispatch {
    /// Enumerate queue and resource causes without flattening the retained
    /// outcome. Even source() cannot represent all independent branches.
    fn causes(&self) -> Vec<&(dyn Error + 'static)> {
        let (queue, resources) = match &self.0 {
            MissionWorkerDispatchOutcome::Prepared {
                queue, resources, ..
            }
            | MissionWorkerDispatchOutcome::NoAdmission { queue, resources } => {
                (Some(queue), resources)
            }
            MissionWorkerDispatchOutcome::Replayed {
                queue, resources, ..
            } => (queue.as_ref(), resources),
        };
        let mut result: Vec<&dyn Error> = Vec::new();
        if let Some(queue) = queue {
            result.extend(queue.permission_refusal.iter().map(|e| e as &dyn Error));
            result.extend(queue.refused.iter().map(|r| &r.error as &dyn Error));
        }
        for resource in resources {
            match resource {
                crate::MissionDispatchResourceRefusal::Occupied { cause, .. }
                | crate::MissionDispatchResourceRefusal::Capacity { cause, .. } => {
                    result.push(cause)
                }
                crate::MissionDispatchResourceRefusal::Fifo { .. } => {}
            }
        }
        result
    }
}
impl Error for UnexpectedDispatch {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.causes().into_iter().next()
    }
}

/// Admit through the exact public mission APIs, then observe twice without a
/// cycle/spawn/provider. Positive reservation and Binding4/current FIFO witnesses
/// precede equality checks. Cleanup is owned by the outer function even on error.
fn exercise(
    store: &mut RootStore,
    input: &Input,
    lease: &mut Option<MissionLease>,
    trace: &mut Value,
) -> TestResult<Value> {
    require(
        store.list_agents(&input.work)?.is_empty(),
        "ledger-fixture-not-fresh-agents",
    )?;
    let budget = store
        .work_budget(&input.work)?
        .ok_or("ledger-budget-missing")?;
    require(
        budget.limit_usd.0.is_finite()
            && budget.limit_usd.0 > 0.0
            && budget.reserve_per_launch_usd.0.is_finite()
            && budget.reserve_per_launch_usd.0 > 0.0,
        "ledger-positive-budget-required",
    )?;
    fs::create_dir(input.root.join("continuation-ledger"))?;
    let start = json!({"schema_version":1,"event_id":"continuation-ledger-start","expected_revision":store.read_work(&input.work)?.revision,
        "reason":"Owned inert continuation ledger fixture","slots":1,"profile":{"provider":"fixture","role":"worker","workspace":"continuation-ledger","level":"standard","capture_output":true}});
    store.execute_external(
        &input.work,
        "mission.start",
        &serde_json::to_vec(&start)?,
        &mut SystemEnvironment,
    )?;
    *lease = Some(store.claim_mission_supervision(
        &input.work,
        "Owned continuation ledger fixture",
        &crate::system_timestamp()?,
    )?);
    let owned = lease.as_ref().ok_or("ledger-lease-missing")?;
    let request = crate::MissionWorkerDispatchRequest {
        schema_version: 1,
        event_id: "continuation-ledger-agent".into(),
        expected_revision: store.read_work(&input.work)?.revision,
    };
    let outcome = store.dispatch_mission_worker(owned, &request, &mut SystemEnvironment)?;
    let intention = match outcome {
        MissionWorkerDispatchOutcome::Prepared {
            intention,
            queue,
            resources,
        } if queue.permission_refusal.is_none()
            && queue.refused.is_empty()
            && resources.is_empty() =>
        {
            intention
        }
        other => return Err(Box::new(UnexpectedDispatch(other))),
    };
    let agent = &intention.agent;
    require(
        intention.created
            && !intention.preview_only
            && agent.status == "queued"
            && agent.supervisor_pid == 0
            && agent.child_pid == 0,
        "ledger-new-queued-required",
    )?;
    let binding: Value = serde_json::from_str(
        agent
            .native_mission
            .as_ref()
            .ok_or("ledger-binding-missing")?
            .0
            .get(),
    )?;
    require(
        binding["schema_version"] == 4 && binding["generation"] == owned.generation(),
        "ledger-binding4-required",
    )?;
    let expected = Some((
        input.work.clone(),
        budget.reserve_per_launch_usd.0,
        "reserved".into(),
    ));
    require(
        crate::agent_execution_store::reservation(&store.db, &agent.id)? == expected,
        "ledger-positive-target-reservation-required",
    )?;
    let mut documents = Vec::new();
    for ordinal in [1, 2] {
        let before = Snapshot::read(store, &input.work, &agent.id)?;
        require(
            before
                .queue
                .turns
                .iter()
                .filter(|t| {
                    t.agent == agent.id
                        && t.work == input.work
                        && t.state == "acquired"
                        && binding["workspace_turn_seq"] == t.seq
                })
                .count()
                == 1,
            "ledger-acquired-fifo-required",
        )?;
        // The ledger read is immediately adjacent to the component call; the
        // complete projection is also retained across these sequential reads.
        let exact_before = ledger(store)?;
        require(
            exact_before
                .iter()
                .filter(|row| {
                    row[0] == Cell::Text(agent.id.as_bytes().to_vec())
                        && row[1] == Cell::Text(input.work.as_bytes().to_vec())
                        && row[2] == Cell::Real(budget.reserve_per_launch_usd.0.to_bits())
                        && row[3] == Cell::Text(b"reserved".to_vec())
                })
                .count()
                == 1,
            "ledger-exact-positive-target-required",
        )?;
        *trace = json!({"phase":"before-observe","ordinal":ordinal,"target_agent_id":agent.id,"target_work_id":input.work,"target_amount_ieee_bits":budget.reserve_per_launch_usd.0.to_bits(),"ledger_before":exact_before,"prior_observations":documents});
        let observed = store.observe_mission_worker_continuation(owned);
        let exact_after = ledger(store);
        if let Ok(after) = &exact_after {
            trace["ledger_after"] = json!(after);
        }
        let observed = match (observed, exact_after) {
            (Ok(observed), Ok(after)) => {
                require(exact_before == after, "ledger-exact-rows-mutated")?;
                observed
            }
            (Err(primary), Err(cleanup)) => {
                return Err(Box::new(Failures(vec![
                    ("observe", primary),
                    ("after-ledger", cleanup),
                ])))
            }
            (Err(error), Ok(_)) => return Err(error),
            (Ok(_), Err(error)) => return Err(error),
        };
        let after = Snapshot::read(store, &input.work, &agent.id)?;
        require(
            before == after && store.db.is_autocommit(),
            "ledger-projections-mutated",
        )?;
        require(
            observed.work == input.work
                && observed.generation == owned.generation()
                && observed.entries.len() == 1,
            "ledger-observation-scope",
        )?;
        let target = &observed.entries[0];
        require(
            target.agent_id == agent.id
                && target.task_id == agent.task_id
                && matches!(&target.state, MissionWorkerContinuationState::QueuedUnregisteredObserved { generation, current_lease_generation: true } if *generation == owned.generation()),
            "ledger-queued-generation-required",
        )?;
        require(
            crate::agent_execution_store::reservation(&store.db, &agent.id)? == expected,
            "ledger-target-reservation-changed",
        )?;
        require(
            !input.root.join("model-received.json").exists(),
            "ledger-provider-marker-present",
        )?;
        documents.push(json!({"ordinal":ordinal,"ledger_before":exact_before,"ledger_after":after.ledger,"all_rows_equal":true,"agent_work_receipts_fifo_equal":true,"target_unique":true,"current_generation":true,"total_changes_unchanged":true,"target_agent_id":agent.id,"target_work_id":input.work,"target_amount_ieee_bits":budget.reserve_per_launch_usd.0.to_bits(),"positive_target_reservation":true}));
    }
    Ok(
        json!({"status":"PASS","observations":documents,"reservation_rows_verified":true,"observations_are_permission":false,"provider_spawn_called":false,"scope":"owned-queued-agent-reservations-same-Root-connection"}),
    )
}

/// Explicitly selected Linux integration test. No fixture means FAIL, never
/// skipped success. Stop/reconcile and lease/Root close happen after observations,
/// on failure too, with every independent error retained; result uses create_new.
#[test]
#[ignore = "requires fresh owned public planning fixture via SWARM_CONTINUATION_LEDGER_FIXTURE"]
fn continuation_keeps_real_reservation_rows() -> TestResult<()> {
    let input = Input::read()?;
    let mut store = RootStore::open(&input.root, true)?;
    let mut lease = None;
    let mut failures = Failures(Vec::new());
    let mut trace = json!({"phase":"setup"});
    let action = exercise(&mut store, &input, &mut lease, &mut trace);
    let document = match action {
        Ok(doc) => Some(doc),
        Err(e) => {
            failures.0.push(("exercise", e));
            None
        }
    };
    // Fresh Work was required before admission. Never clean unrelated Work or
    // signal a recorded process; queued settlement uses the supported public API.
    if lease.is_some() {
        match store.list_agents(&input.work) {
            Ok(agents) => {
                for agent in agents {
                    if agent.supervisor_pid != 0 || agent.child_pid != 0 {
                        failures.0.push((
                            "cleanup-unexpected-process",
                            "ledger-refuses-process-cleanup".into(),
                        ));
                        continue;
                    }
                    if let Err(e) = store.stop_agent(&agent.id, &mut SystemEnvironment) {
                        failures.0.push(("stop-agent", Box::new(e)));
                    }
                    match store.reconcile_agent(&agent.id, &mut SystemEnvironment) {
                        Ok(settled) => {
                            if let Err(e) = require(
                                settled.status == "interrupted"
                                    && settled.desired == "stop"
                                    && !settled.ended.is_empty(),
                                "ledger-cleanup-settlement-not-observed",
                            ) {
                                failures.0.push(("reconcile-state", e));
                            }
                            match crate::agent_execution_store::reservation(&store.db, &agent.id) {
                                Ok(Some((work, amount, state)))
                                    if work == input.work
                                        && amount.is_finite()
                                        && amount > 0.0
                                        && state == "released" => {}
                                Ok(_) => failures.0.push((
                                    "cleanup-reservation-state",
                                    "ledger-cleanup-reservation-not-released".into(),
                                )),
                                Err(e) => {
                                    failures.0.push(("cleanup-reservation-read", Box::new(e)))
                                }
                            }
                        }
                        Err(e) => failures.0.push(("reconcile-agent", Box::new(e))),
                    }
                }
            }
            Err(e) => failures.0.push(("cleanup-list", Box::new(e))),
        }
    }
    if let Some(owned) = lease {
        match crate::system_timestamp() {
            Ok(now) => {
                if let Err(e) = store.update_mission_supervision(&owned, &now, true) {
                    failures.0.push(("release-lease", Box::new(e)));
                }
            }
            Err(e) => failures.0.push(("cleanup-clock", Box::new(e))),
        }
        if let Err(e) = owned.close() {
            failures.0.push(("close-lease", Box::new(e)));
        }
    }
    if let Err(e) = store.close() {
        failures.0.push(("close-root", Box::new(e)));
    }
    let output = if failures.0.is_empty() {
        let mut output = document.ok_or("ledger-result-missing")?;
        output["lease_released"] = json!(true);
        output["lease_closed"] = json!(true);
        output["root_closed"] = json!(true);
        output
    } else {
        failure_document(&failures, &trace)
    };
    let publication = (|| -> TestResult<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&input.result)?;
        file.write_all(&serde_json::to_vec_pretty(&output)?)?;
        file.write_all(b"\n")?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(e) = publication {
        failures.0.push(("publication", e));
    }
    if failures.0.is_empty() {
        Ok(())
    } else {
        Err(Box::new(failures))
    }
}

/// A valid identical witness precedes every counterexample, including equal
/// count/sum owner, amount swap, state and type changes, plus duplicate rows.
/// This validates the comparator only, never attributes mutation to the observer.
#[test]
fn continuation_ledger_equality_keeps_owner_amount_state_types_and_duplicates() {
    let row = |agent: &str, work: &str, amount: Cell, state: &str| {
        [
            Cell::Text(agent.as_bytes().to_vec()),
            Cell::Text(work.as_bytes().to_vec()),
            amount,
            Cell::Text(state.as_bytes().to_vec()),
        ]
    };
    let a = row("a", "w", Cell::Real(1.0_f64.to_bits()), "reserved");
    let b = row("b", "w", Cell::Real(2.0_f64.to_bits()), "reserved");
    let control = vec![a.clone(), b.clone()];
    let variants = vec![
        vec![
            row("a", "other", Cell::Real(1.0_f64.to_bits()), "reserved"),
            b.clone(),
        ],
        vec![
            row("a", "w", Cell::Real(2.0_f64.to_bits()), "reserved"),
            row("b", "w", Cell::Real(1.0_f64.to_bits()), "reserved"),
        ],
        vec![
            row("a", "w", Cell::Real(1.0_f64.to_bits()), "released"),
            b.clone(),
        ],
        vec![row("a", "w", Cell::Integer(1), "reserved"), b.clone()],
        vec![a.clone(), a.clone()],
    ];
    for variant in variants {
        assert_eq!(control, control.clone());
        assert_ne!(control, variant);
    }
    let duplicate_control = vec![a.clone(), a.clone(), b.clone()];
    assert_eq!(duplicate_control, duplicate_control.clone());
    assert_ne!(duplicate_control, vec![a, b.clone(), b]);
}

/// A known native projection is the valid control before testing the external
/// fixture wrapper. Every independent nested/dispatch branch must retain safe
/// codes in JSON and libtest Debug, without messages, prompts or Agent/task IDs.
/// This is pure error rendering; no SQLite connection, fixture or runtime runs.
#[test]
fn continuation_ledger_failure_rendering_keeps_all_safe_error_branches() {
    let sql = |code| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(code),
            Some("PRIVATE-SQL-PROMPT".into()),
        )
    };
    let control = crate::RootStoreError::Cleanup {
        primary: Some(Box::new(crate::RootStoreError::Sql(sql(5)))),
        cleanup: Box::new(crate::RootStoreError::Launch(Box::new(
            AgentLaunchError::caused(
                "ledger-fixture-cleanup",
                std::io::Error::from_raw_os_error(13),
            ),
        ))),
    };
    let control_nodes = crate::native_diagnostics::project(&control);
    assert!(control_nodes.iter().any(|node| node.sqlite_code == Some(5)));
    assert!(control_nodes.iter().any(|node| node.io_errno == Some(13)));
    assert!(control_nodes
        .iter()
        .all(crate::native_diagnostics::NativeGateCause::valid));
    assert!(!serde_json::to_string(&control_nodes)
        .unwrap()
        .contains("PRIVATE"));

    let unexpected = UnexpectedDispatch(MissionWorkerDispatchOutcome::NoAdmission {
        queue: crate::MissionQueueRegistration {
            work: "PRIVATE-WORK-ID".into(),
            revision: 1,
            ready: vec!["PRIVATE-READY-TASK-ID".into()],
            refused: vec![crate::MissionCandidateRefusal {
                task: "PRIVATE-REFUSED-TASK-ID".into(),
                error: crate::MissionSupervisionError::Sql(sql(18)),
            }],
            permission_refusal: Some(crate::MissionSupervisionError::Refused(
                "ledger-fixture-permission-refused",
            )),
        },
        resources: vec![
            crate::MissionDispatchResourceRefusal::Occupied {
                task: "PRIVATE-OCCUPIED-TASK-ID".into(),
                cause: crate::RootStoreError::Sql(sql(14)),
            },
            crate::MissionDispatchResourceRefusal::Capacity {
                task: "PRIVATE-CAPACITY-TASK-ID".into(),
                cause: crate::RootStoreError::Launch(Box::new(AgentLaunchError::caused(
                    "ledger-fixture-resource-io",
                    std::io::Error::from_raw_os_error(28),
                ))),
            },
        ],
    });
    // Each constituent gets a valid direct witness before the local wrapper is
    // introduced, preventing a broken native projector from masking this loss.
    for error in unexpected.causes() {
        let nodes = crate::native_diagnostics::project(error);
        assert!(nodes
            .iter()
            .all(crate::native_diagnostics::NativeGateCause::valid));
        assert!(nodes.iter().any(|node| !node.labels.is_empty()));
        assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE"));
    }
    let failures = Failures(vec![
        ("exercise", Box::new(control)),
        (
            "close-root",
            Box::new(Failures(vec![
                (
                    "after-ledger",
                    Box::new(std::io::Error::other("PRIVATE-IO-MESSAGE")),
                ),
                ("exercise", Box::new(unexpected)),
            ])),
        ),
    ]);
    let output = failure_document(&failures, &json!({"phase":"test"}));
    let debug = format!("{failures:?}");
    let rendered = serde_json::to_string(&output).unwrap();
    assert!(!rendered.contains("PRIVATE"));
    assert!(!debug.contains("PRIVATE"));
    assert_eq!(output["status"], "FAIL");
    assert_eq!(output["mutation_absence_claimed"], false);
    let nodes: Vec<crate::native_diagnostics::NativeGateCause> = serde_json::from_value(
        output
            .get("diagnostics")
            .expect("all safe error branches must reach external JSON")
            .clone(),
    )
    .unwrap();
    assert!(nodes
        .iter()
        .all(crate::native_diagnostics::NativeGateCause::valid));
    for code in [5, 14, 18] {
        assert!(
            nodes.iter().any(|node| node.sqlite_code == Some(code)),
            "missing SQLite branch {code}"
        );
        assert!(
            debug.contains(&format!("sqlite-code:{code}")),
            "libtest lost SQLite branch {code}"
        );
    }
    for code in [13, 28] {
        assert!(
            nodes.iter().any(|node| node.io_errno == Some(code)),
            "missing IO branch {code}"
        );
        assert!(
            debug.contains(&format!("os-errno:{code}")),
            "libtest lost IO branch {code}"
        );
    }
    assert!(nodes
        .iter()
        .any(|node| node.labels.iter().any(|label| label == "io-kind:Other")));
    assert!(nodes.iter().any(|node| node
        .labels
        .iter()
        .any(|label| label == "ledger-fixture-permission-refused")));
    assert!(debug.contains("io-kind:Other"));
    assert!(debug.contains("ledger-fixture-permission-refused"));
}
