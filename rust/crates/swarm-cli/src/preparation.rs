//! Preparation reads, draft edits and explicit locked new-work conversion; no dialogue or start authority.
use crate::{json_bytes, write_output, Arguments, CliError};
use serde::Serialize;
use std::io::Write;
use swarm_engine::{PreparationReadObservation, RootStore, RootStoreError};

/// Encode the selected DTO, retaining unavailable-method causes through later
/// encoding, explicit Root close and output/flush errors. No private source text
/// is rendered as a diagnostic and no successful observation grants execution.
fn bytes<T: Serialize>(read: Result<PreparationReadObservation<T>, RootStoreError>, causes: &mut Vec<RootStoreError>) -> Result<Vec<u8>, CliError> {
    let mut observation = read.map_err(|e| CliError::Root(Box::new(e)))?;
    causes.append(&mut observation.causes);
    json_bytes(&observation.value).map_err(CliError::Output)
}
/// Dispatch create/save/method/adopt-brief/validate-plan/answer-questions/create-missions to the writer or list/methods/show/history to readonly
/// storage, using the historical positional syntax and JSON
/// envelope. Read operations open Root readonly; create/save/method/adopt-brief/validate-plan/answer-questions/create-missions use the writable
/// path below. Close before publishing and preserve every failure cause. before
/// is positive/exclusive; absence is handled by the engine, limits by persisted
/// Admin policy. Only create/save/method/adopt-brief/validate-plan/answer-questions/create-missions write drafts/history/receipts; no provider,
/// visits or launch occurs. create-missions materializes only a new work with
/// unreleased locks, using the same writer/receipt; it does not authorize start.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.get(1).is_some_and(|s|matches!(s.as_str(),"create"|"save"|"method"|"adopt-brief"|"validate-plan"|"answer-questions"|"create-missions")) {return write(a,out);}
    if a.pos.len() < 2 || !a.input.is_empty() || !a.output.is_empty() || a.server || !a.task.is_empty() || a.phase != "delivery" {
        return Err(CliError::Usage);
    }
    let kind = a.pos[1].as_str();
    let before = match (kind, a.pos.len()) {
        ("list" | "methods", 2) | ("show", 3) | ("history", 4) => None,
        ("history", 5) => Some(a.pos[4].parse::<i64>().ok().filter(|v| *v > 0).ok_or(CliError::Usage)?),
        _ => return Err(CliError::Usage),
    };
    let store = RootStore::open(std::path::Path::new(&a.root), false).map_err(|e| CliError::Root(Box::new(e)))?;
    let mut causes = Vec::new();
    let result = match kind {
        "list" => bytes(store.preparation_list(), &mut causes),
        "methods" => bytes(store.preparation_methods(), &mut causes),
        "show" => bytes(store.preparation_show(&a.pos[2]), &mut causes),
        "history" => bytes(store.preparation_history(&a.pos[2], &a.pos[3], before), &mut causes),
        _ => unreachable!("validated operation vocabulary"),
    };
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(CliError::Root(Box::new(e))),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup { primary: Box::new(primary), cleanup: Box::new(cleanup) }),
    };
    result.map_err(|mut primary| {
        for cause in causes { primary = CliError::RootCleanup { primary: Box::new(primary), cleanup: Box::new(cause) }; }
        primary
    })
}

/// Apply create/save/method/adopt-brief/validate-plan/answer-questions/create-missions via the shared Root writer. Positional action/ID must match
/// the document exactly (the CLI never invents identity). Read bounded transport,
/// close before publishing and preserve the known receipt through output/flush
/// or close failures. Brief adoption, structural verdict and locked new-work conversion are explicit;
/// no organization, existing-work update, provider or execution authority.
fn write(a:&Arguments,out:&mut dyn Write)->Result<u8,CliError>{
    use swarm_engine::{RootStoreError as E, SystemEnvironment};
    let kind=a.pos[1].as_str();
    if a.input.is_empty() || !a.output.is_empty() || a.server || !a.task.is_empty() || a.phase!="delivery" || a.pos.len()!=if kind=="create"{2}else{3} {return Err(CliError::Usage);}
    let raw=if a.input=="-" {swarm_engine::read_command_document(&mut std::io::stdin().lock())}else{swarm_engine::read_command_file(std::path::Path::new(&a.input))}.map_err(CliError::CommandInput)?;
    let request=swarm_engine::decode_preparation_command(&raw).map_err(|e|CliError::Root(Box::new(e)))?;
    if request.action!=kind || (kind!="create" && request.preparation_id!=a.pos[2]) {return Err(CliError::Usage);}
    let store=RootStore::open(std::path::Path::new(&a.root),true).map_err(|e|CliError::Root(Box::new(e)))?;
    let mut receipt=None;
    let mut causes=Vec::new();
    let result=store.preparation_command(&raw,&mut SystemEnvironment).map_err(|e|CliError::Root(Box::new(e))).and_then(|mut observation|{
        causes.append(&mut observation.causes);
        let p=observation.value;
        receipt=Some((p.id.clone(),p.revision));json_bytes(&p).map_err(CliError::Output)
    });
    let result=match(result,store.close()){
        (Ok(bytes),Ok(()))=>write_output(out,&bytes).map(|()|0),
        (Err(e),Ok(()))=>Err(e),
        (Ok(_),Err(e))=>Err(CliError::Root(Box::new(e))),
        (Err(primary),Err(cleanup))=>Err(CliError::RootCleanup{primary:Box::new(primary),cleanup:Box::new(cleanup)}),
    };
    result.map_err(|mut primary| {
        for cause in causes {primary=CliError::RootCleanup{primary:Box::new(primary),cleanup:Box::new(cause)};}
        match receipt {
        Some((id,revision))=>CliError::RootCleanup{primary:Box::new(primary),cleanup:Box::new(E::PreparationRecorded{primary:Box::new(E::Refused("preparation-response-not-delivered")),id,revision})},None=>primary
    }})
}
