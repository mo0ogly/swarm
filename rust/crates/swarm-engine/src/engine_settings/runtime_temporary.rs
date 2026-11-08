//! Saved runtime temporary directory (v51).
//!
//! Purpose: one operator-chosen place for temporary files and test fixtures
//! instead of the system default `/tmp` (wiped at reboot) or paths hard-coded
//! in tools. Consumers: engine process start (exports `TMPDIR`), runtime
//! health probe, `tests/fixture_area.{py,cjs}` and the recipe runner.
//! Preconditions: decoded `EngineSettingsValues`.
//! Outputs: the saved path, absolute or Root-relative, as a validated `&str`;
//! [`EngineSettingsValues::runtime_temporary_path`] resolves it against a Root.
//! Errors: `settings-runtime-temporary-not-configured` before v51 (absence
//! requires explicit adoption) and `settings-runtime-temporary-invalid` for an
//! empty path, NUL, `.`/`..` components, redundant separators or `/tmp`
//! itself or below it.
//! No side effects here: no directory is created or probed.
use super::*;

impl EngineSettingsValues {
    /// Return the saved temporary directory text. Pure read; see module docs
    /// for the refusals. A relative value is relative to the project Root.
    pub fn runtime_temporary_directory(&self) -> Result<&str, EngineSettingsError> {
        let raw = self
            .runtime_temporary_directory
            .as_deref()
            .ok_or_else(|| refuse("settings-runtime-temporary-not-configured"))?;
        let invalid = || Err(refuse("settings-runtime-temporary-invalid"));
        if raw.is_empty() || raw.contains('\0') || raw == "/" {
            return invalid();
        }
        let body = raw.strip_prefix('/').unwrap_or(raw);
        if raw.ends_with('/') || body.split('/').any(|c| c.is_empty() || c == "." || c == "..") {
            return invalid();
        }
        // The point of the setting is to leave the reboot-wiped system tmpfs.
        if raw == "/tmp" || raw.starts_with("/tmp/") {
            return invalid();
        }
        Ok(raw)
    }

    /// Resolve the saved directory against `root` (absolute values are kept).
    /// Same refusals as [`Self::runtime_temporary_directory`]; no IO.
    pub fn runtime_temporary_path(
        &self,
        root: &std::path::Path,
    ) -> Result<std::path::PathBuf, EngineSettingsError> {
        let raw = self.runtime_temporary_directory()?;
        Ok(if raw.starts_with('/') { raw.into() } else { root.join(raw) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_default_is_root_relative_and_current_schema() {
        let s = EngineSettingsValues::installation_defaults().unwrap();
        s.validate().unwrap();
        assert_eq!(s.schema_version(), 52);
        assert_eq!(s.runtime_temporary_directory().unwrap(), ".swarm-fixtures");
        assert_eq!(
            s.runtime_temporary_path(std::path::Path::new("/srv/p")).unwrap(),
            std::path::PathBuf::from("/srv/p/.swarm-fixtures")
        );
    }

    #[test]
    fn absence_tmp_and_traversal_refuse_absolute_kept() {
        let mut s = EngineSettingsValues::installation_defaults().unwrap();
        s.web_access_mode = None; s.web_authority_public_key = None; s.web_assertion_max_age_seconds = None; // later independent v52 group
        s.runtime_temporary_directory = None;
        s.validate().unwrap();
        assert_eq!(s.schema_version(), 50);
        assert_eq!(
            s.runtime_temporary_directory().unwrap_err().reason(),
            "settings-runtime-temporary-not-configured"
        );
        for bad in ["", "/", "/tmp", "/tmp/x", "a/../b", "./x", "x/", "a//b", "a\0b"] {
            s.runtime_temporary_directory = Some(bad.into());
            assert_eq!(
                s.runtime_temporary_directory().unwrap_err().reason(),
                "settings-runtime-temporary-invalid",
                "{bad:?}"
            );
            assert!(s.validate().is_err());
        }
        s.runtime_temporary_directory = Some("/var/tmp/swarm".into());
        assert_eq!(
            s.runtime_temporary_path(std::path::Path::new("/srv/p")).unwrap(),
            std::path::PathBuf::from("/var/tmp/swarm")
        );
    }
}
