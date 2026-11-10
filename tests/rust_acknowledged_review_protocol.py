#!/usr/bin/env python3
"""Actual acknowledged review incident must remain recoverable on fresh input.

Never reconsume the old event, rewrite its decision or reset capacity. Four child
activations are explicitly saved before delegation for task/feedback/ack/retry;
all provider programs are inert, no positive SQL/receipt seed or paid call.
"""
import argparse,json,os,traceback
from pathlib import Path
from types import SimpleNamespace
import rust_native_review_acceptance as a
import rust_planning_worker_public_flow as flow
from rust_native_review_decision import rows


def main():
    """Witness valid source first, preserve every actual original before asserting.

    ACK means received, not resolved. A fresh operator input must deliver the
    still-current error as attributed read-only context; retry may consume only
    fresh event IDs. Keep old acknowledgement, task/attempt/control and spending.
    This stage asserts the decision only; second review remains separately tested.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--review-case',choices=['domain-invalid','interrupted'],default='domain-invalid');p.add_argument('--cap-control',type=Path,help='Prior healthy full control required for public context-cap refusal/restoration');args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);root=None;f=a.public.f;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=args.binary.resolve();control_root,control_work=a.prepare(binary,args.profile.resolve(),out/'control',child_activation_budget=4);control=a.public.preview.shown(binary,control_root,out,'healthy',control_work);assert control['tasks'][0]['independent_review']['state']=='passed' and next(s for s in control['planning']['scopes'] if s['id']=='child')['max_activations']==4
        root,work=a.prepare(binary,args.profile.resolve(),out/'source',review_case=args.review_case,child_activation_budget=4);before=a.public.preview.shown(binary,root,out,'before-ack',work);task=before['tasks'][0];review=task['independent_review'];assert review['state']=='error';child=next(s for s in before['planning']['scopes'] if s['id']=='child');assert child['max_activations']==4 and child['activations']==2
        f.publish(root/'codex',flow.feedback.PROGRAM.encode());(root/'codex').chmod(0o700);environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
        flow.proposal(SimpleNamespace(root=root),before,'child',[]);code,ack=a.public.native.invoke(binary,root,out,'ack',['planning','step',work],environment=environment);assert code==0 and ack['run']['decision_applied'],ack;current=ack['run']['work'];old_event=next(e for e in current['planning']['inbox'] if e['id']==review['id']);ack_id=old_event['decision'];assert ack_id
        code,fresh=a.public.native.invoke(binary,root,out,'fresh',['planning','resume',work],dict(schema_version=1,event_id='fresh-review-correction',expected_revision=current['revision'],scope='child',reason='Reviewed response protocol correction is now supplied after acknowledgement'),environment=environment);assert code==0,fresh
        operation=dict(kind='review-retry',id='task',next=('Resume the preserved acknowledged reviewer after known terminal interruption on this fresh activation; keep producer, report and controls unchanged' if args.review_case=='interrupted' else 'Correct the response schema of the preserved acknowledged reviewer incident on this new activation, with unchanged producer and report'))
        flow.proposal(SimpleNamespace(root=root),fresh,'child',[operation]);proposal=json.loads((root/'planning-proposal.json').read_bytes());assert review['id'] not in proposal['input_events']
        if args.cap_control:
            control=json.loads(args.cap_control.read_bytes());assert control['status']=='PASS' and control['actual_second_reviewer'] and not control['faults'] and control['signature']['acknowledged'] and control['signature']['interrupted']==(args.review_case=='interrupted') and control['signature']['binary_sha256']==f.digest(binary.read_bytes())
            expected=dict(contract=f.digest(json.dumps({k:task.get(k) for k in ['title','deliverable','criteria','depends','plan_max_attempts']},sort_keys=True).encode()),settings=f.digest(json.dumps(json.loads((root/'.swarm/engine-settings.json').read_bytes())['values'],sort_keys=True).encode()));assert control['signature']['setup']==expected
            unchanged=a.public.preview.shown(binary,root,out,'before-cap',work);old_money=a.public.finance(root,work);old_jobs=rows(root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY claim_id',(work,));errors=[];original_settings=None
            try:
                original_settings,_=a.public.preview.saved_setting(binary,root,out,'context-cap-low','planning_context_max_events','1')
                code,refused=a.public.native.invoke(binary,root,out,'context-cap-refused',['planning','step',work],environment=environment);assert code!=0 and 'planning-context-event-limit-exceeded' in refused['failure']['causes'],refused
                assert a.public.preview.shown(binary,root,out,'cap-unchanged',work)==unchanged and a.public.finance(root,work)==old_money and rows(root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY claim_id',(work,))==old_jobs
            except BaseException as error:errors.append(error)
            if original_settings is not None:
                try:a.public.preview.saved_setting(binary,root,out,'context-cap-restore','planning_context_max_events',original_settings['values']['planning_context_max_events'])
                except BaseException as error:errors.append(error)
            if errors:raise BaseExceptionGroup('Public cap refusal and restoration failed',errors)
        money=a.public.finance(root,work);code,value=a.public.native.invoke(binary,root,out,'retry',['planning','step',work],environment=environment);jobs=rows(root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? AND scope_id=? ORDER BY prepared_revision,claim_id',(work,'child'));originals={};last_body=None
        for claim,phase in jobs:
            path=root/'.swarm'/('native-planning-'+f.digest(claim.encode())+'.receipt');raw=path.read_bytes();magic,extent,sha,body=raw.split(b'\n',3);assert magic==b'SR1' and int(extent)==len(body) and f.digest(body)==sha.decode();f.publish(out/(claim+'.frame'),raw);originals[claim]=dict(phase=phase,frame_sha256=f.digest(raw));last_body=json.loads(body)
        after=a.public.preview.shown(binary,root,out,'after',work);assert next(e for e in after['planning']['inbox'] if e['id']==review['id'])['decision']==ack_id and after['tasks'][0]['attempts']==task['attempts'] and after['tasks'][0]['independent_review']==review
        f.publish(out/'observation.json',dict(exit=code,failure=value.get('failure'),root=str(root),work=work,old_review=review['id'],old_ack=ack_id,old_ack_preserved=True,proposal=proposal,originals=originals));assert code==0 and value['run']['decision_applied'],dict(exit_code=code,failure=value.get('failure'))
        pid=last_body['run']['child_pid'];prompt=(root/('fixture-prompt-'+str(pid)+'.bin')).read_bytes();f.publish(out/'received-prompt.bin',prompt);assert review['id'].encode() in prompt and b'unresolved_review' in prompt
        assert after['tasks'][0]['next']==operation['next'] and len(a.public.finance(root,work))==len(money)+1 and all(row in a.public.finance(root,work) for row in money)
        f.publish(out/'result.json',dict(status='PASS_DECISION_ONLY',root=str(root),work=work,old_ack=ack_id,old_review=review['id'],old_ack_preserved=True,only_fresh_inputs_consumed=True,old_incident_received=True,actual_planner=True,second_review=False,public_cap_refused_and_restored=bool(args.cap_control),signature=dict(contract=f.digest(json.dumps({k:task.get(k) for k in ['title','deliverable','criteria','depends','plan_max_attempts']},sort_keys=True).encode()),settings=f.digest(json.dumps(json.loads((root/'.swarm/engine-settings.json').read_bytes())['values'],sort_keys=True).encode())),decision_event=last_body['run']['decision_event']));print('PASS acknowledged native review incident on a fresh activation',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
