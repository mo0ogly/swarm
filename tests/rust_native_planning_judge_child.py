#!/usr/bin/env python3
"""A real child planner cannot judge the root or read its cycle history.

Owned inert processes only; valid root delegation and child ACK precede the
wrong-scope operation. No positive SQL/receipt seed, paid model or watchdog.
"""
import argparse,json,os,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_planning_worker_public_flow as flow
from rust_planning_bootstrap import fresh_work,request
from rust_native_review_decision import rows
import rust_native_review_acceptance as review


def main():
    """Require actual child scope binding and preserve the refused call's evidence.

    Root records a real private reason before delegation; four total activations
    and child capacity2 cover this verdict, delegation, child ACK and invalid action.
    The invalid call consumes its real claim; no attempt, task or judge is reset.
    Context privacy/lease binding is not semantic agent identity authentication.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,required=True,type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=args.binary.resolve();profile=json.loads(args.profile.read_bytes());assert profile['no_fixture_feature'] and profile['sha256']==f.digest(binary.read_bytes())
        owned=f.Fixture(binary,out/'source','judge-child');owned.work=fresh_work(owned,'child-judge-create');enabled=owned.ok(['planning','enable',owned.work],request(event='child-judge-enable',max_activations=4));f.publish(owned.root/'codex',flow.feedback.PROGRAM.encode());(owned.root/'codex').chmod(0o700);environment={**os.environ,'CODEX_HOME':str(owned.root/'codex-home')}
        private_reason='ROOT_PRIVATE_GLOBAL_JUDGE_REASON'
        flow.proposal(owned,enabled,'root',[dict(kind='judge',judge_cycle=1,judge_verdict='continue')]);proposal=json.loads((owned.root/'planning-proposal.json').read_bytes());proposal['reason']=private_reason;f.publish(owned.root/'planning-proposal.json',proposal)
        code,value=review.public.native.invoke(binary,owned.root,out,'root-verdict',['planning','step',owned.work],environment=environment);assert code==0 and value['run']['decision_applied'],value;enabled=value['run']['work'];assert enabled['planning']['judge']['history'][0]['reason']==private_reason
        flow.proposal(owned,enabled,'root',[dict(kind='delegate',id='child',title='Owned child',requirements=['req-1'],max_tasks=1,max_activations=2)])
        code,value=review.public.native.invoke(binary,owned.root,out,'delegate',['planning','step',owned.work],environment=environment);assert code==0 and value['run']['decision_applied'],value;current=value['run']['work']
        flow.proposal(owned,current,'child',[]);code,value=review.public.native.invoke(binary,owned.root,out,'healthy-child-ack',['planning','step',owned.work],environment=environment);assert code==0 and value['run']['decision_applied'],value;current=value['run']['work']
        current=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='child-judge-resume',expected_revision=current['revision'],scope='child',reason='Reassess only this owned child scope'))
        original_judge=current['planning']['judge'];money=review.public.finance(owned.root,owned.work)
        flow.proposal(owned,current,'child',[dict(kind='judge',judge_cycle=original_judge['cycle'],judge_verdict='continue')]);code,value=review.public.native.invoke(binary,owned.root,out,'child-judge-refused',['planning','step',owned.work],environment=environment)
        claim,phase=rows(owned.root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? AND scope_id=? ORDER BY prepared_revision DESC',(owned.work,'child'))[0];raw=(owned.root/'.swarm'/('native-planning-'+f.digest(claim.encode())+'.receipt')).read_bytes();magic,extent,sha,body=raw.split(b'\n',3);assert magic==b'SR1' and int(extent)==len(body) and f.digest(body)==sha.decode();f.publish(out/'child-original.frame',raw)
        original=json.loads(body);pid=original['run']['child_pid'];prompt=(owned.root/('fixture-prompt-'+str(pid)+'.bin')).read_bytes();witness=json.loads((owned.root/('model-received-'+str(pid)+'.json')).read_bytes());context,digest=rows(owned.root,'SELECT context_json,context_sha256 FROM native_planning_claims WHERE claim_id=?',(claim,))[0];context_value=json.loads(context)
        f.publish(out/'child-context.json',context_value);f.publish(out/'child-prompt.bin',prompt);f.publish(out/'child-witness.json',witness)
        assert f.digest(context.encode())==digest and context.encode() in prompt and f.digest(prompt)==witness['prompt_sha256']==original['run']['prompt_sha256'] and witness['pid']==pid and witness['ppid']==original['run']['parent_pid']
        assert private_reason.encode() not in prompt and original_judge['history'][0]['reason']==private_reason
        assert context_value['scope']['id']=='child' and 'judge' not in context_value and context_value['judge_available'] is False and context_value['renew_available'] is False
        assert code!=0 and 'planning-judge-root-required' in value['failure']['causes'],value
        after=owned.ok(['work','show',owned.work])['work'];assert after['planning']['judge']==original_judge and not after.get('tasks') and phase=='refused'
        newmoney=review.public.finance(owned.root,owned.work);assert len(newmoney)==len(money)+1 and all(row in newmoney for row in money)
        f.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,actual_child_model=True,root_history_not_disclosed=True,root_judge_unchanged=True,actual_refused_call_preserved=True,authenticated_actor=False,global_watchdog=None));print('PASS actual child planner cannot judge root or receive root history',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
