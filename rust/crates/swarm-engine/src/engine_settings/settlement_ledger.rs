//! Saved separate settlement ledger root policy (v49); historical absence is
//! never a default and never implicitly adopts an installation value.
//!
//! Purpose: expose the explicitly saved absolute path of the SEPARATE
//! settlement ledger Root that the read-only PA-09 reconciliation consumer
//! opens as the ledger side of the cross-check.
//! Preconditions: the caller supplies already decoded `EngineSettingsValues`;
//! shape rules (absolute path, no NUL, no trailing slash except root, nonempty)
//! are enforced here and by `validate`.
//! Outputs: the saved path as a validated `&str`.
//! Errors: `settings-paea-ledger-root-not-configured` when the document
//! predates v49 (absence requires explicit adoption), and
//! `settings-paea-ledger-root-invalid` for any saved path outside the shape
//! rules; null and non-string values already refuse at decode.
//! No side effects: pure configuration read — the saved path designates no
//! initialized ledger, proves no separate storage, grants no effect, authority
//! or acceptance. The consumer still opens both roots itself and enforces two
//! physically distinct roots at every call; no IO, journal adoption, migration
//! or Root initialization occurs here.
use super::*;

impl EngineSettingsValues {
    /// Return the explicitly saved separate settlement ledger root path.
    /// Historical absence refuses with `settings-paea-ledger-root-not-configured`
    /// instead of an implicit fallback; any saved path that is not absolute,
    /// contains NUL, is empty, ends with a redundant separator, or escapes
    /// through a `.`/`..` component refuses with
    /// `settings-paea-ledger-root-invalid`. Pure read: no IO, no ledger open,
    /// no separate-storage proof, no effect or authority, and no state change.
    pub fn paea_settlement_ledger_policy(&self) -> Result<&str, EngineSettingsError> {
        let raw = self
            .paea_settlement_ledger_root
            .as_deref()
            .ok_or_else(|| refuse("settings-paea-ledger-root-not-configured"))?;
        let invalid =
            || Err(refuse("settings-paea-ledger-root-invalid"));
        if raw.is_empty() || raw.contains('\0') || !raw.starts_with('/') {
            return invalid();
        }
        // Reject redundant separators and `.`/`..` traversal components: the
        // saved value is one canonical absolute path, never a glob or escape.
        // The leading empty component of an absolute path is the root itself.
        if raw.ends_with('/') && raw != "/"
            || raw[1..].split('/').any(|c| c.is_empty() || c == "." || c == "..")
        {
            return invalid();
        }
        Ok(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_default_declares_the_catalogued_schema_and_path() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        s.validate().unwrap();
        // The installation default designates the standard separate ledger
        // location; a path never designates an initialized ledger nor proves
        // separation — the consumer enforces distinct roots at every call.
        assert_eq!(s.schema_version(), 49);
        assert_eq!(
            s.paea_settlement_ledger_policy().unwrap(),
            "/var/lib/swarm/paea-settlement-ledger"
        );
    }

    #[test]
    fn historical_absence_refuses_without_fallback() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        let mut v = serde_json::to_value(&s).unwrap();
        v.as_object_mut().unwrap().remove("paea_settlement_ledger_root");
        let s: EngineSettingsValues = serde_json::from_value(v).unwrap();
        s.validate().unwrap();
        assert_eq!(s.schema_version(), 48);
        assert_eq!(
            s.paea_settlement_ledger_policy().unwrap_err().reason(),
            "settings-paea-ledger-root-not-configured"
        );
    }

    #[test]
    fn valid_absolute_paths_are_preserved() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        for raw in ["/var/lib/swarm/ledger", "/srv/paea-settlement"] {
            let mut v = serde_json::to_value(&s).unwrap();
            v.as_object_mut()
                .unwrap()
                .insert("paea_settlement_ledger_root".into(), raw.into());
            let s: EngineSettingsValues = serde_json::from_value(v).unwrap();
            s.validate().unwrap();
            assert_eq!(s.schema_version(), 49);
            assert_eq!(s.paea_settlement_ledger_policy().unwrap(), raw);
        }
    }

    #[test]
    fn relative_traversal_and_redundant_paths_refuse() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        for raw in [
            "relative/ledger",
            "",
            "/a/../b",
            "/a/./b",
            "/double//slash",
            "/trailing/",
            "ledger",
        ] {
            let mut v = serde_json::to_value(&s).unwrap();
            v.as_object_mut()
                .unwrap()
                .insert("paea_settlement_ledger_root".into(), raw.into());
            let s: EngineSettingsValues = serde_json::from_value(v).unwrap();
            assert_eq!(
                s.paea_settlement_ledger_policy().unwrap_err().reason(),
                "settings-paea-ledger-root-invalid"
            );
            assert!(s.validate().is_err());
        }
    }

    #[test]
    fn null_and_non_string_values_refuse_at_decode() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        for raw in [
            serde_json::Value::Null,
            serde_json::Value::from(7),
            serde_json::Value::Array(vec![]),
        ] {
            let mut v = serde_json::to_value(&s).unwrap();
            v.as_object_mut()
                .unwrap()
                .insert("paea_settlement_ledger_root".into(), raw);
            assert!(serde_json::from_value::<EngineSettingsValues>(v).is_err());
        }
    }
}
