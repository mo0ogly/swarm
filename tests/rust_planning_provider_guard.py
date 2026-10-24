#!/usr/bin/env python3
"""Replay claim cooldown guards while metadata configuration remains available.

Only an owned fixture cooldown file is seeded; no real account, quota clearing,
provider process or policy timer is used. Public CLI mutations and read-only ledger
observations must refuse atomically and recover after fixture preconditions change.
"""
import argparse,traceback,os
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import configured,claim_request
from rust_planning_budget import ledger


def main():
    """Require selected-provider cooldown before spending activation/reservation."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_planning_activation.py'),Path('tests/rust_planning_budget.py'),Path('tests/rust_planning_bootstrap.py'),Path('tests/rust_agent_model_route.py')]}));owned=None
    try:
        owned=fixture.Fixture(binary,out,'planning-cooldown');directory=owned.root/'.swarm/provider-cooldowns';assert not directory.exists();directory.mkdir(mode=0o700);file=directory/'fixture.json';cases=[]
        for label,changes,expected in [('active',{},'provider_cooldown'),('malformed-clear',dict(cleared_at='PRIVATE invalid clearance'),'agent-provider-cooldown-clear-time-invalid')]:
            healthy=configured(owned,label+'-healthy');owned.ok(['planning','claim',owned.work],claim_request(healthy,label+'-healthy-claim'))
            value=dict(provider='fixture',observed_at='2026-10-20T00:00:00Z',resets_at=0,source='Owned legacy cooldown fixture',signal='backoff');value.update(changes);fixture.publish(file,value);os.chmod(file,0o600);fixture.publish(out/(label+'-cooldown.json'),value)
            # Configuring the plan is non-executing and remains possible under cooldown.
            candidate=configured(owned,label+'-fault');before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();money=ledger(owned);request=claim_request(candidate,label+'-claim');code,result=owned.call(['planning','claim',owned.work],request)
            assert code!=0 and result['failure']['code']==expected,result;assert 'PRIVATE' not in str(result)
            assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot and ledger(owned)==money
            file.unlink();recovered=owned.ok(['planning','claim',owned.work],request);assert recovered['planning']['activations']==1;cases.append(dict(case=label,code=expected,unchanged=True,recovered=True))
        assert not (owned.root/'model-received.json').exists();fixture.publish(out/'result.json',dict(status='PASS',cases=cases,configuration_under_cooldown_allowed=True,provider_started=False,scope='Owned fixture cooldown data; removal proves no real provider/account health or authorization'))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
