//! Frozen pre-fix closure adapted only to an explicitly owned root/cache.
//! Test-only copy of the retained adapter; only this status comment changed.
//! Admission by root/reviewer is required before this is used as a pre-fix probe.
use crate::engine_settings::EngineSettingsValues;
use std::{path::PathBuf, sync::OnceLock};
/// One fresh owned legacy invocation; no process global or live settings access.
pub(super) struct Legacy { root: PathBuf, area: OnceLock<PathBuf> }
impl Legacy {
    /// Create an empty legacy cache for the supplied isolated fixture only.
    pub(super) fn new(root: PathBuf) -> Self { Self { root, area: OnceLock::new() } }
    /// Retain the exact old selection/validation closure; only root/cache binding
    /// changes. Panic/default behavior is intentional pre-fix evidence, not policy.
pub(super) fn resolve(&self) -> PathBuf {
    self.area
        .get_or_init(|| {
            let saved = self.root.clone().join(".swarm/engine-settings.json");
            let raw = match std::fs::read(saved) {
                Ok(bytes) => {
                    let document: serde_json::Value = serde_json::from_slice(&bytes)
                        .expect(".swarm/engine-settings.json is valid JSON");
                    document
                        .get("values")
                        .and_then(|v| v.get("runtime_temporary_directory"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                }
                Err(_) => None,
            };
            let raw = match raw {
                Some(value) => value,
                None => {
                    let defaults = self.root.clone()
                        .join("rust/crates/swarm-engine/config/engine-settings.json");
                    let document: serde_json::Value = serde_json::from_slice(
                        &std::fs::read(&defaults)
                            .expect("shipped engine-settings.json is readable"),
                    )
                    .expect("shipped engine-settings.json is valid JSON");
                    document
                        .get("runtime_temporary_directory")
                        .and_then(serde_json::Value::as_str)
                        .expect("shipped default configures runtime_temporary_directory")
                        .to_string()
                }
            };
            // Same refusals as the Admin setting itself; a relative value is
            // repository-relative, matching tests/fixture_area.py.
            let values = EngineSettingsValues::installation_defaults().unwrap();
            let mut probe = values;
            probe.runtime_temporary_directory = Some(raw);
            let validated = probe.runtime_temporary_directory().unwrap().to_string();
            let path = PathBuf::from(validated);
            if path.is_absolute() {
                path
            } else {
                self.root.clone().join(path)
            }
        })
        .clone()
}

}
