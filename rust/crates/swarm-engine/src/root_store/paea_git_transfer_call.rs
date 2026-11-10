//! Guarded protected consumption of the crate-owned Git transfer job.
//! Authority and admission checks mirror the established settlement call shape;
//! the native preparation itself performs no reference effect. This entry never
//! grants permission, provisions storage or publishes a ref: the exact signed
//! `repository.promote` reconcile permission is consumed for one observed run.
//! Commit is refused because no attributed-admission path exists yet; exposing
//! this observation is not an accepted service and establishes no OS isolation,
//! upstream admission or independent proof.
use super::{finished, RootBinding, RootStore, RootStoreError as R};
use std::{convert::Infallible, error::Error, fmt};
use swarm_enrollment_authority::{
    authenticated_control::{AuthenticatedControl, Context},
    paea_action::{Identity, Operation, Request, Service, Use},
    personal::coordinator::UseError,
};
use swarm_enrollment_state::Head;
use swarm_protection_contract::{Action, AdmissionContext, Proposal};

/// All authority and preparation branches, including a known receipt after
/// failure. No variant implies rollback. Formatting is fixed and excludes
/// credentials, canonical action, receipts and private nested diagnostics.
pub enum PaeaGitTransferCallError {
    Personal(UseError<Use<PaeaGitTransferReceipt, R>, Infallible>),
    Authority(Use<PaeaGitTransferReceipt, R>),
    Action(R),
    MissingAction,
}
impl PaeaGitTransferCallError {
    /// Complete callback outcome when the held operation reached the backend.
    fn action(&self) -> Option<&Result<PaeaGitTransferReceipt, R>> {
        match self {
            Self::Personal(UseError::AfterUse {
                outcome: Ok(used), ..
            })
            | Self::Authority(used) => used.action.as_ref(),
            _ => None,
        }
    }
    /// Borrow the backend error without discarding primary/authority causes.
    pub fn backend_error(&self) -> Option<&R> {
        match self {
            Self::Action(e) => Some(e),
            _ => self.action().and_then(|a| a.as_ref().err()),
        }
    }
    /// Known prepared receipt despite a later authority failure. This is an
    /// observation of completed native preparation; no ref changed and no new
    /// run is implied. Availability never restores expired/revoked authority.
    pub fn known_receipt(&self) -> Option<&PaeaGitTransferReceipt> {
        self.action().and_then(|a| a.as_ref().ok())
    }
    /// Independent personal, business and backend causes. Standard source()
    /// supplies the first only; consumers must inspect all branches.
    pub fn branches(&self) -> Vec<&(dyn Error + 'static)> {
        let mut out: Vec<&(dyn Error + 'static)> = Vec::new();
        match self {
            Self::Personal(error) => {
                out.push(error);
                if let UseError::AfterUse {
                    outcome: Ok(used), ..
                } = error
                {
                    if let Err(cause) = &used.authority {
                        out.push(cause);
                    }
                }
            }
            Self::Authority(used) => {
                if let Err(cause) = &used.authority {
                    out.push(cause);
                }
            }
            Self::Action(_) | Self::MissingAction => (),
        }
        if let Some(error) = self.backend_error() {
            out.push(error);
        }
        out
    }
    /// Find the retained typed envelope through the existing Box/IO/Root branch
    /// walker; no message parsing, downcast fallback or invented receipt.
    /// Returns the first envelope in branch order, without merging multiple
    /// independent calls; None establishes neither absence of effects nor a
    /// successful run.
    pub fn find<'a>(error: &'a (dyn Error + 'static)) -> Option<&'a Self> {
        let mut pending = vec![error];
        let mut seen: Vec<*const dyn Error> = Vec::new();
        while let Some(error) = pending.pop() {
            let pointer = error as *const dyn Error;
            if seen.iter().any(|old| std::ptr::eq(*old, pointer)) {
                continue;
            }
            seen.push(pointer);
            if let Some(found) = error.downcast_ref::<Self>() {
                return Some(found);
            }
            pending.extend(
                crate::native_error_links(error)
                    .children
                    .into_iter()
                    .rev()
                    .map(|(_, cause)| cause),
            );
        }
        None
    }
    /// Fixed safe diagnostic, never a receipt, a password or nested SQL text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Personal(_) => "paea-git-transfer-personal",
            Self::Authority(_) => "paea-git-transfer-authority",
            Self::Action(_) => "paea-git-transfer-action",
            Self::MissingAction => "paea-git-transfer-action-missing",
        }
    }
}
impl fmt::Display for PaeaGitTransferCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for PaeaGitTransferCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PaeaGitTransferCallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.branches().into_iter().next()
    }
}
/// Frozen public summary of one completed crate-owned preparation. Byte counts
/// and digests describe the actual held private files and verified candidate at
/// preparation time; they prove no current grant, source freshness, ref effect
/// or later usability. No raw path, FD, capability or process identity exists.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PaeaGitTransferReceipt {
    pub candidate: String,
    pub pack_bytes: u64,
    pub pack_sha256: String,
}
/// Frozen receipt conversion from the crate-owned observed type.
fn receipt(observed: &super::paea_git_repository::transfer::TransferReceipt) -> PaeaGitTransferReceipt {
    PaeaGitTransferReceipt {
        candidate: observed.candidate.clone(),
        pack_bytes: observed.pack_bytes,
        pack_sha256: observed.pack_sha256.clone(),
    }
}
/// Exact input frame for one guarded run. Prepared by the trusted caller from
/// validated data; declared policy fields are rechecked under the held
/// authority in the policy module. Explicit close is required on every path.
pub struct PaeaGitTransferCall {
    project_binding: RootBinding,
    service_binding: RootBinding,
    request: Request,
    proposal: Proposal,
    context: AdmissionContext,
    work: String,
    child: String,
    pin: String,
    staging: String,
}
impl PaeaGitTransferCall {
    /// Exact permission data for separately authorized enrollment; no IO.
    pub fn request(&self) -> &Request {
        &self.request
    }
    /// Canonical proposal data to review, not an attestation of its context.
    pub fn proposal(&self) -> &Proposal {
        &self.proposal
    }
    /// Declared admission context, retained verbatim. Commit policy fields are
    /// not attributed here: an observed preparation attributes no admission.
    pub fn context(&self) -> &AdmissionContext {
        &self.context
    }
    /// Compare both prepared physical witnesses against the actually opened
    /// service and project Roots, before any session publication or HTTP
    /// consumption. This is the same comparison the guarded callback performs
    /// under authority, exposed for trusted transport binding checks only;
    /// it performs no IO beyond the witnesses' inode observation, grants no
    /// authority and never substitutes the engine-side recheck inside the call.
    pub fn binding_matches(
        &self,
        project: &RootStore,
        service: &RootStore,
    ) -> Result<bool, R> {
        Ok(self.project_binding.matches(project)? && self.service_binding.matches(service)?)
    }
    /// Explicitly release both held storage witnesses, retaining independent
    /// close failures. No data mutation or authority revocation occurs.
    pub fn close(self) -> Result<(), R> {
        finished(self.project_binding.close(), self.service_binding.close())
    }
}
/// Acquire both witnesses; on any partial success close what was acquired and
/// refuse. Close failures from the surviving witness are joined below the
/// fixed refusal so no handle or cause is silently dropped.
fn witness(
    project: Result<RootBinding, R>,
    service: Result<RootBinding, R>,
) -> Result<(RootBinding, RootBinding), R> {
    match (project, service) {
        (Ok(p), Ok(s)) => Ok((p, s)),
        (Ok(p), Err(_)) | (Err(_), Ok(p)) => {
            let cleanup = p.close();
            Err(match cleanup {
                Ok(()) => R::Refused("paea-git-transfer-witness"),
                Err(cause) => R::Cleanup {
                    primary: Some(Box::new(R::Refused("paea-git-transfer-witness"))),
                    cleanup: Box::new(cause),
                },
            })
        }
        (Err(_), Err(_)) => Err(R::Refused("paea-git-transfer-witness")),
    }
}
impl RootStore {
    /// Prepare exact permission data from the actual held service/project
    /// witnesses and a validated Git proposal. Retains both inode witnesses and
    /// the canonical action digest; grants nothing, opens no Git and reserves
    /// no durable intent. The permission binds Service::GitPromotion plus
    /// Operation::Reconcile on the physical witness-root digest and canonical
    /// action digest. Commit is refused at consumption: an observed preparation
    /// is not a reference commitment and no attributed-admission path exists.
    /// The exact work/child/pin/staging names are trusted-caller inputs that
    /// the job revalidates against the journal pin and managed definition.
    /// Caller must explicitly close the call on every success/refusal path.
    #[allow(clippy::too_many_arguments)]
    pub fn paea_git_transfer_call(
        &self,
        project: &RootStore,
        proposal: &Proposal,
        context: &AdmissionContext,
        work: &str,
        child: &str,
        pin: &str,
        staging: &str,
    ) -> Result<PaeaGitTransferCall, R> {
        if !self.writable || !project.writable {
            return Err(R::Refused("paea-git-transfer-binding"));
        }
        let canonical = swarm_protection_contract::canonicalize(proposal, context)
            .map_err(|_| R::Refused("paea-git-transfer-canonical"))?;
        if !matches!(proposal.action, Action::Git { .. }) {
            return Err(R::Refused("paea-git-transfer-git-required"));
        }
        let project_instance = project.instance()?;
        let service_instance = self.instance()?;
        if project_instance.directory == service_instance.directory
            || project_instance.database == service_instance.database
        {
            return Err(R::Refused("paea-git-transfer-separate-roots-required"));
        }
        let nonce = crate::generate_identifier("paea-git-transfer-")
            .map_err(|_| R::Refused("paea-git-transfer-nonce"))?;
        let root = crate::trust::digest(&serde_json::json!({
            "domain": "paea-git-transfer-roots-v1",
            "project": project_instance,
            "service": service_instance,
            "nonce": nonce,
        }))
        .map_err(|_| R::Refused("paea-git-transfer-root-digest"))?;
        let request = Request::new(
            &root,
            Service::GitPromotion,
            Operation::Reconcile,
            canonical.digest(),
        )
        .map_err(|_| R::Refused("paea-git-transfer-request"))?;
        let (project_binding, service_binding) =
            witness(project.bind_root(), self.bind_root())?;
        Ok(PaeaGitTransferCall {
            project_binding,
            service_binding,
            request,
            proposal: proposal.clone(),
            context: context.clone(),
            work: work.to_owned(),
            child: child.to_owned(),
            pin: pin.to_owned(),
            staging: staging.to_owned(),
        })
    }
    /// Run one exact guarded observation of the crate-owned transfer job under
    /// current borrowed PAEA authority. The callback rechecks physical witness
    /// bindings against both actually opened Roots, policy admission (current
    /// Trust revision, non-disabled effect role, Biba Invoke at I2) and the
    /// exact operation, then opens the job on the provisioned declaration,
    /// runs the native preparation once and closes it. No ref is changed and
    /// no second run is possible inside one call. Authority AfterUse, policy
    /// and native causes remain typed and independently traversable; no retry
    /// or fallback runs. The caller closes both opened Roots afterwards.
    pub fn call_paea_git_transfer(
        &self,
        project: &RootStore,
        prepared: &PaeaGitTransferCall,
        authority: &AuthenticatedControl,
        context: &Context,
        head: &Head,
        password: &[u8],
    ) -> Result<PaeaGitTransferReceipt, PaeaGitTransferCallError> {
        if !self.writable || !project.writable {
            return Err(PaeaGitTransferCallError::Action(R::Refused(
                "paea-git-transfer-binding",
            )));
        }
        let used = authority
            .with_paea_action(context, head, password, prepared.request(), |identity| {
                if identity.request().operation() != Operation::Reconcile {
                    return Err(R::Refused("paea-git-transfer-operation"));
                }
                // Exact physical witness binding under the callback locks:
                // both prepared witnesses must match the actually opened Roots.
                let bound = prepared
                    .project_binding
                    .matches(project)
                    .and_then(|project_ok| {
                        if project_ok {
                            prepared.service_binding.matches(self)
                        } else {
                            Ok(false)
                        }
                    })?;
                if !bound {
                    return Err(R::Refused("paea-git-transfer-root-binding"));
                }
                policy::current_actor(project, identity)?;
                consume(self, project, prepared)
            })
            .map_err(PaeaGitTransferCallError::Personal)?;
        if used.authority.is_err() {
            return Err(PaeaGitTransferCallError::Authority(used));
        }
        used.action
            .ok_or(PaeaGitTransferCallError::MissingAction)?
            .map_err(PaeaGitTransferCallError::Action)
    }
}
/// Owned one-shot native preparation inside the held authority callback. The
/// job owns custody and staging; only its frozen summary escapes. A failed run
/// keeps the job's original cause while its owned witnesses are checked-closed;
/// an unresolved native dependency still refuses close explicitly instead of
/// silently dropping retained processes.
fn consume(
    service: &RootStore,
    project: &RootStore,
    prepared: &PaeaGitTransferCall,
) -> Result<PaeaGitTransferReceipt, R> {
    let mut job = service
        .paea_open_git_transfer_job(
            project,
            &prepared.work,
            &prepared.child,
            &prepared.pin,
            &prepared.staging,
        )
        .map_err(|cause| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-transfer-open",
                cause,
            )))
        })?;

    let code = match job.run_once(service, project, &prepared.proposal) {
        Ok(()) => None,
        Err(code) => Some(code),
    };
    let closed = match job.close() {
        Ok(closed) => closed,
        Err(_) => {
            // The job refuses close while its native cause graph holds an
            // unresolved child; the failure code is the safe diagnostic here.
            return Err(R::Refused(
                code.unwrap_or("paea-git-transfer-close-unresolved"),
            ));
        }
    };
    if let Err(cause) = closed.cleanup {
        return Err(cause);
    }
    if let Some(code) = code {
        return Err(R::Refused(code));
    }
    closed
        .receipt
        .as_ref()
        .map(receipt)
        .ok_or(R::Refused("paea-git-transfer-receipt-missing"))
}
/// Recheck the actual configured Trust revision, non-disabled canonical effect
/// role and Biba Invoke admission under a fresh shared reservation. The signed
/// grant alone never substitutes for current storage state. Operation Commit is
/// structurally refused upstream; no separation policy exception exists here.
mod policy {
    use super::{Identity, R};
    use crate::biba::{check_biba, BibaAccess, Integrity};
    use crate::trust::state::State;
    /// Re-read the actual Trust state and compare the signed revision, mapped
    /// non-disabled effect identity and current Biba admission. This reuses the
    /// established bounded state reader without inventing a new policy layer
    /// or admitting a Commit operation.
    pub(super) fn current_actor(
        project: &crate::RootStore,
        identity: &Identity<'_>,
    ) -> Result<(), R> {
        let raw = project
            .directory
            .read_bounded(
                super::super::trust_store::STATE,
                super::super::trust_store::MAX_STATE,
            )
            .map_err(|_| {
                R::Trust(Box::new(crate::TrustError::Integrity(
                    "paea-git-transfer-trust-read",
                )))
            })?;
        let state =
            State::decode(&raw).map_err(|_| R::Refused("paea-git-transfer-trust-decode"))?;
        state
            .validate()
            .map_err(|_| R::Refused("paea-git-transfer-trust-validate"))?;
        state
            .require_supported()
            .map_err(|_| R::Refused("paea-git-transfer-trust-unsupported"))?;
        if identity.trust_revision() != state.revision {
            return Err(R::Refused("paea-git-transfer-trust-revision"));
        }
        let actor = state
            .identities
            .iter()
            .find(|i| {
                !i.disabled
                    && i.enrollment_principal.as_ref() == Some(identity.principal())
                    && i.roles.iter().any(|r| r == "effect")
            })
            .ok_or(R::Refused("paea-git-transfer-effect-role"))?;
        let level = match actor.integrity {
            0 => Integrity::I0,
            1 => Integrity::I1,
            2 => Integrity::I2,
            3 => Integrity::I3,
            _ => return Err(R::Refused("paea-git-transfer-role-integrity")),
        };
        check_biba(level, Integrity::I2, BibaAccess::Invoke)
            .map_err(|_| R::Refused("paea-git-transfer-effect-integrity"))?;
        Ok(())
    }
}
