//! Read-only preparation of a manual connection probe, never network execution.
use super::provider_connections_write::effective_connection;
use super::{absent, finished, RootStore, RootStoreError as E};
/// Original Go manual-test protocol text, never mission context or an operator
/// duration/retry/model policy. Test adapters must not append user/project data.
pub const API_CONNECTION_PROBE_PROMPT: &str = "Réponds seulement : Connexion réussie.";
/// Private immutable probe snapshot for trusted CLI/web adapters. No Debug or
/// Serialize: URL/key/prompt must not enter logs, argv or generic public DTOs.
/// This in-process value is not an authenticated capability or execution grant.
pub struct ApiConnectionProbeRequest {
    pub base_url: String,
    pub key: String,
    pub model: String,
    pub prompt: &'static str,
    pub maximum_seconds: u64,
    pub observation_ms: u64,
    pub response_max_bytes: Option<usize>,
    pub http1_max_headers: usize,
    pub http2_max_header_bytes: u32,
    pub source_digest: String,
    pub settings_revision: String,
    pub wait_revision: String,
}
impl RootStore {
    /// Check current source digest, data representation, retained-key destination
    /// and disabled state, then snapshot actual wait maximum and saved shared API
    /// response/header/observation limits (v46 shared preparation transport fields). Zero total means
    /// unlimited; ownership lease/silence/preparation timeout are not test deadlines.
    /// Missing policy refuses before contact, never adopts fallback defaults.
    ///
    /// Takes shared settings custody, returns no file/token/Agent/Work/budget
    /// mutation and no network. Probe explicitly targets a connection draft URL;
    /// it does not resolve/publish a CLI alias or claim model/billing readiness.
    /// Caller must check service binding, own cancellation and preserve possible
    /// remote effects through response/output/close. All source/close causes kept.
    pub fn prepare_api_connection_probe(
        &self,
        r: &crate::ApiConnectionChange,
    ) -> Result<ApiConnectionProbeRequest, E> {
        if r.preview {
            return Err(E::Refused("api-connection-probe-preview-refused"));
        }
        let mut c =
            crate::provider_connection_change::validated(r).map_err(|e| E::Launch(Box::new(e)))?;
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let policy = settings.values.agent_launch_policy().map_err(E::Settings)?;
            let raw = match self
                .directory
                .read_capped("ai-connections.json", policy.provider_bytes)
            {
                Ok(raw) => Some(raw),
                Err(e) if absent(&e) => None,
                Err(e) => return Err(E::Storage(e)),
            };
            let source = crate::provider_connection_catalogue::public(raw.as_deref())
                .map_err(|e| E::Launch(Box::new(e)))?;
            if source.digest != r.expected_digest {
                return Err(E::Refused("revision_conflict"));
            }
            let entries = crate::provider_connection_catalogue::records(raw.as_deref())
                .map_err(|e| E::Launch(Box::new(e)))?;
            let old = entries.get(&c.id);
            c = effective_connection(c, r.replace_key, old)?;
            if c.disabled {
                return Err(E::Refused("api-connection-disabled"));
            }
            let wait = self.provider_wait()?;
            // These transport dimensions, including cancellation observation, are shared. Manual probe
            // intent is separate from preparation enablement/turns/timeouts.
            let transport = settings
                .values
                .preparation_call_policy()
                .map_err(E::Settings)?;
            let maximum = u64::try_from(wait.config.values.max_duration_seconds).map_err(|e| {
                E::Launch(Box::new(crate::AgentLaunchError::caused(
                    "provider-wait-values-invalid",
                    e,
                )))
            })?;
            Ok(ApiConnectionProbeRequest {
                base_url: c.base_url,
                key: c.key,
                model: c.model,
                prompt: API_CONNECTION_PROBE_PROMPT,
                maximum_seconds: maximum,
                observation_ms: transport.observation_ms,
                response_max_bytes: transport.response_max_bytes,
                http1_max_headers: transport.http1_max_headers,
                http2_max_header_bytes: transport.http2_max_header_bytes,
                source_digest: source.digest,
                settings_revision: settings.revision.0.to_string(),
                wait_revision: wait.config.revision.to_string(),
            })
        })();
        finished(result, lock.close().map_err(E::Storage))
    }
}
