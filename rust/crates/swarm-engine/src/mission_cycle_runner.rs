//! Keep a real conductor lease alive while one caller-owned operation blocks.
use super::{bound, caused, MissionLease, RootStore, RootStoreError, E};
use std::{
    error::Error,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

/// Local heartbeat-thread observation, never lease freshness or launch authority.
/// After startup, false means the heartbeat exited; true does not replace any
/// authoritative guard. This cannot construct a lease or renew one from JSON.
pub struct MissionCycleMonitor(AtomicBool);
impl MissionCycleMonitor {
    /// Observe thread presence without blocking, IO, mutation or permission.
    pub fn is_running(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
/// Publish thread exit even while unwinding; this is local status, not a policy.
struct Running<'a>(&'a MissionCycleMonitor);
impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0 .0.store(false, Ordering::Release);
    }
}

/// Failed cycle with the actual operation result retained, including known success.
/// `None` means the operation never started. Each optional error is an observed
/// cause, not evidence that a transaction rolled back or a process never ran.
/// Inspect all fields: Error::source exposes only the primary cause. No retry,
/// business acceptance or permission follows from possession of this report.
pub struct MissionCycleFailure<T> {
    pub action: Option<Result<T, RootStoreError>>,
    pub supervision: Option<E>,
    pub journal: Option<E>,
}
/// One safely projected cause in a named cycle phase. Paths describe traversal,
/// not authenticated causality. Only static labels and numeric OS/SQLite codes
/// are emitted; an observed WorkRecorded tuple is not proof of a new commit.
/// The action value T and private Display text are deliberately never serialized.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct MissionCycleCause {
    pub phase: &'static str,
    pub path: Vec<String>,
    pub labels: Vec<String>,
    pub io_errno: Option<i32>,
    pub sqlite_code: Option<i32>,
    pub recorded_work: Option<(String, i64)>,
}
/// Diagnostic observation only, with every listed failure branch retained.
/// No callback, IO, retry, acceptance or permission follows from this document.
/// Known successful T remains in the original failure, never inferred as a Work.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct MissionCycleDiagnostics {
    pub action_started: bool,
    pub action_succeeded: bool,
    pub causes: Vec<MissionCycleCause>,
}
impl<T> MissionCycleFailure<T> {
    /// Project all action/supervision/journal causes through the established
    /// native wrapper matrix, including independent cleanup and custom IO inners.
    /// Preserve each directly observed WorkRecorded identity/revision and path;
    /// do not select an arbitrary latest revision or infer state from successful T.
    /// This pure borrowed inspection leaves the original result/causes intact.
    /// Unknown Error families use source(); their private contents are not
    /// rendered. Repeated trait objects terminate per phase without a hidden cap.
    /// Serialization/output/Root/lease-close failures remain the caller's duty.
    pub fn diagnostics(&self) -> MissionCycleDiagnostics {
        let action = self.action.as_ref().and_then(|v| v.as_ref().err());
        let phases: [(&str, Option<&(dyn Error + 'static)>); 3] = [
            ("action", action.map(|e| e as &dyn Error)),
            ("supervision", self.supervision.as_ref().map(|e| e as &dyn Error)),
            ("journal", self.journal.as_ref().map(|e| e as &dyn Error)),
        ];
        let mut causes = Vec::new();
        for (phase, error) in phases {
            let Some(error) = error else { continue };
            let mut pending = vec![(vec!["root".into()], error)];
            let mut seen: Vec<*const dyn Error> = Vec::new();
            while let Some((path, error)) = pending.pop() {
                let pointer = error as *const dyn Error;
                if seen.iter().any(|prior| std::ptr::eq(*prior, pointer)) {
                    continue;
                }
                seen.push(pointer);
                let links = crate::native_error_links(error);
                let recorded_work = match error.downcast_ref::<RootStoreError>() {
                    Some(RootStoreError::WorkRecorded { work, revision, .. }) =>
                        Some((work.clone(), *revision)),
                    _ => None,
                };
                causes.push(MissionCycleCause {
                    phase, path: path.clone(), labels: links.labels,
                    io_errno: links.io_errno, sqlite_code: links.sqlite_code,
                    recorded_work,
                });
                for (kind, child) in links.children.into_iter().rev() {
                    let mut next = path.clone();
                    next.push(kind.into());
                    pending.push((next, child));
                }
            }
        }
        MissionCycleDiagnostics {
            action_started: self.action.is_some(),
            action_succeeded: self.action.as_ref().is_some_and(Result::is_ok),
            causes,
        }
    }
}
impl<T> fmt::Debug for MissionCycleFailure<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MissionCycleFailure")
            .field("action_started", &self.action.is_some())
            .field(
                "action_succeeded",
                &self.action.as_ref().is_some_and(Result::is_ok),
            )
            .field("supervision_failed", &self.supervision.is_some())
            .field("journal_failed", &self.journal.is_some())
            .finish()
    }
}
impl<T> fmt::Display for MissionCycleFailure<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mission-cycle-failed")
    }
}
impl<T: 'static> Error for MissionCycleFailure<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Some(Err(error)) = &self.action {
            Some(error)
        } else {
            self.supervision
                .as_ref()
                .or(self.journal.as_ref())
                .map(|e| e as &dyn Error)
        }
    }
}

/// Join a heartbeat and its explicitly closed Root without discarding either cause.
/// The peer is opened inside the thread, then checked against the actual lease
/// descriptor/process before signalling readiness. Disconnect stops promptly;
/// timeout means only the persisted cadence elapsed, never a global watchdog.
fn maintain(
    root: &std::path::Path,
    lease: &MissionLease,
    ready: mpsc::Sender<()>,
    stop: mpsc::Receiver<()>,
) -> Result<(), E> {
    let peer = RootStore::open(root, true).map_err(|e| E::Root(Box::new(e)))?;
    let result = (|| {
        bound(&peer, lease)?;
        let cadence = u64::try_from(lease.policy().poll_interval_ms.0)
            .map_err(|e| caused("mission-cycle-cadence-invalid", e))?;
        // A real positive policy has already been validated by the claim.
        lease.policy().validate().map_err(E::Settings)?;
        let pulse = || {
            let now = crate::system_timestamp().map_err(|e| caused("mission-cycle-clock", e))?;
            peer.update_mission_supervision(lease, &now, false)
        };
        pulse()?;
        ready
            .send(())
            .map_err(|e| caused("mission-cycle-ready-disconnected", e))?;
        loop {
            match stop.recv_timeout(Duration::from_millis(cadence)) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                Err(mpsc::RecvTimeoutError::Timeout) => pulse()?,
            }
        }
    })();
    crate::mission_supervision::finish(result, peer.close().map_err(|e| E::Root(Box::new(e))))
}

impl RootStore {
    /// Maintain this actual lease while a single synchronous operation executes.
    /// The Root must have been opened writable. The callback receives the same
    /// Root and lease; it still owes authoritative
    /// per-command and pre-grant checks. A second Root connection on the same
    /// datastore renews at the claim's saved cadence. Startup must acknowledge a
    /// successful pulse before the callback runs. No model, worker or retry starts
    /// unless the callback explicitly starts it; this is not the full dispatcher.
    ///
    /// After the callback, disconnect/join the heartbeat, then record one cycle.
    /// Error retains the callback's exact Result plus heartbeat/close and journal
    /// causes, including success before a late failure. A lost lease is not
    /// reacquired and an already released worker is never forcibly stopped.
    /// Journalling reports this caller-owned operation, not proof of a complete
    /// reconciliation or business decision. Caller retains lease/Root close
    /// obligations. Callback/internal panics
    /// propagate after heartbeat shutdown; no returned-result guarantee on panic.
    pub fn run_mission_cycle<T, F>(
        &mut self,
        lease: &MissionLease,
        operation: F,
    ) -> Result<T, Box<MissionCycleFailure<T>>>
    where
        F: FnOnce(&mut Self, &MissionLease, &MissionCycleMonitor) -> Result<T, RootStoreError>,
    {
        if let Err(error) = bound(self, lease) {
            return Err(Box::new(MissionCycleFailure {
                action: None,
                supervision: Some(error),
                journal: None,
            }));
        }
        if !self.writable {
            return Err(Box::new(MissionCycleFailure {
                action: None,
                supervision: Some(E::Refused("mission-root-read-only")),
                journal: None,
            }));
        }
        let root = self.root.clone();
        let monitor = MissionCycleMonitor(AtomicBool::new(true));
        let (action, supervision) = thread::scope(|scope| {
            let (ready_tx, ready_rx) = mpsc::channel();
            let (stop_tx, stop_rx) = mpsc::channel();
            let observation = &monitor;
            let handle = match thread::Builder::new()
                .name("swarm-mission-heartbeat".into())
                .spawn_scoped(scope, move || {
                    let _running = Running(observation);
                    maintain(&root, lease, ready_tx, stop_rx)
                }) {
                Ok(handle) => handle,
                Err(error) => {
                    return (
                        None,
                        Some(caused("mission-cycle-thread-unavailable", error)),
                    )
                }
            };
            let action = if ready_rx.recv().is_ok() && monitor.is_running() {
                Some(operation(self, lease, &monitor))
            } else {
                None
            };
            drop(stop_tx);
            let supervision = match handle.join() {
                Ok(result) => result.err(),
                Err(payload) => std::panic::resume_unwind(payload),
            };
            (action, supervision)
        });
        let Some(action) = action else {
            return Err(Box::new(MissionCycleFailure {
                action: None,
                supervision: Some(supervision.unwrap_or(E::Refused("mission-cycle-ready-missing"))),
                journal: None,
            }));
        };
        let diagnostic = if supervision.is_some() {
            Some("mission-cycle-supervision-failed")
        } else {
            action.as_ref().err().map(RootStoreError::code)
        };
        let journal = (|| {
            let now = crate::system_timestamp().map_err(|e| caused("mission-cycle-clock", e))?;
            self.record_mission_cycle(lease, &now, diagnostic)
        })()
        .err();
        match (action, supervision, journal) {
            (Ok(value), None, None) => Ok(value),
            (action, supervision, journal) => Err(Box::new(MissionCycleFailure {
                action: Some(action),
                supervision,
                journal,
            })),
        }
    }
}
