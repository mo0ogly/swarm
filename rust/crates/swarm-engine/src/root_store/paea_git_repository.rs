//! Explicit private Git resource declaration, retained custody and inspection.
//! Not a protected effect service: no object transfer, current grant or ref write.
use super::{
    finished, material_path, paea_git_intent, RootBinding, RootInstanceIdentity, RootStore,
    RootStoreError as E,
};
use crate::{
    local_storage::{AtomicStorageFault, LocalDirectory},
    CurrentManagedGitPublication,
};
use serde::Serialize;
use std::{error::Error, fmt};
use swarm_identity_contract::{digest, name};
use swarm_protection_contract::{Action, Proposal};

/// Immutable internal frame. Physical identities describe actual retained handles,
/// not OS confinement, historical authenticity or authority after a restart.
#[derive(Clone, PartialEq, Eq, Serialize)]
struct Record {
    schema_version: u32,
    repository_id: String,
    work_id: String,
    project_id: String,
    project_digest: String,
    repository_request_digest: String,
    service: RootInstanceIdentity,
    project: RootInstanceIdentity,
    directory_name: String,
    directory_identity: (u64, u64),
}
/// Non-authorizing public metadata. No raw path, FD, mutable Command or serde
/// constructor is exposed. Getters describe a binding, never an effect permit.
#[derive(Clone, PartialEq, Eq)]
pub struct PaeaGitRepositoryBinding {
    record: Record,
}
impl PaeaGitRepositoryBinding {
    /// Service-assigned physical resource key; compare exact Proposal Git ID.
    pub fn repository_id(&self) -> &str {
        &self.record.repository_id
    }
    /// Actual work whose managed-repository definition was observed.
    pub fn work_id(&self) -> &str {
        &self.record.work_id
    }
    /// Explicitly provisioned journal project name; not a human attestation.
    pub fn project_id(&self) -> &str {
        &self.record.project_id
    }
    /// Operator-supplied project pin checked against the existing journal.
    pub fn project_digest(&self) -> &str {
        &self.record.project_digest
    }
    /// Recorded managed-repository request digest, not an independent signature.
    pub fn repository_request_digest(&self) -> &str {
        &self.record.repository_request_digest
    }
}
impl fmt::Debug for PaeaGitRepositoryBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PaeaGitRepositoryBinding")
    }
}
/// Original Root/storage/process cause plus metadata actually known after a
/// successful publication and possible late failure. None means unobserved,
/// never absent: inspect typed local-publication uncertainty in the cause tree.
/// Display/Debug redact private paths, frame bytes and process streams.
pub struct PaeaGitRepositoryError {
    pub cause: E,
    known: Option<PaeaGitRepositoryBinding>,
}
impl PaeaGitRepositoryError {
    /// Fixed diagnostic family; original safe phase/codes remain in source().
    pub fn code(&self) -> &'static str {
        "paea-git-repository"
    }
    /// Historical known publication despite a later error. This grants no retry,
    /// usable current capability or claim that a Git ref was changed.
    pub fn known_binding(&self) -> Option<&PaeaGitRepositoryBinding> {
        self.known.as_ref()
    }
}
impl fmt::Display for PaeaGitRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for PaeaGitRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PaeaGitRepositoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}
/// Actual Git storage representation, not an operator limit or assurance level.
/// Both algorithms are already represented by the v1 proposal's 40/64 hex OIDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaeaGitObjectFormat {
    Sha1,
    Sha256,
}
/// Retained project/service witnesses and private bare directory. Keep alive
/// through any future owned process and explicitly close after refusal too.
/// This type exposes no raw command/path and grants no publication authority.
pub struct PaeaGitRepository {
    service: RootBinding,
    project: RootBinding,
    directory: LocalDirectory,
    binding: PaeaGitRepositoryBinding,
}
/// Canonical bytes use the existing deterministic engine encoder. No signing or
/// authority is inferred; exact expected bytes avoid accepting unknown fields.
fn bytes(record: &impl Serialize) -> Result<Vec<u8>, E> {
    crate::encode_model(record).map_err(E::Encode)
}
/// Derive the service resource key from actual Root/directory identities. Two
/// aliases of one retained directory in one service cannot mint distinct IDs.
/// This is physical namespace binding, not cross-host/backup authenticity.
fn resource(service: &RootInstanceIdentity, directory: (u64, u64)) -> Result<String, E> {
    let mut raw = b"paea-git-repository-v1\n".to_vec();
    raw.extend(bytes(&(service, directory))?);
    Ok(crate::provider_configuration::configuration_digest(&raw))
}
/// Private declaration name uses only a derived lowercase digest, never a path
/// supplied by an agent. This layout is a representation invariant, not policy.
fn filename(binding: &PaeaGitRepositoryBinding) -> String {
    format!("paea-git-repository-{}.json", binding.repository_id())
}
/// Observe actual managed repository metadata; no accepted state or grant is
/// created. A changed request digest invalidates the old binding on later use.
fn request_digest(project: &RootStore, work: &str) -> Result<String, E> {
    let work = project.read_work(work)?;
    let repository = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("paea-git-managed-repository-required"))?;
    if !digest(repository.request_digest.as_bytes()) {
        return Err(E::Refused("paea-git-managed-repository-digest"));
    }
    Ok(repository.request_digest.clone())
}
/// Release every independently acquired witness even after another close fails.
fn close_parts(
    directory: LocalDirectory,
    service: RootBinding,
    project: RootBinding,
) -> Result<(), E> {
    finished(
        directory.close().map_err(E::Storage),
        finished(service.close(), project.close()),
    )
}
impl PaeaGitRepository {
    /// Borrow safe immutable metadata, not an authority token or repository path.
    pub fn binding(&self) -> &PaeaGitRepositoryBinding {
        &self.binding
    }
    /// Revalidate physical Roots/child, journal pin and current managed definition.
    /// Path rebinding refuses even while the old directory FD stays alive. This
    /// observes custody now, not a file-content lease or same-UID confinement.
    fn check(&self, service: &RootStore, project: &RootStore) -> Result<(), E> {
        if !self.service.matches(service)? || !self.project.matches(project)? {
            return Err(E::Refused("paea-git-root-binding"));
        }
        material_path::verify_material_path(service, &self.service)?;
        material_path::verify_material_path(project, &self.project)?;
        let actual = service
            .directory
            .child(&self.binding.record.directory_name, false)
            .map_err(E::Storage)?;
        let matched = (|| {
            if actual.identity().map_err(E::Storage)?
                != self.directory.identity().map_err(E::Storage)?
            {
                return Err(E::Refused("paea-git-directory-rebound"));
            }
            Ok(())
        })();
        finished(matched, actual.close().map_err(E::Storage))?;
        let (project_id, pin) = paea_git_intent::meta(&service.db).map_err(|e| {
            E::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-journal",
                e,
            )))
        })?;
        if project_id != self.binding.record.project_id
            || pin != self.binding.record.project_digest
            || request_digest(project, self.binding.work_id())?
                != self.binding.record.repository_request_digest
        {
            return Err(E::Refused("paea-git-project-binding"));
        }
        Ok(())
    }
    /// Read exactly the expected canonical frame under the service descriptor.
    /// Its derived length is a format bound, never an operator policy cutoff.
    fn registered(&self, service: &RootStore) -> Result<(), E> {
        let expected = bytes(&self.binding.record)?;
        let actual = service
            .directory
            .read_bounded(&filename(&self.binding), expected.len())
            .map_err(E::Storage)?;
        if actual != expected {
            return Err(E::Refused("paea-git-declaration-binding"));
        }
        Ok(())
    }
    /// Inspect a Git proposal against this exact declared resource and the real
    /// latest native candidate/review/cohort. Check physical custody before/after;
    /// return only a current observation, not a last-use barrier or authority.
    /// Target direct type/OLD, object transfer, grants and Git COMMIT are NOT
    /// performed. A future effect service must revalidate under its own barrier.
    /// The capability remains owned by the caller after any refusal; close it.
    pub fn inspect(
        &self,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
    ) -> Result<CurrentManagedGitPublication, E> {
        self.inspect_using(service, project, proposal, || {
            project.current_managed_git_publication(self.binding.work_id())
        })
    }
    /// Same physical/declaration/candidate checks inside the actual Root writer.
    /// This consumes no authority and does not open/finish a transaction. Caller
    /// keeps all settings/Trust/plan/mandate reservations through a future effect.
    pub(super) fn inspect_for_effect(
        &self,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
    ) -> Result<CurrentManagedGitPublication, E> {
        project.require_native_git_writer()?;
        self.inspect_using(service, project, proposal, || {
            project.current_managed_git_publication_for_effect(self.binding.work_id())
        })
    }
    /// Common checks; the two private callers select real observer implementations,
    /// never a serialized prechecked boolean. Preserve independent post-check errors.
    fn inspect_using(
        &self,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
        observe: impl FnOnce() -> Result<CurrentManagedGitPublication, E>,
    ) -> Result<CurrentManagedGitPublication, E> {
        proposal.validate().map_err(|e| {
            E::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-proposal",
                e,
            )))
        })?;
        let candidate_oid = match &proposal.action {
            Action::Git {
                repository_id,
                candidate_oid,
                ..
            } if repository_id == self.binding.repository_id() => candidate_oid,
            _ => return Err(E::Refused("paea-git-proposal-repository")),
        };
        self.check(service, project)?;
        self.registered(service)?;
        let result = (|| {
            let observed = observe()?;
            if observed.candidate_commit != *candidate_oid
                || observed.repository_request_digest
                    != self.binding.record.repository_request_digest
            {
                return Err(E::Refused("paea-git-current-candidate"));
            }
            Ok(observed)
        })();
        finished(
            result,
            finished(self.check(service, project), self.registered(service)),
        )
    }
    /// Inspect actual private Git storage format and require compatibility with
    /// the actual reviewed candidate's OID representation. Wrong/unknown format or
    /// non-bare state refuses before object transfer, intent or ref mutation.
    /// This is still an observation, not proof of stored objects or a last-use
    /// authority barrier; the effect owner must recheck under its own reservations.
    pub fn inspect_object_format(
        &self,
        service: &RootStore,
        project: &RootStore,
        proposal: &Proposal,
    ) -> Result<PaeaGitObjectFormat, E> {
        self.inspect(service, project, proposal)?;
        let result = (|| {
            let settings = service.read_settings()?;
            let raw = crate::managed_repository::git(
                &self.directory.process_path(),
                &[
                    "--git-dir",
                    ".",
                    "rev-parse",
                    "--is-bare-repository",
                    "--show-object-format=storage",
                ],
                &settings,
            )
            .map_err(|e| E::Launch(Box::new(e)))?;
            let (format, extent) = match raw.as_slice() {
                b"true\nsha1\n" => (PaeaGitObjectFormat::Sha1, 40),
                b"true\nsha256\n" => (PaeaGitObjectFormat::Sha256, 64),
                _ => return Err(E::Refused("paea-git-object-format")),
            };
            let Action::Git { candidate_oid, .. } = &proposal.action else {
                return Err(E::Refused("paea-git-proposal-repository"));
            };
            if candidate_oid.len() != extent {
                return Err(E::Refused("paea-git-object-format-mismatch"));
            }
            Ok(format)
        })();
        finished(
            result,
            finished(self.check(service, project), self.registered(service)),
        )
    }
    /// Explicitly close the held directory and both Root witnesses. All close
    /// causes survive independently; this never deletes a declaration or ref.
    pub fn close(self) -> Result<(), E> {
        close_parts(self.directory, self.service, self.project)
    }
}
impl RootStore {
    /// Acquire actual resources without adopting a declaration. Requires an
    /// already provisioned Git journal, distinct project/service datastores,
    /// a private single-component bare child prepared by the trusted operator,
    /// and its exact journal project pin. No directories, grants or refs created.
    fn prepare_paea_git_repository(
        &self,
        project: &RootStore,
        work: &str,
        child: &str,
        pin: &str,
    ) -> Result<PaeaGitRepository, E> {
        if !name(work.as_bytes()) || !digest(pin.as_bytes()) {
            return Err(E::Refused("paea-git-binding-input"));
        }
        let service = self.bind_root()?;
        let project_binding = match project.bind_root() {
            Ok(value) => value,
            Err(e) => return finished(Err(e), service.close()),
        };
        let directory = match self.directory.child(child, false) {
            Ok(value) => value,
            Err(e) => {
                return finished(
                    Err(E::Storage(e)),
                    finished(service.close(), project_binding.close()),
                )
            }
        };
        let result = (|| {
            let service_id = service.instance()?;
            let project_id = project_binding.instance()?;
            let directory_id = directory.identity().map_err(E::Storage)?;
            if directory_id == service_id.directory
                || service_id.directory == project_id.directory
                || service_id.database == project_id.database
                || directory_id == project_id.directory
            {
                return Err(E::Refused("paea-git-separate-storage-required"));
            }
            let (project_name, expected_pin) = paea_git_intent::meta(&self.db).map_err(|e| {
                E::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-journal",
                    e,
                )))
            })?;
            if expected_pin != pin {
                return Err(E::Refused("paea-git-project-pin"));
            }
            let settings = self.read_settings()?;
            let path = directory.process_path();
            // The child first chdirs through OUR descriptor, then uses its OWN
            // retained cwd. Relative explicit git-dir prevents ancestor discovery
            // and never depends on a parent FD after an unreaped-process error.
            let raw = crate::managed_repository::git(
                &path,
                &["--git-dir", ".", "rev-parse", "--is-bare-repository"],
                &settings,
            )
            .map_err(|e| E::Launch(Box::new(e)))?;
            if raw != b"true\n" {
                return Err(E::Refused("paea-git-bare-required"));
            }
            let record = Record {
                schema_version: 1,
                repository_id: resource(&service_id, directory_id)?,
                work_id: work.into(),
                project_id: project_name,
                project_digest: pin.into(),
                repository_request_digest: request_digest(project, work)?,
                service: service_id,
                project: project_id,
                directory_name: child.into(),
                directory_identity: directory_id,
            };
            Ok(PaeaGitRepositoryBinding { record })
        })();
        match result {
            Ok(binding) => {
                let held = PaeaGitRepository {
                    service,
                    project: project_binding,
                    directory,
                    binding,
                };
                match held.check(self, project) {
                    Ok(()) => Ok(held),
                    Err(e) => finished(Err(e), held.close()),
                }
            }
            Err(e) => finished(Err(e), close_parts(directory, service, project_binding)),
        }
    }
    /// Trusted explicit provisioning, never an agent request. Publish an immutable
    /// canonical declaration with private atomic NOREPLACE semantics; existing
    /// names refuse unchanged. Pin/project/managed definition/bare custody must
    /// match. This does not authenticate the operator or establish UID isolation.
    /// On uncertainty inspect original storage state and explicitly open/read;
    /// never blindly publish again or delete a possibly visible declaration.
    pub fn paea_provision_git_repository(
        &self,
        project: &RootStore,
        work: &str,
        child: &str,
        pin: &str,
    ) -> Result<PaeaGitRepositoryBinding, PaeaGitRepositoryError> {
        let mut known = None;
        let result = (|| {
            if !self.writable {
                return Err(E::Refused("root-read-only"));
            }
            let held = self.prepare_paea_git_repository(project, work, child, pin)?;
            let result = (|| {
                held.check(self, project)?;
                self.directory
                    .publish_new_with_fault(
                        &filename(&held.binding),
                        &bytes(&held.binding.record)?,
                        AtomicStorageFault::None,
                    )
                    .map_err(E::Storage)?;
                known = Some(held.binding.clone());
                held.check(self, project)?;
                held.registered(self)?;
                Ok(held.binding.clone())
            })();
            finished(result, held.close())
        })();
        result.map_err(|cause| PaeaGitRepositoryError { cause, known })
    }
    /// Reopen an explicitly declared resource under fresh retained descriptors.
    /// Missing/altered frame, wrong physical root/pin/child/work or non-bare Git
    /// refuses; no first-request adoption, replacement, subprocess grant or ref
    /// mutation. Keep the returned capability alive and close it explicitly.
    pub fn paea_open_git_repository(
        &self,
        project: &RootStore,
        work: &str,
        child: &str,
        pin: &str,
    ) -> Result<PaeaGitRepository, PaeaGitRepositoryError> {
        let result = (|| {
            let held = self.prepare_paea_git_repository(project, work, child, pin)?;
            let checked = finished(held.registered(self), held.check(self, project));
            match checked {
                Ok(()) => Ok(held),
                Err(e) => finished(Err(e), held.close()),
            }
        })();
        result.map_err(|cause| PaeaGitRepositoryError { cause, known: None })
    }
}

#[cfg(test)]
#[path = "paea_git_repository/barrier_tests.rs"]
mod barrier_tests;
