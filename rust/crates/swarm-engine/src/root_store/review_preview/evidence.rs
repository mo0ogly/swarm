//! Optional operator observations kept outside the retained Git candidate.
use super::E;
use serde::Deserialize;
use std::{fs, io, path::{Path, PathBuf}};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema_version: u32,
    work_id: String,
    task_id: String,
    attempt_id: String,
    candidate_commit: String,
    report_sha256: String,
    observations: Vec<Observation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    criterion: usize,
    source: String,
    detail: String,
}

/// Locate the optional regular project evidence document for these observed IDs.
/// Symlinked/non-directory ancestors or an invalid leaf are errors. Only absence
/// permits an ordinary review without supplementary evidence. The caller reads
/// under saved limits and binds the content separately before it can be quoted.
pub(super) fn path(root: &Path, work: &str, task: &str) -> Result<Option<String>, E> {
    if ![work, task].iter().all(|id| swarm_identity_contract::name(id.as_bytes())) {
        return Err(E::Refused("review-evidence-id-invalid"));
    }
    let relative = format!("docs/swarm-review-evidence/{work}/{task}.json");
    let mut parents = PathBuf::from(root);
    let mut components = relative.split('/').peekable();
    while let Some(component) = components.next() {
        parents.push(component);
        let metadata = match fs::symlink_metadata(&parents) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(caused("review-evidence-path", error)),
        };
        if components.peek().is_some() {
            if !metadata.is_dir() {
                return Err(E::Refused("review-evidence-directory-required"));
            }
        } else {
            if !metadata.is_file() {
                return Err(E::Refused("review-evidence-regular-file-required"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let owner = root.metadata().map_err(|e| caused("review-evidence-root", e))?.uid();
                if metadata.uid() != owner || metadata.mode() & 0o022 != 0 || metadata.nlink() != 1 {
                    return Err(E::Refused("review-evidence-owner-or-mode"));
                }
            }
        }
    }
    Ok(Some(relative))
}

/// Bind supplementary bytes to this exact work/task/attempt/candidate/report.
/// An observation can refer to a criterion once and must identify its source.
/// Evidence is attributed operator data. No provider identity, criterion truth,
/// mutation or acceptance is established by this parser.
pub(super) fn verify(
    text: &str, work: &str, task: &str, attempt: &str, candidate: &str,
    report: &str, criteria: usize,
) -> Result<(), E> {
    let evidence: Evidence = serde_json::from_str(text)
        .map_err(|e| caused("review-evidence-schema-invalid", e))?;
    if evidence.schema_version != 1 || evidence.work_id != work || evidence.task_id != task
        || evidence.attempt_id != attempt || evidence.candidate_commit != candidate
        || evidence.report_sha256 != report
    {
        return Err(E::Refused("review-evidence-binding-changed"));
    }
    let mut seen = std::collections::BTreeSet::new();
    if evidence.observations.is_empty() || evidence.observations.iter().any(|entry|
        entry.criterion == 0 || entry.criterion > criteria || !seen.insert(entry.criterion)
        || entry.source.trim().is_empty() || entry.detail.trim().is_empty())
    {
        return Err(E::Refused("review-evidence-observations-invalid"));
    }
    Ok(())
}

/// Preserve typed errors behind safe stage labels without echoing file contents.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(crate::AgentLaunchError::caused(code, error)))
}
