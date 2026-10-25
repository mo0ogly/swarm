//! Real result observations for ordinary unstarted or terminal tasks.
//!
//! This is not MissionStatus, readiness, process liveness or acceptance. Reads
//! canonical Work/settings/Agents and actual freshness under the existing material
//! association guard. Stopped failed/interrupted Agents reuse real fallback
//! diagnostics and docs report scans. Completed/submitted results reuse actual
//! stored report paths without a scan. Completed/accepted history also consumes
//! actual gate/dependency freshness; terminal completion never implies acceptance.
//! Ordinary completed/blocked results consume actual attribution or explicit
//! incomplete-delivery absence. Ordinary abandonment retains terminal history
//! without report/review consumption. Active/non-passed-review branches refuse. No SQL writes, provider, retry or policy is added.
//! The initial/final Agent ownership checks reuse the retained-input reader's
//! shared authoritative verifier. The public fixture tests here contain no Agent;
//! they do not establish corrupt-Agent rejection or an Agent admission path.
use super::{finished, RootStore, RootStoreError as E};
use crate::mission_status_contract::ResultPresentation;
use crate::mission_status_tasks::{result_presentation, ReportListing};

/// Local observations plus original semantic stale-evidence causes. Deliberately
/// neither Serialize nor Debug: callers must explicitly select public fields and
/// traverse causes safely. A returned result is never an acceptance or run grant.
pub struct MissionResultObservation {
    pub results: Vec<(String, ResultPresentation)>,
    pub causes: Vec<E>,
}

/// Retain already observed semantic failures when a later projection/coherence
/// error prevents returning the sidecar. Uses the established aggregate error
/// tree; secondary links do not claim an IO cleanup or rollback took place.
fn retain_causes(mut primary: E, causes: Vec<E>) -> E {
    for cause in causes {
        primary = E::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cause),
        };
    }
    primary
}

/// Reattach sidecar causes only after the complete external material guard has
/// finished. A late path/binding/reservation failure cannot discard the original
/// semantic evidence. This grouping proves neither a real close fault nor rollback.
fn finish_material_result(
    result: Result<MissionResultObservation, E>,
    mut causes: Vec<E>,
) -> Result<MissionResultObservation, E> {
    match result {
        Ok(mut observation) => {
            observation.causes.append(&mut causes);
            Ok(observation)
        }
        Err(error) => Err(retain_causes(error, causes)),
    }
}

impl RootStore {
    /// Observe result presentations in stored task order for an existing Work.
    ///
    /// Reads actual Work, complete saved settings and complete Agent history;
    /// each initial/final Agent body must match its authoritative SQL ownership
    /// columns through the shared verifier before history decisions/comparison.
    /// Ordinary blocked tasks with actual failed/interrupted history use the
    /// terminal classifier and report scan. Submitted tasks with the latest real
    /// completed Agent consume their retained report without scanning docs.
    /// Accepted tasks with a completed or absent Agent consume actual freshness;
    /// stale evidence never becomes validated from the retained status alone.
    /// Earlier failed/interrupted/completed history remains visible. Active,
    /// non-passed current-review and hierarchical/managed branches refuse.
    /// Ordinary completed/blocked and abandoned history share the aggregate
    /// admission/observation contract; their separate SDK qualification remains
    /// distinct from public mission status journey evidence.
    /// Current gate/dependency/review evidence uses the existing freshness reader;
    /// known stale causes survive beyond the outer material guard in the
    /// nonserialized sidecar (or final error tree on late refusal), while unknown IO,
    /// codec, policy, association and cleanup errors refuse with original causes.
    /// Terminal reports are scanned under owned docs descriptors; missing docs
    /// means empty only on actual ENOENT. Unused observations remain NotObserved. The pure consumer refuses if a branch needs such evidence.
    ///
    /// Work/settings/Agents are compared before and after, even on projection
    /// failure. Material association and binding/lock closes are checked by the
    /// shared guard. These sequential checks are not an atomic filesystem/SQL
    /// snapshot or proof against an ABA change. No write, reconciliation, process
    /// action, grant or acceptance occurs. Caller still owes Root close and safe
    /// output/flush handling; this API does not implement mission status JSON.
    pub fn mission_status_results(&self, work: &str) -> Result<MissionResultObservation, E> {
        let mut outer_causes = Vec::new();
        let observed = super::material_path::with_material_read(self, || {
            let work = self.read_work(work)?;
            let settings = self.read_settings()?;
            let agents = self.list_agents(&work.id)?;
            for agent in &agents {
                crate::agent_store::verify_agent_ownership(&self.db, agent).map_err(E::Agent)?;
            }
            let result = (|| {
                if work.planning.is_some() {
                    return Err(E::Refused("mission-status-result-planning-unported"));
                }
                // Preserve the existing no-Agent result API for every state.
                // Only new terminal history uses the narrower aggregate scope.
                if !agents.is_empty() {
                    super::mission_status_reports::supported(&work, &agents)?;
                }
                let diagnostics = super::mission_status_reports::terminal_diagnostics(&agents)?;
                let observations =
                    super::mission_status_reports::agent_observations(&agents, &diagnostics)?;
                let tasks = work.tasks.as_deref().unwrap_or(&[]);
                let mut evidence =
                    super::mission_status_evidence::read_freshness(self, &work, &settings)?;
                let mut causes = Vec::new();
                for facts in evidence.values_mut() {
                    causes.append(&mut facts.causes);
                }
                let projected = (|| {
                    let mut results = Vec::with_capacity(tasks.len());
                    for task in tasks {
                        let facts = evidence
                            .remove(&task.id)
                            .ok_or(E::Refused("mission-status-result-freshness-missing"))?;
                        let agent = observations.iter().find(|a| a.agent.task_id == task.id);
                        let inputs = super::mission_status_reports::observed_result_inputs(
                            &self.root, &work, task, agent.map(|a| a.agent),
                            &settings, &mut causes, facts,
                        )?;
                        results.push((task.id.clone(), result_presentation(task, agent, &inputs)?));
                        if let ReportListing::Observed(paths) = inputs.reports {
                            if super::mission_status_reports::task_reports(&self.root, &task.id)?
                                != paths
                            {
                                return Err(E::Refused("mission-status-reports-changed"));
                            }
                        }
                    }
                    if !evidence.is_empty() {
                        return Err(E::Refused("mission-status-result-freshness-extra"));
                    }
                    Ok(results)
                })();
                match projected {
                    Ok(results) => Ok(MissionResultObservation { results, causes }),
                    Err(error) => Err(retain_causes(error, causes)),
                }
            })();
            // All three comparisons run independently; a failed first read cannot
            // hide a settings/Agent read or cleanup failure after projection.
            let work_check = self.read_work(&work.id).and_then(|now| {
                if now == work {
                    Ok(())
                } else {
                    Err(E::Refused("mission-status-result-work-changed"))
                }
            });
            let settings_check = self.read_settings().and_then(|now| {
                if now == settings {
                    Ok(())
                } else {
                    Err(E::Refused("mission-status-result-settings-changed"))
                }
            });
            let agents_check = self.list_agents(&work.id).and_then(|now| {
                for agent in &now {
                    crate::agent_store::verify_agent_ownership(&self.db, agent)
                        .map_err(E::Agent)?;
                }
                if now == agents {
                    Ok(())
                } else {
                    Err(E::Refused("mission-status-result-agents-changed"))
                }
            });
            let checked = finished(finished(work_check, settings_check), agents_check);
            match (result, checked) {
                (Ok(mut observation), Ok(())) => {
                    // The external helper may still refuse in post-path checks
                    // or close. Keep causes outside its discardable Ok value.
                    outer_causes.append(&mut observation.causes);
                    Ok(observation)
                }
                (Ok(observation), Err(error)) => Err(retain_causes(error, observation.causes)),
                (Err(error), checked) => finished(Err(error), checked),
            }
        });
        finish_material_result(observed, outer_causes)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::{SystemEnvironment, Work};
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

    /// Own a fresh inactive Root; retain failures for diagnosis, never touch user
    /// state or execute a provider. Mode0700 is fixture attribution, not policy.
    struct Fixture(PathBuf);
    impl Fixture {
        /// Use the existing identifier source and create only a fresh directory.
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(crate::generate_identifier("status-results-").unwrap());
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        /// Remove this fixture only after success; retain original failed state.
        fn drop(&mut self) {
            if std::thread::panicking() {
                eprintln!("status results fixture retained: {}", self.0.display());
            } else {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }
    /// Supply only the public event/revision envelope and explicit fixture data.
    fn request(event: &str, revision: i64, mut fields: serde_json::Value) -> Vec<u8> {
        fields["schema_version"] = 1.into();
        fields["event_id"] = event.into();
        fields["expected_revision"] = revision.into();
        serde_json::to_vec(&fields).unwrap()
    }
    /// Create actual Work through public writes without a launch profile/Agent.
    fn create(store: &mut RootStore) -> Work {
        store.execute_external("", "work.create", &request("results-create", 0, serde_json::json!({
            "title":"Result reader", "objective":"Observe retained result", "scope":"Owned inactive fixture", "criteria":["No mutation"]
        })), &mut SystemEnvironment).unwrap().result.work
    }
    /// Add an ordinary todo task through the actual public API, without seeding.
    fn task(store: &mut RootStore, work: &Work) -> Work {
        store.execute_external(&work.id, "task.add", &request("results-task", work.revision, serde_json::json!({
            "id":"task", "title":"Read result", "deliverable":"Report", "criteria":["No acceptance"]
        })), &mut SystemEnvironment).unwrap().result.work
    }
    /// Compare complete public retained Work/settings/Agents/events after reads.
    fn unchanged(
        store: &RootStore,
        work: &Work,
        settings: &crate::EngineSettingsDocument,
        events: &[crate::RecordedEvent],
    ) {
        assert!(store.read_work(&work.id).unwrap() == *work);
        assert!(store.settings().unwrap() == *settings);
        assert!(store.list_agents(&work.id).unwrap().is_empty());
        assert!(store.events(&work.id).unwrap().as_slice() == events);
    }
    /// A real empty Work control precedes a public unknown-Work refusal and the
    /// same valid recovery. No empty result is accepted for the missing Work.
    #[test]
    fn mission_status_results_empty_unknown_refusal_and_recovery() {
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
        let work = create(&mut store);
        let settings = store.settings().unwrap();
        let events = store.events(&work.id).unwrap();
        let observed = store.mission_status_results(&work.id).unwrap();
        assert!(observed.results.is_empty() && observed.causes.is_empty());
        assert!(matches!(
            store.mission_status_results("missing-owned-status-work"),
            Err(E::Work(crate::RecordedWorkError::Missing))
        ));
        let recovered = store.mission_status_results(&work.id).unwrap();
        assert!(recovered.results.is_empty() && recovered.causes.is_empty());
        unchanged(&store, &work, &settings, &events);
        store.close().unwrap();
    }
    /// A real todo task needs neither profile nor invented report/review scans;
    /// repeat observation retains exactly the pure not-started result and state.
    #[test]
    fn mission_status_results_todo_without_profile_is_observed_without_mutation() {
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
        let work = create(&mut store);
        let work = task(&mut store, &work);
        let settings = store.settings().unwrap();
        let events = store.events(&work.id).unwrap();
        assert!(work.launch_profile.is_none());
        let observed = store.mission_status_results(&work.id).unwrap();
        assert_eq!(observed.results.len(), 1);
        assert_eq!(observed.results[0].0, "task");
        assert_eq!(observed.results[0].1.state, "not_started");
        assert_eq!(observed.results[0].1.process_state, "not_started");
        assert_eq!(observed.results[0].1.report_state, "absent");
        assert!(observed.causes.is_empty());
        let recovered = store.mission_status_results(&work.id).unwrap();
        assert!(recovered.results == observed.results && recovered.causes.is_empty());
        unchanged(&store, &work, &settings, &events);
        store.close().unwrap();
    }
    /// Public submit requires a completed native attempt. A made-up Agent cannot
    /// create submitted fixture state; preserve that refusal and valid read recovery.
    #[test]
    fn mission_status_results_public_submit_without_attempt_refuses_and_recovers() {
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
        let work = create(&mut store);
        let work = task(&mut store, &work);
        let settings = store.settings().unwrap();
        let events = store.events(&work.id).unwrap();
        assert_eq!(
            store.mission_status_results(&work.id).unwrap().results[0]
                .1
                .state,
            "not_started"
        );
        let submitted = store.execute_external(&work.id, "task.submit", &request("results-submit", work.revision,
            serde_json::json!({"id":"task", "agent_id":"missing-agent", "report_path":"docs/task.md", "report_sha256":"0".repeat(64)})), &mut SystemEnvironment);
        assert!(matches!(
            submitted,
            Err(E::Mutation(crate::WorkEngineError::TaskReport(
                crate::AgentLaunchError::Cause {
                    code: "task-report-agent-invalid",
                    ..
                }
            )))
        ));
        unchanged(&store, &work, &settings, &events);
        assert_eq!(
            store.mission_status_results(&work.id).unwrap().results[0]
                .1
                .state,
            "not_started"
        );
        store.close().unwrap();
    }
    /// Existing no-Agent states must retain their result behavior when terminal
    /// history support is added. Public abandon and reopen provide a real state
    /// witness without synthesizing accepted history or creating any Agent.
    #[test]
    fn mission_status_results_preserves_no_agent_abandoned_and_reopened_states() {
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
        let work = create(&mut store);
        let work = task(&mut store, &work);
        assert_eq!(
            store.mission_status_results(&work.id).unwrap().results[0]
                .1
                .state,
            "not_started"
        );
        let abandoned=store.execute_external(&work.id,"task.update",&request("abandon",work.revision,serde_json::json!({"id":"task","status":"abandoned","next":"Explicitly retained decision"})),&mut SystemEnvironment).unwrap().result.work;
        let events = store.events(&work.id).unwrap();
        let observed = store.mission_status_results(&work.id).unwrap();
        assert_eq!(observed.results[0].1.state, "abandoned");
        assert_eq!(observed.results[0].1.validation_state, "abandoned");
        assert!(store.read_work(&work.id).unwrap() == abandoned);
        assert!(store.events(&work.id).unwrap() == events);
        let reopened = store
            .execute_external(
                &work.id,
                "task.update",
                &request(
                    "reopen",
                    abandoned.revision,
                    serde_json::json!({"id":"task","status":"todo"}),
                ),
                &mut SystemEnvironment,
            )
            .unwrap()
            .result
            .work;
        assert_eq!(
            store.mission_status_results(&work.id).unwrap().results[0]
                .1
                .state,
            "not_started"
        );
        assert!(store.read_work(&work.id).unwrap() == reopened);
        assert!(store.list_agents(&work.id).unwrap().is_empty());
        store.close().unwrap();
    }
    /// A valid publicly recorded gate precedes altered-artifact evidence and
    /// recovery. Staleness stays an observation with its original typed cause,
    /// not a global refusal or a successful empty report scan.
    #[test]
    fn mission_status_results_stale_gate_cause_survives_and_recovers() {
        use sha2::{Digest, Sha256};
        let fixture = Fixture::new();
        let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
        let work = create(&mut store);
        let work = task(&mut store, &work);
        let task = &work.tasks.as_ref().unwrap()[0];
        let proof = b"owned result proof";
        fs::write(fixture.0.join("proof.txt"), proof).unwrap();
        let checks: Vec<_> = task.plan_checks.as_ref().unwrap().iter().map(|(id, phase)| serde_json::json!({
            "id":id,"gate":phase,"domain":"quality","mandatory":true,"severity":"minor","penalty":1,"max_penalty":100
        })).collect();
        let results: Vec<_> = checks
            .iter()
            .map(|check| {
                serde_json::json!({
                    "id":check["id"],"status":"PASS","count":0,"evidence":["proof.txt"]
                })
            })
            .collect();
        let document = serde_json::json!({"method_version":"3","scope_id":task.id,
            "artifacts":{"proof.txt":format!("{:x}", Sha256::digest(proof))},"domains":{"quality":1},"checks":checks,"results":results});
        let saved = store.execute_external(&work.id, "gate", &request("results-gate", work.revision,
            serde_json::json!({"task_id":"task","phase":"delivery","name":"Result proof","document":document})), &mut SystemEnvironment).unwrap().result.work;
        let settings = store.settings().unwrap();
        let events = store.events(&saved.id).unwrap();
        let control = store.mission_status_results(&saved.id).unwrap();
        assert!(control.causes.is_empty());
        assert_ne!(control.results[0].1.validation_state, "failed_or_stale");
        fs::write(fixture.0.join("proof.txt"), b"changed owned proof").unwrap();
        let stale = store.mission_status_results(&saved.id).unwrap();
        assert_eq!(stale.results[0].1.validation_state, "failed_or_stale");
        assert!(stale.causes.iter().any(|cause| matches!(cause, E::Gate(_))));
        // This is a simulated final-helper refusal with a REAL Gate cause from
        // the public artifact read above, not an injected path/close IO failure.
        // The success control and recovery exercise both finalizer branches.
        let final_control = finish_material_result(
            Ok(MissionResultObservation {
                results: stale.results,
                causes: Vec::new(),
            }),
            stale.causes,
        )
        .unwrap();
        assert!(final_control
            .causes
            .iter()
            .any(|cause| matches!(cause, E::Gate(_))));
        let late = finish_material_result(
            Err(E::Refused("material-project-instance-changed")),
            final_control.causes,
        );
        assert!(
            matches!(late, Err(E::Cleanup { primary: Some(primary), cleanup })
            if matches!(*primary, E::Refused("material-project-instance-changed")) && matches!(*cleanup, E::Gate(_)))
        );
        unchanged(&store, &saved, &settings, &events);
        fs::write(fixture.0.join("proof.txt"), proof).unwrap();
        let recovered = store.mission_status_results(&saved.id).unwrap();
        assert!(recovered.causes.is_empty() && recovered.results == control.results);
        unchanged(&store, &saved, &settings, &events);
        store.close().unwrap();
    }
}
