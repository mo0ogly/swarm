"""Verify both orders of native Web preparation stop and candidate publication.

Use a supplied qualified executable, a fresh private local Root and an inert
CLI double. The double emits both complete stdout events and marks completed writes before it can exit.
The harness independently observes the worker socket draining. The first turn completes before stop.
The second accepts stop while the double is live with those events drained.
Keep public HTTP/CLI state, ordering receipts and owned process observations.
"""
from contextlib import closing
import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import select
import stat
import subprocess
import sys
import time
import traceback

from rust_agent_model_route import publish
from rust_native_http_cli import Server

REPO = Path(__file__).resolve().parent.parent
ARGS = argparse.ArgumentParser(description=__doc__)
ARGS.add_argument('--binary',type=Path,required=True)
ARGS.add_argument('--binary-sha256',required=True)
ARGS.add_argument('--output',type=Path,required=True)
ARGS.add_argument('--minimum-free-bytes',type=int,required=True)
CONFIG = ARGS.parse_args()
if not __debug__:
    raise RuntimeError('Assertions must be enabled for the recipe')
BINARY = CONFIG.binary.resolve(strict=True)
SHA = CONFIG.binary_sha256
RESERVE = CONFIG.minimum_free_bytes
OUT = CONFIG.output.absolute()
ROOT = OUT / 'root'
# Historical preparation fixtures occupied up to 4 MB. Admission allows 8 MiB
# for the small Root and the two flows' evidence in this bounded fixture.
PROJECTED = 8 * 1024 * 1024
VERDICTS = dict(result_before_stop='NOT TESTED', stop_after_drain='NOT TESTED')
EVENTS = []
REQUESTS = []
CLI = []
BIRTHS = []
ACTIVE = {}


def digest(path):
    """Hash the complete file without writing to it."""
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def available():
    """Use retained local output device's nonprivileged free-space count."""
    fs = os.fstatvfs(OUT_FD)
    host = os.statvfs("/")
    return min(fs.f_bavail * fs.f_frsize, host.f_bavail * host.f_frsize)


def event(phase, **values):
    """Retain chronological, safe facts and enforce the output reserve."""
    free = available()
    assert free >= RESERVE, (phase, free, RESERVE)
    EVENTS.append(dict(phase=phase, monotonic_ns=time.monotonic_ns(), unix_ns=time.time_ns(),
                       available_bytes=free, **values))
    publish(OUT / 'events.json', EVENTS)
    if phase != 'http':
        print(phase, flush=True)


def cli(*args, body=None, sensitive=False):
    """Use public CLI on the owned Root and retain invocation and safe streams."""
    assert available() >= RESERVE
    n = len(CLI)
    argv = [str(BINARY), *args, '--root', str(ROOT), '--json']
    wire = None
    if body is not None:
        wire = json.dumps(body).encode()
        argv += ['--input', '-']
        if not sensitive:
            publish(OUT / f'cli-{n:02}-input.json', wire)
    c = subprocess.run(argv, input=wire, capture_output=True, env=ENV)
    CLI.append(dict(argv=argv, exit=c.returncode, sensitive_projection=sensitive))
    publish(OUT / 'cli.json', CLI)
    publish(OUT / f'cli-{n:02}-stderr.raw', c.stderr)
    if not sensitive:
        publish(OUT / f'cli-{n:02}-stdout.raw', c.stdout)
    assert c.returncode == 0, (args, c.returncode, c.stderr.decode(errors='replace'))
    return json.loads(c.stdout)


def request(web, path, body=None, csrf=None, session=False, retain=True):
    """Use public HTTP, keeping raw useful bodies and no session/CSRF secrets."""
    assert available() >= RESERVE
    headers = {'Origin': 'http://' + web.address, 'Cookie': web.cookie}
    if csrf is not None:
        headers['X-Swarm-CSRF'] = csrf
    method = 'GET' if body is None else 'POST'
    data = None if body is None else json.dumps(body).encode()
    if data is not None:
        headers['Content-Type'] = 'application/json'
    with closing(http.client.HTTPConnection(web.host, web.port)) as c:
        c.request(method, path, data, headers)
        res = c.getresponse()
        raw = res.read()
    frame = web.record()
    assert frame['event'] == 'web-outcome' and frame['outcome']['status'] == res.status
    if retain or res.status != 200:
        n = len(REQUESTS)
        row = dict(method=method, path=path, status=res.status, observer=frame['event'])
        if not session or res.status != 200:
            row['response'] = f'http-{n:02}-response.raw'
            assert web.key.encode() not in raw
            assert csrf is None or csrf.encode() not in raw
            publish(OUT / row['response'], raw)
        if data is not None:
            row['request'] = f'http-{n:02}-request.raw'
            publish(OUT / row['request'], data)
        REQUESTS.append(row)
        publish(OUT / 'http.json', REQUESTS)
        event('http', method=method, path=path, status=res.status, request_index=n)
    assert res.status == 200, (path, res.status)
    return json.loads(raw), raw


def locate(web, turn_id, label, mode):
    """Observe public state transitions while waiting for a held provider or terminal state."""
    previous = None
    while True:
        turns, raw = request(web, DIALOGUE, retain=False)
        matches = [t for t in turns if t['id'] == turn_id]
        assert len(matches) == 1, matches
        turn = matches[0]
        status = turn['status']
        if status != previous:
            event(label + '-state', status=status, id=turn_id)
            publish(OUT / f'{label}-{status}.raw', raw)
            previous = status
        if status not in ('pending', 'running', 'stopping'):
            assert mode == 'terminal', ('provider did not reach barrier', label, status)
            return turn
        if mode == 'held' and status == 'running':
            markers = list((ROOT / 'preparation-cli').glob('*/ready'))
            if markers:
                assert len(markers) == 1
                return turn, markers[0]
        time.sleep(0.10)  # Observation cadence, no stop rule.


def process(pid):
    """Bind an observed PID to its Linux start time, parent and executable."""
    fields = (Path('/proc') / str(pid) / 'stat').read_text().rsplit(') ', 1)[1].split()
    return dict(pid=pid, start_ticks=int(fields[19]), ppid=int(fields[1]), state=fields[0],
                exe=os.readlink(f'/proc/{pid}/exe'), cwd=os.readlink(f'/proc/{pid}/cwd'))


def identify(label, turn, marker):
    """Count exactly one ready witness; retain pidfds before allowing any release."""
    record = json.loads(marker.read_bytes())
    provider = process(record['pid'])
    worker = process(turn['supervisor_pid'])
    assert not turn.get('answer') and turn['status'] == 'running'
    assert provider['ppid'] == worker['pid'] and provider['cwd'] == str(marker.parent)
    assert provider['exe'] == str(Path('/usr/bin/python3').resolve())
    assert worker['exe'] == str(BINARY) and record['cwd'] == provider['cwd']
    fds = [os.pidfd_open(p['pid']) for p in (provider, worker)]
    assert [process(p['pid'])['start_ticks'] for p in (provider,worker)] == [
        p['start_ticks'] for p in (provider,worker)]
    BIRTHS.append(dict(case=label, ordinal=len(BIRTHS)+1, marker=record,
                       provider=provider, worker=worker))
    publish(OUT / 'provider-launches.json', BIRTHS)
    event(label + '-provider-held', count=len(BIRTHS), **provider)
    return fds, provider, worker


def end_processes(label, fds, provider, worker):
    """Observe kernel termination and removal of the two owned process identities."""
    try:
        for fd, p in zip(fds, (provider, worker)):
            poller = select.poll()
            poller.register(fd, select.POLLIN)
            ready = poller.poll()
            assert ready and ready[0][1] & select.POLLIN, ready
            while Path(f"/proc/{p['pid']}/stat").exists():
                try:
                    fields = Path(f"/proc/{p['pid']}/stat").read_text().rsplit(') ', 1)[1].split()
                except FileNotFoundError:
                    break
                if int(fields[19]) != p['start_ticks']:
                    break
                time.sleep(0.02)
            event(label + '-process-ended', pid=p['pid'], start_ticks=p['start_ticks'],
                  pidfd_flags=ready[0][1], original_identity_gone=True)
        assert not Path(provider['cwd']).exists(), provider['cwd']
    finally:
        for fd in fds:
            os.close(fd)


def brief_state(preparation):
    """Compare adopted data and document revision independently of the turn."""
    assert preparation.get('adopted_brief', {}).get('text') == 'Brief already adopted'
    return {name: preparation[name] for name in ('revision', 'documents', 'adopted_brief')}


PROGRAM = '''#!/usr/bin/python3
import hashlib, json, os, stat, sys, time
from pathlib import Path

root = Path(__ROOT__)
prompt = sys.stdin.buffer.read()
assert stat.S_ISSOCK(os.fstat(1).st_mode)
content = json.dumps(dict(message='Inert D11 candidate ' + str(os.getpid()), brief='Candidate only'))
frames = [dict(type='item.completed', item=dict(type='agent_message', text=content)),
          dict(type='turn.completed', usage=dict(input_tokens=7, output_tokens=8))]
wire = (chr(10).join(json.dumps(frame) for frame in frames) + chr(10)).encode()
sys.stdout.buffer.write(wire)
sys.stdout.buffer.flush()
witness = dict(pid=os.getpid(), cwd=os.getcwd(), stdin_sha256=hashlib.sha256(prompt).hexdigest(),
               stdout_hex=wire.hex(), stdout_sha256=hashlib.sha256(wire).hexdigest(),
               stdout_bytes=len(wire))
Path('ready.new').write_text(json.dumps(witness))
os.replace('ready.new','ready')
while not (root / 'release').is_file():
    time.sleep(0.02)
'''


def drained_stdout(provider,worker):
    """Wait for the two owned AF_UNIX stream endpoints to report empty queues."""
    sender_link = os.readlink(f"/proc/{provider['pid']}/fd/1")
    assert sender_link.startswith('socket:[') and sender_link.endswith(']')
    sender_inode = sender_link[8:-1]
    worker_fds = Path(f"/proc/{worker['pid']}/fd")
    while True:
        assert process(provider['pid'])['start_ticks']==provider['start_ticks']
        assert process(worker['pid'])['start_ticks']==worker['start_ticks']
        rows = [line.split() for line in subprocess.check_output(
            ['/usr/bin/ss','-xanH'],env=ENV,text=True).splitlines()]
        senders = [r for r in rows if len(r)==8 and r[5]==sender_inode]
        assert len(senders)==1 and senders[0][:2]==['u_str','ESTAB'],senders
        sender = senders[0]
        receivers = [r for r in rows if len(r)==8 and r[5]==sender[7] and r[7]==sender_inode]
        assert len(receivers)==1 and receivers[0][:2]==['u_str','ESTAB'],receivers
        receiver = receivers[0]
        links = []
        for descriptor in worker_fds.iterdir():
            try:
                links.append(os.readlink(descriptor))
            except FileNotFoundError:
                continue
        assert f"socket:[{receiver[5]}]" in links
        if int(sender[3])==int(receiver[2])==0:
            return dict(provider_inode=sender_inode,worker_inode=receiver[5],
                        provider_send_q=int(sender[3]),worker_recv_q=int(receiver[2]))
        time.sleep(0.02)


def admitted_output():
    """Allocate a fresh leaf under an existing owned private output parent."""
    parent = OUT.parent.resolve(strict=True)
    assert OUT.parent==parent and RESERVE>0
    assert not OUT.exists() and not OUT.is_symlink()
    identity = parent.stat()
    assert identity.st_uid==os.geteuid() and identity.st_mode&0o077==0
    for path in (parent,Path('/')):
        fs = os.statvfs(path)
        assert fs.f_bavail*fs.f_frsize >= RESERVE+PROJECTED
    OUT.mkdir(mode=0o700)

def run():
    """Admit the exact image, seed the owned fixture, and execute the two cases in order."""
    global DIALOGUE
    image = os.fstat(EXE_FD)
    assert image.st_uid == os.geteuid() and stat.S_IMODE(image.st_mode) == 0o700
    assert digest(Path(f'/proc/self/fd/{EXE_FD}')) == SHA
    assert (BINARY.stat().st_dev, BINARY.stat().st_ino) == (image.st_dev, image.st_ino)
    assert available() >= RESERVE + PROJECTED
    publish(OUT/'provenance.json',dict(binary=str(BINARY),sha256=SHA,
        device=image.st_dev,inode=image.st_ino,uid=image.st_uid,
        driver_sha256=digest(Path(__file__)), ss_sha256=digest(Path('/usr/bin/ss')), interpreter_sha256=digest(Path('/usr/bin/python3').resolve()),
        reserve_bytes=RESERVE,projected_growth_bytes=PROJECTED,environment=ENV,
        argv=[sys.executable,*sys.argv],cwd=os.getcwd(),kernel=list(os.uname()),global_watchdog=None))
    event('preflight')
    ROOT.mkdir(mode=0o700)
    (ROOT / 'home').mkdir(mode=0o700)
    (ROOT / 'codex-home').mkdir(mode=0o700)
    provider = PROGRAM.replace('__ROOT__', repr(str(ROOT))).encode()
    publish(OUT / 'inert-provider.py', provider)
    publish(ROOT / 'codex', provider)
    (ROOT / 'codex').chmod(0o700)
    publish(ROOT / 'codex-home/models_cache.json', dict(models=[
        dict(slug='fixture-standard', visibility='list', supported_reasoning_levels=[])]))
    cli('init')
    publish(ROOT / '.swarm/providers.json', dict(schema_version=1, providers={'fixture':dict(
        command=str(ROOT/'codex'),args=['--model','fixture-standard'],
        model_policy=dict(version=1,billing='external',work_level='standard',page_level='simple',
          levels={k:dict(model='fixture-standard',effort='') for k in ('simple','standard','exigeant')}))}))
    s = cli('engine-settings','show')
    values = dict(s['values'], preparation_write_enabled='1', preparation_call_enabled='1',
                  agent_allow_unverified_provider_context='1',agent_environment_allow=['PATH','HOME','CODEX_HOME'])
    req = dict(schema_version=s['schema_version'],event_id='settings-d11',expected_revision=s['revision'],
               values=values,reason='Owned D11 fixture',preview_token='')
    preview = cli('engine-settings','preview',body=req,sensitive=True)
    cli('engine-settings','apply',body=dict(req,preview_token=preview['preview_token']),sensitive=True)
    publish(OUT / 'settings-input-without-token.json', req)
    assert cli('engine-settings','show')['values'] == values
    p = cli('prepare','create',body=dict(version=1,action='create',event_id='create-d11',
        expected_revision=0,title='D11 owned fixture',text='Fixture need'))
    pid = p['id']
    def change(action, **extra):
        return cli('prepare',action,pid,body=dict(version=1,action=action,event_id='d11-'+action,
                          preparation_id=pid,expected_revision=p['revision'],**extra))
    p = change('budget',budget=dict(limit_usd=2,reserve_per_launch_usd=.5,
                                    estimate_source='Synthetic fixture',reference_date='2026-11-19'))
    p = change('save',document='brief',text='Brief already adopted')
    p = change('adopt-brief',sha256=p['documents']['brief']['sha256'])
    DIALOGUE = '/api/v1/preparations/dialogue?id=' + pid
    show = '/api/v1/preparations/show?id=' + pid
    web = Server(BINARY, ROOT, OUT, 'owned-web', environment=ENV)
    try:
        observed = Path(f'/proc/{web.process.pid}/exe')
        assert observed.resolve() == BINARY and digest(observed) == SHA
        event('web-binary-verified', sha256=SHA, pid=web.process.pid)
        csrf = request(web,'/api/v1/native-session',session=True)[0]['csrf_token']
        cap = next(c for c in request(web,'/api/v1/preparations/capabilities')[0]['providers']
                   if c['name'] == 'fixture')
        before = brief_state(request(web,show)[0])
        assert request(web,DIALOGUE)[0] == []

        def held_turn(label):
            body = dict(version=1,preparation_id=pid,event_id='d11-order-'+label,
                        expected_revision=p['revision'],provider='fixture',
                        capability_hash=cap['digest'],message='Owned D11 '+label,context_mode='full')
            reply,_ = request(web,'/api/v1/preparations/send',body,csrf)
            assert reply['status']=='pending' and not reply.get('answer')
            ACTIVE[label]=reply['id']
            running,marker=locate(web,reply['id'],label+'-start','held')
            original=json.loads(marker.read_bytes())
            wire=bytes.fromhex(original['stdout_hex'])
            frames=[json.loads(line) for line in wire.splitlines()]
            assert [frame['type'] for frame in frames]==['item.completed','turn.completed']
            assert original['stdout_bytes']==len(wire)
            assert original['stdout_sha256']==hashlib.sha256(wire).hexdigest()
            assert not (ROOT/'release').exists()
            publish(OUT/(label+'-provider-stdout.raw'),wire)
            evidence=identify(label,running,marker)
            queues=drained_stdout(evidence[1],evidence[2])
            event(label+'-output-drained',id=reply['id'],bytes=len(wire),**queues)
            return reply['id'],evidence

        def stop(turn_id):
            return request(web,'/api/v1/preparations/stop',dict(preparation_id=pid,turn_id=turn_id),csrf)[0]

        first='result-before-stop'
        VERDICTS['result_before_stop']='FAIL'
        turn_id,witness=held_turn(first)
        publish(ROOT/'release',b'allow first provider to exit')
        event('first-provider-released',id=turn_id)
        candidate=locate(web,turn_id,first+'-finish','terminal')
        end_processes(first,*witness)
        (ROOT/'release').unlink()
        assert candidate['status']=='answered' and candidate['answer']['brief']=='Candidate only'
        assert candidate['answer']['message']=='Inert D11 candidate '+str(witness[1]['pid'])
        event('answered-before-stop',id=turn_id)
        late_stop=stop(turn_id)
        assert all(late_stop[k]==candidate[k] for k in ('id','status','ended_at','answer'))
        assert request(web,DIALOGUE)[0]==[candidate]
        assert brief_state(request(web,show)[0])==before
        ACTIVE.pop(first)
        VERDICTS['result_before_stop']='PASS'
        event('result-before-stop-pass',id=turn_id)

        second='stop-after-drain'
        VERDICTS['stop_after_drain']='FAIL'
        stopped,witness=held_turn(second)
        assert stopped!=turn_id
        ack=stop(stopped)
        assert ack['id']==stopped and ack['status']=='stopping' and not ack.get('answer')
        assert not (ROOT/'release').exists()
        event('stop-accepted-after-drain',id=stopped,status=ack['status'])
        terminal=locate(web,stopped,second+'-finish','terminal')
        end_processes(second,*witness)
        assert terminal['status']=='interrupted' and not terminal.get('answer')
        assert not (ROOT/'release').exists()
        turns,_=request(web,DIALOGUE)
        assert turns==[candidate,terminal] and len(BIRTHS)==2
        assert brief_state(request(web,show)[0])==before
        budget=request(web,'/api/v1/preparations/budget?id='+pid)[0]
        assert budget['reserved_usd']==0 and budget['committed_estimate_usd']==1
        ACTIVE.pop(second)
        report=web.stop()
        publish(OUT/'server-summary.json',report)
        assert report['completed_jobs']==report['observed_outcomes'] and report['rejected_requests']==0
        assert not any(report.get(k) for k in ('first_request_failure','first_service_failure','first_connection_failure'))
        assert cli('prepare','dialogue',pid)==turns
        assert cli('prepare','budget',pid)==budget
        assert brief_state(cli('prepare','show',pid))==before
        VERDICTS['stop_after_drain']='PASS'
        event('stop-after-drain-pass',id=stopped)
    finally:
        web.cleanup()


os.umask(0o077)
admitted_output()
OUT_FD = os.open(OUT,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
EXE_FD = os.open(BINARY,os.O_RDONLY|os.O_NOFOLLOW)
ENV = dict(PATH='/usr/bin:/bin',LANG='C.UTF-8',HOME=str(ROOT/'home'),
           CODEX_HOME=str(ROOT/'codex-home'),PYTHONDONTWRITEBYTECODE='1')
try:
    run()
    print('PREPARATION_WEB_STOP_ORDER_PASS',flush=True)
except BaseException:
    traceback.print_exc()
    raise
finally:
    publish(OUT/'result.json', dict(verdicts=VERDICTS, active_tours=ACTIVE,
        provider_launches=len(BIRTHS), last_available_bytes=available(),
        reserve_bytes=RESERVE, binary_sha256=SHA,
        limit='AF_UNIX queue drain is kernel transport evidence. This recipe orders public decisions and does not instrument parser completion or force a particular precommit interleaving.'))
    os.close(EXE_FD)
    os.close(OUT_FD)
