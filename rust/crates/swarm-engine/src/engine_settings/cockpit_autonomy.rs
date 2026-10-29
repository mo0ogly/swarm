//! Saved cockpit default-autonomy policy (v48); historical absence is never a
//! default and never implicitly adopts an installation value.
//!
//! Purpose: expose the explicitly saved cockpit default autonomy as the
//! established French wire mode of the canonical protocol enum.
//! Preconditions: the caller supplies already decoded `EngineSettingsValues`;
//! the closed 0/1/2 vocabulary is enforced here and by `validate`.
//! Outputs: `"manuel"` (0), `"assiste"` (1) or `"autonome"` (2).
//! Errors: `settings-cockpit-autonomy-not-configured` when the document
//! predates v48 (absence requires explicit adoption), and
//! `settings-cockpit-autonomy-mode-invalid` for any saved value outside the
//! closed enum; null, negative and malformed strings already refuse at decode.
//! No side effects: display/default mode only — no consent, mission start,
//! authorization or override of saved mission controls follows, and no IO,
//! journal adoption or migration occurs.
use super::*;

impl EngineSettingsValues {
    /// Detect the declared v48 member so historical absence stays
    /// distinguishable from an explicit mode; the setting is a single scalar,
    /// so partial groups are impossible.
    pub(super) fn has_cockpit_autonomy_policy(&self) -> bool {
        self.cockpit_default_autonomy.is_some()
    }

    /// Return the explicitly saved cockpit default autonomy as the established
    /// French wire mode: `0` manuel, `1` assiste, `2` autonome. Historical
    /// absence refuses with `settings-cockpit-autonomy-not-configured` instead
    /// of an implicit fallback; any other saved value refuses with
    /// `settings-cockpit-autonomy-mode-invalid`. Pure read: no consent, mission
    /// start, authorization or override of saved mission controls, and no IO
    /// or state change occurs.
    pub fn cockpit_default_autonomy_mode(&self) -> Result<&'static str, EngineSettingsError> {
        let value = self
            .cockpit_default_autonomy
            .ok_or_else(|| refuse("settings-cockpit-autonomy-not-configured"))?
            .0;
        match value {
            0 => Ok("manuel"),
            1 => Ok("assiste"),
            2 => Ok("autonome"),
            _ => Err(refuse("settings-cockpit-autonomy-mode-invalid")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_default_is_autonome_and_current_schema() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        s.validate().unwrap();
        assert_eq!(s.schema_version(), 49);
        assert_eq!(s.cockpit_default_autonomy_mode().unwrap(), "autonome");
    }

    #[test]
    fn historical_absence_refuses_without_fallback() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        let mut v = serde_json::to_value(&s).unwrap();
        v.as_object_mut().unwrap().remove("cockpit_default_autonomy");
        v.as_object_mut().unwrap().remove("paea_settlement_ledger_root");
        let s: EngineSettingsValues = serde_json::from_value(v).unwrap();
        s.validate().unwrap();
        assert_eq!(s.schema_version(), 47);
        assert!(!s.has_cockpit_autonomy_policy());
        assert_eq!(
            s.cockpit_default_autonomy_mode().unwrap_err().reason(),
            "settings-cockpit-autonomy-not-configured"
        );
    }

    #[test]
    fn every_mode_matches_the_closed_vocabulary() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        for (raw, mode) in [("0", "manuel"), ("1", "assiste"), ("2", "autonome")] {
            let mut v = serde_json::to_value(&s).unwrap();
            v.as_object_mut()
                .unwrap()
                .insert("cockpit_default_autonomy".into(), raw.into());
            let s: EngineSettingsValues = serde_json::from_value(v).unwrap();
            s.validate().unwrap();
            assert_eq!(s.cockpit_default_autonomy_mode().unwrap(), mode);
        }
    }

    #[test]
    fn other_saved_values_refuse_closed() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        for raw in ["3", "300"] {
            let mut v = serde_json::to_value(&s).unwrap();
            v.as_object_mut()
                .unwrap()
                .insert("cockpit_default_autonomy".into(), raw.into());
            let s: EngineSettingsValues = serde_json::from_value(v).unwrap();
            assert_eq!(
                s.cockpit_default_autonomy_mode().unwrap_err().reason(),
                "settings-cockpit-autonomy-mode-invalid"
            );
            assert!(s.validate().is_err());
        }
    }

    #[test]
    fn programmatic_out_of_range_and_absence_stay_distinct() {
        let mut s = EngineSettingsValues::installation_defaults().unwrap();
        s.cockpit_default_autonomy = Some(SettingCount(7));
        assert_eq!(
            s.cockpit_default_autonomy_mode().unwrap_err().reason(),
            "settings-cockpit-autonomy-mode-invalid"
        );
        assert!(s.validate().is_err());
        s.cockpit_default_autonomy = None;
        s.validate().unwrap();
        assert_eq!(
            s.cockpit_default_autonomy_mode().unwrap_err().reason(),
            "settings-cockpit-autonomy-not-configured"
        );
    }

    #[test]
    fn null_negative_and_malformed_strings_refuse_at_decode() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        for raw in [
            serde_json::Value::Null,
            serde_json::Value::String("-1".into()),
            serde_json::Value::String("deux".into()),
            serde_json::Value::String("02".into()),
            serde_json::Value::from(2),
        ] {
            let mut v = serde_json::to_value(&s).unwrap();
            v.as_object_mut()
                .unwrap()
                .insert("cockpit_default_autonomy".into(), raw);
            assert!(serde_json::from_value::<EngineSettingsValues>(v).is_err());
        }
    }
}
