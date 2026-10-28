#!/usr/bin/env python3
"""Replay saved planner response policy through the actual offline native CLI.

Owned inactive Roots only; no provider is started. A parsed proposal is data,
not a supported-operation verdict, lease identity or applied business decision.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import apply_policy


def response(**changes):
    """One actual terminal provider fixture, with a complete untrusted proposal."""
    value=dict(input_events=['event-1'],reason='Observe',operations=[]);value.update(changes)
    return dict(type='result',structured_output=value)


def observe(owned,value,expected=None):
    """Require exact refusal/success, safe output and unchanged Root/settings."""
    before=owned.snapshot();settings=(owned.root/'.swarm/engine-settings.json').read_bytes();code,result=owned.call(['planning-output','check'],value if isinstance(value,bytes) else (json.dumps(value,ensure_ascii=False,separators=(',',':'))+'\n').encode())
    if expected:
        assert code!=0 and result['failure']['code']==expected,result;assert 'PRIVATE' not in str(result)
    else:
        assert code==0 and result['proposal_valid'] and result['process_started'] is False and result['decision_applied'] is False,result
    assert owned.snapshot()==before and (owned.root/'.swarm/engine-settings.json').read_bytes()==settings
    assert not (owned.root/'model-received.json').exists();return result


def main():
    """Save/reopen/consume caps, reject drift safely and explicitly adopt real v25."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--legacy-binary',type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_agent_model_route.py'),Path('tests/rust_planning_activation.py')]}))
    try:
        owned=fixture.Fixture(binary,out,'planning-output');assert owned.ok(['engine-settings','show'])['schema_version']==26
        good=observe(owned,response());assert good['input_count']==1 and good['operation_count']==0 and good['reason_bytes']==7
        observe(owned,response());wire=(json.dumps(dict(type='result',result=json.dumps(dict(input_events=['event-1'],reason='bad X',operations=[]),separators=(',',':'))),separators=(',',':'))+'\n').encode();bad=wire.replace(b'bad X',b'bad \xff');assert bad!=wire;observe(owned,wire);observe(owned,bad,'planning-output-utf8');observe(owned,wire)
        observe(owned,response());observe(owned,response(holder='PRIVATE'),'planning-proposal-format')
        observe(owned,response());observe(owned,dict(type='assistant',message=dict(content=[dict(type='text',text='PRIVATE')])),'planning-output-terminal-required')
        # A valid observation is a real positive witness before caps make a complete
        # nonempty response impossible. Zero must never silently mean unlimited.
        observe(owned,response());apply_policy(owned,'disable-structured-response',dict(planning_response_max_bytes='0'));observe(owned,response(),'planning-response-byte-limit-exceeded')
        apply_policy(owned,'small-structured-response',dict(planning_response_max_bytes='128'));observe(owned,response());observe(owned,response(reason='r'*129),'planning-response-byte-limit-exceeded')
        apply_policy(owned,'unlimited-structured-response',dict(planning_response_max_bytes=None));observe(owned,response());assert owned.ok(['engine-settings','show'])['values']['planning_response_max_bytes'] is None
        # Parsing remains distinct from action applicability: an unsupported kind
        # can be syntactically decoded but is never applied by this diagnostic.
        parsed=observe(owned,response(operations=[dict(kind='close',id='root')]));assert parsed['operation_count']==1 and parsed['decision_applied'] is False
        if a.legacy_binary:
            legacy=a.legacy_binary.resolve();fixture.publish(out/'legacy-binary.json',dict(path=str(legacy),sha256=fixture.digest(legacy.read_bytes())));old=fixture.Fixture(legacy,out,'legacy-output');assert old.ok(['engine-settings','show'])['schema_version']==25;old.binary=binary
            observe(old,response(),'settings-planning-response-not-configured');catalog=old.ok(['engine-settings','describe']);fields=[f['name'] for f in catalog['fields'] if f['introduced_schema_version']==26];assert fields==['planning_response_max_bytes'];apply_policy(old,'adopt-structured-response',dict(planning_response_max_bytes='128'),target_schema=26);observe(old,response())
        fixture.publish(out/'result.json',dict(status='PASS',saved_zero_finite_null_consumed=True,refusals_without_mutation=True,legacy_adoption=bool(a.legacy_binary),provider_started=False,decision_applied=False,scope='Offline structured data validation, no runtime gate or operation applicability'))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
