//! Local Git report observations. A hash is neither a review verdict nor authority.
use crate::{
    EngineSettingsDocument, EngineSettingsError, LocalPathError, SettingCount, ValidationRunError,
};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt,
    io::{self, Write},
    path::Path,
    process::Command,
    time::Instant,
};

/// Successful observation of a committed blob, with the persisted policy revision.
/// This does not verify reviewer identity, protocol, freshness or acceptance.
#[derive(Debug, serde::Serialize)]
pub struct GitReportObservation {
    pub scope: &'static str,
    pub candidate_commit: String,
    pub report: String,
    pub citation_contract_version: i64,
    pub sha256: String,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub settings_revision: SettingCount,
}
/// Exact private output retained for explicit diagnosis; formatting exposes only sizes.
pub struct GitCommandOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
impl GitCommandOutput {
    /// Move exact owned private streams into a typed failure without copying.
    pub(crate) fn from_parts(stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        Self { stdout, stderr }
    }

    /// Original stdout bytes, possibly partial on failure. This accessor does not redact secrets.
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }
    /// Original stderr bytes; empty when legacy mode merges both streams into stdout.
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}
impl fmt::Debug for GitCommandOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitCommandOutput")
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .finish()
    }
}
/// Git errors retain their source and owned-process recovery without echoing data.
#[derive(Debug)]
pub enum GitReportError {
    Settings(EngineSettingsError),
    Path(LocalPathError),
    Refused(&'static str),
    Process {
        cause: Box<ValidationRunError>,
        output: Box<GitCommandOutput>,
    },
    Command {
        exit_code: Option<i32>,
        timed_out: bool,
        stdout_sha256: String,
        stderr_sha256: String,
        output: Box<GitCommandOutput>,
    },
}
impl GitReportError {
    /// Stable diagnostic reason; output contents and arguments are never included.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Settings(e) => e.reason(),
            Self::Path(_) => "git-repository-path",
            Self::Refused(reason) => reason,
            Self::Process { .. } => "git-process",
            Self::Command {
                timed_out: true, ..
            } => "git-command-timeout",
            Self::Command { .. } => "git-command-failed",
        }
    }
    /// Stable library error family; a refused read is never an empty successful blob.
    pub fn code(&self) -> &'static str {
        "git-report"
    }
}
impl fmt::Display for GitReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "git-report: {}", self.reason())
    }
}
impl Error for GitReportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Settings(e) => Some(e),
            Self::Path(e) => Some(e),
            Self::Process { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Fallible output accumulation with no hidden retention cap or lossy UTF-8 conversion.
#[derive(Default)]
struct Bytes(Vec<u8>);
impl Write for Bytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.try_reserve(bytes.len()).map_err(io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
/// Return Go's TrimSpace byte view, preserving malformed UTF-8 as non-whitespace.
/// Only complete boundary runes are removed; arbitrary interior bytes remain exact.
fn trimmed(mut bytes: &[u8]) -> &[u8] {
    loop {
        let width = (1..=4.min(bytes.len())).find(|&n| {
            std::str::from_utf8(&bytes[..n])
                .is_ok_and(|s| s.chars().count() == 1 && crate::text::trim_space(s).is_empty())
        });
        match width {
            Some(n) => bytes = &bytes[n..],
            None => break,
        }
    }
    loop {
        let width = (1..=4.min(bytes.len())).find(|&n| {
            std::str::from_utf8(&bytes[bytes.len() - n..])
                .is_ok_and(|s| s.chars().count() == 1 && crate::text::trim_space(s).is_empty())
        });
        match width {
            Some(n) => bytes = &bytes[..bytes.len() - n],
            None => break,
        }
    }
    bytes
}
/// Read and hash `candidate:report` from a local bare Git directory beneath `root`.
///
/// The supplied settings must be a valid journal with explicitly adopted v5 Git
/// policy: zero expires immediately and null has no deadline. Version 2 hashes
/// stdout exactly; legacy 0/1 merge stderr and apply Go TrimSpace. Other versions,
/// non-full hexadecimal object IDs and non-relative Git paths are refused before
/// spawning. The fixed blob read uses no shell, pager, hooks or text conversion.
/// Git environment overrides are removed so they cannot redirect the named store.
///
/// Output is accumulated fallibly without truncation or a hidden policy ceiling;
/// memory therefore follows output size. The shared Linux runner owns pidfd,
/// process group, draining and cleanup. Errors preserve causes/cleanup and never
/// return the hash of partial output as success. No Work or settings mutation is
/// performed. Directory observation is not OS confinement or an authorization
/// check, and this function establishes no independent review or later-action lease.
pub fn hash_git_report(
    root: &Path,
    repository: &str,
    candidate: &str,
    report: &str,
    version: i64,
    settings: &EngineSettingsDocument,
) -> Result<GitReportObservation, GitReportError> {
    use GitReportError as E;
    settings.validate().map_err(E::Settings)?;
    let timeout = settings.values.git_command_timeout().map_err(E::Settings)?;
    if !matches!(version, 0..=2) {
        return Err(E::Refused("git-citation-version-unsupported"));
    }
    if !matches!(candidate.len(), 40 | 64) || !candidate.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(E::Refused("git-candidate-object-id-required"));
    }
    if report.is_empty()
        || report.starts_with('/')
        || report
            .split('/')
            .any(|p| p.is_empty() || matches!(p, "." | ".."))
        || report.contains('\0')
    {
        return Err(E::Refused("git-report-relative-path-required"));
    }
    let directory = crate::local_directory(root, repository).map_err(E::Path)?;
    let deadline = timeout
        .map(|duration| {
            Instant::now()
                .checked_add(duration)
                .ok_or(E::Refused("git-deadline-representation"))
        })
        .transpose()?;
    let mut command = Command::new("git");
    command
        .args([
            "--git-dir=.",
            "--no-pager",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "show",
            "--no-ext-diff",
            "--no-textconv",
            "--end-of-options",
        ])
        .arg(format!("{candidate}:{report}"))
        .current_dir(directory);
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            command.env_remove(key);
        }
    }
    command.env("GIT_TERMINAL_PROMPT", "0");
    let mut stdout = Bytes::default();
    let mut stderr = Bytes::default();
    let process = crate::validation_run::run_owned_command(
        command,
        deadline,
        &mut stdout,
        if version == 2 {
            Some(&mut stderr)
        } else {
            None
        },
    );
    let process = match process {
        Ok(process) => process,
        Err(cause) => {
            return Err(E::Process {
                cause: Box::new(cause),
                output: Box::new(GitCommandOutput {
                    stdout: stdout.0,
                    stderr: stderr.0,
                }),
            })
        }
    };
    if process.timed_out || !process.usage.status.success() {
        return Err(E::Command {
            exit_code: process.usage.status.code(),
            timed_out: process.timed_out,
            stdout_sha256: format!("{:x}", Sha256::digest(&stdout.0)),
            stderr_sha256: format!("{:x}", Sha256::digest(&stderr.0)),
            output: Box::new(GitCommandOutput {
                stdout: stdout.0,
                stderr: stderr.0,
            }),
        });
    }
    let raw = if version == 2 {
        stdout.0.as_slice()
    } else {
        trimmed(&stdout.0)
    };
    Ok(GitReportObservation {
        scope: "git-blob-only",
        candidate_commit: candidate.into(),
        report: report.into(),
        citation_contract_version: version,
        sha256: format!("{:x}", Sha256::digest(raw)),
        stdout_bytes: stdout.0.len() as u64,
        stderr_bytes: stderr.0.len() as u64,
        settings_revision: settings.revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Boundary trimming never repairs malformed UTF-8 or alters interior bytes.
    #[test]
    fn historical_trim_preserves_go_byte_boundaries() {
        for pad in [
            " ", "\r\n", "\u{0085}", "\u{00a0}", "\u{1680}", "\u{200a}", "\u{2028}", "\u{202f}",
            "\u{205f}", "\u{3000}",
        ] {
            let mut raw = pad.as_bytes().to_vec();
            raw.extend_from_slice(b"\xff inside \xf0");
            raw.extend_from_slice(pad.as_bytes());
            assert_eq!(trimmed(&raw), b"\xff inside \xf0");
            assert_eq!(trimmed(pad.as_bytes()), b"");
        }
        for raw in [
            b"\x1c value \x1c".as_slice(),
            "\u{feff} x \u{feff}".as_bytes(),
            "\u{200b} x \u{200b}".as_bytes(),
            b"\0 x \0",
        ] {
            assert_eq!(trimmed(raw), raw);
        }
    }
}
