//! Server-owned mission sessions, separate from the HTTP Root worker lane.
//! Discovery is read-only; engine consent, lease fencing and budgets own admission.
use super::*;
use crate::mission_watch::{self, Stop};
use std::{collections::BTreeMap, future::Future, path::PathBuf, sync::mpsc, thread};
use swarm_engine::{
    ExecutionInterrupt, ExecutionInterruptReason, RootBinding, RootStore, RootStoreError as E,
};

/// Preserve typed internal failure without formatting paths, prompts or payloads.
fn cause(code: &'static str, error: impl Error + Send + Sync + 'static) -> E {
    E::Runtime(swarm_engine::RuntimeError::new(
        swarm_engine::RuntimeOperation::Platform,
        Context {
            code,
            cause: Box::new(error),
        },
    ))
}
struct Context {
    code: &'static str,
    cause: Box<dyn Error + Send + Sync>,
}
impl std::fmt::Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code)
    }
}
impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl Error for Context {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
/// Keep primary and independent cleanup branches; no failure proves rollback.
fn join(primary: Option<E>, cleanup: E) -> E {
    match primary {
        None => cleanup,
        Some(primary) => E::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        },
    }
}
/// Retain Root causes under the existing web error traversal contract.
fn web(error: E) -> NativeServerError {
    NativeServerError::Service(swarm_web::HttpServiceError::Root {
        operation: "web-conductor",
        cause: Box::new(error),
    })
}

/// Existing consent/mode/archive gates, shared by startup and ongoing discovery.
/// This read-only classification grants no lease, provider or future admission.
fn automatic(control: &swarm_engine::MissionControlView) -> bool {
    !control.archived
        && control.paused == Some(false)
        && control.consent_enabled == Some(true)
        && control.autonomy == Some("autonome")
}
/// Reject known unadoptable legacy leases before HTTP can report readiness.
/// Read-only snapshots preserve every legacy row and do not acquire a lease.
/// Current controls are rechecked by ongoing discovery and engine admission;
/// this startup check is neither future liveness nor business authorization.
/// Clock/read/close causes propagate; paused/manual missions stay untouched.
fn preflight(store: &RootStore) -> Result<(), E> {
    let now = swarm_engine::system_timestamp().map_err(|e| cause("web-conductor-clock", e))?;
    for work in store.list_works()? {
        let control = store
            .mission_control_view(&work.id)
            .map_err(|e| E::Mutation(swarm_engine::WorkEngineError::Mission(Box::new(e))))?;
        if automatic(&control)
            && store
                .read_mission_supervision(&work.id, &now)
                .map_err(|e| E::Mutation(swarm_engine::WorkEngineError::Mission(Box::new(e))))?
                .is_some_and(|v| v.state == swarm_engine::MissionSupervisionState::LegacyUnverified)
        {
            return Err(E::Refused("mission-legacy-lease-requires-migration"));
        }
    }
    Ok(())
}

/// One owned join and its local wakeup, never another process's signal authority.
struct Session<'scope> {
    source: ExecutionInterrupt,
    stop: mpsc::Sender<Stop>,
    handle: thread::ScopedJoinHandle<'scope, Result<bool, CliError>>,
}
impl Session<'_> {
    /// Request interruption before waking the cadence wait. A disconnected local
    /// receiver means the thread already finished; its joined result remains owed.
    fn stop(&self) -> Result<(), E> {
        let wake = self
            .source
            .request(ExecutionInterruptReason::ControllerShutdown)
            .map_err(|e| cause("web-conductor-interrupt", e));
        // Even a failed descriptor wake must not suppress the independent loop stop.
        let _finished = self.stop.send(Stop::ControllerShutdown);
        wake
    }
    /// Join the actual thread; retain opaque panic payload through existing helper.
    fn finish(self) -> Result<bool, E> {
        match self.handle.join() {
            Ok(result) => result.map_err(|e| cause("web-conductor-session", e)),
            Err(payload) => Err(crate::mission_watch::owned_panic(payload)),
        }
    }
}

/// Discover at the current saved mission cadence. Each eligible Work owns at most
/// one local thread, with actual lease CAS deduplicating other servers/CLI watches.
/// Busy claims are re-observed on the next configured tick, never a spawn permission.
/// Discovery reads atomic settings and one SQL observation without contending
/// with Admin's writer reservation; it grants no future lease. Session failure
/// stops admission and drains every owned session; no silent retry.
/// No provider is selected here. drive keeps all current identity/budget/role gates.
/// Session JSONL is an ephemeral observation discarded by this adapter; durable
/// engine history and HTTP status remain authoritative, errors are retained below.
fn run(
    store: &mut RootStore,
    binding: &RootBinding,
    root: &Path,
    executable: &Path,
    stop: &mpsc::Receiver<()>,
    failure_notice: tokio::sync::oneshot::Sender<()>,
) -> Result<(), E> {
    thread::scope(|scope| {
        let mut sessions: BTreeMap<String, Session<'_>> = BTreeMap::new();
        let main = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), E> {
            loop {
                match stop.try_recv() {
                    Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                    Err(mpsc::TryRecvError::Empty) => {}
                }
                let observed = RootStore::open(root, false)?;
                let check = match binding.matches(&observed) {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(E::Refused("web-root-binding-changed")),
                    Err(error) => Err(error),
                };
                let closed = observed.close();
                match (check, closed) {
                    (Ok(()), Ok(())) => {}
                    (Err(error), Ok(())) | (Ok(()), Err(error)) => return Err(error),
                    (Err(error), Err(close)) => return Err(join(Some(error), close)),
                }
                let discovery = store.mission_discovery_observation()?;
                let cadence = mission_watch::cadence(&discovery.policy)?;
                let finished: Vec<_> = sessions
                    .iter()
                    .filter(|(_, s)| s.handle.is_finished())
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in finished {
                    if let Some(session) = sessions.remove(&id) {
                        session.finish()?;
                    }
                }
                for (work_id, control) in discovery.missions {
                    if sessions.contains_key(&work_id) {
                        continue;
                    }
                    if !automatic(&control) {
                        continue;
                    }
                    let source =
                        ExecutionInterrupt::new().map_err(|e| cause("web-conductor-source", e))?;
                    let (sender, receiver) = mpsc::channel();
                    let session_source = source.clone();
                    let id = work_id.clone();
                    let handle = thread::Builder::new()
                        .name("swarm-web-mission".into())
                        .spawn_scoped(scope, move || {
                            let peer = RootStore::open(root, true)
                                .map_err(|e| CliError::Root(Box::new(e)))?;
                            match binding.matches(&peer) {
                                Ok(true) => {}
                                matched => {
                                    let error = matched
                                        .err()
                                        .unwrap_or(E::Refused("web-root-binding-changed"));
                                    return Err(CliError::Root(Box::new(match peer.close() {
                                        Ok(()) => error,
                                        Err(close) => join(Some(error), close),
                                    })));
                                }
                            }
                            mission_watch::owned_session(
                                root,
                                executable,
                                &id,
                                peer,
                                session_source,
                                receiver,
                                &mut io::sink(),
                            )
                        })
                        .map_err(|e| cause("web-conductor-thread", e))?;
                    sessions.insert(
                        work_id,
                        Session {
                            source,
                            stop: sender,
                            handle,
                        },
                    );
                }
                match stop.recv_timeout(cadence) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            }
        }))
        .unwrap_or_else(|payload| Err(mission_watch::owned_panic(payload)));
        let _owner = failure_notice.send(());
        let mut failure = main.err();
        for session in sessions.values() {
            if let Err(error) = session.stop() {
                failure = Some(join(failure, error));
            }
        }
        for (_, session) in sessions {
            if let Err(error) = session.finish() {
                failure = Some(join(failure, error));
            }
        }
        failure.map_or(Ok(()), Err)
    })
}

/// Serve HTTP while a separate owned coordinator discovers consented missions.
/// Caller shutdown, HTTP failure or conductor failure closes admission; both owners
/// drain their own jobs, then checked Root/binding close preserves every failure.
/// Read-only initial discovery refuses known legacy leases before serving HTTP;
/// later state changes/failures still close the service through normal supervision.
/// No synchronous request waits on a mission session. This does not kill granted
/// workers, install the binary, accept a mission or recover managed processes.
pub(super) fn serve<'a, F: Future<Output = ()>>(
    server: NativeHttpServer,
    root: PathBuf,
    shutdown: F,
    observer: &'a mut swarm_web::NativeOutcomeObserver<'a>,
) -> Result<swarm_web::NativeServerReport, NativeServerError> {
    let mut store = match server.bound_mission_root() {
        Ok(store) => store,
        Err(error) => {
            return Err(match server.close() {
                Ok(()) => error,
                Err(cleanup) => NativeServerError::Cleanup {
                    primary: Box::new(error),
                    cleanup: Box::new(cleanup),
                },
            })
        }
    };
    if let Err(error) = preflight(&store) {
        let error = match store.close() {
            Ok(()) => error,
            Err(close) => join(Some(error), close),
        };
        return Err(match server.close() {
            Ok(()) => web(error),
            Err(cleanup) => NativeServerError::Cleanup {
                primary: Box::new(web(error)),
                cleanup: Box::new(cleanup),
            },
        });
    }
    let binding = match store.bind_root() {
        Ok(binding) => binding,
        Err(error) => {
            let error = match store.close() {
                Ok(()) => error,
                Err(close) => join(Some(error), close),
            };
            return Err(match server.close() {
                Ok(()) => web(error),
                Err(close) => NativeServerError::Cleanup {
                    primary: Box::new(web(error)),
                    cleanup: Box::new(close),
                },
            });
        }
    };
    let executable = std::env::current_exe();
    let result = thread::scope(|scope| {
        let executable = match executable {
            Ok(path) => path,
            Err(error) => {
                return Err(super::failed_conductor_start(
                    server,
                    cause("web-conductor-executable", error),
                ))
            }
        };
        let (stop_tx, stop_rx) = mpsc::channel();
        let (failure_tx, failure_rx) = tokio::sync::oneshot::channel();
        let peer_store = &mut store;
        let peer_binding = &binding;
        let coordinator = thread::Builder::new()
            .name("swarm-web-conductor".into())
            .spawn_scoped(scope, move || {
                run(
                    peer_store,
                    peer_binding,
                    &root,
                    &executable,
                    &stop_rx,
                    failure_tx,
                )
            });
        let coordinator = match coordinator {
            Ok(handle) => handle,
            Err(error) => {
                return Err(super::failed_conductor_start(
                    server,
                    cause("web-conductor-spawn", error),
                ))
            }
        };
        let result = server.serve_until(
            async {
                tokio::select! {_=shutdown=>{},_=failure_rx=>{}}
            },
            observer,
        );
        let _finished = stop_tx.send(());
        let cleanup = match coordinator.join() {
            Ok(result) => result,
            Err(payload) => Err(mission_watch::owned_panic(payload)),
        };
        merge(result, cleanup)
    });
    let result = merge(result, store.close());
    merge(result, binding.close())
}
/// Preserve HTTP result and every independent coordinator/descriptor close cause.
fn merge(
    primary: Result<swarm_web::NativeServerReport, NativeServerError>,
    cleanup: Result<(), E>,
) -> Result<swarm_web::NativeServerReport, NativeServerError> {
    match (primary, cleanup) {
        (result, Ok(())) => result,
        (Ok(report), Err(error)) => Err(NativeServerError::StoppedOutput {
            phase: "web-conductor-after-http",
            cause: io::Error::other(error),
            report: Box::new(report),
        }),
        (Err(primary), Err(error)) => Err(NativeServerError::Cleanup {
            primary: Box::new(primary),
            cleanup: Box::new(web(error)),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A late coordinator error must preserve the already collected HTTP report,
    /// known Work inside its cause, and every secondary cleanup failure.
    #[test]
    fn late_conductor_failure_keeps_http_report_and_typed_effects() {
        let report = swarm_web::NativeServerReport { last_auxiliary_effects:Vec::new(),
            completed_jobs: 3,
            ..Default::default()
        };
        let failure = E::WorkRecorded {
            primary: Box::new(E::Refused("fixture-late")),
            work: "late-work".into(),
            revision: 4,
        };
        let native = match merge(Ok(report), Err(failure)) {
            Err(error) => error,
            Ok(_) => panic!("late error lost"),
        };
        match &native {
            NativeServerError::StoppedOutput { report, .. } => assert_eq!(report.completed_jobs, 3),
            _ => panic!("report lost"),
        }
        let cli = super::super::native(native, true);
        assert_eq!(cli.recorded_work(), Some(("late-work".into(), 4)));
        assert!(cli
            .typed_causes()
            .iter()
            .any(|e| matches!(e.downcast_ref::<E>(), Some(E::Refused("fixture-late")))));
    }
    /// Actual error wrappers preserve highest known Work and both cleanup causes;
    /// neither the coordinator nor native HTTP formatting may erase nested effects.
    #[test]
    fn conductor_wrapped_known_work_and_cleanup_reach_cli() {
        let original = CliError::Root(Box::new(E::WorkRecorded {
            primary: Box::new(E::Refused("fixture-original")),
            work: "fixture-work".into(),
            revision: 9,
        }));
        let wrapped = web(join(
            Some(cause("web-conductor-session", original)),
            E::Refused("fixture-close"),
        ));
        let cli = super::super::native(wrapped, true);
        assert_eq!(cli.recorded_work(), Some(("fixture-work".into(), 9)));
        let nodes = cli.typed_causes();
        assert!(nodes
            .iter()
            .any(|e| matches!(e.downcast_ref::<E>(), Some(E::Refused("fixture-original")))));
        assert!(nodes
            .iter()
            .any(|e| matches!(e.downcast_ref::<E>(), Some(E::Refused("fixture-close")))));
    }
}
