//! Operator read of files bound to the task’s recorded submission and candidate.
use super::{finished, task_validation, RootStore, E};
use crate::recorded_work::ObservedWork;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Select the supported text and document formats. This reader has no
/// executable renderer. Other declared kinds stay visible without bytes.
fn kind(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("md" | "markdown") => "markdown",
        Some("json" | "jsonl") => "json",
        Some("log") => "log",
        Some("txt" | "csv" | "toml" | "yaml" | "yml" | "rs" | "go" | "py" | "js") => "text",
        Some("docx" | "odt" | "pdf") => "document",
        _ => "unsupported",
    }
}

/// Read the recorded submission's report under the saved delivery size limit
/// and compare its bytes with the submission digest. The caller must first
/// establish task authority and check the Work revision again before replying.
fn submitted_report(
    root: &Path,
    report: &crate::TaskReportRequest,
    limits: &crate::ContextPolicyLimits,
) -> Result<Value, E> {
    let text =
        crate::read_review_delivery(root, &report.report_path, limits).map_err(E::Context)?;
    if crate::provider_configuration::configuration_digest(text.as_bytes()) != report.report_sha256
    {
        return Err(E::Refused("task-review-file-digest-changed"));
    }
    Ok(json!({"id":"report","text":text}))
}

/// Retain one decoded Work for a service bound to a single Root. The cache
/// holds no file bytes, decision, receipt or authorization. Each request checks
/// the persisted Work digest before reuse and rechecks its file provenance.
#[derive(Default)]
pub struct TaskReviewFileReader {
    last_work: Mutex<Option<Arc<ObservedWork>>>,
}

impl RootStore {
    /// Read the requested inventory or named original without retaining a
    /// decode between requests. The revision and all evidence are checked.
    pub fn task_review_files(
        &self,
        work: &str,
        raw: &[u8],
        file: Option<&str>,
        include_report: bool,
    ) -> Result<Value, E> {
        self.task_review_files_cached(
            work,
            raw,
            file,
            include_report,
            &TaskReviewFileReader::default(),
        )
    }

    /// List the submitted report and the selected task's declared control inputs,
    /// or open one named original. An inventory caller may opt in to the report
    /// bytes in the same observation. A file-level read error remains explicit
    /// and does not hide the verified inventory. The request is the revision-bound review
    /// envelope. For managed work, delegation provenance and a held candidate
    /// bind the allowlist, type and SHA-256 to exact Git bytes. Current policy
    /// limits and revision checks apply. Published controls are sufficient for
    /// this historical read without preparing a reviewer. No state is written.
    pub fn task_review_files_cached(
        &self,
        work: &str,
        raw: &[u8],
        file: Option<&str>,
        include_report: bool,
        reader: &TaskReviewFileReader,
    ) -> Result<Value, E> {
        self.task_review_files_profiled(work, raw, file, include_report, reader, false)
    }

    /// Perform the same protected read and optionally report elapsed phase times
    /// on a successful named file. Values use a monotonic clock and contain no
    /// payload, path, authority or policy details. Failures keep their usual causes.
    pub fn task_review_files_profiled(
        &self,
        work: &str,
        raw: &[u8],
        file: Option<&str>,
        include_report: bool,
        reader: &TaskReviewFileReader,
        profile: bool,
    ) -> Result<Value, E> {
        let started = Instant::now();
        let mut receipt_timing = task_validation::ReceiptTiming::default();
        let (mut work_elapsed, mut receipt_elapsed, mut submission_elapsed, mut git_elapsed) =
            (Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO);
        if include_report && file.is_some() {
            return Err(E::Refused("task-review-files-query-invalid"));
        }
        let request = task_validation::decode(raw).map_err(|error| E::Launch(Box::new(error)))?;
        // Lock only to exchange namespaced immutable data, never while doing IO.
        // Poisoning disables the optimization; it cannot authorize a read.
        let previous = reader.last_work.lock().ok().and_then(|value| value.clone());
        let (value, observed) = crate::root_store::material_path::with_material_read(self, || {
            let settings = self.read_settings()?;
            let tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            let observed = (|| {
                let snapshot_started = Instant::now();
                let observed = crate::recorded_work::observe_work(&tx, work, previous.as_ref())
                    .map_err(E::Work)?;
                work_elapsed = snapshot_started.elapsed();
                let current = &observed.work;
                if current.revision != request.expected_revision {
                    return Err(E::Refused("review-revision-conflict"));
                }
                let task = current
                    .tasks
                    .iter()
                    .flatten()
                    .find(|task| task.id == request.task_id)
                    .ok_or(E::Refused("review-task-missing"))?;
                let managed = current
                    .planning
                    .as_deref()
                    .is_some_and(|planning| planning.repository.is_some());
                let receipt_started = Instant::now();
                let candidate = if managed {
                    Some(task_validation::published_managed_candidate(
                        self, current, &task.id, profile.then_some(&mut receipt_timing),
                    )?)
                } else {
                    None
                };
                receipt_elapsed = receipt_started.elapsed();
                let submission_started = Instant::now();
                let report = task_validation::submission(&tx, current, &request.task_id)
                    .map_err(|e| E::Launch(Box::new(e)))?;
                submission_elapsed = submission_started.elapsed();
                Ok((observed, candidate, report))
            })();
            let (observed, candidate, report) = finished(observed, tx.rollback().map_err(E::Sql))?;
            let current = &observed.work;
            let candidate = candidate.as_ref();
            let task = &request.task_id;
            let mut files = BTreeMap::new();
            files.insert(
                "report".to_owned(),
                ("Rapport.md".to_owned(), Some(report.report_sha256.clone())),
            );
            if let Some(candidate) = candidate {
                let repository = current
                    .planning
                    .as_deref()
                    .and_then(|planning| planning.repository.as_deref())
                    .ok_or(E::Refused("review-managed-repository-required"))?;
                let checkpoint = candidate;
                let plan = checkpoint
                    .control_plan
                    .as_ref()
                    .ok_or(E::Refused("review-managed-plan-required"))?;
                for slot in plan.slots.iter().filter(|slot| slot.task_id == *task) {
                    for input in slot.control.inputs.iter().flatten() {
                        let path = if repository.project_subdir.is_empty() {
                            input.clone()
                        } else {
                            format!("{}/{input}", repository.project_subdir)
                        };
                        let hash = checkpoint
                            .review_entry(&path)
                            .filter(|(mode, _)| matches!(*mode, "100644" | "100755"))
                            .map(|(_, hash)| hash.to_owned());
                        files.entry(path.clone()).or_insert((path, hash));
                    }
                }
            }
            let data = if file == Some("report") {
                let limits = settings
                    .values
                    .context_policy_limits()
                    .map_err(E::Settings)?;
                submitted_report(&self.root, &report, &limits)?
            } else if let Some(path) = file {
                let (display, hash) = files
                    .get(path)
                    .ok_or(E::Refused("task-review-file-not-declared"))?;
                let hash = hash
                    .as_ref()
                    .ok_or(E::Refused("task-review-file-unavailable"))?;
                if kind(display) == "unsupported" {
                    return Err(E::Refused("task-review-file-type-unsupported"));
                }
                let limits = settings
                    .values
                    .context_policy_limits()
                    .map_err(E::Settings)?;
                let candidate = candidate.ok_or(E::Refused("task-review-file-unavailable"))?;
                let bytes = crate::managed_repository::read_original_blob(
                    &self.root,
                    current,
                    candidate,
                    path,
                    &settings,
                    limits.review_delivery_bytes,
                    &mut git_elapsed,
                )
                .map_err(|error| E::Launch(Box::new(error)))?;
                if crate::provider_configuration::configuration_digest(&bytes) != *hash {
                    return Err(E::Refused("task-review-file-digest-changed"));
                }
                match kind(display) {
                    "document" => json!({"id":path,"download":STANDARD.encode(bytes)}),
                    _ => {
                        let text = String::from_utf8(bytes)
                            .map_err(|_| E::Refused("task-review-file-not-text"))?;
                        json!({"id":path,"text":text})
                    }
                }
            } else {
                let entries: Vec<_> = files.into_iter().map(|(id, (path, hash))| {
                    let category = kind(&path);
                    json!({"id":id,"path":path,"kind":if category == "unsupported" {"document"} else {category},
                        "readable":hash.is_some() && category != "unsupported"})
                }).collect();
                if include_report {
                    let limits = settings
                        .values
                        .context_policy_limits()
                        .map_err(E::Settings)?;
                    let report = submitted_report(&self.root, &report, &limits)
                        .unwrap_or_else(|error| json!({"id":"report","error":error.code()}));
                    json!({"files":entries,"report":report})
                } else {
                    json!({"files":entries})
                }
            };
            if !observed.is_current(&self.db).map_err(E::Work)? {
                return Err(E::Refused("review-revision-conflict"));
            }
            Ok((
                json!({"schema_version":1,"work":work,"task":task,
                "revision":current.revision,"data":data}),
                observed,
            ))
        })?;
        if let Ok(mut last) = reader.last_work.lock() {
            *last = Some(observed);
        }
        let mut value = value;
        if profile && file.is_some() {
            let elapsed = started.elapsed();
            let millis = |duration: Duration| duration.as_secs_f64() * 1000.0;
            value["timing_ms"] = json!({
                "engine": millis(elapsed),
                "checks_and_preparation": millis(elapsed.saturating_sub(git_elapsed)),
                "work_snapshot": millis(work_elapsed),
                "receipt": millis(receipt_elapsed),
                "receipt_phases": receipt_timing,
                "submission": millis(submission_elapsed),
                "git": millis(git_elapsed),
            });
        }
        Ok(value)
    }
}
