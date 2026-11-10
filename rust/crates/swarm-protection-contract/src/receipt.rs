//! Bind receipt-shaped data to a canonical action and an explicit prior state.
use crate::{
    canonicalize, decode_document, decode_receipt, CanonicalAction, DocumentIdentity,
    EffectReceipt, Error, Event, State,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Offline receipt report schema v2, separate from the v1 input data schemas.
/// Raw hashes bind only the exact supplied byte slices; whitespace, request keys
/// and observation changes remain distinguishable even when canonical identity
/// or proposed state is unchanged. Digests and state labels are not signatures,
/// current authority, authenticated observations or proof of a protected effect.
/// Consumers must retain the inputs and establish protected provenance separately;
/// this report supplies no timestamp, atomic snapshot, uniqueness or approval.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReceiptReport {
    pub schema_version: u32,
    pub scope: &'static str,
    pub document: DocumentIdentity,
    pub receipt_sha256: String,
    pub previous_state: State,
    pub proposed_state: State,
    pub authority: &'static str,
    pub receipt_authenticity: &'static str,
    pub effect: &'static str,
}

/// Strictly decode a v1 document and receipt, recompute canonical action, then
/// check exact action/intent/expected-sink binding and the shared lifecycle.
/// Only after successful validation return a schema-v2 report binding both raw
/// byte slices by SHA256, along with canonical identity and explicit prior state.
///
/// Inputs, expected sink and previous state are supplied data and stay untrusted.
/// Hashes need an authorized recipient; even matching forged data can pass.
/// Decode/representation/binding/lifecycle errors retain existing safe Error
/// codes and typed JSON causes and return no report. This pure function mutates
/// no input, performs no IO or clock/policy/authority lookup and authenticates no
/// sink, signature or observed artifact. No effect, publication or state is applied.
pub fn check_receipt_documents(
    document_raw: &[u8],
    receipt_raw: &[u8],
    expected_sink: &str,
    previous: State,
) -> Result<ReceiptReport, Error> {
    let document = decode_document(document_raw)?;
    let action = canonicalize(&document.proposal, &document.context)?;
    let receipt = decode_receipt(receipt_raw)?;
    let proposed = check_receipt_transition(&action, &receipt, expected_sink, previous)?;
    Ok(ReceiptReport {
        schema_version: 2,
        scope: "offline-data-only",
        document: DocumentIdentity {
            document_sha256: format!("{:x}", Sha256::digest(document_raw)),
            action_digest: action.digest().to_owned(),
            intent_id: action.intent_id().to_owned(),
        },
        receipt_sha256: format!("{:x}", Sha256::digest(receipt_raw)),
        previous_state: previous,
        proposed_state: proposed,
        authority: "not-established",
        receipt_authenticity: "not-established",
        effect: "not-established",
    })
}

/// Check receipt representation, exact action/intent/sink correspondence and the
/// existing lifecycle, returning a proposed next state without mutating anything.
/// `expected_sink` and `previous` must ultimately come from the protected owner;
/// caller-supplied values do not become trusted by passing this function.
///
/// Invalid representation/binding returns a static safe error. Invalid lifecycle
/// returns Error::Transition. UNKNOWN accepts only explicit reconciliation
/// outcomes; terminal states cannot be advanced or replayed by this checker.
/// DENIED follows the shared lifecycle, including every earlier nonterminal
/// phase through COMMIT_REQUESTED; it never establishes absence of effect itself.
/// Reading an already-recorded result is a separate operation. No receipt is
/// signed/authenticated, no observation is verified, no authority is refreshed,
/// and no effect, storage, clock, retry or publication occurs. Even a fully
/// consistent forged receipt can pass: consumers must authenticate its producer,
/// observation and order before persisting or acting on the returned state.
pub fn check_receipt_transition(
    action: &CanonicalAction,
    receipt: &EffectReceipt,
    expected_sink: &str,
    previous: State,
) -> Result<State, Error> {
    receipt.validate()?;
    if !swarm_identity_contract::name(expected_sink.as_bytes()) {
        return Err(Error::Invalid("protection-receipt-expected-sink"));
    }
    if receipt.action_digest != action.digest()
        || receipt.intent_id != action.intent_id()
        || receipt.sink_id != expected_sink
    {
        return Err(Error::Invalid("protection-receipt-binding"));
    }
    let event = match receipt.outcome {
        State::Committed => Event::ObserveCommitted,
        State::Denied => Event::Deny,
        State::Unknown => Event::ObserveUnknown,
        State::ReconciledCommitted => Event::ReconcileCommitted,
        State::ReconciledNotCommitted => Event::ReconcileNotCommitted,
        State::Proposed
        | State::Admitted
        | State::ApprovalPending
        | State::Authorized
        | State::CommitRequested => {
            return Err(Error::Invalid("protection-receipt-representation"))
        }
    };
    previous.transition(event)
}
