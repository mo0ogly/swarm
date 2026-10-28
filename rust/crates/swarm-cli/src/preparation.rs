//! Preparation reads/edits, explicit release and gated API-text worker admission.
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
/// Dispatch preparation reads, pure template checks and explicit draft edits.
/// List/method/template/history/budget/source/release-review/dialogue reads open Root readonly and use
/// saved limits. Template checks project data without adoption or execution.
/// Mutations require explicit JSON action/ID and retain CAS/event receipts;
/// budget input writes only its envelope, source edits only evidence, conversion
/// creates locked tasks, revision pauses and reholds them, team configuration
/// retains locks and release unblocks
/// only the reviewed conversion. No provider launches synchronously here.
/// Every path closes Root before output and preserves primary, cleanup and
/// observation causes, including later encoding/output/flush failure. Historical
/// positional syntax is retained; IDs, events, policy defaults and authority are
/// never invented by the adapter.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.get(1).is_some_and(|s|matches!(s.as_str(),"send"|"stop"|"capabilities")){return calls(a,out);}
    if a.pos.get(1).is_some_and(|s|s=="template-check"){return template_check(a,out);}
    if a.pos.get(1).is_some_and(|s|matches!(s.as_str(),"create"|"save"|"method"|"adopt-brief"|"validate-plan"|"answer-questions"|"create-missions"|"revise-missions"|"release-plan"|"configure-team"|"budget"|"source-add"|"source-remove"|"use-proposal") && (s!="budget" || !a.input.is_empty())) {return write(a,out);}
    if a.pos.len() < 2 || !a.input.is_empty() || !a.output.is_empty() || a.server || !a.task.is_empty() || a.phase != "delivery" {
        return Err(CliError::Usage);
    }
    let kind = a.pos[1].as_str();
    let before = match (kind, a.pos.len()) {
        ("list" | "methods" | "templates", 2) | ("show" | "budget" | "template" | "release-review" | "dialogue", 3) | ("history", 4) | ("source",6) | ("files",3..=5) => None,
        ("history", 5) => Some(a.pos[4].parse::<i64>().ok().filter(|v| *v > 0).ok_or(CliError::Usage)?),
        _ => return Err(CliError::Usage),
    };
    let source_lines=if kind=="source"{Some((a.pos[4].parse::<i64>().map_err(|_|CliError::Usage)?,a.pos[5].parse::<i64>().map_err(|_|CliError::Usage)?))}else{None};
    let store = RootStore::open(std::path::Path::new(&a.root), false).map_err(|e| CliError::Root(Box::new(e)))?;
    let mut causes = Vec::new();
    let result = match kind {
        "list" => bytes(store.preparation_list(), &mut causes),
        "methods" => bytes(store.preparation_methods(), &mut causes),
        "templates"=>bytes(store.preparation_templates(),&mut causes),
        "template"=>bytes(store.preparation_template(&a.pos[2]),&mut causes),
        "show" => bytes(store.preparation_show(&a.pos[2]), &mut causes),
        "release-review" => bytes(store.preparation_release_review(&a.pos[2]), &mut causes),
        "dialogue" => bytes(store.preparation_dialogue(&a.pos[2]), &mut causes),
        "budget" => bytes(store.preparation_budget(&a.pos[2]), &mut causes),
        "files" => bytes(store.preparation_files(&a.pos[2],a.pos.get(3).map_or(".",String::as_str),a.pos.get(4).map_or("",String::as_str)),&mut causes),
        "source" => {
            let (start,end)=source_lines.expect("validated source operation");
            bytes(store.preparation_source(&a.pos[2],&a.pos[3],start,end,&mut swarm_engine::SystemEnvironment),&mut causes)
        },
        "history" => bytes(store.preparation_history(&a.pos[2], &a.pos[3], before), &mut causes),
        _ => unreachable!("validated operation vocabulary"),
    };
    finish_read(store,result,out,causes)
}

/// Apply supported preparation intents through the shared Root writer.
/// Positional action/ID must match
/// the document exactly (the CLI never invents identity). Read bounded transport,
/// close before publishing and preserve the known receipt through output/flush
/// or close failures. Brief adoption, structural verdict and locked new or bound-work conversion are explicit;
/// initial human/CLI team configuration and USD envelope edits are explicit.
/// No synchronous provider call. Explicit
/// release authorizes converted tasks at the reviewed revision; admission keeps
/// its independent identity, budget and execution controls.
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

/// Close a read-only Root before publication, preserving primary, cleanup and
/// observation causes even when encoding/output/flush fails. No receipt or
/// mutation is invented; success means only that the selected DTO was delivered.
fn finish_read(store:RootStore,result:Result<Vec<u8>,CliError>,out:&mut dyn Write,causes:Vec<RootStoreError>)->Result<u8,CliError>{
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
/// Check one questionnaire through the public readonly Root contract. Require
/// explicit JSON input under the shared transport bound; no title/ID/event is
/// invented and neither completeness nor a projected graph authorizes launch.
fn template_check(a:&Arguments,out:&mut dyn Write)->Result<u8,CliError>{
    if a.pos.len()!=2 || a.input.is_empty() || !a.output.is_empty() || a.server || !a.task.is_empty() || a.phase!="delivery"{return Err(CliError::Usage);}
    let raw=if a.input=="-"{swarm_engine::read_command_document(&mut std::io::stdin().lock())}else{swarm_engine::read_command_file(std::path::Path::new(&a.input))}.map_err(CliError::CommandInput)?;
    let store=RootStore::open(std::path::Path::new(&a.root),false).map_err(|e|CliError::Root(Box::new(e)))?;
    let mut causes=Vec::new();let result=bytes(store.preparation_template_check(&raw),&mut causes);
    finish_read(store,result,out,causes)
}

/// Explicit AI send/stop and readonly capability adapter. Input identity is
/// supplied by the caller. A known admitted send remains recorded on dispatch,
/// close or output failure; replay observes the same turn without another debit.
/// Historical active replay remains observation; the native preflight fences dispatch.
fn calls(a:&Arguments,out:&mut dyn Write)->Result<u8,CliError>{
    let kind=a.pos[1].as_str();
    if a.pos.len()!=if kind=="capabilities"{2}else{3} || !a.output.is_empty() || a.server || !a.task.is_empty() || a.phase!="delivery" || (kind=="capabilities")!=a.input.is_empty(){return Err(CliError::Usage);}
    let root=RootStore::open(std::path::Path::new(&a.root),kind!="capabilities").map_err(|e|CliError::Root(Box::new(e)))?;
    let result=(||{
        if kind=="capabilities"{return json_bytes(&root.preparation_call_capabilities().map_err(|e|CliError::Root(Box::new(e)))?).map_err(CliError::Output);}
        let raw=if a.input=="-"{swarm_engine::read_command_document(&mut std::io::stdin().lock())}else{swarm_engine::read_command_file(std::path::Path::new(&a.input))}.map_err(CliError::CommandInput)?;
        let turn=if kind=="send"{let r=swarm_engine::decode_preparation_send(&raw).map_err(|e|CliError::Root(Box::new(e)))?;if r.preparation_id!=a.pos[2]{return Err(CliError::Usage);}let t=root.preparation_send(&raw,&mut swarm_engine::SystemEnvironment).map_err(|e|CliError::Root(Box::new(e)))?;if t.status=="pending" && t.supervisor_pid==0{root.preparation_dispatch_if_native(&t.id).map_err(|e|CliError::Root(Box::new(e)))?;}t}else{
            let r=swarm_engine::decode_preparation_stop(&raw).map_err(|e|CliError::Root(Box::new(e)))?;if r.preparation_id!=a.pos[2]{return Err(CliError::Usage);}root.preparation_stop(&r.preparation_id,&r.turn_id,&mut swarm_engine::SystemEnvironment).map_err(|e|CliError::Root(Box::new(e)))?
        };json_bytes(&turn).map_err(CliError::Output)
    })();finish_read(root,result,out,Vec::new())
}
