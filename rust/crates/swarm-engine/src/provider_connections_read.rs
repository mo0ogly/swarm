//! Descriptor-bound redacted API connection inventory, no contact or mutation.
use super::{absent, finished, RootStore, RootStoreError as E};
impl RootStore {
    /// Read inherited API connection declarations under shared settings custody.
    /// Actual saved provider_bytes limits the checked file; absence is distinct
    /// from IO/type/identity/close failure. Return digest and redacted sorted rows;
    /// key material never leaves the projection. No provider/default/Agent/Work
    /// write or availability/execution authority is created. Namespace-only launch
    /// validation remains unchanged, including explicit API execution refusal.
    pub fn api_connections(&self) -> Result<crate::ApiConnectionCatalogue, E> {
        let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let policy = settings.values.agent_launch_policy().map_err(E::Settings)?;
            let raw = match self.directory.read_capped("ai-connections.json", policy.provider_bytes) {
                Ok(raw) => Some(raw), Err(error) if absent(&error) => None,
                Err(error) => return Err(E::Storage(error)),
            };
            crate::provider_connection_catalogue::public(raw.as_deref()).map_err(|error| E::Launch(Box::new(error)))
        })();
        finished(result, reservation.close().map_err(E::Storage))
    }
}
