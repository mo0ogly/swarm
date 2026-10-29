#!/usr/bin/env python3
"""Verify a supplied Linux boundary driver only on fresh owned synthetic files.
A real unconfined control precedes the confined write/read/child/socket cases.
The driver is a separate fixture, never the product binary or acceptance proof.
No account, live Root, global observation, retry or whole-test watchdog is used.
"""
import argparse,hashlib,json,os,subprocess,tempfile,socket
from pathlib import Path
import rust_agent_model_route as proof


def run(driver,out):
    """Retain original channels, driver/source hashes and denied/allowed effects.
    IO, unavailable-kernel and assertion failures propagate without PASS; retain
    the owned Root for diagnosis rather than deleting it or retrying unchanged.
    """
    out.mkdir(parents=True,exist_ok=False)
    root=Path(tempfile.mkdtemp(prefix='swarm-biba-boundary-'))
    private=root/'.swarm';private.mkdir();workspace=root/'out';workspace.mkdir()
    proof.publish(private/'state.synthetic',b'OWNED_SYNTHETIC_NO_SECRET')
    proof.publish(out/'fixture.json',dict(root=str(root),driver_sha256=proof.digest(driver.read_bytes()),source_sha256=proof.digest(Path(__file__).read_bytes()),global_watchdog=None))
    script=root/'provider.py'
    source='''import os,json,socket
root=ROOT
facts={}
for name,path,flags in [('private_read',root+'/.swarm/state.synthetic',os.O_RDONLY),('private_write',root+'/.swarm/canary',os.O_WRONLY|os.O_CREAT|os.O_EXCL),('official_write',root+'/official-canary',os.O_WRONLY|os.O_CREAT|os.O_EXCL)]:
 try:fd=os.open(path,flags,0o600);os.close(fd);facts[name]=True
 except OSError:facts[name]=False
try:s=socket.socket(socket.AF_UNIX);s.close();facts['unix_socket']=True
except OSError:facts['unix_socket']=False
pid=os.fork()
if pid==0:
 try:fd=os.open(root+'/.swarm/child-canary',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600);os.close(fd);os._exit(11)
 except OSError:os._exit(0)
_,status=os.waitpid(pid,0);facts['child_write']=os.waitstatus_to_exitcode(status)==11
with open('facts.json','x') as f:json.dump(facts,f)
print('OWNED_PROVIDER_OUTPUT',flush=True)
'''.replace('ROOT',repr(str(root)))
    proof.publish(script,source.encode())
    def invoke(label,argv):
        """Keep exact argv and original bytes; an exit alone proves no assertion."""
        proof.publish(out/(label+'-argv.json'),argv)
        r=subprocess.run(argv,cwd=workspace,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        proof.publish(out/(label+'.stdout'),r.stdout);proof.publish(out/(label+'.stderr'),r.stderr)
        assert r.returncode==0,(label,r.returncode,r.stderr.decode())
        assert r.stdout==b'OWNED_PROVIDER_OUTPUT\n',label
        return json.loads((workspace/'facts.json').read_text())
    valid=invoke('unconfined-control',['/usr/bin/python3',str(script)])
    assert all(valid.values()),valid
    for path in [workspace/'facts.json',private/'canary',private/'child-canary',root/'official-canary']:path.unlink()
    observed=invoke('confined',[str(driver),str(root),str(workspace),str(script)])
    assert not any(observed.values()),observed
    assert not any(p.exists() for p in [private/'canary',private/'child-canary',root/'official-canary'])
    alias=root/'alias';alias.mkdir();os.link(private/'state.synthetic',alias/'hardlink')
    r=subprocess.run([str(driver),str(root),str(alias),str(script)],capture_output=True)
    proof.publish(out/'hardlink.stdout',r.stdout);proof.publish(out/'hardlink.stderr',r.stderr)
    assert r.returncode!=0 and b'biba-workspace-hardlink' in r.stderr
    server=socket.socket(socket.AF_UNIX,socket.SOCK_DGRAM)
    server.bind(str(private/'owned.sock'));server.setblocking(False)
    pair_script=root/'socketpair.py'
    pair_source="""import socket,errno
try:a,b=socket.socketpair(socket.AF_UNIX,socket.SOCK_DGRAM)
except OSError as e:
 assert e.errno==errno.EPERM
 print('PAIR_CREATION_DENIED',flush=True);raise SystemExit(0)
a.connect(SOCKET_PATH);a.send(b'OWNED_INERT_PACKET');a.close();b.close()
print('PAIR_PACKET_SENT',flush=True)
""".replace('SOCKET_PATH',repr(str(private/'owned.sock')))
    proof.publish(pair_script,pair_source.encode())
    try:
        valid_pair=subprocess.run(['/usr/bin/python3',str(pair_script)],capture_output=True)
        proof.publish(out/'socketpair-control.stdout',valid_pair.stdout);proof.publish(out/'socketpair-control.stderr',valid_pair.stderr)
        assert valid_pair.returncode==0 and valid_pair.stdout==b'PAIR_PACKET_SENT\n' and server.recv(100)==b'OWNED_INERT_PACKET'
        paired=subprocess.run([str(driver),str(root),str(workspace),str(pair_script)],capture_output=True)
        proof.publish(out/'socketpair.stdout',paired.stdout);proof.publish(out/'socketpair.stderr',paired.stderr)
        assert paired.returncode==0 and paired.stdout==b'PAIR_CREATION_DENIED\n'
        try:packet=server.recv(100)
        except BlockingIOError:packet=None
        assert packet is None
    finally:server.close()
    result=dict(status='PASS',scope='Native standalone boundary fixture only',valid_control=valid,confined=observed,workspace_output=True,hardlink_refused=True,socketpair_refused=True,product_qualified=False,installed=False)
    proof.publish(out/'result.json',result);print('PASS native-boundary-private-official-child-unix-and-hardlink-socketpair',flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--driver',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();run(a.driver.resolve(),a.output.resolve())
