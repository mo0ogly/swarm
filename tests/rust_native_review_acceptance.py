#!/usr/bin/env python3
"""Probe normal acceptance after real owned worker/control/reviewer originals.

No positive SQL seeding, real mission, paid model or authenticated human claim.
Each fault has a successful control first and a fresh fully generated fixture.
Corruption changes only owned evidence temporarily; originals are restored.
"""
import argparse,json,subprocess,sys,traceback
from pathlib import Path
import rust_native_reviewer as public


def prepare(binary,profile,out,review_case='passed',child_task_budget=1,child_activation_budget=3):
    """Generate genuine public evidence and retain its normal-profile provenance.

    The existing normal lifecycle tests all their own guards before returning;
    no task is accepted here, no fake review/SQL receipt is supplied. The saved
    child activation budget is explicit fixture data, default3 for legacy cases;
    callers may preauthorize the additional steps their scenario actually needs.
    """
    command=[sys.executable,str(Path(__file__).with_name('rust_native_reviewer.py')),'--binary',str(binary),'--profile',str(profile),'--output',str(out),'--case',review_case,'--child-task-budget',str(child_task_budget),'--child-activation-budget',str(child_activation_budget)]
    result=subprocess.run(command,stdin=subprocess.DEVNULL)
    if result.returncode:raise ValueError('native-acceptance-valid-source-failed')
    marker=json.loads((out/'fixture.json').read_text());root=Path(marker['root']);work=marker['work']
    # Preserve every original control frame, not only the one later corrupted.
    originals=out/'controls-originals';originals.mkdir();hashes={}
    for path in sorted((root/'.swarm').glob('native-validation-*.receipt')):
        assert path.is_file() and not path.is_symlink();raw=path.read_bytes();magic,extent,digest,body=raw.split(b'\n',3)
        assert magic==b'SR1' and int(extent)==len(body) and digest.decode()==public.f.digest(body)
        public.f.publish(originals/path.name,raw);assert path.read_bytes()==raw;hashes[path.name]=public.f.digest(raw)
    assert hashes
    public.f.publish(originals/'retention.json',dict(root=str(root),work=work,binary_sha256=marker['binary_sha256'],files=hashes,scope='Original owned control frames; no authenticated human or invoice'))
    return root,work


def accept(binary,root,work,out,label,expected):
    """Attempt one public status change; require expected state and failure purity.

    Local operator fields are data, not authenticated human authority. Failed
    operations must preserve Work and financial rows and never invoke a provider.
    """
    before=public.preview.shown(binary,root,out,label+'-before',work);rows=public.finance(root,work);invoked=(root/'reviewer-invoked.json').read_bytes()
    request=dict(schema_version=1,event_id=label,expected_revision=before['revision'],id='task',status='accepted',next='Owned fixture status transition after actual native evidence')
    code,value=public.native.invoke(binary,root,out,label,['task','update',work],request)
    after=public.preview.shown(binary,root,out,label+'-after',work)
    assert public.finance(root,work)==rows and (root/'reviewer-invoked.json').read_bytes()==invoked
    if expected:assert code==0 and after['tasks'][0]['status']=='accepted',value
    else:
        assert code!=0 and after==before,dict(exit_code=code,failure=value.get('failure'),unexpected_status=after['tasks'][0]['status'])
        if 'control-private' in label or 'control-step' in label or label=='corrupted-private':
            assert 'storage-operation:Read' in value['failure']['causes'],value
        elif label=='corrupted-public':assert 'native-review-context-public-changed' in value['failure']['causes'],value
        elif label=='corrupted-validation-public':assert 'review-native-public-receipt-changed' in value['failure']['causes'],value
    return value


def main():
    """Run a positive witness then private/public corruption with exact restoration.

    Keep all original channels and partial findings. An unexpected acceptance
    is a FAIL, never overwritten by the later restoration or a green average.
    Tests have no global watchdog; all native product policies remain effective.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--faults',nargs='+',choices=['private','public','report','control-private','control-step','validation-public'],default=['private','public','control-private','control-step','validation-public','report']);a=p.parse_args();binary=a.binary.resolve();profile=a.profile.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);completed=[]
    public.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        control=out/'control';root,work=prepare(binary,profile,control);accept(binary,root,work,control,'accepted-control',True);completed.append('control');public.f.publish(out/'progress.json',dict(completed=completed))
        for variant in a.faults:
            case=out/variant;root,work=prepare(binary,profile,case);before=public.preview.shown(binary,root,case,'original',work);review=before['tasks'][0]['independent_review']
            if variant=='public':path=root/review['receipt']
            elif variant=='report':path=root/review['report']
            elif variant in ('control-private','control-step'):
                originals=list((root/'.swarm').glob('native-validation-'+('job' if variant=='control-private' else 'step')+'-*.receipt'))
                # This generated fixture has exactly one task/control job, not a product cap.
                assert len(originals)==1;path=originals[0]
            elif variant=='validation-public':path=root/before['tasks'][0]['automatic_validation']['receipt']
            else:path=root/'.swarm'/('native-review-'+public.f.digest(review['id'].encode())+'.receipt')
            raw=path.read_bytes();public.f.publish(case/(variant+'-original.bytes'),raw);changed=bytearray(raw);changed[-1]^=1
            try:
                public.f.publish(path,bytes(changed));accept(binary,root,work,case,'corrupted-'+variant,False)
            finally:public.f.publish(path,raw)
            accept(binary,root,work,case,'restored-'+variant,True);completed.append(variant);public.f.publish(out/'progress.json',dict(completed=completed));print('PASS acceptance '+variant+' refusal/recovery',flush=True)
        for denied in ['changes-requested','domain-invalid']:
            case=out/('nonverdict-'+denied);root,work=prepare(binary,profile,case,denied)
            accept(binary,root,work,case,'nonverdict-refused-'+denied,False)
            completed.append('nonverdict-'+denied);public.f.publish(out/'progress.json',dict(completed=completed))
        public.f.publish(out/'result.json',dict(status='PASS',cases=completed,accepted_fixtures_only=True,authenticated_human=False,real_mission=False,recovery_execs=0,global_watchdog=None));print('PASS native review acceptance integrity',flush=True)
    except BaseException:public.f.publish(out/'failure.json',dict(status='FAIL',completed=completed,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
