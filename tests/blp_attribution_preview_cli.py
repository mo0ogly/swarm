#!/usr/bin/env python3
"""Exercise readonly attribution preview and explicit apply via a qualified CLI.

Allocate an inert configured fixture; retain every input/channel and exact state
hash. No provider, live database, retry, or verification deadline is used.
"""
import argparse,hashlib,json,os,subprocess,sys
from pathlib import Path
from fixture_area import fixture_root


def main():
    """Verify a valid apply control, preview no-write, refusal and exact replay.

    Require a fresh evidence directory and matching immutable production profile.
    A failure retains original channels and fixture path for diagnosis.
    """
    if sys.flags.optimize:raise RuntimeError('optimized Python disables required assertions')
    p=argparse.ArgumentParser(description=__doc__)
    for key in ('binary','profile','output'):p.add_argument('--'+key,type=Path,required=True)
    a=p.parse_args();binary=a.binary.resolve(strict=True);out=a.output.resolve();out.mkdir(mode=0o700)
    profile=json.loads(a.profile.read_text());assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE'
    assert hashlib.sha256(binary.read_bytes()).hexdigest()==profile['sha256']
    root=fixture_root('blp-preview-');(out/'root.txt').write_text(str(root));env=dict(os.environ);env['SWARM_TRUST_TOKEN']='';calls=[]
    def call(args,value=None,success=True):
        """Preserve actual argv/input/output; reject unexpected failure/success."""
        n=len(calls);argv=[str(binary),*args,'--root',str(root),'--json']
        if value is not None:
            f=out/f'{n}.input.json';f.write_text(json.dumps(value));argv+=['--input',str(f)]
        r=subprocess.run(argv,env=env,capture_output=True);(out/f'{n}.stdout').write_bytes(r.stdout);(out/f'{n}.stderr').write_bytes(r.stderr)
        calls.append(dict(argv=argv,exit=r.returncode));(out/'commands.json').write_text(json.dumps(calls,indent=2))
        assert (r.returncode==0)==success,(args,r.returncode,r.stderr.decode());return json.loads(r.stdout if success else r.stderr)
    call(['init'])
    call(['trust','config-apply'],dict(schema_version=1,event_id='trust-init',expected_revision=0,reason='Owned preview fixture',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),identities=[dict(subject='admin',integrity=3,roles=['admin'],token='fixture-admin-token'),dict(subject='worker',integrity=1,roles=['requester'],token='fixture-worker-token'),dict(subject='approver',integrity=2,roles=['approver'],token='fixture-approver-token')]))
    env['SWARM_TRUST_TOKEN']='fixture-admin-token'
    call(['trust','blp-policy-apply'],dict(schema_version=1,event_id='policy-init',expected_revision=0,expected_trust_revision=1,reason='Explicit fixture policy',policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public')],compartments=[],clearances=dict(admin=dict(level='public',compartments=[])))))
    def change(event,digit):return dict(schema_version=1,event_id=event,expected_revision=1,expected_trust_revision=1,canonical_digest=digit*64,project_path=str(root),classification=dict(level='public',compartments=[]),reason='Explicit fixture object')
    control=call(['trust','blp-object-attribute'],change('control','a'));assert control['attributed'] is True;print('PASS valid mutating producer control',flush=True)
    state=root/'.swarm/trust-actions.json';before=state.read_bytes();(out/'before.sha256').write_text(hashlib.sha256(before).hexdigest())
    value=change('preview','b');preview=call(['trust','blp-object-attribute-preview'],value)
    assert preview['preview'] is True and preview['applied'] is False and preview['attributed'] is False and preview['would_attribute'] is True
    assert state.read_bytes()==before;print('PASS readonly preview exact journal unchanged',flush=True)
    stale=dict(value,expected_revision=0);call(['trust','blp-object-attribute-preview'],stale,False);assert state.read_bytes()==before
    applied=call(['trust','blp-object-attribute'],value);assert applied['attributed'] is True;after=state.read_bytes();assert after!=before
    replay=call(['trust','blp-object-attribute'],value);assert replay==applied and state.read_bytes()==after
    call(['trust','blp-object-attribute-preview'],value,False);assert state.read_bytes()==after
    (out/'result.json').write_text(json.dumps(dict(status='PASS',checks=['valid-control','preview-readonly','stale-no-write','apply-once','replay-no-write','recorded-preview-refusal'],binary_sha256=profile['sha256'])));print('PASS stale/refusal, explicit apply, replay',flush=True)

if __name__=='__main__':main()
