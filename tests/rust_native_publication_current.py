#!/usr/bin/env python3
"""Reject altered publication coordinates against real original native controls.

Use an actually published inert fixture, never positive SQL seeding or a live
mission. Every negative SQL change restores exact original command/row bytes.
The real normal CLI replay and fixture diagnostic reader must agree; no watchdog.
"""
import argparse,json,os,sqlite3,subprocess,traceback
from pathlib import Path
import rust_native_managed_reviewer as native
from rust_native_task_validation import invoke


def run(binary,probe,published,out):
    """Verify valid witness, each exact canonical corruption, then full recovery.

    A supplied successful public result and retained fixture birth are required.
    Binding, cohort and result fields must match the private original candidate;
    changing matching SQL row/receipt coordinates cannot establish that authority.
    Report failures preserve raw channels and original stored material for review.
    """
    out.mkdir(parents=True,exist_ok=False,mode=0o700);f=native.inspection.native.fixture
    source=json.loads((published/'result.json').read_text());assert source['status']=='PASS' and source['task_accepted'] and not source['paid_provider']
    root=Path(source['root']);work_id=source['work'];assert any(json.loads(p.read_text()).get('root')==str(root) for p in (published/'native/inspection').rglob('owned-root-*.json'))
    env=dict(os.environ,CODEX_HOME=str(root/'codex-home'));before=json.loads((published/'after.stdout').read_text())['work'];task=next(t for t in before['tasks'] if t['status']=='accepted')
    result=json.loads((published/'publish.stdout').read_text());event=result['publication_event'];request=json.loads((published/'publish.input.json').read_text())
    mark=root/'.rust-migration-fixture';created=not mark.exists()
    if created:f.publish(mark,b'owned native current publication diagnostic\n')
    db=sqlite3.connect(root/'.swarm/state.db');original=db.execute('SELECT request,payload FROM events WHERE id=?',(event,)).fetchone();record=json.loads(original[0]);original_result=db.execute('SELECT result_commit FROM managed_attempts WHERE agent_id=?',(record['producer'],)).fetchone()[0];original_work=db.execute('SELECT revision,body FROM works WHERE id=?',(work_id,)).fetchone();finance=native.finance(root,work_id);checks=[]
    def probe_current(label,valid):
        """Invoke current proof through the existing explicit diagnostic guard."""
        cmd=[str(probe),str(root),task['id'],'current'];r=subprocess.run(cmd,input=json.dumps(before).encode(),capture_output=True,env=env)
        f.publish(out/(label+'.command.json'),dict(argv=cmd,global_watchdog=None));f.publish(out/(label+'.stdout'),r.stdout);f.publish(out/(label+'.stderr'),r.stderr)
        assert (r.returncode==0)==valid,(label,r.stdout,r.stderr)
    def replay(label,valid):
        """A canonical replay must refuse altered evidence without changing Work."""
        code,value=invoke(binary,root,out,label,['task','publish',work_id],request,env);assert (code==0)==valid,(label,value)
        if valid:assert value['replayed'] and value['work']==before and not value['provider_started']
        code,value=invoke(binary,root,out,label+'-work',['work','show',work_id],environment=env)
        if valid:assert code==0 and value['work']==before
        else:
            # Public Work projection also verifies accepted review evidence and
            # must refuse this corruption, rather than expose an untrusted view.
            assert code!=0 and 'managed-publication-original-candidate-binding' in value['failure']['causes'],value
        assert db.execute('SELECT revision,body FROM works WHERE id=?',(work_id,)).fetchone()==original_work
        assert native.finance(root,work_id)==finance
    try:
        for name in ['binding_sha256','cohort_sha256','result_commit']:
            probe_current(name+'-valid',True);replay(name+'-valid-replay',True)
            value=record[name];needle=('"'+name+'":"'+value+'"').encode();assert original[0].count(needle)==1
            changed=original[0].replace(needle,('"'+name+'":"'+'0'*len(value)+'"').encode(),1)
            try:
                db.execute('UPDATE events SET request=?,payload=? WHERE id=?',(changed,changed,event))
                if name=='result_commit':db.execute('UPDATE managed_attempts SET result_commit=? WHERE agent_id=?',('0'*len(value),record['producer']))
                db.commit();probe_current(name+'-refused',False);replay(name+'-replay-refused',False)
            finally:
                db.execute('UPDATE events SET request=?,payload=? WHERE id=?',(*original,event));db.execute('UPDATE managed_attempts SET result_commit=? WHERE agent_id=?',(original_result,record['producer']));db.commit()
            probe_current(name+'-restored',True);replay(name+'-restored-replay',True);checks.append(name+'-bound-to-real-private-candidate')
        assert db.execute('SELECT request,payload FROM events WHERE id=?',(event,)).fetchone()==original
    finally:
        db.close()
        if created:mark.unlink()
    f.publish(out/'result.json',dict(status='PASS',checks=checks,provider_started=False,global_watchdog=None))
    print('MANAGED_PUBLICATION_CURRENT_PASS',len(checks),flush=True)


def main():
    """Pin actual executables and maintained code, retaining original failures."""
    p=argparse.ArgumentParser(description=__doc__)
    for key in ['binary','probe','published-from','output']:p.add_argument('--'+key,type=Path,required=True)
    a=p.parse_args();f=native.inspection.native.fixture;out=a.output.resolve()
    inputs=dict(binary_sha256=f.digest(a.binary.read_bytes()),probe_sha256=f.digest(a.probe.read_bytes()),helpers=native.inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(a.binary.resolve(),a.probe.resolve(),a.published_from.resolve(),out)
        assert f.digest(a.binary.read_bytes())==inputs['binary_sha256'] and f.digest(a.probe.read_bytes())==inputs['probe_sha256'] and all(f.digest(Path(k).read_bytes())==v for k,v in inputs['helpers'].items())
        f.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.exists():f.publish(out/'inputs.json',inputs);f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
