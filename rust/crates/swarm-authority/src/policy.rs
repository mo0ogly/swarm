//! Attribute-based access decision of the authority daemon (phase B, B6;
//! roles and permission matrix, B8).
//!
//! Three layers, evaluated in the daemon before any request is relayed:
//!
//! 1. **Permission catalog** ([`CATALOG`], [`classify_permission`]) — a
//!    protocol invariant mapping each engine or daemon route to one
//!    permission (`missions.write`, `settings.read`, `approvals.approve`…).
//!    Each permission belongs to an action family ([`Action`]) that fixes its
//!    Biba floor. An unknown mutating route maps to `unclassified.write`
//!    (only the built-in `admin` role holds it: deny by default), an unknown
//!    GET/HEAD to `cockpit.read`.
//! 2. **Operator policy** ([`Policy`], schema 2) — the roles (built-in
//!    `viewer`, `operator`, `approver`, `admin`, plus custom ones) and, per
//!    role, granted permissions, optionally only when every listed subject
//!    attribute has the exact value (ABAC). Persisted as `policy.json` in the
//!    daemon configuration directory (installation default
//!    `config/policy-defaults.json`); a schema-1 document (action families
//!    per role) is read and converted.
//! 3. **Biba floor** ([`integrity_floor`]) — on top of the policy, never
//!    lowered by a grant: read ≥ 1, write ≥ 2, approve/administer ≥ 3.
//!
//! Invariants checked by [`Policy::decode`]: built-in roles present, `admin`
//! holds every permission unconditionally (no lockout by editing), role keys
//! unique contract names, grants reference known roles and catalog
//! permissions. Requester/approver separation stays enforced by the engine
//! trust layer. [`decide`] is pure: no IO, no clock.
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

/// One catalog permission: stable key, family, section and French label
/// (translated by the Admin UI catalog).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Permission {
    pub key: &'static str,
    pub family: Action,
    pub section: &'static str,
    pub label: &'static str,
}

/// The permission catalog (protocol invariant; order = display order).
pub const CATALOG: [Permission; 14] = [
    Permission { key: "cockpit.read", family: Action::Read, section: "Missions", label: "Consulter le cockpit, les missions et les agents" },
    Permission { key: "missions.write", family: Action::Write, section: "Missions", label: "Créer et piloter les missions, tâches, graphes et visites" },
    Permission { key: "preparations.read", family: Action::Read, section: "Préparations", label: "Consulter les préparations" },
    Permission { key: "preparations.write", family: Action::Write, section: "Préparations", label: "Mener les préparations (envois, commandes)" },
    Permission { key: "approvals.approve", family: Action::Approve, section: "Approbations", label: "Approuver les actions de confiance et appels Git" },
    Permission { key: "settings.read", family: Action::Read, section: "Réglages", label: "Consulter les réglages du moteur et ses politiques" },
    Permission { key: "settings.write", family: Action::Administer, section: "Réglages", label: "Modifier les réglages du moteur et ses politiques" },
    Permission { key: "providers.read", family: Action::Read, section: "Fournisseurs", label: "Consulter fournisseurs, tarifs et attentes" },
    Permission { key: "providers.write", family: Action::Administer, section: "Fournisseurs", label: "Modifier fournisseurs, tarifs et attentes" },
    Permission { key: "budgets.read", family: Action::Read, section: "Budgets", label: "Consulter budgets, plafonds et limites" },
    Permission { key: "budgets.write", family: Action::Administer, section: "Budgets", label: "Modifier budgets, plafonds et limites" },
    Permission { key: "accounts.manage", family: Action::Administer, section: "Administration", label: "Gérer comptes, sessions, clés, habilitations et réglages du démon" },
    Permission { key: "audit.read", family: Action::Administer, section: "Administration", label: "Consulter et purger le journal d’accès" },
    Permission { key: "unclassified.write", family: Action::Administer, section: "Administration", label: "Routes d’écriture non classées (réservé aux administrateurs)" },
];

/// Catalog entry of `key`.
pub fn permission(key: &str) -> Option<&'static Permission> {
    CATALOG.iter().find(|p| p.key == key)
}

/// Built-in roles: always present, never deleted; `admin` is not editable.
pub const BUILTIN_ROLES: [&str; 4] = ["viewer", "operator", "approver", "admin"];

/// Daemon-served account/session/policy administration (B8); the
/// self-service `/api/v1/authority/me` subtree is handled before the policy.
pub const AUTHORITY_PREFIX: &str = "/api/v1/authority";

const SETTINGS: [&str; 7] = [
    "/api/v1/native-settings",
    "/api/v1/activity-policy",
    "/api/v1/graph-performance",
    "/api/v1/log-read-policy",
    "/api/v1/project-profile",
    "/api/v1/web-requests",
    "/api/v1/paea/reconcile-effects",
];
const PROVIDERS: [&str; 3] = ["/api/v1/providers", "/api/v1/pricing", "/api/v1/provider-wait"];
const BUDGETS: [&str; 4] = ["/api/v1/quotas", "/api/v1/run-limits", "/api/v1/work-budget", "/api/v1/admin-advice-budget"];

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

/// Ordinary work mutations (missions, annotations, visits, graphs...).
const MISSION_WRITES: [&str; 13] = [
    "/api/v1/action",
    "/api/v1/mission",
    "/api/v1/lifecycle/missions",
    "/api/v1/planning",
    "/api/v1/live/annotations",
    "/api/v1/live/scenario",
    "/api/v1/visit",
    "/api/v1/visit-state",
    "/api/v1/work-resume",
    "/api/v1/work-recorded-resume",
    "/api/v1/works",
    "/api/v1/paea/git-call",
    "/api/v1/graph-batch",
];
const PREPARATIONS: &str = "/api/v1/preparations";

/// `path` is `prefix` or below it (segment boundary, no string prefix match).
pub fn under(path: &str, prefix: &str) -> bool {
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

/// Permission of one request (`path` without query). Daemon administration
/// is `accounts.manage` except the audit (`audit.read`). Reads and the
/// read-only computations (`…/preview`, `…/describe`, `…/estimate`, the
/// stateless protection functions) take the read permission of their
/// family; approvals `approvals.approve`; unknown mutations
/// `unclassified.write`.
pub fn classify_permission(method: &str, path: &str) -> &'static str {
    if under(path, AUTHORITY_PREFIX) {
        return if under(path, "/api/v1/authority/audit") { "audit.read" } else { "accounts.manage" };
    }
    let read = matches!(method, "GET" | "HEAD")
        || READ_ONLY_POSTS.contains(&path)
        || ["/preview", "/describe", "/estimate"].iter().any(|s| path.ends_with(s));
    let pick = |write: &'static str, reading: &'static str| if read { reading } else { write };
    if !read && APPROVE_PATHS.contains(&path) {
        return "approvals.approve";
    }
    if SETTINGS.iter().any(|p| under(path, p)) {
        return pick("settings.write", "settings.read");
    }
    if PROVIDERS.iter().any(|p| under(path, p)) {
        return pick("providers.write", "providers.read");
    }
    if BUDGETS.iter().any(|p| under(path, p)) {
        return pick("budgets.write", "budgets.read");
    }
    if under(path, PREPARATIONS) {
        return pick("preparations.write", "preparations.read");
    }
    if read {
        return "cockpit.read";
    }
    if MISSION_WRITES.iter().any(|p| under(path, p)) {
        return "missions.write";
    }
    "unclassified.write"
}

/// Action family of one request (see [`classify_permission`]).
pub fn classify(method: &str, path: &str) -> Action {
    permission(classify_permission(method, path)).map(|p| p.family).unwrap_or(Action::Administer)
}

/// One role of the policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
}

/// One grant: `role` holds `permissions` when every `when` attribute of the
/// subject equals the given value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub role: String,
    pub permissions: Vec<String>,
    #[serde(default)]
    pub when: BTreeMap<String, String>,
}

/// Operator policy document (`policy.json`, schema 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema_version: i64,
    pub roles: Vec<Role>,
    pub grants: Vec<Grant>,
}

/// Schema-1 document (action families per fixed role), read for upgrade.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyV1 {
    #[allow(dead_code)]
    schema_version: i64,
    grants: Vec<GrantV1>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrantV1 {
    role: String,
    actions: Vec<Action>,
    #[serde(default)]
    when: BTreeMap<String, String>,
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

/// Outcome with stable codes (never contains a secret).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub allowed: bool,
    pub action: Action,
    pub permission: &'static str,
    /// `policy-granted`, `policy-no-grant` or `policy-integrity-floor`.
    pub reason: &'static str,
}

fn builtin_label(key: &str) -> &'static str {
    match key {
        "viewer" => "Lecteur",
        "operator" => "Opérateur",
        "approver" => "Approbateur",
        _ => "Administrateur",
    }
}

impl Policy {
    /// Decode and validate. Errors: `authority-policy-format` (malformed,
    /// unknown field, unknown schema), `authority-policy-invalid` (invariant
    /// broken: see module docs). A schema-1 document is converted (each
    /// family becomes every catalog permission of that family; `admin` gets
    /// the whole catalog). No IO.
    pub fn decode(raw: &[u8]) -> Result<Self, &'static str> {
        let version = serde_json::from_slice::<serde_json::Value>(raw).map_err(|_| "authority-policy-format")?["schema_version"].as_i64();
        let p: Self = match version {
            Some(2) => serde_json::from_slice(raw).map_err(|_| "authority-policy-format")?,
            Some(1) => {
                let v1: PolicyV1 = serde_json::from_slice(raw).map_err(|_| "authority-policy-format")?;
                if v1.grants.is_empty() || v1.grants.iter().any(|g| g.actions.is_empty()) {
                    return Err("authority-policy-invalid");
                }
                let mut p = Self {
                    schema_version: 2,
                    roles: BUILTIN_ROLES.iter().map(|k| Role { key: (*k).into(), label: builtin_label(k).into(), description: String::new() }).collect(),
                    grants: v1
                        .grants
                        .into_iter()
                        .map(|g| Grant {
                            permissions: CATALOG.iter().filter(|p| g.actions.contains(&p.family)).map(|p| p.key.to_owned()).collect(),
                            role: g.role,
                            when: g.when,
                        })
                        .collect(),
                };
                p.grants.retain(|g| !(g.role == "admin" && g.when.is_empty()));
                p.grants.push(Grant { role: "admin".into(), permissions: CATALOG.iter().map(|p| p.key.to_owned()).collect(), when: BTreeMap::new() });
                p
            }
            _ => return Err("authority-policy-format"),
        };
        p.validate()?;
        Ok(p)
    }

    fn validate(&self) -> Result<(), &'static str> {
        let invalid = Err("authority-policy-invalid");
        if self.schema_version != 2 {
            return invalid;
        }
        let mut keys = std::collections::BTreeSet::new();
        for r in &self.roles {
            if !swarm_identity_contract::name(r.key.as_bytes()) || !keys.insert(r.key.as_str()) || r.label.trim().is_empty() || r.label.chars().any(char::is_control) || r.description.chars().any(|c| c.is_control() && c != '\n') {
                return invalid;
            }
        }
        if BUILTIN_ROLES.iter().any(|b| !keys.contains(b)) {
            return invalid;
        }
        for g in &self.grants {
            let mut seen = std::collections::BTreeSet::new();
            if !keys.contains(g.role.as_str())
                || g.permissions.iter().any(|p| permission(p).is_none() || !seen.insert(p.as_str()))
                || g.when.iter().any(|(k, v)| !swarm_identity_contract::name(k.as_bytes()) || v.chars().any(char::is_control))
            {
                return invalid;
            }
        }
        // `admin` holds the whole catalog without condition (no lockout).
        let admin_full = self.grants.iter().any(|g| g.role == "admin" && g.when.is_empty() && CATALOG.iter().all(|p| g.permissions.iter().any(|x| x == p.key)));
        if !admin_full {
            return invalid;
        }
        Ok(())
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

    /// Whether `role` is defined by this policy.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r.key == role)
    }
}

/// Decide whether `subject` may send `method path`. Deny by default: allowed
/// only if a grant matches (role held, permission listed, all `when`
/// attributes equal) **and** the subject meets the Biba floor of the
/// permission's family. The floor is checked first so its refusal is
/// reported even under a permissive grant.
pub fn decide(policy: &Policy, subject: &Subject<'_>, method: &str, path: &str) -> Decision {
    let key = classify_permission(method, path);
    let action = permission(key).map(|p| p.family).unwrap_or(Action::Administer);
    if subject.integrity < integrity_floor(action) {
        return Decision { allowed: false, action, permission: key, reason: "policy-integrity-floor" };
    }
    let granted = policy.grants.iter().any(|g| {
        subject.roles.contains(&g.role)
            && g.permissions.iter().any(|p| p == key)
            && g.when.iter().all(|(k, v)| subject.attributes.get(k) == Some(v))
    });
    Decision { allowed: granted, action, permission: key, reason: if granted { "policy-granted" } else { "policy-no-grant" } }
}

/// Whether `subject` holds catalog permission `key` (grant matched and Biba
/// floor met): what the Admin shows the caller. Same rules as [`decide`].
pub fn holds(policy: &Policy, subject: &Subject<'_>, key: &str) -> bool {
    let Some(p) = permission(key) else { return false };
    subject.integrity >= integrity_floor(p.family)
        && policy.grants.iter().any(|g| {
            subject.roles.contains(&g.role) && g.permissions.iter().any(|x| x == key) && g.when.iter().all(|(k, v)| subject.attributes.get(k) == Some(v))
        })
}
