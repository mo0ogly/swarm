//! Read-only observation of a durable Trust approval matching the exact
//! prepared PAEA Git action bytes. This module is a guard input for the
//! single-intent consumption route: it reads the project root's real Trust
//! state under the existing shared reservation and never creates, amends or
//! re-interprets an approval. Producers are the existing Trust operations
//! (`mandate-create`, `request`, `approve`); validation is the established
//! decode/validate/require-supported state chain; the only intended consumer
//! is the transport-side consumption guard, which must still run every other
//! authority, witness, plan and policy check. No bearer, request field or
//! preview page can manufacture a match: the approval evidence must exist in
//! the project's own validated Trust state with its exact original command
//! bytes retained under their recorded digest.
use super::{absent, RootStore, RootStoreError as R};
use crate::trust::state::State;
use serde_json::Value;
use time::OffsetDateTime;

/// Newest durable approval that binds the exact supplied canonical action
/// bytes. `approver` is the authenticated actor subject recorded by the Trust
/// `approve` operation — never a client-proclaimed identity. `revision` is the
/// Trust configuration revision the approval was captured under, which the
/// same locked read confirms is still current: a configuration change voids
/// the agreement until a fresh approval is captured under the new revision.
/// Presence proves a captured exact agreement; it never substitutes authority,
/// plan, witness or policy checks and never establishes human presence beyond
/// what the recorded enrollment evidence itself carries.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PaeaGitAgreementMatch {
    pub action_id: String,
    pub approver: String,
    pub content_digest: String,
    pub revision: i64,
}

/// Exact Trust terms an approver must supply so the existing `approve`
/// operation captures an agreement the guard recognizes. Every field is
/// derived engine-side from the canonical action bytes via the same fixed
/// `paea-git-agreement-v1` recipe the guard matches on — the browser never
/// computes, amends or selects a term. `binding` states which of the guard's
/// two bindings these terms satisfy: `recipe` (the fixed P2 recipe) when the
/// action is a canonical `repository.promote` whose identity fits the Trust
/// resource grammar, `none` otherwise (the action can only be approved by
/// submitting the exact canonical command bytes, P1, which the UI never
/// carries — such an action is presented without derivable terms).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PaeaGitAgreementTerms {
    pub binding: &'static str,
    pub action_id: String,
    pub content_digest: String,
    pub resource: String,
    pub amount_minor: i64,
    pub currency: String,
    pub beneficiary: String,
    pub revision: i64,
}

/// Fixed `paea-git-agreement-v1` recipe: derive the expected Trust terms from
/// a canonical `repository.promote` action. The resource carries the project,
/// repository, target reference and exact candidate OID; the beneficiary is
/// the expected old OID; the amount is the protocol sentinel 1 with the fixed
/// currency `OID`. This binds the full canonical Git-intent identity (project
/// + repository + ref + old OID) plus the exact candidate — the same identity
/// the canonicalization derives — through the historical exact-terms approve
/// guard, without inventing a second approval system. Unknown canonical kinds
/// (for example `payment.settle`) never derive recipe terms, so they can only
/// bind through the exact command bytes path (P1).
fn git_intent_terms(value: &Value) -> Option<(String, String)> {
    let action = value.get("action")?;
    if action.get("kind")?.as_str()? != "repository.promote" {
        return None;
    }
    let project_id = value.get("context")?.get("project_id")?.as_str()?;
    let repository_id = action.get("repository_id")?.as_str()?;
    let target_ref = action.get("target_ref")?.as_str()?;
    let expected_old_oid = action.get("expected_old_oid")?.as_str()?;
    let candidate_oid = action.get("candidate_oid")?.as_str()?;
    // Target refs and OIDs must survive the existing resource grammar, which
    // itself is revalidated on every request/approve command by the exact
    // terms guard; a canonical identity that cannot be a Trust resource never
    // derives recipe terms.
    let resource = format!("{project_id}/{repository_id}/{target_ref}/{candidate_oid}");
    if !crate::trust::command::resource(&resource) {
        return None;
    }
    Some((resource, expected_old_oid.to_owned()))
}

/// Decode one retained original by its exact digest; a missing or non-base64
/// retained original is an integrity refusal, never an empty match.
fn retained(st: &State, digest: &str) -> Result<Vec<u8>, R> {
    let encoded = st
        .originals
        .get(digest)
        .ok_or(R::Refused("paea-git-agreement-original-missing"))?;
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD
        .decode(encoded)
        .map_err(|_| R::Refused("paea-git-agreement-original-encoding"))
}

/// Convert a typed trust failure into the root-store error, preserving cause.
fn failure(e: crate::trust::TrustError) -> R {
    R::Trust(Box::new(e))
}

impl RootStore {
    /// Read-only observation of the newest durable Trust approval binding the
    /// exact supplied canonical PAEA action bytes, for the consumption guard
    /// of the single-intent Git call. Inputs: `action_bytes` must be the exact
    /// canonical `paea-json-v1` bytes whose SHA-256 the preparation already
    /// verified (the caller owes that digest comparison; this method never
    /// trusts a caller-claimed digest). Output: `Ok(Some(match))` when a
    /// matching approval exists whose captured Trust revision is still the
    /// project root's current one at this last use AND whose underlying
    /// mandate is still active in the same locked read; `Ok(None)` when no
    /// approval binds these bytes or when every candidate was captured under
    /// an older revision or under a mandate now revoked, expired or inactive —
    /// absence, a denied or non-exact approval, a pending action, a superseded
    /// approval, a revision change since the approval and a public mandate
    /// revocation after the approval are all deliberately the same explicit
    /// `None` so the transport guard refuses with one typed code. Errors:
    /// storage, decode, validation and unsupported-policy failures are typed
    /// errors and are never converted to `None`; a missing or un-decodable
    /// retained original refuses `paea-git-agreement-original-missing` /
    /// `-encoding`, and an original whose bytes no longer hash to its recorded
    /// digest refuses `paea-git-agreement-original-altered` (integrity, never
    /// empty success).
    /// Effects: none — the shared `trust-actions.lock` reservation is taken,
    /// the bounded state file is read and validated with the established
    /// decode/validate/require-supported chain, and nothing is written, aged,
    /// replayed or granted. Matching requires, for a candidate approval
    /// evidence (`kind=="approve"`, `result=="approved"`, non-empty
    /// `original_digest`): its retained original command bytes hash back to
    /// the recorded `original_digest`; the action it approved still exists,
    /// is `approved`, records the same `content_digest` and the same
    /// authenticated `approver` subject; the action's mandate is still active
    /// (never revoked, not expired, current policy revision) at this read;
    /// one of two exact bindings holds —
    /// (P1) the approval's retained original command bytes are exactly
    /// `action_bytes`, or (P2) `action_bytes` are a `repository.promote`
    /// canonical action whose Git-intent identity (project/repository/
    /// target_ref/expected_old_oid/candidate_oid) equals the approved action's
    /// terms under the fixed `paea-git-agreement-v1` recipe
    /// (`resource="{project}/{repository}/{target_ref}/{candidate}"`,
    /// `amount_minor=1`, `currency="OID"`, `beneficiary=expected_old_oid`, no
    /// connector); and its recorded policy revision equals the state revision
    /// in the same locked read, so the freshness check is atomic with the
    /// match. The newest matching evidence (highest sequence) wins;
    /// superseded approvals never resurface. Limits: this is a root-state
    /// observation, not an authentication of the approver's live consent —
    /// the recorded identity context of the approval remains the only
    /// enrollment evidence, and real human enrollment/approval stays required
    /// for any live execution or acceptance.
    pub fn paea_git_agreement_match(
        &self,
        action_bytes: &[u8],
    ) -> Result<Option<PaeaGitAgreementMatch>, R> {
        let reservation = self.directory.reserve_trust(false).map_err(R::Storage)?;
        let result = (|| {
            let raw = match self
                .directory
                .read_bounded(super::trust_store::STATE, super::trust_store::MAX_STATE)
            {
                Ok(raw) => raw,
                Err(e) if absent(&e) => return Ok(None),
                Err(e) => return Err(R::Storage(e)),
            };
            let state = State::decode(&raw).map_err(failure)?;
            state.validate().map_err(failure)?;
            state.require_supported().map_err(failure)?;
            // Newest matching approval wins: highest sequence among matching
            // candidate approvals; earlier matches are superseded.
            let mut matched: Option<(i64, PaeaGitAgreementMatch)> = None;
            for evidence in &state.evidence {
                if evidence.kind != "approve"
                    || evidence.result != "approved"
                    // Freshness is part of the match itself: an approval
                    // captured under a different configuration revision is
                    // explicitly void at this last use — the same single
                    // locked read decides both binding and currency.
                    || evidence
                        .policy_context
                        .as_ref()
                        .is_none_or(|p| p.revision != state.revision)
                {
                    continue;
                }
                // The retained original command bytes must hash back to the
                // recorded original digest; altered material refuses.
                let original = retained(&state, &evidence.original_digest)?;
                if crate::trust::hash(&original) != evidence.original_digest {
                    return Err(R::Refused("paea-git-agreement-original-altered"));
                }
                // The approved action must still carry the approved state, the
                // same content digest and the same authenticated approver.
                let Some(action) = state.actions.get(&evidence.object_id) else {
                    continue;
                };
                let approver = action
                    .get("approver")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if action.get("state").and_then(Value::as_str) != Some("approved")
                    || approver.is_empty()
                    || approver != evidence.actor
                    || action
                        .get("content_digest")
                        .and_then(Value::as_str)
                        != Some(evidence.content_digest.as_str())
                {
                    continue;
                }
                // Last-use freshness: the mandate the approval rests on must
                // still be active in this same locked read. A public mandate
                // revocation (or expiration) voids every agreement captured
                // under it without any revision change; a revoked, expired or
                // inactive mandate is an explicit None, never a match.
                let mandate_id = action
                    .get("mandate_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let mandate_active = mandate_id.is_empty()
                    || crate::trust::operations::active(
                        &state,
                        mandate_id,
                        OffsetDateTime::now_utc(),
                    )
                    .map_err(failure)?
                    .is_empty();
                if !mandate_active {
                    continue;
                }
                // P1: the approval command's retained original bytes are the
                // exact canonical action bytes.
                let exact_command = original == action_bytes;
                // P2: fixed recipe binding the full canonical Git-intent
                // identity to the approved action's terms.
                let intent = (|| {
                    let value: Value = serde_json::from_slice(action_bytes).ok()?;
                    let (resource, beneficiary) = git_intent_terms(&value)?;
                    let connector_absent = action
                        .get("connector")
                        .cloned()
                        .unwrap_or(Value::Null)
                        .is_null();
                    let get = |key: &str| action.get(key).and_then(Value::as_str).unwrap_or("");
                    (get("resource") == resource
                        && action.get("amount_minor").and_then(Value::as_i64) == Some(1)
                        && get("currency") == "OID"
                        && get("beneficiary") == beneficiary
                        && connector_absent)
                        .then_some(())
                })();
                if exact_command || intent.is_some() {
                    matched = Some((
                        evidence.sequence,
                        PaeaGitAgreementMatch {
                            action_id: evidence.object_id.clone(),
                            approver: approver.to_owned(),
                            content_digest: evidence.content_digest.clone(),
                            revision: state.revision,
                        },
                    ));
                }
            }
            Ok(matched.map(|(_, match_value)| match_value))
        })();
        // Shared reservation release; a close failure refuses the observation
        // and can never be converted into a fabricated match or empty success.
        match reservation.close() {
            Ok(()) => result,
            Err(cause) => match result {
                Ok(_) => Err(R::Storage(cause)),
                Err(primary) => Err(R::Cleanup {
                    primary: Some(Box::new(primary)),
                    cleanup: Box::new(R::Storage(cause)),
                }),
            },
        }
    }

    /// Engine-side derivation of the exact Trust terms an approver must submit
    /// so the existing `approve` operation captures an agreement this guard
    /// recognizes, for the canonical PAEA action bytes supplied. This is the
    /// presentation counterpart of `paea_git_agreement_match`: both read the
    /// same validated state under the same shared reservation, and the terms
    /// come from the same fixed `paea-git-agreement-v1` recipe — never from a
    /// request field, a browser computation or a second approval system.
    /// Inputs: `action_bytes` must be the exact canonical `paea-json-v1` bytes
    /// (the caller owes the digest comparison, as for the match). Output:
    /// `Ok(Some(terms))` with `binding=="recipe"` when the action is a
    /// canonical `repository.promote` whose Git-intent identity fits the Trust
    /// resource grammar; `Ok(None)` when no recipe terms can be derived (the
    /// action binds only through exact command bytes, P1 — presenting no
    /// derivable terms is the honest state, never a synthesized fallback).
    /// The returned `action_id` and `content_digest` are empty placeholders:
    /// the Trust `request` operation creates the durable action when the
    /// requester runs it, and the `approve` guard revalidates every term
    /// against the real action state. Errors: storage, decode, validation and
    /// unsupported-policy failures are typed and never converted to `None`.
    /// Effects: none — shared reservation, bounded read, validated state, no
    /// write, no grant, no approval creation. Limits: an observation only; the
    /// captured agreement still requires the full producer chain (mandate
    /// active, requester, approver role, separation of duties) and never
    /// establishes human presence.
    pub fn paea_git_agreement_terms(
        &self,
        action_bytes: &[u8],
    ) -> Result<Option<PaeaGitAgreementTerms>, R> {
        let reservation = self.directory.reserve_trust(false).map_err(R::Storage)?;
        let result = (|| {
            let raw = match self
                .directory
                .read_bounded(super::trust_store::STATE, super::trust_store::MAX_STATE)
            {
                Ok(raw) => raw,
                Err(e) if absent(&e) => return Ok(None),
                Err(e) => return Err(R::Storage(e)),
            };
            let state = State::decode(&raw).map_err(failure)?;
            state.validate().map_err(failure)?;
            state.require_supported().map_err(failure)?;
            let value: Value = match serde_json::from_slice(action_bytes) {
                Ok(value) => value,
                Err(_) => return Ok(None),
            };
            let (resource, beneficiary) = match git_intent_terms(&value) {
                Some(terms) => terms,
                None => return Ok(None),
            };
            Ok(Some(PaeaGitAgreementTerms {
                binding: "recipe",
                action_id: String::new(),
                content_digest: String::new(),
                resource,
                amount_minor: 1,
                currency: "OID".to_owned(),
                beneficiary,
                revision: state.revision,
            }))
        })();
        match reservation.close() {
            Ok(()) => result,
            Err(cause) => match result {
                Ok(_) => Err(R::Storage(cause)),
                Err(primary) => Err(R::Cleanup {
                    primary: Some(Box::new(primary)),
                    cleanup: Box::new(R::Storage(cause)),
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SystemEnvironment;
    use serde_json::{json, Value};

    /// Isolated Trust fixture with bearer-token identities, mirroring the
    /// established biba test configuration (separation enabled).
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = crate::fixture_root("paea-agreement-");
            let root = RootStore::initialize(&path, &mut SystemEnvironment).unwrap();
            let config = json!({
                "schema_version":1,
                "event_id":"config",
                "expected_revision":0,
                "reason":"Isolated agreement fixture",
                "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},
                "identities":[
                    {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
                    {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},
                    {"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin","effect"]}
                ]
            });
            root.trust("config-apply", &serde_json::to_vec(&config).unwrap(), "")
                .unwrap();
            root.close().unwrap();
            Self(path)
        }
        fn call(&self, op: &str, value: Value, token: &str) -> Value {
            let root = RootStore::open(&self.0, true).unwrap();
            let reply = root
                .trust(op, &serde_json::to_vec(&value).unwrap(), token)
                .unwrap();
            root.close().unwrap();
            serde_json::from_slice(&reply.bytes).unwrap()
        }
        fn revision(&self) -> i64 {
            let root = RootStore::open(&self.0, false).unwrap();
            let revision = root.trust_revision().unwrap().unwrap();
            root.close().unwrap();
            revision
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if !std::thread::panicking() {
                std::fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }

    fn cmd(id: &str, value: Value) -> Value {
        let mut value = value;
        value["schema_version"] = 1.into();
        value["event_id"] = id.into();
        value
    }

    /// Canonical `repository.promote` action bytes with the fixed agreement
    /// recipe terms; `candidate` selects the bound candidate OID.
    fn git_action_bytes(candidate: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema_version":1,
            "canonicalization":"paea-json-v1",
            "action":{"kind":"repository.promote","repository_id":"repo","target_ref":"refs/heads/release","expected_old_oid":"1".repeat(40),"candidate_oid":candidate},
            "context":{"project_id":"project","principal_id":"operator","instance_id":"instance","mandate_id":"mandate","mandate_revision":1,"policy_revision":1,"policy_digest":"aa".repeat(32),"tool_manifest_digest":"bb".repeat(32),"sources":[]}
        }))
        .unwrap()
    }

    /// Full real producer chain on the isolated root: mandate, request and
    /// exact approval for the recipe terms of `candidate`.
    fn approve_candidate(f: &Fixture, event: &str, candidate: &str) -> Value {
        let resource = format!("project/repo/refs/heads/release/{candidate}");
        f.call(
            "mandate-create",
            cmd(
                &format!("{event}-mandate"),
                json!({"mandate_id":format!("mandate-{event}"),"subject":"requester","actions":["settle"],"resource":resource,"max_amount_minor":1000,"currency":"OID","beneficiary":"1".repeat(40)}),
            ),
            "fixture-admin-token",
        );
        let action = f.call(
            "request",
            cmd(
                &format!("{event}-request"),
                json!({"mandate_id":format!("mandate-{event}"),"resource":resource,"amount_minor":1,"currency":"OID","beneficiary":"1".repeat(40),"idempotency_key":format!("key-{event}")}),
            ),
            "fixture-requester-token",
        );
        f.call(
            "approve",
            cmd(
                &format!("{event}-approve"),
                json!({"action_id":action["id"],"mandate_id":format!("mandate-{event}"),"content_digest":action["content_digest"],"resource":resource,"amount_minor":1,"currency":"OID","beneficiary":"1".repeat(40)}),
            ),
            "fixture-approver-token",
        )
    }

    /// The current Trust revision read on the fixture's own bounded state file.
    fn root_revision(f: &Fixture) -> i64 {
        let state: Value = serde_json::from_slice(
            &std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap(),
        )
        .unwrap();
        state["revision"].as_i64().unwrap()
    }

    /// The recipe binding (P2) matches an approved action carrying the exact
    /// Git-intent identity; absence, a different candidate and a pending
    /// action never match. This is the valid control for the refusal witnesses.
    #[test]
    fn agreement_matches_exact_recipe_and_rejects_absence_and_other_candidate() {
        let f = Fixture::new();
        let root = RootStore::open(&f.0, false).unwrap();
        // No approval captured yet: explicit None, never a fabricated match.
        assert!(root
            .paea_git_agreement_match(&git_action_bytes(&"f".repeat(40)))
            .unwrap()
            .is_none());
        let candidate = "f".repeat(40);
        let approved = approve_candidate(&f, "first", &candidate);
        assert_eq!(approved["state"], "approved");
        let bytes = git_action_bytes(&candidate);
        let matched = root.paea_git_agreement_match(&bytes).unwrap().unwrap();
        assert_eq!(matched.action_id, approved["id"].as_str().unwrap());
        assert_eq!(matched.approver, "approver");
        assert_eq!(matched.revision, f.revision());
        assert_eq!(matched.content_digest.len(), 64);
        // A different candidate is a different action: no match, no aliasing.
        assert!(root
            .paea_git_agreement_match(&git_action_bytes(&"e".repeat(40)))
            .unwrap()
            .is_none());
        // Pending (pre-approval) second request never matches either.
        let other = "e".repeat(40);
        let resource = format!("project/repo/refs/heads/release/{other}");
        f.call(
            "mandate-create",
            cmd(
                "second-mandate",
                json!({"mandate_id":"mandate2","subject":"requester","actions":["settle"],"resource":resource,"max_amount_minor":1000,"currency":"OID","beneficiary":"1".repeat(40)}),
            ),
            "fixture-admin-token",
        );
        let pending = f.call(
            "request",
            cmd(
                "second-request",
                json!({"mandate_id":"mandate2","resource":resource,"amount_minor":1,"currency":"OID","beneficiary":"1".repeat(40),"idempotency_key":"key-second"}),
            ),
            "fixture-requester-token",
        );
        assert_eq!(pending["state"], "pending_approval");
        assert!(root
            .paea_git_agreement_match(&git_action_bytes(&other))
            .unwrap()
            .is_none());
        root.close().unwrap();
    }

    /// P1: an approval whose retained command bytes ARE the canonical action
    /// bytes matches even though the recipe does not apply (bytes unrelated
    /// to the Git recipe, bound only by exact command-byte equality).
    #[test]
    fn agreement_accepts_exact_command_bytes_without_recipe() {
        let f = Fixture::new();
        let resource = "audit/exact";
        f.call(
            "mandate-create",
            cmd(
                "mandate",
                json!({"mandate_id":"mandate","subject":"requester","actions":["settle"],"resource":resource,"max_amount_minor":1000,"currency":"EUR","beneficiary":"supplier"}),
            ),
            "fixture-admin-token",
        );
        let action = f.call(
            "request",
            cmd(
                "request",
                json!({"mandate_id":"mandate","resource":resource,"amount_minor":450,"currency":"EUR","beneficiary":"supplier","idempotency_key":"exact"}),
            ),
            "fixture-requester-token",
        );
        // Before approval: the exact command bytes do not bind anything.
        let approval_command = serde_json::to_vec(&cmd(
            "approve",
            json!({"action_id":action["id"],"mandate_id":"mandate","content_digest":action["content_digest"],"resource":resource,"amount_minor":450,"currency":"EUR","beneficiary":"supplier"}),
        ))
        .unwrap();
        let root = RootStore::open(&f.0, false).unwrap();
        assert!(root
            .paea_git_agreement_match(&approval_command)
            .unwrap()
            .is_none());
        root.close().unwrap();
        f.call(
            "approve",
            serde_json::from_slice(&approval_command).unwrap(),
            "fixture-approver-token",
        );
        // Now the retained original command bytes are exactly these bytes: P1.
        let root = RootStore::open(&f.0, false).unwrap();
        let matched = root
            .paea_git_agreement_match(&approval_command)
            .unwrap()
            .unwrap();
        assert_eq!(matched.approver, "approver");
        // An altered variant of those bytes binds nothing.
        let mut other = approval_command.clone();
        other[0] ^= 1;
        assert!(root.paea_git_agreement_match(&other).unwrap().is_none());
        root.close().unwrap();
    }

    /// The recipe binds the full canonical identity: same candidate but a
    /// different expected old OID (beneficiary) never matches.
    #[test]
    fn agreement_binds_beneficiary_old_oid() {
        let f = Fixture::new();
        let candidate = "f".repeat(40);
        let old = "1".repeat(40);
        let resource = format!("project/repo/refs/heads/release/{candidate}");
        f.call(
            "mandate-create",
            cmd(
                "mandate",
                json!({"mandate_id":"mandate","subject":"requester","actions":["settle"],"resource":resource,"max_amount_minor":1000,"currency":"OID","beneficiary":old}),
            ),
            "fixture-admin-token",
        );
        let action = f.call(
            "request",
            cmd(
                "request",
                json!({"mandate_id":"mandate","resource":resource,"amount_minor":1,"currency":"OID","beneficiary":old,"idempotency_key":"key-oid"}),
            ),
            "fixture-requester-token",
        );
        f.call(
            "approve",
            cmd(
                "approve",
                json!({"action_id":action["id"],"mandate_id":"mandate","content_digest":action["content_digest"],"resource":resource,"amount_minor":1,"currency":"OID","beneficiary":old}),
            ),
            "fixture-approver-token",
        );
        let root = RootStore::open(&f.0, false).unwrap();
        // Same candidate, different expected old OID: a different action.
        let mut value: Value =
            serde_json::from_slice(&git_action_bytes(&candidate)).unwrap();
        value["action"]["expected_old_oid"] = "2".repeat(40).into();
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(root
            .paea_git_agreement_match(&bytes)
            .unwrap()
            .is_none());
        root.close().unwrap();
    }

    /// A configuration change voids every earlier agreement at last use: the
    /// same locked read that matches the approval also verifies its captured
    /// revision is still current, so a stale approval is an explicit None.
    #[test]
    fn agreement_is_void_after_trust_revision_change() {
        let f = Fixture::new();
        let candidate = "f".repeat(40);
        approve_candidate(&f, "first", &candidate);
        let root = RootStore::open(&f.0, true).unwrap();
        let matched = root
            .paea_git_agreement_match(&git_action_bytes(&candidate))
            .unwrap()
            .unwrap();
        assert_eq!(matched.revision, f.revision());
        // Real public configuration change through the engine — no journal
        // edit. The initial config bytes are re-supplied under a new event id
        // so the same identities stay configured after the revision bump.
        let initial = json!({
            "schema_version":1,
            "event_id":"agreement-stale",
            "expected_revision":f.revision(),
            "reason":"Explicit revision change to void the captured agreement",
            "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},
            "identities":[
                {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
                {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},
                {"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin","effect"]}
            ]
        });
        root.trust(
            "config-apply",
            &serde_json::to_vec(&initial).unwrap(),
            "",
        )
        .unwrap();
        assert_eq!(f.revision(), matched.revision + 1);
        // The approval evidence is immutable but now stale: explicit None, so
        // the transport guard refuses with its single typed code.
        assert!(root
            .paea_git_agreement_match(&git_action_bytes(&candidate))
            .unwrap()
            .is_none());
        root.close().unwrap();
    }

    /// A public mandate revocation after the approval voids the agreement at
    /// last use without any configuration revision change: the agreement's
    /// freshness is the underlying mandate's current state, not just the
    /// revision captured in the same locked read.
    #[test]
    fn agreement_is_void_after_public_mandate_revocation() {
        let f = Fixture::new();
        let candidate = "f".repeat(40);
        let resource = format!("project/repo/refs/heads/release/{candidate}");
        f.call(
            "mandate-create",
            cmd(
                "mandate",
                json!({"mandate_id":"mandate","subject":"requester","actions":["settle"],"resource":resource,"max_amount_minor":1000,"currency":"OID","beneficiary":"1".repeat(40)}),
            ),
            "fixture-admin-token",
        );
        let action = f.call(
            "request",
            cmd(
                "request",
                json!({"mandate_id":"mandate","resource":resource,"amount_minor":1,"currency":"OID","beneficiary":"1".repeat(40),"idempotency_key":"key-revoked"}),
            ),
            "fixture-requester-token",
        );
        f.call(
            "approve",
            cmd(
                "approve",
                json!({"action_id":action["id"],"mandate_id":"mandate","content_digest":action["content_digest"],"resource":resource,"amount_minor":1,"currency":"OID","beneficiary":"1".repeat(40)}),
            ),
            "fixture-approver-token",
        );
        let root = RootStore::open(&f.0, false).unwrap();
        // Valid control: the exact approved agreement matches before revoke.
        assert!(root
            .paea_git_agreement_match(&git_action_bytes(&candidate))
            .unwrap()
            .is_some());
        root.close().unwrap();
        let revision_before = f.revision();
        // Real public revocation through the engine — no revision change, no
        // journal edit: the approval evidence itself is immutable.
        let revoked = f.call(
            "mandate-revoke",
            cmd("revoke", json!({"mandate_id":"mandate"})),
            "fixture-admin-token",
        );
        assert_eq!(revoked["state"], "revoked");
        assert_eq!(f.revision(), revision_before);
        // The agreement must now be void at last use: explicit None.
        let root = RootStore::open(&f.0, false).unwrap();
        assert!(root
            .paea_git_agreement_match(&git_action_bytes(&candidate))
            .unwrap()
            .is_none());
        root.close().unwrap();
    }

    /// A replayed approval creates no second effect and never fabricates a
    /// second match; a denied approval (wrong role or non-exact terms) never
    /// matches.
    #[test]
    fn agreement_replays_and_denials_never_match() {
        let f = Fixture::new();
        let candidate = "f".repeat(40);
        let resource = format!("project/repo/refs/heads/release/{candidate}");
        f.call(
            "mandate-create",
            cmd(
                "mandate",
                json!({"mandate_id":"mandate","subject":"requester","actions":["settle"],"resource":resource,"max_amount_minor":1000,"currency":"OID","beneficiary":"1".repeat(40)}),
            ),
            "fixture-admin-token",
        );
        let action = f.call(
            "request",
            cmd(
                "request",
                json!({"mandate_id":"mandate","resource":resource,"amount_minor":1,"currency":"OID","beneficiary":"1".repeat(40),"idempotency_key":"key-denied"}),
            ),
            "fixture-requester-token",
        );
        // Wrong role: the requester is refused by the existing approve guard.
        let denied_role = cmd(
            "deny-role",
            json!({"action_id":action["id"],"mandate_id":"mandate","content_digest":action["content_digest"],"resource":resource,"amount_minor":1,"currency":"OID","beneficiary":"1".repeat(40)}),
        );
        assert_eq!(
            f.call("approve", denied_role, "fixture-requester-token")["state"],
            "denied"
        );
        // Non-exact terms: refused by the existing exact-agreement guard.
        let denied_terms = cmd(
            "deny-terms",
            json!({"action_id":action["id"],"mandate_id":"mandate","content_digest":action["content_digest"],"resource":resource,"amount_minor":2,"currency":"OID","beneficiary":"1".repeat(40)}),
        );
        assert_eq!(
            f.call("approve", denied_terms, "fixture-approver-token")["state"],
            "denied"
        );
        let root = RootStore::open(&f.0, false).unwrap();
        assert!(root
            .paea_git_agreement_match(&git_action_bytes(&candidate))
            .unwrap()
            .is_none());
        root.close().unwrap();
        // Exact approval, then an identical replay: the durable state is
        // unchanged and the match stays the same single approval.
        let approval = cmd(
            "approve-exact",
            json!({"action_id":action["id"],"mandate_id":"mandate","content_digest":action["content_digest"],"resource":resource,"amount_minor":1,"currency":"OID","beneficiary":"1".repeat(40)}),
        );
        assert_eq!(
            f.call("approve", approval.clone(), "fixture-approver-token")["state"],
            "approved"
        );
        let raw_before =
            std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap();
        assert_eq!(
            f.call("approve", approval, "fixture-approver-token")["state"],
            "approved"
        );
        assert_eq!(
            std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap(),
            raw_before
        );
        let root = RootStore::open(&f.0, false).unwrap();
        let matched = root
            .paea_git_agreement_match(&git_action_bytes(&candidate))
            .unwrap()
            .unwrap();
        assert_eq!(matched.approver, "approver");
        root.close().unwrap();
    }

    /// The terms observation derives exactly the fixed recipe fields the guard
    /// matches on, reads the same current revision, and never fabricates terms
    /// for non-recipe bytes or when no Trust state exists yet. A full real
    /// approve built from these derived terms is then matched by the guard:
    /// presentation and consumption agree on the same terms, one source.
    #[test]
    fn agreement_terms_derive_recipe_and_produce_a_guard_match() {
        let f = Fixture::new();
        let candidate = "f".repeat(40);
        let root = RootStore::open(&f.0, false).unwrap();
        // No state derivation gap: the derived terms name the exact resource,
        // amount, currency, beneficiary and the current revision.
        let terms = root
            .paea_git_agreement_terms(&git_action_bytes(&candidate))
            .unwrap()
            .unwrap();
        assert_eq!(terms.binding, "recipe");
        assert_eq!(
            terms.resource,
            format!("project/repo/refs/heads/release/{candidate}")
        );
        assert_eq!(terms.amount_minor, 1);
        assert_eq!(terms.currency, "OID");
        assert_eq!(terms.beneficiary, "1".repeat(40));
        assert_eq!(terms.revision, f.revision());
        // Non-recipe bytes (a payment settle is not a repository.promote) and
        // non-JSON bytes derive nothing: explicit None, never a fallback.
        let payment = serde_json::to_vec(&json!({
            "schema_version":1,
            "canonicalization":"paea-json-v1",
            "action":{"kind":"payment.settle","invoice_id":"inv"},
            "context":{"project_id":"project","principal_id":"operator","instance_id":"instance","mandate_id":"mandate","mandate_revision":1,"policy_revision":1,"policy_digest":"aa".repeat(32),"tool_manifest_digest":"bb".repeat(32),"sources":[]}
        }))
        .unwrap();
        assert!(root.paea_git_agreement_terms(&payment).unwrap().is_none());
        assert!(root.paea_git_agreement_terms(b"not-json").unwrap().is_none());
        root.close().unwrap();
        // Approving through the real producer chain with the derived terms —
        // nothing else — yields an agreement the consumption guard matches.
        let resource = terms.resource.clone();
        f.call(
            "mandate-create",
            cmd(
                "terms-mandate",
                json!({"mandate_id":"mandate-terms","subject":"requester","actions":["settle"],"resource":resource,"max_amount_minor":1000,"currency":"OID","beneficiary":terms.beneficiary}),
            ),
            "fixture-admin-token",
        );
        let action = f.call(
            "request",
            cmd(
                "terms-request",
                json!({"mandate_id":"mandate-terms","resource":resource,"amount_minor":terms.amount_minor,"currency":terms.currency,"beneficiary":terms.beneficiary,"idempotency_key":"key-terms"}),
            ),
            "fixture-requester-token",
        );
        assert_eq!(action["state"], "pending_approval");
        f.call(
            "approve",
            cmd(
                "terms-approve",
                json!({"action_id":action["id"],"mandate_id":"mandate-terms","content_digest":action["content_digest"],"resource":resource,"amount_minor":terms.amount_minor,"currency":terms.currency,"beneficiary":terms.beneficiary}),
            ),
            "fixture-approver-token",
        );
        let root = RootStore::open(&f.0, false).unwrap();
        let bytes = git_action_bytes(&candidate);
        assert!(root.paea_git_agreement_match(&bytes).unwrap().is_some());
        root.close().unwrap();
    }
}
