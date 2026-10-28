//! GNU atomic process creation, not launch approval or a service daemon.
//! Explicit inputs and retained ownership on every post-spawn failure. No PID,
//! PATH, shell or ambient environment fallback; credentials remain caller-owned.
use crate::{kernel, Error, ProcessAnchor};
use nix::sys::signal::SigSet;
use std::{
    error::Error as StdError,
    ffi::{CStr, CString},
    fmt,
    os::fd::BorrowedFd,
};

/// Borrow independently approved inputs. Program/cwd must be absolute and argv
/// nonempty; environment and signal policy are explicit, never inherited by a
/// library default. Byte validation/provenance, identity/budget/cancellation and
/// executable-image approval belong to the authoritative caller before launch.
/// Only the three supplied POSIX streams enter the child; every other FD closes.
/// This capability does not create accounts, change credentials or confine code.
pub struct Spec<'a> {
    pub program: &'a CStr,
    pub arguments: &'a [CString],
    pub environment: &'a [CString],
    pub directory: &'a CStr,
    pub stdin: BorrowedFd<'a>,
    pub stdout: BorrowedFd<'a>,
    pub stderr: BorrowedFd<'a>,
    pub signal_mask: &'a SigSet,
    pub signal_defaults: &'a SigSet,
}
impl fmt::Debug for Spec<'_> {
    /// Never expose private arguments, environment, paths or descriptors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AtomicSpawnSpec { <private> }")
    }
}
/// Observed waitid outcome only; unknown kernel classes never become success.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitKind {
    Exited,
    Signaled,
    CoreDumped,
    Other(i32),
}
/// Retained child exit observation, without PID/identity or business-effect claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Exit {
    pub kind: ExitKind,
    pub status: i32,
}
impl Exit {
    /// Only the actual normal exit0 is success; not proof of the intended effect.
    pub fn success(&self) -> bool {
        self.kind == ExitKind::Exited && self.status == 0
    }
}
/// Independent parent-resource cleanup errors. None means no failure recorded,
/// not proof that a particular resource was created or an operation rolled back.
#[derive(Default)]
pub struct Cleanup {
    pub attributes: Option<Error>,
    pub actions: Option<Error>,
    pub stdin: Option<Error>,
    pub stdout: Option<Error>,
    pub stderr: Option<Error>,
}
impl Cleanup {
    /// Borrow every recorded independent cause without formatting private IO text.
    pub fn errors(&self) -> impl Iterator<Item = &Error> {
        [
            self.attributes.as_ref(),
            self.actions.as_ref(),
            self.stdin.as_ref(),
            self.stdout.as_ref(),
            self.stderr.as_ref(),
        ]
        .into_iter()
        .flatten()
    }
    /// Whether any parent cleanup failure was actually recorded.
    pub fn failed(&self) -> bool {
        self.errors().next().is_some()
    }
}
/// Own the atomically returned process object even when already terminated.
/// No Clone/Serialize or public constructor from caller-selected FD/PID. Drop
/// only closes the descriptor: caller must deliberately wait/reap or transfer
/// responsibility. No kill, retry, approval, namespace freeze or mandate inferred.
pub struct Spawned {
    anchor: ProcessAnchor,
    exit: Option<Exit>,
}
impl fmt::Debug for Spawned {
    /// Hide launch inputs, descriptors, PID and status details.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AtomicSpawned { <private> }")
    }
}
impl Spawned {
    /// Borrow the unchanged atomic process object through matching/business use.
    /// A returned anchor is identity evidence only, not independent launch approval.
    pub fn anchor(&self) -> &ProcessAnchor {
        &self.anchor
    }
    /// Query without waiting, or replay the already retained status. An external
    /// reaper can make waitid fail (cause kept); the held identity is never reused.
    /// None is no exit observed, not a lease or proof of current execution.
    pub fn try_wait(&mut self) -> Result<Option<Exit>, Error> {
        if let Some(exit) = self.exit {
            return Ok(Some(exit));
        }
        let exit = kernel::atomic_spawn::wait(self.anchor.process(), false)?;
        self.exit = exit;
        Ok(exit)
    }
    /// Explicitly wait on this pidfd only, with no arbitrary deadline or implicit
    /// retry. Caller supplies any higher-level cancellation/budget policy. Every
    /// error leaves this owner and prior observation available for recovery.
    pub fn wait(&mut self) -> Result<Exit, Error> {
        if let Some(exit) = self.exit {
            return Ok(exit);
        }
        let exit = kernel::atomic_spawn::wait(self.anchor.process(), true)?
            .ok_or_else(|| Error::refused("atomic-spawn-wait-state"))?;
        self.exit = Some(exit);
        Ok(exit)
    }
    /// Deliberately release only the descriptor, never signal or reap the process.
    /// Calling before wait may leave caller-owned running/zombie work; no automatic
    /// rollback or supervision is claimed. Linux close errors are never retried.
    pub fn close(self) -> Result<(), Error> {
        self.anchor.close()
    }
}
/// Native launch failure retains actual invocation, primary and independent
/// cleanup/validation errors plus any process handle returned by successful
/// creation. invoked+no child is not proof that no transient child/setup existed.
/// Caller owns the retained child on failure; never retry an unknown effect.
pub struct Failure {
    pub cause: Error,
    pub invoked: bool,
    pub child: Option<Spawned>,
    pub cleanup: Cleanup,
    pub validation: Option<Error>,
}
impl Failure {
    /// Static primary class only; inspect all independent branches and child.
    pub fn code(&self) -> &'static str {
        self.cause.code()
    }
    /// Every primary/validation/cleanup branch; Source alone is insufficient.
    pub fn error_branches(&self) -> Vec<&(dyn StdError + 'static)> {
        let mut out: Vec<&(dyn StdError + 'static)> = vec![&self.cause];
        if let Some(e) = &self.validation {
            out.push(e)
        }
        for e in self.cleanup.errors() {
            out.push(e)
        }
        out
    }
}
impl fmt::Display for Failure {
    /// Never format argument/environment/path/FD or nested private OS text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}; invoked={}; child_handle_known={}; cleanup_failed={}",
            self.code(),
            self.invoked,
            self.child.is_some(),
            self.cleanup.failed()
        )
    }
}
impl fmt::Debug for Failure {
    /// Same private-data redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Failure {
    /// Primary original; error_branches exposes independent cleanup/validation.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Turn the complete native attempt into a retained owner before examining any
/// post-spawn error. No liveness rejection may discard a fast/finished child.
fn finish(mut a: kernel::atomic_spawn::Attempt) -> Result<Spawned, Failure> {
    let child = a.process.take().map(|process| Spawned {
        anchor: ProcessAnchor { process },
        exit: None,
    });
    if a.primary.is_none() && a.validation.is_none() && !a.cleanup.failed() {
        if let Some(child) = child {
            return Ok(child);
        }
        return Err(Failure {
            cause: Error::refused("atomic-spawn-descriptor-missing"),
            invoked: a.invoked,
            child: None,
            cleanup: a.cleanup,
            validation: None,
        });
    }
    let cause = a
        .primary
        .take()
        .unwrap_or_else(|| Error::refused("atomic-spawn-post-call-failed"));
    Err(Failure {
        cause,
        invoked: a.invoked,
        child,
        cleanup: a.cleanup,
        validation: a.validation,
    })
}
/// Create a process with its descriptor atomically through native GNU pidfd_spawn.
/// Capability/pidfs/input/parent setup refusal precedes invocation. All supplied
/// streams duplicate then all other child FDs close; cwd/env/signals are explicit.
/// No shell/PATH/legacy PID fallback, credential change or launch authorization.
/// Every post-success cleanup/validation failure retains the process in Failure.
/// Program/file dependency provenance and confinement remain caller obligations.
pub fn launch(spec: &Spec<'_>) -> Result<Spawned, Failure> {
    finish(kernel::atomic_spawn::run(spec))
}

#[cfg(test)]
mod tests;
