//! Durable Git business-intent reservation, not a publication or admission service.
//! Only explicit trusted provisioning establishes the journal's project pin.
use super::{mission_supervision_store as sql, RootStore, RootStoreError as R};
use rusqlite::{params, Connection, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use swarm_identity_contract::{digest, name};
use swarm_protection_contract::{canonicalize, Action, AdmissionContext, Proposal};

const META: &str = "CREATE TABLE paea_git_intent_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),schema_version INTEGER NOT NULL CHECK(schema_version=1),project_id TEXT NOT NULL,project_digest TEXT NOT NULL)";
const INTENTS: &str = "CREATE TABLE paea_git_intents(intent_id TEXT PRIMARY KEY,action_digest TEXT NOT NULL,canonical_action BLOB NOT NULL)";

/// Durable pending intent. This is neither proof that Git ran nor an authority.
/// No terminal effect state is inferred from this reservation; reconciliation of
/// the actual direct reference and protected custody remains the service's duty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaeaGitIntent {
    pub intent_id: String,
    pub action_digest: String,
    pub canonical_action: Vec<u8>,
}
/// Only New denotes an insertion by this call. Existing retains the original
/// bytes even if a new candidate/context is requested for the same business key.
/// Neither variant grants permission. Existing MUST NOT trigger a new Git commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaeaGitIntentReservation {
    New(PaeaGitIntent),
    Existing(PaeaGitIntent),
}
/// SQL observation only; an uncertain reservation does not authorize Git.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaeaGitIntentObservation {
    NotAttempted,
    /// COMMIT was attempted; an independent read is required before proceeding.
    Uncertain,
    /// Preserve the observed original row after any later cleanup failure.
    Known(PaeaGitIntentReservation),
}
/// Fixed safe code, original typed cause and independent rollback error. Private
/// paths/action bytes are omitted from Display/Debug; callers must inspect the
/// observation and cleanup as well as source(). No error proves no Git effect.
pub struct PaeaGitIntentError {
    pub code: &'static str,
    pub cause: Box<dyn Error + Send + Sync>,
    pub cleanup: Vec<R>,
    pub observation: PaeaGitIntentObservation,
}
impl PaeaGitIntentError {
    /// Keep a concrete cause without formatting private storage information.
    fn caused(code: &'static str, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            code,
            cause: Box::new(cause),
            cleanup: vec![],
            observation: PaeaGitIntentObservation::NotAttempted,
        }
    }
    /// Closed refusal code; no mutation or authority is implied.
    fn refused(code: &'static str) -> Self {
        Self::caused(code, R::Refused(code))
    }
}
impl fmt::Display for PaeaGitIntentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Debug for PaeaGitIntentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PaeaGitIntentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
type Result<T> = std::result::Result<T, PaeaGitIntentError>;

/// Refuse partial/altered schema and extra triggers/indexes using the existing
/// authoritative auxiliary-schema verifier; absence never creates tables.
fn schema(db: &Connection) -> Result<bool> {
    let mut count = 0;
    for (table, ddl) in [
        ("paea_git_intent_meta", META),
        ("paea_git_intents", INTENTS),
    ] {
        if sql::auxiliary_schema(db, table, ddl, "paea-git-intent-schema")
            .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-schema", e))?
        {
            count += 1;
        }
    }
    match count {
        0 => Ok(false),
        2 => Ok(true),
        _ => Err(PaeaGitIntentError::refused(
            "paea-git-intent-schema-partial",
        )),
    }
}
/// Read independently provisioned project pin. A first request cannot adopt it.
fn meta(db: &Connection) -> Result<(String, String)> {
    if !schema(db)? {
        return Err(PaeaGitIntentError::refused("paea-git-intent-uninitialized"));
    }
    let (project, pin): (String, String) = sql::optional(
        db,
        "SELECT project_id,project_digest FROM main.paea_git_intent_meta WHERE singleton=1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-meta", e))?
    .ok_or_else(|| PaeaGitIntentError::refused("paea-git-intent-meta-missing"))?;
    if !name(project.as_bytes()) || !digest(pin.as_bytes()) {
        return Err(PaeaGitIntentError::refused("paea-git-intent-meta-invalid"));
    }
    Ok((project, pin))
}
/// Check stored canonical bytes by reconstructing their typed Git action and
/// recomputing both identities. Detects inconsistent local corruption, not a
/// malicious owner's consistent rewrite or authenticity of declared context.
fn validate(record: &PaeaGitIntent, project: &str) -> Result<()> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Stored {
        schema_version: u32,
        canonicalization: String,
        action: Action,
        context: AdmissionContext,
    }
    let stored: Stored = serde_json::from_slice(&record.canonical_action)
        .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-canonical", e))?;
    if stored.canonicalization != "paea-json-v1"
        || stored.context.project_id != project
        || !matches!(stored.action, Action::Git { .. })
    {
        return Err(PaeaGitIntentError::refused("paea-git-intent-binding"));
    }
    let proposal = Proposal {
        schema_version: stored.schema_version,
        request_id: "stored-git-intent".into(),
        action: stored.action,
    };
    let canonical = canonicalize(&proposal, &stored.context)
        .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-canonical", e))?;
    if canonical.bytes() != record.canonical_action
        || canonical.digest() != record.action_digest
        || canonical.intent_id() != record.intent_id
    {
        return Err(PaeaGitIntentError::refused("paea-git-intent-binding"));
    }
    Ok(())
}
/// Read one exact business key; missing is explicit None, malformed is an error.
fn read(db: &Connection, intent: &str, project: &str) -> Result<Option<PaeaGitIntent>> {
    let row = sql::optional(db,"SELECT intent_id,action_digest,canonical_action FROM main.paea_git_intents WHERE intent_id=?",[intent],
        |r| Ok(PaeaGitIntent { intent_id:r.get(0)?, action_digest:r.get(1)?, canonical_action:r.get(2)? }))
        .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-read",e))?;
    if let Some(record) = &row {
        validate(record, project)?;
    }
    Ok(row)
}
/// Explicit rollback, suppressing Drop retry and retaining its failure. Failed
/// COMMIT stays uncertain even if this rollback succeeds.
fn rollback(mut tx: Transaction<'_>) -> std::result::Result<(), R> {
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    if tx.is_autocommit() {
        Ok(())
    } else {
        tx.execute_batch("ROLLBACK").map_err(R::Sql)
    }
}
/// Publish one SQL operation; preserve known reservation on release failure and
/// uncertainty on COMMIT failure, plus independent rollback cause.
fn finish<T>(
    tx: Transaction<'_>,
    result: Result<T>,
    known: impl Fn(&T) -> PaeaGitIntentObservation,
) -> Result<T> {
    match result {
        Ok(value) => {
            if let Err(cause) = tx.execute_batch("COMMIT") {
                let mut error =
                    PaeaGitIntentError::caused("paea-git-intent-commit-uncertain", cause);
                error.observation = PaeaGitIntentObservation::Uncertain;
                if let Err(cause) = rollback(tx) {
                    error.cleanup.push(cause);
                }
                return Err(error);
            }
            match rollback(tx) {
                Ok(()) => Ok(value),
                Err(cause) => {
                    let mut e = PaeaGitIntentError::caused("paea-git-intent-release", cause);
                    e.observation = known(&value);
                    Err(e)
                }
            }
        }
        Err(mut e) => {
            if let Err(cause) = rollback(tx) {
                e.cleanup.push(cause);
            }
            Err(e)
        }
    }
}
impl RootStore {
    /// Explicit trusted bootstrap for a dedicated journal and immutable project
    /// pin. Refuses existing/partial schema; one atomic SQL transaction, no Git.
    /// Caller owes separate protected custody and operator authority; this API
    /// proves neither. Existing Root initialization is a separate operation.
    pub fn paea_initialize_git_intents(
        &self,
        project_id: &str,
        project_digest: &str,
    ) -> Result<()> {
        if !self.writable || !name(project_id.as_bytes()) || !digest(project_digest.as_bytes()) {
            return Err(PaeaGitIntentError::refused(
                "paea-git-intent-provisioning-binding",
            ));
        }
        self.instance()
            .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-instance", e))?;
        let tx = Transaction::new_unchecked(&self.db, TransactionBehavior::Immediate)
            .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-reservation", e))?;
        let result = (|| {
            if schema(&tx)? {
                return Err(PaeaGitIntentError::refused(
                    "paea-git-intent-already-initialized",
                ));
            }
            tx.execute_batch(&format!("{META};{INTENTS};"))
                .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-create", e))?;
            sql::execute(
                &tx,
                "INSERT INTO main.paea_git_intent_meta VALUES(1,1,?,?)",
                params![project_id, project_digest],
            )
            .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-meta-create", e))?;
            Ok(())
        })();
        finish(tx, result, |_| PaeaGitIntentObservation::NotAttempted)
    }
    /// Reserve before any Git commit request. Validates Git data and exact
    /// independently provisioned project pin. Declared context is NOT attested.
    /// Atomically inserts one business key under IMMEDIATE; changed transport,
    /// candidate or policy cannot overwrite it. Existing always returns its old
    /// bytes and must enter reconciliation, never a fresh publication. Uncertain
    /// errors prohibit proceeding. Does not check grants, reviews or repository.
    pub fn paea_reserve_git_intent(
        &self,
        proposal: &Proposal,
        context: &AdmissionContext,
        project_digest: &str,
    ) -> Result<PaeaGitIntentReservation> {
        if !self.writable || !digest(project_digest.as_bytes()) {
            return Err(PaeaGitIntentError::refused("paea-git-intent-write-binding"));
        }
        let canonical = canonicalize(proposal, context)
            .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-proposal", e))?;
        if !matches!(proposal.action, Action::Git { .. }) {
            return Err(PaeaGitIntentError::refused("paea-git-intent-git-required"));
        }
        self.instance()
            .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-instance", e))?;
        let tx = Transaction::new_unchecked(&self.db, TransactionBehavior::Immediate)
            .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-reservation", e))?;
        let result = (|| {
            let (project, pin) = meta(&tx)?;
            if project != context.project_id || pin != project_digest {
                return Err(PaeaGitIntentError::refused("paea-git-intent-project-pin"));
            }
            if let Some(record) = read(&tx, canonical.intent_id(), &project)? {
                return Ok(PaeaGitIntentReservation::Existing(record));
            }
            let record = PaeaGitIntent {
                intent_id: canonical.intent_id().into(),
                action_digest: canonical.digest().into(),
                canonical_action: canonical.bytes().into(),
            };
            sql::execute(
                &tx,
                "INSERT INTO main.paea_git_intents VALUES(?,?,?)",
                params![
                    record.intent_id,
                    record.action_digest,
                    record.canonical_action
                ],
            )
            .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-insert", e))?;
            Ok(PaeaGitIntentReservation::New(record))
        })();
        finish(tx, result, |r| PaeaGitIntentObservation::Known(r.clone()))
    }
    /// Independent read after restart or uncertain reservation. Missing is None,
    /// never proof of no Git effect. Checks schema, stored bytes and exact pin;
    /// does not mutate or grant permission, finalize or retry a publication.
    pub fn paea_read_git_intent(
        &self,
        intent_id: &str,
        project_digest: &str,
    ) -> Result<Option<PaeaGitIntent>> {
        if !digest(intent_id.as_bytes()) || !digest(project_digest.as_bytes()) {
            return Err(PaeaGitIntentError::refused("paea-git-intent-read-binding"));
        }
        self.instance()
            .map_err(|e| PaeaGitIntentError::caused("paea-git-intent-instance", e))?;
        let (project, pin) = meta(&self.db)?;
        if pin != project_digest {
            return Err(PaeaGitIntentError::refused("paea-git-intent-project-pin"));
        }
        read(&self.db, intent_id, &project)
    }
}
