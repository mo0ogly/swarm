//! Native task-control preview, protected execution and original-receipt recovery.
//! A preview is no process permit; successful controls and explicit retirement
//! establish neither authenticated human/provider identity nor task acceptance.
mod gate;
mod journal;
mod process;
mod publication;
mod managed_receipt;
mod managed_execution;
mod observation;
use observation::{observe_submission, Observation, RepositoryPurpose};
mod candidate_preparation;
pub use candidate_preparation::TaskValidationCandidate;
mod retirement;
mod review_evidence;
pub(super) use review_evidence::{recorded_evidence, recorded_evidence_for_review_result, recorded_evidence_for_prior_review};
pub use review_evidence::NativeValidationEvidence;
mod runtime;
mod mission_authority;
mod source_authority;
mod source_policy;
mod source_origin;
mod store;
use super::{finished, RootStore, RootStoreError};
use crate::{
    agent_launch::query,
    model_wire::{Fields, Kind},
    AgentLaunchError as E,
};
pub use runtime::TaskValidationOutcome;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, os::unix::fs::MetadataExt, path::Path};

/// Native request names an existing task and optimistic revision only. It cannot
/// supply commands, a producer, an actor or an automatic mission origin.
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub(super) struct Request {
    pub(super) schema_version: i64,
    pub(super) event_id: String,
    pub(super) expected_revision: i64,
    pub(super) task_id: String,
}
/// Strict common command projection; version1 is a protocol invariant, not an
/// operator policy. Invalid names/revision/unknown fields refuse before any IO.
pub(super) fn decode(raw: &[u8]) -> Result<Request, E> {
    const FIELDS: Fields = &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("task_id", Kind::String),
    ];
    let r: Request = crate::work::decode_mode(raw, FIELDS, true)
        .map_err(|e| E::caused("task-validation-request-invalid", e))?;
    if r.schema_version != 1
        || r.expected_revision < 1
        || [&r.event_id, &r.task_id]
            .iter()
            .any(|v| !swarm_identity_contract::name(v.as_bytes()))
    {
        return Err(E::Refused("task-validation-request-invalid"));
    }
    Ok(r)
}
/// Current local observations under one SQL snapshot and saved settings lock.
/// Hashes bind data, not authenticated identity, confinement or future authority.
/// Commands and private producer context are deliberately absent from this DTO.
#[derive(Serialize)]
pub struct TaskValidationPreview {
    pub schema_version: i64,
    pub work_id: String,
    pub work_revision: i64,
    pub task_id: String,
    pub producer_agent_id: String,
    pub attempt_id: String,
    pub settings_revision: crate::SettingCount,
    /// Stored authority source, not proof of a human identity or consent.
    pub authority_mode: &'static str,
    /// Absent for an ordinary task; no planning epoch is fabricated.
    pub scope_generation: Option<crate::SettingCount>,
    pub execution_policy_generation: crate::SettingCount,
    pub authority_event: Option<String>,
    pub authority_revision: Option<crate::SettingCount>,
    pub policy_mode: String,
    pub control_count: usize,
    pub source_sha256: String,
    pub artifacts: BTreeMap<String, String>,
    pub execution_authorized: bool,
    pub task_accepted: bool,
}
/// Private current preparation; saved settings and normalized commands remain
/// coupled to the source observation. No public DTO can supply this executor
/// input or turn its preview into permission. The private gate must recheck it.
struct Prepared {
    preview: TaskValidationPreview,
    policy: crate::ValidationPolicy,
    settings: crate::EngineSettingsDocument,
    original_policy: crate::ValidationPolicy,
    producer_delivery_version: i64,
    managed: Option<managed_execution::Captured>,
}
impl Prepared {
    /// Ordered transport IDs and original normalized control contracts. Managed
    /// cohorts preserve per-task names; no aggregate policy is constructed.
    fn controls(&self) -> Result<Vec<(&str, &crate::ValidationControl)>, E> {
        let controls: Vec<_> = match &self.managed {
            Some(data) => data.candidate.control_plan.as_ref()
                .ok_or(E::Refused("task-validation-managed-plan-required"))?.slots.iter()
                .map(|s| (s.transport_id.as_str(), &s.control)).collect(),
            None => self.policy.controls.as_ref().ok_or(E::Refused("task-validation-controls-required"))?
                .iter().map(|c| (c.id.as_str(), c)).collect(),
        };
        if controls.is_empty() { return Err(E::Refused("task-validation-controls-required")); }
        Ok(controls)
    }
    /// Resolve one checked global ordinal and transport ID without renaming the
    /// original control/result. Negative, missing or changed coordinates refuse.
    fn control(&self, ordinal: i64, transport: &str) -> Result<&crate::ValidationControl, E> {
        let i = usize::try_from(ordinal).map_err(|e| E::caused("task-validation-slot-index", e))?;
        let values = self.controls()?;
        values.get(i).filter(|(id, _)| *id == transport).map(|(_, c)| *c)
            .ok_or(E::Refused("task-validation-control-source-changed"))
    }
    /// Bind the actual complete managed candidate/plan or ordinary policy into
    /// the existing origin domain; no permission or row is created here.
    fn origin(&self, id: &str) -> Result<source_origin::Origin, E> {
        match &self.managed {
            Some(data) => source_origin::Origin::new(id, &self.preview.source_sha256,
                data.candidate.control_plan.as_ref().ok_or(E::Refused("task-validation-managed-plan-required"))?,
                Some(&data.candidate.binding_digest()?)),
            None => source_origin::Origin::new(id, &self.preview.source_sha256, &self.policy, None),
        }
    }
}
/// Reuse authoritative source preparation inside the caller's settings/SQL
/// snapshot. The decoded opaque request cannot supply commands or identity.
/// Reads current files/policies/producer; creates no row, grant or future lease.
pub(super) fn prepare_preview(db: &rusqlite::Connection, root: &Path, settings: &crate::EngineSettingsDocument, work: &str, request: &Request) -> Result<TaskValidationPreview, E> {
    prepare(db, root, settings, work, request).map(|p| p.preview)
}
/// Private helper dispatch; requires the actual Unix channel, registered child
/// and committed permission. It cannot execute from a JSON identity assertion.
/// Typed errors and close failures remain nonzero; no task is accepted.
pub fn execute_validation_gate(
    root: &Path,
    id: &str,
) -> Result<std::convert::Infallible, RootStoreError> {
    process::execute_gate(root, id)
}
/// Exclude a live or unresolved validator from conflicting acceptance, gate or
/// new worker admission. Caller owns the same authoritative mutation transaction;
/// no owner expiry/absence is interpreted as release and SQL causes survive.
pub(crate) fn require_validation_idle(
    db: &rusqlite::Connection,
    work: &str,
    task: &str,
) -> Result<(), E> {
    if store::active(db, work, task)?.is_some() {
        return Err(E::Refused("task-validation-controller-active"));
    }
    Ok(())
}
/// Hash canonical complete data with fallible encoding. The digest carries no
/// secret bytes, permission, actor authenticity or filesystem lease.
fn digest(v: &impl Serialize) -> Result<String, E> {
    let raw = crate::encode_model(v).map_err(|e| E::caused("task-validation-source-encode", e))?;
    Ok(crate::provider_configuration::configuration_digest(&raw))
}
/// Require the latest canonical submission; historical absence remains the
/// original refusal. Optional discovery shares all validation and typed causes.
pub(super) fn submission(db:&rusqlite::Connection,work:&crate::Work,task:&str)->Result<crate::TaskReportRequest,E>{
    latest_submission(db,work,task)?.ok_or(E::Refused("task-validation-submission-required"))
}
/// Read the latest canonical submission without inventing a missing event.
/// Event/request/revision coherence and explicit statement finalization are
/// shared by validation and report relay. None means genuine absence only;
/// malformed newest history refuses rather than selecting older authority.
pub(super) fn latest_submission(
    db: &rusqlite::Connection,
    work: &crate::Work,
    task: &str,
) -> Result<Option<crate::TaskReportRequest>, E> {
    query(db, "SELECT id,revision,request FROM main.events WHERE work_id=? AND kind='task.submit' ORDER BY revision DESC", [&work.id], |rows| {
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let revision: i64 = row.get(1)?;
            let raw: Vec<u8> = row.get(2)?;
            let r = crate::decode_task_report_request(&raw)?;
            if id != r.event_id || r.expected_revision.checked_add(1) != Some(revision) || revision > work.revision {
                return Err(E::Refused("task-validation-submission-inconsistent"));
            }
            if r.id == task { return Ok(Some(r)); }
        }
        Ok(None)
    })
}
/// Prepare current source observations only. Re-read the actual producer's latest
/// completed native attempt, original report and declared control inputs; require
/// current covering authorization and saved command/path/deadline policy. Hash
/// original policy including attribution and relevant saved execution generation.
/// Hierarchical tasks also bind their canonical planning-control epoch; ordinary
/// tasks require no scope metadata and expose absent planning coordinates as null.
/// Unrelated Admin history does not alter authority. Some planning provenance
/// serializes exactly as before, preserving the existing scoped source/2 binding.
/// No model, command, reservation, receipt or Work mutation occurs. SQL/settings
/// ownership belongs to the caller; these sequential file observations are not a
/// filesystem snapshot and are rechecked by the protected execution gate.
fn prepare(
    db: &rusqlite::Connection,
    root: &Path,
    settings: &crate::EngineSettingsDocument,
    work: &str,
    r: &Request,
) -> Result<Prepared, E> {
    prepare_source(db, root, settings, work, r, ReviewSource::Fresh)
}
/// Reobserve source for one exact already-claimed review. The caller must first
/// validate its normalized job and current review record; this internal snapshot
/// allowance grants no process, permission or acceptance. Other review/recovery
/// states still refuse, preserving the public validation admission contract.
pub(super) fn prepare_claimed_review_preview(
    db: &rusqlite::Connection,
    root: &Path,
    settings: &crate::EngineSettingsDocument,
    work: &str,
    request: &Request,
    review: &crate::IndependentReview,
) -> Result<TaskValidationPreview, E> {
    prepare_source(db, root, settings, work, request, ReviewSource::Claimed(review)).map(|p| p.preview)
}
/// Observe a fresh reviewer admission source after Root has verified the exact
/// current qualified failed original and canonical correction. This explicit internal
/// purpose admits only that same error record, not a public boolean or fake Task.
/// The returned source is fresh; original control evidence is checked separately.
/// No claim, provider, reservation, refund or acceptance occurs.
pub(super) fn prepare_reviewer_retry_preview(db: &rusqlite::Connection, root: &Path, settings: &crate::EngineSettingsDocument, work: &str, request: &Request, recovery: &super::review_recovery::Recovery) -> Result<TaskValidationPreview, E> {
    prepare_source(db, root, settings, work, request, ReviewSource::ReviewerRetry(&recovery.previous)).map(|p| p.preview)
}
/// Internal source purpose only. Actual Root provenance is checked by callers;
/// none of these data coordinates can be deserialized into a process permit.
enum ReviewSource<'a> { Fresh, Claimed(&'a crate::IndependentReview), ReviewerRetry(&'a crate::IndependentReview) }
impl<'a> ReviewSource<'a> {
    /// Exact record that must already occupy the actual task, without substitution.
    fn record(&self) -> Option<&'a crate::IndependentReview> {
        match self { Self::Fresh => None, Self::Claimed(r) | Self::ReviewerRetry(r) => Some(r) }
    }
    /// Purpose-specific structural state, never proof or permission by itself.
    fn applicable(&self) -> bool {
        match self { Self::Fresh => true, Self::Claimed(r) => r.state == "running", Self::ReviewerRetry(r) => r.state == "error" }
    }
}
/// Shared source observation with an exact optional internal review allowance.
/// Fresh retains public refusal. Claimed/ReviewerRetry must equal the actual
/// complete running/error record respectively, under independently verified Root
/// job/recovery provenance. Caller owns transaction/settings. No mutation/grant.
fn prepare_source(
    db: &rusqlite::Connection,
    root: &Path,
    settings: &crate::EngineSettingsDocument,
    work: &str,
    r: &Request,
    review_source: ReviewSource<'_>,
) -> Result<Prepared, E> {
    let Observation { work: w, task, producer, submitted, policy, normalized,
        scope_generation, authority } = observe_submission(
            db, root, settings, work, r, review_source, RepositoryPurpose::Ordinary)?;
    let controls = normalized.controls.as_deref()
        .filter(|c| !c.is_empty()).ok_or(E::Refused("task-validation-controls-required"))?;
    let mut artifacts = BTreeMap::from([(
        submitted.report_path.clone(),
        submitted.report_sha256.clone(),
    )]);
    let mut directories = BTreeMap::new();
    for control in controls {
        let dir = crate::local_directory(root, &control.dir)
            .map_err(|e| E::caused("task-validation-directory", e))?;
        let stat =
            std::fs::metadata(&dir).map_err(|e| E::caused("task-validation-directory-stat", e))?;
        directories.insert(
            control.id.clone(),
            (dir, stat.dev().to_string(), stat.ino().to_string()),
        );
        for input in control.inputs.iter().flatten() {
            let (_, hash) = crate::gate_evaluation::artifact_hash(root, input)
                .map_err(|e| E::caused("task-validation-input", e))?;
            if artifacts.get(input).is_some_and(|old| old != &hash) {
                return Err(E::Refused("task-validation-input-changed"));
            }
            artifacts.insert(input.clone(), hash);
        }
    }
    let contract =
        crate::task_review_contract(&task).map_err(|e| E::caused("task-validation-contract", e))?;
    let execution_policy = source_policy::binding(settings)?;
    let source = digest(&(
        "native-task-validation-source/2",
        &w.id,
        &task.id,
        &task.scope_id,
        &authority,
        &task.requirements,
        contract,
        &policy,
        &normalized,
        &submitted,
        digest(&producer)?,
        &artifacts,
        directories,
        &execution_policy,
    ))?;
    let preview = TaskValidationPreview {
        schema_version: 1,
        work_id: w.id,
        work_revision: w.revision,
        task_id: task.id.clone(),
        producer_agent_id: producer.id,
        attempt_id: producer.attempt_id,
        settings_revision: settings.revision,
        authority_mode: if authority.is_some() {
            "planning-task-policy"
        } else {
            "operator-task-policy"
        },
        scope_generation,
        execution_policy_generation: crate::SettingCount(execution_policy.0),
        authority_event: authority.as_ref().map(|v| v.event_id.clone()),
        authority_revision: authority.as_ref().map(|v| crate::SettingCount(v.revision)),
        policy_mode: policy.mode.clone(),
        control_count: controls.len(),
        source_sha256: source,
        artifacts,
        execution_authorized: false,
        task_accepted: false,
    };
    Ok(Prepared {
        preview,
        policy: normalized,
        settings: settings.clone(),
        original_policy: policy.clone(),
        producer_delivery_version: producer.delivery_version,
        managed: None,
    })
}
impl RootStore {
    /// Execute an ordinary or scoped task's currently authorized controls in a foreground CLI host
    /// implementing the private helper. Managed tasks execute the current and
    /// accepted-task cohort against one pinned single-parent candidate, with
    /// Git inspection outside Root locks and private schema3 originals. Reads
    /// actual native producer/report and saved per-task policies; starts local
    /// commands behind recorded identity/permission, without aggregate policy.
    /// Journals precede publication, exact replay never executes again. Original
    /// errors/unknown ownership stay nonzero and retain known Work metadata.
    /// This operator route grants no automatic mission consent, authenticated
    /// human identity, OS confinement, independent review or task acceptance.
    pub fn run_task_validation(
        &mut self,
        work: &str,
        raw: &[u8],
    ) -> Result<TaskValidationOutcome, RootStoreError> {
        runtime::run(self, work, raw)
    }
    /// Apply original control observations after the original owner has departed.
    /// Starts no command. Missing/tampered/unknown receipts, live owner, stale
    /// source, settings or publication errors refuse without an implicit retry.
    pub fn reconcile_task_validation(
        &mut self,
        work: &str,
        raw: &[u8],
    ) -> Result<TaskValidationOutcome, RootStoreError> {
        runtime::reconcile(self, work, raw)
    }
    /// Explicitly retire original control observations after owner departure and
    /// known process completion. Does not apply stale results or start commands.
    /// A durable retirement receipt and atomic Work/job/slot transition preserve
    /// history; exact replay is harmless. Unknown waits, missing/tampered receipts
    /// and live owners refuse. This grants no task acceptance or automatic retry.
    pub fn retire_task_validation(
        &mut self,
        work: &str,
        raw: &[u8],
    ) -> Result<TaskValidationOutcome, RootStoreError> {
        retirement::run(self, work, raw)
    }
    /// Preview task controls against current saved authority, producer and files
    /// under short saved-settings/SQL snapshots. Managed preview pins only an
    /// existing candidate outside Root locks; it never prepares a new stage.
    /// Starts no control and writes no task/event/controller/receipt; managed
    /// inspection can run read-only Git subprocesses outside the snapshots. The returned
    /// digest grants no execution permit and may become stale immediately. Invalid
    /// requests, mission/planning pause or stop, closed scope, archived work,
    /// missing/stale native producer, policy/input
    /// and explicit transaction/reservation cleanup failures retain their causes.
    pub fn preview_task_validation(
        &self,
        work: &str,
        raw: &[u8],
    ) -> Result<TaskValidationPreview, RootStoreError> {
        let launch = |e| RootStoreError::Launch(Box::new(e));
        let r = decode(raw).map_err(launch)?;
        let managed = managed_execution::acquire_fresh(self, work, &r, false)?;
        if let Some(session) = managed {
            let reservation = self.directory.reserve(false, false).map_err(RootStoreError::Storage);
            let result = match reservation {
                Ok(reservation) => {
                    let result = (|| {
                        let settings = self.read_settings()?;
                        let tx = self.db.unchecked_transaction().map_err(RootStoreError::Sql)?;
                        let result = managed_execution::prepare(&tx, &self.directory, &self.root, &settings, work, &r, &session.data)
                            .map(|p| p.preview).map_err(launch);
                        finished(result, tx.rollback().map_err(RootStoreError::Sql))
                    })();
                    finished(result, reservation.close().map_err(RootStoreError::Storage))
                }
                Err(error) => Err(error),
            };
            return finished(result, session.close());
        }

        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let tx = self
                .db
                .unchecked_transaction()
                .map_err(RootStoreError::Sql)?;
            let result = prepare(&tx, &self.root, &settings, work, &r)
                .map(|p| p.preview)
                .map_err(launch);
            finished(result, tx.rollback().map_err(RootStoreError::Sql))
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Malformed or authority-bearing requests must refuse before any Root IO;
    /// native schema/revision/name invariants are distinct from operator policy.
    #[test]
    fn validation_request_cannot_supply_execution_authority() {
        let valid = serde_json::json!({"schema_version":1,"event_id":"check","expected_revision":4,"task_id":"task"});
        assert!(decode(&serde_json::to_vec(&valid).unwrap()).is_ok());
        for patch in [
            serde_json::json!({"actor":"operator"}),
            serde_json::json!({"origin":"automatic"}),
            serde_json::json!({"command":["python3"]}),
            serde_json::json!({"agent_id":"another"}),
            serde_json::json!({"expected_revision":0}),
            serde_json::json!({"schema_version":2}),
            serde_json::json!({"task_id":"../outside"}),
        ] {
            let mut invalid = valid.clone();
            invalid
                .as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            assert!(decode(&serde_json::to_vec(&invalid).unwrap()).is_err());
        }
    }
}

impl RootStore {
    /// Execute the next submitted automatic worker's saved covering controls.
    /// Requires an actual owning lease for this opened Root/process; canonical
    /// current producer/consent/lease/source are rechecked by the claim and gate.
    /// Returns None when planning is explicitly paused or no submitted task with a saved control policy needs
    /// a first batch. Existing controls before first review are verified, never
    /// automatically retried; reviewer-owned tasks use review reconciliation.
    /// Caller must maintain the owning mission cycle heartbeat while this runs;
    /// mission watch uses run_mission_cycle's independent heartbeat thread.
    /// Controls execute foreground under existing persisted execution policies;
    /// this does not run a reviewer, accept a task or prove whole-mission completion.
    /// Typed errors and known claimed/published Work survive cleanup and stdout.
    pub fn run_next_mission_validation(&mut self,lease:&crate::MissionLease)->Result<Option<TaskValidationOutcome>,RootStoreError>{
        let id=super::mission_supervision_store::bound_mission_work(self,lease)
            .map_err(|e|RootStoreError::Launch(Box::new(E::caused("mission-validation-lease",e))))?.to_owned();
        let work=self.read_work(&id)?;
        if work.planning.as_ref().is_some_and(|p|p.paused) { return Ok(None); }
        for task in work.tasks.iter().flatten().filter(|t|t.status=="submitted" && t.validation_policy.is_some()) {
            let request=Request{schema_version:1,event_id:crate::generate_identifier("mission-validation-").map_err(RootStoreError::Runtime)?,expected_revision:work.revision,task_id:task.id.clone()};
            let raw=serde_json::to_vec(&request).map_err(|e|RootStoreError::Launch(Box::new(E::caused("mission-validation-request",e))))?;
            // An existing review owns the next lifecycle edge, including
            // explicit reconciliation. Never rerun controls or claim their
            // freshness by trying the fresh-review admission projection.
            if task.independent_review.is_some(){continue;}
            if task.automatic_validation.is_some() {
                // Verify actual current private/native receipt; no stale success
                // or implicit rerun when a reviewer is merely still pending.
                let preview=self.preview_task_validation(&id,&raw)?;
                review_evidence::recorded_evidence(self,&work,&preview)?;
                continue;
            }
            return runtime::run_using(self,&id,&raw,Some(lease)).map(Some);
        }
        Ok(None)
    }
}

#[cfg(test)]
mod submission_tests {
    use super::*;
    /// SQL event double, never an acquired lease or authenticated producer.
    /// A valid original precedes incoherent-newest refusal and restoration.
    #[test]
    fn mission_submission_optional_absence_and_required_refusal_preserve_history(){
        let db=rusqlite::Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE events(id TEXT,work_id TEXT,revision INTEGER,kind TEXT,request BLOB)").unwrap();
        let mut work=crate::Work{id:"work".into(),revision:2,..Default::default()};
        assert!(latest_submission(&db,&work,"task").unwrap().is_none());assert!(submission(&db,&work,"task").is_err());
        let good=serde_json::json!({"schema_version":1,"event_id":"submit","expected_revision":1,"id":"task","agent_id":"worker","report_path":"docs/task.md","report_sha256":"a".repeat(64)});
        db.execute("INSERT INTO events VALUES('submit','work',2,'task.submit',?)",[serde_json::to_vec(&good).unwrap()]).unwrap();
        assert_eq!(submission(&db,&work,"task").unwrap().agent_id,"worker");assert!(latest_submission(&db,&work,"other").unwrap().is_none());
        work.revision=3;let mut bad=good;bad["event_id"]="wrong".into();bad["expected_revision"]=2.into();
        db.execute("INSERT INTO events VALUES('bad','work',3,'task.submit',?)",[serde_json::to_vec(&bad).unwrap()]).unwrap();
        assert!(matches!(latest_submission(&db,&work,"task"),Err(E::Refused("task-validation-submission-inconsistent"))));
        db.execute("DELETE FROM events WHERE id='bad'",[]).unwrap();assert_eq!(latest_submission(&db,&work,"task").unwrap().unwrap().agent_id,"worker");db.close().unwrap();
    }
}
