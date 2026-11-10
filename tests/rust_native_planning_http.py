#!/usr/bin/env python3
"""Go-compatible planning endpoint through actual protected native HTTP/CLI.

Owned temporary Root/server only. No model process, positive SQL seed, live
mission, global watchdog or authenticated-human assertion. Preserve both primary
and cleanup failures and only stop the server created by this recipe.
"""
import argparse,json,traceback
import http.client as http_client
from pathlib import Path
import rust_agent_model_route as f
import rust_native_http_cli as http
import rust_native_review_acceptance as review
from rust_planning_activation import apply_policy,claim_request
from rust_planning_bootstrap import fresh_work,request
from rust_planning_decision import decision
from rust_native_review_decision import rows


def main():
    """Verify read/enable, cycles, claim/lease/decide/pause/resume and replay.

    Actual session/Work/HEAD/control witnesses precede faults. Strict URL/body
    and access refusals preserve exact Work, canonical event snapshot and finance.
    Existing saved policies are edited/reopened through public CLI; old history
    and spending remain. The HTTP and CLI consumers must observe identical Work.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,required=True,type=Path)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None;server=None;primary=None
    f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and f.digest(binary.read_bytes())==profile['sha256']
        owned=f.Fixture(binary,out/'source','planning-http');owned.work=fresh_work(owned,'http-create')
        apply_policy(owned,'http-reason-high',dict(planning_decision_max_reason_bytes='6000'))
        server=http.Server(binary,owned.root,out,'owned-server',environment=owned.environment);csrf=server.request('/api/v1/native-session')['csrf_token']
        advertised=server.request('/api/v1/native-capabilities')['planning'];assert advertised==['show','enable','claim','lease-renew','decide','judge','renew','pause','resume']
        url='/api/v1/planning?work='+owned.work
        shown=server.request(url);assert shown==owned.ok(['work','show',owned.work])['work']
        connection=http_client.HTTPConnection(server.host,server.port,timeout=None)
        try:
            connection.request('HEAD',url,headers={'Cookie':server.cookie,'Origin':'http://'+server.address});reply=connection.getresponse();assert reply.status==200 and reply.read()==b''
        finally:connection.close()
        assert server.record()['outcome']['status']==200
        work=server.request(url+'&action=enable',request(event='http-enable',max_activations=3,max_decisions=3,max_judge_cycles=2),csrf)
        assert work['planning']['judge']['cycle']==1 and work['planning']['activations']==0
        money=review.public.finance(owned.root,owned.work);events=work['planning']['inbox'];long_reason='é'*2500
        scope=next(s for s in work['planning']['scopes'] if s['id']=='root')
        first=dict(schema_version=1,event_id='http-judge',expected_revision=work['revision'],scope='root',scope_revision=scope['revision'],judge_cycle=1,judge_verdict='continue',reason=long_reason)
        work=server.request(url+'&action=judge',first,csrf);assert work['planning']['judge']['reason']==long_reason
        scope=next(s for s in work['planning']['scopes'] if s['id']=='root');renew=dict(schema_version=1,event_id='http-renew',expected_revision=work['revision'],scope='root',scope_revision=scope['revision'],judge_cycle=1,reason='Open next explicit global cycle')
        work=server.request(url+'&action=renew',renew,csrf);assert work['planning']['judge']['cycle']==2 and len(work['planning']['judge']['history'])==1
        assert work['planning']['inbox']==events and work['planning']['activations']==work['planning']['decisions']==0 and review.public.finance(owned.root,owned.work)==money
        apply_policy(owned,'http-reason-low',dict(planning_decision_max_reason_bytes='200'));assert server.request(url)==work
        assert server.request(url+'&action=judge',first,csrf)==work
        refusals=[]
        def snapshot():
            """Read actual canonical events alongside Work/agent/reservation state.

            Read-only SQL strengthens no-mutation assertions; no event is seeded
            or reconstructed from the HTTP response or inherited count.
            """
            return owned.snapshot(),rows(owned.root,'SELECT * FROM events ORDER BY id',())
        def refused(label,path,body=None,code=None,status=400,**options):
            """Refuse without mutation after valid same-endpoint positive controls."""
            before=snapshot();finance=review.public.finance(owned.root,owned.work);value=server.request(path,body,csrf if body is not None else None,status=status,**options)
            if code is not None:assert value['failure']['code']==code,(label,value)
            assert snapshot()==before and review.public.finance(owned.root,owned.work)==finance
            refusals.append(label)
        scope=next(s for s in work['planning']['scopes'] if s['id']=='root');valid=dict(first,event_id='http-current',expected_revision=work['revision'],scope_revision=scope['revision'],judge_cycle=2,reason='Current valid shape')
        refused('reason-policy',url+'&action=judge',dict(valid,event_id='http-long',reason=long_reason),'planning-decision-reason-limit')
        refused('stale-revision',url+'&action=judge',dict(valid,event_id='http-stale',expected_revision=1),'revision_conflict',409)
        refused('unknown-request-field',url+'&action=judge',dict(valid,forged_permission=True),'planning-request-format')
        refused('unported-end',url+'&action=judge',dict(valid,judge_verdict='end',expected_candidate='declared-only'),'planning-judge-end-managed-evidence-unported')
        for suffix in ['&work=other','&action=judge&action=renew','&actor=engine','&action=']:
            refused('ambiguous-url:'+suffix,url+suffix,code='planning-query-invalid')
        refused('missing-work','/api/v1/planning',code='planning-query-invalid')
        refused('managed-preview',url+'&action=scope-preview',code='web-operation-unported',status=501)
        refused('get-judge-is-not-show',url+'&action=judge',code='web-operation-unported',status=501)
        refused('process-step-not-control',url+'&action=step',valid,'web-operation-unported',501)
        refused('post-needs-action',url,valid,'web-operation-unported',501)
        refused('no-cookie',url+'&action=judge',valid,status=403,authorized=False,observe=False)
        # Deliberately omit the token independently of cookie authorization.
        before=snapshot();server.request(url+'&action=judge',valid,status=403,observe=False);assert snapshot()==before;refusals.append('no-csrf')
        claimed=server.request(url+'&action=claim',claim_request(work,'http-claim',scope_revision=scope['revision']),csrf)
        scope=next(s for s in claimed['planning']['scopes'] if s['id']=='root')
        refused('logical-owner',url+'&action=judge',dict(valid,event_id='http-busy',expected_revision=claimed['revision'],scope_revision=scope['revision']),'planning-judge-owner-active')
        renewal=dict(schema_version=1,event_id='http-lease-renew',expected_revision=claimed['revision'],scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],lease_seconds=90)
        leased=server.request(url+'&action=lease-renew',renewal,csrf);assert leased['planning']['activations']==1 and leased['planning']['judge']==claimed['planning']['judge']
        decided=server.request(url+'&action=decide',decision(leased,'http-ack'),csrf);assert decided['planning']['decisions']==1 and all(e.get('decision') for e in decided['planning']['inbox'])
        paused=server.request(url+'&action=pause',dict(schema_version=1,event_id='http-pause',expected_revision=decided['revision'],reason='Release owned manual claim through supported control'),csrf);assert paused['planning']['paused']
        work=server.request(url+'&action=resume',dict(schema_version=1,event_id='http-resume',expected_revision=paused['revision'],reason='Resume with previous decisions and spending retained'),csrf);assert not work['planning']['paused']
        before=snapshot();assert server.request(url+'&action=judge',first,csrf)==work and snapshot()==before
        assert server.request(url)==owned.ok(['planning','show',owned.work])==work
        assert not list(owned.root.glob('fixture-prompt-*.bin')) and all(row in review.public.finance(owned.root,owned.work) for row in money)
        server.stop()
        f.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,binary_sha256=profile['sha256'],actual_http=True,head_and_read=True,controls=['enable','judge','renew','claim','lease-renew','decide','pause','resume'],refusals=refusals,cli_identical=True,no_model_process=True,pending_inputs_preserved_by_direct_actions=True,history_readable_under_lower_cap=True,replay_without_transition=True,global_watchdog=None,installed=False,authenticated_human=False))
        print('PASS protected native HTTP planning controls, policy, replay and refusal preservation',flush=True)
    except BaseException as error:
        primary=error;f.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()))
    finally:
        if server is not None:http.finish(server,primary)
        elif primary is not None:raise primary


if __name__=='__main__':main()
