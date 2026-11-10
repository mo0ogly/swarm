#!/usr/bin/env python3
"""Verify actual priority writer→stored order and transactional replay/refusal.

Priority is ordinal metadata, not consent/capacity or execution permission. Public
CLI creates tasks and priority events; SQL only observes retained fixture rows.
The separate Rust integration target verifies actual post-admission audit failure.
No Go, provider, live Root, hidden business range or global watchdog.
"""
import argparse,json,sqlite3,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning
from rust_mission_task_order import snapshot


def execution_state(f):
    """Read actual owned consent/control/claims/finance/worker rows without writes.

    These tables exist from the public hierarchical fixture. Unknown schema/type
    or read/close errors propagate with Python's chained original traceback.
    No absence fallback, backup, identity proof or runtime policy is inferred.
    """
    db=sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True)
    try:
        return tuple(tuple(db.execute(sql).fetchall()) for sql in [
            'SELECT * FROM agents ORDER BY id',
            'SELECT * FROM reservations ORDER BY agent_id',
            'SELECT * FROM budgets ORDER BY work_id',
            'SELECT * FROM native_planning_claims ORDER BY claim_id',
            'SELECT * FROM cockpit_controls ORDER BY work_id',
            "SELECT * FROM cockpit_events WHERE kind='mission-policy' ORDER BY seq"])
    finally:db.close()


def main():
    """Require exact values, canonical replay, reopen, preserved controls and refusal."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile',type=Path,required=True);parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    a=parser.parse_args();out=a.output.resolve();out.mkdir(exist_ok=False)
    fixture.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and profile['sha256']==fixture.digest(a.binary.read_bytes())
        ready,f=planning.flow(a.binary.resolve(),out/'source',before_worker=True,return_fixture=True)
        assert ready['status']=='READY_FOR_NATIVE_WORKER' and ready['worker_started'] is False
        initial=f.ok(['mission','order',f.work]);assert initial['tasks'][0]['priority']=='0'
        w=f.ok(['work','show',f.work])['work']
        f.ok(['mission','start',f.work],dict(schema_version=1,event_id='priority-control-witness',expected_revision=w['revision'],reason='Explicit fixture consent control',slots=1,profile=dict(provider='fixture',role='worker',workspace='.',level='standard',capture_output=True)))
        w=f.ok(['work','show',f.work])['work']
        controls=f.ok(['mission','show',f.work]);assert controls['consent_enabled'] is True and controls['autonomy']=='autonome'
        controls.pop('revision');preserved=execution_state(f)
        request=dict(schema_version=1,event_id='priority-high',expected_revision=w['revision'],task_id='task',priority='9223372036854775807',reason='Owned exact ordinal fixture')
        changed=f.ok(['mission','priority',f.work],request)
        assert changed['revision']==w['revision']+1
        assert {k:v for k,v in changed.items() if k not in ['revision','updated']}=={k:v for k,v in w.items() if k not in ['revision','updated']}
        view=f.ok(['mission','show',f.work]);view.pop('revision');assert view==controls and execution_state(f)==preserved
        assert f.ok(['mission','order',f.work])['tasks'][0]['priority']==request['priority']
        saved=snapshot(f);assert f.ok(['mission','priority',f.work],request)==changed;assert snapshot(f)==saved
        for delta in [dict(priority='-9223372036854775808'),dict(event_id='priority-stale')]:
            bad={**request,**delta};code,wire=f.call(['mission','priority',f.work],bad);assert code!=0 and wire['failure']['code'];assert snapshot(f)==saved
        current=changed
        for value in [0,None,'','01','-0','+1',' 1','1.0','9223372036854775808','-9223372036854775809']:
            bad={**request,'event_id':'priority-invalid','expected_revision':current['revision'],'priority':value}
            code,wire=f.call(['mission','priority',f.work],bad);assert code!=0 and wire['failure']['code'];assert snapshot(f)==saved
        for field,value in [('task_id','missing'),('enabled',True),('actor','forged')]:
            bad={**request,'event_id':'priority-invalid','expected_revision':current['revision'],field:value}
            code,wire=f.call(['mission','priority',f.work],bad);assert code!=0 and wire['failure']['code'];assert snapshot(f)==saved
        low={**request,'event_id':'priority-low','expected_revision':current['revision'],'priority':'-9223372036854775808'}
        new=f.ok(['mission','priority',f.work],low);assert new['revision']==current['revision']+1
        assert f.ok(['mission','order',f.work])['tasks'][0]['priority']==low['priority']
        neutral={**request,'event_id':'priority-neutral','expected_revision':new['revision'],'priority':'0'}
        f.ok(['mission','priority',f.work],neutral)
        assert f.ok(['mission','order',f.work])['tasks'][0]['priority']=='0'
        view=f.ok(['mission','show',f.work]);view.pop('revision');assert view==controls and execution_state(f)==preserved
        db=sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True)
        try:assert db.execute("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='priority'",(f.work,)).fetchone()==(3,)
        finally:db.close()
        assert not (f.root/'model-received.json').exists()
        fixture.publish(out/'result.json',dict(status='PASS',root=str(f.root),actual_writer=True,replay_and_reopen=True,audit_refusal_and_recovery="separate qualified Rust integration target",provider_started=False,task_accepted=False))
        print('PASS exact persisted priority, canonical replay, refusals and actual order',flush=True)
    except BaseException as error:
        fixture.publish(out/'failure.json',dict(status='FAIL',message=str(error),traceback=traceback.format_exc()));raise


if __name__=='__main__':main()
