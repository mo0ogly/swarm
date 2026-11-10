#!/usr/bin/env python3
"""Verify native guard refusal/recovery for root-valued active workspace metadata.

A genuine queued disjoint worker precedes negative row corruption. This exercises
actual admission; it is not a legitimate filesystem-root project/exploit, process
execution or proof of tamper resistance against same-UID SQL writers. No provider,
Go, installed service or arbitrary watchdog. Original failed fixture survives.
"""
import argparse,json,sqlite3,traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_mission_task_order import snapshot


def main():
    """Keep valid disjoint witness, refuse corrupted ancestor, restore and recheck."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--profile',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    a=parser.parse_args();out=a.output.resolve();out.mkdir(exist_ok=False);fixture.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and fixture.digest(a.binary.read_bytes())==profile['sha256']
        f=fixture.Fixture(a.binary.resolve(),out,'root-overlap')
        (f.root/'busy').mkdir();(f.root/'candidate').mkdir()
        work=f.ok(['work','show',f.work])['work']
        f.ok(['task','add',f.work],dict(schema_version=1,event_id='busy-task-add',expected_revision=work['revision'],id='busy',title='Genuine first intent',deliverable='Guard fixture',criteria=['No process started']))
        work=f.ok(['work','show',f.work])['work']
        busy=dict(schema_version=1,event_id='busy-agent',expected_revision=work['revision'],task_id='busy',provider='fixture',workspace='busy',capture_output=True)
        original=f.ok(['agent','prepare',f.work],busy);assert original['agent']['status']=='queued' and original['created']
        work=f.ok(['work','show',f.work])['work'];request={**f.request,'event_id':'candidate-agent','expected_revision':work['revision'],'workspace':'candidate'}
        before=snapshot(f);valid=f.ok(['agent','preview',f.work],request);assert valid['preview_only'] and snapshot(f)==before
        db=sqlite3.connect(f.root/'.swarm/state.db')
        try:
            column,body=db.execute('SELECT cwd,body FROM agents WHERE id=?',('busy-agent',)).fetchone()
            altered=json.loads(body);altered['workspace']='/'
            db.execute('UPDATE agents SET cwd=?,body=? WHERE id=?',('/',json.dumps(altered).encode(),'busy-agent'));db.commit()
            # Preview keeps a failed before-fix fixture free of a second admission.
            injected=snapshot(f);code,wire=f.call(['agent','preview',f.work],request)
            assert code!=0, 'expected workspace refusal, got success; original channels retained'
            assert 'agent-launch-workspace-busy' in wire['failure']['causes'], 'wrong refusal; original channels retained'
            assert snapshot(f)==injected and not (f.root/'model-received.json').exists()
            code,wire=f.call(['agent','prepare',f.work],request);assert code!=0 and 'agent-launch-workspace-busy' in wire['failure']['causes'];assert snapshot(f)==injected
            db.execute('UPDATE agents SET cwd=?,body=? WHERE id=?',(column,body,'busy-agent'));db.commit()
        finally:db.close()
        assert snapshot(f)==before
        f.ok(['agent','preview',f.work],request);assert snapshot(f)==before
        actual=f.ok(['agent','prepare',f.work],request);assert actual['created'] and actual['agent']['status']=='queued'
        assert not (f.root/'model-received.json').exists()
        fixture.publish(out/'result.json',dict(status='PASS',root=str(f.root),genuine_first_intent=True,negative_metadata_only=True,filesystem_root_project_tested=False,no_provider=True))
        print('PASS native root workspace observation refuses without mutation and recovers',flush=True)
    except BaseException as error:
        fixture.publish(out/'failure.json',dict(status='FAIL',message=str(error),traceback=traceback.format_exc()));raise


if __name__=='__main__':main()
