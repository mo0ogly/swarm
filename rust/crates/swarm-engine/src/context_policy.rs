//! Effective context caps come only from a complete persisted settings family.
use crate::{EngineSettingsError, EngineSettingsValues, SettingCap};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPolicyLimits {
    pub profile_name_bytes: Option<usize>,
    pub profile_sources: Option<usize>,
    pub profile_path_bytes: Option<usize>,
    pub profile_document_bytes: Option<usize>,
    pub context_bytes: Option<usize>,
    pub worker_context_bytes: Option<usize>,
    pub review_delivery_bytes: Option<usize>,
}
impl EngineSettingsValues {
    pub(crate) fn has_context_policy(&self) -> bool {
        [
            self.project_profile_max_name_bytes,
            self.project_profile_max_sources,
            self.project_profile_max_path_bytes,
            self.project_profile_max_document_bytes,
            self.project_context_max_bytes,
            self.project_worker_context_max_bytes,
            self.review_delivery_max_bytes,
        ]
        .iter()
        .any(Option::is_some)
    }
    pub fn context_policy_limits(&self) -> Result<ContextPolicyLimits, EngineSettingsError> {
        let cap = |value: Option<SettingCap>| -> Result<Option<usize>, EngineSettingsError> {
            let SettingCap(value) = value.ok_or(EngineSettingsError::Refused(
                "settings-context-policy-not-configured",
            ))?;
            value
                .map(|v| {
                    usize::try_from(v.0).map_err(|_| {
                        EngineSettingsError::Refused("settings-context-cap-representation")
                    })
                })
                .transpose()
        };
        Ok(ContextPolicyLimits {
            profile_name_bytes: cap(self.project_profile_max_name_bytes)?,
            profile_sources: cap(self.project_profile_max_sources)?,
            profile_path_bytes: cap(self.project_profile_max_path_bytes)?,
            profile_document_bytes: cap(self.project_profile_max_document_bytes)?,
            context_bytes: cap(self.project_context_max_bytes)?,
            worker_context_bytes: cap(self.project_worker_context_max_bytes)?,
            review_delivery_bytes: cap(self.review_delivery_max_bytes)?,
        })
    }
}
