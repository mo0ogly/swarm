#!/usr/bin/env python3
"""Require an actual blocked scoped worker to wait for supervisor retry approval.

A real inert worker/control/reviewer lifecycle witnesses valid admission first.
No SQL seed, positive fake receipt, execution retry, paid model or global watchdog.
"""
import argparse,os,traceback,json
from pathlib import Path
import rust_native_review_acceptance as acceptance
from rust_native_review_decision import rows


def main():
    """Try public preview/prepare after actual changes_requested; preserve failures.

    Preview is not authority. Refusal must precede reservation/attempt/mutation;
    the already completed original worker/reviewer and their evidence stay intact.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);root=None
    acceptance.public.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();root,work=acceptance.prepare(binary,a.profile.resolve(),out/'source',review_case='changes-requested')
        before=acceptance.public.preview.shown(binary,root,out,'before',work);task=before['tasks'][0];assert task['status']=='blocked' and not task.get('planning_retry') and task['independent_review']['state']=='changes_requested'
        finance=acceptance.public.finance(root,work);report=(root/'handoff.md').read_bytes();reviewer=(root/'reviewer-invoked.json').read_bytes();worker=(root/'model-received.json').read_bytes()
        environment={**os.environ,'CODEX_HOME':str(root/'codex-home')};request=dict(schema_version=1,event_id='blocked-retry-session',expected_revision=before['revision'],task_id='task',provider='fixture',capture_output=True)
        preview_status,preview=acceptance.public.native.invoke(binary,root,out,'preview',['agent','preview',work],request,environment=environment)
        if preview_status==0:request['model_policy_hash']=preview['agent']['model_route']['policy_hash']
        status,value=acceptance.public.native.invoke(binary,root,out,'prepare',['agent','prepare',work],request,environment=environment)
        after=acceptance.public.preview.shown(binary,root,out,'after',work);reserved=rows(root,'SELECT state,amount FROM reservations WHERE agent_id=?',('blocked-retry-session',))
        acceptance.public.f.publish(out/'observation.json',dict(preview_exit=preview_status,prepare_exit=status,work_unchanged=after==before,reservations=reserved,attempts_before=len(task.get('attempts',[])),attempts_after=len(after['tasks'][0].get('attempts',[]))))
        assert status!=0 and 'scoped-worker-retry-decision-required' in value['failure']['causes'],value
        assert after==before and not reserved and acceptance.public.finance(root,work)==finance
        assert (root/'handoff.md').read_bytes()==report and (root/'reviewer-invoked.json').read_bytes()==reviewer and (root/'model-received.json').read_bytes()==worker
        acceptance.public.f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,unapproved_retry_refused=True,new_process=False,global_watchdog=None));print('PASS scoped retry requires supervisor decision',flush=True)
    except BaseException:acceptance.public.f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
