//! Saved audit-text policy for mission quota changes; no author identity grant.
use crate::{AgentLaunchError, EngineSettingsError, SettingCap, SettingCount};
/// UTF8 byte limits: minimum uses trimmed text, maximum original raw text.
/// Zero minimum removes that extra minimum; null maximum removes its cap. A
/// nonblank reason remains the protocol audit invariant, even with minimum0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct QuotaReasonPolicy {
    pub minimum_trimmed_bytes: SettingCount,
    pub maximum_raw_bytes: SettingCap,
}
impl QuotaReasonPolicy {
    /// Validate nonnegative, coherent and platform-representable supplied policy.
    /// No default, truncation, actor inference, IO or historical adoption occurs.
    pub fn validate(&self) -> Result<(), EngineSettingsError> {
        let min = self.minimum_trimmed_bytes.0;
        if min < 0 || self.maximum_raw_bytes.0.is_some_and(|n| n.0 < min) {
            return Err(EngineSettingsError::Refused(
                "settings-quota-reason-bounds-inconsistent",
            ));
        }
        usize::try_from(min).map_err(EngineSettingsError::Count)?;
        if let Some(max) = self.maximum_raw_bytes.0 {
            usize::try_from(max.0).map_err(EngineSettingsError::Count)?;
        }
        Ok(())
    }
    /// Check one untrusted audit reason under explicit validated values. Failure
    /// preserves configuration/representation causes behind safe codes, never
    /// the private reason. No data mutation, write, budget or authority is added.
    pub fn check(&self, reason: &str) -> Result<(), AgentLaunchError> {
        self.validate()
            .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
        let trimmed = crate::text::trim_space(reason);
        let size = i64::try_from(trimmed.len())
            .map_err(|e| AgentLaunchError::caused("quota-reason-size-representation", e))?;
        let raw = i64::try_from(reason.len())
            .map_err(|e| AgentLaunchError::caused("quota-reason-size-representation", e))?;
        if trimmed.is_empty()
            || size < self.minimum_trimmed_bytes.0
            || self.maximum_raw_bytes.0.is_some_and(|max| raw > max.0)
        {
            return Err(AgentLaunchError::Refused("quota-reason-outside-policy"));
        }
        Ok(())
    }
}
