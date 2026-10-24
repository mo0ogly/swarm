//! Global mission departure controls, separate from planner pause and automatic consent.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::io::{self, Write};
/// Read controls/complete task order/recorded workspace queue or apply audited start/resume/pause/unpause/stop.
/// Order is a coherent read-only observation, never eligibility or a dispatch plan.
/// Queue is a coherent read-only observation, never an acquired FIFO turn.
/// Priority is an audited exact ordinal change, preserving all execution controls.
/// Start configures a validated profile and explicit local consent only; resume
/// requires existing consent. Watch delegates to the incremental native conductor;
/// status is a facts-only text view, status-native a distinct versioned JSON DTO.
/// Historical JSON reads supported ordinary/hierarchical facts; missing producers refuse.
/// Control documents are bounded and validated before Root open; common Root
/// owns replay/revision, settings and atomic SQL/Work/event updates. Controls and
/// status do not spawn; delegated watch alone performs admission and bootstrap.
/// Existing mode/consent/consumption and physical ownership remain authoritative.
/// Encode/close/output failures retain cause and known committed Work; caller
/// must read back before recovery. Local operator labels do not authenticate.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let op = a.pos[1].as_str();
    if op == "watch" { return super::mission_watch::execute(a, out); }
    if matches!(op, "status" | "status-native") {
        return super::mission_status::execute(a, out);
    }
    if !matches!(op, "show" | "order" | "queue" | "priority" | "pause" | "unpause" | "stop" | "start" | "resume") {
        return Err(CliError::Usage);
    }
    let input = if matches!(op, "show" | "order" | "queue") {
        if !a.input.is_empty() {
            return Err(CliError::Usage);
        }
        None
    } else {
        if a.input.is_empty() {
            return Err(CliError::Usage);
        }
        Some(
            if a.input == "-" {
                swarm_engine::read_command_document(&mut io::stdin().lock())
            } else {
                swarm_engine::read_command_file(std::path::Path::new(&a.input))
            }
            .map_err(CliError::CommandInput)?,
        )
    };
    let root_error = |e| CliError::Root(Box::new(e));
    let mission_error = |e| {
        root_error(swarm_engine::RootStoreError::Mutation(
            swarm_engine::WorkEngineError::Mission(Box::new(e)),
        ))
    };
    if let Some(raw) = input.as_deref() {
        (if op=="start" {swarm_engine::decode_mission_start_request(raw)} else if op=="priority" {swarm_engine::decode_mission_priority_request(raw)} else {swarm_engine::decode_mission_control_request(raw)}).map_err(mission_error)?;
    }
    let mut store = swarm_engine::RootStore::open(std::path::Path::new(&a.root), !matches!(op, "show" | "order" | "queue"))
        .map_err(root_error)?;
    let mut committed = None;
    let result = (|| match op {
        "queue" => json_bytes(&store.mission_workspace_queue(&a.pos[2]).map_err(mission_error)?).map_err(CliError::Output),
        "order" => json_bytes(&store.mission_task_order(&a.pos[2]).map_err(mission_error)?).map_err(CliError::Output),
        "show" => json_bytes(
            &store
                .mission_control_view(&a.pos[2])
                .map_err(mission_error)?,
        )
        .map_err(CliError::Output),
        "pause" | "unpause" | "stop" | "start" | "resume" | "priority" => {
            let outcome = store
                .execute_external(
                    &a.pos[2],
                    match op {
                        "priority" => "mission.priority",
                        "pause" => "mission.pause",
                        "unpause" => "mission.unpause",
                        "stop" => "mission.stop",
                        "start" => "mission.start",
                        "resume" => "mission.resume",
                        _ => return Err(CliError::Usage),
                    },
                    input.as_deref().ok_or(CliError::Usage)?,
                    &mut swarm_engine::SystemEnvironment,
                )
                .map_err(root_error)?;
            let work = outcome.result.work;
            committed = Some((work.id.clone(), work.revision));
            json_bytes(&work).map_err(CliError::Output)
        }
        _ => Err(CliError::Usage),
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root_error(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if let Some((work, revision)) = committed.or_else(|| primary.recorded_work()) {
            CliError::PlanningStatePreserved {
                primary: Box::new(primary),
                work,
                revision,
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
