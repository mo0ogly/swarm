#!/usr/bin/env python3
"""Replay native quota CLI and protected HTTP on an owned inert Root.

No private SQL mutation, provider, service restart, global watchdog or automatic
retry. Preserve original requests, exact frozen binary and failure evidence.
"""
import argparse,copy,json,traceback
from pathlib import Path
import rust_agent_model_route as f
from rust_planning_activation import configured,apply_policy
from rust_native_http_cli import Server


def change(owned,event,**patch):
    """Prepare one explicit current proposal; optional fields preserve prior data."""
    current=owned.ok(['quotas','show',owned.work]);limits=copy.deepcopy(current['limits']);limits.update(patch)
    return dict(schema_version=1,event_id=event,expected_revision=current['revision'],limits=limits,reason='Explicit owned quota configuration')


def main():
    """Require pure preview, durable changes/replay, policy consumption and recovery."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    binary=a.binary.resolve();f.publish(out/'binary',binary.read_bytes());binary=out/'binary';binary.chmod(0o700)
    f.publish(out/'inputs.json',dict(binary_sha256=f.digest(binary.read_bytes()),sources={str(x):f.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_agent_model_route.py'),Path('tests/rust_planning_activation.py'),Path('tests/rust_native_http_cli.py')]}))
    owned=None;server=None
    try:
        owned=f.Fixture(binary,out,'quotas145');configured(owned,'quota-inert-plan')
        value=change(owned,'quota-cli',planning_activations=8,review_context_files=5,review_context_bytes=25000)
        before=owned.ok(['work','show',owned.work])['work'];snapshot=owned.snapshot();preview=owned.ok(['quotas','preview',owned.work],value)
        assert preview['before']['limits']['planning_activations']==3 and preview['after']['limits']['planning_activations']==8
        assert owned.ok(['work','show',owned.work])['work']==before and owned.snapshot()==snapshot
        applied=owned.ok(['quotas','apply',owned.work],value);assert applied['revision']==before['revision']+1
        assert applied['planning']['reviewer']['max_context_files']==5 and applied['planning']['reviewer']['max_context_bytes']==25000
        assert owned.ok(['quotas','apply',owned.work],value)==applied
        observed=owned.ok(['quotas','show',owned.work]);assert observed['limits']['review_context_files']==5 and observed['consumed']['review_calls']==0
        assert observed['last_authorization']['reason']==value['reason'] and observed['last_authorization']['actor']
        snapshot=owned.snapshot();code,refused=owned.call(['quotas','apply',owned.work],dict(value,event_id='quota-stale'))
        assert code and refused['failure']['code']=='revision_conflict' and owned.snapshot()==snapshot
        # Actual saved reason policy changes, via preview/apply/reopen, reach the quota consumer.
        short=change(owned,'quota-short');short['reason']='abc';code,refused=owned.call(['quotas','preview',owned.work],short)
        assert code and refused['failure']['code']=='quota-reason-outside-policy'
        apply_policy(owned,'quota-reason-three',dict(planning_quota_reason_min_bytes='3',planning_quota_reason_max_bytes='3'))
        accepted=owned.ok(['quotas','apply',owned.work],short);assert accepted['planning']['quota_authorization']['reason']=='abc'
        apply_policy(owned,'quota-reason-restored',dict(planning_quota_reason_min_bytes='8',planning_quota_reason_max_bytes='2000'))
        # Actual TCP endpoint, protected request and new Work event.
        server=Server(binary,owned.root,out,'quotas-http');session=server.request('/api/v1/native-session');csrf=session['csrf_token'];path='?work='+owned.work
        view=server.request('/api/v1/quotas'+path);assert view==owned.ok(['quotas','show',owned.work])
        value=change(owned,'quota-http',planning_activations=9,review_source_bytes=42000);before=owned.ok(['work','show',owned.work])['work'];snapshot=owned.snapshot()
        preview=server.request('/api/v1/quotas/preview'+path,value,csrf);assert preview['after']['limits']['planning_activations']==9 and owned.snapshot()==snapshot
        applied=server.request('/api/v1/quotas/apply'+path,value,csrf);assert applied['revision']==before['revision']+1 and applied['planning']['reviewer']['max_source_bytes']==42000
        assert server.request('/api/v1/quotas/apply'+path,value,csrf)==applied
        assert server.request('/api/v1/quotas'+path)['limits']['planning_activations']==9
        # Valid protected witness above precedes independent access/query/authority faults.
        snapshot=owned.snapshot();current=owned.ok(['work','show',owned.work])['work'];next_value=change(owned,'quota-denied')
        server.request('/api/v1/quotas'+path,authorized=False,status=403,observe=False)
        server.request('/api/v1/quotas/apply'+path,next_value,status=403,observe=False)
        for suffix in ['',path+'&work=other',path+'&actor=engine','?work=..%2FPRIVATE']:
            reply=server.request('/api/v1/quotas'+suffix,status=400);assert reply['failure']['code']=='quota-query-invalid'
        reply=server.request('/api/v1/quotas/preview'+path,dict(next_value,actor='engine'),csrf,status=400);assert reply['failure']['code']=='quota-request-invalid'
        assert owned.snapshot()==snapshot and owned.ok(['work','show',owned.work])['work']==current
        # Full Root is read-only for preview, including failed output writes.
        import subprocess,os
        request_path=out/'full-preview.json';f.publish(request_path,next_value)
        with open(os.devnull,'rb') as stdin,open('/dev/full','wb',buffering=0) as stdout:
            result=subprocess.run([str(binary),'quotas','preview',owned.work,'--root',str(owned.root),'--input',str(request_path),'--json','--lang','en'],stdin=stdin,stdout=stdout,stderr=subprocess.PIPE)
        f.publish(out/'full-preview.stderr',result.stderr);assert result.returncode!=0 and owned.snapshot()==snapshot
        assert not (owned.root/'model-received.json').exists()
        server.cleanup();assert server.process.returncode==0;server=None
        f.publish(out/'result.json',dict(status='PASS',cli_preview_apply_reopen_replay=True,http_protected_preview_apply_replay=True,saved_reason_policy_effective=True,refusals_without_mutation=True,output_failure_observed=True,provider_started=False,installed=False,global_watchdog=None))
        print('PASS native mission quota CLI HTTP saved policy and refusal recovery',flush=True)
    except BaseException as primary:
        f.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()))
        if server:
            try:server.cleanup()
            except BaseException as cleanup:raise BaseExceptionGroup('Quota failure and owned server cleanup',[primary,cleanup])
        raise


if __name__=='__main__':main()
