#!/usr/bin/env python3
"""Qualify the Admin object-attribution form with the real served UI.

Uses an owned inert Root, public configuration/policy/producer calls through the
qualified production binary and a real browser against its served Trust page.
One explicit gesture attributes one exact object; session-alone and weak-bearer
refusals are typed and non-mutating. No live project, classified content,
provider, watchdog, retry or fabricated frontend response. The object digest is
the real SHA-256 of fixture bytes the page itself attributes; project_path is
the fixture root, never a real mission.
"""
import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path

from blp_attribution_history_cli import digest
from fixture_area import fixture_area
from rust_agent_model_route import Fixture, publish
from rust_native_http_cli import Server


def main():
    """Assert the Admin object-attribution form end to end on the served page.

    Requires the immutable qualified production binary/profile and a fresh
    evidence directory. Original channels, inputs and artifact hashes survive
    failure. Browser credentials never enter retained reports.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:
        parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args()
    binary=args.binary.resolve(strict=True)
    out=args.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    profile=json.loads(args.profile.read_bytes())
    sha=digest(binary)
    assert profile['sha256']==sha and profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE'
    f=Fixture(binary,out,'object-attribute');checks=[]
    def check(name,condition):
        checks.append(dict(name=name,passed=bool(condition)));publish(out/'checks.json',checks)
        assert condition,name
        print('PASS '+name,flush=True)
    try:
        f.environment.update(SWARM_TRUST_TOKEN='fixture-admin-token')
        def trust(op,value,token='fixture-admin-token'):
            f.environment['SWARM_TRUST_TOKEN']=token
            try:
                code,result=f.call(['trust',op],value)
                assert code==0,(op,result)
                return result
            finally:f.environment['SWARM_TRUST_TOKEN']='fixture-admin-token'
        trust('config-apply',dict(schema_version=1,event_id='trust-init',expected_revision=0,reason='Owned object-attribution fixture',
            policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),
            identities=[dict(subject='admin',integrity=3,roles=['admin'],token='fixture-admin-token'),
                dict(subject='worker',integrity=1,roles=['requester'],token='fixture-worker-token'),
                dict(subject='approver',integrity=2,roles=['approver'],token='fixture-approver-token')]))
        policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public'),
            dict(id='confidential',name_fr='Confidentiel',name_en='Confidential')],
            compartments=['client-a'],
            clearances={'admin':dict(level='confidential',compartments=['client-a'])})
        trust('blp-policy-apply',dict(schema_version=1,event_id='blp-policy-0',expected_revision=0,
            expected_trust_revision=1,reason='Owned fixture policy',policy=policy))
        show=trust('blp-show',{})
        revision=int(show['record']['history'][-1]['revision'])
        trust_revision=int(show['trust_revision'])
        check('policy-current',show['capability']['configured'] is True and revision==1)
        # Each journey combo attributes ONE exact object; the producer refuses a
        # second (digest, project, revision) attribution under another event id
        # (`blp-attribution-conflict`), so the driver supplies one distinct
        # fixture object per combo in the test's fixed combo order.
        combo_order=[lang+'-'+theme+'-'+str(width)
            for lang in ['fr','en'] for theme in ['sombre','etat'] for width in [1280,390]]
        object_digests=[]
        for index,label in enumerate(combo_order):
            object_bytes=b'PRIVATE_BLP_OBJECT_ATTRIBUTE_FIXTURE '+label.encode()+b'\n'
            object_digests.append(hashlib.sha256(object_bytes).hexdigest())
            publish(out/('object-'+label+'.bytes'),object_bytes)
        web=Server(binary,f.root,out,'object-attribute-http',environment=f.environment)
        web.capture_async(out/'http-observer.raw')
        (out/'evidence').mkdir(mode=0o700)
        config=dict(url='http://'+web.address,root=str(f.root),cookie=web.cookie,
            tokens=dict(admin='fixture-admin-token',worker='fixture-worker-token'),
            digests=object_digests,project_path=str(f.root),
            levels=[level['id'] for level in policy['levels']],
            compartments=policy['compartments'],
            expected_revision=str(revision),expected_trust_revision=str(trust_revision),
            output=str(out/'evidence/ui'),
            public_output=str(out/'ui-result.json'),
            compiled_inputs=str(args.profile.parent/'after.json'))
        publish(out/'evidence/ui-config.json',config)
        run=subprocess.run(['node','tests/blp_object_attribute_ui.cjs',str(out/'evidence/ui-config.json')],env=f.environment,capture_output=True)
        publish(out/'ui.stdout',run.stdout);publish(out/'ui.stderr',run.stderr)
        assert run.returncode==0,'browser failed; see retained diagnostics'
        result=json.loads((out/'ui-result.json').read_bytes())
        check('admin-ui-attribution-form-journey',result['status']=='PASS')
        # The producer persisted exactly one attribution per journey object,
        # each bound to the exact fixture bytes that combo named.
        state=json.loads((f.root/'.swarm/trust-actions.json').read_bytes())
        attributions=state['confidentiality']['history'][-1]['attributions']
        check('one-explicit-attribution-persisted',len(attributions)==len(combo_order)
            and [a['canonical_digest'] for a in attributions]==object_digests
            # project_path is Vec<u8>: serde emits a byte-array, not a string.
            and all(bytes(a['project_path']).decode()==str(f.root) for a in attributions)
            and all(a['level']=='confidential' and a['actor']=='admin' for a in attributions))
        check('qualified-production-image-unchanged',digest(binary)==sha)
    except BaseException as error:
        primary=error
        publish(out/'result.json',dict(status='FAIL',checks=checks,binary_sha256=sha,
            error=repr(error),root=str(f.root)))
        print('OBJECT_ATTRIBUTE_PUBLIC_FAIL '+repr(error),flush=True)
        raise
    publish(out/'result.json',dict(status='PASS',checks=checks,binary_sha256=sha,root=str(f.root),
        scope='owned inert Admin object-attribution fixture; no live project, no declassification, no bulk scan',
        global_watchdog=None,limits='UI form drives the existing exact-once/monotonic producer; physical lineage and BLP-06 remain open'))
    print('OBJECT_ATTRIBUTE_JOURNEY_SHA256 '+digest(out/'result.json'),flush=True)
    print('OBJECT_ATTRIBUTE_PUBLIC_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':
    main()
