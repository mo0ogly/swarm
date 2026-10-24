//! Configured normal reviewer run and explicit no-exec original recovery.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::io::{self, Write};
/// Route bounded review input to Root's one authoritative controller. Review
/// takes Work/input; recovery/retirement takes an engine review ID without argv.
/// Close/encode/flush errors preserve already recorded Work and original causes;
/// no failed command implies rollback or permission to rerun a paid process.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let action = a.pos[1].as_str();
    let run = action == "review";
    if !matches!(
        action,
        "review" | "review-reconcile" | "review-retire-stale"
    ) || run == a.input.is_empty()
    {
        return Err(CliError::Usage);
    }
    let raw = if run {
        Some(
            if a.input == "-" {
                swarm_engine::read_command_document(&mut io::stdin().lock())
            } else {
                swarm_engine::read_command_file(std::path::Path::new(&a.input))
            }
            .map_err(CliError::CommandInput)?,
        )
    } else {
        None
    };
    let root_error = |e| CliError::Root(Box::new(e));
    let store =
        swarm_engine::RootStore::open(std::path::Path::new(&a.root), true).map_err(root_error)?;
    let mut known = None;
    let result = (|| {
        let outcome = if let Some(raw) = raw {
            store.run_task_review(&a.pos[2], &raw)
        } else if action == "review-retire-stale" {
            store.retire_stale_task_review(&a.pos[2])
        } else {
            store.reconcile_task_review(&a.pos[2])
        }
        .map_err(root_error)?;
        known = Some((outcome.work.id.clone(), outcome.work.revision));
        json_bytes(&outcome).map_err(CliError::Output)
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root_error(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if let Some((work, revision)) = known.or_else(|| primary.recorded_work()) {
            CliError::TaskValidationStatePreserved {
                primary: Box::new(primary),
                work,
                revision,
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
