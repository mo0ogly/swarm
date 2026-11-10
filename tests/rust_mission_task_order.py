#!/usr/bin/env python3
"""Verify read-only native mission ordering; this does not select or launch workers.

Public Work/task creation supplies graph inputs. Priority rows are fixture DATA,
not claims, consent or execution permission. A valid witness precedes invalid SQL
type/orphan injections; restore exact values and require recovery. No provider,
Go, installed service or arbitrary watchdog. Preserve original channels/failures.
"""
import argparse,json,sqlite3,traceback
from pathlib import Path
import rust_agent_model_route as fixture


def snapshot(f):
    """Observe every table of the owned fixture, including priorities/events.

    This is in-memory diagnostic comparison only, not backup/restore capability.
    Private fixture originals remain at their retained Root; no SQL is written.
    """
    db=sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True)
    try:return tuple(db.iterdump())
    finally:db.close()


def main():
    """Check complete order, exact signed priority, graph depth and no mutation."""
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--profile',type=Path,required=True);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();out=a.output.resolve();out.mkdir(exist_ok=False)
    fixture.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and profile['sha256']==fixture.digest(a.binary.read_bytes())
        f=fixture.Fixture(a.binary.resolve(),out,'order')
        for task,depends in [('alpha',[]),('child',['alpha']),('grand',['child'])]:
            w=f.ok(['work','show',f.work])['work']
            f.ok(['task','add',f.work],dict(schema_version=1,event_id='add-'+task,expected_revision=w['revision'],id=task,title=task,deliverable='Ordering fixture',criteria=['Observed order only'],depends=depends))
        before=snapshot(f);v=f.ok(['mission','order',f.work]);assert snapshot(f)==before
        assert v['scope']=='order_only_not_eligibility_or_permission'
        assert [t['task_id'] for t in v['tasks']]==['alpha','task','child','grand']
        assert [t['depth'] for t in v['tasks']]==['0','0','1','2']
        assert all(t['priority']=='0' for t in v['tasks'])
        db=sqlite3.connect(f.root/'.swarm/state.db')
        try:
            db.executemany('INSERT INTO cockpit_tasks(work_id,task_id,priority) VALUES(?,?,?)',[(f.work,'child',9223372036854775807),(f.work,'alpha',-9223372036854775808)]);db.commit()
            saved=snapshot(f);v=f.ok(['mission','order',f.work]);assert snapshot(f)==saved
            assert [t['task_id'] for t in v['tasks']]==['child','task','grand','alpha']
            assert v['tasks'][0]['priority']=='9223372036854775807' and v['tasks'][-1]['priority']=='-9223372036854775808'
            for task,value in [('child','invalid-integer'),('missing',1)]:
                if task=='child':db.execute('UPDATE cockpit_tasks SET priority=? WHERE work_id=? AND task_id=?',(value,f.work,task))
                else:db.execute('INSERT INTO cockpit_tasks(work_id,task_id,priority) VALUES(?,?,?)',(f.work,task,value))
                db.commit();invalid=snapshot(f);code,wire=f.call(['mission','order',f.work]);assert code!=0 and wire['failure']['code'];assert snapshot(f)==invalid
                if task=='child':db.execute('UPDATE cockpit_tasks SET priority=? WHERE work_id=? AND task_id=?',(9223372036854775807,f.work,task))
                else:db.execute('DELETE FROM cockpit_tasks WHERE work_id=? AND task_id=?',(f.work,task))
                db.commit();assert f.ok(['mission','order',f.work])==v
        finally:db.close()
        for args,value in [(['mission','order',f.work],dict(unused=True)),(['mission','order','missing'],None)]:
            saved=snapshot(f);code,wire=f.call(args,value);assert code!=0 and wire['failure']['code'];assert snapshot(f)==saved
        assert not (f.root/'model-received.json').exists()
        fixture.publish(out/'result.json',dict(status='PASS',scope='Order only; priority setters/UX and actual dispatch remain unported',mutations='Fixture metadata only; supported reader does not mutate',root=str(f.root)))
        print('PASS native complete task order, graph depth, exact priorities, refusals and recovery',flush=True)
    except BaseException as error:
        fixture.publish(out/'failure.json',dict(status='FAIL',message=str(error),traceback=traceback.format_exc()));raise


if __name__=='__main__':main()
