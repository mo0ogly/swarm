//! Actual managed-control custody outside Root SQL/settings locks. Plain source
//! observations never grant permission; the existing native gate owns that edge.
use super::{
    journal, source_origin, store, Observation, Prepared, RepositoryPurpose, Request, ReviewSource,
};
use crate::{
    managed_repository::{CandidateWitness, ManagedCandidate},
    root_store::{finished, RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

/// Plain actual candidate/input observations, without descriptor ownership.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Captured {
    pub candidate: ManagedCandidate,
    repository_sha256: String,
    root_instance: crate::root_store::RootInstanceIdentity,
    artifacts: BTreeMap<String, String>,
    directories: BTreeMap<String, (String, String, String)>,
}
/// The controller/helper owns custody separately from plain Prepared/SQL results.
/// Must be explicitly closed; never return this through a cleanup wrapper that
/// can discard a successful resource when another close fails.
#[must_use]
pub(super) struct Session {
    pub data: Captured,
    witness: CandidateWitness,
}
impl Session {
    /// Holder-anchored workspace path; caller must retain this session through
    /// owned wait/helper exec. The path alone grants nothing and is not persisted.
    pub fn workspace(&self) -> PathBuf {
        self.witness.workspace_path()
    }
    /// Borrow the retained complete candidate repository for readonly review.
    /// Caller must keep this session alive and compare every input with its
    /// checkpoint digest; no pathname alone proves current custody or authority.
    pub fn repository(&self) -> PathBuf {
        self.witness.repository_path()
    }
    /// Read a retained link literal without following it; the review materializer
    /// must compare its exact digest and apply saved context caps before output.
    pub fn review_link(&self, name: &str) -> Result<String, E> {
        self.witness.review_link(name).map_err(launch)
    }
    /// Release all held candidate descriptors, preserving typed close causes.
    /// No permission, source rollback or process-reap assertion follows.
    pub fn close(self) -> Result<(), E> {
        self.witness.close().map(|_| ()).map_err(launch)
    }
    /// Rehash declared inputs and directory identities without Git or Root locks.
    /// Refuse drift from admission; this is not a whole-filesystem snapshot or
    /// same-UID confinement. Full tracked-source inspection precedes each slot.
    pub fn observe(&self, root: &Path) -> Result<(), E> {
        if capture(
            root,
            &self.witness,
            &self.data.repository_sha256,
            &self.data.root_instance,
        )? != self.data
        {
            return Err(E::Refused("task-validation-managed-inputs-changed"));
        }
        Ok(())
    }
    /// Inspect the exact existing checkpoint/ref/raw tracked sources outside
    /// Root locks before one command/publication. Never reprepare/create a stage.
    /// Fresh temporary custody is explicitly closed, including on mismatch.
    pub fn refresh(&self, store: &RootStore, job: &store::Job) -> Result<(), E> {
        let fresh = acquire_job(store, job, &self.data.candidate)?;
        let result = if fresh.data == self.data {
            Ok(())
        } else {
            Err(E::Refused("task-validation-managed-inputs-changed"))
        };
        finished(result, fresh.close())
    }
}
/// Private create-only admission preimage. Schema1 is a protocol version, not
/// operator policy. The actual SQL origin binds the candidate/whole cohort hash.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    schema_version: i64,
    job_id: String,
    candidate: ManagedCandidate,
}
/// Plain actual producer/settings observations after checked lock release.
struct Inputs {
    observation: Observation,
    settings: crate::EngineSettingsDocument,
    root_instance: crate::root_store::RootInstanceIdentity,
}
/// Preserve a typed launch cause under the existing Root error boundary.
/// No Work mutation, retry or diagnostic rewriting follows.
fn launch(error: A) -> E {
    E::Launch(Box::new(error))
}
/// Read actual managed submission and owner/phase under a short reservation and
/// SQL snapshot. None means genuinely ordinary Work only, never a failed read.
/// Fresh requires idle task; existing jobs require exact current scalar equality.
/// No Git, candidate creation or execution occurs while these locks are held.
fn inputs(
    store: &RootStore,
    work: &str,
    request: &Request,
    job: Option<&store::Job>,
) -> Result<Option<Inputs>, E> {
    let reservation = store.directory.reserve(false, false).map_err(E::Storage)?;
    let result = (|| {
        let settings = store.read_settings()?;
        let tx = store.db.unchecked_transaction().map_err(E::Sql)?;
        let result = (|| {
            if let Some(job) = job {
                if store::job(&tx, &job.id).map_err(launch)?.as_ref() != Some(job) {
                    return Err(E::Refused("task-validation-owner-source-changed"));
                }
            }
            super::super::require_planning_mission_active(&tx, work)
                .map_err(|e| launch(A::caused("task-validation-mission-inactive", e)))?;
            let current = crate::read_recorded_work(&tx, work)
                .map_err(|e| launch(A::caused("task-validation-work", e)))?;
            if job.is_none() && current.revision != request.expected_revision {
                return Err(E::Refused("task-validation-revision-conflict"));
            }
            if !current
                .planning
                .as_deref()
                .is_some_and(|p| p.repository.is_some())
            {
                return Ok(None);
            }
            if job.is_none() {
                super::require_validation_idle(&tx, work, &request.task_id).map_err(launch)?;
            }
            let request = Request {
                expected_revision: if job.is_some() {
                    current.revision
                } else {
                    request.expected_revision
                },
                ..request.clone()
            };
            let observation = super::observe_submission(
                &tx,
                &store.root,
                &settings,
                work,
                &request,
                ReviewSource::Fresh,
                RepositoryPurpose::Candidate,
            )
            .map_err(launch)?;
            let root_instance = crate::root_store::observe_instance(&tx, &store.directory)?;
            Ok(Some(Inputs {
                observation,
                settings,
                root_instance,
            }))
        })();
        finished(result, tx.rollback().map_err(E::Sql))
    })();
    finished(result, reservation.close().map_err(E::Storage))
}
/// Require a physical observed path inside the opened project. Root-relative
/// artifact references may name the private candidate stage, never a host prefix.
fn relative(root: &Path, path: &Path) -> Result<String, E> {
    path.strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .ok_or(E::Refused("task-validation-managed-artifact-outside-root"))
}
/// Observe actual held workspace inputs/dirs with existing checked artifact IO.
/// Every input must first resolve inside the candidate project; Root-relative
/// remapping preserves that real file for the existing gate/evidence consumers.
/// No copied artifact, hidden size cap, Git, permission or Root mutation is added.
fn capture(
    root: &Path,
    witness: &CandidateWitness,
    repository_sha256: &str,
    root_instance: &crate::root_store::RootInstanceIdentity,
) -> Result<Captured, E> {
    witness.verify_retained_identities().map_err(launch)?;
    let candidate = witness.record();
    candidate.verify_binding().map_err(launch)?;
    let base = crate::canonical_project_root(root).map_err(E::Path)?;
    let workspace = crate::local_directory(&witness.workspace_path(), "").map_err(E::Path)?;
    if workspace != Path::new(&candidate.workspace) {
        return Err(E::Refused("task-validation-managed-workspace-changed"));
    }
    relative(&base, &workspace)?;
    let plan = candidate
        .control_plan
        .as_ref()
        .ok_or(E::Refused("task-validation-managed-plan-required"))?;
    let mut artifacts = BTreeMap::new();
    let mut directories = BTreeMap::new();
    for slot in &plan.slots {
        let directory = crate::local_directory(&witness.workspace_path(), &slot.control.dir)
            .map_err(E::Path)?;
        let stat = std::fs::metadata(&directory).map_err(|e| {
            E::Launch(Box::new(A::caused(
                "task-validation-managed-directory-stat",
                e,
            )))
        })?;
        directories.insert(
            slot.transport_id.clone(),
            (
                relative(&base, &directory)?,
                stat.dev().to_string(),
                stat.ino().to_string(),
            ),
        );
        for input in slot.control.inputs.iter().flatten() {
            let (path, hash) =
                crate::gate_evaluation::artifact_hash(&witness.workspace_path(), input).map_err(
                    |e| E::Launch(Box::new(A::caused("task-validation-managed-input", e))),
                )?;
            let name = relative(&base, &path)?;
            if artifacts.get(&name).is_some_and(|old| old != &hash) {
                return Err(E::Refused("task-validation-managed-inputs-changed"));
            }
            artifacts.insert(name, hash);
        }
    }
    Ok(Captured {
        candidate: candidate.clone(),
        repository_sha256: repository_sha256.into(),
        root_instance: root_instance.clone(),
        artifacts,
        directories,
    })
}
/// Prepare or inspect under released Root locks, then retain owned custody.
/// Existing-job inspection is read-only and must equal its complete preimage.
/// Any post-pin failure explicitly releases newly owned witnesses.
fn acquire(
    store: &RootStore,
    input: Inputs,
    create: bool,
    expected: Option<&ManagedCandidate>,
) -> Result<Session, E> {
    let observation = input.observation;
    let repository_sha256 = super::digest(
        &observation
            .work
            .planning
            .as_deref()
            .and_then(|p| p.repository.as_deref())
            .ok_or(E::Refused("task-validation-managed-repository-required"))?,
    )
    .map_err(launch)?;
    let result = crate::managed_repository::verify_result(
        &store.root,
        &observation.work,
        &observation.producer,
        &input.settings,
    )
    .map_err(launch)?;
    if result.report_path != observation.submitted.report_path
        || result.report_sha256 != observation.submitted.report_sha256
    {
        return Err(E::Refused("task-validation-managed-report-changed"));
    }
    if create {
        crate::managed_repository::prepare_candidate(
            &store.root,
            &observation.work,
            &observation.producer,
            &input.settings,
        )
        .map_err(launch)?;
    }
    let witness = crate::managed_repository::pin_candidate(
        &store.root,
        &observation.work,
        &observation.producer,
        &input.settings,
    )
    .map_err(launch)?;
    let observed = (|| {
        if expected.is_some_and(|value| value != witness.record()) {
            return Err(E::Refused("task-validation-managed-candidate-changed"));
        }
        capture(
            &store.root,
            &witness,
            &repository_sha256,
            &input.root_instance,
        )
    })();
    match observed {
        Ok(data) => Ok(Session { data, witness }),
        Err(error) => finished(Err(error), witness.close().map(|_| ()).map_err(launch)),
    }
}
/// Fresh preparation/readonly preview after caller's exact-event replay check.
/// Ordinary Work returns None. True permits candidate Git preparation only;
/// no SQL claim, grant, command or acceptance is performed by this function.
pub(super) fn acquire_fresh(
    store: &RootStore,
    work: &str,
    request: &Request,
    create: bool,
) -> Result<Option<Session>, E> {
    inputs(store, work, request, None)?
        .map(|i| acquire(store, i, create, None))
        .transpose()
}
/// Inspect only the exact existing candidate of a real registered job. No stage
/// creation or fallback to a new original is permitted; missing source refuses.
pub(super) fn acquire_job(
    store: &RootStore,
    job: &store::Job,
    candidate: &ManagedCandidate,
) -> Result<Session, E> {
    let request = Request {
        schema_version: 1,
        event_id: job.event_id.clone(),
        expected_revision: job.work_revision,
        task_id: job.task_id.clone(),
    };
    let input = inputs(store, &job.work_id, &request, Some(job))?
        .ok_or(E::Refused("task-validation-managed-repository-required"))?;
    acquire(store, input, false, Some(candidate))
}
/// Persist one candidate preimage before SQL claim. A later failed claim can
/// leave this immutable orphan; it grants nothing and never replaces an original.
pub(super) fn persist(store: &RootStore, id: &str, session: &Session) -> Result<(), E> {
    journal::persist(
        store,
        &journal::name("candidate", id),
        &Frame {
            schema_version: 1,
            job_id: id.into(),
            candidate: session.data.candidate.clone(),
        },
    )
    .map(|_| ())
}
/// Read a job's actual mode from SQL origin; a managed helper must also recover
/// the exact private candidate preimage and pin its own directories. Neither a
/// command-line path/identity nor the parent-held descriptor is trusted instead.
pub(super) fn for_helper(store: &RootStore, job: &store::Job) -> Result<Option<Session>, E> {
    let origin = source_origin::read(&store.db, &job.id).map_err(launch)?;
    let Some(origin) = origin else {
        return Ok(None);
    };
    if origin.candidate_binding().is_none() {
        return Ok(None);
    }
    let (frame, _): (Frame, String) = journal::read(store, &journal::name("candidate", &job.id))?;
    if frame.schema_version != 1 || frame.job_id != job.id {
        return Err(E::Refused("task-validation-managed-frame-invalid"));
    }
    frame.candidate.verify_binding().map_err(launch)?;
    let binding = frame.candidate.binding_digest().map_err(launch)?;
    origin
        .verify_candidate_binding(Some(&binding))
        .map_err(launch)?;
    acquire_job(store, job, &frame.candidate).map(Some)
}
/// Compose current authoritative Root source using already observed physical
/// candidate data; performs no Git and owns no descriptors. Current saved cohort
/// must exactly match the immutable checkpoint, including normalization/defaults.
pub(super) fn prepare(
    db: &rusqlite::Connection,
    directory: &crate::root_store::LocalDirectory,
    root: &Path,
    settings: &crate::EngineSettingsDocument,
    work: &str,
    request: &Request,
    data: &Captured,
) -> Result<Prepared, A> {
    let actual_instance = crate::root_store::observe_instance(db, directory)
        .map_err(|e| A::caused("task-validation-managed-root-instance", e))?;
    if actual_instance != data.root_instance {
        return Err(A::Refused("task-validation-managed-root-instance-changed"));
    }
    let observed = super::observe_submission(
        db,
        root,
        settings,
        work,
        request,
        ReviewSource::Fresh,
        RepositoryPurpose::Candidate,
    )?;
    let limits = settings
        .values
        .mutation_limits()
        .map_err(|e| A::caused("task-validation-managed-policy", e))?;
    let plan = crate::managed_repository::managed_control_plan(
        &observed.work,
        &request.task_id,
        &limits.policy,
    )?;
    let candidate = &data.candidate;
    candidate.verify_binding()?;
    let repository = observed
        .work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(A::Refused("task-validation-managed-repository-required"))?;
    if candidate.previous != repository.candidate_commit
        || super::digest(repository)? != data.repository_sha256
    {
        return Err(A::Refused("task-validation-managed-repository-changed"));
    }
    if candidate.work != work
        || candidate.task != request.task_id
        || candidate.agent != observed.producer.id
        || candidate.attempt != observed.producer.attempt_id
        || candidate.control_plan.as_ref() != Some(&plan)
    {
        return Err(A::Refused("task-validation-managed-candidate-changed"));
    }
    let mut artifacts = data.artifacts.clone();
    if artifacts
        .insert(
            observed.submitted.report_path.clone(),
            observed.submitted.report_sha256.clone(),
        )
        .is_some_and(|old| old != observed.submitted.report_sha256)
    {
        return Err(A::Refused("task-validation-managed-artifact-collision"));
    }
    let policy_epoch = super::source_policy::binding(settings)?;
    let source = super::digest(&(
        "native-managed-task-validation-source/1",
        &observed.work.id,
        &observed.task.id,
        &observed.task.scope_id,
        &observed.authority,
        &observed.task.requirements,
        crate::task_review_contract(&observed.task)
            .map_err(|e| A::caused("task-validation-contract", e))?,
        candidate,
        &data.repository_sha256,
        &data.root_instance,
        &observed.submitted,
        super::digest(&observed.producer)?,
        &artifacts,
        &data.directories,
        &policy_epoch,
    ))?;
    let preview = super::TaskValidationPreview {
        schema_version: 1,
        work_id: work.into(),
        work_revision: observed.work.revision,
        task_id: request.task_id.clone(),
        producer_agent_id: observed.producer.id,
        attempt_id: observed.producer.attempt_id,
        settings_revision: settings.revision,
        authority_mode: "planning-task-policy",
        scope_generation: observed.scope_generation,
        execution_policy_generation: crate::SettingCount(policy_epoch.0),
        authority_event: observed.authority.as_ref().map(|v| v.event_id.clone()),
        authority_revision: observed
            .authority
            .as_ref()
            .map(|v| crate::SettingCount(v.revision)),
        policy_mode: observed.policy.mode.clone(),
        control_count: plan.slots.len(),
        source_sha256: source,
        artifacts,
        execution_authorized: false,
        task_accepted: false,
    };
    Ok(Prepared {
        preview,
        policy: observed.normalized,
        settings: settings.clone(),
        original_policy: observed.policy,
        producer_delivery_version: observed.producer.delivery_version,
        managed: Some(data.clone()),
    })
}
/// Bind a current source to the exact recorded optional candidate/whole cohort,
/// then the actual mission overlay. Ordinary absence retains legacy semantics;
/// a managed row/body/plan mismatch never downgrades to ordinary success.
pub(super) fn bound_source(
    db: &rusqlite::Connection,
    job: &store::Job,
    base: &str,
    candidate: Option<&ManagedCandidate>,
) -> Result<String, A> {
    let Some(candidate) = candidate else {
        return super::mission_authority::source(db, job, base);
    };
    candidate.verify_binding()?;
    let origin = source_origin::read(db, &job.id)?
        .ok_or(A::Refused("task-validation-managed-origin-required"))?;
    let plan = candidate
        .control_plan
        .as_ref()
        .ok_or(A::Refused("task-validation-managed-plan-required"))?;
    origin.verify_controls(&job.id, plan)?;
    origin.verify_candidate_binding(Some(&candidate.binding_digest()?))?;
    origin.verify_base_source(base)?;
    super::mission_authority::digest(
        &source_origin::digest(&origin)?,
        super::mission_authority::read(db, &job.id)?.as_ref(),
    )
}
