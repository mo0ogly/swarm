#!/usr/bin/env python3
"""Actual second reviewer after canonical correction, without producer restart.

Owned native inert programs only, no positive SQL/receipt seed, paid provider or
watchdog. Distinguish original controls from freshly recomputed admission source;
normal planning churn may leave equal hashes, explicit pause/resume changes it.
"""
import argparse,json,os,subprocess,sys,traceback
from pathlib import Path
import rust_native_review_acceptance as a
from rust_native_review_decision import rows
from rust_native_production_retry import negative_frame,negative_event


def main():
    """Keep all originals, counters, physical witnesses and explicit acceptance.

    Healthy full execution of the same binary/settings/task/authority variant is
    mandatory before negative canonical/private/control/report corruption. Every
    refusal precedes claim/call/mutation and restores exact bytes. Only our Root
    is paused/resumed. --acknowledged runs native ACK and fresh-input correction
    first, preserving the old decision and using initial explicit child capacity.
    New review finishes unaccepted; a separate operator action
    then proves acceptance without reviewer rerun. That is fixture-only state,
    never human identity, real model judgement or global migration acceptance.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--profile',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--renew-authority',action='store_true');p.add_argument('--interrupted',action='store_true',help='Use a known terminal signal instead of malformed response');p.add_argument('--acknowledged',action='store_true',help='Recover an already acknowledged incident on fresh input');p.add_argument('--faults',action='store_true');p.add_argument('--control',type=Path);args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);root=None;f=a.public.f;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=args.binary.resolve();common=[sys.executable,str(Path(__file__).with_name('rust_native_review_protocol_retry.py')),'--binary',str(binary),'--profile',str(args.profile.resolve())]
        if args.acknowledged:
            ack=[sys.executable,str(Path(__file__).with_name('rust_acknowledged_review_protocol.py')),'--binary',str(binary),'--profile',str(args.profile.resolve()),'--output',str(out/'source-decision')]
            if args.interrupted:ack+=['--review-case','interrupted']
            if args.faults:
                assert args.control is not None;ack+=['--cap-control',str(args.control.resolve())]
            subprocess.run(ack,check=True)
        else:
            subprocess.run(common+['--output',str(out/'source-control')],check=True)
            subprocess.run(common+['--output',str(out/'source-decision'),'--error','--retry','--control',str(out/'source-control/result.json')]+(['--failure-case','interrupted'] if args.interrupted else []),check=True)
        setup=json.loads((out/'source-decision/result.json').read_bytes());assert setup['status']=='PASS_DECISION_ONLY';root=Path(setup['root']);work=setup['work'];environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
        def invoke(label,command,request=None):
            return a.public.native.invoke(binary,root,out,label,command,request,environment=environment)
        before=a.public.preview.shown(binary,root,out,'before-second',work);task=before['tasks'][0];old=task['independent_review'];old_frame=root/'.swarm'/('native-review-'+f.digest(old['id'].encode())+'.receipt');old_raw=old_frame.read_bytes();old_body=json.loads(old_raw.split(b'\n',3)[3]);original_report=(root/'handoff.md').read_bytes();old_witness=(root/'reviewer-invoked.json').read_bytes();worker_witness=(root/'model-received.json').read_bytes()
        if args.renew_authority:
            for action in ['pause','resume']:
                code,current=invoke('authority-'+action,['planning',action,work],dict(schema_version=1,event_id='protocol-authority-'+action,expected_revision=before['revision'],reason='Owned new control epoch; preserve checked candidate and correction'));assert code==0,current;before=current
        task=before['tasks'][0]
        signature=dict(binary_sha256=f.digest(binary.read_bytes()),setup=setup['signature'],renew_authority=args.renew_authority,acknowledged=args.acknowledged,interrupted=args.interrupted)
        if args.faults:
            assert args.control is not None;control=json.loads(args.control.read_bytes());assert control['status']=='PASS' and not control['faults'] and control['signature']==signature and control['actual_second_reviewer'],control
        program=a.public.PROGRAM.replace(b'reviewer-invoked.json',b'retry-reviewer-invoked.json').replace(b'raw=sys.stdin.buffer.read()',b"raw=sys.stdin.buffer.read()\nwith Path('retry-reviewer-prompt.bin').open('xb') as received:\n received.write(raw);received.flush();os.fsync(received.fileno())")
        f.publish(root/'codex',program);(root/'codex').chmod(0o700);f.publish(out/'second-reviewer-program.py',program)
        request=dict(schema_version=1,event_id='protocol-second-review',expected_revision=before['revision'],task_id='task')
        money=a.public.finance(root,work);agents=rows(root,'SELECT id,status FROM agents WHERE work_id=? ORDER BY id',(work,));jobs=rows(root,'SELECT id,phase FROM native_review_jobs WHERE work_id=? ORDER BY id',(work,))
        code,inputs=invoke('valid-inputs',['task','review-inputs',work],request);assert code==0 and not inputs['preview']['invocation_authorized'],inputs
        if args.renew_authority:assert inputs['admission_source_sha256']!=inputs['preview']['validation']['source_sha256'],'Public resume must change fresh admission, not historical controls'
        if args.faults:
            controls=list((root/'.swarm').glob('native-validation-step-*.receipt'));assert len(controls)==1
            for label,path,reason in [('old-private',old_frame,'storage-operation:Read'),('original-control',controls[0],'storage-operation:Read'),('report',root/'handoff.md','review-recovery-artifact-changed')]:
                original=path.read_bytes();changed=bytearray(original);changed[-1]^=1;f.publish(out/(label+'-original.bytes'),original)
                with negative_frame(path,original,bytes(changed)):
                    code,refused=invoke(label+'-refused',['task','review',work],request);assert code!=0 and reason in refused['failure']['causes'],refused
                    assert a.public.preview.shown(binary,root,out,label+'-unchanged',work)==before and a.public.finance(root,work)==money and rows(root,'SELECT id,phase FROM native_review_jobs WHERE work_id=? ORDER BY id',(work,))==jobs and not (root/'retry-reviewer-invoked.json').exists()
            decision=setup['decision_event'] if args.acknowledged else json.loads((out/'source-decision/planner-original.frame').read_bytes().split(b'\n',3)[3])['run']['decision_event'];original=rows(root,'SELECT request FROM events WHERE id=? AND work_id=?',(decision,work))[0][0];changed=json.loads(original);changed['operations'][0]['next']='Changed canonical reviewer correction must not authorize this unchanged task';changed=json.dumps(changed,separators=(',',':'),ensure_ascii=False).encode();f.publish(out/'canonical-decision-original.json',original)
            with negative_event(root,work,decision,original,changed):
                code,refused=invoke('canonical-refused',['task','review',work],request);assert code!=0 and 'native-retry-decision-changed' in refused['failure']['causes'],refused
                assert a.public.preview.shown(binary,root,out,'canonical-unchanged',work)==before and a.public.finance(root,work)==money and rows(root,'SELECT id,phase FROM native_review_jobs WHERE work_id=? ORDER BY id',(work,))==jobs and not (root/'retry-reviewer-invoked.json').exists()
        if args.faults and args.acknowledged:
            from rust_acknowledged_review_receipt_faults import negative_receipt
            canonical=json.loads(rows(root,'SELECT request FROM events WHERE id=? AND work_id=?',(setup['decision_event'],work))[0][0])
            claim,context,digest=rows(root,'SELECT claim_id,context_json,context_sha256 FROM native_planning_claims WHERE work_id=? AND scope_id=? AND generation=?',(work,'child',canonical['generation']))[0];f.publish(out/'original-received-context.json',context)
            for label in ['receipt-digest','old-ack','fresh-input','wrong-kind','wrong-owner']:
                value=json.loads(context)
                if label=='old-ack':value['unresolved_review_events'][0]['decision']='different-ack'
                if label=='fresh-input':value['events']=[]
                if label=='wrong-kind':
                    route=value['review_failure_routing']['task'];route['failure_code']='review_fragment_protocol' if args.interrupted else 'review_fragment_interrupted';route['owner']='reviewer_protocol' if args.interrupted else 'reviewer_runtime'
                if label=='wrong-owner':value['review_failure_routing']['task']['owner']='reviewer_protocol' if args.interrupted else 'reviewer_runtime'
                changed=json.dumps(value,separators=(',',':'),ensure_ascii=False)
                changed_digest='0'*64 if label=='receipt-digest' else f.digest(changed.encode())
                with negative_receipt(root,work,claim,(context,digest),(changed,changed_digest)):
                    code,refused=invoke(label+'-refused',['task','review',work],request);assert code!=0 and any(c.startswith('native-review-retry-') for c in refused['failure']['causes']),refused
                    assert a.public.preview.shown(binary,root,out,label+'-unchanged',work)==before and a.public.finance(root,work)==money and rows(root,'SELECT id,phase FROM native_review_jobs WHERE work_id=? ORDER BY id',(work,))==jobs and not (root/'retry-reviewer-invoked.json').exists()
        code,result=invoke('second-review',['task','review',work],request);after=a.public.preview.shown(binary,root,out,'after-second',work);f.publish(out/'second-observation.json',dict(exit=code,result=result,root=str(root),work=work));assert code==0 and result['review_recorded'] and result['verdict_applicable'] and not result['task_accepted'],result
        current=after['tasks'][0];review=current['independent_review'];assert current['status']=='submitted' and review['state']=='passed' and review['id']!=old['id'] and current['previous_reviews'][-1]==old and current['attempts']==task['attempts'] and current['automatic_validation']==task['automatic_validation'] and current['next']==task['next'] and len(after['tasks'])==1
        assert after['planning']['reviewer']['calls']==before['planning']['reviewer']['calls']+1 and rows(root,'SELECT id,status FROM agents WHERE work_id=? ORDER BY id',(work,))==agents
        newmoney=a.public.finance(root,work);assert len(newmoney)==len(money)+1 and all(row in newmoney for row in money)
        new_frame=root/'.swarm'/('native-review-'+f.digest(review['id'].encode())+'.receipt');raw=new_frame.read_bytes();magic,extent,sha,body=raw.split(b'\n',3);assert magic==b'SR1' and int(extent)==len(body) and f.digest(body)==sha.decode();original=json.loads(body);assert original['waited'] and original['grant_attempted'] and original['channel_closed'] and original['output_complete'] and not original['failed']
        witness=json.loads((root/'retry-reviewer-invoked.json').read_bytes());prompt=(root/'retry-reviewer-prompt.bin').read_bytes();assert witness['pid']==original['job']['child_pid'] and witness['ppid']==original['job']['parent_pid'] and witness['stdin_sha256']==f.digest(prompt)==original['job']['prompt_sha256']==inputs['prompt_sha256']
        assert b'review_recovery is attributed reviewer-correction context' in prompt
        directive=json.loads(prompt.decode().split('do not use tools.\n',1)[1])['review_recovery'];assert directive['review']==old['id'] and directive['instruction']==task['next'] and directive['attempt']==old['attempt'] and directive['report_sha256']==old['sha256'] and directive['original_body_sha256']==f.digest(old_raw.split(b'\n',3)[3]) and 'PRIVATE invalid domain fixture' not in prompt.decode()
        assert (directive['failure_code'],directive['failure_owner'])==(('review_fragment_interrupted','reviewer_runtime') if args.interrupted else ('review_fragment_protocol','reviewer_protocol'))
        canonical_id=directive['decision'];canonical_bytes=rows(root,'SELECT request FROM events WHERE id=? AND work_id=? AND kind=?',(canonical_id,work,'planning.decide'))[0][0];assert directive['decision_request_sha256']==f.digest(canonical_bytes)
        if args.acknowledged:
            assert canonical_id==setup['decision_event'] and canonical_id!=setup['old_ack']
            assert next(e for e in after['planning']['inbox'] if e['id']==old['id'])['decision']==setup['old_ack']
        assert original['job']['source_sha256']==inputs['admission_source_sha256'] and original['job']['validation_receipt_sha256']==old_body['job']['validation_receipt_sha256'] and (root/'handoff.md').read_bytes()==original_report and old_frame.read_bytes()==old_raw and (root/'reviewer-invoked.json').read_bytes()==old_witness and (root/'model-received.json').read_bytes()==worker_witness
        for name,data in [('new-review.frame',raw),('old-review.frame',old_raw),('new-reviewer-witness.json',(root/'retry-reviewer-invoked.json').read_bytes()),('new-reviewer-prompt.bin',prompt),('new-certificate.json',(root/review['receipt']).read_bytes())]:f.publish(out/name,data)
        witness_before=(root/'retry-reviewer-invoked.json').read_bytes();code,replay=invoke('second-review-replay',['task','review',work],request);assert code==0 and replay['replayed'] and replay['review_id']==review['id'] and a.public.preview.shown(binary,root,out,'after-review-replay',work)==after and a.public.finance(root,work)==newmoney and (root/'retry-reviewer-invoked.json').read_bytes()==witness_before
        a.accept(binary,root,work,out,'explicit-operator-acceptance-after-retry',True);assert (root/'retry-reviewer-invoked.json').read_bytes()==witness_before
        f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,signature=signature,faults=args.faults,pre_claim_faults=(9 if args.acknowledged else 4) if args.faults else 0,public_cap_refused_and_restored=bool(args.faults and args.acknowledged),actual_second_reviewer=True,same_producer_attempt=True,original_review_and_controls_preserved=True,fresh_admission_recomputed=True,changed_authority_source_distinct=args.renew_authority,actual_correction_received=True,actual_failure_category_received=True,old_spending_preserved=True,replayed_without_exec=True,second_result_initially_unaccepted=True,explicit_operator_acceptance_fixture_only=True,authenticated_human=False,real_model=False,global_watchdog=None));print('PASS actual full native review protocol retry with preserved producer and controls',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
