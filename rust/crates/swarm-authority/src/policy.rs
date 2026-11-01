//! Attribute-based access decision of the authority daemon (phase B, B6).
//!
//! Two layers, evaluated in the daemon before any request is relayed:
//!
//! 1. **Route classification** ([`classify`]) — a protocol invariant mapping
//!    each engine route to one action family: `read`, `write`, `approve`,
//!    `administer`. It mirrors the engine route inventory; an unknown
//!    mutating route classifies as `administer` (deny by default for
//!    everyone but administrators), an unknown GET/HEAD as `read`.
//! 2. **Operator policy** ([`Policy`]) — ordered grants persisted in
//!    `policy.json` in the daemon configuration directory (installation
//!    default shipped in `config/policy-defaults.json`). A grant allows a
//!    set of actions to a role, optionally only when every listed subject
//!    attribute has the exact value (ABAC condition).
//!
//! The Biba floor ([`integrity_floor`]) is enforced on top of the policy and
//! cannot be lowered by any grant: a permissive rule never lets a low
//! integrity subject write, approve or administer. Requester/approver
//! separation stays enforced by the engine trust layer (the daemon does not
//! know who requested an action). [`decide`] is pure: no IO, no clock.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Action families, from least to most privileged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Read,
    Write,
    Approve,
    Administer,
}

/// Biba integrity required for an action family (protocol invariant, not an
/// operator choice): reading needs a signed-in account (1), writing 2,
/// approving and administering 3 — the level of the first `init` admin.
pub fn integrity_floor(action: Action) -> i64 {
    match action {
        Action::Read => 1,
        Action::Write => 2,
        Action::Approve | Action::Administer => 3,
    }
}

/// Settings/policy families whose mutations change how Swarm runs.
const ADMINISTER_PREFIXES: [&str; 14] = [
    "/api/v1/native-settings",
    "/api/v1/activity-policy",
    "/api/v1/admin-advice-budget",
    "/api/v1/graph-performance",
    "/api/v1/log-read-policy",
    "/api/v1/project-profile",
    "/api/v1/provider-wait",
    "/api/v1/providers",
    "/api/v1/quotas",
    "/api/v1/run-limits",
    "/api/v1/web-requests",
    "/api/v1/work-budget",
    "/api/v1/pricing",
    "/api/v1/paea/reconcile-effects",
];

/// Routes that record an approval decision (or submit to the trust queue).
const APPROVE_PATHS: [&str; 2] = ["/api/v1/paea/git-call/approval", "/api/v1/trust-actions"];

/// Mutating routes that only compute and return (no state change): the
/// `/api/v1/protection/*` handlers are stateless functions of the body.
const READ_ONLY_POSTS: [&str; 10] = [
    "/api/v1/cockpit-read",
    "/api/v1/live/alerts/evaluate",
    "/api/v1/pricing/estimate",
    "/api/v1/protection/compare",
    "/api/v1/protection/inspect",
    "/api/v1/protection/check-approval",
    "/api/v1/protection/receipt-report",
    "/api/v1/protection/resolve-payment",
    "/api/v1/protection/linux-prerequisites",
    "/api/v1/paea/git-call/approval/presentation",
];

/// Ordinary work mutations (missions, preparations, annotations, visits...).
const WRITE_PREFIXES: [&str; 13] = [
    "/api/v1/action",
    "/api/v1/mission",
    "/api/v1/lifecycle/missions",
    "/api/v1/preparations",
    "/api/v1/planning",
    "/api/v1/live/annotations",
    "/api/v1/live/scenario",
    "/api/v1/visit",
    "/api/v1/visit-state",
    "/api/v1/work-resume",
    "/api/v1/work-recorded-resume",
    "/api/v1/works",
    "/api/v1/paea/git-call",
];

fn under(path: &str, prefix: &str) -> bool {
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

/// Classify one request. `path` is the URI path without query. Previews,
/// descriptions and estimates (`…/preview`, `…/describe`, `…/estimate`) are
/// read-only by engine contract (they return a token, never mutate).
pub fn classify(method: &str, path: &str) -> Action {
    if matches!(method, "GET" | "HEAD") {
        return Action::Read;
    }
    if READ_ONLY_POSTS.contains(&path) || ["/preview", "/describe", "/estimate"].iter().any(|s| path.ends_with(s)) {
        return Action::Read;
    }
    if APPROVE_PATHS.contains(&path) {
        return Action::Approve;
    }
    if ADMINISTER_PREFIXES.iter().any(|p| under(path, p)) {
        return Action::Administer;
    }
    if WRITE_PREFIXES.iter().any(|p| under(path, p)) {
        return Action::Write;
    }
    Action::Administer
}

/// One grant: `role` may perform `actions` when every `when` attribute of
/// the subject equals the given value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub role: String,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub when: BTreeMap<String, String>,
}

/// Operator policy document (`policy.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema_version: i64,
    pub grants: Vec<Grant>,
}

/// Installation default, read from the shipped policy file.
pub const DEFAULTS: &str = include_str!("../config/policy-defaults.json");

/// Subject attributes the decision uses (from the signed account).
#[derive(Clone, Debug)]
pub struct Subject<'a> {
    pub roles: &'a [String],
    pub integrity: i64,
    pub attributes: &'a BTreeMap<String, String>,
}

/// Outcome with a stable reason code (never contains a secret).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub allowed: bool,
    pub action: Action,
    /// `policy-granted`, `policy-no-grant` or `policy-integrity-floor`.
    pub reason: &'static str,
}

impl Policy {
    /// Decode and validate. Errors: `authority-policy-format` (malformed,
    /// unknown field), `authority-policy-invalid` (schema not 1, empty
    /// grant list, a grant with an unknown role or no action). No IO.
    pub fn decode(raw: &[u8]) -> Result<Self, &'static str> {
        let p: Self = serde_json::from_slice(raw).map_err(|_| "authority-policy-format")?;
        if p.schema_version != 1
            || p.grants.is_empty()
            || p.grants.iter().any(|g| g.actions.is_empty() || !crate::accounts::ROLES.contains(&g.role.as_str()))
        {
            return Err("authority-policy-invalid");
        }
        Ok(p)
    }
    /// Shipped installation default (always valid; checked by tests).
    pub fn installation() -> Result<Self, &'static str> {
        Self::decode(DEFAULTS.as_bytes())
    }
    /// Load `dir/policy.json` if present, else the installation default. A
    /// present invalid file refuses (no silent fallback); other read errors
    /// give `authority-policy-io`.
    pub fn load(dir: &std::path::Path) -> Result<Self, &'static str> {
        match std::fs::read(dir.join("policy.json")) {
            Ok(raw) => Self::decode(&raw),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::installation(),
            Err(_) => Err("authority-policy-io"),
        }
    }
}

/// Decide whether `subject` may send `method path`. Deny by default: allowed
/// only if a grant matches (role held, action listed, all `when` attributes
/// equal) **and** the subject meets the Biba floor of the action. The floor
/// is checked first so its refusal is reported even under a permissive grant.
pub fn decide(policy: &Policy, subject: &Subject<'_>, method: &str, path: &str) -> Decision {
    let action = classify(method, path);
    if subject.integrity < integrity_floor(action) {
        return Decision { allowed: false, action, reason: "policy-integrity-floor" };
    }
    let granted = policy.grants.iter().any(|g| {
        subject.roles.contains(&g.role)
            && g.actions.contains(&action)
            && g.when.iter().all(|(k, v)| subject.attributes.get(k) == Some(v))
    });
    Decision { allowed: granted, action, reason: if granted { "policy-granted" } else { "policy-no-grant" } }
}
