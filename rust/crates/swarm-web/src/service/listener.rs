//! Saved-policy listener ownership. Binding alone does not accept HTTP or launch agents.
use super::{
    root, HttpServiceError, NativeHttpService, NativeJobOutcome, NativeObservationFailure,
};
use std::{
    error::Error,
    fmt, io,
    net::{SocketAddr, TcpListener},
    path::Path,
};
use swarm_engine::{RootStore, SettingCount, WebTransportPolicy};

/// Redacted bootstrap/close failure. Compound failures retain both original causes;
/// no retry, rollback or session deletion is implied by any error code.
pub enum NativeServerError {
    Service(HttpServiceError),
    Observation(Box<NativeObservationFailure>),
    /// Loop has drained/closed; preserve original output/signal cause and its
    /// aggregate report, including last acknowledged effects and typed failures.
    StoppedOutput {
        phase: &'static str,
        cause: io::Error,
        report: Box<super::NativeServerReport>,
    },
    OutcomeEncoding {
        cause: serde_json::Error,
        outcome: Box<NativeJobOutcome>,
    },
    SessionState {
        primary: Box<Self>,
    },
    Io {
        phase: &'static str,
        cause: io::Error,
    },
    SocketClose(nix::errno::Errno),
    Job(tokio::task::JoinError),
    Refused(&'static str),
    Cleanup {
        primary: Box<Self>,
        cleanup: Box<Self>,
    },
}
impl NativeServerError {
    /// Keep exact auxiliary observations through response/observer/stop failures.
    /// Empty is missing retained evidence, not rollback; no retry or grant follows.
    pub fn auxiliary_effects(&self)->Vec<swarm_engine::AuxiliaryEffect>{
        let mut result=swarm_engine::auxiliary_effects(self);
        match self {
            Self::Service(error)=>swarm_engine::extend_auxiliary_effects(&mut result,error.auxiliary_effects()),
            Self::Observation(error)=>swarm_engine::extend_auxiliary_effects(&mut result,error.auxiliary_effects()),
            Self::StoppedOutput{report,..}=>swarm_engine::extend_auxiliary_effects(&mut result,report.last_auxiliary_effects.clone()),
            Self::OutcomeEncoding{outcome,..}=>swarm_engine::extend_auxiliary_effects(&mut result,outcome.auxiliary_effects.clone()),
            Self::SessionState{primary}=>swarm_engine::extend_auxiliary_effects(&mut result,primary.auxiliary_effects()),
            Self::Cleanup{primary,cleanup}=>{swarm_engine::extend_auxiliary_effects(&mut result,primary.auxiliary_effects());swarm_engine::extend_auxiliary_effects(&mut result,cleanup.auxiliary_effects());},
            _=>(),
        }result
    }
    /// Possible manual-provider contact through pending outcomes and late output.
    /// False is absence of a witness, never evidence of no remote effects.
    pub fn provider_contact_may_have_occurred(&self)->bool{match self{
        Self::Service(e)=>e.provider_probe().is_some_and(|p|p.contact_may_have_occurred),
        Self::Observation(e)=>e.pending.iter().any(|p|p.provider_probe.as_ref().is_some_and(|p|p.contact_may_have_occurred)),
        Self::OutcomeEncoding{outcome,..}=>outcome.provider_probe.as_ref().is_some_and(|p|p.contact_may_have_occurred),
        Self::StoppedOutput{report,..}=>report.provider_contact_may_have_occurred,
        Self::SessionState{primary}=>primary.provider_contact_may_have_occurred(),
        Self::Cleanup{primary,cleanup}=>primary.provider_contact_may_have_occurred()||cleanup.provider_contact_may_have_occurred(),_=>false,
    }}
    /// Stable diagnostic family, never a raw credential, path or permission grant.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Service(e) => e.code(),
            Self::Observation(e) => e.code(),
            Self::StoppedOutput { phase, .. } => phase,
            Self::OutcomeEncoding { .. } => "web-outcome-response-encoding",
            Self::SessionState { primary } => primary.code(),
            Self::Io { phase, .. } => phase,
            Self::SocketClose(_) => "web-listener-close",
            Self::Job(_) => "web-root-job-failed",
            Self::Refused(code) => code,
            Self::Cleanup { primary, .. } => primary.code(),
        }
    }
    /// Observe every retained known configuration effect through fatal/cleanup
    /// layers. False is not absence/rollback; acknowledged earlier outcomes belong
    /// to the caller's observer, not this pending-error summary.
    pub fn configuration_may_be_recorded(&self) -> bool {
        match self {
            Self::Service(e) => e.configuration_may_be_recorded(),
            Self::Observation(e) => e.configuration_may_be_recorded(),
            Self::StoppedOutput { report, .. } => {
                report.configuration_outcomes > 0
                    || report
                        .first_service_failure
                        .as_ref()
                        .is_some_and(HttpServiceError::configuration_may_be_recorded)
            }
            Self::OutcomeEncoding { outcome, .. } => outcome.configuration_may_be_recorded,
            Self::SessionState { primary } => primary.configuration_may_be_recorded(),
            Self::Cleanup { primary, cleanup } => {
                primary.configuration_may_be_recorded() || cleanup.configuration_may_be_recorded()
            }
            _ => false,
        }
    }
    /// First exact known settings entry through outcome/cleanup wrappers. Unknown
    /// underlying publication keeps None; all pending entries remain inspectable.
    pub fn recorded_configuration(&self) -> Option<&super::RecordedConfiguration> {
        match self {
            Self::Service(error) => error.recorded_configuration(),
            Self::Observation(error) => error.recorded_configuration(),
            Self::StoppedOutput { report, .. } => report.last_recorded_configuration.as_ref(),
            Self::OutcomeEncoding { outcome, .. } => outcome.recorded_configuration.as_ref(),
            Self::SessionState { primary } => primary.recorded_configuration(),
            Self::Cleanup { primary, cleanup } => primary
                .recorded_configuration()
                .or_else(|| cleanup.recorded_configuration()),
            _ => None,
        }
    }
    /// First retained known Work for compatibility; observation_failure exposes
    /// every pending outcome. Acknowledged earlier outcomes are observer-owned.
    pub fn recorded_work(&self) -> Option<(&str, i64)> {
        match self {
            Self::Service(e) => e.recorded_work(),
            Self::Observation(e) => e.recorded_work(),
            Self::StoppedOutput { report, .. } => report
                .last_recorded_work
                .as_ref()
                .map(|(id, revision)| (id.as_str(), *revision)),
            Self::OutcomeEncoding { outcome, .. } => outcome
                .recorded_work
                .as_ref()
                .map(|(id, revision)| (id.as_str(), *revision)),
            Self::SessionState { primary } => primary.recorded_work(),
            Self::Cleanup { primary, cleanup } => {
                primary.recorded_work().or_else(|| cleanup.recorded_work())
            }
            _ => None,
        }
    }
    /// Final report retained after a late output/signal failure. Summaries are
    /// acknowledged effects only; pending observations remain separate failures.
    pub fn stopped_report(&self) -> Option<&super::NativeServerReport> {
        match self {
            Self::StoppedOutput { report, .. } => Some(report),
            Self::SessionState { primary } => primary.stopped_report(),
            Self::Cleanup { primary, cleanup } => primary
                .stopped_report()
                .or_else(|| cleanup.stopped_report()),
            _ => None,
        }
    }
    /// Inspect the unique failed observer and all outcomes retained after it;
    /// no cause/effect is flattened into a successful empty response or retry.
    pub fn observation_failure(&self) -> Option<&NativeObservationFailure> {
        match self {
            Self::Observation(e) => Some(e),
            Self::SessionState { primary } => primary.observation_failure(),
            Self::Cleanup { primary, cleanup } => primary
                .observation_failure()
                .or_else(|| cleanup.observation_failure()),
            _ => None,
        }
    }
    /// The loop retains later admitted outcomes in the same failed handoff,
    /// including through primary/secondary cleanup aggregation. No new observation.
    pub(super) fn observation_failure_mut(&mut self) -> Option<&mut NativeObservationFailure> {
        match self {
            Self::Observation(e) => Some(e),
            Self::SessionState { primary } => primary.observation_failure_mut(),
            Self::Cleanup { primary, cleanup } => primary
                .observation_failure_mut()
                .or_else(|| cleanup.observation_failure_mut()),
            _ => None,
        }
    }
    /// Known or uncertain session publication survives every cleanup layer.
    /// False does not prove absence or authorize deleting credentials/retrying.
    pub fn session_state_may_be_recorded(&self) -> bool {
        match self {
            Self::Service(e) => e.session_state_may_be_recorded(),
            Self::SessionState { .. } => true,
            Self::Cleanup { primary, cleanup } => {
                primary.session_state_may_be_recorded() || cleanup.session_state_may_be_recorded()
            }
            _ => false,
        }
    }
}
impl fmt::Display for NativeServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for NativeServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for NativeServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Service(e) => Some(e),
            Self::Observation(e) => Some(e.as_ref()),
            Self::StoppedOutput { cause, .. } => Some(cause),
            Self::OutcomeEncoding { cause, .. } => Some(cause),
            Self::SessionState { primary } => Some(primary.as_ref()),
            Self::Io { cause, .. } => Some(cause),
            Self::SocketClose(e) => Some(e),
            Self::Job(e) => Some(e),
            Self::Cleanup { primary, .. } => Some(primary.as_ref()),
            Self::Refused(_) => None,
        }
    }
}
/// Retain two failures in order, or return the sole failure without hiding its
/// typed cause. Both operations have already run; this helper performs no IO.
pub(super) fn finish<T>(
    result: Result<T, NativeServerError>,
    cleanup: Result<(), NativeServerError>,
) -> Result<T, NativeServerError> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(NativeServerError::Cleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
/// Bound private native listener, validated startup snapshot and Root service.
/// Binding does not accept connections; serve_until requires an explicit outcome observer.
/// Owner must explicitly close an unserved listener after all users
/// drain; Drop releases descriptors but supplies no checked-close evidence.
pub struct NativeHttpServer {
    pub(super) listener: TcpListener,
    pub(super) service: NativeHttpService,
    pub(super) policy: WebTransportPolicy,
    pub(super) settings_revision: SettingCount,
}
impl NativeHttpServer {
    /// Bind the ordinary saved loopback transport with one trusted single-intent
    /// source prepared/provisioned outside HTTP. No request selects a source or
    /// grants a role. On bind refusal close the supplied owner explicitly too.
    pub fn bind_enrolled(root_path: &Path, address: Option<&str>, source: super::TrustEnrollmentSource) -> Result<Self, NativeServerError> {
        match Self::bind_source(root_path,address,Some(&source)) {
            Ok(mut server) => { server.service.trust_enrollment=Some(source); Ok(server) },
            Err(primary) => match source.close() {
                Ok(()) => Err(primary),
                Err(cause) => Err(NativeServerError::Cleanup {primary:Box::new(primary),cleanup:Box::new(NativeServerError::Service(HttpServiceError::EnrollmentSource {primary:None,cause:Box::new(cause)}))}),
            },
        }
    }
    /// Read validated settings and establish session/binding from one writable
    /// Root. Optional address is a transient override using the same canonical
    /// loopback validator; it never persists a different policy. Zero connection
    /// cap refuses before binding/session creation. Every failed step closes its
    /// owned store/socket and retains cleanup failures. Existing state is preserved.
    pub fn bind(root_path: &Path, address: Option<&str>) -> Result<Self, NativeServerError> {
        Self::bind_source(root_path,address,None)
    }
    /// Retain one opened Root across source check, policy and session bootstrap;
    /// an intervening pathname reopen cannot substitute another datastore.
    fn bind_source(root_path:&Path,address:Option<&str>,source:Option<&super::TrustEnrollmentSource>)->Result<Self,NativeServerError>{
        let store = RootStore::open(root_path, true)
            .map_err(|e| NativeServerError::Service(root("open", e)))?;
        let configured = (|| {
            if let Some(source)=source { source.check_root(&store).map_err(|e|NativeServerError::Service(root("trust-enrollment-root",e)))?; }
            let mut document = store
                .settings()
                .map_err(|e| NativeServerError::Service(root("settings", e)))?;
            if let Some(address) = address {
                document.values.web_bind_address = Some(address.to_owned());
            }
            let policy = document.values.web_transport_policy().map_err(|e| {
                NativeServerError::Service(root(
                    "settings",
                    swarm_engine::RootStoreError::Settings(e),
                ))
            })?;
            if policy.max_connections == Some(0) {
                return Err(NativeServerError::Refused("web-network-disabled"));
            }
            let listener =
                TcpListener::bind(policy.bind_address).map_err(|cause| NativeServerError::Io {
                    phase: "web-listener-bind",
                    cause,
                })?;
            Ok((document.revision, policy, listener))
        })();
        let (settings_revision, policy, listener) = match configured {
            Ok(value) => value,
            Err(primary) => {
                return finish(
                    Err(primary),
                    store
                        .close()
                        .map_err(|e| NativeServerError::Service(root("bootstrap-close", e))),
                )
            }
        };
        match NativeHttpService::from_open_store(
            root_path,
            store,
            &listener,
            policy.cookie_max_age_seconds,
        ) {
            Ok(service) => Ok(Self {
                listener,
                service,
                policy,
                settings_revision,
            }),
            Err(primary) => finish(
                Err(NativeServerError::Service(primary)),
                nix::unistd::close(listener).map_err(NativeServerError::SocketClose),
            ),
        }
    }
    /// Preserve a known persisted session after successful bind and later failure;
    /// wrapping changes no code/cause and never claims a new credential was written.
    pub(super) fn known_session(primary: NativeServerError) -> NativeServerError {
        NativeServerError::SessionState {
            primary: Box::new(primary),
        }
    }
    /// Observe the actual kernel address (including allocated port), never the
    /// configured port-zero placeholder. No network request or Root access occurs.
    pub fn local_addr(&self) -> Result<SocketAddr, NativeServerError> {
        self.listener
            .local_addr()
            .map_err(|cause| NativeServerError::Io {
                phase: "web-listener-address",
                cause,
            })
    }
    /// Open a writable local conductor peer bound to this server's exact datastore.
    /// This trusted in-process adapter grants no HTTP operation or mission consent.
    /// The caller owes explicit close, ordinary lease acquisition and every launch
    /// guard. On binding failure this method closes the opened peer and retains both
    /// causes; server ownership and listener are unchanged. A later path replacement
    /// still requires a fresh binding check before each new session.
    pub fn bound_mission_root(&self) -> Result<RootStore, NativeServerError> {
        let store = RootStore::open(&self.service.root, true)
            .map_err(|cause| NativeServerError::Service(root("conductor-open", cause)))?;
        match self.service.check_binding(&store) {
            Ok(()) => Ok(store),
            Err(primary) => match store.close() {
                Ok(()) => Err(NativeServerError::Service(primary)),
                Err(cleanup) => Err(NativeServerError::Service(HttpServiceError::Close {
                    primary: Some(Box::new(primary)), cleanup: Box::new(cleanup),
                })),
            },
        }
    }
    /// Active startup settings revision; later saved changes do not mutate it.
    pub fn settings_revision(&self) -> SettingCount {
        self.settings_revision
    }
    /// Validated startup policy, including transient address override if supplied.
    /// This bootstrap currently enforces binding/disabled state only; per-request
    /// limits require the transport loop and are not established by this getter.
    pub fn policy(&self) -> &WebTransportPolicy {
        &self.policy
    }
    /// Close Root binding and listener independently, retaining both failures.
    /// No Work, credential, persisted setting or running agent is removed/stopped.
    pub fn close(self) -> Result<(), NativeServerError> {
        let service = self.service.close().map_err(NativeServerError::Service);
        let socket = nix::unistd::close(self.listener).map_err(NativeServerError::SocketClose);
        finish(service, socket).map_err(Self::known_session)
    }
}
