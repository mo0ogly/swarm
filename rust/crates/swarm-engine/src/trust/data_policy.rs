//! Data provenance policy (T4 « provenance »): typed classification context and
//! the request/action admission chain. Port of Go `trust_data_policy.go`.
//! The policy gates which (classification, destination, provider, tool) path a
//! business command may carry; a denied context never persists any data, and
//! every refusal keeps the historical typed code « authorization_required ».
use super::{
    command::Command,
    state::State,
    TrustError as E,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

/// One resource classification rule. Field order matches the Go producer.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DataResource {
    pub resource: String,
    pub classification: String,
}

/// Operator data policy. Field order matches Go `TrustDataPolicy` because the
/// configuration digest and public projection preserve the declared property
/// order on the wire.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DataPolicy {
    pub resources: Vec<DataResource>,
    pub enabled: bool,
    pub classifications: Vec<String>,
    pub destinations: Vec<String>,
    pub providers: Vec<String>,
    pub tools: Vec<String>,
}

/// Typed business data context. Field order matches Go `TrustDataContext`;
/// the action hash preimage uses this exact order, so a non-canonical input
/// key order digests identically to the historical Go engine.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DataContext {
    pub classification: String,
    pub destination: String,
    pub provider: String,
    pub tool: String,
}

const CLASSIFICATIONS: [&str; 4] = ["public", "internal", "confidential", "restricted"];
const DESTINATIONS: [&str; 4] = [
    "local-ledger",
    "local-process-state",
    "local-access-state",
    "local-attestation",
];
const PROVIDERS: [&str; 1] = ["swarm"];
const TOOLS: [&str; 4] = [
    "simulated-settlement",
    "process-workflow",
    "simulated-access-grant",
    "sealed-attestation",
];

/// Go `trustGroupLabelValid`: trimmed-identical, nonempty, control-free label.
fn label_valid(v: &str) -> bool {
    v.trim() == v && !v.is_empty() && !v.chars().any(char::is_control)
}

fn known_unique(values: &[String], known: &[&str]) -> Result<(), E> {
    let mut seen = BTreeSet::new();
    for v in values {
        if !known.contains(&v.as_str()) || !seen.insert(v.as_str()) {
            return Err(E::Invalid("données : valeurs connues et uniques requises"));
        }
    }
    Ok(())
}

impl DataPolicy {
    /// Full structural validation, Go `validateTrustDataPolicy` parity. A
    /// stored/default policy that fails this is a malformed policy, not an
    /// authorization refusal: the historical Go validator returns plain errors
    /// (no « authorization_required »), so this stays in the format family.
    pub(crate) fn validate(&self) -> Result<(), E> {
        known_unique(&self.classifications, &CLASSIFICATIONS)?;
        known_unique(&self.destinations, &DESTINATIONS)?;
        known_unique(&self.providers, &PROVIDERS)?;
        known_unique(&self.tools, &TOOLS)?;
        let mut seen = BTreeSet::new();
        for rule in &self.resources {
            if !label_valid(&rule.resource)
                || !seen.insert(rule.resource.as_str())
                || !CLASSIFICATIONS.contains(&rule.classification.as_str())
            {
                return Err(E::Invalid(
                    "données : règles de ressources explicites et uniques requises",
                ));
            }
        }
        Ok(())
    }
}

/// Go `trustConnectorID`: nil selection means the ledger simulation. Admission
/// derives the id leniently, before connector validation, exactly as Go does.
fn connector_id(cmd: &Command) -> &str {
    match &cmd.connector {
        None => "local-ledger-simulation",
        Some(c) => c.id.as_str(),
    }
}

fn deny() -> E {
    E::Data("données : contexte ou chemin non autorisé ; aucune donnée conservée")
}

fn allowed_path(
    p: &DataPolicy,
    c: Option<&DataContext>,
    resource: &str,
    destination: &str,
    tool: &str,
) -> Result<(), E> {
    if !p.enabled {
        return if c.is_none() { Ok(()) } else { Err(deny()) };
    }
    let Some(c) = c else { return Err(deny()) };
    if !p.classifications.iter().any(|v| v == &c.classification)
        || !p.destinations.iter().any(|v| v == &c.destination)
        || !p.providers.iter().any(|v| v == &c.provider)
        || !p.tools.iter().any(|v| v == &c.tool)
    {
        return Err(deny());
    }
    let classified = p
        .resources
        .iter()
        .any(|r| r.resource == resource && r.classification == c.classification);
    if !classified {
        return Err(deny());
    }
    if (c.classification != "public" && c.classification != "internal")
        || c.destination != destination
        || c.provider != "swarm"
        || c.tool != tool
    {
        return Err(deny());
    }
    Ok(())
}

/// Effective stored policy: the retained block, or the empty default (nil maps
/// to defaults because Rust state has no Option layer around extensions).
fn effective(st: &State) -> Result<DataPolicy, E> {
    let raw = st
        .extensions
        .get("data_policy")
        .map(|v| v.raw().to_string())
        .unwrap_or_else(|| {
            r#"{"resources":[],"enabled":false,"classifications":[],"destinations":[],"providers":[],"tools":[]}"#
                .to_string()
        });
    let p: DataPolicy = serde_json::from_str(&raw)?;
    p.validate()?;
    Ok(p)
}

/// Request-side admission: the data path is decided before any business edit.
/// Only a request may carry a context; approve/commit/other operations refuse
/// any supplied context. With the policy enabled the context must follow the
/// classified path for the connector's destination/tool. Non-request operations
/// keep Go's shape: the context check happens on the stored action instead.
pub(crate) fn admission(st: &State, operation: &str, cmd: &Command) -> Result<(), E> {
    if operation != "request" {
        if cmd.data_context.is_some() {
            return Err(deny());
        }
        return Ok(());
    }
    let p = effective(st)?;
    if !p.enabled {
        return if cmd.data_context.is_none() {
            Ok(())
        } else {
            Err(deny())
        };
    }
    let (destination, tool) = if connector_id(cmd) == "local-access-simulation" {
        ("local-access-state", "simulated-access-grant")
    } else {
        ("local-ledger", "simulated-settlement")
    };
    allowed_path(&p, cmd.data_context.as_ref(), &cmd.resource, destination, tool)
}

/// Approve/commit re-admission of the stored action's recorded context. Go
/// deliberately skips reconcile here: a committed effect must stay recoverable
/// after an operator narrows the policy. Denial keeps state unchanged.
pub(crate) fn action_admission(st: &State, operation: &str, target: &str) -> Result<(), E> {
    if operation != "approve" && operation != "commit" {
        return Ok(());
    }
    let Some(a) = st.actions.get(target) else {
        return Ok(());
    };
    if operation == "commit"
        && matches!(
            a.get("state").and_then(Value::as_str),
            Some("committed" | "reconciled" | "unknown")
        )
    {
        return Ok(());
    }
    let p = effective(st)?;
    let resource = a.get("resource").and_then(Value::as_str).unwrap_or("");
    let c: Option<DataContext> = match a.get("data_context") {
        None | Some(Value::Null) => None,
        Some(v) => Some(serde_json::from_value(v.clone())?),
    };
    let connector = a
        .get("connector")
        .filter(|v| !v.is_null())
        .and_then(|v| v.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("local-ledger-simulation");
    let (destination, tool) = if connector == "local-access-simulation" {
        ("local-access-state", "simulated-access-grant")
    } else {
        ("local-ledger", "simulated-settlement")
    };
    allowed_path(&p, c.as_ref(), resource, destination, tool)
}

/// Typed context reconstruction from a stored action, for the effect-boundary
/// hash re-derivation. Absent or null stored context decodes as None.
pub(crate) fn stored_context(a: &Value) -> Result<Option<DataContext>, E> {
    match a.get("data_context") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => Ok(Some(serde_json::from_value(v.clone())?)),
    }
}
