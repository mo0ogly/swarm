//! Presentation metadata; effective policy values come from the settings document.
use crate::{EngineSettingsError, EngineSettingsValues};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingUnit {
    Count,
    /// Validated operator text, with its domain checked by the consumer policy.
    Text,
    /// Canonical loopback socket address, validated by the saved network policy.
    Address,
    Characters,
    Seconds,
    Milliseconds,
    Bytes,
    Percent,
    Programs,
    Phases,
    /// Explicit operator string lists; consumer validates vocabulary and rules.
    Strings,
}
/// A named wire choice with a translatable label; selecting it remains an
/// explicit saved operator decision, not an authority granted by this metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSettingChoice {
    pub value: String,
    pub label: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSettingField {
    pub name: String,
    pub label: String,
    pub unit: SettingUnit,
    pub nullable: bool,
    pub help: String,
    pub introduced_schema_version: i64,
    /// Non-policy representation maximum, populated by the authoritative engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technical_maximum: Option<crate::SettingCount>,
    /// Explicit null meaning when it differs from an unlimited operator cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub null_label: Option<String>,
    /// Effect boundary; metadata is not an observation that a server restarted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<EngineSettingApplication>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<EngineSettingChoice>,
}
/// Documented effect boundary for fields whose live consumer needs a restart.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineSettingApplication {
    ServerRestart,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    schema_version: i64,
    fields: Vec<EngineSettingField>,
}
/// Readonly compatibility metadata for saved model-policy v1 identifiers.
/// This describes the native route boundary; it grants no provider capability
/// and does not constrain unauthenticated model facts reported by providers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelPolicyFormat {
    pub version: i64,
    pub model_name_max_bytes: crate::SettingCount,
    pub grammar: &'static str,
}
/// Validated settings presentation and immutable transport/format metadata.
/// Operator choices are carried only by defaults/persisted settings, not by
/// readonly compatibility descriptions. Construct via `installation`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EngineSettingsCatalog {
    pub schema_version: i64,
    pub document_schema_version: i64,
    pub supported_document_versions: Vec<i64>,
    pub defaults: EngineSettingsValues,
    pub fields: Vec<EngineSettingField>,
    pub input_transport: crate::CommandInputTransport,
    pub network_transport: crate::WebTransportConstraints,
    pub model_policy_format: ModelPolicyFormat,
    pub historical_quality_profile: HistoricalQualityProfile,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HistoricalQualityProfile {
    pub method_version: String,
    pub values: crate::QualityPolicyValues,
}
impl EngineSettingsCatalog {
    /// Validate installation presentation metadata against typed defaults and
    /// the supported phase vocabulary. Returns a descriptor or safe format/
    /// consistency error; nullable fields may explicitly start unlimited.
    /// Metadata is not the installed operator configuration.
    /// v4 adds address fields, explicit null labels, restart boundaries and
    /// technical maxima populated from engine constraints, not operator metadata.
    /// Readable historical document versions and field introduction bounds use
    /// the same current schema derived from validated defaults. Rendering this
    /// metadata never adopts missing values into an older saved installation.
    /// This does no IO, publication, task mutation, server restart or authorization.
    pub fn installation() -> Result<Self, EngineSettingsError> {
        let descriptor: Descriptor =
            serde_json::from_slice(include_bytes!("../config/engine-settings-catalog.json"))
                .map_err(EngineSettingsError::Format)?;
        let defaults = EngineSettingsValues::installation_defaults()?;
        let keys = serde_json::to_value(&defaults).map_err(EngineSettingsError::Format)?;
        let object = keys
            .as_object()
            .ok_or(EngineSettingsError::Refused("settings-catalog-values"))?;
        let names: BTreeSet<_> = descriptor.fields.iter().map(|f| f.name.as_str()).collect();
        if descriptor.schema_version != 5
            || names.len() != descriptor.fields.len()
            || names != object.keys().map(String::as_str).collect()
            || descriptor.fields.iter().any(|f| {
                f.label.trim().is_empty()
                    || f.help.trim().is_empty()
                    || !(1..=defaults.schema_version()).contains(&f.introduced_schema_version)
                    || f.null_label
                        .as_ref()
                        .is_some_and(|label| !f.nullable || label.trim().is_empty())
                    || f.technical_maximum.is_some()
                    || f.application.is_some()
                    || (f.unit == SettingUnit::Address
                        && (f.name != "web_bind_address" || f.nullable))
                    || if f.unit == SettingUnit::Phases {
                        f.name != "task_manual_required_gate_phases"
                            || f.choices
                                .iter()
                                .map(|c| c.value.as_str())
                                .collect::<BTreeSet<_>>()
                                != crate::MANUAL_GATE_PHASES.into_iter().collect()
                            || f.choices.len() != crate::MANUAL_GATE_PHASES.len()
                            || f.choices.iter().any(|c| c.label.trim().is_empty())
                    } else {
                        !f.choices.is_empty()
                    }
                    || match object.get(&f.name) {
                        Some(v)
                            if matches!(
                                f.unit,
                                SettingUnit::Programs | SettingUnit::Phases | SettingUnit::Strings
                            ) =>
                        {
                            !v.is_array() || f.nullable
                        }
                        Some(v) if v.is_null() => !f.nullable,
                        Some(v) => !v.is_string(),
                        None => true,
                    }
            })
        {
            return Err(EngineSettingsError::Refused(
                "settings-catalog-inconsistent",
            ));
        }
        let network_transport = crate::WebTransportConstraints::current();
        let mut fields = descriptor.fields;
        for field in &mut fields {
            field.technical_maximum = network_transport.maximum_for(&field.name);
            if crate::WEB_POLICY_FIELDS.contains(&field.name.as_str()) {
                field.application = Some(EngineSettingApplication::ServerRestart);
            }
        }
        Ok(Self {
            schema_version: 5,
            document_schema_version: defaults.schema_version(),
            // Document history is contiguous from protocol v1; derive its range
            // from actual validated installation values, not a second ceiling.
            supported_document_versions: (1..=defaults.schema_version()).collect(),
            defaults,
            fields,
            network_transport,
            input_transport: crate::CommandInputTransport::current(),
            model_policy_format: ModelPolicyFormat {
                version: 1,
                model_name_max_bytes: crate::SettingCount(
                    crate::model_routing_policy::MODEL_POLICY_V1_NAME_MAX_BYTES as i64,
                ),
                grammar: "ASCII alphanumeric first; then ASCII alphanumeric or ._:/-",
            },
            historical_quality_profile: HistoricalQualityProfile {
                method_version: "2".into(),
                values: crate::QualityPolicyValues::legacy_method2()?,
            },
        })
    }
}
