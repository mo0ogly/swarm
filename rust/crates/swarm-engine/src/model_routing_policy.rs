//! Explicit saved model-routing restrictions. No model names, defaults, account
//! capabilities, prices or execution permission are inferred by this module.
use crate::EngineSettingsError;
use std::collections::BTreeSet;

/// Maximum bytes in the existing Go model-policy v1 identifier grammar.
/// This is wire-format compatibility, not a configurable execution/cost ceiling.
/// Reported provider model facts and the pure argv compiler have separate contracts.
pub(crate) const MODEL_POLICY_V1_NAME_MAX_BYTES: usize = 160;

/// Check the v1 ASCII identifier format without IO, fallback or authorization.
/// The first byte must be alphanumeric; remaining bytes also allow `._:/-`.
/// Empty, non-ASCII, control-bearing or oversized names return false. Extending
/// this public format needs an explicit versioned compatibility decision.
pub(crate) fn model_policy_v1_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() <= MODEL_POLICY_V1_NAME_MAX_BYTES
        && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(b))
}

#[cfg(test)]
mod format_tests {
    /// v1 boundary controls precede each malformed identifier; no provider IO.
    #[test]
    fn model_policy_v1_identifier_format_boundaries_and_recovery() {
        let longest = "a".repeat(super::MODEL_POLICY_V1_NAME_MAX_BYTES);
        for invalid in [
            "".into(),
            "a".repeat(161),
            "-name".into(),
            "_name".into(),
            ".name".into(),
            "name space".into(),
            "nameé".into(),
            "name\n".into(),
        ] {
            for valid in ["A", "aB09._:/-", longest.as_str()] {
                assert!(super::model_policy_v1_name(valid));
            }
            assert!(!super::model_policy_v1_name(&invalid));
            assert!(super::model_policy_v1_name(&longest));
        }
    }
}

/// Valid level names in the existing model-policy v1 wire grammar. This list is
/// schema vocabulary; eligibility/defaults are independently saved operator data.
pub(crate) fn model_level(level: &str) -> bool {
    matches!(level, "simple" | "standard" | "exigeant")
}

/// Parsed matching syntax, not a built-in classification of particular models.
#[derive(Clone, Debug)]
enum ModelRule {
    Exact(String),
    Prefix(String),
    Contains(String),
}
impl ModelRule {
    /// Parse an explicit nonempty literal predicate, preserving case/whitespace.
    /// Controls and unknown operators refuse; no regex, IO or fallback occurs.
    fn parse(raw: &str) -> Result<Self, EngineSettingsError> {
        let fail = || EngineSettingsError::Refused("settings-model-rule-invalid");
        let (kind, value) = raw.split_once(':').ok_or_else(fail)?;
        if value.is_empty() || crate::agent_terminal_text::terminal_text(value) != value {
            return Err(fail());
        }
        match kind {
            "exact" => Ok(Self::Exact(value.into())),
            "prefix" => Ok(Self::Prefix(value.into())),
            "contains" => Ok(Self::Contains(value.into())),
            _ => Err(fail()),
        }
    }
    /// Apply the saved literal predicate; matching implies no capability proof.
    fn matches(&self, model: &str) -> bool {
        match self {
            Self::Exact(v) => model == v,
            Self::Prefix(v) => model.starts_with(v),
            Self::Contains(v) => model.contains(v),
        }
    }
}

/// Validated saved restrictions for a future candidate selection. Private fields
/// prevent accidental construction without validation. This is operator policy,
/// not authenticated model identity, catalogue availability or a launch receipt.
#[derive(Clone, Debug)]
pub struct AgentModelRoutingPolicy {
    automatic_levels: BTreeSet<String>,
    restricted_levels: BTreeSet<String>,
    rules: Vec<ModelRule>,
}
impl AgentModelRoutingPolicy {
    /// Parse complete explicit sets. Empty automatic/restricted sets deny those
    /// choices; no rules means no model is flagged. Duplicates and invalid levels
    /// refuse. There are no installation defaults or mutations in this constructor.
    pub(crate) fn from_saved(
        automatic: &[String],
        restricted: &[String],
        rules: &[String],
    ) -> Result<Self, EngineSettingsError> {
        let levels = |values: &[String]| -> Result<BTreeSet<String>, EngineSettingsError> {
            let set: BTreeSet<_> = values.iter().cloned().collect();
            if set.len() != values.len() || values.iter().any(|v| !model_level(v)) {
                return Err(EngineSettingsError::Refused(
                    "settings-model-levels-invalid",
                ));
            }
            Ok(set)
        };
        if rules.iter().collect::<BTreeSet<_>>().len() != rules.len() {
            return Err(EngineSettingsError::Refused(
                "settings-model-rules-duplicate",
            ));
        }
        Ok(Self {
            automatic_levels: levels(automatic)?,
            restricted_levels: levels(restricted)?,
            rules: rules
                .iter()
                .map(|r| ModelRule::parse(r))
                .collect::<Result<_, _>>()?,
        })
    }

    /// Check one candidate against these saved restrictions only. Unknown levels,
    /// empty/control-bearing identifiers, a denied automatic level or a flagged
    /// model at a denied level refuse with static safe codes. This pure check
    /// neither resolves a default nor validates availability, cost, caller rights,
    /// freshness or actual execution; those remain required at launch boundaries.
    pub fn check_selection(
        &self,
        level: &str,
        model: &str,
        automatic: bool,
    ) -> Result<(), EngineSettingsError> {
        if !model_level(level)
            || model.is_empty()
            || crate::agent_terminal_text::terminal_text(model) != model
        {
            return Err(EngineSettingsError::Refused(
                "settings-model-selection-invalid",
            ));
        }
        if automatic && !self.automatic_levels.contains(level) {
            return Err(EngineSettingsError::Refused(
                "settings-model-automatic-level-denied",
            ));
        }
        if self.rules.iter().any(|rule| rule.matches(model))
            && !self.restricted_levels.contains(level)
        {
            return Err(EngineSettingsError::Refused(
                "settings-model-restricted-level-denied",
            ));
        }
        Ok(())
    }
}

/// Strict read-only request. The automatic flag must be supplied explicitly;
/// absence is never interpreted as explicit operator choice or a default grant.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPolicyCheckRequest {
    pub schema_version: i64,
    pub level: String,
    pub model: String,
    pub automatic: bool,
}

/// Successful saved-rule check bound to a settings revision. It intentionally
/// includes no provider receipt, process identity, price, launch or acceptance.
#[derive(serde::Serialize)]
pub struct ModelPolicyCheck {
    schema_version: i64,
    status: &'static str,
    scope: &'static str,
    settings_revision: crate::SettingCount,
}
#[cfg(unix)]
impl crate::RootStore {
    /// Read current checked settings and assess one candidate using only their
    /// routing restrictions. Invalid schema/selection, absent legacy policy and
    /// storage/history errors retain safe typed causes. No file, history, budget
    /// or process is changed. A success is neither catalogue availability nor
    /// identity, fresh route authorization or provider execution permission.
    pub fn check_model_policy(
        &self,
        request: &ModelPolicyCheckRequest,
    ) -> Result<ModelPolicyCheck, crate::RootStoreError> {
        if request.schema_version != 1 {
            return Err(crate::RootStoreError::Settings(
                EngineSettingsError::Refused("settings-model-check-schema-invalid"),
            ));
        }
        let settings = self.settings()?;
        settings
            .values
            .agent_model_routing_policy()
            .and_then(|p| p.check_selection(&request.level, &request.model, request.automatic))
            .map_err(crate::RootStoreError::Settings)?;
        Ok(ModelPolicyCheck {
            schema_version: 1,
            status: "allowed_by_saved_rules",
            scope: "candidate_rules_only",
            settings_revision: settings.revision,
        })
    }
}
