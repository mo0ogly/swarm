#!/usr/bin/env python3
"""Verify historical native originals through exact public publication replay.

Requires a genuinely owned inert cumulative fixture and qualified normal ELF.
No positive SQL seed, paid provider, live mission or global watchdog. Every
corruption follows valid public control and exact restoration before recovery.
"""
import argparse,json,os,sqlite3,traceback
from pathlib import Path
from rust_native_task_validation import invoke
from rust_native_cumulative_publication import restoration
from rust_native_managed_reviewer import inspection


def run(binary,profile,proof,out):
    """Keep canonical Work/rows/provider bytes unchanged across public refusals.

    The saved fixture result and birth must identify this exact isolated Root.
    A public replay supplies its original request, never a fresh publication.
    Alter only this fixture's old/latest public certificate, restoring exact
    bytes with primary and restoration failures retained independently. Verify
    original native-reader causes, no empty success, and real recovery. This
    demonstrates publication integrity, not current PAEA authority or Git effect.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    f=inspection.native.fixture
    qualified=json.loads(profile.read_bytes())
    assert qualified['no_fixture_feature'] and qualified['all_core_features_empty'] and qualified['all_core_test_false']
    assert qualified['sha256']==f.digest(binary.read_bytes())
    source=json.loads((proof/'result.json').read_text())
    assert source['status']=='PASS' and source['cumulative_publication'] and not source['paid_provider']
    root=Path(source['root']);work=source['work']
    assert any(json.loads(p.read_bytes()).get('root')==str(root) for p in proof.rglob('owned-root-*.json'))
    published=json.loads((proof/'published.json').read_bytes())
    assert published['root']==str(root) and published['work']==work
    request=published['request'];environment=dict(os.environ,CODEX_HOME=str(root/'codex-home'))
    def call(label,args,value=None,success=True):
        """Retain all real CLI channels; refuse errors or empty success explicitly."""
        code,wire=invoke(binary,root,out,label,args,value,environment)
        assert (code==0)==success,(label,code,wire)
        if not success:assert wire.get('failure',{}).get('causes'),(label,wire)
        return wire
    before=call('before',['work','show',work])['work']
    original_marker=(root/'managed-reviewer-invoked.json').read_bytes()
    db=sqlite3.connect((root/'.swarm/state.db').as_uri()+'?mode=ro',uri=True)
    def rows():
        """Read exact canonical Work/event/worker rows without changing the fixture."""
        return dict(work=db.execute('SELECT revision,body FROM works WHERE id=?',(work,)).fetchall(),
            events=db.execute('SELECT id,revision,kind,request,payload FROM events WHERE work_id=? ORDER BY revision',(work,)).fetchall(),
            workers=db.execute('SELECT * FROM managed_attempts ORDER BY agent_id').fetchall())
    original=rows();checks=[]
    with restoration(db.close):
        for task_id in ['task','task-2']:
            task=next(t for t in before['tasks'] if t['id']==task_id)
            control=call(task_id+'-valid-replay',['task','publish',work],request)
            assert control['replayed'] and control['work']==before and not control['provider_started']
            file=root/task['independent_review']['receipt'];raw=file.read_bytes();f.publish(out/(task_id+'-original-certificate.bytes'),raw)
            with restoration(lambda:f.publish(file,raw)):
                f.publish(file,raw+b' ')
                denied=call(task_id+'-old-original-refused',['task','publish',work],request,False)
                assert any(code.startswith('native-review-') or code.startswith('review-evidence-') or code=='local-storage' for code in denied['failure']['causes']),denied
                call(task_id+'-work-read-also-refuses',['work','show',work],success=False)
                assert rows()==original and (root/'managed-reviewer-invoked.json').read_bytes()==original_marker
            recovered=call(task_id+'-restored-replay',['task','publish',work],request)
            assert recovered['replayed'] and recovered['work']==before and not recovered['provider_started']
            assert call(task_id+'-restored-work',['work','show',work])['work']==before
            checks.append(task_id+'-original-refuses-latest-public-replay-and-recovers')
        assert rows()==original and (root/'managed-reviewer-invoked.json').read_bytes()==original_marker
        checks.append('no-new-work-event-worker-or-provider')
    f.publish(out/'result.json',dict(status='PASS',checks=checks,binary_sha256=f.digest(binary.read_bytes()),fixture_proof=str(proof),provider_started=False,git_ref_effect_verified=False,global_watchdog=None))
    print('PAEA_HISTORICAL_PUBLICATION_PASS',len(checks),'named assertions',flush=True)


def main():
    """Pin normal binary/profile and helpers; preserve original failures.

    Each path must come from CLI or its matching SWARM_PAEA_* verification
    variable. Missing/empty configuration refuses; no default binary, fixture,
    timeout, effect policy or existing evidence directory is adopted.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    keys={'binary':'SWARM_PAEA_MANAGED_GIT_BINARY','profile':'SWARM_PAEA_MANAGED_GIT_PROFILE','fixture':'SWARM_PAEA_CURRENT_PUBLICATION_FIXTURE','output':'SWARM_PAEA_HISTORICAL_PUBLICATION_EVIDENCE'}
    for name,key in keys.items():
        if key in os.environ and not os.environ[key]:parser.error(key+' must be nonempty')
        parser.add_argument('--'+name,type=Path,default=os.environ.get(key),required=key not in os.environ)
    a=parser.parse_args();f=inspection.native.fixture
    inputs=dict(binary_sha256=f.digest(a.binary.read_bytes()),profile_sha256=f.digest(a.profile.read_bytes()),helpers=inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(a.binary.resolve(),a.profile.resolve(),a.fixture.resolve(),a.output.resolve())
        assert f.digest(a.binary.read_bytes())==inputs['binary_sha256'] and f.digest(a.profile.read_bytes())==inputs['profile_sha256']
        assert all(f.digest(Path(k).read_bytes())==v for k,v in inputs['helpers'].items())
        f.publish(a.output/'inputs.json',inputs)
    except BaseException:
        if a.output.exists():f.publish(a.output/'inputs.json',inputs);f.publish(a.output/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
