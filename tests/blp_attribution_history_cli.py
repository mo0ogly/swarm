#!/usr/bin/env python3
"""Exercise the qualified production CLI's historical BLP validation.

Only an owned inert Root is used. Public init/policy operations provide a valid
control; synthetic attributions are then injected atomically into this fixture
only, like the engine corruption regression. This tests read refusal/redaction
and recovery, not the attribution producer or human declassification.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from fixture_area import fixture_area, fixture_root
from rust_agent_model_route import publish


def digest(path):
    """Hash the complete image with a 1 MiB IO buffer, without a size cutoff.

    Streaming avoids loading a large debug executable into memory beside Cargo.
    Read errors propagate; the buffer is a technical allocation, not policy.
    """
    value=hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda:source.read(1024*1024),b''):value.update(chunk)
    return value.hexdigest()


def main():
    """Assert exact qualified image, valid history, rejection and exact recovery.

    Arguments select immutable binary/profile and a fresh evidence directory.
    Original channels, inputs and fixture identity are retained; no live state,
    provider, watchdog or implicit retry. Fixture poisoning never grants rights.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--profile',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();binary=args.binary.resolve(strict=True)
    out=args.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    profile=json.loads(args.profile.read_bytes());sha=digest(binary)
    assert profile['sha256']==sha and profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE'
    assert profile['no_fixture_feature'] and profile['all_core_features_empty'] and profile['all_core_test_false']
    root=fixture_root('swarm-blp-history-');checks=[];sequence=0
    publish(out/'identity.json',dict(binary=str(binary),binary_sha256=sha,profile=str(args.profile.resolve()),root=str(root),scope='owned inert CLI history fixture; no production mutation'))

    def call(operation,value=None,token='fixture-admin-token'):
        """Run one public CLI command, retaining exact argv and raw channels.

        Only synthetic credentials enter the environment; none are printed.
        Return actual exit/JSON; malformed or missing diagnostics fail visibly.
        """
        nonlocal sequence
        sequence+=1;name=f'{sequence:02d}-{operation}'
        argv=[str(binary),'--root',str(root),'--json']
        argv+=['init'] if operation=='init' else ['trust',operation]
        if value is not None:
            request=out/(name+'.input.json');publish(request,value)
            argv+=['--input',str(request)]
        env=dict(os.environ,SWARM_TRUST_TOKEN=token,TMPDIR=str(fixture_area()))
        result=subprocess.run(argv,stdin=subprocess.DEVNULL,capture_output=True,env=env)
        publish(out/(name+'.argv.json'),argv);publish(out/(name+'.stdout'),result.stdout);publish(out/(name+'.stderr'),result.stderr)
        return result.returncode,json.loads(result.stdout if result.returncode==0 else result.stderr)

    def check(name,condition):
        """Persist each named result immediately, preserving the first failure."""
        checks.append(dict(name=name,passed=bool(condition)));publish(out/'checks.json',checks)
        assert condition,name
        print('PASS '+name,flush=True)

    check('qualified-image-init',call('init')[0]==0)
    config=dict(schema_version=1,event_id='trust-init',expected_revision=0,reason='Owned inert history fixture',
        policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),
        identities=[dict(subject='admin',integrity=3,roles=['admin'],token='fixture-admin-token'),
            dict(subject='worker',integrity=1,roles=['requester'],token='fixture-worker-token'),
            dict(subject='approver',integrity=2,roles=['approver'],token='fixture-approver-token')])
    check('trust-policy-produced',call('config-apply',config,token='')[0]==0)
    policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public'),dict(id='confidential',name_fr='Confidentiel',name_en='Confidential')],
        compartments=['client-a'],clearances={'admin':dict(level='confidential',compartments=['client-a'])})
    for revision in [0,1]:
        proposal=dict(schema_version=1,event_id=f'blp-policy-{revision}',expected_revision=revision,expected_trust_revision=1,reason='Explicit fixture policy',policy=policy)
        check(f'blp-revision-{revision+1}-produced',call('blp-policy-apply',proposal)[0]==0)
    state=root/'.swarm/trust-actions.json';original=json.loads(state.read_bytes())
    marker='PRIVATE_BLP_HISTORY_FIXTURE';key=hashlib.sha256(marker.encode()).hexdigest()
    for index,entry in enumerate(original['confidentiality']['history']):
        entry['attributions']=[dict(canonical_digest=key,project_path=list(str(root).encode()),policy_revision=index+1,
            level='confidential',compartments=['client-a'],actor='admin',at='2026-11-01T00:00:00Z',reason=marker)]
    assert root.parent==fixture_area().resolve() and root.name.startswith('swarm-blp-history-')
    publish(state,original);valid=state.read_bytes();publish(out/'valid-history-state.json',valid)
    code,view=call('blp-show',{})
    check('equal-label-history-positive-control',code==0 and len(view['record']['history'])==2)
    check('positive-read-is-readonly',state.read_bytes()==valid)
    for label,compartments in [('public',[]),('confidential',[])]:
        poisoned=json.loads(valid);entry=poisoned['confidentiality']['history'][1]['attributions'][0]
        entry['level']=label;entry['compartments']=compartments
        publish(state,poisoned);before=state.read_bytes()
        code,error=call('blp-show',{})
        check(f'{label}-downgrade-refuses',code!=0 and error['failure']['code']=='trust-proof-integrity')
        check(f'{label}-refusal-preserves-journal',state.read_bytes()==before)
        check(f'{label}-refusal-redacts-material',marker not in json.dumps(error) and key not in json.dumps(error))
        publish(state,valid)
        check(f'{label}-exact-fixture-recovery',call('blp-show',{})[0]==0 and state.read_bytes()==valid)
    check('image-stayed-identical',digest(binary)==sha)
    publish(out/'result.json',dict(status='PASS',checks=checks,binary_sha256=sha,root=str(root),global_watchdog=None,limits='fixture-injected history, no producer/declassification/independent acceptance'))
    print('BLP_HISTORY_CLI_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
