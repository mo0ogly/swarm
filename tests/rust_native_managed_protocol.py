#!/usr/bin/env python3
"""Verify the real readonly managed citation protocol without final-review authority.

Reuses the maintained managed-input journey and its real controls, then checks
canonical cohort/context, lossless catalog, reply schema and saved Admin policy
mutation/restoration. All Roots are isolated; no SQL write, paid model, acceptance,
provider call or global watchdog. The parser itself is covered by named Rust tests.
"""
import argparse,copy,json,os,traceback,uuid
from pathlib import Path
import jsonschema
import rust_native_managed_review_inputs as inspection
from rust_native_task_validation import invoke


def run(binary,profile,out,reuse=None):
    """Exercise public protocol output and reject regressions with raw evidence.

    A reused Root keeps real originals rather than copying/fabricating accepted
    state. JSON Schema structural checks are separate from exact byte/quote and
    unique-task/criterion validation enforced by the Rust parser.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    fixture=inspection.native.fixture
    inspection.run(binary,profile,out/'inspection',reuse=reuse,context_bytes=0)
    setup=json.loads((out/'inspection/setup.json').read_bytes());root=Path(setup['root']);work=setup['work']
    environment=dict(os.environ,CODEX_HOME=str(root/'codex-home'));checks=[];namespace=uuid.uuid4().hex
    def ok(label,args,value=None):
        code,wire=invoke(binary,root,out,label,args,value,environment=environment)
        assert code==0,(label,wire)
        return wire
    def show(label):return ok(label,['work','show',work])['work']
    def inputs(label):
        before=show(label+'-before');wire=ok(label,['task','review-inputs',work],dict(schema_version=1,event_id=label,expected_revision=before['revision'],task_id='task'))
        assert show(label+'-after')==before and not wire['preview']['provider_started'] and not wire['preview']['task_accepted']
        return wire
    original=inputs('protocol');protocol=original['managed_protocol'];context=protocol['context'];catalog=protocol['catalog']
    assert context['inspection_only'] and context['inspection_scope_version']==1 and context['citation_contract_version']==2 and 'accepted_baseline' not in context
    assert protocol['context_sha256']==catalog['context_sha256'] and context['candidate_commit']==setup['candidate']['commit']==catalog['candidate_commit']
    assert len(context['tasks'])==1 and context['tasks'][0]['task']=='task' and len(context['tasks'][0]['controls'])==3
    task=next(t for t in show('binding-work')['tasks'] if t['id']=='task');binding=context['tasks'][0]['binding']
    assert binding['attempt']==task['attempts'][-1]['id'] and binding['producer']==original['preview']['producer_agent_id']
    materials={s['name']:s for s in original['materials']}
    quotable={name:s for name,s in materials.items() if s['kind'] in ['report','document']}
    assert len(context['sources'])==len(quotable) and {s['path'] for s in context['sources']}==set(quotable)
    assert not {'candidate-cohort.json','engine-controls.json'} & {s['path'] for s in context['sources']}
    assert all(s['content']==materials[s['path']]['content'] and s['sha256']==materials[s['path']]['sha256'] and s['bytes']==len(s['content'].encode()) for s in context['sources'])
    body=json.loads(original['prompt'].rsplit('Do not use tools.\n',1)[1]);assert body['managed_protocol']==protocol
    checks.append('canonical-cohort-protocol-in-public-inputs-and-complete-prompt')
    def go_json(value):
        """Encode Go-compatible string escaping for retained catalogue spans only."""
        return json.dumps(value,ensure_ascii=False,separators=(',',':')).replace('<','\\u003c').replace('>','\\u003e').replace('&','\\u0026').replace('\u2028','\\u2028').replace('\u2029','\\u2029')
    quotes={q['id']:q['text'] for q in catalog['quotes']};assert list(quotes)==list(range(len(quotes)))
    def reconstruct(entry):
        raw=b''
        for span in entry['spans']:
            piece=quotes[span['reference']]
            if span.get('json_string'):piece=go_json(piece)[1:-1]
            assert span['byte_offset']==len(raw) and span['bytes']==len(piece.encode())
            raw+=piece.encode()
        assert len(raw)==entry['bytes'] and fixture.digest(raw)==entry['sha256']
        return raw.decode()
    originals={(entry['name'],entry['kind']):reconstruct(entry) for entry in catalog['originals']}
    for s in context['sources']:assert originals[(s['path'],'source')]==s['content']
    assert not any(name in ['candidate-cohort.json','engine-controls.json'] for name,kind in originals)
    # Canonical field order is the engine's struct declaration, not object-key order.
    ordered=dict(version=catalog['version'],candidate_commit=catalog['candidate_commit'],context_sha256=catalog['context_sha256'],originals=[],quotes=[dict(id=q['id'],text=q['text']) for q in catalog['quotes']])
    for e in catalog['originals']:
        spans=[]
        for s in e['spans']:
            item={}
            if s.get('json_string'):item['json_string']=True
            item.update(reference=s['reference'],byte_offset=s['byte_offset'],bytes=s['bytes']);spans.append(item)
        ordered['originals'].append(dict(name=e['name'],kind=e['kind'],sha256=e['sha256'],bytes=e['bytes'],spans=spans))
    assert fixture.digest(go_json(ordered).encode())==protocol['catalog_sha256']
    checks.append('every-catalogue-original-reconstructs-exact-bytes-and-digests')
    schema=protocol['reply_schema'];validator=jsonschema.Draft202012Validator(schema);jsonschema.Draft202012Validator.check_schema(schema)
    reply=dict(catalog_sha256=protocol['catalog_sha256'],candidate_commit=context['candidate_commit'],tasks=[dict(task=t['task'],reason='Observed required changes from supplied source data',criteria=[dict(index=i+1,verdict='unknown',evidence='Insufficient runtime evidence for this criterion') for i,_ in enumerate(t['criteria'])]) for t in context['tasks']])
    assert not list(validator.iter_errors(reply))
    for patch in [dict(tasks=[]),dict(candidate_commit='wrong'),dict(catalog_sha256='0'*64),dict(extra=True)]:
        bad=dict(reply,**patch);assert list(validator.iter_errors(bad))
    bad=copy.deepcopy(reply);bad['tasks'][0]['criteria'][0]['evidence_ref']=0;assert list(validator.iter_errors(bad))
    checks.append('reply-schema-valid-control-and-malformed-cohort-refusals')
    def saved(label,changes):
        """Persist real Admin policy via public preview/CAS/apply and exact readback."""
        doc=ok(label+'-show',['engine-settings','show']);values=dict(doc['values'],**changes)
        request=dict(schema_version=doc['schema_version'],event_id=label+'-'+namespace,expected_revision=doc['revision'],values=values,reason='Isolated citation admission policy witness',preview_token='')
        preview=ok(label+'-preview',['engine-settings','preview'],request);request['preview_token']=preview['preview_token']
        ok(label+'-apply',['engine-settings','apply'],request);after=ok(label+'-reopen',['engine-settings','show'])
        assert after['values']==values and after['history'][:-1]==doc['history'];return doc
    fields=['review_reply_min_text_bytes','review_reply_max_text_bytes','review_quote_min_normalized_bytes']
    doc=ok('reply-settings-original',['engine-settings','show']);maximum=max(max(len(q['text'].encode()),len(go_json(q['text'])[1:-1].encode())) for q in catalog['quotes'])
    saved('raise-fixture-quote-minimum',dict(review_reply_min_text_bytes=str(maximum+1),review_reply_max_text_bytes=None))
    try:
        changed=inputs('no-admissible-citations');assert changed['managed_protocol']['catalog_sha256']==protocol['catalog_sha256']
        # No pass branch can satisfy schema; source catalogue remains complete.
        changed_validator=jsonschema.Draft202012Validator(changed['managed_protocol']['reply_schema'])
        assert not list(changed_validator.iter_errors(reply))
        for ref in quotes:
            bad=copy.deepcopy(reply);bad['tasks'][0]['criteria'][0]=dict(index=1,verdict='pass',evidence_ref=ref)
            assert list(changed_validator.iter_errors(bad))
    finally:saved('restore-fixture-reply-policy',{k:doc['values'][k] for k in fields})
    restored=inputs('reply-policy-restored');assert restored['prompt_sha256']==original['prompt_sha256']
    checks.append('saved-admin-quote-policy-changes-schema-and-restores-without-source-loss')
    result=dict(status='PASS',checks=checks,root=str(root),work=work,ordinary_inspection_assertions=12,accepted=False,reviewer_started=False,global_watchdog=None)
    fixture.publish(out/'result.json',result);print('MANAGED_REVIEW_PROTOCOL_PASS '+str(len(checks))+' assertions')


def main():
    """Retain exact binary/profile/helper inputs and original failure diagnostics."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--reuse-from',type=Path);a=parser.parse_args();out=a.output.resolve()
    fixture=inspection.native.fixture
    inputs=dict(binary=str(a.binary.resolve()),binary_sha256=fixture.digest(a.binary.read_bytes()),profile_sha256=fixture.digest(a.profile.read_bytes()),helpers=inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
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
