//! BLP reader-frontier guard for the served PAEA git-call preview/presentation
//! projections (BLP-RET-05). Presentation-only read path: the held proposal
//! and declared context are classified material only when a CURRENT control
//! attribution binds their exact canonical bytes to this project root and the
//! current BLP policy. Unknown provenance refuses without any disclosure —
//! even for the highest-cleared reader — and a session alone is never
//! authorization. This module only computes a typed verdict; it never
//! classifies project contents, mints labels from top-of-catalog assumptions,
//! or replaces the prompt-bytes provenance work held by the separate
//! preparation actor.
use super::trust_store::{MAX_STATE, STATE};
use super::{absent, RootStore, RootStoreError as R};
use crate::trust::{authentication, state::State, TrustError as E};
use std::path::Path;

/// Stable reader-guard refusal codes carried to the HTTP layer, which maps
/// them to explicit refusals without echoing any classified payload.
pub(crate) const AUTHENTICATION_REQUIRED: &str = "paea-reader-authentication-required";
pub(crate) const POLICY_REQUIRED: &str = "paea-reader-policy-required";
pub(crate) const POLICY_STALE: &str = "paea-reader-policy-stale";
pub(crate) const ATTRIBUTION_REQUIRED: &str = "paea-reader-attribution-required";
pub(crate) const ATTRIBUTION_MISMATCH: &str = "paea-reader-attribution-mismatch";
pub(crate) const ACCESS_DENIED: &str = "paea-reader-access-denied";

fn failure(e: E) -> R {
    R::Trust(Box::new(e))
}

/// Why the requested classified projection is (not) served. Every variant is
/// an explicit observed state, never an empty success or a fabricated label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderVerdict {
    /// The projection may be served to this authenticated, cleared reader.
    Allowed,
    /// Refuse with the stable code; no classified byte may accompany it.
    Refused(&'static str),
}

/// Shared reader-identity observation behind both the served surfaces' early
/// precheck ([`RootStore::paea_reader_identity_precheck`]) and the full
/// verdict ([`RootStore::paea_reader_verdict`]): one read of this root's
/// trust journal, decoded/validated, with the local bearer authenticated
/// against the CURRENT persisted identities.
/// Shared reader-identity observation behind both the served PAEA surfaces'
/// precheck/verdict and, since BF-16a, the preparation dialogue frontier: one
/// read of this root's trust journal, decoded/validated, with the local
/// bearer authenticated against the CURRENT persisted identities. `pub(super)`
/// lets the sibling dialogue frontier re-use it unchanged so exactly one
/// authentication happens on the same real current bytes; each frontier maps
/// the three observed states to its own stable refusal codes.
pub(super) enum ReaderIdentityState {
    /// No reader credential: a session alone is never authorization.
    BareSession,
    /// Trust journal absent: no identity can be recognized and no policy or
    /// attribution can exist — the full verdict's explicit
    /// `Refused(POLICY_REQUIRED)` observed state.
    AbsentJournal,
    /// Credential recognized against the current validated state; the actor
    /// is carried so the full verdict uses exactly one authentication, as
    /// before the shared observation existed.
    Current { state: State, actor: authentication::Actor },
}

/// Outcome of the shared reader-identity precheck: an explicit refusal code
/// (bare session, absent trust journal) or the recognized credential with
/// the current validated state and its authenticated actor.
enum ReaderIdentityOutcome {
    /// Refuse with the stable code before any further IO; no byte disclosed.
    Refused(&'static str),
    /// Credential recognized against the current validated trust state.
    Current(State, authentication::Actor),
}

impl RootStore {
    /// Shared reader-identity observation behind both the public precheck
    /// ([`Self::paea_reader_identity_precheck`]) and the full verdict
    /// ([`Self::paea_reader_verdict`]): one read of this root's trust
    /// journal, decoded/validated, with the local bearer authenticated
    /// against the CURRENT persisted identities.
    pub(super) fn reader_identity_state(
        &self,
        token: Option<&str>,
    ) -> Result<ReaderIdentityState, R> {
        let Some(token) = token else {
            return Ok(ReaderIdentityState::BareSession);
        };
        let raw = match self.directory.read_bounded(STATE, MAX_STATE) {
            Ok(raw) => raw,
            // Absent journal: trust never configured on this root, so no
            // identity can be recognized and no policy/attribution exists.
            // Explicit refusal state without any fabricated policy or
            // disclosure.
            Err(e) if absent(&e) => return Ok(ReaderIdentityState::AbsentJournal),
            Err(e) => return Err(R::Storage(e)),
        };
        let state = State::decode(&raw).map_err(failure)?;
        state.validate().map_err(failure)?;
        state.require_supported().map_err(failure)?;
        // Authenticate the bearer against the CURRENT persisted identities.
        // Unknown/disabled tokens propagate the typed `TrustError::Authorization`
        // (an HTTP refusal, e.g. 403 with `authorization_required`), unlike a
        // bare session which is the explicit `Refused(AUTHENTICATION_REQUIRED)`
        // verdict above. Both paths carry no classified byte; the distinction
        // is observable and asserted by the web suite's refusal matrix.
        let actor = authentication::authenticated(&state, authentication::Source::Local(token))
            .map_err(failure)?;
        Ok(ReaderIdentityState::Current { state, actor })
    }

    /// Shared reader-identity precheck over the CURRENT trust state, for the
    /// served reader surfaces: decides the credential BEFORE any
    /// settings/ledger/journal IO of the caller, so an unrecognized
    /// credential can learn nothing beyond the typed identity refusal.
    /// Output: `Refused(code)` for a bare session
    /// (`AUTHENTICATION_REQUIRED`) or an absent trust journal
    /// (`POLICY_REQUIRED`); `Current(state, actor)` for a recognized
    /// credential. Unknown/disabled token → typed
    /// `TrustError::Authorization` (HTTP 403 `authorization_required`).
    /// Errors: decode/validate/storage failures propagate typed
    /// (`RootStoreError::Trust`/`Storage`). No disclosure beyond those
    /// states; no classified byte is ever carried. Lock-free, takes no
    /// reservation beside a running protocol, mutates nothing.
    fn reader_identity_precheck(
        &self,
        token: Option<&str>,
    ) -> Result<ReaderIdentityOutcome, R> {
        match self.reader_identity_state(token)? {
            ReaderIdentityState::BareSession => {
                Ok(ReaderIdentityOutcome::Refused(AUTHENTICATION_REQUIRED))
            }
            ReaderIdentityState::AbsentJournal => {
                Ok(ReaderIdentityOutcome::Refused(POLICY_REQUIRED))
            }
            ReaderIdentityState::Current { state, actor } => {
                Ok(ReaderIdentityOutcome::Current(state, actor))
            }
        }
    }

    /// Public shared identity precheck for the served reader surfaces: the
    /// PA-09 reconcile route calls it BEFORE any settings/ledger/journal IO
    /// so an unrecognized credential learns nothing beyond the typed
    /// identity refusal. Input: `token` is the extracted
    /// `Authorization: Bearer <token>` value (`None` = session alone).
    /// Output: `Ok(Some(code))` — refuse with that stable guard code: a
    /// bare session (`paea-reader-authentication-required`; a session alone
    /// is never authorization) or an absent trust journal
    /// (`paea-reader-policy-required`: no identity can be recognized and no
    /// policy or attribution can exist); `Ok(None)` — the credential is
    /// recognized against the CURRENT validated trust state: the caller may
    /// proceed with its reads but MUST still evaluate the full verdict
    /// ([`Self::paea_reader_verdict`]) before serving anything, because
    /// this precheck alone grants no authorization (no policy currency,
    /// attribution or clearance decision). Unknown/disabled token → typed
    /// `TrustError::Authorization` (HTTP 403 `authorization_required`).
    /// Errors: decode/validate/storage failures propagate typed
    /// (`RootStoreError::Trust`/`Storage`). No disclosure beyond those
    /// states; no classified byte is ever carried; no identity is cached
    /// beyond the call. Lock-free, takes no reservation beside a running
    /// protocol, mutates nothing. Limits: reads only this root's trust
    /// journal; performs no policy, attribution or clearance decision; no
    /// enforcement beyond the reader presentation surfaces is claimed
    /// anywhere.
    pub fn paea_reader_identity_precheck(
        &self,
        token: Option<&str>,
    ) -> Result<Option<&'static str>, R> {
        match self.reader_identity_precheck(token)? {
            ReaderIdentityOutcome::Refused(code) => Ok(Some(code)),
            ReaderIdentityOutcome::Current(..) => Ok(None),
        }
    }

    /// Decide whether a Bearer-token reader may receive the classified
    /// projection of one exact prepared action (canonical bytes digest +
    /// project root) under the CURRENT BLP policy, re-read at every
    /// consumption. Inputs: `token` is the extracted
    /// `Authorization: Bearer <token>` value (`None` = session alone, never
    /// authorization); `canonical_digest` is the exact canonical action digest
    /// the projection would disclose; `project_path` is the actual bound
    /// project root. Output: a typed verdict — `Allowed` requires ALL of: a
    /// known non-disabled local identity behind the token; a configured BLP
    /// policy whose recorded authority is CURRENT (recorded trust revision ==
    /// current state revision); a control attribution in the current entry
    /// binding the exact canonical digest, this project root and the current
    /// policy revision; and the reader's clearance dominating the attributed
    /// classification under `Policy::check_read`. A missing prerequisite is
    /// `Refused` with the stable code and NO object data, except
    /// authentication: a token that is unknown or disabled propagates the
    /// typed `TrustError::Authorization` refusal (HTTP 403) rather than the
    /// verdict path, while a bare session is
    /// `Refused(AUTHENTICATION_REQUIRED)`. Errors:
    /// decode/validate/storage failures propagate typed
    /// (`RootStoreError::Trust`); an absent journal is the explicit
    /// `Refused(POLICY_REQUIRED)` observed state, never a fabricated policy.
    /// Lock-free like `trust_revision`: this presentation observation
    /// deliberately takes no reservation beside a running protocol; the
    /// persisted record is validated on every read, so corrupt bytes refuse
    /// instead of passing. Limits: the attribution producer is an authorized
    /// control operation on this same root (`blp-object-attribute`, see
    /// `trust::confidentiality`); this guard re-reads the persisted record but
    /// does not re-run the producer's operator validation at consumption time,
    /// and no enforcement beyond this presentation surface is claimed anywhere
    /// (the record's `runtime_enforced` stays `false`). A later attribution
    /// produced by a different authorized actor plugs in through the same
    /// persisted record without changing this contract.
    pub fn paea_reader_verdict(
        &self,
        token: Option<&str>,
        canonical_digest: &str,
        project_path: &Path,
    ) -> Result<ReaderVerdict, R> {
        // Shared identity precheck FIRST: a bare session and an
        // unknown/disabled credential are decided on the trust journal alone,
        // before any policy or attribution work of this verdict. The
        // authenticated actor from the current validated state is carried to
        // the clearance check below, so exactly one authentication happens —
        // on the same real current bytes as before this shared observation
        // existed. The precheck grants nothing by itself.
        let (state, actor) = match self.reader_identity_precheck(token)? {
            ReaderIdentityOutcome::Refused(code) => return Ok(ReaderVerdict::Refused(code)),
            ReaderIdentityOutcome::Current(state, actor) => (state, actor),
        };
        let Some(record) = state.confidentiality.as_ref() else {
            return Ok(ReaderVerdict::Refused(POLICY_REQUIRED));
        };
        let current = record.current().map_err(failure)?;
        if current.trust_revision() != state.revision {
            // The recorded policy lost currency (identity/configuration
            // change since admission): unknown current authority refuses
            // without data, even for the highest clearance.
            return Ok(ReaderVerdict::Refused(POLICY_STALE));
        }
        // Control attribution bound to the EXACT canonical bytes, project and
        // current policy revision. Absence refuses even a top-cleared reader:
        // clearance alone never invents object classification or provenance.
        let Some((level, _attribution_actor, compartments)) =
            current.attribution_for(canonical_digest, project_path.as_os_str().as_encoded_bytes())
        else {
            return Ok(ReaderVerdict::Refused(ATTRIBUTION_REQUIRED));
        };
        // Attributed classification read under the CURRENT policy with the
        // standard non-read-up rule; an incompatible/invalid stored label
        // refuses without disclosure, never downgrades.
        let object = swarm_confidentiality::Classification {
            level: level.to_owned(),
            compartments: compartments.to_vec(),
        };
        match current.policy().check_read(&actor.subject, Some(&object)) {
            Ok(()) => Ok(ReaderVerdict::Allowed),
            Err(_) => Ok(ReaderVerdict::Refused(ACCESS_DENIED)),
        }
    }
}
