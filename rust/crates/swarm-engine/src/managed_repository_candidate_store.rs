//! Pre-control candidate custody; actual Root permission remains a caller duty.
use super::super::source::{tracked_sources, review_link, TrackedSource};
use super::super::controls::ControlPlan;
use super::super::{candidate::merge_candidate, git, hash, inside, text, LocalDirectory, E};
use super::{repository, result, with_storage};
#[path = "managed_repository_candidate_witness.rs"]
mod witness;
pub(crate) use witness::CandidateWitness;
use crate::{Agent, EngineSettingsDocument, Work};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Component, Path, PathBuf},
};

/// Private native-v2 original, not adoption of a legacy Go checkpoint.
/// Version2 freezes the complete effective cumulative control preimage; old
/// version1 originals remain readable only to return an explicit unbound refusal. Inode
/// observations after descriptors close do not provide OS confinement or prevent
/// later reuse. A native controller still owes owned witnesses and actual grants.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagedCandidate {
    schema_version: u32,
    pub work: String,
    pub task: String,
    pub agent: String,
    pub attempt: String,
    pub previous: String,
    pub result: String,
    pub contract: String,
    /// Exact original/normalized policies and ordered transport slots, not an
    /// execution permit. Optional solely to diagnose historical unbound v1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_plan: Option<ControlPlan>,
    /// Domain digest over the entire checkpoint excluding this digest field.
    /// Optional to diagnose historical unbound originals, never defaulted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    binding_sha256: Option<String>,
    pub commit: String,
    stage: String,
    pub workspace: String,
    copy_identity: (u64, u64),
    git_identity: (u64, u64),
    workspace_identity: (u64, u64),
    tracked: BTreeMap<String, TrackedSource>,
}
impl ManagedCandidate {
    /// Enumerate every retained repository-tree entry without IO or filtering.
    /// Names are relative to the held repository copy, including project prefixes;
    /// mode and digest describe raw regular bytes or a symlink's target bytes.
    /// This grants no path access, permission or freshness; actual readers must
    /// keep the witness and reject changed, unsupported or unreadable content.
    pub(crate) fn review_entries(&self) -> impl Iterator<Item = (&str, &str, &str)> {
        self.tracked.iter().map(|(name, source)| {
            let (mode, sha256) = source.review_identity();
            (name.as_str(), mode, sha256)
        })
    }
    /// Verify the recorded complete checkpoint preimage without filesystem IO.
    /// Requires private schema2, a retained control plan and exact domain digest.
    /// Refuses absent/changed bindings; codec causes remain typed. This proves no
    /// current Git/source identity, saved-limit normalization or native authority.
    pub(crate) fn verify_binding(&self) -> Result<(), E> {
        if self.schema_version != 2 || self.control_plan.is_none()
            || self.binding_sha256.as_deref() != Some(self.binding_digest()?.as_str()) {
            return Err(E::Refused("managed-candidate-preimage-binding-mismatch"));
        }
        Ok(())
    }
    /// Hash the full canonical private preimage under a versioned domain,
    /// excluding only its own digest field to avoid a circular encoding. Includes
    /// effective controls, tracked source hashes and physical observations. This
    /// allocates/encodes only, preserves codec causes and grants no permission,
    /// SQL backing or filesystem confinement. Caller must compare the recorded
    /// value and current source; hashing arbitrary DTO data does not validate it.
    pub(crate) fn binding_digest(&self) -> Result<String, E> {
        let mut preimage = self.clone();
        preimage.binding_sha256 = None;
        let raw = crate::encode_model(&("swarm-managed-candidate-binding-v2", preimage))
            .map_err(|e| E::caused("managed-candidate-binding-encode", e))?;
        Ok(hash(&raw))
    }
}
/// Validate the cumulative control cohort under actual saved settings before
/// binding business inputs. Returns the business key and complete control plan,
/// normalized independently per task. No candidate directory is created. Policy,
/// contract or encoding failures preserve typed causes, with no fallback.
/// Its canonical identity excludes the controller's own optimistic revision;
/// supplied DTOs and valid stored policies still establish no native permission.
fn coordinates(work: &Work, agent: &Agent, result: &str, settings: &EngineSettingsDocument) -> Result<(String, ControlPlan), E> {
    let repo = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?;
    let limits = settings.values.mutation_limits()
        .map_err(|e| E::caused("managed-controls-settings", e))?;
    let plan = super::super::controls::plan(work, &agent.task_id, &limits.policy)?;
    let contract = &plan.contract;
    let key = hash(
        &crate::encode_model(&[
            &work.id,
            &agent.task_id,
            &agent.id,
            &agent.attempt_id,
            &repo.candidate_commit,
            result,
            &contract,
        ])
        .map_err(|e| E::caused("managed-candidate-key", e))?,
    );
    Ok((key, plan))
}
/// Derive only normal contained workspace components and the canonical path.
/// A saved path cannot select another stage, copy or project directory.
fn workspace_path(work: &Work, key: &str, stage: &str) -> Result<PathBuf, E> {
    let repo = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?;
    if !swarm_identity_contract::name(stage.as_bytes())
        || Path::new(&repo.project_subdir)
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(E::Refused("managed-candidate-stage-or-subdir-invalid"));
    }
    let mut path = Path::new(&repo.storage)
        .join("candidates")
        .join(key)
        .join("stages")
        .join(stage)
        .join("copy");
    for component in Path::new(&repo.project_subdir).components() {
        path.push(component.as_os_str());
    }
    Ok(path)
}
/// Resolve and retain the actual workspace through owned private children.
/// Close intermediate descriptors, preserving primary and cleanup errors; the
/// returned handle belongs to the caller and must be explicitly closed. No path
/// fallback, creation, command or execution permission is provided. If a parent
/// release fails after its child opened successfully, explicitly close that
/// newly owned child too, retaining both causes rather than dropping it.
fn open_workspace(copy: &LocalDirectory, work: &Work) -> Result<LocalDirectory, E> {
    let subdir = &work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?
        .project_subdir;
    let mut current = copy
        .duplicate()
        .map_err(|e| E::caused("managed-candidate-workspace-open", e))?;
    for component in Path::new(subdir).components() {
        let next = match component {
            Component::Normal(name) => name
                .to_str()
                .ok_or(E::Refused("managed-candidate-subdir-encoding"))
                .and_then(|name| {
                    current
                        .child(name, false)
                        .map_err(|e| E::caused("managed-candidate-workspace-open", e))
                }),
            _ => Err(E::Refused("managed-candidate-subdir-invalid")),
        };
        let released = current.close()
            .map_err(|e| E::caused("managed-candidate-workspace-close", e));
        current = match (next, released) {
            (Ok(next), Ok(())) => next,
            (Err(primary), release) =>
                return crate::agent_launch_error::finish(Err(primary), release),
            (Ok(next), Err(primary)) => return crate::agent_launch_error::finish(
                Err(primary), next.close()
                    .map_err(|e| E::caused("managed-candidate-workspace-close", e))),
        };
    }
    Ok(current)
}
/// Observe an owned workspace and close its actual descriptor with both errors
/// preserved. A tuple alone cannot prevent inode reuse after this returns.
fn workspace_identity(copy: &LocalDirectory, work: &Work) -> Result<(u64, u64), E> {
    inside(open_workspace(copy, work)?, |directory| {
        directory.owned_identity()
            .map_err(|e| E::caused("managed-candidate-workspace-identity", e))
    })
}
/// Privatize only this freshly created candidate's normal directory components,
/// before any control admission. Existing checkpoints and user source paths are
/// never chmod'ed. Symlink/non-directory components refuse before chmod; caller
/// owns the staging root. This is privacy setup, not same-UID OS confinement.
fn privatize_workspace(copy: &LocalDirectory, work: &Work) -> Result<(), E> {
    let subdir = &work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?
        .project_subdir;
    let mut path = copy.process_path();
    for component in Path::new(subdir).components() {
        match component {
            Component::Normal(name) => path.push(name),
            _ => return Err(E::Refused("managed-candidate-subdir-invalid")),
        }
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|e| E::caused("managed-candidate-workspace-metadata", e))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(E::Refused("managed-candidate-workspace-not-directory"));
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| E::caused("managed-candidate-workspace-permissions", e))?;
    }
    Ok(())
}
/// Check HEAD and single-parent coordinates in an owned, quiescent copy before
/// creating or consuming its checkpoint. Git object reads use saved settings;
/// changed coordinates and command/decoding failures preserve their causes.
/// No worktree diff is run: even --no-textconv can invoke a clean filter. The
/// caller must separately capture/compare tracked_sources against the original
/// raw-byte baseline. This reads metadata only, grants nothing and does not
/// confine another same-UID writer or make the sequential observations atomic.
fn source_state(
    copy: &LocalDirectory,
    commit: &str,
    previous: &str,
    settings: &EngineSettingsDocument,
) -> Result<(), E> {
    if text(git(&copy.process_path(), &["rev-parse", "HEAD"], settings)?)? != commit
        || text(git(
            &copy.process_path(),
            &["rev-list", "--parents", "-n", "1", commit],
            settings,
        )?)? != format!("{commit} {previous}")
    {
        return Err(E::Refused("managed-candidate-head-or-parent-changed"));
    }
    Ok(())
}
/// Read a final original under the caller's managed lane, rechecking all inputs,
/// source identities, exact ref/HEAD/parent and tracked bytes. The stored full
/// controls plan must equal recomputation under actual saved limits. Effective
/// default changes refuse without writing a replacement; v1 unbound originals
/// are preserved and refused, never upgraded by a read. Untracked outputs
/// are retained, never deleted or presented as Git sources. Returns owned
/// duplicates of the same checked copy/git/workspace handles via the caller's
/// separate owner slot. Generic ancestor cleanup returns only DTO data, so a
/// failure cannot implicitly drop the new witness and lose its close errors.
/// Caller must finalize that owner slot even after failure; no path reopen or
/// command follows. Caller releases the managed lane before using the witness.
fn inspect(
    directory: &LocalDirectory,
    bare: &LocalDirectory,
    work: &Work,
    agent: &Agent,
    result: &str,
    key: &str,
    plan: &ControlPlan,
    settings: &EngineSettingsDocument,
    pinned: &mut Option<CandidateWitness>,
) -> Result<ManagedCandidate, E> {
    if pinned.is_some() {
        return Err(E::Refused("managed-candidate-witness-already-retained"));
    }
    let raw = directory
        .read("candidate.json")
        .map_err(|e| E::caused("managed-candidate-read", e))?;
    let saved: ManagedCandidate =
        serde_json::from_slice(&raw).map_err(|e| E::caused("managed-candidate-invalid", e))?;
    if saved.schema_version == 1 && saved.control_plan.is_none() {
        return Err(E::Refused("managed-candidate-controls-unbound"));
    }
    let previous = &work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?
        .candidate_commit;
    if crate::encode_model(&saved).map_err(|e| E::caused("managed-candidate-encode", e))? != raw
        || saved.schema_version != 2
        || saved.work != work.id
        || saved.task != agent.task_id
        || saved.agent != agent.id
        || saved.attempt != agent.attempt_id
        || saved.previous != *previous
        || saved.result != result
        || saved.contract != plan.contract
        || Path::new(&saved.workspace) != workspace_path(work, key, &saved.stage)?
    {
        return Err(E::Refused("managed-candidate-binding-mismatch"));
    }
    if saved.control_plan.as_ref() != Some(plan) {
        return Err(E::Refused("managed-candidate-controls-changed"));
    }
    saved.verify_binding()?;
    let reference = format!("refs/swarm/native-candidates/{key}");
    if result::reference(&bare.process_path(), &reference, settings)?.as_deref()
        != Some(saved.commit.as_str())
    {
        return Err(E::Refused("managed-candidate-ref-changed"));
    }
    inside(
        directory
            .child("stages", false)
            .map_err(|e| E::caused("managed-candidate-stages", e))?,
        |stages| {
            inside(
                stages
                    .child(&saved.stage, false)
                    .map_err(|e| E::caused("managed-candidate-stage", e))?,
                |stage| {
                    inside(
                        stage
                            .child("copy", false)
                            .map_err(|e| E::caused("managed-candidate-copy", e))?,
                        |copy| {
                            inside(
                                copy.child(".git", false)
                                    .map_err(|e| E::caused("managed-candidate-git", e))?,
                                |gitdir| inside(open_workspace(copy, work)?, |workspace| {
                                    if copy.owned_identity().map_err(|e| {
                                        E::caused("managed-candidate-copy-identity", e)
                                    })? != saved.copy_identity
                                        || gitdir.owned_identity().map_err(|e| {
                                            E::caused("managed-candidate-git-identity", e)
                                        })? != saved.git_identity
                                        || workspace.owned_identity().map_err(|e| {
                                            E::caused("managed-candidate-workspace-identity", e)
                                        })? != saved.workspace_identity
                                    {
                                        return Err(E::Refused(
                                            "managed-candidate-directory-replaced",
                                        ));
                                    }
                                    source_state(copy, &saved.commit, &saved.previous, settings)?;
                                    if tracked_sources(copy, &saved.commit, settings)?
                                        != saved.tracked
                                    {
                                        return Err(E::Refused(
                                            "managed-candidate-raw-source-changed",
                                        ));
                                    }
                                    *pinned = Some(CandidateWitness::retain(&saved, copy, gitdir, workspace)?);
                                    Ok(())
                                }),
                            )
                        },
                    )
                },
            )
        },
    )?;
    Ok(saved)
}

/// Prepare a cumulative candidate before any control admission. The caller owns
/// actual SQL/original-terminal attribution, Root/consent/rights and settings;
/// these supplied DTOs and files confer no permission. Git holds only the managed
/// lane outside Root transactions and releases it before controls or inference.
/// A create-only private ref fixes the commit before the final checkpoint. On a
/// late failure that ref is reused; failed scratch stages remain preserved. No
/// controls may be admitted without a final inspected checkpoint. Existing final
/// originals are never reset/rebuilt. Version2 freezes the complete normalized
/// cohort plan alongside original policies; changed effective plans refuse at
/// the same business key, without replacing the original or creating a stage. Errors preserve causes and may leave stage,
/// objects/ref/files, never imply rollback, and trigger caller recovery. No SQL,
/// control/model, deletion, candidate publication or task acceptance occurs.
pub(crate) fn prepare_candidate(
    root: &Path,
    work: &Work,
    agent: &Agent,
    settings: &EngineSettingsDocument,
) -> Result<ManagedCandidate, E> {
    let frozen = result::verify_result(root, work, agent, settings)?;
    let (key, plan) = coordinates(work, agent, &frozen.commit, settings)?;
    let mut pinned = None;
    let result = with_storage(root,work,true,|storage|{
        repository(storage,work,settings)?;
        inside(storage.child("repository.git",false).map_err(|e|E::caused("managed-candidate-repository",e))?,|bare|inside(storage.child("candidates",true).map_err(|e|E::caused("managed-candidate-parent",e))?,|parent|inside(parent.child(&key,true).map_err(|e|E::caused("managed-candidate-directory",e))?,|directory|{
            if directory.names().map_err(|e|E::caused("managed-candidate-list",e))?.iter().any(|name|name==b"candidate.json"){return inspect(directory,bare,work,agent,&frozen.commit,&key,&plan,settings,&mut pinned);}
            let reference=format!("refs/swarm/native-candidates/{key}");
            let prior=result::reference(&bare.process_path(),&reference,settings)?;
            inside(directory.child("stages",true).map_err(|e|E::caused("managed-candidate-stages",e))?,|stages|{
                let stage_id=crate::generate_identifier("candidate-").map_err(|e|E::caused("managed-candidate-stage-id",e))?;
                if stages.names().map_err(|e|E::caused("managed-candidate-list",e))?.iter().any(|name|name==stage_id.as_bytes()){return Err(E::Refused("managed-candidate-stage-collision"));}
                inside(stages.child(&stage_id,true).map_err(|e|E::caused("managed-candidate-stage",e))?,|stage|{
                    let source=bare.process_path();let destination=stage.process_path().join("copy");
                    git(&stage.process_path(),&["clone","--no-checkout","--no-local","--no-hardlinks","--",source.to_str().ok_or(E::Refused("managed-candidate-path-encoding"))?,destination.to_str().ok_or(E::Refused("managed-candidate-path-encoding"))?],settings)?;
                    for path in [&destination,&destination.join(".git")]{std::fs::set_permissions(path,std::fs::Permissions::from_mode(0o700)).map_err(|e|E::caused("managed-candidate-permissions",e))?;}
                    inside(stage.child("copy",false).map_err(|e|E::caused("managed-candidate-copy",e))?,|copy|inside(copy.child(".git",false).map_err(|e|E::caused("managed-candidate-git",e))?,|gitdir|{
                        let previous=&work.planning.as_deref().and_then(|p|p.repository.as_deref()).ok_or(E::Refused("managed-repository-required"))?.candidate_commit;
                        for oid in [Some(previous.as_str()),Some(frozen.commit.as_str()),prior.as_deref()].into_iter().flatten(){git(&copy.process_path(),&["fetch","--no-tags","--no-write-fetch-head","--",source.to_str().ok_or(E::Refused("managed-candidate-path-encoding"))?,oid],settings)?;}
                        let commit=match prior{
                            Some(oid)=>{git(&copy.process_path(),&["checkout","--detach",&oid],settings)?;oid},
                            None=>{
                                let oid=merge_candidate(copy,previous,&frozen.commit,&agent.id,settings)?;
                                let source=copy.process_path();git(&bare.process_path(),&["fetch","--no-tags","--no-write-fetch-head","--",source.to_str().ok_or(E::Refused("managed-candidate-path-encoding"))?,&oid],settings)?;
                                git(&bare.process_path(),&["update-ref",&reference,&oid,""],settings)?;oid
                            },
                        };
                        source_state(copy,&commit,previous,settings)?;
                        privatize_workspace(copy,work)?;
                        let workspace=workspace_path(work,&key,&stage_id)?;
                        let tracked=tracked_sources(copy,&commit,settings)?;
                        let mut value=ManagedCandidate{binding_sha256:None,tracked,schema_version:2,work:work.id.clone(),task:agent.task_id.clone(),agent:agent.id.clone(),attempt:agent.attempt_id.clone(),previous:previous.clone(),result:frozen.commit.clone(),contract:plan.contract.clone(),control_plan:Some(plan.clone()),commit,stage:stage_id.clone(),workspace:workspace.to_str().ok_or(E::Refused("managed-candidate-path-encoding"))?.into(),copy_identity:copy.owned_identity().map_err(|e|E::caused("managed-candidate-copy-identity",e))?,git_identity:gitdir.owned_identity().map_err(|e|E::caused("managed-candidate-git-identity",e))?,workspace_identity:workspace_identity(copy,work)?};
                        value.binding_sha256=Some(value.binding_digest()?);
                        let raw=crate::encode_model(&value).map_err(|e|E::caused("managed-candidate-encode",e))?;
                        directory.publish_new_with_fault("candidate.json",&raw,crate::local_storage::AtomicStorageFault::None).map_err(|e|E::caused("managed-candidate-publication-incomplete",e))?;
                        inspect(directory,bare,work,agent,&frozen.commit,&key,&plan,settings,&mut pinned)
                    }))
                })
            })
        })))
    });
    let result = match pinned {
        Some(witness) => crate::agent_launch_error::finish(result, witness.close().map(|_| ())),
        None => result,
    };
    result.map_err(|e|E::caused("managed-candidate-retention-incomplete",e))
}

/// Read and pin an existing current checkpoint/ref/workspace only. Returned
/// owned copy/git/workspace descriptors survive managed-lane release and must
/// remain held through execution and explicit close. This performs Git outside
/// any Root transaction; callers must not hold Root SQL/settings locks here.
/// Actual native producer,
/// Root/lease and owned execution witnesses remain the controller's duty. No IO
/// writes, fallback, reinitialization, grant or acceptance is performed; typed
/// source/identity/Git/close errors are preserved. Paths are observations only.
pub(crate) fn pin_candidate(
    root: &Path,
    work: &Work,
    agent: &Agent,
    settings: &EngineSettingsDocument,
) -> Result<CandidateWitness, E> {
    let frozen = result::verify_result(root, work, agent, settings)?;
    let (key, plan) = coordinates(work, agent, &frozen.commit, settings)?;
    let mut pinned = None;
    let result = with_storage(root, work, false, |storage| {
        repository(storage, work, settings)?;
        inside(
            storage
                .child("repository.git", false)
                .map_err(|e| E::caused("managed-candidate-repository", e))?,
            |bare| {
                inside(
                    storage
                        .child("candidates", false)
                        .map_err(|e| E::caused("managed-candidate-parent", e))?,
                    |parent| {
                        inside(
                            parent
                                .child(&key, false)
                                .map_err(|e| E::caused("managed-candidate-directory", e))?,
                            |directory| {
                                inspect(
                                    directory,
                                    bare,
                                    work,
                                    agent,
                                    &frozen.commit,
                                    &key,
                                    &plan,
                                    settings,
                                    &mut pinned,
                                )
                            },
                        )
                    },
                )
            },
        )
    });
    finish_pin(result, pinned)
}

/// Release all newly retained descriptors if any enclosing lock/directory
/// cleanup failed. Ownership lives outside generic cleanup wrappers so they
/// cannot drop a successful witness while discarding its close failures. An
/// operation failure remains primary; witness close failures remain secondary.
/// A successful operation without a retained witness refuses as an internal
/// contract failure, never a data-only substitute. No retry or write occurs.
fn finish_pin(
    result: Result<ManagedCandidate, E>,
    pinned: Option<CandidateWitness>,
) -> Result<CandidateWitness, E> {
    match (result, pinned) {
        (Ok(_), Some(witness)) => Ok(witness),
        (Err(primary), Some(witness)) =>
            crate::agent_launch_error::finish(Err(primary), witness.close().map(|_| ())),
        (Err(primary), None) => Err(primary),
        (Ok(_), None) => Err(E::Refused("managed-candidate-witness-not-retained")),
    }
}
/// Verify current candidate data and explicitly close every execution witness.
/// Returns only observations; callers needing lasting inode ownership must use
/// pin_candidate instead. No command, write, grant or acceptance is performed;
/// operation and descriptor release failures preserve their typed causes.
pub(crate) fn verify_candidate(
    root: &Path,
    work: &Work,
    agent: &Agent,
    settings: &EngineSettingsDocument,
) -> Result<ManagedCandidate, E> {
    pin_candidate(root, work, agent, settings)?.close()
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Exercise actual Git/bootstrap/copy/result producers with supplied DTO
    /// fixtures only. No SQL Agent, kernel receipt, grant or acceptance is made;
    /// real native custody/permission checks remain public-journey obligations.
    fn custody_fixture(subdir: &str) -> (PathBuf, Work, Agent, EngineSettingsDocument) {
        let root =
            std::env::temp_dir().join(crate::generate_identifier("candidate-custody-").unwrap());
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let state = root.join(".swarm");
        std::fs::create_dir(&state).unwrap();
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
        let source = root.join("source");
        std::fs::create_dir(&source).unwrap();
        let project = source.join(subdir);
        std::fs::create_dir_all(&project).unwrap();
        println!("CUSTODY_FIXTURE {}", root.display());
        let settings = EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "owned-fixture".into(),
            "2026-10-25T00:00:00Z".into(),
        )
        .unwrap();
        git(&source, &["init", "--quiet"], &settings).unwrap();
        std::fs::write(project.join("tracked.txt"), b"committed source\n").unwrap();
        git(&source, &["add", "-A", "--", "."], &settings).unwrap();
        git(
            &source,
            &[
                "-c",
                "user.name=Swarm",
                "-c",
                "user.email=swarm@localhost",
                "commit",
                "--quiet",
                "-m",
                "base",
            ],
            &settings,
        )
        .unwrap();
        let request = crate::ManagedRepositoryRequest {
            path: project
                .strip_prefix(&root)
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            committed_only: true,
            ..Default::default()
        };
        let repository = super::super::super::prepare(&root, "work", &request, &settings).unwrap();
        let mut work = Work {
            id: "work".into(),
            revision: 1,
            tasks: Some(vec![crate::Task {
                id: "task".into(),
                status: "todo".into(),
                criteria: Some(vec!["criterion".into()]),
                validation_policy: Some(Box::new(crate::ValidationPolicy {
                    mode: "automatic".into(), authorized_at: "fixture-authorized".into(), actor: "fixture".into(),
                    controls: Some(vec![crate::ValidationControl {
                        id: "check".into(), command: Some(vec!["python3".into(), "-c".into(), "print('fixture')".into()]),
                        criteria: Some(vec![1]), justification: "Check the fixture criterion with an explicit stored command".into(),
                        timeout_seconds: 1, ..Default::default()
                    }]),
                })),
                ..Default::default()
            }]),
            planning: Some(Box::new(crate::PlanningState {
                repository: Some(Box::new(repository)),
                ..Default::default()
            })),
            ..Default::default()
        };
        let launch = crate::Launch {
            schema_version: 1,
            event_id: "agent".into(),
            expected_revision: 1,
            task_id: "task".into(),
            provider: "fixture".into(),
            ..Default::default()
        };
        let raw = crate::encode_model(&launch).unwrap();
        let workspace =
            super::super::prepare_worker(&root, &work, &launch, &raw, &settings, None).unwrap();
        let agent = Agent {
            id: "agent".into(),
            work_id: "work".into(),
            task_id: "task".into(),
            attempt_id: "attempt".into(),
            workspace: workspace.clone(),
            role: "worker".into(),
            status: "completed".into(),
            ..Default::default()
        };
        work.tasks.as_mut().unwrap()[0].status = "submitted".into();
        work.tasks.as_mut().unwrap()[0].attempts = Some(vec![crate::Attempt {
            id: "attempt".into(),
            status: "completed".into(),
            ..Default::default()
        }]);
        let docs = Path::new(&workspace).join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(
            docs.join("task.md"),
            b"Observed complete report from the owned data fixture.\n",
        )
        .unwrap();
        assert!(result::retain_result(&root, &work, &agent, &settings)
            .unwrap()
            .is_ok());
        (root, work, agent, settings)
    }

    /// A valid original must not silently acquire another effective command
    /// plan after a saved default changes. Restoring that default must recover
    /// the same checkpoint, without execution or stage recreation.
    #[test]
    fn managed_candidate_freezes_effective_controls_and_recovers_original() {
        let (root, mut work, agent, mut settings) = custody_fixture("");
        work.tasks.as_mut().unwrap()[0].validation_policy.as_mut().unwrap()
            .controls.as_mut().unwrap()[0].timeout_seconds = 0;
        settings.values.validation_default_timeout_seconds = crate::SettingCount(1);
        let first = prepare_candidate(&root, &work, &agent, &settings).unwrap();
        assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == first);
        settings.values.validation_default_timeout_seconds = crate::SettingCount(2);
        assert!(verify_candidate(&root, &work, &agent, &settings).is_err(),
            "an existing candidate silently accepted different effective controls");
        assert!(prepare_candidate(&root, &work, &agent, &settings).is_err());
        settings.values.validation_default_timeout_seconds = crate::SettingCount(1);
        assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == first);
        assert!(prepare_candidate(&root, &work, &agent, &settings).unwrap() == first);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Check the real producer/consumer with cross-task homonyms. Every fault
    /// follows a valid witness; private fixture restoration is not an operator
    /// repair path and establishes neither SQL authority nor acceptance.
    #[test]
    fn managed_candidate_control_original_refuses_corruption_and_unbound_history() {
        let (root, mut work, agent, settings) = custody_fixture("apps/service");
        let mut prior = work.tasks.as_ref().unwrap()[0].clone();
        prior.id = "prior".into();
        prior.status = "accepted".into();
        work.tasks.as_mut().unwrap().insert(0, prior);
        let first = prepare_candidate(&root, &work, &agent, &settings).unwrap();
        let plan = first.control_plan.as_ref().unwrap();
        assert_eq!(plan.slots.len(), 2);
        assert_eq!(plan.slots[0].task_id, "prior");
        assert_eq!(plan.slots[1].task_id, "task");
        assert_eq!(plan.slots[0].control.id, plan.slots[1].control.id);
        assert_ne!(plan.slots[0].transport_id, plan.slots[1].transport_id);
        let (key, _) = coordinates(&work, &agent, &first.result, &settings).unwrap();
        let storage = Path::new(&work.planning.as_ref().unwrap().repository.as_ref().unwrap().storage);
        let directory = storage.join("candidates").join(key);
        let checkpoint = directory.join("candidate.json");
        let before = std::fs::read(&checkpoint).unwrap();
        let stages = std::fs::read_dir(directory.join("stages")).unwrap().count();
        for fault in 0..9 {
            assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == first);
            let mut corrupt = first.clone();
            match fault {
                0 => corrupt.control_plan.as_mut().unwrap().policies[0].normalized
                    .controls.as_mut().unwrap()[0].timeout_seconds += 1,
                1 => corrupt.control_plan.as_mut().unwrap().slots[0].control
                    .command = Some(vec!["different-command".into()]),
                2 => corrupt.control_plan.as_mut().unwrap().slots[0].transport_id = "different".into(),
                3 => corrupt.control_plan.as_mut().unwrap().slots.swap(0, 1),
                4 => corrupt.control_plan = None,
                5 => { corrupt.schema_version = 1; corrupt.control_plan = None; },
                6 => corrupt.control_plan.as_mut().unwrap().policies[0].original.actor = "different".into(),
                7 => corrupt.binding_sha256 = Some("0".repeat(64)),
                _ => corrupt.binding_sha256 = None,
            }
            let raw = crate::encode_model(&corrupt).unwrap();
            std::fs::write(&checkpoint, &raw).unwrap();
            let expected = if fault == 5 {"managed-candidate-controls-unbound"}
                else if fault >= 7 {"managed-candidate-preimage-binding-mismatch"}
                else {"managed-candidate-controls-changed"};
            assert_eq!(verify_candidate(&root, &work, &agent, &settings).err().unwrap().code(), expected);
            let error = prepare_candidate(&root, &work, &agent, &settings).err().unwrap();
            assert_eq!(error.code(), "managed-candidate-retention-incomplete");
            let cause = std::error::Error::source(&error).unwrap().downcast_ref::<E>().unwrap();
            assert_eq!(cause.code(), expected);
            assert_eq!(std::fs::read(&checkpoint).unwrap(), raw);
            assert_eq!(std::fs::read_dir(directory.join("stages")).unwrap().count(), stages);
            std::fs::write(&checkpoint, &before).unwrap();
            assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == first);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    /// The native binding preimage must cover physical observations and source
    /// hashes as well as controls. Pure hash changes establish integrity only;
    /// real source/witness checks and native permission remain separate duties.
    #[test]
    fn managed_candidate_binding_covers_full_physical_and_control_preimage() {
        let (root, work, agent, settings) = custody_fixture("");
        let first = prepare_candidate(&root, &work, &agent, &settings).unwrap();
        let original = first.binding_digest().unwrap();
        assert_eq!(first.binding_sha256.as_deref(), Some(original.as_str()));
        assert_ne!(original, hash(&crate::encode_model(&first).unwrap()));
        for fault in 0..5 {
            let mut changed = first.clone();
            match fault {
                0 => changed.copy_identity.1 += 1,
                1 => changed.git_identity.1 += 1,
                2 => changed.workspace_identity.1 += 1,
                3 => changed.tracked.clear(),
                _ => changed.control_plan.as_mut().unwrap().slots[0].control.timeout_seconds += 1,
            }
            assert_ne!(changed.binding_digest().unwrap(), original);
        }
        // The digest field is the only excluded field, so comparing it to the
        // computed digest remains a separate required consumer assertion.
        let mut changed = first.clone();
        changed.binding_sha256 = Some("different".into());
        assert_eq!(changed.binding_digest().unwrap(), original);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Real child execution uses a held descriptor; the managed lane can be
    /// reacquired while that witness lives. These fixture commands establish
    /// cwd/descriptor behavior, not native permission, provider or acceptance.
    #[test]
    fn managed_candidate_witness_keeps_workspace_alive_without_holding_git_lane() {
        for subdir in ["", "apps/service"] {
            let (root, work, agent, settings) = custody_fixture(subdir);
            let first = prepare_candidate(&root, &work, &agent, &settings).unwrap();
            let witness = pin_candidate(&root, &work, &agent, &settings).unwrap();
            assert!(witness.record() == &first);
            witness.verify_retained_identities().unwrap();
            with_storage(&root, &work, true, |_| Ok(())).unwrap();
            let output = std::process::Command::new("python3")
                .args(["-c", "from pathlib import Path; assert Path('tracked.txt').read_bytes()==b'committed source\\n'; print('witness-cwd-ok')"])
                .current_dir(witness.workspace_path()).output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            assert_eq!(output.stdout, b"witness-cwd-ok\n");
            assert!(witness.close().unwrap() == first);
            assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == first);
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    /// A valid command precedes replacement. Held descriptors still identify the
    /// original inode, while fresh verification refuses the substituted path and
    /// runs no command. Restoring the owned fixture recovers the same original.
    #[test]
    fn managed_candidate_witness_rejects_replaced_path_without_reexecution() {
        use std::os::unix::fs::MetadataExt;
        let (root, work, agent, settings) = custody_fixture("");
        let first = prepare_candidate(&root, &work, &agent, &settings).unwrap();
        let witness = pin_candidate(&root, &work, &agent, &settings).unwrap();
        let marker = root.join("fixture-command-count");
        let output = std::process::Command::new("python3")
            .args(["-c", "import sys; from pathlib import Path; Path(sys.argv[1]).write_text('one execution')"])
            .arg(&marker).current_dir(witness.workspace_path()).output().unwrap();
        assert!(output.status.success());
        assert_eq!(std::fs::read(&marker).unwrap(), b"one execution");
        let copy = Path::new(&first.workspace);
        let parked = copy.with_file_name("copy.parked-by-fixture");
        std::fs::rename(copy, &parked).unwrap();
        std::fs::create_dir(copy).unwrap();
        std::fs::set_permissions(copy, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::create_dir(copy.join(".git")).unwrap();
        std::fs::set_permissions(copy.join(".git"), std::fs::Permissions::from_mode(0o700)).unwrap();
        witness.verify_retained_identities().unwrap();
        let metadata = std::fs::metadata(witness.workspace_path()).unwrap();
        assert_eq!((metadata.dev(), metadata.ino()), first.workspace_identity);
        assert_eq!(pin_candidate(&root, &work, &agent, &settings).err().unwrap().code(),
            "managed-candidate-directory-replaced");
        assert_eq!(std::fs::read(&marker).unwrap(), b"one execution");
        std::fs::remove_dir_all(copy).unwrap();
        std::fs::rename(&parked, copy).unwrap();
        assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == first);
        assert!(witness.close().unwrap() == first);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Isolated child lowers only its own descriptor limit from observed fixture
    /// usage. Real EMFILE after two successful duplicates must close both owned
    /// duplicates and preserve borrowed handles; no product limit is changed.
    #[test]
    fn managed_candidate_witness_partial_acquisition_closes_owned_duplicates() {
        const CHILD: &str = "SWARM_CANDIDATE_WITNESS_FD_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let name = format!("{}::managed_candidate_witness_partial_acquisition_closes_owned_duplicates",
                module_path!().split_once("::").unwrap().1);
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", &name, "--nocapture", "--test-threads=1"])
                .env(CHILD, "owned-fixture").output().unwrap();
            assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(text.contains("1 passed; 0 failed;"));
            assert!(text.contains("WITNESS_PARTIAL_DUPLICATE_CLEANUP_PASS"));
            print!("{text}");
            return;
        }
        let (root, work, agent, settings) = custody_fixture("");
        let record = prepare_candidate(&root, &work, &agent, &settings).unwrap();
        let copy = LocalDirectory::open_owned(Path::new(&record.workspace)).unwrap();
        let gitdir = copy.child(".git", false).unwrap();
        let workspace = copy.duplicate().unwrap();
        let valid = CandidateWitness::retain(&record, &copy, &gitdir, &workspace).unwrap();
        assert!(valid.close().unwrap() == record);
        let fd_count = || std::fs::read_dir("/proc/self/fd").unwrap().count();
        let borrowed_count = fd_count();
        let mut mismatched = record.clone();
        mismatched.git_identity.1 += 1;
        assert_eq!(CandidateWitness::retain(&mismatched, &copy, &gitdir, &workspace)
            .err().unwrap().code(), "managed-candidate-witness-directory-replaced");
        assert_eq!(fd_count(), borrowed_count);
        let old = rustix::process::getrlimit(rustix::process::Resource::Nofile);
        let maximum_fd = std::fs::read_dir("/proc/self/fd").unwrap()
            .map(|entry| entry.unwrap().file_name().to_str().unwrap().parse::<u64>().unwrap()).max().unwrap();
        // Three additional fixture slots match the three descriptors required
        // by retain; fill to actual EMFILE, then release exactly two slots.
        rustix::process::setrlimit(rustix::process::Resource::Nofile,
            rustix::process::Rlimit { current: Some(maximum_fd + 3), maximum: old.maximum }).unwrap();
        let mut filled = Vec::new();
        loop {
            match std::fs::File::open("/dev/null") {
                Ok(file) => filled.push(file),
                Err(error) => { assert_eq!(error.raw_os_error(), Some(rustix::io::Errno::MFILE.raw_os_error())); break; }
            }
        }
        drop(filled.pop().unwrap()); drop(filled.pop().unwrap());
        let failure = CandidateWitness::retain(&record, &copy, &gitdir, &workspace).err().unwrap();
        // The acquisition failed after two owned duplicates, not on the borrowed
        // objects: both slots must now be available again and the third refused.
        let first = std::fs::File::open("/dev/null").unwrap();
        let second = std::fs::File::open("/dev/null").unwrap();
        assert_eq!(std::fs::File::open("/dev/null").err().unwrap().raw_os_error(),
            Some(rustix::io::Errno::MFILE.raw_os_error()));
        drop(first); drop(second);
        rustix::process::setrlimit(rustix::process::Resource::Nofile, old).unwrap();
        assert_eq!(failure.code(), "managed-candidate-witness-workspace-duplicate");
        let storage = std::error::Error::source(&failure).unwrap()
            .downcast_ref::<crate::local_storage::LocalStorageError>().unwrap();
        let cause = std::error::Error::source(storage).unwrap()
            .downcast_ref::<rustix::io::Errno>().unwrap();
        assert_eq!(*cause, rustix::io::Errno::MFILE);
        assert_eq!(copy.owned_identity().unwrap(), record.copy_identity);
        assert_eq!(gitdir.owned_identity().unwrap(), record.git_identity);
        assert_eq!(workspace.owned_identity().unwrap(), record.workspace_identity);
        let count_before_parent_failure = fd_count();
        let witness = CandidateWitness::retain(&record, &copy, &gitdir, &workspace).unwrap();
        assert_eq!(fd_count(), count_before_parent_failure + 3);
        let error = finish_pin(Err(E::Refused("fixture-parent-close-failed")), Some(witness)).err().unwrap();
        assert_eq!(error.code(), "fixture-parent-close-failed");
        assert_eq!(fd_count(), count_before_parent_failure);
        assert_eq!(finish_pin(Ok(record.clone()), None).err().unwrap().code(), "managed-candidate-witness-not-retained");
        workspace.close().unwrap();gitdir.close().unwrap();copy.close().unwrap();
        drop(filled);
        assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == record);
        std::fs::remove_dir_all(root).unwrap();
        println!("WITNESS_PARTIAL_DUPLICATE_CLEANUP_PASS");
    }

    /// An isolated helper pins its OWN descriptors, then execs an inert Python
    /// fixture using its own procfs cwd. The parent retains fixture ownership
    /// and checks the actual replacement process output before cleanup. No
    /// native Root permission, provider or acceptance is claimed.
    #[test]
    fn managed_candidate_witness_helper_keeps_self_descriptors_through_exec() {
        use std::os::unix::process::CommandExt;
        const CHILD: &str = "SWARM_CANDIDATE_WITNESS_EXEC_CHILD";
        if let Some(config) = std::env::var_os(CHILD) {
            let (root, work, agent, settings): (PathBuf, Work, Agent, EngineSettingsDocument) =
                serde_json::from_slice(&std::fs::read(config).unwrap()).unwrap();
            let witness = pin_candidate(&root, &work, &agent, &settings).unwrap();
            witness.verify_retained_identities().unwrap();
            let mut command = std::process::Command::new("python3");
            command.args(["-c", "from pathlib import Path; assert Path('tracked.txt').read_bytes()==b'committed source\\n'; print('HELPER_SELF_EXEC_OK '+str(Path.cwd()))"])
                .current_dir(witness.workspace_path());
            let error = command.exec();
            let failure: Result<(), E> = crate::agent_launch_error::finish(
                Err(E::caused("fixture-helper-exec-failed", error)), witness.close().map(|_| ()));
            panic!("{}", failure.err().unwrap());
        }
        let (root, work, agent, settings) = custody_fixture("apps/service");
        let record = prepare_candidate(&root, &work, &agent, &settings).unwrap();
        let config = root.join("owned-exec-fixture.json");
        std::fs::write(&config, serde_json::to_vec(&(&root, &work, &agent, &settings)).unwrap()).unwrap();
        std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600)).unwrap();
        let name = format!("{}::managed_candidate_witness_helper_keeps_self_descriptors_through_exec",
            module_path!().split_once("::").unwrap().1);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &name, "--nocapture", "--test-threads=1"])
            .env(CHILD, &config).output().unwrap();
        assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.ends_with(&format!("HELPER_SELF_EXEC_OK {}\n", record.workspace)));
        print!("{text}");
        assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == record);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A valid target policy is insufficient when an accepted cohort member has
    /// no authorized controls. Refuse before any candidate stage/ref is created.
    #[test]
    fn managed_candidate_refuses_uncontrolled_cohort_before_creating_candidate() {
        let (control_root, control_work, control_agent, control_settings) = custody_fixture("");
        assert!(prepare_candidate(&control_root, &control_work, &control_agent, &control_settings).is_ok());
        std::fs::remove_dir_all(control_root).unwrap();
        let (root, mut work, agent, settings) = custody_fixture("");
        let candidates = root.join(".swarm/managed/work/candidates");
        assert!(!candidates.exists());
        // The same target policy is valid in isolation before injecting the
        // accepted member's missing policy; no control process is executed.
        let task = &work.tasks.as_ref().unwrap()[0];
        let limits = settings.values.mutation_limits().unwrap();
        let normalized = crate::normalize_validation_policy(task.validation_policy.as_deref().unwrap().clone(), &limits.policy).unwrap();
        crate::validation_policy_covers_task(&normalized, task).unwrap();
        work.tasks.as_mut().unwrap().push(crate::Task {
            id: "accepted".into(), status: "accepted".into(), criteria: Some(vec!["prior criterion".into()]), ..Default::default()
        });
        assert_eq!(prepare_candidate(&root, &work, &agent, &settings).err().unwrap().code(), "managed-controls-policy-required");
        assert!(!candidates.exists());
        work.tasks.as_mut().unwrap().pop();
        assert!(prepare_candidate(&root, &work, &agent, &settings).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn managed_candidate_raw_sources_refuse_index_hidden_changes_and_recover() {
        for flag in ["--assume-unchanged", "--skip-worktree"] {
            let (root, work, agent, settings) = custody_fixture("");
            let candidate = prepare_candidate(&root, &work, &agent, &settings).unwrap();
            assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == candidate);
            let copy = Path::new(&candidate.workspace);
            git(
                copy,
                &["update-index", flag, "--", "tracked.txt"],
                &settings,
            )
            .unwrap();
            assert!(verify_candidate(&root, &work, &agent, &settings).is_ok());
            let original = std::fs::read(copy.join("tracked.txt")).unwrap();
            std::fs::write(copy.join("tracked.txt"), b"hidden index mutation\n").unwrap();
            // Establish that the index-based check alone misses this mutation.
            git(
                copy,
                &[
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--exit-code",
                    "HEAD",
                    "--",
                ],
                &settings,
            )
            .unwrap();
            assert_eq!(
                verify_candidate(&root, &work, &agent, &settings)
                    .err()
                    .unwrap()
                    .code(),
                "managed-candidate-raw-source-changed"
            );
            std::fs::write(copy.join("tracked.txt"), original).unwrap();
            assert!(verify_candidate(&root, &work, &agent, &settings).is_ok());
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn managed_candidate_checkpoint_reuses_exact_ref_after_missing_final_file_and_refuses_source_drift(
    ) {
        for subdir in ["", "apps/service"] {
            let (root, work, agent, settings) = custody_fixture(subdir);
            let first = prepare_candidate(&root, &work, &agent, &settings).unwrap();
            assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == first);
            assert!(prepare_candidate(&root, &work, &agent, &settings).unwrap() == first);
            let (key, _) = coordinates(&work, &agent, &first.result, &settings).unwrap();
            let storage = Path::new(
                &work
                    .planning
                    .as_ref()
                    .unwrap()
                    .repository
                    .as_ref()
                    .unwrap()
                    .storage,
            );
            let directory = storage.join("candidates").join(&key);
            let before = std::fs::read(directory.join("candidate.json")).unwrap();
            std::fs::rename(
                directory.join("candidate.json"),
                directory.join("candidate.original.json"),
            )
            .unwrap();
            let recovered = prepare_candidate(&root, &work, &agent, &settings).unwrap();
            assert_eq!(recovered.commit, first.commit);
            assert_ne!(recovered.stage, first.stage);
            assert!(Path::new(&first.workspace).is_dir());
            assert_eq!(
                std::fs::read(directory.join("candidate.original.json")).unwrap(),
                before
            );
            assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == recovered);
            let tracked = Path::new(&recovered.workspace).join("tracked.txt");
            let original = std::fs::read(&tracked).unwrap();
            std::fs::write(&tracked, b"changed tracked source\n").unwrap();
            assert!(verify_candidate(&root, &work, &agent, &settings).is_err());
            std::fs::write(&tracked, original).unwrap();
            assert!(verify_candidate(&root, &work, &agent, &settings).is_ok());
            let mut changed = work.clone();
            changed.tasks.as_mut().unwrap()[0].title = "changed contract".into();
            let (other, _) = coordinates(&changed, &agent, &first.result, &settings).unwrap();
            assert_ne!(other, key);
            assert!(verify_candidate(&root, &changed, &agent, &settings).is_err());
            assert!(!storage.join("candidates").join(other).exists());
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    /// Exercise the checkpoint consumer with real interpreter/callback witnesses.
    /// A held writable descriptor reproduces ETXTBSY; explicit interpreter reads
    /// the completed script without depending on exec of freshly written bytes.
    /// Both Git transformation families must stay unused during verification;
    /// raw tracked changes refuse and restoring bytes recovers the same original.
    /// Fixtures establish data custody only, never native admission or acceptance.
    #[test]
    fn managed_candidate_source_check_does_not_execute_textconv_or_hide_mutation() {
        for attribute in ["filter=fixture", "diff=fixture"] {
            let (root, work, agent, settings) = custody_fixture("");
            let candidate = prepare_candidate(&root, &work, &agent, &settings).unwrap();
            assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == candidate);
            let path = Path::new(&candidate.workspace);
            let marker = root.join("called");
            let program = root.join("callback.py");
            std::fs::write(&program, format!("#!/usr/bin/env python3\nfrom pathlib import Path\nPath({:?}).write_text('callback')\nprint('same converted output')\n", marker.to_str().unwrap())).unwrap();
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
            // Model the observed ETXTBSY condition deterministically: another
            // owned descriptor may still hold the generated file writable.
            let writable = std::fs::OpenOptions::new().write(true).open(&program).unwrap();
            assert_eq!(std::process::Command::new(&program).output().unwrap_err().raw_os_error(), Some(nix::errno::Errno::ETXTBSY as i32));
            // Match the shebang interpreter without exec'ing generated bytes.
            // No sleep/retry masks a failed callback or failed verification.
            assert!(std::process::Command::new("/usr/bin/env").arg("python3").arg(&program).output().unwrap().status.success());
            drop(writable);
            assert!(marker.exists());
            std::fs::remove_file(&marker).unwrap();
            std::fs::write(path.join(".gitattributes"), format!("tracked.txt {attribute}\n")).unwrap();
            let config = if attribute.starts_with("diff=") {
                "diff.fixture.textconv"
            } else {
                "filter.fixture.clean"
            };
            git(path, &["config", config, program.to_str().unwrap()], &settings).unwrap();
            let checked = verify_candidate(&root, &work, &agent, &settings);
            assert!(!marker.exists(), "verification executed {attribute}");
            assert!(checked.unwrap() == candidate);
            let tracked = path.join("tracked.txt");
            let original = std::fs::read(&tracked).unwrap();
            std::fs::write(&tracked, b"changed tracked bytes\n").unwrap();
            assert_eq!(verify_candidate(&root, &work, &agent, &settings).err().unwrap().code(),
                "managed-candidate-raw-source-changed");
            assert!(!marker.exists(), "refusal executed {attribute}");
            std::fs::write(&tracked, original).unwrap();
            assert!(verify_candidate(&root, &work, &agent, &settings).unwrap() == candidate);
            assert!(!marker.exists());
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}
