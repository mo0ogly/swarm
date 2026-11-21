//! Read-only CLI graph sourced from the recorded Work and conductor observation.
use crate::{json_bytes, write_output, Arguments, CliError};
use serde::Serialize;
use std::{io::Write, path::Path};
use swarm_engine::{RootStore, RootStoreError, Work, ROOT_ORCHESTRATOR_SCOPE};

#[derive(Serialize)]
struct Scope<'a> {
    id: &'a str,
    parent: Option<&'a str>,
    role: &'static str,
    provider: Option<&'a str>,
}

#[derive(Serialize)]
struct RecordedCheck<'a> {
    id: &'a str,
    executed: bool,
    passed: bool,
}

#[derive(Serialize)]
struct Task<'a> {
    id: &'a str,
    title: &'a str,
    scope: Option<&'a str>,
    status: &'a str,
    prerequisites: &'a [String],
    configured_checks: Vec<&'a str>,
    recorded_checks: Vec<RecordedCheck<'a>>,
}

#[derive(Serialize)]
struct Graph<'a> {
    schema_version: u32,
    work: &'a str,
    revision: i64,
    planning_configured: bool,
    conductor_observation: Option<&'static str>,
    reviewer_provider: Option<&'a str>,
    scopes: Vec<Scope<'a>>,
    tasks: Vec<Task<'a>>,
}

fn present(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

/// Project the default root role or the configured scope hierarchy, followed by
/// task prerequisites and recorded control facts. An unconfigured root has no
/// provider or assigned tasks. This projection does not evaluate freshness or
/// publish commands, arbitrary output, private identities or a validation.
fn project<'a>(work: &'a Work, conductor_observation: Option<&'static str>) -> Graph<'a> {
    let planning = work.planning.as_deref();
    let scopes = match planning {
        None => vec![Scope {
            id: ROOT_ORCHESTRATOR_SCOPE,
            parent: None,
            role: "root_planner",
            provider: None,
        }],
        Some(planning) => planning.scopes.iter().flatten().map(|scope| Scope {
            id: &scope.id,
            parent: present(&scope.parent),
            role: if scope.id == ROOT_ORCHESTRATOR_SCOPE && scope.parent.is_empty() {
                "root_planner"
            } else {
                "subplanner"
            },
            provider: scope
                .model_selection
                .as_deref()
                .and_then(|model| present(&model.provider))
                .or_else(|| present(&planning.provider)),
        }).collect(),
    };
    let tasks = work
        .tasks
        .iter()
        .flatten()
        .map(|task| Task {
            id: &task.id,
            title: &task.title,
            scope: present(&task.scope_id),
            status: &task.status,
            prerequisites: task.depends.as_deref().unwrap_or_default(),
            configured_checks: task
                .validation_policy
                .iter()
                .flat_map(|policy| policy.controls.iter().flatten())
                .map(|control| control.id.as_str())
                .collect(),
            recorded_checks: task
                .automatic_validation
                .iter()
                .flat_map(|receipt| receipt.controls.iter().flatten())
                .map(|control| RecordedCheck {
                    id: &control.id,
                    executed: control.executed,
                    passed: control.passed,
                })
                .collect(),
        })
        .collect();
    Graph {
        schema_version: 1,
        work: &work.id,
        revision: work.revision,
        planning_configured: planning.is_some(),
        conductor_observation,
        reviewer_provider: planning
            .and_then(|p| p.reviewer.as_deref())
            .and_then(|reviewer| present(&reviewer.provider)),
        scopes,
        tasks,
    }
}

/// Read the existing project and native conductor lease without obtaining
/// consent, opening a writer or dispatching work. An absent observation stays
/// null. Root read/cleanup and output errors retain their normal causes.
pub(super) fn execute(args: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if args.pos.len() != 3
        || !args.input.is_empty()
        || !args.output.is_empty()
        || args.server
        || !args.task.is_empty()
        || args.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root_error = |e| CliError::Root(Box::new(e));
    let store = RootStore::open(Path::new(&args.root), false).map_err(root_error)?;
    let result = (|| {
        let work = store.read_work(&args.pos[2]).map_err(root_error)?;
        let now =
            swarm_engine::system_timestamp().map_err(|e| root_error(RootStoreError::Runtime(e)))?;
        let observation = store
            .read_mission_supervision(&work.id, &now)
            .map_err(|e| {
                root_error(RootStoreError::Mutation(
                    swarm_engine::WorkEngineError::Mission(Box::new(e)),
                ))
            })?;
        let state = observation
            .as_ref()
            .map(|value| crate::mission_status::supervision_state(value.state));
        json_bytes(&project(&work, state)).map_err(CliError::Output)
    })();
    match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cleanup)) => Err(root_error(cleanup)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
