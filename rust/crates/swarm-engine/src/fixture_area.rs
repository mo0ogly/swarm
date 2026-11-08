//! Repository fixture area for Rust tests (operator /tmp interdiction).
//!
//! Purpose: allocate test fixture roots under the Admin setting
//! `runtime_temporary_directory` (installation default `<repo>/.swarm-fixtures/`,
//! 0700, gitignored) instead of the reboot-wiped system tmpfs — the Rust mirror
//! of `tests/fixture_area.py` (`fixture_root(prefix)`, `fixture_area()`).
//! Consumers: every `#[cfg(test)]` fixture in swarm-engine, swarm-web and
//! swarm-cli that previously used `std::env::temp_dir()` or
//! `tempfile::tempdir()`. Preconditions: run from a workspace checkout
//! (`CARGO_MANIFEST_DIR` locates the repository); the saved setting, when
//! present in `<repo>/.swarm/engine-settings.json`, must pass
//! [`EngineSettingsValues::runtime_temporary_directory`].
//! Outputs: a fresh 0700 directory `<prefix><random>` (or the area itself).
//! Errors: settings refusals propagate unchanged; IO errors from directory
//! creation propagate. Side effects: creates the area (0700) and the root on
//! call, never on import. Limits: callers own retention and cleanup — this
//! area holds fixture evidence, not a disposable cache; `std::env::temp_dir()`
//! remains legitimate only for the categories exempted in
//! `tools/check_tmp_interdiction.py::RUST_EXEMPT` (read-only probes of the
//! system volume, paths asserted not to exist, pure path-composition
//! assertions, and the TMPDIR-pinning assertion) — the guard fails any other
//! use. Roots needing no ancestor Git repository go to the operator fallback
//! area `~/.local/state/swarm-fixtures/` instead (see
//! `tests/git_metadata.rs::unavailable_fields_...` for the worked case).
//! D05 (same day): bare `tempfile::tempdir()`/`TempDir::new()` allocations
//! moved to `tempdir_in(area)`/`new_in(area)` — [`fixture_area`] for crates
//! whose contracts tolerate repository ancestors, [`fallback_area`] where
//! private-directory contracts refuse group/other-writable ancestors (the
//! checkout path is 775; crates without a swarm-engine dependency replicate
//! the fallback inline, mirroring this function). Callers whose code path
//! runs `open_private_directory` (enrollment Documents, provisioning
//! prepare, service installation, anchor/declaration reads — including
//! swarm-cli's identity test modules and trust_enrollment) need
//! [`fallback_area`] even inside swarm-cli. The guard also
//! accepts `CARGO_TARGET_TMPDIR` (cargo-managed scratch) and runner-pinned
//! `TMPDIR` reads as governed areas.
use crate::engine_settings::EngineSettingsValues;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Resolve the repository root from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .expect("swarm-engine lives at rust/crates/swarm-engine")
}

/// Resolve the fixture area from the Admin setting, mirroring
/// `tests/fixture_area.py::configured_area`: the value saved in
/// `<repo>/.swarm/engine-settings.json` (edited through the Admin modal) wins,
/// else the shipped installation default. Relative values resolve against the
/// repository. All settings refusals (empty, traversal, `/tmp`) propagate.
/// Read-only: nothing is created here.
pub fn configured_area() -> PathBuf {
    static AREA: OnceLock<PathBuf> = OnceLock::new();
    AREA
        .get_or_init(|| {
            let saved = repo_root().join(".swarm/engine-settings.json");
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
                    let defaults = repo_root()
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
                repo_root().join(path)
            }
        })
        .clone()
}

/// Return the fixture area directory, creating it 0700 if absent. Mirrors
/// `tests/fixture_area.py::fixture_area`. Errors propagate: a read-only
/// checkout cannot host fixtures.
pub fn fixture_area() -> PathBuf {
    use std::os::unix::fs::DirBuilderExt;
    let area = configured_area();
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&area)
        .unwrap_or_else(|e| panic!("fixture area {} cannot be created: {e}", area.display()));
    area
}

/// Create a private durable fixture root under the repository fixture area.
/// Mirrors `tests/fixture_area.py::fixture_root` and replaces
/// `std::env::temp_dir().join(...)`/`tempfile::tempdir()` fixture allocations
/// that would place project data in `/tmp` on a bare cargo run. The root is
/// `<prefix><random>` (12 random hex chars, like `generate_identifier`), mode
/// 0700 by explicit creation. Callers own retention and cleanup.
pub fn fixture_root(prefix: &str) -> PathBuf {
    use std::os::unix::fs::DirBuilderExt;
    let path = fixture_area().join(crate::generate_identifier(prefix).unwrap());
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .unwrap_or_else(|e| panic!("fixture root {} cannot be created: {e}", path.display()));
    path
}

/// Operator fallback fixture area for fixtures whose contracts refuse
/// group/other-writable ancestors (e.g. `open_private_directory` walks every
/// ancestor and refuses any not owned by root or the current user without the
/// sticky bit — a standard checkout is 775). `~/.local/state/swarm-fixtures/`
/// sits under a fully private 0700 chain, mirroring the recipe.py fallback
/// (`tools/verification/recipe.py`) and the 31 October operator instruction.
/// Created 0700 on call, never on import; errors panic with the path (test
/// helper contract, like [`fixture_area`]). Crates that do not depend on
/// swarm-engine replicate this resolution inline in their test modules; any
/// change here must be mirrored there in the same increment.
pub fn fallback_area() -> PathBuf {
    use std::os::unix::fs::DirBuilderExt;
    let home = std::env::var_os("HOME")
        .unwrap_or_else(|| panic!("HOME required for the operator fallback fixture area"));
    let area = Path::new(&home).join(".local/state/swarm-fixtures");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&area)
        .unwrap_or_else(|e| {
            panic!("fallback fixture area {} cannot be created: {e}", area.display())
        });
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The resolved area is the repository default or the saved value, never
    /// /tmp, and lives inside the checkout for relative settings.
    #[test]
    fn area_resolves_from_saved_setting_else_shipped_default() {
        let area = configured_area();
        assert!(!area.starts_with("/tmp"), "{area:?}");
        let saved = repo_root().join(".swarm/engine-settings.json");
        if saved.is_file() {
            let document: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&saved).unwrap()).unwrap();
            if let Some(value) = document
                .get("values")
                .and_then(|v| v.get("runtime_temporary_directory"))
                .and_then(serde_json::Value::as_str)
            {
                assert_eq!(area, repo_root().join(value), "saved value wins");
                return;
            }
        }
        assert_eq!(area.file_name().unwrap(), ".swarm-fixtures");
    }

    /// A root is created inside the area, mode 0700, uniquely prefixed.
    #[test]
    fn fixture_root_creates_private_unique_directory() {
        use std::os::unix::fs::PermissionsExt;
        let a = fixture_root("d04-area-");
        let b = fixture_root("d04-area-");
        assert_ne!(a, b, "roots are unique");
        assert!(a.starts_with(fixture_area()));
        let mode = std::fs::metadata(&a).unwrap().permissions().mode();
        assert_eq!(mode & 0o7777, 0o700, "{a:?} is private");
        assert!(a.file_name().unwrap().to_str().unwrap().starts_with("d04-area-"));
    }
}
