//! Lease-fenced automatic handoff submission. Report bytes and timestamps are
//! observations, never authenticated authorship, quality or task acceptance.
use super::{finished, LocalDirectory, RootStore, RootStoreError as E};
use crate::work_engine::{MutationFence, MutationFenceReceipt};
use crate::{Agent, EngineSettingsDocument, MissionLease, Work, WorkEngineError};
use rusqlite::Connection;
use std::{error::Error, path::Path};

/// Result of inspecting one retained worker, including workers from older watch
/// generations. Waiting observes no submission; Submitted is known recorded
/// Work state, not validation/acceptance. No process is started or signalled.
pub enum MissionReportRelayOutcome {
    NotApplicable,
    Waiting { reason: &'static str },
    Submitted { work: String, revision: i64, agent_id: String, report: String },
}
/// One contained, nonempty report observed after applying the saved clock margin.
#[derive(Debug, PartialEq, Eq)]
struct Report { path: String, sha256: String }
/// Discovery and retention are separate: the read transaction never writes Git.
enum Selection { Report(Report), Waiting(&'static str), Managed }
/// Read a retained immutable handoff, repeating SQL/original attribution first.
/// No copy access, repair, candidate publication or acceptance occurs.
fn managed_report(root: &Path, db: &Connection, work: &Work, agent: &Agent, settings: &EngineSettingsDocument) -> Result<Report, E> {
    crate::managed_repository::verify_worker(root, db, work, agent, settings, crate::managed_repository::WorkerCheck::Observe).map_err(|e| E::Launch(Box::new(e)))?;
    let value = crate::managed_repository::verify_result(root, work, agent, settings).map_err(|e| E::Launch(Box::new(e)))?;
    Ok(Report { path: value.report_path, sha256: value.report_sha256 })
}
/// Keep original file/clock/codec errors, including nested cleanup failures.
fn caused(code: &'static str, error: impl Error + Send + Sync + 'static) -> E {
    E::Mutation(WorkEngineError::Mission(Box::new(crate::MissionSupervisionError::Cause {
        code, cause: Box::new(error),
    })))
}
/// Select an inclusive freshness window without date subtraction overflow. Inputs
/// are observed nanoseconds and an explicitly validated nonnegative i64 setting;
/// constants only convert seconds to nanoseconds. Empty/stale/ambiguous remain
/// distinct; this pure helper establishes no filesystem containment or identity.
fn select(times: &[(String, u64, i128)], started: i128, skew: i64) -> Result<Result<&str, &'static str>, E> {
    if skew < 0 { return Err(E::Refused("mission-report-clock-skew-invalid")); }
    let since = started - i128::from(skew) * 1_000_000_000;
    let mut found = None;
    let mut stale = false;
    for (path, size, modified) in times {
        if *size == 0 { continue; }
        if *modified < since { stale = true; continue; }
        if found.is_some() { return Ok(Err("mission-report-ambiguous")); }
        found = Some(path.as_str());
    }
    Ok(found.ok_or(if stale { "mission-report-stale" } else { "mission-report-missing" }))
}
/// Discover only in the actual attempt workspace, translate to the canonical
/// project and hash bytes. Shared listing/metadata/hash readers retain unknown
/// IO/path/close causes. Sequential observations are not an atomic file snapshot;
/// the mutation fence repeats selection and task.submit rechecks actual bytes.
fn report(root: &Path, agent: &Agent, skew: i64) -> Result<Result<Report, &'static str>, E> {
    let started = swarm_wire_json::go_rfc3339::parse(&agent.started)
        .map_err(|e| caused("mission-report-start-invalid", e))?.unix_timestamp_nanos();
    let workspace = if agent.workspace.is_empty() { root.to_path_buf() } else {
        crate::canonical_project_root(Path::new(&agent.workspace)).map_err(E::Path)?
    };
    if !workspace.starts_with(root) || !workspace.is_dir() {
        return Err(E::Refused("mission-report-workspace-outside-root"));
    }
    let mut times = Vec::new();
    for name in super::mission_status_reports::task_reports(&workspace, &agent.task_id)? {
        let (_, metadata) = crate::gate_evaluation::artifact_metadata(&workspace, &name)
            .map_err(|e| caused("mission-report-metadata", e))?;
        use std::os::unix::fs::MetadataExt;
        let modified = i128::from(metadata.mtime()) * 1_000_000_000 + i128::from(metadata.mtime_nsec());
        times.push((name, metadata.len(), modified));
    }
    let name = match select(&times, started, skew)? { Ok(name) => name, Err(reason) => return Ok(Err(reason)) };
    let relative = workspace.join(name).strip_prefix(root)
        .map_err(|e| caused("mission-report-relative", e))?.to_path_buf();
    let relative = relative.to_str().ok_or(E::Refused("mission-report-path-not-utf8"))?;
    let (_, sha256) = crate::gate_evaluation::artifact_hash(root, relative)
        .map_err(|e| caused("mission-report-hash", e))?;
    Ok(Ok(Report { path: relative.into(), sha256 }))
}
/// Recheck actual completed worker, original automatic claim and durable terminal
/// receipt against the connected datastore. Require live mission consent/mode,
/// pause/archive guards; no historical label alone authorizes the new mutation.
pub(super) fn producer(db: &Connection, directory: &LocalDirectory, work: &Work, agent_id: &str, settings: &EngineSettingsDocument) -> Result<Agent, E> {
    super::mission_producer::completed(db,directory,work,agent_id,settings,super::mission_producer::OriginRequirement::AutomaticWorker)
}
/// Engine-owned fence: current lease plus original worker proof and exact current
/// selection. Settings are reread under the command's existing reservation; a
/// revision change refuses, never silently adopts a new margin mid-submission.
/// The post-stage check recognizes only its exact own intent (event/revision/
/// report), without treating an older delivered result as a fresh submission.
struct RelayFence<'a> { lease: &'a MissionLease, directory: LocalDirectory, root: &'a Path, agent: &'a str, settings_revision: crate::SettingCount, event_id: &'a str, expected_revision: i64, report: &'a Report }
impl MutationFence for RelayFence<'_> {
    fn stage_receipt(&self, db: &Connection, receipt: MutationFenceReceipt<'_>) -> Result<(), WorkEngineError> { self.lease.stage_receipt(db, receipt) }
    fn check(&self, db: &Connection, work: &str, now: &str) -> Result<(), WorkEngineError> {
        self.lease.check(db, work, now)?;
        let result = (|| {
            let settings = RootStore::read_settings_from(&self.directory)?;
            if settings.revision != self.settings_revision { return Err(E::Refused("mission-report-settings-changed")); }
            let saved = crate::read_recorded_work(db, work).map_err(E::Work)?;
            let row = producer(db, &self.directory, &saved, self.agent, &settings)?;
            if let Some(prior)=super::task_validation::latest_submission(db,&saved,&row.task_id).map_err(|e|E::Launch(Box::new(e)))?.filter(|r|r.agent_id==self.agent){
                // WorkEngine repeats the fence after staging this very intent,
                // before commit. Only that exact canonical own submission is
                // allowed here; preflight already excluded historical delivery.
                if prior.event_id!=self.event_id || prior.expected_revision!=self.expected_revision || prior.report_path!=self.report.path || prior.report_sha256!=self.report.sha256 {
                    return Err(E::Refused("mission-report-already-submitted"));
                }
            }
            let observed = if saved.planning.as_deref().is_some_and(|p|p.repository.is_some()) {
                managed_report(self.root, db, &saved, &row, &settings)?
            } else {
                report(self.root, &row, settings.values.report_clock_skew_seconds().map_err(E::Settings)?)?.map_err(E::Refused)?
            };
            if &observed != self.report { return Err(E::Refused("mission-report-selection-changed")); }
            Ok(())
        })();
        result.map_err(|e| WorkEngineError::Mission(Box::new(crate::MissionSupervisionError::Root(Box::new(e)))))
    }
}
impl RootStore {
    /// Submit one real current worker report under the owning mission lease.
    /// The caller supplies only the retained Agent ID; the engine discovers the
    /// report and rechecks lease/consent/original producer/receipt/settings/bytes
    /// inside the canonical task.submit writer, including before commit. Old
    /// watch generations may finish; manual/active/nonblocked/dialogue work is
    /// not relayed. An existing canonical submission by this producer is not
    /// repeated after quality blocks the task, even across watch generations.
    /// Missing/stale/ambiguous first reports wait without mutation. Verified
    /// managed first workers freeze a changed report and complete Git result before
    /// the canonical handoff. Existing result refs recover without the mutable
    /// copy; late errors may leave retained evidence and never imply rollback.
    /// Candidate publication and independent validation remain separate.
    /// Producer/terminal/original attribution checks still precede this waiting
    /// result; corrupt proof or SQL remains an error, not an empty success.
    /// Typed errors retain causes and known recorded Work after late cleanup;
    /// no retry, provider, validation, acceptance or author authentication occurs.
    pub fn relay_mission_report(&mut self, lease: &MissionLease, agent_id: &str) -> Result<MissionReportRelayOutcome, E> {
        let work_id = super::mission_supervision_store::bound_mission_work(self, lease)
            .map_err(|e| caused("mission-report-binding", e))?.to_owned();
        let row = self.read_agent(agent_id)?;
        if row.work_id != work_id { return Err(E::Refused("mission-report-work-mismatch")); }
        if row.status != "completed" || row.role != "worker" || (row.native_mission.is_none() && row.origin != "conducteur") { return Ok(MissionReportRelayOutcome::NotApplicable); }
        let work = self.read_work(&work_id)?;
        let task = work.tasks.iter().flatten().find(|t| t.id == row.task_id).ok_or(E::Refused("mission-report-task-missing"))?;
        if task.status != "blocked" || task.brainstorm { return Ok(MissionReportRelayOutcome::NotApplicable); }
        let settings = self.settings()?;
        let chosen = super::material_path::with_material_read(self, || {
            // Original FIFO and durable terminal receipt readers require an
            // actual coherent SQL snapshot even during non-mutating discovery.
            super::mission_supervision_store::read_transaction(&self.db, || {
                let observation = (|| {
                    let current = crate::read_recorded_work(&self.db, &work_id).map_err(E::Work)?;
                    let row = producer(&self.db, &self.directory, &current, agent_id, &settings)?;
                    // A quality-blocked task still owns its canonical handoff.
                    // New watch generations cannot resubmit the same producer,
                    // even if its old report bytes later change.
                    if super::task_validation::latest_submission(&self.db,&current,&row.task_id).map_err(|e|E::Launch(Box::new(e)))?.is_some_and(|r|r.agent_id==agent_id){return Ok(None);}
                    if current.planning.as_deref().is_some_and(|p|p.repository.is_some()) {
                        // Verify attribution in the coherent read. Retention runs
                        // after this read closes; the writer repeats all guards.
                        crate::managed_repository::verify_worker(&self.root,&self.db,&current,&row,&settings,
                            crate::managed_repository::WorkerCheck::Observe).map_err(|e|E::Launch(Box::new(e)))?;
                        return Ok(Some(Selection::Managed));
                    }
                    report(&self.root, &row, settings.values.report_clock_skew_seconds().map_err(E::Settings)?)
                        .map(|value| Some(match value { Ok(report)=>Selection::Report(report), Err(reason)=>Selection::Waiting(reason) }))
                })();
                observation.map_err(|e| crate::MissionSupervisionError::Root(Box::new(e)))
            }).map_err(|e| caused("mission-report-observation", e))
        })?;
        let chosen = match chosen {
            None=>return Ok(MissionReportRelayOutcome::NotApplicable),
            Some(Selection::Report(value))=>value,
            Some(Selection::Waiting(reason))=>return Ok(MissionReportRelayOutcome::Waiting { reason }),
            Some(Selection::Managed)=> {
                // The snapshot is inert evidence, not permission or Work mutation.
                // Git must not hold the Root read reservation or SQLite snapshot.
                let value = crate::managed_repository::retain_result(&self.root, &work, &row, &settings).map_err(|e| E::Launch(Box::new(e)))?;
                match value {
                    Ok(value)=>Report { path:value.report_path, sha256:value.report_sha256 },
                    Err(reason)=>return Ok(MissionReportRelayOutcome::Waiting { reason }),
                }
            },
        };
        let event = format!("relay-{}", crate::provider_configuration::configuration_digest(format!("{agent_id}/{}/{}",row.attempt_id,chosen.sha256).as_bytes()));
        let request = crate::TaskReportRequest { schema_version: 1, event_id: event, expected_revision: work.revision, id: task.id.clone(), agent_id: agent_id.into(), report_path: chosen.path.clone(), report_sha256: chosen.sha256.clone(), status: "submitted".into() };
        let raw = serde_json::to_vec(&request).map_err(|e| caused("mission-report-request", e))?;
        let root = self.root.clone();
        let fence = RelayFence { lease, directory: self.directory.duplicate().map_err(E::Storage)?, root: &root, agent: agent_id, settings_revision: settings.revision, event_id: &request.event_id, expected_revision: request.expected_revision, report: &chosen };
        let result = self.execute_with_fence(&work_id, "task.submit", &raw, &mut crate::SystemEnvironment, Some(&fence));
        let recorded = result.as_ref().ok().map(|v| (v.result.work.id.clone(),v.result.work.revision));
        let result = finished(result, fence.directory.close().map_err(E::Storage));
        let outcome = match (result, recorded) {
            (Err(primary), Some((work, revision))) => return Err(E::WorkRecorded { primary: Box::new(primary), work, revision }),
            (Err(error), _) => return Err(error),
            (Ok(outcome), _) => outcome,
        };
        Ok(MissionReportRelayOutcome::Submitted { work: outcome.result.work.id, revision: outcome.result.work.revision, agent_id: agent_id.into(), report: chosen.path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn candidate(name: &str, size: u64, nanos: i128) -> (String, u64, i128) { (name.into(), size, nanos) }
    #[test]
    fn mission_report_selection_inclusive_margin_and_strict_zero() {
        let t = 10_000_000_000;
        let rows = vec![candidate("docs/t.md", 1, t - 2_000_000_000)];
        assert_eq!(select(&rows, t, 2).unwrap(), Ok("docs/t.md"));
        assert_eq!(select(&rows, t, 0).unwrap(), Err("mission-report-stale"));
        let exact = vec![candidate("docs/t.md", 1, t)];
        assert_eq!(select(&exact, t, 0).unwrap(), Ok("docs/t.md"));
        let newer = vec![candidate("docs/t.md", 1, t + 1)];
        assert_eq!(select(&newer, t, 0).unwrap(), Ok("docs/t.md"));
    }
    #[test]
    fn mission_report_selection_empty_stale_ambiguity_and_recovery() {
        let valid = vec![candidate("docs/t.md", 1, 10)];
        assert_eq!(select(&valid, 10, 0).unwrap(), Ok("docs/t.md"));
        let empty = vec![candidate("docs/t.md", 0, 10)];
        assert_eq!(select(&empty, 10, 0).unwrap(), Err("mission-report-missing"));
        let mut ambiguous = valid.clone(); ambiguous.push(candidate("docs/t-handoff.md", 1, 10));
        assert_eq!(select(&ambiguous, 10, 0).unwrap(), Err("mission-report-ambiguous"));
        ambiguous[1].2 = 9;
        assert_eq!(select(&ambiguous, 10, 0).unwrap(), Ok("docs/t.md"));
    }
    #[test]
    fn mission_report_selection_full_setting_range_and_invalid_input() {
        let rows = vec![candidate("docs/t.md", 1, -1)];
        assert_eq!(select(&rows, 10, i64::MAX).unwrap(), Ok("docs/t.md"));
        assert!(select(&rows, 10, -1).is_err());
    }
    #[test]
    fn mission_report_settings_absence_is_not_default_adoption() {
        let mut values = crate::EngineSettingsValues::installation_defaults().unwrap();
        let schema = values.schema_version();
        assert!(schema >= 40);
        assert_eq!(values.report_clock_skew_seconds().unwrap(), 2);
        values.agent_report_clock_skew_seconds = None;
        values.validate().unwrap(); assert_eq!(values.schema_version(), schema);
        assert!(values.report_clock_skew_seconds().is_err());
        let raw = serde_json::to_vec(&values).unwrap();
        let again: crate::EngineSettingsValues = serde_json::from_slice(&raw).unwrap();
        assert!(again.agent_report_clock_skew_seconds.is_none());
    }
    #[test]
    fn mission_report_settings_zero_and_independent_display_policy() {
        let mut values = crate::EngineSettingsValues::installation_defaults().unwrap();
        values.agent_report_clock_skew_seconds = Some(crate::SettingCount(0));
        values.validate().unwrap(); assert_eq!(values.report_clock_skew_seconds().unwrap(), 0);
        values.work_view_status_recent_agents = None;
        // A supplied partial display group still refuses its own invalid shape.
        assert!(values.validate().is_err());
        values.work_view_status_change_events = None;
        // Historical display absence is independent of explicit report policy.
        values.validate().unwrap();
        assert_eq!(values.report_clock_skew_seconds().unwrap(), 0);
        assert!(values.mission_status_view_limits().is_err());
    }
    #[test]
    fn mission_report_settings_reject_noncanonical_and_null() {
        let initial = serde_json::to_value(crate::EngineSettingsValues::installation_defaults().unwrap()).unwrap();
        for invalid in [serde_json::json!("-1"),serde_json::json!("02"),serde_json::json!("+2"),serde_json::Value::Null] {
            let mut value = initial.clone(); value["agent_report_clock_skew_seconds"] = invalid;
            assert!(serde_json::from_value::<crate::EngineSettingsValues>(value).is_err());
        }
    }
}
