#!/usr/bin/env python3
"""Verify reusable profile levels through actual native preview/prepare/replay.

Uses only an owned public CLI fixture and inert executable metadata, never a
provider process, Go, installed Root or refund. A missing requested level means
intentional inheritance, not a default to persist. Preserve failed roots and
all original channels; a fresh output is required and no watchdog is added.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as common


def scenario(binary, output, name):
    """Prove a valid preview then exact saved task/Work levels and replay.

    Configure a task-specific choice only with public model-policy preview/apply.
    Preview preserves state, admission creates one retained estimate, reopening
    and replay preserve it. This checks saved choices, not model authenticity.
    """
    f=common.Fixture(binary,output,name)
    expected=''
    if name=='explicit':
        expected='standard';f.request['level']=expected
    elif name=='task-choice':
        initial=f.prepare(True)['agent']['model_route']
        common.publish(f.cache,{'models':[{'slug':'fixture-simple','visibility':'list'},
            {'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]}]})
        change=dict(schema_version=1,event_id='task-choice',expected_revision=3,
            task_id='task',provider='fixture',level='simple',
            model_policy_hash=initial['policy_hash'],inherit=False)
        f.ok(['task-model','preview',f.work],change)
        chosen=f.ok(['task-model','apply',f.work],change)
        f.request['expected_revision']=chosen['revision']
        expected='simple'
    before=f.snapshot();preview=f.prepare(True)
    assert preview['preview_only'] and not preview['process_started']
    assert f.snapshot()==before and f.reservation() is None
    assert preview['agent']['model_route']['level']==(expected or 'standard')
    prepared=f.prepare();assert prepared['created'] and not prepared['process_started']
    saved=f.ok(['work','show',f.work])['work']
    task=saved['tasks'][0]['launch_profile']
    assert task.get('level','')==expected,('task profile lost requested level',name,task,expected)
    assert saved['launch_profile'].get('level','')==expected,('Work profile lost requested level',name,saved['launch_profile'],expected)
    snapshot=f.snapshot();reservation=f.reservation()
    assert reservation==('reserved',1.0),reservation
    replay=f.prepare()
    assert not replay['created'] and replay['agent']['id']==prepared['agent']['id']
    assert f.snapshot()==snapshot and f.reservation()==reservation
    assert f.ok(['work','show',f.work])['work']==saved
    assert not (f.root/'model-received.json').exists()
    return dict(case=name,status='PASS',root=str(f.root),expected_level=expected,preview_readonly=True,replay_readonly=True)


def main():
    """Retain exact script/binary identity and each case; fail without retry."""
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args();binary=args.binary.resolve();out=args.output.resolve()
    out.mkdir(parents=True,exist_ok=False);checks=[]
    common.publish(out/'method.py',Path(__file__).read_bytes())
    common.publish(out/'inputs.json',dict(binary=str(binary),sha256=common.digest(binary.read_bytes()),global_watchdog=None))
    try:
        for name in ['inherited','explicit','task-choice']:
            value=scenario(binary,out,name);checks.append(value)
            print('PASS reusable-profile-level-'+name,flush=True)
        common.publish(out/'result.json',dict(status='PASS',checks=checks,installed=False))
    except BaseException as e:
        common.publish(out/'failure.json',dict(status='FAIL',message=str(e),traceback=traceback.format_exc(),checks=checks))
        raise


if __name__=='__main__':
    main()
