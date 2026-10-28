#!/usr/bin/env python3
"""Require an actual native tool-free reviewer after valid public source controls.

Owned inert programs/Roots only, no paid provider or fake positive receipt/SQL.
The normal reviewer fixture reads a predeclared reply and writes an invocation
witness; it proves no model authenticity or OS sandbox. No global watchdog.
"""
import argparse,json,os,subprocess,sys,traceback
from types import SimpleNamespace
from rust_planning_budget import ledger
from pathlib import Path
import rust_agent_model_route as f
import rust_native_task_validation as native
import rust_native_review_preview as preview
import rust_native_review_inputs as inputs

PROGRAM=b'''#!/usr/bin/python3
import hashlib,json,os,sys
from pathlib import Path
raw=sys.stdin.buffer.read()
with Path('reviewer-invoked.json').open('x') as witness:
 json.dump(dict(pid=os.getpid(),ppid=os.getppid(),stdin_sha256=hashlib.sha256(raw).hexdigest()),witness);witness.flush();os.fsync(witness.fileno())
reply=Path('fixture-review-reply.json').read_text()
print(json.dumps(dict(type='item.completed',item=dict(type='agent_message',id='review',text=reply))),flush=True)
print(json.dumps(dict(type='turn.completed',usage=dict(input_tokens=7,output_tokens=8))),flush=True)
'''


def retain(root,out,work,review,binary_sha):
    """Copy an actual owned frame/certificate/program before successful completion.

    Check the Root/Work/binary marker before reading private evidence, verify
    framing/body digests, and preserve exact bytes. This is same-UID fixture
    provenance, not authenticated identity, a database backup or financial proof.
    Missing/malformed evidence refuses without reconstructing any positive receipt.
    """
    marker=json.loads((out/'fixture.json').read_text())
    if marker!=dict(root=str(root),work=work,binary_sha256=binary_sha):raise ValueError('review-retention-marker-mismatch')
    target=out/'originals';target.mkdir(exist_ok=False)
    paths={'original.frame':root/'.swarm'/('native-review-'+f.digest(review['id'].encode())+'.receipt'),'certificate.json':root/review['receipt'],'invocation.json':root/'reviewer-invoked.json','inert-provider.py':root/'codex','configured-reply.json':root/'fixture-review-reply.json'}
    hashes={}
    for name,path in paths.items():
        if path.is_symlink() or not path.is_file():raise ValueError('review-original-file-required:'+name)
        raw=path.read_bytes();f.publish(target/name,raw)
        if path.read_bytes()!=raw or (target/name).read_bytes()!=raw:raise ValueError('review-original-copy-drift:'+name)
        hashes[name]=f.digest(raw)
    frame=(target/'original.frame').read_bytes();magic,extent,digest,body=frame.split(b'\n',3)
    assert magic==b'SR1' and int(extent)==len(body) and digest.decode()==f.digest(body)
    receipt=json.loads(body);assert receipt['schema_version']==2 and receipt['job']['id']==review['id'] and receipt['job']['work_id']==work
    assert receipt['waited'] and receipt['grant_attempted'] and receipt['channel_closed']
    assert hashes['certificate.json']==review['receipt_sha256']
    f.publish(target/'retention.json',dict(files=hashes,frame_sha256=hashes['original.frame'],body_sha256=digest.decode(),root=str(root),work=work,review_id=review['id'],scope='Actual fixture originals, not authenticated principal or settlement'))
    return receipt


def finance(root,work):
    """Use the existing read-only ledger observer, preserving read/close causes."""
    return ledger(SimpleNamespace(root=root,work=work))


def scenario(binary,out,case,verify_event_conflict=False,child_task_budget=1,child_activation_budget=3):
    """Run one normal public reviewer after actual worker and control originals.

    Duration1s is an explicit saved product input, not a test watchdog. Source
    revocation and malformed/domain replies are faults only after a valid input
    witness. Failed calls must remain nonzero despite recorded error state;
    explicit reconcile/replay acknowledge that non-verdict without another exec.
    interrupted sends SIGTERM only to the inert reviewer itself after its witness;
    it does not change the candidate or signal any unrelated process.
    Originals are retained before PASS; changes_requested blocks correction,
    other outcomes keep the task submitted, and none accepts it. Child task and
    activation budgets are explicit saved delegation inputs, with legacy defaults
    one/three; extra fixture steps must be authorized before their execution.
    """
    root,work,request=inputs.source(binary,out/'source',child_task_budget=child_task_budget,child_activation_budget=child_activation_budget);sha=f.digest(binary.read_bytes())
    f.publish(out/'fixture.json',dict(root=str(root),work=work,binary_sha256=sha))
    inputs.observed(binary,root,out,'complete-inputs',work,request)
    reply=dict(reason='Owned fixture review, not independent human acceptance',criteria=[dict(index=1,verdict='fail' if case=='changes-requested' else 'pass',evidence='Observed fixture only; not authenticated model or task acceptance.')])
    f.publish(root/'fixture-review-reply.json',reply)
    program=PROGRAM
    if case in ('duration','source'):
        block=b"import signal\n"
        if case=='source':block+=b"Path('handoff.md').write_text('Owned negative fixture: source changed after reviewer admission')\n"
        block+=b"signal.pause()\nreply=Path('fixture-review-reply.json').read_text()"
        program=program.replace(b"reply=Path('fixture-review-reply.json').read_text()",block)
    elif case=='interrupted':program=program.replace(b"reply=Path('fixture-review-reply.json').read_text()",b"import signal\nos.kill(os.getpid(),signal.SIGTERM)\nreply=Path('fixture-review-reply.json').read_text()")
    elif case=='domain-invalid':program=program.replace(b"reply=Path('fixture-review-reply.json').read_text()",b"reply='PRIVATE invalid domain fixture'")
    elif case=='malformed-output':program=program.replace(b"reply=Path('fixture-review-reply.json').read_text()",b"print('PRIVATE invalid JSONL fixture',flush=True)\nreply=Path('fixture-review-reply.json').read_text()")
    if case=='duration':
        code,current=native.invoke(binary,root,out,'duration-show',['quotas','show',work]);assert code==0,current
        code,changed=native.invoke(binary,root,out,'duration-save',['quotas','apply',work],dict(schema_version=1,event_id='review-duration',expected_revision=current['revision'],limits=current['limits'],review_timeout_seconds=1,reason='Explicit owned reviewer deadline contract'));assert code==0,changed
        code,reopened=native.invoke(binary,root,out,'duration-reopen',['quotas','show',work]);assert code==0 and reopened['review_timeout_seconds']==1,reopened
    if case=='zero-capture':
        preview.saved_setting(binary,root,out,'capture-zero','agent_capture_max_bytes','0')
    f.publish(root/'codex',program);(root/'codex').chmod(0o700)
    before=preview.shown(binary,root,out,'before',work);request.update(event_id='native-review-public-call',expected_revision=before['revision'])
    environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
    if verify_event_conflict:
        # Only the operation event differs from the valid prepared request.
        collision=dict(request,event_id=json.loads((out/'source/controls.input.json').read_text())['event_id'])
        rows_before=finance(root,work)
        code,rejected=native.invoke(binary,root,out,'event-collision',['task','review',work],collision,environment=environment)
        assert code!=0 and preview.shown(binary,root,out,'event-collision-after',work)==before and finance(root,work)==rows_before and not (root/'reviewer-invoked.json').exists(),rejected
        assert 'event_conflict' in rejected['failure']['causes'],rejected
    original_report=(root/'handoff.md').read_bytes()
    code,result=native.invoke(binary,root,out,'review',['task','review',work],request,environment=environment)
    failed=case in ('duration','source','interrupted','domain-invalid','malformed-output')
    assert (code!=0)==failed,result
    after=preview.shown(binary,root,out,'after',work);task=after['tasks'][0];review=task['independent_review']
    state='error' if failed else 'changes_requested' if case=='changes-requested' else 'passed'
    assert task['status']==('blocked' if case=='changes-requested' else 'submitted') and review['state']==state,review
    assert after['planning']['reviewer']['calls']==before['planning']['reviewer'].get('calls',0)+1
    invoked=(root/'reviewer-invoked.json').read_bytes();assert json.loads(invoked)['pid']>0
    if not failed:assert result['review_recorded'] and not result['task_accepted'] and not result['replayed'] and result['verdict_applicable']
    rows=finance(root,work);assert any(r[0]==review['id'] and r[3]=='estimated' for r in rows),rows
    code,reconciled=native.invoke(binary,root,out,'reconcile',['task','review-reconcile',review['id']]);assert code==0,reconciled
    assert reconciled['replayed'] and reconciled['verdict_applicable']==(not failed) and not reconciled['task_accepted']
    code,replayed=native.invoke(binary,root,out,'review-replay',['task','review',work],request,environment=environment);assert code==0,replayed
    assert replayed['replayed'] and replayed['review_id']==review['id'] and replayed['verdict_applicable']==(not failed)
    assert preview.shown(binary,root,out,'after-replay',work)==after and (root/'reviewer-invoked.json').read_bytes()==invoked and finance(root,work)==rows
    # A missing original ID is not an empty successful recovery.
    code,rejected=native.invoke(binary,root,out,'missing-recovery',['task','review-reconcile','review-does-not-exist']);assert code!=0 and rejected['failure']['code']=='review-recovery-job-required',rejected
    assert preview.shown(binary,root,out,'after-missing',work)==after and finance(root,work)==rows
    receipt=retain(root,out,work,review,sha);assert receipt['failed']==failed and receipt['duration_expired']==(case=='duration') and receipt['source_interrupted']==(case=='source')
    if case=='interrupted':assert receipt['signal']==15 and receipt['exit_code'] is None and not receipt['output_complete'] and receipt['original_reply'] is None
    capture=receipt['raw_capture'];assert not capture['failed']
    if case=='zero-capture':assert capture['retained_bytes']==0 and capture['stdout']['observed_bytes']>0 and capture['stdout']['truncated']
    elif not failed:
        observed=bytes(capture['stdout']['retained']);assert observed.startswith(b'{') and b'running 1 test' not in observed and f.digest(observed)==capture['stdout']['sha256'] and len(observed)==capture['stdout']['observed_bytes']
    # Each corruption starts after a valid readback; restore exact original bytes.
    for label,path,expected in [('public',root/review['receipt'],'review-publication-recorded-public-changed'),('private',root/'.swarm'/('native-review-'+f.digest(review['id'].encode())+'.receipt'),None)]:
        code,control=native.invoke(binary,root,out,label+'-valid',['task','review-reconcile',review['id']]);assert code==0 and control['replayed'],control
        original=path.read_bytes();changed=bytearray(original);changed[-1]^=1
        try:
            f.publish(path,bytes(changed))
            code,refused=native.invoke(binary,root,out,label+'-corrupted',['task','review-reconcile',review['id']]);assert code!=0,refused
            if expected:assert refused['failure']['code']==expected,refused
            else:assert any(x.startswith('storage-operation:') for x in refused['failure']['causes']),refused
            assert preview.shown(binary,root,out,label+'-unchanged',work)==after and finance(root,work)==rows
        finally:f.publish(path,original)
        code,restored=native.invoke(binary,root,out,label+'-restored',['task','review-reconcile',review['id']]);assert code==0 and restored['replayed'] and restored['verdict_applicable']==(not failed),restored
        assert (root/'reviewer-invoked.json').read_bytes()==invoked
    # A regular stdin cannot be a Unix grant socket or be overwritten by diagnostics.
    gate_input=out/'regular-stdin';f.publish(gate_input,b'Unchanged owned regular stdin fixture')
    with gate_input.open('r+b') as stream:r=subprocess.run([str(binary),'_reviewer_gate',review['id'],'--root',str(root),'--json'],stdin=stream,capture_output=True)
    f.publish(out/'regular-gate.stdout',r.stdout);f.publish(out/'regular-gate.stderr',r.stderr)
    assert r.returncode!=0 and json.loads(r.stderr)['failure']['code']=='review-gate-channel-kind' and gate_input.read_bytes()==b'Unchanged owned regular stdin fixture' and (root/'reviewer-invoked.json').read_bytes()==invoked
    assert preview.shown(binary,root,out,'after-private-refusal',work)==after and finance(root,work)==rows
    if case=='source':
        f.publish(root/'handoff.md',original_report)
        code,historical=native.invoke(binary,root,out,'restored-reconcile',['task','review-reconcile',review['id']]);assert code==0 and not historical['verdict_applicable'] and historical['replayed'],historical
        assert preview.shown(binary,root,out,'after-restored',work)==after and finance(root,work)==rows and (root/'reviewer-invoked.json').read_bytes()==invoked
    return dict(status='PASS',case=case,root=str(root),work=work,review_id=review['id'],state=state,one_native_reviewer_call=True,replay_without_call=True,explicit_recovery=True,original_retained=True,public_private_corruption_refused_and_restored=True,regular_stdin_refused=True,reservation_preserved=True,task_accepted=False,paid_provider=False,model_authenticated=False,os_sandbox=False,binary_sha256=sha,global_watchdog=None)


def main():
    """Freeze normal executable/method inputs and retain each original before PASS.

    Optional profile qualification must match bytes and exclude fixture features.
    Fresh output and Root per case, no global watchdog, mutation of a live Root,
    positive SQL seeding, paid provider or independent-acceptance claim.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--profile',type=Path);p.add_argument('--verify-event-conflict',action='store_true');p.add_argument('--child-task-budget',type=int,default=1);p.add_argument('--child-activation-budget',type=int,default=3);p.add_argument('--case',choices=['passed','changes-requested','domain-invalid','malformed-output','duration','source','interrupted','zero-capture'],default='passed');a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    raw=a.binary.resolve().read_bytes();binary=out/'binary';f.publish(binary,raw);binary.chmod(0o500)
    methods={}
    for module in list(sys.modules.values()):
        file=getattr(module,'__file__',None)
        if file and Path(file).resolve().parent==Path(__file__).resolve().parent:
            path=Path(file).resolve();methods[str(path)]=f.digest(path.read_bytes());f.publish(out/('method-'+path.name),path.read_bytes())
    f.publish(out/'inputs.json',dict(binary_sha256=f.digest(raw),methods=methods,global_watchdog=None))
    try:
        if a.profile:
            profile=json.loads(a.profile.read_text());assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['all_core_test_false'] and profile['all_core_features_empty'] and profile['no_fixture_feature'] and profile['sha256']==f.digest(raw)
        result=scenario(binary,out,a.case,a.verify_event_conflict,a.child_task_budget,a.child_activation_budget)
        assert all(f.digest(Path(path).read_bytes())==sha for path,sha in methods.items()) and binary.read_bytes()==raw
        result['event_collision_refused_with_safe_cause']=a.verify_event_conflict
        f.publish(out/'result.json',result);print('PASS normal reviewer '+a.case+'; original retained, no recovery exec/acceptance',flush=True)
    except BaseException:
        f.publish(out/'failure.json',dict(status='FAIL',case=a.case,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
