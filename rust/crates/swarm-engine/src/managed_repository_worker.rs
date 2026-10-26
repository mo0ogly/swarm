//! First manual or native-fenced managed copies. Original intent is not acceptance.
use super::{commit, git, hash, inside, read_manifest, text, validate, LocalDirectory, E};
use crate::{Agent, EngineSettingsDocument, Launch, Work};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Component, Path},
};
#[path = "managed_repository_result.rs"]
mod result;
pub(crate) use result::{retain_result, verify_result};
#[path = "managed_repository_candidate_store.rs"]
mod candidate_store;
pub(crate) use candidate_store::{prepare_candidate, verify_candidate, pin_candidate, CandidateWitness, ManagedCandidate};

/// Private attribution v1: first attempt only; schema version is not operator policy.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Proof {
    schema_version: u32,
    work: String,
    task: String,
    agent: String,
    base: String,
    path: String,
    staging: String,
    request_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    request_contract_sha256: Option<String>,
    workspace_identity: (u64, u64),
    git_identity: (u64, u64),
}
/// Open the original retained storage and nonblocking managed lane. Preserve
/// operation and lock/directory-close failures; create no Root or lock fallback.
fn with_storage<T>(
    root: &Path,
    work: &Work,
    exclusive: bool,
    run: impl FnOnce(&LocalDirectory) -> Result<T, E>,
) -> Result<T, E> {
    let repo = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?;
    let canonical =
        crate::canonical_project_root(root).map_err(|e| E::caused("managed-project-path", e))?;
    if !swarm_identity_contract::name(work.id.as_bytes())
        || Path::new(&repo.storage) != canonical.join(".swarm/managed").join(&work.id)
    {
        return Err(E::Refused("managed-storage-binding-mismatch"));
    }
    inside(
        LocalDirectory::open(&canonical.join(".swarm"))
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
                        |dir| {
                            let lock = dir
                                .reserve(exclusive, false)
                                .map_err(|e| E::caused("managed-operation-busy", e))?;
                            let result = run(dir);
                            crate::agent_launch_error::finish(
                                result,
                                lock.close().map_err(|e| E::caused("managed-lock-close", e)),
                            )
                        },
                    )
                },
            )
        },
    )
}
/// Check the existing repository originals while holding the managed lane.
/// This validates Git objects, never trusts a copied path or metadata alone.
fn repository(
    dir: &LocalDirectory,
    work: &Work,
    settings: &EngineSettingsDocument,
) -> Result<(), E> {
    let repo = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?;
    let retained =
        read_manifest(dir, "repository.json")?.ok_or(E::Refused("managed-manifest-required"))?;
    if read_manifest(dir, "repository.json.pending")?.is_some_and(|p| p != retained)
        || retained.base_commit != repo.base_commit
        || retained.snapshot_digest != repo.snapshot_digest
    {
        return Err(E::Refused("managed-manifest-baseline-mismatch"));
    }
    validate(&retained, repo, dir, settings)?;
    inside(
        dir.child("repository.git", false)
            .map_err(|e| E::caused("managed-repository-open", e))?,
        |bare| commit(&bare.process_path(), &repo.candidate_commit, settings),
    )
}
/// Return an initial task's future managed workspace for a standing FIFO claim.
/// Validate canonical storage, retained candidate and subdirectory under a shared
/// lane; no copy, manifest, SQL write, provider or host write probe is created.
/// The real fenced factory must create/verify the copy and perform host checks
/// before admission/grant. A future path establishes neither host readiness nor
/// execution permission. Missing proof, invalid path/history and IO/close errors
/// retain their cause; caller independently checks task/model/budget eligibility.
pub(crate) fn candidate_worker(root: &Path, work: &Work, task: &str, settings: &EngineSettingsDocument) -> Result<String, E> {
    let mut selected = work.tasks.iter().flatten().filter(|t| t.id == task);
    let t = selected.next().ok_or(E::Refused("managed-candidate-task-missing"))?;
    if selected.next().is_some() || !swarm_identity_contract::name(task.as_bytes())
        || t.status != "todo" || t.attempts.iter().flatten().next().is_some() {
        return Err(E::Refused("managed-worker-first-attempt-only"));
    }
    let repo = work.planning.as_deref().and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?;
    if Path::new(&repo.project_subdir).components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(E::Refused("managed-candidate-subdir-invalid"));
    }
    with_storage(root, work, false, |dir| {
        repository(dir, work, settings)?;
        let mut workspace = Path::new(&repo.storage).join("copies").join(format!("{task}-1"));
        // Push validated normal components only: join("") adds a trailing slash,
        // which is not a canonical FIFO path. No filesystem resolution is needed.
        for component in Path::new(&repo.project_subdir).components() {
            workspace.push(component.as_os_str());
        }
        workspace.to_str().map(str::to_owned)
            .ok_or(E::Refused("managed-candidate-workspace-utf8"))
    })
}
/// Decode exactly one private canonical original. Unknown fields/duplicates and
/// noncanonical bytes refuse; absence is not a new proof or repair permission.
fn proof(dir: &LocalDirectory, name: &str) -> Result<Proof, E> {
    let raw = dir
        .read(name)
        .map_err(|e| E::caused("managed-worker-proof-read", e))?;
    let p: Proof =
        serde_json::from_slice(&raw).map_err(|e| E::caused("managed-worker-proof-invalid", e))?;
    if crate::encode_model(&p).map_err(|e| E::caused("managed-worker-proof-encode", e))? != raw
        || p.schema_version != 1
    {
        return Err(E::Refused("managed-worker-proof-version-or-encoding"));
    }
    Ok(p)
}
/// Hash the business intention excluding its optimistic SQL revision. The live
/// caller still checks that revision before preparing or executing; an unused
/// copy may survive a rejected admission and a later public budget update.
/// All other request fields, including event/task/provider/instruction, stay bound.
fn contract_digest(raw: &[u8]) -> Result<String, E> {
    let mut r =
        crate::decode_launch(raw).map_err(|e| E::caused("managed-worker-request-invalid", e))?;
    if crate::encode_model(&r).map_err(|e|E::caused("managed-worker-request-encode",e))?!=raw {
        return Err(E::Refused("managed-worker-request-noncanonical"));
    }
    r.expected_revision = 0;
    let bytes =
        crate::encode_model(&r).map_err(|e| E::caused("managed-worker-request-encode", e))?;
    Ok(hash(&bytes))
}
/// Bind the stored task/source/first copy/original intent without filesystem IO.
/// No retry, parent delegation or legacy proof adoption exists. Native automatic
/// origin is bound to the canonical request; its authority is checked separately.
fn binding(work: &Work, r: &Launch, raw: &[u8], p: &Proof) -> Result<(), E> {
    let repo = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?;
    let expected = Path::new(&repo.storage)
        .join("copies")
        .join(format!("{}-1", r.task_id));
    if !swarm_identity_contract::name(r.event_id.as_bytes())
        || !swarm_identity_contract::name(r.task_id.as_bytes())
        || !matches!(r.origin.as_str(), "" | "operateur" | "conducteur")
        || (r.origin == "conducteur" && !swarm_identity_contract::name(r.conductor_id.as_bytes()))
        || p.work != work.id
        || p.task != r.task_id
        || p.agent != r.event_id
        || Path::new(&p.path) != expected
        || (p.request_sha256 != hash(raw)
            && p.request_contract_sha256.as_deref() != Some(contract_digest(raw)?.as_str()))
    {
        return Err(E::Refused("managed-worker-binding-mismatch"));
    }
    if p.request_contract_sha256
        .as_deref()
        .is_some_and(|h| h.len() != 64)
    {
        return Err(E::Refused("managed-worker-contract-hash-invalid"));
    }
    if let Some(h) = &p.request_contract_sha256 {
        if h != &contract_digest(raw)? {
            return Err(E::Refused("managed-worker-binding-mismatch"));
        }
    }
    Ok(())
}
/// Verify a standalone clone/project workspace against retained inode observations.
/// Before execution HEAD must equal the prepared candidate. After execution its
/// mutable content is retained, never treated as integrated or accepted. No same-UID
/// confinement or inode-reuse protection after the owned descriptors close is claimed.
fn workspace(
    p: &Proof,
    work: &Work,
    settings: &EngineSettingsDocument,
    before_execution: bool,
) -> Result<String, E> {
    let repo = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?;
    if Path::new(&repo.project_subdir)
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(E::Refused("managed-worker-subdir-invalid"));
    }
    inside(
        LocalDirectory::open(Path::new(&p.path))
            .map_err(|e| E::caused("managed-worker-copy-open", e))?,
        |copy| {
            inside(
                copy.child(".git", false)
                    .map_err(|e| E::caused("managed-worker-git-open", e))?,
                |gitdir| {
                    if gitdir
                        .identity()
                        .map_err(|e| E::caused("managed-worker-git-identity", e))?
                        != p.git_identity
                    {
                        return Err(E::Refused("managed-worker-git-replaced"));
                    }
                    match std::fs::symlink_metadata(
                        copy.process_path().join(".git/objects/info/alternates"),
                    ) {
                        Ok(_) => return Err(E::Refused("managed-worker-alternates-unported")),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(E::caused("managed-worker-alternates-read", e)),
                    }
                    if text(git(
                        &copy.process_path(),
                        &["rev-parse", "--git-common-dir"],
                        settings,
                    )?)? != ".git"
                    {
                        return Err(E::Refused("managed-worker-shared-git-unported"));
                    }
                    let actual = text(git(
                        &copy.process_path(),
                        &["rev-parse", "--show-toplevel"],
                        settings,
                    )?)?;
                    if Path::new(&actual) != Path::new(&p.path) {
                        return Err(E::Refused("managed-worker-toplevel-mismatch"));
                    }
                    if before_execution
                        && (repo.candidate_commit != p.base
                            || text(git(&copy.process_path(), &["rev-parse", "HEAD"], settings)?)?
                                != p.base)
                    {
                        return Err(E::Refused("managed-worker-base-changed"));
                    }
                    let path = if repo.project_subdir.is_empty() {
                        Path::new(&p.path).to_path_buf()
                    } else {
                        Path::new(&p.path).join(&repo.project_subdir)
                    };
                    let actual = std::fs::canonicalize(&path)
                        .map_err(|e| E::caused("managed-worker-workspace-path", e))?;
                    if actual != path || !actual.starts_with(&p.path) {
                        return Err(E::Refused("managed-worker-workspace-escaped"));
                    }
                    inside(
                        LocalDirectory::open_owned(&actual)
                            .map_err(|e| E::caused("managed-worker-workspace-open", e))?,
                        |dir| {
                            if dir
                                .owned_identity()
                                .map_err(|e| E::caused("managed-worker-workspace-identity", e))?
                                != p.workspace_identity
                            {
                                return Err(E::Refused("managed-worker-workspace-replaced"));
                            }
                            actual
                                .to_str()
                                .map(str::to_owned)
                                .ok_or(E::Refused("managed-worker-workspace-encoding"))
                        },
                    )
                },
            )
        },
    )
}
/// Prepare a first manual or native-fenced copy before host observation; original request bytes
/// remain canonical. Retain failed staging/manifests, never overwrite an unknown
/// copy; matching attribution recovers SQL rollback without another clone. Caller
/// must still record the SQL row and execute all admission/preflight/settlement
/// guards. automatic_conductor is borrowed from a real engine fence, not JSON;
/// public manual requests cannot turn a conducteur label into that capability.
/// No provider, reservation, retry, acceptance or cleanup deletion occurs.
pub(crate) fn prepare_worker(
    root: &Path,
    work: &Work,
    r: &Launch,
    raw: &[u8],
    settings: &EngineSettingsDocument,
    automatic_conductor: Option<&str>,
) -> Result<String, E> {
    let task = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == r.task_id)
        .ok_or(E::Refused("agent-launch-task-missing"))?;
    let origin_matches = match automatic_conductor {
        Some(conductor) => r.origin == "conducteur" && r.conductor_id == conductor
            && swarm_identity_contract::name(conductor.as_bytes()),
        None => matches!(r.origin.as_str(), "" | "operateur") && r.conductor_id.is_empty(),
    };
    if !origin_matches
        || task.attempts.iter().flatten().next().is_some()
        || task.status != "todo"
    {
        return Err(E::Refused("managed-worker-first-attempt-only"));
    }
    let repo = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-repository-required"))?;
    if !swarm_identity_contract::name(r.event_id.as_bytes())
        || !swarm_identity_contract::name(r.task_id.as_bytes())
    {
        return Err(E::Refused("managed-worker-identifier-invalid"));
    }
    with_storage(root, work, true, |dir| {
        repository(dir, work, settings)?;
        let name = format!("native-worker-{}.json", r.event_id);
        if dir
            .names()
            .map_err(|e| E::caused("managed-worker-proof-list", e))?
            .iter()
            .any(|n| n == name.as_bytes())
        {
            let p = proof(dir, &name)?;
            binding(work, r, raw, &p)?;
            return inside(
                dir.child("copies", false)
                    .map_err(|e| E::caused("managed-worker-copies-open", e))?,
                |copies| {
                    let final_name = format!("{}-1", r.task_id);
                    if !copies
                        .names()
                        .map_err(|e| E::caused("managed-worker-copies-list", e))?
                        .iter()
                        .any(|n| n == final_name.as_bytes())
                    {
                        inside(
                            copies
                                .child(&p.staging, false)
                                .map_err(|e| E::caused("managed-worker-stage-open", e))?,
                            |staging| {
                                let mut staged = p.clone();
                                staged.path = Path::new(&repo.storage)
                                    .join("copies")
                                    .join(&p.staging)
                                    .join("copy")
                                    .to_str()
                                    .ok_or(E::Refused("managed-worker-path-encoding"))?
                                    .into();
                                workspace(&staged, work, settings, true)?;
                                copies
                                    .rename_from(staging, "copy", &final_name)
                                    .map_err(|e| E::caused("managed-worker-copy-publish", e))
                            },
                        )?;
                    }
                    workspace(&p, work, settings, true)
                },
            );
        }
        inside(
            dir.child("copies", true)
                .map_err(|e| E::caused("managed-worker-copies-open", e))?,
            |copies| {
                let final_name = format!("{}-1", r.task_id);
                if copies
                    .names()
                    .map_err(|e| E::caused("managed-worker-copies-list", e))?
                    .iter()
                    .any(|n| n == final_name.as_bytes())
                {
                    return Err(E::Refused("managed-worker-unattributed-copy"));
                }
                let stage = crate::generate_identifier("prepare-")
                    .map_err(|e| E::caused("managed-worker-stage-id", e))?;
                inside(
                    copies
                        .child(&stage, true)
                        .map_err(|e| E::caused("managed-worker-stage-open", e))?,
                    |staging| {
                        let clone = staging.process_path().join("copy");
                        inside(
                            dir.child("repository.git", false)
                                .map_err(|e| E::caused("managed-worker-bare-open", e))?,
                            |bare| {
                                let source = bare.process_path();
                                git(
                                    &staging.process_path(),
                                    &[
                                        "clone",
                                        "--no-checkout",
                                        "--no-local",
                                        "--no-hardlinks",
                                        "--",
                                        source
                                            .to_str()
                                            .ok_or(E::Refused("managed-worker-path-encoding"))?,
                                        clone
                                            .to_str()
                                            .ok_or(E::Refused("managed-worker-path-encoding"))?,
                                    ],
                                    settings,
                                )
                                .map(|_| ())
                            },
                        )?;
                        // Only these freshly created owned staging directories are privatized.
                        for path in [&clone, &clone.join(".git")] {
                            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
                                .map_err(|e| E::caused("managed-worker-stage-permissions", e))?;
                        }
                        git(
                            &clone,
                            &["checkout", "--detach", &repo.candidate_commit],
                            settings,
                        )?;
                        let source = Path::new(&repo.storage).join("repository.git");
                        git(
                            &clone,
                            &[
                                "remote",
                                "set-url",
                                "origin",
                                source
                                    .to_str()
                                    .ok_or(E::Refused("managed-worker-path-encoding"))?,
                            ],
                            settings,
                        )?;
                        let project = clone.join(&repo.project_subdir);
                        let actual = std::fs::canonicalize(&project)
                            .map_err(|e| E::caused("managed-worker-stage-project", e))?;
                        let top = std::fs::canonicalize(&clone)
                            .map_err(|e| E::caused("managed-worker-stage-top", e))?;
                        if actual != top.join(&repo.project_subdir) || !actual.starts_with(&top) {
                            return Err(E::Refused("managed-worker-workspace-escaped"));
                        }
                        let workspace_identity = inside(
                            LocalDirectory::open_owned(&actual)
                                .map_err(|e| E::caused("managed-worker-stage-workspace", e))?,
                            |d| {
                                d.owned_identity()
                                    .map_err(|e| E::caused("managed-worker-stage-identity", e))
                            },
                        )?;
                        let git_identity = inside(
                            LocalDirectory::open(&clone.join(".git"))
                                .map_err(|e| E::caused("managed-worker-stage-git", e))?,
                            |d| {
                                d.identity()
                                    .map_err(|e| E::caused("managed-worker-stage-git-identity", e))
                            },
                        )?;
                        let p = Proof {
                            schema_version: 1,
                            work: work.id.clone(),
                            task: r.task_id.clone(),
                            agent: r.event_id.clone(),
                            base: repo.candidate_commit.clone(),
                            staging: stage.clone(),
                            path: Path::new(&repo.storage)
                                .join("copies")
                                .join(&final_name)
                                .to_str()
                                .ok_or(E::Refused("managed-worker-path-encoding"))?
                                .into(),
                            request_sha256: hash(raw),
                            request_contract_sha256: Some(contract_digest(raw)?),
                            workspace_identity,
                            git_identity,
                        };
                        let encoded = crate::encode_model(&p)
                            .map_err(|e| E::caused("managed-worker-proof-encode", e))?;
                        dir.publish_new_with_fault(
                            &name,
                            &encoded,
                            crate::local_storage::AtomicStorageFault::None,
                        )
                        .map_err(|e| E::caused("managed-worker-proof-publish", e))?;
                        copies
                            .rename_from(staging, "copy", &final_name)
                            .map_err(|e| E::caused("managed-worker-copy-publish", e))?;
                        workspace(&p, work, settings, true)
                    },
                )
            },
        )
    })
}
/// Lifecycle boundaries are protocol distinctions, never operator defaults.
/// Only admission/execution require the mutable copy. Completion already has a
/// checked owner/kernel receipt: it records ended work without accepting content.
pub(crate) enum WorkerCheck {
    Record,
    BeforeExecution,
    AfterExecution,
    /// Read retained request/proof/SQL attribution only; never insert or inspect the mutable copy.
    Observe,
    Cancel,
}
/// Apply exactly the physical/SQL checks for the caller's lifecycle boundary.
/// Record inserts attribution inside admission; Record/BeforeExecution check the
/// physical copy before permission. Cancel/AfterExecution require original proof
/// and SQL only (as does Observe): mutable-copy loss cannot prevent recording an unstarted cancel
/// or a kernel-proven ended process. Native automatic rows also require their
/// full original mission/FIFO chain after the receipt is staged. Record uses
/// engine-created Binding4 shape only; BeforeExecution must follow after staging
/// in the same writer before commit. No integration or acceptance follows.
pub(crate) fn verify_worker(
    root: &Path,
    db: &Connection,
    work: &Work,
    agent: &Agent,
    settings: &EngineSettingsDocument,
    mode: WorkerCheck,
) -> Result<String, E> {
    let insert = matches!(mode, WorkerCheck::Record);
    let physical = matches!(mode, WorkerCheck::Record | WorkerCheck::BeforeExecution);
    let before_execution = matches!(mode, WorkerCheck::Record | WorkerCheck::BeforeExecution);
    let raw = crate::agent_launch::query(
        db,
        "SELECT request FROM main.agents WHERE id=?",
        [&agent.id],
        |rows| {
            Ok(rows
                .next()?
                .ok_or(E::Refused("managed-worker-agent-missing"))?
                .get::<_, Vec<u8>>(0)?)
        },
    )?;
    let r =
        crate::decode_launch(&raw).map_err(|e| E::caused("managed-worker-request-invalid", e))?;
    match agent.origin.as_str() {
        "operateur" if agent.native_mission.is_none() && matches!(r.origin.as_str(), "" | "operateur")
            && r.conductor_id.is_empty() => {},
        "conducteur" if r.origin == "conducteur" => {
            // Record precedes the action receipt. Decode the engine-created v4
            // shape now; BeforeExecution below the staged receipt and every
            // subsequent lifecycle read must verify its full original chain.
            let b = if insert {
                let raw = agent.native_mission.as_ref().ok_or(E::Refused("managed-worker-mission-binding-required"))?;
                let b: crate::root_store::mission_agent::Binding = swarm_wire_json::decode(raw.0.get().as_bytes())
                    .map_err(|e| E::caused("managed-worker-mission-binding-invalid", e))?;
                b.validate().map_err(|e| E::caused("managed-worker-mission-binding-invalid", e))?;
                if b.schema_version != 4 { return Err(E::Refused("mission-agent-instance-required")); }
                b
            } else {
                crate::root_store::mission_agent::inspect(db, agent, settings, crate::root_store::mission_agent::Check::Original)?
                    .ok_or(E::Refused("managed-worker-mission-binding-required"))?
            };
            if b.conductor != r.conductor_id || !swarm_identity_contract::name(r.conductor_id.as_bytes()) {
                return Err(E::Refused("managed-worker-conductor-mismatch"));
            }
        },
        _ => return Err(E::Refused("managed-worker-origin-mismatch")),
    }
    with_storage(root, work, false, |dir| {
        let p = proof(dir, &format!("native-worker-{}.json", agent.id))?;
        binding(work, &r, &raw, &p)?;
        let repo = work
            .planning
            .as_deref()
            .and_then(|p| p.repository.as_deref())
            .ok_or(E::Refused("managed-repository-required"))?;
        let expected = if repo.project_subdir.is_empty() {
            Path::new(&p.path).to_path_buf()
        } else {
            Path::new(&p.path).join(&repo.project_subdir)
        };
        if agent.work_id != work.id
            || agent.task_id != p.task
            || Path::new(&agent.workspace) != expected
        {
            return Err(E::Refused("managed-worker-agent-binding-mismatch"));
        }
        if insert {
            db.execute("INSERT INTO main.managed_attempts(agent_id,work_id,task_id,base_commit,path,state) VALUES(?,?,?,?,?,'ready')",rusqlite::params![agent.id,work.id,p.task,p.base,p.path])?;
        }
        let actual=crate::agent_launch::query(db,"SELECT work_id,task_id,base_commit,path,state,result_commit,detail FROM main.managed_attempts WHERE agent_id=?",[&agent.id],|rows| {
            let row=rows.next()?.ok_or(E::Refused("managed-worker-row-missing"))?;
            (0..7).map(|i|row.get::<_,String>(i).map_err(Into::into)).collect::<Result<Vec<_>,E>>()
        })?;
        let ready = actual == [work.id.clone(),p.task.clone(),p.base.clone(),p.path.clone(),"ready".into(),String::new(),String::new()];
        if !ready {
            if physical || actual[..4] != [work.id.clone(),p.task.clone(),p.base.clone(),p.path.clone()] || actual[4] != "integrated" {
                return Err(E::Refused("managed-worker-row-mismatch"));
            }
            crate::managed_publication_record::integrated(db,work,&agent.id,&p.task,&p.base,&actual[5],&actual[6])?;
        }
        if physical {
            repository(dir, work, settings)?;
            workspace(&p, work, settings, before_execution)
        } else {
            expected
                .to_str()
                .map(str::to_owned)
                .ok_or(E::Refused("managed-worker-path-encoding"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// DTO fixture only; hashes bind original bytes but confer no runtime grant.
    fn fixture() -> (Work, Launch, Vec<u8>, Proof) {
        let work = Work {
            id: "w".into(),
            planning: Some(Box::new(crate::PlanningState {
                repository: Some(Box::new(crate::ManagedRepository {
                    storage: "/root/.swarm/managed/w".into(),
                    ..Default::default()
                })),
                ..Default::default()
            })),
            ..Default::default()
        };
        let r = Launch {
            event_id: "agent".into(),
            task_id: "task".into(),
            ..Default::default()
        };
        let raw = crate::encode_model(&r).unwrap();
        let proof = Proof {
            schema_version: 1,
            work: "w".into(),
            task: "task".into(),
            agent: "agent".into(),
            base: "a".repeat(40),
            path: "/root/.swarm/managed/w/copies/task-1".into(),
            staging: "prepare-fixture".into(),
            request_sha256: hash(&raw),
            request_contract_sha256: Some(contract_digest(&raw).unwrap()),
            workspace_identity: (1, 2),
            git_identity: (1, 3),
        };
        (work, r, raw, proof)
    }
    /// A changed canonical intention never adopts a previous copy/event proof.
    #[test]
    fn original_intention_drift_refuses() {
        let (w, mut r, raw, p) = fixture();
        assert!(binding(&w, &r, &raw, &p).is_ok());
        let mut noncanonical=raw.clone();noncanonical.push(b' ');
        assert_eq!(contract_digest(&noncanonical).unwrap_err().code(),"managed-worker-request-noncanonical");
        r.instruction = "Changed instruction".into();
        let changed = crate::encode_model(&r).unwrap();
        assert_eq!(
            binding(&w, &r, &changed, &p).unwrap_err().code(),
            "managed-worker-binding-mismatch"
        );
        r.instruction.clear();
        r.expected_revision = 3;
        let refreshed = crate::encode_model(&r).unwrap();
        assert!(binding(&w, &r, &refreshed, &p).is_ok());
        r.provider = "other".into();
        let wrong = crate::encode_model(&r).unwrap();
        assert!(binding(&w, &r, &wrong, &p).is_err());
    }
    /// Manual/automatic and path boundaries cannot be inferred from a saved label.
    #[test]
    fn automatic_origin_and_unrelated_copy_refuse() {
        let (w, mut r, raw, mut p) = fixture();
        assert!(binding(&w, &r, &raw, &p).is_ok());
        r.origin = "conducteur".into();
        assert!(binding(&w, &r, &raw, &p).is_err());
        r.origin.clear();
        p.path = "/root/other".into();
        assert!(binding(&w, &r, &raw, &p).is_err());
    }
    /// Unknown proof fields and duplicate coordinates refuse, never disappear.
    #[test]
    fn proof_rejects_unknown_and_duplicate_fields() {
        let (_, _, _, p) = fixture();
        let raw = crate::encode_model(&p).unwrap();
        assert!(serde_json::from_slice::<Proof>(&raw).is_ok());
        let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        value["unknown"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<Proof>(value).is_err());
        let mut duplicate = String::from_utf8(raw).unwrap();
        duplicate.insert_str(1, "\"agent\":\"other\",");
        assert!(serde_json::from_str::<Proof>(&duplicate).is_err());
    }
}
