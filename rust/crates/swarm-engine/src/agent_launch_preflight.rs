//! Host preparation observations. Compatibility never proves provider capability.
#![cfg(target_os = "linux")]
use crate::agent_launch_error::finish;
use crate::provider_model_route::ModelRoutePreparation;
use crate::{
    AgentLaunchError, AgentLaunchPolicy, Launch, PreflightCheck, PreflightResult, Provider,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Write},
    mem::MaybeUninit,
    os::{
        fd::IntoRawFd,
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
/// Observed candidate inputs only. Constructor is engine-owned; recheck is still
/// required on admission and again before any future process execution.
pub(crate) struct HostPreparation {
    pub workspace: PathBuf,
    pub environment_names: Vec<String>,
    pub preflight: PreflightResult,
    pub executable_digest: String,
    pub directory_identity: (u64, u64),
}
/// Hash whole supplied bytes; this authenticates neither producer nor account.
pub(crate) fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Close an owned descriptor explicitly, preserving OS failure as safe context.
fn close(file: File) -> Result<(), AgentLaunchError> {
    nix::unistd::close(file.into_raw_fd())
        .map_err(|e| AgentLaunchError::caused("agent-launch-close", e))
}
/// Read the executable fully and bind the observation to one unchanged inode.
/// Buffered transfer size is an allocation detail, not an executable-size policy.
/// Read/metadata/close failures refuse; no executable is launched or trusted.
fn executable(path: &Path) -> Result<String, AgentLaunchError> {
    if !path.is_absolute() {
        return Err(AgentLaunchError::Refused(
            "agent-provider-absolute-command-required",
        ));
    }
    let mut file =
        File::open(path).map_err(|e| AgentLaunchError::caused("agent-provider-open", e))?;
    let result = (|| {
        let before = file
            .metadata()
            .map_err(|e| AgentLaunchError::caused("agent-provider-metadata", e))?;
        if !before.is_file() || before.mode() & 0o111 == 0 {
            return Err(AgentLaunchError::Refused("agent-provider-not-executable"));
        }
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let n = file
                .read(&mut buffer)
                .map_err(|e| AgentLaunchError::caused("agent-provider-read", e))?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let after = file
            .metadata()
            .map_err(|e| AgentLaunchError::caused("agent-provider-metadata", e))?;
        let current = std::fs::metadata(path)
            .map_err(|e| AgentLaunchError::caused("agent-provider-path", e))?;
        let signature =
            |m: &std::fs::Metadata| (m.dev(), m.ino(), m.len(), m.mtime(), m.mtime_nsec());
        if signature(&before) != signature(&after) || signature(&after) != signature(&current) {
            return Err(AgentLaunchError::Refused(
                "agent-provider-changed-during-observation",
            ));
        }
        Ok(format!("{:x}", hasher.finalize()))
    })();
    finish(result, close(file))
}
/// Resolve a real directory within the actual canonical project, matching Go's
/// contained-symlink workspace rule. The result is observation, not future lease.
pub(crate) fn workspace(root: &Path, requested: &str) -> Result<PathBuf, AgentLaunchError> {
    let root = std::fs::canonicalize(root)
        .map_err(|e| AgentLaunchError::caused("agent-root-unavailable", e))?;
    let candidate = if requested.is_empty() {
        root.clone()
    } else {
        root.join(requested)
    };
    let actual = std::fs::canonicalize(candidate)
        .map_err(|e| AgentLaunchError::caused("agent-workspace-unavailable", e))?;
    if !actual.starts_with(&root) || !actual.is_dir() {
        return Err(AgentLaunchError::Refused(
            "agent-workspace-outside-project-or-not-directory",
        ));
    }
    Ok(actual)
}
/// Observe only explicitly configured environment names, preserving missing
/// versus empty values. Persisted scope gets hashes only; credentials never leave
/// this function. Invalid names refuse instead of being silently ignored.
struct EnvironmentObservation {
    names: Vec<String>,
    digests: BTreeMap<String, Option<String>>,
}
/// Observe the saved allowlist and provider additions without persisting values.
fn environment(
    policy: &AgentLaunchPolicy,
    provider: &Provider,
) -> Result<EnvironmentObservation, AgentLaunchError> {
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    let mut observed = BTreeMap::new();
    for name in policy
        .environment
        .iter()
        .chain(provider.env_allow.as_deref().unwrap_or_default())
    {
        if !crate::agent_launch_policy::environment_name(name) {
            return Err(AgentLaunchError::Refused(
                "agent-provider-environment-name-invalid",
            ));
        }
        if seen.insert(name.clone()) {
            names.push(name.clone());
            observed.insert(
                name.clone(),
                std::env::var_os(name).map(|v| hash(v.as_bytes())),
            );
        }
    }
    Ok(EnvironmentObservation {
        names,
        digests: observed,
    })
}
/// Exercise host directory read, isolated write/sync/close/removal and configured
/// free-space floor. Uses an owned directory descriptor; cleanup errors refuse.
/// Linux dirent buffer4096 reads one entry, not a count/volume policy. This proves
/// host observations only; no claim about provider sandbox capabilities is made.
fn host_directory(path: &Path, minimum: u64) -> Result<(u64, u64), AgentLaunchError> {
    let fd = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|e| AgentLaunchError::caused("agent-workspace-open", e))?;
    let directory: File = fd.into();
    let result = (|| {
        let metadata = directory
            .metadata()
            .map_err(|e| AgentLaunchError::caused("agent-workspace-metadata", e))?;
        let mut buffer = [MaybeUninit::uninit(); 4096];
        let mut reader = rustix::fs::RawDir::new(&directory, &mut buffer);
        if let Some(entry) = reader.next() {
            entry.map_err(|e| AgentLaunchError::caused("agent-workspace-read", e))?;
        }
        let name = crate::generate_identifier(".swarm-preflight-")
            .map_err(|e| AgentLaunchError::caused("agent-preflight-identifier", e))?;
        let fd = rustix::fs::openat(
            &directory,
            &name,
            rustix::fs::OFlags::WRONLY
                | rustix::fs::OFlags::CREATE
                | rustix::fs::OFlags::EXCL
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::from_raw_mode(0o600),
        )
        .map_err(|e| AgentLaunchError::caused("agent-workspace-write-create", e))?;
        let mut probe: File = fd.into();
        let write = probe
            .write_all(b"swarm-preflight\n")
            .and_then(|()| probe.sync_all())
            .map_err(|e| AgentLaunchError::caused("agent-workspace-write-sync", e));
        let write = finish(write, close(probe));
        let removed = rustix::fs::unlinkat(&directory, &name, rustix::fs::AtFlags::empty())
            .map_err(|e| AgentLaunchError::caused("agent-preflight-remove", e));
        finish(write, removed)?;
        if minimum > 0 {
            let stat = rustix::fs::fstatvfs(&directory)
                .map_err(|e| AgentLaunchError::caused("agent-workspace-resources", e))?;
            let available = u128::from(stat.f_bavail) * u128::from(stat.f_frsize);
            if available < u128::from(minimum) {
                return Err(AgentLaunchError::Refused(
                    "agent-workspace-free-space-below-policy",
                ));
            }
        }
        Ok((metadata.dev(), metadata.ino()))
    })();
    finish(result, close(directory))
}
/// Produce a host-compatible receipt from the checked model-route candidate.
/// Required/provider probes still refuse. Requested route/catalogue/rules enter
/// the scope only as unverified observations, never capability or authority.
/// Current policy supplies all TTL/resource/environment choices. No child runs.
pub(crate) fn observe(
    root: &Path,
    provider_name: &str,
    model: &ModelRoutePreparation,
    launch: &Launch,
    policy: &AgentLaunchPolicy,
) -> Result<HostPreparation, AgentLaunchError> {
    let provider = model.provider();
    if !policy.allow_unverified {
        return Err(AgentLaunchError::Refused(
            "agent-provider-verified-context-required",
        ));
    }
    if !provider.api_connection_id.is_empty()
        || provider.preflight_required
        || provider
            .preflight_args
            .as_ref()
            .is_some_and(|v| !v.is_empty())
    {
        return Err(AgentLaunchError::Refused(
            "agent-provider-adapter-or-model-route-unported",
        ));
    }
    if !matches!(
        provider.preflight_kind.as_str(),
        "" | "operator-command" | "provider-context"
    ) {
        return Err(AgentLaunchError::Refused(
            "agent-provider-preflight-kind-invalid",
        ));
    }
    let ttl = if provider.preflight_ttl_seconds == 0 {
        policy.ttl
    } else {
        provider.preflight_ttl_seconds
    };
    if ttl < 1 || policy.ttl_cap.is_some_and(|v| ttl > v) {
        return Err(AgentLaunchError::Refused(
            "agent-preflight-ttl-outside-saved-policy",
        ));
    }
    let started = Instant::now();
    let checked = SystemTime::now();
    let stamp = |v: SystemTime| -> Result<String, AgentLaunchError> {
        let d = v
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AgentLaunchError::caused("agent-preflight-clock", e))?;
        crate::format_utc_timestamp(
            i64::try_from(d.as_secs())
                .map_err(|e| AgentLaunchError::caused("agent-preflight-clock", e))?,
            i64::from(d.subsec_nanos()),
        )
        .map_err(|e| AgentLaunchError::caused("agent-preflight-clock", e))
    };
    let until = checked
        .checked_add(Duration::from_secs(ttl as u64))
        .ok_or(AgentLaunchError::Refused("agent-preflight-time-overflow"))?;
    let path = workspace(root, &launch.workspace)?;
    let executable_digest = executable(Path::new(&provider.command))?;
    let directory_identity = host_directory(&path, policy.minimum_free_bytes)?;
    let EnvironmentObservation {
        names: environment_names,
        digests: environment,
    } = environment(policy, provider)?;
    let provider_json = crate::encode_model(provider)
        .map_err(|e| AgentLaunchError::caused("agent-provider-encoding", e))?;
    let path_text = path
        .to_str()
        .ok_or(AgentLaunchError::Refused("agent-workspace-utf8-required"))?;
    let mut scoped = serde_json::json!({"provider":provider_name,"configuration":hash(&provider_json),"executable":executable_digest,"workspace":path_text,"directory":directory_identity,"environment":environment,"mode":launch.mode});
    if let Some(scope) = model.scope() {
        scoped["native_model_sources"] = scope.into();
    }
    let scope = crate::encode_model(&scoped)
        .map_err(|e| AgentLaunchError::caused("agent-preflight-scope-encoding", e))?;
    let mut checks = vec![PreflightCheck {
        name: "host_observations".into(),
        status: "ready".into(),
        detail: "Host read/write/resources observed; provider capabilities remain unverified"
            .into(),
    }];
    if let Some(scope) = model.scope() {
        checks.push(PreflightCheck {
            name: "model_route_sources".into(),
            status: "declared_unverified".into(),
            detail: scope.into(),
        });
    }
    let preflight = PreflightResult {
        schema_version: 1,
        verdict: "compatible".into(),
        checked_at: stamp(checked)?,
        valid_until: stamp(until)?,
        scope: hash(&scope),
        provider: provider_name.into(),
        workspace: path_text.into(),
        duration_ms: i64::try_from(started.elapsed().as_millis())
            .map_err(|e| AgentLaunchError::caused("agent-preflight-duration", e))?,
        verification: "unverified".into(),
        capabilities: Some(BTreeMap::new()),
        checks: Some(checks),
    };
    Ok(HostPreparation {
        workspace: path,
        environment_names,
        preflight,
        executable_digest,
        directory_identity,
    })
}
/// Recheck host observations without repeating probes inside admission. Refuse
/// changed inode/bytes/environment/path or expired receipt before any mutation.
/// The model source hash is part of the same receipt when routing is present.
/// A matching observation still proves neither provider identity nor permission.
pub(crate) fn recheck(
    root: &Path,
    model: &ModelRoutePreparation,
    launch: &Launch,
    policy: &AgentLaunchPolicy,
    observed: &HostPreparation,
) -> Result<(), AgentLaunchError> {
    let provider = model.provider();
    let actual = workspace(root, &launch.workspace)?;
    let meta = std::fs::metadata(&actual)
        .map_err(|e| AgentLaunchError::caused("agent-workspace-recheck", e))?;
    if actual != observed.workspace
        || (meta.dev(), meta.ino()) != observed.directory_identity
        || executable(Path::new(&provider.command))? != observed.executable_digest
    {
        return Err(AgentLaunchError::Refused("agent-preflight-input-changed"));
    }
    let until = swarm_wire_json::go_rfc3339::parse(&observed.preflight.valid_until)
        .map_err(|e| AgentLaunchError::caused("agent-preflight-time-invalid", e))?;
    let now = crate::system_timestamp()
        .map_err(|e| AgentLaunchError::caused("agent-preflight-clock", e))?;
    if swarm_wire_json::go_rfc3339::parse(&now)
        .map_err(|e| AgentLaunchError::caused("agent-preflight-clock", e))?
        >= until
    {
        return Err(AgentLaunchError::Refused("agent-preflight-expired"));
    }
    // Scope recomposition uses current environment without exposing its values.
    let EnvironmentObservation {
        digests: environment,
        ..
    } = environment(policy, provider)?;
    let provider_json = crate::encode_model(provider)
        .map_err(|e| AgentLaunchError::caused("agent-provider-encoding", e))?;
    let mut scoped = serde_json::json!({"provider":observed.preflight.provider,"configuration":hash(&provider_json),"executable":observed.executable_digest,"workspace":observed.preflight.workspace,"directory":observed.directory_identity,"environment":environment,"mode":launch.mode});
    if let Some(scope) = model.scope() {
        scoped["native_model_sources"] = scope.into();
    }
    let scope = crate::encode_model(&scoped)
        .map_err(|e| AgentLaunchError::caused("agent-preflight-scope-encoding", e))?;
    if hash(&scope) != observed.preflight.scope {
        return Err(AgentLaunchError::Refused("agent-preflight-scope-changed"));
    }
    Ok(())
}
