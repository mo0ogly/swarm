//! Mandatory Linux boundary for untrusted I0 provider processes.
//! This crate grants no identity or launch authority. The engine must validate
//! its private gate and bind both directories before calling it. No fallback,
//! provider/account activation, policy timeout or global-system observation.
#![cfg(target_os="linux")]
mod kernel;
use std::{fs::{self,File,OpenOptions},io,os::{fd::{AsRawFd,IntoRawFd},unix::fs::{OpenOptionsExt,MetadataExt}},path::{Path,PathBuf}};
/// Safe static failure family retaining the original IO cause, never raw paths.
#[derive(Debug)] pub struct Error {pub stage:&'static str,cause:io::Error,cleanup:Vec<Error>}
impl std::fmt::Display for Error {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str(self.stage)}}
impl std::error::Error for Error {fn source(&self)->Option<&(dyn std::error::Error+'static)>{Some(&self.cause)}}
fn err(stage:&'static str,cause:io::Error)->Error{Error{stage,cause,cleanup:Vec::new()}}
fn invalid(stage:&'static str)->Error{err(stage,io::Error::from_raw_os_error(libc::EINVAL))}
fn open(path:&Path)->Result<File,Error>{OpenOptions::new().read(true).custom_flags(libc::O_PATH|libc::O_CLOEXEC|libc::O_NOFOLLOW).open(path).map_err(|e|err("biba-path-open",e))}
/// Explicitly close an owned FD; preserve close failure rather than retrying it.
fn close(file:impl IntoRawFd)->Result<(),Error>{let fd=file.into_raw_fd();
    // SAFETY: consume exactly the owned descriptor once, never a borrowed fd.
    if unsafe{libc::close(fd)}<0{Err(err("biba-close",io::Error::last_os_error()))}else{Ok(())}
}
/// Prepared inode-bound ruleset. Construction does not change this process.
pub struct Sandbox {rules:std::os::fd::OwnedFd,workspace:PathBuf}
/// Return the observed kernel ABI, without enforcing or changing any privilege.
pub fn available()->Result<i32,Error>{let abi=kernel::abi().map_err(|e|err("biba-landlock-unavailable",e))?;if abi<6{return Err(invalid("biba-landlock-abi6-required"))}Ok(abi)}
/// Build read-only leaves covering the filesystem except the private Root.
/// Walk only its ancestor chain; no file contents are read and symlinks are not
/// followed. Rules are inode-bound. A workspace leaf can later allow I0 output.
fn read_leaves(at:&Path,private:&Path,out:&mut Vec<PathBuf>)->Result<(),Error>{
    for e in fs::read_dir(at).map_err(|e|err("biba-read-ancestors",e))? {
        let e=e.map_err(|e|err("biba-read-ancestor-entry",e))?;let p=e.path();
        if p==private{continue}
        if private.starts_with(&p){read_leaves(&p,private,out)?;}
        else if !e.file_type().map_err(|e|err("biba-read-ancestor-type",e))?.is_symlink(){out.push(p);}
    }Ok(())
}
/// Reject existing material aliases in an I0 directory. Symlinks are retained as
/// data but their targets get no rights; newly-created cross-boundary hardlinks
/// are blocked by Landlock REFER. Caller must own a fresh isolated workspace.
fn check_tree(at:&Path)->Result<(),Error>{
    for e in fs::read_dir(at).map_err(|e|err("biba-workspace-read",e))?{
        let e=e.map_err(|e|err("biba-workspace-entry",e))?;let m=e.path().symlink_metadata().map_err(|e|err("biba-workspace-metadata",e))?;
        if m.is_file()&&m.nlink()!=1{return Err(invalid("biba-workspace-hardlink"))}
        if m.is_dir(){check_tree(&e.path())?;}
    }Ok(())
}
impl Sandbox {
    /// Prepare the fixed I0 boundary around an engine-owned output directory.
    /// Root-private authority is unreadable except this exact output leaf. The
    /// output must be distinct from/not ancestor of Root and contain no hardlinks.
    /// No enforcement/exec/identity grant occurs here; all IO failures refuse.
    pub fn prepare(root:&Path,workspace:&Path)->Result<Self,Error>{
        available()?;
        let root=root.canonicalize().map_err(|e|err("biba-root",e))?;
        let workspace=workspace.canonicalize().map_err(|e|err("biba-workspace",e))?;
        if root.starts_with(&workspace)||!workspace.is_dir(){return Err(invalid("biba-workspace-overlaps-root"))}
        let private=root.join(".swarm");if private.starts_with(&workspace){return Err(invalid("biba-workspace-overlaps-private-root"))} if !private.is_dir(){return Err(invalid("biba-private-root-required"))}
        check_tree(&workspace)?;
        let rules=kernel::ruleset().map_err(|e|err("biba-ruleset",e))?;
        let result=(||{
            let mut leaves=Vec::new();read_leaves(Path::new("/"),&private,&mut leaves)?;
            for p in leaves {
                let f=open(&p)?;let is_dir=f.metadata().map_err(|e|err("biba-rule-metadata",e))?.is_dir();
                let access=if is_dir{kernel::READ}else{kernel::READ&!(1<<3)};
                let added=kernel::add(&rules,&f,access).map_err(|e|err("biba-read-rule",e));
                finish(added,close(f))?;
            }
            let f=open(&workspace)?;let added=kernel::add(&rules,&f,kernel::ALL).map_err(|e|err("biba-output-rule",e));finish(added,close(f))?;
            Ok(())
        })();
        if let Err(e)=result{return finish(Err(e),close(rules))}
        Ok(Self{rules,workspace})
    }
    /// Enter the ruleset only in a single-threaded, unprivileged helper. All
    /// subsequent descendants inherit it. Failure after restriction remains an
    /// error; the caller must exit, never execute an unconfined fallback.
    pub fn enforce(self)->Result<(),Error>{
        let result=(||{
            if fs::read_dir("/proc/self/task").map_err(|e|err("biba-thread-check",e))?.count()!=1{return Err(invalid("biba-single-thread-required"))}
            let status=fs::read_to_string("/proc/self/status").map_err(|e|err("biba-capability-check",e))?;
            if !status.lines().any(|l|l.strip_prefix("CapEff:").is_some_and(|v|v.trim()=="0000000000000000")){return Err(invalid("biba-unprivileged-helper-required"))}
            std::env::set_current_dir(&self.workspace).map_err(|e|err("biba-workspace-enter",e))?;
            kernel::restrict(&self.rules).map_err(|e|err("biba-restrict",e))?;
            kernel::seccomp().map_err(|e|err("biba-seccomp",e))
        })();
        finish(result,close(self.rules))
    }
    /// Close an unused prepared profile after another gate check refuses.
    pub fn close(self)->Result<(),Error>{close(self.rules)}
}
/// Retain both a primary failure and a cleanup error without converting to success.
fn finish<T>(value:Result<T,Error>,cleanup:Result<(),Error>)->Result<T,Error>{
    match(value,cleanup){(v,Ok(()))=>v,(Ok(_),Err(e))=>Err(e),(Err(mut e),Err(c))=>{e.cleanup.push(c);Err(e)}}
}
impl Error {
    /// Preserve the original IO and every close cause for the engine's wrapper matrix.
    pub fn into_parts(self)->(&'static str,io::Error,Vec<Error>){(self.stage,self.cause,self.cleanup)}
}
