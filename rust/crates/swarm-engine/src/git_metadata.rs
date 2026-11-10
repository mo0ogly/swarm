//! Checked Git display observations. No snapshot, actor authentication or authority.
#![cfg(target_os = "linux")]
use crate::{EngineSettingsDocument, GitCommandOutput, GitReportError, GitState, SettingCount};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    ffi::OsString,
    fmt,
    io::{self, Write},
    os::unix::ffi::OsStringExt,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GitMetadataField {
    Head,
    Branch,
    Changes,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GitMetadataDiagnostic {
    pub field: GitMetadataField,
    pub reason: &'static str,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
}
/// Exact output stays only in the typed cause, never ordinary public formatting.
pub struct GitMetadataIssue {
    pub field: GitMetadataField,
    pub cause: GitReportError,
}
impl fmt::Debug for GitMetadataIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitMetadataIssue")
            .field("field", &self.field)
            .field("reason", &self.cause.reason())
            .finish()
    }
}
/// Per-command observations, not a consistent repository snapshot or cleanliness proof.
pub struct GitMetadataObservation {
    pub state: GitState,
    pub diagnostics: Vec<GitMetadataDiagnostic>,
    pub issues: Vec<GitMetadataIssue>,
    pub settings_revision: SettingCount,
    pub external_filters_disabled: bool,
}
impl fmt::Debug for GitMetadataObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitMetadataObservation")
            .field("issue_count", &self.issues.len())
            .finish_non_exhaustive()
    }
}
pub enum GitMetadataError {
    Report(Box<GitReportError>),
    Text(crate::RuntimeError),
}
impl GitMetadataError {
    /// Safe family; the original typed cause retains any private output.
    pub fn code(&self) -> &'static str {
        "git-metadata"
    }
    /// Stable safe discriminant, excluding configuration values and filenames.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Report(e) => e.reason(),
            Self::Text(_) => "git-output-encoding",
        }
    }
}
impl fmt::Debug for GitMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for GitMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl Error for GitMetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Report(e) => Some(e.as_ref()),
            Self::Text(e) => Some(e),
        }
    }
}
#[derive(Default)]
struct Bytes(Vec<u8>);
impl Write for Bytes {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.0.try_reserve(b.len()).map_err(io::Error::other)?;
        self.0.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct Output {
    run: crate::validation_run::OwnedProcessRun,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
impl Output {
    /// Move original failed output into its checked cause, without allocating a copy.
    fn failure(self) -> GitReportError {
        GitReportError::Command {
            exit_code: self.run.usage.status.code(),
            timed_out: self.run.timed_out,
            stdout_sha256: format!("{:x}", Sha256::digest(&self.stdout)),
            stderr_sha256: format!("{:x}", Sha256::digest(&self.stderr)),
            output: Box::new(GitCommandOutput::from_parts(self.stdout, self.stderr)),
        }
    }
}
fn wrap(error: GitReportError) -> GitMetadataError {
    GitMetadataError::Report(Box::new(error))
}
/// Own/drain/reap a fixed metadata command under one explicit monotonic deadline.
/// Arguments are not interpreted by a shell; inherited Git overrides/traces and
/// optional index writes, prompts, hooks and fsmonitor execution are disabled.
fn run(
    root: &Path,
    args: &[&str],
    overrides: &[OsString],
    timeout: Option<Duration>,
) -> Result<Output, GitMetadataError> {
    let deadline = timeout
        .map(|d| {
            Instant::now()
                .checked_add(d)
                .ok_or_else(|| wrap(GitReportError::Refused("git-deadline-representation")))
        })
        .transpose()?;
    let mut command = Command::new("git");
    command.args([
        "--no-pager",
        "--no-optional-locks",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.fsmonitor=false",
    ]);
    for value in overrides {
        command.arg("-c").arg(value);
    }
    command.args(args).current_dir(root);
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            command.env_remove(key);
        }
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    let (mut stdout, mut stderr) = (Bytes::default(), Bytes::default());
    let process =
        crate::validation_run::run_owned_command(command, deadline, &mut stdout, Some(&mut stderr));
    let process = match process {
        Ok(p) => p,
        Err(cause) => {
            return Err(wrap(GitReportError::Process {
                cause: Box::new(cause),
                output: Box::new(GitCommandOutput::from_parts(stdout.0, stderr.0)),
            }))
        }
    };
    Ok(Output {
        run: process,
        stdout: stdout.0,
        stderr: stderr.0,
    })
}
/// Read names only (never command values), then neutralize every configured
/// filter family. Raw OS bytes preserve subsection names containing whitespace or
/// non-UTF-8. Missing/invalid protection data refuses before invoking status.
fn filter_overrides(
    root: &Path,
    timeout: Option<Duration>,
) -> Result<Vec<OsString>, GitMetadataError> {
    let listed = run(
        root,
        &[
            "config",
            "--name-only",
            "--null",
            "--get-regexp",
            r"^filter\.",
        ],
        &[],
        timeout,
    )?;
    if listed.run.timed_out {
        return Err(wrap(listed.failure()));
    }
    if listed.run.usage.status.code() == Some(1) && listed.stdout.is_empty() {
        return Ok(vec![]);
    }
    if !listed.run.usage.status.success() {
        return Err(wrap(listed.failure()));
    }
    if !listed.stdout.is_empty() && !listed.stdout.ends_with(&[0]) {
        return Err(wrap(GitReportError::Refused("git-filter-config-framing")));
    }
    let mut names = BTreeSet::new();
    for key in listed.stdout.split(|&b| b == 0).filter(|k| !k.is_empty()) {
        let lower = key.to_ascii_lowercase();
        if !lower.starts_with(b"filter.") {
            return Err(wrap(GitReportError::Refused("git-filter-config-key")));
        }
        let dot = key
            .iter()
            .rposition(|&b| b == b'.')
            .ok_or_else(|| wrap(GitReportError::Refused("git-filter-config-key")))?;
        if dot > b"filter.".len() {
            names.insert(key[..dot].to_vec());
        }
    }
    let mut overrides = Vec::new();
    for base in names {
        for suffix in [
            b".clean=".as_slice(),
            b".process=",
            b".smudge=",
            b".required=false",
        ] {
            let mut key = base.clone();
            key.extend_from_slice(suffix);
            overrides.push(OsString::from_vec(key));
        }
    }
    Ok(overrides)
}
/// Read the three fixed fields under actual saved per-command policy. Null is
/// explicitly unlimited. No shell or repository-defined filter is executed.
/// Nonzero exit/timeout leaves that field unavailable with diagnostics/original
/// output; infrastructure, protection-query and policy failures return their cause.
/// Separate observations never attest a Git snapshot, cleanliness or actor rights.
pub fn observe_git_metadata(
    root: &Path,
    settings: &EngineSettingsDocument,
) -> Result<GitMetadataObservation, GitMetadataError> {
    settings
        .validate()
        .map_err(|e| wrap(GitReportError::Settings(e)))?;
    let timeout = settings
        .values
        .git_command_timeout()
        .map_err(|e| wrap(GitReportError::Settings(e)))?;
    observe_git_commands(root, timeout, settings.revision)
}
/// Shared lower-level reader. None is only an explicit caller input, never a
/// Root settings fallback; the legacy environment API declares its unbounded scope.
pub(crate) fn observe_git_commands(
    root: &Path,
    timeout: Option<Duration>,
    settings_revision: SettingCount,
) -> Result<GitMetadataObservation, GitMetadataError> {
    let overrides = filter_overrides(root, timeout)?;
    let mut observed = GitMetadataObservation {
        state: GitState::default(),
        diagnostics: vec![],
        issues: vec![],
        settings_revision,
        external_filters_disabled: !overrides.is_empty(),
    };
    for (field, args) in [
        (GitMetadataField::Head, vec!["rev-parse", "HEAD"]),
        (GitMetadataField::Branch, vec!["branch", "--show-current"]),
        (
            GitMetadataField::Changes,
            vec!["status", "--porcelain", "--untracked-files=normal"],
        ),
    ] {
        let output = run(root, &args, &overrides, timeout)?;
        if output.run.timed_out || !output.run.usage.status.success() {
            let diagnostic = GitMetadataDiagnostic {
                field,
                reason: if output.run.timed_out {
                    "git-command-timeout"
                } else {
                    "git-command-failed"
                },
                exit_code: output.run.usage.status.code(),
                timed_out: output.run.timed_out,
                stdout_sha256: format!("{:x}", Sha256::digest(&output.stdout)),
                stderr_sha256: format!("{:x}", Sha256::digest(&output.stderr)),
            };
            observed.diagnostics.push(diagnostic);
            observed.issues.push(GitMetadataIssue {
                field,
                cause: output.failure(),
            });
            continue;
        }
        let value =
            crate::environment::go_command_text(&output.stdout).map_err(GitMetadataError::Text)?;
        match field {
            GitMetadataField::Head => observed.state.head = value,
            GitMetadataField::Branch => observed.state.branch = value,
            GitMetadataField::Changes => observed.state.changes = value,
        }
    }
    Ok(observed)
}
/// Retain issues in the legacy environment contract; no empty field implies success.
pub(crate) fn into_environment(observed: GitMetadataObservation) -> crate::GitObservation {
    crate::GitObservation {
        state: observed.state,
        issues: observed
            .issues
            .into_iter()
            .map(|i| crate::GitIssue {
                field: match i.field {
                    GitMetadataField::Head => crate::GitField::Head,
                    GitMetadataField::Branch => crate::GitField::Branch,
                    GitMetadataField::Changes => crate::GitField::Changes,
                },
                cause: crate::GitIssueCause::CheckedMetadata(Box::new(i.cause)),
            })
            .collect(),
    }
}
