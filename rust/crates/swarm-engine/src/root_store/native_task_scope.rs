//! Read-only native execution scope observation. A projection is never a grant.
use super::{finished, RootInstanceIdentity, RootStore, RootStoreError};
use crate::NativeTaskScope;
use sha2::{Digest, Sha256};

/// Internal host observation, constructible only after the native broker checks
/// its actual opened Root and currently owned process/task. Not deserializable.
pub(crate) struct NativeExecutionContext {
    scope: NativeTaskScope,
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Synthetic private host context, not kernel/native acceptance. Proves
    /// exclusive source routing and deep persisted-state validation only.
    #[test]
    fn native_mapping_has_no_bearer_fallback_and_disabled_refuses() {
        use crate::trust::{
            authentication::{authenticated, Source},
            state::State,
        };
        let scope = NativeTaskScope {
            root_digest: "a".repeat(64),
            work_id: "work".into(),
            task_id: "task".into(),
            attempt_id: "attempt".into(),
            agent_id: "agent".into(),
        };
        let mut state = State::initial();
        state.identities=crate::trust_types::prepare_identities(serde_json::from_value(
            serde_json::json!([
                {"subject":"admin","token":"synthetic-admin-token","integrity":3,"roles":["admin"]},
                {"subject":"approver","token":"synthetic-approver-token","integrity":2,"roles":["approver"]},
                {"subject":"worker","native_scope":scope,"integrity":2,"roles":["effect"]}
            ])).unwrap()).unwrap();
        let context = NativeExecutionContext {
            scope: scope.clone(),
        };
        assert!(authenticated(&state, Source::Native(&context)).is_ok());
        assert!(authenticated(&state, Source::Local("synthetic-native-token")).is_err());
        state.identities[2].disabled = true;
        assert!(authenticated(&state, Source::Native(&context)).is_err());
        state.identities[2].disabled = false;
        state.identities[2].token_hash = crate::trust_types::token_hash("synthetic-native-token");
        assert!(state.validate().is_err());
        assert!(authenticated(&state, Source::Local("synthetic-native-token")).is_err());
        state.identities[2].token_hash.clear();
        let mut wrong = scope;
        wrong.attempt_id = "other-attempt".into();
        assert!(authenticated(
            &state,
            Source::Native(&NativeExecutionContext { scope: wrong })
        )
        .is_err());
        state.identities.push(state.identities[2].clone());
        assert!(state.validate().is_err());
    }
}
impl NativeExecutionContext {
    /// Borrow the host-observed binding during the held SQL/Trust reservation.
    pub(crate) fn scope(&self) -> &NativeTaskScope {
        &self.scope
    }
}

/// Hash actual physical identities with a versioned domain and fixed-width
/// unsigned encoding; this pure projection alone never authenticates anyone.
fn scope_for(
    instance: &RootInstanceIdentity,
    agent: &crate::Agent,
) -> Result<NativeTaskScope, RootStoreError> {
    let mut digest = Sha256::new();
    digest.update(b"swarm-native-root-instance-v1\0");
    for n in [
        instance.directory.0,
        instance.directory.1,
        instance.database.0,
        instance.database.1,
    ] {
        digest.update(n.to_be_bytes());
    }
    let scope = NativeTaskScope {
        root_digest: format!("{:x}", digest.finalize()),
        work_id: agent.work_id.clone(),
        task_id: agent.task_id.clone(),
        attempt_id: agent.attempt_id.clone(),
        agent_id: agent.id.clone(),
    };
    scope.validate().map_err(RootStoreError::Refused)?;
    Ok(scope)
}

impl RootStore {
    /// Observe whether an explicit enabled mapping selects the native adapter.
    /// This coherent file snapshot is advisory transport selection, not effect
    /// authority: the broker reauthenticates against the current locked journal.
    /// Absence alone means disabled; malformed state/IO failures refuse launch.
    pub(crate) fn native_agent_mapping_enabled(
        &mut self,
        id: &str,
    ) -> Result<bool, RootStoreError> {
        let scope = self.native_agent_scope(id)?;
        let raw = match self
            .directory
            .read_bounded(super::trust_store::STATE, super::trust_store::MAX_STATE)
        {
            Ok(raw) => raw,
            Err(e) if super::absent(&e) => return Ok(false),
            Err(e) => return Err(RootStoreError::Storage(e)),
        };
        let state = crate::trust::state::State::decode(&raw)
            .map_err(|e| RootStoreError::Trust(Box::new(e)))?;
        state
            .validate()
            .map_err(|e| RootStoreError::Trust(Box::new(e)))?;
        Ok(state
            .identities
            .iter()
            .any(|id| !id.disabled && id.native_scope.as_ref() == Some(&scope)))
    }
    /// Observe the exact retained native intention and physical datastore scope.
    /// Requires a prepared agent whose task and last attempt still match. Uses
    /// the opened directory/database identities, never a caller-supplied path
    /// hash. No process authentication, launch permission, trust role or mandate
    /// is created. The observation can become stale immediately; an effect
    /// consumer must independently revalidate its owned runtime context.
    /// Codec, SQL, instance and rollback failures retain their typed causes.
    #[cfg(target_os = "linux")]
    pub fn native_agent_scope(&mut self, id: &str) -> Result<NativeTaskScope, RootStoreError> {
        let instance = self.instance()?;
        let mut tx = self.db.transaction().map_err(RootStoreError::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = (|| {
            let (agent, _, _) = crate::agent_execution_preflight::retained(&tx, id)
                .map_err(|e| RootStoreError::Launch(Box::new(e)))?;
            let work =
                crate::read_recorded_work(&tx, &agent.work_id).map_err(RootStoreError::Work)?;
            let task = work
                .tasks
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|task| task.id == agent.task_id)
                .ok_or(RootStoreError::Refused("native-scope-task-missing"))?;
            if task.status != "running"
                || task.owner != agent.provider
                || !task
                    .attempts
                    .as_deref()
                    .and_then(|a| a.last())
                    .is_some_and(|a| {
                        a.id == agent.attempt_id && a.status == "recorded" && a.ended.is_empty()
                    })
            {
                return Err(RootStoreError::Refused("native-scope-attempt-stale"));
            }
            let scope = scope_for(&instance, &agent)?;
            if super::observe_instance(&tx, &self.directory)? != instance {
                return Err(RootStoreError::Refused("native-scope-root-changed"));
            }
            Ok(scope)
        })();
        finished(result, tx.rollback().map_err(RootStoreError::Sql))
    }

    /// Mediate one owned native request before a protected Trust effect.
    /// Called only by the foreground owner of the connected child output
    /// channel, whose original directory and SQLite handles remain open.
    /// Rechecks physical Root, current PID/stamps/parent/group, SQL headers,
    /// permission, task/attempt, mission consent and budget under a SQL writer
    /// spanning Trust publication. No startup TTL is extended or reapplied.
    /// The closed operation set excludes configuration, approval and mandate
    /// issuance. All business/Biba guards still execute in Trust. Errors retain
    /// known publication across rollback failure; no retry follows automatically.
    pub(crate) fn native_trust(
        &self,
        original: &RootInstanceIdentity,
        expected: &crate::Agent,
        operation: &str,
        raw: &[u8],
    ) -> Result<crate::TrustReply, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = self.native_trust_reserved(original, expected, operation, raw);
        let publication = match &result {
            Ok(reply) => reply.publication,
            Err(error) => error.trust_publication(),
        };
        finished(result, reservation.close().map_err(RootStoreError::Storage)).map_err(|error| {
            match publication {
                Some(publication) => RootStoreError::TrustPublished {
                    primary: Box::new(error),
                    publication,
                },
                None => error,
            }
        })
    }

    /// The settings reader precedes this SQL writer and the Trust journal lock;
    /// keeping all three through publication prevents a concurrent Admin policy
    /// change between observation and effect. Caller closes the reader and
    /// preserves publication on every late error. No new waiting/retry policy.
    fn native_trust_reserved(
        &self,
        original: &RootInstanceIdentity,
        expected: &crate::Agent,
        operation: &str,
        raw: &[u8],
    ) -> Result<crate::TrustReply, RootStoreError> {
        use crate::trust::authentication::Source;
        if !matches!(operation, "request" | "commit" | "reconcile") || &self.instance()? != original
        {
            return Err(RootStoreError::Refused("native-trust-source-refused"));
        }
        let mut tx = rusqlite::Transaction::new_unchecked(
            &self.db,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(RootStoreError::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = (|| {
            let fail = |e| RootStoreError::Launch(Box::new(e));
            let (agent, request, _) =
                crate::agent_execution_preflight::retained(&tx, &expected.id).map_err(fail)?;
            let owner = crate::SupervisorIdentity::current()
                .map_err(|e| fail(crate::AgentLaunchError::caused("native-trust-owner", e)))?;
            let pid = u32::try_from(agent.child_pid)
                .map_err(|e| fail(crate::AgentLaunchError::caused("native-trust-child-pid", e)))?;
            let child = crate::observe_linux_process(pid)
                .map_err(|e| fail(crate::AgentLaunchError::caused("native-trust-child", e)))?;
            if agent.host != owner.host()
                || agent.supervisor_pid != owner.pid()
                || agent.supervisor_identity != owner.process_stamp()
                || agent.child_pid != expected.child_pid
                || agent.child_identity != expected.child_identity
                || child.process_stamp() != agent.child_identity
                || child.parent_pid() != std::process::id()
                || child.process_group_id() != child.pid()
                || matches!(child.state(), 'Z' | 'X' | 'x')
                || agent.work_id != expected.work_id
                || agent.task_id != expected.task_id
                || agent.attempt_id != expected.attempt_id
                || !agent.desired.is_empty()
                || !matches!(agent.status.as_str(), "starting" | "running")
                || agent.native_execution_result.is_some()
                || !agent.ended.is_empty()
                || agent.exit_code.is_some()
                || !agent.stop_kind.is_empty()
            {
                return Err(RootStoreError::Refused("native-trust-execution-stale"));
            }
            let snapshot = agent
                .native_execution
                .as_ref()
                .ok_or(RootStoreError::Refused("native-trust-permission-missing"))?;
            let snapshot = crate::decode_agent_execution_snapshot(snapshot.0.get().as_bytes())
                .map_err(|e| {
                    fail(crate::AgentLaunchError::caused(
                        "native-trust-permission",
                        e,
                    ))
                })?;
            if snapshot.phase != crate::AgentExecutionPhase::PermissionRecorded {
                return Err(RootStoreError::Refused("native-trust-permission-missing"));
            }
            super::agent_execution_origin::require(&tx, &self.directory, &agent.id)
                .map_err(fail)?;
            let settings = self.read_settings()?;
            if settings.revision != snapshot.settings_revision
                || settings
                    .values
                    .agent_execution_policy()
                    .map_err(RootStoreError::Settings)?
                    != snapshot.policy
            {
                return Err(RootStoreError::Refused(
                    "native-trust-execution-policy-changed",
                ));
            }
            super::mission_agent::inspect(
                &tx,
                &agent,
                &settings,
                super::mission_agent::Check::BeforeGrant(&self.directory),
            )
            .map_err(fail)?;
            crate::agent_launch::execution_guards(&tx, &agent.work_id, &request, &agent)
                .map_err(fail)?;
            crate::check_reserved_agent_budget(&tx, &agent.work_id, &agent.id)
                .map_err(|e| fail(crate::AgentLaunchError::caused(e.code(), e)))?;
            let work =
                crate::read_recorded_work(&tx, &agent.work_id).map_err(RootStoreError::Work)?;
            let task = work
                .tasks
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|task| task.id == agent.task_id)
                .ok_or(RootStoreError::Refused("native-trust-task-missing"))?;
            if task.status != "running"
                || task.owner != agent.provider
                || !task
                    .attempts
                    .as_deref()
                    .and_then(|a| a.last())
                    .is_some_and(|a| {
                        a.id == agent.attempt_id && a.status == "recorded" && a.ended.is_empty()
                    })
            {
                return Err(RootStoreError::Refused("native-trust-attempt-stale"));
            }
            let context = NativeExecutionContext {
                scope: scope_for(original, &agent)?,
            };
            self.trust_scoped_reserved(operation, raw, Source::Native(&context), true)
        })();
        let publication = result.as_ref().ok().and_then(|reply| reply.publication);
        finished(result, tx.rollback().map_err(RootStoreError::Sql)).map_err(
            |e| match publication {
                Some(publication) => RootStoreError::TrustPublished {
                    primary: Box::new(e),
                    publication,
                },
                None => e,
            },
        )
    }
}
