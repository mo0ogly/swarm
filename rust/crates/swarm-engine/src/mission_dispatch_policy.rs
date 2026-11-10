//! Explicit saved slot policy, never consent or execution authority.
use crate::{EngineSettingsError, SettingCount};
/// Configured initial slot count and optional operator ceiling. Absence of a cap
/// removes only that ceiling; budgets, provider capacity and isolation still apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissionDispatchPolicy {
    pub default_slots: SettingCount,
    pub max_slots: Option<SettingCount>,
}
impl MissionDispatchPolicy {
    /// Validate positivity and ordering without IO or mutation. Counts are the
    /// existing signed64 representation, not a second hardcoded product ceiling.
    pub fn validate(&self) -> Result<(), EngineSettingsError> {
        if self.default_slots.0 < 1
            || self
                .max_slots
                .is_some_and(|v| v.0 < 1 || v.0 < self.default_slots.0)
        {
            return Err(EngineSettingsError::Refused(
                "settings-mission-dispatch-slots-invalid",
            ));
        }
        Ok(())
    }
    /// Select a saved positive count, or the explicit installation/adopted default
    /// for absent controls. Out-of-policy saved values refuse, never clamp/write.
    /// Does not infer autonomy, pause, consent, capacity or permission to launch.
    pub fn slots(&self, saved: Option<i64>) -> Result<SettingCount, EngineSettingsError> {
        self.validate()?;
        let count = saved.map(SettingCount).unwrap_or(self.default_slots);
        if count.0 < 1 || self.max_slots.is_some_and(|v| count.0 > v.0) {
            return Err(EngineSettingsError::Refused(
                "mission-dispatch-saved-slots-outside-policy",
            ));
        }
        Ok(count)
    }
}
