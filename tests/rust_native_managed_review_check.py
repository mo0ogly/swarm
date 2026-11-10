#!/usr/bin/env python3
"""Check supplied managed JSONL through the readonly public native command.

Reuses real isolated controls and their held candidate, never a live mission or
paid provider. Successful parsing is opinion data, not execution or acceptance.
No global watchdog; retain every public invocation and original refusal.
"""
import argparse,copy,json,os,traceback,uuid
from pathlib import Path
import rust_native_managed_protocol as protocol_fixture
from rust_native_task_validation import invoke
from rust_native_managed_candidate import job_rows


def run(binary,profile,out,reuse=None):
    """Exercise current prompt binding, whole-cohort output and readonly refusals.

    The upstream journey separately checks source custody, original controls,
    lossless citations and persisted Admin policy. Work state must remain exactly
    unchanged for every supplied output, including a syntactically valid pass.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    protocol_fixture.run(binary,profile,out/'protocol',reuse=reuse)
    setup=json.loads((out/'protocol/inspection/setup.json').read_bytes())
    root=Path(setup['root']);work=setup['work'];checks=[]
    environment=dict(os.environ,CODEX_HOME=str(root/'codex-home'))
    def call(label,args,value=None,success=True):
        code,wire=invoke(binary,root,out,label,args,value,environment=environment)
        assert (code==0)==success,(label,wire)
        return wire
    def state(label):return call(label,['work','show',work])['work']
    before=state('check-before');jobs_before=job_rows(root)
    markers_before={p.name:protocol_fixture.inspection.native.fixture.digest(p.read_bytes()) for p in root.glob('model-received*')}
    request=dict(schema_version=1,event_id='review-check-'+uuid.uuid4().hex,expected_revision=before['revision'],task_id='task')
    inputs=call('check-inputs',['task','review-inputs',work],request)
    protocol=inputs['managed_protocol'];context=protocol['context']
    reply=dict(catalog_sha256=protocol['catalog_sha256'],candidate_commit=context['candidate_commit'],tasks=[dict(task=t['task'],reason='Changes required from supplied observations',criteria=[dict(index=i+1,verdict='unknown',evidence='Insufficient runtime evidence for this criterion') for i,_ in enumerate(t['criteria'])]) for t in context['tasks']])
    def events(value,terminal=True):
        raw=json.dumps(dict(type='item.completed',item=dict(type='agent_message',id='reply',text=value if isinstance(value,str) else json.dumps(value))))+'\n'
        if terminal:raw+=json.dumps(dict(type='turn.completed',usage=dict(input_tokens=7,output_tokens=8)))+'\n'
        return raw
    def check(label,value,success=True,**patch):
        supplied=dict(request,prompt_sha256=inputs['prompt_sha256'],output=value);supplied.update(patch)
        result=call(label,['task','review-check',work],supplied,success)
        assert state(label+'-after')==before and job_rows(root)==jobs_before
        assert {p.name:protocol_fixture.inspection.native.fixture.digest(p.read_bytes()) for p in root.glob('model-received*')}==markers_before
        return result
    result=check('valid-unknown',events(reply))
    assert result['supplied_output_checked'] and result['tasks'] and result['opinion_state']=='changes_requested'
    assert result['context_sha256']==protocol['context_sha256'] and result['catalog_sha256']==protocol['catalog_sha256']
    for key in ['provider_started','review_recorded','native_execution_proven','reviewer_authenticated','verdict_applicable','task_accepted']:assert result[key] is False,key
    checks.append('valid-complete-opinion-is-data-without-work-mutation-or-authority')
    settings=call('check-settings',['engine-settings','show'])
    values=settings['values'];minimum=int(values['review_reply_min_text_bytes']);maximum=values['review_reply_max_text_bytes']
    source_names={source['path'] for source in context['sources']}
    eligible={span['reference'] for original in protocol['catalog']['originals'] if original['kind']=='source' and original['name'] in source_names for span in original['spans']}
    reference=next(q['id'] for q in protocol['catalog']['quotes'] if q['id'] in eligible and len(q['text'].encode())>=minimum and (maximum is None or len(q['text'].encode())<=int(maximum)))
    passing=copy.deepcopy(reply)
    for task in passing['tasks']:
        for criterion in task['criteria']:
            criterion.pop('evidence');criterion.update(verdict='pass',evidence_ref=reference)
    passed=check('valid-supplied-pass',events(passing))
    assert passed['opinion_state']=='passed' and not passed['task_accepted'] and not passed['verdict_applicable']
    checks.append('valid-source-citation-pass-grants-no-acceptance')
    check('stale-prompt',events(reply),False,prompt_sha256='0'*64)
    checks.append('original-prompt-sha-required')
    source=Path(setup['workspace'])/'tracked.txt';saved_source=source.read_bytes()
    source.write_bytes(b'Changed tracked candidate bytes\n')
    try:check('changed-raw-source',events(reply),False)
    finally:source.write_bytes(saved_source)
    check('raw-source-restored',events(reply))
    checks.append('held-original-source-drift-refuses-and-restores')
    for label,patch in [('wrong-candidate',dict(candidate_commit='0'*40)),('wrong-catalog',dict(catalog_sha256='0'*64)),('missing-cohort',dict(tasks=[])),('duplicate-task',dict(tasks=reply['tasks']*2))]:
        check(label,events(dict(reply,**patch)),False)
    checks.append('candidate-catalogue-whole-cohort-and-uniqueness-refuse')
    bad=copy.deepcopy(reply);bad['tasks'][0]['criteria'][0]['index']=2
    check('wrong-criterion',events(bad),False)
    check('duplicate-field',events(json.dumps(reply)[:-1]+',"tasks":[]}'),False)
    check('alias-field',events(dict(reply,Tasks=reply['tasks'])),False)
    checks.append('canonical-fields-and-criterion-index-required')
    check('missing-terminal',events(reply,False),False)
    tool=json.dumps(dict(type='item.started',item=dict(type='command_execution',id='tool',command='echo forbidden')))+'\n'
    check('tool-event',tool+events(reply),False)
    checks.append('tool-free-complete-supplied-stream-required')
    def save_cap(label,value):
        """Apply/read back saved response policy via the public Admin CAS path."""
        doc=call(label+'-show',['engine-settings','show'])
        body=dict(schema_version=doc['schema_version'],event_id=label+'-'+uuid.uuid4().hex,expected_revision=doc['revision'],values=dict(doc['values'],review_response_max_bytes=value),reason='Isolated saved reply cap witness',preview_token='')
        preview=call(label+'-preview',['engine-settings','preview'],body);body['preview_token']=preview['preview_token']
        call(label+'-apply',['engine-settings','apply'],body)
        assert call(label+'-reopen',['engine-settings','show'])['values']==body['values']
    original_cap=values['review_response_max_bytes']
    save_cap('zero-reply-cap','0')
    try:check('saved-zero-cap-refuses',events(reply),False)
    finally:save_cap('restore-reply-cap',original_cap)
    check('reply-cap-restored',events(reply))
    checks.append('saved-admin-response-cap-refusal-and-restoration')
    malformed=dict(request,prompt_sha256=inputs['prompt_sha256'],output=events(reply),unexpected=True)
    call('unknown-envelope-field',['task','review-check',work],malformed,False)
    secret='PRIVATE-INVALID-SCALAR-MUST-NOT-APPEAR'
    malformed=dict(request,prompt_sha256=inputs['prompt_sha256'],output=events(reply),expected_revision=secret)
    denied=call('invalid-scalar-safe-cause',['task','review-check',work],malformed,False)
    assert secret not in json.dumps(denied)
    guard=call('native-review-remains-guarded',['task','review',work],dict(request,expected_revision=before['revision']+1),False)
    assert 'review-revision-conflict' in json.dumps(guard)
    assert state('final-state')==before and job_rows(root)==jobs_before
    checks.append('strict-request-and-readonly-final-state')
    fixture=protocol_fixture.inspection.native.fixture
    fixture.publish(out/'result.json',dict(status='PASS',checks=checks,root=str(root),work=work,provider_started=False,accepted=False,global_watchdog=None))
    print('MANAGED_REVIEW_CHECK_PASS '+str(len(checks))+' assertions')


def main():
    """Bind named assertions/refusals to immutable binary/profile/helper bytes."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--reuse-from',type=Path);a=parser.parse_args();out=a.output.resolve()
    fixture=protocol_fixture.inspection.native.fixture
    inputs=dict(binary=str(a.binary.resolve()),binary_sha256=fixture.digest(a.binary.read_bytes()),profile_sha256=fixture.digest(a.profile.read_bytes()),helpers=protocol_fixture.inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(a.binary.resolve(),a.profile.resolve(),out,reuse=a.reuse_from.resolve() if a.reuse_from else None)
        assert fixture.digest(a.binary.read_bytes())==inputs['binary_sha256'] and fixture.digest(a.profile.read_bytes())==inputs['profile_sha256'] and all(fixture.digest(Path(n).read_bytes())==h for n,h in inputs['helpers'].items())
        fixture.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.exists():
            fixture.publish(out/'inputs.json',inputs)
            fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
