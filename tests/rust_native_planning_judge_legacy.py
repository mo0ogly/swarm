#!/usr/bin/env python3
"""Old unexecuted root context refuses safely, then public resume binds a new claim.

Two frozen normal binaries only; no live Root, SQL seed, fake context backfill,
refund, paid call or whole-test watchdog. This does not cover an in-flight old
model result; that requires its own original/known-wait migration proof.
"""
import argparse,json,os,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_planning_worker_public_flow as flow
from rust_planning_activation import configured,claim_request
from rust_planning_call_preview import preview_request
from rust_planning_decision import decision
from rust_native_review_decision import rows
import rust_native_review_acceptance as review


def main():
    """Keep the actual old receipt byte-identical and expose the missing projection.

    Old call-preview is a valid same-claim control. New preview must refuse the
    absent judge contract without changing Work/funding or starting a process.
    Public resume revokes only this owned unexecuted lease; the new claim appends
    its real context and preserves old consumption/record before one actual call.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','legacy-binary','legacy-profile','output']:p.add_argument('--'+name,required=True,type=Path)
    args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=args.binary.resolve();legacy=args.legacy_binary.resolve()
        for program,path in [(binary,args.profile),(legacy,args.legacy_profile)]:
            profile=json.loads(path.read_bytes());assert profile['no_fixture_feature'] and profile['sha256']==f.digest(program.read_bytes())
        owned=f.Fixture(legacy,out/'source','judge-legacy');f.publish(owned.root/'codex',flow.feedback.PROGRAM.encode());(owned.root/'codex').chmod(0o700)
        enabled=configured(owned,'legacy-root');old_id='legacy-root-claim';claimed=owned.ok(['planning','claim',owned.work],claim_request(enabled,old_id));value=preview_request(claimed,old_id);control=owned.ok(['planning','call-preview',owned.work],value);assert control['prepared_inputs_verified'] and not control['process_started']
        old=rows(owned.root,'SELECT context_json,context_sha256,generation FROM native_planning_claims WHERE claim_id=? AND work_id=?',(old_id,owned.work))[0];assert 'judge' not in json.loads(old[0]);f.publish(out/'old-context.json',old[0]);before=owned.ok(['work','show',owned.work])['work'];money=review.public.finance(owned.root,owned.work);markers=set(owned.root.glob('fixture-prompt-*.bin'))
        owned.binary=binary;code,refused=owned.call(['planning','call-preview',owned.work],value);assert code!=0 and 'planning-call-scope-contract-changed' in refused['failure']['causes'],refused
        assert owned.ok(['work','show',owned.work])['work']==before and review.public.finance(owned.root,owned.work)==money and set(owned.root.glob('fixture-prompt-*.bin'))==markers
        resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='fresh-cycle-projection',expected_revision=before['revision'],scope='root',reason='Bind actual current cycle context; preserve the old unexecuted claim'))
        scope=next(s for s in resumed['planning']['scopes'] if s['id']=='root');new_id='fresh-root-claim';fresh=owned.ok(['planning','claim',owned.work],claim_request(resumed,new_id,scope_revision=scope['revision']));new=rows(owned.root,'SELECT context_json,context_sha256,generation FROM native_planning_claims WHERE claim_id=? AND work_id=?',(new_id,owned.work))[0];assert json.loads(new[0])['judge']==fresh['planning']['judge'] and new[2]>old[2] and new[1]!=old[1]
        assert rows(owned.root,'SELECT context_json,context_sha256,generation FROM native_planning_claims WHERE claim_id=? AND work_id=?',(old_id,owned.work))[0]==old and fresh['planning']['activations']==2 and all(row in review.public.finance(owned.root,owned.work) for row in money)
        fresh_money=review.public.finance(owned.root,owned.work);flow.proposal(owned,fresh,'root',[dict(kind='judge',judge_cycle=1,judge_verdict='continue')]);ready=owned.ok(['planning','call-preview',owned.work],preview_request(fresh,new_id));assert ready['prepared_inputs_verified']
        code,result=review.public.native.invoke(binary,owned.root,out,'new-call',['planning','run',owned.work],preview_request(fresh,new_id),environment={**os.environ,'CODEX_HOME':str(owned.root/'codex-home')});raw=(owned.root/'.swarm'/('native-planning-'+f.digest(new_id.encode())+'.receipt')).read_bytes();magic,extent,sha,body=raw.split(b'\n',3);assert magic==b'SR1' and int(extent)==len(body) and f.digest(body)==sha.decode();f.publish(out/'new-original.frame',raw);assert code==0 and result['decision_applied'],result
        after=owned.ok(['work','show',owned.work])['work'];assert after['planning']['judge']['verdict']=='continue' and after['planning']['activations']==2 and review.public.finance(owned.root,owned.work)==fresh_money
        original=json.loads(raw.split(b'\n',3)[3]);pid=original['run']['child_pid'];prompt=(owned.root/('fixture-prompt-'+str(pid)+'.bin')).read_bytes();assert new[0].encode() in prompt and f.digest(prompt)==original['run']['prompt_sha256'];f.publish(out/'new-prompt.bin',prompt)
        f.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,old_unexecuted_claim_preserved=True,old_context_refused_without_mutation=True,public_resume_new_claim=True,actual_new_model=True,no_refund_or_backfill=True,old_inflight_result_covered=False,authenticated_actor=False,global_watchdog=None));print('PASS old unexecuted root claim refused; public fresh binding succeeds',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
