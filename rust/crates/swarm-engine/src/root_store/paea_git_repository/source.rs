//! Owned managed-source lane, physical ancestry and current publication snapshot.
//! No pack launch, current business admission, reference effect or OS confinement.
use super::{
    finished, material_path, CurrentManagedGitPublication, PaeaGitRepository, RootBinding,
    RootStore, E,
};
use crate::local_storage::{LocalDirectory, LocalReservation};
use std::{fmt, path::Path};
use swarm_protection_contract::Proposal;

/// Owned source witnesses independent of a borrowed repository capability.
/// Keep this guard with every dependent process/error until ownership is resolved.
/// It holds a shared managed lane; it does not prevent same-UID filesystem swaps.
/// Call inspect before use and close explicitly. No raw path or command is public.
pub struct PaeaGitSource {
    pub(super) custody: SourceCustody,
    snapshot: CurrentManagedGitPublication,
}
/// Physical source witnesses can be acquired without observing any native Git.
/// They grant no snapshot, authority or effect. Keep them through failed native
/// acquisition, and checked-close only after every dependent child is resolved.
pub(super) struct SourceCustody {
    root: Option<RootBinding>,
    managed: Option<LocalDirectory>,
    work: Option<LocalDirectory>,
    bare: Option<LocalDirectory>,
    lane: Option<LocalReservation>,
    pub(super) work_id: String,
}
impl fmt::Debug for PaeaGitSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PaeaGitSource")
    }
}
/// Close an observed child even when identity checking failed; no child process
/// is launched inside this helper, so no asynchronous dependency is released.
fn inside<T>(
    directory: LocalDirectory,
    run: impl FnOnce(&LocalDirectory) -> Result<T, E>,
) -> Result<T, E> {
    let result = run(&directory);
    finished(result, directory.close().map_err(E::Storage))
}
impl PaeaGitSource {
    /// Complete a custody already held through native observation. Mismatched
    /// Work refuses while returning the same owner, not closing any dependency.
    pub(super) fn complete(
        custody: SourceCustody,
        snapshot: CurrentManagedGitPublication,
    ) -> Result<Self, (SourceCustody, E)> {
        if custody.work_id != snapshot.work_id {
            return Err((custody, E::Refused("paea-git-source-work-binding")));
        }
        Ok(Self { custody, snapshot })
    }
    /// Native candidate saved by a completed observation, not a current-use permit.
    pub fn candidate_oid(&self) -> &str {
        &self.snapshot.candidate_commit
    }
    /// Borrow the physical source retained by this completed observation.
    pub(super) fn bare(&self) -> Result<&LocalDirectory, E> {
        self.custody.bare()
    }
    /// Reobserve exact native publication through the repository's real reader,
    /// requiring the complete saved snapshot and both physical endpoints. All
    /// pre/post failures preserve their causes. This grants no business authority
    /// and starts no pack process. Keep the guard alive on a dependent error.
    pub fn inspect(
        &self,
        repository: &PaeaGitRepository,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
    ) -> Result<CurrentManagedGitPublication, E> {
        self.inspect_using(project, || repository.inspect(service, project, proposal))
    }
    /// Inspect the same held source/current snapshot inside this Root's actual
    /// main WRITE transaction. No new BEGIN or transaction release occurs; a
    /// caller must retain settings/Trust/plan/mandate and source/job ownership.
    /// No/read transaction refuses before observation. This grants no authority.
    pub(super) fn inspect_for_effect(
        &self,
        repository: &PaeaGitRepository,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
    ) -> Result<CurrentManagedGitPublication, E> {
        project.require_native_git_writer()?;
        self.inspect_using(project, || {
            repository.inspect_for_effect(service, project, proposal)
        })
    }
    /// Shared physical/snapshot checks for the two fixed internal readers. The
    /// observation cannot be supplied by a public caller or replaced by a boolean.
    /// Preserve postcheck errors without closing this caller-owned source guard.
    fn inspect_using(
        &self,
        project: &RootStore,
        observe: impl FnOnce() -> Result<CurrentManagedGitPublication, E>,
    ) -> Result<CurrentManagedGitPublication, E> {
        self.custody.check(project)?;
        let result = (|| {
            let current = observe()?;
            if crate::encode_model(&current).map_err(E::Encode)?
                != crate::encode_model(&self.snapshot).map_err(E::Encode)?
            {
                return Err(E::Refused("paea-git-source-publication-changed"));
            }
            Ok(current)
        })();
        finished(result, self.custody.check(project))
    }
    /// Checked-close all source witnesses. The caller must resolve dependent
    /// processes first; this never removes data or proves process completion.
    pub fn close(self) -> Result<(), E> {
        self.custody.close()
    }
}
impl SourceCustody {
    /// Open the exact canonical Work storage and shared lane using only native
    /// filesystem/SQLite descriptor reads. No Git starts, so partial failures
    /// can close safely; no copied path or existing alternate lock is adopted.
    pub(super) fn open(project: &RootStore, work_id: &str) -> Result<Self, E> {
        let work = project.read_work(work_id)?;
        if work.id != work_id {
            return Err(E::Refused("paea-git-source-work-binding"));
        }
        let repo = work
            .planning
            .as_deref()
            .and_then(|p| p.repository.as_deref())
            .ok_or(E::Refused("paea-git-managed-repository-required"))?;
        let canonical = crate::canonical_project_root(&project.root).map_err(|e| {
            E::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-source-project-path",
                e,
            )))
        })?;
        if !swarm_identity_contract::name(work.id.as_bytes())
            || Path::new(&repo.storage) != canonical.join(".swarm/managed").join(&work.id)
        {
            return Err(E::Refused("paea-git-source-storage-binding"));
        }
        let mut custody = Self {
            root: Some(project.bind_root()?),
            managed: None,
            work: None,
            bare: None,
            lane: None,
            work_id: work.id,
        };
        match custody.open_parts().and_then(|_| custody.check(project)) {
            Ok(()) => Ok(custody),
            Err(e) => finished(Err(e), custody.close()),
        }
    }
    /// Require the fully opened internal capability; no pathname fallback exists.
    fn binding(&self) -> Result<&RootBinding, E> {
        self.root
            .as_ref()
            .ok_or(E::Refused("paea-git-source-not-open"))
    }
    /// Borrow the held bare directory for a future crate-owned transfer job.
    /// The caller owes this guard's lifetime and process/authority checks.
    pub(super) fn bare(&self) -> Result<&LocalDirectory, E> {
        self.bare
            .as_ref()
            .ok_or(E::Refused("paea-git-source-not-open"))
    }
    /// Populate descriptor-relative parents and shared lane. No Git process is
    /// launched; failed construction closes every partially opened capability.
    fn open_parts(&mut self) -> Result<(), E> {
        self.managed = Some(
            self.binding()?
                .directory()
                .child("managed", false)
                .map_err(E::Storage)?,
        );
        self.work = Some(
            self.managed
                .as_ref()
                .ok_or(E::Refused("paea-git-source-not-open"))?
                .child(&self.work_id, false)
                .map_err(E::Storage)?,
        );
        let work = self
            .work
            .as_ref()
            .ok_or(E::Refused("paea-git-source-not-open"))?;
        self.lane = Some(work.reserve(false, false).map_err(E::Storage)?);
        self.bare = Some(work.child("repository.git", false).map_err(E::Storage)?);
        Ok(())
    }
    /// Compare current physical ancestry and lock path to retained descriptors.
    /// Substitution refuses even if the old handles remain valid; no repair,
    /// alternate Root, copied DB or second lock is silently adopted.
    pub(super) fn check(&self, project: &RootStore) -> Result<(), E> {
        let binding = self.binding()?;
        if !binding.matches(project)? {
            return Err(E::Refused("paea-git-source-root-binding"));
        }
        material_path::verify_material_path(project, binding)?;
        inside(
            binding
                .directory()
                .child("managed", false)
                .map_err(E::Storage)?,
            |managed| {
                if managed.identity().map_err(E::Storage)?
                    != self
                        .managed
                        .as_ref()
                        .ok_or(E::Refused("paea-git-source-not-open"))?
                        .identity()
                        .map_err(E::Storage)?
                {
                    return Err(E::Refused("paea-git-source-directory-rebound"));
                }
                inside(
                    managed.child(&self.work_id, false).map_err(E::Storage)?,
                    |work| {
                        if work.identity().map_err(E::Storage)?
                            != self
                                .work
                                .as_ref()
                                .ok_or(E::Refused("paea-git-source-not-open"))?
                                .identity()
                                .map_err(E::Storage)?
                        {
                            return Err(E::Refused("paea-git-source-directory-rebound"));
                        }
                        let lane = self
                            .lane
                            .as_ref()
                            .ok_or(E::Refused("paea-git-source-not-open"))?;
                        if work
                            .regular_identity(".engine-settings.lock")
                            .map_err(E::Storage)?
                            != lane.identity().map_err(E::Storage)?
                        {
                            return Err(E::Refused("paea-git-source-lock-rebound"));
                        }
                        inside(
                            work.child("repository.git", false).map_err(E::Storage)?,
                            |bare| {
                                if bare.identity().map_err(E::Storage)?
                                    != self.bare()?.identity().map_err(E::Storage)?
                                {
                                    return Err(E::Refused("paea-git-source-directory-rebound"));
                                }
                                Ok(())
                            },
                        )
                    },
                )
            },
        )
    }
    /// Release all acquired handles and the shared lane with independent checked
    /// cleanup. No objects, files or references are removed. This method cannot
    /// establish process completion; the caller/job must resolve its child first.
    pub(super) fn close(mut self) -> Result<(), E> {
        let mut result = Ok(());
        if let Some(v) = self.bare.take() {
            result = finished(result, v.close().map_err(E::Storage));
        }
        if let Some(v) = self.lane.take() {
            result = finished(result, v.close().map_err(E::Storage));
        }
        if let Some(v) = self.work.take() {
            result = finished(result, v.close().map_err(E::Storage));
        }
        if let Some(v) = self.managed.take() {
            result = finished(result, v.close().map_err(E::Storage));
        }
        if let Some(v) = self.root.take() {
            result = finished(result, v.close());
        }
        result
    }
}
impl PaeaGitRepository {
    /// Retain the actual managed bare source under a shared managed lane after
    /// genuine current-candidate inspection. This legacy observation-first API
    /// is not the protected job acquisition entry. Requires the declared canonical
    /// managed storage path; all opens are descriptor-relative to the real Root.
    /// Call inspect after opening and before use: the saved observation alone is
    /// not a last-use barrier. No Git process starts after lane acquisition in this
    /// constructor, so partial-open failure owns no asynchronous dependency.
    /// No data write, pack, grant, intention or ref effect occurs. Metadata/lock/
    /// native-read and partial-close failures refuse; no copied source is adopted.
    pub fn retain_source(
        &self,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
    ) -> Result<PaeaGitSource, E> {
        let snapshot = self.inspect(service, project, proposal)?;
        let custody = SourceCustody::open(project, &snapshot.work_id)?;
        match PaeaGitSource::complete(custody, snapshot) {
            Ok(source) => Ok(source),
            Err((custody, e)) => finished(Err(e), custody.close()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::barrier_tests as fixture;
    use super::*;
    /// Actual native source uses the same writable Root connection, keeps its
    /// writer on nested-observer refusal, blocks an independent SQL writer, then
    /// recovers after rollback. No business SQL row or grant is inserted.
    #[test]
    #[ignore = "requires newly owned actual native fixture78"]
    fn source_actual_writer_reader_and_recovery() {
        let (path, root, work, oid) = fixture::fixture();
        let (dir, service) = fixture::fresh();
        let pin = "a".repeat(64);
        service
            .paea_initialize_git_intents("fixture-project", &pin)
            .unwrap();
        fixture::bare(dir.path(), "bare", "sha1");
        let binding = service
            .paea_provision_git_repository(&root, &work, "bare", &pin)
            .unwrap();
        let repository = service
            .paea_open_git_repository(&root, &work, "bare", &pin)
            .unwrap();
        let proposal = fixture::proposal(&binding, &oid);
        let source = repository
            .retain_source(&service, &root, &proposal)
            .unwrap();
        let current = source
            .inspect(&repository, &service, &root, &proposal)
            .unwrap();
        assert!(matches!(
            source.inspect_for_effect(&repository, &service, &root, &proposal),
            Err(E::Refused("managed-current-publication-writer-required"))
        ));
        let read =
            rusqlite::Transaction::new_unchecked(&root.db, rusqlite::TransactionBehavior::Deferred)
                .unwrap();
        let _: i64 = root
            .db
            .query_row("SELECT count(*) FROM works", [], |r| r.get(0))
            .unwrap();
        assert!(matches!(
            source.inspect_for_effect(&repository, &service, &root, &proposal),
            Err(E::Refused("managed-current-publication-writer-required"))
        ));
        read.rollback().unwrap();
        source
            .inspect(&repository, &service, &root, &proposal)
            .unwrap();
        let tx = rusqlite::Transaction::new_unchecked(
            &root.db,
            rusqlite::TransactionBehavior::Immediate,
        )
        .unwrap();
        assert_eq!(
            source
                .inspect_for_effect(&repository, &service, &root, &proposal)
                .unwrap(),
            current
        );
        assert!(source
            .inspect(&repository, &service, &root, &proposal)
            .is_err());
        root.require_native_git_writer().unwrap();
        assert_eq!(
            source
                .inspect_for_effect(&repository, &service, &root, &proposal)
                .unwrap(),
            current
        );
        let peer = rusqlite::Connection::open(path.join(".swarm/state.db")).unwrap();
        peer.busy_timeout(std::time::Duration::ZERO).unwrap();
        assert!(
            matches!(peer.execute_batch("BEGIN IMMEDIATE"),Err(rusqlite::Error::SqliteFailure(e,_)) if e.code==rusqlite::ErrorCode::DatabaseBusy)
        );
        tx.rollback().unwrap();
        peer.execute_batch("BEGIN IMMEDIATE; ROLLBACK").unwrap();
        peer.close().unwrap();
        assert_eq!(
            source
                .inspect(&repository, &service, &root, &proposal)
                .unwrap(),
            current
        );
        source.close().unwrap();
        repository.close().unwrap();
        service.close().unwrap();
        root.close().unwrap();
    }
}
