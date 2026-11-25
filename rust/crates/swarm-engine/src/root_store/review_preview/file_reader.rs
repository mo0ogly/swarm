//! Operator read of files bound to the task’s recorded submission and candidate.
use super::{finished, task_validation, RootStore, E};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

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
    let text = crate::read_review_delivery(root, &report.report_path, limits)
        .map_err(E::Context)?;
    if crate::provider_configuration::configuration_digest(text.as_bytes()) != report.report_sha256 {
        return Err(E::Refused("task-review-file-digest-changed"));
    }
    Ok(json!({"id":"report","text":text}))
}

impl RootStore {
    /// List the submitted report and the selected task's declared control inputs,
    /// or open one named original. An inventory caller may opt in to the report
    /// bytes in the same observation. A file-level read error remains explicit
    /// and does not hide the verified inventory. The request is the revision-bound review
    /// envelope. For managed work, delegation provenance and a held candidate
    /// bind the allowlist, type and SHA-256 to exact Git bytes. Current policy
    /// limits and revision checks apply. Published controls are sufficient for
    /// this historical read without preparing a reviewer. No state is written.
    pub fn task_review_files(
        &self,
        work: &str,
        raw: &[u8],
        file: Option<&str>,
        include_report: bool,
    ) -> Result<Value, E> {
        if include_report && file.is_some() {
            return Err(E::Refused("task-review-files-query-invalid"));
        }
        let request = task_validation::decode(raw).map_err(|error| E::Launch(Box::new(error)))?;
        crate::root_store::material_path::with_material_read(self, || {
            let settings = self.read_settings()?;
            let tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            let observed = (|| {
                let observed = crate::recorded_work::observe_work(&tx, work).map_err(E::Work)?;
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
                let candidate = if managed {
                    Some(task_validation::published_managed_candidate(
                        self, current, &task.id,
                    )?)
                } else {
                    None
                };
                let report = task_validation::submission(&tx, current, &request.task_id)
                    .map_err(|e| E::Launch(Box::new(e)))?;
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
                let limits = settings.values.context_policy_limits().map_err(E::Settings)?;
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
                    let limits = settings.values.context_policy_limits().map_err(E::Settings)?;
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
            Ok(json!({"schema_version":1,"work":work,"task":task,
                "revision":current.revision,"data":data}))
        })
    }
}
