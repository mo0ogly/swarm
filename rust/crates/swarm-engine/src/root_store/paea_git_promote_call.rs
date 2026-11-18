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
    /// Typed effect observation envelope: the guarded operation reached the
    /// backend, so `outcome` retains the known recorded/replayed result, the
    /// known reconciliation observation, or is `None` when the publication
    /// may have happened without an observable durable outcome, or when a
    /// reconciliation failed after observing no journal record. `None` is an
    /// explicit unknown, never an
    /// absence claim, a success or a retry authorization. `cause` is the
    /// independent typed primary failure; independent cleanup errors stay
    /// nested in the existing `RootStoreError::Cleanup` shape. Display stays
    /// the fixed safe code; formatting exposes no receipt, credential or private nested text.
    Effect {
        outcome: Option<PaeaGitPromoteOutcome>,
        cause: R,
    },
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
    /// Borrow the typed callback failure carried by this error, when the held
    /// operation reached the backend. Envelope causes surface directly.
    fn callback_error(&self) -> Option<&R> {
        match self {
            Self::Action(cause) => Some(cause),
            Self::Effect { cause, .. } => Some(cause),
            Self::Personal(UseError::AfterUse {
                outcome: Ok(used), ..
            })
            | Self::Authority(used) => used.action.as_ref()?.as_ref().err(),
            _ => None,
        }
    }
    /// Borrow the backend error without discarding primary/authority causes.
    pub fn backend_error(&self) -> Option<&R> {
        match self {
            Self::Action(e) => Some(e),
            Self::Effect { cause, .. } => Some(cause),
            _ => self.action().and_then(|a| a.as_ref().err()),
        }
    }
    /// Known recorded/replayed result or reconciliation observation despite a
    /// later failure, retained directly on the effect envelope or found
    /// through nested callback cleanup/launch/box wrappers. Availability never
    /// restores expired/revoked authority.
    pub fn known_outcome(&self) -> Option<&PaeaGitPromoteOutcome> {
        if let Self::Effect {
            outcome: Some(outcome),
            ..
        } = self
        {
            return Some(outcome);
        }
        if let Some(action) = self.action() {
            if let Ok(outcome) = action {
                return Some(outcome);
            }
        }
        Self::find(self.callback_error()?).and_then(Self::known_outcome)
    }
    /// Business-intention observation for a failed call, independently of its
    /// authority decision. Only a retained durable record establishes recorded;
    /// a pre-effect refusal or empty journal does not establish historical absence.
    /// A reconciliation observation counts as recorded only when it carries the
    /// retained journal record; an observed ref value never establishes it alone.
    /// This pure projection performs no IO and grants no retry or authority.
    pub fn effect_status(&self) -> &'static str {
        match self.known_outcome() {
            Some(PaeaGitPromoteOutcome::Recorded { .. }) => "recorded",
            Some(PaeaGitPromoteOutcome::Reconciled {
                recorded_effect: Some(_),
                ..
            }) => "recorded",
            _ => "unknown",
        }
    }

    /// Safe public effect metadata for this failed call. Known receipt fields
    /// come only from the retained typed outcome; missing knowledge yields an
    /// explicit unknown, never a fabricated no-effect result. No credentials,
    /// private nested diagnostics or canonical payload are rendered. A journal
    /// record is not independent authenticity or permission for another effect.
    pub fn public_effect_observation(&self) -> serde_json::Value {
        match self.known_outcome() {
            Some(PaeaGitPromoteOutcome::Recorded {
                intent_id, action_digest, target_ref, new_oid, replayed,
            }) => serde_json::json!({
                "state":"recorded", "intent_id":intent_id,
                "action_digest":action_digest, "target_ref":target_ref,
                "new_oid":new_oid, "replayed":replayed,
            }),
            Some(PaeaGitPromoteOutcome::Reconciled {
                intent_id, target_ref, observed_oid, recorded_effect,
            }) => match recorded_effect {
                // Recorded only through the retained journal row; the observed
                // ref value never establishes the record by itself.
                Some(effect) => serde_json::json!({
                    "state":"recorded", "intent_id":intent_id,
                    "action_digest":effect.action_digest, "target_ref":target_ref,
                    "new_oid":effect.new_oid,
                    "observed_oid":observed_oid,
                }),
                // No effect row never proves no past effect: the observation
                // stays explicitly unknown beside the successful ref reading.
                None => serde_json::json!({"state":"unknown", "intent_id":intent_id,
                    "target_ref":target_ref, "observed_oid":observed_oid,
                    "journal_state":"no_effect_row"}),
            },
            None => serde_json::json!({"state":"unknown"}),
        }
    }

    /// True when the guarded publication may have happened but no durable
    /// outcome was observed, on this error or through nested callback
    /// wrappers. Unknown never establishes absence of effects and never
    /// authorizes a retry.
    pub fn effect_unknown(&self) -> bool {
        if matches!(self, Self::Effect { outcome: None, .. }) {
            return true;
        }
        self.callback_error().and_then(|cause| Self::find(cause)).is_some_and(Self::effect_unknown)
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
            Self::Effect { .. } | Self::Action(_) | Self::MissingAction => (),
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
        if self.effect_unknown() && self.known_outcome().is_none() {
            return "paea-git-promote-unknown";
        }
        match self {
            Self::Personal(_) => "paea-git-promote-personal",
            Self::Authority(_) => "paea-git-promote-authority",
            Self::Action(_) => "paea-git-promote-action",
            Self::Effect { .. } => "paea-git-promote-effect",
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
/// Known recorded/replayed or reconciliation observation state. Replayed never
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
    /// Explicit reconciliation observation for one canonical intent: BOTH the
    /// historical durable journal effect and the actual current direct Git
    /// reference value, read without any publication, reservation, journal
    /// write or captured agreement. `observed_oid` is `Some` only when Git
    /// successfully listed the exact direct reference (a symbolic ref refuses) and `None` only when Git successfully listed no exact
    /// ref; a failed read never becomes `None` and stays in the typed effect
    /// envelope beside any journal record. An `observed_oid` equal to the
    /// candidate never proves this intent caused it, a `recorded_effect` stays
    /// historical even if the ref later moved or was deleted, and a missing
    /// effect row never proves that no past effect exists. Never an empty
    /// success, a negative result, an absence claim or a grant.
    ///
    /// FR : la réconciliation retourne l'effet journalisé historique et l'oid
    /// Git direct actuellement observé, sans publier, réserver ni écrire le
    /// journal ; l'absence de ligne d'effet ne prouve jamais l'absence d'un
    /// effet passé, et l'oid observé n'impute jamais la cause à cette
    /// intention.
    Reconciled {
        intent_id: String,
        target_ref: String,
        observed_oid: Option<String>,
        recorded_effect: Option<PaeaGitPromoteReconciledEffect>,
    },
}

/// Flat, nonrecursive projection of the historical durable effect row carried
/// by a reconciliation observation. It never nests a full outcome, never
/// describes the current reference state and never claims that the recorded
/// effect caused any observed ref value.
///
/// FR : projection plate de l'effet historique enregistré ; elle ne décrit
/// jamais l'état actuel de la référence et n'impute jamais la cause d'un oid
/// observé.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PaeaGitPromoteReconciledEffect {
    pub action_digest: String,
    pub target_ref: String,
    pub new_oid: String,
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
    /// effect role, Biba Invoke at I2, freshness for Commit, plus the exact
    /// captured agreement revalidated under the held reservation for Commit —
    /// refusing `paea-git-promote-agreement-stale` when no approval still
    /// matches these canonical bytes with an active mandate at the current
    /// revision) and the current
    /// plan scope, then executes under a project writer reservation and the
    /// project Trust reservation. Commit performs one exact reference
    /// transaction with OLD fencing after reserving the durable intent; a
    /// commit error is uncertain and requires explicit reconciliation. The
    /// Commit branch retains the actual matched agreement and passes its
    /// validated mandate expiry to the effect, which derives one fixed
    /// monotonic admission lease from the earliest absolute signed identity
    /// and mandate expiry, rechecks both clocks before the transaction and
    /// inside the prepared callback after the nested Git read, and bounds the
    /// transaction, the nested read and the post-publication read with that
    /// one fixed deadline (controlled waiting only: no physical Git
    /// publication deadline or receiver-enforced lease is claimed).
    /// Reconcile reads the durable journal state and the actual current direct
    /// Git reference (one read-only `for-each-ref` under the existing hardened
    /// runner, with explicit `%(symref)` validation) without any new
    /// publication, reservation, journal write or captured agreement. A failed
    /// ref read keeps its original cause and any journal record in the typed
    /// effect envelope; it never becomes a synthesized result with a fake
    /// observed `None`. Authority AfterUse, policy and backend causes remain typed
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
                            Operation::Commit => {
                                // Last-use agreement freshness at the actual
                                // consumption point: the durable Trust approval
                                // binding these exact canonical bytes must still
                                // match under the reservation held across the Git
                                // effect. The shared reservation excludes a
                                // concurrent public revocation (exclusive lock)
                                // from this check through the reference
                                // transaction, closing the window between the
                                // transport-side lookup and this effect without
                                // any sleep or second synchronization. A
                                // revision change, a revocation/expiry of the
                                // underlying mandate or a superseded approval
                                // after the captured lookup is the same typed
                                // refusal; Reconcile observes the journal and
                                // the current direct ref and needs no agreement.
                                // The actual matched agreement is retained and
                                // its validated mandate expiry (persisted
                                // RFC3339 string; None only for the preexisting
                                // no-mandate branch, where the signed identity
                                // expiry alone bounds the lease) is passed to
                                // the effect; no authority check is bypassed.
                                let agreement = project
                                    .paea_git_agreement_match(prepared.canonical.bytes())?;
                                if agreement.is_none() {
                                    return Err(R::Refused("paea-git-promote-agreement-stale"));
                                }
                                let mandate_expires_at =
                                    agreement.and_then(|matched| matched.expires_at);
                                // Fixture observation only, after final agreement
                                // and while the actual Trust SH reservation lives.
                                #[cfg(all(target_os = "linux", feature = "test-fixtures"))]
                                if let Some(Err(cause)) = crate::paea_git_fixture::run_shared_section_hook() {
                                    return Err(R::Launch(Box::new(crate::AgentLaunchError::caused(
                                        "paea-git-promote-trust-section", cause,
                                    ))));
                                }
                                effect::commit(
                                    self,
                                    project,
                                    prepared,
                                    identity,
                                    mandate_expires_at,
                                )
                            }
                            Operation::Reconcile => effect::reconcile(self, project, prepared),
                        }
                    })();
                    match trust.close() {
                        Ok(()) => inner,
                        Err(cause) => {
                            let cleanup = R::Storage(cause);
                            match inner {
                                // A known outcome survives the failed trust
                                // unlock in the typed observation envelope.
                                Ok(outcome) => Err(R::PaeaGitPromoteCall(Box::new(
                                    PaeaGitPromoteCallError::Effect {
                                        outcome: Some(outcome),
                                        cause: cleanup,
                                    },
                                ))),
                                Err(e) => Err(R::Cleanup {
                                    primary: Some(Box::new(e)),
                                    cleanup: Box::new(cleanup),
                                }),
                            }
                        }
                    }
                })();
                // A known outcome survives a failed project rollback in the
                // typed observation envelope; the plain join would drop it.
                match (result, rollback(project_tx)) {
                    (Ok(outcome), Ok(())) => Ok(outcome),
                    (Ok(outcome), Err(cleanup)) => Err(R::PaeaGitPromoteCall(Box::new(
                        PaeaGitPromoteCallError::Effect {
                            outcome: Some(outcome),
                            cause: cleanup,
                        },
                    ))),
                    (Err(e), cleanup) => finished(Err(e), cleanup),
                }
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
    use super::{
        meta, rollback, PaeaGitPromoteOutcome, PaeaGitPromoteReconciledEffect, PaeaGitPromoteReceipt,
        R, RootStore,
    };
    use rusqlite::{params, Transaction, TransactionBehavior};
    use std::{cell::Cell, error::Error, time::Instant};
    use swarm_enrollment_authority::paea_action::Identity;
    use swarm_protection_contract::Action;
    use swarm_wire_json::go_rfc3339;
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
    #[cfg(test)]
    mod post_effect_regression {
        use super::*;

        /// A recorded effect is a known observation even when custody close
        /// fails. This reproduces the historical loss without opening a Root,
        /// publishing a ref, or injecting a failure into live project storage.
        #[test]
        fn recorded_effect_survives_cleanup() {
            let outcome = PaeaGitPromoteOutcome::Recorded {
                intent_id: "intent".into(),
                action_digest: "digest".into(),
                target_ref: "refs/heads/release".into(),
                new_oid: "candidate".into(),
                replayed: false,
            };
            let error = finished_out(
                Ok(outcome.clone()),
                Err(R::Refused("fixture-custody-close")),
            ).expect_err("cleanup must remain a failed call");
            let envelope = super::super::PaeaGitPromoteCallError::Action(error);
            assert_eq!(envelope.known_outcome(), Some(&outcome),
                "cleanup erased an already-recorded Git outcome");
        }
    }

    /// Join an outcome with custody close, preserving an observed result in
    /// the typed envelope when cleanup fails. No IO, retry or rollback runs.
    pub(super) fn finished_out(
        result: Result<PaeaGitPromoteOutcome, R>,
        cleanup: Result<(), R>,
    ) -> Result<PaeaGitPromoteOutcome, R> {
        match (result, cleanup) {
            (Ok(v), Ok(())) => Ok(v),
            (Err(e), Ok(())) => Err(e),
            (Ok(outcome), Err(cause)) => Err(R::PaeaGitPromoteCall(Box::new(
                super::PaeaGitPromoteCallError::Effect {
                    outcome: Some(outcome), cause,
                },
            ))),
            (Err(e), Err(cleanup)) => Err(R::Cleanup {
                primary: Some(Box::new(e)),
                cleanup: Box::new(cleanup),
            }),
        }
    }
    /// Wrap a post-publication failure in the typed effect observation
    /// envelope: the reference transaction may have published while no durable
    /// outcome was observed, so the observation is the explicit unknown and
    /// never an absence claim. An independent close failure is joined below
    /// the primary cause; no error is suppressed into a success.
    fn observed_unknown(cause: R, cleanup: Result<(), R>) -> Result<PaeaGitPromoteOutcome, R> {
        let cause = match cleanup {
            Ok(()) => cause,
            Err(cleanup) => R::Cleanup {
                primary: Some(Box::new(cause)),
                cleanup: Box::new(cleanup),
            },
        };
        Err(R::PaeaGitPromoteCall(Box::new(
            super::PaeaGitPromoteCallError::Effect {
                outcome: super::PaeaGitPromoteCallError::find(&cause)
                    .and_then(super::PaeaGitPromoteCallError::known_outcome).cloned(),
                cause,
            },
        )))
    }
    /// Return a known recorded/replayed outcome, retaining it in the typed
    /// observation envelope when an independent close fails instead of
    /// dropping the result beside the close cause.
    fn observed_known(
        outcome: PaeaGitPromoteOutcome,
        cleanup: Result<(), R>,
    ) -> Result<PaeaGitPromoteOutcome, R> {
        match cleanup {
            Ok(()) => Ok(outcome),
            Err(cleanup) => Err(R::PaeaGitPromoteCall(Box::new(
                super::PaeaGitPromoteCallError::Effect {
                    outcome: Some(outcome),
                    cause: cleanup,
                },
            ))),
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
                Ok(()) => observed_known(outcome, rollback(tx)),
                // The commit outcome stays unknown and the original SQL cause
                // is retained typed; rollback is never claimed to have run.
                Err(cause) => {
                    let mut e = R::Launch(Box::new(crate::AgentLaunchError::caused(
                        "paea-git-promote-commit-uncertain",
                        cause,
                    )));
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
    /// Fixed admission lease for one guarded publication: the earliest
    /// absolute expiry of the signed identity and the validated current
    /// approval's mandate, mapped ONCE to a fixed monotonic deadline. The
    /// monotonic sample is taken BEFORE the UTC sample (conservative mapping:
    /// relative to the observed UTC sample, assuming trustworthy UTC during
    /// derivation; this is not a trusted-clock attestation), with a checked
    /// positive Duration conversion and checked_add; exact expiry refuses.
    /// Both the absolute expiry and the fixed deadline are rechecked together:
    /// a backward UTC jump cannot renew the monotonic deadline and a forward
    /// UTC jump is caught by the absolute check, while the absolute
    /// signed/mandate expiry remains the source of truth. This bounds
    /// controlled waiting only: no physical Git publication deadline,
    /// receiver-enforced lease or total callback/filesystem/SQL bound is
    /// claimed, and no margin, timeout, retry or new operator setting exists.
    ///
    /// FR : bail d'admission fixe, dérivé une seule fois de l'expiration
    /// absolue la plus ancienne (identité signée contre mandat validé) vers
    /// une échéance monotone conservatrice ; l'expiration absolue reste la
    /// vérité et un recul de l'horloge UTC ne renouvelle jamais l'échéance.
    pub(super) struct PublicationLease {
        /// Earliest absolute expiry (signed identity vs validated mandate).
        expires_at: time::OffsetDateTime,
        /// Fixed monotonic deadline derived once from the absolute expiry.
        deadline: Instant,
    }

    impl PublicationLease {
        /// Derive from explicit clock samples (deterministic and testable):
        /// the monotonic sample must be taken before the UTC sample. Exact
        /// expiry (no positive remaining lifetime) and past expiry refuse
        /// `paea-git-promote-lease-expired`; an unrepresentable monotonic
        /// deadline refuses `paea-git-promote-lease-representation`.
        pub(super) fn from_samples(
            signed: time::OffsetDateTime,
            mandate: Option<time::OffsetDateTime>,
            mono_now: Instant,
            utc_now: time::OffsetDateTime,
        ) -> Result<Self, R> {
            let earliest = match mandate {
                Some(mandate) if mandate < signed => mandate,
                _ => signed,
            };
            let remaining = std::time::Duration::try_from(earliest - utc_now)
                .ok()
                .filter(|remaining| !remaining.is_zero())
                .ok_or(R::Refused("paea-git-promote-lease-expired"))?;
            let deadline = mono_now
                .checked_add(remaining)
                .ok_or(R::Refused("paea-git-promote-lease-representation"))?;
            Ok(Self {
                expires_at: earliest,
                deadline,
            })
        }

        /// Production derivation with the same conservative clock ordering:
        /// monotonic sample first, UTC sample second.
        pub(super) fn derive(
            signed: time::OffsetDateTime,
            mandate: Option<time::OffsetDateTime>,
        ) -> Result<Self, R> {
            let mono_now = Instant::now();
            Self::from_samples(signed, mandate, mono_now, time::OffsetDateTime::now_utc())
        }

        /// The one fixed monotonic deadline, reused unchanged for the
        /// reference transaction, the nested prepared-callback read and the
        /// post-publication read — never restarted per read.
        pub(super) fn deadline(&self) -> Instant {
            self.deadline
        }

        /// Both-clock recheck, deterministic from explicit samples: expired
        /// when the absolute earliest expiry has passed (source of truth) OR
        /// the fixed monotonic deadline has been reached.
        pub(super) fn expired(&self, mono_now: Instant, utc_now: time::OffsetDateTime) -> bool {
            utc_now >= self.expires_at || mono_now >= self.deadline
        }

        /// Production recheck of both clocks with a fixed typed refusal; no
        /// margin, retry or renewal exists.
        pub(super) fn check(&self) -> Result<(), R> {
            if self.expired(Instant::now(), time::OffsetDateTime::now_utc()) {
                return Err(R::Refused("paea-git-promote-lease-expired"));
            }
            Ok(())
        }
    }

    /// Pure prepared-callback decision for one guarded publication, from one
    /// validated exact-ref read result and the post-read lease recheck. A
    /// failed read refuses with its original typed cause — never a
    /// synthesized missing ref, never a silent Abort and never a Commit, so a
    /// zero OLD never turns a deadline/process/read error into a creation. A
    /// lease recheck refusal likewise refuses. A successful absent ref
    /// permits creation only for the all-zero OLD marker; the observed
    /// candidate sets the recovery flag and aborts without a new update (the
    /// preexisting commit-recovery semantics, with no causality inferred
    /// beyond it); the expected OLD permits Commit; any other value refuses
    /// typed.
    pub(super) fn decide_from_read(
        read: Result<Option<String>, R>,
        lease: Result<(), R>,
        expected_old_oid: &str,
        candidate_oid: &str,
        already: &Cell<bool>,
    ) -> Result<crate::GitReferenceDecision, Box<dyn Error + Send + Sync>> {
        let observed = match read {
            Ok(observed) => observed,
            Err(cause) => return Err(Box::new(cause) as Box<dyn Error + Send + Sync>),
        };
        lease.map_err(|cause| Box::new(cause) as Box<dyn Error + Send + Sync>)?;
        match observed {
            None if zero_oid(expected_old_oid) => Ok(crate::GitReferenceDecision::Commit),
            None => Err(Box::new(std::io::Error::other(
                "paea-git-promote-old-missing",
            )) as Box<dyn Error + Send + Sync>),
            Some(current) => {
                if current == candidate_oid {
                    // Crash recovery: the reference is already at the
                    // candidate; record the known outcome, publish nothing.
                    already.set(true);
                    return Ok(crate::GitReferenceDecision::Abort);
                }
                if current != expected_old_oid {
                    return Err(Box::new(std::io::Error::other(
                        "paea-git-promote-old-changed",
                    )) as Box<dyn Error + Send + Sync>);
                }
                Ok(crate::GitReferenceDecision::Commit)
            }
        }
    }

    /// One exact guarded publication. Reserves the durable intent before any
    /// Git, verifies the actual OLD under Git's prepared lock via the decision
    /// callback, runs the reference transaction once and records the observed
    /// effect. A crash between intent and effect is recovered under OLD
    /// fencing: a reference already at the candidate records the known outcome
    /// without a new publication.
    ///
    /// Admission is bounded by one fixed monotonic publication lease derived
    /// once at entry from the earliest absolute expiry of the signed identity
    /// and the validated current approval's mandate (the raw persisted
    /// RFC3339 string supplied by the caller; `None` keeps the signed expiry
    /// alone, the preexisting no-mandate branch). The lease deadline is
    /// sampled monotonic-first and then UTC, so the remaining lifetime is
    /// fixed relative to that observed UTC sample, with trustworthy UTC during
    /// derivation assumed; later preflights cannot restart the lease;
    /// exact expiry refuses. Preflight repository/candidate opens keep the
    /// existing operator policy unchanged; after them BOTH the absolute
    /// expiry and the fixed monotonic deadline are rechecked before the
    /// transaction, and again inside the prepared callback after the nested
    /// Git read, before any Commit decision. The same fixed lease deadline
    /// (never a fresh lease) bounds the reference transaction, the nested
    /// read and the post-publication read; the effective per-command deadline
    /// stays the earlier of the saved operator timeout and the lease. A
    /// backward UTC jump cannot renew the fixed monotonic deadline and the
    /// absolute signed/mandate expiry remains the source of truth. This
    /// bounds controlled waiting only: no physical Git publication deadline,
    /// receiver-enforced lease or total callback/filesystem/SQL bound is
    /// claimed, and a scheduler/COMMIT pass past expiry stays an explicit
    /// unknown.
    ///
    /// FR : la publication est admise sous un bail monotone fixe dérivé de
    /// l'expiration absolue la plus ancienne (identité signée, mandat validé)
    /// ; les deux horloges sont revérifiées avant la transaction et après la
    /// lecture imbriquée, sans marge, relance ni nouveau réglage.
    pub(super) fn commit(
        service: &RootStore,
        project: &RootStore,
        prepared: &super::PaeaGitPromoteCall,
        identity: &Identity<'_>,
        mandate_expires_at: Option<String>,
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
        // One fixed lease for the whole publication. A present mandate expiry
        // is parsed here with the existing Go RFC3339 parser; an invalid
        // persisted value typed-refuses with the original parse cause. The
        // earliest absolute expiry is mapped once to the fixed monotonic
        // deadline (monotonic sample before UTC sample, checked positive
        // conversion and checked_add, exact expiry refuses).
        let mandate_expiry = match &mandate_expires_at {
            Some(raw) => Some(go_rfc3339::parse(raw).map_err(|cause| {
                R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-mandate-expiry",
                    cause,
                )))
            })?),
            None => None,
        };
        let lease = PublicationLease::derive(identity.expires_at(), mandate_expiry)?;
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
        // Slow preflights (intent reservation, replay, repository open,
        // candidate inspect) ran under the existing operator policy; recheck
        // BOTH the absolute expiry and the fixed monotonic deadline before
        // the transaction. No new lifetime is minted: the deadline stays the
        // one fixed at entry.
        if let Err(cause) = lease.check() {
            return finished_out(Err(cause), repository.close());
        }
        // One exact reference transaction against the actual destination. The
        // decision callback verifies the direct ref state under Git's prepared
        // lock; protocol ACKs are observations only. The hardened runner strips
        // Git environment overrides and optional locks for the child process.
        // The transaction deadline is the earlier of the saved per-command
        // operator timeout and the fixed lease deadline; the SAME fixed lease
        // deadline (never a fresh lease) also bounds the nested read below and
        // the post-publication read.
        let settings = match service.read_settings() {
            Ok(settings) => settings,
            Err(cause) => return finished_out(Err(cause), repository.close()),
        };
        let destination = repository.directory_path();
        let already = Cell::new(false);
        let (git, saved_deadline) =
            match crate::managed_repository::git_command(&destination, &[], &settings) {
                Ok(value) => value,
                Err(e) => {
                    return finished_out(
                        Err(R::Launch(Box::new(e))),
                        repository.close(),
                    )
                }
            };
        let transaction_deadline = crate::managed_repository::combine_deadlines(
            saved_deadline,
            Some(lease.deadline()),
        );
        let run = crate::run_git_reference_transaction(
            git,
            &prepared.proposal,
            transaction_deadline,
            None,
            || {
                // Only a successful read can reach a Commit decision. Do not
                // construct and discard a second lease error after a failed
                // read; that original typed failure already refuses the decision.
                // Preserve the parent's earlier operator deadline during the
                // synchronous callback; its nested command cannot extend it.
                let read = read_current_ref_with_deadline(
                    &destination, target_ref, &settings, transaction_deadline,
                );
                let admission = if read.is_ok() { lease.check() } else { Ok(()) };
                decide_from_read(
                    read,
                    admission,
                    expected_old_oid,
                    candidate_oid,
                    &already,
                )
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
            // The transaction was requested, so whether Git published the
            // reference is unknown at this layer; the typed reference error
            // keeps its own observation beside the explicit unknown.
            Err(e) => return observed_unknown(e, repository.close()),
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
        // receipt binds this observation, not a promise. The same fixed lease
        // deadline (never a fresh lease) bounds this post-publication read;
        // the saved operator timeout may still fire first.
        let observed_new = crate::managed_repository::git_with_deadline(
            &destination,
            &["--git-dir", ".", "rev-parse", "--verify", target_ref],
            &settings,
            Some(lease.deadline()),
        )
        .map_err(|e| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-new-oid",
                e,
            )))
        });
        let observed_new = match observed_new {
            Ok(raw) => match String::from_utf8(raw) {
                Ok(value) => value.trim().to_owned(),
                Err(cause) => return observed_unknown(
                    R::Launch(Box::new(crate::AgentLaunchError::caused(
                        "paea-git-promote-new-oid-encoding", cause,
                    ))),
                    repository.close(),
                ),
            },
            // The transaction was acknowledged: the publication stays an
            // explicit unknown until the effect row is durably recorded.
            Err(e) => return observed_unknown(e, repository.close()),
        };
        if observed_new != *candidate_oid {
            // Acknowledged publication with an unreadable or mismatched ref:
            // the durable state is unknown, never an established absence.
            return observed_unknown(
                R::Refused("paea-git-promote-ref-candidate-mismatch"),
                repository.close(),
            );
        }
        let closed = repository.close();
        if let Err(cause) = lease.check() {
            // Preserve both the established late-expiry stage and the typed
            // lease refusal. Publication remains unknown, never absent.
            return observed_unknown(R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-authority-expired", cause,
            ))), closed);
        }
        let receipt = PaeaGitPromoteReceipt {
            target_ref: target_ref.clone(),
            old_oid: expected_old_oid.clone(),
            new_oid: observed_new,
        };
        // After an acknowledged publication every record failure stays an
        // explicit unknown, and a recorded outcome survives a failed close.
        match record(service, &intent_id, &action_digest, &receipt, already.get()) {
            Ok(outcome) => observed_known(outcome, closed),
            Err(cause) => observed_unknown(cause, closed),
        }
    }
    /// Join a reconcile journal observation with a later failure. Any journal
    /// known record survives in the typed effect envelope beside the original
    /// cause (an independent close failure is joined below it); a failed read
    /// never becomes an observed `None` and no reconcile result is synthesized.
    pub(super) fn reconcile_failure(
        journal: Option<PaeaGitPromoteOutcome>,
        cause: R,
        cleanup: Result<(), R>,
    ) -> Result<PaeaGitPromoteOutcome, R> {
        let cause = match cleanup {
            Ok(()) => cause,
            Err(cleanup) => R::Cleanup {
                primary: Some(Box::new(cause)),
                cleanup: Box::new(cleanup),
            },
        };
        Err(R::PaeaGitPromoteCall(Box::new(
            super::PaeaGitPromoteCallError::Effect {
                outcome: journal,
                cause,
            },
        )))
    }
    /// Full lowercase-hex SHA1/SHA256 object id, mirroring the established
    /// 40/64-hex Git object representation validated across this crate.
    /// Shortened, uppercase or non-hex values refuse instead of being observed.
    fn full_oid(oid: &str) -> bool {
        matches!(oid.len(), 40 | 64)
            && oid
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
    /// Parse the fixed `for-each-ref --format=%(refname) %(objectname)
    /// %(symref)` output into the exact direct-ref observation for
    /// `target_ref`. Only a line whose refname equals `target_ref` exactly
    /// counts (prefix results, such as slash-children matched by the same
    /// pattern, are not equivalent); a non-empty `%(symref)` field marks a
    /// symbolic ref, which is rejected rather than represented as an absent direct ref. `Ok(None)` means Git successfully listed no exact direct
    /// ref; malformed lines, extra fields and non-full lowercase-hex object
    /// ids refuse instead of becoming observed values or silent `None`s.
    pub(super) fn parse_exact_direct_ref(
        raw: &[u8],
        target_ref: &str,
    ) -> Result<Option<String>, R> {
        let text = std::str::from_utf8(raw).map_err(|cause| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-ref-encoding", cause,
            )))
        })?;
        let mut exact = None;
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let name = parts.next().ok_or(R::Refused("paea-git-promote-ref-malformed"))?;
            let oid = parts.next().ok_or(R::Refused("paea-git-promote-ref-malformed"))?;
            let symref = parts.next();
            if parts.next().is_some() || !name.starts_with("refs/") {
                return Err(R::Refused("paea-git-promote-ref-malformed"));
            }
            if !full_oid(oid) {
                return Err(R::Refused("paea-git-promote-ref-oid"));
            }
            if name != target_ref { continue; }
            if exact.is_some() {
                return Err(R::Refused("paea-git-promote-ref-duplicate"));
            }
            if symref.is_some() {
                return Err(R::Refused("paea-git-promote-ref-symbolic"));
            }
            exact = Some(oid.to_owned());
        }
        Ok(exact)
    }
    /// Deadline-capable sibling of `read_current_ref`: the exact same
    /// validated read sequence (quiet symbolic-ref kind probe with its
    /// documented exit 1, then the exact for-each-ref listing and
    /// `parse_exact_direct_ref`) under the caller's ONE fixed lease deadline
    /// passed to every command; the effective per-command deadline stays the
    /// earlier of the saved operator timeout and this lease deadline, so an
    /// existing operator timeout may fire first. The lease deadline is never
    /// restarted per command. Typed causes are preserved exactly as in
    /// `read_current_ref`; no update or journal IO occurs. The caller owns
    /// and retains the descriptor-backed directory lifetime.
    pub(super) fn read_current_ref_with_deadline(
        destination: &std::path::Path,
        target_ref: &str,
        settings: &crate::EngineSettingsDocument,
        lease_deadline: Option<Instant>,
    ) -> Result<Option<String>, R> {
        // for-each-ref may omit dangling symbolic refs. Query their kind first;
        // Git's documented quiet exit 1 means non-symbolic, not absent. Only
        // that exact completed command result permits the subsequent listing.
        // All other process/settings/deadline failures retain their causes.
        // https://git-scm.com/docs/git-symbolic-ref#_notes
        match crate::managed_repository::git_with_deadline(
            destination,
            &["--git-dir", ".", "symbolic-ref", "--quiet", "--no-recurse", target_ref],
            settings,
            lease_deadline,
        ) {
            Ok(_) => return Err(R::Refused("paea-git-promote-ref-symbolic")),
            Err(cause) => {
                let non_symbolic = match &cause {
                    crate::AgentLaunchError::Cause { code: "managed-git-command", cause } =>
                        matches!(cause.downcast_ref::<crate::GitReportError>(),
                            Some(crate::GitReportError::Command { exit_code: Some(1), timed_out: false, .. })),
                    _ => false,
                };
                if !non_symbolic {
                    return Err(R::Launch(Box::new(crate::AgentLaunchError::caused(
                        "paea-git-promote-ref-kind", cause,
                    ))));
                }
            }
        }
        crate::managed_repository::git_with_deadline(
            destination,
            &[
                "--git-dir",
                ".",
                "for-each-ref",
                "--format=%(refname) %(objectname) %(symref)",
                "--",
                target_ref,
            ],
            settings,
            lease_deadline,
        )
        .map_err(|e| {
            R::Launch(Box::new(crate::AgentLaunchError::caused(
                "paea-git-promote-ref-read",
                e,
            )))
        })
        .and_then(|raw| {
            parse_exact_direct_ref(&raw, target_ref).map_err(|e| {
                R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-ref-parse",
                    e,
                )))
            })
        })
    }

    /// Read the exact current direct reference with the existing hardened Git
    /// runner and saved per-command policy. A quiet symbolic-ref kind query
    /// rejects symbolic refs, including dangling refs omitted by for-each-ref.
    /// Its documented exit 1 permits the listing, not an absence conclusion.
    /// No update or journal IO occurs.
    /// Success with None means a successful listing without the exact ref;
    /// symbolic/malformed refs and process errors retain typed failures.
    /// The caller owns and retains the descriptor-backed directory lifetime.
    /// Behavior is unchanged for Reconcile: this wrapper delegates to the
    /// deadline-capable sibling with no lease deadline.
    pub(super) fn read_current_ref(
        destination: &std::path::Path,
        target_ref: &str,
        settings: &crate::EngineSettingsDocument,
    ) -> Result<Option<String>, R> {
        read_current_ref_with_deadline(destination, target_ref, settings, None)
    }
    /// Reconciliation: read the durable journal state and the actual current
    /// direct Git reference without any new publication, reservation or
    /// journal write. A stored effect replays its historical observation and
    /// stays historical even when the ref has since moved or been deleted; a
    /// missing effect row never proves an absent past effect. The ref is read
    /// with the existing hardened runner (`for-each-ref` with explicit
    /// `%(symref)` validation, so a symbolic ref is never treated as direct);
    /// `observed_oid` is `None` only when Git successfully lists no exact
    /// direct ref. Any later failure keeps its original cause in the typed
    /// effect envelope beside any journal record and never becomes a
    /// synthesized reconcile result with a fake observed `None`.
    ///
    /// FR : la réconciliation lit le journal durable et la référence Git
    /// directe actuelle sans publier, réserver ni écrire le journal ; un
    /// échec de lecture conserve sa cause et ne devient jamais un résultat
    /// fabriqué avec un `None` observé.
    pub(super) fn reconcile(
        service: &RootStore,
        project: &RootStore,
        prepared: &super::PaeaGitPromoteCall,
    ) -> Result<PaeaGitPromoteOutcome, R> {
        let Action::Git { target_ref, .. } = &prepared.proposal.action else {
            return Err(R::Refused("paea-git-promote-git-required"));
        };
        let (_, pin) = meta(&service.db)?;
        if pin != prepared.pin {
            return Err(R::Refused("paea-git-promote-project-pin"));
        }
        let intent = service
            .paea_read_git_intent(prepared.canonical.intent_id(), &prepared.pin)
            .map_err(|e| {
                R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-intent-read",
                    e,
                )))
            })?;
        let (intent_id, recorded_effect) = match intent {
            None => (prepared.canonical.intent_id().to_owned(), None),
            Some(record) => {
                if record.action_digest != prepared.canonical.digest() {
                    return Err(R::Refused("paea-git-promote-intent-conflict"));
                }
                let effect = match existing_effect(&service.db, &record.intent_id)? {
                    Some((digest, target, new, raw)) => {
                        let stored: PaeaGitPromoteReceipt = serde_json::from_str(&raw)
                            .map_err(|cause| R::Launch(Box::new(crate::AgentLaunchError::caused(
                                "paea-git-promote-effect-decode", cause,
                            ))))?;
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
                        Some(PaeaGitPromoteReconciledEffect {
                            action_digest: digest,
                            target_ref: target,
                            new_oid: new,
                        })
                    }
                    None => None,
                };
                (record.intent_id, effect)
            }
        };
        // The journal observation that must survive any later failure in the
        // typed envelope: a recorded row stays the known historical effect and
        // no row stays no outcome at all, never a synthesized reconcile result.
        let journal = recorded_effect.as_ref().map(|effect| {
            PaeaGitPromoteOutcome::Recorded {
                intent_id: intent_id.clone(),
                action_digest: effect.action_digest.clone(),
                target_ref: effect.target_ref.clone(),
                new_oid: effect.new_oid.clone(),
                replayed: true,
            }
        });
        // Read-only custody and ref observation under the existing authority
        // and Root witnesses: no reservation, no publication, no journal write.
        let repository = match service.paea_open_git_repository(
            project,
            &prepared.work,
            &prepared.child,
            &prepared.pin,
        ) {
            Ok(value) => value,
            Err(e) => {
                return reconcile_failure(
                    journal,
                    R::Launch(Box::new(crate::AgentLaunchError::caused(
                        "paea-git-promote-open",
                        e,
                    ))),
                    Ok(()),
                );
            }
        };
        let Action::Git { repository_id, .. } = &prepared.proposal.action else {
            return reconcile_failure(journal, R::Refused("paea-git-promote-git-required"), repository.close());
        };
        if repository.binding().work_id() != prepared.work
            || repository.binding().repository_id() != repository_id
        {
            return reconcile_failure(journal, R::Refused("paea-git-promote-repository-binding"), repository.close());
        }
        let settings = match service.read_settings() {
            Ok(value) => value,
            Err(cause) => return reconcile_failure(journal, cause, repository.close()),
        };
        let destination = repository.directory_path();
        let observed = read_current_ref(&destination, target_ref, &settings);
        match observed {
            Ok(observed_oid) => {
                let outcome = PaeaGitPromoteOutcome::Reconciled {
                    intent_id,
                    target_ref: target_ref.clone(),
                    observed_oid,
                    recorded_effect,
                };
                observed_known(outcome, repository.close())
            }
            Err(cause) => reconcile_failure(journal, cause, repository.close()),
        }
    }
}

/// Additional typed observation regressions using in-memory causes. The
/// historical custody-loss defect was separately reproduced before integration
/// by effect::post_effect_regression, using only the pre-existing API.
#[cfg(test)]
mod promote_observation_tests {
    use super::*;

    /// One recorded fixture outcome; arbitrary test strings that never leave
    /// this module and are never rendered by the fixed Display.
    fn recorded() -> PaeaGitPromoteOutcome {
        PaeaGitPromoteOutcome::Recorded {
            intent_id: "fixture-intent".into(),
            action_digest: "fixture-digest".into(),
            target_ref: "refs/heads/fixture".into(),
            new_oid: "fixture-oid".into(),
            replayed: false,
        }
    }

    /// Collect SQLite extended codes through the same typed branch walker the
    /// envelope uses; no message parsing is involved.
    fn sqlite_codes(error: &(dyn Error + 'static)) -> Vec<i32> {
        let mut pending = vec![error];
        let mut seen: Vec<*const dyn Error> = Vec::new();
        let mut codes = Vec::new();
        while let Some(error) = pending.pop() {
            let pointer = error as *const dyn Error;
            if seen.iter().any(|old| std::ptr::eq(*old, pointer)) {
                continue;
            }
            seen.push(pointer);
            if let Some(sql) = error.downcast_ref::<rusqlite::Error>() {
                if let Some(code) = sql.sqlite_error() {
                    codes.push(code.extended_code);
                }
            }
            let links = crate::native_error_links(error);
            pending.extend(links.children.into_iter().map(|(_, cause)| cause));
            if let Some(source) = error.source() {
                pending.push(source);
            }
        }
        codes
    }

    /// A known recorded outcome survives a late close failure on the envelope
    /// and stays findable through the typed Root wrapper and cleanup nesting.
    #[test]
    fn late_cleanup_retains_known_recorded_outcome() {
        let outcome = recorded();
        let direct = PaeaGitPromoteCallError::Effect {
            outcome: Some(outcome.clone()),
            cause: R::Refused("fixture-late-close"),
        };
        assert_eq!(direct.known_outcome(), Some(&outcome));
        assert!(!direct.effect_unknown());
        let nested = PaeaGitPromoteCallError::Action(R::PaeaGitPromoteCall(Box::new(
            PaeaGitPromoteCallError::Effect {
                outcome: Some(outcome.clone()),
                cause: R::Cleanup {
                    primary: None,
                    cleanup: Box::new(R::Refused("fixture-close")),
                },
            },
        )));
        assert_eq!(nested.known_outcome(), Some(&outcome));
        assert!(!nested.effect_unknown());
        assert!(matches!(
            nested.backend_error(),
            Some(R::PaeaGitPromoteCall(_))
        ));
    }

    /// The explicit unknown stays unknown through generic IO wrappers and the
    /// fixed safe diagnostics expose it without any receipt or private data.
    #[test]
    fn unknown_envelope_stays_unknown_through_wrappers() {
        let envelope = PaeaGitPromoteCallError::Effect {
            outcome: None,
            cause: R::Refused("fixture-post-effect"),
        };
        let wrapped: Box<dyn Error + Send + Sync> = Box::new(std::io::Error::other(
            R::PaeaGitPromoteCall(Box::new(envelope)),
        ));
        let found = PaeaGitPromoteCallError::find(wrapped.as_ref()).expect("envelope retained");
        assert!(found.effect_unknown());
        assert!(found.known_outcome().is_none());
        #[cfg(target_os = "linux")]
        {
            let links = crate::native_error_links(found);
            assert_eq!(found.code(), "paea-git-promote-unknown");
            assert!(links.labels.iter().any(|l| l == "paea-git-promote-unknown"));
        }
    }

    /// A commit-uncertain record failure keeps the original rusqlite cause and
    /// an independent rollback refusal; the observation stays explicitly
    /// unknown and no outcome is invented.
    #[test]
    fn sql_commit_cause_and_independent_cleanup_survive_envelope() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let sql = db
            .execute_batch("SELECT * FROM fixture_missing_promote_effects")
            .unwrap_err();
        let expected = sql.sqlite_error().unwrap().extended_code;
        let error = PaeaGitPromoteCallError::Effect {
            outcome: None,
            cause: R::Cleanup {
                primary: Some(Box::new(R::Launch(Box::new(crate::AgentLaunchError::caused(
                    "paea-git-promote-commit-uncertain",
                    sql,
                ))))),
                cleanup: Box::new(R::Refused("fixture-rollback")),
            },
        };
        assert!(error.effect_unknown());
        assert!(error.known_outcome().is_none());
        let cause = error.backend_error().expect("envelope retains its cause");
        let links = crate::native_error_links(cause);
        assert!(links.children.iter().any(|(kind, _)| *kind == "primary"));
        assert!(links.children.iter().any(|(kind, _)| *kind == "cleanup"));
        assert!(sqlite_codes(cause).contains(&expected));
    }

    /// A pre-effect refusal invents neither a known outcome nor an unknown
    /// publication, even under cleanup joins. Uses only the pre-existing API,
    /// so it also compiles and passes on a baseline without the envelope.
    #[test]
    fn pre_effect_failure_keeps_no_known_outcome() {
        let direct = PaeaGitPromoteCallError::Action(R::Refused(
            "paea-git-promote-repository-binding",
        ));
        assert!(direct.known_outcome().is_none());
        let joined = PaeaGitPromoteCallError::Action(R::Cleanup {
            primary: Some(Box::new(R::Refused("paea-git-promote-current-candidate"))),
            cleanup: Box::new(R::Refused("fixture-close")),
        });
        assert!(joined.known_outcome().is_none());
    }

    /// The shared custody join must preserve a result observed before cleanup.
    #[test]
    fn finished_out_retains_known_outcome_beside_cleanup() {
        let expected = recorded();
        let joined = effect::finished_out(Ok(expected.clone()), Err(R::Refused("fixture-close")));
        let error = PaeaGitPromoteCallError::Action(joined.err().unwrap());
        assert_eq!(error.known_outcome(), Some(&expected));
        assert!(!error.effect_unknown());
    }
}

/// Source-only regressions for the reconciliation observation contract. These
/// tests open no Root, run no Git and touch no journal; the public native
/// reconcile flow (real custody, real `for-each-ref`, real journal) remains
/// separately unverified. No assertion here grants authority or proves a past
/// or current effect.
#[cfg(test)]
mod reconcile_observation_tests {
    use super::*;

    fn recorded_row() -> PaeaGitPromoteOutcome {
        PaeaGitPromoteOutcome::Recorded {
            intent_id: "fixture-intent".into(),
            action_digest: "fixture-digest".into(),
            target_ref: "refs/heads/fixture".into(),
            new_oid: "fixture-oid".into(),
            replayed: true,
        }
    }

    fn envelope(outcome: PaeaGitPromoteOutcome) -> PaeaGitPromoteCallError {
        PaeaGitPromoteCallError::Effect {
            outcome: Some(outcome),
            cause: R::Refused("fixture-late-close"),
        }
    }

    /// Only the exact refname is observed; slash-children listed by the same
    /// for-each-ref pattern are never prefix-equivalent observations.
    #[test]
    fn reconcile_parse_exact_name_ignores_prefix_children() {
        let raw = b"refs/heads/fixture/child 1111111111111111111111111111111111111111 \nrefs/heads/fixture 2222222222222222222222222222222222222222 \n";
        assert_eq!(
            effect::parse_exact_direct_ref(raw, "refs/heads/fixture").unwrap(),
            Some("2222222222222222222222222222222222222222".to_owned())
        );
    }

    /// Git successfully listing no exact line is the explicit None; a symbolic
    /// ref is rejected rather than conflated with absence.
    #[test]
    fn reconcile_parse_absent_and_symbolic_are_not_object_observations() {
        assert_eq!(
            effect::parse_exact_direct_ref(b"", "refs/heads/fixture").unwrap(),
            None
        );
        assert!(effect::parse_exact_direct_ref(
            b"refs/heads/fixture 3333333333333333333333333333333333333333 refs/heads/other\n",
            "refs/heads/fixture"
        ).is_err());
    }

    /// Malformed lines and shortened or non-hex object ids refuse instead of
    /// becoming observed values or silent Nones.
    #[test]
    fn reconcile_parse_refuses_malformed_and_non_full_oids() {
        assert!(
            effect::parse_exact_direct_ref(b"refs/heads/fixture\n", "refs/heads/fixture").is_err()
        );
        assert!(effect::parse_exact_direct_ref(
            b"refs/heads/fixture abc def\n",
            "refs/heads/fixture"
        )
        .is_err());
        assert!(effect::parse_exact_direct_ref(
            b"refs/heads/fixture 2222 \n",
            "refs/heads/fixture"
        )
        .is_err());
    }

    /// A valid listing is the control before malformed output injections.
    #[test]
    fn reconcile_parser_rejects_corrupt_stream_with_utf8_cause() {
        assert_eq!(effect::parse_exact_direct_ref(b"", "refs/heads/fixture").unwrap(), None);
        assert!(effect::parse_exact_direct_ref(b"garbage\n", "refs/heads/fixture").is_err());
        let error = effect::parse_exact_direct_ref(&[255], "refs/heads/fixture").unwrap_err();
        let mut pending: Vec<&(dyn Error + 'static)> = vec![&error];
        let mut found = false;
        while let Some(error) = pending.pop() {
            found |= error.downcast_ref::<std::str::Utf8Error>().is_some();
            pending.extend(crate::native_error_links(error).children.into_iter().map(|(_, e)| e));
            if let Some(cause) = error.source() { pending.push(cause); }
        }
        assert!(found, "original UTF8 cause retained");
    }

    /// Exercise the production ref reader with real Git on a new inert bare
    /// repository. This is component IO, not signed Root/public-call evidence.
    #[test]
    fn reconcile_real_git_reads_changes_absence_and_refuses_errors() {
        let temporary = tempfile::tempdir_in(crate::fixture_area()).unwrap();
        let settings = crate::EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture".into(), "2026-11-17T00:00:00Z".into(),
        ).unwrap();
        let git = |args: &[&str]| crate::managed_repository::git(temporary.path(), args, &settings).unwrap();
        git(&["init", "--bare", "."]);
        let target = "refs/tags/fixture";
        assert_eq!(effect::read_current_ref(temporary.path(), target, &settings).unwrap(), None);
        let tree = String::from_utf8(git(&["--git-dir", ".", "hash-object", "-w", "-t", "tree", "--stdin"])).unwrap();
        let commit = |message: &str| String::from_utf8(git(&[
            "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
            "--git-dir", ".", "commit-tree", tree.trim(), "-m", message,
        ])).unwrap().trim().to_owned();
        let first = commit("first");
        let second = commit("second");
        assert_ne!(first, second);
        git(&["--git-dir", ".", "update-ref", target, &first]);
        assert_eq!(effect::read_current_ref(temporary.path(), target, &settings).unwrap(), Some(first.clone()));
        git(&["--git-dir", ".", "update-ref", target, &second, &first]);
        assert_eq!(effect::read_current_ref(temporary.path(), target, &settings).unwrap(), Some(second.clone()));
        git(&["--git-dir", ".", "update-ref", "-d", target, &second]);
        assert_eq!(effect::read_current_ref(temporary.path(), target, &settings).unwrap(), None);
        git(&["--git-dir", ".", "update-ref", "refs/tags/actual", &first]);
        git(&["--git-dir", ".", "symbolic-ref", target, "refs/tags/actual"]);
        assert!(effect::read_current_ref(temporary.path(), target, &settings).is_err());
        git(&["--git-dir", ".", "symbolic-ref", target, "refs/tags/missing"]);
        assert!(effect::read_current_ref(temporary.path(), target, &settings).is_err(),
            "dangling symbolic reference must never become observed absence");
        let not_repository = tempfile::tempdir_in(crate::fixture_area()).unwrap();
        assert!(effect::read_current_ref(not_repository.path(), target, &settings).is_err());
    }

    /// A failed ref read preserves its original cause in the typed envelope and
    /// retains the journal record; it never fabricates an observed None.
    #[test]
    fn reconcile_failure_keeps_cause_and_journal_record() {
        let journal = recorded_row();
        let joined = effect::reconcile_failure(
            Some(journal.clone()),
            R::Refused("fixture-ref-read"),
            Err(R::Refused("fixture-close")),
        )
        .expect_err("failure must stay a failure");
        let error = PaeaGitPromoteCallError::find(&joined).expect("envelope retained");
        assert_eq!(error.known_outcome(), Some(&journal));
        assert!(!error.effect_unknown());
        let cause = error.backend_error().expect("cause retained");
        let links = crate::native_error_links(cause);
        assert!(links.children.iter().any(|(kind, _)| *kind == "primary"));
        assert!(links.children.iter().any(|(kind, _)| *kind == "cleanup"));
    }

    /// Without a journal record the failed read stays the explicit unknown; no
    /// reconcile result is synthesized and no absence is ever claimed.
    #[test]
    fn reconcile_failure_without_journal_stays_unknown() {
        let joined = effect::reconcile_failure(None, R::Refused("fixture-ref-read"), Ok(()))
            .expect_err("failure must stay a failure");
        let error = PaeaGitPromoteCallError::find(&joined).expect("envelope retained");
        assert!(error.known_outcome().is_none());
        assert!(error.effect_unknown());
        assert_eq!(error.code(), "paea-git-promote-unknown");
    }

    /// Unchanged history beside a moved ref: the recorded row stays the known
    /// historical effect and the ref observation never rewrites the receipt.
    #[test]
    fn reconciled_moved_ref_keeps_recorded_history_and_observation() {
        let outcome = PaeaGitPromoteOutcome::Reconciled {
            intent_id: "fixture-intent".into(),
            target_ref: "refs/heads/fixture".into(),
            observed_oid: Some("moved-oid".into()),
            recorded_effect: Some(PaeaGitPromoteReconciledEffect {
                action_digest: "fixture-digest".into(),
                target_ref: "refs/heads/fixture".into(),
                new_oid: "fixture-oid".into(),
            }),
        };
        let error = envelope(outcome);
        assert_eq!(error.effect_status(), "recorded");
        let observation = error.public_effect_observation();
        assert_eq!(observation["state"], "recorded");
        assert_eq!(observation["new_oid"], "fixture-oid");
        assert_eq!(observation["observed_oid"], "moved-oid");
    }

    /// A journal missing with the candidate ref present stays unknown: the
    /// observed candidate never proves this intent caused it and a missing
    /// effect row never proves no past effect.
    #[test]
    fn reconciled_journal_missing_with_candidate_ref_stays_unknown() {
        let outcome = PaeaGitPromoteOutcome::Reconciled {
            intent_id: "fixture-intent".into(),
            target_ref: "refs/heads/fixture".into(),
            observed_oid: Some("candidate".into()),
            recorded_effect: None,
        };
        let error = envelope(outcome);
        assert_eq!(error.effect_status(), "unknown");
        let observation = error.public_effect_observation();
        assert_eq!(observation["state"], "unknown");
        assert_eq!(observation["observed_oid"], "candidate");
        assert_eq!(observation["journal_state"], "no_effect_row");
        assert!(observation.get("new_oid").is_none());
    }

    /// Git successfully listing no exact direct ref is the explicit None,
    /// distinct from any failed read, which never reaches a Reconciled value.
    #[test]
    fn reconciled_absent_ref_is_explicit_none_not_a_failed_read() {
        let outcome = PaeaGitPromoteOutcome::Reconciled {
            intent_id: "fixture-intent".into(),
            target_ref: "refs/heads/fixture".into(),
            observed_oid: None,
            recorded_effect: None,
        };
        let error = envelope(outcome);
        assert_eq!(error.effect_status(), "unknown");
        let observation = error.public_effect_observation();
        assert_eq!(observation["state"], "unknown");
        assert!(observation["observed_oid"].is_null());
    }
}


#[cfg(test)]
mod paea_publication_lease_guards {
    use super::effect::{decide_from_read, PublicationLease};
    use super::R;
    use std::cell::Cell;
    use std::time::{Duration, Instant};
    use time::OffsetDateTime;

    /// Fixed UTC fixture builder from actual time crate types; no clock is
    /// sampled for the absolute fixtures below.
    fn utc(y: i32, m: u8, d: u8, h: u8, mi: u8, s: u8) -> OffsetDateTime {
        time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(y, time::Month::try_from(m).expect("month"), d)
                .expect("date"),
            time::Time::from_hms(h, mi, s).expect("time"),
        )
        .assume_utc()
    }

    fn lease_or_panic(lease: Result<PublicationLease, R>) -> PublicationLease {
        match lease {
            Ok(lease) => lease,
            Err(_) => panic!("valid control lease must derive"),
        }
    }

    /// The lease bounds at the EARLIEST of the signed identity expiry and the
    /// validated mandate expiry; with no mandate the signed expiry alone
    /// bounds (the preexisting no-mandate branch).
    #[test]
    fn lease_takes_earliest_of_signed_and_mandate() {
        let mono = Instant::now();
        let base = utc(2026, 1, 1, 0, 0, 0);
        let signed = utc(2031, 1, 1, 0, 0, 0);
        let mandate = utc(2029, 6, 1, 0, 0, 0);
        // Earlier mandate is the bound: alive just before it, expired at it,
        // even though the signed identity is still valid then.
        let lease = lease_or_panic(PublicationLease::from_samples(
            signed,
            Some(mandate),
            mono,
            base,
        ));
        assert!(!lease.expired(mono, utc(2029, 5, 31, 23, 59, 59)));
        assert!(lease.expired(mono, mandate));
        // No mandate: the signed expiry alone bounds the lease.
        let lease = lease_or_panic(PublicationLease::from_samples(signed, None, mono, base));
        assert!(!lease.expired(mono, mandate));
        assert!(lease.expired(mono, signed));
        // Earlier signed expiry wins when the mandate is later.
        let lease = lease_or_panic(PublicationLease::from_samples(
            mandate,
            Some(signed),
            mono,
            base,
        ));
        assert!(lease.expired(mono, mandate));
        assert!(!lease.expired(mono, base));
    }

    /// Exact expiry rejects and past expiry rejects, for the signed expiry,
    /// the mandate expiry and the mandate==UTC-sample edge.
    #[test]
    fn exact_and_past_expiry_refuse() {
        let mono = Instant::now();
        let base = utc(2026, 1, 1, 0, 0, 0);
        lease_or_panic(PublicationLease::from_samples(
            utc(2031, 1, 1, 0, 0, 0), None, mono, base,
        ));
        assert!(matches!(
            PublicationLease::from_samples(base, None, mono, base),
            Err(_)
        ));
        assert!(matches!(
            PublicationLease::from_samples(utc(2025, 1, 1, 0, 0, 0), None, mono, base),
            Err(_)
        ));
        assert!(matches!(
            PublicationLease::from_samples(
                utc(2031, 1, 1, 0, 0, 0),
                Some(utc(2025, 6, 1, 0, 0, 0)),
                mono,
                base
            ),
            Err(_)
        ));
        assert!(matches!(
            PublicationLease::from_samples(utc(2031, 1, 1, 0, 0, 0), Some(base), mono, base),
            Err(_)
        ));
    }

    /// A backward UTC jump cannot renew the fixed monotonic deadline and a
    /// forward UTC jump is caught by the absolute check; the monotonic
    /// deadline boundary itself is exact.
    #[test]
    fn backward_utc_cannot_renew_and_forward_utc_is_caught() {
        let mono = Instant::now();
        let base = utc(2026, 1, 1, 0, 0, 0);
        let signed = utc(2027, 1, 1, 0, 0, 0);
        let lease = lease_or_panic(PublicationLease::from_samples(signed, None, mono, base));
        let deadline = lease.deadline();
        // Forward UTC jump past the absolute expiry: caught by the absolute
        // check even though the monotonic deadline has not been reached.
        assert!(lease.expired(mono, signed));
        // Backward UTC jump (before the derivation sample) cannot renew the
        // monotonic deadline: at the fixed deadline the lease is expired
        // despite the rolled-back clock, and just before it, it is not.
        let just_before = deadline
            .checked_sub(Duration::from_nanos(1))
            .expect("earlier monotonic instant");
        assert!(!lease.expired(just_before, utc(2025, 1, 1, 0, 0, 0)));
        assert!(lease.expired(deadline, utc(2025, 1, 1, 0, 0, 0)));
    }

    /// A real Git control is read with the normal policy and a future fixed
    /// deadline. An explicitly expired fixed deadline refuses with the
    /// original typed Git failure; no sleep or new duration is used by reads.
    #[test]
    fn bounded_ref_read_uses_fixed_deadline_and_preserves_default_behavior() {
        let temporary = tempfile::tempdir_in(crate::fixture_area()).unwrap();
        let settings = crate::EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture".into(), "2026-11-17T00:00:00Z".into(),
        ).unwrap();
        crate::managed_repository::git(temporary.path(), &["init", "--bare", "."], &settings).unwrap();
        assert_eq!(super::effect::read_current_ref(temporary.path(), "refs/tags/fixture", &settings).unwrap(), None);
        let base = utc(2026, 1, 1, 0, 0, 0);
        let lease = lease_or_panic(PublicationLease::from_samples(
            utc(2031, 1, 1, 0, 0, 0), None, Instant::now(), base,
        ));
        assert_eq!(super::effect::read_current_ref_with_deadline(
            temporary.path(), "refs/tags/fixture", &settings, Some(lease.deadline()),
        ).unwrap(), None);
        let expired = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        let cause = super::effect::read_current_ref_with_deadline(
            temporary.path(), "refs/tags/fixture", &settings, Some(expired),
        ).unwrap_err();
        let mut pending: Vec<&(dyn std::error::Error + 'static)> = vec![&cause];
        let mut seen: Vec<*const dyn std::error::Error> = Vec::new();
        let mut found_git = false;
        while let Some(error) = pending.pop() {
            let pointer = error as *const dyn std::error::Error;
            if seen.iter().any(|old| std::ptr::eq(*old, pointer)) { continue; }
            seen.push(pointer);
            found_git |= error.downcast_ref::<crate::GitReportError>().is_some();
            pending.extend(crate::native_error_links(error).children.into_iter().map(|(_, e)| e));
            if let Some(source) = error.source() { pending.push(source); }
        }
        assert!(found_git, "original typed runner failure remains traversable");
        assert_eq!(super::effect::read_current_ref(temporary.path(), "refs/tags/fixture", &settings).unwrap(), None);
    }

    /// Exercise the lease's own checked representation failure on Linux, not
    /// merely Instant::checked_add. The finite binary search is over the u64
    /// seconds representation, a technical boundary, not an execution policy.
    #[test]
    fn representation_refusals_never_wrap() {
        let mono = Instant::now();
        let base = utc(2026, 1, 1, 0, 0, 0);
        lease_or_panic(PublicationLease::from_samples(base + time::Duration::seconds(2), None, mono, base));
        assert!(mono.checked_add(Duration::from_secs(u64::MAX)).is_none());
        let (mut low, mut high) = (0_u64, u64::MAX);
        while high - low > 1 {
            let middle = low + (high - low) / 2;
            if mono.checked_add(Duration::from_secs(middle)).is_some() { low = middle; }
            else { high = middle; }
        }
        let boundary = mono.checked_add(Duration::from_secs(low)).unwrap();
        assert!(matches!(PublicationLease::from_samples(
            base + time::Duration::seconds(2), None, boundary, base,
        ), Err(R::Refused("paea-git-promote-lease-representation"))));
    }

    /// A failed nested read with a zero OLD is a typed refusal with its
    /// original cause: never a creation, never a silent Abort. The valid
    /// control is the successful absent ref, which commits only for zero OLD.
    #[test]
    fn zero_old_read_error_never_commits() {
        let already = Cell::new(false);
        let committed = decide_from_read(Ok(None), Ok(()), &"0".repeat(40), &"f".repeat(40), &already);
        assert!(matches!(committed, Ok(crate::GitReferenceDecision::Commit)));
        let outcome = decide_from_read(
            Err(R::Refused("fixture-nested-read")), Ok(()), &"0".repeat(40), &"f".repeat(40), &already,
        ).unwrap_err();
        assert!(matches!(outcome.downcast_ref::<R>(), Some(R::Refused("fixture-nested-read"))));
        assert!(!already.get());
    }

    /// A lease recheck refusal after the nested read refuses the decision:
    /// never a Commit and never a silent Abort, for zero and nonzero OLD.
    #[test]
    fn lease_refusal_after_read_never_commits() {
        for expected in ["0".repeat(40), "1".repeat(40)] {
            let already = Cell::new(false);
            let observed = if expected == "0".repeat(40) { None } else { Some(expected.clone()) };
            assert!(matches!(decide_from_read(
                Ok(observed.clone()), Ok(()), &expected, &"f".repeat(40), &already,
            ), Ok(crate::GitReferenceDecision::Commit)));
            let outcome = decide_from_read(
                Ok(observed), Err(R::Refused("fixture-lease-expired")), &expected, &"f".repeat(40), &already,
            ).unwrap_err();
            assert!(matches!(outcome.downcast_ref::<R>(), Some(R::Refused("fixture-lease-expired"))));
            assert!(!already.get());
        }
    }

    /// Exact validated-observation decisions: the observed candidate sets the
    /// recovery flag and aborts without a new update; the expected OLD
    /// commits; any other value and any absent ref with a nonzero OLD refuse
    /// typed.
    #[test]
    fn decision_matches_candidate_expected_and_refuses_others() {
        let candidate = "f".repeat(40);
        let expected = "1".repeat(40);
        let already = Cell::new(false);
        let outcome = decide_from_read(
            Ok(Some(candidate.clone())),
            Ok(()),
            &expected,
            &candidate,
            &already,
        );
        assert!(matches!(outcome, Ok(crate::GitReferenceDecision::Abort)));
        assert!(already.get());
        let already = Cell::new(false);
        let outcome = decide_from_read(
            Ok(Some(expected.clone())),
            Ok(()),
            &expected,
            &candidate,
            &already,
        );
        assert!(matches!(outcome, Ok(crate::GitReferenceDecision::Commit)));
        assert!(!already.get());
        let already = Cell::new(false);
        let outcome = decide_from_read(
            Ok(Some("2".repeat(40))),
            Ok(()),
            &expected,
            &candidate,
            &already,
        );
        assert!(matches!(outcome, Err(_)));
        assert!(!already.get());
        let outcome = decide_from_read(Ok(None), Ok(()), &expected, &candidate, &already);
        assert!(matches!(outcome, Err(_)));
        assert!(!already.get());
    }
}
