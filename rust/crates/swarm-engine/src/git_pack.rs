//! Disk-streamed fixed Git pack commands. No project admission or ref authority.
#![cfg(target_os = "linux")]
use crate::{EngineSettingsDocument, ValidationRunError};
use std::{
    error::Error,
    fmt,
    fs::File,
    io::{self, Seek},
    os::unix::fs::MetadataExt,
    path::Path,
    process::{ExitStatus, Stdio},
};

/// Fixed no-ref command shape. Export reads revision lines; Import reads a pack.
#[derive(Clone, Copy, Debug)]
pub enum GitPackOperation {
    Export,
    Import,
}
/// Which caller-owned output failed synchronization; no private pathname.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitPackOutput {
    Stdout,
    Stderr,
}
/// One observed output sync failure, retaining its exact channel and IO cause.
/// This metadata is diagnostic only and cannot prove object rollback or retry safety.
pub struct GitPackSyncError {
    output: GitPackOutput,
    cause: io::Error,
}
impl GitPackSyncError {
    /// Output file whose sync_all actually failed, or explicit synthetic unit input.
    pub fn output(&self) -> GitPackOutput {
        self.output
    }
    /// Safe fixed phase label, never a file path or OS error message.
    pub fn code(&self) -> &'static str {
        match self.output {
            GitPackOutput::Stdout => "git-pack-stdout-sync",
            GitPackOutput::Stderr => "git-pack-stderr-sync",
        }
    }
}
impl fmt::Display for GitPackSyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for GitPackSyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for GitPackSyncError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}

/// Observed successful owned command. Object writes are not a business effect receipt.
#[derive(Debug)]
pub struct GitPackRun {
    pub exit_status: ExitStatus,
    pub timed_out: bool,
}
/// Safe envelope retaining process ownership and independent output-sync causes.
/// `may_have_written` is conservative; a refusal never establishes object rollback.
pub struct GitPackError {
    code: &'static str,
    cause: Box<dyn Error + Send + Sync>,
    cleanup: Vec<Box<dyn Error + Send + Sync>>,
    pub may_have_written: bool,
    pub exit_status: Option<ExitStatus>,
    pub timed_out: Option<bool>,
}
impl GitPackError {
    /// Fixed safe label, excluding paths, revision input and Git output.
    pub fn code(&self) -> &'static str {
        self.code
    }
    /// Preserve all primary/cleanup branches for the native error graph.
    pub fn branches(&self) -> Vec<&(dyn Error + 'static)> {
        let mut b = vec![self.cause.as_ref() as &(dyn Error + 'static)];
        b.extend(
            self.cleanup
                .iter()
                .map(|e| e.as_ref() as &(dyn Error + 'static)),
        );
        b
    }
    /// Recover the actual runner's unresolved child/cleanup capability, if any.
    pub fn process_error_mut(&mut self) -> Option<&mut ValidationRunError> {
        self.cause.downcast_mut()
    }
    /// Construct a preflight failure without claiming any process was started.
    fn preflight(code: &'static str, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            code,
            cause: Box::new(cause),
            cleanup: vec![],
            may_have_written: false,
            exit_status: None,
            timed_out: None,
        }
    }
}
impl fmt::Display for GitPackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Debug for GitPackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitPackError")
            .field("code", &self.code)
            .field("may_have_written", &self.may_have_written)
            .finish()
    }
}
impl Error for GitPackError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

/// Run a fixed export/import with input owned by the child and disk output sinks.
/// Input ownership is consumed into child stdin (standard File/Stdio drop);
/// no separate checked-close receipt is claimed for input. Output files remain
/// caller-owned and need their own checked close.
/// All three files must be distinct regular files, positioned at zero; outputs
/// must be empty and writable. Caller owns private creation, file close/recovery,
/// directory capability lifetime, exact revision validation and saved settings.
/// Existing per-command policy supplies the deadline; no size/retry cap is added.
/// Both outputs are synced even on runner/exit failure, preserving independent
/// sync errors. Partial objects/files may remain. The error retains an unreaped
/// child when cleanup fails; the caller must retain dependent cwd capabilities.
/// No Root binding, object format/identity, current grant, durable intention or
/// reference commitment is checked here. Success alone grants no effect authority.
pub fn run_git_pack(
    root: &Path,
    operation: GitPackOperation,
    mut input: File,
    stdout: &mut File,
    stderr: &mut File,
    settings: &EngineSettingsDocument,
) -> Result<GitPackRun, GitPackError> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "git-pack-files");
    let mut identities = vec![];
    for (index, file) in [&mut input, &mut *stdout, &mut *stderr]
        .into_iter()
        .enumerate()
    {
        let m = file
            .metadata()
            .map_err(|e| GitPackError::preflight("git-pack-file-metadata", e))?;
        let position = file
            .stream_position()
            .map_err(|e| GitPackError::preflight("git-pack-file-position", e))?;
        let id = (m.dev(), m.ino());
        if !m.is_file() || position != 0 || (index > 0 && m.len() != 0) || identities.contains(&id)
        {
            return Err(GitPackError::preflight("git-pack-files", invalid()));
        }
        identities.push(id);
    }
    let args: &[&str] = match operation {
        GitPackOperation::Export => &["--git-dir", ".", "pack-objects", "--revs", "--stdout"],
        GitPackOperation::Import => &["--git-dir", ".", "index-pack", "--stdin", "--fix-thin"],
    };
    let (command, deadline) = crate::managed_repository::git_command(root, args, settings)
        .map_err(|e| GitPackError::preflight("git-pack-command", e))?;
    let run = crate::validation_run::run_controlled_command(
        command,
        Stdio::from(input),
        deadline,
        stdout,
        Some(stderr),
        None,
    );
    let result = match run {
        Err(cause) => Err(GitPackError {
            code: "git-pack-process",
            cause: Box::new(cause),
            cleanup: vec![],
            may_have_written: true,
            exit_status: None,
            timed_out: None,
        }),
        Ok(run) => {
            let status = run.usage.status;
            if run.timed_out || !status.success() {
                Err(GitPackError {
                    code: "git-pack-exit",
                    cause: Box::new(io::Error::other("git-pack-unsuccessful")),
                    cleanup: vec![],
                    may_have_written: true,
                    exit_status: Some(status),
                    timed_out: Some(run.timed_out),
                })
            } else {
                Ok(GitPackRun {
                    exit_status: status,
                    timed_out: run.timed_out,
                })
            }
        }
    };
    finish_output_sync(result, [stdout.sync_all(), stderr.sync_all()])
}

/// Compose already observed output sync results with a runner outcome. Both
/// syscalls are evaluated by the caller even if the primary command failed.
/// Preserve status/timeout and every cause with channel identity; never panic or
/// replace a runner failure with a successful empty result. Unit inputs remain
/// synthetic and are not kernel fsync evidence.
fn finish_output_sync(
    mut result: Result<GitPackRun, GitPackError>,
    sync: [io::Result<()>; 2],
) -> Result<GitPackRun, GitPackError> {
    for (output, synced) in [GitPackOutput::Stdout, GitPackOutput::Stderr]
        .into_iter()
        .zip(sync)
    {
        if let Err(cause) = synced {
            let sync = GitPackSyncError { output, cause };
            result = match result {
                Ok(run) => Err(GitPackError {
                    code: "git-pack-sync",
                    cause: Box::new(sync),
                    cleanup: vec![],
                    may_have_written: true,
                    exit_status: Some(run.exit_status),
                    timed_out: Some(run.timed_out),
                }),
                Err(mut error) => {
                    error.cleanup.push(Box::new(sync));
                    Err(error)
                }
            };
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    /// Pure composition witnesses a successful runner, then each sync fault alone
    /// and both together. These supplied IO results do not claim syscall injection.
    #[test]
    fn pack_sync_after_success_preserves_status_and_channels() {
        let success = || {
            Ok(GitPackRun {
                exit_status: ExitStatus::from_raw(0),
                timed_out: false,
            })
        };
        assert!(finish_output_sync(success(), [Ok(()), Ok(())])
            .unwrap()
            .exit_status
            .success());
        for (out, err) in [(true, false), (false, true), (true, true)] {
            let sync = |fail| {
                if fail {
                    Err(io::Error::from_raw_os_error(5))
                } else {
                    Ok(())
                }
            };
            let e = finish_output_sync(success(), [sync(out), sync(err)]).unwrap_err();
            assert_eq!(e.code(), "git-pack-sync");
            assert!(e.may_have_written);
            assert!(e.exit_status.unwrap().success());
            assert_eq!(e.timed_out, Some(false));
            let channels = e
                .branches()
                .into_iter()
                .map(|b| b.downcast_ref::<GitPackSyncError>().unwrap().output())
                .collect::<Vec<_>>();
            let expected = if out && err {
                vec![GitPackOutput::Stdout, GitPackOutput::Stderr]
            } else if out {
                vec![GitPackOutput::Stdout]
            } else {
                vec![GitPackOutput::Stderr]
            };
            assert_eq!(channels, expected);
        }
    }
}
