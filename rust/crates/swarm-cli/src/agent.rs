//! Stored-agent observation, durable stop and unregistered-launch cancellation.
use super::{json_bytes, write_output, Arguments, CliError};
use std::{io::Write, path::Path};

/// Execute show/stop/reconcile/logs and declared guard replay through native RootStore.
/// Strict argument checks precede opening; show never infers a live process from
/// stored PID/heartbeat. Dialogue boundaries remain unsupported; manual preview/prepare dispatch below.
/// Stop records intent only; reconcile cancels an unregistered manual attempt or,
/// on Linux, replays a durable completion after its original supervisor departs.
/// Missing/stale/incomplete proof refuses; no provider is rerun or task accepted. Close/write/flush failures retain causes
/// and distinguish already-recorded lifecycle changes from failed output.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    #[cfg(target_os = "linux")]
    if a.pos.get(1).is_some_and(|v| v == "run") {
        return run(a, out);
    }
    #[cfg(target_os = "linux")]
    if a.pos
        .get(1)
        .is_some_and(|v| matches!(v.as_str(), "preview" | "prepare"))
    {
        return prepare(a, out);
    }
    if a.pos.get(1).is_some_and(|v| v == "logs") {
        return super::agent_logs::execute(a, out);
    }
    if a.pos.get(1).is_some_and(|v| v == "guard") {
        return guard(a, out);
    }
    if a.pos.get(1).is_some_and(|v| v == "limits") {
        return limits(a, out);
    }
    if a.pos.len() != 3
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let operation = a.pos[1].as_str();
    if !matches!(operation, "show" | "stop" | "reconcile") {
        return Err(CliError::Unsupported);
    }
    let stop = operation == "stop";
    let cancel = operation == "reconcile";
    let root_error = |e| CliError::Root(Box::new(e));
    let mut store =
        swarm_engine::RootStore::open(Path::new(&a.root), stop || cancel).map_err(root_error)?;
    let mut recorded = false;
    let result = (|| {
        let agent = if stop {
            let agent = store
                .stop_agent(&a.pos[2], &mut swarm_engine::SystemEnvironment)
                .map_err(root_error)?;
            recorded = true;
            agent
        } else if cancel {
            #[cfg(target_os = "linux")]
            let agent = store
                .reconcile_agent(&a.pos[2], &mut swarm_engine::SystemEnvironment)
                .map_err(root_error)?;
            #[cfg(not(target_os = "linux"))]
            let agent = store
                .cancel_queued_agent(&a.pos[2], &mut swarm_engine::SystemEnvironment)
                .map_err(root_error)?;
            recorded = true;
            agent
        } else {
            let agent = store.read_agent(&a.pos[2]).map_err(root_error)?;
            if agent.mode == "dialogue" {
                return Err(CliError::Unsupported);
            }
            agent
        };
        if stop {
            if a.json {
                json_bytes(
                    &serde_json::json!({"desired":agent.desired,"termination_confirmed":false}),
                )
                .map_err(CliError::Output)
            } else {
                Ok(if a.language == "en" {
                    b"Stop requested; termination has not been confirmed.\n".to_vec()
                } else {
                    "Arrêt demandé ; fin du processus non confirmée.\n"
                        .as_bytes()
                        .to_vec()
                })
            }
        } else if cancel {
            if a.json {
                json_bytes(&serde_json::json!({"agent":agent,"cancelled_before_start":agent.supervisor_pid==0,"completion_recovered":agent.supervisor_pid!=0,"accepted_by_cancellation":false})).map_err(CliError::Output)
            } else if agent.supervisor_pid != 0 {
                Ok(if a.language == "en" {
                    b"Native completion reconciled; the task still requires validation.\n".to_vec()
                } else {
                    "Fin native réconciliée ; la tâche exige encore une validation.\n"
                        .as_bytes()
                        .to_vec()
                })
            } else {
                Ok(if a.language == "en" {
                    b"Unregistered launch cancelled; cancellation does not accept the task.\n"
                        .to_vec()
                } else {
                    "Lancement non pris en charge annulé ; l’annulation ne valide pas la tâche.\n"
                        .as_bytes()
                        .to_vec()
                })
            }
        } else {
            let now = swarm_engine::system_timestamp()
                .map_err(|e| root_error(swarm_engine::RootStoreError::Runtime(e)))?;
            let heartbeat = swarm_engine::observe_agent_heartbeat(&agent, &now)
                .map_err(|e| root_error(swarm_engine::RootStoreError::Launch(Box::new(e))))?;
            let observed = if !agent.is_active() {
                agent.status.as_str()
            } else if heartbeat.is_some() {
                "native-heartbeat/process-liveness-unverified"
            } else {
                "unknown/native-supervision-unported"
            };
            json_bytes(&serde_json::json!({"agent":agent,"desired":agent.desired,"observed_status":observed,"supervision_heartbeat":heartbeat}))
                .map_err(CliError::Output)
        }
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(root_error(error)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if recorded && cancel {
            CliError::AgentCancellationPreserved {
                primary: Box::new(primary),
                english: a.language == "en",
            }
        } else if recorded {
            CliError::AgentStopPreserved {
                primary: Box::new(primary),
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}

/// Read a strict fixture/diagnostic preview and current saved Admin values.
/// Actual RootStore resolves scope overlays; output errors stay errors and every
/// path closes the store. This grants no launch or previous-attempt authenticity.
fn limits(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 5
        || a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let raw = if a.input == "-" {
        swarm_engine::read_command_document(&mut std::io::stdin().lock())
    } else {
        swarm_engine::read_command_file(Path::new(&a.input))
    }
    .map_err(CliError::CommandInput)?;
    let input = swarm_engine::decode_runtime_limit_preview(&raw)
        .map_err(|e| CliError::Root(Box::new(swarm_engine::RootStoreError::RunLimitsBody(e))))?;
    let mut store = swarm_engine::RootStore::open(Path::new(&a.root), false)
        .map_err(|e| CliError::Root(Box::new(e)))?;
    let result = store
        .preview_agent_limits(&a.pos[2], &a.pos[3], &a.pos[4], &input)
        .map_err(|e| CliError::Root(Box::new(e)))
        .and_then(|value| {
            json_bytes(
                &serde_json::json!({"limits":value,"preview_only":true,"launch_authorized":false}),
            )
            .map_err(CliError::Output)
        });
    match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(CliError::Root(Box::new(e))),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}

/// Prepare a strict manual launch through the native factory, without starting
/// a child. Preview rolls SQL back; durable admission survives output/flush/close
/// failure and returns a specific diagnostic for reread/replay. Invalid input
/// fails before opening the root; original typed causes and cleanup are retained.
#[cfg(target_os = "linux")]
fn prepare(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let raw = if a.input == "-" {
        swarm_engine::read_command_document(&mut std::io::stdin().lock())
    } else {
        swarm_engine::read_command_file(Path::new(&a.input))
    }
    .map_err(CliError::CommandInput)?;
    let r = swarm_engine::decode_launch(&raw).map_err(|e| {
        CliError::Root(Box::new(swarm_engine::RootStoreError::Agent(
            swarm_engine::AgentStoreError::Body(e),
        )))
    })?;
    let preview = a.pos[1] == "preview";
    let mut store = swarm_engine::RootStore::open(Path::new(&a.root), true)
        .map_err(|e| CliError::Root(Box::new(e)))?;
    let mut recorded = false;
    let result=store.prepare_agent_launch(&a.pos[2],&r,preview,&mut swarm_engine::SystemEnvironment).map_err(|e|CliError::Root(Box::new(e))).and_then(|value| {
        recorded = !preview;
        if a.json {json_bytes(&serde_json::json!({"agent":value.agent,"created":value.created,"preview_only":value.preview_only,"process_started":false})).map_err(CliError::Output)}
        else {Ok(if a.language=="en" {if preview {b"Launch preview; no intention recorded or process started.\n".to_vec()}else{b"Launch intention recorded; no process started.\n".to_vec()}}else if preview {"Préparation simulée ; aucune intention enregistrée ni processus démarré.\n".as_bytes().to_vec()}else{"Intention enregistrée ; aucun processus démarré.\n".as_bytes().to_vec()})}
    });
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(CliError::Root(Box::new(e))),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if recorded {
            CliError::AgentLaunchPreserved {
                primary: Box::new(primary),
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}

/// Read-only declared-event diagnostic for a stored agent. Strict transport/JSON
/// checks precede opening; RootStore checks row consistency and frozen policy.
/// No caller trace is authority, execution proof or stop command. Every outcome
/// explicitly closes the store; primary/cleanup and output/flush failures remain
/// failures, with no new lifecycle state or consumed budget to undo.
fn guard(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let raw = if a.input == "-" {
        swarm_engine::read_command_document(&mut std::io::stdin().lock())
    } else {
        swarm_engine::read_command_file(Path::new(&a.input))
    }
    .map_err(CliError::CommandInput)?;
    let root_error = |e| CliError::Root(Box::new(e));
    let trace = swarm_engine::decode_agent_guard_trace(&raw)
        .map_err(|e| root_error(swarm_engine::RootStoreError::Guard(e)))?;
    let mut store = swarm_engine::RootStore::open(Path::new(&a.root), false).map_err(root_error)?;
    let result = store
        .replay_agent_guard(&a.pos[2], &trace)
        .map_err(root_error)
        .and_then(|v| json_bytes(&v).map_err(CliError::Output));
    match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root_error(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}

/// Execute foreground native manual supervision and report its settled Agent.
/// Exit is never task acceptance. Output/close failures preserve the fact that
/// lifecycle effects may already be durable; caller must reread before replay.
#[cfg(target_os = "linux")]
fn run(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let mut store = swarm_engine::RootStore::open(Path::new(&a.root), true)
        .map_err(|e| CliError::Root(Box::new(e)))?;
    let result = store.run_agent(&a.pos[2]);
    let settled = result.is_ok();
    let result = result
        .map_err(|e| CliError::Root(Box::new(e)))
        .and_then(|v| json_bytes(&v).map_err(CliError::Output));
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(CliError::Root(Box::new(e))),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    // The original CLI error chain remains intact. A settled process must never
    // be mistaken for a failed preparation which can be blindly relaunched.
    result.map(|()| 0).map_err(|primary| {
        if settled {
            CliError::AgentExecutionPreserved {
                primary: Box::new(primary),
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
