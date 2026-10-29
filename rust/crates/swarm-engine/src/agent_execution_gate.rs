//! Private startup channel; provider text never authorizes an execution.
#![cfg(target_os = "linux")]
use crate::agent_execution_preflight::ExecutionPreparation;
use crate::{AgentLaunchError, RootStore, RootStoreError};
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    error::Error,
    io::{self, Read, Write},
    os::unix::net::UnixStream,
    path::Path,
};
/// Safe private-channel projection of a helper refusal before exec. The sender
/// is the owned engine helper, not provider stdout. This frame proves no actor
/// authentication; a malformed/missing frame cannot authorize refunds or success.
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct GateFailure {
    pub schema_version: i64,
    pub kind: String,
    pub code: String,
    pub io_errno: Option<i32>,
    pub sqlite_code: Option<i32>,
    /// Absent on legacy frames; this names only the listed safe native families.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causes_scope: Option<String>,
    /// Ordered known typed objects, not private text or authenticated authority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<crate::native_diagnostics::NativeGateCause>,
}
/// Preserve safe labels and every listed native compound branch without raw
/// paths, credentials, SQL, context or arbitrary Display strings. Legacy scalar
/// errno fields remain the first observations; scope identifies the additional
/// typed-object projection rather than claiming all custom Error graphs complete.
pub(crate) fn diagnostic(error: &RootStoreError) -> GateFailure {
    let causes = crate::native_diagnostics::project(error);
    GateFailure {
        schema_version: 1,
        kind: "pre_exec_failure".into(),
        code: error.code().into(),
        io_errno: causes.iter().find_map(|c| c.io_errno),
        sqlite_code: causes.iter().find_map(|c| c.sqlite_code),
        causes_scope: Some("known-native-compound-v1".into()),
        causes,
    }
}
/// Wrap an original OS/codec cause with a safe static execution family.
fn caused(code: &'static str, error: impl Error + Send + Sync + 'static) -> RootStoreError {
    RootStoreError::Launch(Box::new(AgentLaunchError::caused(code, error)))
}
/// Recheck observed host/model data against the checked settings-value snapshot
/// and the original (not refreshed) preparation TTL. Cache/env changes refuse
/// immediately before exec; values are observations, not an enduring lease.
/// Errors preserve causes and close prepared descriptors without a provider.
fn fresh(root: &Path, prepared: &ExecutionPreparation) -> Result<(), RootStoreError> {
    prepared
        .model
        .recheck(
            &prepared.request,
            &prepared.model_values,
            &prepared.launch_policy,
        )
        .map_err(|e| RootStoreError::Launch(Box::new(e)))?;
    crate::agent_launch_preflight::recheck(
        root,
        &prepared.model,
        &prepared.request,
        &prepared.launch_policy,
        &prepared.host,
    )
    .map_err(|e| RootStoreError::Launch(Box::new(e)))?;
    let receipt = prepared
        .agent
        .preflight
        .as_ref()
        .ok_or(RootStoreError::Refused("agent-execution-preflight-missing"))?;
    let now = crate::system_timestamp().map_err(RootStoreError::Runtime)?;
    let parse = |v: &str| {
        swarm_wire_json::go_rfc3339::parse(v).map_err(|e| caused("agent-execution-clock", e))
    };
    if parse(&now)? >= parse(&receipt.valid_until)? {
        return Err(RootStoreError::Refused("agent-execution-preflight-expired"));
    }
    Ok(())
}
/// Preserve the worker-specific argv conflict check before shared immutable
/// exec transport preparation. The checked model/host supplies argv/environment
/// and absolute workspace; no authority, freshness or gate check moves here.
/// Shared codec/kernel/cleanup failures retain their original typed causes.
fn prepare_exec(
    prepared: &ExecutionPreparation,
) -> Result<crate::native_provider_exec::PreparedExec, RootStoreError> {
    let provider = prepared.model.provider();
    if prepared.agent.command != provider.command || prepared.agent.args != provider.args {
        return Err(RootStoreError::Refused(
            "agent-execution-provider-arguments-conflict",
        ));
    }
    crate::native_provider_exec::PreparedExec::prepare(
        provider,
        &prepared.host.environment_names,
        &prepared.host.workspace,
        prepared.agent.prompt.as_bytes(),
    ).map(|command| command.with_workspace_identity(prepared.host.directory_identity))
}
/// Acquire the actual Root binding before helper-side gate inspection, then
/// close the preparing store explicitly. On failure both the store and any
/// acquired binding are closed with the complete primary/cleanup tree retained.
/// No new grant is created; prepare_agent_gate keeps its existing process,
/// permission, profile, policy, freshness and transaction checks.
fn prepare_bound_gate(
    root: &Path,
    id: &str,
) -> Result<(ExecutionPreparation, crate::RootBinding), RootStoreError> {
    let mut store = RootStore::open(root, false)?;
    let binding = match store.bind_root() {
        Ok(binding) => binding,
        Err(error) => return crate::root_store::finished(Err(error), store.close()),
    };
    let prepared = crate::root_store::finished(store.prepare_agent_gate(id), store.close());
    match prepared {
        Ok(prepared) => Ok((prepared, binding)),
        Err(error) => crate::root_store::finished(Err(error), binding.close()),
    }
}

/// Reopen Root only when it matches the preparing store's owned directory and
/// actual SQLite inode. The callback cannot run on a different instance; every
/// returning path closes the reopened store and preserves cleanup errors. The
/// binding is internal owned state, not a request tuple or an execution grant.
fn with_bound_telemetry_root<T>(
    root: &Path,
    binding: &crate::RootBinding,
    read: impl FnOnce(&RootStore) -> Result<T, RootStoreError>,
) -> Result<T, RootStoreError> {
    let store = RootStore::open(root, false)?;
    let result = (|| {
        if !binding.matches(&store)? {
            return Err(RootStoreError::Refused(
                "agent-execution-root-instance-changed",
            ));
        }
        read(&store)
    })();
    crate::root_store::finished(result, store.close())
}

/// Saved-settings telemetry exporter environment and, for Codex, leading
/// `-c otel.…` arguments for this worker launch (LIVE-UX-03), computed after
/// every gate/freshness check so they never enter preflight digests. Empty when not opted in or no live local web
/// server. Settings are read only from a Root matching the retained preparation
/// binding; that binding remains owned through the launcher call. Endpoint is
/// read relative to its owned directory; actual token SQLite is associated before
/// migration/issuance. This is not hostile same-UID OS confinement. Root/binding/
/// settings errors preserve their Root codes; endpoint/token launcher failures
/// use `agent-telemetry-setup-failed` with the cause retained, instead of running
/// silently uninstrumented. Reviewer and planning launches are not covered.
fn telemetry_environment(
    root: &Path,
    id: &str,
    prepared: &ExecutionPreparation,
    binding: &crate::RootBinding,
) -> Result<crate::LaunchTelemetry, RootStoreError> {
    let settings = with_bound_telemetry_root(root, binding, RootStore::settings)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| caused("agent-execution-clock", e))?;
    crate::telemetry_endpoint::launch_telemetry_environment_bound(
        root,
        binding,
        &settings.values,
        id,
        &prepared.agent.provider,
        i64::try_from(now.as_millis()).unwrap_or(i64::MAX),
    )
    .map_err(|e| caused("agent-telemetry-setup-failed", e))
}
/// Preserve the real token INSERT observation across later gate/exec cleanup
/// failures. Only the effect marker and typed original cause are retained; the
/// secret is never logged or copied into diagnostics. This creates no grant.
fn after_telemetry_error(error: RootStoreError, token_issued: bool) -> RootStoreError {
    if !token_issued {
        return error;
    }
    caused(
        "agent-telemetry-after-token-failed",
        crate::TelemetryStoreError::TokenIssued {
            primary: Box::new(crate::TelemetryStoreError::Root(Box::new(error))),
        },
    )
}
/// Consume the private one-byte release only in an owned engine-helper process.
/// Arms kernel parent-death SIGKILL before waiting for a grant; this bounds the
/// actual provider leader lifetime after an abrupt owner loss, not descendants
/// or durable crash settlement. Permission/parent/stamp checks remain required.
/// Root must contain this actual PID/starttime/parent/group and a committed native
/// permission snapshot; ordinary JSON/flags, stdout text and stale metadata are
/// never sufficient. An owned binding pins the preparing Root through the final
/// telemetry-settings reopen and is explicitly closed before provider exec.
/// A duplicated CLOEXEC socket carries only pre-exec failures;
/// it is not inherited by the provider. Successful exec never returns, so this
/// function cannot report a fabricated normal exit without starting the command.
/// No task is accepted here. Requires the Swarm CLI host's private gate dispatch,
/// Linux procfs/memfd/seals and the configured local-root operator authority;
/// these checks do not provide cryptographic identity. The shared exec transport
/// additionally requires the mandatory I0 Linux boundary; an unsafe workspace
/// or unavailable backend refuses before any provider exec.
pub fn execute_agent_gate(root: &Path, id: &str) -> Result<Infallible, RootStoreError> {
    if !swarm_identity_contract::name(id.as_bytes()) {
        return Err(RootStoreError::Refused("agent-execution-id-invalid"));
    }
    // 0/1/2 are POSIX stdio descriptors; 3 is the first non-stdio slot, not policy.
    let fd = rustix::io::fcntl_dupfd_cloexec(io::stdin(), 3)
        .map_err(|e| caused("agent-gate-channel-duplicate", e))?;
    let mut channel = UnixStream::from(fd);
    if let Err(error) = channel.peer_addr() {
        return crate::root_store::finished(
            Err(caused("agent-gate-channel-kind", error)),
            nix::unistd::close(channel).map_err(|e| caused("agent-gate-channel-close", e)),
        );
    }
    let result: Result<Infallible, RootStoreError> = (|| {
        crate::supervisor_identity::arm_parent_death_signal()
            .map_err(|e| caused("agent-gate-parent-death", e))?;
        let mut grant = [0u8; 1];
        io::stdin()
            .read_exact(&mut grant)
            .map_err(|e| caused("agent-gate-input", e))?;
        if grant != [1] {
            return Err(RootStoreError::Refused("agent-gate-release-invalid"));
        }
        let (prepared, binding) = prepare_bound_gate(root, id)?;
        let mut token_issued = false;
        let command = (|| {
            let command = prepare_exec(&prepared)?;
            if let Err(error) = fresh(root, &prepared) {
                return crate::root_store::finished(Err(error), command.close());
            }
            let telemetry = match telemetry_environment(root, id, &prepared, &binding) {
                Ok(entries) => entries,
                Err(error) => return crate::root_store::finished(Err(error), command.close()),
            };
            token_issued = telemetry.token_issued;
            let command = command
                .add_environment(telemetry.environment)?
                .add_leading_arguments(telemetry.arguments)?;
            // Catch a persistent Root replacement during the pathname-based
            // telemetry launcher. This cannot undo an already issued token and
            // does not prove that no telemetry side effect reached a copy.
            if let Err(error) = with_bound_telemetry_root(root, &binding, |_| Ok(())) {
                return crate::root_store::finished(Err(error), command.close());
            }
            Ok(command)
        })();
        let command = match (command, binding.close()) {
            (Ok(command), Ok(())) => Ok(command),
            (Err(error), closed) => crate::root_store::finished(Err(error), closed),
            (Ok(command), Err(error)) => crate::root_store::finished(Err(error), command.close()),
        }
        .map_err(|error| after_telemetry_error(error, token_issued))?;
        command
            .execute(root)
            .map_err(|error| after_telemetry_error(error, token_issued))
    })();
    // Every returning path is a pre-exec refusal. Keep the socket live until exec so
    // EOF is not produced by premature descriptor ownership/drop in this code.
    let error = match result {
        Err(error) => error,
        Ok(never) => match never {},
    };
    let payload = serde_json::to_vec(&diagnostic(&error))
        .map_err(|e| caused("agent-gate-diagnostic-encoding", e));
    let report = match payload {
        Ok(payload) => channel
            .write_all(&payload)
            .map_err(|e| caused("agent-gate-diagnostic-write", e)),
        Err(e) => Err(e),
    };
    let report = crate::root_store::finished(
        report,
        nix::unistd::close(channel).map_err(|e| caused("agent-gate-channel-close", e)),
    );
    crate::root_store::finished(Err(error), report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, fs, os::unix::fs::PermissionsExt, path::PathBuf};

    /// Private fresh Roots, retained on panic; no provider, process or live DB.
    struct Fixture(PathBuf);
    impl Fixture {
        /// Create an owned private parent for independently initialized Roots.
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(crate::generate_identifier("gate-telemetry-binding-").unwrap());
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
        /// Initialize through the supported Root API; no permission SQL is forged.
        fn initialize(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            RootStore::initialize(&path, &mut crate::SystemEnvironment)
                .unwrap()
                .close()
                .unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if !std::thread::panicking() {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }

    #[test]
    fn gate_telemetry_settings_refuse_replaced_database_in_same_directory_and_recover() {
        let fixture = Fixture::new();
        let root = fixture.initialize("original");
        let other = fixture.initialize("other");
        let store = RootStore::open(&root, false).unwrap();
        let binding = store.bind_root().unwrap();
        store.close().unwrap();
        with_bound_telemetry_root(&root, &binding, RootStore::settings).unwrap();
        let db = root.join(".swarm/state.db");
        let saved = root.join("original-state.db");
        let foreign = other.join(".swarm/state.db");
        fs::rename(&db, &saved).unwrap();
        fs::rename(&foreign, &db).unwrap();
        let read = Cell::new(false);
        let error = with_bound_telemetry_root(&root, &binding, |store| {
            read.set(true);
            store.settings()
        })
        .unwrap_err();
        assert!(matches!(
            error,
            RootStoreError::Refused("agent-execution-root-instance-changed")
        ));
        assert!(
            !read.get(),
            "foreign database settings callback must never run"
        );
        fs::rename(&db, &foreign).unwrap();
        fs::rename(&saved, &db).unwrap();
        with_bound_telemetry_root(&root, &binding, RootStore::settings).unwrap();
        binding.close().unwrap();
    }

    #[test]
    fn gate_telemetry_settings_refuse_replaced_root_before_read_and_recover() {
        let fixture = Fixture::new();
        let root = fixture.initialize("original");
        let other = fixture.initialize("other");
        let store = RootStore::open(&root, false).unwrap();
        let binding = store.bind_root().unwrap();
        let expected = store.settings().unwrap();
        store.close().unwrap();
        let valid = with_bound_telemetry_root(&root, &binding, RootStore::settings).unwrap();
        assert_eq!(
            crate::encode_model(&valid).unwrap(),
            crate::encode_model(&expected).unwrap()
        );
        fs::rename(root.join(".swarm"), root.join("original-swarm")).unwrap();
        fs::rename(other.join(".swarm"), root.join(".swarm")).unwrap();
        let read = Cell::new(false);
        let error = with_bound_telemetry_root(&root, &binding, |store| {
            read.set(true);
            store.settings()
        })
        .unwrap_err();
        assert!(matches!(
            error,
            RootStoreError::Refused("agent-execution-root-instance-changed")
        ));
        assert!(!read.get(), "foreign settings callback must never run");
        fs::rename(root.join(".swarm"), other.join(".swarm")).unwrap();
        fs::rename(root.join("original-swarm"), root.join(".swarm")).unwrap();
        let recovered = with_bound_telemetry_root(&root, &binding, RootStore::settings).unwrap();
        assert_eq!(
            crate::encode_model(&recovered).unwrap(),
            crate::encode_model(&expected).unwrap()
        );
        binding.close().unwrap();
    }

    #[test]
    fn gate_late_failure_preserves_issued_token_marker_and_original_cause() {
        let unchanged = after_telemetry_error(RootStoreError::Refused("fixture-original"), false);
        assert!(matches!(
            unchanged,
            RootStoreError::Refused("fixture-original")
        ));
        let error = after_telemetry_error(RootStoreError::Refused("fixture-original"), true);
        let mut next: Option<&(dyn std::error::Error + 'static)> = Some(&error);
        let mut marker = false;
        let mut original = false;
        while let Some(error) = next {
            if let Some(error) = error.downcast_ref::<crate::TelemetryStoreError>() {
                marker |= error.token_issued();
            }
            if let Some(RootStoreError::Refused("fixture-original")) =
                error.downcast_ref::<RootStoreError>()
            {
                original = true;
            }
            next = error.source();
        }
        assert!(
            marker && original,
            "known effect and typed original must remain in the cause chain"
        );
    }
}
