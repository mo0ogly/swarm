//! Manual text-only network probe. No Work, provider file or budget is mutated.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};
use swarm_engine::{RootStore, RootStoreError};
/// Owned safe observation of a manual probe, separate from local recorded state.
/// Contact is possible, never confirmed remote execution/billing. Original causes
/// stay opaque in-process; no key, URL, request or response body is retained here.
pub struct ProviderProbeReceipt {
    pub contact_may_have_occurred: bool,
    pub error: Option<&'static str>,
    pub latency_ms: u128,
    cause: Option<Box<dyn Error + Send + Sync>>,
}
impl ProviderProbeReceipt {
    /// Original sanitized client/runtime/deadline cause, without formatted content.
    /// Consumers own safe classification; this does not authorize another call.
    pub fn cause(&self) -> Option<&(dyn Error + Send + Sync + 'static)> {
        self.cause.as_deref()
    }
}
/// Execution custody failure. Panic payload remains private/owned; ordinary
/// panic-hook side effects are outside this diagnostic redaction guarantee.
enum ProbeExecutionError {
    Spawn(std::io::Error),
    Panic {
        _payload: std::sync::Mutex<Box<dyn std::any::Any + Send>>,
    },
}
impl std::fmt::Debug for ProbeExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::fmt::Display for ProbeExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("api-probe-execution-failed")
    }
}
impl Error for ProbeExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spawn(e) => Some(e),
            Self::Panic { .. } => None,
        }
    }
}
/// Own and join the private IO runtime on a scoped thread, including when a
/// typed caller already entered Tokio. Spawn IO and opaque panic causes remain
/// errors, never empty success. No detached task, retry or new policy cap; the
/// HTTP owner retains its saved lane until result observation.
fn run_probe_io<T: Send>(work: impl FnOnce() -> T + Send) -> Result<T, ProbeExecutionError> {
    std::thread::scope(|scope| {
        let task = std::thread::Builder::new()
            .spawn_scoped(scope, work)
            .map_err(ProbeExecutionError::Spawn)?;
        task.join().map_err(|payload| ProbeExecutionError::Panic {
            _payload: std::sync::Mutex::new(payload),
        })
    })
}
impl NativeHttpService {
    /// Admit strict POST before opening the bound Root. Snapshot current saved
    /// source, wait and transport policies, check/close Root BEFORE any network.
    /// Run the existing cancellable HTTP client without redirect/retry/tool use.
    /// The saved maximum alone bounds total time; zero is unlimited. Saved
    /// observation_ms governs peer/shutdown observation, never a total cutoff.
    /// A negative provider result is HTTP200 with ok=false and an owned receipt;
    /// malformed/stale/disabled requests refuse before contact. No mission context,
    /// model executable, published key or inferred provider account availability.
    pub(super) fn api_connection_probe(
        &self,
        request: &Request<Vec<u8>>,
        peer_closed: &(dyn Fn() -> bool + Sync),
    ) -> Result<RouteBody, E> {
        if request.method() != Method::POST {
            return Err(E::Method("POST"));
        }
        if request.uri().query().is_some() {
            return Err(E::Refused("api-connection-probe-query-invalid"));
        }
        swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
        let change = swarm_engine::decode_api_connection_change(request.body())
            .map_err(|e| root("api-connection-input", RootStoreError::Launch(Box::new(e))))?;
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let prepared = (|| {
            self.check_binding(&store)?;
            store
                .prepare_api_connection_probe(&change)
                .map_err(|e| root("api-connection-probe", e))
        })();
        let prepared = close(prepared, store.close())?;
        let started = Instant::now();
        let contact = swarm_api_client::ContactObservation::default();
        let result=run_probe_io(||{let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build();match runtime{
          Err(e)=>Err(("api-probe-runtime-failed",Some(Box::new(e) as Box<dyn Error+Send+Sync>))),
          Ok(runtime)=>runtime.block_on(async {
            let request=swarm_api_client::Request {base_url:prepared.base_url.clone(),key:prepared.key.clone(),model:prepared.model.clone(),prompt:prepared.prompt.to_owned(),transport:swarm_api_client::Transport::Json,response_max_bytes:prepared.response_max_bytes,event_max_bytes:None,http1_max_headers:prepared.http1_max_headers,http2_max_header_bytes:prepared.http2_max_header_bytes};
            let cancelled=async{loop {if peer_closed() || self.probe_shutdown.load(std::sync::atomic::Ordering::Acquire){break;}tokio::time::sleep(Duration::from_millis(prepared.observation_ms)).await;}};
            let deadline=async{if prepared.maximum_seconds==0{std::future::pending::<Result<(),tokio::time::error::Elapsed>>().await}else{tokio::time::timeout(Duration::from_secs(prepared.maximum_seconds),std::future::pending::<()>()).await}};
            tokio::select!{biased;
              _=cancelled=>Err(("api-probe-cancelled",None)),
              elapsed=deadline=>match elapsed {Err(e)=>Err(("api-probe-timeout",Some(Box::new(e) as Box<dyn Error+Send+Sync>))),Ok(())=>unreachable!("pending deadline cannot complete")},
              answer=swarm_api_client::probe(request,&contact)=>answer.map_err(|e|(e.code(),Some(Box::new(e) as Box<dyn Error+Send+Sync>))),
            }
          })
        }}).unwrap_or_else(|cause|Err(("api-probe-execution-failed",Some(Box::new(cause) as Box<dyn Error+Send+Sync>))));
        let (text, error, cause) = match result {
            Ok(answer) => (
                Some(if prepared.key.is_empty() {
                    answer.content
                } else {
                    answer.content.replace(&prepared.key, "[redacted]")
                }),
                None,
                None,
            ),
            Err((code, cause)) => (None, Some(code), cause),
        };
        let receipt = Arc::new(ProviderProbeReceipt {
            contact_may_have_occurred: contact.may_have_occurred(),
            error,
            latency_ms: started.elapsed().as_millis(),
            cause,
        });
        let value = serde_json::json!({"ok":error.is_none(),"text":text,"error":error,"latency_ms":receipt.latency_ms,"provider_contact_may_have_occurred":receipt.contact_may_have_occurred,"source_digest":prepared.source_digest,"settings_revision":prepared.settings_revision,"wait_revision":prepared.wait_revision});
        let bytes = serde_json::to_vec(&value).map_err(|e| E::ProviderProbe {
            primary: Box::new(E::Json(e)),
            receipt: Arc::clone(&receipt),
        })?;
        let mut body = RouteBody::plain(bytes);
        body.provider_probe = Some(receipt);
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The actual probe bridge must be callable from a typed adapter already
    /// inside Tokio; it owns/join its IO context without nesting block_on.
    #[test]
    fn probe_io_bridge_is_safe_inside_an_entered_runtime() {
        let outer = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let value = outer.block_on(async {
            run_probe_io(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async { 7_u8 })
            })
            .unwrap()
        });
        assert_eq!(value, 7);
    }
    /// A child execution failure retains its payload opaquely and never
    /// becomes an empty successful text reply or a detached running job.
    #[test]
    fn probe_io_bridge_retains_joined_panic_as_failure() {
        let failure = run_probe_io(|| panic!("owned bridge fixture panic")).unwrap_err();
        assert!(
            matches!(&failure,ProbeExecutionError::Panic{_payload} if _payload.lock().unwrap().downcast_ref::<&str>()==Some(&"owned bridge fixture panic"))
        );
        assert_eq!(format!("{failure:?}"), "api-probe-execution-failed");
    }
    /// Acknowledgement failure must retain a negative probe and its original
    /// cause independently of response ownership/configuration/Work mutations.
    #[test]
    fn probe_receipt_survives_response_observer_and_late_encoding_failure() {
        let receipt = Arc::new(ProviderProbeReceipt {
            contact_may_have_occurred: true,
            error: Some("api-probe-fixture-failure"),
            latency_ms: 0,
            cause: Some(Box::new(std::io::Error::other("private fixture cause"))),
        });
        let error = E::ProviderProbe {
            primary: Box::new(E::Refused("api-probe-output-failed")),
            receipt: Arc::clone(&receipt),
        };
        let reply = super::super::error_reply(error, false).unwrap();
        assert_eq!(
            reply.response.status(),
            http::StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(reply.response.body()).unwrap()
                ["provider_contact_may_have_occurred"],
            true
        );
        let (outcome, _response) = super::super::NativeJobOutcome::from_reply(reply);
        assert!(!outcome.configuration_may_be_recorded && outcome.recorded_work.is_none());
        assert!(Arc::ptr_eq(
            outcome.provider_probe.as_ref().unwrap(),
            &receipt
        ));
        assert!(outcome
            .provider_probe
            .as_ref()
            .unwrap()
            .cause()
            .unwrap()
            .is::<std::io::Error>());
        let failure = outcome
            .observe(&mut |_| Err(std::io::Error::other("fixture output failed")))
            .unwrap_err();
        let failed = super::super::NativeServerError::Observation(Box::new(failure));
        assert!(failed.provider_contact_may_have_occurred());
        let failure = failed.observation_failure().unwrap();
        let wire = serde_json::to_value(failure.pending[0].record()).unwrap();
        assert_eq!(wire["provider_probe"]["error"], "api-probe-fixture-failure");
        assert_eq!(
            wire["provider_probe"]["provider_contact_may_have_occurred"],
            true
        );
        assert!(!wire.to_string().contains("private fixture cause"));
    }
}
