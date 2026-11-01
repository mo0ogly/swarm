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

impl RootStore {
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
        // Session alone: no identity behind the request. Refuse before any
        // object lookup, without revealing whether a projection exists.
        let Some(token) = token else {
            return Ok(ReaderVerdict::Refused(AUTHENTICATION_REQUIRED));
        };
        let raw = match self.directory.read_bounded(STATE, MAX_STATE) {
            Ok(raw) => raw,
            // Absent journal: trust never configured on this root, so no
            // policy and no attribution can exist. Explicit refusal without
            // any fabricated policy or disclosure.
            Err(e) if absent(&e) => return Ok(ReaderVerdict::Refused(POLICY_REQUIRED)),
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
            .map_err(|_| R::Trust(Box::new(E::Authorization)))?;
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
