//! Mandatory Linux boundary for untrusted I0 provider processes.
//! This crate grants no identity or launch authority. The engine must validate
//! its private gate and bind both directories before calling it. No fallback,
//! provider/account activation, policy timeout or global-system observation.
#![cfg(target_os="linux")]
mod kernel;
use std::{fs::{self,File,OpenOptions},io,os::{fd::{AsRawFd,FromRawFd,IntoRawFd},unix::fs::{OpenOptionsExt,MetadataExt}},path::{Path,PathBuf}};
/// Safe static failure family retaining the original IO cause, never raw paths.
#[derive(Debug)] pub struct Error {pub stage:&'static str,cause:io::Error,cleanup:Vec<Error>}
impl std::fmt::Display for Error {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str(self.stage)}}
impl std::error::Error for Error {fn source(&self)->Option<&(dyn std::error::Error+'static)>{Some(&self.cause)}}
fn err(stage:&'static str,cause:io::Error)->Error{Error{stage,cause,cleanup:Vec::new()}}
fn invalid(stage:&'static str)->Error{err(stage,io::Error::from_raw_os_error(libc::EINVAL))}
fn open(path:&Path)->Result<File,Error>{
    let file=OpenOptions::new().read(true).custom_flags(libc::O_PATH|libc::O_CLOEXEC|libc::O_NOFOLLOW).open(path).map_err(|e|err("biba-path-open",e))?;
    // The helper's caller owns fds 0-2 and may reassign them (stdin dup2) while
    // a Sandbox lives. Owning a descriptor below 3 lets that reassignment close
    // it behind this crate's back; never accept one — repin above 3 immediately.
    if file.as_raw_fd()>=3{return Ok(file)}
    let raw=file.as_raw_fd();
    // SAFETY: raw is the owned descriptor just opened, still valid here.
    let pinned=unsafe{libc::fcntl(raw,libc::F_DUPFD_CLOEXEC,3)};
    if pinned<0{return Err(err("biba-path-repin",io::Error::last_os_error()))}
    // SAFETY: adopt exactly the duplicate produced above; it is owned and unheld.
    Ok(unsafe{File::from_raw_fd(pinned)})
    // Dropping `file` closes the original low descriptor; the duplicate is the
    // same open-file description, so the pinned inode is preserved.
}
/// Explicitly close an owned FD; preserve close failure rather than retrying it.
fn close(file:impl IntoRawFd)->Result<(),Error>{let fd=file.into_raw_fd();
    // SAFETY: consume exactly the owned descriptor once, never a borrowed fd.
    if unsafe{libc::close(fd)}<0{Err(err("biba-close",io::Error::last_os_error()))}else{Ok(())}
}
/// Prepared inode-bound ruleset. Construction does not change this process.
pub struct Sandbox {rules:std::os::fd::OwnedFd,workspace:File,private:File}
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
        let observed=workspace.metadata().map_err(|e|err("biba-workspace-metadata",e))?;
        Self::prepare_bound(root,workspace,(observed.dev(),observed.ino()))
    }
    /// Prepare using the gate's recorded workspace device/inode. This observation
    /// is not an identity or grant. Pin the exact directory before inspecting it;
    /// every later rule and cwd consumes that FD, never a re-resolved pathname.
    /// Every descriptor this crate returns is owned above 2 (`biba-path-repin`
    /// otherwise): the caller owns 0-2 and may reassign them while the Sandbox
    /// lives, which would close a low pinned FD behind its back.
    /// A replaced/symlink/unsafe directory refuses; all owned FD close causes
    /// survive. This does not freeze writes by an outside unconfined same-UID actor.
    pub fn prepare_bound(root:&Path,workspace:&Path,expected:(u64,u64))->Result<Self,Error>{
        available()?;
        let root=root.canonicalize().map_err(|e|err("biba-root",e))?;
        let path=workspace.canonicalize().map_err(|e|err("biba-workspace",e))?;
        if root.starts_with(&path){return Err(invalid("biba-workspace-overlaps-root"))}
        let private_path=root.join(".swarm");
        if private_path.starts_with(&path){return Err(invalid("biba-workspace-overlaps-private-root"))}
        let workspace=open(&path)?;
        let checked=(||{
            let m=workspace.metadata().map_err(|e|err("biba-workspace-metadata",e))?;
            if !m.is_dir()||(m.dev(),m.ino())!=expected{return Err(invalid("biba-workspace-instance-changed"))}
            check_tree(&PathBuf::from(format!("/proc/self/fd/{}",workspace.as_raw_fd())))
        })();
        if let Err(e)=checked{return finish(Err(e),close(workspace))}
        let private=match open(&private_path){Ok(f)=>f,Err(e)=>return finish(Err(e),close(workspace))};
        let checked=private.metadata().map_err(|e|err("biba-private-root-metadata",e)).and_then(|m|if m.is_dir(){Ok(())}else{Err(invalid("biba-private-root-required"))});
        if let Err(e)=checked{return finish(finish(Err(e),close(workspace)),close(private))}
        let rules=match kernel::ruleset(){Ok(r)=>r,Err(e)=>return finish(finish(Err(err("biba-ruleset",e)),close(workspace)),close(private))};
        let result=(||{
            let mut leaves=Vec::new();read_leaves(Path::new("/"),&private_path,&mut leaves)?;
            for p in leaves {
                let f=open(&p)?;let is_dir=f.metadata().map_err(|e|err("biba-rule-metadata",e))?.is_dir();
                let access=if is_dir{kernel::READ}else{kernel::READ&!(1<<3)};
                let added=kernel::add(&rules,&f,access).map_err(|e|err("biba-read-rule",e));
                finish(added,close(f))?;
            }
            kernel::add(&rules,&workspace,kernel::ALL).map_err(|e|err("biba-output-rule",e))
        })();
        if let Err(e)=result{return finish(finish(finish(Err(e),close(rules)),close(workspace)),close(private))}
        Ok(Self{rules,workspace,private})
    }
    /// Enter the ruleset only in a single-threaded, unprivileged helper. All
    /// subsequent descendants inherit it. Failure after restriction remains an
    /// error; the caller must exit, never execute an unconfined fallback.
    pub fn enforce(self)->Result<(),Error>{
        let result=(||{
            if fs::read_dir("/proc/self/task").map_err(|e|err("biba-thread-check",e))?.count()!=1{return Err(invalid("biba-single-thread-required"))}
            let status=fs::read_to_string("/proc/self/status").map_err(|e|err("biba-capability-check",e))?;
            if !status.lines().any(|l|l.strip_prefix("CapEff:").is_some_and(|v|v.trim()=="0000000000000000")){return Err(invalid("biba-unprivileged-helper-required"))}
            // Walk the retained descriptor, never a re-resolved pathname:
            // prepare_bound pinned the workspace above fd 2, so a caller-side
            // stdin dup2 cannot have reassigned it. A duplicate is defensive;
            // the retained O_PATH description is what check_tree observes.
            let pinned=unsafe{libc::fcntl(self.workspace.as_raw_fd(),libc::F_DUPFD_CLOEXEC,3)};
            if pinned<0{return Err(err("biba-workspace-repin",io::Error::last_os_error()))}
            let walked=(||{
                check_tree(&PathBuf::from(format!("/proc/self/fd/{}",pinned)))?;
                // SAFETY: duplicate of the retained O_PATH descriptor.
                if unsafe{libc::fchdir(pinned)}<0{return Err(err("biba-workspace-enter",io::Error::last_os_error()))}
                Ok(())
            })();
            // SAFETY: pinned was produced by fcntl above and not stored elsewhere.
            let closed=if unsafe{libc::close(pinned)}<0{Err(err("biba-close",io::Error::last_os_error()))}else{Ok(())};
            finish(walked,closed)?;
            kernel::restrict(&self.rules).map_err(|e|err("biba-restrict",e))?;
            kernel::seccomp().map_err(|e|err("biba-seccomp",e))
        })();
        finish(finish(finish(result,close(self.rules)),close(self.workspace)),close(self.private))
    }
    /// Close an unused prepared profile after another gate check refuses.
    pub fn close(self)->Result<(),Error>{finish(finish(close(self.rules),close(self.workspace)),close(self.private))}
}
/// Cumulative engine-posed refusal of Internet sockets for classified provider
/// helpers, installed AFTER `Sandbox::enforce`. Seccomp filters stack and can
/// only restrict: this denies socket/socketpair with EPERM when the requested
/// domain is AF_INET or AF_INET6, preserving every other domain's existing
/// verdict (AF_UNIX is already refused by the mandatory profile). This is an
/// additive operator policy surface, not a change to the mandatory profile and
/// not a network destination allowance. Load failure refuses; the caller must
/// exit, never execute an unconfined or Internet-allowed fallback.
pub fn deny_internet()->Result<(),Error>{
    kernel::deny_internet().map_err(|e|err("biba-internet-deny",e))
}
/// Retain both a primary failure and a cleanup error without converting to success.
fn finish<T>(value:Result<T,Error>,cleanup:Result<(),Error>)->Result<T,Error>{
    match(value,cleanup){(v,Ok(()))=>v,(Ok(_),Err(e))=>Err(e),(Err(mut e),Err(c))=>{e.cleanup.push(c);Err(e)}}
}
impl Error {
    /// Preserve the original IO and every close cause for the engine's wrapper matrix.
    pub fn into_parts(self)->(&'static str,io::Error,Vec<Error>){(self.stage,self.cause,self.cleanup)}
}
