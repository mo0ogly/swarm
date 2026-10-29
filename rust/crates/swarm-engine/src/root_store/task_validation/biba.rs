//! I0 execution for already-authorized controls; no identity or acceptance grant.
use crate::{root_store::{finished,RootStoreError as E},native_provider_exec as boundary};
use std::{fs::File,path::Path,process::{Command,Stdio},os::{unix::{fs::MetadataExt,process::CommandExt}},convert::Infallible};
use swarm_biba_linux::Sandbox;
/// Retain the typed IO/kernel cause under a static safe control diagnostic.
fn cause(code:&'static str,error:impl std::error::Error+Send+Sync+'static)->E {
    E::Launch(Box::new(crate::AgentLaunchError::caused(code,error)))
}
/// Owned control transport prepared inside its verified private gate. Close it
/// explicitly if Root closure refuses. Successful exec never returns; no result
/// or task is accepted. Only an attributed managed I0 workspace is supported.
pub(super) struct Prepared { command:Command, profile:Sandbox, input:File }
impl Prepared {
    /// Bind the holder workspace and contained control cwd, restrict inherited
    /// environment to the saved launch allowlist and open null stdin before
    /// restriction. No provider program is started and no policy is invented.
    /// IO/kernel/close failures refuse with causes; no unconfined fallback exists.
    pub(super) fn prepare(root:&Path,workspace:&Path,mut command:Command,environment:&[String])->Result<Self,E>{
        let base=workspace.canonicalize().map_err(|e|cause("task-validation-i0-workspace",e))?;
        let cwd=command.get_current_dir().ok_or(E::Refused("task-validation-i0-cwd-required"))?
            .canonicalize().map_err(|e|cause("task-validation-i0-cwd",e))?;
        let relative=cwd.strip_prefix(&base).map_err(|e|cause("task-validation-i0-cwd-outside",e))?;
        command.current_dir(if relative.as_os_str().is_empty(){Path::new(".")}else{relative});
        let program=Path::new(command.get_program());
        if !program.is_absolute() && program.components().count()==1
            && (!environment.iter().any(|n|n=="PATH")||std::env::var_os("PATH").is_none()) {
            return Err(E::Refused("task-validation-i0-path-not-configured"));
        }
        command.env_clear();
        for name in environment {if let Some(value)=std::env::var_os(name){command.env(name,value);}}
        command.stdin(Stdio::inherit());
        let observed=workspace.metadata().map_err(|e|cause("task-validation-i0-workspace",e))?;
        let input=File::open("/dev/null").map_err(|e|cause("task-validation-i0-stdin",e))?;
        match boundary::prepare_boundary(root,workspace,(observed.dev(),observed.ino())) {
            Ok(profile)=>Ok(Self{command,profile,input}),
            Err(error)=>finished(Err(error),nix::unistd::close(input).map_err(|e|cause("task-validation-i0-stdin-close",e))),
        }
    }
    /// Release unused profile/stdin after another gate fails; retain both causes.
    pub(super) fn close(self)->Result<(),E>{
        finished(boundary::close_boundary(self.profile),nix::unistd::close(self.input).map_err(|e|cause("task-validation-i0-stdin-close",e)))
    }
    /// Attach empty owned stdin, enter the pinned profile, then execute only the
    /// normalized command. Descendants inherit it. Any return requires helper
    /// exit and preserves setup/kernel/exec causes; no Root operation is accepted.
    pub(super) fn execute(mut self)->Result<Infallible,E>{
        let attach=rustix::stdio::dup2_stdin(&self.input).map_err(|e|cause("task-validation-i0-stdin-attach",e));
        let attached=finished(attach,nix::unistd::close(self.input).map_err(|e|cause("task-validation-i0-stdin-close",e)));
        if let Err(error)=attached{return finished(Err(error),boundary::close_boundary(self.profile))}
        boundary::enforce_boundary(self.profile)?;
        Err(cause("task-validation-command-exec",self.command.exec()))
    }
}
