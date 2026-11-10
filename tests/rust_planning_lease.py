#!/usr/bin/env python3
"""Replay explicit live scope-lease renewals on owned inactive public Roots.

No clock sleeps, temporal watchdog, SQL injection or provider process. Healthy
controls precede each fault. Preserve original command responses and failed Roots.
"""
import argparse,traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import configured,claim_request,apply_policy


def claimed(owned,label):
    """Enable and claim a fresh public Work with explicitly chosen fixture duration."""
    enabled=configured(owned,label)
    return owned.ok(['planning','claim',owned.work],claim_request(enabled,label+'-claim'))


def renewal(work,label,**changes):
    """Echo current durable concurrency fences; no generation or duration fallback."""
    scope=next(s for s in work['planning']['scopes'] if s['id']=='root')
    value=dict(schema_version=1,event_id=label,expected_revision=work['revision'],scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],lease_seconds=60);value.update(changes);return value


def refuses(owned,value,expected):
    """Assert exact safe failure and unchanged public/agent/reservation state."""
    before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();code,result=owned.call(['planning','lease-renew',owned.work],value)
    assert code!=0 and result['failure']['code']==expected,result
    assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot
    assert not (owned.root/'model-received.json').exists()
    return dict(code=expected,unchanged=True)


def main():
    """Require renewable live fences, public policy consumption and stale refusal."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_planning_activation.py'),Path('tests/rust_planning_bootstrap.py'),Path('tests/rust_agent_model_route.py')]}));owned=None
    try:
        owned=fixture.Fixture(binary,out,'lease');cases=[]
        faults=[('holder',dict(holder='other-holder'),'stale_lease'),('generation',dict(generation=99),'stale_lease'),('zero-generation',dict(generation=0),'stale_lease'),('scope-revision',dict(scope_revision=99),'revision_conflict'),('unknown-scope',dict(scope='unknown'),'planning-scope-not-found'),('unapplied-reason',dict(reason='Unapplied metadata'),'planning-claim-field-not-applicable'),('too-short',dict(lease_seconds=4),'planning-lease-outside-policy'),('too-long',dict(lease_seconds=301),'planning-lease-outside-policy')]
        for label,patch,expected in faults:
            healthy=claimed(owned,label+'-healthy');good=owned.ok(['planning','lease-renew',owned.work],renewal(healthy,label+'-healthy-renew'));assert good['planning']['activations']==1 and good['planning']['decisions']==0
            candidate=claimed(owned,label+'-fault');value=renewal(candidate,label+'-renew',**patch);cases.append(dict(case=label,**refuses(owned,value,expected)));recovered=owned.ok(['planning','lease-renew',owned.work],renewal(candidate,label+'-renew'));assert recovered['planning']['activations']==1
        # Keep this positive range edit valid under configured v27 defaults; no
        # field is fabricated for real older binaries without wait policy.
        saved_default=owned.ok(['engine-settings','show'])['values'].get('planning_default_lease_seconds')
        default_change={'planning_default_lease_seconds':'8'} if saved_default is not None else {}
        candidate=claimed(owned,'policy');value=renewal(candidate,'policy-renew',lease_seconds=10);before=owned.ok(['work','show',owned.work]);apply_policy(owned,'tighten-renew-bounds',dict(planning_scope_lease_min_seconds='7',planning_scope_lease_max_seconds='9',**default_change));assert owned.ok(['work','show',owned.work])==before
        refuses(owned,value,'planning-lease-outside-policy');apply_policy(owned,'repair-renew-bound',dict(planning_scope_lease_max_seconds='10'));recovered=owned.ok(['planning','lease-renew',owned.work],value);assert recovered['planning']['activations']==1;assert owned.ok(['planning','lease-renew',owned.work],value)==recovered
        restore_default={'planning_default_lease_seconds':saved_default} if saved_default is not None else {}
        apply_policy(owned,'restore-renew-bounds',dict(planning_scope_lease_min_seconds='5',planning_scope_lease_max_seconds='300',**restore_default))
        candidate=claimed(owned,'paused');old=renewal(candidate,'paused-renew');paused=owned.ok(['planning','pause',owned.work],dict(schema_version=1,event_id='pause-renewal',expected_revision=candidate['revision']));refuses(owned,{**old,'expected_revision':paused['revision']},'planning-decision-stopped')
        resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='resume-renewal',expected_revision=paused['revision'],reason='Explicit changed instruction after pause'));scope=next(s for s in resumed['planning']['scopes'] if s['id']=='root');fresh=owned.ok(['planning','claim',owned.work],claim_request(resumed,'replacement-claim',scope_revision=scope['revision'],holder='replacement-holder'))
        refuses(owned,{**old,'expected_revision':fresh['revision'],'scope_revision':scope['revision']},'stale_lease');renewed=owned.ok(['planning','lease-renew',owned.work],renewal(fresh,'replacement-renew'));assert renewed['planning']['activations']==2 and renewed['planning']['decisions']==0
        assert not (owned.root/'model-received.json').exists();fixture.publish(out/'result.json',dict(status='PASS',negative_cases=cases,saved_policy_consumed=True,pause_and_replaced_generation_refused=True,activations_preserved=True,provider_started=False,scope='Explicit operator lease renewal only; no automatic heartbeat or authenticated agent identity'))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
