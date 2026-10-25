//! Trust policy and identity types for the swarm trust subsystem.
//!
//! These types are data carriers only. Nothing here performs IO, reads
//! configuration, authenticates anyone, or grants authority: a
//! `TrustIdentity` produced by [`prepare_identities`] is persisted input,
//! not a claim about any live principal. All bounds below are technical
//! compatibility limits (Go duration encoding, existing JSON bounds), not
//! policy decisions; every field remains persisted and editable in Admin
//! by the parent process.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Static trust policy. All four fields are mandatory in input: no serde
/// defaults, nulls, or fractional durations are accepted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TrustPolicy {
    /// Mandate lifetime in seconds; 0 means unlimited. Range 0..=9223372036.
    pub mandate_ttl_seconds: i64,
    /// Unknown-action reconcile interval in seconds; 0 means immediate.
    /// Range 0..=9223372036.
    pub unknown_reconcile_seconds: i64,
    /// Evidence retention in days; 1..=9007199254740991 (existing exact
    /// JSON-integer bound).
    pub evidence_retention_days: i64,
    /// Whether separation of duties is required.
    pub require_separation: bool,
}

impl TrustPolicy {
    /// Installation-only default. Not a deserialization fallback: input
    /// must always carry all four fields explicitly.
    pub(crate) fn defaults() -> Self {
        TrustPolicy {
            mandate_ttl_seconds: 3600,
            unknown_reconcile_seconds: 0,
            evidence_retention_days: 365,
            require_separation: true,
        }
    }

    /// Validates technical bounds only (Go duration representation for the
    /// two durations, existing JSON bound for retention). Returns a static,
    /// content-free error string on failure.
    ///
    /// Effects: none (pure). Never accepts null or fractional values here;
    /// serde itself rejects non-integers before this runs.
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        const MAX_DURATION: i64 = 9_223_372_036; // Go-compatible seconds bound
        const MAX_DAYS: i64 = 9_007_199_254_740_991; // exact JSON-integer bound
        if !(0..=MAX_DURATION).contains(&self.mandate_ttl_seconds) {
            return Err("mandate_ttl_seconds out of range");
        }
        if !(0..=MAX_DURATION).contains(&self.unknown_reconcile_seconds) {
            return Err("unknown_reconcile_seconds out of range");
        }
        if !(1..=MAX_DAYS).contains(&self.evidence_retention_days) {
            return Err("evidence_retention_days out of range");
        }
        Ok(())
    }
}

/// Deserialization-only shape for incoming identity material. Deliberately
/// not `Debug`/`Display` so the raw token can never leak through formatting.
/// `disabled` defaults to `false` when omitted, but an explicit JSON `null`
/// is rejected by serde.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TrustIdentityInput {
    pub subject: String,
    pub token: String,
    pub integrity: i64,
    pub roles: Vec<String>,
    #[serde(default)]
    pub disabled: bool,
}

/// Persisted identity. Stores the SHA-256 token digest, never the raw token.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct TrustIdentity {
    pub subject: String,
    pub token_hash: String,
    pub integrity: i64,
    pub roles: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Domain-separated SHA-256 hex digest of a token.
/// Pure function; the token is hashed, never stored or logged.
pub(crate) fn token_hash(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"trust-token-v1\x00");
    h.update(token.as_bytes());
    let d = h.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Credential representation check: minimum length, forbidden whitespace
/// (space, tab, CR, LF), and forbidden lowercase `remplacer-`/`replace-`
/// prefixes. Not an authentication: it only filters well-formed tokens.
pub(crate) fn credential_valid(token: &str) -> bool {
    if token.len() < 16 {
        return false;
    }
    if token
        .bytes()
        .any(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
    {
        return false;
    }
    let lower_starts = |p: &str| {
        token
            .as_bytes()
            .get(..p.len())
            .map(|s| s.eq_ignore_ascii_case(p.as_bytes()))
            .unwrap_or(false)
    };
    !(lower_starts("remplacer-") || lower_starts("replace-"))
}

/// Prepares persisted identities from inputs.
///
/// Rules (historical Go contract preserved):
/// - at least 3 identities;
/// - subjects unique and token digests unique;
/// - subject must satisfy `swarm_identity_contract::name`;
/// - integrity in 1..=3;
/// - token passes [`credential_valid`];
/// - roles restricted to `requester|approver|admin|effect`, then sorted
///   lexicographically; duplicates are kept, not deduplicated and not
///   rejected (Go allows them);
/// - `disabled` is honored verbatim.
///
/// Errors are static and content-free: no token or subject material is ever
/// included. No IO, no authentication, no authority derived from input.
pub(crate) fn prepare_identities(
    inputs: Vec<TrustIdentityInput>,
) -> Result<Vec<TrustIdentity>, &'static str> {
    if inputs.len() < 3 {
        return Err("at least three identities required");
    }
    let mut out = Vec::with_capacity(inputs.len());
    let mut seen_subject: Vec<String> = Vec::new();
    let mut seen_hash: Vec<String> = Vec::new();
    for i in inputs {
        if !swarm_identity_contract::name(i.subject.as_bytes()) {
            return Err("subject does not match identity contract");
        }
        if seen_subject.contains(&i.subject) {
            return Err("duplicate subject");
        }
        if !(1..=3).contains(&i.integrity) {
            return Err("integrity out of range");
        }
        if !credential_valid(&i.token) {
            return Err("token fails credential format checks");
        }
        let hash = token_hash(&i.token);
        if seen_hash.contains(&hash) {
            return Err("duplicate token digest");
        }
        let mut roles = Vec::with_capacity(i.roles.len());
        for r in &i.roles {
            match r.as_str() {
                "requester" | "approver" | "admin" | "effect" => roles.push(r.clone()),
                _ => return Err("role not permitted"),
            }
        }
        roles.sort();
        seen_subject.push(i.subject.clone());
        seen_hash.push(hash.clone());
        out.push(TrustIdentity {
            subject: i.subject,
            token_hash: hash,
            integrity: i.integrity,
            roles,
            disabled: i.disabled,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        subject: &str,
        token: &str,
        integrity: i64,
        roles: &[&str],
        disabled: bool,
    ) -> TrustIdentityInput {
        TrustIdentityInput {
            subject: subject.to_string(),
            token: token.to_string(),
            integrity,
            roles: roles.iter().map(|s| s.to_string()).collect(),
            disabled,
        }
    }

    #[test]
    fn valid_identities_round_trip() {
        let out = prepare_identities(vec![
            input("alpha-1", "token-one-abcdef", 1, &["admin"], false),
            input("beta-2", "token-two-abcdef", 2, &["requester"], false),
            input("gamma-3", "token-three-abcd", 3, &["approver"], true),
        ])
        .expect("valid");
        assert_eq!(out.len(), 3);
        assert!(out[2].disabled);
        assert_eq!(out[0].roles, vec!["admin".to_string()]);
    }

    #[test]
    fn policy_zero_and_range_rejected() {
        let mut p = TrustPolicy {
            mandate_ttl_seconds: 0,
            unknown_reconcile_seconds: 0,
            evidence_retention_days: 1,
            require_separation: true,
        };
        assert!(p.validate().is_ok()); // zero durations are legal (unlimited/immediate)
        p.mandate_ttl_seconds = 9_223_372_037;
        assert!(p.validate().is_err());
        p.mandate_ttl_seconds = 0;
        p.evidence_retention_days = 0;
        assert!(p.validate().is_err());
        p.evidence_retention_days = 1;
        p.unknown_reconcile_seconds = -1;
        assert!(p.validate().is_err());
    }

    #[test]
    fn policy_null_and_unknown_rejected() {
        let bad = r#"{"mandate_ttl_seconds":null,"unknown_reconcile_seconds":0,"evidence_retention_days":1,"require_separation":true}"#;
        assert!(serde_json::from_str::<TrustPolicy>(bad).is_err());
        let frac = r#"{"mandate_ttl_seconds":1.5,"unknown_reconcile_seconds":0,"evidence_retention_days":1,"require_separation":true}"#;
        assert!(serde_json::from_str::<TrustPolicy>(frac).is_err());
        let unk = r#"{"mandate_ttl_seconds":1,"unknown_reconcile_seconds":0,"evidence_retention_days":1,"require_separation":true,"extra":1}"#;
        assert!(serde_json::from_str::<TrustPolicy>(unk).is_err());
    }

    #[test]
    fn duplicates_rejected_roles_sorted_dedup_kept() {
        // duplicate subject
        assert!(prepare_identities(vec![
            input("a-1", "token-one-abcdef", 1, &[], false),
            input("a-1", "token-two-abcdef", 2, &[], false),
            input("c-3", "token-three-abcd", 3, &[], false),
        ])
        .is_err());
        // duplicate token
        assert!(prepare_identities(vec![
            input("a-1", "same-token-aaaaaa", 1, &[], false),
            input("b-2", "same-token-aaaaaa", 2, &[], false),
            input("c-3", "token-three-abcd", 3, &[], false),
        ])
        .is_err());
        // duplicate roles kept and sorted, not deduplicated
        let out = prepare_identities(vec![
            input(
                "a-1",
                "token-one-abcdef",
                1,
                &["effect", "admin", "effect"],
                false,
            ),
            input("b-2", "token-two-abcdef", 2, &[], false),
            input("c-3", "token-three-abcd", 3, &[], false),
        ])
        .expect("ok");
        assert_eq!(
            out[0].roles,
            vec![
                "admin".to_string(),
                "effect".to_string(),
                "effect".to_string()
            ]
        );
    }

    #[test]
    fn credential_and_disabled_rules() {
        assert!(!credential_valid("short"));
        assert!(!credential_valid("has space in it yes"));
        assert!(!credential_valid("replace-this-is-long"));
        assert!(!credential_valid("remplacer-long-token"));
        assert!(credential_valid("Remplace-long-token")); // case-sensitive prefix check
        assert!(token_hash("x") != token_hash("y"));
        // disabled input is honored (omitted => false is exercised by serde default)
        let out = prepare_identities(vec![
            input("a-1", "token-one-abcdef", 1, &[], false),
            input("b-2", "token-two-abcdef", 2, &[], false),
            input("c-3", "token-three-abcd", 3, &[], false),
        ])
        .expect("ok");
        assert!(!out[0].disabled);
    }
}
