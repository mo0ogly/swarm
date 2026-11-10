#!/usr/bin/env python3
"""Replay a local native worker→handoff→gate→operator acceptance.

Owned roots only, no provider account, global watchdog or automatic acceptance.
All CLI input/output and original failures survive under --output. Gate PASS
assertions belong to this controlled fixture, never an independent review.
"""
import argparse, json, os, sqlite3, traceback
from pathlib import Path
import rust_agent_model_route as m

PROGRAM=m.PROGRAM+r"""
import tempfile
body=('# Local native fixture handoff\n\nInput SHA256: '+observed['stdin_sha256']+'\nRequested model: '+model+'\nObserved fixture only; not authenticated model or task acceptance.\n').encode()
fd,name=tempfile.mkstemp(dir='.',prefix='.handoff-')
with os.fdopen(fd,'wb') as stream:stream.write(body);stream.flush();os.fsync(stream.fileno())
os.replace(name,'handoff.md')
"""

def state(f):
    """Read committed work/events/agent/receipt; no private runtime writes."""
    v=f.ok(['work','show',f.work])
    return f.snapshot(),v['events']

def reject(f,args,request):
    """Require explicit JSON refusal and unchanged committed state, including event journal."""
    before=state(f);code,wire=f.call(args,request)
    assert code!=0 and wire['failure']['code'],wire
    assert state(f)==before,(args,wire)
    return wire['failure']['code']

def handoff(f,event='submit'):
    """Derive exact current revision/hash, never impersonate a conductor or fabricate an attempt."""
    w=f.ok(['work','show',f.work])['work']
    return dict(schema_version=1,event_id=event,expected_revision=w['revision'],id='task',agent_id='session',report_path='handoff.md',report_sha256=m.digest((f.root/'handoff.md').read_bytes()))

def update(f,status,event):
    """Use the supported operator task transition with optimistic revision."""
    w=f.ok(['work','show',f.work])['work']
    r=dict(schema_version=1,event_id=event,expected_revision=w['revision'],id='task',status=status)
    if status=='blocked':r['blocker']='Operator requests another handoff check'
    return r

def fixture(binary,output,label):
    """Run one real controlled child after public setup, then verify its actual report."""
    f=m.Fixture(binary,output,label)
    compile(PROGRAM,str(f.root/'codex'),'exec')
    m.publish(f.root/'codex',PROGRAM.encode());os.chmod(f.root/'codex',0o700)
    # Amend criteria through the same public producer consumed by delivery checks.
    w=f.ok(['work','show',f.work])['work']
    f.ok(['task','update',f.work],dict(schema_version=1,event_id='criteria',expected_revision=w['revision'],id='task',criteria=['Actual fixture argv/stdin and report content checked']))
    f.request['expected_revision']=f.ok(['work','show',f.work])['work']['revision']
    prepared=f.prepare();actual=f.run_and_assert(prepared)
    report=(f.root/'handoff.md').read_bytes()
    assert actual['stdin_sha256'].encode() in report and b'not authenticated model' in report
    return f

def submit(f,event='submit'):
    """Submit the real completed worker without changing attempts or budget receipt."""
    before=f.snapshot();r=handoff(f,event);v=f.ok(['task','submit',f.work],r)
    assert v['work']['tasks'][0]['status']=='submitted'
    after=f.snapshot()
    assert after[0]['tasks'][0]['attempts']==before[0]['tasks'][0]['attempts']
    assert after[1:]==before[1:]
    return r

def gate(f,event='gate'):
    """Evaluate fixture assertions with real generated check IDs and an explicit event.

    Callers renewing a gate supply a fresh event; default preserves single-gate
    recipes. No idempotence conflict is bypassed, and this is no independent review.
    """
    w=f.ok(['work','show',f.work])['work'];task=w['tasks'][0]
    checks=[dict(id=k,gate=v,mandatory=True,severity='minor',penalty=1,max_penalty=100) for k,v in task['plan_checks'].items()]
    checks.append(dict(id='fixture-blocking',domain='quality',gate='delivery',mandatory=True,severity='minor',penalty=1,max_penalty=100))
    doc=dict(method_version='3',scope_id=task['id'],artifacts={'handoff.md':m.digest((f.root/'handoff.md').read_bytes())},domains={'quality':1},checks=checks,results=[dict(id=c['id'],status='PASS',count=0,evidence=['handoff.md']) for c in checks])
    v=f.ok(['gate',f.work,'--task','task'],dict(schema_version=1,event_id=event,expected_revision=w['revision'],task_id='task',phase='delivery',name='Controlled delivery gate',document=doc))
    assert v['current_gates']['task']['valid'] is True
    assert v['work']['tasks'][0]['status']=='submitted'

def complete(binary,output):
    """Valid full path plus no-gate, stale-artifact rejection, restoration and durable replay."""
    f=fixture(binary,output,'complete');r=submit(f)
    no_gate=reject(f,['task','update',f.work],update(f,'accepted','accept-no-gate'))
    assert no_gate=='gate-evidence-invalid'
    gate(f);report=f.root/'handoff.md';original=report.read_bytes()
    m.publish(report,original+b'Changed after gate\n')
    assert f.ok(['work','show',f.work])['current_gates']['task']['valid'] is False
    request=update(f,'accepted','accept')
    stale=reject(f,['task','update',f.work],request)
    assert stale=='gate-evidence-invalid'
    # Event replay precedes current source reread, without changing state.
    before=state(f);f.ok(['task','submit',f.work],r);assert state(f)==before
    m.publish(report,original)
    accepted=f.ok(['task','update',f.work],request)
    assert accepted['work']['tasks'][0]['status']=='accepted'
    assert accepted['validation']['tasks']['task']['fresh'] is True
    before=state(f);f.ok(['task','update',f.work],request);assert state(f)==before
    assert f.reservation()==('estimated',1.0)
    return dict(case='complete',status='PASS',root=str(f.root),refusals=[no_gate,stale],scope='explicit fixture operator acceptance, no independent review or model authentication')

def refusals(binary,output):
    """Valid submission witness precedes faults; public block restores a reviewable task."""
    f=fixture(binary,output,'refusals');submit(f,'witness')
    f.ok(['task','update',f.work],update(f,'blocked','operator-block'))
    report=f.root/'handoff.md';original=report.read_bytes();r=handoff(f,'retry');codes=[]
    for patch in [dict(expected_revision=r['expected_revision']-1),dict(report_sha256='0'*64),dict(agent_id='unknown'),dict(origin='conductor'),dict(status='accepted')]:
        codes.append(reject(f,['task','submit',f.work],{**r,**patch}))
    m.publish(report,b' \n');codes.append(reject(f,['task','submit',f.work],{**r,'report_sha256':m.digest(report.read_bytes())}));m.publish(report,original)
    outside=output/'outside-report.txt';m.publish(outside,original)
    codes.append(reject(f,['task','submit',f.work],{**r,'report_path':os.path.relpath(outside,f.root)}))
    link=f.root/'outside-link';link.symlink_to(outside)
    codes.append(reject(f,['task','submit',f.work],{**r,'report_path':'outside-link'}))
    value=f.ok(['engine-settings','show'])['values']['review_delivery_max_bytes']
    f.settings_change(review_delivery_max_bytes='0')
    codes.append(reject(f,['task','submit',f.work],r))
    f.settings_change(review_delivery_max_bytes=value)
    submit(f,'retry')
    assert codes==['revision_conflict','task-report-digest-changed','task-report-agent-invalid','task-report-request-invalid','task-report-request-invalid','task-report-empty','task-report-file-invalid','task-report-file-invalid','task-report-file-invalid'],codes
    return dict(case='refusals',status='PASS',root=str(f.root),refusals=codes)

def superseded(binary,output):
    """An active new attempt and then its cancellation cannot authorize an older worker report."""
    f=fixture(binary,output,'superseded');submit(f,'witness')
    f.ok(['task','update',f.work],update(f,'blocked','operator-block'))
    f.request.update(event_id='session-two',expected_revision=f.ok(['work','show',f.work])['work']['revision'])
    queued=f.prepare();assert queued['agent']['status']=='queued'
    active=reject(f,['task','submit',f.work],handoff(f,'old-agent'))
    cancelled=f.ok(['agent','reconcile','session-two'])
    assert cancelled['cancelled_before_start'] is True
    assert cancelled['accepted_by_cancellation'] is False
    old=reject(f,['task','submit',f.work],handoff(f,'old-agent'))
    assert active=='active_agent' and old=='task-report-current-completed-agent-required'
    assert f.reservation()==('estimated',1.0)
    return dict(case='superseded',status='PASS',root=str(f.root),refusals=[active,old])

def alias_identity(binary,output):
    """A controlled corrupt alias row cannot select another agent's stored body.

    Only this fixture injects SQL corruption, without touching Work/consumption.
    Removing that exact added fault row restores the original records; no real
    agent/history is removed and only public operations submit the report.
    """
    f=fixture(binary,output,'alias');submit(f,'valid-witness')
    f.ok(['task','update',f.work],update(f,'blocked','operator-block'))
    db=sqlite3.connect(f.root/'.swarm/state.db')
    try:
        row=db.execute("SELECT work_id,task_id,cwd,status,desired,body,request FROM agents WHERE id='session'").fetchone();assert row
        db.execute('INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,?,?,?)',('alias',*row));db.commit()
    finally:db.close()
    r=handoff(f,'alias-submit');r['agent_id']='alias'
    code=reject(f,['task','submit',f.work],r);assert code=='task-report-agent-invalid'
    f.denied(['agent','show','alias'],None,'agent-row-identity-mismatch')
    db=sqlite3.connect(f.root/'.swarm/state.db')
    try:
        assert db.execute("SELECT work_id,task_id,cwd,status,desired,body,request FROM agents WHERE id='session'").fetchone()==row
        assert db.execute("SELECT work_id,task_id,cwd,status,desired,body,request FROM agents WHERE id='alias'").fetchone()==row
        db.execute("DELETE FROM agents WHERE id='alias'");db.commit()
    finally:db.close()
    submit(f,'recovered-submit')
    return dict(case='alias-identity',status='PASS',root=str(f.root),refusals=[code,'agent-row-identity-mismatch'])

def main():
    """Retain exact recipe/binary/helper hashes and original diagnostics; never retry silently."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();binary=a.binary.resolve();output=a.output.resolve();output.mkdir(parents=True,exist_ok=False)
    m.publish(output/'driver.py',Path(__file__).read_bytes());m.publish(output/'inputs.json',dict(binary=str(binary),binary_sha256=m.digest(binary.read_bytes()),helper_sha256=m.digest(Path(m.__file__).read_bytes()),global_watchdog=None))
    checks=[]
    try:
        for scenario in [complete,refusals,superseded,alias_identity]:
            result=scenario(binary,output);checks.append(result);print('PASS native-manual-mission-'+result['case'],flush=True)
        m.publish(output/'result.json',dict(status='PASS',checks=checks))
    except BaseException as error:
        m.publish(output/'failure.json',dict(status='FAIL',type=type(error).__name__,message=str(error),traceback=traceback.format_exc(),checks=checks));raise

if __name__=='__main__':main()
