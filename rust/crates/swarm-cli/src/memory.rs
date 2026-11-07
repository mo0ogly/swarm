//! Versioned-memory surface: `swarm memory list` plus the mutation preview/
//! commit subcommands (Go `projectMemoryCLI` minus `summary-source`, which is
//! the M20 increment). Mutations reuse the RootStore external-mutation path —
//! the same engine transaction, replay, guards and preview-token checks as the
//! Go `mutateWithHook` route — and report a recorded Work state like the other
//! CLI mutation families. M20 summary-source is refused with usage until its
//! own increment; no wire behavior is invented here.
use crate::{Arguments, CliError, json_bytes, write_output};
use std::io::Write;
use std::path::Path;
use swarm_engine::RootStore;

/// The subcommands served by this module. `list` is read-only; commit
/// subcommands map to engine `memory.<action>` kinds; preview subcommands are
/// the read-only proposed outcome at the Work's current revision
/// (`RootStore::preview_project_memory`, Go `projectMemoryPreview`).
/// `summary-source` is deliberately absent (M20 stays unported).
const COMMIT_KINDS: &[(&str, &str)] = &[
    ("admit", "memory.admit"),
    ("replace", "memory.replace"),
    ("withdraw", "memory.withdraw"),
    ("select", "memory.select"),
];
const PREVIEW_ACTIONS: &[&str] = &["admit-preview", "replace-preview", "withdraw-preview", "select-preview"];

/// Strip the Go preview suffix: `-preview` subcommands call the same action.
fn preview_action(name: &str) -> Option<&'static str> {
    PREVIEW_ACTIONS
        .iter()
        .find(|candidate| **candidate == name)
        .map(|found| &found[..found.len() - "-preview".len()])
}

/// Run the guarded read-only catalog listing or a typed memory mutation.
/// `list` takes no input, no output file, no server route, delivery phase
/// only. Mutation subcommands require `--input` (file or `-` for stdin), open
/// a writable Root and run `RootStore::execute_external("work", kind, raw)`
/// so decode, per-action shape checks, preview-token binding, replay and
/// transaction ownership all live in the engine — this module adds no policy.
/// JSON stays a typed document; human output adds a localized heading.
/// Read/encode/close causes refuse, never empty success; a recorded Work
/// outcome before a later failure is reported as state-preserved.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let kind = a.pos.get(1).map(String::as_str).ok_or(CliError::Usage)?;
    if kind == "summary-source" {
        // M20 extractive summary source is a later increment: refuse with
        // usage instead of inventing its packet wire behavior here.
        return Err(CliError::Usage);
    }
    let mutation = match kind {
        "list" => None,
        _ => Some(
            COMMIT_KINDS
                .iter()
                .find(|(name, _)| *name == kind)
                .map(|(_, engine_kind)| *engine_kind)
                .or_else(|| preview_action(kind))
                .ok_or(CliError::Usage)?,
        ),
    };
    if mutation.is_some() {
        if a.pos.len() != 3
            || a.input.is_empty()
            || !a.output.is_empty()
            || a.server
            || !a.task.is_empty()
            || a.phase != "delivery"
        {
            return Err(CliError::Usage);
        }
    } else if a.pos.len() != 2
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root_error = |e| CliError::Root(Box::new(e));
    let mut store = RootStore::open(Path::new(&a.root), !kind.ends_with("-preview") && mutation.is_some())
        .map_err(root_error)?;
    let mut committed = None;
    let result = (|| {
        let bytes = match mutation {
            Some(engine_kind) => {
                let raw = if a.input == "-" {
                    swarm_engine::read_command_document(&mut std::io::stdin().lock())
                } else {
                    swarm_engine::read_command_file(Path::new(&a.input))
                }
                .map_err(CliError::CommandInput)?;
                if let Some(action) = preview_action(kind) {
                    // Read-only preview: the proposed outcome at the Work's
                    // current revision. No commit, no recorded state follows.
                    let preview = store
                        .preview_project_memory(&a.pos[2], &raw, action)
                        .map_err(root_error)?;
                    json_bytes(&preview).map_err(CliError::Output)?
                } else {
                    let outcome = store
                        .execute_external(
                            &a.pos[2],
                            engine_kind,
                            &raw,
                            &mut swarm_engine::SystemEnvironment,
                        )
                        .map_err(root_error)?;
                    let work = outcome.result.work;
                    committed = Some((work.id.clone(), work.revision));
                    json_bytes(&work).map_err(CliError::Output)?
                }
            }
            None => {
                json_bytes(&store.memory_catalog("").map_err(root_error)?)
                    .map_err(CliError::Output)?
            }
        };
        if a.json {
            return Ok(bytes);
        }
        // Localized human heading per surface (agent.rs convention): the
        // catalog lists memory entries; a preview shows a proposal; a commit
        // records the mutated Work state.
        let heading: &str = if mutation.is_none() {
            if a.language == "en" { "Memory catalog" } else { "Catalogue mémoire" }
        } else if preview_action(kind).is_some() {
            if a.language == "en" { "Memory preview" } else { "Aperçu mémoire" }
        } else if a.language == "en" {
            "Memory recorded"
        } else {
            "Mémoire enregistrée"
        };
        let mut human = format!("{heading}\n").into_bytes();
        human.extend(bytes);
        Ok(human)
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(root_error(error)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if let Some((work, revision)) = committed.or_else(|| primary.recorded_work()) {
            CliError::PlanningStatePreserved {
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
