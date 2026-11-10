//! Caller-owned concurrent role jobs; no lease renewal or independent process authority.
use super::*;
use std::thread::{Scope, ScopedJoinHandle};

/// Structural role identities, not an operator concurrency/attempt ceiling.
/// The conductor owns at most one in-flight call for each existing role; real
/// engine admission, leases and saved provider/budget policies remain mandatory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Role { Controls, Reviewer, Planner }
impl Role {
    /// Deterministic publication order for independent owned role handles.
    pub(super) const ALL: [Self; 3] = [Self::Controls, Self::Reviewer, Self::Planner];
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
/// Other roles may run concurrently; this role is restarted only after its join
/// and checked publication. The conductor remains the sole shared-output writer.
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
            let result = deferred_scope_change(role, &work, result, &mut recorded, &mut output);
            let result = deferred_selection(role, &work, result, recorded.as_ref(), &mut output);
            Report { result, output, recorded }
        })
        .map_err(|error| phase("mission-watch-role-thread", error))
}
/// Publish a live settled pre-exec scope change without discarding its refusal.
/// Only the real producer's private marker qualifies; no label matching, replay,
/// compound cleanup, provider exit or printed error can synthesize this outcome.
/// Preserve actual committed state and counts. Reobservation is not permission:
/// any next activation still passes the engine's current guards and budgets.
fn deferred_scope_change(role: Role, work: &str, result: Result<(), RootError>,
    recorded: &mut Option<(String, i64)>, output: &mut Vec<u8>) -> Result<(), RootError> {
    let Err(error) = result else { return result; };
    if role != Role::Planner || !output.is_empty()
        || recorded.as_ref().is_some_and(|(id, _)| id != work) { return Err(error); }
    let Some((known, claim, scope, revision)) = error.recorded_planning_scope_change()
        .filter(|(known, _, _, _)| *known == work) else { return Err(error); };
    let (known, claim, scope) = (known.to_owned(), claim.to_owned(), scope.to_owned());
    if recorded.as_ref().is_none_or(|(id, old)| id == &known && *old < revision) {
        *recorded = Some((known.clone(), revision));
    }
    let wrapped = CliError::Root(Box::new(error));
    let causes = crate::safe_causes(&wrapped);
    match emit(output, &json!({"schema_version":1,"event":"mission-role-observation-stale",
        "phase":"planner","work":known,"claim_id":claim,"scope":scope,
        "recorded_revision":revision,"causes":causes,"claim_recorded":true,
        "provider_attempted":false,"budget_consumption_preserved":true,
        "fresh_observation_required":true,"provider_retry_authorized":false,
        "mission_complete":false,"observation_is_permission":false})) {
        Ok(()) => Ok(()),
        Err(publication) => {
            let CliError::Root(original) = wrapped else { unreachable!("constructed Root wrapper"); };
            Err(join(Some(*original), publication))
        }
    }
}
/// Reobserve only a planner selection rejected by the exact pre-mutation
/// revision guard after successful rollback and checked peer cleanup. In this
/// producer every error after a recorded claim is WorkRecorded; any compound,
/// output, recorded state or other role refuses classification. A newer revision
/// is a changed precondition, not authority to retry a provider or reset budgets.
/// Emit the retained cause and explicit no-claim/no-provider facts; output failure
/// preserves both causes. The next saved-cadence pass rechecks every real guard.
fn deferred_selection(
    role: Role,
    work: &str,
    result: Result<(), RootError>,
    recorded: Option<&(String, i64)>,
    output: &mut Vec<u8>,
) -> Result<(), RootError> {
    let Err(error) = result else { return result; };
    let revisions = if role == Role::Planner && recorded.is_none() && output.is_empty() {
        match &error {
            RootError::Mutation(swarm_engine::WorkEngineError::Guard(
                swarm_engine::MutationGuardError::Revision { expected, observed: Some(observed) }
            )) if *expected > 0 && observed > expected => Some((*expected, *observed)),
            _ => None,
        }
    } else { None };
    let Some((expected, observed)) = revisions else { return Err(error); };
    let wrapped = CliError::Root(Box::new(error));
    let causes = crate::safe_causes(&wrapped);
    let publication = emit(output, &json!({"schema_version":1,"event":"mission-role-observation-stale",
        "phase":"planner","work":work,"expected_revision":expected,"observed_revision":observed,
        "causes":causes,"claim_attempted":true,"claim_recorded":false,
        "provider_attempted":false,"fresh_observation_required":true,
        "provider_retry_authorized":false,"mission_complete":false,
        "observation_is_permission":false}));
    match publication {
        Ok(()) => Ok(()),
        Err(publication) => {
            let CliError::Root(original) = wrapped else { unreachable!("constructed Root wrapper"); };
            Err(join(Some(*original), publication))
        }
    }
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
/// Before conductor release, wake all owned roles once and join every handle.
/// Classify each clean recorded user interruption before aggregating failures;
/// two clean stops must not become an unclassifiable compound failure.
/// Publication and independent failures remain retained while later peers drain.
/// A real observed stop retains its actual reason; a main error requests only local
/// ControllerShutdown. Independent errors, failed wake/close/output remain errors.
/// This never stops or waits for separately granted worker processes.
pub(super) fn finish(
    handles: Vec<ScopedJoinHandle<'_, Report>>,
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
    let stopping_cleanly = main.is_ok();
    let stop = main.as_ref().ok().copied();
    let mut failure = main.err();
    for handle in handles {
        if let Err(error) = publish(joined(handle), work, recorded, out) {
            let observed = if stopping_cleanly {
                interrupted_phase(error, work, signals, recorded, out).map(|_| ())
            } else {
                Err(error)
            };
            if let Err(error) = observed { failure = Some(join(failure, error)); }
        }
    }
    match failure {
        Some(error) => Err(preserve_planning(recorded.as_ref(), error)),
        None => Ok(stop.expect("successful main retained its stop")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// All protocol roles are distinct; publication order grants no authority.
    #[test]
    fn role_identities_cover_each_phase_once() {
        let roles: std::collections::BTreeSet<_> = Role::ALL.into_iter().collect();
        assert_eq!(roles.len(), Role::ALL.len());
        assert_eq!(Role::ALL, [Role::Controls, Role::Reviewer, Role::Planner]);
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
                    vec![handle],
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

    /// Three scoped peers cross one barrier before collective join. This proves
    /// owned thread overlap/publication only, not provider or role admission.
    #[test]
    fn concurrent_roles_overlap_and_publish_every_receipt() {
        let mut signals = Signals::new().unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(Role::ALL.len()));
        let mut recorded = None;
        let mut output = Vec::new();
        let stop = std::thread::scope(|scope| {
            let handles = Role::ALL.into_iter().enumerate().map(|(i, _)| {
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    Report { result: Ok(()), output: format!("{{\"peer\":{i}}}\n").into_bytes(), recorded: Some(("work".into(), i as i64 + 1)) }
                })
            }).collect();
            finish(handles, Ok(Stop::ControllerShutdown), "work", &mut signals, &mut recorded, &mut output).unwrap()
        });
        assert_eq!(stop, Stop::ControllerShutdown);
        assert_eq!(String::from_utf8(output).unwrap(), "{\"peer\":0}\n{\"peer\":1}\n{\"peer\":2}\n");
        assert_eq!(recorded, Some(("work".into(), 3)));
        signals.close().unwrap();
    }

    /// A panic and independent failures never hide later peer output or its
    /// highest recorded revision. Synthetic reports grant no engine authority.
    #[test]
    fn concurrent_role_failures_drain_siblings_and_preserve_all_causes() {
        let mut signals = Signals::new().unwrap();
        let mut recorded = None;
        let mut output = Vec::new();
        let error = std::thread::scope(|scope| {
            let panic = scope.spawn(|| -> Report { std::panic::panic_any(7_u32) });
            let failed = scope.spawn(|| Report { result: Err(RootError::Refused("fixture-review")), output: b"review\n".to_vec(), recorded: Some(("work".into(), 3)) });
            let later = scope.spawn(|| Report { result: Err(RootError::WorkRecorded { primary: Box::new(RootError::Refused("fixture-planner")), work: "work".into(), revision: 6 }), output: b"planner\n".to_vec(), recorded: Some(("work".into(), 4)) });
            finish(vec![panic, failed, later], Err(RootError::Refused("fixture-main")), "work", &mut signals, &mut recorded, &mut output).unwrap_err()
        });
        assert_eq!(output, b"review\nplanner\n");
        assert_eq!(recorded, Some(("work".into(), 6)));
        assert_eq!(error.recorded_work(), Some(("work", 6)));
        let causes = crate::safe_causes(&CliError::Root(Box::new(error)));
        for code in ["fixture-main", "mission-watch-owned-job-panicked", "fixture-review", "fixture-planner"] {
            assert!(causes.iter().any(|v| v.contains(code)), "missing {code}");
        }
        signals.close().unwrap();
    }

    /// Recognize each synthetic clean planner receipt before aggregation. The
    /// cached signal is local test state, not an actual OS/provider stop proof.
    #[test]
    fn multiple_clean_role_receipts_do_not_become_compound_failure() {
        let mut signals = Signals::new().unwrap();
        signals.stopped = Some(Stop::Interrupt);
        let mut recorded = None;
        let mut output = Vec::new();
        let stop = std::thread::scope(|scope| {
            let handles = [2_i64, 5_i64].into_iter().map(|revision| scope.spawn(move || Report {
                result: Err(RootError::WorkRecorded { primary: Box::new(RootError::Refused("planning-execution-interrupted")), work: "work".into(), revision }),
                output: Vec::new(), recorded: None,
            })).collect();
            finish(handles, Ok(Stop::Interrupt), "work", &mut signals, &mut recorded, &mut output).unwrap()
        });
        assert_eq!(stop, Stop::Interrupt);
        assert_eq!(recorded, Some(("work".into(), 5)));
        let rows: Vec<serde_json::Value> = output.split(|b| *b == b'\n').filter(|v| !v.is_empty()).map(|v| serde_json::from_slice(v).unwrap()).collect();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|v| v["event"] == "mission-planning-interrupted" && v["reason"] == "sigint"));
        signals.close().unwrap();
    }
    /// The genuine pre-claim revision guard is the valid deferral witness.
    /// No role, cleanup, uncertain/recorded result or stale direction is guessed.
    #[test]
    fn preclaim_revision_observation_defers_without_provider_retry() {
        fn conflict(expected: i64, observed: Option<i64>) -> RootError {
            RootError::Mutation(swarm_engine::WorkEngineError::Guard(
                swarm_engine::MutationGuardError::Revision { expected, observed }))
        }
        let mut output = Vec::new();
        deferred_selection(Role::Planner, "work", Err(conflict(2, Some(3))), None, &mut output).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(value["event"], "mission-role-observation-stale");
        assert_eq!(value["work"], "work");
        assert_eq!(value["expected_revision"], 2);
        assert_eq!(value["observed_revision"], 3);
        assert_eq!(value["claim_recorded"], false);
        assert_eq!(value["provider_attempted"], false);
        assert_eq!(value["provider_retry_authorized"], false);
        assert!(value["causes"].as_array().unwrap().iter().any(|v| v == "revision_conflict"));
        for role in [Role::Controls, Role::Reviewer] {
            assert!(deferred_selection(role, "work", Err(conflict(2, Some(3))), None, &mut Vec::new()).is_err());
        }
        for (expected, observed) in [(2,Some(1)),(2,Some(2)),(-1,Some(2)),(2,None)] {
            assert!(deferred_selection(Role::Planner, "work", Err(conflict(expected,observed)), None, &mut Vec::new()).is_err());
        }
        assert!(deferred_selection(Role::Planner, "work", Err(conflict(2,Some(3))), Some(&("work".into(),2)), &mut Vec::new()).is_err());
        assert!(deferred_selection(Role::Planner, "work", Err(conflict(2,Some(3))), None, &mut b"existing output".to_vec()).is_err());
        for error in [RootError::Refused("revision_conflict"), RootError::WorkRecorded{primary:Box::new(conflict(2,Some(3))),work:"work".into(),revision:2}, RootError::Cleanup{primary:Some(Box::new(conflict(2,Some(3)))),cleanup:Box::new(RootError::Refused("fixture-close"))}] {
            assert!(deferred_selection(Role::Planner, "work", Err(error), None, &mut Vec::new()).is_err());
        }
    }

    /// Printed refusal text/recorded revision is insufficient to create the
    /// private live marker. Existing bytes, roles and known state stay intact.
    #[test]
    fn scope_change_adapter_refuses_unmarked_or_compound_failures() {
        for role in Role::ALL {
            for bytes in [vec![], b"already-published".to_vec()] {
                let mut output=bytes.clone();
                let mut recorded=Some(("work".into(),9));
                let error=RootError::WorkRecorded{work:"work".into(),revision:7,
                    primary:Box::new(RootError::Refused("planning-provider-gate-refused"))};
                let result=deferred_scope_change(role,"work",Err(error),&mut recorded,&mut output);
                assert!(result.is_err());
                assert_eq!(output,bytes);
                assert_eq!(recorded,Some(("work".into(),9)));
            }
        }
    }

}
