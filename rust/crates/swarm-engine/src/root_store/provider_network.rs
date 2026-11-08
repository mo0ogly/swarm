//! Current network policy snapshot for provider transports. Data-only read;
//! no reservation, exec grant or OS restriction is created here.
use super::{absent, finished, RootStore, RootStoreError as E};
use crate::trust::state::State;

impl RootStore {
    /// Read the current operator network policy under a short Trust reservation.
    /// A journal/policy hole refuses (missing state can never become false);
    /// invalid or unsupported state refuses rather than allowing. Missing
    /// confidentiality record keeps the historical (allowed) behavior — the
    /// BLP policy itself is an explicit operator artifact, absent by default.
    /// Trust is released before any provider/SQL callback; this read carries
    /// no egress lock and serializes with policy-apply writes only. The
    /// returned boolean is a frozen transport decision, not a live authority:
    /// callers own freshness rechecks and the actual kernel pose. This is a
    /// read-only observation, not an execution grant or OS restriction.
    pub fn provider_internet_denied_snapshot(&self) -> Result<bool, E> {
        let reservation = self.directory.reserve_trust(false).map_err(E::Storage)?;
        let result = (|| {
            let state = match self.directory.read_bounded(super::trust_store::STATE, super::trust_store::MAX_STATE) {
                Ok(raw) => State::decode(&raw).map_err(|e| E::Trust(Box::new(e)))?,
                Err(error) if absent(&error) => return Ok(false),
                Err(error) => return Err(E::Storage(error)),
            };
            state.validate().map_err(|e| E::Trust(Box::new(e)))?;
            state.require_supported().map_err(|e| E::Trust(Box::new(e)))?;
            let Some(record) = state.confidentiality.as_ref() else { return Ok(false) };
            let current = record.current().map_err(|e| E::Trust(Box::new(e)))?;
            Ok(current.policy().provider_internet_denied)
        })();
        finished(result, reservation.close().map_err(E::Storage))
    }
}
