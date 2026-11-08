//! Preparation quarantine at admission and actual controlled provider egress.
//! This lock is not source attribution, an allowed classified-send capability,
//! OS confinement or confidentiality inheritance for provider responses.
use super::{absent, finished, RootStore, RootStoreError as E};
use crate::trust::state::State;

/// Preserve both operation and release failures. A successful callback can have
/// effects even when releasing its reservation fails; retain an explicit marker
/// instead of letting the cleanup error imply that nothing happened. Callers
/// owning a recorded receipt must additionally retain that receipt.
fn released<T>(operation: Result<T, E>, release: Result<(), E>) -> Result<T, E> {
    match (operation, release) {
        (Ok(_), Err(cleanup)) => Err(E::Cleanup {
            primary: Some(Box::new(E::Refused("preparation-egress-operation-completed"))),
            cleanup: Box::new(cleanup),
        }),
        (operation, release) => finished(operation, release),
    }
}

impl RootStore {
    /// Execute one preparation admission or controlled provider operation under
    /// the opened Root's shared egress reservation. Explicit current quarantine
    /// refuses BEFORE invoking the callback. Missing journal/policy and an
    /// explicit false retain legacy behavior, never claim BLP authorization.
    /// Invalid state/storage refuses without callback; it never becomes false.
    /// BLP-policy/identity mutations serialize with the callback, including IO:
    /// applying quarantine waits for an engaged operation to finish or existing stop/
    /// timeout handling to cancel it. This creates no new deadline or OS guard.
    /// Callback and release errors retain causes; callback success followed by
    /// release failure carries preparation-egress-operation-completed, so Err
    /// never proves absence of contact, state mutation or remote completion.
    pub fn with_preparation_egress<T>(&self, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        let initial = self.instance()?;
        let reservation = self.directory.reserve_preparation_egress(false).map_err(E::Storage)?;
        let result = (|| {
            self.preparation_quarantine_snapshot()?;
            if self.instance()? != initial { return Err(E::Refused("preparation-root-instance-changed")); }
            operation()
        })();
        released(result, reservation.close().map_err(E::Storage))
    }

    /// Read current quarantine under a short Trust reservation while the caller
    /// holds the egress reader. Release Trust before any provider/SQL callback;
    /// unrelated business journal writes remain possible during provider IO.
    fn preparation_quarantine_snapshot(&self) -> Result<(), E> {
        let reservation=self.directory.reserve_trust(false).map_err(E::Storage)?;
        let result=(|| {
            let state = match self.directory.read_bounded(super::trust_store::STATE, super::trust_store::MAX_STATE) {
                Ok(raw) => Some(State::decode(&raw).map_err(|e| E::Trust(Box::new(e)))?),
                Err(error) if absent(&error) => None,
                Err(error) => return Err(E::Storage(error)),
            };
            if let Some(state) = state {
                state.validate().map_err(|e| E::Trust(Box::new(e)))?;
                state.require_supported().map_err(|e| E::Trust(Box::new(e)))?;
                if let Some(record) = state.confidentiality.as_ref() {
                    let current = record.current().map_err(|e| E::Trust(Box::new(e)))?;
                    if current.policy().preparation_quarantine {
                        // A stale authority cannot silently disable its prior
                        // quarantine. Only explicit current control can lift it.
                        return Err(E::Refused("blp-preparation-quarantined"));
                    }
                }
            }
            Ok(())
        })();
        finished(result, reservation.close().map_err(E::Storage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Successful callbacks and cleanup failures are separate observed outcomes;
    /// no source text or provider secret is necessary for these typed witnesses.
    #[test]
    fn release_failure_preserves_completed_operation_and_primary_error() {
        assert_eq!(released(Ok(7), Ok(())).unwrap(), 7);
        let error = released(Ok(7), Err(E::Refused("fixture-release"))).unwrap_err();
        match error {
            E::Cleanup { primary: Some(primary), cleanup } => {
                assert_eq!(primary.code(), "preparation-egress-operation-completed");
                assert_eq!(cleanup.code(), "fixture-release");
            }
            _ => panic!("completion marker missing"),
        }
        let error = released::<()>(Err(E::Refused("fixture-operation")), Err(E::Refused("fixture-release"))).unwrap_err();
        match error {
            E::Cleanup { primary: Some(primary), cleanup } => {
                assert_eq!(primary.code(), "fixture-operation");
                assert_eq!(cleanup.code(), "fixture-release");
            }
            _ => panic!("original cause missing"),
        }
    }
}
