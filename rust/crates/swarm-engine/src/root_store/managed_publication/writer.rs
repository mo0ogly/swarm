//! Native managed publication: one protected SQL transition, never a model call.
use super::*;
use crate::managed_publication_record::{self as receipt, Record};
use crate::root_store::native_transaction::{self as transaction, Change};
use rusqlite::params;

/// Public result distinguishes recorded publication/replay from a provider call.
/// Work is the actual current recorded state; late close/output errors preserve
/// its coordinates and never imply rollback or grant another execution.
#[derive(Serialize)]
pub struct ManagedPublicationOutcome {
    pub schema_version: i64,
    pub publication_event: String,
    pub candidate_commit: String,
    pub task_accepted: bool,
    pub replayed: bool,
    pub provider_started: bool,
    pub work: Work,
}
/// Verify historical task custody and the latest published whole-cohort opinion.
/// Earlier tasks/originals remain unchanged; the latest controls and criterion
/// verdicts, not a copied review, cover the current candidate. Caller owns actual
/// Root/settings/SQL inputs; all original/chain/report causes refuse, no writes.
/// Every earlier publication's own native originals are revalidated even when
/// the caller requests the latest task; SQL chain integrity is not sufficient.
/// Work grows with the retained chain; existing per-Git policy remains in force,
/// with no new history cutoff, global timeout, cache lease or actor authority.
pub(in crate::root_store) fn current(root: &RootStore, db: &rusqlite::Connection,
    work: &Work, task: &crate::Task, settings: &crate::EngineSettingsDocument) -> Result<Record,E> {
    let mut records=receipt::chain(db,work).map_err(|e|E::Launch(Box::new(e)))?;
    let own=records.iter().position(|r|r.task_id==task.id)
        .ok_or(E::Refused("managed-publication-record-required"))?;
    let latest=records.last().ok_or(E::Refused("managed-publication-record-required"))?;
    let latest_task=work.tasks.iter().flatten().find(|t|t.id==latest.task_id)
        .ok_or(E::Refused("managed-publication-task-required"))?;
    let (context,candidate,metadata)=original(root,db,work,latest_task,settings,latest)?;
    let limits=settings.values.mutation_limits().map_err(E::Settings)?;
    let plan=crate::managed_repository::managed_control_plan(work,&latest.task_id,&limits.policy)
        .map_err(|e|E::Launch(Box::new(e)))?;
    if candidate.control_plan.as_ref()!=Some(&plan) {
        return Err(E::Refused("managed-publication-current-controls-changed"));
    }
    let review=latest_task.independent_review.as_deref().ok_or(E::Refused("managed-publication-review-required"))?;
    verify_recorded_cohort(root,db,work,review,&context,&candidate,&metadata)?;
    // Every prior publication contributes the current cumulative baseline.
    // Checking only the requested task could hide a damaged earlier original
    // when the caller selected the latest task. Never infer original integrity
    // from the canonical SQL chain or from the latest cohort's verdict alone.
    for record in &records[..records.len()-1] {
        let mut tasks=work.tasks.iter().flatten().filter(|t|t.id==record.task_id);
        let earlier=tasks.next().ok_or(E::Refused("managed-publication-task-required"))?;
        if tasks.next().is_some() { return Err(E::Refused("managed-publication-task-ambiguous")); }
        original(root,db,work,earlier,settings,record)?;
    }
    Ok(records.remove(own))
}
/// Verify one immutable publication's native originals, worker-result base and
/// actual Git report under current saved policy. The complete canonical chain
/// must already be checked; historical evidence alone does not cover a newer
/// candidate. Returns only private observation data; no mutation or fallback.
fn original(root:&RootStore, db:&rusqlite::Connection, work:&Work, task:&crate::Task,
    settings:&crate::EngineSettingsDocument, record:&Record)
    ->Result<(ManagedReviewContext,crate::managed_repository::ManagedCandidate,serde_json::Value),E>{
    let review=task.independent_review.as_deref().ok_or(E::Refused("managed-publication-review-required"))?;
    let original = root.recorded_native_review(db,&work.id,&review.id)?;
    if !original.verdict_applicable || original.review != *review || review.state != "passed"
        || original.task != task.id || original.original_receipt_sha256 != record.original_sha256
        || original.validation_receipt_sha256 != record.validation_sha256 || original.managed_context.is_none() {
        return Err(E::Refused("managed-publication-original-changed"));
    }
    let (candidate,metadata) = task_validation::recorded_managed_candidate(root,work,&task.id,&record.validation_sha256)?;
    if candidate.commit != record.candidate_commit || candidate.previous != record.previous_candidate
        || candidate.result != record.result_commit || candidate.agent != record.producer
        || candidate.attempt != record.attempt || candidate.contract != record.cohort_sha256
        || candidate.binding_digest().map_err(|e|E::Launch(Box::new(e)))? != record.binding_sha256 {
        return Err(E::Refused("managed-publication-original-candidate-binding"));
    }
    let producer=crate::task_report::completed_producer(db,&work.id,task,&record.producer)
        .map_err(|e|E::Launch(Box::new(e)))?;
    let result=crate::managed_repository::verify_result(&root.root,work,&producer,settings)
        .map_err(|e|E::Launch(Box::new(e)))?;
    if result.worker_base()!=record.worker_base().map_err(|e|E::Launch(Box::new(e)))?
        || result.commit!=record.result_commit {
        return Err(E::Refused("managed-publication-original-worker-base"));
    }
    let context=original.managed_context.ok_or(E::Refused("managed-publication-context-required"))?;
    report(root,work,task,settings,&context)?;
    Ok((context,candidate,metadata))
}
/// Read the actual immutable Git blob and Root artifacts under current saved Git
/// policy. Paths derive from the validated managed repository and canonical task;
/// no empty legacy git_report is treated as a native report or accepted fallback.
fn report(root: &RootStore, work: &Work, task: &crate::Task, settings: &crate::EngineSettingsDocument, context: &ManagedReviewContext) -> Result<(),E> {
    let repo = work.planning.as_deref().and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-publication-repository-required"))?;
    let review = task.independent_review.as_deref().ok_or(E::Refused("managed-publication-review-required"))?;
    let relative = std::path::Path::new(&repo.storage).strip_prefix(&root.root)
        .map_err(|e| caused("managed-publication-storage",e))?.join("repository.git");
    let relative = relative.to_str().ok_or(E::Refused("managed-publication-storage-encoding"))?;
    let report = if repo.project_subdir.is_empty() { format!("docs/{}.md",task.id) }
        else { format!("{}/docs/{}.md",repo.project_subdir,task.id) };
    let observed = crate::hash_git_report(&root.root,relative,&review.candidate_commit,&report,2,settings)
        .map_err(|e| caused("managed-publication-git-report",e))?;
    if observed.sha256 != review.sha256 { return Err(E::Refused("managed-publication-git-report-changed")); }
    for (path,expected) in review.report_artifacts.iter().flatten().chain(std::iter::once((&review.report,&review.sha256))) {
        let hash = if let Some(name) = path.strip_prefix("candidate/").or_else(||path.strip_prefix("candidate-link/")) {
            crate::hash_git_report(&root.root,relative,&review.candidate_commit,name,2,settings)
                .map_err(|e|caused("managed-publication-git-artifact",e))?.sha256
        } else {
            let name = if let Some(cohort) = path.strip_prefix("cohort/") {
                let (id,name) = cohort.split_once('/').ok_or(E::Refused("managed-publication-cohort-artifact"))?;
                let bound = context.tasks.iter().flatten().find(|t|t.task == id)
                    .ok_or(E::Refused("managed-publication-cohort-artifact"))?;
                if bound.binding.report != name || &bound.binding.report_sha256 != expected { return Err(E::Refused("managed-publication-cohort-artifact")); }
                name
            } else { path.as_str() };
            crate::gate_evaluation::artifact_hash(&root.root,name)
                .map_err(|e|caused("managed-publication-artifact",e))?.1
        };
        if &hash != expected { return Err(E::Refused("managed-publication-artifact-changed")); }
    }
    let limits = settings.values.context_policy_limits().map_err(E::Settings)?;
    crate::check_project_context_freshness(&root.root,review.workflow.as_deref().and_then(|w|w.project.as_deref()),"reviewer",&limits)
        .map_err(|e| caused("managed-publication-project-context",e))?;
    Ok(())
}
/// Replay an exact historical or latest request after verifying its preserved
/// accepted task and the latest whole-cohort publication. Returns the original
/// publication coordinates alongside current Work, never a new execution.
/// Conflicting event IDs and stale/invalid originals refuse with causes.
fn replay(root: &RootStore, db: &rusqlite::Connection, work_id: &str, request: &task_validation::Request,
    settings: &crate::EngineSettingsDocument) -> Result<Option<ManagedPublicationOutcome>,E> {
    let Some(record) = receipt::read(db,&request.event_id).map_err(|e| E::Launch(Box::new(e)))? else { return Ok(None); };
    if record.work_id != work_id || record.task_id != request.task_id || record.expected_revision != request.expected_revision {
        return Err(E::Refused("managed-publication-event-conflict"));
    }
    let work = crate::read_recorded_work(db,work_id).map_err(E::Work)?;
    let task = work.tasks.iter().flatten().find(|t|t.id == request.task_id).ok_or(E::Refused("managed-publication-task-required"))?;
    let verified = current(root,db,&work,task,settings)?;
    if verified.event_id != record.event_id { return Err(E::Refused("managed-publication-event-conflict")); }
    let policy = settings.quality_policy_snapshot().map_err(E::Settings)?;
    crate::check_gate_evidence(task,&root.root,&policy).map_err(|e|caused("managed-publication-gate",e))?;
    Ok(Some(ManagedPublicationOutcome { schema_version:1,publication_event:record.event_id,
        candidate_commit:record.candidate_commit,task_accepted:true,replayed:true,provider_started:false,work }))
}
impl RootStore {
    /// Publish a reviewed managed candidate and accept its submitted task
    /// atomically with the normalized worker settlement and canonical event. Uses
    /// the strict native task/revision envelope and current saved settings. Pins
    /// and refreshes candidate sources outside SQL/settings locks; verifies native
    /// originals, full cohort, report, gate, prerequisites and idle ownership inside
    /// the writer. Cumulative publication retains prior task/review originals,
    /// links the previous canonical receipt and separates worker base from parent.
    /// Exact replay reads the original publication without model/control/debit;
    /// collisions, drift and missing evidence refuse without a staged mutation.
    /// Commit uncertainty and late release failures retain their causes and known
    /// Work coordinates. SQL publication does not mutate an advisory Git branch,
    /// authenticate an operator or accept the mission/global judge outcome.
    pub fn publish_managed_candidate(&self, work_id: &str, raw: &[u8]) -> Result<ManagedPublicationOutcome,E> {
        if !self.writable { return Err(E::Refused("root-read-only")); }
        let request = task_validation::decode(raw).map_err(|e|E::Launch(Box::new(e)))?;
        let mut known = None;
        let replayed = material_path::with_material_read(self,|| {
            let settings = self.read_settings()?;
            let value = transaction::apply(self,|tx| replay(self,tx,work_id,&request,&settings).map(Change::Observe))?;
            if let Some(value) = &value { known = Some((value.work.id.clone(),value.work.revision)); }
            Ok(value)
        }).map_err(|primary|match &known { Some((work,revision))=>E::WorkRecorded {primary:Box::new(primary),work:work.clone(),revision:*revision},None=>primary })?;
        if let Some(value) = replayed { return Ok(value); }
        let before = observe(self,work_id,&request,None)?;
        let candidate = task_validation::acquire_claimed_review_candidate(self,work_id,&request,Some(&before.review))?
            .ok_or(E::Refused("managed-publication-candidate-required"))?;
        let result = (|| {
            candidate.refresh(self)?;
            let checked = observe(self,work_id,&request,Some(&candidate))?;
            if before.work != checked.work || before.settings_sha256 != checked.settings_sha256 {
                return Err(E::Refused("managed-publication-inputs-changed"));
            }
            candidate.refresh(self)?;
            let producer=crate::task_report::completed_producer(&self.db,work_id,
                checked.work.tasks.iter().flatten().find(|t|t.id==request.task_id)
                    .ok_or(E::Refused("managed-publication-task-required"))?,&checked.review.producer)
                .map_err(|e|E::Launch(Box::new(e)))?;
            let result=crate::managed_repository::verify_result(&self.root,&checked.work,&producer,&self.read_settings()?)
                .map_err(|e|E::Launch(Box::new(e)))?;
            let worker_base=result.worker_base().to_owned();
            material_path::with_material_operation(self,|| crate::agent_storage_retry::execute_borrowed(
                &self.db,&self.directory,false,|_,settings| {
                let value = transaction::apply(self,|tx| {
                    if let Some(value) = replay(self,tx,work_id,&request,settings)? { return Ok(Change::Observe(value)); }
                    let work = crate::read_recorded_work(tx,work_id).map_err(E::Work)?;
                    if work != checked.work || work.revision != request.expected_revision
                        || receipt::digest(settings).map_err(|e|E::Launch(Box::new(e)))? != checked.settings_sha256 {
                        return Err(E::Refused("managed-publication-inputs-changed"));
                    }
                    let task = work.tasks.iter().flatten().find(|t|t.id == request.task_id)
                        .ok_or(E::Refused("managed-publication-task-required"))?;
                    super::super::managed_baseline::inspect(self,tx,&work,&task.id,settings)?;
                    let previous=receipt::chain(tx,&work).map_err(|e|E::Launch(Box::new(e)))?.pop();
                    let previous_publication=previous.as_ref().map(|r|Ok(receipt::PreviousPublication {
                        event_id:r.event_id.clone(),sha256:receipt::digest(r).map_err(|e|E::Launch(Box::new(e)))?
                    })).transpose()?;
                    if checked.review.state != "passed" || task.status != "submitted" {
                        return Err(E::Refused("managed-publication-passed-submission-required"));
                    }
                    task_validation::require_validation_idle(tx,work_id,&task.id).map_err(|e|E::Launch(Box::new(e)))?;
                    Self::require_idle_task_review(tx,work_id,&task.id).map_err(|e|E::Launch(Box::new(e)))?;
                    let active = crate::agent_launch::query(tx,
                        "SELECT 1 FROM main.agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')",
                        params![work_id,task.id],|rows| Ok(rows.next()?.is_some())).map_err(|e|E::Launch(Box::new(e)))?;
                    if active { return Err(E::Refused("managed-publication-worker-active")); }
                    let original = self.recorded_native_review(tx,work_id,&checked.review.id)?;
                    if !original.verdict_applicable || original.review != checked.review || original.task != task.id {
                        return Err(E::Refused("managed-publication-original-changed"));
                    }
                    let validation = task_validation::recorded_evidence_for_review_result(self,&work,&task.id,&original.validation_receipt_sha256)?;
                    candidate.preview(self,tx,settings,work_id,&request,Some(&checked.review)).map_err(|e|E::Launch(Box::new(e)))?;
                    verify_cohort(self,tx,&work,&checked.review,&checked.context,&candidate)?;
                    report(self,&work,task,settings,&checked.context)?;
                    let proof = PrecheckedAcceptance { work:&work,settings,task:&task.id };
                    let limits = settings.values.mutation_limits().map_err(E::Settings)?;
                    let policy = settings.quality_policy_snapshot().map_err(E::Settings)?;
                    let context = crate::TaskLifecycleContext { root:&self.root,quality_policy:Some(&policy),task_limits:&limits.task,validation_limits:&limits.policy };
                    let transition = crate::Request { schema_version:1,event_id:request.event_id.clone(),expected_revision:work.revision,id:task.id.clone(), status:"accepted".into(), next:format!("managed candidate {} published",candidate.candidate().commit),..Default::default() };
                    let accepted = crate::task_lifecycle::transition_managed_publication(&work,&transition,&context,settings,&mut crate::environment::SystemEnvironment,&proof)
                        .map_err(|e|caused("managed-publication-acceptance",e))?;
                    // Recheck source/original material after gate and prerequisite IO.
                    candidate.preview(self,tx,settings,work_id,&request,Some(&checked.review)).map_err(|e|E::Launch(Box::new(e)))?;
                    verify_cohort(self,tx,&work,&checked.review,&checked.context,&candidate)?;
                    let held = candidate.candidate();
                    let accepted_task_sha256 = receipt::digest(&accepted).map_err(|e|E::Launch(Box::new(e)))?;
                    let mut next = work.clone();
                    *next.tasks.as_mut().and_then(|ts|ts.iter_mut().find(|t|t.id == request.task_id)).ok_or(E::Refused("managed-publication-task-required"))? = accepted;
                    next.planning.as_deref_mut().and_then(|p|p.repository.as_deref_mut()).ok_or(E::Refused("managed-publication-repository-required"))?.candidate_commit = held.commit.clone();
                    next.revision = work.revision.checked_add(1).ok_or(E::Refused("managed-publication-revision-overflow"))?;
                    next.updated = crate::system_timestamp().map_err(|e|caused("managed-publication-clock",e))?;
                    let record = Record { schema_version:if previous_publication.is_some(){2}else{1},
                        worker_base:previous_publication.as_ref().map(|_|worker_base.clone()),previous_publication,
                        event_id:request.event_id.clone(),expected_revision:work.revision,
                        work_id:work.id.clone(),task_id:task.id.clone(),producer:checked.review.producer.clone(),attempt:checked.review.attempt.clone(),
                        result_commit:held.result.clone(),candidate_commit:held.commit.clone(),previous_candidate:held.previous.clone(),review_id:checked.review.id.clone(),
                        original_sha256:original.original_receipt_sha256,context_sha256:checked.review.context_sha256.clone(),validation_sha256:validation.receipt_sha256,
                        binding_sha256:held.binding_digest().map_err(|e|E::Launch(Box::new(e)))?,cohort_sha256:held.contract.clone(),
                        accepted_task_sha256,settings_sha256:checked.settings_sha256.clone(),accepted_work:next.clone() };
                    let body = crate::encode_model(&record).map_err(E::Encode)?;
                    let staged = crate::stage_recorded_change_with_hook(tx,crate::RecordedChange { work:&next,expected_revision:work.revision,event:&request.event_id,kind:"managed.publication",request:&body },|db,_| {
                        let changed = db.execute("UPDATE main.managed_attempts SET state='integrated',result_commit=?,detail=? WHERE agent_id=? AND work_id=? AND task_id=? AND base_commit=? AND state='ready' AND result_commit='' AND detail=''",
                            params![held.result,request.event_id,checked.review.producer,work.id,task.id,worker_base])?;
                        if changed != 1 { return Err(Box::new(A::Refused("managed-publication-worker-row")) as Box<dyn std::error::Error+Send+Sync>); }
                        crate::agent_launch::query(db,"SELECT state,result_commit,detail FROM managed_attempts WHERE agent_id=?",[&record.producer],|rows| {
                            let row=rows.next()?.ok_or(A::Refused("managed-publication-worker-row"))?;
                            let actual:(String,String,String)=(row.get(0)?,row.get(1)?,row.get(2)?);
                            if actual != ("integrated".into(),record.result_commit.clone(),record.event_id.clone()) { return Err(A::Refused("managed-publication-worker-row")); }
                            Ok(())
                        }).map_err(|e|Box::new(e) as Box<dyn std::error::Error+Send+Sync>)
                    }).map_err(|e|caused("managed-publication-stage",e))?;
                    if !matches!(staged,crate::RecordedChangeOutcome::Staged) { return Err(E::Refused("managed-publication-unexpected-replay")); }
                    receipt::integrated(tx,&next,&record.producer,&record.task_id,&worker_base,&record.result_commit,&record.event_id).map_err(|e|E::Launch(Box::new(e)))?;
                    Ok(Change::Mutate(ManagedPublicationOutcome { schema_version:1,publication_event:request.event_id.clone(),candidate_commit:held.commit.clone(),task_accepted:true,replayed:false,provider_started:false,work:next }))
                })?;
                known = Some((value.work.id.clone(),value.work.revision));
                Ok(value)
            }))
        })();
        let result = finished(result,candidate.close());
        result.map_err(|primary|match known { Some((work,revision))=>E::WorkRecorded { primary:Box::new(primary),work,revision },None=>primary })
    }
}
