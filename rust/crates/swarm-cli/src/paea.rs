//! Read-only PAEA effect-journal reconciliation consumer. Opens two distinct
//! actual Roots (service and separate settlement ledger), runs the engine's
//! PA-09 verifier and emits its exact v1 report. No effect, mutation, authority
//! or identity use occurs; refusal codes and explicit journal states are
//! reported, never repaired or hidden.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::{io::Write, path::Path};
use swarm_engine::RootStore;

/// Dispatch the read-only `paea reconcile-effects LEDGER --root ROOT` command.
/// Both roots are opened readonly; each close error is retained as an
/// ancillary cause on the primary failure instead of being invented into a
/// separate success or dropped. The ledger positional is a directory argument,
/// never a --root substitution, so the service Root and the settlement ledger
/// cannot be confused. Business mismatches stay findings inside a successful
/// exit-0 report: the verifier reveals them, it does not rank them. Storage
/// failures are errors; no auxiliary state is written. Observation grants no
/// authority and proves no acceptance.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.pos[1] != "reconcile-effects"
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root = |e| CliError::Root(Box::new(e));
    let store = RootStore::open(Path::new(&a.root), false).map_err(root)?;
    let result = (|| {
        let ledger = RootStore::open(Path::new(&a.pos[2]), false).map_err(root);
        let (report, ledger) = match ledger {
            Ok(ledger) => {
                let report = store
                    .paea_reconcile_effect_journals(&ledger)
                    .map_err(root)
                    .and_then(|report| json_bytes(&report).map_err(CliError::Output));
                let closed = ledger.close();
                (report, closed.err())
            }
            Err(e) => (Err(e), None),
        };
        match (report, ledger) {
            (Ok(bytes), None) => Ok(bytes),
            (Err(primary), None) => Err(primary),
            // A successful report with a failed ledger close must not be
            // silently downgraded: the observation's checked-release guarantee
            // failed, so the report is withheld and the close cause retained.
            (Ok(_), Some(cleanup)) => Err(root(cleanup)),
            (Err(primary), Some(cleanup)) => Err(CliError::RootCleanup {
                primary: Box::new(primary),
                cleanup: Box::new(cleanup),
            }),
        }
    })();
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(e) => {
            let _ = store.close();
            return Err(e);
        }
    };
    let result = match (write_output(out, &bytes), store.close()) {
        (Ok(()), Ok(())) => Ok(0),
        (Err(e), Ok(())) => Err(e),
        (Ok(()), Err(e)) => Err(root(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result
}
