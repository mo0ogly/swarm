#!/usr/bin/env python3
"""Run the actual managed native reviewer with an owned inert provider.

Real managed workers/candidate/three controls precede invocation; no positive SQL,
accepted task, paid model, automatic retry or global verification watchdog. Assert
whole-cohort original/certificate/context, one debit, replay and refusal/recovery.
"""
import argparse,json,os,subprocess,traceback,uuid
from pathlib import Path
import rust_native_managed_review_inputs as inspection
from rust_native_task_validation import invoke
from rust_native_reviewer import finance

PROGRAM=b'''#!/usr/bin/python3
import hashlib,json,os,sys
from pathlib import Path
raw=sys.stdin.buffer.read()
body=json.loads(raw.decode().rsplit('Do not use tools.\\n',1)[1]);protocol=body['managed_protocol'];context=protocol['context']
config=json.loads(Path('managed-review-fixture.json').read_text())
with Path('managed-reviewer-invoked.json').open('x') as f:
 json.dump(dict(pid=os.getpid(),ppid=os.getppid(),stdin_sha256=hashlib.sha256(raw).hexdigest(),context_sha256=protocol['context_sha256'],catalog_sha256=protocol['catalog_sha256'],native_invocation_supported=body['native_invocation_supported']),f);f.flush();os.fsync(f.fileno())
entries=[]
for task in context['tasks']:
 refs={span['reference'] for original in protocol['catalog']['originals'] if original['name']==task['binding']['report'] and original['kind']=='source' for span in original['spans']}
 ref=next(q['id'] for q in protocol['catalog']['quotes'] if q['id'] in refs and len(q['text'].encode())>=8)
 entries.append(dict(task=task['task'],reason='Current candidate inspected through actual native fixture',criteria=[dict(index=i+1,verdict='pass',evidence_ref=ref) for i,_ in enumerate(task.get('criteria') or [])]))
if config['case']=='partial':entries=[]
if config['case']=='publish-refusal':Path('.swarm/native-review-context-v1-'+protocol['context_sha256']+'.json').mkdir()
if config['case']=='source':Path(config['source']).write_text('Changed native reviewer candidate source\\n')
reply=dict(catalog_sha256=protocol['catalog_sha256'],candidate_commit=context['candidate_commit'],tasks=entries)
print(json.dumps(dict(type='item.completed',item=dict(type='agent_message',id='review',text=json.dumps(reply)))),flush=True)
print(json.dumps(dict(type='turn.completed',usage=dict(input_tokens=7,output_tokens=8))),flush=True)
'''


def run(binary,profile,out,case='pass',reuse=None,before_binary=None,pending_dependent=False):
    """Run one owned native lifecycle; preserve original refusal before recovery.

    Optional reuse names actual retained setup/control originals, never copied
    state. Old-binary refusal is tested before changing the inert program.
    pending_dependent is declared at fixture birth, never injected into retained
    state. All
    faults follow valid original inputs; mutable fixture bytes are restored in
    finally. Native opinion publication never accepts a task or settles billing.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False);fixture=inspection.native.fixture
    inspection.run(binary,profile,out/'inspection',reuse=reuse,context_bytes=0,pending_dependent=pending_dependent)
    setup=json.loads((out/'inspection/setup.json').read_bytes());root=Path(setup['root']);work=setup['work']
    environment=dict(os.environ,CODEX_HOME=str(root/'codex-home'));checks=[]
    def call(label,args,value=None,success=True,exe=binary):
        code,wire=invoke(exe,root,out,label,args,value,environment=environment)
        assert (code==0)==success,(label,wire)
        return wire
    def shown(label):return call(label,['work','show',work])['work']
    before=shown('before-review');request=dict(schema_version=1,event_id='managed-native-review-'+uuid.uuid4().hex,expected_revision=before['revision'],task_id='task')
    if before_binary:
        refused=call('old-native-guard',['task','review',work],request,False,exe=before_binary)
        assert 'review-managed-candidate-unported' in json.dumps(refused),refused
        assert shown('after-old-guard')==before
        checks.append('old-binary-managed-invocation-refusal-before-change')
    inputs=call('valid-inputs',['task','review-inputs',work],request);protocol=inputs['managed_protocol']
    source=Path(setup['workspace'])/'tracked.txt';source_before=source.read_bytes()
    # The only executable changed belongs to this isolated fixture; provider
    # config/identity comes from real public setup, never a fabricated SQL row.
    fixture.publish(root/'managed-review-fixture.json',dict(case=case,source=str(source)))
    fixture.publish(root/'codex',PROGRAM);(root/'codex').chmod(0o700)
    rows_before=finance(root,work)
    expected_success=case=='pass'
    try:
        result=call('native-review',['task','review',work],request,expected_success)
        after=shown('after-native');task=next(t for t in after['tasks'] if t['id']=='task');review=task['independent_review']
        marker=(root/'managed-reviewer-invoked.json').read_bytes();witness=json.loads(marker)
        assert witness['pid']>0 and witness['native_invocation_supported'] and witness['context_sha256']==protocol['context_sha256']
        assert after['planning']['reviewer']['calls']==before['planning']['reviewer'].get('calls',0)+1
        assert task['status']!='accepted'
        rows=finance(root,work);assert len(rows)==len(rows_before)+1 and any(row[0]==review['id'] and row[3]=='estimated' for row in rows)
        checks.append('actual-helper-provider-stream-and-one-budget-reservation-without-acceptance')
        private=root/'.swarm'/('native-review-'+fixture.digest(review['id'].encode())+'.receipt')
        frame=private.read_bytes();magic,size,digest,body=frame.split(b'\n',3)
        assert magic==b'SR1' and int(size)==len(body) and digest.decode()==fixture.digest(body)
        original=json.loads(body);assert original['waited'] and original['grant_attempted'] and original['channel_closed'] and original['schema_version']==2
        snapshot=original['managed_review'];assert snapshot['protocol']==protocol and snapshot['prompt_sha256']==witness['stdin_sha256']
        assert snapshot['protocol']['context_sha256']==protocol['context_sha256'] and (original['failed'] or original['original_reply'])
        fixture.publish(out/'retained-original.frame',frame)
        checks.append('framed-original-retains-full-cohort-protocol-frozen-policy-and-actual-wait')
        if case=='publish-refusal':
            assert review['state']=='running' and not original['failed']
            target=root/'.swarm'/('native-review-context-v1-'+protocol['context_sha256']+'.json');assert target.is_dir();target.rmdir()
            recovered=call('recover-original',['task','review-reconcile',review['id']]);assert recovered['review_recorded'] and recovered['verdict_applicable'] and not recovered['task_accepted']
            after=shown('after-recovery');review=next(t for t in after['tasks'] if t['id']=='task')['independent_review']
            checks.append('original-publication-refusal-recovers-after-owned-fault-removal-without-new-call')
        elif case=='source':
            assert source.read_bytes()!=source_before
            fixture.publish(source,source_before)
            recovered=call('source-restored-reconcile',['task','review-reconcile',review['id']]);assert recovered['verdict_applicable']==(not original['failed']) and not recovered['task_accepted']
            after=shown('after-source-restoration');review=next(t for t in after['tasks'] if t['id']=='task')['independent_review']
            checks.append('live-candidate-source-drift-refuses-and-restored-original-recovers-without-call')
        elif case=='partial':
            assert original['failed'] and original['original_reply'] and review['state']=='error' and not review.get('managed_tasks')
            recovered=call('failed-original-reconcile',['task','review-reconcile',review['id']]);assert recovered['replayed'] and not recovered['verdict_applicable']
            checks.append('partial-cohort-is-explicit-failed-call-not-partial-verdict')
        if not original['failed']:
            assert review['state']=='passed' and review['candidate_commit']==protocol['context']['candidate_commit']
            assert len(review['managed_tasks'])==len(protocol['context']['tasks'])
            context_file=root/review['context'];context_bytes=context_file.read_bytes();assert fixture.digest(context_bytes)==review['context_sha256']==protocol['context_sha256']
            assert json.loads(context_bytes)==protocol['context']
            checks.append('complete-cohort-candidate-and-content-addressed-context-published')
            # Valid replay precedes corruption. Retain exact owned bytes; no new
            # provider or refund follows a public-context corruption/refusal.
            call('context-valid',['task','review-reconcile',review['id']])
            try:
                fixture.publish(context_file,context_bytes+b' ')
                call('context-corruption-refusal',['task','review-reconcile',review['id']],success=False)
                assert shown('context-refused-work')==after
            finally:fixture.publish(context_file,context_bytes)
            call('context-restored',['task','review-reconcile',review['id']])
            checks.append('published-context-corruption-refuses-and-restores-without-state-change')
        replay=call('event-replay',['task','review',work],request);assert replay['replayed'] and replay['review_id']==review['id'] and not replay['task_accepted']
        assert shown('after-replay')==after and finance(root,work)==rows and (root/'managed-reviewer-invoked.json').read_bytes()==marker
        checks.append('exact-event-and-explicit-original-replay-never-rerun-provider-or-debit')
        fixture.publish(out/'result.json',dict(status='PASS',case=case,checks=checks,root=str(root),work=work,review_id=review['id'],paid_provider=False,accepted=False,model_authenticated=False,os_sandbox=False,global_watchdog=None))
        print('MANAGED_NATIVE_REVIEWER_PASS '+case+' '+str(len(checks))+' assertions',flush=True)
    finally:
        if source.read_bytes()!=source_before:fixture.publish(source,source_before)



def recover_original(binary,profile,out,previous):
    """Resume the actual retained successful call after generic publication fix.

    Requires owned original setup/marker and a current running review; public
    reconcile checks actual owner departure, wait, ledger, inputs and reservation.
    No SQL/receipt edits, new provider execution or consumption reset occurs.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False);fixture=inspection.native.fixture
    setup=json.loads((previous/'inspection/setup.json').read_bytes());root=Path(setup['root']);work=setup['work']
    request=json.loads((previous/'native-review.input.json').read_bytes());protocol=json.loads((previous/'valid-inputs.stdout').read_bytes())['managed_protocol']
    environment=dict(os.environ,CODEX_HOME=str(root/'codex-home'))
    def call(label,args,value=None):
        code,reply=invoke(binary,root,out,label,args,value,environment=environment);assert code==0,(label,reply);return reply
    before=call('before',['work','show',work])['work'];old=next(t for t in before['tasks'] if t['id']=='task')['independent_review'];assert old['state']=='running'
    marker=(root/'managed-reviewer-invoked.json').read_bytes();rows=finance(root,work);calls=before['planning']['reviewer']['calls']
    restored=call('recover-actual-original',['task','review-reconcile',old['id']]);assert restored['review_recorded'] and restored['verdict_applicable'] and not restored['task_accepted']
    after=call('after',['work','show',work])['work'];review=next(t for t in after['tasks'] if t['id']=='task')['independent_review']
    assert review['state']=='passed' and len(review['managed_tasks'])==len(protocol['context']['tasks'])
    assert review['context_sha256']==protocol['context_sha256'] and fixture.digest((root/review['context']).read_bytes())==review['context_sha256']
    for label,args,value in [('reconcile-replay',['task','review-reconcile',old['id']],None),('exact-request-replay',['task','review',work],request)]:
        result=call(label,args,value);assert result['replayed'] and result['verdict_applicable'] and not result['task_accepted']
    assert call('final',['work','show',work])['work']==after and after['planning']['reviewer']['calls']==calls and finance(root,work)==rows and (root/'managed-reviewer-invoked.json').read_bytes()==marker
    fixture.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,review_id=old['id'],original_from=str(previous),recovered_without_provider_or_debit=True,accepted=False,global_watchdog=None))
    print('MANAGED_NATIVE_ORIGINAL_RECOVERY_PASS',flush=True)


def main():
    """Freeze actual normal-profile binary and the maintained helper import closure."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--case',choices=['pass','partial','publish-refusal','source'],default='pass');parser.add_argument('--reuse-from',type=Path);parser.add_argument('--before-binary',type=Path);parser.add_argument('--recover-from',type=Path)
    a=parser.parse_args();out=a.output.resolve();fixture=inspection.native.fixture
    inputs=dict(binary=str(a.binary.resolve()),binary_sha256=fixture.digest(a.binary.read_bytes()),profile_sha256=fixture.digest(a.profile.read_bytes()),helpers=inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        if a.recover_from:recover_original(a.binary.resolve(),a.profile.resolve(),out,a.recover_from.resolve())
        else:run(a.binary.resolve(),a.profile.resolve(),out,a.case,reuse=a.reuse_from.resolve() if a.reuse_from else None,before_binary=a.before_binary.resolve() if a.before_binary else None)
        assert fixture.digest(a.binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(f).read_bytes())==h for f,h in inputs['helpers'].items())
        fixture.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.exists():fixture.publish(out/'inputs.json',inputs);fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
