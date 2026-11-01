//! Saved cockpit access mode group (v52).
//!
//! Purpose: let the operator move the cockpit from the persisted owner key to
//! the authority daemon (phase B) through the Admin, with the daemon public
//! key and assertion freshness window as explicit settings.
//! Preconditions: decoded `EngineSettingsValues`.
//! Outputs: [`WebAccessPolicy`].
//! Errors: `settings-web-access-not-configured` before v52,
//! `settings-web-access-partial` when only some of the three fields exist,
//! `settings-web-access-mode-invalid`, `settings-web-access-key-invalid`
//! (authority mode needs a 32-byte base64url key; owner-key mode needs it
//! empty) and `settings-web-access-window-invalid` (zero).
//! No side effects: the server consumes it at start; nothing here opens a
//! socket or changes who is signed in.
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

/// Who may open the cockpit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebAccessMode {
    /// Persisted local key (historical behaviour).
    OwnerKey,
    /// Assertions signed by the authority daemon whose public key is given.
    Authority { public_key: [u8; 32] },
}

/// Validated v52 policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebAccessPolicy {
    pub mode: WebAccessMode,
    pub assertion_max_age_seconds: i64,
}

impl EngineSettingsValues {
    /// Whether any member of the v52 group is present.
    pub(super) fn has_web_access_policy(&self) -> bool {
        self.web_access_mode.is_some()
            || self.web_authority_public_key.is_some()
            || self.web_assertion_max_age_seconds.is_some()
    }

    /// Return the validated access policy; see module docs for refusals.
    pub fn web_access_policy(&self) -> Result<WebAccessPolicy, EngineSettingsError> {
        let (Some(mode), Some(key), Some(window)) = (
            self.web_access_mode.as_deref(),
            self.web_authority_public_key.as_deref(),
            self.web_assertion_max_age_seconds,
        ) else {
            return Err(refuse(if self.has_web_access_policy() {
                "settings-web-access-partial"
            } else {
                "settings-web-access-not-configured"
            }));
        };
        if window.0 < 1 {
            return Err(refuse("settings-web-access-window-invalid"));
        }
        let mode = match mode {
            "owner-key" if key.is_empty() => WebAccessMode::OwnerKey,
            "owner-key" => return Err(refuse("settings-web-access-key-invalid")),
            "authority" => {
                let raw = URL_SAFE_NO_PAD.decode(key).map_err(|_| refuse("settings-web-access-key-invalid"))?;
                let public_key: [u8; 32] = raw.try_into().map_err(|_| refuse("settings-web-access-key-invalid"))?;
                WebAccessMode::Authority { public_key }
            }
            _ => return Err(refuse("settings-web-access-mode-invalid")),
        };
        Ok(WebAccessPolicy { mode, assertion_max_age_seconds: window.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_default_is_owner_key_and_current_schema() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        s.validate().unwrap();
        assert_eq!(s.schema_version(), 52);
        let p = s.web_access_policy().unwrap();
        assert_eq!((p.mode, p.assertion_max_age_seconds), (WebAccessMode::OwnerKey, 30));
    }

    #[test]
    fn authority_key_partial_group_and_invalid_values() {
        let mut s = EngineSettingsValues::installation_defaults().unwrap();
        s.web_access_mode = Some("authority".into());
        s.web_authority_public_key = Some(URL_SAFE_NO_PAD.encode([5u8; 32]));
        assert_eq!(s.web_access_policy().unwrap().mode, WebAccessMode::Authority { public_key: [5; 32] });
        for (mode, key, code) in [
            ("authority", "", "settings-web-access-key-invalid"),
            ("authority", "short", "settings-web-access-key-invalid"),
            ("owner-key", "AAAA", "settings-web-access-key-invalid"),
            ("open", "", "settings-web-access-mode-invalid"),
        ] {
            s.web_access_mode = Some(mode.into());
            s.web_authority_public_key = Some(key.into());
            assert_eq!(s.web_access_policy().unwrap_err().reason(), code, "{mode} {key}");
            assert!(s.validate().is_err());
        }
        s.web_access_mode = Some("owner-key".into());
        s.web_authority_public_key = Some(String::new());
        s.web_assertion_max_age_seconds = Some(SettingCount(0));
        assert_eq!(s.web_access_policy().unwrap_err().reason(), "settings-web-access-window-invalid");
        s.web_assertion_max_age_seconds = None;
        assert_eq!(s.web_access_policy().unwrap_err().reason(), "settings-web-access-partial");
        assert!(s.validate().is_err());
        s.web_access_mode = None;
        s.web_authority_public_key = None;
        s.validate().unwrap();
        assert_eq!(s.schema_version(), 51);
        assert_eq!(s.web_access_policy().unwrap_err().reason(), "settings-web-access-not-configured");
    }
}
