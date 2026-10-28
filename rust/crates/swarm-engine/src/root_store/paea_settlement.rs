//! Fictitious settlement kernel over a separately opened native Root datastore.
//! No daemon, payment provider, admission-context attestation or OS isolation.
use super::mission_supervision_store as sql;
mod call;
mod policy;
use super::{finished, RootBinding, RootStore, RootStoreError as R};
pub use call::PaeaSettlementCallError;
use rusqlite::{params, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{error::Error as StdError, fmt};
use swarm_enrollment_authority::paea_action::{Identity, Operation, Request, Service};
use swarm_identity_contract::{digest, name, Principal, Scope};
use swarm_protection_contract::{
    canonicalize, Action, AdmissionContext, CanonicalAction, Proposal,
};

const META: &str = "CREATE TABLE paea_settlement_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),schema_version INTEGER NOT NULL CHECK(schema_version=1),project_id TEXT NOT NULL,project_digest TEXT NOT NULL)";
const DEBTS: &str = "CREATE TABLE paea_settlement_debts(invoice_id TEXT PRIMARY KEY,invoice_version TEXT NOT NULL,amount_minor INTEGER NOT NULL CHECK(amount_minor>0),currency TEXT NOT NULL,beneficiary TEXT NOT NULL)";
const EFFECTS: &str = "CREATE TABLE paea_settlement_effects(intent_id TEXT PRIMARY KEY,invoice_id TEXT NOT NULL UNIQUE REFERENCES paea_settlement_debts(invoice_id),action_digest TEXT NOT NULL,receipt TEXT NOT NULL)";

/// An actual ledger observation, not a signed independent receipt or approval.
/// Canonical context remains the supplied action data; observed authority fields
/// come separately from the held current grant, never from its claimed context.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaeaSettlementReceipt {
    pub intent_id: String,
    pub action_digest: String,
    pub invoice_id: String,
    pub canonical_action: Vec<u8>,
    pub authority_enrollment_id: String,
    pub authority_project_digest: String,
    pub authority_principal: Principal,
    pub authority_scope: Scope,
    pub authority_head_sequence: i64,
    pub authority_head_digest: String,
    pub trust_revision: i64,
    /// Clock sampled before the write/COMMIT attempt, not exact publication time.
    pub commit_requested_at: String,
}
/// Known recorded/replayed state. Replayed never denotes a new payment.
#[derive(Clone, Debug, Serialize)]
pub enum PaeaSettlementOutcome {
    /// Durable row observed; replayed=true performs no new commitment.
    Recorded {
        receipt: PaeaSettlementReceipt,
        replayed: bool,
    },
    /// Under the fenced ledger read, no row exists for this business intent.
    /// This is an explicit reconciliation result, not an empty success or grant.
    ReconciledNotCommitted {
        intent_id: String,
        invoice_id: String,
    },
}

/// Preserve what is known even when a later storage/release error occurs.
#[derive(Clone, Debug)]
pub enum PaeaSettlementObservation {
    /// This call did not attempt SQL commitment; inspect cleanup errors too.
    NotAttempted,
    /// SQL commitment was attempted but its outcome was not established.
    Uncertain,
    /// A previous/current result was observed; never erase on a late error.
    Known(Box<PaeaSettlementOutcome>),
}
/// Safe fixed code with original typed cause and independent cleanup branches.
/// Display/Debug redact private SQL/path/action data. source() returns the primary
/// cause; consumers must also inspect cleanup and observation, not infer rollback.
pub struct PaeaSettlementError {
    pub code: &'static str,
    pub cause: Box<dyn StdError + Send + Sync>,
    pub cleanup: Vec<R>,
    pub observation: PaeaSettlementObservation,
}
impl PaeaSettlementError {
    /// Retain an original cause before any commitment attempt, without formatting it.
    fn caused(code: &'static str, cause: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            code,
            cause: Box::new(cause),
            cleanup: Vec::new(),
            observation: PaeaSettlementObservation::NotAttempted,
        }
    }
    /// Fixed refusal, never caller text; no callback or mutation is inferred.
    fn refused(code: &'static str) -> Self {
        Self::caused(code, R::Refused(code))
    }
}
impl fmt::Display for PaeaSettlementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Debug for PaeaSettlementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for PaeaSettlementError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.cause.as_ref())
    }
}
type Result<T> = std::result::Result<T, PaeaSettlementError>;

/// Two retained physical storage witnesses and immutable exact action data.
/// No authority is granted. Keep this alive through signing and current held use;
/// close explicitly even after refusal. A restart needs a newly bound request.
pub struct PaeaSettlementRequest {
    project: RootBinding,
    ledger: RootBinding,
    request: Request,
    proposal: Proposal,
    context: AdmissionContext,
    canonical: CanonicalAction,
}
impl PaeaSettlementRequest {
    /// Exact permission data for separately authorized enrollment; no IO or grant.
    pub fn request(&self) -> &Request {
        &self.request
    }
    /// Canonical data to review, not an attestation of its claimed context.
    pub fn canonical(&self) -> &CanonicalAction {
        &self.canonical
    }
    /// Effective canonical context: Commit policy fields are storage-attributed;
    /// remaining fields are declarations. Reconcile retains historical context.
    pub fn context(&self) -> &AdmissionContext {
        &self.context
    }

    /// Release both held witnesses, retaining independent failures. No data mutation.
    pub fn close(self) -> std::result::Result<(), R> {
        finished(self.project.close(), self.ledger.close())
    }
}

/// Validate all three exact schemas without creating or adopting partial history.
fn schema(db: &rusqlite::Connection) -> std::result::Result<bool, R> {
    let mut count = 0;
    for (table, ddl) in [
        ("paea_settlement_meta", META),
        ("paea_settlement_debts", DEBTS),
        ("paea_settlement_effects", EFFECTS),
    ] {
        if sql::auxiliary_schema(db, table, ddl, "paea-settlement-schema")
            .map_err(|e| R::SqlTransaction(Box::new(e)))?
        {
            count += 1;
        }
    }
    match count {
        0 => Ok(false),
        3 => Ok(true),
        _ => Err(R::Refused("paea-settlement-schema-partial")),
    }
}
/// Read the unique configured project association; absence/corruption is refusal.
fn meta(db: &rusqlite::Connection) -> Result<(String, String)> {
    if !schema(db).map_err(|e| PaeaSettlementError::caused("paea-settlement-schema", e))? {
        return Err(PaeaSettlementError::refused(
            "paea-settlement-uninitialized",
        ));
    }
    let (project, digest_value): (String, String) = sql::optional(
        db,
        "SELECT project_id,project_digest FROM main.paea_settlement_meta WHERE singleton=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|e| PaeaSettlementError::caused("paea-settlement-meta", e))?
    .ok_or_else(|| PaeaSettlementError::refused("paea-settlement-meta-missing"))?;
    if !name(project.as_bytes()) || !digest(digest_value.as_bytes()) {
        return Err(PaeaSettlementError::refused("paea-settlement-meta-invalid"));
    }
    Ok((project, digest_value))
}
/// Extract already validated exact payment terms; Git cannot enter this service.
fn terms(proposal: &Proposal) -> Result<(&str, &str, i64, &str, &str)> {
    proposal
        .validate()
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-proposal", e))?;
    match &proposal.action {
        Action::Payment {
            invoice_id,
            invoice_version,
            amount_minor,
            currency,
            beneficiary_account_id,
        } => Ok((
            invoice_id,
            invoice_version,
            amount_minor
                .parse()
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-amount", e))?,
            currency,
            beneficiary_account_id,
        )),
        _ => Err(PaeaSettlementError::refused(
            "paea-settlement-payment-required",
        )),
    }
}
/// Check the actual current plan under the caller's IMMEDIATE project writer.
/// Read-only work projection; no migration, grant or fallback is created.
fn current_plan(project: &RootStore, identity: &Identity<'_>) -> Result<()> {
    let scope = identity.scope();
    let version: i64 = sql::optional(
        &project.db,
        "SELECT projection_version FROM main.agent_identity_plan_heads WHERE work_id=?",
        [&scope.work_id],
        |r| r.get(0),
    )
    .map_err(|e| PaeaSettlementError::caused("paea-settlement-plan-read", e))?
    .ok_or_else(|| PaeaSettlementError::refused("paea-settlement-plan-missing"))?;
    let projection = match version {
        1 => crate::IdentityPlanProjection::V1,
        2 => crate::IdentityPlanProjection::V2,
        _ => {
            return Err(PaeaSettlementError::refused(
                "paea-settlement-plan-projection",
            ))
        }
    };
    let stamp = crate::read_identity_plan_stamp(&project.db, &scope.work_id, projection)
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-plan-read", e))?;
    if !stamp.matches_scope(scope) {
        return Err(PaeaSettlementError::refused("paea-settlement-plan-stale"));
    }
    Ok(())
}
/// Explicit rollback without an implicit Drop retry. Preserve rollback failure;
/// an already auto-committed/rolled-back connection needs no further operation.
fn rollback(mut tx: Transaction<'_>) -> std::result::Result<(), R> {
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    if tx.is_autocommit() {
        return Ok(());
    }
    tx.execute_batch("ROLLBACK").map_err(R::Sql)
}
/// Commit provisioning explicitly. A failed COMMIT retains uncertainty and its
/// independent rollback cause; it never claims no schema/data was published.
fn provisioned(tx: Transaction<'_>, result: Result<()>) -> Result<()> {
    match result {
        Ok(()) => match tx.execute_batch("COMMIT") {
            Ok(()) => rollback(tx)
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-initialize-release", e)),
            Err(cause) => {
                let mut e = PaeaSettlementError::caused(
                    "paea-settlement-initialize-commit-uncertain",
                    cause,
                );
                e.observation = PaeaSettlementObservation::Uncertain;
                if let Err(cleanup) = rollback(tx) {
                    e.cleanup.push(cleanup);
                }
                Err(e)
            }
        },
        Err(mut e) => {
            if let Err(cleanup) = rollback(tx) {
                e.cleanup.push(cleanup);
            }
            Err(e)
        }
    }
}
/// Combine a known outcome and independent explicit transaction releases.
fn released(
    mut result: Result<PaeaSettlementOutcome>,
    cleanup: std::result::Result<(), R>,
) -> Result<PaeaSettlementOutcome> {
    if let Err(cleanup) = cleanup {
        match &mut result {
            Ok(known) => {
                let mut e = PaeaSettlementError::caused("paea-settlement-release", cleanup);
                e.observation = PaeaSettlementObservation::Known(Box::new(known.clone()));
                return Err(e);
            }
            Err(e) => e.cleanup.push(cleanup),
        }
    }
    result
}
/// Validate a stored canonical action independently of the current request.
/// Bind observed project/policy metadata to retained ledger/canonical facts and
/// validate enrollment/time representation without a new freshness cutoff.
/// This detects inconsistent local corruption, not malicious rewriting by an
/// owner. No signature, historical authority or declared principal is attested.
fn validate_receipt(
    receipt: &PaeaSettlementReceipt,
    intent: &str,
    invoice: &str,
    action: &str,
    project_digest: &str,
) -> Result<()> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Stored {
        schema_version: u32,
        canonicalization: String,
        action: Action,
        context: AdmissionContext,
    }
    let stored: Stored = serde_json::from_slice(&receipt.canonical_action)
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-receipt-canonical", e))?;
    if stored.canonicalization != "paea-json-v1" {
        return Err(PaeaSettlementError::refused(
            "paea-settlement-receipt-profile",
        ));
    }
    let proposal = Proposal {
        schema_version: stored.schema_version,
        request_id: "observed-settlement".into(),
        action: stored.action,
    };
    let canonical = canonicalize(&proposal, &stored.context)
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-receipt-canonical", e))?;
    let (actual_invoice, ..) = terms(&proposal)?;
    time::OffsetDateTime::parse(
        &receipt.commit_requested_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| PaeaSettlementError::caused("paea-settlement-receipt-incompatible", e))?;
    if receipt.intent_id != intent
        || receipt.invoice_id != invoice
        || actual_invoice != invoice
        || receipt.action_digest != action
        || canonical.intent_id() != intent
        || canonical.digest() != action
        || canonical.bytes() != receipt.canonical_action
        || !name(receipt.authority_enrollment_id.as_bytes())
        || receipt.authority_project_digest != project_digest
        || receipt.trust_revision != stored.context.policy_revision
        || !receipt.authority_principal.valid()
        || receipt.authority_principal.kind != "human"
        || !receipt.authority_scope.valid()
        || !digest(receipt.authority_project_digest.as_bytes())
        || !digest(receipt.authority_head_digest.as_bytes())
        || receipt.authority_head_sequence <= 0
        || receipt.trust_revision <= 0
    {
        return Err(PaeaSettlementError::refused(
            "paea-settlement-receipt-incompatible",
        ));
    }
    Ok(())
}

impl RootStore {
    /// Explicit trusted provisioning of a dedicated ledger, not a peer operation.
    /// Requires independently protected writable storage and owner-supplied debt
    /// terms/project pin. Validates all data before one atomic SQL transaction;
    /// refuses existing/partial schema, never adopts first-request data or updates
    /// existing debts. No Root initialization, account creation or live provider.
    /// This library entry alone establishes neither operator identity nor custody.
    pub fn paea_initialize_settlement(
        &self,
        project_id: &str,
        project_digest: &str,
        debts: &[Proposal],
    ) -> Result<()> {
        if !self.writable || !name(project_id.as_bytes()) || !digest(project_digest.as_bytes()) {
            return Err(PaeaSettlementError::refused(
                "paea-settlement-provisioning-binding",
            ));
        }
        self.instance()
            .map_err(|e| PaeaSettlementError::caused("paea-settlement-provisioning-instance", e))?;
        let mut ids = std::collections::BTreeSet::new();
        for debt in debts {
            let (invoice, ..) = terms(debt)?;
            if !ids.insert(invoice) {
                return Err(PaeaSettlementError::refused(
                    "paea-settlement-debt-duplicate",
                ));
            }
        }
        let tx =
            Transaction::new_unchecked(&self.db, TransactionBehavior::Immediate).map_err(|e| {
                PaeaSettlementError::caused("paea-settlement-initialize-reservation", e)
            })?;
        let result = (|| {
            if schema(&tx).map_err(|e| PaeaSettlementError::caused("paea-settlement-schema", e))? {
                return Err(PaeaSettlementError::refused(
                    "paea-settlement-already-initialized",
                ));
            }
            tx.execute_batch(&format!("{META};{DEBTS};{EFFECTS};"))
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-schema-create", e))?;
            sql::execute(
                &tx,
                "INSERT INTO main.paea_settlement_meta VALUES(1,1,?,?)",
                params![project_id, project_digest],
            )
            .map_err(|e| PaeaSettlementError::caused("paea-settlement-meta-create", e))?;
            for debt in debts {
                let (id, version, amount, currency, beneficiary) = terms(debt)?;
                sql::execute(
                    &tx,
                    "INSERT INTO main.paea_settlement_debts VALUES(?,?,?,?,?)",
                    params![id, version, amount, currency, beneficiary],
                )
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-debt-create", e))?;
            }
            Ok(())
        })();
        provisioned(tx, result)
    }
    /// Prepare exact permission data from this project and a distinct opened
    /// ledger. Retains both inode witnesses and canonical action, grants nothing.
    /// Commit policy revision/digest come from current opened Trust storage;
    /// supplied context is validated and cloned without mutation. Reconcile keeps
    /// its historical context. Other fields remain declarations: upstream control
    /// still owes mandate/approval/lineage/agent attribution before granting rights.
    /// Caller must explicitly close the request on every success/refusal path.
    pub fn paea_settlement_request(
        &self,
        ledger: &RootStore,
        proposal: &Proposal,
        context: &AdmissionContext,
        operation: Operation,
    ) -> Result<PaeaSettlementRequest> {
        terms(proposal)?;
        let context = match operation {
            Operation::Commit => policy::attribute(self, context)?,
            Operation::Reconcile => context.clone(),
        };
        let canonical = canonicalize(proposal, &context)
            .map_err(|e| PaeaSettlementError::caused("paea-settlement-canonical", e))?;
        let project_binding = self
            .bind_root()
            .map_err(|e| PaeaSettlementError::caused("paea-settlement-project-witness", e))?;
        let ledger_binding = match ledger.bind_root() {
            Ok(b) => b,
            Err(cause) => {
                let mut e = PaeaSettlementError::caused("paea-settlement-ledger-witness", cause);
                if let Err(cleanup) = project_binding.close() {
                    e.cleanup.push(cleanup);
                }
                return Err(e);
            }
        };
        let result = (|| {
            let project_instance = project_binding
                .instance()
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-project-witness", e))?;
            let ledger_instance = ledger_binding
                .instance()
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-ledger-witness", e))?;
            if project_instance.directory == ledger_instance.directory
                || project_instance.database == ledger_instance.database
            {
                return Err(PaeaSettlementError::refused(
                    "paea-settlement-separate-ledger-required",
                ));
            }
            let (project_id, _) = meta(&ledger.db)?;
            if project_id != context.project_id {
                return Err(PaeaSettlementError::refused(
                    "paea-settlement-project-mismatch",
                ));
            }
            let nonce = crate::generate_identifier("paea-settlement-")
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-nonce", e))?;
            let root=crate::trust::digest(&serde_json::json!({"domain":"paea-settlement-roots-v1","project":project_instance,"ledger":ledger_instance,"nonce":nonce})).map_err(|e|PaeaSettlementError::caused("paea-settlement-root-digest",e))?;
            Request::new(
                &root,
                Service::FictitiousPayment,
                operation,
                canonical.digest(),
            )
            .map_err(|e| PaeaSettlementError::caused("paea-settlement-request", e))
        })();
        match result {
            Ok(request) => Ok(PaeaSettlementRequest {
                project: project_binding,
                ledger: ledger_binding,
                request,
                proposal: proposal.clone(),
                context: context.clone(),
                canonical,
            }),
            Err(mut e) => {
                if let Err(cleanup) = finished(project_binding.close(), ledger_binding.close()) {
                    e.cleanup.push(cleanup);
                }
                Err(e)
            }
        }
    }
    /// Consume current borrowed PAEA authority for an exact fictitious settlement
    /// or reconciliation. Two IMMEDIATE reservations fence current project plan
    /// and ledger uniqueness; no cross-store write atomicity is claimed. Refuse
    /// wrong physical roots, permission, project pin, plan, expiry or debt terms.
    /// Existing identical action returns a replay; changed content for the same
    /// invoice returns the old known outcome in an error, never another payment.
    /// A commit error is uncertain; explicit reconciliation under a fresh grant
    /// is required. Personal/business after-use errors remain outside this result.
    /// No public transport, OS isolation, upstream admission or independent proof.
    pub fn paea_settle(
        &self,
        project: &RootStore,
        prepared: &PaeaSettlementRequest,
        identity: &Identity<'_>,
    ) -> Result<PaeaSettlementOutcome> {
        if !self.writable
            || !project.writable
            || !prepared
                .ledger
                .matches(self)
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-ledger-witness", e))?
            || !prepared
                .project
                .matches(project)
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-project-witness", e))?
            || identity.request().permission() != prepared.request.permission()
            || identity.request().service() != Service::FictitiousPayment
        {
            return Err(PaeaSettlementError::refused("paea-settlement-binding"));
        }
        let project_tx = Transaction::new_unchecked(&project.db, TransactionBehavior::Immediate)
            .map_err(|e| PaeaSettlementError::caused("paea-settlement-project-reservation", e))?;
        let trust = match project.directory.reserve_trust(false) {
            Ok(reservation) => reservation,
            Err(cause) => {
                return released(
                    Err(PaeaSettlementError::caused(
                        "paea-settlement-trust-reservation",
                        cause,
                    )),
                    rollback(project_tx),
                )
            }
        };
        let result = (|| {
            current_plan(project, identity)?;
            policy::current_actor(project, identity, &prepared.context)?;
            self.settlement_locked(prepared, identity)
        })();
        let result = released(result, trust.close().map_err(R::Storage));
        released(result, rollback(project_tx))
    }
    /// Execute under the project's retained writer, always explicitly releasing
    /// this ledger transaction and preserving commitment/cleanup observations.
    fn settlement_locked(
        &self,
        prepared: &PaeaSettlementRequest,
        identity: &Identity<'_>,
    ) -> Result<PaeaSettlementOutcome> {
        let tx = Transaction::new_unchecked(&self.db, TransactionBehavior::Immediate)
            .map_err(|e| PaeaSettlementError::caused("paea-settlement-ledger-reservation", e))?;
        let mut commit = false;
        let result = (|| {
            let (project_id, project_digest) = meta(&tx)?;
            if project_id != prepared.context.project_id
                || project_digest != identity.project_digest()
                || identity.principal().kind != "human"
                || time::OffsetDateTime::now_utc() >= identity.expires_at()
            {
                return Err(PaeaSettlementError::refused(
                    "paea-settlement-current-authority",
                ));
            }
            let (id, version, amount, currency, beneficiary) = terms(&prepared.proposal)?;
            let existing:Option<(String,String,String,String)>=sql::optional(&tx,"SELECT intent_id,invoice_id,action_digest,receipt FROM main.paea_settlement_effects WHERE invoice_id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|PaeaSettlementError::caused("paea-settlement-effect-read",e))?;
            if let Some((intent, invoice, action, raw)) = existing {
                let receipt: PaeaSettlementReceipt = serde_json::from_str(&raw).map_err(|e| {
                    PaeaSettlementError::caused("paea-settlement-receipt-decode", e)
                })?;
                validate_receipt(&receipt, &intent, &invoice, &action, &project_digest)?;
                let incompatible = receipt.intent_id != prepared.canonical.intent_id()
                    || receipt.action_digest != prepared.canonical.digest();
                let old = PaeaSettlementOutcome::Recorded {
                    receipt,
                    replayed: true,
                };
                if incompatible {
                    let mut e =
                        PaeaSettlementError::refused("paea-settlement-business-intent-conflict");
                    e.observation = PaeaSettlementObservation::Known(Box::new(old));
                    return Err(e);
                }
                return Ok(old);
            }
            if prepared.request.operation() != Operation::Commit {
                return Ok(PaeaSettlementOutcome::ReconciledNotCommitted {
                    intent_id: prepared.canonical.intent_id().into(),
                    invoice_id: id.into(),
                });
            }
            let stored:Option<(String,i64,String,String)>=sql::optional(&tx,"SELECT invoice_version,amount_minor,currency,beneficiary FROM main.paea_settlement_debts WHERE invoice_id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|PaeaSettlementError::caused("paea-settlement-debt-read",e))?;
            if stored != Some((version.into(), amount, currency.into(), beneficiary.into())) {
                return Err(PaeaSettlementError::refused(
                    "paea-settlement-debt-incompatible",
                ));
            }
            let now = time::OffsetDateTime::now_utc();
            if now >= identity.expires_at() {
                return Err(PaeaSettlementError::refused(
                    "paea-settlement-authority-expired",
                ));
            }
            let receipt = PaeaSettlementReceipt {
                intent_id: prepared.canonical.intent_id().into(),
                action_digest: prepared.canonical.digest().into(),
                invoice_id: id.into(),
                canonical_action: prepared.canonical.bytes().into(),
                authority_enrollment_id: identity.enrollment_id().into(),
                authority_project_digest: identity.project_digest().into(),
                authority_principal: identity.principal().clone(),
                authority_scope: identity.scope().clone(),
                authority_head_sequence: identity.authority_head().sequence,
                authority_head_digest: identity.authority_head().digest.clone(),
                trust_revision: identity.trust_revision(),
                commit_requested_at: now
                    .format(&time::format_description::well_known::Rfc3339)
                    .map_err(|e| PaeaSettlementError::caused("paea-settlement-time", e))?,
            };
            let raw = serde_json::to_string(&receipt)
                .map_err(|e| PaeaSettlementError::caused("paea-settlement-receipt-encode", e))?;
            let n = sql::execute(
                &tx,
                "INSERT INTO main.paea_settlement_effects VALUES(?,?,?,?)",
                params![
                    receipt.intent_id,
                    receipt.invoice_id,
                    receipt.action_digest,
                    raw
                ],
            )
            .map_err(|e| PaeaSettlementError::caused("paea-settlement-effect-write", e))?;
            if n != 1 {
                return Err(PaeaSettlementError::refused(
                    "paea-settlement-effect-not-recorded",
                ));
            }
            if time::OffsetDateTime::now_utc() >= identity.expires_at() {
                return Err(PaeaSettlementError::refused(
                    "paea-settlement-authority-expired",
                ));
            }
            commit = true;
            Ok(PaeaSettlementOutcome::Recorded {
                receipt,
                replayed: false,
            })
        })();
        if commit {
            match result {
                Ok(outcome) => match tx.execute_batch("COMMIT") {
                    Ok(()) => released(Ok(outcome), rollback(tx)),
                    Err(cause) => {
                        let mut e =
                            PaeaSettlementError::caused("paea-settlement-commit-uncertain", cause);
                        e.observation = PaeaSettlementObservation::Uncertain;
                        if let Err(cleanup) = rollback(tx) {
                            e.cleanup.push(cleanup);
                        }
                        Err(e)
                    }
                },
                Err(e) => released(Err(e), rollback(tx)),
            }
        } else {
            released(result, rollback(tx))
        }
    }
}
