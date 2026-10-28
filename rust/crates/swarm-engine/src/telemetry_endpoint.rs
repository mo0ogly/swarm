//! Local OTLP endpoint record `.swarm/telemetry-endpoint.json` (LIVE-UX-03 E2E-2).
//!
//! `web_bind_address` may request an ephemeral port, so a provider launch
//! cannot derive the receiver address from settings. The running native web
//! server publishes its bound address with its own process identity (PID,
//! kernel start stamp, host tuple); a launch reads it and uses it only while
//! that exact process is still live on this host. A stale record (server gone,
//! PID reused, other host) reads as absent: no token is issued and nothing is
//! exported. The record carries no credential.

use crate::supervisor_identity::{observe_linux_process, SupervisorIdentity};
use crate::telemetry_store::{TelemetryStoreError, telemetry_finished};
use std::error::Error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

/// File name under the Root's `.swarm` directory.
pub const TELEMETRY_ENDPOINT: &str = "telemetry-endpoint.json";

/// Published receiver address and the identity of the publishing process.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryEndpoint {
    pub schema_version: i64,
    /// Loopback `IP:PORT` actually bound by the server.
    pub address: String,
    pub pid: u32,
    pub process_stamp: String,
    pub host: String,
}

fn identity_error(e: crate::SupervisorIdentityError) -> TelemetryStoreError {
    TelemetryStoreError::Io(io::Error::other(e))
}

fn record_path(root: &Path) -> Result<PathBuf, TelemetryStoreError> {
    let root = crate::local_path::canonical_project_root(root).map_err(TelemetryStoreError::Path)?;
    let dir = root.join(".swarm");
    match fs::symlink_metadata(&dir) {
        Ok(m) if m.is_dir() => Ok(dir.join(TELEMETRY_ENDPOINT)),
        Ok(_) => Err(TelemetryStoreError::Refused("telemetry-store-unsafe")),
        Err(e) => Err(TelemetryStoreError::Io(e)),
    }
}

/// Publish `address` for the current process, atomically (private temporary
/// file, fsync, rename): a reader sees the previous record or the complete
/// new one, never a partial write. Only loopback addresses are accepted
/// (`telemetry-endpoint-not-loopback`). Replaces an existing record: one
/// server binds a Root at a time (see `RootBinding`).
pub fn publish_telemetry_endpoint(root: &Path, address: SocketAddr) -> Result<TelemetryEndpoint, TelemetryStoreError> {
    if !address.ip().is_loopback() {
        return Err(TelemetryStoreError::Refused("telemetry-endpoint-not-loopback"));
    }
    let me = SupervisorIdentity::current().map_err(identity_error)?;
    let record = TelemetryEndpoint {
        schema_version: 1,
        address: address.to_string(),
        pid: u32::try_from(me.pid()).map_err(|e| TelemetryStoreError::Io(io::Error::other(e)))?,
        process_stamp: me.process_stamp().to_owned(),
        host: me.host().to_owned(),
    };
    let path = record_path(root)?;
    let temporary = path.with_file_name(format!(".{TELEMETRY_ENDPOINT}.{}", record.pid));
    let bytes = serde_json::to_vec(&record).map_err(|e| TelemetryStoreError::Io(e.into()))?;
    let written = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })();
    if let Err(e) = written {
        // Keep the original cause; a leftover temporary is removed best effort
        // and its own failure is not allowed to replace the primary error.
        let _ = fs::remove_file(&temporary);
        return Err(TelemetryStoreError::Io(e));
    }
    Ok(record)
}

/// Read the published endpoint if its publisher is still the same live process
/// on this host; `Ok(None)` when absent or stale. A record that is not a
/// private regular file of this user, or not decodable, refuses
/// (`telemetry-store-unsafe` / store unavailable) instead of reading as absent.
pub fn read_telemetry_endpoint(
    root: &Path,
) -> Result<Option<TelemetryEndpoint>, TelemetryStoreError> {
    let path = record_path(root)?;
    let directory = crate::local_storage::LocalDirectory::open_owned(
        path.parent().ok_or(TelemetryStoreError::Refused("telemetry-store-directory-missing"))?,
    )
    .map_err(TelemetryStoreError::Storage)?;
    let result = read_endpoint_bound(&directory);
    telemetry_finished(
        result,
        directory.close().map_err(TelemetryStoreError::Storage),
    )
}

/// Read the private endpoint relative to the actually retained directory. Missing
/// records are optional; storage/decode/process errors remain explicit, including
/// checked close errors from LocalDirectory::read. No pathname reopen occurs.
fn read_endpoint_bound(
    directory: &crate::local_storage::LocalDirectory,
) -> Result<Option<TelemetryEndpoint>, TelemetryStoreError> {
    let raw = match directory.read(TELEMETRY_ENDPOINT) {
        Ok(raw) => raw,
        Err(error)
            if error
                .source()
                .and_then(|cause| cause.downcast_ref::<rustix::io::Errno>())
                == Some(&rustix::io::Errno::NOENT) =>
        {
            return Ok(None)
        }
        Err(error) => return Err(TelemetryStoreError::Storage(error)),
    };
    let record: TelemetryEndpoint =
        serde_json::from_slice(&raw).map_err(|e| TelemetryStoreError::Io(e.into()))?;
    if record.schema_version != 1 {
        return Err(TelemetryStoreError::Refused(
            "telemetry-store-version-unsupported",
        ));
    }
    let me = SupervisorIdentity::current().map_err(identity_error)?;
    if record.host != me.host() {
        return Ok(None);
    }
    match observe_linux_process(record.pid) {
        Ok(process) if process.process_stamp() == record.process_stamp => Ok(Some(record)),
        Ok(_) => Ok(None),
        Err(e) if e.confirms_absence() => Ok(None),
        Err(e) => Err(identity_error(e)),
    }
}

/// Remove the record only if it is still exactly `mine` (a later server may
/// have replaced it). Absence is success.
pub fn retract_telemetry_endpoint(root: &Path, mine: &TelemetryEndpoint) -> Result<(), TelemetryStoreError> {
    let path = record_path(root)?;
    let raw = match fs::read(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(TelemetryStoreError::Io(e)),
    };
    if serde_json::from_slice::<TelemetryEndpoint>(&raw).ok().as_ref() != Some(mine) {
        return Ok(());
    }
    match fs::remove_file(&path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(TelemetryStoreError::Io(e)),
        _ => Ok(()),
    }
}

/// Wall-clock milliseconds for token issuance; a clock before the epoch
/// yields 0 (the token's issue time is informational, never a validity bound).
pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Apply the saved v33 retention once, e.g. when the local web server starts,
/// so a Root that receives no new telemetry still drops expired rows. Nothing
/// happens without a console store, before v33, or with unlimited retention;
/// an older store is migrated first (this is a write path). Returns the rows
/// removed. Errors include settings/store failures and `telemetry-store-busy`.
/// Store closure is checked on success and failure, preserving both the primary
/// operation and secondary close causes. Migration/purge may already have
/// committed when a later close fails: Err is not evidence of rollback or no
/// removed rows. On that failure no PurgeReport is returned; callers must retain
/// the error tree and avoid interpreting the missing report as zero consumption.
pub fn apply_saved_retention(root: &Path, settings: &crate::EngineSettingsValues, now_ms: i64) -> Result<crate::PurgeReport, TelemetryStoreError> {
    let days = settings
        .telemetry_retention_policy()
        .map_err(|e| TelemetryStoreError::Io(io::Error::other(e)))?
        .and_then(|(days, _)| days);
    let Some(days) = days else {
        return Ok(crate::PurgeReport::default());
    };
    let mut store = match crate::TelemetryStore::open(root, false) {
        Ok(store) => store,
        Err(TelemetryStoreError::Refused("telemetry-store-missing")) => return Ok(crate::PurgeReport::default()),
        Err(e) => return Err(e),
    };
    let result = store.ensure_current().and_then(|()| store.apply_retention(days, now_ms));
    let closed = store.close();
    telemetry_finished(result,closed)
}

/// Exporter setup for one provider launch: environment entries (may hold the
/// per-attempt secret; only for `execve`) and leading arguments (never a
/// secret, argv is visible locally).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct LaunchTelemetry {
    pub environment: Vec<(String, String)>,
    pub arguments: Vec<String>,
    /// The token INSERT succeeded; retain this observation on later failures.
    /// It grants neither provider execution nor delivery/acceptance.
    pub token_issued: bool,
}

/// Debug output never discloses the per-attempt token or exporter environment.
impl std::fmt::Debug for LaunchTelemetry {
    fn fmt(&self, formatter:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("LaunchTelemetry")
            .field("environment_entries",&self.environment.len())
            .field("argument_entries",&self.arguments.len())
            .field("token_issued",&self.token_issued).finish()
    }
}

/// Merge the actual token write result with checked store closure. A successful
/// INSERT followed by close failure keeps a known TokenIssued marker; paired
/// failures preserve both causes. The secret is never retained in an error.
fn finish_issued_token(issued:Result<String,TelemetryStoreError>,closed:Result<(),TelemetryStoreError>) -> Result<String,TelemetryStoreError> {
    match (issued,closed) {
        (Ok(token),Ok(())) => Ok(token),
        (Ok(_),Err(error)) => Err(TelemetryStoreError::TokenIssued {primary:Box::new(error)}),
        (Err(error),closed) => telemetry_finished(Err(error),closed),
    }
}

/// Exporter setup for one provider launch, or empty when telemetry is not
/// opted in for `provider` (saved v30 list) or no live local server has
/// published an endpoint. Otherwise issues a fresh ingest token for `agent_id`
/// (digest stored in `.swarm/telemetry.db`) and returns:
/// - `claude`: Claude Code's OTLP/JSON exporter variables;
/// - `codex`: the bearer header in `OTEL_EXPORTER_OTLP_HEADERS` (Codex reads it
///   when its `[otel]` exporter sets no header; it does not expand `${VAR}`
///   inside headers) and global `-c otel.exporter=…`, `-c otel.metrics_exporter=…`,
///   `-c otel.trace_exporter=…` (full `/v1/logs`, `/v1/metrics`, `/v1/traces`
///   paths, JSON) and `-c otel.log_user_prompt=false`
///   arguments. Verified on 2026-10-22 against codex-cli 0.163.0 with a local
///   fake model; later Codex versions are not checked.
///
/// Export intervals stay the provider's own defaults (no Swarm policy). The
/// `swarm.agent_id` resource attribute (Claude only) is informational;
/// ingestion attaches by token only. Errors: settings validation, endpoint or
/// store failures are returned so the launch refuses explicitly rather than
/// running silently uninstrumented; an opted-in provider without an exporter
/// recipe is `telemetry-provider-unsupported`.
pub fn launch_telemetry_environment(
    root: &Path,
    settings: &crate::EngineSettingsValues,
    agent_id: &str,
    provider: &str,
    now_ms: i64,
) -> Result<LaunchTelemetry, TelemetryStoreError> {
    // Preserve the standalone API's no-Root-required default-off behavior.
    if !settings
        .telemetry_providers()
        .map_err(|error| TelemetryStoreError::Io(io::Error::other(error)))?
        .iter()
        .any(|entry| entry == provider)
    {
        return Ok(LaunchTelemetry::default());
    }
    let path = record_path(root)?;
    let directory = crate::local_storage::LocalDirectory::open_owned(
        path.parent().ok_or(TelemetryStoreError::Refused("telemetry-store-directory-missing"))?,
    )
    .map_err(TelemetryStoreError::Storage)?;
    let result = launch_in_directory(
        root,
        &directory,
        settings,
        agent_id,
        provider,
        now_ms,
        || {},
    );
    let issued = result.as_ref().is_ok_and(|launch| launch.token_issued);
    telemetry_finished(
        result,
        directory.close().map_err(TelemetryStoreError::Storage),
    )
    .map_err(|error| {
        if issued {
            TelemetryStoreError::TokenIssued {
                primary: Box::new(error),
            }
        } else {
            error
        }
    })
}

/// Launch telemetry using the directory capability of the gate's original Root.
/// The binding must come from actual preparation, never caller JSON. Endpoint
/// reads are descriptor-relative and token SQLite is associated before mutation.
/// The caller still owns binding closure and all later exec/freshness errors.
pub(crate) fn launch_telemetry_environment_bound(
    root: &Path,
    binding: &crate::RootBinding,
    settings: &crate::EngineSettingsValues,
    agent_id: &str,
    provider: &str,
    now_ms: i64,
) -> Result<LaunchTelemetry, TelemetryStoreError> {
    launch_in_directory(
        root,
        binding.directory(),
        settings,
        agent_id,
        provider,
        now_ms,
        || {},
    )
}

/// Shared implementation. The internal hook is a deterministic fixture barrier
/// after the actual endpoint read; production passes a no-op, never a grant or
/// retry policy. Coherent directory substitution refuses before token creation.
fn launch_in_directory(
    root: &Path,
    directory: &crate::local_storage::LocalDirectory,
    settings: &crate::EngineSettingsValues,
    agent_id: &str,
    provider: &str,
    now_ms: i64,
    after_endpoint: impl FnOnce(),
) -> Result<LaunchTelemetry, TelemetryStoreError> {
    let enabled = settings
        .telemetry_providers()
        .map_err(|e| TelemetryStoreError::Io(io::Error::other(e)))?
        .iter()
        .any(|p| p == provider);
    if !enabled {
        return Ok(LaunchTelemetry::default());
    }
    if !matches!(provider, "claude" | "codex") {
        return Err(TelemetryStoreError::Refused(
            "telemetry-provider-unsupported",
        ));
    }
    let Some(endpoint) = read_endpoint_bound(directory)? else {
        return Ok(LaunchTelemetry::default());
    };
    after_endpoint();
    let store = crate::TelemetryStore::open_bound(root, directory, true)?;
    let issued = store.issue_token(agent_id, now_ms);
    let closed = store.close();
    let token = finish_issued_token(issued,closed)?;
    let base = format!("http://{}/api/otel", endpoint.address);
    let header = (
        "OTEL_EXPORTER_OTLP_HEADERS".to_owned(),
        format!("Authorization=Bearer {token}"),
    );
    if provider == "codex" {
        let exporter = |path: &str| {
            format!(r#"{{otlp-http={{endpoint="{base}/v1/{path}",protocol="json"}}}}"#)
        };
        return Ok(LaunchTelemetry {
            environment: vec![header],
            token_issued: true,
            arguments: vec![
                "-c".into(),
                format!("otel.exporter={}", exporter("logs")),
                "-c".into(),
                format!("otel.metrics_exporter={}", exporter("metrics")),
                "-c".into(),
                format!("otel.trace_exporter={}", exporter("traces")),
                "-c".into(),
                "otel.log_user_prompt=false".into(),
            ],
        });
    }
    let environment = [
        ("CLAUDE_CODE_ENABLE_TELEMETRY", "1".to_owned()),
        ("OTEL_METRICS_EXPORTER", "otlp".to_owned()),
        ("OTEL_LOGS_EXPORTER", "otlp".to_owned()),
        // Traces (enhanced telemetry beta; span shapes probed 2026-10-22, no
        // paid call) and prompts explicitly off, whatever the user's shell says.
        ("OTEL_TRACES_EXPORTER", "otlp".to_owned()),
        ("CLAUDE_CODE_ENHANCED_TELEMETRY_BETA", "1".to_owned()),
        ("OTEL_LOG_USER_PROMPTS", "0".to_owned()),
        ("OTEL_EXPORTER_OTLP_PROTOCOL", "http/json".to_owned()),
        ("OTEL_EXPORTER_OTLP_ENDPOINT", base),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .chain([
        header,
        (
            "OTEL_RESOURCE_ATTRIBUTES".to_owned(),
            format!("swarm.agent_id={agent_id}"),
        ),
    ])
    .collect();
    Ok(LaunchTelemetry {
        environment,
        arguments: Vec::new(),
        token_issued: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn root() -> Scratch {
        let path = std::env::temp_dir().join(crate::generate_identifier("endpoint-").unwrap());
        fs::create_dir_all(path.join(".swarm")).unwrap();
        Scratch(path)
    }

    #[test]
    fn publish_read_retract_and_stale_or_unsafe_records() {
        let dir = root();
        assert_eq!(read_telemetry_endpoint(&dir.0).unwrap(), None);
        let address: SocketAddr = "127.0.0.1:4318".parse().unwrap();
        let mine = publish_telemetry_endpoint(&dir.0, address).unwrap();
        assert_eq!(read_telemetry_endpoint(&dir.0).unwrap().as_ref(), Some(&mine));
        let path = dir.0.join(".swarm").join(TELEMETRY_ENDPOINT);
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        // A dead publisher (same host, other start stamp) reads as absent.
        let mut stale = mine.clone();
        stale.process_stamp = "0".into();
        fs::write(&path, serde_json::to_vec(&stale).unwrap()).unwrap();
        assert_eq!(read_telemetry_endpoint(&dir.0).unwrap(), None);
        // Retract leaves a record that is not ours.
        retract_telemetry_endpoint(&dir.0, &mine).unwrap();
        assert!(path.exists());
        fs::write(&path, serde_json::to_vec(&mine).unwrap()).unwrap();
        retract_telemetry_endpoint(&dir.0, &mine).unwrap();
        assert!(!path.exists());
        retract_telemetry_endpoint(&dir.0, &mine).unwrap();
        // Refusals: non-loopback, readable by others, garbage.
        assert!(matches!(
            publish_telemetry_endpoint(&dir.0, "192.0.2.1:80".parse().unwrap()),
            Err(TelemetryStoreError::Refused("telemetry-endpoint-not-loopback"))
        ));
        fs::write(&path, b"{}").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(read_telemetry_endpoint(&dir.0).unwrap_err().code(), "telemetry-store-unsafe");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(read_telemetry_endpoint(&dir.0).is_err(), "undecodable record refuses");
    }

    #[test]
    fn saved_retention_applies_once_and_needs_no_store() {
        let dir = root();
        let mut settings = crate::EngineSettingsValues::installation_defaults().unwrap();
        assert_eq!(apply_saved_retention(&dir.0, &settings, 1).unwrap(), crate::PurgeReport::default(), "no store: nothing to do, nothing created");
        assert!(!dir.0.join(".swarm/telemetry.db").exists());
        let store = crate::TelemetryStore::open(&dir.0, true).unwrap();
        store.issue_token("a", 0).unwrap();
        store.revoke_agent("a", 1).unwrap();
        store.close().unwrap();
        let day = 86_400_000;
        assert_eq!(apply_saved_retention(&dir.0, &settings, 30 * day).unwrap().tokens, 0, "30 days not yet elapsed");
        assert_eq!(apply_saved_retention(&dir.0, &settings, 30 * day + 2).unwrap().tokens, 1);
        settings.agent_telemetry_retention_days = Some(crate::SettingCap(None));
        assert_eq!(apply_saved_retention(&dir.0, &settings, i64::MAX).unwrap(), crate::PurgeReport::default(), "unlimited keeps everything");
    }

    #[test]
    fn launch_environment_is_opt_in_needs_a_live_server_and_issues_a_token() {
        let dir = root();
        let none = LaunchTelemetry::default();
        let mut settings = crate::EngineSettingsValues::installation_defaults().unwrap();
        publish_telemetry_endpoint(&dir.0, "127.0.0.1:4318".parse().unwrap()).unwrap();
        assert_eq!(launch_telemetry_environment(&dir.0, &settings, "a", "claude", 1).unwrap(), none, "default off");
        assert!(!dir.0.join(".swarm/telemetry.db").exists(), "nothing issued while off");
        settings.agent_telemetry_providers = Some(vec!["claude".into()]);
        assert_eq!(launch_telemetry_environment(&dir.0, &settings, "a", "codex", 1).unwrap(), none, "other provider");
        let claude = launch_telemetry_environment(&dir.0, &settings, "a", "claude", 1).unwrap();
        assert!(claude.arguments.is_empty(), "Claude is configured by environment only");
        let get = |k: &str| claude.environment.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()).unwrap();
        assert_eq!(get("OTEL_EXPORTER_OTLP_ENDPOINT"), "http://127.0.0.1:4318/api/otel");
        assert_eq!([get("OTEL_TRACES_EXPORTER"), get("CLAUDE_CODE_ENHANCED_TELEMETRY_BETA"), get("OTEL_LOG_USER_PROMPTS")], ["otlp", "1", "0"], "traces on, prompts explicitly off");
        let token = get("OTEL_EXPORTER_OTLP_HEADERS").strip_prefix("Authorization=Bearer ").unwrap().to_owned();
        let store = crate::TelemetryStore::open(&dir.0, false).unwrap();
        assert_eq!(store.token_agent(&token).unwrap().as_deref(), Some("a"));
        store.close().unwrap();
        // Codex: secret only in the environment; exporter endpoints as global -c overrides.
        settings.agent_telemetry_providers = Some(vec!["codex".into()]);
        let codex = launch_telemetry_environment(&dir.0, &settings, "c", "codex", 1).unwrap();
        assert_eq!(codex.environment.len(), 1);
        let (name, value) = &codex.environment[0];
        let secret = value.strip_prefix("Authorization=Bearer ").unwrap();
        assert_eq!(name, "OTEL_EXPORTER_OTLP_HEADERS");
        assert_eq!(codex.arguments, [
            "-c",
            r#"otel.exporter={otlp-http={endpoint="http://127.0.0.1:4318/api/otel/v1/logs",protocol="json"}}"#,
            "-c",
            r#"otel.metrics_exporter={otlp-http={endpoint="http://127.0.0.1:4318/api/otel/v1/metrics",protocol="json"}}"#,
            "-c",
            r#"otel.trace_exporter={otlp-http={endpoint="http://127.0.0.1:4318/api/otel/v1/traces",protocol="json"}}"#,
            "-c",
            "otel.log_user_prompt=false",
        ]);
        assert!(codex.arguments.iter().all(|a| !a.contains(secret)), "no secret in argv");
        // No live server: nothing injected, no new token.
        fs::remove_file(dir.0.join(".swarm").join(TELEMETRY_ENDPOINT)).unwrap();
        assert_eq!(launch_telemetry_environment(&dir.0, &settings, "b", "codex", 1).unwrap(), none);
        // Invalid saved list refuses the launch explicitly.
        settings.agent_telemetry_providers = Some(vec!["aider".into()]);
        assert!(launch_telemetry_environment(&dir.0, &settings, "a", "claude", 1).is_err());
    }

    /// Count only actual fixture token rows through a real read-only connection.
    /// This never fabricates a permission, digest or successful launch receipt.
    fn token_count(root:&Path) -> i64 {
        let connection=rusqlite::Connection::open_with_flags(root.join(".swarm/telemetry.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW).unwrap();
        let count=connection.query_row("SELECT count(*) FROM tokens",[],|row|row.get(0)).unwrap();
        connection.close().unwrap();
        count
    }

    #[test]
    fn telemetry_bound_coherent_directory_substitution_refuses_before_token_and_recovers() {
        let original=root();
        let replacement=root();
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        publish_telemetry_endpoint(&original.0,listener.local_addr().unwrap()).unwrap();
        publish_telemetry_endpoint(&replacement.0,listener.local_addr().unwrap()).unwrap();
        let mut settings=crate::EngineSettingsValues::installation_defaults().unwrap();
        settings.agent_telemetry_providers=Some(vec!["claude".into()]);
        let control=launch_telemetry_environment(&original.0,&settings,"control","claude",1).unwrap();
        assert!(control.token_issued);
        assert_eq!(token_count(&original.0),1);
        crate::TelemetryStore::open(&replacement.0,true).unwrap().close().unwrap();
        let directory=crate::local_storage::LocalDirectory::open_owned(&original.0.join(".swarm")).unwrap();
        let saved=original.0.join("retained-original-swarm");
        let error=launch_in_directory(&original.0,&directory,&settings,"refused","claude",2,|| {
            fs::rename(original.0.join(".swarm"),&saved).unwrap();
            fs::rename(replacement.0.join(".swarm"),original.0.join(".swarm")).unwrap();
        }).unwrap_err();
        assert_eq!(error.code(),"telemetry-store-instance-changed");
        assert!(!error.token_issued());
        assert_eq!(token_count(&original.0),0,"copy received no token");
        fs::rename(original.0.join(".swarm"),replacement.0.join(".swarm")).unwrap();
        fs::rename(&saved,original.0.join(".swarm")).unwrap();
        assert_eq!(token_count(&original.0),1,"original kept exactly its control token");
        let recovered=launch_in_directory(&original.0,&directory,&settings,"recovered","claude",3,||{}).unwrap();
        assert!(recovered.token_issued);
        assert_eq!(token_count(&original.0),2);
        directory.close().unwrap();
    }

    #[test]
    fn telemetry_issued_close_failure_and_dual_failure_preserve_known_effect_and_causes() {
        let control=finish_issued_token(Ok("fixture-not-a-real-token".into()),Ok(())).unwrap();
        assert_eq!(control,"fixture-not-a-real-token");
        let recorded=finish_issued_token(Ok("fixture-not-a-real-token".into()),
            Err(TelemetryStoreError::Refused("fixture-close"))).unwrap_err();
        assert!(recorded.token_issued());
        assert_eq!(recorded.code(),"telemetry-token-issued");
        assert!(!format!("{recorded:?}").contains("fixture-not-a-real-token"));
        let paired=finish_issued_token(Err(TelemetryStoreError::Refused("fixture-issued")),
            Err(TelemetryStoreError::Refused("fixture-close"))).unwrap_err();
        match paired {
            TelemetryStoreError::Cleanup {primary:Some(primary),cleanup} => {
                assert_eq!(primary.code(),"fixture-issued");
                assert_eq!(cleanup.code(),"fixture-close");
            },
            _ => panic!("lost the secondary close cause"),
        }
        let success=LaunchTelemetry {environment:vec![("HEADER".into(),"fixture-private-token".into())],arguments:vec![],token_issued:true};
        assert!(!format!("{success:?}").contains("fixture-private-token"));
    }

}
