//! Persisted/frozen detection policy; metadata grants no runtime authority.
use crate::{EngineSettingsError, SettingCap, SettingCount};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
/// Native-only additive metadata, omitted on historical Go Agent bodies.
/// All core fields are required, including explicit null for unlimited history.
/// Unknown extension values survive lifecycle serialization; consumers refuse
/// them rather than silently ignore a possible future execution constraint.
#[derive(Clone, PartialEq, Serialize)]
pub struct AgentGuardSnapshot {
    pub schema_version: i64,
    pub settings_revision: String,
    pub lookback_multiplier: SettingCap,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, crate::GoRaw>,
}
impl std::fmt::Debug for AgentGuardSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentGuardSnapshot { data: redacted }")
    }
}
/// Validated v17 policy; constructed only from explicit saved/frozen values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentGuardPolicy {
    multiplier: Option<i64>,
}
impl AgentGuardPolicy {
    /// Validate a saved nullable positive multiplier, without substituting defaults.
    /// Pure data validation: no reads/writes, identity, execution or budget grant.
    pub(crate) fn from_saved(value: Option<i64>) -> Result<Self, EngineSettingsError> {
        if value.is_some_and(|v| v < 1) {
            return Err(EngineSettingsError::Refused(
                "settings-tool-guard-window-positive-or-unlimited",
            ));
        }
        Ok(Self { multiplier: value })
    }
    /// Freeze this policy with its actual validated settings revision. Reject
    /// nonpositive revisions; no historical snapshot is upgraded or mutated.
    pub(crate) fn freeze(
        &self,
        revision: SettingCount,
    ) -> Result<AgentGuardSnapshot, EngineSettingsError> {
        if revision.0 < 1 {
            return Err(EngineSettingsError::Refused(
                "agent-guard-snapshot-revision-invalid",
            ));
        }
        Ok(AgentGuardSnapshot {
            schema_version: 1,
            settings_revision: revision.0.to_string(),
            lookback_multiplier: SettingCap(self.multiplier.map(SettingCount)),
            extensions: BTreeMap::new(),
        })
    }
    /// Positive arithmetic factor or explicit no-expiry. No ceiling is invented.
    pub(crate) fn multiplier(&self) -> Option<i64> {
        self.multiplier
    }
}
impl AgentGuardSnapshot {
    /// Validate the native snapshot, preserving absence/unknown future metadata
    /// as a refusal rather than falling back to today's Admin values. This reads
    /// no current settings and confers neither authenticity nor execution rights.
    pub fn policy(&self) -> Result<AgentGuardPolicy, EngineSettingsError> {
        let revision = self.settings_revision.parse::<i64>().ok();
        if self.schema_version != 1
            || !self.extensions.is_empty()
            || !revision.is_some_and(|v| v > 0 && v.to_string() == self.settings_revision)
        {
            return Err(EngineSettingsError::Refused("agent-guard-snapshot-invalid"));
        }
        AgentGuardPolicy::from_saved(self.lookback_multiplier.0.map(|v| v.0))
    }
    /// Stable prompt fragment binds the frozen policy/revision into context SHA.
    /// Invalid metadata refuses with its typed cause before any Agent insertion.
    pub(crate) fn context(&self) -> Result<String, EngineSettingsError> {
        let policy = self.policy()?;
        let window = policy.multiplier().map_or_else(
            || "sans expiration".into(),
            |v| format!("{v} × le plafond de répétitions, en résultats terminés"),
        );
        Ok(format!("Politique de détection figée : fenêtre d'échecs entrelacés {window} ; configuration {}. Ces observations ne prouvent ni l'exécution d'un outil ni l'acceptation du travail.\n",self.settings_revision))
    }
}

/// Parse native required fields from opaque JSON, without Go zero/default/merge
/// projection. Unknown values remain flat RawValue spans, not recursive Value
/// trees. Duplicate keys refuse; missing nullable fields are not explicit null.
/// JSON/field errors retain their serde diagnostic through the calling decoder;
/// this makes no IO, current-policy lookup or authority inference.
impl<'de> Deserialize<'de> for AgentGuardSnapshot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        #[derive(Deserialize)]
        struct Required {
            schema_version: i64,
            settings_revision: String,
            lookback_multiplier: SettingCap,
        }
        let raw = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
        let required: Required = serde_json::from_str(raw.get()).map_err(D::Error::custom)?;
        let boundaries =
            crate::model_wire::Boundaries::new(raw.get().as_bytes()).map_err(D::Error::custom)?;
        let entries = crate::model_wire::object(raw.get().as_bytes(), &boundaries)
            .map_err(D::Error::custom)?;
        let mut seen = BTreeSet::new();
        let mut extensions = BTreeMap::new();
        for (key, value) in entries {
            if !seen.insert(key.clone()) {
                return Err(D::Error::custom("duplicate native guard snapshot field"));
            }
            if !matches!(
                key.as_str(),
                "schema_version" | "settings_revision" | "lookback_multiplier"
            ) {
                let value = swarm_wire_json::decode::<Box<serde_json::value::RawValue>>(value)
                    .map_err(D::Error::custom)?;
                extensions.insert(key, crate::GoRaw(value));
            }
        }
        if ["schema_version", "settings_revision", "lookback_multiplier"]
            .iter()
            .any(|key| !seen.contains(*key))
        {
            return Err(D::Error::custom(
                "missing required native guard snapshot field",
            ));
        }
        Ok(Self {
            schema_version: required.schema_version,
            settings_revision: required.settings_revision,
            lookback_multiplier: required.lookback_multiplier,
            extensions,
        })
    }
}
