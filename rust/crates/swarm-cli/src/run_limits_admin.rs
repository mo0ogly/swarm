//! Scope administration delegates all semantics/SQL to the shared Root boundary.
use crate::{Arguments,CliError,json_bytes,write_output};
use std::io::{self,Write};
/// Convert only the Go CLI's explicit empty placeholder, never a scope default.
fn empty(value:&str)->&str{if value=="-"{""}else{value}}
/// Read exactly the supported command input framing, retaining original errors.
fn input(a:&Arguments)->Result<Vec<u8>,CliError>{
    if a.input.is_empty(){return Err(CliError::Usage);}
    if a.input=="-"{swarm_engine::read_command_document(&mut io::stdin().lock())}
    else{swarm_engine::read_command_file(std::path::Path::new(&a.input))}.map_err(CliError::CommandInput)
}
/// Positional scope must agree with any nonempty JSON declaration, as in Go.
/// Root still owns scope syntax, schema, current policy, revision and token.
fn bind(scope:&mut String,mission:&mut String,key:&mut String,pos:&[String])->Result<(),CliError>{
    let expected=[pos[2].as_str(),empty(&pos[3]),empty(&pos[4])];
    if [scope.as_str(),mission.as_str(),key.as_str()].iter().zip(expected).any(|(actual,wanted)|!actual.is_empty()&&*actual!=wanted){return Err(CliError::Usage);}
    *scope=expected[0].into();*mission=expected[1].into();*key=expected[2].into();Ok(())
}
/// Read/effective/preview/apply/history/rollback on the configured Root only.
/// Explicit revision-CAS or inspected preview, exact replay returns current entry. No frozen
/// Agent/counter/provider is changed. Known/uncertain SQL effects survive
/// JSON encoding, checked Root close and output/flush failures.
pub(super) fn execute(a:&Arguments,out:&mut dyn Write)->Result<u8,CliError>{
    if a.pos.len()<2 || !a.output.is_empty() || a.server || !a.task.is_empty() || a.phase!="delivery"{return Err(CliError::Usage);}
    let op=a.pos[1].as_str();
    if !matches!(op,"show"|"history"|"effective"|"preview"|"apply"|"rollback-preview"|"rollback"){return Err(CliError::Usage);}
    let rollback=matches!(op,"rollback"|"rollback-preview");let writing=matches!(op,"apply"|"rollback");let readonly=matches!(op,"show"|"history"|"effective");
    if a.pos.len()!=if rollback{6}else{5} || (readonly&&!a.input.is_empty()){return Err(CliError::Usage);}
    let root_error=|e|CliError::Root(Box::new(e));
    let mut change=None;let mut restore=None;
    if !readonly{
        let raw=input(a)?;
        if rollback{
            let mut r=swarm_engine::decode_api_run_limits_rollback(&raw).map_err(|e|root_error(swarm_engine::RootStoreError::Launch(Box::new(e))))?;
            bind(&mut r.scope,&mut r.mission,&mut r.key,&a.pos)?;
            let target=a.pos[5].parse::<i64>().map_err(|e|root_error(swarm_engine::RootStoreError::Launch(Box::new(swarm_engine::AgentLaunchError::Cause{code:"run-limits-target-invalid",cause:Box::new(e)}))))?;
            if r.to_revision!=0 && r.to_revision!=target{return Err(CliError::Usage);}r.to_revision=target;restore=Some(r);
        }else{
            let mut r=swarm_engine::decode_api_run_limits_config_change(&raw).map_err(|e|root_error(swarm_engine::RootStoreError::Launch(Box::new(e))))?;
            bind(&mut r.scope,&mut r.mission,&mut r.key,&a.pos)?;change=Some(r);
        }
    }
    let mut store=swarm_engine::RootStore::open(std::path::Path::new(&a.root),writing).map_err(root_error)?;let mut known=false;
    let result=(||{
        let value=match op{
            "show"=>json_bytes(&store.run_limits_entry(&a.pos[2],empty(&a.pos[3]),empty(&a.pos[4])).map_err(root_error)?),
            "history"=>json_bytes(&store.run_limits_history(&a.pos[2],empty(&a.pos[3]),empty(&a.pos[4])).map_err(root_error)?),
            "effective"=>json_bytes(&store.run_limits_effective(empty(&a.pos[2]),empty(&a.pos[3]),empty(&a.pos[4])).map_err(root_error)?),
            "preview"=>json_bytes(&store.preview_run_limits(change.as_ref().ok_or(CliError::Usage)?).map_err(root_error)?),
            "apply"=>{let view=store.apply_run_limits(change.as_ref().ok_or(CliError::Usage)?,&mut swarm_engine::SystemEnvironment).map_err(root_error)?;known=view.applied;json_bytes(&view.entry)},
            "rollback-preview"=>{let r=restore.as_ref().ok_or(CliError::Usage)?;json_bytes(&store.preview_run_limits_rollback(&r.as_change(),r.to_revision).map_err(root_error)?)},
            "rollback"=>{let r=restore.as_ref().ok_or(CliError::Usage)?;let view=store.rollback_run_limits(&r.as_change(),r.to_revision,&mut swarm_engine::SystemEnvironment).map_err(root_error)?;known=view.applied;json_bytes(&view.entry)},
            _=>return Err(CliError::Usage)
        };value.map_err(CliError::Output)
    })();
    let result=match(result,store.close()){(Ok(bytes),Ok(()))=>write_output(out,&bytes),(Err(error),Ok(()))=>Err(error),(Ok(_),Err(error))=>Err(root_error(error)),(Err(primary),Err(cleanup))=>Err(CliError::RootCleanup{primary:Box::new(primary),cleanup:Box::new(cleanup)})};
    result.map(|()|0).map_err(|primary|if known{CliError::ConfigurationStatePreserved{primary:Box::new(primary),english:a.language=="en"}}else{primary})
}
