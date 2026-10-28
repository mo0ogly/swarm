//! Versioned token pricing, not provider billing, budget admission or identity.
use crate::{
    model_wire::{Fields, Kind},
    GoFloat, WorkError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

/// Inherited Go catalogue byte-format guard, distinct from an operator budget.
/// No monetary, token-volume or version-count ceiling is bundled in this module.
pub const RATE_CATALOGUE_MAX_BYTES: usize = 2 * 1024 * 1024;
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ModelRate {
    pub version: String,
    pub provider: String,
    pub model: String,
    pub currency: String,
    pub input_per_million: Option<GoFloat>,
    pub output_per_million: Option<GoFloat>,
    pub cache_read_per_million: Option<GoFloat>,
    pub cache_write_per_million: Option<GoFloat>,
    pub source: String,
    pub reference_date: String,
    pub actor: String,
    pub updated: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RateCatalogue {
    pub schema_version: i64,
    pub digest: String,
    pub rates: Option<Vec<ModelRate>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RateChange {
    pub schema_version: i64,
    pub expected_digest: String,
    pub rate: ModelRate,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RateQuoteRequest {
    pub rate_version: String,
    pub non_cached_input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
}
#[derive(Clone, Debug, Serialize)]
pub struct RateQuote {
    pub status: String,
    pub estimated_amount: Option<GoFloat>,
    pub rate: ModelRate,
    pub tokens: RateQuoteRequest,
    pub missing_rates: Vec<String>,
}
pub enum PricingError {
    Decode(WorkError),
    Encode(WorkError),
    Date(time::error::ComponentRange),
    Clock(swarm_wire_json::go_rfc3339::GoTimestampError),
    Refused(&'static str),
}
impl PricingError {
    /// Stable public reason; private request contents remain in typed causes.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Decode(_) => "pricing-document-invalid",
            Self::Encode(_) => "pricing-encoding-failed",
            Self::Date(_) => "pricing-date-invalid",
            Self::Clock(_) => "pricing-engine-clock-invalid",
            Self::Refused(s) => s,
        }
    }
}
impl fmt::Display for PricingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for PricingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PricingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(e) | Self::Encode(e) => Some(e),
            Self::Date(e) => Some(e),
            Self::Clock(e) => Some(e),
            _ => None,
        }
    }
}
type Result<T> = std::result::Result<T, PricingError>;
/// Exact Go field shapes/order. Nil price is unknown, never an implicit zero.
fn rate_shape() -> Fields {
    &[
        ("version", Kind::String),
        ("provider", Kind::String),
        ("model", Kind::String),
        ("currency", Kind::String),
        ("input_per_million", Kind::Pointer(&Kind::Float64)),
        ("output_per_million", Kind::Pointer(&Kind::Float64)),
        ("cache_read_per_million", Kind::Pointer(&Kind::Float64)),
        ("cache_write_per_million", Kind::Pointer(&Kind::Float64)),
        ("source", Kind::String),
        ("reference_date", Kind::String),
        ("actor", Kind::String),
        ("updated", Kind::String),
    ]
}
/// Decode strict Go-shaped save intent without IO, authorization or application.
/// Unknown/type/range/UTF8/trailing errors preserve the original codec cause.
pub fn decode_rate_change(raw: &[u8]) -> Result<RateChange> {
    crate::work::decode_mode(
        raw,
        &[
            ("schema_version", Kind::Int64),
            ("expected_digest", Kind::String),
            ("rate", Kind::Object(rate_shape)),
        ],
        true,
    )
    .map_err(PricingError::Decode)
}
/// Decode strict explicit quote volumes; validation happens when quoting.
/// No prices are read or inferred and no provider is called.
pub fn decode_rate_quote(raw: &[u8]) -> Result<RateQuoteRequest> {
    crate::work::decode_mode(
        raw,
        &[
            ("rate_version", Kind::String),
            ("non_cached_input_tokens", Kind::Pointer(&Kind::Int64)),
            ("output_tokens", Kind::Pointer(&Kind::Int64)),
            ("cache_read_tokens", Kind::Int64),
            ("cache_write_tokens", Kind::Int64),
        ],
        true,
    )
    .map_err(PricingError::Decode)
}
/// Validate labels, three-letter currency, documented provenance, real YYYY-MM-DD
/// and explicit finite nonnegative prices. Conditions remain unverified text.
/// This authenticates no provider/actor and enforces no monetary ceiling.
pub fn validate_model_rate(r: &ModelRate) -> Result<()> {
    let model = r.model.as_bytes();
    if !swarm_identity_contract::name(r.provider.as_bytes())
        || model.is_empty()
        || model.len() > 160
        || !model[0].is_ascii_alphanumeric()
        || !model
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'/' | b'-'))
        || r.currency.len() != 3
        || !r.currency.bytes().all(|b| b.is_ascii_uppercase())
    {
        return Err(PricingError::Refused(
            "pricing-provider-model-currency-invalid",
        ));
    }
    if !crate::text::nonempty(&r.source) || r.source.len() > 2000 {
        return Err(PricingError::Refused("pricing-source-required"));
    }
    let date = r.reference_date.as_bytes();
    if date.len() != 10
        || date[4] != b'-'
        || date[7] != b'-'
        || !date
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return Err(PricingError::Refused("pricing-date-format"));
    }
    let num = |s: &[u8]| s.iter().fold(0i32, |v, b| v * 10 + i32::from(b - b'0'));
    let month = time::Month::try_from(num(&date[5..7]) as u8).map_err(PricingError::Date)?;
    time::Date::from_calendar_date(num(&date[..4]), month, num(&date[8..]) as u8)
        .map_err(PricingError::Date)?;
    let prices = [
        r.input_per_million,
        r.output_per_million,
        r.cache_read_per_million,
        r.cache_write_per_million,
    ];
    if prices
        .iter()
        .flatten()
        .any(|p| !p.0.is_finite() || p.0 < 0.0)
    {
        return Err(PricingError::Refused("pricing-finite-nonnegative-required"));
    }
    if prices.iter().all(Option::is_none) {
        return Err(PricingError::Refused("pricing-explicit-rate-required"));
    }
    Ok(())
}
/// Validate save schema and proposed price before any lock/state change.
/// Supplied version/actor/time are ignored by the engine-owned save operation.
pub fn validate_rate_change(change: &RateChange) -> Result<()> {
    if change.schema_version != 1 {
        return Err(PricingError::Refused("pricing-schema-invalid"));
    }
    validate_model_rate(&change.rate)
}
/// Hash exact file bytes for concurrency, not identity/authenticity or rollback protection.
pub(crate) fn rate_digest(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
impl RateCatalogue {
    /// Explicit absent-file state; zero prices are never invented.
    pub fn empty() -> Self {
        Self {
            schema_version: 1,
            digest: rate_digest(&[]),
            rates: Some(vec![]),
        }
    }
    /// Decode strict retained catalogue, validate every price and replace supplied
    /// digest with its exact byte hash. The actual Go producer initializes schema1
    /// and rates[] before merge; omitted/null scalars retain those values while
    /// explicit rates:null clears the slice. A present empty/invalid file is an error;
    /// the inherited byte-format guard refuses without truncating or mutating it.
    pub fn decode(raw: &[u8]) -> Result<Self> {
        if raw.len() > RATE_CATALOGUE_MAX_BYTES {
            return Err(PricingError::Refused("pricing-catalogue-format-too-large"));
        }
        let mut c: Self = crate::work::decode_mode_from(
            raw,
            &[
                ("schema_version", Kind::Int64),
                ("digest", Kind::String),
                ("rates", Kind::Array(&Kind::Object(rate_shape))),
            ],
            true,
            Some(br#"{"schema_version":1,"rates":[]}"#),
        )
        .map_err(PricingError::Decode)?;
        if c.schema_version != 1 {
            return Err(PricingError::Refused("pricing-schema-invalid"));
        }
        for r in c.rates.iter().flatten() {
            validate_model_rate(r)?;
        }
        c.digest = rate_digest(raw);
        Ok(c)
    }
    /// Observe one exact retained version and quote explicit volumes; unknown
    /// versions refuse. No history, reservations, budget or usage is changed.
    pub fn quote(&self, request: &RateQuoteRequest) -> Result<RateQuote> {
        let rate = self
            .rates
            .iter()
            .flatten()
            .find(|r| r.version == request.rate_version)
            .ok_or(PricingError::Refused("pricing-version-unknown"))?;
        quote_model_rate(rate, request)
    }
}
/// Quote a selected immutable version with separate non-cached/cache categories.
/// Missing used categories yield unknown; zero volumes need no rate. Negative or
/// missing explicit volumes, mismatched version, invalid rates and nonfinite
/// arithmetic refuse with no IO. Floating simulations are not invoice arithmetic.
pub fn quote_model_rate(rate: &ModelRate, request: &RateQuoteRequest) -> Result<RateQuote> {
    validate_model_rate(rate)?;
    if rate.version != request.rate_version {
        return Err(PricingError::Refused("pricing-version-unknown"));
    }
    let input = request
        .non_cached_input_tokens
        .ok_or(PricingError::Refused("pricing-explicit-volumes-required"))?;
    let output = request
        .output_tokens
        .ok_or(PricingError::Refused("pricing-explicit-volumes-required"))?;
    let parts = [
        ("input", input, rate.input_per_million),
        ("output", output, rate.output_per_million),
        (
            "cache_read",
            request.cache_read_tokens,
            rate.cache_read_per_million,
        ),
        (
            "cache_write",
            request.cache_write_tokens,
            rate.cache_write_per_million,
        ),
    ];
    if parts.iter().any(|(_, n, _)| *n < 0) {
        return Err(PricingError::Refused(
            "pricing-nonnegative-volumes-required",
        ));
    }
    let mut total = 0.0;
    let mut missing = vec![];
    for (name, tokens, price) in parts {
        if tokens == 0 {
            continue;
        }
        if let Some(price) = price {
            // Preserve the actual legacy Go finite evaluation order.
            let product = tokens as f64 * price.0;
            let amount = if product.is_infinite() {
                (tokens as f64 / 1e6) * price.0
            } else {
                product / 1e6
            };
            total += amount;
            if !total.is_finite() {
                return Err(PricingError::Refused("pricing-quote-not-representable"));
            }
        } else {
            missing.push(name.into());
        }
    }
    Ok(RateQuote {
        status: if missing.is_empty() {
            "estimated"
        } else {
            "unknown"
        }
        .into(),
        estimated_amount: missing.is_empty().then_some(GoFloat(total)),
        rate: rate.clone(),
        tokens: request.clone(),
        missing_rates: missing,
    })
}
