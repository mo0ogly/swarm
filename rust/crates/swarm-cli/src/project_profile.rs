//! Explicit project-profile metadata, candidate preview and configuration CAS.
use crate::{Arguments,CliError,json_bytes,write_output};
use std::{io::Write,path::Path};
use swarm_engine::RootStore;

/// Run guarded show/check/list or explicit preview/apply/select. Inputs pass the
/// shared command transport and authoritative Root projection; no candidate,
/// digest or permission is invented. JSON remains a typed document; human output
/// adds a localized heading. Known configuration survives encoding/close/flush
/// failure and must be reread before another CAS. No source prompt or agent runs.
pub(super) fn execute(a:&Arguments,out:&mut dyn Write)->Result<u8,CliError>{
    let kind=a.pos.get(1).map(String::as_str).ok_or(CliError::Usage)?;
    let input=matches!(kind,"preview"|"apply"|"select");let writing=matches!(kind,"apply"|"select");
    if !matches!(kind,"show"|"check"|"list"|"preview"|"apply"|"select") || a.pos.len()!=if kind=="select"{3}else{2} || input==a.input.is_empty() || !a.output.is_empty() || a.server || !a.task.is_empty() || a.phase!="delivery"{return Err(CliError::Usage);}
    let raw=if input {Some(if a.input=="-"{swarm_engine::read_command_document(&mut std::io::stdin().lock())}else{swarm_engine::read_command_file(Path::new(&a.input))}.map_err(CliError::CommandInput)?)}else{None};
    let root_error=|e|CliError::Root(Box::new(e));
    let store=RootStore::open(Path::new(&a.root),writing).map_err(root_error)?;
    let mut saved=false;
    let result=(||{
        let bytes=match kind {
            "list"=>json_bytes(&store.project_profile_catalog().map_err(root_error)?),
            "preview"=>json_bytes(&store.project_profile_preview(raw.as_deref().ok_or(CliError::Usage)?).map_err(root_error)?),
            "apply"|"select"=>{
                let value=if kind=="apply"{store.project_profile_apply(raw.as_deref().ok_or(CliError::Usage)?)}else{store.project_profile_select(&a.pos[2],raw.as_deref().ok_or(CliError::Usage)?)}.map_err(root_error)?;
                saved=true;json_bytes(&value)
            },
            _=>json_bytes(&store.project_profile_status().map_err(root_error)?),
        }.map_err(CliError::Output)?;
        if a.json {return Ok(bytes);}
        let mut human=if a.language=="en"{b"Project profile\n".to_vec()}else{b"Consignes du projet\n".to_vec()};human.extend(bytes);Ok(human)
    })();
    let result=match(result,store.close()){
        (Ok(bytes),Ok(()))=>write_output(out,&bytes),
        (Err(error),Ok(()))=>Err(error),
        (Ok(_),Err(error))=>Err(CliError::Root(Box::new(error))),
        (Err(primary),Err(cleanup))=>Err(CliError::RootCleanup{primary:Box::new(primary),cleanup:Box::new(cleanup)}),
    };
    result.map(|()|0).map_err(|primary|if saved{CliError::ConfigurationStatePreserved{primary:Box::new(primary),english:a.language=="en"}}else{primary})
}
