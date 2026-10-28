#!/usr/bin/env python3
"""G9 public CLI/HTTP on an owned inactive Root, without account or model calls.

No global watchdog, mission completion or live database mutation. Saved policy
is changed only through settings preview/apply. Fault injection targets solely
this test's private database after a valid control, with original bytes retained.
"""
from pathlib import Path
import argparse,json,sqlite3,traceback
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server

def run(binary,output):
    """Require meaningful reads, preview/CAS, replay, rollback and recovery.

    Original commands/replies and cleanup failures remain retained. A positive
    API receipt is not global migration acceptance or provider admission proof.
    """
    output.mkdir(parents=True,exist_ok=False)
    publish(output/'harness.py',Path(__file__).read_bytes())
    f=Fixture(binary,output,'limits');before=f.snapshot();checks=[];s=None;primary=None
    def change(event,revision,**extra):
        return dict(schema_version=1,event_id=event,scope='project',expected_revision=revision,values=dict(silence_seconds=2,tool_seconds=3,max_tool_calls=4,max_repeated_calls=2,max_consecutive_errors=2),reason='Owned G9 fixture',**extra)
    def request(path,body=None,status=200):
        return s.request(path,body,csrf=csrf,status=status,observe=False)
    def rows():
        db=sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True)
        try:return [db.execute('SELECT * FROM '+table+' ORDER BY rowid').fetchall() for table in ['run_limits_config','run_limits_history']]
        finally:db.close()
    try:
        assert f.ok(['run-limits','show','project','-','-'])['revision']==0
        assert f.ok(['run-limits','history','project','-','-']) is None
        s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');csrf=s.request('/api/v1/native-session',observe=False)['csrf_token']
        url='/api/v1/run-limits?scope=project';view=request(url)
        assert view['entry']['revision']==0 and view['history'] is None and view['exact']['entry']['values']['observation_mode']=='0' and view['bounds']
        checks.append('empty-cli-http-null-history-exact-zero-projection')
        r=change('first',0);original=rows();pv=request('/api/v1/run-limits/preview',r)
        assert pv['preview'] and not pv['applied'] and pv['revision_ok'] and pv['preview_token'] and rows()==original
        saved=request('/api/v1/run-limits/apply',{**r,'preview_token':pv['preview_token']})
        assert saved['applied'] and saved['revision']==1 and saved['values']['silence_seconds']==2
        assert f.ok(['run-limits','show','project','-','-'])['revision']==1
        checks.append('preview-without-mutation-token-apply-cli-readback')
        original=rows();request('/api/v1/run-limits/apply',change('stale',0),409);assert rows()==original
        bad=change('negative',1);bad['values']['max_tool_calls']=-1;request('/api/v1/run-limits/apply',bad,400);assert rows()==original
        request('/api/v1/run-limits?scope=project&scope=project',status=400);assert rows()==original
        assert request(url)['entry']['revision']==1
        checks.append('stale-negative-duplicate-query-no-mutation-recovery')
        second=change('second',1);second['values']['silence_seconds']=5
        old_preview=request('/api/v1/run-limits/preview',second)
        assert f.ok(['run-limits','apply','project','-','-'],second)['revision']==2
        original=rows();old_preview_request={**second,'event_id':'different','expected_revision':2,'preview_token':old_preview['preview_token']}
        request('/api/v1/run-limits/apply',old_preview_request,409);assert rows()==original
        # Original first event replays to CURRENT entry and adds no history.
        replay=request('/api/v1/run-limits/apply',r);assert replay['replayed'] and replay['revision']==2 and rows()==original
        checks.append('legacy-cli-cas-token-stale-exact-replay-current-entry')
        rollback=dict(scope='project',event_id='rollback',to_revision=1,expected_revision=2,reason='')
        p=request('/api/v1/run-limits/rollback/preview',rollback);assert p['proposed']['silence_seconds']==2
        result=request('/api/v1/run-limits/rollback',{**rollback,'preview_token':p['preview_token']});assert result['revision']==3 and result['reason']=='retour à la révision 1'
        hist=f.ok(['run-limits','history','project','-','-']);assert [v['revision'] for v in hist]==[3,2,1] and hist[0]['rollback_of']==1
        original=rows();request('/api/v1/run-limits/rollback',{**rollback,'event_id':'absent-target','to_revision':900,'expected_revision':3},404);assert rows()==original
        checks.append('rollback-preview-new-revision-complete-history-missing-target-refused')
        # Explicitly outside old Go reason/count caps, inside current saved policy.
        wide=change('wide-reason',3);wide['reason']='reason '*500
        result=request('/api/v1/run-limits/apply',wide);assert result['revision']==4 and result['reason']==wide['reason']
        # Same token must not survive an actual persisted settings revision change.
        r5=change('settings-stale',4);p=request('/api/v1/run-limits/preview',r5)
        settings=f.ok(['engine-settings','show'])['values'];key='runtime_limit_max_tool_calls_ceiling'
        candidates=[k for k in settings if 'runtime' in k and 'tool' in k]
        publish(output/'runtime-setting-keys.json',candidates)
        # Actual unrelated existing saved setting also invalidates inspected policy snapshot.
        cap=settings['preparation_call_response_max_bytes'];f.settings_change(preparation_call_response_max_bytes=str(int(cap)+1))
        original=rows();request('/api/v1/run-limits/apply',{**r5,'preview_token':p['preview_token']},409);assert rows()==original
        p=request('/api/v1/run-limits/preview',r5);assert request('/api/v1/run-limits/apply',{**r5,'preview_token':p['preview_token']})['revision']==5
        checks.append('no-inherited-reason-cap-saved-settings-token-stale-recovery')
        effective=request('/api/v1/run-limits/effective');assert effective['scope']=='administrative-current-policy' and effective['effective']['silence_seconds']==2
        for scope,mission,key in [('mission','m',''),('role','m','worker'),('task','m','task')]:
            rr=change('scope-'+scope,0);rr.update(scope=scope,mission_id=mission,scope_key=key)
            assert request('/api/v1/run-limits/apply',rr)['revision']==1
            assert f.ok(['run-limits','show',scope,mission,key or '-'])['revision']==1
        assert f.snapshot()==before and not list(f.root.rglob('model-received.json'))
        checks.append('all-scopes-effective-read-no-work-agent-provider-mutation')
        # Private-only fault, after a valid revision5 control; retain original body.
        assert request(url)['entry']['revision']==5
        db=sqlite3.connect(f.root/'.swarm/state.db')
        try:
            raw=db.execute("SELECT body FROM run_limits_config WHERE scope='project'").fetchone()[0]
            publish(output/'valid-current-before-fault.bin',raw.encode() if isinstance(raw,str) else raw)
            db.execute("UPDATE run_limits_config SET body=? WHERE scope='project'",(b'not-json',));db.commit()
        finally:db.close()
        request(url,status=500)
        repaired=request('/api/v1/run-limits/apply',change('repair',5));assert repaired['revision']==6 and repaired['current'] is None
        assert request(url)['entry']['revision']==6 and len(request(url)['history'])==6
        assert f.snapshot()==before
        checks.append('valid-control-malformed-private-current-visible-error-cas-repair-history-preserved')
        # Persisted definition ceiling is editable, enforced and removable; no
        # second historical Go10000 cutoff. Existing replay is not rejudged.
        f.settings_change(runtime_max_tool_calls_ceiling='100')
        too_wide=change('saved-ceiling',6);too_wide['values']['max_tool_calls']=101
        original=rows();request('/api/v1/run-limits/apply',too_wide,400);assert rows()==original
        f.settings_change(runtime_max_tool_calls_ceiling=None)
        too_wide['values']['max_tool_calls']=10001
        assert request('/api/v1/run-limits/apply',too_wide)['revision']==7
        assert request('/api/v1/run-limits/effective')['effective']['max_tool_calls']==10001
        f.settings_change(runtime_max_tool_calls_ceiling='10000')
        view=request(url);assert view['issues'] and view['entry']['values']['max_tool_calls']==10001
        original=rows();replay=request('/api/v1/run-limits/apply',too_wide);assert replay['replayed'] and replay['revision']==7 and rows()==original
        restored=request('/api/v1/run-limits/rollback',dict(scope='project',event_id='restore-valid-policy',to_revision=1,expected_revision=7,reason='Explicit recovery'))
        assert restored['revision']==8 and not request(url)['issues']
        assert request('/api/v1/run-limits/effective')['effective']['max_tool_calls']==4
        checks.append('persisted-ceiling-refusal-unlimited-consumer-historical-replay-policy-recovery')
    except BaseException as error:primary=error
    cleanup=None
    if s:
        try:s.cleanup()
        except BaseException as error:cleanup=error
    if primary or cleanup:raise BaseExceptionGroup('Run-limits flow or owned cleanup failed',([primary] if primary else [])+([cleanup] if cleanup else []))
    assert len(checks)==9
    publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),live_database_mutated=False,provider_started=False))
    print('RUN_LIMITS_PUBLIC_VERIFIED',len(checks),flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
