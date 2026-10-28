//! Immutable worker output, before candidate publication or quality acceptance.
//! The owning Root caller checks current completion, rights and lease separately.
use super::{proof, repository, with_storage, workspace};
use super::super::{commit, git, hash, inside, text, LocalDirectory, E};
use crate::{Agent, EngineSettingsDocument, Work};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Private checkpoint v1 binds a result to one original native worker. A Git ref
/// is recoverable before this file exists; neither checkpoint advances candidate.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RetainedResult {
    schema_version: u32,
    work: String,
    task: String,
    agent: String,
    attempt: String,
    base: String,
    attribution_sha256: String,
    pub(crate) commit: String,
    pub(crate) report_path: String,
    pub(crate) report_sha256: String,
}
impl RetainedResult {
    /// Borrow the worker base from this already verified immutable result.
    /// `verify_result` binds it to the original worker proof and Git parent;
    /// this getter adds no IO, current-source guarantee or execution authority.
    pub(crate) fn worker_base(&self) -> &str { &self.base }
}

/// Decode exactly one regular ls-tree record. Modes and OID lengths are Git
/// representation invariants, not operator limits. Reject malformed/multiple
/// records and mismatched paths; no revision expression is accepted as an OID.
fn parse_blob_entry(raw: &[u8], path: &str) -> Result<String, E> {
    let invalid = || E::Refused("managed-result-report-entry-invalid");
    let body = raw.strip_suffix(&[0]).ok_or_else(invalid)?;
    if body.contains(&0) { return Err(invalid()); }
    let body = std::str::from_utf8(body).map_err(|_| invalid())?;
    let (header, actual) = body.split_once('\t').ok_or_else(invalid)?;
    let parts: Vec<_> = header.split(' ').collect();
    if actual != path || parts.len() != 3 || !matches!(parts[0], "100644" | "100755") || parts[1] != "blob"
        || !matches!(parts[2].len(), 40 | 64) || !parts[2].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err(invalid());
    }
    Ok(parts[2].into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_result_blob_control_then_invalid_entries() {
        for (mode, length) in [("100644", 40), ("100755", 64)] {
            let oid = "a".repeat(length);
            let good = format!("{mode} blob {oid}\tdocs/my report.md\0");
            assert_eq!(parse_blob_entry(good.as_bytes(), "docs/my report.md").unwrap(), oid);
            for bad in [good.replace(mode, "120000"), good.replace(" blob ", " tree "), good.replace("\tdocs/", "\tother/"), good.trim_end_matches('\0').into(), format!("{good}{good}"), good.replace('a', "A"), String::new()] {
                assert!(parse_blob_entry(bad.as_bytes(), "docs/my report.md").is_err(), "{bad:?}");
            }
        }
    }
}

/// Read an exact ref without interpreting a failing Git command as absence.
/// Prefix neighbours are ignored; malformed matching records refuse.
pub(super) fn reference(bare: &Path, name: &str, settings: &EngineSettingsDocument) -> Result<Option<String>, E> {
    let raw = text(git(bare, &["for-each-ref", "--format=%(refname) %(objectname)", name], settings)?)?;
    let mut found = None;
    for row in raw.lines() {
        let (key, value) = row.split_once(' ').ok_or(E::Refused("managed-result-ref-invalid"))?;
        if key == name {
            if found.is_some() { return Err(E::Refused("managed-result-ref-duplicate")); }
            commit(bare, value, settings)?;
            found = Some(value.into());
        }
    }
    Ok(found)
}

/// Read one regular Git blob with the persisted delivery cap, preserving exact
/// whitespace. Symlink/submodule/tree reports refuse. Missing is explicit; Git,
/// size, UTF-8 and whitespace-only failures never become an empty success.
fn blob(bare: &Path, revision: &str, path: &str, settings: &EngineSettingsDocument) -> Result<Option<Vec<u8>>, E> {
    let raw = git(bare, &["ls-tree", "-z", revision, "--", path], settings)?;
    if raw.is_empty() { return Ok(None); }
    let oid = parse_blob_entry(&raw, path)?;
    let size = text(git(bare, &["cat-file", "-s", &oid], settings)?)?.parse::<usize>()
        .map_err(|e| E::caused("managed-result-report-size", e))?;
    let limits = settings.values.context_policy_limits().map_err(|e| E::caused(e.reason(), e))?;
    if limits.review_delivery_bytes.is_some_and(|cap| size > cap) {
        return Err(E::Refused("managed-result-report-too-large"));
    }
    let bytes = git(bare, &["cat-file", "blob", &oid], settings)?;
    if bytes.len() != size || std::str::from_utf8(&bytes).map_err(|e| E::caused("managed-result-report-utf8", e))?.trim().is_empty() {
        return Err(E::Refused("managed-result-report-empty-or-changed"));
    }
    Ok(Some(bytes))
}

/// Validate the result's exact base parent and extract its changed report. These
/// observations prove immutable object content, not model authorship or quality.
fn report_bytes(bare: &Path, oid: &str, base: &str, path: &str, settings: &EngineSettingsDocument) -> Result<Vec<u8>, E> {
    commit(bare, oid, settings)?;
    if text(git(bare, &["rev-list", "--parents", "-n", "1", oid], settings)?)? != format!("{oid} {base}") {
        return Err(E::Refused("managed-result-parent-mismatch"));
    }
    let bytes = blob(bare, oid, path, settings)?.ok_or(E::Refused("managed-result-report-missing"))?;
    if blob(bare, base, path, settings)?.as_ref() == Some(&bytes) {
        return Err(E::Refused("managed-result-report-unchanged"));
    }
    Ok(bytes)
}

/// Freeze a completed first worker in its managed store. Caller has verified the
/// original request/SQL attribution and actual completion under a coherent Root
/// read, and must repeat authority at submission. Git runs without a Root writer
/// transaction; the managed lane serializes only retention. Existing result refs
/// are reused after interruption, never overwritten from the mutable copy.
/// Missing initial report waits. Other failures retain causes and may leave Git
/// objects, ref or published files; they do not imply rollback. No retry, SQL,
/// provider, candidate publication, task acceptance or deletion is performed.
pub(crate) fn retain_result(root: &Path, work: &Work, agent: &Agent, settings: &EngineSettingsDocument) -> Result<Result<RetainedResult, &'static str>, E> {
    with_storage(root, work, true, |dir| {
        repository(dir, work, settings)?;
        let p = proof(dir, &format!("native-worker-{}.json", agent.id))?;
        let repo = work.planning.as_deref().and_then(|v| v.repository.as_deref()).ok_or(E::Refused("managed-repository-required"))?;
        let path = if repo.project_subdir.is_empty() { format!("docs/{}.md", agent.task_id) } else { format!("{}/docs/{}.md", repo.project_subdir, agent.task_id) };
        let name = format!("refs/swarm/attempts/{}", agent.id);
        inside(dir.child("repository.git", false).map_err(|e| E::caused("managed-result-repository", e))?, |bare| {
            let oid = match reference(&bare.process_path(), &name, settings)? {
                Some(value) => value,
                None => {
                    workspace(&p, work, settings, false)?;
                    let selected = Path::new(&p.path).join(&path);
                    match std::fs::symlink_metadata(&selected) {
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Err("managed-result-report-missing")),
                        Err(e) => return Err(E::caused("managed-result-report-metadata", e)),
                        Ok(_) => {},
                    }
                    let limits = settings.values.context_policy_limits().map_err(|e| E::caused(e.reason(), e))?;
                    let observed = crate::read_review_delivery(Path::new(&p.path), &path, &limits).map_err(|e| E::caused("managed-result-report-read", e))?;
                    if observed.trim().is_empty() { return Err(E::Refused("managed-result-report-empty")); }
                    inside(LocalDirectory::open_owned(Path::new(&p.path)).map_err(|e| E::caused("managed-result-copy", e))?, |copy| {
                        inside(copy.child(".git", false).map_err(|e| E::caused("managed-result-git", e))?, |gitdir| {
                            if gitdir.identity().map_err(|e| E::caused("managed-result-git-identity", e))? != p.git_identity {
                                return Err(E::Refused("managed-result-git-replaced"));
                            }
                            // Recheck while the copy/Git descriptors remain owned. These
                            // checks are attribution, not confinement against the same UID.
                            workspace(&p, work, settings, false)?;
                            let tree = text(super::super::index::snapshot_tree(&copy.process_path(), settings)?)?;
                            // Inherited technical identity for generated objects only;
                            // it neither attributes a human nor commits source checkout.
                            let oid = text(git(&copy.process_path(), &["-c", "user.name=Swarm", "-c", "user.email=swarm@localhost", "commit-tree", &tree, "-p", &p.base, "-m", &format!("Résultat {}", agent.id)], settings)?)?;
                            let frozen = report_bytes(&copy.process_path(), &oid, &p.base, &path, settings)?;
                            if frozen != observed.as_bytes() { return Err(E::Refused("managed-result-report-changed")); }
                            let source = copy.process_path();
                            git(&bare.process_path(), &["fetch", "--no-tags", "--no-write-fetch-head", "--", source.to_str().ok_or(E::Refused("managed-result-copy-encoding"))?, &oid], settings)?;
                            git(&bare.process_path(), &["update-ref", &name, &oid, ""], settings)?;
                            Ok(oid)
                        })
                    })?
                },
            };
            let bytes = report_bytes(&bare.process_path(), &oid, &p.base, &path, settings)?;
            inside(dir.child("results", true).map_err(|e| E::caused("managed-result-parent", e))?, |results| {
                inside(results.child(&agent.id, true).map_err(|e| E::caused("managed-result-directory", e))?, |result| {
                    let value = RetainedResult {
                        schema_version: 1, work: work.id.clone(), task: agent.task_id.clone(), agent: agent.id.clone(), attempt: agent.attempt_id.clone(), base: p.base.clone(),
                        attribution_sha256: hash(&crate::encode_model(&p).map_err(|e| E::caused("managed-result-attribution", e))?), commit: oid,
                        report_path: format!(".swarm/managed/{}/results/{}/report.md", work.id, agent.id), report_sha256: hash(&bytes),
                    };
                    let manifest = crate::encode_model(&value).map_err(|e| E::caused("managed-result-encode", e))?;
                    for (name, expected) in [("report.md", bytes.as_slice()), ("result.json", manifest.as_slice())] {
                        let exists = result.names().map_err(|e| E::caused("managed-result-list", e))?.iter().any(|n| n == name.as_bytes());
                        if exists {
                            if result.read(name).map_err(|e| E::caused("managed-result-read", e))? != expected { return Err(E::Refused("managed-result-retained-conflict")); }
                        } else {
                            result.publish_new_with_fault(name, expected, crate::local_storage::AtomicStorageFault::None).map_err(|e| E::caused("managed-result-publication-incomplete", e))?;
                        }
                    }
                    Ok(Ok(value))
                })
            })
        })
    }).map_err(|e| E::caused("managed-result-retention-incomplete", e))
}

/// Verify an already retained result without opening the worker copy or mutating
/// any store. Caller separately verifies SQL/original attribution, rights and
/// terminal receipt. Exact canonical manifest, Git ref/parent/report bytes and
/// artifact hash must agree. Missing/corrupt/replaced data refuses; no repair or
/// acceptance is inferred. Owned descriptors and all close causes are retained.
pub(crate) fn verify_result(root: &Path, work: &Work, agent: &Agent, settings: &EngineSettingsDocument) -> Result<RetainedResult, E> {
    with_storage(root, work, false, |dir| {
        repository(dir, work, settings)?;
        let p = proof(dir, &format!("native-worker-{}.json", agent.id))?;
        inside(dir.child("results", false).map_err(|e| E::caused("managed-result-parent", e))?, |results| {
            inside(results.child(&agent.id, false).map_err(|e| E::caused("managed-result-directory", e))?, |result| {
                let raw = result.read("result.json").map_err(|e| E::caused("managed-result-read", e))?;
                let value: RetainedResult = serde_json::from_slice(&raw).map_err(|e| E::caused("managed-result-invalid", e))?;
                if crate::encode_model(&value).map_err(|e| E::caused("managed-result-encode", e))? != raw
                    || value.schema_version != 1 || value.work != work.id || value.task != agent.task_id || value.agent != agent.id || value.attempt != agent.attempt_id || value.base != p.base
                    || value.attribution_sha256 != hash(&crate::encode_model(&p).map_err(|e| E::caused("managed-result-attribution", e))?)
                    || value.report_path != format!(".swarm/managed/{}/results/{}/report.md", work.id, agent.id) {
                    return Err(E::Refused("managed-result-binding-mismatch"));
                }
                inside(dir.child("repository.git", false).map_err(|e| E::caused("managed-result-repository", e))?, |bare| {
                    let name = format!("refs/swarm/attempts/{}", agent.id);
                    if reference(&bare.process_path(), &name, settings)?.as_ref() != Some(&value.commit) { return Err(E::Refused("managed-result-ref-mismatch")); }
                    let repo = work.planning.as_deref().and_then(|v| v.repository.as_deref()).ok_or(E::Refused("managed-repository-required"))?;
                    let path = if repo.project_subdir.is_empty() { format!("docs/{}.md", agent.task_id) } else { format!("{}/docs/{}.md", repo.project_subdir, agent.task_id) };
                    let bytes = report_bytes(&bare.process_path(), &value.commit, &p.base, &path, settings)?;
                    if hash(&bytes) != value.report_sha256 || result.read("report.md").map_err(|e| E::caused("managed-result-report-read", e))? != bytes { return Err(E::Refused("managed-result-report-mismatch")); }
                    Ok(value)
                })
            })
        })
    })
}
