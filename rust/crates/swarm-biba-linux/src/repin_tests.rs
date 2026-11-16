//! Proposed transition guards only: no syscall, real FD, child or stdio mutation.
//! Not compiled/run in this source-only packet. Synthetic errno does not prove
//! an actual kernel close failure; a real isolated low-FD witness remains open.
use super::*;
use std::{cell::RefCell,os::fd::RawFd,rc::Rc};
type Events=Rc<RefCell<Vec<(&'static str,RawFd)>>>;
#[derive(Debug)]
struct Mock {raw:RawFd,events:Events,consumed:bool}
impl AsRawFd for Mock {fn as_raw_fd(&self)->RawFd{self.raw}}
impl Drop for Mock {
    /// Flag a forgotten simulated owner; never perform or infer an OS close.
    fn drop(&mut self){if !self.consumed{self.events.borrow_mut().push(("drop",self.raw));}}
}
struct Fx {events:Events,dup_errno:Option<i32>,close_errnos:Vec<Option<i32>>,closes:RefCell<usize>}
impl Fx {
    /// Own simulated values only; numbers 0-2 never identify real stdio here.
    fn new(dup_errno:Option<i32>,close_errnos:&[Option<i32>])->Self{
        Self{events:Rc::new(RefCell::new(vec![])),dup_errno,close_errnos:close_errnos.to_vec(),closes:RefCell::new(0)}
    }
    fn handle(&self,raw:RawFd)->Mock{Mock{raw,events:self.events.clone(),consumed:false}}
    /// Record dup and return a fresh simulated owner or the injected IO cause.
    fn duplicate(&self,owned:&Mock)->io::Result<Mock>{
        self.events.borrow_mut().push(("dup",owned.raw));
        match self.dup_errno{Some(errno)=>Err(io::Error::from_raw_os_error(errno)),None=>Ok(self.handle(7))}
    }
    /// Record one consumed owner's close attempt even if its injected result
    /// fails. No real FD is released, retried or inferred to have been released.
    fn close(&self,mut owned:Mock)->Result<(),Error>{
        owned.consumed=true;self.events.borrow_mut().push(("close",owned.raw));
        let index=*self.closes.borrow();*self.closes.borrow_mut()+=1;
        match self.close_errnos.get(index).copied().flatten(){Some(errno)=>Err(err("biba-close",io::Error::from_raw_os_error(errno))),None=>Ok(())}
    }
    /// Invoke the production transition with only simulated effects.
    fn repin(&self,raw:RawFd,stage:&'static str)->Result<Mock,Error>{
        repin_with(self.handle(raw),stage,|h|self.duplicate(h),|h|self.close(h))
    }
    fn observed(&self)->Vec<(&'static str,RawFd)>{self.events.borrow().clone()}
}
/// Exercise the same healthy low transition before each injected refusal.
fn witness(raw:RawFd,stage:&'static str){
    let fx=Fx::new(None,&[]);let pinned=fx.repin(raw,stage).unwrap();
    assert_eq!(pinned.as_raw_fd(),7);assert!(pinned.as_raw_fd()>=3);
    assert_eq!(fx.observed(),vec![("dup",raw),("close",raw)]);
    fx.close(pinned).unwrap();assert_eq!(fx.observed(),vec![("dup",raw),("close",raw),("close",7)]);
}
/// Traverse the supported structured handoff, not only Error::source().
fn causes(error:Error)->Vec<(&'static str,Option<i32>)>{
    let (stage,cause,cleanup)=error.into_parts();let mut out=vec![(stage,cause.raw_os_error())];
    for child in cleanup{out.extend(causes(child));}out
}
#[test]
/// Low0..2 inputs repin; an already-high owner has no duplicate/close side effect.
fn low_repin_and_high_passthrough_witnesses(){
    for low in 0..=2{witness(low,"biba-ruleset-repin");}
    let fx=Fx::new(None,&[]);let high=fx.repin(5,"biba-ruleset-repin").unwrap();
    assert_eq!(high.as_raw_fd(),5);assert!(fx.observed().is_empty());fx.close(high).unwrap();
    assert_eq!(fx.observed(),vec![("close",5)]);
}
#[test]
/// A valid control precedes injected dup+close errors; neither cause is flattened.
fn dup_refusal_and_original_close_error_keep_both_causes(){
    witness(1,"biba-ruleset-repin");
    let fx=Fx::new(Some(libc::EMFILE),&[Some(libc::EIO)]);
    assert_eq!(causes(fx.repin(1,"biba-ruleset-repin").unwrap_err()),vec![("biba-ruleset-repin",Some(libc::EMFILE)),("biba-close",Some(libc::EIO))]);
    assert_eq!(fx.observed(),vec![("dup",1),("close",1)]);
}
#[test]
/// After a valid control, both close attempts fail and both causes survive.
fn original_and_duplicate_close_errors_refuse_without_retry_or_drop(){
    witness(2,"biba-ruleset-repin");
    let fx=Fx::new(None,&[Some(libc::EIO),Some(libc::EBADF)]);
    assert_eq!(causes(fx.repin(2,"biba-ruleset-repin").unwrap_err()),vec![("biba-close",Some(libc::EIO)),("biba-close",Some(libc::EBADF))]);
    assert_eq!(fx.observed(),vec![("dup",2),("close",2),("close",7)]);
}
#[test]
/// Each refusal has a fresh control; no retry/implicit Drop occurs, then recovery.
fn each_refusal_attempts_only_its_owned_closes_and_fresh_witness_recovers(){
    witness(0,"biba-ruleset-repin");
    let refused=Fx::new(Some(libc::EMFILE),&[]);
    assert_eq!(causes(refused.repin(0,"biba-ruleset-repin").unwrap_err()),vec![("biba-ruleset-repin",Some(libc::EMFILE))]);
    assert_eq!(refused.observed(),vec![("dup",0),("close",0)]);
    witness(0,"biba-ruleset-repin");
    let close_failed=Fx::new(None,&[Some(libc::EIO),None]);
    assert_eq!(causes(close_failed.repin(0,"biba-ruleset-repin").unwrap_err()),vec![("biba-close",Some(libc::EIO))]);
    assert_eq!(close_failed.observed(),vec![("dup",0),("close",0),("close",7)]);
    witness(0,"biba-ruleset-repin");
}
#[test]
/// Path/ruleset stages retain a primary and subsequent cleanup through into_parts.
fn path_and_ruleset_stages_share_primary_cleanup_handoff(){
    for stage in ["biba-path-repin","biba-ruleset-repin"]{
        witness(1,stage);let fx=Fx::new(Some(libc::EMFILE),&[Some(libc::EIO)]);
        let result:Result<(),Error>=fx.repin(1,stage).map(|owned|fx.close(owned).unwrap());
        let combined=finish(result,Err(err("biba-close",io::Error::from_raw_os_error(libc::ENOSPC))));
        assert_eq!(causes(combined.unwrap_err()),vec![(stage,Some(libc::EMFILE)),("biba-close",Some(libc::EIO)),("biba-close",Some(libc::ENOSPC))]);
        assert_eq!(fx.observed(),vec![("dup",1),("close",1)]);
    }
}
#[test]
/// Synthetic metadata refusal still attempts close, retains both causes and recovers.
fn leaf_metadata_refusal_keeps_explicit_close_error_after_healthy_witness(){
    let healthy=Fx::new(None,&[]);
    with_close(healthy.handle(7),|h|{healthy.events.borrow_mut().push(("metadata",h.raw));Ok(())},|h|healthy.close(h)).unwrap();
    assert_eq!(healthy.observed(),vec![("metadata",7),("close",7)]);
    let failed=Fx::new(None,&[Some(libc::EIO)]);
    let result:Result<(),Error>=with_close(failed.handle(7),|h|{
        failed.events.borrow_mut().push(("metadata",h.raw));
        Err(err("biba-rule-metadata",io::Error::from_raw_os_error(libc::EACCES)))
    },|h|failed.close(h));
    assert_eq!(causes(result.unwrap_err()),vec![("biba-rule-metadata",Some(libc::EACCES)),("biba-close",Some(libc::EIO))]);
    assert_eq!(failed.observed(),vec![("metadata",7),("close",7)]);
    let recovered=Fx::new(None,&[]);with_close(recovered.handle(7),|_|Ok(()),|h|recovered.close(h)).unwrap();
    assert_eq!(recovered.observed(),vec![("close",7)]);
}
