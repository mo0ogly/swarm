//! Versioned quality policy data. This is not gate acceptance or authority.
use crate::{EngineSettingsDocument, EngineSettingsError, EngineSettingsValues, SettingCount};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityPolicyValues {
    pub quality_delivery_min_percent: SettingCount,
    pub quality_domain_min_percent: SettingCount,
    pub quality_band_ok_min_percent: SettingCount,
    pub quality_band_warn_min_percent: SettingCount,
    pub quality_band_blocked_min_percent: SettingCount,
}
impl QualityPolicyValues {
    /// Historical method definition, never an effective-setting fallback.
    pub fn legacy_method2() -> Result<Self, EngineSettingsError> {
        let value: Self =
            serde_json::from_slice(include_bytes!("../config/quality-policy-legacy2.json"))
                .map_err(EngineSettingsError::Format)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), EngineSettingsError> {
        if [
            self.quality_delivery_min_percent,
            self.quality_domain_min_percent,
            self.quality_band_ok_min_percent,
            self.quality_band_warn_min_percent,
            self.quality_band_blocked_min_percent,
        ]
        .iter()
        .any(|n| !(0..=100).contains(&n.0))
        {
            return Err(EngineSettingsError::Refused(
                "settings-quality-percent-representation",
            ));
        }
        if self.quality_band_ok_min_percent.0 < self.quality_band_warn_min_percent.0
            || self.quality_band_warn_min_percent.0 < self.quality_band_blocked_min_percent.0
        {
            return Err(EngineSettingsError::Refused(
                "settings-quality-bands-inconsistent",
            ));
        }
        Ok(())
    }
}
impl EngineSettingsValues {
    pub fn quality_policy_values(&self) -> Result<QualityPolicyValues, EngineSettingsError> {
        let required = |v: Option<SettingCount>| {
            v.ok_or(EngineSettingsError::Refused(
                "settings-quality-policy-not-configured",
            ))
        };
        let values = QualityPolicyValues {
            quality_delivery_min_percent: required(self.quality_delivery_min_percent)?,
            quality_domain_min_percent: required(self.quality_domain_min_percent)?,
            quality_band_ok_min_percent: required(self.quality_band_ok_min_percent)?,
            quality_band_warn_min_percent: required(self.quality_band_warn_min_percent)?,
            quality_band_blocked_min_percent: required(self.quality_band_blocked_min_percent)?,
        };
        values.validate()?;
        Ok(values)
    }
    pub(crate) fn has_quality_policy(&self) -> bool {
        [
            self.quality_delivery_min_percent,
            self.quality_domain_min_percent,
            self.quality_band_ok_min_percent,
            self.quality_band_warn_min_percent,
            self.quality_band_blocked_min_percent,
        ]
        .iter()
        .any(Option::is_some)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QualityPolicySnapshot {
    pub schema_version: i64,
    pub generation: SettingCount,
    pub values: QualityPolicyValues,
    pub digest: String,
    pub settings_revision: SettingCount,
    pub legacy_method2_thresholds_compatible: bool,
}
impl QualityPolicySnapshot {
    /// Consistency only; callers still need an authoritative current journal.
    pub fn validate_binding(&self) -> Result<(), EngineSettingsError> {
        self.values.validate()?;
        if self.schema_version != 1 || self.generation.0 < 0 || self.settings_revision.0 < 1 {
            return Err(EngineSettingsError::Refused(
                "settings-quality-binding-invalid",
            ));
        }
        let raw =
            swarm_wire_json::encode(&("swarm-quality-policy/1", self.generation, &self.values))
                .map_err(EngineSettingsError::Format)?;
        let digest = format!("{:x}", Sha256::digest(raw));
        let compatible =
            self.generation.0 == 0 && self.values == QualityPolicyValues::legacy_method2()?;
        if self.digest != digest || self.legacy_method2_thresholds_compatible != compatible {
            return Err(EngineSettingsError::Refused(
                "settings-quality-binding-invalid",
            ));
        }
        Ok(())
    }
}
impl EngineSettingsDocument {
    pub fn quality_policy_snapshot(&self) -> Result<QualityPolicySnapshot, EngineSettingsError> {
        self.validate()?;
        let values = self.values.quality_policy_values()?;
        let legacy = QualityPolicyValues::legacy_method2()?;
        let mut previous = None;
        let mut generation = 0i64;
        for entry in &self.history {
            if !entry.change.values.has_quality_policy() {
                continue;
            }
            let current = entry.change.values.quality_policy_values()?;
            match &previous {
                None => generation = i64::from(current != legacy),
                Some(old) if old != &current => {
                    generation = generation
                        .checked_add(1)
                        .ok_or(EngineSettingsError::Refused(
                            "settings-quality-generation-representation",
                        ))?
                }
                Some(_) => {}
            }
            previous = Some(current);
        }
        let raw =
            swarm_wire_json::encode(&("swarm-quality-policy/1", SettingCount(generation), &values))
                .map_err(EngineSettingsError::Format)?;
        Ok(QualityPolicySnapshot {
            schema_version: 1,
            generation: SettingCount(generation),
            legacy_method2_thresholds_compatible: generation == 0 && values == legacy,
            values,
            digest: format!("{:x}", Sha256::digest(raw)),
            settings_revision: self.revision,
        })
    }
}
