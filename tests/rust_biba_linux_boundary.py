#!/usr/bin/env python3
"""Verify a supplied Linux boundary driver only on fresh owned synthetic files.
A real unconfined control precedes the confined write/read/child/socket cases.
The driver is a separate fixture, never the product binary or acceptance proof.
No account, live Root, global observation, retry or whole-test watchdog is used.
"""
import argparse,hashlib,json,os,subprocess,tempfile,socket,sys

if sys.flags.optimize:
    raise RuntimeError('optimized Python cannot verify boundary assertions')
from fixture_area import fixture_area
from pathlib import Path
import rust_agent_model_route as proof
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools/verification'))
import native_build_outcome as outcomes


class DriverPin:
    """Observe supplied driver bytes; matching hashes establish no provenance."""
    def __init__(self,driver,expected):
        """Store path and expected SHA256 without IO or added authority."""
        self.driver=driver;self.expected=expected

    def check(self):
        """Return observed hash; propagate unreadable/changed bytes without retry."""
        actual=proof.digest(self.driver.read_bytes())
        if actual!=self.expected:raise ValueError('driver image differs from the supplied SHA256')
        return dict(driver_sha256=actual,scope='byte observation only')


def publish_outcome(path,text):
    """Pass collector JSON text as bytes to the atomic publisher; errors propagate."""
    proof.publish(path,text.encode())


def stdio_probe(driver,out,root,workspace,expected_driver_sha256=None):
    """Run the shared Git/stdio discriminator in an already owned inert Root.

    Callers supply a fresh fixture with private synthetic seed and I0 workspace,
    never a live Root. Retains driver argv and original child channels; returns
    parsed null/Git facts only after active assertions and unchanged private bytes.
    Direct streams and exit receipts precede an independent posthash attempt
    after every spawn attempt, even after process/publication errors. IO, JSON
    and assertion failures propagate; failed/missing evidence stays explicit.
    When the collector fails after known child completion with a recorded stdout
    hash, independently read/hash-check and parse those retained bytes. Group a
    read/hash/JSON failure with the original collector error and carry its exact
    native_outcome to the final consumer. Unknown completion/output is not parsed
    as a successful empty response. Parsing never authorizes PASS after an error.
    No final PASS follows a refusal. The child rejects optimized Python before probes. No build, retry,
    Internet/workspace-swap control or product authority is created here.
    """
    expected_driver_sha256=expected_driver_sha256 or proof.digest(driver.read_bytes())
    proof.publish(workspace/'native_build_outcome.py',Path(outcomes.__file__).read_bytes())
    private=root/'.swarm'
    # A nonpersistent stdio device is a technical capability, not I1 output.
    # Use the actual installed Git, with identical explicit inputs before the
    # protected-output attempt; retain both argv/channels and original bytes.
    stdio_script=root/'stdio-null-git.py'
    stdio_source="""import os,json,stat,subprocess,sys
from pathlib import Path
sys.path.insert(0,str(Path.cwd()))
import native_build_outcome as outcomes

def publish_receipt(path,text):
 '''Write a fresh fixture receipt; propagate failures without rollback claims.'''
 with path.open('x') as stream:
  stream.write(text);stream.flush();os.fsync(stream.fileno())

class GitExitMarker:
 '''Stable diagnostic label; identity distinguishes only the collector status error.'''
 def __str__(self):
  '''Render safe stable diagnostics without process bytes.'''
  return 'git-diff-status-requires-caller-validation'
 def __repr__(self):
  '''Reuse the safe marker text for exception-group formatting.'''
  return str(self)

class OwnedObservation:
 '''Mark owned fixture scope only; grant no binary/build/storage admission.'''
 def check(self):
  '''Return scope without IO, side effects, qualification or authority.'''
  return dict(scope='owned synthetic Git observation only')

def git_run(name,argv):
 '''Run Git once, retaining direct channels and receipts before assertions.

 Diff nonzero is an observed status for callers to validate, not an IO error.
 Secondary failures propagate with retained causes and publication uncertainty.
 No retry, timeout, acceptance or external admission is added.
 '''
 status_marker=GitExitMarker()
 publish_receipt(Path(name+'-argv.json'),json.dumps(argv))
 try:
  result=outcomes.execute(argv,out=Path.cwd(),name=name,admission=OwnedObservation(),
   publish=publish_receipt,digest=lambda p:__import__('hashlib').sha256(p.read_bytes()).hexdigest(),
   failure_code=status_marker,env=env)
 except BaseException as error:
  details=getattr(error,'native_outcome',None)
  issues=getattr(error,'exceptions',(error,))
  process_only=(details is not None and len(issues)==1 and type(issues[0]) is ValueError
   and len(issues[0].args)==1 and issues[0].args[0] is status_marker)
  if not process_only:
   try:publish_receipt(Path(name+'-failure.json'),json.dumps(outcomes.failure_document(error)))
   except BaseException as secondary:raise BaseExceptionGroup('Git and failure publication refused',[error,secondary])
   raise
  result=subprocess.CompletedProcess(argv,details['subprocess']['returncode'])
 result.stdout=Path(name+'.stdout').read_bytes();result.stderr=Path(name+'.stderr').read_bytes()
 return result

if sys.flags.optimize:
 raise RuntimeError('optimized Python cannot verify stdio assertions')
root=ROOT
fd=os.open('/dev/null',os.O_RDWR)
m=os.fstat(fd)
assert stat.S_ISCHR(m.st_mode) and os.major(m.st_rdev)==1 and os.minor(m.st_rdev)==3
message=b'owned nonpersistent stdio witness'
assert os.write(fd,message)==len(message)
os.close(fd)
Path('stdio-empty').write_bytes(b'')
Path('stdio-data').write_bytes(b'{"value":1}\\n')
Path('stdio-git-global').write_bytes(b'')
env=dict(PATH='/usr/bin:/bin',HOME=str(Path.cwd()),LANG='C',GIT_CONFIG_NOSYSTEM='1',GIT_CONFIG_GLOBAL=str(Path.cwd()/'stdio-git-global'))
base=['/usr/bin/git','diff','--no-index']
good=[*base,'--output=stdio-witness.patch','stdio-empty','stdio-data']
valid=git_run('git-witness',good)
assert valid.returncode==1 and b'+{"value":1}' in Path('stdio-witness.patch').read_bytes(),(valid.returncode,valid.stderr)
bad=[*base,'--output='+root+'/.swarm/state.synthetic','stdio-empty','stdio-data']
denied=git_run('git-refusal',bad)
assert denied.returncode==128 and b'Permission denied' in denied.stderr,(denied.returncode,denied.stderr)
original_patch=Path('stdio-witness.patch').read_bytes()
sentinel=b'OWNED_SENTINEL_REQUIRES_A_NEW_WRITE'
assert sentinel!=original_patch
Path('stdio-witness.patch').write_bytes(sentinel)
recovery=git_run('git-recovery',good)
assert recovery.returncode==1 and Path('stdio-witness.patch').read_bytes()==original_patch,(recovery.returncode,recovery.stderr)
print(json.dumps(dict(null_device_verified=True,python_optimization=sys.flags.optimize,git_witness=dict(argv=good,exit=valid.returncode,stderr=valid.stderr.decode()),git_refusal=dict(argv=bad,exit=denied.returncode,stderr=denied.stderr.decode()),git_environment=env,git_recovery=dict(argv=good,exit=recovery.returncode,stderr=recovery.stderr.decode(),patch_sha256=__import__('hashlib').sha256(original_patch).hexdigest()))),flush=True)
""".replace('ROOT',repr(str(root)))
    proof.publish(stdio_script,stdio_source.encode())
    null_argv=[str(driver),str(root),str(workspace),str(stdio_script)]
    proof.publish(out/'stdio-argv.json',null_argv)
    try:
        outcomes.execute(null_argv,out=out,name='stdio',admission=DriverPin(driver,expected_driver_sha256),
            publish=publish_outcome,digest=lambda path:proof.digest(path.read_bytes()),
            failure_code='stdio-driver-nonzero',cwd=workspace)
    except BaseException as collector_error:
        details=getattr(collector_error,'native_outcome',None)
        process=details.get('subprocess') if isinstance(details,dict) else None
        if not isinstance(process,dict):raise
        recorded_hash=process.get('stdout_sha256')
        known_stdout=(isinstance(recorded_hash,str) and len(recorded_hash)==64
            and all(c in '0123456789abcdef' for c in recorded_hash))
        if not (process.get('execution_attempted') is True
                and type(process.get('returncode')) is int and known_stdout):raise
        try:
            retained=(out/'stdio.stdout').read_bytes()
            if proof.digest(retained)!=recorded_hash:
                raise ValueError('retained stdio stdout differs from recorded channel hash')
            json.loads(retained)
        except BaseException as protocol_error:
            failure=BaseExceptionGroup('Stdio collector and retained-output validation failed',
                [collector_error,protocol_error])
            failure.native_outcome=details
            raise failure from None
        raise
    null_facts=json.loads((out/'stdio.stdout').read_bytes())
    assert null_facts['null_device_verified'] and null_facts['git_witness']['exit']==1 and null_facts['git_refusal']['exit']==128 and null_facts['python_optimization']==0
    assert (private/'state.synthetic').read_bytes()==b'OWNED_SYNTHETIC_NO_SECRET'
    print('PASS native-boundary-real-null-git-and-protected-output',flush=True)
    return null_facts


def run(driver,out):
    """Retain original channels, driver/source hashes and denied/allowed effects.
    IO, unavailable-kernel and assertion failures propagate without PASS; retain
    the owned Root for diagnosis rather than deleting it or retrying unchanged.
    Fixed null-device stdio is tested separately from every refused I1 operation;
    actual Git output is witnessed in I0 before its protected-output attempt.
    """
    out.mkdir(parents=True,exist_ok=False)
    root=Path(tempfile.mkdtemp(prefix='swarm-biba-boundary-', dir=fixture_area()))
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
    null_facts=stdio_probe(driver,out,root,workspace)
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
    result=dict(status='PASS',scope='Native standalone boundary fixture only',valid_control=valid,confined=observed,workspace_output=True,hardlink_refused=True,socketpair_refused=True,null_sink_device_verified=True,git_output_witness_and_refusal=True,git_output_recovery=True,stdio=null_facts,product_qualified=False,installed=False)
    proof.publish(out/'result.json',result);print('PASS native-boundary-private-official-child-unix-and-hardlink-socketpair',flush=True)


def run_git_stdio(driver,out,expected_driver_sha256):
    """Verify only Git/stdio using an externally qualified, hash-pinned driver.

    Requires a lowercase SHA256 supplied by the allocating build/review owner;
    this pin is not build provenance or permission by itself. Rejects a mismatch
    before fixture creation, then creates one inert Root and performs only the
    shared null/Git witness, protected-output refusal and fresh-write recovery.
    Attempts a driver-byte postcheck after every spawn attempt, independently
    of process and receipt failure. Keeps available channels/receipts and writes
    failure.json when possible; failed publication never implies rollback. No build, cleanup, retry,
    legacy controls, model or service. Returns only by writing a bounded PASS
    record; all IO/protocol/assertion failures propagate without acceptance.
    """
    if (not isinstance(expected_driver_sha256,str) or len(expected_driver_sha256)!=64
            or any(c not in '0123456789abcdef' for c in expected_driver_sha256)):
        raise ValueError('expected driver SHA256 must be lowercase hex')
    actual=proof.digest(driver.read_bytes())
    if actual!=expected_driver_sha256:
        raise ValueError('driver image differs from the supplied SHA256')
    out.mkdir(parents=True,exist_ok=False)
    try:
        root=Path(tempfile.mkdtemp(prefix='swarm-biba-git-stdio-',dir=fixture_area()))
        private=root/'.swarm';private.mkdir();workspace=root/'out';workspace.mkdir()
        proof.publish(private/'state.synthetic',b'OWNED_SYNTHETIC_NO_SECRET')
        proof.publish(out/'fixture.json',dict(root=str(root),driver_sha256=actual,
            source_sha256=proof.digest(Path(__file__).read_bytes()),scenario='git-stdio',
            python_optimization=sys.flags.optimize,global_watchdog=None))
        facts=stdio_probe(driver,out,root,workspace,expected_driver_sha256)
        proof.publish(out/'result.json',dict(status='PASS',scenario='git-stdio',
            scope='Targeted native boundary fixture only; not production qualification',
            driver_sha256=actual,driver_pin_scope='pre/post bytes; not build provenance',
            null_sink_device_verified=facts['null_device_verified'],
            git_output_witness_and_refusal=(facts['git_witness']['exit']==1 and facts['git_refusal']['exit']==128),
            git_output_recovery=(facts['git_recovery']['exit']==1),stdio=facts,
            python_optimization=sys.flags.optimize,product_qualified=False,installed=False))
        print('PASS pinned-driver-image-before-and-after',flush=True)
        print('BIBA_GIT_STDIO_PASS',flush=True)
    except BaseException as error:
        try:
            failure=outcomes.failure_document(error)
            failure['receipt_paths']=[str(out/name) for name in ('stdio.stdout','stdio.stderr','stdio-result.json','stdio-admission-after.json')]
            proof.publish(out/'failure.json',failure)
        except BaseException as secondary:
            raise BaseExceptionGroup('Stdio and failure publication refused',[error,secondary])
        raise



if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--driver',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--scenario',choices=['all','git-stdio'],default='all')
    p.add_argument('--driver-sha256')
    a=p.parse_args()
    if a.scenario=='git-stdio':
        if a.driver_sha256 is None:p.error('--scenario git-stdio requires --driver-sha256')
        run_git_stdio(a.driver.resolve(),a.output.resolve(),a.driver_sha256)
    else:
        if a.driver_sha256 is not None:p.error('--driver-sha256 is only supported with --scenario git-stdio')
        run(a.driver.resolve(),a.output.resolve())
