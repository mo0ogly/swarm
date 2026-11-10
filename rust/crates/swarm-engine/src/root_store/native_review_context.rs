//! Read original verdicts or failed/stale non-verdicts for context, never authority.
use super::{finished, RootStore, RootStoreError as E};
use crate::{AgentLaunchError as A, ExchangeArtifact, IndependentReview, PlanningEvent, Work};
use rusqlite::{params, Connection};

/// Complete public review data with its verified certificate reference. No raw
/// private journal, provider facts, PID/host or future execution right is exposed.
pub(crate) struct NativeReviewObservation {
    pub review: IndependentReview,
    /// Verified original managed context for internal current-source comparison.
    /// Private attributed data, not serialized in public DTOs or an acceptance permit.
    pub(super) managed_context: Option<crate::ManagedReviewContext>,
    pub certificate: ExchangeArtifact,
    /// Recorded verdict versus failed/stale observation; never an execution right.
    pub verdict_applicable: bool,
    /// Known original response failure/interruption; never a verdict or grant.
    pub retry_failure: Option<crate::review_evidence::NativeReviewFailure>,
    /// Private original body digest only; SR1 framing is verified separately.
    /// No capture or provider text is exposed.
    pub original_receipt_sha256: String,
    /// Original control body digest bound by the authoritative reviewer job.
    pub(super) validation_receipt_sha256: String,
    pub(super) task: String,
    pub(super) revision: i64,
}
/// Match a persisted production retry signal to the newest canonical retry
/// decision for this task in the same readonly snapshot. Exact correction,
/// scope and original review input must match. Malformed/stale history refuses;
/// SQL/codec/finalization causes survive. No actor authenticity, mutation or grant.
fn verify_production_retry_decision(db: &Connection, work: &Work, task: &crate::Task) -> Result<(), E> {
    let review = task.independent_review.as_deref().ok_or(E::Refused("native-retry-original-review-required"))?;
    verify_production_retry_decision_for_review(db, work, task, review)
}
/// Read only a canonical decision bound to the exact verified current/prior
/// review; the caller establishes which real task history entry is applicable.
/// Missing/stale/codec/SQL causes refuse, with no synthetic state or mutation.
fn verify_production_retry_decision_for_review(db: &Connection, work: &Work, task: &crate::Task, review: &IndependentReview) -> Result<(), E> {
    verify_production_retry_decision_for_input(db, work, task, &review.id)
}
/// Match a canonical retry to an already verified owned review/end input and
/// its atomic acknowledgement. Caller verifies the input's original identity;
/// this reader creates no synthetic review or authorization from a string.
pub(super) fn verify_production_retry_decision_for_input(db: &Connection, work: &Work, task: &crate::Task, input: &str) -> Result<(), E> {
    verify_retry_decision_for_input(db, work, task, input, "retry", None).map(|_| ())
}
/// Read the canonical reviewer-only correction and its exact acknowledgement.
/// This internal discriminator is protocol, not a request-supplied permission.
/// Same malformed-history/IO/codec refusal contract as production; no mutation.
pub(super) fn verify_review_retry_decision_for_input(db: &Connection, work: &Work, task: &crate::Task, input: &str, original_sha: &str, failure: crate::review_evidence::NativeReviewFailure) -> Result<String, E> {
    verify_retry_decision_for_input(db, work, task, input, "review-retry", Some(RetryOriginal { body_sha: original_sha, failure }))
}
/// Shared canonical read under an internal static operation discriminator. Only
/// the corresponding latest task correction may match; source/input/next and
/// stored acknowledgement stay exact. No execution, budget or state change.
fn verify_retry_decision_for_input(db: &Connection, work: &Work, task: &crate::Task, input: &str, kind: &'static str, original: Option<RetryOriginal<'_>>) -> Result<String, E> {
    let mut events = work.planning.iter().flat_map(|p| p.inbox.iter().flatten()).filter(|e| e.id == input && e.task == task.id && e.scope == task.scope_id);
    let received = events.next().ok_or(E::Refused("native-retry-input-acknowledgement-required"))?;
    if events.next().is_some() { return Err(E::Refused("native-retry-input-acknowledgement-ambiguous")); }
    crate::agent_launch::query(db, "SELECT id,revision,request FROM main.events WHERE work_id=? AND kind='planning.decide' ORDER BY revision DESC", [&work.id], |rows| {
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?; let revision: i64 = row.get(1)?; let raw: Vec<u8> = row.get(2)?;
            let request = crate::decode_planning_request(&raw).map_err(|e| A::caused("native-retry-decision-format", e))?;
            if request.event_id != id || request.expected_revision.checked_add(1) != Some(revision) || revision > work.revision {
                return Err(A::Refused("native-retry-decision-inconsistent"));
            }
            if let Some(op) = request.operations.iter().flatten().find(|op| op.kind == kind && op.id == task.id) {
                if request.scope != task.scope_id || op.next != task.next || !crate::text::nonempty(&op.next)
                    {
                    return Err(A::Refused("native-retry-decision-changed"));
                }
                let direct = received.decision == id && request.input_events.iter().flatten().any(|v| v == input);
                if !direct {
                    if kind != "review-retry" { return Err(A::Refused("native-retry-decision-changed")); }
                    verify_acknowledged_review_receipt(db, work, task, received, &request, revision, original.ok_or(A::Refused("native-retry-original-required"))?)?;
                }
                return Ok(id);
            }
        }
        Err(A::Refused("native-retry-decision-required"))
    }).map_err(|e| E::Launch(Box::new(e)))
}
/// Actual private original observation, not caller-supplied permission.
#[derive(Clone, Copy)]
struct RetryOriginal<'a> { body_sha: &'a str, failure: crate::review_evidence::NativeReviewFailure }
/// Validate the immutable received context of the *new* correction, not today's
/// replaceable delivery. The old incident/ack/body and all fresh consumed inputs
/// must match the actual Work. Claim event, scope, holder, revision and context
/// digest bind the receipt; malformed/missing/ambiguous SQL or JSON refuses with
/// cause. No old event is reconsumed, no identity or execution right is inferred.
fn verify_acknowledged_review_receipt(db: &Connection, work: &Work, task: &crate::Task, received: &PlanningEvent,
    request: &crate::PlanningRequest, revision: i64, original: RetryOriginal<'_>) -> Result<(), A> {
    if !crate::planning_call_inputs::schema(db, false)? { return Err(A::Refused("native-review-retry-receipt-required")); }
    let (claim_id, claim_revision, recorded_at, context, digest): (String,i64,String,String,String) = crate::agent_launch::query(db,
        "SELECT claim_id,claim_revision,recorded_at,context_json,context_sha256 FROM native_planning_claims WHERE work_id=? AND scope_id=? AND holder=? AND generation=? AND scope_revision=?",
        params![work.id,request.scope,request.holder,request.generation,request.scope_revision], |rows| {
            let r = rows.next()?.ok_or(A::Refused("native-review-retry-receipt-required"))?;
            let value = (r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?);
            if rows.next()?.is_some() { return Err(A::Refused("native-review-retry-receipt-ambiguous")); }
            Ok(value)
        })?;
    let (claim_raw, claim_at, event_revision): (Vec<u8>,String,i64) = crate::agent_launch::query(db,
        "SELECT request,at,revision FROM events WHERE id=? AND work_id=? AND kind='planning.claim'",params![claim_id,work.id],|rows| {
            let r = rows.next()?.ok_or(A::Refused("native-review-retry-claim-required"))?;
            let value = (r.get(0)?,r.get(1)?,r.get(2)?);
            if rows.next()?.is_some() { return Err(A::Refused("native-review-retry-claim-ambiguous")); }
            Ok(value)
        })?;
    let claim = crate::decode_planning_request(&claim_raw).map_err(|e| A::caused("native-review-retry-claim-format",e))?;
    if claim.event_id != claim_id || claim.expected_revision.checked_add(1) != Some(claim_revision)
        || claim_revision != event_revision || claim_revision >= revision || claim_at != recorded_at
        || claim.scope != request.scope || claim.scope_revision != request.scope_revision || claim.holder != request.holder
        || crate::provider_configuration::configuration_digest(context.as_bytes()) != digest {
        return Err(A::Refused("native-review-retry-receipt-binding"));
    }
    let value: serde_json::Value = serde_json::from_str(&context).map_err(|e| A::caused("native-review-retry-context-format",e))?;
    let events: Vec<PlanningEvent> = serde_json::from_value(value.get("events").cloned().ok_or(A::Refused("native-review-retry-context-events"))?)
        .map_err(|e| A::caused("native-review-retry-context-events",e))?;
    let incidents: Vec<PlanningEvent> = serde_json::from_value(value.get("unresolved_review_events").cloned().ok_or(A::Refused("native-review-retry-context-incident"))?)
        .map_err(|e| A::caused("native-review-retry-context-incident",e))?;
    let routing: std::collections::BTreeMap<String,crate::ReviewFailureRouting> = serde_json::from_value(value.get("review_failure_routing").cloned().ok_or(A::Refused("native-review-retry-context-routing"))?)
        .map_err(|e| A::caused("native-review-retry-context-routing",e))?;
    let route = routing.get(&task.id).ok_or(A::Refused("native-review-retry-context-routing"))?;
    let inputs = request.input_events.as_deref().unwrap_or_default();
    if received.decision.is_empty() || inputs.is_empty() || inputs.iter().any(|id| id == &received.id)
        || value.get("work").and_then(|v| v.as_str()) != Some(work.id.as_str())
        || value.pointer("/scope/id").and_then(|v| v.as_str()) != Some(task.scope_id.as_str())
        || value.get("historical_only").and_then(|v| v.as_bool()) != Some(false)
        || value.get("review_retry_unavailable").and_then(|v| v.as_bool()) != Some(false)
        || value.get("review_protocol_originals").and_then(|v| v.get(&task.id)).and_then(|v| v.as_str()) != Some(original.body_sha)
        || incidents.iter().filter(|v| v.id == received.id).count() != 1 || !incidents.iter().any(|v| v == received)
        || route.result_event != received.id || route.review_id != received.id || route.attempt != received.attempt
        || route.failure_code != original.failure.code() || route.operation != "review-retry" || route.owner != original.failure.owner()
        || route.producer_correction_required || !route.candidate_commit.is_empty() {
        return Err(A::Refused("native-review-retry-context-changed"));
    }
    for id in inputs {
        let mut matching = events.iter().filter(|e| e.id == *id);
        let old = matching.next().ok_or(A::Refused("native-review-retry-fresh-input-required"))?;
        let mut current = work.planning.iter().flat_map(|p| p.inbox.iter().flatten()).filter(|e| e.id == *id);
        let now = current.next().ok_or(A::Refused("native-review-retry-fresh-input-required"))?;
        let mut expected = old.clone(); expected.decision = request.event_id.clone();
        if matching.next().is_some() || current.next().is_some() || !old.decision.is_empty() || old.scope != task.scope_id || &expected != now {
            return Err(A::Refused("native-review-retry-fresh-input-changed"));
        }
    }
    Ok(())
}
/// Exact scalar ledger projection, not caller-supplied review authority.
struct Row {
    task: String,
    attempt: String,
    producer: String,
    contract: String,
    source: String,
    validation: String,
    hash: String,
    revision: i64,
    phase: String,
}
/// Minimal required receipt projection. Unknown fields, including raw capture
/// byte arrays and provider facts, are skipped by serde without building Values.
/// The framed original is still read whole; this does not bound its stored size.
#[derive(serde::Deserialize)]
struct Original {
    schema_version: u32,
    #[serde(default)]
    managed_review:Option<crate::managed_review_record::Snapshot>,
    failed: bool,
    process_attempted: bool,
    waited: bool,
    output_complete: bool,
    exit_code: Option<i32>,
    signal: Option<i32>,
    grant_attempted: bool,
    channel_closed: bool,
    raw_capture: Capture,
    #[serde(default)]
    causes: Vec<crate::native_diagnostics::NativeGateCause>,
    duration_expired: Option<bool>,
    drain_expired: Option<bool>,
    source_interrupted: Option<bool>,
    job: OriginalJob,
    original_reply: Option<Vec<u8>>,
    finished_at: String,
}
#[derive(serde::Deserialize)]
struct Capture {
    failed: bool,
}
#[derive(serde::Deserialize)]
struct OriginalJob {
    #[serde(default)]
    prompt_sha256:Option<String>,
    id: String,
    work_id: String,
    task_id: String,
    attempt_id: String,
    producer_id: String,
    contract_sha256: String,
    source_sha256: String,
    validation_receipt_sha256: String,
    directory_device: String,
    directory_inode: String,
    permission: i64,
    phase: String,
}
/// Classify only original known terminal failures. Common grant/wait/channel/
/// job/public bindings are independently checked by the enclosing reader. A
/// signal with an unfinished stream is distinct from a decoded protocol error.
/// Unknown expiry, broken capture, changed source or unrelated causes refuse
/// classification. No IO, default policy, public text matching or mutation.
fn classify_retry_failure(original: &Original, verdict: bool, state: &str) -> Option<crate::review_evidence::NativeReviewFailure> {
    use crate::review_evidence::NativeReviewFailure as F;
    if verdict || !original.failed || state != "error" || original.raw_capture.failed
        || original.duration_expired != Some(false) || original.drain_expired != Some(false)
        || original.source_interrupted != Some(false)
        || !original.causes.iter().all(crate::native_diagnostics::NativeGateCause::valid) { return None; }
    if original.exit_code == Some(0) && original.signal.is_none()
        && original.causes.iter().any(|c| c.labels.iter().any(|v| v == "review_fragment_protocol")) { return Some(F::Protocol); }
    if original.exit_code.is_none() && original.signal.is_some_and(|s| s > 0)
        && !original.output_complete && original.original_reply.is_none() && !original.causes.is_empty()
        && original.causes.iter().all(|c| c.path == ["root"] && c.labels == ["planning-output-stream-failed"] && c.io_errno.is_none() && c.sqlite_code.is_none()) { return Some(F::Interrupted); }
    None
}
/// Wire representations, not operator policy: v1 includes its job-addressed
/// reference; v2 excludes both self fields and uses its complete body's SHA256.
#[derive(Clone, Copy)]
pub(super) enum CertificateFormat {
    LegacyV1,
    ContentV2,
}
/// Expected anchored certificate document and its exact compact byte extent.
/// Computing the extent temporarily serializes it before filesystem IO. This
/// avoids guessing overhead and does not claim a global allocation bound.
pub(super) struct ExpectedCertificate {
    pub file_name: String,
    pub sha256: String,
    pub byte_len: usize,
    pub document: serde_json::Value,
}
/// Build the six-field document without IO or authority. Legacy serialization
/// retains receipt exactly as before; both versions exclude their own digest.
/// A non-object review refuses rather than encoding a missing/empty success.
fn certificate_document(
    mut review: serde_json::Value,
    work: &str,
    task: &str,
    source: &str,
    validation: &str,
    format: CertificateFormat,
) -> Result<serde_json::Value, E> {
    let object = review
        .as_object_mut()
        .ok_or(E::Refused("native-review-certificate-review-shape"))?;
    object.remove("receipt_sha256");
    if matches!(format, CertificateFormat::ContentV2) {
        object.remove("receipt");
    }
    let version = match format {
        CertificateFormat::LegacyV1 => 1,
        CertificateFormat::ContentV2 => 2,
    };
    Ok(
        serde_json::json!({"schema_version":version,"review":review,"work_id":work,"task_id":task,"source_sha256":source,"validation_receipt_sha256":validation}),
    )
}
/// Encode the current complete compact public body without either self reference.
/// Its SHA can be used as the final filename before mutating SQL/Work. Codec
/// errors retain their cause; this function performs no IO and grants no rights.
#[cfg(test)]
pub(super) fn certificate_bytes(
    review: serde_json::Value,
    work: &str,
    task: &str,
    source: &str,
    validation: &str,
) -> Result<Vec<u8>, E> {
    certificate_bytes_format(
        review,
        work,
        task,
        source,
        validation,
        CertificateFormat::ContentV2,
    )
}
/// Shared wire encoder for old-read compatibility and explicit test-only v1
/// publication. Default candidate publication selects v2; this is no operator format knob.
/// Serialization failures preserve cause, without filesystem or business effects.
pub(super) fn certificate_bytes_format(
    review: serde_json::Value,
    work: &str,
    task: &str,
    source: &str,
    validation: &str,
    format: CertificateFormat,
) -> Result<Vec<u8>, E> {
    let value = certificate_document(review, work, task, source, validation, format)?;
    serde_json::to_vec(&value).map_err(|e| caused("native-review-certificate-encode", e))
}
/// Classify only exact engine reference syntax. SHA256 lowercase hex length is
/// a wire invariant, not a business limit. This proves neither file nor owner.
fn certificate_location(
    id: &str,
    reference: &str,
    sha: &str,
) -> Result<(String, CertificateFormat), E> {
    if sha.len() != 64
        || !sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(E::Refused("native-review-certificate-digest-shape"));
    }
    let legacy = format!(
        "native-review-public-{}.json",
        crate::provider_configuration::configuration_digest(id.as_bytes())
    );
    let content = format!("native-review-public-v2-{sha}.json");
    if reference == format!(".swarm/{legacy}") {
        Ok((legacy, CertificateFormat::LegacyV1))
    } else if reference == format!(".swarm/{content}") {
        Ok((content, CertificateFormat::ContentV2))
    } else {
        Err(E::Refused("native-review-certificate-reference"))
    }
}
/// Derive the exact stored v1/v2 representation from immutable result metadata.
/// Require typed reference/digest fields, never an arbitrary path or empty hash.
/// The returned bytes count/doc are expectations, not proof that a file exists.
/// Only after bounded read, SHA and full document equality may data be consumed.
pub(super) fn expected_certificate(
    review: serde_json::Value,
    work: &str,
    task: &str,
    source: &str,
    validation: &str,
) -> Result<ExpectedCertificate, E> {
    let text = |key: &str| {
        review
            .get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or(E::Refused("native-review-certificate-record-field"))
    };
    let (name, format) =
        certificate_location(text("id")?, text("receipt")?, text("receipt_sha256")?)?;
    let sha = text("receipt_sha256")?.to_owned();
    let document = certificate_document(review, work, task, source, validation, format)?;
    let byte_len = serde_json::to_vec(&document)
        .map_err(|e| caused("native-review-certificate-size", e))?
        .len();
    Ok(ExpectedCertificate {
        file_name: name,
        sha256: sha,
        byte_len,
        document,
    })
}
/// Preserve original SQL/IO/codec failures behind a static safe family.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}
impl RootStore {
    /// Recheck the archived native correction authority at each execution gate.
    /// Actual current Root row/task must match the caller; the immediately prior
    /// attempt/review, canonical retry decision, private/public reviewer+control
    /// originals and current contract/report/project all remain mandatory. No
    /// synthetic task validation, new reservation or ownership transfer occurs.
    /// A first scoped attempt needs no retry proof. An immediately preceding
    /// failed/interrupted attempt uses its original completion/end signal, not
    /// an unrelated older review. Other unknown histories refuse.
    /// IO/SQL/codec/rollback/close uncertainty prevents grant with typed causes;
    /// checks are sequential observations, not a filesystem lease or actor auth.
    pub(crate) fn verify_native_retry_execution(root: &std::path::Path, supplied: &Work, task_id: &str, settings: &crate::EngineSettingsDocument) -> Result<(), E> {
        let task = supplied.tasks.iter().flatten().find(|t| t.id == task_id).ok_or(E::Refused("native-retry-task-required"))?;
        let attempts = task.attempts.as_deref().unwrap_or_default();
        if supplied.planning.is_none() || attempts.len() < 2 { return Ok(()); }
        if matches!(attempts[attempts.len()-2].status.as_str(), "failed" | "interrupted") {
            return Self::verify_failed_worker_retry_execution(root, supplied, task_id);
        }
        if task.previous_reviews.as_ref().is_none_or(Vec::is_empty) { return Err(E::Refused("native-retry-prior-authority-required")); }
        let store = Self::open(root, false)?;
        let result = (|| {
            let mut tx = store.db.unchecked_transaction().map_err(|e| caused("native-retry-execution-snapshot", e))?;
            let read = (|| {
                let current = crate::read_recorded_work(&tx, &supplied.id).map_err(E::Work)?;
                let mut selected = current.tasks.iter().flatten().filter(|t| t.id == task_id);
                let actual = selected.next().ok_or(E::Refused("native-retry-task-required"))?;
                if selected.next().is_some() || current.revision != supplied.revision || actual != task { return Err(E::Refused("native-retry-execution-source-changed")); }
                let previous = task.previous_reviews.as_ref().and_then(|r| r.last()).ok_or(E::Refused("native-retry-prior-review-required"))?;
                let attempts = task.attempts.as_deref().unwrap_or_default();
                if attempts.len() < 2 || attempts[attempts.len()-2].id != previous.attempt || attempts[attempts.len()-2].status != "completed"
                    || task.independent_review.is_some() || task.planning_retry || previous.state != "changes_requested"
                    || !crate::review_evidence::native_receipt_reference(&previous.receipt) {
                    return Err(E::Refused("native-retry-execution-history-changed"));
                }
                let observed = store.recorded_native_review(&tx, &current.id, &previous.id)?;
                if !observed.verdict_applicable || observed.task != task_id || &observed.review != previous || observed.revision > current.revision {
                    return Err(E::Refused("native-retry-original-review-changed"));
                }
                verify_production_retry_decision_for_review(&tx, &current, task, previous)?;
                if crate::review_contract(task).map_err(E::Encode)? != previous.contract { return Err(E::Refused("native-retry-contract-changed")); }
                let limits = settings.values.context_policy_limits().map_err(E::Settings)?;
                crate::check_project_context_freshness(root, previous.workflow.as_deref().and_then(|w| w.project.as_deref()), "reviewer", &limits)
                    .map_err(|e| caused("native-retry-project-changed", e))?;
                for (name, expected) in previous.report_artifacts.iter().flatten() {
                    let (_, actual) = crate::gate_evaluation::artifact_hash(root, name).map_err(|e| caused("native-retry-artifact-read", e))?;
                    if &actual != expected { return Err(E::Refused("native-retry-artifact-changed")); }
                }
                let (_, actual) = crate::gate_evaluation::artifact_hash(root, &previous.report).map_err(|e| caused("native-retry-report-read", e))?;
                if actual != previous.sha256 { return Err(E::Refused("native-retry-report-changed")); }
                super::task_validation::recorded_evidence_for_prior_review(&store, &current, task_id, previous, &observed.validation_receipt_sha256)?;
                Ok(())
            })();
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            finished(read, tx.rollback().map_err(|e| caused("native-retry-execution-rollback", e)))
        })();
        finished(result, store.close())
    }

    /// Recognize only v1/v2 certificate reference syntax for classification.
    /// No ledger, original, file, freshness, actor or permission is established.
    pub(crate) fn native_certificate_reference(review: &IndependentReview) -> bool {
        certificate_location(&review.id, &review.receipt, &review.receipt_sha256).is_ok()
    }
    /// Read only an original recorded native review using the caller's actual
    /// SQL snapshot. Validate ledger, anchored private receipt/hash/root, immutable
    /// result event and public certificate, including explicit failed/stale outcomes.
    /// No live owner is required for data
    /// readback. This is publication integrity, not current source freshness,
    /// authenticated model/human identity, acceptance, settlement or permission.
    /// Original framed extent controls private reads; unknown capture arrays are
    /// skipped during decoding. Public reads use the exact compact certificate
    /// extent, derived from the original event. Context/report caps still apply.
    pub(super) fn recorded_native_review(
        &self,
        db: &Connection,
        work: &str,
        id: &str,
    ) -> Result<NativeReviewObservation, E> {
        if !swarm_identity_contract::name(id.as_bytes()) {
            return Err(E::Refused("native-review-context-id"));
        }
        let row=db.query_row("SELECT task_id,attempt_id,producer_id,contract_sha256,source_sha256,validation_receipt_sha256,receipt_sha256,published_revision,phase FROM native_review_jobs WHERE id=? AND work_id=?",params![id,work],|r|Ok(Row {task:r.get(0)?,attempt:r.get(1)?,producer:r.get(2)?,contract:r.get(3)?,source:r.get(4)?,validation:r.get(5)?,hash:r.get(6)?,revision:r.get(7)?,phase:r.get(8)?})).map_err(|e|caused("native-review-context-ledger",e))?;
        if !matches!(row.phase.as_str(), "recorded" | "refused") || row.revision <= 0 {
            return Err(E::Refused("native-review-context-not-recorded"));
        }
        let verdict_applicable = row.phase == "recorded";
        let suffix = crate::provider_configuration::configuration_digest(id.as_bytes());
        let raw = self
            .directory
            .read_native_receipt(&format!("native-review-{suffix}.receipt"))
            .map_err(E::Storage)?;
        if crate::provider_configuration::configuration_digest(&raw) != row.hash {
            return Err(E::Refused("native-review-context-private-changed"));
        }
        let original: Original = serde_json::from_slice(&raw)
            .map_err(|e| caused("native-review-context-private-json", e))?;
        let (device, inode) = self.directory.identity().map_err(E::Storage)?;
        if original.schema_version != 2
            || !original.process_attempted
            || !original.waited
            || usize::from(original.exit_code.is_some()) + usize::from(original.signal.is_some())
                != 1
            || original.exit_code.is_some_and(|c| !(0..=255).contains(&c))
            || original.signal.is_some_and(|s| s <= 0)
            || original.job.permission != 1
            || original.job.phase != "permitted"
            || ((verdict_applicable || !original.failed)
                && (original.failed
                    || !original.output_complete
                    || original.exit_code != Some(0)
                    || original.original_reply.is_none()))
            || !original.grant_attempted
            || !original.channel_closed
            || ((verdict_applicable || !original.failed) && original.raw_capture.failed)
            || original.job.id != id
            || original.job.work_id != work
            || original.job.task_id != row.task
            || original.job.attempt_id != row.attempt
            || original.job.producer_id != row.producer
            || original.job.contract_sha256 != row.contract
            || original.job.source_sha256 != row.source
            || original.job.validation_receipt_sha256 != row.validation
            || original.job.directory_device != device.to_string()
            || original.job.directory_inode != inode.to_string()
        {
            return Err(E::Refused("native-review-context-original-binding"));
        }
        let event = format!("{id}-result");
        let (kind, revision, request): (String, i64, Vec<u8>) = db
            .query_row(
                "SELECT kind,revision,request FROM events WHERE id=? AND work_id=?",
                params![event, work],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| caused("native-review-context-result-event", e))?;
        if kind != "review.result" || revision != row.revision {
            return Err(E::Refused("native-review-context-event-revision"));
        }
        let review: IndependentReview = serde_json::from_slice(&request)
            .map_err(|e| caused("native-review-context-record-json", e))?;
        let managed_fields=review.managed_tasks.is_some() || !review.candidate_commit.is_empty()
            || !review.previous_candidate.is_empty() || !review.context.is_empty() || !review.context_sha256.is_empty();
        if review.fragment_journal.is_some() || review.batches.is_some() || !review.git_report.is_empty(){return Err(E::Refused("native-review-context-managed-unported"));}
        match &original.managed_review {
            Some(managed)=>{
                managed.validate().map_err(|e|caused("native-managed-context-original",e))?;
                if original.job.prompt_sha256.as_deref()!=Some(managed.prompt_sha256.as_str()){return Err(E::Refused("native-managed-context-prompt-changed"));}
                if verdict_applicable {
                    let raw=original.original_reply.as_deref().ok_or(E::Refused("native-managed-context-reply-required"))?;
                    if !managed.matches(raw,&review,&row.task).map_err(|e|caused("native-managed-context-record",e))?{return Err(E::Refused("native-managed-context-record-changed"));}
                    let (name,bytes)=managed.context().map_err(|e|caused("native-managed-context-original",e))?;
                    let actual=self.directory.read_bounded(&name,bytes.len()).map_err(E::Storage)?;
                    if actual!=bytes{return Err(E::Refused("native-managed-context-file-changed"));}
                }else if managed_fields{return Err(E::Refused("native-managed-nonverdict-has-verdict"));}
            }
            None if managed_fields=>return Err(E::Refused("native-review-context-managed-unported")),
            None=>{},
        }
        let reply_digest = original
            .original_reply
            .as_deref()
            .map(crate::provider_configuration::configuration_digest)
            .unwrap_or_default();
        if review.id != id
            || review.attempt != row.attempt
            || review.producer != row.producer
            || review.contract != row.contract
            || review.result_event != event
            || review.reply_sha256 != reply_digest
            || (verdict_applicable
                && !matches!(review.state.as_str(), "passed" | "changes_requested"))
            || (!verdict_applicable
                && (review.state != if original.failed { "error" } else { "stale" }
                    || review.criteria.as_ref().is_some_and(|v| !v.is_empty())))
            || original.finished_at != review.finished
        {
            return Err(E::Refused("native-review-context-record-binding"));
        }
        let expected = expected_certificate(
            serde_json::to_value(&review)
                .map_err(|e| caused("native-review-context-record-encode", e))?,
            work,
            &row.task,
            &row.source,
            &row.validation,
        )?;
        let bytes = self
            .directory
            .read_bounded(&expected.file_name, expected.byte_len)
            .map_err(E::Storage)?;
        if crate::provider_configuration::configuration_digest(&bytes) != expected.sha256 {
            return Err(E::Refused("native-review-context-public-changed"));
        }
        let certificate: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| caused("native-review-context-public-json", e))?;
        if certificate != expected.document {
            return Err(E::Refused("native-review-context-certificate-binding"));
        }
        let retry_failure = classify_retry_failure(&original, verdict_applicable, &review.state);
        Ok(NativeReviewObservation {
            certificate: ExchangeArtifact {
                path: review.receipt.clone(),
                sha256: review.receipt_sha256.clone(),
            },
            verdict_applicable,
            retry_failure,
            original_receipt_sha256: row.hash,
            validation_receipt_sha256: row.validation,
            managed_context: original.managed_review.map(|value| value.protocol.context),
            task: row.task,
            revision: row.revision,
            review,
        })
    }
    /// Verify the current task's purpose-specific native result through a fresh readonly
    /// Root/SQL snapshot. Caller retains its actual Work/settings transaction;
    /// matching the current revision/task and the original result is mandatory.
    /// Framed private bytes, scalar job/event and exact certificate are checked
    /// by the same reader as planner context. Original control job/step frames
    /// and their public/event bindings are also re-read using the exact body SHA
    /// bound into that reviewer job. No inbox acknowledgement, mutation,
    /// grant, acceptance or authenticated reviewer/human identity follows.
    /// Internal purpose keeps passed acceptance separate from changes_requested
    /// production retry. ReviewerRetry requires a known original response failure
    /// or terminal interruption and never promotes it into a verdict. An existing retry
    /// signal also requires its canonical
    /// decision/input/correction. Ordinary/managed data is not a fallback. All open/read/codec/
    /// rollback/close failures retain their causes; a source check is no FS lease.
    pub(crate) fn verify_current_native_review(
        root: &std::path::Path,
        work: &Work,
        task_id: &str,
        purpose: crate::review_evidence::CurrentReviewUse,
        settings: &crate::EngineSettingsDocument,
    ) -> Result<(), E> {
        let store = Self::open(root, false)?;
        let result = (|| {
            let mut tx = store.db.unchecked_transaction()
                .map_err(|e| caused("native-review-current-snapshot", e))?;
            let read = (|| {
                let current = crate::read_recorded_work(&tx, &work.id).map_err(E::Work)?;
                let mut supplied = work.tasks.iter().flatten().filter(|t| t.id == task_id);
                let task = supplied.next().ok_or(E::Refused("native-review-current-task-required"))?;
                if supplied.next().is_some() {
                    return Err(E::Refused("native-review-current-task-ambiguous"));
                }
                let mut stored = current.tasks.iter().flatten().filter(|t| t.id == task_id);
                let actual = stored.next().ok_or(E::Refused("native-review-current-task-required"))?;
                if stored.next().is_some() || current.revision != work.revision || actual != task {
                    return Err(E::Refused("native-review-current-task-changed"));
                }
                let review = task.independent_review.as_deref()
                    .ok_or(E::Refused("native-review-current-result-required"))?;
                if review.managed_tasks.is_some() || !review.candidate_commit.is_empty() {
                    if purpose != crate::review_evidence::CurrentReviewUse::Acceptance {
                        return Err(E::Refused("native-managed-current-review-unported"));
                    }
                    super::managed_publication::verify_published_current(&store,&tx,&current,task,settings)?;
                }
                let observed = store.recorded_native_review(&tx, &work.id, &review.id)?;
                let applicable = match purpose {
                    crate::review_evidence::CurrentReviewUse::ReviewerRetry => observed.retry_failure.is_some(),
                    _ => observed.verdict_applicable,
                };
                if !applicable || observed.review.state != purpose.state()
                    || observed.task != task_id || &observed.review != review
                    || observed.revision > current.revision
                {
                    return Err(E::Refused("native-review-current-result-changed"));
                }
                if purpose == crate::review_evidence::CurrentReviewUse::ProductionRetry && task.planning_retry {
                    verify_production_retry_decision(&tx, &current, task)?;
                }
                super::task_validation::recorded_evidence_for_review_result(
                    &store, &current, task_id, &observed.validation_receipt_sha256,
                )?;
                Ok(())
            })();
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            finished(read, tx.rollback().map_err(|e| caused("native-review-current-rollback", e)))
        })();
        finished(result, store.close())
    }
    /// Read a selected native event through a fresh readonly Root/SQL snapshot,
    /// matching its current authoritative scoped inbox item to the original
    /// recorded outcome and task owner (the result payload is not a Work snapshot). Close/rollback failures
    /// remain errors with original cause. No acknowledgement, model call or Work
    /// change occurs; acknowledged incidents also require their canonical scoped
    /// consumption decision. This supplies untrusted data, not agent acceptance.
    pub(crate) fn native_review_for_context(
        root: &std::path::Path,
        work: &Work,
        event: &PlanningEvent,
    ) -> Result<NativeReviewObservation, E> {
        let store = Self::open(root, false)?;
        let result = (|| {
            let mut tx = store
                .db
                .unchecked_transaction()
                .map_err(|e| caused("native-review-context-snapshot", e))?;
            let read = (|| {
                let observed = store.recorded_native_review(&tx, &work.id, &event.id)?;
                let current = crate::read_recorded_work(&tx, &work.id).map_err(E::Work)?;
                let pending = current
                    .planning
                    .as_ref()
                    .and_then(|p| p.inbox.as_ref())
                    .and_then(|items| items.iter().find(|e| e.id == event.id))
                    .ok_or(E::Refused("native-review-context-inbox-required"))?;
                let owner = current
                    .tasks
                    .iter()
                    .flatten()
                    .find(|t| t.id == observed.task)
                    .ok_or(E::Refused("native-review-context-task-required"))?;
                if !event.decision.is_empty() {
                    let (raw, revision): (Vec<u8>, i64) = tx.query_row(
                        "SELECT request,revision FROM events WHERE id=? AND work_id=? AND kind='planning.decide'",
                        params![event.decision,work.id], |r| Ok((r.get(0)?,r.get(1)?)))
                        .map_err(|e| caused("native-review-context-ack-required", e))?;
                    let ack = crate::decode_planning_request(&raw).map_err(|e| caused("native-review-context-ack-format", e))?;
                    if ack.event_id != event.decision || ack.scope != owner.scope_id
                        || ack.expected_revision.checked_add(1) != Some(revision) || revision > current.revision
                        || !ack.input_events.iter().flatten().any(|id| id == &event.id) {
                        return Err(E::Refused("native-review-context-ack-changed"));
                    }
                }
                let expected = PlanningEvent {
                    id: observed.review.id.clone(),
                    scope: owner.scope_id.clone(),
                    kind: "independent_review".into(),
                    task: observed.task.clone(),
                    attempt: observed.review.attempt.clone(),
                    message: format!("{} : {}", observed.review.state, observed.review.reason),
                    artifacts: if observed.verdict_applicable {
                        Some(vec![ExchangeArtifact {
                            path: observed.review.report.clone(),
                            sha256: observed.review.sha256.clone(),
                        }])
                    } else {
                        None
                    },
                    at: observed.review.finished.clone(),
                    decision: event.decision.clone(),
                    ..Default::default()
                };
                if pending != event
                    || event != &expected
                    || observed.revision > work.revision
                    || observed.revision > current.revision
                    || !owner
                        .attempts
                        .iter()
                        .flatten()
                        .any(|a| a.id == observed.review.attempt)
                {
                    return Err(E::Refused("native-review-context-inbox-changed"));
                }
                Ok(observed)
            })();
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            finished(
                read,
                tx.rollback()
                    .map_err(|e| caused("native-review-context-rollback", e)),
            )
        })();
        finished(result, store.close())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Wire classification only, never a Root/job/receipt grant. Real native
    /// signal+wait and canonical recovery are exercised by the public recipe.
    /// Unknown expiry/capture/cause and ambiguous termination must not classify.
    #[test]
    fn native_retry_failure_classification_separates_known_signal_and_protocol() {
        use crate::review_evidence::NativeReviewFailure as F;
        let wire = serde_json::json!({
            "schema_version":2,"failed":true,"process_attempted":true,"waited":true,
            "output_complete":false,"exit_code":null,"signal":15,"grant_attempted":true,"channel_closed":true,
            "raw_capture":{"failed":false},"causes":[{"path":["root"],"labels":["planning-output-stream-failed"],"io_errno":null,"sqlite_code":null}],
            "duration_expired":false,"drain_expired":false,"source_interrupted":false,"original_reply":null,"finished_at":"wire-only",
            "job":{"id":"wire-only","work_id":"wire-only","task_id":"wire-only","attempt_id":"wire-only","producer_id":"wire-only", "contract_sha256":"wire-only","source_sha256":"wire-only","validation_receipt_sha256":"wire-only","directory_device":"wire-only","directory_inode":"wire-only","permission":1,"phase":"permitted"}
        });
        let decode = |v| serde_json::from_value::<Original>(v).unwrap();
        assert!(matches!(classify_retry_failure(&decode(wire.clone()), false, "error"),Some(F::Interrupted)));
        assert!(classify_retry_failure(&decode(wire.clone()), true, "error").is_none());
        assert!(classify_retry_failure(&decode(wire.clone()), false, "passed").is_none());
        for (pointer,replacement) in [
            ("/failed",serde_json::json!(false)), ("/exit_code",serde_json::json!(0)),
            ("/signal",serde_json::Value::Null), ("/signal",serde_json::json!(0)),
            ("/output_complete",serde_json::json!(true)), ("/original_reply",serde_json::json!([])),
            ("/raw_capture/failed",serde_json::json!(true)), ("/duration_expired",serde_json::json!(true)),
            ("/duration_expired",serde_json::Value::Null), ("/drain_expired",serde_json::json!(true)),
            ("/source_interrupted",serde_json::json!(true)), ("/causes",serde_json::json!([])),
            ("/causes/0/labels",serde_json::json!(["unknown"])), ("/causes/0/io_errno",serde_json::json!(5)),
        ] {
            let mut changed = wire.clone(); *changed.pointer_mut(pointer).unwrap() = replacement;
            assert!(classify_retry_failure(&decode(changed), false, "error").is_none(),"{pointer}");
        }
        let mut protocol = wire.clone();protocol["exit_code"]=serde_json::json!(0);protocol["signal"]=serde_json::Value::Null;
        protocol["causes"][0]["labels"]=serde_json::json!(["review_fragment_protocol"]);
        assert!(matches!(classify_retry_failure(&decode(protocol),false,"error"),Some(F::Protocol)));
        // EOF/stream failure without a terminal signal never becomes retryable.
        let mut eof = wire;eof["signal"]=serde_json::Value::Null;eof["exit_code"]=serde_json::json!(0);
        assert!(classify_retry_failure(&decode(eof),false,"error").is_none());
    }
    /// Byte-format fixture only: no Root, job, OS process or positive engine
    /// receipt. Removing both self references makes the complete canonical body
    /// independently hashable; a changed result must have a distinct address.
    #[test]
    fn native_public_certificate_v2_omits_self_and_separates_payloads() {
        let mut review = IndependentReview {
            id: "wire-only".into(),
            reason: "first wire value".into(),
            receipt: ".swarm/old-self-reference.json".into(),
            receipt_sha256: "not-a-signature".into(),
            ..Default::default()
        };
        let first = certificate_bytes(
            serde_json::to_value(&review).unwrap(),
            "wire-work",
            "wire-task",
            &"0".repeat(64),
            &"1".repeat(64),
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(
            value["schema_version"], 2,
            "new body must explicitly identify its non-self-referential protocol"
        );
        assert!(
            value["review"].get("receipt").is_none()
                && value["review"].get("receipt_sha256").is_none()
        );
        review.reason = "second wire value".into();
        let second = certificate_bytes(
            serde_json::to_value(&review).unwrap(),
            "wire-work",
            "wire-task",
            &"0".repeat(64),
            &"1".repeat(64),
        )
        .unwrap();
        assert_ne!(
            crate::provider_configuration::configuration_digest(&first),
            crate::provider_configuration::configuration_digest(&second)
        );
    }
    /// Actual historical v1 bytes retained from a real inert fixture, never
    /// seeded as a new Root receipt. The omitted self hash is reconstructed from
    /// those exact bytes solely to check representation and reference handling.
    #[test]
    fn native_public_certificate_real_v1_bytes_match_legacy_expected() {
        let raw = include_bytes!("../../../../../tests/fixtures/native-review-public-v1.json");
        let original: serde_json::Value = serde_json::from_slice(raw).unwrap();
        assert_eq!(original["schema_version"], 1);
        let mut review = original["review"].clone();
        review["receipt_sha256"] = crate::provider_configuration::configuration_digest(raw).into();
        let expected = expected_certificate(
            review,
            original["work_id"].as_str().unwrap(),
            original["task_id"].as_str().unwrap(),
            original["source_sha256"].as_str().unwrap(),
            original["validation_receipt_sha256"].as_str().unwrap(),
        )
        .unwrap();
        assert_eq!(expected.byte_len, raw.len());
        assert_eq!(expected.document, original);
        assert_eq!(
            format!(".swarm/{}", expected.file_name),
            original["review"]["receipt"].as_str().unwrap()
        );
    }
}
