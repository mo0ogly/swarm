//! CLI preparation selection and owned subprocess lifecycle; no Work dispatch.
use super::*;
use crate::{ModelRoute, Provider};
use std::{
    collections::BTreeSet,
    io::{self, Write},
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

/// Private claim result; neither branch is serializable or a public execution API.
/// Admission has already moved its reservation to estimated before construction.
pub enum PreparationWorkerRequest {
    Api(PreparationApiRequest),
    Cli(PreparationCliRequest),
}
/// Saved limits for the already registered CLI worker. Command/prompt are reread
/// only behind its anonymous helper gate; these values grant no new authority.
pub struct PreparationCliRequest {
    pub response_max_bytes: Option<usize>,
    pub event_max_bytes: Option<usize>,
    pub timeout_seconds: u64,
    pub observation_ms: u64,
}
pub(super) enum Selected {
    Api(crate::provider_connection_catalogue::ConnectionState),
    Cli {
        provider: Provider,
        route: Option<ModelRoute>,
        environment: Vec<String>,
    },
}
impl Selected {
    /// Return the requested persisted route, never authenticated vendor identity.
    pub(super) fn route(&self, digest: &str) -> Option<ModelRoute> {
        match self {
            Self::Api(p) => Some(ModelRoute {
                model: p.model.clone(),
                policy_hash: digest.into(),
                reason: "configured-api-fixed-model".into(),
                ..Default::default()
            }),
            Self::Cli { route, .. } => route.clone(),
        }
    }
}
/// Existing preparation adapter contract: harness/custom/probed adapters remain
/// unsupported. This filter grants neither account access nor effective isolation.
pub(super) fn eligible(p: &Provider) -> bool {
    p.api_connection_id.is_empty()
        && !p.preflight_required
        && p.preflight_args.as_ref().is_none_or(Vec::is_empty)
        && matches!(
            Path::new(&p.command).file_name().and_then(|s| s.to_str()),
            Some("claude" | "codex")
        )
}
impl RootStore {
    /// Observe configured provider, route, executable bytes and allowed environment.
    /// The combined digest is freshness only. No contact, execution or model probe;
    /// CLI needs the saved unverified-context grant and cannot bypass preflight.
    pub(super) fn preparation_selected_provider(
        &self,
        name: &str,
        settings: &EngineSettingsValues,
    ) -> Result<(Selected, String), E> {
        self.preparation_selected_provider_level(name, settings, "")
    }
    /// Select the requested persisted CLI level with unchanged routing rules,
    /// announcements, environment/executable/source checks and execution grant.
    /// Empty/auto normalize to legacy empty; API stays fixed-model. No fallback,
    /// child, model contact, financial mutation or new permission occurs.
    pub(super) fn preparation_selected_provider_level(
        &self,
        name: &str,
        settings: &EngineSettingsValues,
        level: &str,
    ) -> Result<(Selected, String), E> {
        let level = requested_level(level)?;
        if name.starts_with("api-") {
            if !level.is_empty() {
                return Err(E::Refused("preparation-send-invalid"));
            }
            let (p, d) = self.preparation_api_provider(name, settings)?;
            return Ok((Selected::Api(p), d));
        }
        let policy = settings.agent_launch_policy().map_err(E::Settings)?;
        if !policy.allow_unverified {
            return Err(E::Refused("agent-provider-verified-context-required"));
        }
        let raw = self
            .directory
            .read_capped("providers.json", policy.provider_bytes)
            .map_err(E::Storage)?;
        let api = match self
            .directory
            .read_capped("ai-connections.json", policy.provider_bytes)
        {
            Ok(raw) => Some(raw),
            Err(error) if super::super::absent(&error) => None,
            Err(error) => return Err(E::Storage(error)),
        };
        let catalogue =
            crate::decode_providers(&raw).map_err(|e| cause("preparation-provider-invalid", e))?;
        if catalogue.schema_version != 1 {
            return Err(E::Refused("preparation-provider-schema-invalid"));
        }
        crate::provider_connection_catalogue::guard(api.as_deref(), &catalogue, "")
            .map_err(|e| E::Launch(Box::new(e)))?;
        let original = catalogue
            .providers
            .as_ref()
            .and_then(|p| p.get(name))
            .ok_or(E::Refused("preparation-provider-not-configured"))?;
        if !eligible(original) {
            return Err(E::Refused("preparation-provider-adapter-unported"));
        }
        let assistant = crate::prepare_assistant_provider(original)
            .map_err(|e| cause("preparation-cli-adapter", e))?;
        let requested = crate::Launch {
            provider: name.into(),
            level: level.into(),
            ..Default::default()
        };
        let routed = crate::provider_model_route::resolve_configuration_metadata_for_purpose(
            &assistant,
            &requested,
            settings,
            &policy,
            "preparation",
        )
        .map_err(|e| E::Launch(Box::new(e)))?;
        let provider = routed.provider().clone();
        let executable = crate::agent_launch_preflight::executable(Path::new(&provider.command))
            .map_err(|e| E::Launch(Box::new(e)))?;
        let mut environment = Vec::new();
        let mut seen = BTreeSet::new();
        let mut observed = Vec::new();
        for name in policy
            .environment
            .iter()
            .chain(provider.env_allow.as_deref().unwrap_or_default())
        {
            if !crate::agent_launch_policy::environment_name(name) {
                return Err(E::Refused("preparation-cli-environment-invalid"));
            }
            if seen.insert(name.clone()) {
                use std::os::unix::ffi::OsStrExt;
                environment.push(name.clone());
                observed.push((name, std::env::var_os(name).map(|v| hash(v.as_bytes()))));
            }
        }
        let effective = crate::encode_model(&(provider.clone(), routed.route(), routed.scope()))
            .map_err(|e| cause("preparation-cli-provider-encode", e))?;
        let observed = crate::encode_model(&observed)
            .map_err(|e| cause("preparation-cli-environment-encode", e))?;
        let api_digest = crate::encode_model(&api.as_deref().map(hash))
            .map_err(|e| cause("preparation-cli-source-encode", e))?;
        let digest = hash(
            &[
                hash(&raw).as_bytes(),
                b"\0",
                &api_digest,
                b"\0",
                executable.as_bytes(),
                b"\0",
                &effective,
                b"\0",
                &observed,
            ]
            .concat(),
        );
        Ok((
            Selected::Cli {
                provider,
                route: routed.route().cloned(),
                environment,
            },
            digest,
        ))
    }
}

/// Project only expected per-mode routing refusals. Stable machine reason codes
/// from typed settings selection are protocol, not rendered error text. Other
/// settings/IO/codec/cleanup failures propagate rather than successful emptiness.
fn mode_refusal(error: &E) -> Option<&'static str> {
    match error {
        E::Refused(code) => Some(*code),
        E::Launch(error) => match error.as_ref() {
            crate::AgentLaunchError::Refused(code) => Some(*code),
            crate::AgentLaunchError::Cause { code, cause }
                if matches!(
                    cause.downcast_ref::<crate::EngineSettingsError>(),
                    Some(crate::EngineSettingsError::Refused(
                        "settings-model-automatic-level-denied"
                            | "settings-model-restricted-level-denied"
                    ))
                ) =>
            {
                Some(code)
            }
            _ => None,
        },
        _ => None,
    }
}
impl RootStore {
    /// Observe automatic and all v1 policy modes without executing a provider.
    /// Each fresh digest/model is derived from the same selected route; unavailable
    /// auto never selects another mode. Settings supply every restriction; mode
    /// names are the existing policy-v1 vocabulary, not an operator ceiling.
    /// Read errors retain causes. This creates no admission, reservation or retry.
    pub(super) fn preparation_cli_capability(
        &self,
        name: &str,
        settings: &crate::EngineSettingsDocument,
    ) -> Result<
        (
            Option<PreparationApiCapability>,
            Option<PreparationUnavailableProvider>,
        ),
        E,
    > {
        let settings_digest = &settings
            .history
            .last()
            .ok_or(E::Refused("preparation-settings-history-missing"))?
            .digest;
        let mut levels = Vec::new();
        for (request, label) in [
            ("", "auto"),
            ("simple", "simple"),
            ("standard", "standard"),
            ("exigeant", "exigeant"),
        ] {
            let selection = if request.is_empty() {
                self.preparation_selected_provider(name, &settings.values)
            } else {
                self.preparation_selected_provider_level(name, &settings.values, request)
            };
            match selection {
                Ok((selected, source)) => {
                    let digest = hash(format!("{}\0{}", source, settings_digest).as_bytes());
                    let route = selected
                        .route(&digest)
                        .ok_or(E::Refused("preparation-cli-model-route-required"))?;
                    levels.push(PreparationLevelCapability {
                        level: label.into(),
                        available: true,
                        model: route.model,
                        policy_hash: route.policy_hash,
                        digest,
                        reason: String::new(),
                    });
                }
                Err(error) => {
                    let Some(code) = mode_refusal(&error) else {
                        return Err(error);
                    };
                    levels.push(PreparationLevelCapability {
                        level: label.into(),
                        available: false,
                        model: String::new(),
                        policy_hash: String::new(),
                        digest: String::new(),
                        reason: code.into(),
                    });
                }
            }
        }
        // The automatic observation is first by protocol, not chosen by success.
        let auto = levels
            .first()
            .ok_or(E::Refused("preparation-cli-capability-empty"))?;
        if !levels.iter().any(|l| l.available) {
            return Ok((
                None,
                Some(PreparationUnavailableProvider {
                    name: name.into(),
                    reason: auto.reason.clone(),
                }),
            ));
        }
        let capability = PreparationApiCapability {
            name: name.into(),
            model: auto.model.clone(),
            digest: auto.digest.clone(),
            levels: Some(levels),
        };
        Ok((Some(capability), None))
    }
}

/// Streaming sink preserves parser causes for the owned process controller.
struct Output(crate::PreparationCliOutput);
impl Write for Output {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.0.push(b).map_err(io::Error::other)?;
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
/// Private pre-exec failure received on the engine-only CLOEXEC channel, never
/// on provider stdout/stderr. Preserve the redacted frame and typed OS cause.
struct HelperFailure {
    frame: crate::agent_execution_gate::GateFailure,
    io: Option<io::Error>,
}
impl std::fmt::Display for HelperFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "preparation-cli-helper: {}", self.frame.code)
    }
}
impl std::fmt::Debug for HelperFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for HelperFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.io
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}
struct Control<'a> {
    root: &'a RootStore,
    id: &'a str,
    gate: Option<std::os::unix::net::UnixStream>,
    token: &'a str,
    next: Instant,
    period: Duration,
}
impl crate::validation_run::OwnedProcessControl for Control<'_> {
    /// Check the registered parent immediately before releasing the private gate.
    fn started(&mut self, _pid: u32) -> io::Result<()> {
        if !self
            .root
            .preparation_worker_continue(self.id)
            .map_err(io::Error::other)?
        {
            return Err(io::Error::other("preparation-call-interrupted"));
        }
        let gate = self
            .gate
            .as_mut()
            .ok_or_else(|| io::Error::other("preparation-cli-gate-missing"))?;
        gate.write_all(self.token.as_bytes())?;
        gate.shutdown(std::net::Shutdown::Write)?;
        Ok(())
    }

    fn next_wake(&self) -> Option<Instant> {
        Some(self.next)
    }
    /// Cancellation/current-source errors enter the shared owned cleanup path.
    fn observe(&mut self) -> io::Result<Option<rustix::process::Signal>> {
        if Instant::now() >= self.next {
            if !self
                .root
                .preparation_worker_continue(self.id)
                .map_err(io::Error::other)?
            {
                return Err(io::Error::other("preparation-call-interrupted"));
            }
            self.next = Instant::now()
                .checked_add(self.period)
                .ok_or_else(|| io::Error::other("preparation-call-observation-representation"))?;
        }
        Ok(None)
    }
}
impl RootStore {
    /// Run only an already claimed turn behind a second private helper gate.
    /// Reuses pidfd/owned-group cleanup: leader remains unreaped until group kill.
    /// Saved timeout/observation/output limits are rederived from the native turn;
    /// request values are not authority. No retry/refund or Work task dispatch.
    /// Stream/controller/close/wait failures retain their original typed causes.
    pub fn preparation_worker_run_cli(
        &self,
        id: &str,
        token: &str,
    ) -> Result<(String, Option<crate::Usage>), E> {
        if !self.preparation_worker_continue(id)? {
            return Err(E::Refused("preparation-call-interrupted"));
        }
        let (_, n) = turn(&self.db, id)?;
        if token != hash(n.nonce.as_bytes()) {
            return Err(E::Refused("preparation-worker-registration-mismatch"));
        }
        let now = Instant::now();
        let deadline = if n.policy.timeout == 0 {
            None
        } else {
            Some(
                now.checked_add(Duration::from_secs(n.policy.timeout))
                    .ok_or(E::Refused("preparation-call-deadline-representation"))?,
            )
        };
        use std::os::unix::fs::DirBuilderExt;
        let workspace = std::env::temp_dir()
            .join(crate::generate_identifier("swarm-preparation-cli-").map_err(E::Runtime)?);
        let root_text = self
            .root
            .to_str()
            .ok_or(E::Refused("preparation-cli-root-utf8"))?;
        let workspace_text = workspace
            .to_str()
            .ok_or(E::Refused("preparation-cli-workspace-utf8"))?;
        let (parent, child) = std::os::unix::net::UnixStream::pair()
            .map_err(|e| cause("preparation-cli-gate-create", e))?;
        if let Err(error) = std::fs::DirBuilder::new().mode(0o700).create(&workspace) {
            return finished(
                finished(
                    Err(cause("preparation-cli-workspace-create", error)),
                    nix::unistd::close(parent).map_err(|e| cause("preparation-cli-gate-close", e)),
                ),
                nix::unistd::close(child).map_err(|e| cause("preparation-cli-gate-close", e)),
            );
        }
        let mut command = Command::new("/proc/self/exe");
        command.args(["--root", root_text, "_prepare_cli_exec", id, workspace_text]);
        let mut control = Control {
            root: self,
            id,
            gate: Some(parent),
            token,
            next: now,
            period: Duration::from_millis(n.policy.observation),
        };
        let mut output = Output(crate::PreparationCliOutput::new(
            n.policy.response,
            n.policy.event,
        ));
        let mut stderr = io::sink();
        let run = crate::validation_run::run_controlled_command(
            command,
            std::os::fd::OwnedFd::from(child).into(),
            deadline,
            &mut output,
            Some(&mut stderr),
            Some(&mut control),
        );
        let helper = if let Some(mut gate) = control.gate.take() {
            use std::io::Read;
            let mut bytes = Vec::new();
            let read = gate
                .read_to_end(&mut bytes)
                .map_err(|e| cause("preparation-cli-helper-read", e));
            let closed =
                nix::unistd::close(gate).map_err(|e| cause("preparation-cli-gate-close", e));
            finished(read, closed).and_then(|_| {
                if bytes.is_empty() {
                    return Ok(());
                }
                let frame: crate::agent_execution_gate::GateFailure =
                    serde_json::from_slice(&bytes)
                        .map_err(|e| cause("preparation-cli-helper-frame", e))?;
                if frame.schema_version != 1 || frame.kind != "pre_exec_failure" {
                    return Err(E::Refused("preparation-cli-helper-frame-invalid"));
                }
                let io = frame.io_errno.map(io::Error::from_raw_os_error);
                Err(cause("preparation-cli-helper", HelperFailure { frame, io }))
            })
        } else {
            Err(E::Refused("preparation-cli-gate-missing"))
        };
        // Preserve an unresolved owned process and its workspace on cleanup error;
        // remove after known cleanup; unknown ownership/reap keeps the directory.
        let cleaned = if run.is_ok()
            || run.as_ref().is_err_and(|e| {
                e.unresolved_process_id().is_none() && e.ownership_lost_process_id().is_none()
            }) {
            std::fs::remove_dir_all(&workspace)
                .map_err(|e| cause("preparation-cli-workspace-remove", e))
        } else {
            Ok(())
        };
        let run = finished(
            finished(run.map_err(|e| cause("preparation-cli-process", e)), helper),
            cleaned,
        )?;
        if run.timed_out {
            return Err(E::Refused("preparation-call-timeout"));
        }
        if !run.usage.status.success() {
            return Err(E::Refused("preparation-cli-exit-failed"));
        }
        if !self.preparation_worker_continue(id)? {
            return Err(E::Refused("preparation-call-interrupted"));
        }
        output
            .0
            .finish()
            .map_err(|e| cause("preparation-cli-output", e))
    }
    /// Consume a private gate in the helper whose actual parent is the registered
    /// worker. Recheck Root/settings/provider bytes immediately before direct exec.
    /// Provider stdin is sealed, argv/environment explicit; cwd is an owned private
    /// temporary directory. This proves requested flags, not effective confinement.
    /// Successful exec never returns; caller must exit on error. No raw PID adoption.
    fn preparation_cli_prepare(
        &self,
        id: &str,
        token: &str,
        workspace: &Path,
    ) -> Result<crate::native_provider_exec::PreparedExec, E> {
        crate::supervisor_identity::arm_parent_death_signal()
            .map_err(|e| cause("preparation-cli-parent-death", e))?;
        let parent = u32::try_from(nix::unistd::getppid().as_raw())
            .map_err(|e| cause("preparation-cli-parent", e))?;
        let observed =
            crate::observe_linux_process(parent).map_err(|e| cause("preparation-cli-parent", e))?;
        let identity = crate::SupervisorIdentity::current()
            .map_err(|e| cause("preparation-cli-identity", e))?;
        let (t, n) = turn(&self.db, id)?;
        if t.status != "running"
            || t.supervisor_pid != i64::from(parent)
            || t.supervisor_stamp != observed.process_stamp()
            || t.host != identity.host()
            || token != hash(n.nonce.as_bytes())
        {
            return Err(E::Refused("preparation-worker-registration-mismatch"));
        }
        let settings = self.read_settings()?;
        self.preparation_call_current(&t, &n, &settings)?;
        let p = stored(&self.db, &t.preparation_id)?;
        if p.revision != t.revision || p.method_hash != t.method_hash {
            return Err(E::Refused("preparation-call-stale"));
        }
        let (selected, digest) = self.preparation_selected_provider_level(
            &t.provider,
            &settings.values,
            &n.request_level,
        )?;
        if digest != n.provider_source {
            return Err(E::Refused("preparation-provider-source-changed"));
        }
        let Selected::Cli {
            provider,
            environment,
            ..
        } = selected
        else {
            return Err(E::Refused("preparation-cli-adapter-required"));
        };
        use std::os::unix::fs::MetadataExt;
        let observed = std::fs::symlink_metadata(workspace)
            .map_err(|e| cause("preparation-cli-workspace-observation", e))?;
        if !observed.is_dir() || observed.uid() != rustix::process::geteuid().as_raw()
            || observed.mode() & 0o077 != 0 {
            return Err(E::Refused("preparation-cli-workspace-not-private"));
        }
        let prepared = crate::native_provider_exec::PreparedExec::prepare(
            &provider,
            &environment,
            workspace,
            t.prompt.as_bytes(),
        )?.with_workspace_identity((observed.dev(), observed.ino()));
        // One final freshness observation; pathname executable replacement remains
        // the same unverified-host limitation as the existing native exec transport.
        let result = self
            .preparation_selected_provider_level(&t.provider, &settings.values, &n.request_level)
            .and_then(|(_, d)| {
                if d == digest {
                    Ok(())
                } else {
                    Err(E::Refused("preparation-provider-source-changed"))
                }
            });
        if let Err(e) = result {
            return finished(Err(e), prepared.close());
        }
        Ok(prepared)
    }
}

/// Consume the owned helper's private gate and close Root before direct provider
/// exec. Parent/kernel/token/source checks remain internal; argv carries no prompt.
/// Returns only on refusal/setup/close/exec failure, retaining all cleanup causes.
/// The shared transport additionally requires an isolated I0 Linux boundary.
/// Unsafe workspace/backend failure refuses before provider exec, without fallback.
pub fn execute_preparation_cli_gate(
    root: &Path,
    id: &str,
    token: &str,
    workspace: &Path,
) -> Result<std::convert::Infallible, E> {
    let fd = match rustix::io::fcntl_dupfd_cloexec(std::io::stdin(), 3) {
        Ok(fd) => fd,
        Err(error) => {
            return finished(
                Err(cause("preparation-cli-helper-channel", error)),
                nix::unistd::close(0).map_err(|e| cause("preparation-cli-gate-close", e)),
            )
        }
    };
    let input_closed = nix::unistd::close(0).map_err(|e| cause("preparation-cli-gate-close", e));
    let mut channel = std::os::unix::net::UnixStream::from(fd);
    let result = (|| {
        input_closed?;
        channel
            .peer_addr()
            .map_err(|e| cause("preparation-cli-helper-channel-kind", e))?;
        let store = RootStore::open(root, false)?;
        let prepared = store.preparation_cli_prepare(id, token, workspace);
        match (prepared, store.close()) {
            // Shared exec requires the I0 boundary; failure never falls back to raw exec.
            (Ok(prepared), Ok(())) => prepared.execute(root),
            (Err(primary), closed) => finished(Err(primary), closed),
            (Ok(prepared), Err(primary)) => finished(Err(primary), prepared.close()),
        }
    })();
    match result {
        Ok(never) => match never {},
        Err(primary) => {
            let frame = crate::agent_execution_gate::diagnostic(&primary);
            let sent = serde_json::to_vec(&frame)
                .map_err(|e| cause("preparation-cli-helper-frame", e))
                .and_then(|raw| {
                    channel
                        .write_all(&raw)
                        .map_err(|e| cause("preparation-cli-helper-frame-write", e))
                });
            let closed = nix::unistd::close(channel)
                .map_err(|e| cause("preparation-cli-helper-channel-close", e));
            finished(finished(Err(primary), sent), closed)
        }
    }
}

#[cfg(test)]
mod level_tests {
    use super::*;
    #[test]
    fn only_typed_selection_refusals_become_unavailable_modes() {
        let expected = E::Launch(Box::new(crate::AgentLaunchError::caused(
            "settings-model-automatic-level-denied",
            crate::EngineSettingsError::Refused("settings-model-automatic-level-denied"),
        )));
        assert_eq!(
            mode_refusal(&expected),
            Some("settings-model-automatic-level-denied")
        );
        let io = E::Launch(Box::new(crate::AgentLaunchError::caused(
            "settings-model-automatic-level-denied",
            std::io::Error::other("same wording is not a selection refusal"),
        )));
        assert!(mode_refusal(&io).is_none());
        let invalid = E::Launch(Box::new(crate::AgentLaunchError::caused(
            "settings-invalid",
            crate::EngineSettingsError::Refused("settings-invalid"),
        )));
        assert!(mode_refusal(&invalid).is_none());
    }
}
