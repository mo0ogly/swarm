//! Native Admin pricing routes reuse Root's saved validation, digest conflict
//! detection and catalogue decoding. No provider call, budget admission or
//! identity is introduced by this adapter.

use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{
    decode_rate_change, decode_rate_quote, encode_model, validate_rate_change, ModelRate,
    RateQuote, RootStore, RootStoreError, SystemEnvironment,
};

/// Server-selected supported operation, never deserialized as caller authority.
#[derive(Clone, Copy)]
pub(super) enum PricingOperation {
    Show,
    Save,
    Estimate,
}

impl NativeHttpService {
    /// Admit fixed methods/empty query and bounded strict intent before opening
    /// the descriptor-bound Root. Save appends with actual digest/engine metadata;
    /// estimation is read-only. Encode/close errors preserve known or uncertain
    /// publication; no settings event/revision, authenticated actor or retry is
    /// invented. Corrupt stored catalogues are server failures, not bad input.
    pub(super) fn pricing_route(
        &self,
        operation: PricingOperation,
        request: &Request<Vec<u8>>,
    ) -> Result<RouteBody, E> {
        let post = !matches!(operation, PricingOperation::Show);
        if (post && request.method() != Method::POST)
            || (!post && request.method() != Method::GET && request.method() != Method::HEAD)
        {
            return Err(E::Method(if post { "POST" } else { "GET, HEAD" }));
        }
        if request.uri().query().is_some() {
            return Err(E::Refused("pricing-query-invalid"));
        }
        if post {
            swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
        } else if !request.body().is_empty() {
            return Err(E::Refused("pricing-read-body-invalid"));
        }
        let change = if matches!(operation, PricingOperation::Save) {
            let change = decode_rate_change(request.body()).map_err(|e| pricing_input("pricing-input", e))?;
            validate_rate_change(&change).map_err(|e| pricing_input("pricing-input", e))?;
            Some(change)
        } else { None };
        let quote_request = if matches!(operation, PricingOperation::Estimate) {
            Some(decode_rate_quote(request.body()).map_err(|e| pricing_input("pricing-quote-input", e))?)
        } else { None };
        let store = RootStore::open(&self.root, matches!(operation, PricingOperation::Save))
            .map_err(|e| root("open", e))?;
        let mut recorded = false;
        let result = (|| {
            self.check_binding(&store)?;
            let bytes = match operation {
                PricingOperation::Show => {
                    let catalogue = store.rate_catalogue().map_err(|e| root("pricing-read", e))?;
                    encode_model(&catalogue).map_err(E::Model)?
                }
                PricingOperation::Save => {
                    let change = change.as_ref().ok_or(E::Refused("pricing-input-missing"))?;
                    let rate: ModelRate = store
                        .save_rate(change, &mut SystemEnvironment)
                        .map_err(|e| root("pricing-save", e))?;
                    recorded = true;
                    encode_model(&rate).map_err(E::Model)?
                }
                PricingOperation::Estimate => {
                    let request = quote_request.as_ref().ok_or(E::Refused("pricing-input-missing"))?;
                    let catalogue = store.rate_catalogue().map_err(|e| root("pricing-read", e))?;
                    let quote: RateQuote = catalogue
                        .quote(request)
                        .map_err(|e| pricing_input("pricing-quote", e))?;
                    encode_model(&quote).map_err(E::Model)?
                }
            };
            Ok(RouteBody {
                observation_causes: Vec::new(),
                bytes,
                recorded_work: None,
                configuration_recorded: recorded,
                recorded_configuration: None,
            })
        })();
        close(result, store.close()).map_err(|primary| {
            if recorded || primary.configuration_may_be_recorded() {
                // Save intent already validated: known/uncertain publication stays
                // a typed state, never a fabricated RecordedConfiguration digest.
                E::ConfigurationState {
                    primary: Box::new(primary),
                }
            } else {
                primary
            }
        })
    }
}

/// Map engine pricing decode/validation refusals to the root store error shape
/// so Published/Storage causes remain traversable through Root { operation }.
fn pricing_input(operation: &'static str, cause: swarm_engine::PricingError) -> E {
    root(operation, RootStoreError::Pricing(cause))
}
