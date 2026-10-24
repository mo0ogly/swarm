//! Fresh checks for native manual or proven automatic intentions; no child starts here.
#![cfg(target_os = "linux")]
use crate::{
    agent_launch as launch,
    agent_launch_preflight::{self, HostPreparation},
    local_storage::LocalDirectory,
};
use crate::{Agent, AgentLaunchError, EngineSettingsDocument, Launch};
use rusqlite::{types::ValueRef, Transaction};
use std::path::Path;
/// Internal startup boundary, never supplied as user authority or a JSON flag.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionStage {
    Queued,
    Supervisor,
    Registered,
    Child,
}
/// Fresh observations only, not an execution grant. The caller owns its SQL/
/// settings snapshot and must recheck at registration/permission boundaries.
pub(crate) struct ExecutionPreparation {
    pub agent: Agent,
    pub body: Vec<u8>,
    pub work: crate::Work,
    pub request: Launch,
    pub model: crate::provider_model_route::ModelRoutePreparation,
    /// Values observed in this checked settings snapshot, excluding its history.
    /// The final helper rereads model files/env against these rules before exec;
    /// this is not an enduring settings lease or execution authority.
    pub model_values: crate::EngineSettingsValues,
    pub host: HostPreparation,
    pub launch_policy: crate::AgentLaunchPolicy,
    pub reserved: crate::agent_execution_store::ReservationReceipt,
}
/// Require native body/request and SQL headers to agree in this transaction.
/// Unknown original request content cannot disappear through Go projection and
/// become an executable request. Missing/type/codec/finalization causes survive.
pub(crate) fn retained(
    tx: &Transaction<'_>,
    id: &str,
) -> Result<(Agent, Launch, Vec<u8>), AgentLaunchError> {
    launch::query(
        tx,
        "SELECT body,status,desired,work_id,task_id,cwd,request FROM main.agents WHERE id=?",
        [id],
        |rows| {
            let row = rows
                .next()?
                .ok_or(AgentLaunchError::Refused("agent-execution-missing"))?;
            let bytes = |column| -> Result<Vec<u8>, AgentLaunchError> {
                match row.get_ref(column)? {
                    ValueRef::Blob(v) | ValueRef::Text(v) => Ok(v.to_vec()),
                    _ => Err(AgentLaunchError::Refused("agent-execution-row-type")),
                }
            };
            let body = bytes(0)?;
            let agent = crate::decode_agent(&body)
                .map_err(|e| AgentLaunchError::caused("agent-execution-body-invalid", e))?;
            for (column, expected) in [
                (1, agent.status.as_str()),
                (2, agent.desired.as_str()),
                (3, agent.work_id.as_str()),
                (4, agent.task_id.as_str()),
                (5, agent.workspace.as_str()),
            ] {
                if bytes(column)? != expected.as_bytes() {
                    return Err(AgentLaunchError::Refused("agent-execution-row-conflict"));
                }
            }
            let raw = bytes(6)?;
            let request = crate::decode_launch(&raw)
                .map_err(|e| AgentLaunchError::caused("agent-execution-request-invalid", e))?;
            if crate::encode_model(&request)
                .map_err(|e| AgentLaunchError::caused("agent-execution-request-encoding", e))?
                != raw
                || agent.id != id
                || request.event_id != id
                || request.task_id != agent.task_id
                || request.provider != agent.provider
            {
                return Err(AgentLaunchError::Refused(
                    "agent-execution-request-conflict",
                ));
            }
            Ok((agent, request, body))
        },
    )
}
/// Verify the current kernel relationship appropriate to the internal boundary.
/// Queued/supervisor startup refuses output declarations before permission;
/// a released helper can see a parent-owned heartbeat without inheriting grants.
/// No PID integer alone or retained stamp is accepted as a signal capability.
fn process_boundary(agent: &Agent, stage: ExecutionStage) -> Result<(), AgentLaunchError> {
    let current = crate::SupervisorIdentity::current()
        .map_err(|e| AgentLaunchError::caused("agent-execution-process-identity", e))?;
    if agent.host != current.host() {
        return Err(AgentLaunchError::Refused("agent-execution-other-host"));
    }
    match stage {
        ExecutionStage::Queued => {
            if agent.status != "queued"
                || agent.supervisor_pid != 0
                || agent.child_pid != 0
                || !agent.supervisor_identity.is_empty()
                || !agent.child_identity.is_empty()
                || agent.native_execution.is_some()
                || agent.native_execution_result.is_some()
                || agent.native_provider_facts.is_some()
                || agent.usage.is_some()
                || agent.reported_model.is_some()
            {
                return Err(AgentLaunchError::Refused("agent-execution-already-owned"));
            }
        }
        ExecutionStage::Supervisor => {
            if agent.status != "starting"
                || agent.supervisor_pid != current.pid()
                || agent.supervisor_identity != current.process_stamp()
                || agent.child_pid != 0
                || !agent.child_identity.is_empty()
                || agent.native_execution.is_some()
                || agent.native_execution_result.is_some()
                || agent.native_provider_facts.is_some()
                || agent.usage.is_some()
                || agent.reported_model.is_some()
            {
                return Err(AgentLaunchError::Refused(
                    "agent-execution-supervisor-conflict",
                ));
            }
        }
        ExecutionStage::Registered => {
            if agent.status != "starting"
                || agent.supervisor_pid != current.pid()
                || agent.supervisor_identity != current.process_stamp()
            {
                return Err(AgentLaunchError::Refused(
                    "agent-execution-supervisor-conflict",
                ));
            }
            let child = u32::try_from(agent.child_pid)
                .map_err(|e| AgentLaunchError::caused("agent-execution-child-representation", e))?;
            let child = crate::observe_linux_process(child)
                .map_err(|e| AgentLaunchError::caused("agent-execution-child-identity", e))?;
            if child.parent_pid() != std::process::id()
                || child.process_group_id() != child.pid()
                || child.process_stamp() != agent.child_identity
            {
                return Err(AgentLaunchError::Refused("agent-execution-child-conflict"));
            }
            let raw = agent
                .native_execution
                .as_ref()
                .ok_or(AgentLaunchError::Refused(
                    "agent-execution-snapshot-missing",
                ))?;
            let snapshot = crate::decode_agent_execution_snapshot(raw.0.get().as_bytes())
                .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
            if snapshot.phase != crate::AgentExecutionPhase::AwaitingPermission
                || agent.native_execution_result.is_some()
            {
                return Err(AgentLaunchError::Refused(
                    "agent-execution-permission-conflict",
                ));
            }
        }
        ExecutionStage::Child => {
            let actual = crate::observe_linux_process(std::process::id())
                .map_err(|e| AgentLaunchError::caused("agent-execution-process-identity", e))?;
            let parent = u32::try_from(agent.supervisor_pid).map_err(|e| {
                AgentLaunchError::caused("agent-execution-parent-representation", e)
            })?;
            let parent = crate::observe_linux_process(parent)
                .map_err(|e| AgentLaunchError::caused("agent-execution-parent-identity", e))?;
            if !matches!(agent.status.as_str(), "starting" | "running")
                || agent.child_pid != current.pid()
                || agent.child_identity != current.process_stamp()
                || actual.process_group_id() != actual.pid()
                || actual.parent_pid() != parent.pid()
                || parent.process_stamp() != agent.supervisor_identity
            {
                return Err(AgentLaunchError::Refused("agent-execution-child-conflict"));
            }
            let raw = agent
                .native_execution
                .as_ref()
                .ok_or(AgentLaunchError::Refused(
                    "agent-execution-snapshot-missing",
                ))?;
            let snapshot = crate::decode_agent_execution_snapshot(raw.0.get().as_bytes())
                .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
            if snapshot.phase != crate::AgentExecutionPhase::PermissionRecorded
                || agent.native_execution_result.is_some()
                || !agent.ended.is_empty()
                || agent.exit_code.is_some()
                || !agent.stop_kind.is_empty()
            {
                return Err(AgentLaunchError::Refused(
                    "agent-execution-permission-not-recorded",
                ));
            }
        }
    }
    Ok(())
}
/// Revalidate one native manual or canonically proven automatic intention.
/// Automatic origins require historical command/claim linkage and CURRENT mission
/// consent/lease/profile/capacity before grant; a supplied label cannot select it.
/// Checks run under the caller's current SQL/settings
/// snapshot. Checks current work/attempt, headers, policy, guards, reserved budget,
/// context/workflow, provider/environment/workspace/model-source bytes and original TTL.
/// Task choices are rebound from current Work at every process boundary; their
/// selected route, stored requested route and unique source receipt must match fresh resolution;
/// this proves neither the applied model nor account availability or pricing.
/// Automatic authority requires Binding4 matching this actual directory and
/// connected database, with canonical provenance checked in the same snapshot.
/// Every newly permitted native child also requires its immutable permission-time
/// storage provenance; missing legacy metadata cannot authorize a fresh exec.
/// Frozen run/guard limits are consumed as frozen, never reset or normalized from
/// current defaults. Host observations are compatibility only, not provider or
/// actor authentication. May make/remove a temporary write probe, starts no child.
/// Every supplied-data/kernel/IO/SQL/codec failure keeps its typed safe cause.
pub(crate) fn inspect(
    tx: &Transaction<'_>,
    root: &Path,
    dir: &LocalDirectory,
    settings: &EngineSettingsDocument,
    id: &str,
    stage: ExecutionStage,
) -> Result<ExecutionPreparation, AgentLaunchError> {
    let (agent, mut request, body) = retained(tx, id)?;
    let mission=crate::root_store::mission_agent::inspect(tx,&agent,settings,crate::root_store::mission_agent::Check::BeforeGrant(dir))?;
    launch::supported_branch(&request,mission.as_ref().map(|m|m.conductor.as_str()))?;
    if (agent.origin != "operateur" && mission.is_none())
        || agent.role != "worker"
        || !agent.mode.is_empty()
        || !agent.parent.is_empty()
        || !agent.previous.is_empty()
        || agent.brainstorm
        || agent.prepared_context.is_some()
        || agent.dialogue_restore.is_some()
    {
        return Err(AgentLaunchError::Refused("agent-execution-branch-unported"));
    }
    if !agent.desired.is_empty() {
        return Err(AgentLaunchError::Refused(
            "agent-execution-stop-or-unknown-desired",
        ));
    }
    process_boundary(&agent, stage)?;
    if stage==ExecutionStage::Child { crate::root_store::agent_execution_origin::require(tx,dir,id)?; }
    launch::execution_guards(tx, &agent.work_id, &request, &agent)?;
    let work = crate::read_recorded_work(tx, &agent.work_id)
        .map_err(|e| AgentLaunchError::caused("agent-execution-work-invalid", e))?;
    let expected = request
        .expected_revision
        .checked_add(1)
        .ok_or(AgentLaunchError::Refused(
            "agent-execution-revision-overflow",
        ))?;
    if work.revision != expected {
        return Err(AgentLaunchError::Refused("agent-execution-context-stale"));
    }
    let index = work
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .position(|t| t.id == agent.task_id)
        .ok_or(AgentLaunchError::Refused("agent-execution-task-missing"))?;
    crate::task_model::bind_launch(&work, &mut request)?;
    launch::supported_work(&work, index)?;
    let task = &work
        .tasks
        .as_ref()
        .ok_or(AgentLaunchError::Refused("agent-execution-task-missing"))?[index];
    if task.status != "running"
        || task.owner != agent.provider
        || !task
            .attempts
            .as_ref()
            .and_then(|a| a.last())
            .is_some_and(|a| {
                a.id == agent.attempt_id && a.status == "recorded" && a.ended.is_empty()
            })
    {
        return Err(AgentLaunchError::Refused("agent-execution-attempt-stale"));
    }
    crate::RootStore::verify_native_retry_execution(root, &work, &agent.task_id, settings)
        .map_err(|e| AgentLaunchError::caused("native-retry-execution-evidence", e))?;
    crate::check_reserved_agent_budget(tx, &agent.work_id, &agent.id)
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let snapshot = agent
        .native_tool_guard_policy
        .as_ref()
        .ok_or(AgentLaunchError::Refused("agent-guard-snapshot-missing"))?;
    crate::AgentToolGuard::new(agent.limits.clone(), snapshot)
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let policy = settings
        .values
        .agent_launch_policy()
        .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
    crate::agent_reviewer_preflight::check(root, dir, work.planning.as_deref(), &policy)?;
    if policy
        .context_bytes
        .is_some_and(|cap| agent.prompt.len() > cap)
        || agent.timeout_seconds < 1
        || policy
            .timeout_cap
            .is_some_and(|cap| agent.timeout_seconds > cap)
    {
        return Err(AgentLaunchError::Refused(
            "agent-execution-context-outside-policy",
        ));
    }
    let context = agent
        .context
        .as_ref()
        .ok_or(AgentLaunchError::Refused("agent-execution-context-missing"))?;
    if context.sha256 != agent_launch_preflight::hash(agent.prompt.as_bytes())
        || i64::try_from(agent.prompt.len()).ok() != Some(context.bytes)
    {
        return Err(AgentLaunchError::Refused(
            "agent-execution-context-mismatch",
        ));
    }
    launch::field_guide(root)?;
    let (source_provider, provider_raw) = launch::provider(dir, &request, &policy)?;
    launch::cooldown(root, &agent.provider)?;
    let model = crate::provider_model_route::resolve(
        &source_provider,
        &request,
        &settings.values,
        &policy,
    )?;
    crate::task_model::check_resolved(&work, &agent.task_id, model.route())?;
    if agent.model_route.as_deref() != model.route() {
        return Err(AgentLaunchError::Refused(
            "agent-execution-model-route-changed",
        ));
    }
    let provider = model.provider();
    let host = agent_launch_preflight::observe(root, &agent.provider, &model, &request, &policy)?;
    let original = agent.preflight.as_ref().ok_or(AgentLaunchError::Refused(
        "agent-execution-preflight-missing",
    ))?;
    model.check_recorded_scope(original)?;
    let now = crate::system_timestamp()
        .map_err(|e| AgentLaunchError::caused("agent-execution-clock", e))?;
    let parse = |raw: &str| {
        swarm_wire_json::go_rfc3339::parse(raw)
            .map_err(|e| AgentLaunchError::caused("agent-execution-clock", e))
    };
    if parse(&now)? >= parse(&original.valid_until)? {
        return Err(AgentLaunchError::Refused(
            "agent-execution-preflight-expired",
        ));
    }
    if host.preflight.scope != original.scope
        || host.workspace.to_str() != Some(agent.workspace.as_str())
        || provider.command != agent.command
        || provider.args != agent.args
        || agent.env_allow.as_deref() != Some(host.environment_names.as_slice())
    {
        return Err(AgentLaunchError::Refused(
            "agent-execution-preflight-changed",
        ));
    }
    let workflow = crate::read_agent_workflow(
        root,
        "worker",
        settings
            .values
            .agent_workflow_body_limit()
            .map_err(|e| AgentLaunchError::caused(e.reason(), e))?,
        &settings
            .values
            .context_policy_limits()
            .map_err(|e| AgentLaunchError::caused(e.reason(), e))?,
    )
    .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    if agent.workflow.as_deref() != Some(&workflow.workflow)
        || !agent.prompt.starts_with(&workflow.prompt)
    {
        return Err(AgentLaunchError::Refused(
            "agent-execution-workflow-changed",
        ));
    }
    if let Some(raw) = &agent.native_execution {
        let snapshot = crate::decode_agent_execution_snapshot(raw.0.get().as_bytes())
            .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        if snapshot.work_revision != work.revision || snapshot.context_sha256 != context.sha256 {
            return Err(AgentLaunchError::Refused(
                "agent-execution-snapshot-context-conflict",
            ));
        }
    }
    let (_, current_provider_raw) = launch::provider(dir, &request, &policy)?;
    if current_provider_raw != provider_raw {
        return Err(AgentLaunchError::Refused(
            "agent-provider-catalogue-changed",
        ));
    }
    model.recheck(&request, &settings.values, &policy)?;
    agent_launch_preflight::recheck(root, &model, &request, &policy, &host)?;
    Ok(ExecutionPreparation {
        agent,
        body,
        work,
        request,
        model,
        model_values: settings.values.clone(),
        host,
        launch_policy: policy,
        reserved: crate::agent_execution_store::reservation(tx, id)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The actual full queued preflight passes before each injection. Supplying
    /// output observations before permission refuses without mutation; restoring
    /// only the isolated fixture bytes permits the same intention to recover.
    /// No child, extra attempt, budget reset or live database is involved.
    #[test]
    fn preflight_output_claims_before_permission_refuse_and_recover() {
        use std::os::unix::fs::DirBuilderExt;
        let root = std::env::temp_dir().join(crate::generate_identifier("facts79-").unwrap());
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap();
        eprintln!("owned prestart metadata fixture: {}", root.display());
        let mut store =
            crate::RootStore::initialize(root.as_path(), &mut crate::SystemEnvironment).unwrap();
        let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Title","objective":"Goal","scope":"Fixture","criteria":["Proof"]}"#,&mut crate::SystemEnvironment).unwrap().result.work.id;
        store.execute_external(&work,"task.add",br#"{"schema_version":1,"event_id":"add","expected_revision":1,"id":"task","title":"Task","deliverable":"Report","criteria":["Proof"]}"#,&mut crate::SystemEnvironment).unwrap();
        let setup = LocalDirectory::open(&root.join(".swarm")).unwrap();
        setup
            .publish(
                "providers.json",
                br#"{"schema_version":1,"providers":{"fixture":{"command":"/bin/cat","args":[]}}}"#,
            )
            .unwrap();
        setup.close().unwrap();
        let request=crate::decode_launch(br#"{"schema_version":1,"event_id":"session","expected_revision":2,"task_id":"task","provider":"fixture"}"#).unwrap();
        store
            .prepare_agent_launch(&work, &request, false, &mut crate::SystemEnvironment)
            .unwrap();
        let settings = store.settings().unwrap();
        store.close().unwrap();
        let directory = LocalDirectory::open(&root.as_path().join(".swarm")).unwrap();
        let mut db = rusqlite::Connection::open(root.as_path().join(".swarm/state.db")).unwrap();
        let original: Vec<u8> = db
            .query_row(
                "SELECT CAST(body AS BLOB) FROM agents WHERE id='session'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let work_before: Vec<u8> = db
            .query_row(
                "SELECT CAST(body AS BLOB) FROM works WHERE id=?",
                [&work],
                |r| r.get(0),
            )
            .unwrap();
        let logs_before: i64 = db
            .query_row("SELECT count(*) FROM agent_logs", [], |r| r.get(0))
            .unwrap();
        for (key, value) in [
            (
                "usage",
                serde_json::json!({"input_tokens":0,"output_tokens":0,"source":"claimed","scope":"claimed","at":"2026-10-19T00:00:00Z"}),
            ),
            (
                "reported_model",
                serde_json::json!({"model":"claimed","source":"claimed","at":"2026-10-19T00:00:00Z"}),
            ),
            (
                "native_provider_facts",
                serde_json::json!({"schema_version":1,"source":"claimed","usage_state":"observed"}),
            ),
        ] {
            let tx = db.transaction().unwrap();
            assert!(inspect(
                &tx,
                root.as_path(),
                &directory,
                &settings,
                "session",
                ExecutionStage::Queued
            )
            .is_ok());
            tx.rollback().unwrap();
            let mut body: serde_json::Value = serde_json::from_slice(&original).unwrap();
            body[key] = value;
            let injected = serde_json::to_vec(&body).unwrap();
            db.execute("UPDATE agents SET body=? WHERE id='session'", [&injected])
                .unwrap();
            let tx = db.transaction().unwrap();
            let result = inspect(
                &tx,
                root.as_path(),
                &directory,
                &settings,
                "session",
                ExecutionStage::Queued,
            );
            assert!(
                matches!(
                    result,
                    Err(AgentLaunchError::Refused("agent-execution-already-owned"))
                ),
                "{key}"
            );
            tx.rollback().unwrap();
            let retained: Vec<u8> = db
                .query_row(
                    "SELECT CAST(body AS BLOB) FROM agents WHERE id='session'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(retained, injected);
            let work_after: Vec<u8> = db
                .query_row(
                    "SELECT CAST(body AS BLOB) FROM works WHERE id=?",
                    [&work],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(work_after, work_before);
            let logs_after: i64 = db
                .query_row("SELECT count(*) FROM agent_logs", [], |r| r.get(0))
                .unwrap();
            assert_eq!(logs_after, logs_before);
            db.execute("UPDATE agents SET body=? WHERE id='session'", [&original])
                .unwrap();
            let tx = db.transaction().unwrap();
            assert!(inspect(
                &tx,
                root.as_path(),
                &directory,
                &settings,
                "session",
                ExecutionStage::Queued
            )
            .is_ok());
            tx.rollback().unwrap();
        }
        db.close().unwrap();
        directory.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
