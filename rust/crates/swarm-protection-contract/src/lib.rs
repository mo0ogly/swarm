//! PAEA v1 data contracts for fictitious settlement and fixture Git promotion.
//!
//! This crate performs pure structural checks and canonicalization. It authenticates
//! no caller, admits no mandate, consults no policy, reads no clock, contacts no
//! effect service and grants no permission. Its output is not an approval, an effect
//! receipt, a confinement claim or an assurance qualification. Legacy Trust bytes
//! are intentionally untouched; the new canonical profile is not RFC 8785/JCS.
//! Receipt-report output schema v2 does not change the v1 input data contracts.
#![forbid(unsafe_code)]

mod canonical;
mod comparison;
mod inspection;
mod payment_registry;
mod receipt;
mod strict_json;
mod types;

pub use canonical::{canonicalize, CanonicalAction};
pub use comparison::{compare_documents, DocumentComparison, DocumentIdentity};
pub use inspection::{inspect_document, InspectionReport};
pub use payment_registry::{resolve_payment_documents, PaymentResolution};
pub use receipt::{check_receipt_documents, check_receipt_transition, ReceiptReport};
pub use types::{Action, AdmissionContext, ApprovalBinding, EffectReceipt, Proposal, SourceRef};

use serde::{Deserialize, Serialize};
use std::{error::Error as StdError, fmt};

/// New PAEA wire schema identifier, independent of historical Trust versions.
pub const SCHEMA_VERSION: u32 = 1;

/// Safe public failure with the original JSON cause available to private callers.
/// No error text includes input contents, secrets, paths or caller-selected names.
#[derive(Debug)]
pub enum Error {
    /// Malformed, ambiguous or schema-incompatible JSON; no operation occurred.
    Json(serde_json::Error),
    /// A data field violates a declared representation or domain invariant.
    Invalid(&'static str),
    /// The requested lifecycle transition cannot establish the stated observation.
    Transition,
}

impl Error {
    /// Return a static safe diagnostic code without echoing untrusted JSON.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Json(_) => "protection-document-invalid",
            Self::Invalid(code) => code,
            Self::Transition => "protection-transition-invalid",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.code())
    }
}
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Json(cause) => Some(cause),
            _ => None,
        }
    }
}

/// Decode one strict proposal, rejecting duplicates at every nesting level,
/// unknown fields, floats, invalid Unicode, trailing input and invalid values.
/// Uses serde_json's existing nesting protection; adds no operator size policy.
/// Success establishes data shape only and has no IO or authorization effect.
pub fn decode_proposal(raw: &[u8]) -> Result<Proposal, Error> {
    let value: Proposal = strict_json::decode(raw)?;
    value.validate()?;
    Ok(value)
}

/// Decode an attributed-context-shaped document for interchange/testing.
/// Its contents remain untrusted until a protected producer establishes them;
/// decoding does not turn a supplied principal, label or policy hash into authority.
/// Rejects malformed/duplicate/unknown/invalid fields without side effects.
pub fn decode_context(raw: &[u8]) -> Result<AdmissionContext, Error> {
    let value: AdmissionContext = strict_json::decode(raw)?;
    value.validate()?;
    Ok(value)
}

/// Offline interchange input. Both fields are supplied data; even a well-formed
/// context establishes no authority. The runtime must use its own protected source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub proposal: Proposal,
    pub context: AdmissionContext,
}

/// Decode and validate a complete offline document, including duplicate keys in
/// either nested object. No project, credential, clock or effect service is read.
pub fn decode_document(raw: &[u8]) -> Result<Document, Error> {
    let value: Document = strict_json::decode(raw)?;
    value.proposal.validate()?;
    value.context.validate()?;
    Ok(value)
}

/// Decode receipt-shaped data strictly; successful parsing authenticates neither
/// the sink nor the observation. Signature and effect checks remain mandatory.
pub fn decode_receipt(raw: &[u8]) -> Result<EffectReceipt, Error> {
    let value: EffectReceipt = strict_json::decode(raw)?;
    value.validate()?;
    Ok(value)
}

/// Decode approval-shaped data strictly; validity/attention/separation and current
/// approver authority must be established elsewhere. No permission is produced.
pub fn decode_approval(raw: &[u8]) -> Result<ApprovalBinding, Error> {
    let value: ApprovalBinding = strict_json::decode(raw)?;
    value.validate()?;
    Ok(value)
}

/// Observed action lifecycle. Stored labels never establish their own truth.
/// A future storage owner must authenticate events and serialize actual effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    /// Untrusted proposed action.
    Proposed,
    /// Structurally and authoritatively admitted by a future control service.
    Admitted,
    /// Waiting for the required exact approval.
    ApprovalPending,
    /// Authorized for a bounded action, subject to last-use revalidation.
    Authorized,
    /// Durable intention exists, but the effect outcome is not yet established.
    CommitRequested,
    /// Effect service established commitment.
    Committed,
    /// No commitment was permitted by the relevant boundary.
    Denied,
    /// The effect may have occurred and must be reconciled.
    Unknown,
    /// Reconciliation established commitment.
    ReconciledCommitted,
    /// Reconciliation established no commitment.
    ReconciledNotCommitted,
}

/// Evidence event consumed by the pure transition checker. Constructing this
/// enum is not evidence; the caller must obtain the corresponding actual proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Admission was established.
    Admit,
    /// An approval is required.
    RequireApproval,
    /// Required approval and current preconditions were established.
    Authorize,
    /// The durable commitment request was recorded.
    RequestCommit,
    /// The service observed commitment.
    ObserveCommitted,
    /// The relevant boundary established denial without commitment.
    Deny,
    /// The effect outcome could not be established.
    ObserveUnknown,
    /// Reconciliation established commitment.
    ReconcileCommitted,
    /// Reconciliation established no commitment.
    ReconcileNotCommitted,
}

impl State {
    /// Check the v1 lifecycle without IO, authorization or implicit replay.
    /// UNKNOWN cannot be denied, retried or relabeled successful; it accepts only
    /// an explicit reconciliation observation. Terminal states have no outgoing
    /// mutation. Invalid transitions return an error and leave the input unchanged.
    pub fn transition(self, event: Event) -> Result<Self, Error> {
        use Event as E;
        use State as S;
        match (self, event) {
            (S::Proposed, E::Admit) => Ok(S::Admitted),
            (S::Admitted, E::RequireApproval) => Ok(S::ApprovalPending),
            (S::ApprovalPending, E::Authorize) => Ok(S::Authorized),
            (S::Authorized, E::RequestCommit) => Ok(S::CommitRequested),
            (S::CommitRequested, E::ObserveCommitted) => Ok(S::Committed),
            (S::CommitRequested, E::ObserveUnknown) => Ok(S::Unknown),
            (S::Unknown, E::ReconcileCommitted) => Ok(S::ReconciledCommitted),
            (S::Unknown, E::ReconcileNotCommitted) => Ok(S::ReconciledNotCommitted),
            (
                S::Proposed | S::Admitted | S::ApprovalPending | S::Authorized | S::CommitRequested,
                E::Deny,
            ) => Ok(S::Denied),
            _ => Err(Error::Transition),
        }
    }
}
