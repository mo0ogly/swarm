//! Crate-owned object preparation. No business grant, ref commit or service API.
use super::{finished, PaeaGitRepository, PaeaGitSource, RootStore, E};
use crate::validation_run::has_unresolved_process;
use crate::{local_storage::pack_files::GitPackFiles, EngineSettingsDocument, GitPackOperation};
use std::{error::Error, fmt, io};
use swarm_protection_contract::Proposal;
/// Observed preparation only; cannot authorize a reference effect or retry.
pub(crate) struct TransferReceipt {
    pub candidate: String,
    pub pack_bytes: u64,
    pub pack_sha256: String,
}
/// Original error and exact attempted phase. Import may have changed object
/// storage before an error; no object rollback or zero effect is inferred.
pub(crate) struct TransferFailure {
    pub phase: &'static str,
    pub cause: Box<dyn Error + Send + Sync>,
}
impl fmt::Debug for TransferFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.phase)
    }
}
impl fmt::Display for TransferFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.phase)
    }
}
impl Error for TransferFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
/// Private typestate: physical custody precedes native observation. Vacant is
/// only a synchronous move boundary and always refuses, never a path fallback.
enum SourceState {
    Pending(super::source::SourceCustody),
    Ready(PaeaGitSource),
    Vacant,
}
impl SourceState {
    /// Borrow the same physical witnesses in either acquisition phase.
    fn custody(&self) -> Result<&super::source::SourceCustody, E> {
        match self {
            Self::Pending(c) => Ok(c),
            Self::Ready(s) => Ok(&s.custody),
            Self::Vacant => Err(E::Refused("paea-git-source-not-open")),
        }
    }
    /// Only an actual completed native observation exposes a current Source.
    fn ready(&self) -> Result<&PaeaGitSource, E> {
        match self {
            Self::Ready(s) => Ok(s),
            _ => Err(E::Refused("paea-git-source-not-observed")),
        }
    }
    /// Observe while Pending is already owned by the job. Native and postcheck
    /// failures leave that exact custody in place, including unresolved children
    /// retained by the caller's failure. A complete snapshot changes typestate.
    fn acquire(
        &mut self,
        destination: &PaeaGitRepository,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
    ) -> Result<(), E> {
        if matches!(self, Self::Ready(_)) {
            return Ok(());
        }
        let custody = self.custody()?;
        custody.check(project)?;
        let result = destination.inspect(service, project, proposal);
        let snapshot = finished(result, custody.check(project))?;
        let current = std::mem::replace(self, Self::Vacant);
        let Self::Pending(custody) = current else {
            *self = current;
            return Err(E::Refused("paea-git-source-acquisition-state"));
        };
        match PaeaGitSource::complete(custody, snapshot) {
            Ok(source) => {
                *self = Self::Ready(source);
                Ok(())
            }
            Err((custody, error)) => {
                *self = Self::Pending(custody);
                Err(error)
            }
        }
    }
    /// Checked-close the actual phase owner; never closes dependent processes.
    fn close(self) -> Result<(), E> {
        match self {
            Self::Pending(c) => c.close(),
            Self::Ready(s) => s.close(),
            Self::Vacant => Err(E::Refused("paea-git-source-acquisition-state")),
        }
    }
}
/// Every dependent capability is owned before any native observation/process.
/// Starts once, retains failures and partial files, and refuses close while the
/// native cause graph contains an unresolved/lost child. Drop is not confinement:
/// trusted callers must use checked close and preserve a refused returned owner.
pub(crate) struct GitTransferJob {
    source: SourceState,
    destination: PaeaGitRepository,
    files: GitPackFiles,
    started: bool,
    phase: &'static str,
    receipt: Option<TransferReceipt>,
    failure: Option<TransferFailure>,
    recovery_errors: Vec<io::Error>,
}
/// Closed job keeps the original failure, actual receipt and independent cleanup
/// causes. A prepared receipt plus close failure is not an empty failed transfer.
pub(crate) struct ClosedTransfer {
    pub receipt: Option<TransferReceipt>,
    pub failure: Option<TransferFailure>,
    pub cleanup: Result<(), E>,
    pub recovery_errors: Vec<io::Error>,
}
impl GitTransferJob {
    /// Test-only compatibility ownership transfer for earlier ready-source cases.
    /// Production construction uses from_repository, anchoring staging and source
    /// custody before native observation. Test files must be newly created through
    /// GitPackFiles. No path, caller-prechecked boolean or process is accepted.
    #[cfg(test)]
    pub(crate) fn new(
        source: PaeaGitSource,
        destination: PaeaGitRepository,
        files: GitPackFiles,
    ) -> Self {
        Self {
            source: SourceState::Ready(source),
            destination,
            files,
            started: false,
            phase: "created",
            receipt: None,
            failure: None,
            recovery_errors: vec![],
        }
    }
    /// Open a job's source witnesses and strict new staging before any Git.
    /// Staging is anchored to the actual held service Root after full physical/
    /// declaration checks, not an arbitrary directory passed by the caller.
    /// A read-only service Root refuses before storage creation. Pure preflight
    /// errors checked-close all acquired owners; a failed mkdir
    /// may leave artifacts under the supplied validated name. No child exists yet.
    pub(crate) fn from_repository(
        destination: PaeaGitRepository,
        service: &RootStore,
        project: &RootStore,
        staging: &str,
    ) -> Result<Self, E> {
        if !service.writable {
            return finished(Err(E::Refused("root-read-only")), destination.close());
        }
        let result = (|| {
            destination.check(service, project)?;
            destination.registered(service)?;
            super::source::SourceCustody::open(project, destination.binding().work_id())
        })();
        let custody = match result {
            Ok(c) => c,
            Err(e) => return finished(Err(e), destination.close()),
        };
        let files = match GitPackFiles::create(destination.service.directory(), staging) {
            Ok(files) => files,
            Err(e) => {
                return finished(
                    Err(E::Storage(e)),
                    finished(custody.close(), destination.close()),
                )
            }
        };
        Ok(Self {
            source: SourceState::Pending(custody),
            destination,
            files,
            started: false,
            phase: "created",
            receipt: None,
            failure: None,
            recovery_errors: vec![],
        })
    }
    /// Read the observed receipt only after complete preparation. No current grant
    /// or source freshness after this observation can be inferred from its bytes.
    pub(crate) fn receipt(&self) -> Option<&TransferReceipt> {
        self.receipt.as_ref()
    }
    /// Keep the original error available even after attempted process recovery.
    pub(crate) fn failure(&self) -> Option<&TransferFailure> {
        self.failure.as_ref()
    }
    /// Execute once with a writable service Root, real source/destination
    /// observations and saved settings. Read-only mode refuses before any Git.
    /// Export/import stream to held private files; exact candidate is verified and
    /// destination refs must stay equal. Failure remains in this owner, even after
    /// partial import. A second invocation refuses without replacing that cause.
    pub(crate) fn run_once(
        &mut self,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
    ) -> Result<(), &'static str> {
        if self.started {
            return Err("paea-git-transfer-already-started");
        }
        self.started = true;
        match self.prepare(service, project, proposal) {
            Ok(receipt) => {
                self.receipt = Some(receipt);
                self.phase = "prepared";
                Ok(())
            }
            Err(cause) => {
                self.failure = Some(TransferFailure {
                    phase: self.phase,
                    cause,
                });
                Err("paea-git-transfer-failed")
            }
        }
    }
    /// Fixed hardened Git metadata commands; pack bytes never use Vec.
    /// Ref enumeration uses the existing runner Vec and has no new volume cap;
    /// service resource confinement remains a prerequisite for public exposure.
    /// Caller retains directory/job and a process failure in the enclosing owner.
    fn git(&self, args: &[&str], settings: &EngineSettingsDocument) -> Result<Vec<u8>, E> {
        crate::managed_repository::git(&self.destination.directory.process_path(), args, settings)
            .map_err(|e| E::Launch(Box::new(e)))
    }
    /// Shared one-shot preparation body. Commands request no business SQL or reference update;
    /// import object writes are intentionally retained on all later failures.
    /// The settings read below is deliberately unreserved: git command settings
    /// may change concurrently with this transfer. Reflected staleness is limited
    /// to command deadline/env for this one job; no ref/grant effect follows,
    /// and publication stays atomic. Holding a shared reservation for the whole
    /// transfer is a possible hardening owned by the settings consumers.
    fn prepare(
        &mut self,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
    ) -> Result<TransferReceipt, Box<dyn Error + Send + Sync>> {
        self.phase = "preflight";
        if !service.writable {
            return Err(Box::new(E::Refused("root-read-only")));
        }
        self.phase = "acquiring";
        self.source
            .acquire(&self.destination, service, project, proposal)?;
        self.phase = "checking";
        self.source
            .ready()?
            .inspect(&self.destination, service, project, proposal)?;
        if self.source.ready()?.bare()?.identity()? == self.destination.directory.identity()? {
            return Err(Box::new(E::Refused(
                "paea-git-transfer-source-destination-alias",
            )));
        }
        let format = self
            .destination
            .inspect_object_format(service, project, proposal)?;
        let settings = service.read_settings()?;
        let actual = crate::managed_repository::git(
            &self.source.ready()?.bare()?.process_path(),
            &[
                "--git-dir",
                ".",
                "rev-parse",
                "--show-object-format=storage",
            ],
            &settings,
        )?;
        let expected = match format {
            super::PaeaGitObjectFormat::Sha1 => b"sha1\n".as_slice(),
            super::PaeaGitObjectFormat::Sha256 => b"sha256\n".as_slice(),
        };
        if actual != expected {
            return Err(Box::new(E::Refused("paea-git-transfer-source-format")));
        }
        let before = self.git(
            &[
                "--git-dir",
                ".",
                "for-each-ref",
                "--format=%(refname) %(objectname)",
            ],
            &settings,
        )?;
        let candidate = self.source.ready()?.candidate_oid().to_owned();
        self.files
            .write_revision(format!("{candidate}\n").as_bytes())?;
        self.phase = "export";
        {
            let (input, out, err) = self.files.export_streams()?;
            crate::run_git_pack(
                &self.source.ready()?.bare()?.process_path(),
                GitPackOperation::Export,
                input.try_clone()?,
                out,
                err,
                &settings,
            )?;
        }
        let (pack_bytes, pack_sha256) = self.files.pack_identity()?;
        self.files.prepare_import()?;
        self.phase = "import";
        {
            let (input, out, err) = self.files.import_streams()?;
            crate::run_git_pack(
                &self.destination.directory.process_path(),
                GitPackOperation::Import,
                input.try_clone()?,
                out,
                err,
                &settings,
            )?;
        }
        self.phase = "postcheck";
        if self.git(&["--git-dir", ".", "cat-file", "-t", &candidate], &settings)? != b"commit\n" {
            return Err(Box::new(E::Refused("paea-git-transfer-candidate-type")));
        }
        if self.git(
            &[
                "--git-dir",
                ".",
                "for-each-ref",
                "--format=%(refname) %(objectname)",
            ],
            &settings,
        )? != before
        {
            return Err(Box::new(E::Refused("paea-git-transfer-refs-changed")));
        }
        self.source
            .ready()?
            .inspect(&self.destination, service, project, proposal)?;
        Ok(TransferReceipt {
            candidate,
            pack_bytes,
            pack_sha256,
        })
    }
    /// Nonblocking try_wait on actual children retained by supported native wrappers.
    /// No command retry or numeric PID signaling. Each recovery IO failure is kept;
    /// unsupported wrappers/lost ownership still block close via the full graph.
    pub(crate) fn poll_recovery(&mut self) {
        if let Some(error) = self.failure.as_mut() {
            self.recovery_errors
                .extend(crate::validation_run::poll_retained_processes(
                    error.cause.as_mut(),
                ));
        }
    }
    /// Close all three independent owners only after known process ownership is
    /// resolved. On refusal return the entire job, preserving FDs/children/causes.
    /// Checked cleanup never deletes partial objects, files or a durable intention.
    pub(crate) fn close(self) -> Result<ClosedTransfer, Box<Self>> {
        if self
            .failure
            .as_ref()
            .is_some_and(|e| has_unresolved_process(e))
        {
            return Err(Box::new(self));
        }
        let Self {
            source,
            destination,
            files,
            receipt,
            failure,
            recovery_errors,
            ..
        } = self;
        let cleanup = finished(
            files.close().map_err(E::Storage),
            finished(source.close(), destination.close()),
        );
        Ok(ClosedTransfer {
            receipt,
            failure,
            cleanup,
            recovery_errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::barrier_tests as fixture;
    use super::*;
    /// Genuine historical publication is read only; service, files and imported
    /// objects are newly owned fixtures. No grant/accepted SQL state is fabricated.
    fn setup() -> (
        tempfile::TempDir,
        RootStore,
        RootStore,
        String,
        Proposal,
        GitTransferJob,
    ) {
        let (_path, project, work, oid) = fixture::fixture();
        let (dir, service) = fixture::fresh();
        let pin = "a".repeat(64);
        service
            .paea_initialize_git_intents("fixture-project", &pin)
            .unwrap();
        fixture::bare(dir.path(), "bare", "sha1");
        let binding = service
            .paea_provision_git_repository(&project, &work, "bare", &pin)
            .unwrap();
        let proposal = fixture::proposal(&binding, &oid);
        let destination = service
            .paea_open_git_repository(&project, &work, "bare", &pin)
            .unwrap();
        let source = destination
            .retain_source(&service, &project, &proposal)
            .unwrap();
        let files = GitPackFiles::create(&service.directory, "pack-job").unwrap();
        (
            dir,
            service,
            project,
            work,
            proposal,
            GitTransferJob::new(source, destination, files),
        )
    }
    /// Real native candidate travels through the owned job into its exact declared
    /// destination, with receipt/digest/size and no references or project Work change.
    #[test]
    #[ignore = "requires retained genuine native publication fixture"]
    fn owned_transfer_real_candidate_objects_and_no_ref() {
        let (dir, service, project, work, proposal, mut job) = setup();
        let before = project.read_work(&work).unwrap();
        job.run_once(&service, &project, &proposal).unwrap();
        let settings = service.read_settings().unwrap();
        assert!(job
            .git(&["--git-dir", ".", "for-each-ref"], &settings)
            .unwrap()
            .is_empty());
        let raw = std::fs::read(dir.path().join(".swarm/pack-job/candidate.pack")).unwrap();
        let receipt = job.receipt().unwrap();
        assert!(receipt.pack_bytes > 0);
        assert_eq!(receipt.pack_sha256.len(), 64);
        use sha2::Digest;
        assert_eq!(receipt.pack_bytes, raw.len() as u64);
        assert_eq!(
            receipt.pack_sha256,
            format!("{:x}", sha2::Sha256::digest(&raw))
        );
        assert!(raw.starts_with(b"PACK"));
        let swarm_protection_contract::Action::Git { candidate_oid, .. } = &proposal.action else {
            panic!("git proposal")
        };
        assert_eq!(&receipt.candidate, candidate_oid);
        assert_eq!(
            job.git(
                &["--git-dir", ".", "cat-file", "-t", candidate_oid],
                &settings
            )
            .unwrap(),
            b"commit\n"
        );
        assert!(job.failure().is_none());
        assert_eq!(
            job.run_once(&service, &project, &proposal),
            Err("paea-git-transfer-already-started")
        );
        let closed = job
            .close()
            .unwrap_or_else(|_| panic!("unexpected unresolved process"));
        assert!(closed.failure.is_none());
        assert!(closed.receipt.is_some());
        assert!(closed.recovery_errors.is_empty());
        closed.cleanup.unwrap();
        assert_eq!(project.read_work(&work).unwrap(), before);
        assert!(
            dir.path()
                .join(".swarm/pack-job/candidate.pack")
                .metadata()
                .unwrap()
                .len()
                > 0
        );
        service.close().unwrap();
        project.close().unwrap();
    }
    /// Wrong candidate refuses before export, retains the shared source lane and
    /// empty artifacts/cause, refuses replay; a newly created valid job recovers.
    #[test]
    #[ignore = "requires retained genuine native publication fixture"]
    fn owned_transfer_refusal_new_job_recovery() {
        let (_dir, service, project, work, proposal, mut job) = setup();
        let mut bad = proposal.clone();
        let swarm_protection_contract::Action::Git { candidate_oid, .. } = &mut bad.action else {
            panic!("git proposal")
        };
        *candidate_oid = "f".repeat(candidate_oid.len());
        assert_eq!(
            job.run_once(&service, &project, &bad),
            Err("paea-git-transfer-failed")
        );
        let phase = job.failure().unwrap().phase;
        assert_eq!(phase, "checking");
        assert!(job.receipt().is_none());
        let managed = project.directory.child("managed", false).unwrap();
        let held = managed.child(&work, false).unwrap();
        assert!(held.reserve(true, false).is_err());
        assert_eq!(
            job.run_once(&service, &project, &proposal),
            Err("paea-git-transfer-already-started")
        );
        assert_eq!(job.failure().unwrap().phase, phase);
        let closed = job
            .close()
            .unwrap_or_else(|_| panic!("unexpected unresolved process"));
        assert!(closed.failure.is_some());
        assert!(closed.receipt.is_none());
        closed.cleanup.unwrap();
        held.reserve(true, false).unwrap().close().unwrap();
        held.close().unwrap();
        managed.close().unwrap();
        let pin = "a".repeat(64);
        let destination = service
            .paea_open_git_repository(&project, &work, "bare", &pin)
            .unwrap();
        let source = destination
            .retain_source(&service, &project, &proposal)
            .unwrap();
        let files = GitPackFiles::create(&service.directory, "recovered-job").unwrap();
        let mut recovered = GitTransferJob::new(source, destination, files);
        recovered.run_once(&service, &project, &proposal).unwrap();
        let closed = recovered
            .close()
            .unwrap_or_else(|_| panic!("unexpected unresolved process"));
        assert!(closed.receipt.is_some());
        closed.cleanup.unwrap();
        service.close().unwrap();
        project.close().unwrap();
    }
    /// Synthetic error attachment with a real Child tests the owner's gate, not a
    /// naturally injected runner failure: close returns every capability until reap.
    #[test]
    #[ignore = "requires retained genuine native publication fixture"]
    fn owned_transfer_unreaped_close_returns_owner_then_recovers() {
        let (_dir, service, project, _work, _proposal, mut job) = setup();
        job.started = true;
        job.phase = "export";
        job.failure = Some(TransferFailure {
            phase: "export",
            cause: Box::new(crate::validation_run::retained_child_for_owner_test()),
        });
        let mut job = match job.close() {
            Err(job) => job,
            Ok(_) => panic!("released unreaped dependency"),
        };
        assert!(job
            .source
            .custody()
            .unwrap()
            .bare()
            .unwrap()
            .identity()
            .is_ok());
        assert!(job.destination.directory.identity().is_ok());
        while job.failure().is_some_and(|e| has_unresolved_process(e)) {
            job.poll_recovery();
            assert!(job.recovery_errors.is_empty());
            std::thread::yield_now();
        }
        let closed = job
            .close()
            .unwrap_or_else(|_| panic!("reaped owner should close"));
        assert!(closed.receipt.is_none());
        assert!(closed.failure.is_some());
        closed.cleanup.unwrap();
        service.close().unwrap();
        project.close().unwrap();
    }
    /// New disposable service and genuine read-only source, without first calling
    /// the legacy observation-first retain_source. Builder itself starts no Git.
    fn pending_setup(
        staging: &str,
    ) -> (
        tempfile::TempDir,
        RootStore,
        RootStore,
        String,
        Proposal,
        GitTransferJob,
    ) {
        let (_path, project, work, oid) = fixture::fixture();
        let (dir, service) = fixture::fresh();
        let pin = "a".repeat(64);
        service
            .paea_initialize_git_intents("fixture-project", &pin)
            .unwrap();
        fixture::bare(dir.path(), "bare", "sha1");
        let binding = service
            .paea_provision_git_repository(&project, &work, "bare", &pin)
            .unwrap();
        let proposal = fixture::proposal(&binding, &oid);
        let job = service
            .paea_open_git_transfer_job(&project, &work, "bare", &pin, staging)
            .unwrap();
        (dir, service, project, work, proposal, job)
    }
    /// Physical custody/lane and actual service-root staging precede native
    /// observation. A real current candidate completes typestate and import.
    #[test]
    #[ignore = "requires retained genuine native publication fixture"]
    fn pending_builder_acquires_native_source_and_service_staging() {
        let (dir, service, project, work, proposal, mut job) = pending_setup("pending-pack");
        assert!(matches!(job.source, SourceState::Pending(_)));
        assert!(job.source.ready().is_err());
        assert!(dir
            .path()
            .join(".swarm/pending-pack/candidate.pack")
            .is_file());
        let managed = project.directory.child("managed", false).unwrap();
        let lane = managed.child(&work, false).unwrap();
        assert!(lane.reserve(true, false).is_err());
        let before = project.read_work(&work).unwrap();
        job.run_once(&service, &project, &proposal).unwrap();
        assert!(matches!(job.source, SourceState::Ready(_)));
        assert!(job.receipt().unwrap().pack_bytes > 0);
        let settings = service.read_settings().unwrap();
        assert!(job
            .git(&["--git-dir", ".", "for-each-ref"], &settings)
            .unwrap()
            .is_empty());
        let candidate = &job.receipt().unwrap().candidate;
        assert_eq!(
            job.git(&["--git-dir", ".", "cat-file", "-t", candidate], &settings)
                .unwrap(),
            b"commit\n"
        );
        let closed = job
            .close()
            .unwrap_or_else(|_| panic!("unexpected unreaped source"));
        assert!(closed.receipt.is_some());
        closed.cleanup.unwrap();
        lane.reserve(true, false).unwrap().close().unwrap();
        lane.close().unwrap();
        managed.close().unwrap();
        assert_eq!(project.read_work(&work).unwrap(), before);
        service.close().unwrap();
        project.close().unwrap();
    }
    /// Actual refused initial native observation leaves Pending custody and lane
    /// in the failed job. No replay; close and a newly owned valid job recover.
    #[test]
    #[ignore = "requires retained genuine native publication fixture"]
    fn pending_initial_refusal_keeps_custody_and_new_job_recovers() {
        let (_dir, service, project, work, proposal, mut job) = pending_setup("pending-failed");
        let mut bad = proposal.clone();
        let swarm_protection_contract::Action::Git { candidate_oid, .. } = &mut bad.action else {
            panic!("git proposal")
        };
        *candidate_oid = "f".repeat(candidate_oid.len());
        assert_eq!(
            job.run_once(&service, &project, &bad),
            Err("paea-git-transfer-failed")
        );
        assert_eq!(job.failure().unwrap().phase, "acquiring");
        assert!(matches!(job.source, SourceState::Pending(_)));
        assert!(job.receipt().is_none());
        let managed = project.directory.child("managed", false).unwrap();
        let lane = managed.child(&work, false).unwrap();
        assert!(lane.reserve(true, false).is_err());
        assert_eq!(
            job.run_once(&service, &project, &proposal),
            Err("paea-git-transfer-already-started")
        );
        let closed = job
            .close()
            .unwrap_or_else(|_| panic!("unexpected unreaped source"));
        assert!(closed.failure.is_some());
        closed.cleanup.unwrap();
        lane.reserve(true, false).unwrap().close().unwrap();
        lane.close().unwrap();
        managed.close().unwrap();
        let destination = service
            .paea_open_git_repository(&project, &work, "bare", &"a".repeat(64))
            .unwrap();
        let mut recovered =
            GitTransferJob::from_repository(destination, &service, &project, "new-pending")
                .unwrap();
        recovered.run_once(&service, &project, &proposal).unwrap();
        let closed = recovered
            .close()
            .unwrap_or_else(|_| panic!("unexpected unreaped source"));
        assert!(closed.receipt.is_some());
        closed.cleanup.unwrap();
        service.close().unwrap();
        project.close().unwrap();
    }
    /// Real Child attached to a synthetic initial-acquisition error proves the
    /// Pending phase also returns its whole owner on close until kernel reap.
    #[test]
    #[ignore = "requires retained genuine native publication fixture"]
    fn pending_unreaped_initial_failure_retains_all_owners() {
        let (_dir, service, project, work, _proposal, mut job) = pending_setup("pending-unreaped");
        job.started = true;
        job.failure = Some(TransferFailure {
            phase: "acquiring",
            cause: Box::new(crate::validation_run::retained_child_for_owner_test()),
        });
        let mut job = match job.close() {
            Err(job) => job,
            Ok(_) => panic!("released pending source before reap"),
        };
        assert!(matches!(job.source, SourceState::Pending(_)));
        job.source.custody().unwrap().check(&project).unwrap();
        let managed = project.directory.child("managed", false).unwrap();
        let lane = managed.child(&work, false).unwrap();
        assert!(lane.reserve(true, false).is_err());
        while job.failure().is_some_and(|e| has_unresolved_process(e)) {
            job.poll_recovery();
            assert!(job.recovery_errors.is_empty());
            std::thread::yield_now();
        }
        let closed = job
            .close()
            .unwrap_or_else(|_| panic!("reaped pending source should close"));
        assert!(closed.failure.is_some());
        assert!(closed.receipt.is_none());
        closed.cleanup.unwrap();
        lane.reserve(true, false).unwrap().close().unwrap();
        lane.close().unwrap();
        managed.close().unwrap();
        service.close().unwrap();
        project.close().unwrap();
    }
    /// Collision preserves original staging bytes and releases the preflight lane.
    /// An unrelated service Root refuses before creating any new staging path.
    #[test]
    #[ignore = "requires retained genuine native publication fixture"]
    fn pending_builder_collision_and_wrong_service_refuse_without_adoption() {
        let (dir, service, project, work, _proposal, job) = pending_setup("occupied");
        let closed = job
            .close()
            .unwrap_or_else(|_| panic!("pure preflight close"));
        closed.cleanup.unwrap();
        let marker = dir.path().join(".swarm/occupied/candidate.pack");
        std::fs::write(&marker, b"original").unwrap();
        let destination = service
            .paea_open_git_repository(&project, &work, "bare", &"a".repeat(64))
            .unwrap();
        assert!(
            GitTransferJob::from_repository(destination, &service, &project, "occupied").is_err()
        );
        assert_eq!(std::fs::read(&marker).unwrap(), b"original");
        let managed = project.directory.child("managed", false).unwrap();
        let lane = managed.child(&work, false).unwrap();
        lane.reserve(true, false).unwrap().close().unwrap();
        lane.close().unwrap();
        managed.close().unwrap();
        let (other_dir, other) = fixture::fresh();
        let destination = service
            .paea_open_git_repository(&project, &work, "bare", &"a".repeat(64))
            .unwrap();
        let result =
            GitTransferJob::from_repository(destination, &other, &project, "wrong-service");
        match result {
            Err(E::Refused("paea-git-root-binding")) => {}
            _ => panic!("wrong service must refuse"),
        };
        assert!(!other_dir.path().join(".swarm/wrong-service").exists());
        assert!(!dir.path().join(".swarm/wrong-service").exists());
        other.close().unwrap();
        service.close().unwrap();
        project.close().unwrap();
    }
    /// A declaration can survive a malformed bare. The normal Root factory must
    /// still construct Pending without launching Git; the job refuses the actual
    /// native bare check before pack writes, keeps failure, then a new job recovers.
    #[test]
    #[ignore = "requires retained genuine native publication fixture"]
    fn pending_root_factory_defers_malformed_bare_to_owned_job() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, service, project, work, proposal, job) = pending_setup("valid-birth");
        let closed = job.close().unwrap_or_else(|_| panic!("pure preflight"));
        closed.cleanup.unwrap();
        let head = dir.path().join(".swarm/bare/HEAD");
        let original = std::fs::read(&head).unwrap();
        let mode = std::fs::metadata(&head).unwrap().permissions().mode();
        std::fs::remove_file(&head).unwrap();
        let pending = service.paea_open_git_transfer_job(
            &project,
            &work,
            "bare",
            &"a".repeat(64),
            "malformed-owned",
        );
        let mut job = match pending {
            Ok(job) => job,
            Err(e) => {
                std::fs::write(&head, &original).unwrap();
                std::fs::set_permissions(&head, std::fs::Permissions::from_mode(mode)).unwrap();
                panic!("FS-only factory unexpectedly required Git: {e}")
            }
        };
        assert!(matches!(job.source, SourceState::Pending(_)));
        let observed = job.run_once(&service, &project, &proposal);
        std::fs::write(&head, &original).unwrap();
        std::fs::set_permissions(&head, std::fs::Permissions::from_mode(mode)).unwrap();
        assert_eq!(observed, Err("paea-git-transfer-failed"));
        assert!(job.receipt().is_none());
        assert_eq!(job.failure().unwrap().phase, "checking");
        assert_eq!(
            dir.path()
                .join(".swarm/malformed-owned/candidate.pack")
                .metadata()
                .unwrap()
                .len(),
            0
        );
        let closed = job
            .close()
            .unwrap_or_else(|_| panic!("unexpected unreaped bare check"));
        assert!(closed.failure.is_some());
        closed.cleanup.unwrap();
        let mut recovered = service
            .paea_open_git_transfer_job(&project, &work, "bare", &"a".repeat(64), "bare-restored")
            .unwrap();
        recovered.run_once(&service, &project, &proposal).unwrap();
        let closed = recovered
            .close()
            .unwrap_or_else(|_| panic!("unexpected unreaped recovery"));
        assert!(closed.receipt.is_some());
        closed.cleanup.unwrap();
        service.close().unwrap();
        project.close().unwrap();
    }
    /// A real read-only Root must not create service staging or execute an
    /// already-created writable job. Control succeeds first on the same service;
    /// all owners close before checking the observed refusal, even in the red run.
    #[test]
    #[ignore = "requires retained genuine native publication fixture"]
    fn pending_factory_and_run_refuse_read_only_service() {
        let (dir, service, project, work, proposal, mut control) = pending_setup("mode-control");
        control.run_once(&service, &project, &proposal).unwrap();
        let closed = control
            .close()
            .unwrap_or_else(|_| panic!("control unresolved"));
        assert!(closed.receipt.is_some());
        closed.cleanup.unwrap();
        let readonly = RootStore::open(dir.path(), false).unwrap();
        let constructor_rejected = match readonly.paea_open_git_transfer_job(
            &project,
            &work,
            "bare",
            &"a".repeat(64),
            "readonly-constructor",
        ) {
            Err(E::Refused("root-read-only")) => true,
            Err(_) => false,
            Ok(job) => {
                let closed = job
                    .close()
                    .unwrap_or_else(|_| panic!("pure constructor unresolved"));
                closed.cleanup.unwrap();
                false
            }
        };
        let staging_created = dir.path().join(".swarm/readonly-constructor").exists();
        let mut job = service
            .paea_open_git_transfer_job(&project, &work, "bare", &"a".repeat(64), "readonly-run")
            .unwrap();
        let result = job.run_once(&readonly, &project, &proposal);
        let run_rejected = result.is_err()
            && job.failure().is_some_and(|e| {
                matches!(
                    e.cause.downcast_ref::<E>(),
                    Some(E::Refused("root-read-only"))
                )
            });
        let closed = job
            .close()
            .unwrap_or_else(|_| panic!("read-only attempt unresolved"));
        closed.cleanup.unwrap();
        readonly.close().unwrap();
        service.close().unwrap();
        project.close().unwrap();
        eprintln!("READONLY_OBSERVED constructor_rejected={constructor_rejected} staging_created={staging_created} run_rejected={run_rejected}");
        assert!(
            constructor_rejected && !staging_created && run_rejected,
            "read-only service must refuse before staging/native execution"
        );
    }
}
