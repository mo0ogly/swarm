//! One caller-owned ordered role job; no lease renewal or independent process authority.
use super::*;
use std::thread::{Scope, ScopedJoinHandle};

/// Internal execution order, not a configurable concurrency or attempt ceiling.
/// Controls drain before reviews; reviews drain before a planner activation.
/// `recorded` means the actual producer returned a Work, never guessed eligibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role { Controls, Reviewer, Planner }
impl Role {
    /// Advance only after successful join/publication. Revisit a producing control
    /// or review to drain its next item; an empty phase advances without fake work.
    /// A planner activation always yields back to current controls on the next pass.
    pub(super) fn next(self, recorded: bool) -> Self {
        match (self, recorded) {
            (Self::Controls, true) => Self::Controls,
            (Self::Controls, false) => Self::Reviewer,
            (Self::Reviewer, true) => Self::Reviewer,
            (Self::Reviewer, false) => Self::Planner,
            (Self::Planner, _) => Self::Controls,
        }
    }
    /// Diagnostic thread names are technical labels, never agent identities.
    fn thread_name(self) -> &'static str {
        match self { Self::Controls => "swarm-watch-controls", Self::Reviewer => "swarm-watch-reviewer", Self::Planner => "swarm-watch-planner" }
    }
}

/// Safe public bytes and observed Work survive operation, output and close errors.
/// This private report is constructed by the real producer, never from JSON.
pub(super) struct Report {
    pub(super) result: Result<(), RootError>,
    pub(super) output: Vec<u8>,
    pub(super) recorded: Option<(String, i64)>,
}
/// Opaque original panic payload; formatting never renders its private value.
struct Panic {
    _payload: std::sync::Mutex<Box<dyn std::any::Any + Send>>,
}
impl fmt::Debug for Panic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mission-watch-owned-job-panicked")
    }
}
impl fmt::Display for Panic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
impl Error for Panic {}
/// Convert a caught unwind without rendering the original payload or claiming rollback.
pub(super) fn panicked(payload: Box<dyn std::any::Any + Send>) -> RootError {
    phase(
        "mission-watch-owned-job-panicked",
        Panic {
            _payload: std::sync::Mutex::new(payload),
        },
    )
}
/// Start one selected phase in an owned scoped thread under the actual lease.
/// One real controls/review outcome is buffered; None emits no successful record.
/// The conductor chooses the next phase only after join and checked publication.
/// The Root opens inside that thread and is bound before the role's own writers.
/// No output stream, heartbeat, new policy or process ownership crosses threads.
/// Spawn/open/operation/close failures return in the report; known output survives.
pub(super) fn start<'scope, 'env>(
    scope: &'scope Scope<'scope, 'env>,
    role: Role,
    root: PathBuf,
    work: String,
    lease: &'env MissionLease,
    source: swarm_engine::ExecutionInterrupt,
) -> Result<ScopedJoinHandle<'scope, Report>, RootError> {
    std::thread::Builder::new()
        .name(role.thread_name().into())
        .spawn_scoped(scope, move || {
            let mut output = Vec::new();
            let mut recorded = None;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                RootStore::with_mission_peer(&root, lease, |store| {
                    store.with_execution_interrupt(&source, |store| {
                        // Catch inside the peer callback so explicit close still runs.
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            match role {
                                Role::Controls => {
                                    if let Some(validation) = store.run_next_mission_validation(lease)? {
                                        recorded = Some((validation.work.id.clone(), validation.work.revision));
                                        emit(&mut output, &json!({"schema_version":1,"event":"mission-controls-recorded","work":validation.work.id,"revision":validation.work.revision,"job_id":validation.job_id,"controls_passed":validation.controls_passed,"task_accepted":false,"automatic_reviewer":false,"mission_complete":false}))?;
                                    }
                                    Ok(())
                                }
                                Role::Reviewer => {
                                    if let Some(review) = store.run_next_mission_review(lease)? {
                                        recorded = Some((review.work.id.clone(), review.work.revision));
                                        emit(&mut output, &json!({"schema_version":1,"event":"mission-review-recorded","work":review.work.id,"revision":review.work.revision,"review_id":review.review_id,"review_recorded":review.review_recorded,"verdict_applicable":review.verdict_applicable,"replayed":review.replayed,"task_accepted":review.task_accepted,"mission_complete":false}))?;
                                    }
                                    Ok(())
                                }
                                Role::Planner => {
                                    let observed = store.observe_mission_worker_continuation(lease)
                                        .map_err(|e| phase("mission-watch-planner-context", *e))?;
                                    let pending = observed.entries.iter().any(|e| planning_context_pending(&e.state));
                                    planning_once(store, &work, pending, &mut output, &mut recorded)
                                }
                            }
                        }))
                        .unwrap_or_else(|payload| Err(panicked(payload)))
                    })
                })
            }))
            .unwrap_or_else(|payload| Err(panicked(payload)));
            Report {
                result,
                output,
                recorded,
            }
        })
        .map_err(|error| phase("mission-watch-role-thread", error))
}
/// Join only our actual handle. An unexpected unwind is an error, never completion.
pub(super) fn joined(handle: ScopedJoinHandle<'_, Report>) -> Report {
    handle.join().unwrap_or_else(|payload| Report {
        result: Err(panicked(payload)),
        output: Vec::new(),
        recorded: None,
    })
}
/// Publish the producer's safe JSONL in the main thread. Preserve the latest known
/// same-Work revision even when write/flush or the peer's close failed. No bytes
/// imply acceptance; an error can follow output already visible to its consumer.
pub(super) fn publish(
    report: Report,
    work: &str,
    recorded: &mut Option<(String, i64)>,
    out: &mut dyn Write,
) -> Result<(), RootError> {
    for (known, revision) in report.recorded.iter().map(|(w, r)| (w.as_str(), *r)).chain(
        report
            .result
            .as_ref()
            .err()
            .and_then(RootError::recorded_work),
    ) {
        if known == work
            && recorded
                .as_ref()
                .is_none_or(|(w, r)| w != known || revision > *r)
        {
            *recorded = Some((known.to_owned(), revision));
        }
    }
    let result = if report.output.is_empty() {
        report.result
    } else {
        match out.write_all(&report.output).and_then(|()| out.flush()) {
            Ok(()) => report.result,
            Err(error) => Err(join(
                report.result.err(),
                phase("mission-watch-output", error),
            )),
        }
    };
    result.map_err(|error| preserve_planning(recorded.as_ref(), error))
}
/// Before conductor release, stop and join the owned job, then publish its result.
/// A real observed stop retains its actual reason; a main error requests only local
/// ControllerShutdown. Independent errors, failed wake/close/output remain errors.
/// This never stops or waits for separately granted worker processes.
pub(super) fn finish(
    handle: ScopedJoinHandle<'_, Report>,
    mut main: Result<Stop, RootError>,
    work: &str,
    signals: &mut Signals,
    recorded: &mut Option<(String, i64)>,
    out: &mut dyn Write,
) -> Result<Stop, RootError> {
    let reason = match main.as_ref() {
        Ok(Stop::Interrupt) => swarm_engine::ExecutionInterruptReason::Interrupt,
        Ok(Stop::Terminate) => swarm_engine::ExecutionInterruptReason::Terminate,
        Ok(Stop::ControllerShutdown | Stop::PermissionWithdrawn) => swarm_engine::ExecutionInterruptReason::ControllerShutdown,
        Err(_) => swarm_engine::ExecutionInterruptReason::ControllerShutdown,
    };
    if let Err(wake) = signals.source.request(reason) {
        main = Err(join(
            main.err(),
            phase("mission-watch-role-shutdown", wake),
        ));
    }
    let result = publish(joined(handle), work, recorded, out);
    match (main, result) {
        (Ok(stop), Ok(())) => Ok(stop),
        (Ok(stop), Err(error)) => {
            interrupted_phase(error, work, signals, recorded, out).map(|_| stop)
        }
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(peer)) => Err(join(Some(error), peer)),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Producing phases drain; empty phases advance and planner always yields.
    /// These are ordering transitions, not provider or public publication evidence.
    #[test]
    fn ordered_role_phases_do_not_promote_empty_results_or_skip_receipts() {
        use Role::*;
        assert_eq!(Controls.next(true), Controls);
        assert_eq!(Controls.next(false), Reviewer);
        assert_eq!(Reviewer.next(true), Reviewer);
        assert_eq!(Reviewer.next(false), Planner);
        assert_eq!(Planner.next(true), Controls);
        assert_eq!(Planner.next(false), Controls);
    }
    /// Exercise actual local source and scoped join for main failure, clean
    /// interruption and independent failure. Cached Stop and Work are synthetic;
    /// this is not an OS signal, role publication or process-cleanup proof.
    #[test]
    fn planner_finish_distinguishes_main_shutdown_and_actual_stop_metadata() {
        for (stop, interrupted) in [
            (None, false),
            (Some(Stop::Interrupt), true),
            (Some(Stop::Terminate), false),
            (Some(Stop::PermissionWithdrawn), true),
        ] {
            let mut signals = Signals::new().unwrap();
            signals.stopped = stop;
            let source = signals.source.clone();
            let observed = source.clone();
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
            let peer_barrier = barrier.clone();
            let mut recorded = Some(("work".to_owned(), 1));
            let result = std::thread::scope(|scope| {
                let handle = scope.spawn(move || {
                    peer_barrier.wait();
                    while source.reason().unwrap().is_none() {
                        std::thread::yield_now();
                    }
                    let primary = RootError::Refused(if interrupted {
                        "planning-execution-interrupted"
                    } else {
                        "fixture-peer"
                    });
                    Report {
                        result: Err(RootError::WorkRecorded {
                            primary: Box::new(primary),
                            work: "work".into(),
                            revision: 3,
                        }),
                        output: Vec::new(),
                        recorded: None,
                    }
                });
                barrier.wait();
                finish(
                    handle,
                    stop.map_or_else(|| Err(RootError::Refused("fixture-main")), Ok),
                    "work",
                    &mut signals,
                    &mut recorded,
                    &mut Vec::new(),
                )
            });
            signals.close().unwrap();
            assert_eq!(
                observed.reason().unwrap(),
                Some(match stop {
                    None => swarm_engine::ExecutionInterruptReason::ControllerShutdown,
                    Some(Stop::Interrupt) => swarm_engine::ExecutionInterruptReason::Interrupt,
                    Some(Stop::Terminate) => swarm_engine::ExecutionInterruptReason::Terminate,
                    Some(Stop::ControllerShutdown | Stop::PermissionWithdrawn) => swarm_engine::ExecutionInterruptReason::ControllerShutdown,
                })
            );
            if interrupted {
                assert_eq!(result.unwrap(), stop.unwrap());
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.recorded_work(), Some(("work", 3)));
                let causes = crate::safe_causes(&CliError::Root(Box::new(error)));
                assert!(causes.iter().any(|s| s.contains("fixture-peer")));
                if stop.is_none() {
                    assert!(causes.iter().any(|s| s.contains("fixture-main")));
                }
            }
        }
    }
    /// Pure report/output contract, not role execution or signal evidence.
    #[test]
    fn planner_report_keeps_original_and_latest_work_on_output_failure() {
        struct Sink(bool);
        impl Write for Sink {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                if self.0 {
                    Ok(b.len())
                } else {
                    Err(std::io::ErrorKind::BrokenPipe.into())
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
        }
        let report = || Report {
            result: Err(RootError::WorkRecorded {
                primary: Box::new(RootError::Refused("fixture-planner")),
                work: "work".into(),
                revision: 3,
            }),
            output: b"{}\n".to_vec(),
            recorded: Some(("work".into(), 2)),
        };
        let mut known = None;
        assert!(publish(
            Report {
                result: Ok(()),
                output: b"{}\n".to_vec(),
                recorded: Some(("work".into(), 1))
            },
            "work",
            &mut known,
            &mut Vec::new()
        )
        .is_ok());
        for flush in [false, true] {
            let error = publish(report(), "work", &mut known, &mut Sink(flush)).unwrap_err();
            assert_eq!(error.recorded_work(), Some(("work", 3)));
            let causes = crate::safe_causes(&CliError::Root(Box::new(error)));
            assert!(causes.iter().any(|s| s.contains("fixture-planner")));
            assert!(causes.iter().any(|s| s.contains("BrokenPipe")));
        }
    }
}
