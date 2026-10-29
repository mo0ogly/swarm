//! Guarded protected Git reference promotion under current PAEA authority: one
//! exact reference transaction with OLD fencing, a durable intent reserved
//! before any Git effect and a recorded effect in the service journal. Authority
//! and admission checks mirror the established transfer/settlement call shapes.
//! This entry never grants permission, provisions storage or signs a grant; the
//! exact signed `repository.promote` commit permission is consumed for one
//! observed reference publication. Commit uncertainty and late failures retain
//! their known state; exposing this entry is not an accepted service and
//! establishes no OS isolation, upstream admission, human presence or
//! independent proof.
use super::{
    finished, mission_supervision_store as sql, RootBinding, RootStore, RootStoreError as R,
};
use rusqlite::{params, Connection, Transaction, TransactionBehavior};
use std::{convert::Infallible, error::Error, fmt};
use swarm_enrollment_authority::{
    authenticated_control::{AuthenticatedControl, Context},
    paea_action::{Operation, Request, Service, Use},
    personal::coordinator::UseError,
};
use swarm_enrollment_state::Head;
use swarm_protection_contract::{
    canonicalize, Action, AdmissionContext, CanonicalAction, Proposal,
};

const META: &str = "CREATE TABLE paea_git_promote_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),schema_version INTEGER NOT NULL CHECK(schema_version=1),project_id TEXT NOT NULL,project_digest TEXT NOT NULL)";
const EFFECTS: &str = "CREATE TABLE paea_git_promote_effects(intent_id TEXT PRIMARY KEY,action_digest TEXT NOT NULL,target_ref TEXT NOT NULL,new_oid TEXT NOT NULL,receipt TEXT NOT NULL)";

/// All authority, policy and backend branches, including a known outcome after
/// failure. No variant implies rollback. Formatting is fixed and excludes
/// credentials, canonical action, receipts and private nested diagnostics.
pub enum PaeaGitPromoteCallError {
    Personal(UseError<Use<PaeaGitPromoteOutcome, R>, Infallible>),
    Authority(Use<PaeaGitPromoteOutcome, R>),
    Action(R),
    MissingAction,
}
impl PaeaGitPromoteCallError {
    /// Complete callback outcome when the held operation reached the backend.
    fn action(&self) -> Option<&Result<PaeaGitPromoteOutcome, R>> {
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
    /// Known recorded/replayed or negative reconciliation outcome despite a
    /// later failure. Availability never restores expired/revoked authority.
    pub fn known_outcome(&self) -> Option<&PaeaGitPromoteOutcome> {
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
    /// walker; no message parsing, downcast fallback or invented outcome.
    /// Returns the first envelope in branch order, without merging multiple
    /// independent calls; None establishes neither absence of effects nor success.
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
            Self::Personal(_) => "paea-git-promote-personal",
            Self::Authority(_) => "paea-git-promote-authority",
            Self::Action(_) => "paea-git-promote-action",
            Self::MissingAction => "paea-git-promote-action-missing",
        }
    }
}
impl fmt::Display for PaeaGitPromoteCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for PaeaGitPromoteCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PaeaGitPromoteCallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.branches().into_iter().next()
    }
}
/// Known recorded/replayed or negative reconciliation state. Replayed never
/// denotes a new reference publication by this call.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum PaeaGitPromoteOutcome {
    /// A durable effect row exists for this exact canonical intent.
    /// replayed=true performed no new reference publication by this call.
    Recorded {
        intent_id: String,
        action_digest: String,
        target_ref: String,
        new_oid: String,
        replayed: bool,
    },
    /// Under the fenced journal read, no durable effect row exists for this
    /// canonical intent. This is an explicit reconciliation result, not an
    /// empty success, a ref observation or a grant.
    ReconciledNotCommitted {
        intent_id: String,
        target_ref: String,
    },
}
/// Frozen observation of one completed reference publication. Object IDs
/// describe the actual observed reference state at recording time; they prove
/// no current grant, later usability or continued ref state. No raw path, FD,
/// capability or process identity exists.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PaeaGitPromoteReceipt {
    pub target_ref: String,
    pub old_oid: String,
    pub new_oid: String,
}
/// Exact input frame for one guarded publication. Prepared by the trusted
/// caller from validated data; declared policy fields are rechecked under the
/// held authority. Explicit close is required on every path.
pub struct PaeaGitPromoteCall {
    project_binding: RootBinding,
    service_binding: RootBinding,
    request: Request,
    proposal: Proposal,
    context: AdmissionContext,
    canonical: CanonicalAction,
    work: String,
    child: String,
    pin: String,
}
impl PaeaGitPromoteCall {
    /// Exact permission data for separately authorized enrollment; no IO.
    pub fn request(&self) -> &Request {
        &self.request
    }
    /// Canonical proposal data to review, not an attestation of its context.
    pub fn proposal(&self) -> &Proposal {
        &self.proposal
    }
    /// Canonical action identity used by the durable intent and the recorded
    /// effect. Not an attestation of the declared context.
    pub fn canonical(&self) -> &CanonicalAction {
        &self.canonical
    }
    /// Declared admission context, retained verbatim. Commit policy fields are
    /// attributed at preparation from the actual opened Trust storage.
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
                Ok(()) => R::Refused("paea-git-promote-witness"),
                Err(cause) => R::Cleanup {
                    primary: Some(Box::new(R::Refused("paea-git-promote-witness"))),
                    cleanup: Box::new(cause),
                },
            })
        }
        (Err(_), Err(_)) => Err(R::Refused("paea-git-promote-witness")),
    }
}
/// Validate all exact schemas without creating or adopting partial history.
fn schema(db: &Connection) -> Result<bool, R> {
    let mut count = 0;
    for (table, ddl) in [
        ("paea_git_promote_meta", META),
        ("paea_git_promote_effects", EFFECTS),
    ] {
        if sql::auxiliary_schema(db, table, ddl, "paea-git-promote-schema")
            .map_err(|e| R::SqlTransaction(Box::new(e)))?
        {
            count += 1;
        }
    }
    match count {
        0 => Ok(false),
        2 => Ok(true),
        _ => Err(R::Refused("paea-git-promote-schema-partial")),
    }
}
/// Read the unique configured project association; absence/corruption is refusal.
fn meta(db: &Connection) -> Result<(String, String), R> {
    if !schema(db)? {
        return Err(R::Refused("paea-git-promote-uninitialized"));
    }
    let (project, pin): (String, String) = sql::optional(
        db,
        "SELECT project_id,project_digest FROM main.paea_git_promote_meta WHERE singleton=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|e| R::SqlTransaction(Box::new(e)))?
    .ok_or(R::Refused("paea-git-promote-meta-missing"))?;
    if !swarm_identity_contract::name(project.as_bytes())
        || !swarm_identity_contract::digest(pin.as_bytes())
    {
        return Err(R::Refused("paea-git-promote-meta-invalid"));
    }
    Ok((project, pin))
}
/// Explicit rollback without an implicit Drop retry. Preserve rollback failure;
/// an already auto-committed/rolled-back connection needs no further operation.
fn rollback(mut tx: Transaction<'_>) -> Result<(), R> {
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    if tx.is_autocommit() {
        return Ok(());
    }
    tx.execute_batch("ROLLBACK").map_err(R::Sql)
}
impl RootStore {
    /// Explicit trusted bootstrap of the promotion effect journal. Requires the
    /// independently provisioned intent journal on this same Root. Refuses
    /// existing/partial schema; one atomic SQL transaction, no Git. This entry
    /// alone establishes neither operator identity nor custody.
    pub fn paea_initialize_git_promotions(
        &self,
        project_id: &str,
        project_digest: &str,
    ) -> Result<(), R> {
        if !self.writable
            || !swarm_identity_contract::name(project_id.as_bytes())
            || !swarm_identity_contract::digest(project_digest.as_bytes())
        {
            return Err(R::Refused("paea-git-promote-provisioning-binding"));
        }
        super::paea_git_intent::meta(&self.db).map_err(|e| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-intent-journal",
                e,
            )))
        })?;
        let tx = Transaction::new_unchecked(&self.db, TransactionBehavior::Immediate)
            .map_err(R::Sql)?;
        let result = (|| {
            if schema(&tx)? {
                return Err(R::Refused("paea-git-promote-already-initialized"));
            }
            tx.execute_batch(&format!("{META};{EFFECTS};"))
                .map_err(R::Sql)?;
            sql::execute(
                &tx,
                "INSERT INTO main.paea_git_promote_meta VALUES(1,1,?,?)",
                params![project_id, project_digest],
            )
            .map_err(|e| R::SqlTransaction(Box::new(e)))?;
            Ok(())
        })();
        match result {
            Ok(()) => match tx.execute_batch("COMMIT") {
                Ok(()) => rollback(tx),
                Err(_cause) => {
                    let mut e = R::Refused("paea-git-promote-initialize-uncertain");
                    if let Err(cleanup) = rollback(tx) {
                        e = R::Cleanup {
                            primary: Some(Box::new(e)),
                            cleanup: Box::new(cleanup),
                        };
                    }
                    Err(e)
                }
            },
            Err(e) => finished(Err(e), rollback(tx)),
        }
    }
    /// Prepare exact permission data from the actual held service/project
    /// witnesses and a validated Git proposal. Retains both inode witnesses and
    /// the canonical action; grants nothing, opens no Git and reserves no
    /// durable intent. The permission binds Service::GitPromotion plus the
    /// requested operation on the physical witness-root digest and canonical
    /// action digest. Commit policy fields are attributed here from the actual
    /// opened project Trust storage under a shared reservation, mirroring the
    /// settlement attribution contract. The exact work/child/pin names are
    /// trusted-caller inputs that the repository custody revalidates against
    /// the journal pin and managed definition. Caller must explicitly close the
    /// call on every success/refusal path.
    #[allow(clippy::too_many_arguments)]
    pub fn paea_git_promote_call(
        &self,
        project: &RootStore,
        proposal: &Proposal,
        context: &AdmissionContext,
        operation: Operation,
        work: &str,
        child: &str,
        pin: &str,
    ) -> Result<PaeaGitPromoteCall, R> {
        if !self.writable || !project.writable {
            return Err(R::Refused("paea-git-promote-binding"));
        }
        proposal.validate().map_err(|e| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-proposal",
                e,
            )))
        })?;
        if !matches!(proposal.action, Action::Git { .. }) {
            return Err(R::Refused("paea-git-promote-git-required"));
        }
        let context = match operation {
            Operation::Commit => policy::attribute(project, context)?,
            Operation::Reconcile => context.clone(),
        };
        let canonical =
            canonicalize(proposal, &context).map_err(|e| {
                R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-canonical",
                    e,
                )))
            })?;
        let project_instance = project.instance()?;
        let service_instance = self.instance()?;
        if project_instance.directory == service_instance.directory
            || project_instance.database == service_instance.database
        {
            return Err(R::Refused("paea-git-promote-separate-roots-required"));
        }
        let nonce = crate::generate_identifier("paea-git-promote-")
            .map_err(|_| R::Refused("paea-git-promote-nonce"))?;
        let root = crate::trust::digest(&serde_json::json!({
            "domain": "paea-git-promote-roots-v1",
            "project": project_instance,
            "service": service_instance,
            "nonce": nonce,
        }))
        .map_err(|e| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-root-digest",
                e,
            )))
        })?;
        let request = Request::new(&root, Service::GitPromotion, operation, canonical.digest())
            .map_err(|e| {
                R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-request",
                    e,
                )))
            })?;
        let (project_binding, service_binding) =
            witness(project.bind_root(), self.bind_root())?;
        Ok(PaeaGitPromoteCall {
            project_binding,
            service_binding,
            request,
            proposal: proposal.clone(),
            context,
            canonical,
            work: work.to_owned(),
            child: child.to_owned(),
            pin: pin.to_owned(),
        })
    }
    /// Run one exact guarded publication or reconciliation under current
    /// borrowed PAEA authority. The callback rechecks physical witness bindings
    /// against both actually opened Roots, the exact permission/service/
    /// operation, policy admission (current Trust revision, non-disabled
    /// effect role, Biba Invoke at I2, freshness for Commit) and the current
    /// plan scope, then executes under a project writer reservation and the
    /// project Trust reservation. Commit performs one exact reference
    /// transaction with OLD fencing after reserving the durable intent; a
    /// commit error is uncertain and requires explicit reconciliation.
    /// Reconcile reads the durable journal state without any new publication or
    /// Git process. Authority AfterUse, policy and backend causes remain typed
    /// and independently traversable; no retry or fallback runs. The caller
    /// closes both opened Roots afterwards.
    pub fn call_paea_git_promote(
        &self,
        project: &RootStore,
        prepared: &PaeaGitPromoteCall,
        authority: &AuthenticatedControl,
        context: &Context,
        head: &Head,
        password: &[u8],
    ) -> Result<PaeaGitPromoteOutcome, PaeaGitPromoteCallError> {
        if !self.writable || !project.writable {
            return Err(PaeaGitPromoteCallError::Action(R::Refused(
                "paea-git-promote-binding",
            )));
        }
        let used = authority
            .with_paea_action(context, head, password, prepared.request(), |identity| {
                if identity.request().permission() != prepared.request().permission()
                    || identity.request().service() != Service::GitPromotion
                {
                    return Err(R::Refused("paea-git-promote-service"));
                }
                if identity.request().operation() != prepared.request().operation() {
                    return Err(R::Refused("paea-git-promote-operation"));
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
                    return Err(R::Refused("paea-git-promote-root-binding"));
                }
                // One immediate project writer fences the plan/policy/candidate
                // snapshot; the Trust reservation is held across policy checks
                // and the Git effect. No cross-store atomicity is claimed
                // between the SQL effect and the Git reference.
                let project_tx =
                    Transaction::new_unchecked(&project.db, TransactionBehavior::Immediate)
                        .map_err(R::Sql)?;
                let result = (|| {
                    let trust = project.directory.reserve_trust(false).map_err(R::Storage)?;
                    let inner = (|| {
                        policy::current_plan(project, identity)?;
                        policy::current_actor(project, identity, &prepared.context)?;
                        match prepared.request().operation() {
                            Operation::Commit => effect::commit(self, project, prepared, identity),
                            Operation::Reconcile => effect::reconcile(self, prepared),
                        }
                    })();
                    match trust.close() {
                        Ok(()) => inner,
                        Err(cause) => {
                            let cleanup = R::Storage(cause);
                            match inner {
                                Ok(outcome) => {
                                    let _ = outcome;
                                    Err(R::Cleanup {
                                        primary: None,
                                        cleanup: Box::new(cleanup),
                                    })
                                }
                                Err(e) => Err(R::Cleanup {
                                    primary: Some(Box::new(e)),
                                    cleanup: Box::new(cleanup),
                                }),
                            }
                        }
                    }
                })();
                finished(result, rollback(project_tx))
            })
            .map_err(PaeaGitPromoteCallError::Personal)?;
        if used.authority.is_err() {
            return Err(PaeaGitPromoteCallError::Authority(used));
        }
        used.action
            .ok_or(PaeaGitPromoteCallError::MissingAction)?
            .map_err(PaeaGitPromoteCallError::Action)
    }
}
/// Exact current policy attribution and fail-closed compatibility checks,
/// mirroring the established settlement policy module without inventing a new
/// policy layer.
mod policy {
    use super::{sql, R, RootStore};
    use crate::biba::{check_biba, BibaAccess, Integrity};
    use crate::trust::state::State;
    use swarm_enrollment_authority::paea_action::{Identity, Operation};
    use swarm_protection_contract::AdmissionContext;
    /// Read bounded Trust storage with typed decode/validation refusals.
    fn read(project: &RootStore) -> Result<State, R> {
        let raw = project
            .directory
            .read_bounded(
                super::super::trust_store::STATE,
                super::super::trust_store::MAX_STATE,
            )
            .map_err(|_| {
                R::Trust(Box::new(crate::TrustError::Integrity(
                    "paea-git-promote-trust-read",
                )))
            })?;
        let state =
            State::decode(&raw).map_err(|_| R::Refused("paea-git-promote-trust-decode"))?;
        state
            .validate()
            .map_err(|_| R::Refused("paea-git-promote-trust-validate"))?;
        state
            .require_supported()
            .map_err(|_| R::Refused("paea-git-promote-trust-unsupported"))?;
        Ok(state)
    }
    /// Domain-separated digest of the real base policy, optional role policy
    /// and control blocks. This is a pinned policy projection, not a signature.
    fn fingerprint(state: &State) -> Result<String, R> {
        let controls = state
            .controls()
            .map_err(|_| R::Refused("paea-git-promote-policy-controls"))?;
        crate::trust::digest(&serde_json::json!({
            "domain": "paea-trust-policy-v1",
            "policy": state.policy,
            "role_separation": state.role_separation,
            "controls": controls,
        }))
        .map_err(|e| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-policy-digest",
                e,
            )))
        })
    }
    /// Produce a separate context with policy fields from opened current
    /// storage, mirroring the settlement attribution contract. A close error
    /// refuses preparation without a business effect.
    pub(super) fn attribute(
        project: &RootStore,
        declared: &AdmissionContext,
    ) -> Result<AdmissionContext, R> {
        declared
            .validate()
            .map_err(|_| R::Refused("paea-git-promote-context"))?;
        let reservation = project
            .directory
            .reserve_trust(false)
            .map_err(R::Storage)?;
        let result = (|| {
            let state = read(project)?;
            let mut context = declared.clone();
            context.policy_revision = state.revision;
            context.policy_digest = fingerprint(&state)?;
            Ok(context)
        })();
        match reservation.close() {
            Ok(()) => result,
            Err(cause) => match result {
                Ok(_) => Err(R::Storage(cause)),
                Err(e) => Err(R::Cleanup {
                    primary: Some(Box::new(e)),
                    cleanup: Box::new(R::Storage(cause)),
                }),
            },
        }
    }
    /// Re-read the actual configured Trust revision, non-disabled canonical
    /// effect role and Biba Invoke admission under the caller-held reservation.
    /// The signed grant alone never substitutes for current storage state.
    pub(super) fn current_actor(
        project: &RootStore,
        identity: &Identity<'_>,
        context: &AdmissionContext,
    ) -> Result<(), R> {
        let state = read(project)?;
        if identity.trust_revision() != state.revision {
            return Err(R::Refused("paea-git-promote-trust-revision"));
        }
        let actor = state
            .identities
            .iter()
            .find(|i| {
                !i.disabled
                    && i.enrollment_principal.as_ref() == Some(identity.principal())
                    && i.roles.iter().any(|r| r == "effect")
            })
            .ok_or(R::Refused("paea-git-promote-effect-role"))?;
        let level = match actor.integrity {
            0 => Integrity::I0,
            1 => Integrity::I1,
            2 => Integrity::I2,
            3 => Integrity::I3,
            _ => return Err(R::Refused("paea-git-promote-role-integrity")),
        };
        check_biba(level, Integrity::I2, BibaAccess::Invoke)
            .map_err(|_| R::Refused("paea-git-promote-effect-integrity"))?;
        if identity.request().operation() == Operation::Commit {
            if state.policy.require_separation
                || state
                    .role_separation
                    .as_ref()
                    .is_some_and(|p| p.enabled && !p.incompatible_roles.is_empty())
            {
                return Err(R::Refused(
                    "paea-git-promote-attributed-admission-required",
                ));
            }
            if context.policy_revision != state.revision
                || context.policy_digest != fingerprint(&state)?
            {
                return Err(R::Refused("paea-git-promote-policy-stale"));
            }
        }
        Ok(())
    }
    /// Check the actual current plan under the caller's project writer.
    /// Read-only work projection; no migration, grant or fallback is created.
    pub(super) fn current_plan(project: &RootStore, identity: &Identity<'_>) -> Result<(), R> {
        let scope = identity.scope();
        let version: i64 = sql::optional(
            &project.db,
            "SELECT projection_version FROM main.agent_identity_plan_heads WHERE work_id=?",
            [&scope.work_id],
            |r| r.get(0),
        )
        .map_err(|e| R::SqlTransaction(Box::new(e)))?
        .ok_or(R::Refused("paea-git-promote-plan-missing"))?;
        let projection = match version {
            1 => crate::IdentityPlanProjection::V1,
            2 => crate::IdentityPlanProjection::V2,
            _ => return Err(R::Refused("paea-git-promote-plan-projection")),
        };
        let stamp = crate::read_identity_plan_stamp(&project.db, &scope.work_id, projection)
            .map_err(|e| R::SqlTransaction(Box::new(e)))?;
        if !stamp.matches_scope(scope) {
            return Err(R::Refused("paea-git-promote-plan-stale"));
        }
        Ok(())
    }
}
/// Durable intent, reference transaction and recorded effect storage.
mod effect {
    use super::{meta, rollback, PaeaGitPromoteOutcome, PaeaGitPromoteReceipt, R, RootStore};
    use rusqlite::{params, Transaction, TransactionBehavior};
    use std::{cell::Cell, error::Error};
    use swarm_enrollment_authority::paea_action::Identity;
    use swarm_protection_contract::Action;
    /// True only for the absent-reference marker accepted by Git's update-ref.
    fn zero_oid(oid: &str) -> bool {
        !oid.is_empty() && oid.bytes().all(|b| b == b'0')
    }
    /// Bind a stored effect to the exact expected reference state. Detects
    /// inconsistent local corruption, not a malicious owner's consistent rewrite.
    fn validate_effect(
        receipt: &PaeaGitPromoteReceipt,
        target_ref: &str,
        new_oid: &str,
        old_oid: &str,
    ) -> Result<(), R> {
        if receipt.target_ref != target_ref || receipt.new_oid != new_oid || receipt.old_oid != old_oid
        {
            return Err(R::Refused("paea-git-promote-effect-incompatible"));
        }
        Ok(())
    }
    /// Read one durable effect row; missing is explicit None, corruption is an
    /// error. Returns (action_digest, target_ref, new_oid, receipt_json).
    fn existing_effect(
        db: &rusqlite::Connection,
        intent_id: &str,
    ) -> Result<Option<(String, String, String, String)>, R> {
        super::sql::optional(
            db,
            "SELECT action_digest,target_ref,new_oid,receipt FROM main.paea_git_promote_effects WHERE intent_id=?",
            [intent_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| R::SqlTransaction(Box::new(e)))
    }
    /// Join an outcome with an explicit custody close, preserving a known
    /// outcome through a later close failure.
    fn finished_out(
        result: Result<PaeaGitPromoteOutcome, R>,
        cleanup: Result<(), R>,
    ) -> Result<PaeaGitPromoteOutcome, R> {
        match (result, cleanup) {
            (Ok(v), Ok(())) => Ok(v),
            (Err(e), Ok(())) => Err(e),
            (Ok(_), Err(e)) => Err(R::Cleanup {
                primary: None,
                cleanup: Box::new(e),
            }),
            (Err(e), Err(cleanup)) => Err(R::Cleanup {
                primary: Some(Box::new(e)),
                cleanup: Box::new(cleanup),
            }),
        }
    }
    /// Record one effect row in one explicit service-journal transaction,
    /// preserving uncertainty on COMMIT failure. An identical existing row
    /// replays instead of failing.
    fn record(
        service: &RootStore,
        intent_id: &str,
        action_digest: &str,
        receipt: &PaeaGitPromoteReceipt,
        replayed: bool,
    ) -> Result<PaeaGitPromoteOutcome, R> {
        let tx = Transaction::new_unchecked(&service.db, TransactionBehavior::Immediate)
            .map_err(R::Sql)?;
        let result = (|| {
            if let Some((digest, target, new, raw)) = existing_effect(&tx, intent_id)? {
                let stored: PaeaGitPromoteReceipt = serde_json::from_str(&raw)
                    .map_err(|_| R::Refused("paea-git-promote-effect-decode"))?;
                validate_effect(&stored, &target, &new, &receipt.old_oid)?;
                if digest != action_digest {
                    return Err(R::Refused("paea-git-promote-intent-conflict"));
                }
                return Ok(PaeaGitPromoteOutcome::Recorded {
                    intent_id: intent_id.to_owned(),
                    action_digest: digest,
                    target_ref: target,
                    new_oid: new,
                    replayed: true,
                });
            }
            let raw = serde_json::to_string(receipt)
                .map_err(|_| R::Refused("paea-git-promote-effect-encode"))?;
            let n = super::sql::execute(
                &tx,
                "INSERT INTO main.paea_git_promote_effects(intent_id,action_digest,target_ref,new_oid,receipt) VALUES(?,?,?,?,?)",
                params![intent_id, action_digest, receipt.target_ref, receipt.new_oid, raw],
            )
            .map_err(|e| R::SqlTransaction(Box::new(e)))?;
            if n != 1 {
                return Err(R::Refused("paea-git-promote-effect-not-recorded"));
            }
            Ok(PaeaGitPromoteOutcome::Recorded {
                intent_id: intent_id.to_owned(),
                action_digest: action_digest.to_owned(),
                target_ref: receipt.target_ref.clone(),
                new_oid: receipt.new_oid.clone(),
                replayed,
            })
        })();
        match result {
            Ok(outcome) => match tx.execute_batch("COMMIT") {
                Ok(()) => rollback(tx).map(|()| outcome),
                Err(_cause) => {
                    let mut e = R::Refused("paea-git-promote-commit-uncertain");
                    if let Err(cleanup) = rollback(tx) {
                        e = R::Cleanup {
                            primary: Some(Box::new(e)),
                            cleanup: Box::new(cleanup),
                        };
                    }
                    Err(e)
                }
            },
            Err(e) => match rollback(tx) {
                Ok(()) => Err(e),
                Err(cleanup) => Err(R::Cleanup {
                    primary: Some(Box::new(e)),
                    cleanup: Box::new(cleanup),
                }),
            },
        }
    }
    /// One exact guarded publication. Reserves the durable intent before any
    /// Git, verifies the actual OLD under Git's prepared lock via the decision
    /// callback, runs the reference transaction once and records the observed
    /// effect. A crash between intent and effect is recovered under OLD
    /// fencing: a reference already at the candidate records the known outcome
    /// without a new publication.
    pub(super) fn commit(
        service: &RootStore,
        project: &RootStore,
        prepared: &super::PaeaGitPromoteCall,
        identity: &Identity<'_>,
    ) -> Result<PaeaGitPromoteOutcome, R> {
        if identity.principal().kind != "human"
            || time::OffsetDateTime::now_utc() >= identity.expires_at()
        {
            return Err(R::Refused("paea-git-promote-current-authority"));
        }
        let Action::Git {
            repository_id,
            target_ref,
            expected_old_oid,
            candidate_oid,
        } = &prepared.proposal.action
        else {
            return Err(R::Refused("paea-git-promote-git-required"));
        };
        // Durable intent reservation precedes any Git effect on the service
        // journal; Existing retains the original bytes for the business key.
        let reservation = service
            .paea_reserve_git_intent(&prepared.proposal, &prepared.context, &prepared.pin)
            .map_err(|e| {
                R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-intent",
                    e,
                )))
            })?;
        let (intent_id, action_digest, _fresh) = match &reservation {
            super::super::paea_git_intent::PaeaGitIntentReservation::New(record) => {
                (record.intent_id.clone(), record.action_digest.clone(), true)
            }
            super::super::paea_git_intent::PaeaGitIntentReservation::Existing(record) => {
                if record.action_digest != prepared.canonical.digest() {
                    return Err(R::Refused("paea-git-promote-intent-conflict"));
                }
                (record.intent_id.clone(), record.action_digest.clone(), false)
            }
        };
        // Replay without any Git when the effect already exists.
        if let Some((digest, target, new, raw)) = existing_effect(&service.db, &intent_id)? {
            let stored: PaeaGitPromoteReceipt = serde_json::from_str(&raw)
                .map_err(|_| R::Refused("paea-git-promote-effect-decode"))?;
            validate_effect(&stored, &target, &new, expected_old_oid)?;
            if digest != action_digest {
                return Err(R::Refused("paea-git-promote-intent-conflict"));
            }
            return Ok(PaeaGitPromoteOutcome::Recorded {
                intent_id,
                action_digest,
                target_ref: target,
                new_oid: new,
                replayed: true,
            });
        }
        // Actual current candidate under the project writer, before any Git.
        let repository = match service
            .paea_open_git_repository(project, &prepared.work, &prepared.child, &prepared.pin)
        {
            Ok(value) => value,
            Err(e) => {
                return Err(R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-open",
                    e,
                ))))
            }
        };
        if repository.binding().work_id() != prepared.work
            || repository.binding().repository_id() != repository_id
        {
            return finished_out(
                Err(R::Refused("paea-git-promote-repository-binding")),
                repository.close(),
            );
        }
        let observed = repository
            .inspect_for_effect(service, project, &prepared.proposal)
            .map_err(|e| {
                R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-inspect",
                    e,
                )))
            });
        let observed = match observed {
            Ok(value) => value,
            Err(e) => return finished_out(Err(e), repository.close()),
        };
        if observed.candidate_commit != *candidate_oid {
            return finished_out(
                Err(R::Refused("paea-git-promote-current-candidate")),
                repository.close(),
            );
        }
        // One exact reference transaction against the actual destination. The
        // decision callback verifies the direct ref state under Git's prepared
        // lock; protocol ACKs are observations only. The hardened runner strips
        // Git environment overrides and optional locks for the child process.
        let settings = service.read_settings()?;
        let destination = repository.directory_path();
        let already = Cell::new(false);
        let (git, deadline) =
            match crate::managed_repository::git_command(&destination, &[], &settings) {
                Ok(value) => value,
                Err(e) => {
                    return finished_out(
                        Err(R::Launch(Box::new(e))),
                        repository.close(),
                    )
                }
            };
        let run = crate::run_git_reference_transaction(
            git,
            &prepared.proposal,
            deadline,
            None,
            || {
                let current = crate::managed_repository::git(
                    &destination,
                    &["--git-dir", ".", "rev-parse", "--verify", target_ref],
                    &settings,
                );
                match current {
                    // Absent direct ref: only the zero-OID marker may commit a
                    // creation; Git itself fences the actual update under its
                    // prepared lock.
                    Err(_) if zero_oid(expected_old_oid) => {
                        Ok(crate::GitReferenceDecision::Commit)
                    }
                    Err(_) => Ok(crate::GitReferenceDecision::Abort),
                    Ok(raw) => {
                        let current = String::from_utf8(raw)
                            .map_err(|_| {
                                Box::new(std::io::Error::other(
                                    "paea-git-promote-old-encoding",
                                ))
                                    as Box<dyn Error + Send + Sync>
                            })?
                            .trim()
                            .to_owned();
                        if current == *candidate_oid {
                            // Crash recovery: the reference is already at the
                            // candidate; record the known outcome, publish nothing.
                            already.set(true);
                            return Ok(crate::GitReferenceDecision::Abort);
                        }
                        if current != *expected_old_oid {
                            return Err(Box::new(std::io::Error::other(
                                "paea-git-promote-old-changed",
                            ))
                                as Box<dyn Error + Send + Sync>);
                        }
                        Ok(crate::GitReferenceDecision::Commit)
                    }
                }
            },
        )
        .map_err(|e| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-transaction",
                e,
            )))
        });
        let run = match run {
            Ok(value) => value,
            Err(e) => return finished_out(Err(e), repository.close()),
        };
        if run.observation != crate::GitReferenceObservation::CommitAcknowledged
            && !(run.observation == crate::GitReferenceObservation::AbortAcknowledged
                && already.get())
        {
            return finished_out(
                Err(R::Refused("paea-git-promote-not-committed")),
                repository.close(),
            );
        }
        // Observe the actual new ref value after the protocol; the recorded
        // receipt binds this observation, not a promise.
        let observed_new = crate::managed_repository::git(
            &destination,
            &["--git-dir", ".", "rev-parse", "--verify", target_ref],
            &settings,
        )
        .map_err(|e| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-new-oid",
                e,
            )))
        });
        let observed_new = match observed_new {
            Ok(raw) => String::from_utf8(raw)
                .map_err(|_| R::Refused("paea-git-promote-new-oid-encoding"))?
                .trim()
                .to_owned(),
            Err(e) => return finished_out(Err(e), repository.close()),
        };
        if observed_new != *candidate_oid {
            return finished_out(
                Err(R::Refused("paea-git-promote-ref-candidate-mismatch")),
                repository.close(),
            );
        }
        let closed = repository.close();
        if time::OffsetDateTime::now_utc() >= identity.expires_at() {
            return finished_out(
                Err(R::Refused("paea-git-promote-authority-expired")),
                closed,
            );
        }
        let receipt = PaeaGitPromoteReceipt {
            target_ref: target_ref.clone(),
            old_oid: expected_old_oid.clone(),
            new_oid: observed_new,
        };
        finished_out(
            record(service, &intent_id, &action_digest, &receipt, already.get()),
            closed,
        )
    }
    /// Reconciliation: read the durable journal state without any new
    /// publication or Git process. Missing intent or absent effect is an
    /// explicit negative result; a stored effect replays its observation.
    pub(super) fn reconcile(
        service: &RootStore,
        prepared: &super::PaeaGitPromoteCall,
    ) -> Result<PaeaGitPromoteOutcome, R> {
        let Action::Git { target_ref, .. } = &prepared.proposal.action else {
            return Err(R::Refused("paea-git-promote-git-required"));
        };
        let (project, pin) = meta(&service.db)?;
        if pin != prepared.pin {
            return Err(R::Refused("paea-git-promote-project-pin"));
        }
        let _ = project;
        let intent = service
            .paea_read_git_intent(prepared.canonical.intent_id(), &prepared.pin)
            .map_err(|e| {
                R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-intent-read",
                    e,
                )))
            })?;
        let record = match intent {
            Some(record) => record,
            None => {
                return Ok(PaeaGitPromoteOutcome::ReconciledNotCommitted {
                    intent_id: prepared.canonical.intent_id().to_owned(),
                    target_ref: target_ref.clone(),
                })
            }
        };
        if record.action_digest != prepared.canonical.digest() {
            return Err(R::Refused("paea-git-promote-intent-conflict"));
        }
        match existing_effect(&service.db, &record.intent_id)? {
            Some((digest, target, new, raw)) => {
                let stored: PaeaGitPromoteReceipt = serde_json::from_str(&raw)
                    .map_err(|_| R::Refused("paea-git-promote-effect-decode"))?;
                let Action::Git {
                    expected_old_oid, ..
                } = &prepared.proposal.action
                else {
                    return Err(R::Refused("paea-git-promote-git-required"));
                };
                validate_effect(&stored, &target, &new, expected_old_oid)?;
                if digest != record.action_digest {
                    return Err(R::Refused("paea-git-promote-intent-conflict"));
                }
                Ok(PaeaGitPromoteOutcome::Recorded {
                    intent_id: record.intent_id,
                    action_digest: record.action_digest,
                    target_ref: target,
                    new_oid: new,
                    replayed: true,
                })
            }
            None => Ok(PaeaGitPromoteOutcome::ReconciledNotCommitted {
                intent_id: record.intent_id,
                target_ref: target_ref.clone(),
            }),
        }
    }
}
