//! Saved API-text preparation call policy; no implicit historical adoption.
use super::*;
/// Effective saved limits. These grant no provider or agent authority. Null caps
/// remove only their own bound; zero timeouts disable that duration, positive
/// polling intervals bound observation frequency rather than provider execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparationCallPolicy {
    pub enabled: bool,
    pub message_max_bytes: Option<usize>,
    pub context_max_bytes: Option<usize>,
    pub turn_limit: Option<usize>,
    pub response_max_bytes: Option<usize>,
    pub timeout_seconds: u64,
    pub timeout_maximum_seconds: u64,
    pub observation_ms: u64,
    pub refresh_ms: u64,
    pub http1_max_headers: usize,
    pub http2_max_header_bytes: u32,
    pub transport: String,
}
impl EngineSettingsValues {
    /// Detect any declared member; partial groups must not become defaults.
    pub(super) fn has_preparation_call_policy(&self) -> bool {
        self.preparation_call_enabled.is_some()
            || self.preparation_call_message_max_bytes.is_some()
            || self.preparation_call_context_max_bytes.is_some()
            || self.preparation_call_turn_limit.is_some()
            || self.preparation_call_response_max_bytes.is_some()
            || self.preparation_call_timeout_seconds.is_some()
            || self.preparation_call_timeout_maximum_seconds.is_some()
            || self.preparation_call_observation_ms.is_some()
            || self.preparation_call_refresh_ms.is_some()
            || self.preparation_call_http1_max_headers.is_some()
            || self.preparation_call_http2_max_header_bytes.is_some()
            || self.preparation_call_transport.is_some()
    }
    /// Validate the complete v46 group, technical integer representations and
    /// timeout ordering. No IO, adoption, provider selection or network contact.
    /// Missing/partial groups, invalid switches, durations or transport refuse.
    pub fn preparation_call_policy(&self) -> Result<PreparationCallPolicy, EngineSettingsError> {
        let missing = || refuse("settings-preparation-calls-not-configured");
        let count = |v: Option<SettingCount>| -> Result<u64, EngineSettingsError> {
            let n = v.ok_or_else(missing)?.0;
            u64::try_from(n).map_err(|_| refuse("settings-preparation-call-negative"))
        };
        let cap =
            |v: Option<SettingCap>| v.ok_or_else(missing)?.0.map(|n| size(Some(n))).transpose();
        let enabled = count(self.preparation_call_enabled)?;
        if enabled > 1 {
            return Err(refuse("settings-preparation-call-enabled-invalid"));
        }
        let timeout = count(self.preparation_call_timeout_seconds)?;
        let maximum = count(self.preparation_call_timeout_maximum_seconds)?;
        if maximum > 0 && (timeout == 0 || timeout > maximum) {
            return Err(refuse("settings-preparation-call-timeout-invalid"));
        }
        let observation = count(self.preparation_call_observation_ms)?;
        let refresh = count(self.preparation_call_refresh_ms)?;
        let headers = count(self.preparation_call_http1_max_headers)?;
        let h2 = count(self.preparation_call_http2_max_header_bytes)?;
        if observation == 0 || refresh == 0 || headers == 0 || h2 == 0 {
            return Err(refuse("settings-preparation-call-positive-required"));
        }
        let transport = self
            .preparation_call_transport
            .as_deref()
            .ok_or_else(missing)?;
        if !matches!(transport, "json" | "sse") {
            return Err(refuse("settings-preparation-call-transport-invalid"));
        }
        Ok(PreparationCallPolicy {
            enabled: enabled == 1,
            message_max_bytes: cap(self.preparation_call_message_max_bytes)?,
            context_max_bytes: cap(self.preparation_call_context_max_bytes)?,
            turn_limit: cap(self.preparation_call_turn_limit)?,
            response_max_bytes: cap(self.preparation_call_response_max_bytes)?,
            timeout_seconds: timeout,
            timeout_maximum_seconds: maximum,
            observation_ms: observation,
            refresh_ms: refresh,
            http1_max_headers: usize::try_from(headers)
                .map_err(|_| refuse("settings-preparation-call-headers-representation"))?,
            http2_max_header_bytes: u32::try_from(h2)
                .map_err(|_| refuse("settings-preparation-call-headers-representation"))?,
            transport: transport.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_complete_disabled_and_versioned() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        let p = s.preparation_call_policy().unwrap();
        assert!(s.schema_version() >= 46);
        assert!(!p.enabled);
        assert_eq!((p.timeout_seconds, p.timeout_maximum_seconds), (300, 600));
    }
    #[test]
    fn legacy_absence_is_not_adopted() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        let mut v = serde_json::to_value(s).unwrap();
        v.as_object_mut()
            .unwrap()
            .retain(|k, _| !k.starts_with("preparation_call_"));
        let s: EngineSettingsValues = serde_json::from_value(v).unwrap();
        s.validate().unwrap();
        // Later independently adopted groups retain their newer schema.
        assert_eq!(s.schema_version(), EngineSettingsValues::installation_defaults().unwrap().schema_version());
        assert!(s.preparation_call_policy().is_err());
    }
    #[test]
    fn partial_null_zero_and_switch_are_distinct() {
        let mut s = EngineSettingsValues::installation_defaults().unwrap();
        s.preparation_call_context_max_bytes = Some(SettingCap(None));
        s.preparation_call_message_max_bytes = Some(SettingCap(Some(SettingCount(0))));
        let p = s.preparation_call_policy().unwrap();
        assert_eq!(p.context_max_bytes, None);
        assert_eq!(p.message_max_bytes, Some(0));
        s.preparation_call_enabled = Some(SettingCount(2));
        assert!(s.validate().is_err());
        s.preparation_call_enabled = None;
        assert!(s.validate().is_err());
    }
    #[test]
    fn timeout_and_header_representation_refuse() {
        let mut s = EngineSettingsValues::installation_defaults().unwrap();
        s.preparation_call_timeout_seconds = Some(SettingCount(0));
        assert!(s.validate().is_err());
        s.preparation_call_timeout_maximum_seconds = Some(SettingCount(0));
        s.validate().unwrap();
        s.preparation_call_http2_max_header_bytes = Some(SettingCount(i64::from(u32::MAX) + 1));
        assert!(s.validate().is_err());
    }
}
