//! Saved single-use cockpit login ticket lifetime (v50).
//!
//! Purpose: give `web-ticket` an operator-chosen lifetime instead of a value
//! hard-coded in the CLI or server.
//! Preconditions: decoded `EngineSettingsValues`.
//! Outputs: the strictly positive lifetime in seconds.
//! Errors: `settings-web-login-ticket-not-configured` for documents before
//! v50 (absence requires explicit adoption) and
//! `settings-web-login-ticket-seconds-invalid` for zero.
//! No side effects: no IO, ticket, cookie or authority is created here.
use super::*;

impl EngineSettingsValues {
    /// Return the saved login ticket lifetime in seconds. Historical absence
    /// refuses instead of falling back to an installation value; zero refuses
    /// (negative values already refuse at decode). Pure read.
    pub fn web_login_ticket_seconds(&self) -> Result<i64, EngineSettingsError> {
        let value = self
            .web_login_ticket_seconds
            .ok_or_else(|| refuse("settings-web-login-ticket-not-configured"))?
            .0;
        if value < 1 {
            return Err(refuse("settings-web-login-ticket-seconds-invalid"));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_default_is_current_schema() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        s.validate().unwrap();
        assert_eq!(s.schema_version(), 52);
        assert_eq!(s.web_login_ticket_seconds().unwrap(), 60);
    }

    #[test]
    fn historical_absence_and_zero_refuse_without_fallback() {
        let mut s = EngineSettingsValues::installation_defaults().unwrap();
        s.runtime_temporary_directory = None; // later independent v51 group
        s.web_access_mode = None; s.web_authority_public_key = None; s.web_assertion_max_age_seconds = None;
        s.web_login_ticket_seconds = None;
        s.validate().unwrap();
        assert_eq!(s.schema_version(), 49);
        assert_eq!(
            s.web_login_ticket_seconds().unwrap_err().reason(),
            "settings-web-login-ticket-not-configured"
        );
        s.web_login_ticket_seconds = Some(SettingCount(0));
        assert_eq!(s.schema_version(), 50);
        assert!(s.validate().is_err());
        let mut v = serde_json::to_value(EngineSettingsValues::installation_defaults().unwrap()).unwrap();
        v["web_login_ticket_seconds"] = "-1".into();
        v.as_object_mut().unwrap().remove("runtime_temporary_directory");
        for k in ["web_access_mode", "web_authority_public_key", "web_assertion_max_age_seconds"] { v.as_object_mut().unwrap().remove(k); }
        assert!(serde_json::from_value::<EngineSettingsValues>(v).is_err());
    }
}
