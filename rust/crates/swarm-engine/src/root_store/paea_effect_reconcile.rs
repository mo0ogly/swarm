//! Read-only PA-09 reconciliation of real durable effect journals against the
//! durable intent reservation, by independently re-deriving each canonical
//! action from stored bytes. Correlation is established by recomputation, never
//! by trusting an agent report, a caller declaration or the stored digest alone.
//! Corrupt/incompatible rows are reported, never repaired, deleted or re-spun;
//! an uninitialized journal is an explicit state, not an empty success. Hash
//! integrity proves neither authenticity of declared contexts, collection
//! completeness, external anchors nor immutable storage: a same-UID malicious
//! writer can rewrite all rows consistently. No authority, publication, policy
//! change, provisioning or public transport exists here.
use super::{finished, mission_supervision_store as sql, RootStore, RootStoreError as R};
use rusqlite::{params, Connection, Row};
use serde::Serialize;
use std::collections::BTreeMap;
use swarm_identity_contract::{digest, name};
use swarm_protection_contract::{canonicalize, Action, AdmissionContext, Proposal};

const SETTLEMENT_META: &str = "CREATE TABLE paea_settlement_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),schema_version INTEGER NOT NULL CHECK(schema_version=1),project_id TEXT NOT NULL,project_digest TEXT NOT NULL)";
const SETTLEMENT_DEBTS: &str = "CREATE TABLE paea_settlement_debts(invoice_id TEXT PRIMARY KEY,invoice_version TEXT NOT NULL,amount_minor INTEGER NOT NULL CHECK(amount_minor>0),currency TEXT NOT NULL,beneficiary TEXT NOT NULL)";
const SETTLEMENT_EFFECTS: &str = "CREATE TABLE paea_settlement_effects(intent_id TEXT PRIMARY KEY,invoice_id TEXT NOT NULL UNIQUE REFERENCES paea_settlement_debts(invoice_id),action_digest TEXT NOT NULL,receipt TEXT NOT NULL)";
const GIT_INTENT_META: &str = "CREATE TABLE paea_git_intent_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),schema_version INTEGER NOT NULL CHECK(schema_version=1),project_id TEXT NOT NULL,project_digest TEXT NOT NULL)";
const GIT_INTENTS: &str = "CREATE TABLE paea_git_intents(intent_id TEXT PRIMARY KEY,action_digest TEXT NOT NULL,canonical_action BLOB NOT NULL)";
const GIT_PROMOTE_META: &str = "CREATE TABLE paea_git_promote_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),schema_version INTEGER NOT NULL CHECK(schema_version=1),project_id TEXT NOT NULL,project_digest TEXT NOT NULL)";
const GIT_PROMOTE_EFFECTS: &str = "CREATE TABLE paea_git_promote_effects(intent_id TEXT PRIMARY KEY,action_digest TEXT NOT NULL,target_ref TEXT NOT NULL,new_oid TEXT NOT NULL,receipt TEXT NOT NULL)";

/// One reconciliation verdict per real durable effect row. `verified=false` is
/// always accompanied by at least one exact refusal code; `verified=true` rows
/// still establish only recomputed correspondence, not authenticity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PaeaEffectReconcileFinding {
    pub intent_id: String,
    pub action_digest: String,
    pub verified: bool,
    /// Exact codes of every failed check; empty only when verified.
    pub refusals: Vec<&'static str>,
}

/// Explicit journal state per domain. `Corrupt` carries the exact schema codes;
/// it never triggers repair or adoption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum PaeaEffectReconcileJournal {
    /// The auxiliary schema is absent on this root: no effect was reconciled.
    Uninitialized,
    /// Schema present; rows reconciled with the listed findings.
    Consistent {
        #[serde(rename = "effectCount")]
        effect_count: usize,
        findings: Vec<PaeaEffectReconcileFinding>,
    },
    /// Schema present but altered/partial/extra objects detected. Reported as
    /// exact refusal codes; no row is reconciled.
    Corrupt { codes: Vec<&'static str> },
}

/// Complete read-only reconciliation report for both real effect domains.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PaeaEffectReconcileReport {
    pub schema_version: u32,
    pub settlement: PaeaEffectReconcileJournal,
    pub git: PaeaEffectReconcileJournal,
}

/// Stored canonical frame re-derived independently of any stored digest.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    schema_version: u32,
    canonicalization: String,
    action: Action,
    context: AdmissionContext,
}

/// Stored settlement effect receipt (exact producer field set; the principal
/// and scope objects are validated by their own strict decoders).
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SettlementReceipt {
    intent_id: String,
    action_digest: String,
    invoice_id: String,
    canonical_action: Vec<u8>,
    authority_enrollment_id: String,
    authority_project_digest: String,
    #[serde(rename = "authority_principal")]
    _authority_principal: serde_json::Value,
    #[serde(rename = "authority_scope")]
    _authority_scope: serde_json::Value,
    authority_head_sequence: i64,
    authority_head_digest: String,
    trust_revision: i64,
    commit_requested_at: String,
}

/// Stored Git promotion receipt (exact producer field set).
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct GitPromoteReceipt {
    target_ref: String,
    old_oid: String,
    new_oid: String,
}

/// One finding marked incompatible with the given refusal codes.
fn incompatible(intent_id: &str, action_digest: &str, refusals: Vec<&'static str>) -> PaeaEffectReconcileFinding {
    PaeaEffectReconcileFinding {
        intent_id: intent_id.to_owned(),
        action_digest: action_digest.to_owned(),
        verified: false,
        refusals,
    }
}

/// Re-derive the canonical action from stored intent bytes and bind it to the
/// expected business key and effect digest. Missing/corrupt bytes yield a
/// finding with the exact refusal, never an empty success.
fn correlate(
    intent_id: &str,
    action_digest: &str,
    canonical_action: Option<&[u8]>,
    git_domain: bool,
) -> (PaeaEffectReconcileFinding, Option<Stored>) {
    let mut refusals: Vec<&'static str> = Vec::new();
    let Some(bytes) = canonical_action else {
        refusals.push("paea-effect-reconcile-intent-missing");
        return (incompatible(intent_id, action_digest, refusals), None);
    };
    let stored: Stored = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => {
            refusals.push("paea-effect-reconcile-canonical-decode");
            return (incompatible(intent_id, action_digest, refusals), None);
        }
    };
    if stored.canonicalization != "paea-json-v1" {
        refusals.push("paea-effect-reconcile-canonical-profile");
        return (incompatible(intent_id, action_digest, refusals), None);
    }
    let domain_ok = matches!(&stored.action, Action::Git { .. }) == git_domain;
    let canonical = match canonicalize(
        &Proposal {
            schema_version: stored.schema_version,
            request_id: "paea-effect-reconcile".into(),
            action: stored.action.clone(),
        },
        &stored.context,
    ) {
        Ok(value) => value,
        Err(_) => {
            refusals.push("paea-effect-reconcile-canonical-rederive");
            return (incompatible(intent_id, action_digest, refusals), None);
        }
    };
    if canonical.bytes() != bytes {
        refusals.push("paea-effect-reconcile-canonical-bytes");
    }
    if canonical.digest() != action_digest {
        refusals.push("paea-effect-reconcile-effect-digest");
    }
    if canonical.intent_id() != intent_id {
        refusals.push("paea-effect-reconcile-intent-id");
    }
    if !domain_ok {
        refusals.push("paea-effect-reconcile-domain");
    }
    (PaeaEffectReconcileFinding {
        intent_id: intent_id.to_owned(),
        action_digest: action_digest.to_owned(),
        verified: refusals.is_empty(),
        refusals,
    }, Some(stored))
}

/// Journal schema gate: `Some(state)` for Uninitialized/Corrupt, `None` when
/// every table is present and exact.
fn checked_schema(
    db: &Connection,
    tables: &[(&str, &str)],
    code: &'static str,
) -> Result<Option<PaeaEffectReconcileJournal>, R> {
    let mut present = 0usize;
    for (table, ddl) in tables {
        match sql::auxiliary_schema(db, table, ddl, code) {
            Ok(true) => present += 1,
            Ok(false) => {}
            Err(_) => {
                return Ok(Some(PaeaEffectReconcileJournal::Corrupt {
                    codes: vec![code],
                }))
            }
        }
    }
    match present {
        0 => Ok(Some(PaeaEffectReconcileJournal::Uninitialized)),
        n if n == tables.len() => Ok(None),
        _ => Ok(Some(PaeaEffectReconcileJournal::Corrupt {
            codes: vec![code],
        })),
    }
}

/// Read all rows of one journal table, ordered by business key.
fn read_rows<T>(
    db: &Connection,
    query: &str,
    read: impl Fn(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, R> {
    let mut statement = db
        .prepare(query)
        .map_err(|e| R::SqlTransaction(Box::new(e)))?;
    let mut rows = statement
        .query([])
        .map_err(|e| R::SqlTransaction(Box::new(e)))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| R::SqlTransaction(Box::new(e)))? {
        out.push(read(row).map_err(|e| R::SqlTransaction(Box::new(e)))?);
    }
    Ok(out)
}

impl RootStore {
    /// Read-only PA-09 reconciliation of both real effect domains: the
    /// settlement ledger (separate root) and this service's Git journals.
    /// Each stored canonical action is re-derived from its durable bytes and
    /// bound to the effect digest and business key by recomputation. Corrupt
    /// journals are reported, never repaired; uninitialized is explicit, not
    /// empty success. No authority, effect, policy change, provisioning or
    /// transport occurs. Storage failures are errors; business mismatches are
    /// findings in the report.
    pub fn paea_reconcile_effect_journals(
        &self,
        ledger: &RootStore,
    ) -> Result<PaeaEffectReconcileReport, R> {
        // Mirror the settlement producer's separation witness: settlement and
        // service roots must be distinct physical stores, otherwise one
        // compromised store could fabricate both sides of the cross-check.
        // Identity observation only; no effect or permission follows.
        let project_binding = self.bind_root()?;
        let separation = (|| {
            let ledger_binding = ledger.bind_root()?;
            let result = (|| {
                let project_instance = project_binding.instance()?;
                let ledger_instance = ledger_binding.instance()?;
                if project_instance.directory == ledger_instance.directory
                    || project_instance.database == ledger_instance.database
                {
                    return Err(R::Refused("paea-effect-reconcile-separate-ledger-required"));
                }
                Ok(())
            })();
            finished(result, ledger_binding.close())
        })();
        finished(separation, project_binding.close())?;
        let settlement = ledger.reconcile_settlement()?;
        let git = self.reconcile_git()?;
        Ok(PaeaEffectReconcileReport {
            schema_version: 1,
            settlement,
            git,
        })
    }

    /// Reconcile the real settlement ledger: receipt bytes re-decoded, the
    /// canonical payment action re-derived, digest/intent/invoice/debt bound
    /// by recomputation, authority metadata checked for representation only.
    fn reconcile_settlement(&self) -> Result<PaeaEffectReconcileJournal, R> {
        let tables = [
            ("paea_settlement_meta", SETTLEMENT_META),
            ("paea_settlement_debts", SETTLEMENT_DEBTS),
            ("paea_settlement_effects", SETTLEMENT_EFFECTS),
        ];
        if let Some(other) = checked_schema(&self.db, &tables, "paea-effect-reconcile-settlement-schema")? {
            return Ok(other);
        }
        let rows: Vec<(String, String, String, String)> = read_rows(
            &self.db,
            "SELECT intent_id,invoice_id,action_digest,receipt FROM main.paea_settlement_effects ORDER BY intent_id",
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        let mut findings = Vec::new();
        for (intent_id, invoice_id, action_digest, raw) in &rows {
            let mut refusals: Vec<&'static str> = Vec::new();
            let receipt: SettlementReceipt = match serde_json::from_str(raw) {
                Ok(value) => value,
                Err(_) => {
                    refusals.push("paea-effect-reconcile-settlement-receipt-decode");
                    findings.push(incompatible(intent_id, action_digest, refusals));
                    continue;
                }
            };
            if receipt.intent_id != *intent_id
                || receipt.invoice_id != *invoice_id
                || receipt.action_digest != *action_digest
            {
                refusals.push("paea-effect-reconcile-settlement-receipt-binding");
            }
            let stored: Stored = match serde_json::from_slice(&receipt.canonical_action) {
                Ok(value) => value,
                Err(_) => {
                    refusals.push("paea-effect-reconcile-canonical-decode");
                    findings.push(incompatible(intent_id, action_digest, refusals));
                    continue;
                }
            };
            if stored.canonicalization != "paea-json-v1" {
                refusals.push("paea-effect-reconcile-canonical-profile");
                findings.push(incompatible(intent_id, action_digest, refusals));
                continue;
            }
            let domain_ok = matches!(&stored.action, Action::Payment { .. });
            let canonical = match canonicalize(
                &Proposal {
                    schema_version: stored.schema_version,
                    request_id: "paea-effect-reconcile".into(),
                    action: stored.action.clone(),
                },
                &stored.context,
            ) {
                Ok(value) => value,
                Err(_) => {
                    refusals.push("paea-effect-reconcile-canonical-rederive");
                    findings.push(incompatible(intent_id, action_digest, refusals));
                    continue;
                }
            };
            if canonical.bytes() != receipt.canonical_action.as_slice() {
                refusals.push("paea-effect-reconcile-canonical-bytes");
            }
            if canonical.digest() != *action_digest {
                refusals.push("paea-effect-reconcile-effect-digest");
            }
            if canonical.intent_id() != *intent_id {
                refusals.push("paea-effect-reconcile-intent-id");
            }
            let Action::Payment {
                invoice_id: stored_invoice,
                invoice_version,
                amount_minor,
                currency,
                beneficiary_account_id,
            } = &stored.action
            else {
                refusals.push("paea-effect-reconcile-domain");
                findings.push(incompatible(intent_id, action_digest, refusals));
                continue;
            };
            if !domain_ok {
                refusals.push("paea-effect-reconcile-domain");
            }
            if stored_invoice != invoice_id {
                refusals.push("paea-effect-reconcile-invoice-id");
            }
            let Ok(amount) = amount_minor.parse::<i64>() else {
                refusals.push("paea-effect-reconcile-amount");
                findings.push(incompatible(intent_id, action_digest, refusals));
                continue;
            };
            let debt: Option<(String, i64, String, String)> = sql::optional(
                &self.db,
                "SELECT invoice_version,amount_minor,currency,beneficiary FROM main.paea_settlement_debts WHERE invoice_id=?",
                params![invoice_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|e| R::SqlTransaction(Box::new(e)))?;
            if debt.as_ref()
                != Some(&(
                    invoice_version.clone(),
                    amount,
                    currency.clone(),
                    beneficiary_account_id.clone(),
                ))
            {
                refusals.push("paea-effect-reconcile-debt-incompatible");
            }
            if !name(receipt.authority_enrollment_id.as_bytes())
                || !digest(receipt.authority_project_digest.as_bytes())
                || !digest(receipt.authority_head_digest.as_bytes())
                || receipt.authority_head_sequence <= 0
                || receipt.trust_revision <= 0
            {
                refusals.push("paea-effect-reconcile-receipt-authority-shape");
            }
            if time::OffsetDateTime::parse(
                &receipt.commit_requested_at,
                &time::format_description::well_known::Rfc3339,
            )
            .is_err()
            {
                refusals.push("paea-effect-reconcile-receipt-time");
            }
            findings.push(PaeaEffectReconcileFinding {
                intent_id: intent_id.clone(),
                action_digest: action_digest.clone(),
                verified: refusals.is_empty(),
                refusals,
            });
        }
        Ok(PaeaEffectReconcileJournal::Consistent {
            effect_count: findings.len(),
            findings,
        })
    }

    /// Reconcile this service's Git journals: each promotion effect is joined
    /// to its durable intent reservation (bytes reserved before any Git), the
    /// canonical Git action is re-derived and bound to the effect digest, and
    /// the receipt's reference columns must match the effect row exactly.
    fn reconcile_git(&self) -> Result<PaeaEffectReconcileJournal, R> {
        let tables = [
            ("paea_git_intent_meta", GIT_INTENT_META),
            ("paea_git_intents", GIT_INTENTS),
            ("paea_git_promote_meta", GIT_PROMOTE_META),
            ("paea_git_promote_effects", GIT_PROMOTE_EFFECTS),
        ];
        if let Some(other) = checked_schema(&self.db, &tables, "paea-effect-reconcile-git-schema")? {
            return Ok(other);
        }
        let intents: Vec<(String, String, Vec<u8>)> = read_rows(
            &self.db,
            "SELECT intent_id,action_digest,canonical_action FROM main.paea_git_intents ORDER BY intent_id",
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let intents: BTreeMap<String, (String, Vec<u8>)> = intents
            .into_iter()
            .map(|(intent_id, action_digest, bytes)| (intent_id, (action_digest, bytes)))
            .collect();
        let effects: Vec<(String, String, String, String, String)> = read_rows(
            &self.db,
            "SELECT intent_id,action_digest,target_ref,new_oid,receipt FROM main.paea_git_promote_effects ORDER BY intent_id",
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )?;
        let mut findings = Vec::new();
        for (intent_id, action_digest, target_ref, new_oid, raw) in &effects {
            let (mut finding, stored) = correlate(
                intent_id,
                action_digest,
                intents.get(intent_id).map(|(_, bytes)| bytes.as_slice()),
                true,
            );
            if let Some(stored) = stored {
                let receipt: Option<GitPromoteReceipt> = serde_json::from_str(raw).ok();
                match receipt {
                    Some(receipt) => {
                        if receipt.target_ref != *target_ref || receipt.new_oid != *new_oid {
                            finding.refusals.push("paea-effect-reconcile-git-receipt-binding");
                        }
                        // The durable intent's expected old OID must match the
                        // observed pre-promotion reference recorded in the
                        // receipt; a mismatch means the promoted transition did
                        // not start from the reserved state. A non-Git action
                        // here is a domain mismatch already refused above.
                        if let Action::Git { expected_old_oid, .. } = &stored.action {
                            if *expected_old_oid != receipt.old_oid {
                                finding.refusals.push("paea-effect-reconcile-git-old-oid");
                            }
                        }
                        if finding.refusals.is_empty() {
                            finding.verified = true;
                        } else {
                            finding.verified = false;
                        }
                    }
                    None => {
                        finding.refusals.push("paea-effect-reconcile-git-receipt-decode");
                        finding.verified = false;
                    }
                }
            }
            findings.push(finding);
        }
        Ok(PaeaEffectReconcileJournal::Consistent {
            effect_count: findings.len(),
            findings,
        })
    }
}
