//! Native task controls, reviewer evidence preview and protected receipt recovery.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::io::{self, Write};

/// Route bounded explicit requests to Root's authoritative validation contract.
/// Control/reviewer previews have no mutation or reviewer invocation grant;
/// review-check validates supplied managed JSONL against fresh held sources and
/// saved limits, without authenticating its sender, recording a review or exec;
/// validation-candidate retains a managed Git candidate outside Root locks and
/// grants no control job, review or acceptance; validate runs only the task's saved authorized controls;
/// validation-reconcile applies fresh original receipts; validation-retire explicitly
/// abandons original observations after known process completion, without a command.
/// No caller-supplied command, default timeout, retry or acceptance is introduced.
/// Encode and close Root before checked output/flush. Every known recorded Work
/// survives preparation/runtime/encoding/close/output failures with original typed
/// causes. A nonzero result never proves rollback or authorizes another process.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let action = a.pos[1].as_str();
    if !matches!(
        action,
        "review-preview" | "review-inputs" | "review-check"
            | "validation-preview"
            | "validation-candidate"
            | "validate"
            | "validation-reconcile"
            | "validation-retire"
    ) {
        return Err(CliError::Usage);
    }
    let raw = if a.input == "-" {
        swarm_engine::read_command_document(&mut io::stdin().lock())
    } else {
        swarm_engine::read_command_file(std::path::Path::new(&a.input))
    }
    .map_err(CliError::CommandInput)?;
    let root_error = |error| CliError::Root(Box::new(error));
    let mut store = swarm_engine::RootStore::open(
        std::path::Path::new(&a.root),
        !matches!(action, "validation-preview" | "review-preview" | "review-inputs" | "review-check"),
    )
    .map_err(root_error)?;
    let mut known = None;
    let result = (|| {
        if action == "review-check" {
            return json_bytes(&store.check_task_review_output(&a.pos[2], &raw).map_err(root_error)?)
                .map_err(CliError::Output);
        }
        if action == "review-inputs" {
            return json_bytes(&store.materialize_task_review(&a.pos[2], &raw).map_err(root_error)?)
                .map_err(CliError::Output);
        }
        if action == "review-preview" {
            return json_bytes(
                &store
                    .preview_task_review(&a.pos[2], &raw)
                    .map_err(root_error)?,
            )
            .map_err(CliError::Output);
        }
        if action == "validation-candidate" {
            return json_bytes(&store.prepare_task_validation_candidate(&a.pos[2], &raw)
                .map_err(root_error)?).map_err(CliError::Output);
        }
        if action == "validation-preview" {
            return json_bytes(
                &store
                    .preview_task_validation(&a.pos[2], &raw)
                    .map_err(root_error)?,
            )
            .map_err(CliError::Output);
        }
        let outcome = if action == "validate" {
            store.run_task_validation(&a.pos[2], &raw)
        } else if action == "validation-retire" {
            store.retire_task_validation(&a.pos[2], &raw)
        } else {
            store.reconcile_task_validation(&a.pos[2], &raw)
        }
        .map_err(root_error)?;
        known = Some((outcome.work.id.clone(), outcome.work.revision));
        json_bytes(&outcome).map_err(CliError::Output)
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(error)) => Err(root_error(error)),
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
