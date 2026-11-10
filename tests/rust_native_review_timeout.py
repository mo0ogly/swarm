#!/usr/bin/env python3
"""Verify native mission reviewer duration via CLI and protected TCP quotas API.

Owned inert worker/control fixture only; no reviewer, paid provider, private row
writes, service restart or global watchdog. Retain original request channels.
"""
import argparse,copy,json,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_native_task_validation as native
import rust_native_task_validation_failures as guards
import rust_native_review_preview as preview
import rust_native_review_inputs as inputs
from rust_native_http_cli import Server,finish


def proposal(binary,root,out,label,work,seconds):
    """Read authoritative current limits and request only an explicit duration."""
    code,current=native.invoke(binary,root,out,label+'-view',['quotas','show',work]);assert code==0,current
    return dict(schema_version=1,event_id=label,expected_revision=current['revision'],limits=copy.deepcopy(current['limits']),review_timeout_seconds=seconds,reason='Explicit mission reviewer duration'),current


def main():
    """Require pure preview, durable replay, native field placement and no use reset.

    Positive and zero witnesses precede negative/clock-representation faults.
    Read-only SQL checks original canonical event bytes only, never seeding data.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);binary=out/'binary';f.publish(binary,a.binary.resolve().read_bytes());binary.chmod(0o700)
    server=None;primary=None
    try:
        root,work,request=inputs.source(binary,out/'source');before=preview.shown(binary,root,out,'before',work);witness=(root/'validation-executed.json').read_bytes()
        value,limits=proposal(binary,root,out,'timeout-cli',work,7);assert limits['review_timeout_seconds']==0
        code,proposed=native.invoke(binary,root,out,'timeout-preview',['quotas','preview',work],value);assert code==0 and proposed['before']['review_timeout_seconds']==0 and proposed['after']['review_timeout_seconds']==7,proposed
        assert preview.shown(binary,root,out,'preview-after',work)==before
        code,applied=native.invoke(binary,root,out,'timeout-apply',['quotas','apply',work],value);assert code==0 and applied['planning']['reviewer']['timeout_seconds']==7,applied
        code,replayed=native.invoke(binary,root,out,'timeout-replay',['quotas','apply',work],value);assert code==0 and replayed==applied,replayed
        code,shown=native.invoke(binary,root,out,'timeout-reopen',['quotas','show',work]);assert code==0 and shown['review_timeout_seconds']==7 and shown['consumed']==limits['consumed'],shown
        null_value,_=proposal(binary,root,out,'timeout-null',work,None)
        code,proposed=native.invoke(binary,root,out,'null-preserves',['quotas','preview',work],null_value);assert code==0 and proposed['after']['review_timeout_seconds']==7,proposed
        server=Server(binary,root,out,'timeout-http');session=server.request('/api/v1/native-session');csrf=session['csrf_token'];suffix='?work='+work
        value,_=proposal(binary,root,out,'timeout-http-zero',work,0);before_http=preview.shown(binary,root,out,'http-before',work)
        proposed=server.request('/api/v1/quotas/preview'+suffix,value,csrf);assert proposed['after']['review_timeout_seconds']==0 and preview.shown(binary,root,out,'http-preview-after',work)==before_http
        applied=server.request('/api/v1/quotas/apply'+suffix,value,csrf);assert applied['planning']['reviewer'].get('timeout_seconds',0)==0
        shown=server.request('/api/v1/quotas'+suffix);assert shown['review_timeout_seconds']==0 and shown['consumed']==limits['consumed']
        with guards.database(root) as db:
            row=db.execute("SELECT request FROM events WHERE work_id=? AND id=? AND kind='quotas.configure'",[work,value['event_id']]).fetchone();assert row and json.loads(row[0])['review_timeout_seconds']==0
        for label,seconds,code in [('negative',-1,'quota-review-timeout-invalid'),('unrepresentable',2**63-1,'quota-review-timeout-unrepresentable')]:
            value,_=proposal(binary,root,out,'timeout-'+label,work,seconds);before=preview.shown(binary,root,out,label+'-before',work)
            error=server.request('/api/v1/quotas/preview'+suffix,value,csrf,status=400);assert error['failure']['code']==code,error
            assert preview.shown(binary,root,out,label+'-after',work)==before
        assert (root/'validation-executed.json').read_bytes()==witness
        summary=server.stop()
        proof=dict(status='PASS',cli_http_persist_replay=True,zero_removes_deadline=True,null_preserves=True,canonical_event_contains_native_field=True,invalid_refused_without_mutation=True,consumption_preserved=True,reviewer_started=False,runtime_deadline_exercised=False,paid_provider=False,owner_summary=summary,binary_sha256=f.digest(binary.read_bytes()),global_watchdog=None)
    except BaseException as error:
        primary=error
        try:f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        except BaseException as secondary:primary=BaseExceptionGroup('Scenario and evidence publication failed',[error,secondary])
    if server is not None:finish(server,primary)
    elif primary is not None:raise primary
    f.publish(out/'result.json',proof)
    print('PASS native reviewer duration configuration without invocation',flush=True)


if __name__=='__main__':main()
