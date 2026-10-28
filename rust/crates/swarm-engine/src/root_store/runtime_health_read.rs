//! Storage/version diagnostics through a retained Root binding, no SQLite reads.
use super::{finished, material_path, RootBinding, RootStore, RootStoreError as E};
use crate::{mission_status_contract::RuntimeHealth, WorkEnvironment};
use std::path::Path;

/// Legacy health fields with safe nonfatal observation families. Storage ready
/// never grants launch rights or means SQL, providers or mission checks passed.
#[derive(serde::Serialize)]
pub struct RuntimeHealthView {
    #[serde(flatten)]
    /// Go-compatible storage/version observations, not whole-engine readiness.
    pub runtime: RuntimeHealth,
    /// Safe cause families for unavailable ancillary observations; no raw paths/errors.
    pub observation_errors: Vec<&'static str>,
}
/// Read-only DTO and original ancillary causes. Consumers must preserve causes
/// through encoding/output/flush and expose unavailable observations; not a grant.
pub struct RuntimeHealthRead {
    /// Public safe display document.
    pub view: RuntimeHealthView,
    /// Original typed causes retained independently from successful DTO serialization.
    pub causes: Vec<E>,
}

/// Preserve ancillary observations when a subsequent path/release check fails.
/// Existing compound-error envelopes retain every branch, without claiming that
/// an observation itself was a cleanup operation. Successful reads stay intact.
fn finish(
    read: Result<RuntimeHealthRead, E>,
    check: Result<(), E>,
) -> Result<RuntimeHealthRead, E> {
    match (read, check) {
        (Ok(read), Err(mut primary)) => {
            for cause in read.causes {
                primary = E::Cleanup {
                    primary: Some(Box::new(primary)),
                    cleanup: Box::new(cause),
                };
            }
            Err(primary)
        }
        (read, check) => finished(read, check),
    }
}
impl RootBinding {
    /// Observe storage/version for the original opened project without SQL IO.
    /// The caller provides its material path and trusted clock; current directory
    /// and database O_PATH association are checked before and after observation.
    /// A shared saved-settings reservation guards source choice; no defaults are
    /// adopted. Existing runtime_storage_policy and command/Git policy are the
    /// only operator choices. statfs failures yield blocked/unknown plus original
    /// ancillary causes; malformed/missing policy, clock, association or checked
    /// release fail explicitly. No provider, agent, signal, SQL or policy mutation
    /// occurs. A retained same-inode DB can be unreadable and storage still ready:
    /// storage health is not database integrity, permission or business completion.
    /// The binding remains caller-owned and must eventually be explicitly closed.
    pub fn runtime_health<Ev: WorkEnvironment>(
        &self,
        root: &Path,
        environment: &mut Ev,
    ) -> Result<RuntimeHealthRead, E> {
        let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
        let result = (|| {
            material_path::verify_bound_path(root, self)?;
            let settings = RootStore::read_settings_from(&self.directory)?;
            let at = environment.timestamp().map_err(E::Runtime)?;
            let observed = crate::mission_status_runtime::observe(root, &settings, &at)?;
            let observation_errors = observed.causes.iter().map(E::code).collect();
            Ok(RuntimeHealthRead {
                view: RuntimeHealthView {
                    runtime: observed.runtime,
                    observation_errors,
                },
                causes: observed.causes,
            })
        })();
        let result = finish(result, material_path::verify_bound_path(root, self));
        finish(result, reservation.close().map_err(E::Storage))
    }
}
