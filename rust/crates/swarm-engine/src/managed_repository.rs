//! Committed-only managed bootstrap/copies; attribution is no publication grant.
#![cfg(target_os = "linux")]
use crate::agent_launch_error::finish;
use crate::local_storage::LocalDirectory;
use crate::{
    AgentLaunchError as E, EngineSettingsDocument, ManagedRepository, ManagedRepositoryRequest,
};
#[path = "managed_repository_worker.rs"]
mod worker;
#[path = "managed_repository_candidate.rs"]
mod candidate;
#[path = "managed_repository_index.rs"]
mod index;
#[path = "managed_repository_source.rs"]
mod source;
#[path = "managed_repository_controls.rs"]
mod controls;
pub(crate) use controls::{ControlPlan as ManagedControlPlan, plan as managed_control_plan};
pub(crate) use worker::{candidate_worker, clean_lane_contention, prepare_worker, prepare_candidate, verify_candidate, pin_candidate, CandidateWitness, ManagedCandidate, retain_result, verify_result, verify_worker, WorkerCheck};
pub(crate) use worker::with_storage as with_bound_storage;
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

/// Own and reap a fixed internal Git command under the saved per-command policy.
/// No shell, hooks, fsmonitor, environment Git overrides or optional index writes.
/// Private raw output stays in typed GitReportError, never public formatting.
/// Crate callers owe fixed validated arguments and any descriptor-backed cwd
/// lifetime; this runner does not grant authority or validate arbitrary Git args.
pub(crate) fn git(root: &Path, args: &[&str], settings: &EngineSettingsDocument) -> Result<Vec<u8>, E> {
    let (command, deadline) = git_command(root, args, settings)?;
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let run =
        crate::validation_run::run_owned_command(command, deadline, &mut stdout, Some(&mut stderr));
    match run {
        Err(cause) => Err(E::caused(
            "managed-git-process",
            crate::GitReportError::Process {
                cause: Box::new(cause),
                output: Box::new(crate::GitCommandOutput::from_parts(stdout, stderr)),
            },
        )),
        Ok(run) if run.timed_out || !run.usage.status.success() => Err(E::caused(
            "managed-git-command",
            crate::GitReportError::Command {
                exit_code: run.usage.status.code(),
                timed_out: run.timed_out,
                stdout_sha256: hash(&stdout),
                stderr_sha256: hash(&stderr),
                output: Box::new(crate::GitCommandOutput::from_parts(stdout, stderr)),
            },
        )),
        Ok(_) => Ok(stdout),
    }
}
/// Build a fixed internal Git command with the saved per-command deadline.
/// Callers owe validated arguments and retained directory capabilities. This
/// strips Git overrides/hooks and owns no process, output or business authority.
/// Invalid settings or unrepresentable deadlines refuse before launch.
pub(crate) fn git_command(root: &Path, args: &[&str], settings: &EngineSettingsDocument) -> Result<(Command, Option<Instant>), E> {
    let timeout = settings
        .values
        .git_command_timeout()
        .map_err(|e| E::caused(e.reason(), e))?;
    let deadline = timeout
        .map(|d| {
            Instant::now()
                .checked_add(d)
                .ok_or(E::Refused("managed-git-deadline-representation"))
        })
        .transpose()?;
    let mut command = Command::new("git");
    command.args([
        "--no-pager",
        "--no-optional-locks",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "protocol.file.allow=always",
        "-c",
        "core.fsync=all",
        "-c",
        "protocol.allow=never",
        "-c",
        "uploadpack.packObjectsHook=",
        "-c",
        "core.alternateRefsCommand=",
        "-c",
        "init.templateDir=",
    ]);
    command.args(args).current_dir(root);
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            command.env_remove(key);
        }
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    Ok((command, deadline))
}
/// Exact Go-compatible request/snapshot digest, never an authenticity claim.
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Decode fixed Git path/ID output; malformed text refuses with original cause.
fn text(bytes: Vec<u8>) -> Result<String, E> {
    crate::environment::go_command_text(&bytes).map_err(|e| E::caused("managed-git-text", e))
}
/// Close each owned descriptor even when its operation failed; preserve both causes.
fn inside<T>(
    directory: LocalDirectory,
    run: impl FnOnce(&LocalDirectory) -> Result<T, E>,
) -> Result<T, E> {
    let result = run(&directory);
    finish(
        result,
        directory
            .close()
            .map_err(|e| E::caused("managed-directory-close", e)),
    )
}

/// Bind a retained managed-storage locator to the already opened datastore and
/// Work directory. Historical root aliases are accepted only when both private
/// directories have the same device/inode as the canonical traversal; every
/// descriptor stays owned through `run`, preventing inode reuse during the check.
/// The stored layout must remain absolute `.swarm/managed/<work>`, with no parent
/// traversal or symlink children below the state directory. Operation IO uses
/// the caller's canonical directory, never an alias-derived replacement.
/// No locator/manifest/history is rewritten and no execution or acceptance is
/// granted. IO, identity and independent close failures propagate with causes;
/// this is not confinement against another process with the same UID.
fn with_storage_binding<T>(
    state: &LocalDirectory,
    directory: &LocalDirectory,
    work: &str,
    storage: &str,
    run: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let path = Path::new(storage);
    let managed = path.parent().ok_or(E::Refused("managed-storage-binding-mismatch"))?;
    let state_path = managed.parent().ok_or(E::Refused("managed-storage-binding-mismatch"))?;
    if !path.is_absolute()
        || path.components().any(|c| matches!(c, std::path::Component::ParentDir))
        || path.file_name().and_then(|s| s.to_str()) != Some(work)
        || managed.file_name().and_then(|s| s.to_str()) != Some("managed")
        || state_path.file_name().and_then(|s| s.to_str()) != Some(".swarm")
    {
        return Err(E::Refused("managed-storage-binding-mismatch"));
    }
    inside(
        LocalDirectory::open(state_path).map_err(|e| E::caused("managed-storage-alias-state", e))?,
        |alias_state| {
            let identity = |dir: &LocalDirectory| dir.owned_identity()
                .map_err(|e| E::caused("managed-storage-alias-identity", e));
            if identity(alias_state)? != identity(state)? {
                return Err(E::Refused("managed-storage-binding-mismatch"));
            }
            inside(
                alias_state.child("managed", false)
                    .map_err(|e| E::caused("managed-storage-alias-parent", e))?,
                |parent| inside(
                    parent.child(work, false)
                        .map_err(|e| E::caused("managed-storage-alias-work", e))?,
                    |alias_directory| {
                        if identity(alias_directory)? != identity(directory)? {
                            return Err(E::Refused("managed-storage-binding-mismatch"));
                        }
                        run()
                    },
                ),
            )
        },
    )
}
/// Decode retained manifest strictly; normalize only Go omitempty's empty list.
/// Nil/[] included lists have identical Go wire/selection semantics. Original
/// file bytes and nonempty ordering remain untouched; this is not adoption.
fn read_manifest(dir: &LocalDirectory, name: &str) -> Result<Option<ManagedRepository>, E> {
    let names = dir
        .names()
        .map_err(|e| E::caused("managed-manifest-list", e))?;
    if !names.iter().any(|n| n.as_slice() == name.as_bytes()) {
        return Ok(None);
    }
    let bytes = dir
        .read(name)
        .map_err(|e| E::caused("managed-manifest-read", e))?;
    let mut repo: ManagedRepository =
        crate::work::decode_mode(&bytes, crate::models::shape_managed_repository(), true)
            .map_err(|e| E::caused("managed-manifest-invalid", e))?;
    if repo.included.as_ref().is_some_and(Vec::is_empty) {
        repo.included = None;
    }
    Ok(Some(repo))
}
/// Require canonical immutable Git commit identity, not a revision expression.
fn commit(dir: &Path, oid: &str, settings: &EngineSettingsDocument) -> Result<(), E> {
    if !matches!(oid.len(), 40 | 64)
        || !oid
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(E::Refused("managed-commit-invalid"));
    }
    let expression = format!("{oid}^{{commit}}");
    if text(git(dir, &["rev-parse", "--verify", &expression], settings)?)? != oid {
        return Err(E::Refused("managed-commit-not-canonical"));
    }
    Ok(())
}
/// Validate every retained binding and both actual commit objects before reuse.
/// Pending/final divergence, missing objects and changed choices refuse unchanged.
fn validate(
    repo: &ManagedRepository,
    expected: &ManagedRepository,
    dir: &LocalDirectory,
    settings: &EngineSettingsDocument,
) -> Result<(), E> {
    if repo.source != expected.source
        || repo.storage != expected.storage
        || repo.project_subdir != expected.project_subdir
        || repo.request_digest != expected.request_digest
        || repo.included.as_deref().unwrap_or_default()
            != expected.included.as_deref().unwrap_or_default()
    {
        return Err(E::Refused("managed-manifest-binding-mismatch"));
    }
    if repo.snapshot_digest.len() != 64
        || !repo
            .snapshot_digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(E::Refused("managed-snapshot-invalid"));
    }
    inside(
        dir.child("repository.git", false)
            .map_err(|e| E::caused("managed-repository-open", e))?,
        |bare| {
            commit(&bare.process_path(), &repo.base_commit, settings)?;
            commit(&bare.process_path(), &repo.candidate_commit, settings)
        },
    )
}
/// Prepare/reuse one committed-only copy under its private nonblocking lane.
/// The caller already validated the whole fresh Work and original mutation intent.
/// Source files/index/HEAD are read only; no claim, agent, review or acceptance is
/// enabled. Dirty snapshots remain explicitly unported. Files survive failures
/// for diagnosis/recovery; pending publication precedes no-replace bare rename.
/// SQL rollback does not erase filesystem effects. All owned closes are checked.
pub(crate) fn prepare(
    root: &Path,
    work: &str,
    request: &ManagedRepositoryRequest,
    settings: &EngineSettingsDocument,
) -> Result<ManagedRepository, E> {
    if !swarm_identity_contract::name(work.as_bytes()) {
        return Err(E::Refused("managed-work-id-invalid"));
    }
    if !request.committed_only
        || request
            .include_dirty
            .as_ref()
            .is_some_and(|v| !v.is_empty())
    {
        return Err(E::Refused("managed-dirty-snapshot-unported"));
    }
    let root =
        crate::canonical_project_root(root).map_err(|e| E::caused("managed-project-path", e))?;
    let source = crate::local_directory(&root, &request.path)
        .map_err(|e| E::caused("managed-source-path", e))?;
    inside(
        LocalDirectory::open_owned(&source).map_err(|e| E::caused("managed-source-open", e))?,
        |source_dir| {
            let top = PathBuf::from(text(git(
                &source_dir.process_path(),
                &["rev-parse", "--show-toplevel"],
                settings,
            )?)?);
            let subdir = source
                .strip_prefix(&top)
                .map_err(|e| E::caused("managed-project-outside-repository", e))?;
            let digest = hash(
                &crate::encode_model(request)
                    .map_err(|e| E::caused("managed-request-encode", e))?,
            );
            let expected = ManagedRepository {
                source: source.to_string_lossy().into_owned(),
                storage: root
                    .join(".swarm/managed")
                    .join(work)
                    .to_string_lossy()
                    .into_owned(),
                project_subdir: subdir
                    .to_str()
                    .ok_or(E::Refused("managed-project-encoding"))?
                    .into(),
                request_digest: digest,
                included: request.include_dirty.clone().filter(|v| !v.is_empty()),
                ..Default::default()
            };
            inside(
                LocalDirectory::open(&root.join(".swarm"))
                    .map_err(|e| E::caused("managed-state-directory", e))?,
                |state| {
                    inside(
                        state
                            .child("managed", true)
                            .map_err(|e| E::caused("managed-parent-directory", e))?,
                        |parent| {
                            inside(
                                parent
                                    .child(work, true)
                                    .map_err(|e| E::caused("managed-work-directory", e))?,
                                |dir| {
                                    let lock = dir
                                        .reserve_managed(true, true)
                                        .map_err(|e| E::caused("managed-operation-busy", e))?;
                                    let result = locked(dir, &top, &expected, settings);
                                    finish(
                                        result,
                                        lock.close()
                                            .map_err(|e| E::caused("managed-lock-close", e)),
                                    )
                                },
                            )
                        },
                    )
                },
            )
        },
    )
}
/// Reuse matching prepared state or retain a new immutable copy and manifest.
/// Unknown orphan repositories and malformed/contradictory proofs never overwrite.
/// A moved bare with missing final manifest is recoverable only from its pending
/// original and present canonical commit objects. No source fallback is permitted.
fn locked(
    dir: &LocalDirectory,
    top: &Path,
    expected: &ManagedRepository,
    settings: &EngineSettingsDocument,
) -> Result<ManagedRepository, E> {
    let final_manifest = read_manifest(dir, "repository.json")?;
    let pending = read_manifest(dir, "repository.json.pending")?;
    if let Some(retained) = final_manifest.as_ref().or(pending.as_ref()) {
        if final_manifest
            .as_ref()
            .zip(pending.as_ref())
            .is_some_and(|(a, b)| a != b)
        {
            return Err(E::Refused("managed-manifest-divergence"));
        }
        validate(retained, expected, dir, settings)?;
        if final_manifest.is_none() {
            let raw = dir
                .read("repository.json.pending")
                .map_err(|e| E::caused("managed-pending-read", e))?;
            dir.publish_new_with_fault(
                "repository.json",
                &raw,
                crate::local_storage::AtomicStorageFault::None,
            )
            .map_err(|e| E::caused("managed-manifest-publish", e))?;
        }
        return Ok(retained.clone());
    }
    if dir
        .names()
        .map_err(|e| E::caused("managed-storage-list", e))?
        .iter()
        .any(|n| n == b"repository.git")
    {
        return Err(E::Refused("managed-orphan-repository"));
    }
    inside(
        LocalDirectory::open_owned(top).map_err(|e| E::caused("managed-source-top", e))?,
        |source| {
            let base = text(git(
                &source.process_path(),
                &["rev-parse", "HEAD"],
                settings,
            )?)?;
            commit(&source.process_path(), &base, settings)?;
            let staging_name = crate::generate_identifier("repository-build-")
                .map_err(|e| E::caused("managed-staging-id", e))?;
            inside(
                dir.child(&staging_name, true)
                    .map_err(|e| E::caused("managed-staging-directory", e))?,
                |staging| {
                    let destination = staging.process_path().join("repository.git");
                    git(
                        &staging.process_path(),
                        &[
                            "clone",
                            "--bare",
                            "--no-local",
                            "--no-hardlinks",
                            "--",
                            source
                                .process_path()
                                .to_str()
                                .ok_or(E::Refused("managed-source-encoding"))?,
                            destination
                                .to_str()
                                .ok_or(E::Refused("managed-storage-encoding"))?,
                        ],
                        settings,
                    )?;
                    std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o700))
                        .map_err(|e| E::caused("managed-copy-permissions", e))?;
                    inside(
                        staging
                            .child("repository.git", false)
                            .map_err(|e| E::caused("managed-copy-open", e))?,
                        |bare| {
                            commit(&bare.process_path(), &base, settings)?;
                            git(
                                &bare.process_path(),
                                &[
                                    "config",
                                    "remote.origin.url",
                                    top.to_str().ok_or(E::Refused("managed-source-encoding"))?,
                                ],
                                settings,
                            )?;
                            git(
                                &bare.process_path(),
                                &["update-ref", "refs/heads/swarm-result", &base],
                                settings,
                            )?;
                            git(
                                &bare.process_path(),
                                &["symbolic-ref", "HEAD", "refs/heads/swarm-result"],
                                settings,
                            )?;
                            Ok(())
                        },
                    )?;
                    let mut repo = expected.clone();
                    repo.base_commit = base.clone();
                    repo.candidate_commit = base.clone();
                    repo.snapshot_digest = hash(
                        &crate::encode_model(&serde_json::json!({"base":base,"files":{}}))
                            .map_err(|e| E::caused("managed-snapshot-encode", e))?,
                    );
                    let raw = crate::encode_model(&repo)
                        .map_err(|e| E::caused("managed-manifest-encode", e))?;
                    dir.publish_new_with_fault(
                        "repository.json.pending",
                        &raw,
                        crate::local_storage::AtomicStorageFault::None,
                    )
                    .map_err(|e| E::caused("managed-pending-publish", e))?;
                    dir.rename_from(staging, "repository.git", "repository.git")
                        .map_err(|e| E::caused("managed-copy-publish", e))?;
                    dir.publish_new_with_fault(
                        "repository.json",
                        &raw,
                        crate::local_storage::AtomicStorageFault::None,
                    )
                    .map_err(|e| E::caused("managed-manifest-publish", e))?;
                    Ok(repo)
                },
            )
        },
    )
}

/// Observe a stored managed Work's original prepared repository and current commit.
/// Requires the exact private storage location, coherent final/pending manifests
/// with unchanged baseline bindings, and both original/current commit objects.
/// Shared lane prevents a concurrent Rust preparation while observing. No repair,
/// source access, clone, counter/spend, process grant or acceptance occurs. Missing
/// or stale proofs and all independent lock/directory close failures refuse.
pub(crate) fn observe(
    root: &Path,
    work: &crate::Work,
    settings: &EngineSettingsDocument,
) -> Result<(), E> {
    let repo = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?;
    if !swarm_identity_contract::name(work.id.as_bytes()) {
        return Err(E::Refused("managed-work-id-invalid"));
    }
    let root =
        crate::canonical_project_root(root).map_err(|e| E::caused("managed-project-path", e))?;
    inside(
        LocalDirectory::open(&root.join(".swarm"))
            .map_err(|e| E::caused("managed-state-directory", e))?,
        |state| {
            inside(
                state
                    .child("managed", false)
                    .map_err(|e| E::caused("managed-parent-directory", e))?,
                |parent| {
                    inside(
                        parent
                            .child(&work.id, false)
                            .map_err(|e| E::caused("managed-work-directory", e))?,
                        |dir| with_storage_binding(state, dir, &work.id, &repo.storage, || {
                            let lock = dir
                                .reserve_managed(false, false)
                                .map_err(|e| E::caused("managed-operation-busy", e))?;
                            let result = (|| {
                                let retained = read_manifest(dir, "repository.json")?
                                    .ok_or(E::Refused("managed-manifest-required"))?;
                                if read_manifest(dir, "repository.json.pending")?
                                    .is_some_and(|pending| pending != retained)
                                {
                                    return Err(E::Refused("managed-manifest-divergence"));
                                }
                                if retained.base_commit != repo.base_commit
                                    || retained.snapshot_digest != repo.snapshot_digest
                                {
                                    return Err(E::Refused("managed-manifest-baseline-mismatch"));
                                }
                                validate(&retained, repo, dir, settings)?;
                                inside(
                                    dir.child("repository.git", false)
                                        .map_err(|e| E::caused("managed-repository-open", e))?,
                                    |bare| {
                                        commit(
                                            &bare.process_path(),
                                            &repo.candidate_commit,
                                            settings,
                                        )
                                    },
                                )
                            })();
                            finish(
                                result,
                                lock.close().map_err(|e| E::caused("managed-lock-close", e)),
                            )
                        }),
                    )
                },
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real committed bootstrap is the valid control. Historical root aliases
    /// keep their original manifest bytes; another datastore with the same Work
    /// name, a missing alias and divergent originals refuse, then recover.
    /// Uses synthetic fixture-only Git attribution, independent of personal or
    /// CI-runner configuration. No operator config, production Git attribution,
    /// agent, provider, database migration or live Root is changed.
    #[test]
    fn historical_root_alias_reads_without_rebinding_or_losing_refusals() {
        use std::os::unix::fs::{symlink, DirBuilderExt};
        let base=crate::fixture_root("managed-root-alias-");
        println!("ALIAS_FIXTURE {}",base.display());
        let root=base.join("canonical");
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(root.join(".swarm")).unwrap();
        let source=root.join("source");std::fs::create_dir(&source).unwrap();
        let settings=EngineSettingsDocument::initial(crate::EngineSettingsValues::installation_defaults().unwrap(),"owned-fixture".into(),"2026-11-08T00:00:00Z".into()).unwrap();
        git(&source,&["init","--quiet"],&settings).unwrap();
        std::fs::write(source.join("tracked.txt"),b"valid committed control\n").unwrap();
        git(&source,&["add","tracked.txt"],&settings).unwrap();
        // Synthetic fixture attribution matches the existing git_pack fixture.
        // Keep it local to this inert commit: never read or change the operator's
        // identity, repository config or production Git command contract.
        git(&source,&["-c","user.name=Fixture","-c","user.email=fixture@invalid","-c","core.hooksPath=/dev/null","-c","commit.gpgsign=false","commit","--quiet","-m","Owned alias observation fixture"],&settings).expect("synthetic alias fixture commit");
        let id="w-alias-test";
        let mut repo=prepare(&root,id,&ManagedRepositoryRequest{path:"source".into(),committed_only:true,..Default::default()},&settings).unwrap();
        let mut work=crate::Work{id:id.into(),planning:Some(Box::new(crate::PlanningState{repository:Some(Box::new(repo.clone())),..Default::default()})),..Default::default()};
        observe(&root,&work,&settings).unwrap();
        let alias=base.join("historical");symlink(&root,&alias).unwrap();
        repo.storage=alias.join(".swarm/managed").join(id).to_str().unwrap().into();
        let storage=root.join(".swarm/managed").join(id);
        let raw=crate::encode_model(&repo).unwrap();
        for file in ["repository.json","repository.json.pending"] {std::fs::write(storage.join(file),&raw).unwrap();}
        work.planning.as_mut().unwrap().repository=Some(Box::new(repo));
        observe(&root,&work,&settings).unwrap();
        assert_eq!(std::fs::read(storage.join("repository.json")).unwrap(),raw);
        // The Go producer used operation.lock. Reading that historical layout
        // must retain its lane without creating a competing Rust lock file.
        std::fs::rename(storage.join(".engine-settings.lock"),storage.join("operation.lock")).unwrap();
        observe(&root,&work,&settings).unwrap();
        assert!(!storage.join(".engine-settings.lock").exists());
        assert_eq!(std::fs::read(storage.join("repository.json")).unwrap(),raw);
        let other=base.join("other");
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(other.join(".swarm/managed").join(id)).unwrap();
        for file in ["repository.json","repository.json.pending"] {std::fs::write(other.join(".swarm/managed").join(id).join(file),&raw).unwrap();}
        std::fs::remove_file(&alias).unwrap();symlink(&other,&alias).unwrap();
        assert_eq!(observe(&root,&work,&settings).unwrap_err().code(),"managed-storage-binding-mismatch");
        std::fs::remove_file(&alias).unwrap();symlink(&root,&alias).unwrap();
        observe(&root,&work,&settings).unwrap();
        std::fs::remove_file(&alias).unwrap();assert!(observe(&root,&work,&settings).is_err());
        symlink(&root,&alias).unwrap();observe(&root,&work,&settings).unwrap();
        std::fs::write(storage.join("repository.json.pending"),b"{}").unwrap();
        assert_eq!(observe(&root,&work,&settings).unwrap_err().code(),"managed-manifest-divergence");
        std::fs::write(storage.join("repository.json.pending"),&raw).unwrap();
        observe(&root,&work,&settings).unwrap();
        assert_eq!(std::fs::read(storage.join("repository.json")).unwrap(),raw);
    }

    /// Actual wire decoding treats omitted/empty included lists identically,
    /// retaining original files and nonempty list order. No repository grant.
    #[test]
    fn manifest_empty_list_matches_omission_without_rewriting_original_bytes() {
        let root = crate::fixture_root("managed-wire-");
        let dir = LocalDirectory::open(&root).unwrap();
        dir.publish("nil.json", b"{}").unwrap();
        dir.publish("empty.json", br#"{"included":[]}"#).unwrap();
        assert_eq!(
            read_manifest(&dir, "nil.json").unwrap(),
            read_manifest(&dir, "empty.json").unwrap()
        );
        assert_eq!(dir.read("empty.json").unwrap(), br#"{"included":[]}"#);
        dir.publish("ordered.json", br#"{"included":["b","a"]}"#)
            .unwrap();
        assert_eq!(
            read_manifest(&dir, "ordered.json")
                .unwrap()
                .unwrap()
                .included,
            Some(vec!["b".into(), "a".into()])
        );
        dir.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    use std::os::unix::fs::symlink;

    /// Descriptor-relative directory lookup refuses symlinks and keeps its inode
    /// after a pathname rename; fixture permissions express the storage invariant.
    #[test]
    fn directories_pin_inodes_and_refuse_symlink_children() {
        let root = crate::fixture_root("managed-dir-");
        let dir = LocalDirectory::open(&root).unwrap();
        let child = dir.child("owned", true).unwrap();
        std::fs::rename(root.join("owned"), root.join("moved")).unwrap();
        symlink(root.join("moved"), root.join("owned")).unwrap();
        assert!(dir.child("owned", true).is_err());
        std::fs::write(child.process_path().join("pinned"), b"actual inode").unwrap();
        assert_eq!(
            std::fs::read(root.join("moved/pinned")).unwrap(),
            b"actual inode"
        );
        assert!(dir.child("../outside", true).is_err());
        child.close().unwrap();
        dir.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    /// No-replace publication retains both original directories on conflict;
    /// removing only the owned fixture conflict permits an actual move/reopen.
    #[test]
    fn repository_rename_never_replaces_a_prior_target() {
        let root = crate::fixture_root("managed-move-");
        let dir = LocalDirectory::open(&root).unwrap();
        let staging = dir.child("staging", true).unwrap();
        let source = staging.child("repository.git", true).unwrap();
        source.close().unwrap();
        let prior = dir.child("repository.git", true).unwrap();
        prior.close().unwrap();
        std::fs::write(root.join("repository.git/original"), b"preserve").unwrap();
        assert!(dir
            .rename_from(&staging, "repository.git", "repository.git")
            .is_err());
        assert_eq!(
            std::fs::read(root.join("repository.git/original")).unwrap(),
            b"preserve"
        );
        assert!(root.join("staging/repository.git").is_dir());
        std::fs::remove_dir_all(root.join("repository.git")).unwrap();
        dir.rename_from(&staging, "repository.git", "repository.git")
            .unwrap();
        assert!(!root.join("staging/repository.git").exists());
        dir.child("repository.git", false).unwrap().close().unwrap();
        staging.close().unwrap();
        dir.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
