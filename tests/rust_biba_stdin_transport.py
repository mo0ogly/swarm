#!/usr/bin/env python3
"""Discriminate closed-stdin handling using exact retained/current transport.
Compile sparse fixture adapters around the real source, not a copied repository.
Shared public error/provider types come from declared Cargo artifacts; this is
not a normal engine-profile or global acceptance proof. No provider is executed.
"""
import argparse,json,subprocess
from pathlib import Path
import rust_agent_model_route as proof

GLUE='''pub use swarm_engine::{Provider,SettingCount};
#[path="agent_launch_error.rs"]mod agent_launch_error;
pub use agent_launch_error::AgentLaunchError;
// Unexercised sparse Root error adapter; no error-continuity qualification.
#[derive(Debug)]pub enum RootStoreError{Launch(Box<AgentLaunchError>),Refused(&'static str),Cleanup{primary:Option<Box<Self>>,cleanup:Box<Self>}}
mod root_store {use super::RootStoreError as E;pub fn finished<T>(v:Result<T,E>,c:Result<(),E>)->Result<T,E>{match(v,c){(Ok(v),Ok(()))=>Ok(v),(Err(e),Ok(()))=>Err(e),(Ok(_),Err(e))=>Err(E::Cleanup{primary:None,cleanup:Box::new(e)}),(Err(p),Err(c))=>Err(E::Cleanup{primary:Some(Box::new(p)),cleanup:Box::new(c)})}}}
// Unexercised workspace adapter; entering it is a fixture failure.
mod local_storage {pub struct LocalDirectory;impl LocalDirectory{pub fn child(&self,_:&str,_:bool)->Result<Self,std::io::Error>{panic!("outside fixture scope")}pub fn close(self)->Result<(),std::io::Error>{panic!("outside fixture scope")}}}
// This empty-environment fixture never exercises environment validation.
mod agent_launch_policy {pub fn environment_name(_: &str)->bool{panic!("outside fixture scope")}}
#[path="transport.rs"]mod transport;
fn main(){transport::closed_stdin_probe()}
'''
PROBE='''
/// Exercise real prepare/attach/close, without gate, isolation or provider exec.
pub fn closed_stdin_probe(){
 use std::io::Read;
 nix::unistd::close(0).unwrap();
 let provider=crate::Provider{command:"/owned-fixture/provider".into(),..Default::default()};
 let transport=PreparedExec::prepare(&provider,&[],std::path::Path::new("/owned-fixture"),b"OWNED_PROMPT").unwrap();
 eprintln!("memfd_slot={}",std::os::fd::AsRawFd::as_raw_fd(&transport.input));
 rustix::stdio::dup2_stdin(&transport.input).unwrap();
 nix::unistd::close(transport.input).unwrap();
 let mut observed=Vec::new();
 if std::io::stdin().read_to_end(&mut observed).is_err()||observed!=b"OWNED_PROMPT" {std::process::exit(23)}
 println!("PASS closed-stdin-exact-prompt");
}
'''


def run(args):
    """Retain source hashes, Cargo artifact selection, compile and original exits.
    A failed compiler is never a behavioral refusal. Before must compile then
    lose the prompt; current must compile and read exact bytes from fd0.
    """
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    entries=[json.loads(line) for line in args.artifacts.read_bytes().splitlines() if line.startswith(b'{')]
    def artifact(name):
        matches=[v for v in entries if v.get('reason')=='compiler-artifact' and v['target']['name']==name and 'lib' in v['target']['kind']]
        assert len(matches)==1,(name,len(matches))
        files=[Path(f) for f in matches[0]['filenames'] if f.endswith('.rlib')];assert len(files)==1
        return files[0]
    externs={n:artifact(n) for n in ['swarm_engine','nix','rustix','rusqlite']}
    externs['swarm_biba_linux']=args.biba_library.resolve()
    proof.publish(out/'artifacts.json',{n:dict(path=str(p),sha256=proof.digest(p.read_bytes())) for n,p in externs.items()})
    for label,source,expected in [('before',args.before,23),('current',args.current,0)]:
        work=out/label;work.mkdir()
        raw=source.read_bytes();proof.publish(work/'source.json',dict(path=str(source),sha256=proof.digest(raw),adapter_scope='Append probe; real prepare body unchanged'))
        proof.publish(work/'transport.rs',raw+PROBE.encode());proof.publish(work/'main.rs',GLUE.encode())
        helper=Path(__file__).resolve().parents[1]/'rust/crates/swarm-engine/src/native_provider_biba.rs'
        proof.publish(work/'native_provider_biba.rs',helper.read_bytes())
        workspace=helper.with_name('native_provider_workspace.rs');proof.publish(work/'native_provider_workspace.rs',workspace.read_bytes())
        proof.publish(work/'workspace-source.json',dict(path=str(workspace),sha256=proof.digest(workspace.read_bytes()),exercised=False))
        errors=helper.with_name('agent_launch_error.rs');proof.publish(work/'agent_launch_error.rs',errors.read_bytes())
        proof.publish(work/'error-source.json',dict(path=str(errors),sha256=proof.digest(errors.read_bytes())))
        binary=work/'probe';argv=['rustc','--edition=2021',str(work/'main.rs'),'-L','dependency='+str(args.dependencies.resolve()),'-L','dependency='+str(args.biba_library.resolve().parent),'-o',str(binary)]
        for name,path in externs.items():argv+=['--extern',name+'='+str(path)]
        proof.publish(work/'compiler-argv.json',argv)
        c=subprocess.run(argv,capture_output=True)
        proof.publish(work/'compiler.stdout',c.stdout);proof.publish(work/'compiler.stderr',c.stderr)
        assert c.returncode==0,(label,'compiler',c.stderr.decode()[-1800:])
        r=subprocess.run([str(binary)],stdin=subprocess.DEVNULL,capture_output=True)
        proof.publish(work/'stdout',r.stdout);proof.publish(work/'stderr',r.stderr)
        proof.publish(work/'result.json',dict(exit=r.returncode,binary_sha256=proof.digest(binary.read_bytes())))
        assert r.returncode==expected,(label,r.returncode,r.stderr)
        if label=='before':assert b'memfd_slot=0' in r.stderr
        else:assert r.stdout==b'PASS closed-stdin-exact-prompt\n' and b'memfd_slot=0' not in r.stderr
    proof.publish(out/'result.json',dict(status='PASS',scope='Sparse real-source transport fixture, no provider exec or engine-profile qualification',before_exit=23,current_exit=0))
    print('PASS real-transport-closed-stdin-before-after',flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    for n in ['before','current','artifacts','biba-library','dependencies','output']:p.add_argument('--'+n,type=Path,required=True)
    run(p.parse_args())
