#!/usr/bin/env python3
"""Replay explicit legacy adoption and reviewer policy consumption via native CLI.

Only owned Roots/inert worker/control fixtures; no reviewer, paid provider,
private state edit, service restart, retry or global watchdog. Keep all channels.
"""
import argparse,copy,json,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_native_task_validation as native
import rust_native_review_preview as preview
import rust_native_review_inputs as inputs


def call(binary,root,out,name,args,value=None):
    """Use bounded native public requests; preserve refusal instead of empty data."""
    return native.invoke(binary,root,out,name,args,value)


def change(binary,root,out,name,values,schema):
    """Preview/CAS/save/reopen and replay one explicit settings change.

    Preview leaves bytes intact; replays preserve exact saved history/revision.
    Return original and saved documents, not just the proposed values.
    """
    code,old=call(binary,root,out,name+'-show',['engine-settings','show']);assert code==0,old
    request=dict(schema_version=schema,event_id=name,expected_revision=old['revision'],values=values,reason='Explicit reviewer response policy adoption',preview_token='')
    before=(root/'.swarm/engine-settings.json').read_bytes()
    code,proposed=call(binary,root,out,name+'-preview',['engine-settings','preview'],request);assert code==0,proposed
    assert (root/'.swarm/engine-settings.json').read_bytes()==before
    request['preview_token']=proposed['preview_token']
    code,saved=call(binary,root,out,name+'-apply',['engine-settings','apply'],request);assert code==0,saved
    code,reopened=call(binary,root,out,name+'-reopen',['engine-settings','show']);assert code==0,reopened
    assert reopened['values']==values and reopened['history'][:len(old['history'])]==old['history']
    saved_bytes=(root/'.swarm/engine-settings.json').read_bytes()
    code,replayed=call(binary,root,out,name+'-replay',['engine-settings','apply'],request);assert code==0,replayed
    assert replayed['replayed'] is True and saved['replayed'] is False
    assert replayed['document']==saved['document']==reopened
    assert (root/'.swarm/engine-settings.json').read_bytes()==saved_bytes
    return old,reopened


def main():
    """Observe v28 bytes intact before explicit v29 adoption, then actual Root use."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--legacy-binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    binaries=[]
    for name,path in [('binary',a.binary),('legacy-binary',a.legacy_binary)]:
        raw=path.resolve().read_bytes();target=out/name;f.publish(target,raw);target.chmod(0o700);binaries.append(target)
    binary,legacy=binaries;root=out/'legacy-root';root.mkdir(mode=0o700)
    try:
        code,_=call(legacy,root,out,'legacy-init',['init']);assert code==0
        code,old=call(legacy,root,out,'legacy-show',['engine-settings','show']);assert code==0 and old['schema_version']==28 and 'review_response_max_bytes' not in old['values'],old
        original=(root/'.swarm/engine-settings.json').read_bytes()
        code,read=call(binary,root,out,'current-reads-legacy',['engine-settings','show']);assert code==0 and read==old,read
        assert (root/'.swarm/engine-settings.json').read_bytes()==original
        code,catalog=call(binary,root,out,'catalog',['engine-settings','describe']);assert code==0 and catalog['document_schema_version']==31,catalog  # v30 telemetry, v31 cache health; v30 telemetry providers (LIVE-UX-03) follows v29
        field=next(x for x in catalog['fields'] if x['name']=='review_response_max_bytes');assert field['nullable'] and field['introduced_schema_version']==29 and catalog['defaults']['review_response_max_bytes'] is None
        values=copy.deepcopy(old['values']);values['review_response_max_bytes']='3';_,adopted=change(binary,root,out,'adopt29',values,29);assert adopted['schema_version']==29
        assert len(adopted['history'])==len(old['history'])+1
        # Actual materialization consumes the saved value, independent of planning cap.
        context_root,work,request=inputs.source(binary,out/'context')
        initial=inputs.observed(binary,context_root,out,'initial-inputs',work,request);assert initial['response_limit_bytes'] is None
        before=preview.shown(binary,context_root,out,'work-before',work);witness=(context_root/'validation-executed.json').read_bytes()
        for index,value in enumerate(['3','0',None]):
            preview.saved_setting(binary,context_root,out,'review-cap-'+str(index),'review_response_max_bytes',value)
            result=inputs.observed(binary,context_root,out,'review-consumer-'+str(index),work,request)
            expected=int(value) if value is not None else None;assert result['response_limit_bytes']==expected
            body=json.loads(result['prompt'].split('do not use tools.\n',1)[1]);assert body['reply_contract']['response_max_bytes']==expected
        preview.saved_setting(binary,context_root,out,'planner-cap-zero','planning_response_max_bytes','0')
        result=inputs.observed(binary,context_root,out,'independent-inputs',work,request);assert result['response_limit_bytes'] is None
        code,current=call(binary,context_root,out,'invalid-show',['engine-settings','show']);assert code==0,current
        values=copy.deepcopy(current['values']);values['review_response_max_bytes']='-1'
        before_settings=(context_root/'.swarm/engine-settings.json').read_bytes()
        code,denied=call(binary,context_root,out,'negative-refused',['engine-settings','preview'],dict(schema_version=current['schema_version'],event_id='negative-cap',expected_revision=current['revision'],values=values,reason='Refuse negative reviewer response cap',preview_token=''));assert code,denied
        assert (context_root/'.swarm/engine-settings.json').read_bytes()==before_settings
        assert preview.shown(binary,context_root,out,'work-after',work)==before and (context_root/'validation-executed.json').read_bytes()==witness
        f.publish(out/'result.json',dict(status='PASS',legacy_read_exact=True,explicit_adoption_history_preserved=True,saved_caps_consumed=True,planner_cap_independent=True,negative_refused_without_mutation=True,reviewer_started=False,paid_provider=False,global_watchdog=None,binary_sha256=f.digest(binary.read_bytes()),legacy_sha256=f.digest(legacy.read_bytes())))
        print('PASS native saved reviewer response policy and explicit legacy adoption',flush=True)
    except BaseException:
        f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc(),root=str(root)));raise


if __name__=='__main__':main()
