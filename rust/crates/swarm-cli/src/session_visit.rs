//! Explicit per-operator consultation read/mark; never Work/task acceptance.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::{
    io::{self, Write},
    path::Path,
};
/// Validate/decode the optional native revision fence before Root IO. Show never
/// mutates; mark uses the actual local operator and clock, not request identity.
/// Keep known/uncertain Root receipts through encoding, checked Root close,
/// output/flush and human/JSON diagnostics. No provider, policy adoption, business
/// completion or unchanged retry is authorized. Caller owns the returned outcome.
/// Historical mission seen accepts no input and projects only Go's Visit fields
/// in JSON, or its translated confirmation in text. It uses the same writer and
/// retains the same auxiliary receipt after an encoding/close/output failure.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let historical = a.pos.first().is_some_and(|v| v == "mission")
        && a.pos.get(1).is_some_and(|v| v == "seen");
    if a.pos.len() != 3
        || !(historical || matches!(a.pos[1].as_str(), "show" | "mark"))
        || !swarm_identity_contract::name(a.pos[2].as_bytes())
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let mark = historical || a.pos[1] == "mark";
    if (!mark || historical) && !a.input.is_empty() {
        return Err(CliError::Usage);
    }
    let root = |e| CliError::Root(Box::new(e));
    let input = if a.input.is_empty() {
        swarm_engine::VisitMarkInput::historical()
    } else {
        let raw = if a.input == "-" {
            swarm_engine::read_command_document(&mut io::stdin().lock())
        } else {
            swarm_engine::read_command_file(Path::new(&a.input))
        }
        .map_err(CliError::CommandInput)?;
        swarm_engine::decode_visit_mark(&raw)
            .map_err(|e| root(swarm_engine::RootStoreError::Launch(Box::new(e))))?
    };
    let store = swarm_engine::RootStore::open(Path::new(&a.root), mark).map_err(root)?;
    let mut effect = None;
    let bytes = if mark {
        store
            .mark_session_visit(&a.pos[2], &input, &mut swarm_engine::SystemEnvironment)
            .map_err(root)
            .and_then(|result| {
                effect = Some(result.effect.clone());
                if historical {
                    if a.json {
                        json_bytes(&result.state.visit).map_err(CliError::Output)
                    } else {
                        Ok(if a.language == "en" {
                            b"Revision marked as seen.\n".to_vec()
                        } else {
                            "Révision marquée comme vue.\n".as_bytes().to_vec()
                        })
                    }
                } else {
                    json_bytes(&result).map_err(CliError::Output)
                }
            })
    } else {
        store
            .session_visit(&a.pos[2], &mut swarm_engine::SystemEnvironment)
            .map_err(root)
            .and_then(|v| json_bytes(&v).map_err(CliError::Output))
    };
    let result = match (bytes, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(root(error)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    match (result, effect) {
        (Err(primary), Some(effect)) => Err(CliError::AuxiliaryState {
            primary: Box::new(primary),
            effects: vec![effect],
            english: a.language == "en",
        }),
        (result, _) => result,
    }
}
