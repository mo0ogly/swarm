//! Initial reviewer configuration defaults, not overrides of an active mission.
use crate::{EngineSettingsError, SettingCount};
/// Complete explicit persisted values used only when a reviewer is configured.
/// Zero is the existing reviewer wire contract for no operator cap, unlike
/// other SettingCap families where zero forbids. This creates no call budget,
/// process, source read, provider authority or evidence of enforcement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewerCreationDefaults {
    pub delivery_files: SettingCount,
    pub context_files: SettingCount,
    pub source_bytes: SettingCount,
    pub context_bytes: SettingCount,
}
impl ReviewerCreationDefaults {
    /// Reject negative supplied counts/bytes; all fit their existing Go i64
    /// representation. No default adoption, hidden ceiling or mutation occurs.
    pub fn validate(&self) -> Result<(), EngineSettingsError> {
        if [
            self.delivery_files,
            self.context_files,
            self.source_bytes,
            self.context_bytes,
        ]
        .iter()
        .any(|v| v.0 < 0)
        {
            return Err(EngineSettingsError::Refused(
                "settings-reviewer-creation-negative",
            ));
        }
        Ok(())
    }
}
