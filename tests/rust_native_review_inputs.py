#!/usr/bin/env python3
"""Replay native complete review inputs with owned public worker/control fixtures.

No paid reviewer, private SQL writes, service restart, retry or global watchdog.
Every fault starts from a successful witness; originals and channels are retained.
"""
import argparse,copy,json,subprocess,traceback
from pathlib import Path
import rust_native_task_validation as native
import rust_native_review_preview as existing
import rust_agent_model_route as fixture


def observed(binary,root,out,label,work,request,expected=True):
    """Assert a read-only complete observation or refusal without Work mutation."""
    before=existing.shown(binary,root,out,label+'-before',work)
    code,value=native.invoke(binary,root,out,label,['task','review-inputs',work],dict(request,expected_revision=before['revision']))
    assert (code==0)==expected,value
    assert existing.shown(binary,root,out,label+'-after',work)==before
    if expected:
        assert value['preview']['invocation_context_materialized'] is True
        assert not any(value['preview'][key] for key in ['invocation_authorized','provider_started','task_accepted'])
        assert value['prompt_sha256']==fixture.digest(value['prompt'].encode())
        assert value['source_bytes']==sum(len(x['content'].encode()) for x in value['materials'])
        assert all(x['sha256']==fixture.digest(x['content'].encode()) for x in value['materials'])
        assert all(x['quote_kind']=='source' for x in value['materials'] if x['kind'] in ['report','document'])
        body=json.loads(value['prompt'].split('do not use tools.\n',1)[1])
        task=next(t for t in before['tasks'] if t['id']==request['task_id'])
        assert body['criteria']==task['criteria'] and body['title']==task['title']
        assert body['reply_contract']['criterion_count']==len(task['criteria'])
        assert 'ROOT-PRIVATE-TITLE' not in value['prompt']
        assert 'reply_contract' in value['prompt']
    return value


def source(binary,out,**options):
    """Create public worker/source then execute its actual authorized control once."""
    out.mkdir();setup=native.public.flow(binary,out,control_script=options.pop("control_script",native.CONTROL),**options);root=Path(setup['root']);work=setup['work_id']
    if options.get('deliverable')=='docs/delivery.md':
        (root/'docs').mkdir(exist_ok=True)
        fixture.publish(root/'docs/delivery.md',b'Complete declared original. [Unfollowed](../private.md)\n')
        fixture.publish(root/'private.md',b'UNDECLARED-PRIVATE-ORIGINAL')
    request=dict(schema_version=1,event_id='review-inputs',expected_revision=setup['revision'],task_id='task')
    code,done=native.invoke(binary,root,out,'controls',['task','validate',work],request);assert code==0 and done['controls_passed'],done
    request['expected_revision']=done['work']['revision']
    return root,work,request


def quotas(binary,root,out,label,work,**patch):
    """Change only public saved quotas, preserving consumption and audit history."""
    code,current=native.invoke(binary,root,out,label+'-show',['quotas','show',work]);assert code==0,current
    limits=copy.deepcopy(current['limits']);limits.update(patch)
    code,changed=native.invoke(binary,root,out,label,['quotas','apply',work],dict(schema_version=1,event_id=label,expected_revision=current['revision'],limits=limits,reason='Verify saved reviewer context boundaries'));assert code==0,changed
    return current['limits']


def main():
    """Verify full originals, policy refusals/recovery, privacy and output failure.

    Freeze the supplied binary; retain real Root and command channels on failure.
    Separate no-output-sharing and truncated-output fixtures prevent a common
    rejection from masking the privacy contract. No reviewer process is launched.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    raw=a.binary.resolve().read_bytes();binary=out/'binary';fixture.publish(binary,raw);binary.chmod(0o700)
    fixture.publish(out/'inputs.json',dict(binary_sha256=fixture.digest(raw),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_native_task_validation.py'),Path('tests/rust_native_review_preview.py'),Path('tests/rust_planning_worker_public_flow.py')]}))
    root=None
    try:
        root,work,request=source(binary,out/'setup',deliverable='docs/delivery.md')
        witness=(root/'validation-executed.json').read_bytes()
        inputs=observed(binary,root,out,'inputs',work,request)
        report=(root/'handoff.md').read_text();materials=inputs['materials']
        assert next(x for x in materials if x['kind']=='report')['content']==report
        document=next(x for x in materials if x['kind']=='document');assert document['content']==(root/'docs/delivery.md').read_text()
        assert 'UNDECLARED-PRIVATE-ORIGINAL' not in inputs['prompt']
        assert any(x['kind']=='engine_controls' for x in materials)
        assert any(x['kind']=='control_output' and x['content']=='CONTROL-PROBE-SUCCESS\n' for x in materials)
        # Independent healthy witness then each saved cap refusal and restoration.
        for index,(field,value) in enumerate([('review_context_files',1),('review_source_bytes',1),('review_context_bytes',1)]):
            observed(binary,root,out,f'cap-{index}-witness',work,request)
            old=quotas(binary,root,out,f'cap-{index}-save',work,**{field:value})
            refused=observed(binary,root,out,f'cap-{index}-refused',work,request,False)
            expected=['review-context-files-exceeded','project-context-invalid','review-context-bytes-exceeded'][index]
            assert refused['failure']['code']==expected,refused
            if index==1:assert 'source-exceeds-configured-cap' in refused['failure']['message']
            quotas(binary,root,out,f'cap-{index}-restore',work,**{field:old[field]})
            observed(binary,root,out,f'cap-{index}-recovered',work,request)
        # Inclusive exact bounds and explicit zero semantics use the same full prompt.
        old=quotas(binary,root,out,'exact-caps',work,review_context_files=len(inputs['materials']),review_source_bytes=inputs['source_bytes'],review_context_bytes=len(inputs['prompt'].encode()))
        observed(binary,root,out,'exact-caps-inputs',work,request)
        quotas(binary,root,out,'zero-caps',work,review_context_files=0,review_source_bytes=0,review_context_bytes=0)
        observed(binary,root,out,'zero-caps-inputs',work,request)
        quotas(binary,root,out,'original-caps',work,**{key:old[key] for key in ['review_context_files','review_source_bytes','review_context_bytes']})
        # Global read and reply policies also reach this actual public consumer.
        old_settings,_=existing.saved_setting(binary,root,out,'delivery-cap-one','review_delivery_max_bytes','1')
        refused=observed(binary,root,out,'delivery-cap-refused',work,request,False)
        assert refused['failure']['code']=='task-validation-report',refused
        assert 'project-context-invalid' in refused['failure']['causes']
        assert 'source-exceeds-configured-cap' in refused['failure']['causes']
        existing.saved_setting(binary,root,out,'delivery-cap-restored','review_delivery_max_bytes',old_settings['values']['review_delivery_max_bytes'])
        observed(binary,root,out,'delivery-cap-recovered',work,request)
        old_settings,_=existing.saved_setting(binary,root,out,'reply-min-nine','review_reply_min_text_bytes','9')
        changed=observed(binary,root,out,'reply-min-consumer',work,request)
        body=json.loads(changed['prompt'].split('do not use tools.\n',1)[1]);assert body['reply_contract']['text_min_trimmed_utf8_bytes']==9
        existing.saved_setting(binary,root,out,'reply-min-restored','review_reply_min_text_bytes',old_settings['values']['review_reply_min_text_bytes'])
        observed(binary,root,out,'reply-min-recovered',work,request)
        original=(root/'docs/delivery.md').read_bytes();(root/'docs/delivery.md').unlink()
        try:observed(binary,root,out,'missing-declared',work,request,False)
        finally:fixture.publish(root/'docs/delivery.md',original)
        observed(binary,root,out,'document-restored',work,request)
        original_report=(root/'handoff.md').read_bytes();fixture.publish(root/'handoff.md',original_report+b'changed')
        try:observed(binary,root,out,'changed-report',work,request,False)
        finally:fixture.publish(root/'handoff.md',original_report)
        observed(binary,root,out,'report-restored',work,request)
        observed(binary,root,out,'forbidden-actor',work,dict(request,actor='engine'),False)
        before=existing.shown(binary,root,out,'full-before',work);input_path=out/'full.input.json';fixture.publish(input_path,dict(request,expected_revision=before['revision']))
        with open('/dev/full','wb',buffering=0) as full:
            result=subprocess.run([str(binary),'task','review-inputs',work,'--root',str(root),'--input',str(input_path),'--json'],stdin=subprocess.DEVNULL,stdout=full,stderr=subprocess.PIPE)
        fixture.publish(out/'full.stderr',result.stderr);assert result.returncode and b'preserved' not in result.stderr
        assert existing.shown(binary,root,out,'full-after',work)==before
        assert (root/'validation-executed.json').read_bytes()==witness
        # Unshared actual output is absent, even though the authorized command prints.
        private_script=native.CONTROL.replace("print('CONTROL-PROBE-SUCCESS',flush=True)","secret=os.urandom(32).hex();Path('unshared-secret.txt').write_text(secret);print(secret,flush=True)")
        private_root,private_work,private_request=source(binary,out/'unshared',share_control_output=False,control_script=private_script)
        private=observed(binary,private_root,out,'unshared-inputs',private_work,private_request)
        assert not any(x['kind']=='control_output' for x in private['materials'])
        assert (private_root/'unshared-secret.txt').read_text() not in private['prompt']
        # A separate shared fixture checks truncation, without changing authorization.
        trunc_out=out/'truncated';trunc_out.mkdir();setup=native.public.flow(binary,trunc_out,control_script=native.CONTROL);trunc_root=Path(setup['root']);trunc_work=setup['work_id']
        existing.saved_setting(binary,trunc_root,out,'shared-capture-one','validation_max_output_bytes','1')
        trunc_request=dict(schema_version=1,event_id='truncated-controls',expected_revision=setup['revision'],task_id='task')
        code,done=native.invoke(binary,trunc_root,out,'truncated-controls',['task','validate',trunc_work],trunc_request);assert code==0 and done['controls_passed'],done
        trunc_request['expected_revision']=done['work']['revision'];truncated=observed(binary,trunc_root,out,'truncated-inputs',trunc_work,trunc_request)
        assert not any(x['kind']=='control_output' for x in truncated['materials'])
        receipt=json.loads(next(x for x in truncated['materials'] if x['kind']=='engine_controls')['content'])
        assert any(c['review_output_truncated'] for c in receipt['controls'])
        # An actual hashed PNG control input is required; current image support must refuse.
        import base64
        image=base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl2nWQAAAAASUVORK5CYII=')
        image_root,image_work,image_request=source(binary,out/'image',control_inputs=['handoff.md','docs/screenshots/witness.png'],fixture_files=[('docs/screenshots/witness.png',image)])
        existing.preview(binary,image_root,out,'image-provenance-witness',image_work,image_request)
        refused=observed(binary,image_root,out,'required-image-unported',image_work,image_request,False)
        assert refused['failure']['code']=='review-image-input-unported',refused
        fixture.publish(out/'result.json',dict(status='PASS',full_original_inputs=True,read_only=True,saved_caps_and_recovery=True,missing_declared_refused=True,undeclared_links_ignored=True,unshared_output_absent=True,truncated_output_not_quoted=True,reviewer_started=False,paid_provider=False,binary_sha256=fixture.digest(raw),global_watchdog=None))
        print('PASS native reviewer original inputs without invocation',flush=True)
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
