//! Owned-child fixture compiled against either immutable public Biba library.
//! No enforce/provider/Root DB/command exec/service. All observations remain
//! fixture evidence; a successful real close followed by injected errno is synthetic.
use std::{ffi::CString,fmt::Write as _,fs::{self,File},io::{self,Write as _},os::{fd::{AsRawFd,BorrowedFd,IntoRawFd,OwnedFd,RawFd},unix::fs::MetadataExt},path::Path};
use swarm_biba_linux::{Error,Sandbox};
const TAG:&str="anon_inode:[landlock-ruleset]";
fn invalid()->io::Error{io::Error::new(io::ErrorKind::InvalidInput,"low-fd-fixture-contract")}
/// Consume one owner with one close attempt; never retry or infer release.
fn close_owned(owned:impl IntoRawFd)->io::Result<()>{
    if unsafe{libc::close(owned.into_raw_fd())}<0{Err(io::Error::last_os_error())}else{Ok(())}
}
/// Observe only this child's ruleset-kind handles. The healthy case must prove
/// the current kernel tag and one low/high object path; no global authority claim.
fn rulesets()->io::Result<Vec<RawFd>>{
    let mut out=Vec::new();
    for entry in fs::read_dir("/proc/self/fd")?{
        let entry=entry?;let target=fs::read_link(entry.path())?;
        if target.to_str()==Some(TAG){out.push(entry.file_name().to_str().ok_or_else(invalid)?.parse().map_err(|_|invalid())?);}
    }out.sort_unstable();Ok(out)
}
/// Preserve every library branch through its supported structured handoff.
fn causes(error:Error,out:&mut Vec<(&'static str,Option<i32>)>){
    let (stage,cause,cleanup)=error.into_parts();out.push((stage,cause.raw_os_error()));
    for child in cleanup{causes(child,out);}
}
/// Append only static stage tokens and errno; never render private IO Display.
fn error_lines(out:&mut String,phase:&str,errors:Vec<(&'static str,Option<i32>)>)->io::Result<()>{
    for (stage,errno) in errors{
        if !stage.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_'){return Err(invalid())}
        writeln!(out,"cause\t{phase}\t{stage}\t{}",errno.map_or("none".into(),|n|n.to_string())).map_err(|_|invalid())?;
    }Ok(())
}
/// Resolve only fixed-signature fixture exports, not a variadic syscall hook.
unsafe fn symbol(name:&str)->io::Result<*mut libc::c_void>{
    let name=CString::new(name).map_err(|_|invalid())?;
    let value=unsafe{libc::dlsym(libc::RTLD_DEFAULT,name.as_ptr())};
    if value.is_null(){Err(invalid())}else{Ok(value)}
}
/// Validate inherited channel identity before any BorrowedFd/File adoption.
fn channel(fd:RawFd)->io::Result<(u64,u64)>{
    if fd<=2||unsafe{libc::fcntl(fd,libc::F_GETFD)}<0{return Err(invalid())}
    let mut info=std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe{libc::fstat(fd,info.as_mut_ptr())}<0{return Err(io::Error::last_os_error())}
    let info=unsafe{info.assume_init()};
    if info.st_mode&libc::S_IFMT!=libc::S_IFREG{return Err(invalid())}
    Ok((info.st_dev as u64,info.st_ino as u64))
}
/// Save this child's stdio, free0..2, prepare/close without enforcement, then
/// restore and explicitly close backups. Build the report in memory so report
/// IO cannot skip profile close. Failures keep the child incomplete/nonzero;
/// parent stderr/report retain safe primary/cleanup errno, no rollback claim.
fn run()->io::Result<()>{
    let args:Vec<_>=std::env::args().collect();if args.len()!=5{return Err(invalid())}
    let report_fd:RawFd=args[3].parse().map_err(|_|invalid())?;
    let event_fd:RawFd=args[4].parse().map_err(|_|invalid())?;
    if report_fd==event_fd||channel(report_fd)?==channel(event_fd)?{return Err(invalid())}
    let phase:unsafe extern "C" fn(i32)->i32=unsafe{std::mem::transmute(symbol("swarm_biba_close_probe_phase")?)};
    let status:unsafe extern "C" fn()->i32=unsafe{std::mem::transmute(symbol("swarm_biba_close_probe_status")?)};
    let workspace=Path::new(&args[2]);let meta=workspace.metadata()?;let expected=(meta.dev(),meta.ino());
    let mut lines=String::new();let mut errors:Vec<(&str,io::Error)>=Vec::new();let mut saved:Vec<OwnedFd>=Vec::new();
    for fd in 0..=2{
        // SAFETY: parent created this child's fresh stdio; validate it remains open.
        if unsafe{libc::fcntl(fd,libc::F_GETFD)}<0{errors.push(("save-stdio",io::Error::last_os_error()));break}
        match unsafe{BorrowedFd::borrow_raw(fd)}.try_clone_to_owned(){
            Ok(owner)=>{if owner.as_raw_fd()<=2||[report_fd,event_fd].contains(&owner.as_raw_fd()){errors.push(("backup-collision",invalid()));}saved.push(owner)},
            Err(e)=>{errors.push(("save-stdio",e));break},
        }
    }
    let mut freed=false;
    if errors.is_empty(){
        let empty=match rulesets(){Ok(fds)=>fds.is_empty(),Err(e)=>{errors.push(("initial-ruleset-scan",e));false}};
        if !empty{if errors.is_empty(){errors.push(("preexisting-ruleset",invalid()))}}
        else{
            freed=true;
            for fd in 0..=2{if unsafe{libc::close(fd)}<0{errors.push(("free-stdio",io::Error::last_os_error()))}}
        }
    }
    let operation=(||->io::Result<()>{
        if !errors.is_empty(){return Err(invalid())}
        for fd in 0..=2{if unsafe{libc::fcntl(fd,libc::F_GETFD)}!=-1||io::Error::last_os_error().raw_os_error()!=Some(libc::EBADF){return Err(invalid())}}
        lines.push_str("stdio\tfree\t0\t1\t2\n");if unsafe{phase(1)}!=0{return Err(invalid())}
        match Sandbox::prepare_bound(Path::new(&args[1]),workspace,expected){
            Ok(profile)=>{
                let observed=rulesets();
                lines.push_str("prepare\tok\n");
                if let Ok(fds)=&observed{for fd in fds{lines.push_str(&format!("retained\t{fd}\n"));}}
                let changed_phase=unsafe{phase(2)};
                // Always consume the profile before propagating observation errors.
                let closed=profile.close();
                match closed{
                    Ok(())=>lines.push_str("cleanup\tok\n"),
                    Err(error)=>{lines.push_str("cleanup\trefused\n");let mut tree=vec![];causes(error,&mut tree);error_lines(&mut lines,"cleanup",tree)?;},
                }
                let fds=observed?;if fds.len()!=1||fds[0]<3||changed_phase!=0{return Err(invalid())}
            },
            Err(error)=>{lines.push_str("prepare\trefused\n");let mut tree=vec![];causes(error,&mut tree);error_lines(&mut lines,"prepare",tree)?;},
        }
        if !rulesets()?.is_empty()||unsafe{status()}!=0{return Err(invalid())}Ok(())
    })();
    if let Err(e)=operation{errors.push(("operation",e))}
    if unsafe{phase(3)}!=0{errors.push(("restore-phase",invalid()))}
    if freed{for (fd,backup) in (0..=2).zip(saved.iter()){
        if unsafe{libc::dup2(backup.as_raw_fd(),fd)}!=fd{errors.push(("restore-stdio",io::Error::last_os_error()))}
    }}
    for backup in saved{if let Err(e)=close_owned(backup){errors.push(("backup-close",e))}}
    if unsafe{libc::close(event_fd)}<0{errors.push(("event-close",io::Error::last_os_error()))}
    for (stage,e) in &errors{writeln!(&mut lines,"driver-error\t{stage}\t{}",e.raw_os_error().map_or("none".into(),|n|n.to_string())).map_err(|_|invalid())?;}
    writeln!(&mut lines,"restored\t{}",if errors.is_empty(){"yes"}else{"no"}).map_err(|_|invalid())?;
    // SAFETY: validated dedicated inherited report fd, uniquely owned in this child.
    let mut report=unsafe{<File as std::os::fd::FromRawFd>::from_raw_fd(report_fd)};
    let written=(||->io::Result<()>{report.write_all(lines.as_bytes())?;report.flush()?;report.sync_all()})();
    let closed=close_owned(report);
    // Keep secondary report-close errno in safe stderr if report writing failed.
    if let Err(ref e)=closed{eprintln!("BIBA_LOW_FD_REPORT_CLOSE errno={:?}",e.raw_os_error());}
    written?;closed?;if !errors.is_empty(){return Err(invalid())}Ok(())
}
fn main(){if let Err(error)=run(){eprintln!("BIBA_LOW_FD_DRIVER_FAILURE errno={:?}",error.raw_os_error());std::process::exit(2)}}
